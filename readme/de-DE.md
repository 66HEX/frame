<div align="center">
  <img src="../icon.png" width="256" height="256" alt="Frame Icon" />
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
	<a href="https://github.com/sponsors/66HEX">
		<img src="https://img.shields.io/badge/Sponsor-GitHub-pink?style=flat-square&logo=githubsponsors" alt="GitHub Sponsors" />
	</a>
</div>

**Frame** ist ein hochleistungsfähiges Medienkonvertierungsprogramm, das auf dem Tauri v2-Framework aufbaut. Es bietet eine native Schnittstelle für FFmpeg-Operationen, die eine granulare Steuerung von Video-, Audio- und Bildkonvertierungsparametern ermöglicht. Die Anwendung nutzt ein Rust-basiertes Backend für die gleichzeitige Task-Verwaltung und Prozessausführung, gekoppelt mit einem Svelte 5-Frontend für die Konfiguration und Zustandsüberwachung.

<br />
<div align="center">
  <img src="../preview.png" alt="Frame Application Preview" width="800" />
</div>
<br />

> [!WARNUNG]
> **Ungezeichnete Bewerbungsmitteilung**
> Da die Anwendung derzeit unsigniert ist, wird sie von Ihrem Betriebssystem gekennzeichnet:
>
> - **macOS:** Das System markiert die Anwendung und ihre Sidecar-Binärdateien mit einem Quarantäne-Attribut. Um die Anwendung auszuführen, entfernen Sie das Attribut manuell:
>   ``bash
>   xattr -dr com.apple.quarantine /Anwendungen/Frame.app
>   ```
> - **Windows:** Windows SmartScreen kann den Start der Anwendung verhindern. Klicken Sie auf **"Weitere Informationen "** und dann **"Trotzdem ausführen "**, um fortzufahren.

## GitHub-Sponsoren

Wenn Frame Ihnen hilft, sollten Sie das Projekt auf GitHub Sponsors unterstützen:

[**Sponsorenrahmen**](https://github.com/sponsors/66HEX)

Aktuelle Finanzierungsziele:

- **Apple Developer Program:** `$99/Jahr` zum Signieren und Beglaubigen von macOS-Builds.
- **Microsoft Code-Signatur-Zertifikat:** geschätzte $300-$700/Jahr, um Windows-Builds zu signieren und SmartScreen-Reibungen zu reduzieren.

Die Beiträge der Sponsoren werden zunächst für diese Freigabekosten verwendet.

Unter [GitHub-Sponsoren] (https://github.com/sponsors/66HEX) finden Sie alle Details zum Sponsoring, Vorschläge für die Stufen und eine Checkliste für den Start.

## Eigenschaften

### Medienkonvertierung Kern

- **Medientypen:** Video, Audio, Bild.
- **Unterstützte Ausgabeformate:**
  - **Video:** `mp4`, `mkv`, `webm`, `mov`, `gif`
  - **Audio:** `mp3`, `m4a`, `wav`, `flac`
  - **Bild:** `png`, `jpg`, `webp`, `bmp`, `tiff`
- **Video-Encoder:**
  - libx264" (H.264 / AVC)
  - libx265" (H.265 / HEVC)
  - vp9" (Google VP9)
  - prores" (Apple ProRes)
  - libsvtav1" (Skalierbare Video-Technologie AV1)
  - **Hardwarebeschleunigung:** `h264_videotoolbox` (Apple Silicon), `hevc_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA), `hevc_nvenc` (NVIDIA), `av1_nvenc` (NVIDIA).
