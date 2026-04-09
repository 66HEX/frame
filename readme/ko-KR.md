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

**프레임**은 타우리 v2 프레임워크에 구축된 고성능 미디어 변환 유틸리티로, FFmpeg 작업을 위한 기본 인터페이스를 제공하여 비디오, 오디오 및 이미지 변환 매개변수를 세밀하게 제어할 수 있습니다. 이 애플리케이션은 동시 작업 관리 및 프로세스 실행을 위해 Rust 기반 백엔드와 구성 및 상태 모니터링을 위한 Svelte 5 프론트엔드를 활용합니다.

<br />
<div align="center">
  <img src="../preview.png" alt="Frame Application Preview" width="800" />
</div>
<br />

> [!경고]
> **서명되지 않은 신청 안내**
> 현재 애플리케이션이 서명되지 않았으므로 운영 체제에서 플래그를 지정합니다:
>
> - **macOS:** 시스템에서 앱과 해당 사이드카 바이너리에 격리 속성을 지정합니다. 앱을 실행하려면 수동으로 속성을 제거하세요:
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:** Windows 스마트스크린으로 인해 애플리케이션이 시작되지 않을 수 있습니다. 계속하려면 **"추가 정보"**를 클릭한 다음 **"어쨌든 실행"**을 클릭하세요.

## GitHub 스폰서

Frame이 도움이 된다면 GitHub 스폰서에서 프로젝트를 지원하는 것을 고려해 보세요:

