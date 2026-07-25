use super::{
    FluentBuilder, InteractiveElement, ParentElement, Styled, apply_accessible_slider, color, div,
    relative, theme,
};

pub(in crate::app) const FRAME_SLIDER_VISUAL_HEIGHT: f32 = 20.0;
pub(in crate::app) const FRAME_SLIDER_TRACK_HEIGHT: f32 = 6.0;
pub(in crate::app) const FRAME_SLIDER_TRACK_TOP: f32 = 7.0;
pub(in crate::app) const FRAME_SLIDER_TRACK_RADIUS: f32 = 1.5;
pub(in crate::app) const FRAME_SLIDER_FILL_RADIUS: f32 = 1.0;
pub(in crate::app) const FRAME_SLIDER_HANDLE_WIDTH: f32 = 20.0;
pub(in crate::app) const FRAME_SLIDER_HANDLE_HEIGHT: f32 = FRAME_SLIDER_VISUAL_HEIGHT;
pub(in crate::app) const FRAME_SLIDER_HANDLE_TOP: f32 = 0.0;

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
        .relative()
        .h(theme::ui_rem(FRAME_SLIDER_VISUAL_HEIGHT))
        .w_full()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(!disabled, gpui::Styled::cursor_pointer)
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(theme::ui_rem(FRAME_SLIDER_TRACK_TOP))
                .h(theme::ui_rem(FRAME_SLIDER_TRACK_HEIGHT))
                .rounded(theme::ui_rem(FRAME_SLIDER_TRACK_RADIUS))
                .bg(color(palette.fill_subtle)),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .top(theme::ui_rem(FRAME_SLIDER_TRACK_TOP))
                .h(theme::ui_rem(FRAME_SLIDER_TRACK_HEIGHT))
                .w(relative(clamped_fraction))
                .rounded(theme::ui_rem(FRAME_SLIDER_FILL_RADIUS))
                .bg(color(palette.text_primary)),
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
    div()
        .id(id)
        .absolute()
        .left(relative(fraction.clamp(0.0, 1.0)))
        .top(theme::ui_rem(FRAME_SLIDER_HANDLE_TOP))
        .ml(theme::ui_rem(-(FRAME_SLIDER_HANDLE_WIDTH / 2.0)))
        .w(theme::ui_rem(FRAME_SLIDER_HANDLE_WIDTH))
        .h(theme::ui_rem(FRAME_SLIDER_HANDLE_HEIGHT))
        .when(enabled, gpui::Styled::cursor_ew_resize)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::float_cmp,
        reason = "Component tests compare exact deterministic layout constants."
    )]

    use super::*;

    #[test]
    fn frame_slider_track_matches_original_svelte_range_height() {
        assert_eq!(FRAME_SLIDER_TRACK_HEIGHT, 6.0);
    }

    #[test]
    fn frame_slider_handle_remains_hit_target_only() {
        assert_eq!(FRAME_SLIDER_HANDLE_WIDTH, 20.0);
        assert_eq!(FRAME_SLIDER_HANDLE_HEIGHT, FRAME_SLIDER_VISUAL_HEIGHT);
    }
}
