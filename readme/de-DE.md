<div align="center">
  <img src="../icon.png" width="128" height="128" alt="Frame Icon" />
  <h1>Frame</h1>
</div>

<div align="center">

[English](../README.md) | [简体中文](./zh-CN.md) | [日本語](./ja-JP.md) | [한국어](./ko-KR.md) | [Español](./es-ES.md) | [Русский](./ru-RU.md) | [Français](./fr-FR.md) | [Deutsch](./de-DE.md) | [Italiano](./it-IT.md)

</div>

<div align="center">
	<img src="https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri" alt="Tauri" />
	<img src="https://img.shields.io/badge/Svelte-v5-red?style=flat-square&logo=svelte" alt="Svelte" />
	<img src="https://img.shields.io/badge/Rust-Edition_2024-black?style=flat-square&logo=rust" alt="Rust" />
	<img src="https://img.shields.io/badge/TypeScript-5.9.3-blue?style=flat-square&logo=typescript" alt="TypeScript" />
	<img src="https://img.shields.io/badge/Tailwind_CSS-v4-38bdf8?style=flat-square&logo=tailwindcss" alt="Tailwind" />
	<img src="https://img.shields.io/badge/license-GPL--3.0-green?style=flat-square" alt="License" />
</div>

**Frame** ist ein leistungsstarkes Medienkonvertierungs-Tool, das auf dem Tauri v2 Framework basiert. Es bietet eine native Oberfläche für FFmpeg-Operationen mit präziser Kontrolle über Video- und Audio-Transcodierungsparameter. Die Anwendung nutzt ein Rust-Backend für parallele Aufgabenverwaltung und Prozessausführung, kombiniert mit einem Svelte 5 Frontend für Konfiguration und Statusüberwachung.

<br />
<div align="center">
  <img src="../preview.png" alt="Frame Anwendungsvorschau" width="800" />
</div>
<br />

> [!WARNING]
> **Hinweis zur unsignierten Anwendung**
> Da die Anwendung derzeit nicht signiert ist, zeigt Ihr Betriebssystem Warnungen an:
>
> - **macOS:** Das System versieht die App mit einem Quarantäne-Attribut. Zum Ausführen entfernen Sie das Attribut manuell:
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:** SmartScreen kann den Start blockieren. Klicken Sie auf **„Weitere Informationen"** und dann **„Trotzdem ausführen"**.

## Funktionen

### Konvertierungskern

- **Container-Unterstützung:** `mp4`, `mkv`, `webm`, `mov`, `mp3`, `m4a`, `wav`, `flac`.
- **Video-Encoder:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (SVT-AV1)
  - **Hardware-Beschleunigung:** `h264_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA)
- **Audio-Encoder:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`
- **Bitratenkontrolle:** Konstante Qualität (CRF) oder Ziel-Bitrate (kbps)
- **Skalierung:** Bikubisch, Lanczos, Bilinear, Nächster Nachbar
- **Metadaten-Analyse:** Automatische Extraktion von Stream-Informationen via `ffprobe`

### Architektur und Workflow

- **Parallele Verarbeitung:** Asynchroner Task-Queue-Manager in Rust (`tokio::mpsc`), Begrenzung paralleler FFmpeg-Prozesse (Standard: 2)
- **Echtzeit-Telemetrie:** Parsing des FFmpeg `stderr`-Streams für präzise Fortschrittsverfolgung
- **Preset-Verwaltung:** Konfigurationspersistenz für wiederverwendbare Konvertierungsprofile

## Technologie-Stack

### Backend (Rust / Tauri)

- **Kern:** Tauri v2 (Rust Edition 2024)
- **Runtime:** `tokio` (Asynchrones I/O)
- **Serialisierung:** `serde`, `serde_json`
- **Prozessverwaltung:** `tauri-plugin-shell` für Sidecar-Ausführung (FFmpeg/FFprobe)
- **Systemintegration:** `tauri-plugin-dialog`, `tauri-plugin-fs`, `window-vibrancy`

### Frontend (SvelteKit)

- **Framework:** Svelte 5 (Runes API)
- **Build-System:** Vite
- **Styling:** Tailwind CSS v4, `clsx`, `tailwind-merge`
- **Zustandsverwaltung:** Svelte 5 `$state` / `$props`
- **Internationalisierung:** Mehrsprachige Oberfläche mit automatischer Systemspracherkennung
- **Typografie:** Geist Mono (eingebettet)

### Installation

#### Über Homebrew (macOS)

Der einfachste Weg, Frame auf macOS zu installieren und aktuell zu halten, ist über unseren Homebrew Tap:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Vorgefertigte Releases

Der einfachste Weg, Frame auszuführen, ist das Herunterladen eines vorgefertigten Pakets von [GitHub Releases](https://github.com/66HEX/frame/releases). Jedes Release enthält Builds für macOS (Intel/Apple Silicon), Windows und Linux (AppImage/Deb). Die Binärdateien sind nicht signiert, daher kann Ihr OS Warnungen anzeigen.

### Voraussetzungen

- Node.js Runtime (oder Bun)
- Rust Toolchain (`cargo`)
- **FFmpeg** und **FFprobe** Binärdateien im Verzeichnis `src-tauri/binaries/`
  - Namenskonvention: `ffmpeg-<target-triple>` (z.B. `ffmpeg-aarch64-apple-darwin`)

> Tipp: Führen Sie `bun run setup:binaries` aus, um automatisch die passenden Binärdateien herunterzuladen. Verwenden Sie `--force` zum Aktualisieren.

### Build-Anleitung

1.  **Abhängigkeiten installieren:**

    ```bash
    bun install
    ```

2.  **Entwicklungsserver starten:**

    ```bash
    bun run tauri dev
    ```

3.  **Für Produktion kompilieren:**
    ```bash
    bun run tauri build
    ```

## Verwendung

1.  **Eingabe:** Verwenden Sie den Systemdialog zur Dateiauswahl
2.  **Konfiguration:**
    - **Quelle:** Erkannte Datei-Metadaten anzeigen
    - **Ausgabe:** Container-Format und Ausgabedateiname wählen
    - **Video:** Codec, Bitrate/CRF, Auflösung und Framerate konfigurieren
    - **Audio:** Codec, Bitrate, Kanäle und spezifische Spuren wählen
    - **Voreinstellungen:** Wiederverwendbare Konvertierungsprofile speichern und laden
3.  **Ausführung:** Konvertierungsprozess über Rust-Backend starten
4.  **Überwachung:** Echtzeit-Logs und Fortschrittsanzeige in der UI

## Lizenz

GPLv3-Lizenz. Siehe [LICENSE](../LICENSE) für Details.
