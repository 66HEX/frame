//! Graphite surface hierarchy on the existing compact component geometry.

use crate::app::primitives::color;
use crate::theme::{self, metrics};
use gpui::prelude::*;
use gpui::{Div, IntoElement, div, px};

pub(in crate::app) trait FrameStyle: Styled + Sized {
    fn input_surface(self, palette: &theme::ThemePalette) -> Self {
        self.rounded(theme::ui_rem(theme::RADIUS_MD))
            .border_1()
            .border_color(color(palette.transparent))
            .bg(color(palette.input_background))
            .shadow(theme::shadows::recessed(palette))
            .text_color(color(palette.text_primary))
    }

    fn invalid_surface(self, palette: &theme::ThemePalette) -> Self {
        self.border_color(color(palette.danger.with_opacity(if palette.is_light() {
            1.0
        } else {
            0.5
        })))
        .ring(px(2.0))
        .ring_color(color(palette.danger.with_opacity(if palette.is_light() {
            0.2
        } else {
            0.4
        })))
    }

    fn popover_surface(self, palette: &theme::ThemePalette) -> Self {
        self.rounded(theme::ui_rem(theme::RADIUS_LG))
            .bg(theme::shadows::surface_fill(
                palette.surface_elevated,
                palette,
            ))
            .text_color(color(palette.token("popover-foreground")))
            .shadow(theme::shadows::FLOATING.shadows())
    }

    fn dialog_surface(self, palette: &theme::ThemePalette) -> Self {
        self.rounded(theme::ui_rem(theme::RADIUS_XL))
            .bg(theme::shadows::surface_fill(
                palette.surface_elevated,
                palette,
            ))
            .text_color(color(palette.token("popover-foreground")))
            .shadow(theme::shadows::ShadowLevel::Xl.shadows())
    }

    fn tab_surface(self, selected: bool, palette: &theme::ThemePalette) -> Self {
        self.rounded(theme::ui_rem(theme::RADIUS_MD))
            .border_1()
            .border_color(color(palette.transparent))
            .bg(color(palette.transparent))
            .text_color(color(if selected {
                palette.text_primary
            } else if palette.is_dark() {
                palette.text_muted
            } else {
                palette.text_primary.with_opacity(0.6)
            }))
    }
}
impl<T: Styled + Sized> FrameStyle for T {}

pub(in crate::app) fn frame_field() -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(theme::ui_rem(metrics::FIELD_GAP))
}

pub(in crate::app) fn frame_field_group() -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(theme::ui_rem(metrics::GROUP_GAP))
}

pub(in crate::app) fn frame_label(label: impl Into<String>, palette: &theme::ThemePalette) -> Div {
    div()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .line_height(gpui::relative(1.375))
        .text_color(color(palette.text_primary))
        .child(label.into())
}

pub(in crate::app) fn frame_description(
    text: impl Into<String>,
    palette: &theme::ThemePalette,
) -> Div {
    div()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .line_height(gpui::relative(metrics::TEXT_LINE_HEIGHT))
        .text_color(color(palette.text_muted))
        .child(text.into())
}

pub(in crate::app) fn frame_section(
    title: impl Into<String>,
    palette: &theme::ThemePalette,
) -> Div {
    frame_field_group().child(frame_label(title, palette).text_size(theme::ui_rem(14.0)))
}

pub(in crate::app) fn frame_badge(value: impl Into<String>, palette: &theme::ThemePalette) -> Div {
    div()
        .flex_shrink_0()
        .h(theme::ui_rem(metrics::BADGE_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(color(palette.transparent))
        .px(theme::ui_rem(8.0))
        .py(theme::ui_rem(2.0))
        .bg(color(palette.fill_subtle))
        .text_color(color(palette.secondary_foreground))
        .text_size(theme::ui_rem(metrics::BADGE_TEXT))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .font_features(crate::assets::frame_tabular_number_font_features())
        .child(value.into())
}

pub(in crate::app) fn frame_panel_header() -> Div {
    div()
        .relative()
        .w_full()
        .min_h(theme::ui_rem(crate::app::layout::PANEL_HEADER_HEIGHT))
        .flex()
        .items_center()
        .gap(theme::ui_rem(metrics::FIELD_GAP))
        .px(theme::ui_rem(metrics::CARD_PADDING))
}

pub(in crate::app) fn frame_toolbar(palette: &theme::ThemePalette) -> Div {
    div()
        .flex()
        .items_center()
        .gap(theme::ui_rem(metrics::CONTROL_GAP))
        .p(theme::ui_rem(4.0))
        .popover_surface(palette)
        .rounded(theme::ui_rem(theme::RADIUS_MD + 4.0))
}

pub(in crate::app) fn frame_empty(content: impl IntoElement, palette: &theme::ThemePalette) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(theme::ui_rem(16.0))
        .p(theme::ui_rem(24.0))
        .rounded(theme::ui_rem(theme::RADIUS_XL))
        .text_center()
        .text_size(theme::ui_rem(12.0))
        .text_color(color(palette.text_muted))
        .child(content)
}

pub(in crate::app) fn frame_tabs_list(palette: &theme::ThemePalette) -> Div {
    div()
        .relative()
        .flex()
        .items_center()
        .h(theme::ui_rem(metrics::TABS_HEIGHT))
        .p(theme::ui_rem(metrics::TABS_PADDING))
        .rounded(theme::ui_rem(theme::RADIUS_MD + metrics::TABS_PADDING))
        .bg(color(palette.input_background))
        .shadow(theme::shadows::recessed(palette))
}
