use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::Styled;
use gpui::{
    App, Context, FocusHandle, InteractiveElement, Orientation, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Toggled, Window, px,
};
use std::collections::HashMap;

#[cfg(test)]
#[path = "accessibility/focus_ring_tests.rs"]
mod focus_ring_tests;

const FOCUS_RING_WIDTH: f32 = theme::metrics::FOCUS_WIDTH;
pub(in crate::app) const APP_ROOT_FOCUS_ID: &str = "app-root";

/// One tab stop per rail; selection, pointer focus and arrow-key focus agree.
pub(in crate::app) struct TabFocusGroup {
    active: String,
    handles: HashMap<String, FocusHandle>,
}

impl TabFocusGroup {
    pub(in crate::app) fn render(
        id: impl Into<gpui::ElementId>,
        active: String,
        tabs: impl IntoIterator<Item = String>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let state = window.use_keyed_state(id, cx, |_, _| HashMap::<String, FocusHandle>::new());
        let handles = state.update(cx, |handles, cx| {
            let mut next = HashMap::new();
            for id in tabs {
                let focus = handles.remove(&id).unwrap_or_else(|| cx.focus_handle());
                next.insert(id.clone(), focus.tab_stop(id == active));
            }
            if handles.values().any(|handle| handle.is_focused(window))
                && let Some(focus) = next.get(&active)
            {
                focus.focus(window, cx);
            }
            *handles = next;
            handles.clone()
        });
        Self { active, handles }
    }

    pub(in crate::app) fn selected(&self, id: &str) -> bool {
        self.active == id
    }

    pub(in crate::app) fn handle(&self, id: &str) -> FocusHandle {
        self.handles[id].clone()
    }
}

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
                window.blur(cx);
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

    pub(in crate::app) fn register_shell_focus(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Keyed state is render-only. Keep the same handle in the registry so
        // dialog event handlers can restore it without touching render state.
        let handle = titlebar_settings_focus(window, cx);
        self.focus_registry.handles.insert(
            FrameFocusKey::Control("titlebar-settings".into()),
            FrameFocusEntry {
                handle,
                last_seen_epoch: self.focus_registry.frame_epoch,
            },
        );
        let handle = welcome_open_file_focus(window, cx);
        self.focus_registry.handles.insert(
            FrameFocusKey::Control("welcome-open-file".into()),
            FrameFocusEntry {
                handle,
                last_seen_epoch: self.focus_registry.frame_epoch,
            },
        );
    }

    pub(in crate::app) fn restore_focus_after_settings_close(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.file_queue.files().is_empty() {
            self.focus_registered_control("welcome-open-file", window, cx)
        } else {
            self.focus_registered_control("titlebar-settings", window, cx)
        }
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
        self.restore_focus_after_settings_close(window, cx)
    }
}

