# LoopForge 0.2.0

Instrument von **Alphatester x KRGSH** für VST3, CLAP und als Standalone-App. Die Oberfläche übernimmt das bereitgestellte LoopForge-Design mit gemeinsamer Sound-Idee, PLAY / GENERATE / EDIT, Stereo-Waveforms und durchgehend sichtbarem Keyboard. Die technischen Bundle-Namen und Plugin-IDs bleiben für vorhandene Projekte erhalten. Bank öffnen/speichern, Umbenennen, Audioimport, Modellprüfung, Skalierung und Host-Sync befinden sich im Bank-/Einstellungsmenü oben.

## Sound-Idee, Templates und Artwork

Die gemeinsame Sound-Idee besteht aus Name, Text und Revision. Sie ergänzt den Instrument-Prompt für jede neue Generierung. Der Auftrag und jede fertige Variante behalten einen unveränderlichen Stand dieser Idee; Änderungen wirken auf zukünftige Generierungen. „Full prompt“ zeigt den zusammengesetzten Text. Der Template-Browser bietet getrennte Sound-Ideen und Instrument-Prompts, Suche, Factory-Presets und eigene Templates. Eigene Templates werden im Benutzerprofil (`prompt-templates.json`) und in der aktuellen Bank gespeichert. Importierte Bank-Templates stehen im Browser zur Verfügung.

PLAY schaltet zwischen großer Stereo-Waveform mit Playhead und Coverart mit kompaktem Fortschrittsbalken um, während die Wiedergabe weiterläuft. PNG/JPEG-Artwork bis 4 MiB lässt sich importieren und wird in Bank und DAW-Zustand eingebettet. Ohne importiertes Artwork erscheint eine native grafische Bank-Coverfläche. Die vier Regler sind mit Level, Tone (Tiefpass), Tune und Pan vorbelegt. „Map“ erlaubt pro Slot die Zuordnung zu Pegel, Pan, Transposition, Filtern oder Loopgrenzen. Die bestehenden Host-Parameter-IDs bleiben unverändert.

## Soundgruppen und globale Quantisierung

Im rechten EDIT-Bereich Gruppen erstellen, benennen, durch Klick auf das Farbfeld einfärben und Slots hinzufügen oder entfernen. Eine Bank unterstützt bis zu 16 Gruppen. Gruppenzugehörigkeit erscheint als gemeinsamer Farbstreifen auf den Keyboard-Tasten. Die Mitgliederliste scrollt unabhängig und umfasst alle Seiten.

Innerhalb einer Gruppe spielt ein Mitglied; beim Wechsel übernimmt das Ziel die musikalische Position am tatsächlichen Wechselzeitpunkt. Unterschiedlich lange Loops wickeln diese Position in Takten/Beats auf ihre eigene Länge um. Mehrere Gruppen können parallel laufen. Gruppensounds verwenden Toggle; der erneute Aufruf eines laufenden Mitglieds stoppt es. Eine neu gewählte Variante ersetzt eine bereits vorgemerkte Variante. Der kurze Übergangs-Fade vermeidet Klicks.

GLOBAL bietet Aus, 1/16, 1/8, 1/4, 1/2, einen, zwei oder vier Takte. Negative interne Bankwerte stehen für Takt-Vielfache und folgen deshalb Taktartänderungen. Das Raster bestimmt Starts und Gruppenwechsel. Ein orangefarbener Status markiert vorgemerkte Slots. Die bisherige Spur läuft bis zur Grenze weiter. Der Ziel-Loop übernimmt die dann erreichte Position, nicht die Position beim Tastendruck. Host-Sync verwendet DAW-Tempo und -Position; die Standalone-App verwendet dieselbe Bedienung mit interner Clock. Das Schnitt-Snap im Editor ist davon unabhängig.

## Installation

Das Paket enthält die Standalone-App, beide Pluginformate und den Hintergrunddienst mit ACE-Step-Engine. Die Desktop-App muss nicht laufen. Vor dem Ersetzen der Plugins REAPER schließen.

