# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.2] - 2026-01-25

### Added

- **UI:** Added a "Window Tint" slider in App Settings that lets you control the background opacity (20‑100%), persists the choice, and applies it immediately across the app window.

### Changed

- **Windows UI:** Switched the desktop effect from Mica to Acrylic for both the main window and splash screen to better reflect the adjustable tint and improve consistency with system styling.

## [0.3.1] - 2026-01-25

### Added

- **Auto-Update:** Added a user preference to enable or disable automatic update checks on startup. This can be toggled via a new checkbox in the App Updates section of the settings.

## [0.3.0] - 2026-01-25

### Added

- **UI:** Added a custom settings sheet and implemented animations for all overlays.
- **Auto-Update:** Added Markdown parsing and text styling for release notes in the update dialog.

### Removed

- **UI:** Removed Windows titlebar in favor of Linux titlebar which aligns better with the overall design and user experience.


## [0.2.3] - 2026-01-25

### Added

- **Auto-Update:** Implemented a robust in-app update mechanism with a custom UI overlay, powered by the Tauri Updater plugin and GitHub Releases. Supports automatic checking, downloading, and restarting the application.

### Changed

- **Design:** Reduced window tint opacity for a cleaner look.

### Fixed

- **CI/CD:** Fixed multiple issues with the build pipeline, including correct artifact tagging, macOS updater bundle generation (`.app.tar.gz`), and signature verification.

### Removed

- **UI:** Removed HTML title and webview window title.

## [0.2.3-beta.3] - 2026-01-25

### Fixed

- **CI/CD:** Corrected the release tagging strategy in the build pipeline. Update artifacts now correctly point to version tags (e.g., `0.2.3-beta.3`) instead of prefixed tags, resolving 404 errors during update checks.

## [0.2.3-beta.2] - 2026-01-25

### Fixed

- **Auto-Update:** Resolved signature verification errors by properly passing the private key password to the bundler in the CI/CD pipeline.
- **macOS Updates:** Enabled updater support for macOS by adding the `.app` bundle target, allowing for the generation of required `.tar.gz` artifacts.

## [0.2.3-beta.1] - 2026-01-25

### Added

- **Auto-Update:** Implemented in-app update mechanism with a custom UI overlay, powered by the Tauri Updater plugin and GitHub Releases.

### Changed

- **Design:** Reduced window tint opacity for a cleaner look.

### Removed

- **UI:** Removed HTML title and webview window title.

## [0.2.2] - 2026-01-24

### Changed

- **Cleanup:** Further codebase cleanup.

## [0.2.1] - 2026-01-24

### Changed

- **Design:** Improved color palette contrast and introduced a colder hue for better visual aesthetics.
- **Cleanup:** Removed unused light mode design tokens.

### Fixed

- **UI:** Resolved inconsistencies in card colors.
- **Type Safety:** Fixed an async `onMount` type error in the main page component.

## [0.2.0] - 2026-01-24

### Added

- **Drag & Drop:** Support for dragging files directly into the application window with a visual overlay.
- **Hardware Acceleration:** Enhanced support for Apple VideoToolbox and NVIDIA NVENC with dedicated quality sliders (1-100).
- **Smart Codec Filtering:** Intelligently hides hardware codecs not supported by the user's OS.
- **Cross-Platform Support:** Official builds for Windows (x64), Linux (x64/arm64), and macOS (Intel).
- **Native Experience:** Implemented global tab-key blocking and focus ring removal for a native application feel.
- **Splash Screen:** Added a polished startup splash screen.
- **Global Settings:** New "App" tab for configuring parameters like Max Concurrency.

### Removed

- **Estimation:** Removed the estimated output size panel to prioritize UI simplicity.

### Changed

- **Architecture:** Major refactoring of the frontend into modular, reusable components (Svelte 5 Runes).
- **License:** Project re-licensed to GPLv3.

### Fixed

- **Windows UI:** Resolved window dragging artifacts and transparency issues.
- **Input Validation:** Numeric fields now strictly reject non-digit input.

## [0.2.0-beta.4] - 2026-01-23

### Added

- **Hardware Acceleration UX:** Added a dedicated quality slider (1-100) for Hardware Encoders (VideoToolbox, NVENC) which now correctly maps to native quality flags (`-q:v`, `-cq:v`) instead of CRF.
- **Smart Codec Filtering:** The application now intelligently hides hardware codecs not supported by the user's operating system (e.g., hiding NVENC on macOS).

### Removed

- **Estimation:** Removed the estimated output size panel to prioritize UI simplicity.

### Changed

- **UI:** Updated scrollbar styling to better integrate with the application theme.

## [0.2.0-beta.3] - 2026-01-23

### Added

- **Splash Screen:** Implemented a dedicated splash screen with "Late Show" logic for smoother startup.

### Fixed

- **Windows UI:** Disabled window transparency on Windows to resolve title bar artifacts when dragging.

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

[Unreleased]: https://github.com/66HEX/frame/compare/0.3.2...HEAD
[0.3.2]: https://github.com/66HEX/frame/compare/0.3.1...0.3.2
[0.3.1]: https://github.com/66HEX/frame/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/66HEX/frame/compare/0.2.3...0.3.0
[0.2.3]: https://github.com/66HEX/frame/compare/0.2.2...0.2.3
[0.2.3-beta.3]: https://github.com/66HEX/frame/compare/0.2.3-beta.2...0.2.3-beta.3
[0.2.3-beta.2]: https://github.com/66HEX/frame/compare/0.2.3-beta.1...0.2.3-beta.2
[0.2.3-beta.1]: https://github.com/66HEX/frame/compare/0.2.2...0.2.3-beta.1
[0.2.2]: https://github.com/66HEX/frame/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/66HEX/frame/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/66HEX/frame/compare/0.2.0-beta.4...0.2.0
[0.2.0-beta.4]: https://github.com/66HEX/frame/compare/0.2.0-beta.3...0.2.0-beta.4
[0.2.0-beta.3]: https://github.com/66HEX/frame/compare/0.2.0-beta.2...0.2.0-beta.3
[0.2.0-beta.2]: https://github.com/66HEX/frame/compare/0.2.0-beta.1...0.2.0-beta.2
[0.2.0-beta.1]: https://github.com/66HEX/frame/compare/0.1.0...0.2.0-beta.1
[0.1.0]: https://github.com/66HEX/frame/releases/tag/0.1.0
