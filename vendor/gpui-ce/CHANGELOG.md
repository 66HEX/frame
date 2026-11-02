# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). This project somewhat adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html), aside from the major version being 0.

Items are sorted by issue or pull request number.

## [Unreleased]

### Highlights

(only filled out when preparing the release)

- MSRV bumped from `1.92.0` to `1.95.0`

### Breaking Changes

- `AppContext` refactored into subdomain traits. [#127](https://github.com/gpui-ce/gpui-ce/pull/127),
  - `App::notify()` -> `AppContext::notify()`
  - `App::emit()` -> `AppContext::emit()`
  - `App::insert_global_entity()` -> `AppContext::insert_global_entity()`
  - `App::remove_global_entity()` -> `AppContext::remove_global_entity()`
  - `App::global_entities()` -> `AppContext::global_entities()`
- Replaced `DefiniteLength::Fraction(f32)` with `DefiniteLength::Relative(Relative)`. [#230](https://github.com/gpui-ce/gpui-ce/pull/230), [#255](https://github.com/gpui-ce/gpui-ce/pull/255)
- Removed `Deferred::with_priority()`, use `Deferred::priority()`. [#245](https://github.com/gpui-ce/gpui-ce/pull/245)
- Types `T` changed from `Hsla` to `Background` [#252](https://github.com/gpui-ce/gpui-ce/pull/252)
  - `BoxShadow::color: T`
  - `Style::border_color: Option<T>`
  - `RingColor::Color(T)`, `RingColor::resolve() -> T`
  - `PaintQuad::border_color: T`
  - `PaintQuad::border_color(self, impl IntoColor<T>)` -> `PaintQuad::border_color(self, impl Into<T>)`
  - `quad(.., border_color: impl IntoColor<T>) -> PaintQuad` -> `quad(.., border_color: impl Into<T>) -> PaintQuad`
  - `outline(.., border_color: impl IntoColor<T>) -> PaintQuad` -> `outline(.., border_color: impl Into<T>) -> PaintQuad`
  - Additional changes to `gpui_render` and `gpui_macros` not enumerated
- `AssetRegistry` API. [#259](https://github.com/gpui-ce/gpui-ce/pull/259)
  - `Application::with_assets(impl AssetSource)` -> `Application::with_assets(impl Into<AssetRegistry>)`
  - `App::asset_source() -> &Arc<dyn AssetSource>` -> `App::assets() -> &Arc<AssetRegistry>`
  - `SvgRenderer::new(Arc<dyn AssetSource>)` -> `SvgRenderer::new(Arc<AssetRegistry>)`
- Switched text system to [parley](https://github.com/linebender/parley). [#267](https://github.com/gpui-ce/gpui-ce/pull/267)
- Renamed `Window::activate_window()` to `Window::activate()`. [#272](https://github.com/gpui-ce/gpui-ce/pull/272)
- Smooth corner support [#338](https://github.com/gpui-ce/gpui-ce/pull/338)
  - Added `PaintQuad::corner_smoothing: f32`
  - Removed `Window::pain_drop_shadows_with_corner_smoothing` & Added `corner_smoothing: f32` to `Window::pain_drop_shadows`
  - Removed `Window::paint_inset_shadows_with_corner_smoothing` & Added `corner_smoothing: f32` to `Window::paint_inset_shadows`
  - Removed `Window::paint_backdrop_filter_with_corner_smoothing` & Added `corner_smoothing: f32` to `Window::paint_backdrop_filter`
  - Removed `Window::with_filter_layer_with_corner_smoothing` & Added `corner_smoothing: f32` to `Window::with_filter_layer`
  - Removed `Window::paint_quad_with_corner_smoothing` & Added `corner_smoothing: f32` to `Window::paint_quad`
  - Removed `Window::paint_image_with_corner_smoothing` & Added `corner_smoothing: f32` to `Window::paint_image`

### Additions

Changelog established.

- Declarative style transitions API. [#125](https://github.com/gpui-ce/gpui-ce/pull/125) [#231](https://github.com/gpui-ce/gpui-ce/pull/231)
  - `InteractiveElement::transitions(..)`
  - struct `Animated<T, Time>`, `Progress`, `Easing`, `MotionSample`, `Motion`
  - enum `Repeat`
  - const fns `secs(u64) -> std::time::Duration`, `millis(u64) -> std::time::Duration`
  - `impl Lerp` for `palette::Oklab`, `palette::Oklaba`, `palette::Hsla`, `AbsoluteLength`, `DefiniteLength`, `Length`, `Background`, `Fill`
  - trait `DurationWithEasing` for `std::time::Duration`
  - `Window::use_transition` and `Window::use_keyed_transition` argument `duration: std::time::Duration` replaced with `motion: impl Into<Motion>`
  - Further expanded by [#281](https://github.com/gpui-ce/gpui-ce/pull/281) [#284](https://github.com/gpui-ce/gpui-ce/pull/284)
- `StatefulInteractiveElement::on_prepaint()`. [#129](https://github.com/gpui-ce/gpui-ce/pull/129)
  - struct `InteractivityPrepaint` for the callback argument
- `Size::at_anchor()` (extracted functionality from `Bounds::from_anchor_and_size()`). [#130](https://github.com/gpui-ce/gpui-ce/pull/130)
- `InteractiveElement::on_hover()` no longer triggers `debug_assert` when called multiple times on the same element. [#131](https://github.com/gpui-ce/gpui-ce/pull/131)
- Window icon support for Wayland windowing platform. [#133](https://github.com/gpui-ce/gpui-ce/pull/133)
- Unified renderer shaders using WGSL and added opt-in macOS WGPU renderer. [#215](https://github.com/gpui-ce/gpui-ce/pull/215) [#287](https://github.com/gpui-ce/gpui-ce/pull/287)
  - `SurfaceSupport::WindowsCapture`, `SurfaceSupport::Unsupported`
- Tailwind-style ring API. [#216](https://github.com/gpui-ce/gpui-ce/pull/216) [#231](https://github.com/gpui-ce/gpui-ce/pull/231)
  - enum `RingColor`, struct `RingStyle`
  - `Styled::ring(impl Into<Pixels>)`
  - `Styled::ring_color(impl IntoColor<Hsla>)`
  - `Styled::inset_ring(impl Into<Pixels>)`
  - `Styled::inset_ring_color(impl IntoColor<Hsla>)`
- Customizable caret width & height for `EditableText`. [#218](https://github.com/gpui-ce/gpui-ce/pull/218)
  - `EditableText::placeholder_color(color: Hsla)` argument is now `impl IntoColor<Hsla>`
  - `EditableText::caret_w(impl Into<Pixels>)`
  - `EditableText::caret_h(impl Into<Pixels>)`
- Reworked drag-drop API [#227](https://github.com/gpui-ce/gpui-ce/pull/227), [#270](https://github.com/gpui-ce/gpui-ce/pull/270), [deprecation](https://github.com/gpui-ce/gpui-ce/commit/c818c3a1c0af68d1d45aa42654b5752903c7f51e)
  - struct `DropListener<T>` (owns drop callbacks)
  - `InteractiveElement::on_drag_alt()`
  - `InteractiveElement::on_drag_over()`
  - `InteractiveElement::on_drag_over_any()`
  - `InteractiveElement::on_drop_alt(DropListener<T>)`
  - `InteractiveElement::on_drop()` is now an alias for `InteractiveElement::on_drop_alt()`
  - `App::active_drag() -> Option<&AnyDrag>`, `App::start_drag(AnyDrag)`, `App::stop_drag(AnyDrag)`, `App::stop_active_drag(Window) -> bool`
  - `AnyDrag::new()`, `AnyDrag::offset()`, `AnyDrag::cursor_style()`, `AnyDrag::external_payload()`, `AnyDrag::is_type<T>()`
- Aria support for disabled/modal/live/live_atomic. [#233](https://github.com/gpui-ce/gpui-ce/pull/233)
  - `StatefulInteractiveElement::aria_disabled()`
  - `StatefulInteractiveElement::aria_modal()`
  - `StatefulInteractiveElement::aria_live()`
  - `StatefulInteractiveElement::aria_live_atomic()`
- Corner smoothing API. [#235](https://github.com/gpui-ce/gpui-ce/pull/235) [#338](https://github.com/gpui-ce/gpui-ce/pull/338)
  - `Styled::rounded_smoothing(f32)`
  - `Styled::rounded_smoothing_ios()` alias for `Styled::rounded_smoothing(0.6)`
  - `corner_smoothing` arguments to many structs and paint functions
- Allow elements (& descendants) to be excluded from the accessibility tree. [#234](https://github.com/gpui-ce/gpui-ce/pull/234)
  - `StatefulInteractiveElement::aria_hidden()`
  - `Element::is_a11y_hidden() -> bool`
- Surface API for custom shaders. [#237](https://github.com/gpui-ce/gpui-ce/pull/237)
- `Styled::whitespace(WhiteSpace)` alias for `Styled::text_style().white_space = Some(WhiteSpace)`. [#243](https://github.com/gpui-ce/gpui-ce/pull/243)
- `Window::set_visible(bool)`. [#239](https://github.com/gpui-ce/gpui-ce/pull/239)
- `InteractiveElement::on_mouse_move_all()`. [#244](https://github.com/gpui-ce/gpui-ce/pull/244)
- `AssetRegistry` API. [#259](https://github.com/gpui-ce/gpui-ce/pull/259)
  - `AssetSource::iter_preloaded`, `AssetSource::iter_ondemand`
  - `AssetRegistry` & `AssetEntry` types and related functions/types
- `Deferred` elements support auto-incrementing priority when nested. [#245](https://github.com/gpui-ce/gpui-ce/pull/245)
  - enum `DeferredPriority`
  - `Deferred::priority_auto()`, `Deferred::content_mask()`
- Expose `HitTest` struct and information about the most recent hit test. [#247](https://github.com/gpui-ce/gpui-ce/pull/247)
  - Added struct `HitTestEntry`
  - Added `Hitbox::tags: Vec<SharedString>`
  - `HitboxBehavior::blocks_mouse() -> bool`
  - `HitboxBehavior::occludes() -> bool`
  - `Window::insert_hitbox_mut() -> &mut Hitbox`
  - `Window::mouse_hit_test() -> &HitTest`
  - Bump MSRV from `1.92.0` to `1.95.0` to support `Window::insert_hitbox_mut` via `Vec::push_mut()`
- Gradient support for borders and shadows. [#252](https://github.com/gpui-ce/gpui-ce/pull/252)
- Customizable dashed borders API. [#254](https://github.com/gpui-ce/gpui-ce/pull/254)
  - `Quad`, `Style`, `PaintQuad` properties
    - `border_dashed_length: f32`
    - `border_dashed_gap: f32`
  - `Style`, `PaintQuad` methods
    - `border_dashed_length(f32)`
    - `border_dashed_gap(f32)`
- Bidirectional text support. [#263](https://github.com/gpui-ce/gpui-ce/pull/263), [#266](https://github.com/gpui-ce/gpui-ce/pull/266)
  - `enum LayoutDirection { Inherit, LeftToRight, RightToLeft, Auto }`
  - `enum UnicodeBidi { Normal, Embed, Isolate, BidiOverride, IsolateOverride, Plaintext }`
  - `Style` members
    - `direction: LayoutDirection`
    - `unicode_bidi: UnicodeBidi`
    - `unicode_bidi_explicit: bool`
  - `Styled` API
    - `Styled::direction(LayoutDirection)` alias for `Styled::style().direction = Some(LayoutDirection)`
    - `Styled::rtl()` alias for `Style::direction(LayoutDirection::RightToLeft)`
    - `Styled::ltr()` alias for `Style::direction(LayoutDirection::LeftToRight)`
    - `Styled::text_start()` alias for `Styled::text_align(TextAlign::Start)`
    - `Styled::text_end()` alias for `Styled::text_align(TextAlign::End)`
    - `Styled::justify_flex_start()` alias for `Styled::style().justify_content = Some(JustifyContent::Start)`
    - `Styled::justify_flex_end()` alias for `Styled::style().justify_content = Some(JustifyContent::End)`
    - `Styled::unicode_bidi(UnicodeBidi)`
- Allow accessibility tree to be forcibly generated without a screen reader. [#274](https://github.com/gpui-ce/gpui-ce/pull/274)
  - `Application::with_accessibility_forced()`
  - `Window::set_a11y_forced(bool)`
- Element inline layouts. [#262](https://github.com/gpui-ce/gpui-ce/pull/262)
  - `enum VerticalAlign { Baseline, Middle, Top, Bottom }`
  - `Display::Inline`, `Display::InlineFlex`
  - `Styled::display(Display)`, `Styled::inline()`, `Styled::inline_flex()`
  - `Styled::vertical_align(VerticalAlign)`, `Styled::align_baseline()`, `Styled::align_top()`, `Styled::align_middle()`, `Styled::align_bottom()`
  - `Hitbox::contains(&Point<Pixels>) -> bool`
- API for inspecting the most recently build accessibility tree. [#279](https://github.com/gpui-ce/gpui-ce/pull/279)
  - `Window::a11y_tree() -> Option<&accesskit::TreeUpdate`
  - `Window::a11y_node_bounds(accesskit::NodeId) -> Option<Bounds<Pixels>>`
  - `Window::a11y_frame_number() -> u64`
- Hot Reloading via `hot-patching` feature flag. [#292](https://github.com/gpui-ce/gpui-ce/pull/292)
- Trigger accessibility actions programmatically. [#293](https://github.com/gpui-ce/gpui-ce/pull/293)
  - `Window::perform_a11y_action(..)`
- Window activation via platform token on Wayland. [#272](https://github.com/gpui-ce/gpui-ce/pull/272)
  - `Window::activate_with_token(&str)`
- Overflow fade-clipping. [#298](https://github.com/gpui-ce/gpui-ce/pull/298)
- `WindowOptions` is now a builder. [#299](https://github.com/gpui-ce/gpui-ce/pull/299) [#301](https://github.com/gpui-ce/gpui-ce/pull/301)
- `Styled::refine_style(&StyleRefinement)` alias for `Styled::style().refine(StyleRefinement)`. [#302](https://github.com/gpui-ce/gpui-ce/pull/302)
- Convenience derive macro's for `Styled`, `ParentElement`, `InteractiveElement`, `StatefulInteractiveElement`. [#300](https://github.com/gpui-ce/gpui-ce/pull/300)
- `Window::show_character_palette()` implementation on Windows and Linux. [#308](https://github.com/gpui-ce/gpui-ce/pull/308)
- Variable font width support. [#316](https://github.com/gpui-ce/gpui-ce/pull/316)
  - struct `FontWidth(f32)`
  - `Style::font_width: FontWidth`
  - `HighlightStyle::font_width: Option<FontWidth>`
  - `Styled::font_width(impl Into<FontWidth>)`
- Directional resize cursors. [#322](https://github.com/gpui-ce/gpui-ce/pull/322)
  - `CursorStyle` variants `ResizeUpLeft`, `ResizeUpRight`, `ResizeDownLeft`, `ResizeDownRight`
  - `InteractiveElement::cursor_with()`, `InteractiveElement::on_hover_region()`
  - struct `ResizeRegion`
  - `Window::set_cursor_style_with()`
  - `Window::set_window_cursor_style_with()`
- `BaseSwitch` element. [#329](https://github.com/gpui-ce/gpui-ce/pull/329)
- `image-default-formats` feature flag, which enables [`default-formats` feature flag](https://docs.rs/crate/image/latest/features#default-formats) on `image` crate dependency. [#331](https://github.com/gpui-ce/gpui-ce/pull/331)
  - gpui now defaults to only enabling image feature flags: `png`, `gif`, `webp`
- `PaintQuad::new` and property-building functions. [#338](https://github.com/gpui-ce/gpui-ce/pull/338)
- Elements can be disabled. [#339](https://github.com/gpui-ce/gpui-ce/pull/339)
  - `InteractiveElement::disabled()`
  - `InteractiveElement::disabled_style()`
- Rendering rehauled to support WGPU/WGSL renderer. [#215](https://github.com/gpui-ce/gpui-ce/pull/215)
  - Added feature flags `gpui_apple/runtime_shaders`, `gpui_macos/runtime_shaders`, `gpui_platform/runtime_shaders`
  - Linux/FreeBSD screen capture was incorporated at some point.
- `gpui/wayland` and `gpui/x11` now toggle the corresponding `gpui_platform` flags, so Linux builds can target either backend or both. [#373](https://github.com/gpui-ce/gpui-ce/issues/373), [#379](https://github.com/gpui-ce/gpui-ce/pull/379)

### Removals

- `WindowOptions::window_background: WindowBackgroundAppearance`, replaced by: [#299](https://github.com/gpui-ce/gpui-ce/pull/299)
  - `WindowOptions::macos_window_background: MacosWindowBackground`
  - `WindowOptions::windows_window_background: WindowsWindowBackground`
  - `WindowOptions::linux_window_background: LinuxWindowBackground`
  - `WindowOptions::web_window_background: WebWindowBackground`
  - Added `WindowOptions::background_appearance()`

### Fixes

- `EditableText` selection remaining visible after losing focus. [#250](https://github.com/gpui-ce/gpui-ce/issues/250), [#267](https://github.com/gpui-ce/gpui-ce/pull/267)
- Proportional pixel snapping for fractional padding on automatically sized axes. [#217](https://github.com/gpui-ce/gpui-ce/pull/217)
- Linux-to-macOS cross-compilation in `gpui_media`. [#232](https://github.com/gpui-ce/gpui-ce/pull/232)
- Duplicate characters during web IME composition. [#236](https://github.com/gpui-ce/gpui-ce/pull/236)
- Element opacity now applies to all descendants as one group. [#330](https://github.com/gpui-ce/gpui-ce/pull/330)
- Incorrect RTL glyph ordering in `EditableText`. [#267](https://github.com/gpui-ce/gpui-ce/pull/267)
- `ColorExt::blend` diverged from upstream/original behavior [#327](https://github.com/gpui-ce/gpui-ce/issues/327), [#357](https://github.com/gpui-ce/gpui-ce/pull/357)

### Security Changes

### Deprecations

- [`quad() -> PaintQuad`, `fill() -> PaintQuad`, `outline() -> PaintQuad`] -> `PaintQuad::new` builder api. [#338](https://github.com/gpui-ce/gpui-ce/pull/338), [restore to deprecation](https://github.com/gpui-ce/gpui-ce/commit/e41c444fcb288f537ef9968fefcb4c0f9be68330)
- `Application::run_embedded` -> (existing) `Application::run`. [#364](https://github.com/gpui-ce/gpui-ce/pull/364)
- `InteractiveElement` functions suffixed with `_style` [#339](https://github.com/gpui-ce/gpui-ce/pull/339)
  - `InteractiveElement::hover()` -> `gpui::InteractiveElement::hover_style()`
  - `InteractiveElement::group_hover()` -> `gpui::InteractiveElement::group_hover_style()`
  - `InteractiveElement::group_drag_over()` -> `gpui::InteractiveElement::group_drag_over_style()`
  - `InteractiveElement::focus()` -> `gpui::InteractiveElement::focus_style()`
  - `InteractiveElement::in_focus()` -> `gpui::InteractiveElement::in_focus_style()`
  - `InteractiveElement::focus_visible()` -> `gpui::InteractiveElement::focus_visible_style()`
  - `InteractiveElement::active()` -> `gpui::InteractiveElement::active_style()`
  - `InteractiveElement::group_active()` -> `gpui::InteractiveElement::group_active_style()`
- `InteractiveElement::drag_over()` -> `InteractiveElement::on_drag_over()`. [#227](https://github.com/gpui-ce/gpui-ce/pull/227), [deprecation](https://github.com/gpui-ce/gpui-ce/commit/c818c3a1c0af68d1d45aa42654b5752903c7f51e)

### Contributors

Thanks to the people who made this release happen!

- Björn Harrtell (@bjornharrtell)
- Calin P (@wyvernbw)
- Cameron Campbell (@cameronpcampbell)
- Christopher Addison (@chris-addison)
- David Acevedo (@dacevedo12)
- Domen Kožar (@domenkozar)
- Dustin Yost (@temportalflux)
- François (@francois-unity)
- Jayson Saavedra (@saavy1)
- Kayla Fortin (@kaylafortin)
- Miles Wirht (@philocalyst)
- Sabin Regmi (@wheregmis)
- Sampo Kivistö (@Havunen)
- Skye Ibrahim (@DeSitterUniverse)
- Takashi Fujita (@tgfjt)
- William Whittaker (@143mailliw)
- Zachary Freed (@FuZzledd)

## [0.2.2] - 2026-08-28

_Existed before changelog._
