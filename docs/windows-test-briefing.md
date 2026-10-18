# Music Generator – Windows-Test: Briefing für die Claude-Code-Session auf dem PC

## Was das ist

„Music Generator" ist eine lokale Web-App (FastAPI + Vanilla-JS), die über acestep.cpp
(ACE-Step 1.5, GGUF-Modelle) Songs erzeugt. Die Mac-Fassung ist fertig und öffentlich:
https://github.com/rabbitfiremediacreation/music-generator

Die Windows-Fassung ist ein **ungetesteter Erstentwurf** im privaten Repo:
https://github.com/rabbitfiremediacreation/music-generator
(Branch `main`; auf dem Mac heißt derselbe Stand Branch `windows` im Hauptprojekt.)

Ziel dieser Session: Installation auf diesem Windows-PC (NVIDIA-Grafikkarte) zum Laufen bringen,
Fehler in den Skripten beheben, Ergebnis ins private Repo pushen.

## Einstieg

```powershell
git clone https://github.com/rabbitfiremediacreation/music-generator.git $HOME\MusicGenerator
cd $HOME\MusicGenerator
.\Windows-Install.bat        # oder: .\Windows-Install.ps1 -Yes  (ohne Rückfragen)
```

Danach: `Music Generator ON.bat` startet alles und öffnet http://localhost:8765,
`Music Generator OFF.bat` beendet alles.

## Aufbau (nur die für Windows relevanten Teile)

| Datei | Zweck |
|---|---|
| `Windows-Install.ps1` / `Windows-Install.bat` | Installer. Optionen: `-Yes`, `-LM 0.6B|1.7B|4B`, `-NoStart`, `-Build` |
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

## Ergebnis des Windows-Tests (10. Okt 2026, GTX 1060 6 GB, i7-7700, 64 GB RAM, Windows 10)

Installation, ON, OFF, Song-Erzeugung (30 s und 90 s), „Server stoppen“ und Update-Check laufen. Gefunden und behoben:

- **Python:** uv wählt das neueste Python (3.15.0), dafür hat pydantic-core keine Wheels, `uv sync` scheitert.
  Installer ruft jetzt `uv sync --python 3.12` auf.
- **CUDA fällt auf Pascal aus:** `ggml-cuda.dll` der fertigen Binaries ist mit CUDA 13 gebaut (importiert `cublas64_13.dll`,
  die nicht mitgeliefert wird) und enthält nur sm_86/89/120/121 (RTX 30xx bis 50xx). ggml lädt die DLL nicht und nimmt
  automatisch `Vulkan0` (GTX 1060 über den NVIDIA-Treiber, Vulkan 1.4). Kein Eingriff nötig, nur langsamer:
  LM 1.7B 41-54 tok/s, DiT 50 Schritte für 30 s in ~30 s, VAE 30 s in 13 s. Mit RTX 30xx+ sollte CUDA0 greifen (ungetestet).
- **Sprachmodell nach Grafikspeicher:** Nach RAM-Regel wäre 4B dran; es lädt 4,2 GB, dann scheitert der 1-GB-KV-Cache
  (`ErrorOutOfDeviceMemory`), der Server stirbt, Selbsttest meldet nur „Verbindung verweigert“ → Modus split.
  Installer deckelt jetzt nach VRAM (<6 GB 0.6B, <8 GB 1.7B, sonst 4B) und zeigt bei Absturz den Log-Schwanz. 1.7B: Modus single.
- **VAE-Decoder:** braucht ohne Kachelung 6,7 GB für 30 s Audio. Installer schreibt `VAE_CHUNK` (128 bei <8 GB, 256 bei <12,
  512 bei <16) nach `data/engine.conf`, ON gibt `--vae-chunk` an ace-server weiter. 90 s = 36 Kacheln, 33 s.
- **„Server stoppen“ tat nichts:** Python startete PowerShell mit `DETACHED_PROCESS` (keine Konsole). PowerShell 5.1
  startet dann, führt den Befehl aber nicht aus (Exit 0, keine Ausgabe, kein Fehler). Mit `CREATE_NO_WINDOW` (eigene
  unsichtbare Konsole) läuft es. Gefixt in `app/main.py` (Shutdown, getestet) und `app/updater.py` (`_restart`, gleicher
  Mechanismus, nicht getestet, weil die Installation nach dem Push auf dem neuesten Stand ist).
