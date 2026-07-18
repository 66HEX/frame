use super::*;
use crate::numeric::unit_f64_to_f32;

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

struct PreviewTimelineTrackBoundsProbe {
    owner: Entity<FrameRoot>,
}

impl IntoElement for PreviewTimelineTrackBoundsProbe {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PreviewTimelineTrackBoundsProbe {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            position: Position::Absolute,
            size: size(relative(1.0).into(), relative(1.0).into()),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            ..Style::default()
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.owner.update(cx, |root, _cx| {
            root.set_preview_timeline_track_bounds(bounds);
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
}

pub(in crate::app) fn preview_timeline(
    state: &PreviewShellState,
    inputs: PreviewTimecodeInputFocuses<'_>,
    window: &mut Window,
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
                    PreviewTimecodeFieldSpec {
                        label: "Start time",
                        value: inputs.start_value.to_string(),
                        enabled: trim_enabled,
                        width: 128.0,
                        kind: Some(FrameTextInputKind::PreviewStartTime),
                        focus: inputs.start,
                    },
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
                .gap(px(6.0))
                .child(preview_timeline_label("Trim"))
                .child(preview_timeline_track(state, cx)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(preview_timeline_label(" "))
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

pub(in crate::app) struct PreviewTimecodeFieldSpec<'a> {
    label: &'static str,
    value: String,
    enabled: bool,
    width: f32,
    kind: Option<FrameTextInputKind>,
    focus: Option<&'a FocusHandle>,
}

pub(in crate::app) fn preview_timecode_field(
    spec: PreviewTimecodeFieldSpec<'_>,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    let PreviewTimecodeFieldSpec {
        label,
        value,
        enabled,
        width,
        kind,
        focus,
    } = spec;
    let field = if let (Some(kind), Some(focus)) = (kind, focus) {
        frame_text_input(
            FrameTextInputSpec {
                id: match kind {
                    FrameTextInputKind::PreviewStartTime => "preview-start-time",
                    FrameTextInputKind::PreviewEndTime => "preview-end-time",
                    _ => "preview-timecode",
                },
                value: &value,
                placeholder: "--:--:--.---",
                disabled: !enabled,
                focus: Some(focus),
                kind,
            },
            window,
            cx,
        )
        .font_features(assets::frame_tabular_number_font_features())
        .into_any_element()
    } else {
        div()
            .w_full()
            .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
            .flex()
            .items_center()
            .text_size(px(theme::TEXT_LABEL_SIZE))
            .text_color(color(theme::FOREGROUND))
            .font_features(assets::frame_tabular_number_font_features())
            .child(value)
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(preview_timeline_label(label))
        .child(
            div()
                .w(px(width))
                .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
                .child(field),
        )
}

pub(in crate::app) fn preview_timeline_label(label: &'static str) -> gpui::Div {
    div()
        .h(px(12.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(theme::FRAME_GRAY_600))
        .child(theme::ui_text(label))
}

#[expect(
    clippy::too_many_lines,
    reason = "Timeline track composes scrub, trim handles, a11y metadata, and drag/key handling together."
)]
pub(in crate::app) fn preview_timeline_track(
    state: &PreviewShellState,
    cx: &Context<FrameRoot>,
) -> impl IntoElement {
    let enabled = preview_trim_enabled(state);
    let duration = state.playback.duration();
    let current_time = state.playback.current_time();
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

    let track = div()
        .id("preview-timeline-track")
        .relative()
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w_full()
        .rounded(px(theme::RADIUS_SM))
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, gpui::Styled::cursor_pointer)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|root, event: &MouseDownEvent, _window, cx| {
                if root.commit_preview_timeline_seek_at_position_with_context(event.position, cx) {
                    cx.notify();
                }
            }),
        )
        .when(enabled, |this| {
            this.on_drag(
                PreviewTimelineDrag {
                    target: TimelineDragTarget::Scrub,
                },
                |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
            )
        })
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewTimelineDrag>, _window, cx| {
                let drag = *event.drag(cx);
                let percent =
                    timeline_slider_percent_from_bounds(event.event.position, event.bounds);
                if root.apply_preview_timeline_drag_with_context(drag.target, percent, cx) {
                    cx.notify();
                }
            },
        ))
        .capture_any_mouse_up(cx.listener(|root, _event: &MouseUpEvent, _window, cx| {
            if root.end_preview_timeline_drag_with_context(cx) {
                cx.notify();
            }
        }));

    let owner = cx.entity();
    let decrement_owner = owner.clone();

    apply_accessible_slider(
        track,
        "Preview position",
        enabled,
        current_time,
        0.0,
        duration,
        format_time(current_time),
    )
    .on_a11y_action(gpui::AccessibleAction::Increment, move |_, _window, cx| {
        owner.update(cx, |root, cx| {
            if root.adjust_preview_timeline_from_keyboard_with_context(
                TimelineDragTarget::Scrub,
                "right",
                cx,
            ) {
                cx.notify();
            }
        });
    })
    .on_a11y_action(gpui::AccessibleAction::Decrement, move |_, _window, cx| {
        decrement_owner.update(cx, |root, cx| {
            if root.adjust_preview_timeline_from_keyboard_with_context(
                TimelineDragTarget::Scrub,
                "left",
                cx,
            ) {
                cx.notify();
            }
        });
    })
    .on_key_down(
        cx.listener(|root, event: &gpui::KeyDownEvent, _window, cx| {
            if !timeline_keyboard_key_is_handled(event.keystroke.key.as_str()) {
                return;
            }
            if root.adjust_preview_timeline_from_keyboard_with_context(
                TimelineDragTarget::Scrub,
                event.keystroke.key.as_str(),
                cx,
            ) {
                cx.notify();
            }
            cx.stop_propagation();
        }),
    )
    .child(
        div()
            .absolute()
            .left_0()
            .right_0()
            .top(px(track_top))
            .h(px(PREVIEW_TRACK_HEIGHT))
            .rounded(px(1.5))
            .bg(color(theme::FRAME_GRAY_100)),
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
    .child(PreviewTimelineTrackBoundsProbe { owner: cx.entity() })
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
        state.playback.start_value(),
        duration,
        cx,
    ))
    .child(preview_timeline_handle(
        TimelineDragTarget::End,
        end_fraction,
        enabled,
        state.playback.end_value(),
        duration,
        cx,
    ))
}

