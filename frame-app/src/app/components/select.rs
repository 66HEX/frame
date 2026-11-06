use super::FrameStyle;
use crate::app::accessibility::{
    apply_accessible_select_option, apply_accessible_select_option_with_focus,
    apply_accessible_select_trigger, apply_accessible_select_trigger_with_focus,
};
use crate::app::layout::SETTINGS_CONTROL_HEIGHT;
use crate::app::primitives::{
    ButtonVariant, button_colors, button_mouse_down, color, icon_svg, parse_hex,
};
use crate::app::root::FrameRoot;
use crate::numeric::usize_to_f32;
use crate::{assets, theme};
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    App, Context, FocusHandle, InteractiveElement, IntoElement, MouseButton, MouseMoveEvent,
    PlatformInput, ScrollHandle, ScrollWheelEvent, StatefulInteractiveElement, Window, div,
};

pub(in crate::app) const FRAME_SELECT_MAX_HEIGHT: f32 = 192.0;
pub(in crate::app) const FRAME_SELECT_CONTENT_PADDING: f32 = 4.0;
pub(in crate::app) const FRAME_SELECT_OPTION_HEIGHT: f32 = 28.0;
pub(in crate::app) const FRAME_COLOR_SWATCH_SIZE: f32 = 14.0;
// The pointer target overlays the options; it never reserves a layout gutter.
const FRAME_SELECT_SCROLLBAR_WIDTH: f32 = 12.0;

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
    _window: &mut Window,
    _cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let mut colors = button_colors(ButtonVariant::Outline, false, enabled, palette);
    colors.background = palette.fill_subtle;
    colors.hover_background = palette.secondary_hover;
    let trigger = div()
        .id(id.clone())
        .debug_selector(|| id.clone())
        .group(id)
        .min_h(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .min_w_0()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .px(theme::ui_rem(theme::metrics::CONTROL_PADDING_X))
        .input_surface(palette)
        .bg(color(colors.background))
        .shadow(theme::shadows::ShadowLevel::Xxs.shadows())
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_REGULAR)
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .when(enabled, |this| {
            this.cursor_default()
                .hover_style(move |style| style.bg(color(colors.hover_background)))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .child(content)
        .child(div().flex_shrink_0().child(icon_svg(
            assets::ICON_UNFOLD_MORE,
            14.0,
            color(palette.text_muted),
        )));

    let pointer_focus = focus.cloned();
    let trigger = trigger.on_mouse_down(MouseButton::Left, move |_, window, cx| {
        button_mouse_down(enabled, window, cx);
        // Explicit focus lets arrows address the menu after a pointer click.
        if enabled && let Some(focus) = &pointer_focus {
            focus.focus(window, cx);
        }
        cx.stop_propagation();
    });

    if let Some(focus) = focus {
        apply_accessible_select_trigger_with_focus(
            trigger, label, enabled, expanded, focus, palette,
        )
    } else {
        apply_accessible_select_trigger(trigger, label, enabled, expanded, palette)
    }
}

pub(in crate::app) fn frame_select_popover(
    id: &str,
    progress: f32,
    list: impl IntoElement,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .relative()
        .id(id.to_owned())
        .debug_selector({
            let id = id.to_owned();
            move || id
        })
        .w_full()
        .max_h(gpui::relative(1.0))
        .flex()
        .flex_col()
        .overflow_hidden()
        .popover_surface(palette)
        .opacity(progress)
        .shadow(theme::shadows::FLOATING.shadows())
        .occlude()
        .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
            cx.stop_propagation();
        })
        .child(list)
}

pub(in crate::app) fn frame_select_options_list(
    id: &str,
    scroll_handle: &ScrollHandle,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id.to_owned())
        .role(gpui::Role::ListBox)
        .min_h_0()
        .max_h(theme::ui_rem(FRAME_SELECT_MAX_HEIGHT))
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .p(theme::ui_rem(FRAME_SELECT_CONTENT_PADDING))
        .on_scroll_wheel(refresh_select_hover_after_scroll)
}

pub(in crate::app) fn frame_select_scrollbar(
    id: impl Into<String>,
    scroll_handle: ScrollHandle,
    content_height: f32,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    super::scrollbar::frame_vertical_scrollbar_clipped(
        id,
        scroll_handle,
        content_height,
        theme::RADIUS_LG,
        palette,
    )
    .w(theme::ui_rem(FRAME_SELECT_SCROLLBAR_WIDTH))
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
            (selected_index.map_or(0, |index| index + 1)..len).find(|index| is_enabled(*index))
        }
        "up" => (0..selected_index.unwrap_or(len))
            .rev()
            .find(|index| is_enabled(*index)),
        _ => None,
    }
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
    let id = id.into();
    let label = label.into();
    let display_label = theme::ui_text_owned(label.clone());
    let text_color = palette.text_primary;

    let option = div()
        .id(id.clone())
        .debug_selector(|| id)
        .min_h(theme::ui_rem(FRAME_SELECT_OPTION_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .px(theme::ui_rem(8.0))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_REGULAR)
        .text_color(color(text_color))
        .opacity(if enabled { 1.0 } else { 0.5 })
        // Pointer movement focuses the row, matching Radix SelectItem. A separate
        // hover fill would override focus and persist after keyboard navigation.
        .when(enabled, gpui::Styled::cursor_default)
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            button_mouse_down(enabled, window, cx);
        })
        .child(div().min_w_0().truncate().child(display_label))
        .when(selected, |this| {
            this.child(
                gpui::svg()
                    .path(assets::ICON_CHECK)
                    .w(theme::ui_rem(14.0))
                    .h(theme::ui_rem(14.0)),
            )
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
        .child(frame_color_swatch(value))
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

pub(in crate::app) fn frame_color_swatch(value: &str) -> gpui::Div {
    div()
        .w(theme::ui_rem(FRAME_COLOR_SWATCH_SIZE))
        .h(theme::ui_rem(FRAME_COLOR_SWATCH_SIZE))
        .flex_shrink_0()
        .rounded(theme::ui_rem(theme::RADIUS_XS))
        .bg(parse_hex(value))
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
    fn frame_select_target_handles_the_keyboard_navigation_matrix() {
        let cases = [
            (4, Some(3), "down", &[0][..], None),
            (4, Some(2), "home", &[0][..], Some(1)),
            (4, Some(2), "end", &[3][..], Some(2)),
            (0, None, "down", &[][..], None),
            (3, Some(1), "up", &[0, 1, 2][..], None),
            (3, Some(1), "home", &[0, 1, 2][..], None),
            (3, Some(1), "end", &[0, 1, 2][..], None),
            (1, Some(0), "up", &[][..], None),
            (1, Some(0), "down", &[][..], None),
            (1, Some(0), "home", &[][..], Some(0)),
            (1, Some(0), "end", &[][..], Some(0)),
            (3, Some(1), "escape", &[][..], None),
        ];

        for (option_count, current, key, disabled, expected) in cases {
            let actual = frame_select_target_index(option_count, current, key, |index| {
                !disabled.contains(&index)
            });
            assert_eq!(
                actual, expected,
                "navigation failed for count={option_count}, current={current:?}, key={key:?}, disabled={disabled:?}"
            );
        }
    }
}
