use super::{
    ButtonVariant, Context, FluentBuilder, FrameRoot, InteractiveElement, MouseButton,
    ParentElement, SETTINGS_CONTROL_HEIGHT, StatefulInteractiveElement, Styled, Window,
    animated_button_colors, apply_accessible_toggle_button, button_colors,
    button_highlight_shadows, button_mouse_down, color, div, frame_selection_dot, hover_motion,
    mix_color, mix_scalar, px, retarget_hover_motion, selected_motion, theme,
};

pub(in crate::app) fn frame_list_item(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let selected_progress = selected_motion(format!("{id}-selected"), selected, window, cx);
    let hover_transition = hover_motion(format!("{id}-hover"), window, cx);
    let hover_progress = *hover_transition.evaluate(window, cx);
    let emphasis_progress = selected_progress.max(hover_progress);

    let item = div()
        .id(id)
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .rounded(px(theme::RADIUS_SM))
        .border_l(px(2.0))
        .border_color(mix_color(
            theme::TRANSPARENT,
            theme::FRAME_GRAY_600,
            selected_progress,
        ))
        .bg(mix_color(
            theme::TRANSPARENT,
            theme::FRAME_GRAY_100,
            selected_progress,
        ))
        .pl(px(mix_scalar(8.0, 12.0, selected_progress)))
        .pr(px(12.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(mix_color(
            theme::FRAME_GRAY_600,
            theme::FOREGROUND,
            emphasis_progress,
        ))
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, |this| this.hover(gpui::Styled::cursor_pointer))
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && enabled, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        });

    apply_accessible_toggle_button(item, label, enabled, selected)
}

pub(in crate::app) fn frame_list_item_with_caption(
    id: impl Into<String>,
    title: impl Into<String>,
    caption: impl Into<String>,
    selected: bool,
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let title = title.into();
    let display_title = theme::ui_text_owned(title.clone());
    let caption = theme::ui_text_owned(caption.into());

    frame_list_item(id, title, selected, enabled, window, cx)
        .gap_3()
        .child(
            div()
                .text_color(color(theme::FOREGROUND))
                .child(display_title),
        )
        .child(
            div()
                .truncate()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .font_weight(theme::TEXT_WEIGHT_REGULAR)
                .text_color(color(theme::FRAME_GRAY_600))
                .child(caption),
        )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FrameTrackListItemLayout {
    Inline,
    Stacked,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct FrameTrackListItemText {
    pub(in crate::app) index_label: String,
    pub(in crate::app) primary: String,
    pub(in crate::app) detail: String,
}

pub(in crate::app) fn frame_track_list_item(
    id: impl Into<String>,
    text: FrameTrackListItemText,
    selected: bool,
    enabled: bool,
    layout: FrameTrackListItemLayout,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let colors = button_colors(ButtonVariant::Secondary, selected, enabled);
    let animated = animated_button_colors(id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;
    let FrameTrackListItemText {
        index_label,
        primary,
        detail,
    } = text;
    let accessible_label = if detail.is_empty() {
        primary.clone()
    } else {
        format!("{primary}, {detail}")
    };

    let label_row = div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_color(color(theme::FRAME_GRAY_600))
                .font_weight(theme::TEXT_WEIGHT_REGULAR)
                .child(index_label),
        )
        .child(div().text_color(color(theme::FOREGROUND)).child(primary));

    let content = match layout {
        FrameTrackListItemLayout::Inline => label_row.when(!detail.is_empty(), |this| {
            this.child(
                div()
                    .truncate()
                    .font_weight(theme::TEXT_WEIGHT_REGULAR)
                    .text_color(color(theme::FRAME_GRAY_600))
                    .child(detail),
            )
        }),
        FrameTrackListItemLayout::Stacked => div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .child(label_row)
            .when(!detail.is_empty(), |this| {
                this.child(
                    div()
                        .truncate()
                        .font_weight(theme::TEXT_WEIGHT_REGULAR)
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(detail),
                )
            }),
    };

    let item = div()
        .id(id)
        .min_h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .py(px(6.0))
        .bg(background)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(enabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && enabled, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .child(content)
        .child(frame_selection_dot(selected));

    apply_accessible_toggle_button(item, accessible_label, enabled, selected)
}
