<div align="center">
  <img src="./icon.png" width="128" height="128" alt="Frame Icon" />
  <h1>Frame</h1>
</div>

<div align="center">

[English](./README.md) | [简体中文](./readme/zh-CN.md) | [日本語](./readme/ja-JP.md) | [한국어](./readme/ko-KR.md) | [Español](./readme/es-ES.md) | [Русский](./readme/ru-RU.md) | [Français](./readme/fr-FR.md) | [Deutsch](./readme/de-DE.md) | [Italiano](./readme/it-IT.md)

</div>

<div align="center">
	<img src="https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri" alt="Tauri" />
	<img src="https://img.shields.io/badge/Svelte-v5-red?style=flat-square&logo=svelte" alt="Svelte" />
	<img src="https://img.shields.io/badge/Rust-Edition_2024-black?style=flat-square&logo=rust" alt="Rust" />
	<img src="https://img.shields.io/badge/TypeScript-5.9.3-blue?style=flat-square&logo=typescript" alt="TypeScript" />
	<img src="https://img.shields.io/badge/Tailwind_CSS-v4-38bdf8?style=flat-square&logo=tailwindcss" alt="Tailwind" />
	<img src="https://img.shields.io/badge/license-GPL--3.0-green?style=flat-square" alt="License" />
</div>

**Frame** is a high-performance media conversion utility built on the Tauri v2 framework. It provides a native interface for FFmpeg operations, allowing for granular control over video and audio transcoding parameters. The application leverages a Rust-based backend for concurrent task management and process execution, coupled with a Svelte 5 frontend for configuration and state monitoring.

<br />
<div align="center">
  <img src="./preview.png" alt="Frame Application Preview" width="800" />
</div>
<br />

> [!WARNING]  
> **Unsigned Application Notice**
> Since the application is currently unsigned, your operating system will flag it:
>
> - **macOS:** The system will flag the app and its sidecar binaries with a quarantine attribute. To run the app, remove the attribute manually:
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:** Windows SmartScreen may prevent the application from starting. Click **"More info"** and then **"Run anyway"** to proceed.

## Features

### Media Conversion Core

- **Container Support:** `mp4`, `mkv`, `webm`, `mov`, `mp3`, `m4a`, `wav`, `flac`.
- **Video Encoders:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (Scalable Video Technology AV1)
  - **Hardware Acceleration:** `h264_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA).
- **Audio Encoders:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`, `alac` (Apple Lossless), `flac` (Free Lossless Audio Codec), `pcm_s16le` (WAV).
- **Bitrate Control:** Constant Rate Factor (CRF) or Target Bitrate (kbps).
- **Scaling:** Bicubic, Lanczos, Bilinear, Nearest Neighbor.
- **Metadata Probing:** Automated extraction of stream details (codec, duration, bitrate, channel layout) via `ffprobe`.

### Architecture & Workflow

- **Concurrent Processing:** Async task queue manager implemented in Rust (`tokio::mpsc`) limiting concurrent FFmpeg processes (default: 2).
- **Real-time Telemetry:** Stream parsing of FFmpeg `stderr` for accurate progress tracking and log output.
- **Preset Management:** Configuration persistence for reusable conversion profiles.

## Technical Stack

### Backend (Rust / Tauri)

- **Core:** Tauri v2 (Rust Edition 2024).
- **Runtime:** `tokio` (Async I/O).
- **Serialization:** `serde`, `serde_json`.
- **Process Management:** `tauri-plugin-shell` for sidecar execution (FFmpeg/FFprobe).
- **System Integration:** `tauri-plugin-dialog`, `tauri-plugin-fs`, `window-vibrancy`.

### Frontend (SvelteKit)

- **Framework:** Svelte 5 (Runes API).
- **Build System:** Vite.
- **Styling:** Tailwind CSS v4, `clsx`, `tailwind-merge`.
- **State Management:** Svelte 5 `$state` / `$props`.
- **Internationalization:** Multi-language interface with automatic system language detection.
- **Typography:** Geist Sans (embedded), Geist Mono (embedded).

## Installation

### Download Prebuilt Binaries

The easiest way to get started is to download the latest release for your platform (macOS, Windows, or Linux) directly from GitHub.

[**Download Latest Release**](https://github.com/66HEX/frame/releases)

> **Note:** Since the application is not yet code-signed, you may need to manually approve it in your system settings (see the warning at the top of this file).

### Homebrew (macOS)

For macOS users, you can install and update Frame easily using our custom Homebrew Tap:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Linux System Requirements

Even when using the **AppImage**, Frame relies on the system's **WebKitGTK** and **GStreamer** libraries for rendering the UI and handling media playback. If the application crashes upon adding a source or the video preview remains blank, you likely need to install the missing GStreamer plugins.

- **Ubuntu / Debian:**

  ```bash
  sudo apt update
  sudo apt install libwebkit2gtk-4.1-0 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav
  ```

- **Arch Linux:**

  ```bash
  sudo pacman -S --needed webkit2gtk-4.1 gst-plugins-base gst-plugins-good gst-libav
  ```

- **Fedora:**
  ```bash
  sudo dnf install webkit2gtk4.1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-libav
  ```

### Build from Source

If you prefer to build the application yourself or want to contribute, follow these steps.

**1. Prerequisites**

- **Rust:** [Install Rust](https://www.rust-lang.org/tools/install)
- **Bun (or Node.js):** [Install Bun](https://bun.sh/)
- **OS Dependencies:** Follow the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system.

**2. Setup Project**

Clone the repository and install dependencies:

```bash
git clone https://github.com/66HEX/frame.git
cd frame
bun install
```

**3. Setup Binaries**

Frame requires FFmpeg and FFprobe sidecar binaries. We provide a script to fetch the correct version for your platform automatically:

```bash
bun run setup:ffmpeg
```

**4. Build or Run**

- **Development:**

  ```bash
  bun tauri dev
  ```

- **Production Build:**
  ```bash
  bun tauri build
  ```

## Usage

1.  **Input:** Use the system dialog to select files.
2.  **Configuration:**
    - **Source:** View detected file metadata.
    - **Output:** Select container format and output filename.
    - **Video:** Configure codec, bitrate/CRF, resolution, and framerate.
    - **Audio:** Select codec, bitrate, channels, and specific tracks.
    - **Presets:** Save and load reusable conversion profiles.
3.  **Execution:** Initiates the conversion process via the Rust backend.
4.  **Monitoring:** View real-time logs and percentage counters in the UI.

## Star History

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
  <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
</picture>

## Acknowledgments & Third-Party Code

- **Real-ESRGAN**: Copyright (c) 2021, Xintao Wang. Licensed under [BSD 3-Clause](https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE).
- **FFmpeg**: Licensed under [GPLv3](https://www.ffmpeg.org/legal.html).

## License

GPLv3 License. See [LICENSE](LICENSE) for details.
