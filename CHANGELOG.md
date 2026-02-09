# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.21.2] - 2026-02-09

### Fixed

- **Runtime Log Highlighting Dependencies:** Moved `shiki` from `devDependencies` to `dependencies` and added direct runtime dependency on `@shikijs/themes`, so packaged builds include required log-highlighting modules.
- **Output Name Path Safety:** Hardened output name handling on both frontend and backend so custom names cannot escape the source directory via absolute paths or traversal segments.
- **Error Event Duplication:** Removed duplicate `conversion-error` emission from the worker path so failures are reported once through manager flow.
- **Settings Persistence Race:** Prevented settings write-back before initial hydration completes, fixing startup-time overwrites of saved preferences.
- **Subtitle Burn Path Escaping:** Expanded FFmpeg subtitle filter escaping for special characters (including quotes and bracket/comma tokens) to avoid burn-in command breakage on valid file paths.
- **Localization Consistency:** Replaced remaining hardcoded UI strings in logs/source panels with i18n keys and synced locale dictionaries.
- **Concurrency Limit Reactivity:** Applying a new max concurrency value now immediately re-processes the queue so pending tasks can start without waiting for another queue event.
- **Trim Range Validation:** Task validation now rejects non-increasing trim ranges (`end_time <= start_time`) and malformed trim timestamps before enqueue.
- **Output Extension Consistency:** Custom output names now always end with the selected container extension, preventing mismatches between UI container choice and written file suffix.
- **Dialog Directory Scope:** Folder picker permission scopes now respect the `recursive` flag when granting Tauri directory access, avoiding unintentionally broad directory grants.
- **Native Dialog Kind Contract:** Backend dialog kind parsing now explicitly accepts `question`, matching the frontend `askNativeDialog` type contract.
- **ML Upscale FPS Timeline:** Upscale re-encode now always reads extracted PNG frames at source FPS, while optional target FPS conversion is applied only on output (`-r`), preventing unintended playback speed changes.
- **ML Upscale Metadata Preserve:** In upscale mode, `metadata.mode = preserve` now maps metadata from the original source input (`-map_metadata 1`) so source tags are retained as expected.
- **ML Upscale Argument Regression Tests:** Added focused unit tests for upscale encode argument building (source FPS input timing and metadata mode behavior) to catch future pipeline regressions.

## [0.21.1] - 2026-02-08

### Fixed

- **Windows ML Upscaling Paths:** Normalized extended-length Windows paths (`\\?\...`) before passing frame and model directories to the Real-ESRGAN sidecar, fixing `_wfopen ... failed` errors during upscaling.

## [0.21.0] - 2026-02-08

### Fixed

- **Queue Cancellation Semantics:** Canceling a queued conversion now prevents it from starting later in the background. Queue state now tracks canceled IDs, avoids duplicate task IDs, and removes canceled items before worker launch.
- **Process Signal Safety:** Pause/resume/cancel controls now ignore invalid PID `0` placeholders, preventing unsafe Unix signal targeting. ML upscale startup no longer publishes a fake PID, and unexpected encoder shutdowns now return an explicit worker error instead of reporting silent success.
- **FFmpeg Stream Mapping:** Standard conversions now always map streams deterministically (`0:v:0`, `0:a?`, `0:s?`) when track overrides are not selected, eliminating ambiguous defaults and missing-stream edge cases.
- **Audio Config Application:** Audio codec and bitrate settings are now applied consistently even when no explicit source audio tracks are selected.
- **Cross-Tab Config Consistency:** Configuration normalization now runs in the shared state layer (including preset application), so container/codec/preset/upscale compatibility is enforced even if the Video tab was never opened.
- **Compatibility Rule Unification:** Video container/codec/preset compatibility now comes from a shared frontend module used by both UI selection logic and config normalization, preventing drift between tab behavior and saved state validation.
- **Input Validation Hardening:** Backend validation now rejects incompatible codec/container combinations and blocks ML upscaling with audio-only outputs before task enqueue.
- **Subtitle Container Handling:** Subtitle stream behavior is now container-aware (`mov_text` for MP4/MOV, `webvtt` for WebM, `copy` for MKV) and no longer auto-maps subtitle tracks when only burn-in subtitles are requested.
- **Queue Startup Recovery:** Conversion queue startup now handles per-file enqueue failures gracefully, marking only failed items as errors while continuing with valid tasks and keeping processing state in sync.
- **Timecode Input UX:** Trim timecode fields now support pasting valid values (`HH:MM:SS.mmm`, `MM:SS`, or seconds), reducing manual editing friction.
- **Lint Stability:** Added a scoped ESLint override for Shiki-rendered log HTML in `LogsView`, resolving the `svelte/no-at-html-tags` lint failure for trusted, syntax-highlighted output.
- **Queue Logic:** Resolved an issue where completed files would be re-queued for conversion when restarting the batch. The queue now explicitly ignores files with a "Completed" status.
- **Titlebar UX:** The "Start Conversion" button is now disabled when all selected files have already been successfully processed, providing better visual feedback and preventing accidental re-runs.

