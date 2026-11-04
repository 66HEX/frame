# frame-app architecture

`frame-app` contains the native GPUI application and its adapters to `frame-core`
and `frame-updater`. Rendering lives under `src/app/`; conversion settings,
queue rules, preview geometry, persistence and process management remain separate
from the views.

## Application shell

| Location | Responsibility |
| --- | --- |
| `src/lib.rs` | Public crate modules and compatibility re-exports. |
| `src/app/mod.rs` | Application entry points and the stable `frame_app` action namespace. |
| `src/app/root.rs` | `FrameRoot` fields, construction, selected configuration and persistence. |
| `src/app/runtime.rs` | Application startup, key bindings and native window options. |
| `src/app/model.rs` | View selection, application snapshots and visual-fixture names. |
| `src/app/layout.rs` | Shared layout dimensions. These remain available at the crate root. |
| `src/app/render/` | Root composition, ordered focus registration, overlay attachment and platform window framing. |
| `src/app/workspace.rs` | Workspace columns, welcome screen and inspector composition. |

`FrameRoot` still owns the same fields in the same declaration order. Feature
modules implement its methods without introducing another state store. The root
render synchronizes the preview, registers focus, builds the content, attaches
preferences/import/update overlays, then finishes the accessibility frame.

## UI features

| Folder | Contents |
| --- | --- |
| `app/chrome/` | Titlebar, platform window controls, main-view navigation and drag state. |
| `app/preferences/` | Application settings sheet, appearance controls and their actions. |
| `app/updates/` | Update actions, session restoration, status, dialog, release notes and preferences section. |
| `app/queue/` | File list, import/drop handling, metadata probes and conversion actions. |
| `app/logs/` | Log rendering, scrolling and copy-feedback state. |
| `app/inspector/` | Per-file conversion settings tabs and their render state. |
| `app/preview/` | Canvas, crop, overlay, timeline and toolbar rendering, UI state and actions. |
| `app/input/` | Text editing, focus, binding, pointer handling, timecode drafts and input elements. |
| `app/components/` | Shared buttons, selects, checkboxes, sliders, scrollbars, tooltips and color controls. |

The inspector's `video/`, `audio_filters/`, `video_filters/`, `presets/` and `subtitles/`
folders separate tab composition from field controls and interactions. Preview
actions are grouped by canvas, crop, overlay, playback, seeking, request building
and runtime scheduling. `preview/viewport/` keeps media painting, rounded clipping
and bounds tracking beside the viewport view.

`app/primitives.rs` owns shared surface and button builders. `app/motion.rs`
owns transition helpers; `app/accessibility.rs` owns focus and accessibility
semantics. Color and shadow definitions live in `src/theme/`. Feature views use
these shared components and tokens.

## Domain and platform code

- `settings/model/`: configuration types, defaults, codec definitions, presets
  and metadata. Field order and serialization attributes are part of the contract.
- `settings/options/`: available choices derived from source metadata and config.
- `settings/updates/`: configuration mutations and normalization by settings domain.
- `preview/`: crop geometry, playback/timeline rules and overlay calculations.
- `preview_engine/ffmpeg_backend/`: process lifecycle, playback clock, video
  streaming, frame decoding, audio streaming/output and command plans.
- `file_queue/`, `conversion_events/`, `conversion_runner/`: queue rules,
  conversion progress and execution.
- `app_persistence/`, `update_session/`: stored settings and update-session recovery,
  with adjacent tests.
- Assets, native dialogs, notifications, runtime binary discovery and source
  probing retain their dedicated modules at `src/`.

GPUI callbacks coordinate work through these modules. FFmpeg process control stays
in the preview engine and conversion runner. Keep changes to codecs or conversion
arguments in `frame-core`, with typed adapters in `frame-app`.

## Module and test conventions

Keep production imports explicit. A module facade declares its children and
exports the interface used by its callers; it does not supply a general import
prelude to those children. GPUI's trait prelude is appropriate in rendering files.
Use the narrowest visibility needed by the owning feature or `app`.

Split files at responsibility boundaries, rather than a fixed line count. Keep
small related types, builders and calculations together. Avoid `include!` source
fragments and wrappers that exist only to hide an oversized file.

Domain tests stay beside their domain. Cross-feature application tests live in
`app/tests/`: `preview/` covers preview integration, `workflows/` covers user
actions and persistence, and the remaining modules cover import, window behavior,
fixtures, surfaces and render contracts. Shared test imports are compiled only
under `cfg(test)`.

## Paths changed in the November 2026 refactor

| Previous location | Current location |
| --- | --- |
| `app/chrome.rs` | `app/chrome/`, `app/preferences/`, `app/updates/`, `app/queue/drop_overlay.rs` |
| `app/settings_panel/` | `app/inspector/` |
| `app/preview_panel/`, `app/preview_actions.rs` | `app/preview/`, `app/preview/actions/` |
| `app/file_list_panel.rs`, `app/files.rs`, `app/metadata.rs`, `app/conversion.rs` | `app/queue/` |
| `app/logs_panel.rs`, `app/logs_state.rs` | `app/logs/` |
| `app/settings_actions.rs` | `app/preferences/actions.rs`, `app/inspector/presets/actions.rs`, `app/inspector/subtitles/import.rs` |
| `app/state.rs` | `app/root.rs`; feature state types live with their owners |
| `app/render.rs`, `app/tests.rs` | `app/render/`, `app/tests/` |
| `settings/model.rs`, `settings/options.rs`, `settings/updates.rs` | Corresponding directories with `mod.rs` facades |
| `preview_engine/ffmpeg_backend.rs` | `preview_engine/ffmpeg_backend/` |
| `theme.rs`, `app_persistence.rs`, `update_session.rs` | Corresponding directories with `mod.rs` entry points |

See [refactor verification](refactor-verification.md) for the reference commit,
comparison procedure and its limits.
