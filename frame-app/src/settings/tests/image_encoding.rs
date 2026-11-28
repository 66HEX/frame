use super::*;

#[test]
fn apply_image_jpeg_quality_clamps_to_visible_range() {
    let mut config = ConversionConfig::default();

    assert!(apply_image_jpeg_quality(&mut config, 150));

    assert_eq!(config.image_jpeg_quality, 100);
}

#[test]
fn apply_image_webp_compression_clamps_to_ffmpeg_range() {
    let mut config = ConversionConfig::default();

    assert!(apply_image_webp_compression(&mut config, 10));

    assert_eq!(config.image_webp_compression, 6);
}

#[test]
fn apply_image_png_prediction_rejects_unknown_mode() {
    let mut config = ConversionConfig::default();

    assert!(!apply_image_png_prediction(&mut config, "adaptive"));

    assert_eq!(config.image_png_prediction, "paeth");
}

#[test]
fn image_webp_preset_options_select_current_preset() {
    let config = ConversionConfig {
        image_webp_preset: "photo".to_string(),
        ..ConversionConfig::default()
    };

    let selected = image_webp_preset_options(&config, false)
        .into_iter()
        .find(|option| option.is_selected)
        .expect("selected webp preset should be present");

    assert_eq!(selected.id, "photo");
}

#[test]
fn normalize_video_config_repairs_invalid_image_encoding_values() {
    let mut config = ConversionConfig {
        image_jpeg_quality: 0,
        image_jpeg_huffman: "fancy".to_string(),
        image_webp_quality: 200,
        image_webp_compression: 9,
        image_webp_preset: "portrait".to_string(),
        image_png_compression: 12,
        image_png_prediction: "adaptive".to_string(),
        image_tiff_compression: "zip".to_string(),
        ..ConversionConfig::default()
    };

    assert_normalizes_once(normalize_video_config, &mut config, None);

    assert_eq!(config.image_jpeg_quality, 1);
    assert_eq!(config.image_jpeg_huffman, "optimal");
    assert_eq!(config.image_webp_quality, 100);
    assert_eq!(config.image_webp_compression, 6);
    assert_eq!(config.image_webp_preset, "default");
    assert_eq!(config.image_png_compression, 9);
    assert_eq!(config.image_png_prediction, "paeth");
    assert_eq!(config.image_tiff_compression, "packbits");
}

#[test]
fn saved_jpeg_xl_preset_restores_the_compression_mode() {
    let image = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        ..SourceMetadata::default()
    };
    let mut config = ConversionConfig::default();
    assert!(apply_output_container(&mut config, "jxl"));
    normalize_output_config(&mut config, Some(&image));
    assert_eq!(config.video_codec, "libjxl");
    assert!(config.image_jxl_lossless);

    config.image_jxl_lossless = false;
    let preset = create_custom_preset("photo-jxl".into(), "Photo", &config);
    let stored = serde_json::to_value(preset).unwrap();
    let restored = serde_json::from_value(stored.clone()).unwrap();
    let mut next_image = ConversionConfig::default();
    assert!(apply_preset(&mut next_image, &restored, Some(&image)));
    assert_eq!(next_image.container, "jxl");
    assert!(!next_image.image_jxl_lossless);

    let mut legacy = stored;
    legacy["config"]
        .as_object_mut()
        .unwrap()
        .remove("imageJxlLossless");
    let restored: PresetDefinition = serde_json::from_value(legacy).unwrap();
    assert!(restored.config.image_jxl_lossless);
}
