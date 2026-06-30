use super::{
    FluentBuilder, InteractiveElement, ParentElement, Styled, color, div, px, relative, theme,
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
    fraction: f32,
    disabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .relative()
        .h(px(FRAME_SLIDER_VISUAL_HEIGHT))
        .w_full()
        .opacity(if disabled { 0.5 } else { 1.0 })
        .when(!disabled, gpui::Styled::cursor_pointer)
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(FRAME_SLIDER_TRACK_TOP))
                .h(px(FRAME_SLIDER_TRACK_HEIGHT))
                .rounded(px(FRAME_SLIDER_TRACK_RADIUS))
                .bg(color(theme::FRAME_GRAY_100)),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .top(px(FRAME_SLIDER_TRACK_TOP))
                .h(px(FRAME_SLIDER_TRACK_HEIGHT))
                .w(relative(fraction.clamp(0.0, 1.0)))
                .rounded(px(FRAME_SLIDER_FILL_RADIUS))
                .bg(color(theme::FOREGROUND)),
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
        .top(px(FRAME_SLIDER_HANDLE_TOP))
        .ml(px(-(FRAME_SLIDER_HANDLE_WIDTH / 2.0)))
        .w(px(FRAME_SLIDER_HANDLE_WIDTH))
        .h(px(FRAME_SLIDER_HANDLE_HEIGHT))
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
