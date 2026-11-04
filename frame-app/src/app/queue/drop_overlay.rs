use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{ParentElement, Styled};
use gpui::{Context, ExternalPaths, InteractiveElement, IntoElement, Window, div};

pub(in crate::app) fn drag_drop_overlay(
    is_open: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let progress = surface_motion("drag-drop-overlay-motion", is_open, window, cx);

    if !is_open && motion_is_hidden(progress) {
        cx.defer_in(window, |root, _window, cx| {
            if root.finish_drag_drop_overlay_close() {
                cx.notify();
            }
        });
    }

    div()
        .id("drag-drop-overlay")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(color(palette.overlay_backdrop.with_alpha(0.60 * progress)))
        .backdrop_blur(theme::ui_rem(4.0 * progress).to_pixels(window.rem_size()))
        .opacity(progress)
        .occlude()
        .on_drop(cx.listener(|root, paths: &ExternalPaths, _window, cx| {
            cx.stop_propagation();
            root.close_drag_drop_overlay();
            FrameRoot::import_source_paths(paths.paths().to_vec(), cx);
            cx.notify();
        }))
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::ui_rem(theme::RADIUS_LG))
                .bg(color(palette.fill_subtle))
                .shadow(theme::shadows::MODAL.shadows())
                .child(
                    div()
                        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                        .text_color(color(palette.text_primary))
                        .child(theme::ui_text("Import source files")),
                ),
        )
}
