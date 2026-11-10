//! Mira Tabs for compact choices and forms whose selection changes the view.
use super::{FrameOption, FrameStyle, FrameTabIndicator, frame_tabs_list};
use crate::app::accessibility::{TabFocusGroup, tab_focus_base, tab_focus_ring};
use crate::app::{motion, primitives::color, root::FrameRoot};
use crate::theme;
use gpui::prelude::*;
use gpui::{Context, StatefulInteractiveElement, Window, div};
use std::rc::Rc;

pub(in crate::app) fn frame_option_tabs(
    id: impl Into<String>,
    label: impl Into<String>,
    choices: Vec<FrameOption>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let id = id.into();
    let label = label.into();
    let active = choices
        .iter()
        .find(|choice| choice.item.selected && choice.item.enabled)
        .or_else(|| choices.iter().find(|choice| choice.item.enabled))
        .map_or(String::new(), |choice| choice.item.id.clone());
    let group = TabFocusGroup::render(
        format!("{id}-tabs"),
        active,
        choices.iter().map(|choice| choice.item.id.clone()),
        window,
        cx,
    );
    let enabled: Rc<Vec<_>> = Rc::new(
        choices
            .iter()
            .filter(|choice| choice.item.enabled)
            .map(|choice| {
                (
                    choice.item.id.clone(),
                    group.handle(&choice.item.id),
                    choice.action.clone(),
                )
            })
            .collect(),
    );
    let selector = id.clone();
    let indicator = FrameTabIndicator::new(id.clone(), window, cx);
    let mut list = frame_tabs_list(palette)
        .id(id)
        .debug_selector(move || selector)
        .role(gpui::Role::TabList)
        .aria_label(label)
        .w_full()
        .child(indicator.background(palette));
    for choice in choices {
        let item = choice.item;
        let focus = group
            .handle(&item.id)
            .tab_stop(item.enabled && group.selected(&item.id));
        let pointer_focus = focus.clone();
        let keys = enabled.clone();
        let key_id = item.id.clone();
        list = list.child(
            indicator
                .track_with_opacity(
                    frame_tab_trigger(
                        item.id.clone(),
                        choice.display.clone(),
                        item.selected,
                        item.enabled,
                        &focus,
                        palette,
                    ),
                    item.selected.then(|| item.id.clone()),
                    if item.enabled { 1.0 } else { 0.5 },
                )
                .on_click(cx.listener(move |root, _, window, cx| {
                    if item.enabled {
                        pointer_focus.focus(window, cx);
                        (choice.action)(root, window, cx);
                        cx.notify();
                    }
                    cx.stop_propagation();
                }))
                .on_key_down(
                    cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                        let Some(current) = keys.iter().position(|(id, _, _)| id == &key_id) else {
                            return;
                        };
                        let next = match event.keystroke.key.as_str() {
                            "left" => (current + keys.len() - 1) % keys.len(),
                            "right" => (current + 1) % keys.len(),
                            "home" => 0,
                            "end" => keys.len() - 1,
                            _ => return,
                        };
                        keys[next].1.focus(window, cx);
                        (keys[next].2)(root, window, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .child(theme::ui_text_owned(choice.display)),
                ),
        );
    }
    div().w_full().min_w_0().child(list)
}

/// Shared geometry and states for labelled navigation tabs and option tabs.
pub(in crate::app) fn frame_tab_trigger(
    id: String,
    label: String,
    selected: bool,
    enabled: bool,
    focus: &gpui::FocusHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let selector = id.clone();
    div()
        .id(id)
        .debug_selector(move || selector)
        .role(gpui::Role::Tab)
        .aria_label(label)
        .aria_selected(selected)
        .aria_disabled(!enabled)
        .track_focus(focus)
        .map(|element| tab_focus_base(element, palette))
        .focus_visible_style(move |style| tab_focus_ring(style, palette))
        .transitions(|t| {
            motion::control_style_transitions(t).outline_color(motion::interaction_motion())
        })
        .flex_auto()
        .min_w_0()
        .h(theme::ui_rem(theme::metrics::TAB_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .gap(theme::ui_rem(theme::metrics::TAB_ICON_GAP))
        .px(theme::ui_rem(6.0))
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .tab_surface(selected, palette)
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .opacity(if enabled { 1.0 } else { 0.5 })
        .cursor_default()
        .when(enabled, |tab| {
            tab.hover_style(|style| style.text_color(color(palette.text_primary)))
        })
}