[**스폰서 프레임**](https://github.com/sponsors/66HEX)

현재 펀딩 목표:

- **Apple 개발자 프로그램:** '$99/년'으로 macOS 빌드에 서명하고 공증할 수 있습니다.
- **Microsoft 코드 서명 인증서: Windows 빌드에 서명하고 스마트스크린 마찰을 줄이기 위해 '$300-$700/년'으로 추정됩니다.

스폰서 기부금은 이러한 릴리스 서명 비용에 먼저 사용됩니다.

전체 스폰서십 세부 정보, 티어 제안 및 출시 체크리스트는 [GitHub 스폰서](https://github.com/sponsors/66HEX)를 참조하세요.

## 특징

### 미디어 변환 코어

- **미디어 유형:** 동영상, 오디오, 이미지.
- **지원되는 출력 형식:**
  - **동영상:** `mp4`, `mkv`, `webm`, `mov`, `gif`
  - **오디오:** `mp3`, `m4a`, `wav`, `flac`
  - **이미지:** `png`, `jpg`, `webp`, `bmp`, `tiff`
- **비디오 인코더:**
  - `libx264`(H.264/AVC)
  - 'libx265'(H.265/HEVC)
  - `vp9`(Google VP9)
  - '프로레스'(Apple ProRes)
  - `libsvtav1`(확장 가능한 비디오 기술 AV1)
  - **하드웨어 가속:** `h264_videotoolbox` (Apple Silicon), `hevc_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA), `hevc_nvenc` (NVIDIA), `av1_nvenc` (NVIDIA).
- **이미지 인코더:** `png`, `mjpeg`(JPEG), `libwebp`(WebP), `bmp`, `tiff`.
- **오디오 인코더:** `aac`, `ac3`(Dolby Digital), `libopus`, `mp3`, `alac`(Apple 무손실), `flac`(무료 무손실 오디오 코덱), `pcm_s16le`(WAV).
- **비트레이트 제어:** 고정 비율 계수(CRF) 또는 목표 비트레이트(kbps).
- **스케일링:** 바이큐빅, 랑코스, 이중선형, 가장 가까운 이웃.
- **메타데이터 프로빙: ** '오프프로브'를 통해 스트림 세부 정보(코덱, 지속 시간, 비트 전송률, 채널 레이아웃)를 자동으로 추출합니다.
- **AI 업스케일링:** 고화질 동영상 및 이미지 업스케일링을 위한 통합 'Real-ESRGAN'(x2, x4)을 제공합니다.

### 아키텍처 및 워크플로

- **동시 처리:** Rust에서 구현된 비동기 작업 대기열 관리자(`tokio::mpsc`)가 동시 FFmpeg 프로세스를 제한합니다(기본값: 2).
- **실시간 텔레메트리: 정확한 진행 상황 추적 및 로그 출력을 위한 FFmpeg `stderr`의 스트림 파싱.
- **사전 설정 관리: ** 재사용 가능한 전환 프로필을 위한 구성 지속성.

## 기술 스택

### 백엔드(Rust/타우리)

- **핵심:** 타우리 v2(Rust Edition 2024).
- **런타임:** `tokio` (비동기 I/O).
- **직렬화:** `serde`, `serde_json`.
- **프로세스 관리:** 사이드카 실행을 위한 '타우리 플러그인 셸'(FFmpeg/FFprobe).
- **시스템 통합:** `타우리 플러그인-다이얼로그`, `타우리 플러그인-에프에스`.

### 프론트엔드(SvelteKit)

- **프레임워크:** Svelte 5(Runes API).
- **빌드 시스템:** Vite.
- **스타일링:** Tailwind CSS v4, `clsx`, `tailwind-merge`.
- **상태 관리:** Svelte 5 `$state` / `$props`.
- **국제화: ** 자동 시스템 언어 감지 기능을 갖춘 다국어 인터페이스.
- **타이포그래피:** 로스켈리 모노(내장).

## 설치

### 사전 빌드된 바이너리 다운로드

가장 쉽게 시작할 수 있는 방법은 GitHub에서 플랫폼(macOS, Windows 또는 Linux)에 맞는 최신 릴리스를 직접 다운로드하는 것입니다.

[**최신 버전 다운로드**](https://github.com/66HEX/frame/releases)

> **참고: 애플리케이션이 아직 코드 서명되지 않았으므로 시스템 설정에서 수동으로 승인해야 할 수 있습니다(이 파일 상단의 경고 참조).

### WinGet(Windows)

프레임은 공식 WinGet 리포지토리에서 `66HEX.Frame` 식별자로 사용할 수 있습니다.

```powershell
winget install --id 66HEX.Frame -e
```

업데이트하려면:

```powershell
winget upgrade --id 66HEX.Frame -e
```

### Homebrew(macOS)

MacOS 사용자의 경우, 사용자 지정 홈브루 탭을 사용하여 프레임을 쉽게 설치하고 업데이트할 수 있습니다:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Linux 시스템 요구 사항

앱이미지**를 사용하는 경우에도 프레임은 UI 렌더링과 미디어 재생 처리를 위해 시스템의 **WebKitGTK** 및 **GStreamer** 라이브러리를 사용합니다. Linux의 기본 대화상자는 **XDG 데스크톱 포털** 통합(및 데스크톱 전용 백엔드)과 `제니티`를 폴백으로 사용합니다. 소스 추가 시 애플리케이션이 충돌하거나 동영상 미리보기가 비어 있거나 파일 대화 상자가 제대로 열리지 않는다면 아래 패키지들을 설치하세요.

- **우분투/데비안:**

  ```bash
  sudo apt update
  sudo apt install libwebkit2gtk-4.1-0 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **Arch Linux:**

  ```bash
  sudo pacman -S --needed webkit2gtk-4.1 gst-plugins-base gst-plugins-good gst-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **페도라:**
  ```bash
  sudo dnf install webkit2gtk4.1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

> **KDE 사용자:** `xdg-desktop-portal-kde`(`xdg-desktop-portal-gtk` 대신)를 설치하면 플라즈마 네이티브 테마 대화 상자를 볼 수 있습니다.

### 소스에서 빌드

애플리케이션을 직접 빌드하거나 기여하고 싶다면 다음 단계를 따르세요.

**1. 전제 조건**

- **러스트:** [러스트 설치](https://www.rust-lang.org/tools/install)
- **번(또는 Node.js):** [번 설치](https://bun.sh/)
- **운영체제 종속성:** 사용 중인 운영체제의 [Tauri 필수 요구 사항](https://v2.tauri.app/start/prerequisites/)을 따르세요.

**2. 프로젝트 설정**

리포지토리를 복제하고 종속 요소를 설치합니다:

```bash
git clone https://github.com/66HEX/frame.git
cd frame
bun install
```

**바이너리 설정**

프레임에는 AI 업스케일링을 위해 FFmpeg/FFprobe 사이드카 바이너리와 Real-ESRGAN 사이드카 에셋이 필요합니다. 플랫폼에 맞는 버전을 자동으로 가져올 수 있는 스크립트를 제공합니다:

```bash
bun run setup:ffmpeg
bun run setup:upscaler
```

**4. 빌드 또는 실행**

- **개발:**

  ```bash
  bun tauri dev
  ```

- **프로덕션 빌드:**
  ```bash
  bun tauri build
  ```

## 사용법

1.  **입력: ** 시스템 대화 상자를 사용하여 파일을 선택합니다.
2.  **구성:**
    - **출처:** 감지된 파일 메타데이터 보기.
    - **출력:** 컨테이너 형식과 출력 파일명을 선택합니다.
    - **동영상:** 코덱, 비트 전송률/CRF, 해상도 및 프레임 속도를 구성합니다.
    - **이미지:** 이미지 해상도/확대, 픽셀 형식 및 선택적 AI 업스케일링을 구성합니다.
    - **오디오:** 코덱, 비트 전송률, 채널 및 특정 트랙을 선택합니다.
    - **사전 설정:** 재사용 가능한 변환 프로필을 저장하고 로드합니다.
3.  **실행:** Rust 백엔드를 통해 변환 프로세스를 시작합니다.
4.  **모니터링:** UI에서 실시간 로그와 퍼센트 카운터를 확인할 수 있습니다.

## 별 기록

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
  <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
</picture>

## 승인 및 타사 코드

- **Real-ESRGAN**: Copyright (c) 2021, Xintao Wang. [BSD 3-Clause](https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE)에 따라 라이선스가 부여됩니다.
- **FFmpeg**: [GPLv3](https://www.ffmpeg.org/legal.html)에 따라 라이선스가 부여되었습니다.

## 라이선스

자세한 내용은 [라이선스](../LICENSE)를 참조하세요.
