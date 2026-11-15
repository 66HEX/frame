use crate::app::components::{FrameTooltip, TooltipPlacement, frame_text_button_with_focus};
use crate::app::preview::actions::source_export::{
    SourceFrameExportRenderState, SourceFrameExportStatus,
};
use crate::app::primitives::{ButtonVariant, color};
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, InteractiveElement, Window, div};

pub(in crate::app) const SOURCE_FRAME_DESCRIPTION: &str =
    "Save a full-resolution PNG from the source, without conversion settings.";

#[derive(Clone, Copy)]
pub(in crate::app) struct SourceFrameExportFocuses<'a> {
    pub(in crate::app) export: &'a FocusHandle,
    pub(in crate::app) cancel: &'a FocusHandle,
    pub(in crate::app) open: &'a FocusHandle,
}

pub(in crate::app) fn source_export_toolbar(
    state: &SourceFrameExportRenderState,
    focuses: SourceFrameExportFocuses<'_>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let enabled = state.disabled_reason.is_none();
    let description = state.disabled_reason.map_or_else(
        || SOURCE_FRAME_DESCRIPTION.to_owned(),
        |reason| format!("{SOURCE_FRAME_DESCRIPTION} {reason}"),
    );
    let export = frame_text_button_with_focus(
        "source-frame-export",
        "Export source frame",
        ButtonVariant::Secondary,
        false,
        enabled,
        focuses.export,
        palette,
        window,
        cx,
    )
    .aria_description(description.clone())
    .on_click(cx.listener(|root, _: &ClickEvent, _, cx| root.start_source_frame_export(cx)));
    let export = FrameTooltip::new("source-frame-export", description)
        .placement(TooltipPlacement::Above)
        .for_control(export, Some(focuses.export), enabled, palette, window, cx);
    let mut controls = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(theme::ui_rem(theme::metrics::CONTROL_GAP))
        .child(export);
    if state.running {
        controls = controls.child(
            frame_text_button_with_focus(
                "source-frame-cancel",
                "Cancel",
                ButtonVariant::Ghost,
                false,
                state.status != SourceFrameExportStatus::Cancelling,
                focuses.cancel,
                palette,
                window,
                cx,
            )
            .aria_label("Cancel source frame export")
            .on_click(
                cx.listener(|root, _: &ClickEvent, _, cx| root.cancel_source_frame_export(cx)),
            ),
        );
    }
    if state.result_name.is_some() {
        controls = controls.child(
            frame_text_button_with_focus(
                "source-frame-open",
                "Open result",
                ButtonVariant::Ghost,
                false,
                true,
                focuses.open,
                palette,
                window,
                cx,
            )
            .aria_description("Open the last exported source frame.")
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                root.open_source_frame_result(window, cx);
            })),
        );
    }
    let message = source_export_status(state);
    let status = div()
        .id("source-frame-export-status")
        .w_full()
        .min_w_0()
        .truncate()
        .text_color(color(palette.text_muted))
        .text_size(theme::ui_rem(11.0))
        .role(gpui::Role::Status)
        .aria_live(gpui::accesskit::Live::Polite)
        .aria_live_atomic(true)
        .aria_label(message.clone())
        .child(message.clone());
    let status = FrameTooltip::new("source-frame-export-status", message)
        .placement(TooltipPlacement::Above)
        .for_control(status, None, true, palette, window, cx);
    div()
        .id("source-frame-export-toolbar")
        .flex_none()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(theme::ui_rem(theme::metrics::CONTROL_GAP))
        .pt(theme::ui_rem(8.0))
        .child(controls)
        .child(status)
}

fn source_export_status(state: &SourceFrameExportRenderState) -> String {
    match &state.status {
        SourceFrameExportStatus::Idle => state
            .disabled_reason
            .unwrap_or("PNG from the source, without conversion settings.")
            .to_owned(),
        SourceFrameExportStatus::Exporting => "Exporting source frame…".into(),
        SourceFrameExportStatus::Cancelling => "Cancelling source frame export…".into(),
        SourceFrameExportStatus::Saved => format!(
            "Saved {}",
            state.result_name.as_deref().unwrap_or("source frame")
        ),
        SourceFrameExportStatus::Cancelled => "Source frame export cancelled.".into(),
        SourceFrameExportStatus::Error(error) => format!("Source frame export failed: {error}"),
    }
}

#[cfg(test)]
mod tests;
