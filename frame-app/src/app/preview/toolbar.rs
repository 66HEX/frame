use crate::app::components::frame_toolbar;
use crate::app::components::{FrameTooltip, TooltipPlacement};
use crate::app::layout::{
    PREVIEW_TOOLBAR_BUTTON_SIZE, PREVIEW_TOOLBAR_ICON_SIZE, PREVIEW_TOOLBAR_OFFSET,
};
use crate::app::preview::crop_overlay::FlipAxis;
use crate::app::preview::panel::{PreviewShellState, PreviewToolFocuses};
use crate::app::preview::preview_visual_controls_enabled;
use crate::app::preview::state::PreviewCanvasZoomDirection;
use crate::app::primitives::{ButtonVariant, color};
use crate::app::root::FrameRoot;
use crate::{assets, theme};
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    ClickEvent, Context, FocusHandle, InteractiveElement, StatefulInteractiveElement, Window, div,
    px, relative,
};

const PREVIEW_TOOLBAR_PADDING: f32 = 4.0;
const PREVIEW_TOOLBAR_GAP: f32 = theme::metrics::CONTROL_GAP;
const PREVIEW_TOOLBAR_VERTICAL_SEPARATOR_HEIGHT: f32 = 18.0;
const PREVIEW_TOOLBAR_BUTTON_COUNT: f32 = 5.0;
const PREVIEW_TOOLBAR_GAP_COUNT: f32 = 4.0;

pub(in crate::app) const fn preview_toolbar_height() -> f32 {
    (PREVIEW_TOOLBAR_PADDING * 2.0)
        + (PREVIEW_TOOLBAR_BUTTON_SIZE * PREVIEW_TOOLBAR_BUTTON_COUNT)
        + (PREVIEW_TOOLBAR_GAP * PREVIEW_TOOLBAR_GAP_COUNT)
}

pub(in crate::app) const fn preview_toolbar_center_margin() -> f32 {
    -(preview_toolbar_height() / 2.0)
}