- `*.lnk` in `.gitignore` (der Installer legt sie im Programmordner an).
- Nicht geändert: Desktop-Verknüpfungen landen bei OneDrive-Umleitung unter `OneDrive\Desktop`, das ist korrekt so.
- Hinweis für Tests aus Claude Code heraus: Die Shell des Claude-Desktop-Tools läuft in einer MSIX-Sandbox, die `AppData`
  umleitet; uv-Installationen dort sind kaputt (Junction zeigt ins Leere). Installer und Skripte im Terminal-Panel laufen lassen.

## Bekannte Unsicherheiten (Stand vor dem Test, zur Einordnung)

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
- Commits mit `Co-Authored-By: Claude …`-Zeile, pushen nach `origin main` des öffentlichen Repos.
- Mac-Dateien (`*.command`, `scripts/symbole.sh`, `Mac-Install.command`) nicht anfassen; sie werden später zurückgemerged.
- Werte, die Mac und Windows teilen (Ports, acestep-Version, HF-Adresse), nur in `scripts/common.sh` ändern.
- Songs/Einstellungen liegen in `data/` und sind in `.gitignore`.

## Nachtrag vom Mac (10. Okt 2026, nach dem PC-Test)

Die Härtung aus dem öffentlichen Repo (Hinweise von Jürgen/schimmilab.de) ist in diesen Branch gemerged:
TrustedHostMiddleware (nur localhost/127.0.0.1), `/api/update/apply` mit Pflicht-Header `X-Lang`,
Modelle von fester Hugging-Face-Revision, `ACESTEP_REV` als voller Hash, `MODEL_FILES`-Tabelle
(Name:MB:SHA-256) in `scripts/common.sh`.

**Auf dem PC geprüft (10. Okt 2026, nach dem Nachtrag):** Das In-App-Update von 67e852b auf ce4ba5a lief komplett durch
(`git pull`, Windows-Neustart per `_restart` mit `CREATE_NO_WINDOW`, App wieder da). Fremder `Host`-Header bekommt 400.
`common.ps1` liest alle 6 Tabelleneinträge; die SHA-256 der bereits geladenen Modelle stimmen mit der Tabelle überein.
Schnelltest mit beiseitegelegter `vae-BF16.gguf`: Download, Prüfsumme, Umbenennen ok, Installer gibt jetzt „Prüfsumme ok" aus.
Nebenbefund behoben: `updater.py` dekodierte Git-Ausgabe mit cp1252, Commit-Texte hatten kaputte Umlaute; jetzt UTF-8.

**Vom Mac als ungetestet markiert:**
- `scripts/common.ps1` liest jetzt zusätzlich `MODEL_FILES` aus `common.sh` in `$MODEL_MB` / `$MODEL_SHA`.
- `Windows-Install.ps1` baut `$FILES` daraus und prüft nach jedem Download die SHA-256 per `Get-FileHash`
  (Download nach `<datei>.neu`, dann Hash, dann umbenennen). Vorhandene Dateien werden nicht erneut geprüft.
- Beim `-Build`-Pfad wird `--help` gegen die ersten 7 Zeichen des Hashes verglichen.
- Schnelltest ohne Neuinstallation: eine Modelldatei umbenennen, `.\Windows-Install.ps1 -Yes -NoStart` laufen lassen,
  Prüfsumme muss „ok" melden; danach die umbenannte Datei löschen.

## Nachtrag: Deinstallation

`Windows-Uninstall.bat` / `Windows-Uninstall.ps1` sind neu und wurden ohne PowerShell geschrieben. Bitte auf dem PC prüfen:
`.\Windows-Uninstall.ps1` (nur anschauen und mit Enter abbrechen), dann in einer Kopie des Ordners `.\Windows-Uninstall.ps1 -Yes`.
Erwartet: Server werden beendet, `engine`, `.venv` und `data` (außer `songs` und `music.db*`) verschwinden, die vier Verknüpfungen (Ordner und Desktop) auch.
Mac und Linux nutzen `scripts/uninstall.sh` und sind getestet.

