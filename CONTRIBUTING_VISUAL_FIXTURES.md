# Visual Fixtures

Frame includes runtime visual fixtures for manual GPUI review. They seed the app
with deterministic state so contributors can inspect UI surfaces without finding
matching media files, waiting for conversions, or triggering update checks.

The source of truth is `frame-app/src/app/model.rs` for fixture names and
`frame-app/src/app/fixtures.rs` for seeded state.

## Running a Fixture

Set `FRAME_GPUI_VISUAL_FIXTURE` before launching the app:

```bash
FRAME_GPUI_VISUAL_FIXTURE=update-available cargo xtask run
```

Any fixture can be combined with a supported interface scale using
`FRAME_GPUI_UI_SCALE`. Values may be written as `80` or `80%`; supported
presets are `80`, `90`, `100`, `110`, `125`, `150`, `175`, and `200`.
Use `FRAME_VISUAL_THEME=dark|light` to override the persisted color theme for
the fixture process without writing the override back to settings.

```bash
FRAME_GPUI_VISUAL_FIXTURE=settings-subtitles \
FRAME_GPUI_UI_SCALE=200 \
FRAME_VISUAL_THEME=light \
cargo xtask run
```

Use a clean app window for visual review when possible. Fixtures run at startup
and intentionally override only the state they need for the target scenario.

## Deterministic Native PNG Capture

The `render-snapshot` example draws a hidden 1200 × 800 logical-pixel window,
settles motion, moves the simulated pointer outside the window, and draws three
frames synchronously before GPU readback. It uses in-memory settings and does
not advance asynchronous preview work while capturing.

```bash
cargo build --locked -p frame-app --example render-snapshot \
  --features gpui_platform/test-support

FRAME_GPUI_VISUAL_FIXTURE=settings-video \
FRAME_GPUI_UI_SCALE=100 \
FRAME_VISUAL_THEME=dark \
target/debug/examples/render-snapshot /tmp/frame-settings-video.png
```

Set `FRAME_VISUAL_CLICK_LABEL="Video encoder"` with
`FRAME_GPUI_VISUAL_FIXTURE=settings-video-hardware` to capture the open encoder
menu. The example clicks the rendered accessibility node with native pointer
events before the final draw.

For exact comparisons, capture both builds on the same machine, display scale,
font environment and graphics backend. Compare the PNG bytes or SHA-256 hashes.
The example deliberately fixes hover position: a physical pointer over the file
list otherwise changes the visibility of row actions even in a hidden window.
Use the interactive fixtures above to review motion, hover and asynchronous media.

The [structural refactor verification](frame-app/docs/refactor-verification.md)
records a complete light/dark, 100%/200% comparison.

## Appearance Regression Pair

Run this pair after changing shared card, button, input, or separator styling.
It replaces unit assertions against the internal order of `BoxShadow` layers
and raw button color tokens with a review of the rendered result.

```bash
FRAME_GPUI_VISUAL_FIXTURE=app-settings \
FRAME_GPUI_UI_SCALE=100 \
FRAME_VISUAL_THEME=dark \
cargo xtask run
```

```bash
FRAME_GPUI_VISUAL_FIXTURE=app-settings \
FRAME_GPUI_UI_SCALE=100 \
FRAME_VISUAL_THEME=light \
cargo xtask run
```

For both themes, verify the following rendered relationships:

1. the settings sheet and its cards remain visually separated from the app
   surface without clipped shadows;
2. default, secondary, ghost, disabled, hovered, and pressed buttons remain
   distinguishable;
3. text inputs and select controls use the same recessed depth as neighboring
   buttons;
4. separators remain crisp at the platform pixel scale and do not turn into a
   blurred band;
5. focus rings, text contrast, and disabled-state contrast remain legible.

Capture before-and-after images at the same window size when a change is
intentional. The code-level WCAG contrast and focus behavior tests remain the
automated safety net; this pair checks what those structural tests could not:
the final GPUI rendering.

## CPU Render Benchmark

Use the native benchmark to compare the cost of rebuilding a complete window:

```bash
FRAME_GPUI_VISUAL_FIXTURE=settings-video \
FRAME_GPUI_UI_SCALE=100 \
FRAME_VISUAL_THEME=dark \
cargo run --locked -p frame-app --example render-benchmark
```

The benchmark uses a 1200 × 800 window, native fonts and in-memory fixture state.
It warms up for 20 frames, then reports the median, p95 and maximum CPU time over
120 full redraws. Playback tasks do not advance during the synchronous loop;
the measurement excludes GPU presentation and is not a playback FPS result.
No persisted settings or media files are required.

Compare `workspace-empty`, `workspace-image`, `settings-source`, `settings-video`
and `preview-crop-custom` on the same machine, with the same scale and theme.
The development profile optimizes both GPUI and Taffy because GPUI instantiates
Taffy's generic layout code. Optimizing Taffy alone leaves that work unoptimized.
Frame's application code retains its normal debug settings.

