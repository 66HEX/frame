use super::*;
use crate::numeric::unit_f64_to_f32;

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
    let rect = preview_crop_visual_rect(&state.crop);
    let x = unit_f64_to_f32(rect.x);
    let y = unit_f64_to_f32(rect.y);
    let width = unit_f64_to_f32(rect.width);
    let height = unit_f64_to_f32(rect.height);
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
        .child(preview_crop_handle(DragHandle::NorthWest, x, y))
        .child(preview_crop_handle(DragHandle::North, x + width / 2.0, y))
        .child(preview_crop_handle(DragHandle::NorthEast, right, y))
        .child(preview_crop_handle(
            DragHandle::East,
            right,
            y + height / 2.0,
        ))
        .child(preview_crop_handle(DragHandle::SouthEast, right, bottom))
        .child(preview_crop_handle(
            DragHandle::South,
            x + width / 2.0,
            bottom,
        ))
        .child(preview_crop_handle(DragHandle::SouthWest, x, bottom))
        .child(preview_crop_handle(DragHandle::West, x, y + height / 2.0))
}

pub(in crate::app) fn preview_crop_visual_rect(state: &PreviewCropRenderState) -> CropRect {
    let rect = state.draft_crop.unwrap_or_else(default_crop_rect);
    clamp_rect(transform_crop_rect(
        rect,
        PreviewRotation::from(state.rotation.as_str()),
        state.flip_horizontal,
        state.flip_vertical,
        false,
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
    )
    .on_drag(
        PreviewCropDrag { handle },
        |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
    )
}

pub(in crate::app) fn crop_handle_cursor(
    handle: gpui::Stateful<gpui::Div>,
    drag_handle: DragHandle,
) -> gpui::Stateful<gpui::Div> {
    match crop_handle_screen_cursor(drag_handle) {
        "ns-resize" => handle.cursor_ns_resize(),
        "ew-resize" => handle.cursor_ew_resize(),
        "nesw-resize" => handle.cursor_nesw_resize(),
        "nwse-resize" => handle.cursor_nwse_resize(),
        _ => handle.cursor_grab(),
    }
}

pub(in crate::app) const fn crop_handle_screen_cursor(drag_handle: DragHandle) -> &'static str {
    crate::preview::handle_cursor(drag_handle, false)
}

pub(in crate::app) const fn crop_handle_id(handle: DragHandle) -> &'static str {
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
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut bar = div()
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(parse_hex(PREVIEW_TOOLBAR_BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows());

    for option in ASPECT_OPTIONS {
        let id = option.id;
        bar = bar.child(
            compact_text_button(
                option.display,
                state.crop.crop_aspect == id,
                true,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                if root.select_preview_crop_aspect(id) {
                    cx.notify();
                }
            })),
        );
    }

    let bar = bar
        .child(preview_toolbar_vertical_separator())
        .child(
            compact_text_button("Reset", false, true, window, cx).on_click(cx.listener(
                |root, _: &ClickEvent, _window, cx| {
                    if root.reset_preview_crop_selection() {
                        cx.notify();
                    }
                },
            )),
        )
        .child(
            compact_text_button_variant(
                "Apply",
                ButtonVariant::Default,
                false,
                state.crop.has_crop_dimensions,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.apply_selected_crop() {
                    cx.notify();
                }
            })),
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
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = if selected {
        ButtonVariant::Default
    } else {
        ButtonVariant::Ghost
    };

    compact_text_button_variant(label, variant, selected, enabled, window, cx)
}

pub(in crate::app) fn compact_text_button_variant(
    label: &'static str,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(variant, selected, enabled);
    let id = format!("preview-crop-action-{}", label.to_ascii_lowercase());
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;
    let highlighted = selected || matches!(variant, ButtonVariant::Default);

    div()
        .id(id)
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .px(px(10.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(background)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .when(highlighted, |this| this.shadow(button_highlight_shadows()))
        .when(enabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| {
                    style
                        .bg(color(colors.active_background))
                        .text_color(color(colors.hover_foreground))
                })
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && enabled, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .child(theme::ui_text(label))
}
