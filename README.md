# Music Generator

Lokale Musikgenerierung als rahmenlose **Tauri-Desktop-App** für macOS, Windows und Linux. Rust verwaltet Modelle, Warteschlange und Bibliothek; [acestep.cpp](https://github.com/ServeurpersoCom/acestep.cpp) übernimmt die Musikberechnung.

**Herunterladen, entpacken, öffnen.** Keine Installer und keine Python-/Node-Installation beim Anwender. Der Startassistent lädt die benötigten Modelle und prüft die Hardware. Danach funktioniert die Musikgenerierung offline.

| Plattform | Bundle |
| --- | --- |
| macOS 26+, Apple Silicon | DMG und ZIP mit `Music Generator.app` |
| macOS 13.3+, Intel x64 (experimentell) | Eigenes DMG und ZIP, ausschließlich CPU |
| Windows 10/11, x64 | ZIP mit EXE, Engine und WebView2 |
| Linux, x64 | AppImage mit Vulkan/CPU-Engine |

Intel-Macs verwenden das separate `macos-x64`-Bundle. Sprachmodell und Klangerzeugung rechnen auf der CPU; ein Song kann viele Minuten dauern. Mit 10 Sekunden, dem 0.6B-Sprachmodell und dem Standard-Klangmodell beginnen. Der Assistent prüft den Engine-Start und überspringt den GPU-Test. Native Builds werden auf macOS 15 geprüft; echte Generierung auf weiteren Intel-Macs und der Mindestversion bleibt manuell abzunehmen. Die Intel-/CPU-Hinweise des ursprünglichen Installers bleiben unter [docs/legacy/](docs/legacy/NOTICE.md) historische Dokumentation.

Die ersten Bundles sind unsignierte Testausgaben. Betriebssystem-Warnungen sind möglich. Downloads werden unter [Releases](https://github.com/frankhildebrandt/music-generator/releases) veröffentlicht; lokal gebaute Pakete liegen in `release/`.

- Bestehende Bedienung, zwölf Sprachen, Bibliothek, Favoriten, Ordner und Varianten.
- Easy/Pro-Umschalter: Projekte mit Mehrspur-Timeline, Mixer, Takes und gezielter Spur-/Bereichsgenerierung; [Pro-Workflow und Grenzen](docs/pro-mode.md).
- Mikrofonideen und Audioanalyse, Waveform-Player und native WAV-/TXT-/ZIP-Exporte.
- Wahlweise MP3-Export mit 192 kbit/s für Einzeldateien und ZIPs; die Bibliothek behält das Original-WAV.
- Songlänge von 10 Sekunden bis 10 Minuten; oberhalb von fünf Minuten Hinweise auf hohen Zeit- und Speicherbedarf. Lange Songs sind auf Intel-Macs und Rechnern mit 8 GB noch nicht praktisch geprüft.
- Sprachmodell wahlweise automatisch oder nur auf CPU; die Synthese kann weiterhin die Grafikkarte nutzen.
- Fortsetzbare Modelldownloads mit SHA-256-Prüfung; Sprachmodelle 0.6B, 1.7B und 4B, optional XL.
- Übernahme einer alten Installation; Originaldaten bleiben erhalten.
- Modelle und Songs außerhalb der App, mit wählbaren Speicherorten. Updates durch Austausch des Bundles.
- macOS App Sandbox mit gespeicherten Ordnerfreigaben und vererbter Sandbox für die Engine.

## Entwickeln und bauen

Node.js 24+, Rust und die nativen [Tauri-Bauvoraussetzungen](https://v2.tauri.app/start/prerequisites/) werden nur für die Entwicklung benötigt.

```sh
npm ci
npm run engine:prepare
npm run dev
```

```sh
npm run lint
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run bundle
```

Die plattformspezifischen Pipelines stehen in `.github/workflows/desktop.yml`. Build, Datenablage, Import, Signierung und manuelle Abnahme beschreibt [docs/desktop.md](docs/desktop.md).

## Aufbau

| Bereich | Inhalt |
| --- | --- |
| `frontend/` | Vite-Oberfläche, TypeScript-Tauri-Schnittstelle, Assistent und erhaltene Studio-Bedienung |
| `src-tauri/` | Rust-Backend, SQLite, Queue, Download- und Engine-Verwaltung |
| `models/` | Versionierte Modell-/Engine- und WebView2-Downloadmanifeste |
| `scripts/desktop/` | Vorbereitung der Engine und fertige Plattformpakete |
| `static/` | Styles, Bilder und vendorter Waveform-Player |
| `docs/legacy/` | Frühere Browser-/Installer-Dokumentation, gilt nicht für Desktop |

Der Python-Webserver und die Installations-/ON/OFF-Skripte wurden durch den Desktopbetrieb abgelöst. Die historischen PDF-Dokumentwerkzeuge sind keine Laufzeitabhängigkeit der App.

## Lizenz

Der Anwendungscode steht unter [MIT](LICENSE). Hinweise zu Engine, Modellen und weiteren Komponenten: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
