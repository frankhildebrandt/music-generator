# Music Generator

Lokale Musikgenerierung als rahmenlose **Tauri-Desktop-App** für macOS, Windows und Linux. Rust verwaltet Modelle, Warteschlange und Bibliothek; [acestep.cpp](https://github.com/ServeurpersoCom/acestep.cpp) übernimmt die Musikberechnung.

**Herunterladen, entpacken, öffnen.** Keine Installer und keine Python-/Node-Installation beim Anwender. Der Startassistent lädt die benötigten Modelle und prüft die Hardware. Danach funktioniert die Musikgenerierung offline.

| Plattform | Bundle |
| --- | --- |
| macOS 26+, Apple Silicon | DMG und ZIP mit `Music Generator.app` |
| Windows 10/11, x64 | ZIP mit EXE, Engine und WebView2 |
| Linux, x64 | AppImage mit Vulkan/CPU-Engine |

Die ersten Bundles sind unsignierte Testausgaben. Betriebssystem-Warnungen sind möglich. Downloads werden unter [Releases](https://github.com/frankhildebrandt/music-generator/releases) veröffentlicht; lokal gebaute Pakete liegen in `release/`.

- Bestehende Bedienung, zwölf Sprachen, Bibliothek, Favoriten, Ordner und Varianten.
- Mikrofonideen und Audioanalyse, Waveform-Player und native WAV-/TXT-/ZIP-Exporte.
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
