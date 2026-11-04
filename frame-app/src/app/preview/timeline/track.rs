use crate::app::accessibility::apply_accessible_slider;
use crate::app::components::{frame_range_track, frame_slider_fraction_from_bounds};
use crate::app::layout::{
    PREVIEW_PLAYHEAD_HEIGHT, PREVIEW_TIMELINE_CONTROL_HEIGHT, PREVIEW_TIMELINE_HANDLE_WIDTH,
    PREVIEW_TRACK_HEIGHT,
};
use crate::app::preview::panel::PreviewShellState;
use crate::app::preview::timeline::view::preview_trim_enabled;
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::numeric::unit_f64_to_f32;
use crate::preview::{TimelineDragTarget, format_time};
use crate::theme;
use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, DragMoveEvent, Element, ElementId, Entity, GlobalElementId, IntoElement,
    LayoutId, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Position, Render, Style, Window,
    canvas, div, px, relative, size,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PreviewTimelineDrag {
    pub(super) target: TimelineDragTarget,
}

pub(in crate::app) struct PreviewTimelineDragPreview;

impl Render for PreviewTimelineDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(theme::ui_rem(0.0)).h(theme::ui_rem(0.0))
    }
}

pub(super) struct PreviewTimelineTrackBoundsProbe {
    pub(super) owner: Entity<FrameRoot>,
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

#[expect(
    clippy::too_many_lines,
    reason = "Timeline track composes scrub, trim handles, a11y metadata, and drag/key handling together."
)]
pub(in crate::app) fn preview_timeline_track(
    state: &PreviewShellState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let measured_width = window.use_keyed_state("timeline-hit-width", cx, |_, _| 0.0_f32);
    let track_width = *measured_width.read(cx);
    let palette = state.palette;
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
    let start_target = timeline_hit_target(start_fraction, end_fraction, track_width, false);
    let end_target = timeline_hit_target(start_fraction, end_fraction, track_width, true);

    let track = div()
        .id("preview-timeline-track")
        .debug_selector(|| "preview-timeline-track".to_string())
        .relative()
        .h(theme::ui_rem(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w_full()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
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
            |root, event: &DragMoveEvent<PreviewTimelineDrag>, window, cx| {
                let drag = *event.drag(cx);
                let percent = frame_slider_fraction_from_bounds(
                    event.event.position,
                    event.bounds,
                    window.rem_size(),
                );
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
        palette,
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
        frame_range_track(start_fraction, end_fraction, palette)
            .absolute()
            .left_0()
            .right_0()
            .top(theme::ui_rem(track_top)),
    )
    // Position measurements, playhead, and trim targets share the inset value range.
    .child(
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(theme::ui_rem(theme::RANGE_TRACK_INSET))
            .right(theme::ui_rem(theme::RANGE_TRACK_INSET))
            .child(PreviewTimelineTrackBoundsProbe { owner: cx.entity() })
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let scale = window.rem_size().as_f32() / crate::appearance::BASE_REM_PX;
                        measured_width.update(cx, |width, cx| {
                            let next = bounds.size.width.as_f32() / scale;
                            if (*width - next).abs() > 0.01 {
                                *width = next;
                                cx.notify();
                            }
                        });
                    },
                    |_, (), _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(playhead_fraction))
                    .ml(px(-0.5))
                    .top(theme::ui_rem(playhead_top))
                    .h(theme::ui_rem(PREVIEW_PLAYHEAD_HEIGHT))
                    .w(px(1.0))
                    .bg(color(palette.text_primary)),
            )
            .child(preview_timeline_handle(
                TimelineDragTarget::Start,
                start_target,
                enabled,
                state.playback.start_value(),
                duration,
                palette,
                cx,
            ))
            .child(preview_timeline_handle(
                TimelineDragTarget::End,
                end_target,
                enabled,
                state.playback.end_value(),
                duration,
                palette,
                cx,
            )),
    )
}

pub(in crate::app) fn preview_timeline_handle(
    target: TimelineDragTarget,
    hit_target: (f32, f32),
    enabled: bool,
    value: f64,
    duration: f64,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let (handle_id, label) = match target {
        TimelineDragTarget::Start => ("preview-timeline-start-handle", "Trim start"),
        TimelineDragTarget::End => ("preview-timeline-end-handle", "Trim end"),
        TimelineDragTarget::Scrub => ("preview-timeline-scrub-handle", "Preview position"),
    };

    let handle = div()
        .id(handle_id)
        .debug_selector(move || handle_id.to_string())
        .absolute()
        .top_0()
        .left(theme::ui_rem(hit_target.0))
        .h(theme::ui_rem(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w(theme::ui_rem(hit_target.1))
        .rounded(theme::ui_rem(theme::RADIUS_SM))
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
        palette,
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

// Split nearby endpoints at their midpoint, moving extra hit space outwards.
// Limit overhang to half a target so adjacent time fields and Play stay usable.
pub(super) fn timeline_hit_target(start: f32, end: f32, width: f32, is_end: bool) -> (f32, f32) {
    let target = PREVIEW_TIMELINE_HANDLE_WIDTH;
    let midpoint = start.midpoint(end) * width;
    let (left, right) = if is_end {
        let left = end.mul_add(width, -(target / 2.0)).max(midpoint);
        (left, (left + target).min(width + target / 2.0))
    } else {
        let right = start.mul_add(width, target / 2.0).min(midpoint);
        ((right - target).max(-target / 2.0), right)
    };
    (left, (right - left).max(0.0))
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