### Added

- **Hardware Decoding Support:** Integrated GPU-accelerated video decoding for input files using NVIDIA CUDA and Apple VideoToolbox. This reduces CPU load and improves conversion speed by offloading the decoding phase to the hardware.
- **Log Syntax Highlighting:** Integrated Shiki highlighting engine into `LogsView` for improved readability of FFmpeg output.
- **Custom Log Language:** Developed a comprehensive TextMate grammar for FFmpeg logs, featuring specialized highlighting for codecs, timestamps, file paths, CPU capabilities, and conversion phases (DECODE/ENCODE/UPSCALE).

### Changed

- **Settings Accessibility:** Users can now switch between all configuration tabs (Source, Output, Video, etc.) even after a file has been converted, while maintaining the locked state of individual settings.
- **Store Convention Consistency:** `updateStore` now uses the same object-based `$state` pattern as other frontend stores, reducing architectural divergence in shared state management.

## [0.20.0] - 2026-02-08

### Added

- **Codec-Container Compatibility:** Video encoders are now automatically filtered and disabled based on the selected output container (e.g., WebM only shows VP9/AV1 codecs). Incompatible encoders display an "Incompatible container" message and switch to a compatible codec automatically.

### Changed

- **Backend Refactoring:** Split monolithic `ffmpeg.rs` (1048 lines) into focused modules: `utils.rs`, `args.rs`, `upscale.rs`, `worker.rs`. Improves maintainability without changing functionality.

### Fixed

- **MKV Metadata Parsing:** Fixed metadata tags (Artist, Album, Genre, Date, Comment) not being read from MKV files. The parser now correctly handles both uppercase (MKV) and lowercase (MP4) tag variants.
- **Progress Display:** Resolved an issue where the UI would remain stuck on "Queued" status during the ML upscaling decode phase. A new `conversion-started` event now immediately updates the status to "Converting" when processing begins.
- **Windows Progress Indicator:** Fixed progress percentage not updating for h264 and h264_nvenc codecs on Windows. The FFmpeg stderr parser now correctly handles Windows-style carriage return (`\r`) line separators.
- **ML Upscale Parameter Parity:** The AI upscaling pipeline now supports all parameters from the standard conversion: rotation, flip, subtitle burn, FPS change, NVENC/VideoToolbox options, audio processing (codec, bitrate, volume, normalize, channels), metadata handling, and subtitle track selection.
- **ML Upscale Temp Cleanup:** Temporary PNG frame files are now properly deleted when an upscaling task fails or is cancelled from the UI.
- **Progress Reporting:** Fixed an issue where progress would remain at 0% for some files due to strict time parsing. The parser now correctly handles FFmpeg output with raw seconds or flexible time formats.

## [0.19.0] - 2026-02-07

### Added

- **AI Upscaling:** Integrated AI-powered video upscaling using Real-ESRGAN models (x2, x4) for high-quality resolution enhancement.
- **Features Architecture:** Introduced a new modular architecture in `src/lib/features/` to separate business logic from UI components.
  - `conversion`: Logic for queue management, presets, and conversion progress.
  - `files`: Logic for file list management and drag-and-drop operations.
  - `update`: Logic for app update checks and installation.
- **Component Reorganization:** Improved project structure by organizing components into logical subdirectories (`file-list`, `layout`, `logs`).
- **Unified Exports:** Implemented index files for feature modules and component groups to simplify imports and improve maintainability.

