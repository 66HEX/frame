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
- **Typography:** Geist Mono (embedded).

### Installation

#### via Homebrew (macOS)

The easiest way to install and keep Frame updated on macOS is via our custom Homebrew Tap:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Use Prebuilt Releases

The easiest way to run Frame is to grab a prebuilt package from the [GitHub Releases](https://github.com/66HEX/frame/releases) page. Each release ships builds for macOS (Intel/Apple Silicon), Windows, and Linux (AppImage/Deb). Keep in mind the binaries aren’t code-signed yet, so your OS may warn you and require manual approval.

### Prerequisites

- Node.js runtime (or Bun).
- Rust toolchain (`cargo`).
- **FFmpeg** and **FFprobe** binaries must be present in the `src-tauri/binaries/` directory.
  - Naming convention: `ffmpeg-<target-triple>` (e.g., `ffmpeg-aarch64-apple-darwin` or `ffmpeg-x86_64-pc-windows-msvc.exe`).

> Tip: Run `bun run setup:binaries` (or `npm run setup:binaries`) to automatically download the correct binaries for your OS/architecture. Use `--force` to refresh existing downloads.

### Build Instructions

1.  **Install dependencies:**

    ```bash
    bun install
    ```

2.  **Start development server:**

    ```bash
    bun run tauri dev
    ```

3.  **Compile for production:**
    ```bash
    bun run tauri build
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

[![Star History Chart](https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&legend=top-left)](https://www.star-history.com/#66HEX/frame&type=timeline&legend=top-left)

## License

GPLv3 License. See [LICENSE](LICENSE) for details.
