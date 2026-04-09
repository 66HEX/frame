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

**Frame**是一款基于Tauri v2框架的高性能媒体转换工具。 它为FFmpeg操作提供了一个本地接口，允许对视频、音频和图像转换参数进行细粒度控制。 该应用程序利用基于Rust的后端进行并发任务管理和进程执行，并结合Svelte 5前端进行配置和状态监控。

<br />
<div align="center">
  <img src="../preview.png" alt="Frame Application Preview" width="800" />
</div>
<br />

> [！警告]
> **未签署的申请通知**
> 由于应用程序目前未签名，操作系统会对其进行标记：
>
> - **要运行应用程序，请手动删除该属性：
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows：** Windows SmartScreen 可能会阻止应用程序启动。 单击**"更多信息 "**，然后**"无论如何运行 "**继续。

## GitHub 赞助商

如果 Frame 对您有帮助，请考虑支持 GitHub 赞助商项目：

[**赞助商框架**](https://github.com/sponsors/66HEX)

目前的筹资目标：

- **苹果开发者计划：** 99 美元/年，用于签署和公证 macOS 构建。
- **Microsoft 代码签名证书：** 估计为 300-700 美元/年，用于签署 Windows 构建并减少 SmartScreen 的摩擦。

赞助商的捐款首先用于支付这些解除合同的签署费用。

请参阅 [GitHub Sponsors](https://github.com/sponsors/66HEX)，了解完整的赞助详情、层级建议和启动清单。

## 特点

### 媒体转换核心

- **媒体类型：** 视频、音频、图像。
- **支持的输出格式：**
  - **视频：** `mp4`, `mkv`, `webm`, `mov`, `gif`
  - **音频：** `mp3`、`m4a`、`wav`、`flac
  - **Image:** `png`, `jpg`, `webp`, `bmp`, `tiff`
- **视频编码器：**
  - libx264（H.264 / AVC）
  - libx265（H.265 / HEVC）
  - vp9（谷歌 VP9）
  - `prores`（Apple ProRes）
  - libsvtav1（可扩展视频技术 AV1）
  - **硬件加速：** `h264_videotoolbox` (Apple Silicon)、`hevc_videotoolbox` (Apple Silicon)、`h264_nvenc` (NVIDIA)、`hevc_nvenc` (NVIDIA)、`av1_nvenc` (NVIDIA)。
- **图像编码器：** `png`, `mjpeg` (JPEG), `libwebp` (WebP), `bmp`, `tiff`。
- **音频编码器：** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`, `alac` (Apple Lossless), `flac` (Free Lossless Audio Codec), `pcm_s16le` (WAV).
- **比特率控制：** 恒定速率系数 (CRF) 或目标比特率 (kbps)。
- **缩放：** Bicubic、Lanczos、Bilinear、Nearest Neighbor。
- **元数据探测：** 通过 `ffprobe` 自动提取流细节（编解码器、持续时间、比特率、通道布局）。
- **人工智能升频：** 集成的 "Real-ESRGAN "可实现高质量视频和图像升频（x2、x4）。

### 架构和工作流程

- **并发处理：** 使用 Rust (`tokio::mpsc`) 实现的异步任务队列管理器可限制并发的 FFmpeg 进程（默认：2）。
- **实时遥测：** FFmpeg `stderr` 的流解析，用于准确的进度跟踪和日志输出。
- **预置管理：** 为可重复使用的转换配置文件提供配置持久性。

## 技术堆栈

### 后端（Rust/Tauri）

- **核心：** 金牛座 v2（2024 年生锈版）。
- **Runtime:** `tokio` (Async I/O)。
- **序列化：** `serde`、`serde_json`。
- **进程管理：** "tauri-plugin-shell "用于副卡执行（FFmpeg/FFprobe）。
- **系统集成：** `tauri-plugin-dialog`, `tauri-plugin-fs`.

### 前端（SvelteKit）

- **框架：** Svelte 5（符文 API）。
- **构建系统：** Vite。
- **样式：** 尾风 CSS v4、`clsx`、`tailwind-merge`。
- **状态管理：** Svelte 5 `$state` / `$props`。
- **国际化：** 多语言界面，可自动检测系统语言。
- **排版：** Loskeley Mono（嵌入式）。

## 安装

### 下载预制二进制文件

最简单的入门方法是直接从 GitHub 下载适用于你的平台（macOS、Windows 或 Linux）的最新版本。

[**下载最新版本**](https://github.com/66HEX/frame/releases)

> **注：** 由于应用程序尚未进行代码签名，您可能需要在系统设置中手动批准它（请参阅本文件顶部的警告）。

### WinGet（视窗）

Frame 可在官方 WinGet 代码库中的 `66HEX.Frame` 标识下使用。

```powershell
winget install --id 66HEX.Frame -e
```

要更新

```powershell
winget upgrade --id 66HEX.Frame -e
```

### 自制软件（macOS）

对于 macOS 用户，你可以使用我们定制的 Homebrew Tap 轻松安装和更新 Frame：

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Linux 系统要求

即使使用 **AppImage**，Frame 仍依赖系统的 **WebKitGTK** 和 **GStreamer** 库来渲染用户界面和处理媒体回放。 Linux 上的本地对话框还需要 **XDG Desktop Portal** 集成（加上桌面专用的后端）和 `zenity` 作为备用。 如果应用程序在添加源时崩溃、视频预览仍为空白或文件对话框无法正确打开/主题，请安装以下软件包。

- **Ubuntu / Debian:**

  ```bash
  sudo apt update
  sudo apt install libwebkit2gtk-4.1-0 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **Linux：**

  ```bash
  sudo pacman -S --needed webkit2gtk-4.1 gst-plugins-base gst-plugins-good gst-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **Fedora:**
  ```bash
  sudo dnf install webkit2gtk4.1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

> **KDE 用户：** 安装 `xdg-desktop-portal-kde` （而不是 `xdg-desktop-portal-gtk`）以获得 Plasma 原生主题对话框。

### 从源构建

如果您更愿意自己构建应用程序，或者希望贡献自己的一份力量，请按照以下步骤操作。

**1. 先决条件**

- **铁锈：** [安装铁锈](https://www.rust-lang.org/tools/install)
- **Bun（或 Node.js）：** [Install Bun](https://bun.sh/)
- **操作系统依赖性：** 遵循操作系统的 [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)。

**2.设置项目**

克隆版本库并安装依赖项：

```bash
git clone https://github.com/66HEX/frame.git
cd frame
bun install
```

**3. 设置二进制文件**

Frame 需要 FFmpeg/FFprobe 副卡二进制文件和 Real-ESRGAN 副卡资产来进行 AI 升频。 我们提供脚本来自动获取适合您平台的正确版本：

```bash
bun run setup:ffmpeg
bun run setup:upscaler
```

**4.建设或运行**

- **发展：**

  ```bash
  bun tauri dev
  ```

- **生产建设：**
  ```bash
  bun tauri build
  ```

## 使用方法

1.  **输入：** 使用系统对话框选择文件。
2.  **配置：**
    - **来源：** 查看检测到的文件元数据。
    - **输出：** 选择容器格式和输出文件名。
    - **视频：** 配置编解码器、比特率/CRF、分辨率和帧速率。
    - **图像：** 配置图像分辨率/缩放、像素格式和可选的 AI 放大。
    - **音频：** 选择编解码器、比特率、声道和特定音轨。
    - **预置：** 保存和加载可重复使用的转换配置文件。
3.  **执行：** 通过 Rust 后端启动转换过程。
4.  **监控：** 在用户界面上查看实时日志和百分比计数器。

## 明星历史

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
  <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
</picture>

## 鸣谢和第三方代码

- **Real-ESRGAN**: Copyright (c) 2021, Xintao Wang. Licensed under [BSD 3-Clause](https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE).
- **FFmpeg**: 许可证授权于 [GPLv3](https://www.ffmpeg.org/legal.html)。

## 许可证

GPLv3 许可。详情请参见 [LICENSE](../LICENSE)。
