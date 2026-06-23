# Frame GPUI-CE

Native GPUI-CE rewrite of Frame.

This crate owns the new application shell, GPUI views, GPUI-specific state, and bundled assets used by the native app. Shared conversion/probe logic should live in `../frame-core`; the existing Tauri/Svelte app remains outside this crate until the rewrite reaches parity.

The rewrite intentionally stays self-contained here: local Frame UI wrappers are built directly on GPUI-CE primitives, assets live under `gpui-ce/assets/`, and no external GPUI component library is used.
