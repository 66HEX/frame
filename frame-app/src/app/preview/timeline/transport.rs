//! Contextual playback capsule and its precise trim surface, contained by the canvas.
use crate::app::TextInputCancel;
use crate::app::components::{
    FrameStyle, FrameTooltip, SurfaceOrigin, frame_button_base, frame_surface_motion, frame_toolbar,
};
use crate::app::input::FrameTextInputKind;
use crate::app::layout::{PREVIEW_TOOLBAR_OFFSET, PREVIEW_TRIM_PANEL_WIDTH};
use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::preview::panel::{PreviewShellState, PreviewTimecodeInputFocuses};
use crate::app::preview::state::PreviewTransportState;
use crate::app::preview::timeline::timecode::{PreviewTimecodeFieldSpec, preview_timecode_field};
use crate::app::preview::timeline::track::preview_timeline_track;
use crate::app::preview::timeline::view::{
    preview_play_button, preview_timeline_labels, preview_trim_enabled,
};
use crate::app::primitives::{ButtonVariant, color, icon_svg};
use crate::app::root::FrameRoot;
use crate::preview::{PreviewMediaKind, format_time};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{
    AnyElement, Bounds, Context, FocusHandle, KeyDownEvent, MouseButton, Pixels, Role,
    Subscription, WeakEntity, Window, div,
};
use std::{cell::Cell, rc::Rc};

struct TransportFocusState {
    capsule: FocusHandle,
    trigger: FocusHandle,
    panel: FocusHandle,
    _subscriptions: [Subscription; 2],
}

