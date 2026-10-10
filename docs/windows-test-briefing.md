# Music Generator – Windows-Test: Briefing für die Claude-Code-Session auf dem PC

## Was das ist

„Music Generator" ist eine lokale Web-App (FastAPI + Vanilla-JS), die über acestep.cpp
(ACE-Step 1.5, GGUF-Modelle) Songs erzeugt. Die Mac-Fassung ist fertig und öffentlich:
https://github.com/rabbitfiremediacreation/music-generator

Die Windows-Fassung ist ein **ungetesteter Erstentwurf** im privaten Repo:
https://github.com/rabbitfiremediacreation/music-generator-windows
(Branch `main`; auf dem Mac heißt derselbe Stand Branch `windows` im Hauptprojekt.)

Ziel dieser Session: Installation auf diesem Windows-PC (NVIDIA-Grafikkarte) zum Laufen bringen,
Fehler in den Skripten beheben, Ergebnis ins private Repo pushen.

## Einstieg

```powershell
git clone https://github.com/rabbitfiremediacreation/music-generator-windows.git $HOME\MusicGenerator
cd $HOME\MusicGenerator
.\Install.bat        # oder: .\Install.ps1 -Yes  (ohne Rückfragen)
```

Danach: `Music Generator ON.bat` startet alles und öffnet http://localhost:8765,
`Music Generator OFF.bat` beendet alles.

## Aufbau (nur die für Windows relevanten Teile)

| Datei | Zweck |
|---|---|
| `Install.ps1` / `Install.bat` | Installer. Optionen: `-Yes`, `-LM 0.6B|1.7B|4B`, `-NoStart`, `-Build` |
| `Music Generator ON.ps1` / `.bat` | startet ace-server (Port 8085, bei Modus split zusätzlich 8086 auf CPU) und uvicorn (Port 8765) |
| `Music Generator OFF.ps1` / `.bat` | beendet die Prozesse auf den drei Ports |
| `scripts/common.ps1` | Pfade, Port-Hilfsfunktionen, `Start-Hintergrund`. Liest PORT, SYNTH_PORT, LM_PORT, ACESTEP_REPO, ACESTEP_REV, HF_BASE **aus `scripts/common.sh`** (eine Quelle für Mac und Windows) |
| `scripts/metal_selftest.py` | Selbsttest per HTTP: läuft das Sprachmodell auf der GPU korrekt? Exit 0 = Modus `single`, sonst `split` (LM auf CPU) |
| `app/main.py` `/api/shutdown` | Knopf „Server stoppen" in der App, Windows-Zweig startet `Music Generator OFF.ps1` |
| `app/updater.py` `_restart` | Neustart nach Update, Windows-Zweig wartet per `Get-NetTCPConnection` auf freien Port |
| `app/db.py` | liest `data/engine.conf` (MODE, LM_FILE), setzt daraus `lm_url` |
| `assets/icon-on.ico`, `icon-off.ico` | Icons für die `.lnk`-Verknüpfungen |
| `README-WINDOWS.md` | Anleitung |

Vom Installer erzeugt (nicht im Repo): `engine/acestep.cpp/build/Release/*.exe|dll`,
`engine/acestep.cpp/models/*.gguf`, `data/engine.conf`, `data/*.log`, `data/*.err.log`, `.venv/`.

## Was der Installer tut

1. PC prüfen (GPU per `Win32_VideoController` und `nvidia-smi`, RAM, freier Platz).
2. Sprachmodell wählen (0.6B < 12 GB RAM, 1.7B < 24 GB, sonst 4B).
3. Werkzeuge: `git` (winget, falls fehlt), `uv` (astral.sh-Installer), `uv sync`.
4. Modellserver: **fertige Binaries** von https://www.serveurperso.com/temp/acestep.cpp-win64/build/Release/
   (Index wird per Regex nach `*.exe`/`*.dll` durchsucht, Download mit `curl.exe -C -`).
   Stand dort: 7. Okt 2026, entspricht acestep.cpp HEAD. Unsere Mac-Version ist `b7ba6d9` (24. Sep);
   dazwischen nur Build-System- und ggml-Änderungen, keine API-Änderung.
   Mit `-Build` stattdessen: Quellcode klonen, cmake mit `-DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES=native`
   (braucht VS Build Tools + CUDA Toolkit, der Installer nennt die winget-Befehle und bricht ab).
5. Modelle von HuggingFace (`HF_BASE` aus common.sh): vae-BF16, Qwen3-Embedding-0.6B-Q8_0,
   acestep-v15-sft-Q8_0, acestep-5Hz-lm-<SIZE>-Q8_0.
6. Selbsttest: ace-server auf Port 8099 starten, `metal_selftest.py` ausführen, Modus in `data/engine.conf` schreiben.
7. Verknüpfungen `.lnk` (powershell -File …) in Programmordner und auf Desktop, dann ON ausführen.

## Bekannte Unsicherheiten (hier zuerst nachsehen)

- **Alle `.ps1` sind ohne PowerShell geschrieben und nie ausgeführt worden.** Syntax- oder Laufzeitfehler sind wahrscheinlich.
  Dateien haben UTF-8-BOM (wegen Umlauten in PowerShell 5.1) und per `.gitattributes` CRLF.
- **Starten die fertigen Binaries ohne CUDA-Toolkit?** `ggml-cuda.dll` ist 122 MB (CUDA vermutlich statisch gelinkt),
  aber ob `cublas64_*.dll` o. ä. fehlt, ist unbekannt. Symptom: ace-server beendet sich sofort, Fehler in `data\selftest.err.log`
  bzw. `data\ace-server.err.log`. Lösung dann: fehlende DLLs ins `build\Release` legen oder NVIDIA-Treiber/CUDA-Runtime installieren,
  oder `-Build`.
- **Wählt ace-server CUDA?** In `data\ace-server.err.log` muss `[Load] ... backend: CUDA0` stehen, nicht `CPU`.
  Backend-Auswahl: automatisch „best", erzwingen per Umgebungsvariable `GGML_BACKEND=CUDA0|Vulkan0|CPU`
  (so startet ON im Modus split das Sprachmodell mit `GGML_BACKEND=CPU`).
- **Selbsttest:** Auf dem Mac M4 rechnet das 1.7B-Sprachmodell unter Metal falsch (0 Codes), daher der Test.
  Ob das unter CUDA auch passiert, ist unbekannt. Ergebnis steht in der Installer-Ausgabe („Modus single/split").
- **`Start-Process -RedirectStandardOutput`** braucht getrennte Dateien für stdout/stderr, daher `*.log` + `*.err.log`.
- **`uv` im PATH:** Installer ergänzt `%USERPROFILE%\.local\bin`. Die `.lnk`-Verknüpfungen starten eine frische PowerShell,
  common.ps1 setzt den PATH deshalb selbst.
- **Nicht umgesetzt:** OFF schließt keine Browser-Tabs (Mac macht das per AppleScript).
- **`app/updater.py` `REPO_URL`** zeigt im Windows-Branch auf das private Repo; der Update-Knopf in der App macht `git pull origin main`.

## Konventionen

- Antworten und Kommentare auf Deutsch, Lee duzen, kurz.
- Commits mit `Co-Authored-By: Claude …`-Zeile, pushen nach `origin main` des privaten Repos.
- Mac-Dateien (`*.command`, `scripts/symbole.sh`, `Install.command`) nicht anfassen; sie werden später zurückgemerged.
- Werte, die Mac und Windows teilen (Ports, acestep-Version, HF-Adresse), nur in `scripts/common.sh` ändern.
- Songs/Einstellungen liegen in `data/` und sind in `.gitignore`.
