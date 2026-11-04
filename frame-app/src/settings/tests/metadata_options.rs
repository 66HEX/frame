use super::*;

fn tagged_metadata() -> SourceMetadata {
    SourceMetadata {
        tags: Some(SourceTags {
            title: Some("Original Title".to_string()),
            artist: Some("Original Artist".to_string()),
            album: Some("Original Album".to_string()),
            genre: Some("Documentary".to_string()),
            date: Some("2026".to_string()),
            comment: Some("Camera note".to_string()),
        }),
        ..SourceMetadata::default()
    }
}

#[test]
fn default_metadata_mode_matches_original_preserve_mode() {
    let config = ConversionConfig::default();

    assert_eq!(config.metadata.mode, MetadataMode::Preserve);
}

#[test]
fn metadata_mode_options_mark_current_mode_selected() {
    let config = ConversionConfig {
        metadata: MetadataConfig {
            mode: MetadataMode::Clean,
            ..MetadataConfig::default()
        },
        ..ConversionConfig::default()
    };

    let options = metadata_mode_options(&config, false);

    assert!(options[1].is_selected);
    assert_eq!(options[1].label, "Clean");
}

#[test]
fn metadata_field_options_use_source_tags_as_preserve_placeholders() {
    let options = metadata_field_options(
        &ConversionConfig::default(),
        Some(&tagged_metadata()),
        false,
    );

    assert_eq!(options[0].placeholder, "Original Title");
    assert_eq!(options[1].placeholder, "Original Artist");
}

#[test]
fn transport_stream_metadata_fields_use_program_tag_placeholders() {
    let config = ConversionConfig {
        container: "m2ts".to_string(),
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        transport_stream: Some(frame_core::types::TransportStreamMetadata {
            service_name: Some("Camera Service".to_string()),
            service_provider: Some("Camera Vendor".to_string()),
            ..frame_core::types::TransportStreamMetadata::default()
        }),
        ..SourceMetadata::default()
    };

    let options = metadata_field_options(&config, Some(&metadata), false);

    assert_eq!(options.len(), 2);
    assert_eq!(options[0].field, MetadataField::ServiceName);
    assert_eq!(options[0].placeholder, "Camera Service");
    assert_eq!(options[1].field, MetadataField::ServiceProvider);
    assert_eq!(options[1].placeholder, "Camera Vendor");
}

#[test]
fn switching_transport_container_preserves_generic_and_service_metadata_drafts() {
    let mut config = ConversionConfig {
        metadata: MetadataConfig {
            title: Some("Generic title".to_string()),
            service_name: Some("Service draft".to_string()),
            service_provider: Some("Provider draft".to_string()),
            ..MetadataConfig::default()
        },
        ..ConversionConfig::default()
    };

    apply_output_container(&mut config, "m2t");
    normalize_output_config(&mut config, None);
    apply_output_container(&mut config, "m2ts");
    normalize_output_config(&mut config, None);

    assert_eq!(config.metadata.title.as_deref(), Some("Generic title"));
    assert_eq!(
        config.metadata.service_name.as_deref(),
        Some("Service draft")
    );
    assert_eq!(
        config.metadata.service_provider.as_deref(),
        Some("Provider draft")
    );
}

#[test]
fn metadata_field_options_hide_album_and_genre_for_images() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..tagged_metadata()
    };

    let fields = metadata_field_options(&ConversionConfig::default(), Some(&metadata), false)
        .into_iter()
        .map(|option| option.id)
        .collect::<Vec<_>>();

    assert_eq!(fields, ["title", "artist", "date", "comment"]);
}

#[test]
fn metadata_field_options_use_blank_placeholder_outside_preserve_mode() {
    let config = ConversionConfig {
        metadata: MetadataConfig {
            mode: MetadataMode::Replace,
            ..MetadataConfig::default()
        },
        ..ConversionConfig::default()
    };

    let options = metadata_field_options(&config, Some(&tagged_metadata()), false);

    assert_eq!(options[0].placeholder, "");
}

#[test]
fn transport_replace_description_discloses_required_neutral_fallbacks() {
    let config = ConversionConfig {
        container: "mts".to_string(),
        metadata: MetadataConfig {
            mode: MetadataMode::Replace,
            ..MetadataConfig::default()
        },
        ..ConversionConfig::default()
    };

    assert_eq!(
        metadata_mode_description(&config),
        "MPEG-TS requires both service identity values. Empty Service name and Service provider fields are replaced with Service01 and Frame."
    );
}

#[test]
fn non_transport_replace_description_keeps_generic_contract() {
    let config = ConversionConfig {
        metadata: MetadataConfig {
            mode: MetadataMode::Replace,
            ..MetadataConfig::default()
        },
        ..ConversionConfig::default()
    };

    assert_eq!(
        metadata_mode_description(&config),
        MetadataMode::Replace.description()
    );
}

#[test]
fn apply_metadata_mode_updates_selected_mode() {
    let mut config = ConversionConfig::default();

    assert!(apply_metadata_mode(&mut config, MetadataMode::Replace));

    assert_eq!(config.metadata.mode, MetadataMode::Replace);
}

#[test]
fn apply_metadata_field_stores_text_value() {
    let mut config = ConversionConfig::default();

    assert!(apply_metadata_field(
        &mut config,
        MetadataField::Title,
        "Render Title",
    ));

    assert_eq!(config.metadata.title.as_deref(), Some("Render Title"));
}

#[test]
fn normalize_output_config_clears_image_only_hidden_metadata_fields() {
    let mut config = ConversionConfig {
        metadata: MetadataConfig {
            album: Some("Album".to_string()),
            genre: Some("Genre".to_string()),
            ..MetadataConfig::default()
        },
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..SourceMetadata::default()
    };

    assert!(normalize_output_config(&mut config, Some(&metadata)));

    assert_eq!(config.metadata.album, None);
    assert_eq!(config.metadata.genre, None);
}
