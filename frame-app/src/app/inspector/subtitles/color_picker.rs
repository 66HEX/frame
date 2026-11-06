use crate::app::accessibility::{
    apply_accessible_slider, apply_accessible_slider_with_focus, focus_visible_ring,
    handle_modal_tab_navigation,
};
use crate::app::components::{
    FRAME_COLOR_PICKER_HUE_VISUAL_HEIGHT, FRAME_COLOR_PICKER_SV_HEIGHT,
    frame_color_picker_hue_handle, frame_color_picker_hue_track, frame_color_picker_panel,
    frame_color_picker_sv_canvas, frame_color_select_value, frame_select_trigger_content,
    frame_select_trigger_content_with_focus,
};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::shared::settings_field_label;
use crate::app::inspector::subtitles::color::{
    settings_subtitle_hue_label, settings_subtitle_sv_label, settings_subtitle_sv_value,
    subtitle_hue_hsv_for_key, subtitle_sv_hsv_for_key,
};
use crate::app::inspector::subtitles::popover::subtitle_popover_progress;
use crate::app::inspector::subtitles::state::{
    SettingsSubtitleColorDrag, SettingsSubtitleColorDragKind, SettingsSubtitleColorPopoverFocuses,
    SettingsSubtitleColorTarget, SettingsSubtitleHsv, SettingsSubtitlePopover,
};
use crate::app::inspector::subtitles::view::SettingsSubtitleColorFieldSpec;
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::*;
use gpui::{
    App, Bounds, ClickEvent, Context, DragMoveEvent, Element, ElementId, Entity, FocusHandle,
    GlobalElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent, Pixels, Position, Render,
    Style, Window, deferred, div, relative, size,
};

pub(super) struct SettingsSubtitleColorDragPreview;

impl Render for SettingsSubtitleColorDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub(super) struct SettingsSubtitleColorBoundsProbe {
    pub(super) owner: Entity<FrameRoot>,
    pub(super) target: SettingsSubtitleColorTarget,
    pub(super) kind: SettingsSubtitleColorDragKind,
}

impl IntoElement for SettingsSubtitleColorBoundsProbe {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SettingsSubtitleColorBoundsProbe {
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
            root.set_subtitle_color_picker_bounds(self.target, self.kind, bounds);
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
    reason = "The color field keeps trigger behavior, draft color setup, and popover mounting together."
)]
pub(super) fn settings_subtitle_color_field(
    spec: SettingsSubtitleColorFieldSpec<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let SettingsSubtitleColorFieldSpec {
        label,
        id,
        value,
        disabled,
        target,
        popover_focuses,
        focus,
        active_popover,
        rendered_popover,
        draft,
        hsv,
        palette,
    } = spec;
    let popover = match target {
        SettingsSubtitleColorTarget::Font => SettingsSubtitlePopover::FontColor,
        SettingsSubtitleColorTarget::Outline => SettingsSubtitlePopover::OutlineColor,
    };
    let enabled = !disabled;
    let click_value = value.clone();
    let key_value = value.clone();
    let trigger_content = frame_color_select_value(&value, palette);
    let trigger = if let Some(focus) = popover_focuses.trigger {
        frame_select_trigger_content_with_focus(
            id,
            label,
            trigger_content,
            enabled,
            active_popover == Some(popover),
            focus,
            palette,
            window,
            cx,
        )
    } else {
        frame_select_trigger_content(
            id,
            label,
            trigger_content,
            enabled,
            active_popover == Some(popover),
            palette,
            window,
            cx,
        )
    };
    let click_first_focus = popover_focuses.sv.cloned();
    let key_first_focus = popover_focuses.sv.cloned();
    let anchor = crate::app::components::PopoverAnchor::default();
    let measured_anchor = anchor.clone();

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label(label, palette))
        .child(
            trigger
                .on_prepaint(move |info, _, _| measured_anchor.measure(info.bounds))
                .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    if !enabled {
                        return;
                    }
                    root.toggle_subtitle_color_popover(popover, target, &click_value);
                    if root.subtitle_ui.popover == Some(popover)
                        && let Some(focus) = click_first_focus.as_ref()
                    {
                        focus.focus(window, cx);
                    }
                    cx.notify();
                }))
                .on_key_down(
                    cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                        if !enabled {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "down" | "up" => {
                                root.open_subtitle_color_popover(popover, target, &key_value);
                                if let Some(focus) = key_first_focus.as_ref() {
                                    focus.focus(window, cx);
                                }
                                cx.stop_propagation();
                                cx.notify();
                            }
                            "enter" | "space" => {
                                root.toggle_subtitle_color_popover(popover, target, &key_value);
                                if root.subtitle_ui.popover == Some(popover)
                                    && let Some(focus) = key_first_focus.as_ref()
                                {
                                    focus.focus(window, cx);
                                }
                                cx.stop_propagation();
                                cx.notify();
                            }
                            "escape" => {
                                root.close_subtitle_popover();
                                cx.stop_propagation();
                                cx.notify();
                            }
                            _ => {}
                        }
                    }),
                ),
        );

    let open = active_popover == Some(popover);
    let progress = subtitle_popover_progress(popover, open, window, cx);
    if rendered_popover == Some(popover) {
        field = field.child(
            deferred(crate::app::components::frame_floating_popover(
                settings_subtitle_color_picker(
                    target,
                    hsv,
                    draft,
                    focus,
                    popover_focuses,
                    progress,
                    palette,
                    window,
                    cx,
                ),
                anchor,
                progress,
                0.0,
                window,
            ))
            .priority(10),
        );
    }

    field
}

