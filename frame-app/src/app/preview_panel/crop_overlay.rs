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

pub(in crate::app) fn preview_crop_overlay(
    state: &PreviewShellState,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
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
        .child(crop_outline_rect(x, y, width, height, cx))
        .child(crop_vertical_guide_line(x + width / 3.0, y, height))
        .child(crop_vertical_guide_line(x + (width * 2.0) / 3.0, y, height))
        .child(crop_horizontal_guide_line(x, y + height / 3.0, width))
        .child(crop_horizontal_guide_line(
            x,
            y + (height * 2.0) / 3.0,
            width,
        ))
        .child(preview_crop_handle(DragHandle::NorthWest, x, y, cx))
        .child(preview_crop_handle(
            DragHandle::North,
            x + width / 2.0,
            y,
            cx,
        ))
        .child(preview_crop_handle(DragHandle::NorthEast, right, y, cx))
        .child(preview_crop_handle(
            DragHandle::East,
            right,
            y + height / 2.0,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::SouthEast,
            right,
            bottom,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::South,
            x + width / 2.0,
            bottom,
            cx,
        ))
        .child(preview_crop_handle(DragHandle::SouthWest, x, bottom, cx))
        .child(preview_crop_handle(
            DragHandle::West,
            x,
            y + height / 2.0,
            cx,
        ))
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
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let outline = div()
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
        );

    apply_accessible_button(outline, crop_handle_label(DragHandle::Move), true)
        .aria_description(crop_keyboard_description())
        .on_key_down(cx.listener(|root, event: &gpui::KeyDownEvent, window, cx| {
            let key = event.keystroke.key.as_str();
            let changed = match key {
                "enter" => {
                    let changed = root.apply_selected_crop();
                    root.focus_registered_control("preview-tool-crop", window, cx);
                    changed
                }
                "escape" => {
                    let changed = root.toggle_selected_crop_mode();
                    root.focus_registered_control("preview-tool-crop", window, cx);
                    changed
                }
                "delete" | "backspace" => root.reset_preview_crop_selection(),
                _ => root.adjust_preview_crop_from_keyboard_with_step(
                    DragHandle::Move,
                    key,
                    event.keystroke.modifiers.shift,
                ),
            };
            if changed {
                cx.notify();
            }
            if crop_keyboard_key_is_handled(DragHandle::Move, key)
                || matches!(key, "enter" | "escape" | "delete" | "backspace")
            {
                cx.stop_propagation();
            }
        }))
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
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let handle_element = crop_handle_cursor(
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
    );

    apply_accessible_button(handle_element, crop_handle_label(handle), true)
        .aria_description(crop_keyboard_description())
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                let changed = match key {
                    "enter" => {
                        let changed = root.apply_selected_crop();
                        root.focus_registered_control("preview-tool-crop", window, cx);
                        changed
                    }
                    "escape" => {
                        let changed = root.toggle_selected_crop_mode();
                        root.focus_registered_control("preview-tool-crop", window, cx);
                        changed
                    }
                    "delete" | "backspace" => root.reset_preview_crop_selection(),
                    _ => root.adjust_preview_crop_from_keyboard_with_step(
                        handle,
                        key,
                        event.keystroke.modifiers.shift,
                    ),
                };
                if changed {
                    cx.notify();
                }
                if crop_keyboard_key_is_handled(handle, key)
                    || matches!(key, "enter" | "escape" | "delete" | "backspace")
                {
                    cx.stop_propagation();
                }
            }),
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

pub(in crate::app) const fn crop_handle_label(handle: DragHandle) -> &'static str {
    match handle {
        DragHandle::Move => "Move crop selection",
        DragHandle::North => "Resize crop top edge",
        DragHandle::South => "Resize crop bottom edge",
        DragHandle::East => "Resize crop right edge",
        DragHandle::West => "Resize crop left edge",
        DragHandle::NorthEast => "Resize crop top right corner",
        DragHandle::NorthWest => "Resize crop top left corner",
        DragHandle::SouthEast => "Resize crop bottom right corner",
        DragHandle::SouthWest => "Resize crop bottom left corner",
    }
}

fn crop_keyboard_key_is_handled(handle: DragHandle, key: &str) -> bool {
    let horizontal = matches!(
        handle,
        DragHandle::Move
            | DragHandle::East
            | DragHandle::West
            | DragHandle::NorthEast
            | DragHandle::NorthWest
            | DragHandle::SouthEast
            | DragHandle::SouthWest
    );
    let vertical = matches!(
        handle,
        DragHandle::Move
            | DragHandle::North
            | DragHandle::South
            | DragHandle::NorthEast
            | DragHandle::NorthWest
            | DragHandle::SouthEast
            | DragHandle::SouthWest
    );

    matches!(key, "left" | "right") && horizontal || matches!(key, "up" | "down") && vertical
}

const fn crop_keyboard_description() -> &'static str {
    "Use arrow keys to adjust the crop. Hold Shift for a larger step. Press Enter to apply, Escape to exit, or Delete to reset."
}

pub(in crate::app) fn preview_crop_aspect_bar(
    state: &PreviewShellState,
    focuses: PreviewEditToolbarFocus<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let first_focus = focuses.first.clone();
    let last_focus = focuses.last.clone();
    let mut bar = div()
        .id("preview-crop-toolbar")
        .track_focus(focuses.panel)
        .tab_stop(false)
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_MD))
        .bg(parse_hex(PREVIEW_TOOLBAR_BACKGROUND))
        .p(px(4.0))
        .shadow(card_surface_shadows())
        .on_key_down(
            cx.listener(move |_root, event: &gpui::KeyDownEvent, window, cx| {
                handle_modal_tab_navigation(event, &first_focus, &last_focus, window, cx);
            }),
        );

    for option in ASPECT_OPTIONS {
        let id = option.id;
        let button = if id == "free" {
            compact_text_button_with_focus(
                option.display,
                state.crop.crop_aspect == id,
                true,
                focuses.first,
                window,
                cx,
            )
        } else {
            compact_text_button(
                option.display,
                state.crop.crop_aspect == id,
                true,
                window,
                cx,
            )
        };
        bar = bar.child(
            button.on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
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
            compact_text_button_variant_inner(
                "Apply",
                ButtonVariant::Default,
                false,
                state.crop.has_crop_dimensions,
                Some(focuses.last),
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                if root.apply_selected_crop() {
                    root.focus_registered_control("preview-tool-crop", window, cx);
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

pub(in crate::app) fn compact_text_button_with_focus(
    label: &'static str,
    selected: bool,
    enabled: bool,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let variant = if selected {
        ButtonVariant::Default
    } else {
        ButtonVariant::Ghost
    };

    compact_text_button_variant_inner(label, variant, selected, enabled, Some(focus), window, cx)
}

pub(in crate::app) fn compact_text_button_variant(
    label: &'static str,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    compact_text_button_variant_inner(label, variant, selected, enabled, None, window, cx)
}

fn compact_text_button_variant_inner(
    label: &'static str,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(variant, selected, enabled);
    let id = format!("preview-crop-action-{}", label.to_ascii_lowercase());
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;
    let highlighted = selected || matches!(variant, ButtonVariant::Default);

    let button = div()
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
        .child(theme::ui_text(label));

    let button = apply_button_motion(button, motion, enabled);

    if let Some(focus) = focus {
        let button = apply_accessible_button_with_focus(button, label, enabled, focus);
        if selected {
            button.aria_toggled(gpui::Toggled::True)
        } else {
            button
        }
    } else if selected {
        apply_accessible_toggle_button(button, label, enabled, true)
    } else {
        apply_accessible_button(button, label, enabled)
    }
}
