# Contributing to Frame

Thank you for your interest in contributing to **Frame**! We appreciate your help in making this media conversion utility even better.

## Technical Stack

- **Backend:** Rust (Tauri v2)
- **Frontend:** Svelte 5 (TypeScript)
- **Package Manager:** [Bun](https://bun.sh/)
- **Core Engine:** FFmpeg & FFprobe (Sidecars)

## Getting Started

### Prerequisites

To build and run Frame locally, you will need:

1.  **Rust:** [Install Rust](https://www.rust-lang.org/tools/install)
2.  **Bun:** [Install Bun](https://bun.sh/)
3.  **Tauri Dependencies:** Follow the [Tauri setup guide](https://v2.tauri.app/start/prerequisites/) for your OS.

### Local Setup

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/66HEX/frame.git
    cd frame
    ```

2.  **Install dependencies:**

    ```bash
    bun install
    ```

3.  **Setup FFmpeg binaries:**
    The application requires FFmpeg/FFprobe sidecars in `src-tauri/binaries/`. We provide a script to fetch them:

    ```bash
    bun run setup:ffmpeg
    ```

4.  **Run in development mode:**
    ```bash
    bun tauri dev
    ```

## Development Workflow

### Project Structure

- `src/`: Svelte 5 frontend components and stores.
- `src-tauri/src/`: Rust backend logic (task management, FFmpeg argument building).
- `src-tauri/capabilities/`: Tauri permission configurations.
- `scripts/`: Build and setup scripts.

### Coding Standards

- **Rust:** Use `cargo fmt` for formatting and `cargo clippy` for linting.
- **Frontend:** Follow the existing Svelte 5 patterns (runes like `$state`, `$effect`). Use Prettier for formatting.
- **FFmpeg:** When adding new FFmpeg features, ensure they are validated in `src-tauri/src/conversion.rs` and added to the `ConversionConfig` struct.

### Testing & Quality Control

Before submitting a PR, please ensure:

1.  **Build:** The project builds correctly: `bun tauri build --no-bundle`.
2.  **Rust Tests:** All backend tests pass: `cd src-tauri && cargo test`.
3.  **Type Check & Lint:** Run `bun run check` and `bun run lint` to catch frontend issues.
4.  **Formatting:** Ensure all code is properly formatted:
    - For Rust: `cd src-tauri && cargo fmt`
    - For Frontend: `bun run format`

## Pull Request Process

1.  Create a new branch for your feature or bugfix: `git checkout -b feature/your-feature-name`.
2.  Make your changes and commit them with descriptive messages.
3.  Push to your fork and submit a Pull Request.
4.  Provide a clear description of the changes and any relevant issue numbers.

## Reporting Issues

If you find a bug or have a feature request, please [open an issue](https://github.com/66HEX/frame/issues). Include as much detail as possible, such as your operating system and the FFmpeg logs (accessible via the "Logs" view in the app).

---

By contributing to this project, you agree that your contributions will be licensed under the project's [LICENSE](LICENSE).
