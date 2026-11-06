use crate::app::accessibility::apply_accessible_toggle_button;
use crate::app::layout::SETTINGS_CONTROL_HEIGHT;
use crate::app::primitives::{button_mouse_down, color};
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{FluentBuilder, Styled};
use gpui::{Context, InteractiveElement, Window, div};

/// Shared spacing for compact option, track and preset lists.
pub(in crate::app) fn frame_list_group() -> gpui::Div {
    div()
        .w_full()
        .grid()
        .grid_cols(1)
        .gap(theme::ui_rem(theme::metrics::LIST_GAP))
}

pub(in crate::app) fn frame_list_item(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    _window: &mut Window,
    _cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let item = div()
        .id(id)
        .min_h(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .border_1()
        .border_color(color(palette.input))
        .bg(color(palette.fill_strong.with_alpha(if selected {
            palette.fill_strong.alpha
        } else {
            0.0
        })))
        .px(theme::ui_rem(theme::metrics::CONTROL_PADDING_X))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(palette.text_primary))
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, |this| {
            this.cursor_default()
                .hover_style(move |style| style.bg(color(palette.fill_strong)))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed);

    interactive_item(item, label, enabled, selected, palette)
}

// Mira Item has transition-colors duration-100; its 3px ring appears immediately.
fn interactive_item(
    item: gpui::Stateful<gpui::Div>,
    label: String,
    enabled: bool,
    selected: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let motion = gpui::Motion::new(std::time::Duration::from_millis(100))
        .with_easing(crate::app::motion::css_transition_ease);
    apply_accessible_toggle_button(item, label, enabled, selected, palette)
        .transitions(|t| {
            t.bg(motion.clone())
                .text_color(motion.clone())
                .border_color(motion)
        })
        .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
}
