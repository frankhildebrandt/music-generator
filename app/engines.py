"""Anbindung an Modell-Server.

acestep.cpp (ace-server) arbeitet asynchron:
  POST /lm    -> {"id": ...}   LM ergänzt Metadaten, Lyrics und audio_codes
  POST /synth -> {"id": ...}   DiT + VAE rendern Audio
  GET  /job?id=X            -> {"status": ...}
  GET  /job?id=X&result=1   -> Ergebnis (/lm: JSON-Array, /synth: multipart/mixed)
"""
import asyncio
import email
import email.policy
import json
import time
from typing import Awaitable, Callable

import httpx

Progress = Callable[[str], Awaitable[None]]


def _fmt_time(seconds: int) -> str:
    m, s = divmod(int(seconds), 60)
    return f"{m}:{s:02d}"



async def _get_retry(c: httpx.AsyncClient, base: str, params: dict, tries: int = 4) -> httpx.Response:
    """GET /job mit Wiederholung: bricht die Verbindung zum Modellserver kurz ab (z. B. beim Abholen des fertigen
    Songs), ist das Ergebnis dort weiter abrufbar. Beim Retry mit frischer Verbindung statt gleich als Fehler zu melden."""
    last: Exception | None = None
    for i in range(tries):
        try:
            if i == 0:
                return await c.get(f"{base}/job", params=params)
            async with httpx.AsyncClient(timeout=120) as fresh:
                return await fresh.get(f"{base}/job", params=params)
        except (httpx.ReadError, httpx.RemoteProtocolError, httpx.ReadTimeout, httpx.WriteError) as e:
            last = e
            await asyncio.sleep(1.5 * (i + 1))
    raise EngineError(f"Verbindung zum Modellserver abgebrochen ({last.__class__.__name__}) – bitte erneut versuchen")


class EngineError(Exception):
    pass


# Felder, die wir an ace-server durchreichen (Rest bleibt auf Server-Defaults)
ACE_FIELDS = {
    "caption", "lyrics", "bpm", "duration", "keyscale", "timesignature",
    "vocal_language", "seed", "lm_temperature", "lm_cfg_scale", "inference_steps",
    "guidance_scale", "output_format", "lm_model", "synth_model", "use_cot_caption",
}


# DiT-Modell: SFT (50 Schritte, klingt deutlich besser als turbo). Standard mit 2 Mrd. Parametern, optional XL mit 4 Mrd.
DIT_MODELS = {"standard": "acestep-v15-sft-Q8_0.gguf", "xl": "acestep-v15-xl-sft-Q8_0.gguf"}


