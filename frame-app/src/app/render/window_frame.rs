#[cfg(target_os = "linux")]
use crate::app::layout::LINUX_WINDOW_FRAME_INSET;
use crate::theme;
#[cfg(target_os = "linux")]
use gpui::prelude::*;
use gpui::{IntoElement, Window};
#[cfg(target_os = "linux")]
use gpui::{div, px};

#[cfg(target_os = "linux")]
pub(super) fn linux_window_frame(
    root: gpui::Stateful<gpui::Div>,
    window: &Window,
    _palette: &'static theme::ThemePalette,
) -> impl IntoElement {
    let should_draw_frame = matches!(
        window.window_decorations(),
        gpui::Decorations::Client { tiling }
            if !(tiling.top || tiling.right || tiling.bottom || tiling.left)
    );

    if !should_draw_frame {
        return div().size_full().child(root);
    }

    div().size_full().p(px(LINUX_WINDOW_FRAME_INSET)).child(
        root.rounded(px(theme::RADIUS_LG))
            .shadow(theme::shadows::FLOATING.shadows()),
    )
}

#[cfg(not(target_os = "linux"))]
pub(super) fn linux_window_frame(
    root: gpui::Stateful<gpui::Div>,
    _window: &Window,
    _palette: &'static theme::ThemePalette,
) -> impl IntoElement {
    root
}
