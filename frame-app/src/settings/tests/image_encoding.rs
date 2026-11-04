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

    assert!(normalize_video_config(&mut config, None));

    assert_eq!(config.image_jpeg_quality, 1);
    assert_eq!(config.image_jpeg_huffman, "optimal");
    assert_eq!(config.image_webp_quality, 100);
    assert_eq!(config.image_webp_compression, 6);
    assert_eq!(config.image_webp_preset, "default");
    assert_eq!(config.image_png_compression, 9);
    assert_eq!(config.image_png_prediction, "paeth");
    assert_eq!(config.image_tiff_compression, "packbits");
}
