use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FlipAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PreviewCropDrag {
    pub(super) handle: DragHandle,
}

pub(in crate::app) fn preview_crop_overlay(state: &PreviewShellState) -> gpui::Div {
    let rect = state.crop.draft_crop.unwrap_or_else(default_crop_rect);
    let x = rect.x as f32;
    let y = rect.y as f32;
    let width = rect.width as f32;
    let height = rect.height as f32;
    let right = (x + width).min(1.0);
    let bottom = (y + height).min(1.0);

    div()
        .absolute()
        .inset_0()
        .child(crop_mask_rect(0.0, 0.0, 1.0, y.clamp(0.0, 1.0)))
        .child(crop_mask_rect(0.0, y, x.clamp(0.0, 1.0), height))
        .child(crop_mask_rect(right, y, (1.0 - right).max(0.0), height))
        .child(crop_mask_rect(0.0, bottom, 1.0, (1.0 - bottom).max(0.0)))
        .child(crop_outline_rect(x, y, width, height))
        .child(crop_vertical_guide_line(x + width / 3.0, y, height))
        .child(crop_vertical_guide_line(x + (width * 2.0) / 3.0, y, height))
        .child(crop_horizontal_guide_line(x, y + height / 3.0, width))
        .child(crop_horizontal_guide_line(
            x,
            y + (height * 2.0) / 3.0,
            width,
        ))
        .child(preview_crop_handle(DragHandle::NorthWest, x, y, state))
        .child(preview_crop_handle(
            DragHandle::North,
            x + width / 2.0,
            y,
            state,
        ))
        .child(preview_crop_handle(DragHandle::NorthEast, right, y, state))
        .child(preview_crop_handle(
            DragHandle::East,
            right,
            y + height / 2.0,
            state,
        ))
        .child(preview_crop_handle(
            DragHandle::SouthEast,
            right,
            bottom,
            state,
        ))
        .child(preview_crop_handle(
            DragHandle::South,
            x + width / 2.0,
            bottom,
            state,
        ))
        .child(preview_crop_handle(DragHandle::SouthWest, x, bottom, state))
        .child(preview_crop_handle(
            DragHandle::West,
            x,
            y + height / 2.0,
            state,
        ))
}

pub(in crate::app) fn crop_mask_rect(left: f32, top: f32, width: f32, height: f32) -> gpui::Div {
    div()
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(relative(width.clamp(0.0, 1.0)))
        .h(relative(height.clamp(0.0, 1.0)))
        .bg(hsla(0.0, 0.0, 0.0, 0.55))
}

pub(in crate::app) fn crop_outline_rect(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id("preview-crop-move-handle")
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(relative(width.clamp(0.0, 1.0)))
        .h(relative(height.clamp(0.0, 1.0)))
        .border_1()
        .border_color(color(theme::FOREGROUND.with_alpha(0.90)))
        .cursor_grab()
        .on_drag(
            PreviewCropDrag {
                handle: DragHandle::Move,
            },
            |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
        )
}

pub(in crate::app) fn crop_vertical_guide_line(left: f32, top: f32, height: f32) -> gpui::Div {
    div()
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(px(1.0))
        .h(relative(height.clamp(0.0, 1.0)))
        .bg(color(theme::FOREGROUND.with_alpha(0.70)))
}

pub(in crate::app) fn crop_horizontal_guide_line(left: f32, top: f32, width: f32) -> gpui::Div {
    div()
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(relative(width.clamp(0.0, 1.0)))
        .h(px(1.0))
        .bg(color(theme::FOREGROUND.with_alpha(0.70)))
}

pub(in crate::app) fn preview_crop_handle(
    handle: DragHandle,
    x: f32,
    y: f32,
    state: &PreviewShellState,
) -> gpui::Stateful<gpui::Div> {
    crop_handle_cursor(
        div()
            .id(format!("preview-crop-handle-{}", crop_handle_id(handle)))
            .absolute()
            .left(relative(x.clamp(0.0, 1.0)))
            .top(relative(y.clamp(0.0, 1.0)))
            .ml(px(-(CROP_HANDLE_SIZE / 2.0)))
            .mt(px(-(CROP_HANDLE_SIZE / 2.0)))
            .w(px(CROP_HANDLE_SIZE))
            .h(px(CROP_HANDLE_SIZE))
            .rounded_full()
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.0, 0.45))
            .bg(color(theme::FOREGROUND))
            .shadow(card_surface_shadows()),
        handle,
        is_side_rotation(&state.crop.rotation),
    )
    .on_drag(
        PreviewCropDrag { handle },
        |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
    )
}

pub(in crate::app) fn crop_handle_cursor(
    handle: gpui::Stateful<gpui::Div>,
    drag_handle: DragHandle,
    is_side_rotation: bool,
) -> gpui::Stateful<gpui::Div> {
    match crate::preview::handle_cursor(drag_handle, is_side_rotation) {
        "ns-resize" => handle.cursor_ns_resize(),
        "ew-resize" => handle.cursor_ew_resize(),
        "nesw-resize" => handle.cursor_nesw_resize(),
        "nwse-resize" => handle.cursor_nwse_resize(),
        _ => handle.cursor_grab(),
    }
}

pub(in crate::app) fn crop_handle_id(handle: DragHandle) -> &'static str {
    match handle {
        DragHandle::Move => "move",
        DragHandle::North => "n",
        DragHandle::South => "s",
        DragHandle::East => "e",
        DragHandle::West => "w",
        DragHandle::NorthEast => "ne",
        DragHandle::NorthWest => "nw",
        DragHandle::SouthEast => "se",
        DragHandle::SouthWest => "sw",
    }
}

pub(in crate::app) fn preview_crop_aspect_bar(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut bar = div()
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows());

    for option in ASPECT_OPTIONS {
        let id = option.id;
        bar = bar.child(
            compact_text_button(option.display, state.crop.crop_aspect == id, true).on_click(
                cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    if root.select_preview_crop_aspect(id) {
                        cx.notify();
                    }
                }),
            ),
        );
    }

    let bar = bar
        .child(preview_toolbar_separator().h(px(18.0)).w(px(1.0)))
        .child(
            compact_text_button("Reset", false, true).on_click(cx.listener(
                |root, _: &ClickEvent, _window, cx| {
                    if root.reset_preview_crop_selection() {
                        cx.notify();
                    }
                },
            )),
        )
        .child(
            compact_text_button("Apply", false, state.crop.has_crop_dimensions).on_click(
                cx.listener(|root, _: &ClickEvent, _window, cx| {
                    if root.apply_selected_crop() {
                        cx.notify();
                    }
                }),
            ),
        );

    div()
        .absolute()
        .bottom(px(16.0))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(bar)
}

pub(in crate::app) fn compact_text_button(
    label: &'static str,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let variant = if selected {
        ButtonVariant::Default
    } else {
        ButtonVariant::Ghost
    };
    let colors = button_colors(variant, selected, enabled);

    div()
        .id(format!(
            "preview-crop-action-{}",
            label.to_ascii_lowercase()
        ))
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .px(px(10.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(if selected {
            color(colors.background)
        } else {
            color(theme::TRANSPARENT)
        })
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
        })
        .when(!enabled, |this| this.cursor_not_allowed())
        .child(label)
}