- macOS: `MusicGeneratorLoops.vst3` nach `~/Library/Audio/Plug-Ins/VST3/`, `MusicGeneratorLoops.clap` nach `~/Library/Audio/Plug-Ins/CLAP/` kopieren. Ganze Bundles einschließlich Resources kopieren. Apple Silicon benötigt wie die App macOS 26; Intel ist auf macOS 13.3 ausgerichtet.
- Windows x64: VST3-Bundle nach `C:\Program Files\Common Files\VST3\`, CLAP-Datei zusammen mit `MusicGeneratorLoops.resources` nach `C:\Program Files\Common Files\CLAP\` kopieren. Die Resources neben der CLAP-Datei müssen erhalten bleiben. Ziel ist Windows 10 oder neuer.
- In REAPER gegebenenfalls unter Preferences → Plug-ins → VST bzw. LV2/CLAP erneut scannen. Ein Instrument auf einer Spur laden; für zusätzliche Stereo-Ausgänge über das FX-Routing Ausgänge auf weitere Spuren legen.

macOS-Pakete sind lokal ad-hoc signiert, nicht notarisiert. Für öffentliche Verteilung ist eine Developer-ID-Signierung mit Notarisierung erforderlich. SHA256.json prüft die entpackten Dateien, die ZIP hat eine eigene .sha256-Datei.

## Standalone ohne DAW

`MusicGeneratorLoops.app` (macOS) beziehungsweise `MusicGeneratorLoops.exe` (Windows) starten. Unter Windows muss `MusicGeneratorLoops.resources` daneben bleiben. Unter Preferences (Cmd+, / Ctrl+,) Audioausgang, Samplerate, Puffer und MIDI-Eingang einstellen. Die App benötigt keinen Audioeingang. Die Bildschirmtasten und der Play-Schalter funktionieren auch ohne MIDI-Keyboard.

GENERATE → Tempo / BPM setzt in der Standalone-App zugleich das Wiedergabe- und Exporttempo (4/4, Startwert 120 BPM). Alle Slots werden über den Stereoausgang abgehört; gespeicherte Buszuweisungen bleiben für den Austausch mit der DAW erhalten. Das Bankmenü mit Open bank / Save bank tauscht komplette `.mgloops`-Banks mit beiden Pluginformaten aus. Banks vor dem Beenden speichern; die Standalone-App stellt sie nicht automatisch beim nächsten Start wieder her. Modelle werden aus demselben App-Datenverzeichnis erkannt.

## Spielen

128 MIDI-Noten besitzen feste Slots, auf acht Seiten mit 16 Tasten. Jede Taste wählt und spielt ihren Loop. Shift-Klick wählt nur aus. Rechtsklick aktiviert den Slot als expliziten KI-Kontext. Auch schwarze Tasten können eigene Loops enthalten.

- Gate spielt, solange die Note gehalten wird. Toggle schaltet mit jedem Note-on um. One-shot spielt einen Zyklus. Wiederholte Gate-/One-shot-Noten starten neu.
- Globale Quantisierung steuert Starts und Gruppenwechsel. Choke, Retrigger und Ausgangsbus befinden sich in PLAY; Soundgruppen werden in EDIT verwaltet.
- Maximal 32 Stimmen; die älteste Stimme wird bei Überschreitung ausgeblendet. Jeder Slot hat eine Stimme und genau einen Ausgang: Master oder einer von 16 zusätzlichen Stereo-Bussen.
- Level, Tone (Tiefpass), Tune und Pan sind die vier Performance-Regler. EDIT enthält zusätzlich Hochpass und numerische Loopgrenzen. Alle acht Parameter pro Note besitzen feste Host-IDs und sind automatisierbar.
- Der Player übernimmt Tempo, Taktart und Position vom Host. Nach Stop können neue Noten mit dem letzten Tempo gespielt werden. Ohne Hostinformationen gelten 120 BPM und 4/4. Die Stretch-Latenz wird dem Host gemeldet.

## Generieren und Modelle übernehmen

Das Plugin verwendet automatisch die Modelle aus dem Datenverzeichnis der App: unter Windows `%LOCALAPPDATA%\de.alphatester.music-generator\models`, unter macOS aus dem App-Container beziehungsweise `~/Library/Application Support/de.alphatester.music-generator/models`. Ein in der App gespeicherter eigener Modellordner wird ebenfalls erkannt. Vorhandene Downloads werden gegenüber leeren Profilordnern bevorzugt; ein Dateiauswahldialog ist nicht erforderlich. „Check models“ prüft die Modelle und lädt fehlende Dateien direkt in den gemeinsam genutzten Modellordner. Alte manuelle Plugin-Ordnerangaben werden nicht mehr verwendet. Standard für neue Generierungen mit und ohne Kontext ist XL (`acestep-v15-xl-sft-Q8_0.gguf`); erforderlich sind XL, Qwen-Textencoder und VAE. Der fortsetzbare Downloader und die SHA256-Prüfung stammen aus demselben Rust-Kern wie die App. Dateisperren koordinieren parallele Downloads.

GENERATE ist zweigeteilt: links stehen die Einstellungen mit direkter Instrumentauswahl per Dropdown, rechts die erzeugten Varianten als Waveform-Karten. Beide Bereiche lassen sich mit Mausrad/Trackpad oder den sichtbaren Scrollleisten unabhängig scrollen; Generate und Keyboard bleiben erreichbar. Jeder Klick auf Generate legt sofort eine Karte pro Variante an. Modellladen und Kontextaufbereitung zeigen einen laufenden Status; danach werden echte DiT-Schritte und Prozentwerte angezeigt. Während Decoding, Loopaufbereitung und WAV-Rendering bleibt die Karte als in Arbeit sichtbar. Abbruch und Fehler bleiben an der jeweiligen Karte erkennbar. Die Waveform erscheint erst, wenn tatsächlich Audiodaten vorliegen.

Beim Abspielen zeigt ein latenzberücksichtigter Playhead die Position in den Ergebnis-Waveforms und im Editor; in PLAY stehen Stereo-Waveform und kompakte Fortschrittsanzeige zur Verfügung.

Fertige Karten bieten Play/Stop, Replace und WAV-Dragging. Die Waveform selbst kann auf eine Keyboard-Taste gezogen werden; eine normale Auswahl spielt sie vor. Replace übernimmt sie auf die ausgewählte Note. Das Ersetzen einer belegten Taste ist per Undo rückgängig zu machen. Die Ergebnisse der aktuellen Sitzung bleiben in der scrollbaren Liste (bis zu 128 Karten); übernommene Loops mit Save bank sichern. WAVs werden vor dem Drag im Hintergrund fertig gerendert und bleiben dauerhaft im Profil erhalten.

In GENERATE Instrument, Beschreibung, Tonart (z. B. `A minor` oder `C major`), BPM, Taktlänge, Seed und Varianten einstellen. Standard sind vier Takte bei Hosttempo. Für weitere Instrumente andere Slots per Rechtsklick als Kontext auswählen. Ein WAV kann zunächst in einen anderen Slot importiert und als Kontext markiert werden. Der Zielslot ist ausgeschlossen. Jeder Auftrag speichert seinen Kontext vor dem Einreihen unveränderlich.

Ohne Kontext verwendet die Engine Textgenerierung, mit Kontext `lego`. Die Engine erzeugt mindestens zehn Sekunden und zwei zusätzliche Takte Rohmaterial. Daraus wird ein taktgenauer Abschnitt ohne lange Randstille gewählt und auf die exakte musikalische Framezahl aufbereitet. Fehlt ein geeigneter vollständiger Abschnitt, bleibt der vorhandene Loop erhalten und die Generierung meldet einen Fehler. Die fertige Variante vorhören und mit „Replace“ oder einem Drag auf die gewünschte Taste übernehmen. Dur-/Mollwechsel erfordern eine neue Generierung; Tune transponiert Halbtonschritte unabhängig vom Tempo. Mehrere Plugininstanzen teilen eine serielle Warteschlange pro Benutzer. Ein Dienstfehler unterbricht geladene Loops nicht.

Die erste Generierung lädt große Modelle in den Speicher und kann entsprechend dauern. „Check models“ startet erforderliche Downloads nur auf ausdrückliche Bedienung. Ein Plugin-Scan lädt keine Modelle.

## App-Importe und Bearbeitung

Import audio oder Datei-Drop unterstützt WAV, MP3, Bibliotheks-ZIP und Pro-Stem-ZIP. Mehrere Dateien belegen aufeinanderfolgende Slots. Ein ZIP wird begrenzt und ohne Übernahme seiner internen Pfade entpackt. Pro-Projektmetadaten liefern Tempo und Tonart; gerenderte Stems starten mit neutralem Pegel und Pan. Bei einfachen Audiodateien BPM, Tonart und Taktlänge passend einstellen.

EDIT zeigt beide Stereokanäle mit Zoom, Taktmarkierungen und Auswahl. Ziehen setzt Grenzen mit einstellbarem Schnitt-Snap (Aus bis 1/32); Shift-Ziehen markiert eine Rauschstelle. Numerische Start-/Endwerte erlauben genaue Schnitte.

- Zero check sucht nahe Schnittpunkte anhand beider Stereokanäle. Die Auswahllänge bleibt erhalten; ein Nullübergang in beiden Kanälen ist nicht immer möglich.
- Trim / fade rendert die Auswahl auf die eingestellte Taktlänge. Die einstellbare End-/Anfangsüberblendung (Standard 5 ms) verändert die Zykluslänge nicht.
- Fit loop entfernt lange Randstille auf dem Taktraster. Falls kein vollständiger Abschnitt passt, wiederholt es eine kürzere vollständige Phrase (z. B. vier Takte zweimal für acht Takte). Die Taktlänge bleibt exakt, interne Pausen bleiben erhalten; Undo stellt den vorherigen Stand wieder her.
- Bearbeitung entfernt Gleichspannungsanteile. Normalize wendet nach Crossfade eine Peak-Normalisierung auf −1 dBFS an.
- De-hiss benötigt eine markierte Rauschstelle und verwendet offline spektrale Rauschminderung mit einstellbarer Reduktion. Hoch-/Tiefpass stehen für die weitere Klangbearbeitung bereit.
- A: original und B: edited dienen dem Vergleich. Original und Bearbeitungsrezept bleiben gespeichert; Undo/Redo stellt frühere Bearbeitungen wieder her.

## WAV und portable Projekte

„Render WAV“ erzeugt im Hintergrund einen vollständigen Zyklus bei Hosttempo und Host-Samplerate mit aktueller Transposition, Pegel, Pan, Filtern und Loopgrenzen. Danach „Drag WAV → REAPER“ in die Arrangementansicht ziehen.  Float32-Stereo-WAVs bleiben im Benutzerprofil unter `exports` erhalten, damit REAPER dauerhaft darauf verweisen kann. Sie werden beim Schließen des Plugins nicht gelöscht.

DAW-Zustände enthalten Originalaudio, bearbeitetes Audio, Slotparameter, Sound-Idee, Gruppen, Templates, Macro-Zuordnungen, Artwork und Generierungseinstellungen. Ein gespeichertes Projekt kann deshalb ohne die ursprünglichen Dateien oder Modelle spielen. Open bank / Save bank verwendet ein versioniertes `.mgloops`-Format für den Austausch zwischen Standalone, VST3 und CLAP. Das neue Bankformat Version 2 liest auch Banken der Version 1; ältere Pluginversionen können Version 2 nicht öffnen. Derzeit beträgt die Grenze 512 MiB pro eingebetteter Bank; entsprechend kurze Loops verwenden. Lange Banks können das Speichern des Projekts merklich vergrößern.

Dienstprofil: macOS `~/Library/Application Support/MusicGeneratorLoops`, Windows `%LOCALAPPDATA%\MusicGeneratorLoops`. Entwickler können `MUSIC_LOOPS_PROFILE` und `MUSIC_LOOPS_RESOURCES` für isolierte Tests setzen. Ergebnisse, Exporte und Modelle werden nicht automatisch gelöscht. Aufträge laufen nicht nach einem Dienstneustart weiter; sie müssen neu gestartet werden.

## Bauen und Prüfen

Node 24+, Rust stable, CMake 3.24+ und native C++-Werkzeuge installieren (Xcode Command Line Tools bzw. Visual Studio 2022 Desktop C++). `npm ci`, dann `npm run plugin:build`. Es werden fest versionierte iPlug2-, VST3-, CLAP- und Signalsmith-Abhängigkeiten geladen. Die bestehende Desktop-Engine wird verifiziert oder reproduzierbar vorbereitet. Artefakte erscheinen unter `release/`.

`npm run plugin:validate` baut den offiziellen VST3-Validator und prüft das Plugin. `npm run plugin:verify` prüft Paketdateien und Signaturen. `npm run test:plugin` prüft DSP und Bankportabilität einschließlich einer Prüfung auf Allokationen im Audioprozess. `cargo test --manifest-path crates/music-core/Cargo.toml` prüft gemeinsame Audioverarbeitung; `cargo test --manifest-path src-tauri/Cargo.toml` die bestehende App. `plugin/build-native/clap_host <Plugin-Binary>` prüft CLAP-Laden, MIDI-Offsets, Routing und Zustandswiederherstellung. Die CI baut auf nativen Runnern für alle drei Zielarchitekturen; ein CI-Build ersetzt die praktische DAW-Abnahme nicht.

Der Stand ist eine Entwicklungsversion. Durchgeführte und noch offene praktische Prüfungen stehen in [loop-plugin-verification.md](loop-plugin-verification.md). Windows und Intel-Mac gelten bis zur jeweiligen praktischen Abnahme nicht als verifiziert.