### Ergebnis auf dem PC (13. Okt 2026, Stand 560338a, gleicher PC wie oben)

**Funktioniert.** Getestet wurde der Weg, den ein normaler Nutzer geht: alles entfernen, Songs mitlöschen,
Ordner löschen, frisch klonen, neu installieren. Kein Fehler, keine Änderung am Skript nötig.

Ausgangslage: laufende App (Web-App und Synthese-Server), 7 Songs (82 MB), Modelle inklusive XL-Klangmodell.

**1. Deinstallation** per `Windows-Uninstall.bat`, Antworten wie getippt: „Uninstall now?“ → `y`, „Keep songs and library?“ → `n`.
(Für den Test wurden die beiden Antworten per Pipe übergeben; `Read-Host` liest sie dann aus der Eingabe.)

| Schritt | Ergebnis |
|---|---|
| Übersicht vorab | zeigt `engine\` 14,5 GB, `.venv\` 16 MB, `data\` 85 MB, die Verknüpfungen und was bleibt (Ordner, Git, uv, eigener Songordner) |
| Server beenden | „Stopping server on port 8765 / 8085“, danach kein Port mehr offen, kein `ace-server` mehr |
| Songs-Frage | erscheint mit Größe (82 MB), Vorgabe „behalten“, mit `n` beantwortet |
| Entfernen | `engine\`, `.venv\`, `data\` komplett weg (inkl. `songs` und `music.db`) |
| Verknüpfungen | alle vier entfernt, auch die auf dem umgeleiteten Desktop (`OneDrive\Desktop`) |
| Ende | „Done. To finish, delete this folder: …“, dann „Press any key to close…“ (hält das Fenster beim Doppelklick offen, richtig so) |

Nachkontrolle: Im Programmordner blieben nur die Repo-Dateien (7,6 MB), rund 14,6 GB waren frei.
Git und uv waren weiter installiert, wie angekündigt. Danach den Ordner von Hand gelöscht, wie das Skript es sagt.

**2. Neuinstallation** nach `README-WINDOWS.md`: `git clone` aus dem öffentlichen Repo (ohne Anmeldung), dann `Windows-Install.bat -Yes -XL`.

- Binaries neu geladen, alle fünf Modelle geladen, jede Prüfsumme „ok“.
- Selbsttest lief neu (nach frischer Installation nicht mehr „already tested“): Planung ok, 50 Codes → Modus `single`.
- `data\engine.conf` neu geschrieben, `VAE_CHUNK=128`, ace-server startet mit `--vae-chunk 128`.
- Verknüpfungen in Programmordner und auf dem Desktop wieder da, App startete am Ende selbst.
- App: leere Bibliothek, beide Klangmodelle verfügbar, Update-Check ok.
- 30-Sekunden-Song nach der Neuinstallation in etwa 60 s fertig, Audio nicht stumm.

**Nicht getestet:**
- Abbrechen mit Enter bei „Uninstall now?“ (sollte nichts ändern).
- Songs behalten (Enter bzw. `-Yes`): dann müssten `data\songs` und `music.db*` bleiben, der Rest von `data\` weg sein.
- Ein eigener Songordner außerhalb des Programmordners (laut Skript wird er nicht angefasst).

## Aufgabe für die PC-Session: Prüfsummen der Programmdateien und VirusTotal

Anlass: Ein Nutzer meldet, dass Norton `ace-server.exe` als `IDP.Generic` blockt (Reputationserkennung, wahrscheinlich Fehlalarm).
Die fertigen Programmdateien kommen von `https://www.serveurperso.com/temp/acestep.cpp-win64/build/Release/` und werden bei uns noch nicht per Prüfsumme geprüft.
Bitte auf dem PC (Terminal-Panel, im Programmordner) ausführen und das Ergebnis hier eintragen:

```powershell
Get-FileHash engine\acestep.cpp\build\Release\*.exe, engine\acestep.cpp\build\Release\*.dll -Algorithm SHA256 | Format-Table Hash, @{n='Datei';e={Split-Path $_.Path -Leaf}} -AutoSize
Get-AuthenticodeSignature engine\acestep.cpp\build\Release\ace-server.exe | Format-List Status, SignerCertificate
(Get-Item engine\acestep.cpp\build\Release\ace-server.exe).LastWriteTime
```