## Available Fixtures

| Fixture | Visual scenario | What it is useful for checking |
| --- | --- | --- |
| `app-settings` | Runtime app settings sheet | Settings modal layout, concurrency draft value, modal focus treatment, close action, and app-level settings spacing. |
| `app-settings-theme-open` | App settings with the Theme select open | Dark/Light option order, selected checkmark, keyboard focus treatment, popover placement, and equal-width Appearance columns. |
| `app-settings-ui-open` | App settings with UI scale open at 200% | Scaled geometry and typography, scrollable sheet body, popover placement, and titlebar coexistence. |
| `logs-active` | Logs tab with an active FFmpeg conversion | Log tab navigation, active file state, plain FFmpeg log rendering, progress line wrapping, monospaced text rendering, and scroll density. |
| `preview-ready` | Workspace with a selected ready video source | Preview panel shell, selected video metadata, timeline controls, toolbar visibility, empty frame handling, and source video state. |
| `preview-crop` | Preview panel with crop mode enabled | Crop aspect bar, crop overlay geometry, crop handles, preview toolbar coexistence, and canvas framing while editing crop bounds. |
| `preview-crop-options` | Crop aspect ratio selector open | Standard select options, selected state, keyboard navigation, scrolling, and popover placement at different UI scales. |
| `preview-crop-custom` | Custom crop fields with the Video tab visible | Ratio/pixel mode switching, width and height fields, validation, and live resolution synchronization. |
| `settings-source` | Source settings tab with ready video metadata | File information rows, video stream metadata rows, source tab spacing, and selected source summary. |
| `settings-output` | Output settings tab with a custom output name | Output filename field, container selection state, output tab layout, and long-name alignment. |
| `settings-video` | Video settings tab with custom resolution and CRF mode | Video codec controls, custom width and height inputs, CRF controls, bitrate mode layout, and dense control grouping. |
| `settings-video-av1` | SVT-AV1 selected with preset 4 and 10-bit pixels | Native preset numbers, encoding speed guidance, and the explicit 10-bit pixel format selection. |
| `settings-video-prores` | ProRes MOV with 10-bit 4:2:2 pixels | Encoder and pixel format controls, with unused quality and speed controls hidden. |
| `settings-video-vp9` | VP9 WebM with speed 3 and CRF 0 | Native speeds 0–5 and the Best quality endpoint. |
| `settings-video-hardware` | Unavailable selected NVENC; unavailable strict VideoToolbox and available software fallback | Preserved selection, explicit hardware policy captions and the fixed-width Check again action. Open Video encoder to inspect both policies. |
| `settings-audio` | Audio settings tab with an audio source and tracks | Audio codec controls, VBR quality, channel selection, volume and normalize controls, track selection rows, and audio-only source treatment. |
| `settings-images` | Image settings tab with a selected PNG source | Image output controls, custom image dimensions, image-source metadata, and non-video settings visibility. |
| `settings-metadata` | Metadata tab with source tags and output metadata drafts | Source metadata presentation, editable metadata fields, long value wrapping, and tag/value alignment. |
| `settings-subtitles` | Subtitles tab with selectable sidecars, source tracks, and burn-in styling | External subtitle rows and metadata editor, default/forced states, source track rows, burn-in file state, font controls, color swatches, outline color, position controls, and selected track state. |
| `settings-subtitles-popover` | Subtitles tab with the font color picker open | Color picker popover placement, swatch state, HSV draft color, popover layering, and focus treatment inside the settings panel. |
| `settings-presets` | Presets tab with a custom preset draft | Preset list rendering, custom preset row, draft preset name, action buttons, and preset form spacing. |
| `update-available` | Update dialog with release notes and a platform asset | Update dialog layout, release notes markdown rendering, scroll behavior, close animation, footer actions, and platform-specific asset copy. |
| `workspace-empty` | Empty workspace | Welcome/import screen, empty queue layout, primary and secondary import actions, and first-run visual balance. |
| `workspace-audio` | Workspace with a selected audio source | Queue row for audio media, audio metadata display, preview controls hidden for audio-only sources, and audio conversion defaults. |
| `workspace-image` | Workspace with a selected image source | Queue row for image media, image metadata display, image preview state, and image conversion defaults. |

## Maintenance Checklist

When adding or changing a visual fixture:

1. Add the enum case in `VisualFixture`.
2. Add the environment key in `visual_fixture_from_env_value`.
3. Seed the state in `FrameRoot::apply_visual_fixture`.
4. Add or update focused coverage under `frame-app/src/app/tests/` or beside the
   owning UI component.
5. Update this document with the fixture key and the scenario it covers.

Keep fixture data realistic enough to exercise layout pressure: long labels,
multiple tracks, release notes, logs, or metadata values should be included when
the target UI needs wrapping, scrolling, or dense row treatment.
