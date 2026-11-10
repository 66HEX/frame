use crate::app::preview::panel::PreviewShellState;
use crate::app::preview::toolbar::preview_tool_button;
use crate::app::root::FrameRoot;
use crate::assets;
use crate::preview::{PreviewMediaKind, format_time};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, IntoElement, Window};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::app) struct PreviewTimelineLabels {
    pub(in crate::app) start: String,
    pub(in crate::app) end: String,
    pub(in crate::app) duration: String,
}

pub(in crate::app) fn preview_timeline_labels(state: &PreviewShellState) -> PreviewTimelineLabels {
    if state.availability.media_kind == PreviewMediaKind::Image
        || state.availability.media_kind == PreviewMediaKind::Unknown
        || state.duration_seconds <= 0.0
    {
        return PreviewTimelineLabels {
            start: "--:--:--.---".to_string(),
            end: "--:--:--.---".to_string(),
            duration: "--:--:--.---".to_string(),
        };
    }

    PreviewTimelineLabels {
        start: format_time(state.playback.start_value()),
        end: format_time(state.playback.end_value()),
        duration: format_time(state.playback.end_value() - state.playback.start_value()),
    }
}

pub(in crate::app) fn preview_trim_enabled(state: &PreviewShellState) -> bool {
    !state.availability.trim_disabled && state.duration_seconds > 0.0
}

pub(in crate::app) fn preview_play_button(
    state: &PreviewShellState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let enabled = preview_trim_enabled(state);
    let icon = if state.playback.is_playing() {
        assets::ICON_PAUSE
    } else {
        assets::ICON_PLAY
    };

    preview_tool_button(
        "preview-playback-toggle",
        icon,
        if state.playback.is_playing() {
            "Pause preview"
        } else {
            "Play preview"
        },
        None,
        enabled,
        crate::app::components::TooltipPlacement::Above,
        state.palette,
        window,
        cx,
    )
    .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
        if root.toggle_preview_playback_with_context(cx) {
            cx.notify();
        }
    }))
}
