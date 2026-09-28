# Visual Fixtures

Frame includes runtime visual fixtures for manual GPUI review. They seed the app
with deterministic state so contributors can inspect UI surfaces without finding
matching media files, waiting for conversions, or triggering update checks.

The source of truth is `frame-app/src/lib.rs` for fixture names and
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

## Available Fixtures

| Fixture | Visual scenario | What it is useful for checking |
| --- | --- | --- |
| `app-settings` | Runtime app settings sheet | Settings modal layout, concurrency draft value, modal focus treatment, close action, and app-level settings spacing. |
| `app-settings-theme-open` | App settings with the Theme select open | Dark/Light option order, selected checkmark, keyboard focus treatment, popover placement, and equal-width Appearance columns. |
| `app-settings-ui-open` | App settings with UI scale open at 200% | Scaled geometry and typography, scrollable sheet body, popover placement, and titlebar coexistence. |
| `logs-active` | Logs tab with an active FFmpeg conversion | Log tab navigation, active file state, plain FFmpeg log rendering, progress line wrapping, monospaced text rendering, and scroll density. |
| `preview-ready` | Workspace with a selected ready video source | Preview panel shell, selected video metadata, timeline controls, toolbar visibility, empty frame handling, and source video state. |
| `preview-crop` | Preview panel with crop mode enabled | Crop aspect bar, crop overlay geometry, crop handles, preview toolbar coexistence, and canvas framing while editing crop bounds. |
| `settings-source` | Source settings tab with ready video metadata | File information rows, video stream metadata rows, source tab spacing, and selected source summary. |
| `settings-output` | Output settings tab with a custom output name | Output filename field, container selection state, output tab layout, and long-name alignment. |
| `settings-video` | Video settings tab with custom resolution and CRF mode | Video codec controls, custom width and height inputs, CRF controls, bitrate mode layout, and dense control grouping. |
| `settings-audio` | Audio settings tab with an audio source and tracks | Audio codec controls, VBR quality, channel selection, volume and normalize controls, track selection rows, and audio-only source treatment. |
| `settings-images` | Image settings tab with a selected PNG source | Image output controls, custom image dimensions, image-source metadata, and non-video settings visibility. |
| `settings-image-sequence` | Image settings tab with a 30-second, 30-fps video targeting JPEG | Single/sequence controls, an estimated 900-frame count, VFR guidance, and sequence size recommendations. |
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
4. Add or update focused coverage in `frame-app/src/app/tests.rs`.
5. Update this document with the fixture key and the scenario it covers.

Keep fixture data realistic enough to exercise layout pressure: long labels,
multiple tracks, release notes, logs, or metadata values should be included when
the target UI needs wrapping, scrolling, or dense row treatment.
