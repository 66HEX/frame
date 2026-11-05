# Frame's GPUI-CE vendor snapshot

Upstream: [gpui-ce/gpui-ce](https://github.com/gpui-ce/gpui-ce).
This is a tracked source copy, not a submodule. Frame's root `Cargo.toml` patches
its Git dependencies to this directory.

- Current upstream base: [`bc577476a0eb55173ef985585cfb487f75ab1cab`](https://github.com/gpui-ce/gpui-ce/commit/bc577476a0eb55173ef985585cfb487f75ab1cab), the `main` tip checked on 2026-11-02.
- Previous upstream base: `ce08af96db6c3474ab3b388325cdd1a7c3de06d9`.
- Original import in Frame: `affde69b65c3d0428bfb0848b7dbfb7c843d7ba5` (2026-06-29).
- Downstream history audited through Frame commit `ff037aa11df61adc3c97259660a1c186688ffe1f`.

The original base is recorded in the parent of the import commit's `Cargo.lock`.
Comparing the old vendor tree to that base includes the GPU changes made during
the import itself; looking only at later vendor commits misses them.

## Preserved downstream changes

Commit references below belong to the Frame repository.

| History | Behavior | Current implementation |
| --- | --- | --- |
| `affde69` | GPU preview rotation and flips | `Window::paint_image_transformed`, the scene sprite matrix and its ABI, and shared Rust/WGSL sprite shaders. The old separate Metal, HLSL and WGSL patches now share one implementation. Clipping stays in window coordinates; rounded corners stay in image coordinates. |
| `6f3184b` | Controls above titlebar areas do not initiate a window move | Window-control hit testing uses the topmost hitbox. Tests cover both a covering control and an exposed titlebar area. |
| `56543d9`, `940f7d8`, `6744eac` | Linux client-side rounded frames; delayed titlebar dragging | Frame options and X11/Wayland state retain the corner radius. WGPU clips the final composite, disables rounding for maximized/fullscreen/tiled windows, and retains the setting across device recovery. The reverted immediate mouse-down native move is **not** restored. Frame still starts dragging after pointer motion. |
| `d27bfbd` | Keyboard focus rings, descendant focus styling and accessible input metadata | `contains_focus` and `aria_invalid` remain downstream. Upstream now provides `RingStyle`, `aria_description`, `aria_disabled` and `aria_value`. Frame uses the upstream non-layout ring API. Pointer motion restores hover while retaining keyboard focus visibility; a pointer click clears the ring. |
| `335b1f1`, `8d680f4` | Reuse video atlas tiles while refreshing pixels | Versioned `RenderImage`, `PlatformAtlas::get_or_update_with`, and Metal/WGPU/DirectX uploads. Equal versions skip work; equal dimensions reuse allocations; resize/removal invalidate the cache. Invalid upload bytes preserve the last valid version. Upstream glyph-cache cleanup and allocator deallocation remain intact. |
| `21f2e10` | Action trait lint compatibility | Retained in `gpui/src/action.rs`. |
| `0b8bc59`, `9ac1ec4`, `2ccdb5d`, `7fb9d89`, `ee37ff0` | Compatibility with dependency upgrades | Syn 3 receiver handling, Rand 0.10 imports, CoreVideo 0.6 image-buffer conversion in both Apple render paths, and Windows 0.62 interfaces. |
| `04388d0`, `899aeac`, `373cdb2`, `8ed02cf`, `d3afaef`, `6b14eb8`, `56a5c04`, `02ec4af` | Dependency and supply-chain maintenance | Retain the newer AccessKit, itertools, strum, mach2, pollster, Windows and CoreVideo versions. Both lockfiles retain async-executor 1.14.1. Dependencies replaced upstream are not reintroduced. |

The toolbar fixes from `77dff1d` live in Frame's application code, outside this
vendor directory. Their movement threshold and control event isolation remain.
Frame also sets upstream's `app_owns_titlebar_drag` option to avoid native AppKit
interception, while retaining its existing `is_movable` policy. Once a blank-area
press arms a drag, Frame handles the first pointer move even if a native event
batch carries it beyond the titlebar. Buttons still cannot arm that gesture.

## Subtree animation scale

`Window::with_paint_scale` and `with_prepaint_scale` support Frame's Mira enter/exit zoom without reflowing text. Scene operations retain untransformed primitives and explicit scale scopes so cached replay and nested scopes do not double-transform. Quads, glyph/image sprites, paths, rings, shadows, filters and pointer hitboxes follow the same origin. Ancestor clipping remains in window coordinates. The scene regression test covers nested scaling, scope restoration and cached replay under a new scale. Frame's integration test exercises opening/closing all six selects and both color pickers with motion enabled.

## Integration changes

Upstream renamed Cargo packages to `gpui-ce` / `gpui_ce_*`, moved Apple's renderer
to `gpui_apple`, introduced shared shader generation in `gpui_render`, and replaced
the text backend with Parley. Frame explicitly enables both X11 and Wayland,
which upstream now disables by default. Frame adapts package aliases, explicit
color conversion, text caret geometry, paint builders, style methods and deferred
priority. Asset enumeration now exposes full paths from the root for upstream's
`AssetRegistry`; a regression test checks every bundled icon and font. The main
content has an explicit zero minimum height so long settings pages cannot push
the preview timeline and queue below the window; a rendered custom-crop test
checks their bounds. The crop keyboard test now sends both Enter key-down and key-up, as a
real keyboard does; upstream deliberately rejects an unmatched key-up.

The pinned shader generator is
[`schell/wgsl-rs` at `b2de16a`](https://github.com/schell/wgsl-rs/tree/b2de16a1b81e67d76bc265c296184f882864badb).
Its crate manifests omit license metadata; `deny.toml` verifies the workspace's
MIT license file by hash. Removed Git sources and obsolete advisory exceptions
were removed from that configuration.

### CSS shadow clipping (2026-11-03)

The shared shadow shader clips outer shadows out of the element's rounded border
box. This prevents multilayer shadows from darkening translucent control fills.
Both ordinary and smoothed-corner paths use the element coverage; inset shadows
keep their inner clipping. Tests cover clear interiors, outside edges, rounded
corners, blur radii from 0 to 42.5, and preserved inset highlights. No shader ABI
or platform-specific rendering code changes.

## Regression checks

From Frame's repository root:

```sh
cargo xtask ci
cargo deny check advisories licenses sources
cargo build --locked -p frame-app --bin frame
cargo test --manifest-path vendor/gpui-ce/Cargo.toml -p gpui-ce --lib
cargo test --manifest-path vendor/gpui-ce/Cargo.toml -p gpui_ce_apple --lib
cargo test --manifest-path vendor/gpui-ce/Cargo.toml -p gpui_ce_render -p gpui_ce_wgpu --features gpui_ce_wgpu/test-support --lib
```

`tests/frame_preview.rs` is shared by Metal and WGPU pixel tests. It checks rotation,
reflection, rounded and smoothed corners, viewport clipping and new pixels in a
reused tile. Backend atlas tests cover caching, invalid uploads, resizing and
removal. WGPU also tests rounded frame corners and switching back to a square
frame. Core tests cover titlebar occlusion, accessibility metadata and input
modality.

On this Mac, upstream's WGPU test
`quad_border_backgrounds_use_the_fill_coordinate_space` fails its two-level color
tolerance: red is 228 instead of 225 at (10, 10). The identical failure was reproduced
in the unmodified `bc57747` checkout. Its assertion is retained unchanged.

Windows can be type-checked from macOS with
`GPUI_RENDER_ALLOW_MISSING_DXBC=1 cargo check --locked -p frame-app --target x86_64-pc-windows-gnu`.
This upstream option does not produce a runnable Windows binary; native Windows
builds must compile DXBC normally.

Validation on 2026-11-02:

- Frame's `cargo xtask ci` passes: 954 tests, Clippy with warnings denied,
  formatting, shell syntax, workflow consistency and whitespace checks.
- GPUI core: 413 tests pass; Metal: 15 pass; shared shaders: 30 pass;
  WGPU: 36 pass, with the single upstream failure described above.
- `cargo deny check advisories licenses sources` passes.
- Windows Frame type-check passes with the upstream DXBC opt-out. Linux GPUI
  type-check passes for `x86_64-unknown-linux-musl` with both `wayland,x11` enabled.
  Neither is a native runtime check on those platforms.
- The native macOS review build restores the previous crop layout and icons.
  Checked the ratio menu, typed custom width and live resolution synchronization,
  keyboard focus, light settings at 200% scale, and dark crop at 100%. Native
  drag events move blank titlebar space and leave the window stationary when
  initiated on Workspace, Settings or Add source controls.

Before releasing, complete the broader native checklist:

1. Open video, audio and image files. Play, pause, seek, zoom and switch files;
   check that frames update without stale textures or flicker.
2. Rotate through all four orientations and toggle both flips. Edit/apply/cancel
   a crop, including custom ratio and pixel inputs; check preview and export.
3. Open selects, tooltips, color picker, logs and dialogs in both themes at 100%
   and 200% UI scale. Check text, icons, shadows, focus rings and animations.
4. Tab into controls, move the pointer, then click. Check both hover feedback and
   keyboard focus. Type, select, paste and scroll long text, including Polish
   characters and emoji; check timecode editing and native IME positioning.
5. Drag empty titlebar space, then drag toolbar buttons/tabs. Only empty space
   should move the window. Check the native titlebar double-click action.
6. On Linux, check X11 and Wayland floating, maximized, tiled and fullscreen
   windows. On Windows, repeat preview transformations and playback with DirectX.

### Development performance follow-up (2026-11-03)

CPU sampling of full native redraws found most time in Taffy's layout code,
including with playback tasks stopped. The video settings tree was much more
expensive than the empty workspace. Frame's development profile now optimizes
`gpui-ce` and `taffy` at level 3 and `gpui_ce_parley` at level 2; GPUI also needs
optimization because it instantiates Taffy's generic code. Frame's own code
retains the default debug profile. No runtime behavior or dependency versions
change in this follow-up.

Measured median CPU frame times on the same Mac at 1200 × 800, dark theme,
100% scale:

| Fixture | Unoptimized development build | Selectively optimized development build |
| --- | --- | --- |
| `workspace-empty` | 0.85 ms | 0.14 ms |
| `workspace-image` | 43.82 ms | 4.50 ms |
| `settings-source` | 52.15 ms | 5.35 ms |
| `settings-video` | 127.51 ms | 13.01 ms |
| `settings-output` | 40.54 ms | 4.30 ms |
| `preview-ready` | 52.98 ms | 5.34 ms |
| `preview-crop-custom` | 132.91 ms | 13.39 ms |

The final build measured 120 frames after 20 warm-up frames. These numbers
exclude playback and GPU presentation; they are not end-to-end FPS measurements.
The native benchmark and its limits are documented in
[`CONTRIBUTING_VISUAL_FIXTURES.md`](../../CONTRIBUTING_VISUAL_FIXTURES.md#cpu-render-benchmark).
Frame's full `cargo xtask ci` passes with the new profile: 954 tests and all
formatting, Clippy, shell, workflow and whitespace checks.

### UI acceptance and merge validation (2026-11-03)

Frame's follow-up UI changes unify select/picker transitions, honor system reduced
motion (including preview zoom/pan), move focus with active tabs, and enlarge
slider, scrollbar and editing-handle hit areas without overlapping neighbors.
These application changes do not add further downstream GPUI patches.

- The final local `cargo xtask ci` passes: 973 tests, formatting, Clippy with
  warnings denied, shell syntax, generated workflows and whitespace checks.
  Existing opt-in media/audio tests remain outside the default local gate.
- Rendered GPUI tests cover all six selects and both color pickers in both themes
  at 100%/200%, repeated tab navigation, disappearing tabs, live motion changes,
  hit-area drags, disabled sliders and tightly spaced handles.
- 40 further native macOS render scenarios pass. Video-settings median CPU draw
  time is 13.2–14.1 ms at 100% and 16.7–16.8 ms at 200%; custom crop is
  13.5 ms and 16.9 ms respectively. These are CPU timings, not playback FPS.
- The repository owner confirmed all manual tests passed on the macOS review
  build and authorized merging the GPUI update and UI fixes. Native Windows and
  Linux runtime acceptance is not claimed.
- A fresh `cargo deny check` passes. The known upstream WGPU color-tolerance
  failure documented above remains unchanged.

## Next update

1. Record the new upstream SHA and preserve a copy of the working vendor tree.
2. Diff this tree against `bc57747` to isolate downstream patches. Replay those
   changes onto the new upstream, adapting moved code instead of restoring old
   files wholesale. Include this document and the shared Frame regression tests.
3. Update Git revisions/package aliases and both lockfiles. Recheck the dependency
   audit; do not restore removed/yanked dependencies from an upstream lockfile.
4. Run the checks above and native platform tests. Compare any upstream test
   failure against an unmodified upstream checkout before attributing it to Frame.
5. Update this file and Frame's changelog with the new base and patch dispositions.
