use crate::app::accessibility::apply_accessible_button;
use crate::app::components::frame_handle_hit_region;
use crate::app::components::{FrameStyle, FrameTooltip, TooltipPlacement};
use crate::app::layout::{
    PREVIEW_TOOLBAR_BUTTON_SIZE, PREVIEW_TOOLBAR_ICON_SIZE, PREVIEW_TOOLBAR_OFFSET,
};
use crate::app::preview::auto_crop::{AutoCropRenderState, AutoCropStatus};
use crate::app::preview::crop::default_crop_rect;
use crate::app::preview::panel::{
    PreviewCropOptionsProps, PreviewCropRenderState, PreviewEditToolbarFocus, PreviewShellState,
};
use crate::app::preview::state::CROP_HANDLE_SIZE;
use crate::app::preview::toolbar::{
    preview_tool_button, preview_toolbar_vertical_separator, preview_toolbar_width,
    preview_zoom_toolbar_width,
};
use crate::app::preview::{
    PreviewTimelineDragPreview, preview_crop_custom_fields, preview_crop_options,
    preview_resize_handle, preview_selection_line_color,
};
use crate::app::primitives::{ButtonVariant, color};
use crate::app::root::FrameRoot;
use crate::numeric::unit_f64_to_f32;
use crate::preview::{CropRect, DragHandle, PreviewRotation, clamp_rect, transform_crop_rect};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, Window, div, hsla, px, relative};

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
    let palette = state.palette;
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
        .child(crop_outline_rect(x, y, width, height, palette, cx))
        .child(crop_vertical_guide_line(x + width / 3.0, y, height))
        .child(crop_vertical_guide_line(x + (width * 2.0) / 3.0, y, height))
        .child(crop_horizontal_guide_line(x, y + height / 3.0, width))
        .child(crop_horizontal_guide_line(
            x,
            y + (height * 2.0) / 3.0,
            width,
        ))
        .child(preview_crop_handle(
            DragHandle::NorthWest,
            x,
            y,
            (width / 2.0, height / 2.0),
            palette,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::North,
            x + width / 2.0,
            y,
            (width / 2.0, height / 2.0),
            palette,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::NorthEast,
            right,
            y,
            (width / 2.0, height / 2.0),
            palette,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::East,
            right,
            y + height / 2.0,
            (width / 2.0, height / 2.0),
            palette,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::SouthEast,
            right,
            bottom,
            (width / 2.0, height / 2.0),
            palette,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::South,
            x + width / 2.0,
            bottom,
            (width / 2.0, height / 2.0),
            palette,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::SouthWest,
            x,
            bottom,
            (width / 2.0, height / 2.0),
            palette,
            cx,
        ))
        .child(preview_crop_handle(
            DragHandle::West,
            x,
            y + height / 2.0,
            (width / 2.0, height / 2.0),
            palette,
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
    // Media overlay: the mask must remain perceptible over both bright and dark source pixels.
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
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let outline = div()
        .id("preview-crop-move-handle")
        .debug_selector(|| "preview-crop-move-handle".into())
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(relative(width.clamp(0.0, 1.0)))
        .h(relative(height.clamp(0.0, 1.0)))
        .border_1()
        .border_color(preview_selection_line_color())
        .cursor_grab()
        .on_drag(
            PreviewCropDrag {
                handle: DragHandle::Move,
            },
            |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
        );

    apply_accessible_button(outline, crop_handle_label(DragHandle::Move), true, palette)
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
        .bg(preview_selection_line_color())
}

pub(in crate::app) fn crop_horizontal_guide_line(left: f32, top: f32, width: f32) -> gpui::Div {
    div()
        .absolute()
        .left(relative(left.clamp(0.0, 1.0)))
        .top(relative(top.clamp(0.0, 1.0)))
        .w(relative(width.clamp(0.0, 1.0)))
        .h(px(1.0))
        .bg(preview_selection_line_color())
}

pub(in crate::app) fn preview_crop_handle(
    handle: DragHandle,
    x: f32,
    y: f32,
    spacing: (f32, f32),
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    let handle_element = crop_handle_cursor(
        div()
            .id(format!("preview-crop-handle-{}", crop_handle_id(handle)))
            .debug_selector(move || format!("preview-crop-handle-{}", crop_handle_id(handle)))
            .flex()
            .items_center()
            .justify_center()
            .child(preview_resize_handle(CROP_HANDLE_SIZE, palette)),
        handle,
    )
    .on_drag(
        PreviewCropDrag { handle },
        |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
    );

    let handle_element =
        apply_accessible_button(handle_element, crop_handle_label(handle), true, palette)
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
            );
    frame_handle_hit_region(x, y, spacing.0, spacing.1, handle_element)
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
    options: PreviewCropOptionsProps<'_>,
    focuses: PreviewEditToolbarFocus<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = state.palette;
    let mut controls = div()
        .flex()
        .flex_shrink_0()
        .flex_wrap()
        .min_w_0()
        .max_w_full()
        .items_center()
        .gap_1();

    controls = controls.child(preview_crop_options(state, focuses, options, window, cx));
    if options.custom {
        controls = controls.child(preview_crop_custom_fields(
            state, focuses, options, window, cx,
        ));
    }

    let controls = controls
        .child(preview_toolbar_vertical_separator(palette))
        .when(
            state.crop.auto_crop.available
                || state.crop.auto_crop.status == AutoCropStatus::Analyzing,
            |this| this.child(auto_crop_button(&state.crop.auto_crop, palette, window, cx)),
        )
        .child(
            preview_tool_button(
                "preview-crop-action-reset",
                assets::ICON_REFRESH,
                "Reset",
                None,
                true,
                TooltipPlacement::Above,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.reset_preview_crop_selection() {
                    cx.notify();
                }
            })),
        )
        .child(
            compact_text_button_variant_inner(
                "Apply",
                ButtonVariant::Primary,
                false,
                state.crop.has_crop_dimensions
                    && (!options.custom || options.custom_values.is_some()),
                Some(focuses.last),
                palette,
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
        .top(theme::ui_rem(16.0))
        .bottom(theme::ui_rem(16.0))
        .left_0()
        .right_0()
        // Reserve the transform and zoom toolbars' hit regions, even when wrapping.
        .pl(theme::ui_rem(
            PREVIEW_TOOLBAR_OFFSET + preview_toolbar_width() + 8.0,
        ))
        .pr(theme::ui_rem(
            PREVIEW_TOOLBAR_OFFSET + preview_zoom_toolbar_width() + 8.0,
        ))
        .flex()
        .items_end()
        .justify_center()
        .child(
            div()
                .relative()
                .id("preview-crop-toolbar")
                .debug_selector(|| "preview-crop-toolbar".into())
                .track_focus(focuses.panel)
                .tab_stop(false)
                .flex()
                .flex_col()
                .min_w_0()
                .min_h_0()
                .max_w_full()
                .max_h_full()
                .popover_surface(palette)
                .p(theme::ui_rem(4.0))
                .shadow(theme::shadows::FLOATING.shadows())
                .child(
                    div()
                        .id("preview-crop-toolbar-controls")
                        .debug_selector(|| "preview-crop-toolbar-controls".into())
                        .min_h_0()
                        .overflow_y_scroll()
                        .child(controls),
                )
                .child(auto_crop_status(&state.crop.auto_crop.status)),
        )
}

fn auto_crop_button(
    state: &AutoCropRenderState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let analyzing = state.status == AutoCropStatus::Analyzing;
    let label = if analyzing {
        "Cancel analysis"
    } else {
        "Auto crop"
    };
    let message = auto_crop_status_text(&state.status);
    let icon = match state.status {
        AutoCropStatus::Idle => assets::ICON_AUTO_CROP,
        AutoCropStatus::Analyzing => assets::ICON_SQUARE,
        AutoCropStatus::Ready | AutoCropStatus::NoBars => assets::ICON_CHECK,
        AutoCropStatus::Error(_) => assets::ICON_WARNING,
    };
    let icon_color =
        matches!(state.status, AutoCropStatus::Error(_)).then(|| color(palette.danger));
    let button = crate::app::components::frame_button_base(
        "preview-crop-auto",
        label,
        ButtonVariant::Ghost,
        analyzing,
        true,
        theme::metrics::ControlSize::Default,
        palette,
        window,
        cx,
    )
    .w(theme::ui_rem(PREVIEW_TOOLBAR_BUTTON_SIZE))
    .px_0()
    .aria_description(message)
    .child(crate::app::components::frame_animated_icon(
        "preview-crop-auto",
        Some(icon),
        PREVIEW_TOOLBAR_ICON_SIZE,
        icon_color,
        window,
        cx,
    ));
    let tooltip = if message.is_empty() {
        label.to_string()
    } else {
        format!("{label} · {message}")
    };
    FrameTooltip::new("preview-crop-auto", tooltip)
        .placement(TooltipPlacement::Above)
        .keep_open_on_click()
        .for_control(button, None, true, palette, window, cx)
        .debug_selector(|| "preview-crop-auto".into())
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if analyzing {
                if root.cancel_selected_auto_crop() {
                    cx.notify();
                }
            } else {
                root.start_selected_auto_crop(cx);
            }
        }))
}

