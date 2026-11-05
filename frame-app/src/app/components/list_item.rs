use crate::app::accessibility::apply_accessible_toggle_button;
use crate::app::components::checkbox::frame_selection_dot;
use crate::app::layout::SETTINGS_CONTROL_HEIGHT;
use crate::app::primitives::{ButtonVariant, button_colors, button_mouse_down, color};
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
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

#[expect(
    clippy::too_many_arguments,
    reason = "The list-item builder keeps content, interaction state, palette, and render context explicit."
)]
pub(in crate::app) fn frame_list_item_with_caption(
    group: &super::FrameChoiceFocus,
    id: impl Into<String>,
    title: impl Into<String>,
    caption: impl Into<String>,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let focus = group.register(&id, selected, enabled, cx);
    let title = title.into();
    let display_title = theme::ui_text_owned(title.clone());
    let caption = theme::ui_text_owned(caption.into());

    group
        .apply(
            frame_list_item(id.clone(), title, selected, enabled, palette, window, cx),
            id,
            enabled,
            &focus,
            cx,
        )
        .child(
            div()
                .flex_none()
                .text_color(color(palette.text_primary))
                .child(display_title),
        )
        .child(
            div()
                .min_w_0()
                .flex_auto()
                .truncate()
                .text_right()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .font_weight(theme::TEXT_WEIGHT_REGULAR)
                .text_color(color(palette.text_muted))
                .child(caption),
        )
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct FrameTrackListItemText {
    pub(in crate::app) index_label: String,
    pub(in crate::app) primary: String,
    pub(in crate::app) detail: String,
    pub(in crate::app) trailing: String,
    pub(in crate::app) layout: FrameTrackListItemLayout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FrameTrackListItemLayout {
    Compact,
    Detailed,
}

pub(in crate::app) fn frame_track_list_item(
    id: impl Into<String>,
    text: FrameTrackListItemText,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    _window: &mut Window,
    _cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let colors = button_colors(ButtonVariant::Outline, selected, enabled, palette);
    let (label, accessible_label) = frame_track_list_item_label(text, palette);

    let item = div()
        .id(id)
        .min_h(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .px(theme::ui_rem(theme::metrics::CONTROL_PADDING_X))
        .py(theme::ui_rem(6.0))
        .bg(color(colors.background))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .when(enabled, |this| {
            this.cursor_default()
                .hover_style(move |style| style.bg(color(colors.hover_background)))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .child(label)
        .child(frame_selection_dot(
            crate::app::motion::motion_target(selected),
            palette,
        ));

    interactive_item(item, accessible_label, enabled, selected, palette)
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

fn frame_track_list_item_label(
    text: FrameTrackListItemText,
    palette: &'static theme::ThemePalette,
) -> (gpui::Div, String) {
    let FrameTrackListItemText {
        index_label,
        primary,
        detail,
        trailing,
        layout,
    } = text;
    let accessible_label = [primary.as_str(), detail.as_str(), trailing.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    let index = div()
        .flex_none()
        .text_color(color(palette.text_muted))
        .font_weight(theme::TEXT_WEIGHT_REGULAR)
        .child(index_label);
    let primary_label = div()
        .flex_none()
        .text_color(color(palette.text_primary))
        .child(primary);

    let label = match layout {
        FrameTrackListItemLayout::Compact => div()
            .min_w_0()
            .flex_1()
            .flex()
            .items_center()
            .gap_2()
            .child(index)
            .child(primary_label)
            .when(!detail.is_empty(), |this| {
                this.child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .font_weight(theme::TEXT_WEIGHT_REGULAR)
                        .text_color(color(palette.text_muted))
                        .child(format!("• {detail}")),
                )
            }),
        FrameTrackListItemLayout::Detailed => div()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .min_w_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(index)
                    .child(primary_label),
            )
            .when(!detail.is_empty() || !trailing.is_empty(), |this| {
                let metadata = match (detail.is_empty(), trailing.is_empty()) {
                    (false, false) => format!("{detail} • {trailing}"),
                    (false, true) => detail,
                    (true, false) => trailing,
                    (true, true) => String::new(),
                };
                this.child(
                    div()
                        .min_w_0()
                        .w_full()
                        .truncate()
                        .font_features(crate::assets::frame_tabular_number_font_features())
                        .font_weight(theme::TEXT_WEIGHT_REGULAR)
                        .text_color(color(palette.text_muted))
                        .child(metadata),
                )
            }),
    };
    (label, accessible_label)
}
