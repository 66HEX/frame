use super::{
    App, ButtonVariant, Context, FluentBuilder, FrameRoot, InteractiveElement, IntoElement,
    MouseButton, MouseMoveEvent, ParentElement, PlatformInput, SETTINGS_CONTROL_HEIGHT,
    ScrollHandle, ScrollWheelEvent, StatefulInteractiveElement, Styled, Window,
    animated_button_colors, apply_accessible_select_option,
    apply_accessible_select_option_with_focus, apply_accessible_select_trigger,
    apply_accessible_select_trigger_with_focus, apply_button_motion, assets, button_colors,
    button_highlight_shadows, button_mouse_down, card_surface_shadows, color, div, icon_svg,
    input_highlight_shadows, parse_hex, theme,
};
use crate::app::accessibility::handle_modal_tab_navigation;
use crate::numeric::usize_to_f32;
use gpui::FocusHandle;

pub(in crate::app) const FRAME_SELECT_MAX_HEIGHT: f32 = 192.0;
pub(in crate::app) const FRAME_SELECT_CONTENT_PADDING: f32 = 4.0;
pub(in crate::app) const FRAME_SELECT_OPTION_HEIGHT: f32 = 28.0;
pub(in crate::app) const FRAME_COLOR_SWATCH_SIZE: f32 = 14.0;

#[expect(
    clippy::too_many_arguments,
    reason = "Select triggers need explicit labels, state, palette, and GPUI render context."
)]
pub(in crate::app) fn frame_select_trigger(
    id: impl Into<String>,
    label: impl Into<String>,
    display: &str,
    enabled: bool,
    expanded: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_trigger_content(
        id,
        label,
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_color(color(palette.text_primary))
            .child(theme::ui_text(display)),
        enabled,
        expanded,
        palette,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Select triggers need explicit labels, state, rendering context, and a focus handle."
)]
pub(in crate::app) fn frame_select_trigger_with_focus(
    id: impl Into<String>,
    label: impl Into<String>,
    display: &str,
    enabled: bool,
    expanded: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_trigger_content_inner(
        id,
        label,
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_color(color(palette.text_primary))
            .child(theme::ui_text(display)),
        enabled,
        expanded,
        Some(focus),
        palette,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Content-based select triggers need explicit semantics, state, palette, and render context."
)]
pub(in crate::app) fn frame_select_trigger_content(
    id: impl Into<String>,
    label: impl Into<String>,
    content: impl IntoElement,
    enabled: bool,
    expanded: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_trigger_content_inner(
        id, label, content, enabled, expanded, None, palette, window, cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Select triggers need optional explicit focus handles while preserving the existing visual builder."
)]
pub(in crate::app) fn frame_select_trigger_content_with_focus(
    id: impl Into<String>,
    label: impl Into<String>,
    content: impl IntoElement,
    enabled: bool,
    expanded: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_trigger_content_inner(
        id,
        label,
        content,
        enabled,
        expanded,
        Some(focus),
        palette,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared select trigger builder preserves the existing visual contract and optionally wires a focus handle."
)]
fn frame_select_trigger_content_inner(
    id: impl Into<String>,
    label: impl Into<String>,
    content: impl IntoElement,
    enabled: bool,
    expanded: bool,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let colors = button_colors(ButtonVariant::Secondary, false, enabled, palette);
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;

    let trigger = div()
        .id(id.clone())
        .group(id)
        .min_h(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .min_w_0()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .px(theme::ui_rem(10.0))
        .bg(background)
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows(palette))
        .when(enabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .child(content)
        .child(
            div()
                .flex_shrink_0()
                .child(icon_svg(assets::ICON_UNFOLD_MORE, 12.0, foreground)),
        );

    let trigger = apply_button_motion(trigger, motion, enabled).on_mouse_down(
        MouseButton::Left,
        move |_, _window, cx| {
            cx.stop_propagation();
        },
    );

    if let Some(focus) = focus {
        apply_accessible_select_trigger_with_focus(
            trigger, label, enabled, expanded, focus, palette,
        )
    } else {
        apply_accessible_select_trigger(trigger, label, enabled, expanded, palette)
    }
}

pub(in crate::app) fn frame_select_popover(
    id: &'static str,
    top: f32,
    progress: f32,
    list: impl IntoElement,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .absolute()
        .id(id)
        .top(theme::ui_rem(top))
        .left_0()
        .right_0()
        .max_h(theme::ui_rem(FRAME_SELECT_MAX_HEIGHT))
        .overflow_hidden()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(color(palette.surface_elevated))
        .opacity(progress)
        .shadow(if palette.is_light() {
            card_surface_shadows(palette)
        } else {
            button_highlight_shadows(palette)
        })
        .occlude()
        .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
            cx.stop_propagation();
        })
        .child(list)
}

