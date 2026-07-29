use super::{
    ButtonVariant, Context, FluentBuilder, FrameRoot, InteractiveElement, ParentElement,
    SETTINGS_CONTROL_HEIGHT, Styled, Window, apply_accessible_toggle_button, apply_button_motion,
    button_colors, button_highlight_shadows, button_motion, color, div, frame_selection_dot,
    mix_color, mix_rgba, mix_scalar, selected_motion, theme,
};

pub(in crate::app) fn frame_list_item(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    let selected_progress = selected_motion(format!("{id}-selected"), selected, window, cx);
    let motion = button_motion(format!("{id}-hover"), window, cx);
    let hover_progress = *motion.hover_transition.evaluate(window, cx);
    let emphasis_progress = selected_progress.max(hover_progress);

    let item = div()
        .id(id)
        .min_h(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .border_l(theme::ui_rem(2.0))
        .border_color(mix_color(
            palette.transparent,
            palette.text_muted,
            selected_progress,
        ))
        .bg(mix_color(
            palette.transparent,
            palette.fill_subtle,
            selected_progress,
        ))
        .pl(theme::ui_rem(mix_scalar(8.0, 12.0, selected_progress)))
        .pr(theme::ui_rem(12.0))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(mix_color(
            palette.text_muted,
            palette.text_primary,
            emphasis_progress,
        ))
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, |this| this.hover(gpui::Styled::cursor_pointer))
        .when(!enabled, gpui::Styled::cursor_not_allowed);

    let item = apply_button_motion(item, motion, enabled);

    apply_accessible_toggle_button(item, label, enabled, selected, palette)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The list-item builder keeps content, interaction state, palette, and render context explicit."
)]
pub(in crate::app) fn frame_list_item_with_caption(
    id: impl Into<String>,
    title: impl Into<String>,
    caption: impl Into<String>,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let title = title.into();
    let display_title = theme::ui_text_owned(title.clone());
    let caption = theme::ui_text_owned(caption.into());

    frame_list_item(id, title, selected, enabled, palette, window, cx)
        .gap_3()
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
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let resting_colors = button_colors(ButtonVariant::Secondary, false, enabled, palette);
    let selected_colors = button_colors(ButtonVariant::Secondary, true, enabled, palette);
    let selected_progress = selected_motion(format!("{id}-selected"), selected, window, cx);
    let motion = button_motion(id.clone(), window, cx);
    let hover_progress = *motion.hover_transition.evaluate(window, cx);
    let background = mix_rgba(
        mix_color(
            resting_colors.background,
            selected_colors.background,
            selected_progress,
        ),
        mix_color(
            resting_colors.hover_background,
            selected_colors.hover_background,
            selected_progress,
        ),
        hover_progress,
    );
    let foreground = mix_rgba(
        mix_color(
            resting_colors.foreground,
            selected_colors.foreground,
            selected_progress,
        ),
        mix_color(
            resting_colors.hover_foreground,
            selected_colors.hover_foreground,
            selected_progress,
        ),
        hover_progress,
    );
    let opacity = mix_scalar(
        resting_colors.opacity,
        selected_colors.opacity,
        selected_progress,
    );
    let (label, accessible_label) = frame_track_list_item_label(text, palette);

    let item = div()
        .id(id)
        .min_h(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .px(theme::ui_rem(10.0))
        .py(theme::ui_rem(6.0))
        .bg(background)
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(opacity)
        .shadow(button_highlight_shadows(palette))
        .when(enabled, |this| this.hover(gpui::Styled::cursor_pointer))
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .child(label)
        .child(frame_selection_dot(selected_progress, palette));

    let item = apply_button_motion(item, motion, enabled);

    apply_accessible_toggle_button(item, accessible_label, enabled, selected, palette)
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