#[expect(
    clippy::too_many_lines,
    reason = "The declarative toolbar layout keeps its mutually dependent controls in visual order."
)]
pub(in crate::app) fn preview_toolbar(
    state: &PreviewShellState,
    focuses: PreviewToolFocuses<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = state.palette;
    let transform_enabled = preview_visual_controls_enabled(state);
    let crop_enabled = transform_enabled && state.crop.has_crop_dimensions;
    let overlay_enabled = transform_enabled && state.availability.overlay_available;

    frame_toolbar(palette)
        .absolute()
        .top(relative(0.5))
        .mt(theme::ui_rem(preview_toolbar_center_margin()))
        .left(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .flex_col()
        .gap(theme::ui_rem(PREVIEW_TOOLBAR_GAP))
        .child(
            preview_tool_button(
                "preview-tool-rotate",
                assets::ICON_ROTATE_CW,
                "Rotate preview",
                None,
                transform_enabled,
                state.tooltip_visible_id.as_deref(),
                TooltipPlacement::Right,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.rotate_selected_preview() {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                "preview-tool-flip-horizontal",
                assets::ICON_FLIP_HORIZONTAL,
                "Flip horizontally",
                Some(state.crop.flip_horizontal),
                transform_enabled,
                state.tooltip_visible_id.as_deref(),
                TooltipPlacement::Right,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_flip(FlipAxis::Horizontal) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                "preview-tool-flip-vertical",
                assets::ICON_FLIP_VERTICAL,
                "Flip vertically",
                Some(state.crop.flip_vertical),
                transform_enabled,
                state.tooltip_visible_id.as_deref(),
                TooltipPlacement::Right,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_flip(FlipAxis::Vertical) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button_with_focus(
                "preview-tool-crop",
                assets::ICON_CROP,
                "Crop",
                Some(state.crop.crop_mode || state.crop.applied_crop.is_some()),
                crop_enabled,
                focuses.crop,
                state.tooltip_visible_id.as_deref(),
                TooltipPlacement::Right,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.toggle_selected_crop_mode() {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button_with_focus(
                "preview-tool-overlay",
                assets::ICON_FILE_IMAGE,
                "Overlay image",
                Some(state.overlay.overlay_mode || state.overlay.has_overlay),
                overlay_enabled,
                focuses.overlay,
                state.tooltip_visible_id.as_deref(),
                TooltipPlacement::Right,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                if root.trigger_selected_overlay(window, cx) {
                    cx.notify();
                }
            })),
        )
}

pub(in crate::app) fn preview_zoom_toolbar(
    state: &PreviewShellState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = state.palette;
    let enabled = preview_visual_controls_enabled(state);

    frame_toolbar(palette)
        .absolute()
        .right(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .bottom(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.surface))
        .p(theme::ui_rem(4.0))
        .shadow(theme::shadows::FLOATING.shadows())
        .child(
            preview_tool_button(
                "preview-zoom-out",
                assets::ICON_MINUS,
                "Zoom out",
                None,
                enabled,
                state.tooltip_visible_id.as_deref(),
                TooltipPlacement::Above,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.zoom_preview_canvas(PreviewCanvasZoomDirection::Out, cx) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_tool_button(
                "preview-zoom-in",
                assets::ICON_PLUS,
                "Zoom in",
                None,
                enabled,
                state.tooltip_visible_id.as_deref(),
                TooltipPlacement::Above,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                if root.zoom_preview_canvas(PreviewCanvasZoomDirection::In, cx) {
                    cx.notify();
                }
            })),
        )
}

pub(in crate::app) fn preview_toolbar_vertical_separator(
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex_none()
        .h(theme::ui_rem(PREVIEW_TOOLBAR_VERTICAL_SEPARATOR_HEIGHT))
        .w(px(theme::shadows::EDGE_WIDTH))
        .bg(color(palette.fill_selected))
}

#[expect(
    clippy::too_many_arguments,
    reason = "Toolbar buttons keep semantics, interaction state, palette, and render context explicit."
)]
pub(in crate::app) fn preview_tool_button(
    id: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    selected: Option<bool>,
    enabled: bool,
    tooltip_visible_id: Option<&str>,
    tooltip_placement: TooltipPlacement,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    preview_tool_button_inner(
        id,
        icon,
        label,
        selected,
        enabled,
        None,
        tooltip_visible_id,
        tooltip_placement,
        palette,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Preview tool buttons need explicit focus handles for modal focus restoration."
)]
pub(in crate::app) fn preview_tool_button_with_focus(
    id: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    selected: Option<bool>,
    enabled: bool,
    focus: &FocusHandle,
    tooltip_visible_id: Option<&str>,
    tooltip_placement: TooltipPlacement,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    preview_tool_button_inner(
        id,
        icon,
        label,
        selected,
        enabled,
        Some(focus),
        tooltip_visible_id,
        tooltip_placement,
        palette,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared tool button builder preserves the existing visual contract and optionally wires a focus handle."
)]
fn preview_tool_button_inner(
    id: impl Into<String>,
    icon: &'static str,
    label: impl Into<String>,
    selected: Option<bool>,
    enabled: bool,
    focus: Option<&FocusHandle>,
    tooltip_visible_id: Option<&str>,
    tooltip_placement: TooltipPlacement,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let button_id = id.into();
    let label = label.into();
    let button = crate::app::components::frame_button_base(
        button_id.clone(),
        &label,
        ButtonVariant::Ghost,
        selected.unwrap_or(false),
        enabled,
        theme::metrics::ControlSize::Default,
        palette,
        window,
        cx,
    )
    .when_some(selected, |button, selected| {
        button.aria_toggled(gpui::Toggled::from(selected))
    })
    .w(theme::ui_rem(PREVIEW_TOOLBAR_BUTTON_SIZE))
    .px_0()
    .child(
        gpui::svg()
            .path(icon)
            .size(theme::ui_rem(PREVIEW_TOOLBAR_ICON_SIZE)),
    );
    let button = if let Some(focus) = focus {
        button.track_focus(focus)
    } else {
        button
    };
    FrameTooltip {
        hovered: tooltip_visible_id == Some(button_id.as_str()),
        id: button_id,
        label,
        placement: tooltip_placement,
    }
    .for_control(button, focus, enabled, palette, window, cx)
}
