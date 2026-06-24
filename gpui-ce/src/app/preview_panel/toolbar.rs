use super::*;

pub(in crate::app) fn preview_toolbar(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let transform_enabled = preview_visual_controls_enabled(state);
    let crop_enabled = transform_enabled && state.crop.has_crop_dimensions;
    let overlay_enabled = transform_enabled && state.availability.overlay_available;

    div()
        .absolute()
        .top(px(PREVIEW_TOOLBAR_OFFSET))
        .left(px(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .flex_col()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::BACKGROUND))
        .p(px(4.0))
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
        .child(preview_tool_button(
            assets::ICON_FILE_IMAGE,
            false,
            overlay_enabled,
        ))
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
        .h(px(1.0))
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
    let icon_color = color(colors.foreground);
    let button_id = format!("preview-tool-{}", icon.replace(['/', '.'], "-"));

    div()
        .id(button_id)
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
        .text_color(icon_color)
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
        .child(icon_svg(icon, PREVIEW_TOOLBAR_ICON_SIZE, icon_color))
}
