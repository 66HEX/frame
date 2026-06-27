# Frame App

Native GPUI-CE application for Frame.

This crate owns the application shell, GPUI views, GPUI-specific state, and bundled assets used by the native app. Shared conversion/probe logic is consumed through the `frame-core` crate; GPUI app code and bundled assets stay in this directory.

The app intentionally stays self-contained here: local Frame UI wrappers are built directly on GPUI-CE primitives, assets live under `frame-app/assets/`, and no external GPUI component library is used.

## Source Layout

- `src/main.rs` is only the GPUI entrypoint.
- `src/app_info.rs` owns app identity constants shared by runtime and package
  metadata.
- `src/app/` owns `FrameRoot`, window runtime, shell rendering, import/conversion/metadata workflows, and UI panels.
- `src/app/input/`, `src/app/preview_panel/`, and `src/app/settings_panel/` split the largest GPUI UI surfaces into focused submodules.
- `src/file_queue/`, `src/settings/`, `src/preview/`, and `src/conversion_runner/` contain tested domain logic outside rendering code.
- `src/assets/` embeds only files from `frame-app/assets/`.
- `resources/app-icons/` contains the native desktop package icon set consumed
  by `build.rs`, `cargo bundle`, and Linux packaging.

Build output stays under `frame-app/target/` and is ignored by `frame-app/.gitignore`. macOS-specific native-window glue is limited to hiding AppKit's standard titlebar buttons so the custom Frame controls are the only visible traffic lights. Windows icons are embedded by `build.rs`; Linux packages should use `scripts/bundle-linux` so the `.desktop` file and hicolor icons are installed together.