pub(in crate::app) fn frame_select_options_list(
    id: &'static str,
    scroll_handle: &ScrollHandle,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .role(gpui::Role::ListBox)
        .max_h(theme::ui_rem(FRAME_SELECT_MAX_HEIGHT))
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .p(theme::ui_rem(FRAME_SELECT_CONTENT_PADDING))
        .on_scroll_wheel(refresh_select_hover_after_scroll)
}

pub(in crate::app) fn frame_select_content_height(option_count: usize) -> f32 {
    usize_to_f32(option_count).mul_add(
        FRAME_SELECT_OPTION_HEIGHT,
        FRAME_SELECT_CONTENT_PADDING * 2.0,
    )
}

pub(in crate::app) fn frame_select_target_index(
    len: usize,
    selected_index: Option<usize>,
    key: &str,
    is_enabled: impl Fn(usize) -> bool,
) -> Option<usize> {
    if len == 0 {
        return None;
    }
    match key {
        "home" => (0..len).find(|index| is_enabled(*index)),
        "end" => (0..len).rev().find(|index| is_enabled(*index)),
        "down" => {
            let start = selected_index.unwrap_or(len - 1);
            (1..=len)
                .map(|offset| (start + offset) % len)
                .find(|index| is_enabled(*index))
        }
        "up" => {
            let start = selected_index.unwrap_or(0);
            (1..=len)
                .map(|offset| (start + len - offset) % len)
                .find(|index| is_enabled(*index))
        }
        _ => None,
    }
}

pub(in crate::app) const fn frame_select_option_focus<'a>(
    index: usize,
    option_count: usize,
    first_focus: Option<&'a FocusHandle>,
    last_focus: Option<&'a FocusHandle>,
) -> Option<&'a FocusHandle> {
    if index == 0 {
        first_focus
    } else if index + 1 == option_count {
        last_focus
    } else {
        None
    }
}

pub(in crate::app) const fn frame_select_last_focus<'a>(
    option_count: usize,
    first_focus: Option<&'a FocusHandle>,
    last_focus: Option<&'a FocusHandle>,
) -> Option<&'a FocusHandle> {
    if option_count <= 1 {
        first_focus
    } else {
        last_focus
    }
}

pub(in crate::app) fn focus_frame_select_initial_target(
    key: &str,
    option_count: usize,
    first_focus: Option<&FocusHandle>,
    last_focus: Option<&FocusHandle>,
    scroll_handle: &ScrollHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) {
    if option_count == 0 {
        return;
    }
    let target_index = if matches!(key, "up" | "end") {
        option_count.saturating_sub(1)
    } else {
        0
    };
    scroll_handle.scroll_to_item(target_index);
    let focus = if target_index == 0 {
        first_focus
    } else {
        last_focus.or(first_focus)
    };
    if let Some(focus) = focus {
        focus.focus(window, cx);
    }
}

pub(in crate::app) fn apply_frame_select_popover_focus_trap(
    popover: gpui::Stateful<gpui::Div>,
    panel_focus: Option<&FocusHandle>,
    first_focus: Option<&FocusHandle>,
    last_focus: Option<&FocusHandle>,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let (Some(panel_focus), Some(first_focus), Some(last_focus)) =
        (panel_focus, first_focus, last_focus)
    else {
        return popover;
    };
    let first_focus = first_focus.clone();
    let last_focus = last_focus.clone();
    popover
        .track_focus(panel_focus)
        .tab_stop(false)
        .on_key_down(
            cx.listener(move |_root, event: &gpui::KeyDownEvent, window, cx| {
                handle_modal_tab_navigation(event, &first_focus, &last_focus, window, cx);
            }),
        )
}

#[derive(Clone, Copy)]
pub(in crate::app) struct FrameSelectFocusTarget<'a> {
    pub(in crate::app) current_index: usize,
    pub(in crate::app) target_index: usize,
    pub(in crate::app) first_focus: Option<&'a FocusHandle>,
    pub(in crate::app) last_focus: Option<&'a FocusHandle>,
    pub(in crate::app) scroll_handle: &'a ScrollHandle,
}

pub(in crate::app) fn focus_frame_select_target(
    key: &str,
    target: FrameSelectFocusTarget<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) {
    target.scroll_handle.scroll_to_item(target.target_index);
    let boundary_focus = match key {
        "home" => target.first_focus,
        "end" => target.last_focus,
        "down" if target.target_index <= target.current_index => target.first_focus,
        "up" if target.target_index >= target.current_index => target.last_focus,
        "down" => {
            window.focus_next(cx);
            None
        }
        "up" => {
            window.focus_prev(cx);
            None
        }
        _ => None,
    };
    if let Some(focus) = boundary_focus {
        focus.focus(window, cx);
    }
}

