//! Shared layout and animated surface for the pinned radix-mira dialog preset.

use super::{FrameStyle, SurfaceOrigin, frame_surface_motion};
use crate::app::primitives::color;
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{Context, Div, FocusHandle, IntoElement, Stateful, Window, div, relative};

pub(in crate::app) fn frame_dialog_panel(
    id: &'static str,
    title: &str,
    description: Option<&str>,
    focus: &FocusHandle,
    palette: &theme::ThemePalette,
    window: &Window,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_owned())
        .role(gpui::Role::Dialog)
        .aria_modal(true)
        .aria_label(title.to_owned())
        .when_some(description, |panel, description| {
            panel.aria_description(description.to_owned())
        })
        .track_focus(focus)
        .tab_stop(false)
        .relative()
        .w_full()
        .min_w_0()
        .max_h(
            (window.viewport_size().height - theme::ui_rem(32.0).to_pixels(window.rem_size()))
                .max(gpui::px(0.0)),
        )
        .overflow_y_scroll()
        .when(dialog_desktop_layout(window), |panel| {
            panel.max_w(theme::ui_rem(384.0))
        })
        .flex()
        .flex_col()
        .gap(theme::ui_rem(16.0))
        .p(theme::ui_rem(16.0))
        .text_size(theme::ui_rem(12.0))
        .line_height(relative(1.625))
        .dialog_surface(palette)
        .occlude()
        .on_click(|_, _, cx| cx.stop_propagation())
}

pub(in crate::app) fn frame_dialog_header(
    title: &str,
    description: Option<&str>,
    palette: &theme::ThemePalette,
) -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .pr(theme::ui_rem(24.0))
        .flex()
        .flex_col()
        .gap(theme::ui_rem(4.0))
        .child(
            div()
                .w_full()
                .min_w_0()
                .whitespace_normal()
                .text_size(theme::ui_rem(14.0))
                .line_height(relative(20.0 / 14.0))
                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                .text_color(color(palette.token("popover-foreground")))
                .child(title.to_owned()),
        )
        .when_some(description, |header, description| {
            header.child(
                div()
                    .w_full()
                    .min_w_0()
                    .whitespace_normal()
                    .text_size(theme::ui_rem(12.0))
                    .line_height(relative(1.625))
                    .text_color(color(palette.token("muted-foreground")))
                    .child(description.to_owned()),
            )
        })
}

/// Add secondary actions first and the primary action last, as in the reference.
pub(in crate::app) fn frame_dialog_footer(palette: &theme::ThemePalette, window: &Window) -> Div {
    div()
        .flex_shrink_0()
        .mx(theme::ui_rem(-16.0))
        .mb(theme::ui_rem(-16.0))
        .p(theme::ui_rem(16.0))
        .border_t_1()
        .border_color(color(palette.token("border")))
        .bg(color(palette.token("muted").with_opacity(0.5)))
        .rounded_b(theme::ui_rem(theme::RADIUS_BASE * 1.4))
        .flex()
        .flex_col_reverse()
        .gap(theme::ui_rem(8.0))
        .when(dialog_desktop_layout(window), |footer| {
            footer.flex_row().flex_wrap().justify_end()
        })
}

fn dialog_desktop_layout(window: &Window) -> bool {
    window.viewport_size().width >= theme::ui_rem(640.0).to_pixels(window.rem_size())
}

/// The caller keeps content mounted until its presence state finishes closing.
/// During exit, callers also disable their individual accessible action controls.
pub(in crate::app) fn frame_dialog_overlay(
    id: &'static str,
    content: impl IntoElement,
    progress: f32,
    is_open: bool,
    palette: &theme::ThemePalette,
    window: &Window,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_owned())
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p(theme::ui_rem(16.0))
        .bg(color(palette.overlay_backdrop))
        .opacity(progress.clamp(0.0, 1.0))
        .backdrop_blur(theme::ui_rem(4.0).to_pixels(window.rem_size()))
        .aria_disabled(!is_open)
        .occlude()
        .capture_any_mouse_down(move |_, window, cx| {
            if !is_open {
                window.prevent_default();
                cx.stop_propagation();
            }
        })
        .capture_any_mouse_up(move |_, window, cx| {
            if !is_open {
                window.prevent_default();
                cx.stop_propagation();
            }
        })
        .capture_key_down(move |_, window, cx| {
            if !is_open {
                window.prevent_default();
                cx.stop_propagation();
            }
        })
        .child(frame_surface_motion(
            content,
            progress,
            SurfaceOrigin::Center,
        ))
}

pub(in crate::app) fn frame_dialog_close_button<T: 'static>(
    id: &'static str,
    label: &str,
    enabled: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<T>,
) -> Stateful<Div> {
    let pressed_top = theme::ui_rem(8.0).to_pixels(window.rem_size()) + gpui::px(1.0);
    super::frame_button_base(
        id,
        label,
        crate::app::primitives::ButtonVariant::Ghost,
        false,
        enabled,
        theme::metrics::ControlSize::Sm,
        palette,
        window,
        cx,
    )
    .track_focus(focus)
    .size(theme::ui_rem(24.0))
    .p_0()
    .child(crate::app::primitives::icon_svg(
        assets::ICON_CLOSE,
        12.0,
        color(palette.text_primary),
    ))
    .absolute()
    .top(theme::ui_rem(8.0))
    .right(theme::ui_rem(8.0))
    // The shared button's pressed style uses relative positioning. Preserve this
    // absolute anchor so pressing X cannot move it into the dialog's flex flow.
    .active_style(move |style| style.absolute().top(pressed_top))
}
