use super::*;

#[test]
fn default_video_source_matches_original_default_tab_set() {
    let tabs = tab_ids(super::visible_settings_tabs(
        &ConversionConfig::default(),
        None,
    ));

    assert_eq!(
        tabs,
        vec![
            "source",
            "output",
            "video",
            "video-filters",
            "audio",
            "audio-filters",
            "subtitles",
            "metadata",
            "presets"
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
    let tabs = tab_ids(super::visible_settings_tabs(
        &ConversionConfig {
            container: "mp3".to_string(),
            ..ConversionConfig::default()
        },
        Some(&metadata),
    ));

    assert_eq!(
        tabs,
        vec![
            "source",
            "output",
            "audio",
            "audio-filters",
            "metadata",
            "presets"
        ]
    );
}

#[test]
fn image_source_shows_images_and_hides_video_audio_subtitles() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        video_codec: Some("png".to_string()),
        ..SourceMetadata::default()
    };
    let tabs = tab_ids(super::visible_settings_tabs(
        &ConversionConfig {
            container: "png".to_string(),
            ..ConversionConfig::default()
        },
        Some(&metadata),
    ));

    assert_eq!(
        tabs,
        vec![
            "source",
            "output",
            "video-filters",
            "images",
            "metadata",
            "presets"
        ]
    );
}

#[test]
fn copy_mode_hides_video_tab_but_keeps_audio_and_subtitles_when_supported() {
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        ..ConversionConfig::default()
    };
    let tabs = tab_ids(super::visible_settings_tabs(&config, None));

    assert_eq!(
        tabs,
        vec![
            "source",
            "output",
            "audio",
            "subtitles",
            "metadata",
            "presets"
        ]
    );
}

#[test]
fn active_hidden_tab_falls_back_to_output() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Audio),
        video_codec: None,
        ..SourceMetadata::default()
    };
    let active = resolve_active_settings_tab(
        SettingsTab::Video,
        &ConversionConfig::default(),
        Some(&metadata),
    );

    assert_eq!(active, SettingsTab::Output);
}