Dann für **`ace-server.exe`** und **`ggml-cuda.dll`** den Hash bei VirusTotal nachschlagen (nur nachschlagen, nichts hochladen):
`https://www.virustotal.com/gui/file/<SHA256>`
Eintragen: Anzahl der Treffer, ob Norton (NortonLifeLock/Symantec) dabei ist, welche Namen die Erkennungen tragen, Signaturstatus.

Außerdem: Ist die Datei auf dem PC mit Microsoft Defender sauber? (`Get-MpThreatDetection`)
Ergebnis bitte als Tabelle ins Briefing und im Anschluss prüfen, ob sich daraus feste Prüfsummen für den Installer ableiten lassen (der Server auf `/temp/` kann Dateien jederzeit austauschen, feste Hashes würden dann den Installer brechen; Alternative wäre eine eigene, versionierte Veröffentlichung im Projekt).

### Ergebnis auf dem PC (13. Okt 2026, Stand 254679d)

Geladen am 13. Okt 2026, 14:57 (Neuinstallation), `ace-server.exe --help` meldet **`acestep.cpp 6013134 (2026-10-12)`**.

| Prüfung | `ace-server.exe` | `ggml-cuda.dll` |
|---|---|---|
| SHA-256 | `caed09ca26245dcad8f276d0f5b764f20edbe5d8391ab30d4af6be13301a8aab` | `2eeffd1561695d01a924782d89df7054ff6892b61a682456ab7e5a0664f0482c` |
| Größe | 1,4 MB | 121,9 MB |
| Signatur (`Get-AuthenticodeSignature`) | `NotSigned`, kein Zertifikat | `NotSigned`, kein Zertifikat |
| VirusTotal (nur nachgeschlagen) | **„Item not found“**: Datei dort unbekannt, keine Treffer, keine Norton-Erkennung | **„Item not found“** |
| Virenscanner auf dem PC | Avira Security aktiv, keine Blockade (Server läuft); Defender ist inaktiv, weil Avira übernimmt, `Get-MpThreatDetection` ohne Einträge | wie links |

„Item not found“ heißt: Diese Dateien hat noch nie jemand bei VirusTotal eingereicht. Für Norton passt das zu `IDP.Generic`
(neu, selten, unsigniert = keine Reputation). Hochgeladen wurde nichts. Ein Upload wäre öffentlich und ist eine Entscheidung für Lee.

Alle Prüfsummen (SHA-256) dieses Downloads:

| Datei | SHA-256 |
|---|---|
| ace-lm.exe | `2e3d5946bf933d48ad6adbbeb7a9cc253b6b3fca7fef8558864c29a9c77c3b6a` |
| ace-server.exe | `caed09ca26245dcad8f276d0f5b764f20edbe5d8391ab30d4af6be13301a8aab` |
| ace-synth.exe | `91ed3b4dd4eb08cbea8e6f6d178fdc4d8839a9e344bbba320e50ee534cf96158` |
| ace-understand.exe | `7ef505ee0c29353f6fb09bbfde830d3a55fa5b162ca31b9ba429e4f9a49701b4` |
| mp3-codec.exe | `f548b7dea8671d37cb626a2381aca331fc85c04ff55c905075f12bde12468fa4` |
| neural-codec.exe | `562784908b94887a6de780e23513c431b939aca3bb2a97f17ab6420e830b569f` |
| quantize.exe | `457caee99f55cfc703839b51b585d0d389946b5f91059cc7bf6795a3b348fde5` |
| test-lm-prompt.exe | `22bc3b579aa86f626554902f530a8b057531002ef2812c8abd8dee0aeea4854b` |
| test-model-store.exe | `47f0bf9374ff58f043896f237e840a3ed4d82d968bcbda385e3904a1a08c6f48` |
| test-philox.exe | `98adf8a7ede133f7db6e0f5223a9773e1ec7522f01f1eae15d602110c0d5c19d` |
| vulkan-shaders-gen.exe | `487731a5966a7c50fd33b9bbe4d40d269f819a6f95002edea6ead879ab810626` |
| ggml.dll | `bd1339284fd2f00d2a6bf46c460c82a767c2e68e3b231de3e654554d9ee844ad` |
| ggml-base.dll | `ae87be3ace6f19d9317fc9eeb49760f4a0970204e6444b438c87c54735372560` |
| ggml-cpu-alderlake.dll | `c4870b4e067d55968274182619a7ddf3d9467e2381727e84408bd89c022572ce` |
| ggml-cpu-cannonlake.dll | `b7ba23a603710db6eb3630bc21aa0d6efcce2c1bedc75e5d662dd50d0c2825cf` |
| ggml-cpu-cascadelake.dll | `68003e14d7a32a84259c575642f4c31d7d498d35e518cff70941fe739c422789` |
| ggml-cpu-haswell.dll | `b8425c82df457442425dc1e888272d8ae954f10a068a08ea4366a0b1f5673a29` |
| ggml-cpu-icelake.dll | `6d6bcc4d2fe1234b20856a173925901a27552f8285c1b9f201e2a8c6afa5d00a` |
| ggml-cpu-sandybridge.dll | `c155878713ef0b03620207d8ea8144c96620bd9c0587ca59097b5a3f78ea806c` |
| ggml-cpu-skylakex.dll | `8f0e90d465d9bdf7fbeb87471de4ca4ebc397ece9204a2bc53c0f9b9853a9d1a` |
| ggml-cpu-sse42.dll | `13cb0565b3465888e67a3bcb22691a834fa949d80b4d4bd85e8a9d7a9a7333d4` |
| ggml-cpu-x64.dll | `9dc115cf281799778c7ffd0a5e641a5019542399a7f845511030334c28a2506f` |
| ggml-cuda.dll | `2eeffd1561695d01a924782d89df7054ff6892b61a682456ab7e5a0664f0482c` |
| ggml-vulkan.dll | `a01306174c500e3d6015332ebffe20c1549f35e4cf0a201b5844f8748018ccff` |

**Feste Prüfsummen im Installer? Nein, nicht gegen die `/temp/`-Adresse.** Der Server hat die Dateien nachweislich schon
ausgetauscht: Der Download vom 10. Okt meldete `d881ad2 (2026-10-07)`, der vom 13. Okt `6013134 (2026-10-12)`. Mit festen
Hashes wäre jede Neuinstallation zwischen diesen Tagen mit „Prüfsumme stimmt nicht“ abgebrochen. Außerdem laufen die
Binaries damit auf einer anderen acestep.cpp-Version als `ACESTEP_REV` (`b7ba6d9`), ohne dass wir es merken.

Empfehlung: einen getesteten Satz Binaries als eigene, versionierte Veröffentlichung im Projekt ablegen (GitHub Release,
eine ZIP-Datei) und deren SHA-256 wie bei den Modellen in `common.sh` eintragen. Dann sind Version und Prüfsumme fest,
der Installer bricht nicht durch fremde Updates, und Nutzer können den Hash nachschlagen. Diesen Satz einmal selbst bei
VirusTotal einreichen und bei Norton als Fehlalarm melden, dann bekommt genau diese Datei Reputation.
Bis dahin könnte der Installer den Hash nach dem Download wenigstens ausgeben und in `VERSION.txt` schreiben.

### Getestet nach dem Update auf 254679d (In-App-Update, kein Installer-Lauf nötig)

- **Player (Issue 9):** Song A spielt, dann B, A, B innerhalb von 150 ms geklickt. Der alte Song war nach 50 ms stumm,
  danach spielte nur B, ein einziges Audio-Element. Ok.
- **Tastatur und Beschriftungen (Issue 8):** Titel, Prompt und Stil sind per `aria-labelledby` beschriftet. Alle 29 Stil-Karten
  haben `role="button"` und `tabindex="0"`. Enter schaltet eine Karte an, die Leertaste wieder aus. Ok.
