use crate::app::accessibility::apply_accessible_slider;
use crate::app::components::{FrameTooltip, surface_scale};
use crate::app::layout::{
    PREVIEW_PLAYHEAD_HEIGHT, PREVIEW_TIMELINE_CONTROL_HEIGHT, PREVIEW_TIMELINE_HANDLE_WIDTH,
    PREVIEW_TIMELINE_INSET, PREVIEW_TRACK_HEIGHT,
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
    div, px, relative, size,
};

use std::{cell::Cell, rc::Rc};

const TRIM_GRIP_WIDTH: f32 = 6.0;
const TRIM_GRIP_HEIGHT: f32 = 18.0;
const TIMELINE_BAND_TOP: f32 = 12.0;
const TRIM_GRIP_TOP: f32 = 5.0;

#[derive(Clone, Debug)]
pub(super) struct PreviewTimelineDrag {
    pub(super) target: TimelineDragTarget,
    grab_offset: Rc<Cell<Pixels>>,
}

impl PreviewTimelineDrag {
    fn new(target: TimelineDragTarget) -> Self {
        Self {
            target,
            grab_offset: Rc::new(Cell::new(px(0.0))),
        }
    }
}

pub(in crate::app) struct PreviewTimelineDragPreview;

impl Render for PreviewTimelineDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(theme::ui_rem(0.0)).h(theme::ui_rem(0.0))
    }
}

pub(super) struct PreviewTimelineTrackBoundsProbe {
    pub(super) owner: Entity<FrameRoot>,
    surface_bounds: Rc<Cell<Bounds<Pixels>>>,
    scale: f32,
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
        let origin = self.surface_bounds.get().center();
        let bounds = Bounds::new(
            origin + (bounds.origin - origin) * self.scale,
            bounds.size.map(|dimension| dimension * self.scale),
        );
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
    surface_bounds: Rc<Cell<Bounds<Pixels>>>,
    progress: f32,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let palette = state.palette;
    let enabled = preview_trim_enabled(state);
    let duration = state.playback.duration();
    let current_time = state.playback.current_time();
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

    let scrub_owner = cx.entity();
    let track = div()
        .id("preview-timeline-track")
        .debug_selector(|| "preview-timeline-track".to_string())
        .relative()
        .h(theme::ui_rem(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w_full()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
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
                PreviewTimelineDrag::new(TimelineDragTarget::Scrub),
                move |_drag, _cursor_offset, window, cx| {
                    let position = window.mouse_position();
                    scrub_owner.update(cx, |root, cx| {
                        if let Some(bounds) = root.preview_ui.timeline_track_bounds {
                            let percent = timeline_slider_percent_from_bounds(position, bounds);
                            if root.apply_preview_timeline_drag_with_context(
                                TimelineDragTarget::Scrub,
                                percent,
                                cx,
                            ) {
                                cx.notify();
                            }
                        }
                    });
                    cx.new(|_| PreviewTimelineDragPreview)
                },
            )
        })
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<PreviewTimelineDrag>, _window, cx| {
                let drag = event.drag(cx).clone();
                let Some(bounds) = root.preview_ui.timeline_track_bounds else {
                    return;
                };
                let mut position = event.event.position;
                position.x += drag.grab_offset.get();
                let percent = timeline_slider_percent_from_bounds(position, bounds);
                if root.apply_preview_timeline_drag_with_context(drag.target, percent, cx) {
                    cx.notify();
                }
            },
        ))
        .capture_any_mouse_up(cx.listener(|root, event: &MouseUpEvent, _window, cx| {
            let drag = cx
                .active_drag()
                .and_then(|drag| drag.value.downcast_ref::<PreviewTimelineDrag>().cloned());
            if let (Some(drag), Some(bounds)) = (drag, root.preview_ui.timeline_track_bounds) {
                let mut position = event.position;
                position.x += drag.grab_offset.get();
                let percent = timeline_slider_percent_from_bounds(position, bounds);
                root.apply_preview_timeline_drag_with_context(drag.target, percent, cx);
            }
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
    // Rail, selection, playhead and endpoints share the exact same value range.
    .child(
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(theme::ui_rem(PREVIEW_TIMELINE_INSET))
            .right(theme::ui_rem(PREVIEW_TIMELINE_INSET))
            .child(timeline_band(start_fraction, end_fraction, palette))
            .child(PreviewTimelineTrackBoundsProbe {
                owner: cx.entity(),
                surface_bounds,
                scale: surface_scale(progress),
            })
            .child(
                div()
                    .id("preview-timeline-playhead")
                    .debug_selector(|| "preview-timeline-playhead".to_owned())
                    .absolute()
                    .left(relative(playhead_fraction))
                    .ml(theme::ui_rem(-1.0))
                    .top(theme::ui_rem(3.0))
                    .h(theme::ui_rem(PREVIEW_PLAYHEAD_HEIGHT))
                    .w(theme::ui_rem(2.0))
                    .rounded_full()
                    .bg(color(palette.text_primary))
                    .shadow(theme::shadows::ShadowLevel::Xs.shadows())
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(theme::ui_rem(-1.5))
                            .w(theme::ui_rem(5.0))
                            .h(theme::ui_rem(3.0))
                            .rounded(theme::ui_rem(2.0))
                            .bg(color(palette.text_primary)),
                    ),
            )
            .child(preview_timeline_handle(
                TimelineDragTarget::Start,
                start_fraction,
                state.playback.dragging() == Some(TimelineDragTarget::Start),
                enabled,
                state.playback.start_value(),
                duration,
                palette,
                window,
                cx,
            ))
            .child(preview_timeline_handle(
                TimelineDragTarget::End,
                end_fraction,
                state.playback.dragging() == Some(TimelineDragTarget::End),
                enabled,
                state.playback.end_value(),
                duration,
                palette,
                window,
                cx,
            )),
    )
}

fn timeline_band(start: f32, end: f32, palette: &'static theme::ThemePalette) -> gpui::Div {
    div()
        .absolute()
        .top(theme::ui_rem(TIMELINE_BAND_TOP))
        .h(theme::ui_rem(PREVIEW_TRACK_HEIGHT))
        .w_full()
        .rounded(theme::ui_rem(3.0))
        .bg(color(palette.input_background))
        .shadow(theme::shadows::recessed(palette))
        .child(
            div()
                .id("preview-timeline-selection")
                .debug_selector(|| "preview-timeline-selection".to_owned())
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(start))
                .right(relative(1.0 - end))
                .rounded_full()
                .bg(color(palette.text_muted)),
        )
}

