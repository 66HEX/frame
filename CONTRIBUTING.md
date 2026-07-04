# Contributing to Frame

Thank you for your interest in contributing to **Frame**. This document covers
the current project structure, local setup, checks, and pull request standards.

## Technical Stack

- **Application:** Rust native desktop app built with GPUI-CE.
- **Core Engine:** FFmpeg and FFprobe runtime binaries.
- **Shared Logic:** `frame-core` for conversion arguments, probing data,
  compatibility rules, filters, and validation.
- **Native UI:** `frame-app` for the application shell, GPUI views, app state,
  dialogs, bundled assets, and runtime integration.
- **Automation:** `cargo xtask` for checks, workflow generation, and native
  package entrypoints.
- **Scripts:** `script/` contains the xtask-called packaging entrypoints.

## Getting Started

### Prerequisites

To build and run Frame locally, you will need:

1. **Rust:** [Install Rust](https://www.rust-lang.org/tools/install)
2. **Platform toolchain:** install the C/C++ build tools and native desktop
   libraries required by Rust and GPUI-CE for your operating system.

### Local Setup

1. **Clone the repository:**

   ```bash
   git clone https://github.com/66HEX/frame.git
   cd frame
   ```

2. **Setup FFmpeg binaries:**

   The application looks for FFmpeg and FFprobe in `frame-app/resources/binaries/`
   during local development. Download the platform-specific tools with:

   ```bash
   cargo xtask setup-ffmpeg
   ```

3. **Run in development mode:**

   ```bash
   cargo xtask run
   ```

4. **Build a release binary:**

   ```bash
   cargo xtask build --release
   ```

5. **Build native packages when needed:**

   ```bash
   cargo xtask workflows
   cargo xtask bundle macos
   cargo xtask bundle linux
   cargo xtask bundle windows
   ```

   Windows app icons are embedded into the `.exe` by `frame-app/build.rs`
   during the normal Cargo build, and the Windows package script places the
   FFmpeg and FFprobe runtime files next to the app.

## Development Workflow

### Project Structure

- `frame-app/`: native GPUI-CE application, views, app state, dialogs, runtime
  binary lookup, bundled assets, native app identity, and package app icons.
- `frame-app/src/app/`: `FrameRoot`, workspace/logs rendering, settings panels,
  preview shell, import flow, conversion actions, and UI primitives.
- `frame-app/src/file_queue/`, `frame-app/src/settings/`, `frame-app/src/preview/`,
  and `frame-app/src/conversion_runner/`: tested domain logic kept outside
  rendering code where possible.
- `frame-core/`: shared conversion/probe logic, FFmpeg argument construction,
  filters, media compatibility rules, and conversion event types.
- `frame-core/media-rules.json`: source of truth for container, codec, stream,
  and pixel format compatibility.
- `tooling/xtask/`: Rust automation entrypoint for checks, bundling, and
  generated workflows.
- `script/`: platform bundling entrypoints called by `xtask` and GitHub
  Actions.
- `CHANGELOG.md`: product release history.

### Coding Standards

- **Rust formatting:** run `cargo fmt` for touched crates before submitting.
- **Rust linting:** keep `cargo clippy --all-targets -- -D warnings` clean.
- **Architecture:** prefer existing `frame-app/src/app/primitives.rs` UI building
  blocks and domain modules before adding new ad hoc rendering code.
- **Conversion Logic:** add FFmpeg behavior in `frame-core` first, then bridge it
  into `frame-app` through typed settings/configuration code.
- **Media Compatibility:** update `frame-core/media-rules.json` and focused tests
  when adding formats, codecs, stream-copy rules, or pixel formats.
- **Runtime Binaries:** do not commit downloaded files from
  `frame-app/resources/binaries/`.
- **Application Icons:** keep desktop package icons under
  `frame-app/resources/app-icons/`; iOS, Android, and Store/MSIX-specific icon
  sets are not part of the current desktop packaging flow.

### Testing & Quality Control

Before submitting a PR, run the relevant checks:

```bash
cargo xtask ci
```

For UI changes, add or update focused GPUI tests where practical. Visual parity
fixtures live in the GPUI test modules. Runtime visual fixture names and review
scenarios are listed in [Visual Fixtures](CONTRIBUTING_VISUAL_FIXTURES.md).

## Pull Request Process

1. Create a new branch for your feature or bugfix:

   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make focused commits with descriptive messages.
3. Include tests or explain why a change is not practical to test.
4. Push to your fork and submit a pull request.
5. Provide a clear description of the behavior change, screenshots for UI work,
   and any relevant issue numbers.

## Reporting Issues

If you find a bug or have a feature request, please
[open an issue](https://github.com/66HEX/frame/issues). Include your operating
system, source media details when relevant, reproduction steps, and FFmpeg logs
from the Logs view when conversion behavior is involved.

## Financial Support

If you want to support the long-term maintenance of Frame, especially
code-signing for macOS and Windows builds, use
[GitHub Sponsors](https://github.com/sponsors/66HEX).

---

By contributing to this project, you agree that your contributions will be
licensed under the project's [LICENSE](LICENSE).