- **Lyrics einklappen = Instrumental:** Haken aus, Lyrics offen, Klick auf die Kopfzeile: zugeklappt, Haken an. Ok.
- **Zahlen in den Einstellungen:** `"abc"`, `0`, `-5` und `null` geben 422 mit verständlicher Meldung (DE und EN),
  `"2.5"` und `"45"` werden als Zahl gespeichert (Timeout als Ganzzahl). Werte danach zurückgesetzt. Ok.
- **Analyse per Drag and Drop:** Mit Stil „my own style“ einen Lo-Fi-Song abgelegt. Nach 18 s stand im Stil-Feld
  `my own style, lo-fi, hip hop, piano, drums, bass, vinyl crackle`: eigener Stil bleibt vorn, sechs Begriffe, Karten
  „hip hop“, „lo-fi“, „piano“ leuchten. Ok.

## Auftrag an die PC-Session: eigene, versionierte Veröffentlichung der Windows-Programmdateien

**Von Lee am 13. Okt 2026 beauftragt.** Hintergrund: Die Programmdateien kommen aus `/temp/` beim Autor von acestep.cpp, der Server tauscht sie
ohne Ankündigung aus (10. Okt `d881ad2`, 13. Okt `6013134`), sie sind unsigniert und bei VirusTotal unbekannt (Norton meldet `IDP.Generic`).
Ziel: ein einmal getesteter, fester Satz mit Prüfsumme, den der Installer von uns lädt.

**Schon erledigt (Mac, Commit nach `08774ac`):** `Windows-Install.ps1` gibt nach dem Download die Version und die SHA-256 von `ace-server.exe` aus und
schreibt Version plus alle Prüfsummen in `engine\acestep.cpp\build\Release\VERSION.txt` (erste Zeile `PREBUILT=...` bleibt, der Selbsttest liest sie).
**Bitte zuerst testen:** frische Installation (oder nur der Binaries-Schritt in einer Kopie), prüfen dass die Ausgabe stimmt, `VERSION.txt` 24 `SHA256`-Zeilen
plus `SOURCE=` und `SERVER=` hat und der Selbsttest weiter „already tested"/„trial run" wie gewohnt läuft.

**Dann die Veröffentlichung:**
1. Den **auf dem PC getesteten Satz** (die 24 Dateien aus `build\Release`, Stand `6013134`, Hashes siehe oben, unverändert) in eine ZIP packen:
   `acestep-cpp-win64-6013134.zip`. Hineinlegen: die Dateien (flach oder als `build/Release/…`, so wie der Installer sie entpacken soll),
   `SHA256SUMS.txt` (Hashes der Einzeldateien), `LICENSE-acestep.cpp.txt` und `LICENSE-ggml.txt` (beide MIT, aus `engine/acestep.cpp` bzw. `engine/acestep.cpp/ggml`)
   und `README.txt` mit Herkunft: „Unveränderte fertige Programmdateien von https://www.serveurperso.com/temp/acestep.cpp-win64/ , Version 6013134 (2026-10-12),
   Quellcode: https://github.com/ServeurpersoCom/acestep.cpp , weitergegeben unter MIT-Lizenz".
2. SHA-256 der ZIP berechnen.
3. GitHub-Release im öffentlichen Repo anlegen (`gh release create engine-win64-6013134 <zip> --title ... --notes ...`), Notes mit Herkunft, Version, ZIP-Hash und dem Hinweis,
   dass die Dateien unsigniert sind (Norton kann `IDP.Generic` melden, siehe README-WINDOWS).
   **Bevor du `gh release create` ausführst: kurz bei Lee bestätigen lassen** (öffentliche Veröffentlichung).
4. In `scripts/common.sh` eintragen: `ENGINE_WIN_URL` (Download-Adresse des Release-Assets), `ENGINE_WIN_SHA256` (Hash der ZIP), `ENGINE_WIN_REV="6013134"`.
   `scripts/common.ps1` liest die drei Werte mit (wie schon `HF_BASE` usw.).
