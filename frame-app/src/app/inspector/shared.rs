use crate::app::components::{frame_badge, frame_description, frame_label};
use crate::app::primitives::color;
use crate::numeric::{rounded_f64_to_u32, u32_to_f32};
use crate::settings::SettingsTab;
use crate::{assets, theme};
use gpui::div;
use gpui::prelude::{ParentElement, Styled};

pub(in crate::app) fn settings_field_label(
    label: &'static str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    frame_label(theme::ui_text(label), palette)
}

pub(in crate::app) fn settings_value_badge(
    value: String,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    frame_badge(value, palette)
}

pub(in crate::app) fn settings_hint_text(
    text: &'static str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    frame_description(theme::ui_text(text), palette)
}

pub(in crate::app) fn settings_value_row(
    label: &'static str,
    value: impl Into<String>,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .grid()
        .grid_cols(2)
        .gap_4()
        .child(
            div()
                .text_color(color(palette.text_muted))
                .child(theme::ui_text(label)),
        )
        .child(
            div()
                .text_right()
                .text_color(color(palette.text_primary))
                .child(value.into()),
        )
}

pub(in crate::app) const fn settings_tab_icon(tab: SettingsTab) -> &'static str {
    match tab {
        SettingsTab::Metadata => assets::ICON_FILE_UP,
        SettingsTab::Output => assets::ICON_FILE_DOWN,
        SettingsTab::Presets => assets::ICON_BOOKMARK,
    }
}

pub(in crate::app) fn is_lossless_audio_codec(codec: &str) -> bool {
    matches!(codec, "flac" | "alac" | "pcm_s16le" | "pcm_bluray")
}

pub(in crate::app) fn parse_audio_value(value: &str, fallback: u32) -> u32 {
    value.trim().parse::<u32>().unwrap_or(fallback)
}

pub(in crate::app) fn range_fraction(value: u32, min: u32, max: u32) -> f32 {
    if max <= min {
        return 0.0;
    }
    let value = value.clamp(min, max) - min;
    u32_to_f32(value) / u32_to_f32(max - min)
}

pub(in crate::app) fn range_value_from_fraction(fraction: f64, min: u32, max: u32) -> u32 {
    if max <= min {
        return min;
    }
    let span = f64::from(max - min);
    rounded_f64_to_u32(fraction.clamp(0.0, 1.0).mul_add(span, f64::from(min)))
}

pub(in crate::app) fn range_value_for_key(
    value: u32,
    min: u32,
    max: u32,
    key: &str,
) -> Option<u32> {
    if max <= min {
        return None;
    }

    let value = value.clamp(min, max);
    let page_step = ((max - min) / 10).max(1);
    let next = match key {
        "left" | "down" => value.saturating_sub(1),
        "right" | "up" => value.saturating_add(1),
        "pageup" => value.saturating_add(page_step),
        "pagedown" => value.saturating_sub(page_step),
        "home" => min,
        "end" => max,
        _ => return None,
    };

    Some(next.clamp(min, max))
}

/// Track inclusion retains multi-track export while using the same compact
/// binary Tabs pattern as every other two-option setting.
#[expect(
    clippy::too_many_arguments,
    reason = "Track fields receive their accessible identity, metadata, selection and render context explicitly."
)]
pub(in crate::app) fn settings_track_choice(
    id: String,
    title: String,
    detail: String,
    selected: bool,
    enabled: bool,
    on_change: impl Fn(
        &mut crate::app::root::FrameRoot,
        &mut gpui::Window,
        &mut gpui::Context<crate::app::root::FrameRoot>,
    ) + 'static,
    palette: &'static theme::ThemePalette,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<crate::app::root::FrameRoot>,
) -> gpui::Div {
    let control = crate::app::components::frame_boolean_field(
        id,
        title.clone(),
        selected,
        enabled,
        ["Exclude", "Include"],
        move |root, _, window, cx| on_change(root, window, cx),
        palette,
        window,
        cx,
    );
    div()
        .flex()
        .items_center()
        .gap(theme::ui_rem(8.0))
        .min_w_0()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(frame_label(title, palette))
                .child(frame_description(detail, palette)),
        )
        .child(div().w(theme::ui_rem(144.0)).flex_shrink_0().child(control))
}