### Performance

- **Log Virtualization:** Implemented a virtualized list for the application logs, enabling smooth scrolling and rendering of thousands of entries without UI lag.

### Fixed

- **Video Trimming:** Resolved an issue where trimming a segment from the middle of a video would ignore the end point. The logic now uses a calculated duration (`-t`) instead of an absolute end time (`-to`) when a start offset is present.
- **Progress Reporting:** Fixed inaccurate progress bars during trimmed conversions. Progress is now correctly calculated relative to the trimmed segment length rather than the full source duration.

### Fixed

- **Process Lifecycle:** Resolved a "zombie process" issue on macOS where the application would remain running in the dock after closing the main window, due to hidden helper windows keeps the event loop alive.
- **UI Contrast:** Fixed text contrast in `LogsView` to improve readability.

## [0.18.1] - 2026-02-05

### Added

- **Native Dialogs:** Implemented a unified `askNativeDialog` system for cross-platform confirmation messages. Includes specialized macOS support via a hidden `always_on_top` helper window to ensure dialogs stay above the main HUD window without breaking visual effects or passing clicks to background applications.
- **Visual Feedback:** Added a global background overlay with backdrop blur that automatically activates whenever a native file or message dialog is open, blocking interactions with the main window while the dialog is active.
- **Error Reporting:** Conversion failures (e.g., hardware encoder issues) now display a native error dialog with the failure reason instead of silently failing. Error messages are also logged to the conversion log panel.

### Changed

- **Code Architecture:** Refactored the monolithic `conversion.rs` (1712 lines) into a modular structure with dedicated files for types, error handling, manager logic, FFmpeg argument building, media probing, and Tauri commands. Improves maintainability without changing public API.
- **Styling:** Cleaned up `src/routes/layout.css` by removing unused CSS classes and optimizing the global stylesheet.

### Fixed

- **Preview Panel:** Trim slider and timecode inputs are now disabled after conversion completes, preventing pointless edits to already-processed files.

## [0.18.0] - 2026-02-02

### Added

- **AV1 Hardware Acceleration:** Added support for NVIDIA's AV1 hardware encoder (`av1_nvenc`) for compatible RTX 40-series GPUs. Integrated with the existing quality slider for consistent VBR control.
- **Hardware Encoder Controls:** The video panel now exposes NVENC-specific AQ toggles (spatial and temporal) and a software-fallback switch for VideoToolbox, mirroring the new ffmpeg flag support.

### Changed

- **Preset Awareness:** Hardware encoders now only show presets they actually accept, and NVENC selections are automatically mapped to valid ffmpeg preset names to prevent failed launches with legacy user presets.
- **FFmpeg Argument Builder:** Updated to emit the correct hardware flags (`-cq:v`/AQ options for NVENC, `-allow_sw` for VideoToolbox) and to skip unsupported parameters like `-preset` for VideoToolbox, ensuring conversions no longer fail when switching between software and hardware encoders.

## [0.17.0] - 2026-02-02

### Added

- **Preview Panel:** Enhanced the video playback overlay with interactive behavior.
  - **Dynamic Overlay:** The play/pause overlay now automatically appears on hover during playback and remains visible when paused.
  - **Animated Transitions:** Implemented smooth Svelte fade transitions for the overlay and playback controls.
  - **Contextual Icons:** The overlay button now dynamically toggles between Play and Pause icons based on the current playback state.

### Fixed

- **Preview Panel:** Resolved a frame flickering issue where the video would jump back to the start frame while adjusting the trim handles. The playback loop logic is now suppressed during active dragging to ensure a smooth frame preview.

### Changed

- **Icon System:** Migrated the entire application icon set from `lucide-svelte` to Phosphor Icons.
- **Icon Architecture:** Implemented a central icon management system in `src/lib/icons` using a standardized "internal naming" convention (e.g., `IconPlay`, `IconTrash`, `IconClose`). This decouples UI components from specific libraries and simplifies future icon set swaps.
- **Performance:** Converted all raw SVG icons into native Svelte 5 components with support for reactive `size` and `class` properties.

## [0.16.0] - 2026-02-01

### Added