fn timeline_grip(
    target: TimelineDragTarget,
    group: &'static str,
    dragging: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let start = target == TimelineDragTarget::Start;
    let id = if start {
        "preview-timeline-start-grip"
    } else {
        "preview-timeline-end-grip"
    };
    // The solid cap extends out of the kept range. Even a very short selection
    // retains two distinct grips, while the inner edges mark exact trim times.
    div()
        .id(id)
        .debug_selector(move || id.to_owned())
        .absolute()
        .top(theme::ui_rem(TRIM_GRIP_TOP))
        .left(theme::ui_rem(if start {
            PREVIEW_TIMELINE_HANDLE_WIDTH - TRIM_GRIP_WIDTH
        } else {
            0.0
        }))
        .w(theme::ui_rem(TRIM_GRIP_WIDTH))
        .h(theme::ui_rem(TRIM_GRIP_HEIGHT))
        .rounded(theme::ui_rem(3.0))
        .bg(color(palette.text_muted))
        .shadow(theme::shadows::ShadowLevel::Xxs.shadows())
        .group_hover_style(group, |style| style.bg(color(palette.text_primary)))
        .when(dragging, |grip| {
            grip.bg(color(palette.text_primary))
                .ring(px(2.0))
                .ring_color(color(palette.focus_ring.with_opacity(0.6)))
        })
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Endpoint rendering keeps shared geometry, pointer gestures and accessible actions together."
)]
pub(in crate::app) fn preview_timeline_handle(
    target: TimelineDragTarget,
    fraction: f32,
    dragging: bool,
    enabled: bool,
    value: f64,
    duration: f64,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let (handle_id, label) = match target {
        TimelineDragTarget::Start => ("preview-timeline-start-handle", "Trim start"),
        TimelineDragTarget::End => ("preview-timeline-end-handle", "Trim end"),
        TimelineDragTarget::Scrub => ("preview-timeline-scrub-handle", "Preview position"),
    };

    // Mouse focus can rerender between press and the drag threshold. Retain the
    // original grab point across those frames, not only within a rendered Div.
    let drag_key = match target {
        TimelineDragTarget::Start => "preview-start-grab",
        TimelineDragTarget::End => "preview-end-grab",
        TimelineDragTarget::Scrub => "preview-scrub-grab",
    };
    let drag = window
        .use_keyed_state(drag_key, cx, |_, _| PreviewTimelineDrag::new(target))
        .read(cx)
        .clone();
    let grab_offset = drag.grab_offset.clone();
    let drag_owner = cx.entity();
    let handle = div()
        .id(handle_id)
        .debug_selector(move || handle_id.to_string())
        .group(handle_id)
        .absolute()
        .top_0()
        // Keep the hit target and cap anchored to the current layout, including
        // the very first frame and a resize. No previous measurement is needed.
        .left(relative(fraction))
        .ml(theme::ui_rem(if target == TimelineDragTarget::Start {
            -PREVIEW_TIMELINE_HANDLE_WIDTH
        } else {
            0.0
        }))
        .h(theme::ui_rem(PREVIEW_TIMELINE_CONTROL_HEIGHT))
        .w(theme::ui_rem(PREVIEW_TIMELINE_HANDLE_WIDTH))
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |root, event: &MouseDownEvent, _window, cx| {
                if let Some(bounds) = root.preview_ui.timeline_track_bounds {
                    grab_offset
                        .set(bounds.left() + bounds.size.width * fraction - event.position.x);
                }
                cx.stop_propagation();
            }),
        )
        .when(enabled, gpui::Styled::cursor_ew_resize)
        .child(timeline_grip(target, handle_id, dragging, palette));

    let handle = if enabled {
        handle.on_drag(drag, move |drag, _cursor_offset, window, cx| {
            // Apply the threshold-crossing move in the same screen coordinates
            // as subsequent moves and release, avoiding layout pixel rounding.
            let mut position = window.mouse_position();
            position.x += drag.grab_offset.get();
            drag_owner.update(cx, |root, cx| {
                if let Some(bounds) = root.preview_ui.timeline_track_bounds {
                    let percent = timeline_slider_percent_from_bounds(position, bounds);
                    if root.apply_preview_timeline_drag_with_context(target, percent, cx) {
                        cx.notify();
                    }
                }
            });
            cx.new(|_| PreviewTimelineDragPreview)
        })
    } else {
        handle
    };

    let owner = cx.entity();
    let decrement_owner = owner.clone();

    let handle = apply_accessible_slider(
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
    );
    FrameTooltip::new(handle_id, format!("Drag to set {}", label.to_lowercase()))
        .for_control(handle, None, enabled, palette, window, cx)
        .absolute()
}

#[cfg(test)]
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
        "pageup" => current_time + large_step,
        "pagedown" => current_time - large_step,
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
