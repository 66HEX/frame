# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0-beta.2] - 2026-01-23

### Added

- **macOS Intel support:** Added builds and binaries for x86_64 Mac architecture.
- **Smart scrolling:** Implemented automatic scrolling in the logs view.
- **Global Settings:** New "App" tab in settings for global configuration.
- **Conversion Safety:** Disable the remove button for files currently being converted to prevent errors.

### Changed

- **Estimation Algorithm:** Refactored and fine-tuned the file size estimation logic for better accuracy.
- **UI Consistency:** Standardized title bar button sizes across all platforms.
- **UI Cleanup:** General cleanup and refinement of UI components.
- **Platform Compatibility:** Gated vibrancy imports to improve stability across different OS.

### Fixed

- **Input Validation:** Restricted numeric input fields to digits only.
- **CI/CD:** Resolved binary caching conflicts and build dependency issues.
- **Windows Packaging:** Removed problematic MSI target.

## [0.2.0-beta.1] - 2026-01-22

### Added

- **Cross-platform support:** Added builds for Windows x86_64, Linux x86_64, and Linux aarch64.
- **Selective conversion:** Ability to convert only selected assets instead of processing the entire batch.

### Changed

- **UI:** Enhanced visual alignment in the main assets table.
- **License:** Project license changed to GPLv3.
- **Architecture:** Refactored views into reusable components for better maintainability.
- **Code Organization:** Improved separation of concerns across the codebase.

## [0.1.0] - 2026-01-19

### Added

- Initial public release of Frame.
- Native macOS UI for FFmpeg-based media conversion.
- **Container Support:** MP4, MKV, WebM, MOV, and MP3.
- **Video Encoders:** H.264, H.265, VP9, ProRes, AV1.
- **Audio Encoders:** AAC, Opus, MP3, AC3.
- **Hardware Acceleration:** Support for Apple VideoToolbox and NVIDIA NVENC.
- Concurrent conversion pipeline with real-time progress tracking.
- Automatic media metadata probing via FFprobe.
- Preset-based configuration system.

[0.2.0-beta.2]: https://github.com/66HEX/frame/compare/0.2.0-beta.1...0.2.0-beta.2
[0.2.0-beta.1]: https://github.com/66HEX/frame/compare/0.1.0...0.2.0-beta.1
[0.1.0]: https://github.com/66HEX/frame/releases/tag/0.1.0