#[expect(
    clippy::too_many_arguments,
    reason = "The color picker needs color state, input focus, popover focus handles, animation progress, and render context."
)]
pub(super) fn settings_subtitle_color_picker(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    draft: &str,
    focus: Option<&FocusHandle>,
    popover_focuses: SettingsSubtitleColorPopoverFocuses<'_>,
    progress: f32,
    palette: &'static theme::ThemePalette,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let input_kind = match target {
        SettingsSubtitleColorTarget::Font => FrameTextInputKind::SubtitleFontColorHex,
        SettingsSubtitleColorTarget::Outline => FrameTextInputKind::SubtitleOutlineColorHex,
    };

    let panel_id = match target {
        SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-picker",
        SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-picker",
    };
    let panel = frame_color_picker_panel(
        progress,
        settings_subtitle_sv_square(target, hsv, popover_focuses.sv, palette, cx),
        settings_subtitle_hue_slider(target, hsv, popover_focuses.hue, palette, cx),
        frame_text_input(
            FrameTextInputSpec {
                id: match target {
                    SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-hex",
                    SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-hex",
                },
                value: draft,
                placeholder: "#FFFFFF",
                disabled: false,
                focus,
                kind: input_kind,
            },
            palette,
            window,
            cx,
        ),
        palette,
    )
    .id(panel_id)
    .overflow_y_scroll()
    .debug_selector(|| panel_id.to_string());

    let Some(panel_focus) = popover_focuses.panel else {
        return panel;
    };
    let Some(first_focus) = popover_focuses.sv else {
        return panel;
    };
    let Some(last_focus) = focus else {
        return panel;
    };

    let first_focus = first_focus.clone();
    let last_focus = last_focus.clone();
    let trigger_focus = popover_focuses.trigger.cloned();
    panel
        .track_focus(panel_focus)
        .tab_stop(false)
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                if handle_modal_tab_navigation(event, &first_focus, &last_focus, window, cx) {
                    return;
                }
                if event.keystroke.key.as_str() == "escape" {
                    root.close_subtitle_popover();
                    if let Some(focus) = trigger_focus.as_ref() {
                        focus.focus(window, cx);
                    }
                    cx.stop_propagation();
                    cx.notify();
                }
            }),
        )
}

