# Frame GPUI-CE

Native GPUI-CE rewrite of Frame.

This crate owns the new application shell, GPUI views, GPUI-specific state, and bundled assets used by the native app. Shared conversion/probe logic should live in `../frame-core`; the existing Tauri/Svelte app remains outside this crate until the rewrite reaches parity.

