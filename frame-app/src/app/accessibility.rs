use super::{
    App, Context, FocusHandle, FrameRoot, StatefulInteractiveElement, Styled, Window, color, px,
    theme,
};
use gpui::{InteractiveElement, Orientation, Role, SharedString, StyleRefinement, Toggled};
use std::collections::HashMap;

const FOCUS_RING_WIDTH: f32 = 3.0;
const FOCUS_RING_ALPHA: f32 = 0.55;
pub(in crate::app) const APP_ROOT_FOCUS_ID: &str = "app-root";

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(in crate::app) enum FrameFocusKey {
    Control(String),
}

#[derive(Default)]
pub(in crate::app) struct FrameFocusRegistry {
    handles: HashMap<FrameFocusKey, FrameFocusEntry>,
    frame_epoch: u64,
}

struct FrameFocusEntry {
    handle: FocusHandle,
    last_seen_epoch: u64,
}

impl FrameFocusRegistry {
    const fn begin_frame(&mut self) {
        self.frame_epoch = self.frame_epoch.saturating_add(1);
    }

    fn ensure(
        &mut self,
        key: FrameFocusKey,
        enabled: bool,
        cx: &Context<FrameRoot>,
    ) -> FocusHandle {
        let entry = self.handles.entry(key).or_insert_with(|| FrameFocusEntry {
            handle: cx.focus_handle().tab_stop(enabled),
            last_seen_epoch: self.frame_epoch,
        });
        entry.last_seen_epoch = self.frame_epoch;
        entry.handle = entry.handle.clone().tab_stop(enabled);
        entry.handle.clone()
    }

    fn finish_frame(&mut self, window: &mut Window, cx: &mut App, fallback: Option<&FocusHandle>) {
        let frame_epoch = self.frame_epoch;
        let mut focused_removed = false;
        self.handles.retain(|_, entry| {
            let keep = entry.last_seen_epoch == frame_epoch;
            if !keep && entry.handle.is_focused(window) {
                focused_removed = true;
            }
            keep
        });

        if focused_removed {
            if let Some(fallback) = fallback {
                fallback.focus(window, cx);
            } else {
                window.blur();
            }
        }
    }
}

impl FrameRoot {
    pub(in crate::app) const fn begin_accessibility_frame(&mut self) {
        self.focus_registry.begin_frame();
    }

    pub(in crate::app) fn ensure_focus(
        &mut self,
        key: FrameFocusKey,
        enabled: bool,
        cx: &Context<Self>,
    ) -> FocusHandle {
        self.focus_registry.ensure(key, enabled, cx)
    }

    pub(in crate::app) fn finish_accessibility_frame(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        fallback: Option<&FocusHandle>,
    ) {
        self.focus_registry.finish_frame(window, cx, fallback);
    }

    pub(in crate::app) fn focus_registered_control(
        &self,
        id: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = FrameFocusKey::Control(id.into());
        let Some(entry) = self.focus_registry.handles.get(&key) else {
            return false;
        };
        entry.handle.focus(window, cx);
        true
    }

    pub(in crate::app) fn restore_focus_after_settings_close(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.focus_registered_control("titlebar-settings", window, cx)
            || self.focus_registered_control(APP_ROOT_FOCUS_ID, window, cx)
    }

    pub(in crate::app) fn restore_focus_after_update_dialog_close(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.settings_ui.is_present
            && self.focus_registered_control("app-settings-update-check-now", window, cx)
        {
            return true;
        }
        self.focus_registered_control("titlebar-settings", window, cx)
            || self.focus_registered_control(APP_ROOT_FOCUS_ID, window, cx)
    }
}

