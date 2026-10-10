# Music Generator für Windows (Testfassung)

Gleiche App wie auf dem Mac, Modellserver läuft über CUDA auf der NVIDIA-Grafikkarte.
Der Installer lädt die fertigen Windows-Binaries von acestep.cpp (kein Compiler nötig).

## Installation

Voraussetzungen: Windows 10/11 (64 Bit), NVIDIA-Grafikkarte mit aktuellem Treiber,
mindestens 8 GB Arbeitsspeicher, 7–11 GB frei.

1. Falls `git` fehlt: PowerShell öffnen und `winget install Git.Git` ausführen (oder der Installer macht es).
2. In der PowerShell:

```powershell
git clone https://github.com/rabbitfiremediacreation/music-generator-windows.git $HOME\MusicGenerator
```

3. Im Ordner `MusicGenerator` die Datei **Install.bat** doppelklicken.

Der Installer

1. prüft Grafikkarte, Arbeitsspeicher und freien Platz,
2. schlägt das passende Sprachmodell vor (0.6B unter 12 GB, 1.7B bis 24 GB, 4B darüber),
3. installiert was fehlt: `git`, `uv` (ohne Admin-Rechte),
4. lädt die fertigen acestep.cpp-Binaries (ca. 180 MB),
5. lädt nur die benötigten Modelle (fortsetzbar),
6. testet, ob das Sprachmodell auf der Grafikkarte richtig rechnet (sonst CPU),
7. legt die Verknüpfungen **Music Generator ON / OFF** im Programmordner und auf dem Desktop an und startet die App.

Er kann jederzeit erneut laufen; Erledigtes wird übersprungen.
Optionen (in der PowerShell): `.\Install.ps1 -Yes`, `-LM 0.6B|1.7B|4B`, `-NoStart`,
`-Build` (selbst kompilieren, braucht Visual Studio Build Tools und CUDA Toolkit).

## Benutzen

- **Music Generator ON** startet Modellserver und App und öffnet http://localhost:8765
- **Music Generator OFF** beendet alles (Browser-Tabs bleiben offen)

## Unterschiede zum Mac

| | Mac | Windows |
|---|---|---|
| Start/Stopp | `.command`-Dateien | `.bat` / `.ps1` plus Verknüpfungen mit Symbol |
| Modellserver | wird kompiliert (Metal) | fertige Binaries (CUDA, Vulkan, CPU; Backend wird zur Laufzeit gewählt) |
| Gemeinsame Werte | `scripts/common.sh` | `scripts/common.ps1` liest Ports und Adressen aus `common.sh` |
| Logs | `data/*.log` | `data/*.log` und `data/*.err.log` |

## Wenn etwas nicht geht

- **Modellserver startet nicht, Meldung wegen fehlender DLL:** NVIDIA-Treiber aktualisieren.
  Die Binaries bringen CUDA selbst mit, brauchen aber einen Treiber, der zur CUDA-Version passt.
- **Alles sehr langsam:** In `data/ace-server.err.log` nachsehen, welches Backend gewählt wurde
  (`[Load] ... backend: CUDA0` ist richtig, `CPU` falsch).
- **Update in der App** holt wie auf dem Mac die neue Fassung von GitHub und startet neu.