pub(in crate::app) fn frame_select_option(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    frame_select_option_inner(id, label, selected, enabled, None, palette)
}

pub(in crate::app) fn frame_select_option_with_focus(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    frame_select_option_inner(id, label, selected, enabled, Some(focus), palette)
}

fn frame_select_option_inner(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    let display_label = theme::ui_text_owned(label.clone());
    let text_color = if selected {
        palette.text_primary
    } else {
        palette.text_muted
    };

    let option = div()
        .id(id.into())
        .min_h(theme::ui_rem(FRAME_SELECT_OPTION_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_XS))
        .px(theme::ui_rem(12.0))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(text_color))
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, |this| {
            this.hover(|style| {
                style
                    .bg(color(palette.fill_subtle))
                    .text_color(color(palette.text_primary))
                    .cursor_pointer()
            })
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            button_mouse_down(enabled, window, cx);
        })
        .child(div().min_w_0().truncate().child(display_label))
        .when(selected, |this| {
            this.child(icon_svg(
                assets::ICON_CHECK,
                12.0,
                color(palette.text_primary),
            ))
        });

    if let Some(focus) = focus {
        apply_accessible_select_option_with_focus(option, label, enabled, selected, focus, palette)
    } else {
        apply_accessible_select_option(option, label, enabled, selected, palette)
    }
}

pub(in crate::app) fn frame_color_select_value(
    value: &str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap_2()
        .child(frame_color_swatch(value, palette))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .w_full()
                .truncate()
                .text_color(color(palette.text_primary))
                .child(value.to_uppercase()),
        )
}

pub(in crate::app) fn frame_color_swatch(
    value: &str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .w(theme::ui_rem(FRAME_COLOR_SWATCH_SIZE))
        .h(theme::ui_rem(FRAME_COLOR_SWATCH_SIZE))
        .flex_shrink_0()
        .rounded(theme::ui_rem(theme::RADIUS_XS))
        .bg(parse_hex(value))
        .shadow(input_highlight_shadows(palette))
}

fn refresh_select_hover_after_scroll(
    _event: &ScrollWheelEvent,
    window: &mut Window,
    _cx: &mut App,
) {
    window.refresh();
    window.on_next_frame(|window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: window.mouse_position(),
                pressed_button: None,
                modifiers: window.modifiers(),
            }),
            cx,
        );
    });
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::float_cmp,
        reason = "Component tests compare exact deterministic layout constants."
    )]

    use super::*;

    #[test]
    fn frame_select_content_height_includes_vertical_padding() {
        let expected = 3.0_f32.mul_add(
            FRAME_SELECT_OPTION_HEIGHT,
            FRAME_SELECT_CONTENT_PADDING * 2.0,
        );
        assert!((frame_select_content_height(3) - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn frame_color_swatch_uses_compact_visual_size() {
        assert_eq!(FRAME_COLOR_SWATCH_SIZE, 14.0);
    }

    #[test]
    fn frame_select_target_wraps_to_next_enabled_option() {
        let target = frame_select_target_index(4, Some(3), "down", |index| index != 0);
        assert_eq!(target, Some(1));
    }

    #[test]
    fn frame_select_target_supports_home_and_end() {
        let home = frame_select_target_index(4, Some(2), "home", |index| index != 0);
        let end = frame_select_target_index(4, Some(2), "end", |index| index != 3);
        assert_eq!((home, end), (Some(1), Some(2)));
    }

    #[test]
    fn frame_select_target_handles_empty_and_fully_disabled_lists() {
        assert_eq!(frame_select_target_index(0, None, "down", |_| true), None);
        assert_eq!(frame_select_target_index(3, Some(1), "up", |_| false), None);
        assert_eq!(
            frame_select_target_index(3, Some(1), "home", |_| false),
            None
        );
        assert_eq!(
            frame_select_target_index(3, Some(1), "end", |_| false),
            None
        );
    }

    #[test]
    fn frame_select_target_keeps_a_single_enabled_option_stable() {
        for key in ["up", "down", "home", "end"] {
            assert_eq!(
                frame_select_target_index(1, Some(0), key, |_| true),
                Some(0)
            );
        }
    }

    #[test]
    fn frame_select_target_ignores_unhandled_keys() {
        assert_eq!(
            frame_select_target_index(3, Some(1), "escape", |_| true),
            None
        );
    }
}
