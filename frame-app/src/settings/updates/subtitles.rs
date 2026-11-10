use crate::file_filters::is_supported_selectable_subtitle_path_for_container;
use crate::settings::model::{
    ConversionConfig, ExternalSubtitleTrack, SUBTITLE_FONT_SIZES, SubtitlePosition,
};
use crate::settings::options::normalized_hex_color;

pub fn apply_subtitle_burn_path(config: &mut ConversionConfig, path: Option<String>) -> bool {
    let path = path
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty());
    if config.subtitle_burn_path == path {
        return false;
    }

    config.subtitle_burn_path = path;
    true
}

pub fn add_external_subtitle_tracks(
    config: &mut ConversionConfig,
    paths: impl IntoIterator<Item = String>,
) -> Option<usize> {
    let mut last_added = None;
    for path in paths {
        let path = path.trim();
        if path.is_empty()
            || !is_supported_selectable_subtitle_path_for_container(
                std::path::Path::new(path),
                &config.container,
            )
            || config
                .external_subtitle_tracks
                .iter()
                .any(|track| track.path == path)
        {
            continue;
        }

        config.external_subtitle_tracks.push(ExternalSubtitleTrack {
            path: path.to_string(),
            ..ExternalSubtitleTrack::default()
        });
        last_added = Some(config.external_subtitle_tracks.len() - 1);
    }
    last_added
}

pub fn remove_external_subtitle_track(config: &mut ConversionConfig, index: usize) -> bool {
    if index >= config.external_subtitle_tracks.len() {
        return false;
    }
    config.external_subtitle_tracks.remove(index);
    true
}

pub fn apply_external_subtitle_language(
    config: &mut ConversionConfig,
    index: usize,
    language: &str,
) -> bool {
    let Some(track) = config.external_subtitle_tracks.get_mut(index) else {
        return false;
    };
    let language = normalize_subtitle_metadata_value(language);
    if track.language == language {
        return false;
    }
    track.language = language;
    true
}

pub fn apply_external_subtitle_title(
    config: &mut ConversionConfig,
    index: usize,
    title: &str,
) -> bool {
    let Some(track) = config.external_subtitle_tracks.get_mut(index) else {
        return false;
    };
    let title = normalize_subtitle_metadata_value(title);
    if track.title == title {
        return false;
    }
    track.title = title;
    true
}

pub fn apply_external_subtitle_default(
    config: &mut ConversionConfig,
    index: usize,
    is_default: bool,
) -> bool {
    if index >= config.external_subtitle_tracks.len() {
        return false;
    }

    let mut changed = false;
    for (track_index, track) in config.external_subtitle_tracks.iter_mut().enumerate() {
        let next = is_default && track_index == index;
        if track.is_default != next && (track_index == index || is_default) {
            track.is_default = next;
            changed = true;
        }
    }
    changed
}

pub fn apply_external_subtitle_forced(
    config: &mut ConversionConfig,
    index: usize,
    is_forced: bool,
) -> bool {
    let Some(track) = config.external_subtitle_tracks.get_mut(index) else {
        return false;
    };
    if track.is_forced == is_forced {
        return false;
    }
    track.is_forced = is_forced;
    true
}

pub(super) fn normalize_subtitle_metadata_value(value: &str) -> Option<String> {
    let value: String = value.chars().filter(|ch| !ch.is_control()).collect();
    (!value.trim().is_empty()).then_some(value)
}

pub fn apply_subtitle_font_name(config: &mut ConversionConfig, font: &str) -> bool {
    let font = font.trim();
    let font = if font.is_empty() {
        None
    } else {
        Some(font.to_string())
    };
    if config.subtitle_font_name == font {
        return false;
    }

    config.subtitle_font_name = font;
    true
}

pub fn apply_subtitle_font_size(config: &mut ConversionConfig, size: &str) -> bool {
    let size = size.trim();
    let size = if size.is_empty() {
        None
    } else if SUBTITLE_FONT_SIZES.contains(&size) {
        Some(size.to_string())
    } else {
        return false;
    };

    if config.subtitle_font_size == size {
        return false;
    }

    config.subtitle_font_size = size;
    true
}

pub fn apply_subtitle_font_color(config: &mut ConversionConfig, color: &str) -> bool {
    apply_subtitle_color(&mut config.subtitle_font_color, color)
}

pub fn apply_subtitle_outline_color(config: &mut ConversionConfig, color: &str) -> bool {
    apply_subtitle_color(&mut config.subtitle_outline_color, color)
}

pub fn apply_subtitle_position(config: &mut ConversionConfig, position: SubtitlePosition) -> bool {
    let position = Some(position.id().to_string());
    if config.subtitle_position == position {
        return false;
    }

    config.subtitle_position = position;
    true
}

pub fn toggle_subtitle_track_selection(config: &mut ConversionConfig, index: u32) -> bool {
    if config.selected_subtitle_tracks.contains(&index) {
        config
            .selected_subtitle_tracks
            .retain(|selected_index| *selected_index != index);
    } else {
        config.selected_subtitle_tracks.push(index);
    }

    true
}

pub(super) fn apply_subtitle_color(target: &mut Option<String>, color: &str) -> bool {
    let Some(color) = normalized_hex_color(color) else {
        return false;
    };
    if target.as_deref() == Some(color.as_str()) {
        return false;
    }

    *target = Some(color);
    true
}