pub(in crate::app) fn focus_visible_ring(style: StyleRefinement) -> StyleRefinement {
    style
        .ring_width(px(FOCUS_RING_WIDTH))
        .ring_color(color(theme::FRAME_BLUE.with_alpha(FOCUS_RING_ALPHA)))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TabNavigation {
    Next,
    Previous,
    First,
    Last,
}

fn tab_navigation_for_key(key: &str, shift: bool) -> Option<TabNavigation> {
    if key != "tab" {
        return None;
    }

    Some(if shift {
        TabNavigation::Previous
    } else {
        TabNavigation::Next
    })
}

fn modal_tab_navigation_for_key(
    key: &str,
    shift: bool,
    first_focused: bool,
    last_focused: bool,
) -> Option<TabNavigation> {
    match tab_navigation_for_key(key, shift)? {
        TabNavigation::Previous if first_focused => Some(TabNavigation::Last),
        TabNavigation::Next if last_focused => Some(TabNavigation::First),
        navigation => Some(navigation),
    }
}

pub(in crate::app) fn handle_tab_navigation(
    event: &gpui::KeyDownEvent,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let Some(navigation) = tab_navigation_for_key(
        event.keystroke.key.as_str(),
        event.keystroke.modifiers.shift,
    ) else {
        return false;
    };

    match navigation {
        TabNavigation::Next => window.focus_next(cx),
        TabNavigation::Previous => window.focus_prev(cx),
        TabNavigation::First | TabNavigation::Last => {}
    }
    cx.stop_propagation();
    true
}

pub(in crate::app) fn handle_modal_tab_navigation(
    event: &gpui::KeyDownEvent,
    first_focus: &FocusHandle,
    last_focus: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let Some(navigation) = modal_tab_navigation_for_key(
        event.keystroke.key.as_str(),
        event.keystroke.modifiers.shift,
        first_focus.is_focused(window),
        last_focus.is_focused(window),
    ) else {
        return false;
    };

    match navigation {
        TabNavigation::Next => window.focus_next(cx),
        TabNavigation::Previous => window.focus_prev(cx),
        TabNavigation::First => first_focus.focus(window, cx),
        TabNavigation::Last => last_focus.focus(window, cx),
    }
    cx.stop_propagation();
    true
}

pub(in crate::app) fn apply_accessible_button(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let element = element.role(Role::Button).aria_label(label);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_button_with_focus(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    focus: &FocusHandle,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::Button)
        .aria_label(label)
        .track_focus(focus);
    if enabled {
        element.tab_stop(true).focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_toggle_button(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    selected: bool,
) -> gpui::Stateful<gpui::Div> {
    apply_accessible_button(element, label, enabled).aria_toggled(Toggled::from(selected))
}

pub(in crate::app) fn apply_accessible_checkbox(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    checked: bool,
    indeterminate: bool,
) -> gpui::Stateful<gpui::Div> {
    let toggled = if indeterminate {
        Toggled::Mixed
    } else {
        Toggled::from(checked)
    };
    let element = element
        .role(Role::CheckBox)
        .aria_label(label)
        .aria_toggled(toggled);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_checkbox_with_focus(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    checked: bool,
    indeterminate: bool,
    focus: &FocusHandle,
) -> gpui::Stateful<gpui::Div> {
    let toggled = if indeterminate {
        Toggled::Mixed
    } else {
        Toggled::from(checked)
    };
    let element = element
        .role(Role::CheckBox)
        .aria_label(label)
        .aria_toggled(toggled)
        .track_focus(focus);
    if enabled {
        element.tab_stop(true).focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_select_trigger(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    expanded: bool,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::ComboBox)
        .aria_label(label)
        .aria_expanded(expanded);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_select_trigger_with_focus(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    expanded: bool,
    focus: &FocusHandle,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::ComboBox)
        .aria_label(label)
        .aria_expanded(expanded)
        .track_focus(focus);
    if enabled {
        element.tab_stop(true).focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_select_option(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    selected: bool,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::ListBoxOption)
        .aria_label(label)
        .aria_selected(selected);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_select_option_with_focus(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    selected: bool,
    focus: &FocusHandle,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::ListBoxOption)
        .aria_label(label)
        .aria_selected(selected)
        .track_focus(focus);
    if enabled {
        element.tab_stop(true).focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_slider(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    value: f64,
    min: f64,
    max: f64,
    value_text: impl Into<String>,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::Slider)
        .aria_label(label)
        .aria_orientation(Orientation::Horizontal)
        .aria_numeric_value(value)
        .aria_min_numeric_value(min)
        .aria_max_numeric_value(max)
        .aria_value(value_text);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Slider accessibility needs the same ARIA value contract plus an explicit focus handle."
)]
pub(in crate::app) fn apply_accessible_slider_with_focus(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    value: f64,
    min: f64,
    max: f64,
    value_text: impl Into<String>,
    focus: &FocusHandle,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::Slider)
        .aria_label(label)
        .aria_orientation(Orientation::Horizontal)
        .aria_numeric_value(value)
        .aria_min_numeric_value(min)
        .aria_max_numeric_value(max)
        .aria_value(value_text)
        .track_focus(focus);
    if enabled {
        element.tab_stop(true).focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_text_input(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    value: impl Into<String>,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::TextInput)
        .aria_label(label)
        .aria_value(value);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible(focus_visible_ring)
    } else {
        element.tab_stop(false).aria_disabled(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_ring_uses_non_layout_affecting_ring_style() {
        let style = focus_visible_ring(StyleRefinement::default());

        assert_eq!(style.ring_width, Some(px(FOCUS_RING_WIDTH).into()));
        assert_eq!(
            style.ring_color,
            Some(color(theme::FRAME_BLUE.with_alpha(FOCUS_RING_ALPHA)).into())
        );
        assert!(style.box_shadow.is_none());
    }

    #[test]
    fn modal_tab_navigation_wraps_shift_tab_from_first_to_last() {
        assert_eq!(
            modal_tab_navigation_for_key("tab", true, true, false),
            Some(TabNavigation::Last)
        );
    }

    #[test]
    fn modal_tab_navigation_wraps_tab_from_last_to_first() {
        assert_eq!(
            modal_tab_navigation_for_key("tab", false, false, true),
            Some(TabNavigation::First)
        );
    }
}