pub(super) fn settings_subtitle_sv_square(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let drag = SettingsSubtitleColorDrag {
        target,
        kind: SettingsSubtitleColorDragKind::SaturationValue,
        base_hsv: hsv,
    };
    let square = div()
        .id(match target {
            SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-sv",
            SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-sv",
        })
        .role(gpui::Role::ColorWell)
        .aria_label(settings_subtitle_sv_label(target))
        .aria_value(settings_subtitle_sv_value(hsv))
        .relative()
        .h(theme::ui_rem(FRAME_COLOR_PICKER_SV_HEIGHT))
        .w_full()
        .overflow_hidden()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.fill_subtle))
        .cursor_crosshair()
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |root, event: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                if root.commit_subtitle_color_at_position(
                    target,
                    SettingsSubtitleColorDragKind::SaturationValue,
                    event.position,
                ) {
                    cx.notify();
                }
            }),
        )
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<SettingsSubtitleColorDrag>, _window, cx| {
                let drag = *event.drag(cx);
                if drag.kind != SettingsSubtitleColorDragKind::SaturationValue {
                    return;
                }
                if root.commit_subtitle_color_drag_at_position(drag, event.event.position) {
                    cx.notify();
                }
            },
        ))
        .child(SettingsSubtitleColorBoundsProbe {
            owner: cx.entity(),
            target,
            kind: SettingsSubtitleColorDragKind::SaturationValue,
        })
        .child(frame_color_picker_sv_canvas(hsv.h, hsv.s, hsv.v))
        .on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsSubtitleColorDragPreview)
        })
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                let Some(next_hsv) = subtitle_sv_hsv_for_key(hsv, event.keystroke.key.as_str())
                else {
                    return;
                };
                if root.commit_subtitle_hsv_color(target, next_hsv) {
                    cx.notify();
                }
                cx.stop_propagation();
            }),
        );

    if let Some(focus) = focus {
        square
            .track_focus(focus)
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        square
            .focusable()
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "The hue slider combines pointer, accessibility increment/decrement, and keyboard behavior for one control."
)]
pub(super) fn settings_subtitle_hue_slider(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let drag = SettingsSubtitleColorDrag {
        target,
        kind: SettingsSubtitleColorDragKind::Hue,
        base_hsv: hsv,
    };
    let owner = cx.entity();
    let decrement_owner = owner.clone();

    let id = match target {
        SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-hue",
        SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-hue",
    };
    let slider = div()
        .id(id)
        .debug_selector(move || id.to_string())
        .relative()
        .h(theme::ui_rem(FRAME_COLOR_PICKER_HUE_VISUAL_HEIGHT))
        .w_full()
        .cursor_default()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |root, event: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                if root.commit_subtitle_color_at_position(
                    target,
                    SettingsSubtitleColorDragKind::Hue,
                    event.position,
                ) {
                    cx.notify();
                }
            }),
        )
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<SettingsSubtitleColorDrag>, _window, cx| {
                let drag = *event.drag(cx);
                if drag.kind != SettingsSubtitleColorDragKind::Hue {
                    return;
                }
                if root.commit_subtitle_color_drag_at_position(drag, event.event.position) {
                    cx.notify();
                }
            },
        ))
        .child(SettingsSubtitleColorBoundsProbe {
            owner: cx.entity(),
            target,
            kind: SettingsSubtitleColorDragKind::Hue,
        })
        .child(frame_color_picker_hue_track())
        .child(frame_color_picker_hue_handle(hsv.h, palette))
        .on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsSubtitleColorDragPreview)
        });

    let slider = if let Some(focus) = focus {
        apply_accessible_slider_with_focus(
            slider,
            settings_subtitle_hue_label(target),
            true,
            hsv.h,
            0.0,
            360.0,
            format!("{:.0} degrees", hsv.h.round()),
            focus,
            palette,
        )
    } else {
        apply_accessible_slider(
            slider,
            settings_subtitle_hue_label(target),
            true,
            hsv.h,
            0.0,
            360.0,
            format!("{:.0} degrees", hsv.h.round()),
            palette,
        )
    };

    slider
        .on_a11y_action(gpui::AccessibleAction::Increment, move |_, _window, cx| {
            owner.update(cx, move |root, cx| {
                if let Some(next_hsv) = subtitle_hue_hsv_for_key(hsv, "right")
                    && root.commit_subtitle_hsv_color(target, next_hsv)
                {
                    cx.notify();
                }
            });
        })
        .on_a11y_action(gpui::AccessibleAction::Decrement, move |_, _window, cx| {
            decrement_owner.update(cx, move |root, cx| {
                if let Some(next_hsv) = subtitle_hue_hsv_for_key(hsv, "left")
                    && root.commit_subtitle_hsv_color(target, next_hsv)
                {
                    cx.notify();
                }
            });
        })
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                let Some(next_hsv) = subtitle_hue_hsv_for_key(hsv, event.keystroke.key.as_str())
                else {
                    return;
                };
                if root.commit_subtitle_hsv_color(target, next_hsv) {
                    cx.notify();
                }
                cx.stop_propagation();
            }),
        )
}
