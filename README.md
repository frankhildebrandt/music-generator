# Music Generator

Eigene Songs lokal erzeugen – ohne Limits, ohne Abo, ohne Cloud.
Web-Oberfläche für [ACE-Step 1.5](https://github.com/ace-step/ACE-Step-1.5) über
[acestep.cpp](https://github.com/ServeurpersoCom/acestep.cpp).

- **Mac** (Apple Silicon, Metal): diese Seite. Ausführliche Anleitung mit Bildern: **[Installationsanleitung.pdf](Installationsanleitung.pdf)**
- **Windows** (NVIDIA, CUDA/Vulkan): **[README-WINDOWS.md](README-WINDOWS.md)**
- **Linux** (NVIDIA/AMD, ohne Installer): Abschnitt [Linux](#linux) unten

## Installation

Voraussetzungen: Mac mit Apple Silicon (M1–M4), mindestens 8 GB Arbeitsspeicher, 7–11 GB frei.

```bash
git clone https://github.com/rabbitfiremediacreation/music-generator.git ~/MusicGenerator
open ~/MusicGenerator/Install.command
```

Der Installer

1. prüft Chip, Arbeitsspeicher und freien Platz,
2. schlägt das passende Sprachmodell vor (0.6B unter 12 GB, 1.7B bis 24 GB, 4B darüber),
3. installiert was fehlt: Apple-Entwicklerwerkzeuge, `uv`, `cmake` (ohne Homebrew, ohne Passwort),
4. baut acestep.cpp in der getesteten Version (`b7ba6d9`),
5. lädt nur die benötigten Modelle (fortsetzbar),
6. testet, ob das Sprachmodell auf dem Grafikchip richtig rechnet (sonst CPU, siehe unten),
7. richtet die Symbole für **Music Generator ON / OFF** ein und startet die App.

Er kann jederzeit erneut laufen; Erledigtes wird übersprungen.
Optionen: `--yes` (keine Rückfragen), `--lm 0.6B|1.7B|4B`, `--no-start`.

## Benutzen

- **Music Generator ON** startet Modellserver und App und öffnet http://localhost:8765
- **Music Generator OFF** beendet alles und schließt die Browser-Tabs

## Aktualisieren

In der App: Zahnrad › **Nach Updates suchen** › **Jetzt aktualisieren** (holt die neue Fassung von GitHub und startet die App neu; ändert sich acestep.cpp, weist sie auf den Installer hin). Oder von Hand:

```bash
cd ~/MusicGenerator && git pull
open Install.command
```

Songs und Einstellungen in `data/` bleiben erhalten.

## Linux

Die App selbst (Python, Oberfläche, Datenbank) ist plattformneutral und acestep.cpp baut auch unter Linux.
Einen Installer gibt es dafür nicht, der Weg von Hand ist aber kurz. **Ungetestet**, Rückmeldungen willkommen.

Voraussetzungen: git, cmake, ein C++-Compiler, [uv](https://docs.astral.sh/uv/), für NVIDIA das CUDA-Toolkit (AMD/Intel: Vulkan-SDK).

```bash
git clone https://github.com/rabbitfiremediacreation/music-generator.git ~/MusicGenerator
cd ~/MusicGenerator && uv sync
# Modellserver in der getesteten Version bauen (Hash steht in scripts/common.sh)
git clone --recurse-submodules https://github.com/ServeurpersoCom/acestep.cpp engine/acestep.cpp
(cd engine/acestep.cpp && git checkout b7ba6d9 && ./buildcuda.sh)     # oder ./buildvulkan.sh, ./buildcpu.sh
# Modelle laden: die drei festen plus ein Sprachmodell (Adresse und SHA-256 in scripts/common.sh)
(cd engine/acestep.cpp && ./models.sh --lm 1.7B --sft)
# Betriebsart: ein Server für alles
mkdir -p data && printf 'MODE=single\nLM_FILE=acestep-5Hz-lm-1.7B-Q8_0.gguf\n' > data/engine.conf
```

Starten (zwei Terminals oder mit `&` im Hintergrund), dann http://localhost:8765 öffnen:

```bash
engine/acestep.cpp/build/ace-server --host 127.0.0.1 --port 8085 --models engine/acestep.cpp/models --max-batch 1
uv run uvicorn app.main:app --host 127.0.0.1 --port 8765
```

Hinweise: `models.sh` lädt mehr Varianten als nötig, gebraucht werden `vae-BF16`, `Qwen3-Embedding-0.6B-Q8_0`, `acestep-v15-sft-Q8_0`
und das gewählte `acestep-5Hz-lm-*-Q8_0`. Die Knöpfe „Server stoppen" und der Neustart nach einem Update rufen die Mac-Skripte auf
und funktionieren unter Linux nicht; Server im Terminal beenden und neu starten. Songs liegen wie überall in `data/songs`.

## Aufbau

| Pfad | Inhalt |
|---|---|
| `Install.command` | Installer (Mac) |
| `Music Generator ON/OFF.command` | Start und Stopp (Mac) |
| `Install.ps1`, `Music Generator ON/OFF.ps1` + `.bat` | Installer, Start und Stopp (Windows) |
| `scripts/common.sh` | gemeinsame Pfade, Ports, acestep.cpp-Version, Modell-Prüfsummen (Mac und Windows) |
| `scripts/common.ps1` | Windows-Gegenstück, liest die Werte aus `common.sh` |
| `scripts/metal_selftest.py` | Grafikchip-Test des Installers |
| `app/` | FastAPI-Backend: Queue, Bibliothek, Anbindung an ace-server |
| `static/` | Oberfläche |
| `docs/anleitung/` | Quelle der Installationsanleitung. `capture.py` fotografiert die laufende App (`uv run --with websockets python docs/anleitung/capture.py`), `build_pdf.py` baut daraus das PDF (`uv run --with reportlab python docs/anleitung/build_pdf.py Installationsanleitung.pdf`) |
| `engine/` | acestep.cpp und Modelle (vom Installer, nicht im Repo) |
| `data/` | Songs, Datenbank, Logs, `engine.conf` (nicht im Repo) |

## Grafikchip und die zwei Modi

Auf dem Apple M4 liefert das Sprachmodell unter Metal in acestep.cpp `b7ba6d9` fehlerhafte Ergebnisse
(0 Audio-Codes). Der Installer erkennt das mit einem kurzen Probelauf und schreibt nach `data/engine.conf`:

- `MODE=split` – Sprachmodell auf der CPU (Port 8086), Klangerzeugung auf Metal (Port 8085)
- `MODE=single` – alles auf einem Metal-Server (Port 8085)

Nach einem Update von acestep.cpp (`ACESTEP_REV` in `scripts/common.sh`) testet der Installer neu.

## Lizenz der Modelle

ACE-Step 1.5 steht unter Apache 2.0. Die Modelle werden beim Installieren von
[Hugging Face](https://huggingface.co/Serveurperso/ACE-Step-1.5-GGUF) geladen und sind nicht Teil dieses Repositorys.
Vor kommerzieller Nutzung die Lizenz- und Trainingsdaten-Lage selbst prüfen.

## Lizenz

Der Code in diesem Repository steht unter der [MIT-Lizenz](LICENSE).
`static/vendor/wavesurfer.min.js` stammt von [wavesurfer.js](https://github.com/katspaugh/wavesurfer.js) (BSD-3-Clause).