impl TransportFocusState {
    fn new(
        viewport: &FocusHandle,
        owner: WeakEntity<FrameRoot>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_owner = owner.clone();
        let on_focus = cx.on_focus_in(viewport, window, move |_, _, cx| {
            let _ = focus_owner.update(cx, |_, cx| cx.notify());
        });
        let on_blur = cx.on_focus_out(viewport, window, move |_, _, _, cx| {
            let _ = owner.update(cx, |_, cx| cx.notify());
        });
        Self {
            capsule: cx.focus_handle().tab_stop(false),
            trigger: cx.focus_handle(),
            panel: cx.focus_handle().tab_stop(false),
            _subscriptions: [on_focus, on_blur],
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Keep the contextual visibility, focus lifecycle and overlay event boundary together."
)]
pub(in crate::app) fn preview_transport(
    state: &PreviewShellState,
    inputs: PreviewTimecodeInputFocuses<'_>,
    viewport_focus: &FocusHandle,
    transport: PreviewTransportState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<AnyElement> {
    let owner = cx.weak_entity();
    let interaction = window.use_keyed_state("preview-transport-focus", cx, |window, cx| {
        TransportFocusState::new(viewport_focus, owner, window, cx)
    });
    let temporal = matches!(
        state.availability.media_kind,
        PreviewMediaKind::Video | PreviewMediaKind::Audio
    );
    let eligible = temporal
        && preview_trim_enabled(state)
        && !state.crop.crop_mode
        && !state.overlay.overlay_mode;
    let open = eligible && transport.open;
    let capsule_focus = interaction.read(cx).capsule.clone();
    let trigger_focus = interaction.read(cx).trigger.clone();
    let panel_focus = interaction.read(cx).panel.clone();
    // Selection and availability can remove the surface without a close click.
    // Its local focus handles outlive the DOM, so restore a mounted ancestor first.
    if !eligible
        && (capsule_focus.contains_focused(window, cx) || panel_focus.contains_focused(window, cx))
    {
        viewport_focus.focus(window, cx);
    } else if !open && panel_focus.contains_focused(window, cx) {
        trigger_focus.focus(window, cx);
    }
    let visible = eligible
        && (transport.hovered
            || viewport_focus.contains_focused(window, cx)
            || open
            || state.playback.dragging().is_some());
    let progress = surface_motion("preview-transport-motion", visible, window, cx);
    let panel_progress = surface_motion("preview-trim-panel-motion", open, window, cx);
    if !eligible || motion_is_hidden(progress) {
        return None;
    }
    let mut control_state = state.clone();
    control_state.availability.trim_disabled = !visible;
    let panel_bounds = Rc::new(Cell::new(Bounds::default()));
    let outside_panel_bounds = panel_bounds.clone();
    let outside_capsule = capsule_focus.clone();
    let outside_panel = panel_focus.clone();
    let outside_viewport = viewport_focus.clone();
    let capsule = frame_toolbar(state.palette)
        .id("preview-transport-capsule")
        .when(visible, |capsule| {
            capsule.track_focus(&capsule_focus).tab_stop(false)
        })
        .on_mouse_down_out(
            cx.listener(move |root, event: &gpui::MouseDownEvent, window, cx| {
                if !root.preview_ui.transport.open
                    || outside_panel_bounds.get().contains(&event.position)
                {
                    return;
                }
                root.end_preview_timeline_drag_with_context(cx);
                root.preview_ui.transport.open = false;
                let capsule = outside_capsule.clone();
                let panel = outside_panel.clone();
                let viewport = outside_viewport.clone();
                // The real pointer target may claim focus later in this event.
                cx.defer_in(window, move |_, window, cx| {
                    if capsule.contains_focused(window, cx) || panel.contains_focused(window, cx) {
                        viewport.focus(window, cx);
                    }
                });
                cx.notify();
            }),
        )
        .mr(theme::ui_rem(44.0))
        .flex_shrink_0()
        .when(visible, |capsule| {
            capsule.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        })
        .child(preview_play_button(&control_state, window, cx))
        .child(trim_trigger(
            state,
            open,
            visible,
            &trigger_focus,
            &panel_focus,
            window,
            cx,
        ));
    let mut surface = div()
        .id("preview-transport")
        .debug_selector(|| "preview-transport".to_owned())
        .absolute()
        .left(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .right(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .bottom(theme::ui_rem(PREVIEW_TOOLBAR_OFFSET))
        .flex()
        .flex_col()
        .items_center()
        .gap(theme::ui_rem(8.0))
        .opacity(progress)
        .disabled(!visible);
    if !motion_is_hidden(panel_progress) {
        let mut panel_state = state.clone();
        panel_state.availability.trim_disabled = !open;
        surface = surface.child(trim_panel(
            &panel_state,
            inputs,
            &panel_focus,
            open,
            panel_progress,
            panel_bounds,
            window,
            cx,
        ));
    }
    let escape_focus = trigger_focus.clone();
    surface = surface
        .child(capsule)
        .on_key_down(cx.listener(move |root, event: &KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape" && root.preview_ui.transport.open {
                close_trim(root, &escape_focus, window, cx);
                cx.stop_propagation();
            }
        }))
        .capture_action(cx.listener(move |root, _: &TextInputCancel, window, cx| {
            if !root.preview_ui.transport.open {
                return;
            }
            let cancelled = root
                .focused_text_input_kind(window)
                .is_some_and(|kind| root.cancel_preview_timecode_input(kind));
            if cancelled {
                cx.notify();
            } else {
                close_trim(root, &trigger_focus, window, cx);
            }
            cx.stop_propagation();
        }));
    Some(surface.into_any_element())
}

fn close_trim(
    root: &mut FrameRoot,
    trigger: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) {
    // Move focus before unmounting inputs so their ordinary blur commits still run.
    trigger.focus(window, cx);
    root.end_preview_timeline_drag_with_context(cx);
    root.preview_ui.transport.open = false;
    cx.notify();
}

fn trim_trigger(
    state: &PreviewShellState,
    open: bool,
    enabled: bool,
    trigger_focus: &FocusHandle,
    panel_focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let time = format_time(state.playback.current_time());
    let time = time.split('.').next().unwrap_or(&time).to_string();
    let button = frame_button_base(
        "preview-trim-trigger",
        "Trim preview",
        ButtonVariant::Ghost,
        open,
        enabled,
        theme::metrics::ControlSize::Default,
        state.palette,
        window,
        cx,
    )
    .when(enabled, |button| button.track_focus(trigger_focus))
    .aria_expanded(open)
    .font_features(assets::frame_tabular_number_font_features())
    .child(time)
    .child(icon_svg(
        assets::ICON_UNFOLD_MORE,
        12.0,
        color(state.palette.text_muted),
    ));
    let trigger = trigger_focus.clone();
    let panel = panel_focus.clone();
    FrameTooltip::new(
        "preview-trim-trigger",
        format!(
            "Trim and playback · Duration {}",
            preview_timeline_labels(state).duration
        ),
    )
    .for_control(
        button,
        Some(trigger_focus),
        enabled,
        state.palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _, window, cx| {
        if root.preview_ui.transport.open {
            close_trim(root, &trigger, window, cx);
        } else {
            root.preview_ui.transport.open = true;
            let panel = panel.clone();
            cx.defer_in(window, move |_, window, cx| panel.focus(window, cx));
            cx.notify();
        }
    }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "The trim surface shares state, focus, animation, and measured outside-click bounds."
)]
fn trim_panel(
    state: &PreviewShellState,
    inputs: PreviewTimecodeInputFocuses<'_>,
    focus: &FocusHandle,
    enabled: bool,
    progress: f32,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> AnyElement {
    let palette = state.palette;
    let track_surface_bounds = bounds.clone();
    let panel = div()
        .id("preview-trim-panel")
        .debug_selector(|| "preview-trim-panel".to_owned())
        .on_prepaint(move |info, _, _| bounds.set(info.bounds))
        .role(Role::Group)
        .aria_label("Trim preview")
        .when(enabled, |panel| panel.track_focus(focus).tab_stop(false))
        .w_full()
        .max_w(theme::ui_rem(PREVIEW_TRIM_PANEL_WIDTH))
        .min_w_0()
        .p(theme::ui_rem(8.0))
        .flex()
        .flex_col()
        .gap(theme::ui_rem(4.0))
        .popover_surface(palette)
        .opacity(progress)
        .when(enabled, |panel| {
            panel
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        })
        .child(preview_timeline_track(
            state,
            track_surface_bounds,
            progress,
            window,
            cx,
        ))
        .child(
            div()
                .flex()
                .gap(theme::ui_rem(8.0))
                .child(preview_timecode_field(
                    PreviewTimecodeFieldSpec {
                        label: "Start",
                        value: inputs.start_value.to_string(),
                        enabled,
                        kind: Some(FrameTextInputKind::PreviewStartTime),
                        focus: inputs.start,
                    },
                    palette,
                    window,
                    cx,
                ))
                .child(preview_timecode_field(
                    PreviewTimecodeFieldSpec {
                        label: "End",
                        value: inputs.end_value.to_string(),
                        enabled,
                        kind: Some(FrameTextInputKind::PreviewEndTime),
                        focus: inputs.end,
                    },
                    palette,
                    window,
                    cx,
                )),
        );
    div()
        .w_full()
        .max_w(theme::ui_rem(PREVIEW_TRIM_PANEL_WIDTH))
        .child(frame_surface_motion(panel, progress, SurfaceOrigin::Center))
        .into_any_element()
}
