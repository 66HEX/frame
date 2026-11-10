use crate::settings::model::{
    ConversionConfig, SUBTITLE_FONT_SIZES, SUBTITLE_POSITIONS, SourceMetadata, SubtitleFontOption,
    SubtitleFontSizeOption, SubtitlePosition, SubtitlePositionOption, SubtitleTrackOption,
};
use crate::settings::rules::is_subtitle_codec_allowed_for_container;
use crate::settings::source_info::display_source_value;
use frame_core::capabilities::AvailableEncoders;

#[must_use]
pub fn subtitle_track_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
    available_encoders: &AvailableEncoders,
) -> Vec<SubtitleTrackOption> {
    metadata
        .map(|metadata| {
            metadata
                .subtitle_tracks
                .iter()
                .map(|track| {
                    let compatible = is_subtitle_codec_allowed_for_container(
                        &config.container,
                        &track.codec,
                    );
                    let requires_unavailable_dvbsub = config.container.eq_ignore_ascii_case("m2t")
                        && matches!(track.codec.as_str(), "hdmv_pgs_subtitle" | "dvd_subtitle")
                        && !available_encoders.dvbsub;
                    let reason = if !compatible {
                        Some(format!(
                            "{} cannot be represented in {}; choose another output or leave it unselected",
                            track.codec, config.container
                        ))
                    } else if requires_unavailable_dvbsub {
                        Some(
                            "This bitmap track requires the DVB subtitle encoder, which is unavailable in the active FFmpeg runtime"
                                .to_string(),
                        )
                    } else {
                        None
                    };
                    let is_selected = config.selected_subtitle_tracks.contains(&track.index);
                    SubtitleTrackOption {
                        index: track.index,
                        index_label: format!("#{}", track.index),
                        codec: display_source_value(Some(&track.codec)),
                        detail: subtitle_track_detail(
                            track.language.as_deref(),
                            track.label.as_deref(),
                        ),
                        is_selected,
                        is_disabled: disabled
                            || (!is_selected && (!compatible || requires_unavailable_dvbsub)),
                        disabled_reason: reason,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[must_use]
pub fn subtitle_font_options(
    config: &ConversionConfig,
    fonts: &[String],
    disabled: bool,
) -> Vec<SubtitleFontOption> {
    let selected = config.subtitle_font_name.as_deref().unwrap_or_default();
    fonts
        .iter()
        .map(|font| SubtitleFontOption {
            name: font.clone(),
            is_selected: selected == font,
            is_disabled: disabled,
        })
        .collect()
}

#[must_use]
pub fn subtitle_font_size_options(
    config: &ConversionConfig,
    disabled: bool,
) -> [SubtitleFontSizeOption; 14] {
    let selected = config.subtitle_font_size.as_deref().unwrap_or_default();
    SUBTITLE_FONT_SIZES.map(|size| SubtitleFontSizeOption {
        size,
        is_selected: selected == size,
        is_disabled: disabled,
    })
}

#[must_use]
pub fn subtitle_position_options(
    config: &ConversionConfig,
    disabled: bool,
) -> [SubtitlePositionOption; 3] {
    let selected = subtitle_position(config);
    SUBTITLE_POSITIONS.map(|position| SubtitlePositionOption {
        position,
        label: position.label(),
        is_selected: selected == position,
        is_disabled: disabled,
    })
}

#[must_use]
pub fn subtitle_position(config: &ConversionConfig) -> SubtitlePosition {
    config
        .subtitle_position
        .as_deref()
        .and_then(SubtitlePosition::from_id)
        .unwrap_or(crate::settings::model::DEFAULT_SUBTITLE_POSITION)
}

#[must_use]
pub fn subtitle_burn_file_label(config: &ConversionConfig) -> String {
    config
        .subtitle_burn_path
        .as_deref()
        .and_then(|path| path.rsplit(['/', '\\']).next())
        .filter(|name| !name.is_empty())
        .map_or_else(
            || "Select .srt or .ass file".to_string(),
            ToString::to_string,
        )
}

#[must_use]
pub fn subtitle_color_value(value: Option<&String>, fallback: &str) -> String {
    value
        .and_then(|value| normalized_hex_color(value.as_str()))
        .unwrap_or_else(|| fallback.to_string())
}

#[must_use]
pub fn normalized_hex_color(value: &str) -> Option<String> {
    let source = value.trim().trim_start_matches('#');
    if source.len() == 3 && source.chars().all(|ch| ch.is_ascii_hexdigit()) {
        let mut expanded = String::from("#");
        for ch in source.chars() {
            expanded.push(ch.to_ascii_lowercase());
            expanded.push(ch.to_ascii_lowercase());
        }
        return Some(expanded);
    }
    if source.len() == 6 && source.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Some(format!("#{}", source.to_ascii_lowercase()));
    }

    None
}

pub(super) fn subtitle_track_detail(language: Option<&str>, label: Option<&str>) -> String {
    [language, label]
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" • ")
}