5. `Windows-Install.ps1`: Standardweg wird: ZIP von `ENGINE_WIN_URL` laden, SHA-256 prüfen (bei Abweichung löschen und abbrechen, wie bei den Modellen),
   nach `engine\acestep.cpp\build\Release` entpacken, `VERSION.txt` wie oben schreiben. Der bisherige `/temp/`-Weg entfällt (oder nur mit eigener Option, die klar warnt).
   Der Installer soll dabei auch merken, wenn bereits Dateien einer anderen Version liegen, und sie ersetzen (VERSION.txt vergleichen).
6. Frisch klonen, `Windows-Install.bat -Yes`: Download der ZIP, Prüfsumme „ok", App startet, 30-Sekunden-Song. Auch den Abbruchfall testen: ZIP-Hash in `common.sh` absichtlich verfälschen, Installer muss
   mit Prüfsummenfehler abbrechen und die Datei löschen. Danach zurückändern.
7. README-WINDOWS (Abschnitt „Virenscanner schlägt Alarm") und Anleitungstexte anpassen: Herkunft jetzt „unveränderte Programmdateien des acestep.cpp-Autors in unserer Veröffentlichung, Prüfsumme geprüft"; Punkt „Für diese Dateien gibt es noch keine Prüfsumme" entfernen.
8. Ergebnis ins Briefing, Commit mit Co-Authored-By, Push.

**Nicht ohne Lee:** Einreichen der ZIP oder von `ace-server.exe` bei VirusTotal und bei Norton (öffentlich bzw. an Dritte). Das macht Lee selbst oder gibt es der Session ausdrücklich frei.
Mac und Linux sind von all dem nicht betroffen.

### Stand auf dem PC (14. Okt 2026)

**Vorab-Test (Commit c18ea0e, `/temp/`-Weg):** Binaries-Ordner beiseitegelegt, `Windows-Install.bat -Yes -NoStart`. Ausgabe
`done: acestep.cpp 6013134 (2026-10-12)` und der erwartete Hash von `ace-server.exe`. `VERSION.txt`: erste Zeile `PREBUILT=…`,
dann `SOURCE=`, `SERVER=` und 24 `SHA256`-Zeilen, alle gleich mit den Dateien und dem Stand vom 13. Okt. Selbsttest lief wegen
der neuen Kennung frisch („trial run“, Modus `single`), ein zweiter Lauf meldete „already tested“. Ok.
Abhängigkeiten nach dem In-App-Update: FastAPI 0.143, Starlette 1.8, Pydantic 2.14 auf Python 3.12. Host-Schutz (400),
Pflicht-Header (422), Zahlenprüfung, Startseite und ein neuer Song liefen fehlerfrei.

**Schritte 1 und 2 erledigt:** `acestep-cpp-win64-6013134.zip`, 126,4 MB, 28 Dateien flach (24 unveränderte Programmdateien
aus dem getesteten Satz, `SHA256SUMS.txt`, `LICENSE-acestep.cpp.txt`, `LICENSE-ggml.txt`, `README.txt` mit Herkunft).
Lizenzen aus genau den Commits geholt: acestep.cpp `6013134c136d9fb4e970cf690e76925be9553da0`, ggml (Submodul, Fork
`ServeurpersoCom/ggml`) `2eddaf94a8cdbbf1c30017d28313cd661a9da543`, beide MIT.
ZIP-SHA-256: `647f4ffd1cb2f2e20ac3d54e92a45ed83d3cf9278e55e8b4da1c788fbfb74ac2`.
Gegenprobe: entpackt, alle 24 Hashes stimmen, `ace-server.exe --help` meldet `6013134`.

**Schritt 3 erledigt (von Lee am 14. Okt freigegeben):** `gh` per winget installiert, Lee hat sich angemeldet. Release
[`engine-win64-6013134`](https://github.com/rabbitfiremediacreation/music-generator/releases/tag/engine-win64-6013134)
„acestep.cpp program files for Windows (6013134)“, nicht als „Latest“ markiert, Asset `acestep-cpp-win64-6013134.zip`.
GitHub meldet für das Asset `sha256:647f4ffd…74ac2`; ein Download über `ENGINE_WIN_URL` ergibt dieselbe Prüfsumme.

**Schritte 4, 5 und 7 umgesetzt:**
- `common.sh`: `ENGINE_WIN_REV`, `ENGINE_WIN_URL` (Release-Asset), `ENGINE_WIN_SHA256`. `common.ps1` liest sie, `PREBUILT_URL` entfällt.
- `Windows-Install.ps1`: ZIP laden, SHA-256 prüfen (Abweichung: löschen, abbrechen), neben `build\Release` in `Release.neu` entpacken und
  prüfen, laufende Modellserver beenden, alte `.exe`/`.dll`/`VERSION.txt` entfernen, neue Dateien hinüberschieben. Die alte
  Installation bleibt also bis zuletzt intakt. `VERSION.txt` bekommt zusätzlich `ENGINE_REV=` und `ZIP_SHA256=`.
  „present“ nur, wenn `ENGINE_REV` passt (oder bei `-Build`-Installationen ohne `VERSION.txt` mit `.git`); sonst wird ersetzt.
  Der `/temp/`-Weg ist entfernt.
- README-WINDOWS (Herkunft, Release-Link, 130 MB, Hinweis „noch keine Prüfsumme“ entfernt), `docs/anleitung/texts.py` 180 → 130 MB
  in allen drei Sprachen. **Die PDFs sind noch nicht neu gebaut** (auf dem Mac `build_pdf.py`).

**Getestet gegen die lokale ZIP** (URL vorübergehend auf `file:///…` gestellt, danach zurück):

| Test | Ergebnis |
|---|---|
| Upgrade einer Installation vom `/temp/`-Weg (ohne `ENGINE_REV`), Synthese-Server läuft | „replacing installed version (unknown) with 6013134“, Prüfsumme ok, Server beendet, Dateien ersetzt, `VERSION.txt` vollständig, keine Reste, Selbsttest neu → `single` |
| Zweiter Lauf | „present … (version 6013134)“, kein Download, „already tested“ |
| Falscher ZIP-Hash in `common.sh`, installierte Version als `alt` markiert | Abbruch „Checksum … does not match“, ZIP gelöscht, kein `Release.neu`, installierte Programmdateien unverändert |
| Song mit den Dateien aus der ZIP | 30 s in ca. 60 s, Audio ok, Backend `Vulkan0` |

Zwei Fehler im ersten Entwurf beim Testen gefunden und behoben: `Expand-Archive` (PowerShell 5.1) nimmt nur Dateien mit
Endung `.zip` (war `.zip.neu`, und die alten Dateien waren da schon gelöscht → jetzt erst entpacken, dann tauschen);
`@(Get-Content)` nötig, sonst liefert `-match` bei einzeiliger `VERSION.txt` „False“ statt der Zeile.

**Schritt 6 erledigt (gegen das echte Release):** Den lokalen Stand frisch in einen Testordner geklont, nur die Modelle
hineinkopiert (die Programmdateien nicht), `Windows-Install.bat -Yes`: Download vom Release, „checksum ok“, entpackt, `VERSION.txt` mit
`ENGINE_REV=6013134`, `SOURCE=` auf das Release, 24 `SHA256`-Zeilen, keine Reste, Selbsttest → `single`, App startete selbst,
30-Sekunden-Song in ca. 60 s mit Audio. Danach die Hauptinstallation per `ENGINE_REV=alt` gezwungen, vom Release zu ersetzen:
ok. Testordner gelöscht, Desktop-Verknüpfungen zeigen wieder auf die Hauptinstallation.
Nachbesserung dabei: `Expand-Archive` blendet in PowerShell 5.1 einen eigenen Fortschrittsbalken ein, der die Ausgabe
überdeckt und das Entpacken bremst. Jetzt `$ProgressPreference = "SilentlyContinue"` nur ums Entpacken, Ausgabe „unpacking ...“.

**Noch offen:** PDFs auf dem Mac neu bauen (`texts.py` sagt jetzt 130 MB). VirusTotal und Norton weiter nur durch Lee;
Kandidat zum Einreichen ist genau diese ZIP bzw. `ace-server.exe` (`caed09ca…c8aab`).
Versionswechsel später: neue ZIP bauen, neues Release `engine-win64-<rev>`, dann die drei `ENGINE_WIN_*` in `common.sh`
ändern. Bestehende Installationen ersetzen die Programmdateien beim nächsten Installer-Lauf automatisch.