- **Windows Titlebar:** Introduced a dedicated titlebar component for Windows, replacing the previously shared Linux titlebar. This provides a more native look and feel on Windows systems.
- **Dynamic Font Switching:** Added the ability to toggle between Geist Mono and Geist Sans fonts across the entire application.
  - **New Visual Setting:** Added a font family selector in the App Settings under the Visuals section.
  - **Persistence:** The chosen font preference is saved and automatically applied on subsequent launches.
- **Subtitle Support:** Comprehensive handling of subtitle tracks within the application.
  - **Soft-subs:** Added ability to select and passthrough existing subtitle tracks from the source file. By default, all tracks are preserved if none are explicitly selected.
  - **Hard-subs (Burn-in):** Support for burning in external subtitle files (`.srt`, `.ass`, `.vtt`) directly into the video stream. The process includes automatic path escaping for cross-platform compatibility.

### Changed

- **UI:** Replaced text-based setting tabs with intuitive icon-based buttons (Source, Output, Video, Audio, Metadata, Presets) for a cleaner and more compact interface.
- **Documentation:** Updated README with Linux system requirements for AppImage users.

### Fixed

- **macOS Dialog Reparenting:** Native file dialogs are now spawned from an invisible helper window so the main HUD window keeps its rounded corners while the picker is open, eliminating the rectangular flash that previously appeared when the dialog borrowed the app window.

## [0.15.0] - 2026-02-01

### Added

- **Smart Scaling (Letterbox/Pillarbox):** Implemented intelligent scaling for custom resolutions. When both width and height are specified (e.g., in 4K or Social presets), the application now maintains the original aspect ratio by adding black bars (padding) instead of stretching the video.
- **Social Media Presets:** Added 6 new built-in presets for YouTube (1080p, 4K), X (Landscape, Portrait), TikTok/Reels, and Discord, optimized according to 2025 platform recommendations.
- **HEVC Hardware Acceleration:** Added support for H.265 (HEVC) hardware encoding via `hevc_videotoolbox` (Apple Silicon/Intel) and `hevc_nvenc` (NVIDIA).
- **Smart Encoder Detection:** The application now dynamically scans `ffmpeg` capabilities at startup to only show encoders supported by the user's hardware (e.g., hiding NVENC on macOS or VideoToolbox on Windows), replacing the previous static OS-based filtering.

### Changed

- **Code Architecture:** Refactored the Interactive Crop Tool logic into a dedicated utility module (`crop.ts`), improving maintainability and component readability.

### Fixed

- **Preset Matching:** Improved the logic for identifying the "Applied" preset in the UI by including video bitrate, custom resolution dimensions, and bitrate mode in the comparison, resolving an issue where multiple presets would appear as selected simultaneously.

## [0.14.0] - 2026-02-01

### Added

- **Batch Preset Application:** Added a new "Apply to All" button in the Presets tab. This allows users to instantly apply a selected preset to all pending files in the queue after a confirmation dialog, significantly speeding up batch configuration workflows.

## [0.13.1] - 2026-02-01

### Fixed

- **CI/CD:** Switched the Linux AMD64 build runner to `ubuntu-24.04` and pinned specific WebKitGTK versions to resolve `EGL_BAD_PARAMETER` errors when running the AppImage on modern Linux distributions like Arch/CachyOS.

## [0.13.0] - 2026-02-01

### Added

- **Interactive Crop Tool:** A powerful new tool for cropping videos directly in the preview panel.
  - **Visual Composition:** Includes a draggable area with a rule-of-thirds (3x3) grid overlay and various aspect ratio presets (Free, 1:1, 16:9, etc.).
  - **Auto-Zoom:** Automatically zooms and centers the preview on the cropped area after application to ensure pixel-perfect inspection.
  - **Robust Transformations:** Fully integrates with rotation and flip controls. The crop coordinates automatically adapt to video orientation changes, and interaction handles remain intuitive (mouse direction matches visual movement) even when the video is rotated or mirrored.

### Changed

