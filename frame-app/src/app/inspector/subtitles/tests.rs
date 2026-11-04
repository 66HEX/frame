use super::*;
use crate::app::components::frame_select_target_index;
use crate::app::*;

#[test]
fn m2ts_sidecars_hide_all_unrepresentable_metadata_controls() {
    assert_eq!(
        external_subtitle_metadata_visibility("m2ts"),
        ExternalSubtitleMetadataVisibility {
            language: false,
            descriptive: false,
        }
    );
}

#[test]
fn m2t_sidecars_show_only_representable_language_control() {
    assert_eq!(
        external_subtitle_metadata_visibility("m2t"),
        ExternalSubtitleMetadataVisibility {
            language: true,
            descriptive: false,
        }
    );
}

#[test]
fn subtitle_font_keyboard_selection_skips_disabled_options() {
    let options = [
        SubtitleFontOption {
            name: "Alpha".to_string(),
            is_selected: true,
            is_disabled: false,
        },
        SubtitleFontOption {
            name: "Beta".to_string(),
            is_selected: false,
            is_disabled: true,
        },
        SubtitleFontOption {
            name: "Gamma".to_string(),
            is_selected: false,
            is_disabled: false,
        },
    ];

    let down_index = frame_select_target_index(options.len(), Some(0), "down", |index| {
        !options[index].is_disabled
    });
    let up_index = frame_select_target_index(options.len(), Some(0), "up", |index| {
        !options[index].is_disabled
    });

    assert_eq!(
        down_index.map(|index| options[index].name.as_str()),
        Some("Gamma")
    );
    assert_eq!(
        up_index.map(|index| options[index].name.as_str()),
        Some("Gamma")
    );
}

#[test]
fn subtitle_font_size_keyboard_selection_supports_home_and_end() {
    let options = [
        SubtitleFontSizeOption {
            size: "12",
            is_selected: false,
            is_disabled: true,
        },
        SubtitleFontSizeOption {
            size: "14",
            is_selected: true,
            is_disabled: false,
        },
        SubtitleFontSizeOption {
            size: "16",
            is_selected: false,
            is_disabled: false,
        },
    ];

    let home_index = frame_select_target_index(options.len(), Some(1), "home", |index| {
        !options[index].is_disabled
    });
    let end_index = frame_select_target_index(options.len(), Some(1), "end", |index| {
        !options[index].is_disabled
    });

    assert_eq!(home_index.map(|index| options[index].size), Some("14"));
    assert_eq!(end_index.map(|index| options[index].size), Some("16"));
}