pub(in crate::app) fn titlebar_settings_focus(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> FocusHandle {
    window
        .use_keyed_state("titlebar-settings-focus", cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone()
        .tab_stop(true)
}

pub(in crate::app) fn welcome_open_file_focus(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> FocusHandle {
    window
        .use_keyed_state("welcome-open-file-focus", cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone()
        .tab_stop(true)
}

/// An absent CSS box shadow starts transparent, not at `currentColor`.
/// GPUI interpolates `CurrentColor` -> `Color` discretely, which would paint the
/// text color until the last frame while the ring is already growing.
pub(in crate::app) fn focus_ring_base<T: Styled>(
    mut element: T,
    palette: &theme::ThemePalette,
) -> T {
    if element.style().ring.color.is_none() {
        element = element.ring_color(color(palette.focus_ring.with_alpha(0.0)));
    }
    element
}

/// Keep the shared ring appearance, but paint it above custom media/corner masks.
/// GPUI's outline uses the same geometry as its ring, with a foreground paint pass.
pub(in crate::app) fn focus_ring_above_content<T: Styled>(mut element: T) -> T {
    let style = element.style();
    style.outline = std::mem::take(&mut style.ring);
    element
}

pub(in crate::app) fn focus_visible_ring(
    style: StyleRefinement,
    palette: &'static theme::ThemePalette,
) -> StyleRefinement {
    style
        .ring(px(FOCUS_RING_WIDTH))
        .ring_color(color(
            palette.focus_ring.with_opacity(theme::metrics::FOCUS_ALPHA),
        ))
        .border_color(color(palette.focus_ring))
}

pub(in crate::app) fn composite_focus_ring(
    style: StyleRefinement,
    palette: &'static theme::ThemePalette,
) -> StyleRefinement {
    style
        .ring(px(3.0))
        .ring_color(color(palette.focus_ring.with_opacity(0.5)))
        .border_color(color(palette.focus_ring))
}

/// Keep outline width at 1px; CSS outline-color transitions from transparent.
pub(in crate::app) fn tab_focus_base<T: Styled>(element: T, palette: &theme::ThemePalette) -> T {
    focus_ring_base(element, palette)
        .outline(px(1.0))
        .outline_color(color(palette.focus_ring.with_alpha(0.0)))
}

pub(in crate::app) fn tab_focus_ring(
    style: StyleRefinement,
    palette: &'static theme::ThemePalette,
) -> StyleRefinement {
    composite_focus_ring(style, palette).outline_color(color(palette.focus_ring))
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

/// Keyboard scrolling for focused read-only areas (inspectors, release notes).
pub(in crate::app) fn handle_scroll_area_key(
    event: &gpui::KeyDownEvent,
    scroll: &gpui::ScrollHandle,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let mut offset = scroll.offset();
    let page = scroll.bounds().size.height * 0.9;
    match event.keystroke.key.as_str() {
        "up" => offset.y += px(40.0),
        "down" => offset.y -= px(40.0),
        "pageup" => offset.y += page,
        "pagedown" => offset.y -= page,
        "home" => offset.y = px(0.0),
        "end" => offset.y = -scroll.max_offset().y,
        _ => return false,
    }
    scroll.set_offset(offset);
    window.refresh();
    cx.stop_propagation();
    true
}

/// Move through the rendered stops inside a dialog, including dynamic actions.
pub(in crate::app) fn handle_focus_scope_tab_navigation(
    event: &gpui::KeyDownEvent,
    scope: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let Some(navigation) = tab_navigation_for_key(
        event.keystroke.key.as_str(),
        event.keystroke.modifiers.shift,
    ) else {
        return false;
    };
    let mut first = None;
    loop {
        if navigation == TabNavigation::Previous {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
        }
        if scope.contains_focused(window, cx) {
            break;
        }
        let candidate = window.focused(cx);
        if candidate.is_none() || candidate == first {
            scope.focus(window, cx);
            break;
        }
        if first.is_none() {
            first = candidate;
        }
    }
    cx.stop_propagation();
    true
}

pub(in crate::app) fn apply_accessible_button(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let element = focus_ring_base(element, palette)
        .role(Role::Button)
        .aria_label(label)
        // Drag handles share button semantics and focus feedback, but their
        // geometry must follow pointer/keyboard input without interpolation.
        .transitions(crate::app::motion::control_style_transitions);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_toggle_button(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    selected: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    apply_accessible_button(element, label, enabled, palette)
        .aria_toggled(Toggled::from(selected))
        .focus_visible_style(move |style| composite_focus_ring(style, palette))
}

pub(in crate::app) fn apply_accessible_checkbox(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    checked: bool,
    indeterminate: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let toggled = if indeterminate {
        Toggled::Mixed
    } else {
        Toggled::from(checked)
    };
    let element = focus_ring_base(element, palette)
        .role(Role::CheckBox)
        .aria_label(label)
        .aria_toggled(toggled)
        .transitions(|t| t.ring(crate::app::motion::interaction_motion()));
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_checkbox_with_focus(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    checked: bool,
    indeterminate: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let toggled = if indeterminate {
        Toggled::Mixed
    } else {
        Toggled::from(checked)
    };
    let element = focus_ring_base(element, palette)
        .role(Role::CheckBox)
        .aria_label(label)
        .aria_toggled(toggled)
        .track_focus(focus)
        .transitions(|t| t.ring(crate::app::motion::interaction_motion()));
    if enabled {
        element
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_select_trigger(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    expanded: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::ComboBox)
        .aria_label(label)
        .aria_expanded(expanded)
        .transitions(|t| {
            t.border_color(crate::app::motion::interaction_motion())
                .bg(crate::app::motion::interaction_motion())
                .text_color(crate::app::motion::interaction_motion())
        });
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_select_trigger_with_focus(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    expanded: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::ComboBox)
        .aria_label(label)
        .aria_expanded(expanded)
        .track_focus(focus)
        .transitions(|t| {
            t.border_color(crate::app::motion::interaction_motion())
                .bg(crate::app::motion::interaction_motion())
                .text_color(crate::app::motion::interaction_motion())
        });
    if enabled {
        element
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

fn select_option_highlight(
    style: StyleRefinement,
    palette: &theme::ThemePalette,
) -> StyleRefinement {
    style
        .bg(color(palette.token("accent")))
        .text_color(color(palette.token("accent-foreground")))
}

pub(in crate::app) fn apply_accessible_select_option(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    selected: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::ListBoxOption)
        .aria_label(label)
        .aria_selected(selected);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_style(move |style| select_option_highlight(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_select_option_with_focus(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    selected: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::ListBoxOption)
        .aria_label(label)
        .aria_selected(selected)
        .track_focus(focus);
    if enabled {
        element
            .tab_stop(true)
            .focus_style(move |style| select_option_highlight(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Slider accessibility requires the full ARIA range plus explicit theme and render context."
)]
pub(in crate::app) fn apply_accessible_slider(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    value: f64,
    min: f64,
    max: f64,
    value_text: impl Into<String>,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::Slider)
        .aria_label(label)
        .aria_orientation(Orientation::Horizontal)
        .aria_numeric_value(value)
        .aria_min_numeric_value(min)
        .aria_max_numeric_value(max)
        .aria_value(value_text.into());
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
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
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::Slider)
        .aria_label(label)
        .aria_orientation(Orientation::Horizontal)
        .aria_numeric_value(value)
        .aria_min_numeric_value(min)
        .aria_max_numeric_value(max)
        .aria_value(value_text.into())
        .track_focus(focus);
    if enabled {
        element
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

pub(in crate::app) fn apply_accessible_text_input(
    element: gpui::Stateful<gpui::Div>,
    label: impl Into<SharedString>,
    enabled: bool,
    value: impl Into<String>,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let element = element
        .role(Role::TextInput)
        .aria_label(label)
        .aria_value(value.into())
        .transitions(|t| {
            t.border_color(crate::app::motion::interaction_motion())
                .bg(crate::app::motion::interaction_motion())
                .text_color(crate::app::motion::interaction_motion())
        });
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible_style(move |style| focus_visible_ring(style, palette))
    } else {
        element.disabled(true).tab_stop(false).aria_disabled(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_ring_uses_non_layout_affecting_ring_style() {
        let palette = theme::palette(crate::appearance::ColorTheme::Dark);
        let style = focus_visible_ring(StyleRefinement::default(), palette);

        assert_eq!(style.ring.width, Some(px(FOCUS_RING_WIDTH)));
        assert_eq!(
            style.ring.color,
            Some(gpui::RingColor::Color(
                color(palette.focus_ring.with_opacity(theme::metrics::FOCUS_ALPHA)).into()
            ))
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
