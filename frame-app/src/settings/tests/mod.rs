use super::*;

fn assert_normalizes_once(
    normalize: fn(&mut ConversionConfig, Option<&SourceMetadata>) -> bool,
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
) {
    let before = config.clone();
    let changed = normalize(config, metadata);
    assert!(changed);
    assert_eq!(changed, before != *config);

    let normalized = config.clone();
    let changed = normalize(config, metadata);
    assert!(!changed);
    assert_eq!(changed, normalized != *config);
}

mod source_metadata;

mod output_name;

mod output_options;

mod audio_track_options;

mod subtitle_options;

mod preset_options;

mod metadata_options;

mod audio_codec_options;

mod audio_encoding_options;

mod video_options;

mod image_encoding;

mod output_config;

mod source_info_formatting;

mod source_info_sections;

mod visible_output_sections;

mod av1_native_presets;

mod output_geometry;
