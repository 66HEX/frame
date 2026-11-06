//! Controlled Radix Collapsible contract. Closed content is unmounted, so it
//! cannot receive keyboard focus. Mira's primitive has no height animation.
use super::{frame_animated_icon, frame_button_base, frame_field_group};
use crate::app::{
    motion,
    primitives::{ButtonVariant, color},
    root::FrameRoot,
};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{Context, StatefulInteractiveElement, Window, div};

#[expect(
    clippy::too_many_arguments,
    reason = "Controlled disclosure carries content, toggle handler and the shared render context."
)]
pub(in crate::app) fn frame_collapsible(
    id: impl Into<String>,
    label: &'static str,
    open: bool,
    content: Option<gpui::Div>,
    on_toggle: impl Fn(&mut FrameRoot, &mut Window, &mut Context<FrameRoot>) + 'static,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let id = id.into();
    let focus = window
        .use_keyed_state(format!("{id}-focus"), cx, |_, cx| {
            cx.focus_handle().tab_stop(true)
        })
        .read(cx)
        .clone();
    let click_focus = focus.clone();
    let trigger = frame_button_base(
        format!("{id}-trigger"),
        label,
        ButtonVariant::Ghost,
        false,
        true,
        theme::metrics::ControlSize::Default,
        palette,
        window,
        cx,
    )
    .track_focus(&focus)
    .aria_expanded(open)
    .w_full()
    .h(theme::ui_rem(9.0 * theme::metrics::SPACING))
    .rounded_none()
    // Include the transparent 1px button border in the shared content inset.
    .px(theme::ui_rem(theme::metrics::CARD_PADDING - 1.0))
    .justify_between()
    .when(open, |trigger| trigger.bg(color(palette.ghost_hover)))
    // Edge-to-edge rows keep focus feedback inside the scrolling panel's clip.
    .inset_ring_color(color(palette.focus_ring.with_alpha(0.0)))
    .focus_visible_style(move |style| {
        style
            .ring(gpui::px(0.0))
            .inset_ring(gpui::px(theme::metrics::FOCUS_WIDTH))
            .inset_ring_color(color(
                palette.focus_ring.with_opacity(theme::metrics::FOCUS_ALPHA),
            ))
            .border_color(color(palette.focus_ring))
    })
    .transitions(|t| motion::control_style_transitions(t).inset_ring(motion::interaction_motion()))
    .active_style(gpui::Styled::top_0)
    .child(label)
    .child(frame_animated_icon(
        format!("{id}-trigger"),
        Some(if open {
            assets::ICON_MINUS
        } else {
            assets::ICON_PLUS
        }),
        14.0,
        None,
        window,
        cx,
    ))
    .on_click(cx.listener(move |root, _, window, cx| {
        cx.stop_propagation();
        click_focus.focus(window, cx);
        on_toggle(root, window, cx);
        cx.notify();
    }));
    div()
        .flex()
        .flex_col()
        .w_full()
        .flex_shrink_0()
        .border_b_1()
        .border_color(color(palette.border_subtle))
        .child(trigger)
        .when_some(content.filter(|_| open), |section, content| {
            section.child(
                frame_field_group()
                    .p(theme::ui_rem(theme::metrics::CARD_PADDING))
                    .child(content),
            )
        })
}
