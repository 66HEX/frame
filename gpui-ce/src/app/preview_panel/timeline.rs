use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewTimelineDrag {
    target: TimelineDragTarget,
}

pub(super) struct PreviewTimelineDragPreview;

impl Render for PreviewTimelineDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(0.0)).h(px(0.0))
    }
}

pub(in crate::app) fn preview_timeline(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let labels = preview_timeline_labels(state);
    let trim_enabled = preview_trim_enabled(state);

    div()
        .mt(px(PREVIEW_TIMELINE_TOP_MARGIN))
        .px_2()
        .flex()
        .items_center()
        .gap_4()
        .child(
            div()
                .flex()
                .gap_4()
                .child(preview_timecode_field(
                    "START TIME",
                    labels.start,
                    trim_enabled,
                    128.0,
                ))
                .child(preview_timecode_field(
                    "END TIME",
                    labels.end,
                    trim_enabled,
                    128.0,
                ))
                .child(preview_timecode_field(
                    "DURATION",
                    labels.duration,
                    false,
                    104.0,
                )),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(preview_timeline_label("TRIM"))
                .child(preview_timeline_track(state, cx)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(preview_timeline_label(" "))
                .child(preview_play_button(state)),
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

pub(in crate::app) fn preview_timecode_field(
    label: &'static str,
    value: String,
    enabled: bool,
    width: f32,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(preview_timeline_label(label))
        .child(
            div()
                .w(px(width))
                .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
                .flex()
                .items_center()
                .rounded(px(theme::RADIUS_SM))
                .bg(color(theme::BACKGROUND))
                .px(px(10.0))
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(if enabled {
                    color(theme::FOREGROUND)
                } else {
                    color(theme::FRAME_GRAY_600)
                })
                .shadow(input_highlight_shadows())
                .child(value),
        )
}

pub(in crate::app) fn preview_timeline_label(label: &'static str) -> gpui::Div {
    div()
        .h(px(12.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600))
        .child(label)
}

pub(in crate::app) fn preview_timeline_track(
    state: &PreviewShellState,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let enabled = preview_trim_enabled(state);
    let track_top = centered_offset(PREVIEW_TIMELINE_CONTROL_HEIGHT, PREVIEW_TRACK_HEIGHT);
    let playhead_top = centered_offset(PREVIEW_TIMELINE_CONTROL_HEIGHT, PREVIEW_PLAYHEAD_HEIGHT);
    let start_fraction = timeline_fraction_from_percent(
        state
            .playback
            .to_timeline_percent(state.playback.start_value()),
    );
    let end_fraction = timeline_fraction_from_percent(
        state
            .playback
            .to_timeline_percent(state.playback.end_value()),
    );
    let playhead_fraction = timeline_fraction_from_percent(
        state
            .playback
            .to_timeline_percent(state.playback.current_time()),
    );

    div()
        .id("preview-timeline-track")
        .relative()
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w_full()
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, |this| this.cursor_pointer())
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewTimelineDrag>, _window, cx| {
                let drag = *event.drag(cx);
                let percent =
                    timeline_slider_percent_from_bounds(event.event.position, event.bounds);
                if root.apply_selected_trim_drag(drag.target, percent) {
                    cx.notify();
                }
            },
        ))
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(track_top))
                .h(px(PREVIEW_TRACK_HEIGHT))
                .rounded(px(1.5))
                .bg(color(theme::FRAME_GRAY_100))
                .shadow(input_highlight_shadows()),
        )
        .child(
            div()
                .absolute()
                .left(relative(start_fraction))
                .right(relative((1.0 - end_fraction).max(0.0)))
                .top(px(track_top))
                .h(px(PREVIEW_TRACK_HEIGHT))
                .rounded(px(1.0))
                .bg(color(theme::FOREGROUND)),
        )
        .child(
            div()
                .absolute()
                .left(relative(playhead_fraction))
                .ml(px(-0.5))
                .top(px(playhead_top))
                .h(px(PREVIEW_PLAYHEAD_HEIGHT))
                .w(px(1.0))
                .bg(color(theme::FOREGROUND)),
        )
        .child(preview_timeline_handle(
            TimelineDragTarget::Start,
            start_fraction,
            enabled,
        ))
        .child(preview_timeline_handle(
            TimelineDragTarget::End,
            end_fraction,
            enabled,
        ))
}

pub(in crate::app) fn preview_timeline_handle(
    target: TimelineDragTarget,
    fraction: f32,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let handle_id = match target {
        TimelineDragTarget::Start => "preview-timeline-start-handle",
        TimelineDragTarget::End => "preview-timeline-end-handle",
        TimelineDragTarget::Scrub => "preview-timeline-scrub-handle",
    };

    let handle = div()
        .id(handle_id)
        .absolute()
        .top_0()
        .left(relative(fraction))
        .ml(px(-(PREVIEW_TIMELINE_HANDLE_WIDTH / 2.0)))
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w(px(PREVIEW_TIMELINE_HANDLE_WIDTH))
        .when(enabled, |this| this.cursor_ew_resize());

    if enabled {
        handle.on_drag(
            PreviewTimelineDrag { target },
            |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
        )
    } else {
        handle
    }
}

pub(in crate::app) fn preview_play_button(state: &PreviewShellState) -> impl IntoElement {
    let enabled = preview_trim_enabled(state);
    let icon = if state.playback.is_playing() {
        assets::ICON_PAUSE
    } else {
        assets::ICON_PLAY
    };

    preview_tool_button(icon, false, enabled)
}

pub(in crate::app) fn centered_offset(container: f32, child: f32) -> f32 {
    ((container - child) / 2.0).max(0.0)
}

pub(in crate::app) fn timeline_fraction_from_percent(percent: f64) -> f32 {
    (percent / 100.0).clamp(0.0, 1.0) as f32
}

pub(in crate::app) fn timeline_slider_percent_from_bounds(
    position: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
) -> f64 {
    let width = bounds.size.width.as_f32();
    if width <= 0.0 {
        return 0.0;
    }

    let x = (position.x - bounds.origin.x).as_f32();
    f64::from((x / width).clamp(0.0, 1.0))
}