- **Dashboard layout:** Split the left column into a 12-row grid so the trim preview card permanently occupies the top section while the file list sits below it. This removes the floating trim modal and gives the timeline controls dedicated real estate.
- **Trim workflow:** The trimming card now applies start/end changes immediately (no Save/Cancel buttons) and is always visible with the selected file, providing constant video preview and faster adjustments without opening overlays.
- **Transform controls:** Rotation and flip moved out of the Video tab and into the preview card as icon-only buttons, with rotation cycling through 0/90/180/270° on each click for quicker access while adjusting trims.
- **Localization:** Linux titlebar buttons now use the same translated strings as the macOS variant (no more hard-coded English labels).

## [0.12.0] - 2026-02-01

### Changed

- **Dashboard layout:** Split the left column into a 12-row grid so the trim preview card permanently occupies the top section while the file list sits below it. This removes the floating trim modal and gives the timeline controls dedicated real estate.
- **Trim workflow:** The trimming card now applies start/end changes immediately (no Save/Cancel buttons) and is always visible with the selected file, providing constant video preview and faster adjustments without opening overlays.
- **Transform controls:** Rotation and flip moved out of the Video tab and into the preview card as icon-only buttons, with rotation cycling through 0/90/180/270° on each click for quicker access while adjusting trims.
- **Localization:** Linux titlebar buttons now use the same translated strings as the macOS variant (no more hard-coded English labels).

## [0.11.0] - 2026-01-31

### Added

- **Video Transform:** New section in the Video tab for quick orientation fixes and mirroring.
  - **Rotation:** Rotate video by 90°, 180°, or 270° with a single click.
  - **Flip:** Toggle horizontal or vertical mirror reflections.
  - **Filter Integration:** Transformations are processed efficiently within the FFmpeg filter chain, compatible with existing scaling options.
- **Media Inspector:** Expanded the 'Source' tab into a comprehensive technical inspector.
  - Displays detailed video metadata: Profile, Pixel Format, Color Space, Color Range, and Primaries.
  - Displays detailed audio metadata: Sample rate (Hz/kHz) and bitrate per track.
  - Multi-track support: Lists technical details for all audio streams found in the file.
  - Redesigned UI with categorized sections (File, Video, Audio) for better readability.

## [0.10.0] - 2026-01-31

### Added

- **Metadata Editor:** Comprehensive metadata support with a dedicated tab.
  - **Modes:** Choose between `Preserve` (keep original, overwrite specific), `Clean` (remove all), or `Replace` (remove original, add new).
  - **Fields:** Edit standard tags like Title, Artist, Album, Genre, Date, and Comment.
  - **Visualization:** Placeholders in `Preserve` mode show the file's current metadata values for reference.

## [0.9.0]

### Added

- **Audio Control:** Added a volume slider allowing adjustment from 0% to 200%.
- **Loudness Normalization:** Added EBU R128 loudness normalization for consistent audio levels across files.

### Changed

- **UI:** Refined the layout of the trim modal and file list icons for better visual alignment.
- **UX:** Changed disabled state behavior to prevent interaction cursor.

## [0.8.0] - 2026-01-29

### Added

- **Video Trimming:** New interactive modal for precise video trimming. Features include:
  - **Visual Timeline:** Draggable handles for setting start and end points with a real-time video preview.
  - **Timecode Precision:** Dedicated `TimecodeInput` component for millisecond-accurate manual entry.
  - **Live Preview:** Instant seek to start/end points and looped playback of the selected range.

## [0.7.1] - 2026-01-28

### Added

- **Task Cancellation:** Safely cancel active or paused tasks by removing them from the list (active tasks must be paused first to prevent accidental cancellation). This ensures that background FFmpeg processes are correctly terminated and queue slots are freed.

## [0.7.0] - 2026-01-28

### Added

- **Task Control:** Added ability to pause and resume active conversion tasks directly from the file list. Supported on macOS, Linux, and Windows.
- **Notifications:** Added native system notifications that trigger when a conversion queue finishes processing, summarizing the results (successes and errors).

### Fixed

- **CI/CD:** Removed deprecated `depends_on macos` directive from the Homebrew Cask generation workflow to resolve `brew doctor` warnings.

## [0.6.0] - 2026-01-28

### Added

- **Audio File Support:** Added full support for importing and converting standalone audio files (MP3, WAV, FLAC, M4A, AAC).
- **Smart UI Adaptation:** The interface now automatically adapts when an audio file is selected:
  - **Tab Management:** The "Video" tab is automatically disabled.
  - **Container Filtering:** Video containers (MP4, MKV, etc.) are disabled in the output settings to prevent invalid configurations.
  - **Preset Filtering:** Incompatible video presets are visually disabled in the presets library.
