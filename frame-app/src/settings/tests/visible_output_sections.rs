use super::*;

#[test]
fn default_video_source_exposes_all_video_output_sections() {
    let sections = section_ids(super::visible_output_sections(
        &ConversionConfig::default(),
        None,
    ));

    assert_eq!(
        sections,
        vec![
            "general",
            "video",
            "video-filters",
            "audio",
            "audio-filters",
            "subtitles",
            "metadata"
        ]
    );
}

#[test]
fn audio_source_hides_video_images_and_subtitles() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Audio),
        video_codec: None,
        ..SourceMetadata::default()
    };
    let sections = section_ids(super::visible_output_sections(
        &ConversionConfig {
            container: "mp3".to_string(),
            ..ConversionConfig::default()
        },
        Some(&metadata),
    ));

    assert_eq!(
        sections,
        vec!["general", "audio", "audio-filters", "metadata"]
    );
}

#[test]
fn image_source_shows_images_and_hides_video_audio_subtitles() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        video_codec: Some("png".to_string()),
        ..SourceMetadata::default()
    };
    let sections = section_ids(super::visible_output_sections(
        &ConversionConfig {
            container: "png".to_string(),
            ..ConversionConfig::default()
        },
        Some(&metadata),
    ));

    assert_eq!(
        sections,
        vec!["general", "images", "video-filters", "metadata"]
    );
}

#[test]
fn copy_mode_hides_video_settings_but_keeps_audio_and_subtitles_when_supported() {
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        ..ConversionConfig::default()
    };
    let sections = section_ids(super::visible_output_sections(&config, None));

    assert_eq!(sections, vec!["general", "audio", "subtitles", "metadata"]);
}

#[test]
fn inspector_navigation_is_independent_of_source_and_container() {
    assert_eq!(
        ALL_SETTINGS_TABS.map(SettingsTab::id),
        ["metadata", "output", "presets"]
    );
}
fn section_ids(sections: Vec<OutputSection>) -> Vec<&'static str> {
    sections.into_iter().map(OutputSection::id).collect()
}
