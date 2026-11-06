//! Shared Mira tab trigger for labelled navigation and option tabs.
use super::FrameStyle;
use crate::app::accessibility::{tab_focus_base, tab_focus_ring};
use crate::app::{motion, primitives::color};
use crate::theme;
use gpui::prelude::*;
use gpui::{StatefulInteractiveElement, div};

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
        .flex_1()
        .min_w_0()
        .h(theme::ui_rem(theme::metrics::TAB_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .gap(theme::ui_rem(6.0))
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
