"""Exercise the real local service; --generate also runs ACE-Step inference."""
import argparse
import json
import math
import os
from pathlib import Path
import socket
import sqlite3
import struct
import subprocess
import tempfile
import time
import wave

parser = argparse.ArgumentParser()
parser.add_argument("--service", required=True, type=Path)
parser.add_argument("--models", type=Path)
parser.add_argument("--output", required=True, type=Path)
parser.add_argument("--generate", action="store_true")
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
if not hasattr(socket, "AF_UNIX") or os.name == "nt":
    parser.error("This smoke driver targets the Unix-socket transport")

with tempfile.TemporaryDirectory(prefix="mgl-", dir="/tmp") as temporary:
    profile = Path(temporary)
    app_profile = profile / "app-data"
    app_profile.mkdir()
    if args.models:
        with sqlite3.connect(app_profile / "music.db") as db:
            db.execute("CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT)")
            db.execute("INSERT INTO settings VALUES('models_dir', ?)", (json.dumps(str(args.models.resolve())),))
    log = (args.output / "service.log").open("w")
    service = subprocess.Popen([str(args.service.resolve())], env={**os.environ, "MUSIC_LOOPS_PROFILE": temporary, "MUSIC_GENERATOR_PROFILE": str(app_profile)}, stdout=log, stderr=log)

    def call(action, data=None, instance="smoke", expect_ok=True):
        request = json.dumps({"version": 1, "instance": instance, "action": action, "data": data or {}}).encode()
        with socket.socket(socket.AF_UNIX) as client:
            client.settimeout(180)
            client.connect(str(profile / "service.sock"))
            client.sendall(struct.pack("<I", len(request)) + request)

            def read(size):
                result = b""
                while len(result) < size:
                    chunk = client.recv(size - len(result))
                    assert chunk, "Service disconnected"
                    result += chunk
                return result

            response = json.loads(read(struct.unpack("<I", read(4))[0]))
            assert response["ok"] == expect_ok, response
            return response.get("data", response.get("error"))

    def wait_job(identifier):
        previous = None
        for _ in range(900):
            state = call("job", {"id": identifier})
            if state["status"] != previous:
                print(state["status"], state.get("message"), flush=True)
                previous = state["status"]
            if state["status"] not in ("queued", "running"):
                assert state["status"] == "done", state
                return state
            time.sleep(1)
        raise TimeoutError("Generation exceeded fifteen minutes")

    try:
        for _ in range(100):
            if (profile / "service.sock").exists():
                break
            assert service.poll() is None, "Service exited at startup"
            time.sleep(0.1)
        assert (profile / "service.sock").stat().st_mode & 0o777 == 0o600
        expected_models = args.models.resolve() if args.models else app_profile / "models"
        assert Path(call("status")["models"]) == expected_models
        assert Path(call("models")["folder"]) == expected_models
        if not args.models:
            with sqlite3.connect(app_profile / "music.db") as db:
                db.execute("CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT)")
                relocated = profile / "relocated-models"
                relocated.mkdir()
                db.execute("INSERT INTO settings VALUES('models_dir', ?)", (json.dumps(str(relocated)),))
            assert Path(call("models")["folder"]) == relocated
        source = profile / "source.wav"
        with wave.open(str(source), "wb") as audio:
            audio.setparams((2, 2, 48000, 0, "NONE", "not compressed"))
            audio.writeframes(b"".join(struct.pack("<hh", int(math.sin(n * 0.05) * 8000), int(math.sin(n * 0.07) * 6000)) for n in range(48000)))
        imported = call("import", {"path": str(source)})["results"][0]
        edited = call("edit", {"path": imported["path"], "recipe": {"end": 24000, "frames": 24000, "crossfade_ms": 5, "normalize_db": -1}})
        output = Path(edited["path"]).read_bytes()
        assert struct.unpack_from("<I", output, 40)[0] == 24000 * 8
        (args.output / "edited.wav").write_bytes(output)
        cut = call("zero", {"path": imported["path"], "start": 100, "frames": 24000})
        assert 0 <= cut["start"] <= 580 and math.isfinite(cut["score"])
        request = {"slot": 36, "instrument": "drums", "prompt": "Isolated dry funk drums only. No melody, bass or vocals.", "key": "A minor", "bpm": 120, "beats": 16, "seed": 23001, "steps": 50, "variants": 1, "context": []}
        forbidden = {**request, "context": [{"slot": 36, "path": imported["path"]}]}
        call("generate", forbidden, expect_ok=False)
        if args.generate:
            first_id = call("generate", request)["id"]
            call("job", {"id": first_id}, instance="different-instance", expect_ok=False)
            cancelled = call("generate", request)["id"]
            call("cancel", {"id": cancelled})
            assert call("job", {"id": cancelled})["status"] == "cancelled"
            first = wait_job(first_id)
            (args.output / "text2music.json").write_text(json.dumps(first, indent=2))
            request.update(slot=37, instrument="bass", prompt="Isolated fingerstyle funk bass only, matching the drums. No drums, chords or vocals.", seed=23002, context=[{"slot": 36, "path": first["results"][0]["path"]}])
            second = wait_job(call("generate", request)["id"])
            (args.output / "lego.json").write_text(json.dumps(second, indent=2))
            for name, result in (("drums", first), ("bass", second)):
                audio = Path(result["results"][0]["path"]).read_bytes()
                assert struct.unpack_from("<I", audio, 40)[0] == 384000 * 8
                (args.output / f"{name}.wav").write_bytes(audio)
        call("detach")
        print("Service IPC, private socket, WAV import, exact edits, zero check and context rejection passed", flush=True)
    finally:
        service.terminate()
        service.wait(timeout=10)
        log.close()
