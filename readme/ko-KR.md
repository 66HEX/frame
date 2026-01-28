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

**Frame**은 Tauri v2 프레임워크 기반의 고성능 미디어 변환 유틸리티입니다. FFmpeg 작업을 위한 네이티브 인터페이스를 제공하며, 비디오 및 오디오 트랜스코딩 매개변수를 세밀하게 제어할 수 있습니다. Rust 기반 백엔드로 동시 작업 관리와 프로세스 실행을 처리하고, Svelte 5 프론트엔드로 설정과 상태 모니터링을 수행합니다.

<br />
<div align="center">
  <img src="../preview.png" alt="Frame 애플리케이션 미리보기" width="800" />
</div>
<br />

> [!WARNING]
> **서명되지 않은 애플리케이션 안내**
> 애플리케이션이 현재 서명되지 않아 운영체제에서 경고가 표시됩니다:
>
> - **macOS:** 시스템이 앱과 사이드카 바이너리에 격리 속성을 추가합니다. 실행하려면 수동으로 속성을 제거하세요:
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:** SmartScreen이 앱 실행을 차단할 수 있습니다. **"추가 정보"**를 클릭한 후 **"실행"**을 선택하세요.

## 기능

### 미디어 변환 코어

- **컨테이너 지원:** `mp4`, `mkv`, `webm`, `mov`, `mp3`, `m4a`, `wav`, `flac`.
- **비디오 인코더:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (SVT-AV1)
  - **하드웨어 가속:** `h264_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA)
- **오디오 인코더:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`
- **비트레이트 제어:** 고정 품질 (CRF) 또는 목표 비트레이트 (kbps)
- **스케일링:** 바이큐빅, 란초스, 바이리니어, 니어레스트
- **메타데이터 분석:** `ffprobe`를 통한 스트림 정보 자동 추출 (코덱, 재생 시간, 비트레이트, 채널 레이아웃)

### 아키텍처 및 워크플로우

- **동시 처리:** Rust로 구현된 비동기 작업 큐 관리자 (`tokio::mpsc`), 동시 FFmpeg 프로세스 수 제한 (기본값: 2)
- **실시간 텔레메트리:** FFmpeg `stderr` 스트림 파싱으로 정확한 진행률 추적 및 로그 출력
- **프리셋 관리:** 재사용 가능한 변환 프로필 설정 저장

## 기술 스택

### 백엔드 (Rust / Tauri)

- **코어:** Tauri v2 (Rust Edition 2024)
- **런타임:** `tokio` (비동기 I/O)
- **직렬화:** `serde`, `serde_json`
- **프로세스 관리:** `tauri-plugin-shell` (FFmpeg/FFprobe 사이드카 실행)
- **시스템 통합:** `tauri-plugin-dialog`, `tauri-plugin-fs`, `window-vibrancy`

### 프론트엔드 (SvelteKit)

- **프레임워크:** Svelte 5 (Runes API)
- **빌드 시스템:** Vite
- **스타일링:** Tailwind CSS v4, `clsx`, `tailwind-merge`
- **상태 관리:** Svelte 5 `$state` / `$props`
- **국제화:** 다국어 인터페이스, 시스템 언어 자동 감지
- **타이포그래피:** Geist Mono (내장)

### 설치

#### Homebrew 사용 (macOS)

macOS에서 Frame을 설치하고 업데이트하는 가장 쉬운 방법은 Homebrew Tap을 사용하는 것입니다:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### 사전 빌드 릴리스 사용

Frame을 실행하는 가장 쉬운 방법은 [GitHub Releases](https://github.com/66HEX/frame/releases) 페이지에서 사전 빌드 패키지를 다운로드하는 것입니다. 각 릴리스에는 macOS (Intel/Apple Silicon), Windows, Linux (AppImage/Deb) 빌드가 포함되어 있습니다. 바이너리가 서명되지 않아 OS에서 경고가 표시되고 수동 승인이 필요할 수 있습니다.

### 사전 요구 사항

- Node.js 런타임 (또는 Bun)
- Rust 툴체인 (`cargo`)
- **FFmpeg** 및 **FFprobe** 바이너리를 `src-tauri/binaries/` 디렉토리에 배치
  - 명명 규칙: `ffmpeg-<target-triple>` (예: `ffmpeg-aarch64-apple-darwin` 또는 `ffmpeg-x86_64-pc-windows-msvc.exe`)

> 팁: `bun run setup:binaries` (또는 `npm run setup:binaries`)를 실행하면 OS/아키텍처에 맞는 바이너리를 자동 다운로드합니다. `--force`로 기존 다운로드를 갱신할 수 있습니다.

### 빌드 방법

1.  **의존성 설치:**

    ```bash
    bun install
    ```

2.  **개발 서버 시작:**

    ```bash
    bun run tauri dev
    ```

3.  **프로덕션 빌드:**
    ```bash
    bun run tauri build
    ```

## 사용법

1.  **입력:** 시스템 대화상자로 파일 선택
2.  **설정:**
    - **소스:** 감지된 파일 메타데이터 확인
    - **출력:** 컨테이너 형식과 출력 파일명 선택
    - **비디오:** 코덱, 비트레이트/CRF, 해상도, 프레임레이트 설정
    - **오디오:** 코덱, 비트레이트, 채널, 특정 트랙 선택
    - **프리셋:** 재사용 가능한 변환 프로필 저장 및 불러오기
3.  **실행:** Rust 백엔드를 통해 변환 프로세스 시작
4.  **모니터링:** UI에서 실시간 로그와 진행률 확인

## 라이선스

GPLv3 라이선스. 자세한 내용은 [LICENSE](../LICENSE) 참조.
