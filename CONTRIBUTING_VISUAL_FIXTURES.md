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

Use a clean app window for visual review when possible. Fixtures run at startup
and intentionally override only the state they need for the target scenario.

## Available Fixtures

| Fixture | Visual scenario | What it is useful for checking |
| --- | --- | --- |
| `app-settings` | Runtime app settings sheet | Settings modal layout, concurrency draft value, modal focus treatment, close action, and app-level settings spacing. |
| `logs-active` | Logs tab with an active FFmpeg conversion | Log tab navigation, active file state, FFmpeg log syntax highlighting, progress line wrapping, monospaced text rendering, and scroll density. |
| `preview-ready` | Workspace with a selected ready video source | Preview panel shell, selected video metadata, timeline controls, toolbar visibility, empty frame handling, and source video state. |
| `preview-crop` | Preview panel with crop mode enabled | Crop aspect bar, crop overlay geometry, crop handles, preview toolbar coexistence, and canvas framing while editing crop bounds. |
| `settings-source` | Source settings tab with ready video metadata | File information rows, video stream metadata rows, source tab spacing, and selected source summary. |
| `settings-output` | Output settings tab with a custom output name | Output filename field, container selection state, output tab layout, and long-name alignment. |
| `settings-video` | Video settings tab with custom resolution and CRF mode | Video codec controls, custom width and height inputs, CRF controls, bitrate mode layout, and dense control grouping. |
| `settings-audio` | Audio settings tab with an audio source and tracks | Audio codec controls, VBR quality, channel selection, volume and normalize controls, track selection rows, and audio-only source treatment. |
| `settings-images` | Image settings tab with a selected PNG source | Image output controls, custom image dimensions, image-source metadata, and non-video settings visibility. |
| `settings-metadata` | Metadata tab with source tags and output metadata drafts | Source metadata presentation, editable metadata fields, long value wrapping, and tag/value alignment. |
| `settings-subtitles` | Subtitles tab with subtitle tracks and burn-in styling | Subtitle track rows, burn-in file state, font controls, color swatches, outline color, position controls, and selected track state. |
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
