use crate::app::input::FrameTextInputKind;
use crate::app::layout::PREVIEW_TIMELINE_TOP_MARGIN;
use crate::app::preview::panel::{PreviewShellState, PreviewTimecodeInputFocuses};
use crate::app::preview::timeline::timecode::{
    PreviewTimecodeFieldSpec, preview_timecode_field, preview_timeline_label,
};
use crate::app::preview::timeline::track::preview_timeline_track;
use crate::app::preview::toolbar::preview_tool_button;
use crate::app::root::FrameRoot;
use crate::preview::{PreviewMediaKind, format_time};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, IntoElement, Window, div};

pub(in crate::app) fn preview_timeline(
    state: &PreviewShellState,
    inputs: PreviewTimecodeInputFocuses<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = state.palette;
    let labels = preview_timeline_labels(state);
    let trim_enabled = preview_trim_enabled(state);

    div()
        .mt(theme::ui_rem(PREVIEW_TIMELINE_TOP_MARGIN))
        .px_2()
        .flex()
        .items_center()
        .gap_4()
        .child(
            div()
                .flex()
                .gap_4()
                .child(preview_timecode_field(
                    PreviewTimecodeFieldSpec {
                        label: "Start time",
                        value: inputs.start_value.to_string(),
                        enabled: trim_enabled,
                        width: 128.0,
                        kind: Some(FrameTextInputKind::PreviewStartTime),
                        focus: inputs.start,
                    },
                    palette,
                    window,
                    cx,
                ))
                .child(preview_timecode_field(
                    PreviewTimecodeFieldSpec {
                        label: "End time",
                        value: inputs.end_value.to_string(),
                        enabled: trim_enabled,
                        width: 128.0,
                        kind: Some(FrameTextInputKind::PreviewEndTime),
                        focus: inputs.end,
                    },
                    palette,
                    window,
                    cx,
                ))
                .child(preview_timecode_field(
                    PreviewTimecodeFieldSpec {
                        label: "Duration",
                        value: labels.duration,
                        enabled: false,
                        width: 104.0,
                        kind: None,
                        focus: None,
                    },
                    palette,
                    window,
                    cx,
                )),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(theme::ui_rem(6.0))
                .child(preview_timeline_label("Trim", palette))
                .child(preview_timeline_track(state, window, cx)),
        )
        .child(
            crate::app::components::frame_field()
                .w(theme::ui_rem(
                    crate::app::layout::PREVIEW_TOOLBAR_BUTTON_SIZE,
                ))
                .flex_shrink_0()
                .child(preview_timeline_label(" ", palette))
                .child(preview_play_button(state, window, cx)),
        )
}

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
        state.tooltip_visible_id.as_deref(),
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
