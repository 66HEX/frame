use crate::app::accessibility::apply_accessible_button;
use crate::app::layout::{
    TITLEBAR_ACTION_ICON_SIZE, TITLEBAR_HEIGHT, TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
    TITLEBAR_LINUX_WINDOW_CONTROLS_GAP, TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X,
    TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_PLACEHOLDER_WIDTH, TITLEBAR_TRAFFIC_LIGHT_SIZE,
    TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH, TITLEBAR_WINDOWS_WINDOW_ICON_SIZE,
    TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE,
};
use crate::app::primitives::{
    ButtonColors, animated_button_colors, apply_button_motion, color, icon_svg,
};
use crate::app::root::FrameRoot;
use crate::{assets, theme};
use gpui::prelude::Styled;
use gpui::{
    ClickEvent, Context, InteractiveElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Window, WindowControlArea, div,
};

pub(in crate::app) fn macos_native_window_controls_placeholder() -> gpui::Div {
    div()
        .w(theme::ui_rem(
            TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_PLACEHOLDER_WIDTH,
        ))
        .h(theme::ui_rem(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .mr_2()
}

pub(in crate::app) fn windows_window_controls(
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .absolute()
        .top_0()
        .right_0()
        .h_full()
        .flex()
        .items_center()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            titlebar_window_button(
                "titlebar-windows-minimize",
                assets::ICON_MINUS,
                "Minimize window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_WINDOWS_WINDOW_ICON_SIZE,
                    width: TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                    height: TITLEBAR_HEIGHT,
                    radius: 0.0,
                },
                false,
                palette,
                window,
                cx,
            )
            .window_control_area(WindowControlArea::Min)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.minimize_window();
            })),
        )
        .child(
            titlebar_window_button(
                "titlebar-windows-maximize",
                assets::ICON_SQUARE,
                "Maximize window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE,
                    width: TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                    height: TITLEBAR_HEIGHT,
                    radius: 0.0,
                },
                false,
                palette,
                window,
                cx,
            )
            .window_control_area(WindowControlArea::Max)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.zoom_window();
            })),
        )
        .child(
            titlebar_window_button(
                "titlebar-windows-close",
                assets::ICON_CLOSE,
                "Close window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_WINDOWS_WINDOW_ICON_SIZE,
                    width: TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                    height: TITLEBAR_HEIGHT,
                    radius: 0.0,
                },
                true,
                palette,
                window,
                cx,
            )
            .window_control_area(WindowControlArea::Close)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.remove_window();
            })),
        )
}

pub(in crate::app) fn linux_window_controls(
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .absolute()
        .top_0()
        .right_0()
        .h_full()
        .flex()
        .items_center()
        .gap(theme::ui_rem(TITLEBAR_LINUX_WINDOW_CONTROLS_GAP))
        .px(theme::ui_rem(TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            titlebar_window_button(
                "titlebar-linux-minimize",
                assets::ICON_MINUS,
                "Minimize window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_ACTION_ICON_SIZE,
                    width: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    height: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    radius: theme::RADIUS_SM,
                },
                false,
                palette,
                window,
                cx,
            )
            .window_control_area(WindowControlArea::Min)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.minimize_window();
            })),
        )
        .child(
            titlebar_window_button(
                "titlebar-linux-maximize",
                assets::ICON_SQUARE,
                "Maximize window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_ACTION_ICON_SIZE,
                    width: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    height: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    radius: theme::RADIUS_SM,
                },
                false,
                palette,
                window,
                cx,
            )
            .window_control_area(WindowControlArea::Max)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.zoom_window();
            })),
        )
        .child(
            titlebar_window_button(
                "titlebar-linux-close",
                assets::ICON_CLOSE,
                "Close window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_ACTION_ICON_SIZE,
                    width: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    height: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    radius: theme::RADIUS_SM,
                },
                true,
                palette,
                window,
                cx,
            )
            .window_control_area(WindowControlArea::Close)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.remove_window();
            })),
        )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct TitlebarWindowButtonMetrics {
    pub(in crate::app) icon_size: f32,
    pub(in crate::app) width: f32,
    pub(in crate::app) height: f32,
    pub(in crate::app) radius: f32,
}

#[expect(
    clippy::too_many_arguments,
    reason = "Window chrome keeps platform metrics, semantics, palette, and render context explicit."
)]
pub(in crate::app) fn titlebar_window_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    metrics: TitlebarWindowButtonMetrics,
    destructive: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let hover_background = if destructive {
        palette.danger_surface
    } else {
        palette.fill_subtle
    };
    let active_background = if destructive {
        palette.danger_surface
    } else {
        palette.fill_selected
    };
    let hover_foreground = if destructive {
        palette.danger
    } else {
        palette.text_primary
    };
    let foreground = palette.text_muted;
    let colors = ButtonColors {
        gradient_to: None,
        background: palette.transparent,
        hover_background,
        active_background,
        foreground,
        hover_foreground,
        opacity: 1.0,
    };
    let animated = animated_button_colors(id, colors, window, cx);
    let background = animated.background;
    let icon_color = animated.foreground;
    let motion = animated.motion;

    let button = div()
        .id(id)
        .debug_selector(|| id.to_string())
        .w(theme::ui_rem(metrics.width))
        .h(theme::ui_rem(metrics.height))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(metrics.radius))
        .bg(background)
        .text_color(icon_color)
        .hover_style(gpui::Styled::cursor_pointer)
        .active_style(move |style| style.bg(color(active_background)))
        .on_mouse_move(|_, _, cx| cx.stop_propagation())
        .child(icon_svg(icon, metrics.icon_size, icon_color));
    let button = apply_button_motion(button, motion, true);

    apply_accessible_button(button, label, true, palette).tab_stop(false)
}