- **Auto-Format Switching:** Importing an audio-only file automatically switches the output configuration to a compatible audio format (e.g., MP3) if a video container was previously selected.

## [0.5.0] - 2026-01-27

### Added

- **Internationalization:** Multi-language interface with automatic system language detection. Supported languages: English, 简体中文, 日本語, 한국어, Español, Русский, Français, Deutsch, Italiano.
- **Documentation:** Added localized README files for all supported languages.

## [0.4.0] - 2026-01-26

### Added

- **Audio Lossless:** Full support for lossless audio conversion including FLAC, WAV (PCM), and ALAC (Apple Lossless).
- **Containers:** Added `.flac`, `.wav`, and `.m4a` to the output container options.
- **Presets:** Added dedicated built-in presets for "Audio FLAC" and "Audio WAV" (Lossless).
- **Distribution:** Added official Homebrew Tap support. Users can now install via `brew tap 66HEX/frame && brew install --cask frame`.
- **Developer Experience:** Added `bun run setup:ffmpeg` to pull platform-specific FFmpeg/FFprobe binaries into `src-tauri/binaries`, mirroring the CI release workflow.

### Changed

- **UX:** Changing the output container now automatically switches the audio codec to a compatible default (e.g., selecting FLAC container auto-selects FLAC codec), preventing invalid configurations.

## [0.3.3] - 2026-01-25

### Changed

- **Window Effects:** Dropped the `window_vibrancy` crate in favor of Tauri's built-in `WindowEffect` / `EffectsBuilder`, keeping the same Acrylic / HudWindow visuals while relying on the maintained `tauri_utils` surface.
- **UI:** Removed the global `border-radius` on the HTML root since rounded corners are handled elsewhere.

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

[Unreleased]: https://github.com/66HEX/frame/compare/0.21.2...HEAD
[0.21.2]: https://github.com/66HEX/frame/compare/0.21.1...0.21.2
[0.21.1]: https://github.com/66HEX/frame/compare/0.21.0...0.21.1
[0.21.0]: https://github.com/66HEX/frame/compare/0.20.0...0.21.0
[0.20.0]: https://github.com/66HEX/frame/compare/0.19.0...0.20.0
[0.19.0]: https://github.com/66HEX/frame/compare/0.18.1...0.19.0
[0.18.1]: https://github.com/66HEX/frame/compare/0.18.0...0.18.1
[0.18.0]: https://github.com/66HEX/frame/compare/0.17.0...0.18.0
[0.17.0]: https://github.com/66HEX/frame/compare/0.16.0...0.17.0
[0.16.0]: https://github.com/66HEX/frame/compare/0.15.0...0.16.0
[0.15.0]: https://github.com/66HEX/frame/compare/0.14.0...0.15.0
[0.14.0]: https://github.com/66HEX/frame/compare/0.13.1...0.14.0
[0.13.1]: https://github.com/66HEX/frame/compare/0.13.0...0.13.1
[0.13.0]: https://github.com/66HEX/frame/compare/0.12.0...0.13.0
[0.12.0]: https://github.com/66HEX/frame/compare/0.11.0...0.12.0
[0.11.0]: https://github.com/66HEX/frame/compare/0.10.0...0.11.0
[0.10.0]: https://github.com/66HEX/frame/compare/0.9.0...0.10.0
[0.9.0]: https://github.com/66HEX/frame/compare/0.8.0...0.9.0
[0.8.0]: https://github.com/66HEX/frame/compare/0.7.1...0.8.0
[0.7.1]: https://github.com/66HEX/frame/compare/0.7.0...0.7.1
[0.7.0]: https://github.com/66HEX/frame/compare/0.6.0...0.7.0
[0.6.0]: https://github.com/66HEX/frame/compare/0.5.0...0.6.0
[0.5.0]: https://github.com/66HEX/frame/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/66HEX/frame/compare/0.3.3...0.4.0
[0.3.3]: https://github.com/66HEX/frame/compare/0.3.2...0.3.3
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