- **Bildkodierer:** `png`, `mjpeg` (JPEG), `libwebp` (WebP), `bmp`, `tiff`.
- **Audio-Encoder:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`, `alac` (Apple Lossless), `flac` (Free Lossless Audio Codec), `pcm_s16le` (WAV).
- **Bitratensteuerung:** Constant Rate Factor (CRF) oder Ziel-Bitrate (kbps).
- **Skalierung:** Bikubisch, Lanczos, Bilinear, Nächster Nachbar.
- **Metadaten-Sondierung:** Automatisierte Extraktion von Stream-Details (Codec, Dauer, Bitrate, Kanal-Layout) über `ffprobe`.
- **AI Upscaling:** Integriertes `Real-ESRGAN` für hochwertiges Video- und Bild-Upscaling (x2, x4).

### Architektur und Arbeitsablauf

- **Gleichzeitige Verarbeitung:** Ein in Rust implementierter asynchroner Task-Queue-Manager (`tokio::mpsc`) begrenzt die Anzahl gleichzeitiger FFmpeg-Prozesse (Standard: 2).
- **Echtzeit-Telemetrie:** Stream-Parsing von FFmpeg `stderr` für genaue Fortschrittsverfolgung und Protokollausgabe.
- **Preset Management:** Konfigurationspersistenz für wiederverwendbare Konvertierungsprofile.

## Technischer Stapel

### Backend (Rust / Tauri)

- **Kern:** Tauri v2 (Rust Edition 2024).
- **Laufzeit:** `tokio` (Async I/O).
- **Serialisierung:** `serde`, `serde_json`.
- **Prozessverwaltung:** `tauri-plugin-shell` für die Ausführung von Sidecars (FFmpeg/FFprobe).
- **Systemintegration:** `tauri-plugin-dialog`, `tauri-plugin-fs`.

### Frontend (SvelteKit)

- **Framework:** Svelte 5 (Runen-API).
- **Build System:** Vite.
- **Styling:** Tailwind CSS v4, `clsx`, `tailwind-merge`.
- **State Management:** Svelte 5 `$state` / `$props`.
- **Internationalisierung:** Mehrsprachige Oberfläche mit automatischer Erkennung der Systemsprache.
- **Typografie:** Loskeley Mono (eingebettet).

## Einrichtung

### Vorgefertigte Binärdateien herunterladen

Der einfachste Weg, um loszulegen, ist, die neueste Version für Ihre Plattform (macOS, Windows oder Linux) direkt von GitHub herunterzuladen.

[**Download der neuesten Version**](https://github.com/66HEX/frame/releases)

> **Hinweis:** Da die Anwendung noch nicht code-signiert ist, müssen Sie sie möglicherweise manuell in Ihren Systemeinstellungen genehmigen (siehe die Warnung am Anfang dieser Datei).

### WinGet (Windows)

Frame ist im offiziellen WinGet-Repository unter dem Bezeichner `66HEX.Frame` verfügbar.

```powershell
winget install --id 66HEX.Frame -e
```

Zu aktualisieren:

```powershell
winget upgrade --id 66HEX.Frame -e
```

### Homebrew (macOS)

Nutzer von macOS können Frame ganz einfach mit unserem Homebrew Tap installieren und aktualisieren:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Linux-Systemanforderungen

Selbst wenn das **AppImage** verwendet wird, verlässt sich Frame auf die **WebKitGTK**- und **GStreamer**-Bibliotheken des Systems, um die Benutzeroberfläche zu rendern und die Medienwiedergabe zu handhaben. Native Dialoge unter Linux erfordern außerdem die **XDG Desktop Portal**-Integration (plus ein Desktop-spezifisches Backend) und `zenity` als Fallback. Wenn die Anwendung beim Hinzufügen einer Quelle abstürzt, die Videovorschau leer bleibt oder Dateidialoge nicht korrekt geöffnet/geöffnet werden können, installieren Sie die folgenden Pakete.

- **Ubuntu / Debian:**

  ```bash
  sudo apt update
  sudo apt install libwebkit2gtk-4.1-0 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **Arch Linux:**

  ```bash
  sudo pacman -S --needed webkit2gtk-4.1 gst-plugins-base gst-plugins-good gst-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **Fedora:**
  ```bash
  sudo dnf install webkit2gtk4.1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

> **KDE-Benutzer:** Installieren Sie `xdg-desktop-portal-kde` (anstelle von `xdg-desktop-portal-gtk`), um Plasma-native Dialoge zu erhalten.

### Aus der Quelle aufbauen

Wenn Sie es vorziehen, die Anwendung selbst zu erstellen oder einen Beitrag zu leisten, folgen Sie diesen Schritten.

**1. voraussetzungen**

- **Rust:** [Rust installieren](https://www.rust-lang.org/tools/install)
- **Bun (oder Node.js):** [Bun installieren](https://bun.sh/)
- **OS-Abhängigkeiten:** Beachten Sie die [Tauri-Voraussetzungen] (https://v2.tauri.app/start/prerequisites/) für Ihr Betriebssystem.

**2. einrichten Projekt**

Klonen Sie das Repository und installieren Sie die Abhängigkeiten:

```bash
git clone https://github.com/66HEX/frame.git
cd frame
bun install
```

**3. einrichten der Binärdateien

Frame benötigt FFmpeg/FFprobe-Sidecar-Binärdateien und Real-ESRGAN-Sidecar-Assets für die AI-Hochskalierung. Wir stellen Skripte zur Verfügung, die automatisch die richtigen Versionen für Ihre Plattform abrufen:

```bash
bun run setup:ffmpeg
bun run setup:upscaler
```

**4. bauen oder laufen**

- **Entwicklung:**

  ```bash
  bun tauri dev
  ```

- **Produktionsgebäude:**
  ```bash
  bun tauri build
  ```

## Verwendung

1.  **Eingabe:** Verwenden Sie den Systemdialog, um Dateien auszuwählen.
2.  **Konfiguration:**
    - **Quelle:** Ansicht der erkannten Datei-Metadaten.
    - **Ausgabe:** Wählen Sie das Containerformat und den Namen der Ausgabedatei.
    - **Video:** Konfigurieren Sie Codec, Bitrate/CRF, Auflösung und Bildwiederholrate.
    - **Bilder:** Konfigurieren Sie die Bildauflösung/Skalierung, das Pixelformat und die optionale AI-Hochskalierung.
    - **Audio:** Wählen Sie Codec, Bitrate, Kanäle und bestimmte Spuren.
    - **Voreinstellungen:** Speichern und laden Sie wiederverwendbare Konvertierungsprofile.
3.  **Ausführung:** Initiiert den Konvertierungsprozess über das Rust-Backend.
4.  **Überwachung:** Anzeige von Echtzeitprotokollen und Prozentzählern in der Benutzeroberfläche.

## Geschichte der Stars

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
  <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
</picture>

## Danksagungen & Code von Drittanbietern

- **Real-ESRGAN**: Copyright (c) 2021, Xintao Wang, lizenziert unter [BSD 3-Clause] (https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE).
- **FFmpeg**: Lizenziert unter [GPLv3] (https://www.ffmpeg.org/legal.html).

## Lizenz

GPLv3-Lizenz, siehe [LICENSE](../LICENSE) für Details.
