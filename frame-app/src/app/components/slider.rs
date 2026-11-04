use crate::app::accessibility::apply_accessible_slider;
use crate::app::primitives::color;
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{InteractiveElement, div, relative};

pub(in crate::app) const FRAME_SLIDER_VISUAL_HEIGHT: f32 = super::MIN_POINTER_TARGET;
pub(in crate::app) const FRAME_SLIDER_TRACK_TOP: f32 =
    (FRAME_SLIDER_VISUAL_HEIGHT - theme::RANGE_TRACK_HEIGHT) / 2.0;
pub(in crate::app) const FRAME_SLIDER_HANDLE_WIDTH: f32 = super::MIN_POINTER_TARGET;
pub(in crate::app) const FRAME_SLIDER_HANDLE_HEIGHT: f32 = FRAME_SLIDER_VISUAL_HEIGHT;
pub(in crate::app) const FRAME_SLIDER_HANDLE_TOP: f32 = 0.0;

/// Shared recessed rail for values, trim ranges, and download progress.
pub(in crate::app) fn frame_range_track(
    start: f32,
    end: f32,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let start = start.clamp(0.0, 1.0);
    let end = end.clamp(start, 1.0);
    let radius = theme::RANGE_TRACK_HEIGHT / 2.0;

    div()
        .relative()
        .h(theme::ui_rem(theme::RANGE_TRACK_HEIGHT))
        .w_full()
        .rounded(theme::ui_rem(radius))
        .bg(color(palette.canvas))
        .shadow(theme::shadows::recessed_control())
        .child(
            div()
                .absolute()
                .top(theme::ui_rem(theme::RANGE_TRACK_INSET))
                .bottom(theme::ui_rem(theme::RANGE_TRACK_INSET))
                .left(theme::ui_rem(theme::RANGE_TRACK_INSET))
                .right(theme::ui_rem(theme::RANGE_TRACK_INSET))
                .when(end > start, |this| {
                    this.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(relative(start))
                            .right(relative(1.0 - end))
                            .rounded(theme::ui_rem(radius - theme::RANGE_TRACK_INSET))
                            .bg(color(palette.text_primary)),
                    )
                }),
        )
}

/// Pointer values use the inner rail, so its visible endpoints remain 0 and 100%.
pub(in crate::app) fn frame_slider_content_bounds(
    mut bounds: gpui::Bounds<gpui::Pixels>,
    rem_size: gpui::Pixels,
) -> gpui::Bounds<gpui::Pixels> {
    let inset = theme::ui_rem(theme::RANGE_TRACK_INSET)
        .to_pixels(rem_size)
        .min(bounds.size.width / 2.0);
    bounds.origin.x += inset;
    bounds.size.width -= inset * 2.0;
    bounds
}

pub(in crate::app) fn frame_slider_fraction_from_bounds(
    position: gpui::Point<gpui::Pixels>,
    bounds: gpui::Bounds<gpui::Pixels>,
    rem_size: gpui::Pixels,
) -> f64 {
    let bounds = frame_slider_content_bounds(bounds, rem_size);
    let width = bounds.size.width.as_f32();
    if width <= 0.0 {
        return 0.0;
    }
    f64::from(((position.x - bounds.origin.x).as_f32() / width).clamp(0.0, 1.0))
}

pub(in crate::app) fn frame_slider(
    id: &'static str,
    label: impl Into<String>,
    fraction: f32,
    disabled: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    let clamped_fraction = fraction.clamp(0.0, 1.0);
    let value_percent = f64::from(clamped_fraction) * 100.0;
    let slider = div()
        .id(id)
        .debug_selector(move || id.to_string())
        .relative()
        .h(theme::ui_rem(FRAME_SLIDER_VISUAL_HEIGHT))
        .w_full()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(!disabled, gpui::Styled::cursor_pointer)
        .child(
            frame_range_track(0.0, clamped_fraction, palette)
                .absolute()
                .left_0()
                .right_0()
                .top(theme::ui_rem(FRAME_SLIDER_TRACK_TOP)),
        );

    apply_accessible_slider(
        slider,
        label,
        !disabled,
        value_percent,
        0.0,
        100.0,
        format!("{value_percent:.0}%"),
        palette,
    )
}

pub(in crate::app) fn frame_slider_handle(
    id: &'static str,
    fraction: f32,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let fraction = fraction.clamp(0.0, 1.0);
    let inset_offset = theme::RANGE_TRACK_INSET * 2.0_f32.mul_add(-fraction, 1.0);
    div()
        .id(id)
        .absolute()
        .left(relative(fraction))
        .top(theme::ui_rem(FRAME_SLIDER_HANDLE_TOP))
        .ml(theme::ui_rem(
            inset_offset - FRAME_SLIDER_HANDLE_WIDTH / 2.0,
        ))
        .w(theme::ui_rem(FRAME_SLIDER_HANDLE_WIDTH))
        .h(theme::ui_rem(FRAME_SLIDER_HANDLE_HEIGHT))
        .when(enabled, gpui::Styled::cursor_ew_resize)
}
