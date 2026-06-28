use super::*;

const PREVIEW_TOOLBAR_PADDING: f32 = 4.0;
const PREVIEW_TOOLBAR_GAP: f32 = 8.0;
const PREVIEW_TOOLBAR_SEPARATOR_HEIGHT: f32 = 1.0;
const PREVIEW_TOOLBAR_BUTTON_COUNT: f32 = 5.0;
const PREVIEW_TOOLBAR_GAP_COUNT: f32 = 5.0;

pub(in crate::app) const fn preview_toolbar_height() -> f32 {
    (PREVIEW_TOOLBAR_PADDING * 2.0)
        + (PREVIEW_TOOLBAR_BUTTON_SIZE * PREVIEW_TOOLBAR_BUTTON_COUNT)
        + PREVIEW_TOOLBAR_SEPARATOR_HEIGHT
        + (PREVIEW_TOOLBAR_GAP * PREVIEW_TOOLBAR_GAP_COUNT)
}

pub(in crate::app) const fn preview_toolbar_center_margin() -> f32 {
    -(preview_toolbar_height() / 2.0)
}

pub(in crate::app) fn preview_toolbar(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let transform_enabled = preview_visual_controls_enabled(state);
    let crop_enabled = transform_enabled && state.crop.has_crop_dimensions;
    let overlay_enabled = transform_enabled && state.availability.overlay_available;

    div()
        .absolute()
        .top(relative(0.5))
        .mt(px(preview_toolbar_center_margin()))
        .left(px(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .flex_col()
        .gap(px(PREVIEW_TOOLBAR_GAP))
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::BACKGROUND))
        .p(px(PREVIEW_TOOLBAR_PADDING))
        .shadow(card_surface_shadows())
        .child(
            preview_tool_button(assets::ICON_ROTATE_CW, false, transform_enabled).on_click(
                cx.listener(|root, _: &ClickEvent, _window, cx| {
                    if root.rotate_selected_preview() {
                        cx.notify();
                    }
                }),
            ),
        )
        .child(
            preview_tool_button(
                assets::ICON_FLIP_HORIZONTAL,
                state.crop.flip_horizontal,
                transform_enabled,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_flip(FlipAxis::Horizontal) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                assets::ICON_FLIP_VERTICAL,
                state.crop.flip_vertical,
                transform_enabled,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_flip(FlipAxis::Vertical) {
                    cx.notify();
                }
            })),
        )
        .child(preview_toolbar_separator())
        .child(
            preview_tool_button(
                assets::ICON_CROP,
                state.crop.crop_mode || state.crop.applied_crop.is_some(),
                crop_enabled,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_crop_mode() {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                assets::ICON_FILE_IMAGE,
                state.overlay.overlay_mode || state.overlay.overlay.is_some(),
                overlay_enabled,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.trigger_selected_overlay(cx) {
                    cx.notify();
                }
            })),
        )
}

pub(in crate::app) fn preview_zoom_toolbar(state: &PreviewShellState) -> gpui::Div {
    let enabled = preview_visual_controls_enabled(state);

    div()
        .absolute()
        .right(px(PREVIEW_TOOLBAR_OFFSET))
        .bottom(px(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows())
        .child(preview_tool_button(assets::ICON_MINUS, false, enabled))
        .child(preview_tool_button(assets::ICON_PLUS, false, enabled))
}

pub(in crate::app) fn preview_toolbar_separator() -> gpui::Div {
    div()
        .h(px(PREVIEW_TOOLBAR_SEPARATOR_HEIGHT))
        .w_full()
        .bg(color(theme::BACKGROUND))
        .shadow(horizontal_separator_shadows())
}

pub(in crate::app) fn preview_tool_button(
    icon: &'static str,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let variant = if selected {
        ButtonVariant::Default
    } else {
        ButtonVariant::Ghost
    };
    let colors = button_colors(variant, false, enabled);
    let button_id = format!("preview-tool-{}", icon.replace(['/', '.'], "-"));

    div()
        .id(button_id.clone())
        .group(button_id.clone())
        .w(px(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .h(px(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(if selected {
            color(colors.background)
        } else {
            color(theme::TRANSPARENT)
        })
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .when(!enabled, |this| this.cursor_not_allowed())
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .child(icon_svg_with_hover(
            icon,
            PREVIEW_TOOLBAR_ICON_SIZE,
            color(colors.foreground),
            button_id,
            color(colors.hover_foreground),
        ))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
}