class AceStepCpp:
    name = "acestep_cpp"

    def __init__(self, base_url: str, lm_url: str = "", poll_interval: float = 1.5, timeout_minutes: float = 30,
                 lm_model: str = "", dit: str = "standard"):
        self.dit_model = DIT_MODELS.get(dit, DIT_MODELS["standard"])
        self.base = base_url.rstrip("/")
        # Auf Apple M4 liefert das LM unter Metal Unsinn (0 Codes) -> eigener CPU-Server
        self.lm_base = (lm_url or base_url).rstrip("/")
        self.poll = poll_interval
        self.timeout = timeout_minutes * 60
        # Liegen mehrere Sprachmodelle im Ordner, nähme der Server sonst das erste (alphabetisch)
        self.lm_model = lm_model
        self._job: tuple[str, str] | None = None   # (Server, Job-ID) des gerade laufenden Auftrags

    async def cancel(self) -> None:
        """Laufenden Auftrag beim Modellserver abbrechen, damit er nicht weiterrechnet."""
        if not self._job:
            return
        base, job_id = self._job
        try:
            async with httpx.AsyncClient(timeout=5) as c:
                await c.post(f"{base}/job", params={"id": job_id, "cancel": 1})
        except httpx.HTTPError:
            pass

    async def health(self) -> dict:
        async with httpx.AsyncClient(timeout=5) as c:
            r = await c.get(f"{self.base}/health")
            r.raise_for_status()
            if self.lm_base != self.base:
                try:
                    (await c.get(f"{self.lm_base}/health")).raise_for_status()
                except Exception as e:
                    raise EngineError(f"LM-Server {self.lm_base} nicht erreichbar") from e
            info = {"ok": True, "health": r.json()}
            try:
                p = await c.get(f"{self.base}/props")
                if p.status_code == 200:
                    info["props"] = p.json()
            except Exception:
                pass
            return info

    async def _wait(self, c: httpx.AsyncClient, base: str, job_id: str, label: str, start: float, progress: Progress) -> httpx.Response:
        self._job = (base, job_id)
        # start = Zeitpunkt des Gesamtauftrags (nicht dieser Phase) -> die Anzeige zählt einfach durch
        while True:
            r = await _get_retry(c, base, {"id": job_id})
            r.raise_for_status()
            status = str(r.json().get("status", "")).lower()
            if status == "done":
                res = await _get_retry(c, base, {"id": job_id, "result": 1})
                res.raise_for_status()
                return res
            if any(w in status for w in ("err", "fail", "cancel")):
                raise EngineError(f"{label} fehlgeschlagen: {r.text}")
            elapsed = int(time.monotonic() - start)
            if elapsed > self.timeout:
                await self.cancel()   # Modellserver nicht am alten Auftrag weiterrechnen lassen
                raise EngineError(f"{label}: Zeitüberschreitung nach {elapsed}s")
            await progress(_fmt_time(elapsed))
            await asyncio.sleep(self.poll)

    async def generate(self, params: dict, progress: Progress) -> tuple[bytes, str, dict]:
        req = {k: v for k, v in params.items() if k in ACE_FIELDS and v not in (None, "")}
        req.setdefault("use_cot_caption", False)  # Prompt wörtlich übernehmen
        req["output_format"] = "wav16"            # 48 kHz / 16 Bit, verlustfrei
        req["synth_model"] = self.dit_model
        if self.lm_model:
            req["lm_model"] = self.lm_model
        start = time.monotonic()   # zählt über beide Phasen (Sprachmodell + Synthese) durch
        async with httpx.AsyncClient(timeout=60) as c:
            # 1) Sprachmodell: Songstruktur + Audio-Codes
            r = await c.post(f"{self.lm_base}/lm", json=req)
            if r.status_code >= 400:
                raise EngineError(f"/lm: HTTP {r.status_code} {r.text[:300]}")
            res = await self._wait(c, self.lm_base, str(r.json()["id"]), "Sprachmodell", start, progress)
            enriched = res.json()
            if isinstance(enriched, list):
                enriched = enriched[0]
            if not enriched.get("audio_codes"):
                raise EngineError("Sprachmodell hat keine Audio-Codes geliefert (LM auf Metal? LM-Server auf CPU nutzen)")
            enriched["output_format"] = req["output_format"]
            enriched["synth_model"] = req["synth_model"]

            # 2) Synthese
            r = await c.post(f"{self.base}/synth", json=enriched)
            if r.status_code >= 400:
                raise EngineError(f"/synth: HTTP {r.status_code} {r.text[:300]}")
            res = await self._wait(c, self.base, str(r.json()["id"]), "Synthese", start, progress)
            audio, ext = _extract_audio(res)
        meta = {k: enriched.get(k) for k in ("bpm", "duration", "keyscale", "timesignature", "vocal_language", "lyrics", "caption")}
        return audio, ext, meta


    async def analyze(self, audio: bytes, filename: str) -> dict:
        """Song hochladen, Modell beschreibt Stil, Tempo, Tonart, Lyrics (/understand, läuft auf dem LM-Server)."""
        async with httpx.AsyncClient(timeout=60) as c:
            r = await c.post(f"{self.lm_base}/understand", files={"audio": (filename, audio)})
            if r.status_code >= 400:
                raise EngineError(f"/understand: HTTP {r.status_code} {r.text[:300]}")
            res = await self._wait(c, self.lm_base, str(r.json()["id"]), "Analyse", time.monotonic(), lambda _msg: asyncio.sleep(0))
            data = _extract_json(res)
            return data[0] if isinstance(data, list) else data


def _extract_json(res: httpx.Response):
    """/understand liefert wie /synth multipart/mixed: JSON-Teil (Metadaten) + Roh-Latents."""
    ctype = res.headers.get("content-type", "")
    if ctype.startswith("application/json"):
        return res.json()
    if "multipart" not in ctype:
        raise EngineError(f"Unerwartete Antwort von /understand: {ctype}")
    msg = email.message_from_bytes(
        f"Content-Type: {ctype}\r\n\r\n".encode() + res.content, policy=email.policy.HTTP
    )
    for part in msg.iter_parts():
        if part.get_content_type().startswith("application/json"):
            return json.loads(part.get_payload(decode=True))
    raise EngineError("Kein JSON-Teil in der /understand-Antwort gefunden")


def _extract_audio(res: httpx.Response) -> tuple[bytes, str]:
    ctype = res.headers.get("content-type", "")
    if ctype.startswith("audio/"):
        return res.content, _ext_for(ctype)
    if "multipart" not in ctype:
        raise EngineError(f"Unerwartete Antwort von /synth: {ctype}")
    msg = email.message_from_bytes(
        f"Content-Type: {ctype}\r\n\r\n".encode() + res.content, policy=email.policy.HTTP
    )
    for part in msg.iter_parts():
        pt = part.get_content_type()
        if pt.startswith("audio/"):
            return part.get_payload(decode=True), _ext_for(pt)
    raise EngineError("Kein Audio-Teil in der /synth-Antwort gefunden")


def _ext_for(ctype: str) -> str:
    return "mp3" if "mpeg" in ctype or "mp3" in ctype else "wav"


def make_engine(settings: dict):
    return AceStepCpp(settings["server_url"], settings.get("lm_url", ""), settings["poll_interval"],
                      settings["timeout_minutes"], settings.get("lm_model", ""), settings.get("dit", "standard"))
