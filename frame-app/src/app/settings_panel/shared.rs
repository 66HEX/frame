use super::*;

pub(in crate::app) fn settings_field_label(label: &'static str) -> gpui::Div {
    div()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(theme::FRAME_GRAY_600))
        .child(label)
}

pub(in crate::app) fn settings_value_badge(value: String) -> gpui::Div {
    div()
        .h(px(18.0))
        .flex()
        .items_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::FRAME_GRAY_400))
        .px(px(6.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(theme::FOREGROUND))
        .font_features(assets::frame_tabular_number_font_features())
        .shadow(button_highlight_shadows())
        .child(value)
}

pub(in crate::app) fn settings_hint_text(text: &'static str) -> gpui::Div {
    div()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600))
        .child(text)
}

pub(in crate::app) fn settings_value_row(
    label: &'static str,
    value: impl Into<String>,
) -> gpui::Div {
    div()
        .grid()
        .grid_cols(2)
        .gap_4()
        .child(div().text_color(color(theme::FRAME_GRAY_600)).child(label))
        .child(
            div()
                .text_right()
                .text_color(color(theme::FOREGROUND))
                .child(value.into()),
        )
}

pub(in crate::app) fn settings_tab_icon(tab: SettingsTab) -> &'static str {
    match tab {
        SettingsTab::Source => assets::ICON_FILE_UP,
        SettingsTab::Output => assets::ICON_FILE_DOWN,
        SettingsTab::Video => assets::ICON_FILE_VIDEO,
        SettingsTab::Images => assets::ICON_FILE_IMAGE,
        SettingsTab::Audio => assets::ICON_MUSIC,
        SettingsTab::Subtitles => assets::ICON_CAPTIONS,
        SettingsTab::Metadata => assets::ICON_TAGS,
        SettingsTab::Presets => assets::ICON_BOOKMARK,
    }
}

pub(in crate::app) fn is_lossless_audio_codec(codec: &str) -> bool {
    matches!(codec, "flac" | "alac" | "pcm_s16le")
}

pub(in crate::app) fn parse_audio_value(value: &str, fallback: u32) -> u32 {
    value.trim().parse::<u32>().unwrap_or(fallback)
}

pub(in crate::app) fn range_fraction(value: u32, min: u32, max: u32) -> f32 {
    if max <= min {
        return 0.0;
    }
    let value = value.clamp(min, max) - min;
    value as f32 / (max - min) as f32
}

pub(in crate::app) fn range_value_from_fraction(fraction: f64, min: u32, max: u32) -> u32 {
    if max <= min {
        return min;
    }
    let span = f64::from(max - min);
    (f64::from(min) + fraction.clamp(0.0, 1.0) * span).round() as u32
}