const fn auto_crop_status_text(status: &AutoCropStatus) -> &str {
    match status {
        AutoCropStatus::Idle => "",
        AutoCropStatus::Analyzing => "Analyzing video…",
        AutoCropStatus::Ready => "Adjust the crop if needed, then Apply.",
        AutoCropStatus::NoBars => "No black bars detected.",
        AutoCropStatus::Error(message) => message.as_str(),
    }
}

fn auto_crop_status(status: &AutoCropStatus) -> gpui::Stateful<gpui::Div> {
    // Keep a live region for assistive technology without changing toolbar geometry.
    div()
        .id("preview-crop-auto-status")
        .debug_selector(|| "preview-crop-auto-status".into())
        .absolute()
        .top_0()
        .left_0()
        .size(px(1.0))
        .role(gpui::Role::Status)
        .aria_live(gpui::accesskit::Live::Polite)
        .aria_live_atomic(true)
        .aria_label(auto_crop_status_text(status).to_owned())
}

#[expect(
    clippy::too_many_arguments,
    reason = "The compact button primitive keeps its state, optional focus, palette, and render context explicit."
)]
fn compact_text_button_variant_inner(
    label: &'static str,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = format!("preview-crop-action-{}", label.to_ascii_lowercase());
    let button = crate::app::components::frame_text_button_inner(
        id, label, variant, selected, enabled, focus, palette, window, cx,
    );
    if selected {
        button.aria_toggled(gpui::Toggled::True)
    } else {
        button
    }
}
