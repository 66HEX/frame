use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::primitives::{color, icon_svg};
use crate::app::root::FrameRoot;
use crate::{assets, theme};
use gpui::prelude::{ParentElement, Styled};
use gpui::{Context, ExternalPaths, InteractiveElement, IntoElement, Window, div, px};

const DROP_HEADING_SIZE: f32 = 20.0;

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
        .debug_selector(|| "drag-drop-overlay".into())
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p_6()
        .bg(color(palette.background.with_opacity(0.84)))
        .backdrop_blur(theme::ui_rem(6.0).to_pixels(window.rem_size()))
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
                .absolute()
                .inset(theme::ui_rem(12.0))
                .rounded(theme::ui_rem(theme::RADIUS_LG + 8.0))
                .border(px(1.0))
                .border_dashed()
                .border_dashed_length(6.0)
                .border_dashed_gap(4.0)
                .border_color(color(palette.focus_ring.with_opacity(0.25))),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_4()
                .text_center()
                .child(icon_svg(
                    assets::ICON_FILE_IMPORT,
                    32.0,
                    color(palette.text_muted),
                ))
                .child(
                    div()
                        .text_size(theme::ui_rem(DROP_HEADING_SIZE))
                        .line_height(gpui::relative(1.4))
                        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                        .text_color(color(palette.text_primary))
                        .child(theme::ui_text("Drop to add files")),
                ),
        )
}
