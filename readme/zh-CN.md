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

**Frame** 是基于 Tauri v2 框架构建的高性能媒体转换工具。它为 FFmpeg 操作提供原生界面，支持对视频和音频转码参数进行精细控制。应用采用 Rust 后端实现并发任务管理和进程执行，配合 Svelte 5 前端进行配置和状态监控。

<br />
<div align="center">
  <img src="../preview.png" alt="Frame 应用预览" width="800" />
</div>
<br />

> [!WARNING]
> **未签名应用提示**
> 由于应用目前未签名，操作系统会发出警告：
>
> - **macOS：** 系统会对应用及其附属二进制文件添加隔离属性。运行前需手动移除：
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows：** SmartScreen 可能阻止应用启动。点击 **"更多信息"** 然后 **"仍要运行"** 即可。

## 功能特性

### 媒体转换核心

- **容器支持：** `mp4`, `mkv`, `webm`, `mov`, `mp3`, `m4a`, `wav`, `flac`.
- **视频编码器：**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (SVT-AV1)
  - **硬件加速：** `h264_videotoolbox` (Apple Silicon)、`h264_nvenc` (NVIDIA)
- **音频编码器：** `aac`、`ac3` (杜比数字)、`libopus`、`mp3`
- **码率控制：** 固定质量因子 (CRF) 或目标码率 (kbps)
- **缩放算法：** 双三次、Lanczos、双线性、最近邻
- **元数据探测：** 通过 `ffprobe` 自动提取流信息（编码器、时长、码率、声道布局）

### 架构与工作流

- **并发处理：** Rust 实现的异步任务队列管理器（`tokio::mpsc`），限制并发 FFmpeg 进程数（默认：2）
- **实时遥测：** 解析 FFmpeg `stderr` 流，实现精确进度跟踪和日志输出
- **预设管理：** 配置持久化，支持可复用的转换配置

## 技术栈

### 后端 (Rust / Tauri)

- **核心：** Tauri v2 (Rust Edition 2024)
- **运行时：** `tokio` (异步 I/O)
- **序列化：** `serde`、`serde_json`
- **进程管理：** `tauri-plugin-shell` 用于 sidecar 执行 (FFmpeg/FFprobe)
- **系统集成：** `tauri-plugin-dialog`、`tauri-plugin-fs`、`window-vibrancy`

### 前端 (SvelteKit)

- **框架：** Svelte 5 (Runes API)
- **构建系统：** Vite
- **样式：** Tailwind CSS v4、`clsx`、`tailwind-merge`
- **状态管理：** Svelte 5 `$state` / `$props`
- **国际化：** 多语言界面，自动检测系统语言
- **字体：** Geist Mono (内嵌)

### 安装

#### 通过 Homebrew (macOS)

在 macOS 上安装和更新 Frame 最简单的方式是使用我们的 Homebrew Tap：

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### 使用预构建版本

运行 Frame 最简单的方式是从 [GitHub Releases](https://github.com/66HEX/frame/releases) 页面下载预构建包。每个版本都提供 macOS (Intel/Apple Silicon)、Windows 和 Linux (AppImage/Deb) 的构建。请注意，二进制文件尚未签名，操作系统可能会发出警告并需要手动批准。

### 前置要求

- Node.js 运行时（或 Bun）
- Rust 工具链 (`cargo`)
- **FFmpeg** 和 **FFprobe** 二进制文件必须放在 `src-tauri/binaries/` 目录下
  - 命名规范：`ffmpeg-<target-triple>`（如 `ffmpeg-aarch64-apple-darwin` 或 `ffmpeg-x86_64-pc-windows-msvc.exe`）

> 提示：运行 `bun run setup:binaries`（或 `npm run setup:binaries`）可自动下载适合你系统的二进制文件。使用 `--force` 可刷新已有下载。

### 构建说明

1.  **安装依赖：**

    ```bash
    bun install
    ```

2.  **启动开发服务器：**

    ```bash
    bun run tauri dev
    ```

3.  **编译生产版本：**
    ```bash
    bun run tauri build
    ```

## 使用方法

1.  **输入：** 使用系统对话框选择文件
2.  **配置：**
    - **源文件：** 查看检测到的文件元数据
    - **输出：** 选择容器格式和输出文件名
    - **视频：** 配置编码器、码率/CRF、分辨率和帧率
    - **音频：** 选择编码器、码率、声道和特定轨道
    - **预设：** 保存和加载可复用的转换配置
3.  **执行：** 通过 Rust 后端启动转换进程
4.  **监控：** 在界面中查看实时日志和进度百分比

## 许可证

GPLv3 许可证。详见 [LICENSE](../LICENSE)。