pub(in crate::app) fn preview_timeline_handle(
    target: TimelineDragTarget,
    fraction: f32,
    enabled: bool,
    value: f64,
    duration: f64,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let (handle_id, label) = match target {
        TimelineDragTarget::Start => ("preview-timeline-start-handle", "Trim start"),
        TimelineDragTarget::End => ("preview-timeline-end-handle", "Trim end"),
        TimelineDragTarget::Scrub => ("preview-timeline-scrub-handle", "Preview position"),
    };

    let handle = div()
        .id(handle_id)
        .absolute()
        .top_0()
        .left(relative(fraction))
        .ml(px(-(PREVIEW_TIMELINE_HANDLE_WIDTH / 2.0)))
        .h(px(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w(px(PREVIEW_TIMELINE_HANDLE_WIDTH))
        .rounded(px(theme::RADIUS_SM))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .when(enabled, gpui::Styled::cursor_ew_resize);

    let handle = if enabled {
        handle.on_drag(
            PreviewTimelineDrag { target },
            |_drag, _position, _window, cx| cx.new(|_| PreviewTimelineDragPreview),
        )
    } else {
        handle
    };

    let owner = cx.entity();
    let decrement_owner = owner.clone();

    apply_accessible_slider(
        handle,
        label,
        enabled,
        value,
        0.0,
        duration,
        format_time(value),
    )
    .on_a11y_action(gpui::AccessibleAction::Increment, move |_, _window, cx| {
        owner.update(cx, |root, cx| {
            if root.adjust_preview_timeline_from_keyboard_with_context(target, "right", cx) {
                cx.notify();
            }
        });
    })
    .on_a11y_action(gpui::AccessibleAction::Decrement, move |_, _window, cx| {
        decrement_owner.update(cx, |root, cx| {
            if root.adjust_preview_timeline_from_keyboard_with_context(target, "left", cx) {
                cx.notify();
            }
        });
    })
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
            if !timeline_keyboard_key_is_handled(event.keystroke.key.as_str()) {
                return;
            }
            if root.adjust_preview_timeline_from_keyboard_with_context(
                target,
                event.keystroke.key.as_str(),
                cx,
            ) {
                cx.notify();
            }
            cx.stop_propagation();
        }),
    )
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
        false,
        enabled,
        window,
        cx,
    )
    .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
        if root.toggle_preview_playback_with_context(cx) {
            cx.notify();
        }
    }))
}

pub(in crate::app) fn centered_offset(container: f32, child: f32) -> f32 {
    ((container - child) / 2.0).max(0.0)
}

pub(in crate::app) fn timeline_fraction_from_percent(percent: f64) -> f32 {
    unit_f64_to_f32(percent / 100.0)
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

pub(in crate::app) fn timeline_keyboard_time_for_key(
    current_time: f64,
    duration: f64,
    key: &str,
) -> Option<f64> {
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }

    let small_step = (duration / 100.0).clamp(0.1, 1.0);
    let large_step = (duration / 10.0).clamp(1.0, 10.0);
    let next = match key {
        "left" | "down" => current_time - small_step,
        "right" | "up" => current_time + small_step,
        "pageup" => current_time - large_step,
        "pagedown" => current_time + large_step,
        "home" => 0.0,
        "end" => duration,
        _ => return None,
    };

    Some(next.clamp(0.0, duration))
}

pub(in crate::app) fn timeline_keyboard_key_is_handled(key: &str) -> bool {
    matches!(
        key,
        "left" | "down" | "right" | "up" | "pageup" | "pagedown" | "home" | "end"
    )
}
