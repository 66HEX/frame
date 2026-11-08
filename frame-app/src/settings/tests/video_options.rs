use super::*;
use frame_core::capabilities::AvailableEncoders;

fn codec_option<'a>(options: &'a [VideoCodecOption], codec: &str) -> Option<&'a VideoCodecOption> {
    options.iter().find(|option| option.codec == codec)
}

#[test]
fn default_config_matches_original_video_defaults() {
    let config = ConversionConfig::default();

    assert_eq!(config.video_codec, "libx264");
    assert_eq!(config.video_bitrate_mode, "crf");
    assert_eq!(config.video_bitrate, "5000");
    assert_eq!(config.resolution, "original");
    assert_eq!(config.scaling_algorithm, "bicubic");
    assert_eq!(config.fps, "original");
    assert_eq!(config.crf, 23);
    assert_eq!(config.quality, 50);
    assert_eq!(config.preset, "medium");
    assert_eq!(config.pixel_format, "auto");
    assert_eq!(config.image_jpeg_quality, 85);
    assert_eq!(config.image_jpeg_huffman, "optimal");
    assert!(!config.image_webp_lossless);
    assert_eq!(config.image_webp_quality, 75);
    assert_eq!(config.image_webp_compression, 4);
    assert_eq!(config.image_webp_preset, "default");
    assert_eq!(config.image_png_compression, 9);
    assert_eq!(config.image_png_prediction, "paeth");
    assert_eq!(config.image_tiff_compression, "packbits");
    assert_eq!(config.gif_colors, 256);
    assert_eq!(config.gif_dither, "sierra2_4a");
    assert_eq!(config.gif_loop, 0);
}

#[test]
fn video_codec_options_hide_unavailable_hardware_encoders() {
    let options = video_codec_options(
        &ConversionConfig::default(),
        &AvailableEncoders::default(),
        false,
    );

    assert!(codec_option(&options, "h264_videotoolbox").is_none());
    assert!(codec_option(&options, "h264_nvenc").is_none());
}

#[test]
fn video_codec_options_show_available_hardware_encoders() {
    let encoders = AvailableEncoders {
        h264_videotoolbox: true,
        ..AvailableEncoders::default()
    };

    let options = video_codec_options(&ConversionConfig::default(), &encoders, false);

    assert!(codec_option(&options, "h264_videotoolbox").is_some());
}

#[test]
fn apply_video_codec_rejects_container_incompatible_codec() {
    let mut config = ConversionConfig {
        container: "webm".to_string(),
        ..ConversionConfig::default()
    };

    assert!(!apply_video_codec(&mut config, "libx264"));

    assert_eq!(config.video_codec, "libx264");
}

#[test]
fn apply_pixel_format_rejects_incompatible_encoder_format_pair() {
    let mut config = ConversionConfig {
        container: "mp4".to_string(),
        video_codec: "vp9".to_string(),
        ..ConversionConfig::default()
    };

    assert!(!apply_pixel_format(&mut config, "yuv420p10le"));

    assert_eq!(config.pixel_format, "auto");
}

#[test]
fn normalize_video_config_for_gif_forces_original_gif_contract() {
    let mut config = ConversionConfig {
        container: "gif".to_string(),
        video_codec: "libx264".to_string(),
        pixel_format: "yuv420p".to_string(),
        hw_decode: true,
        ..ConversionConfig::default()
    };

    assert_normalizes_once(normalize_video_config, &mut config, None);

    assert_eq!(config.video_codec, "gif");
    assert_eq!(config.pixel_format, "auto");
    assert!(!config.hw_decode);
}

#[test]
fn normalize_video_config_resets_visual_filters_in_copy_mode() {
    let mut config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        resolution: "720p".to_string(),
        fps: "30".to_string(),
        pixel_format: "yuv420p".to_string(),
        flip_horizontal: true,
        ..ConversionConfig::default()
    };

    assert_normalizes_once(normalize_video_config, &mut config, None);

    assert_eq!(config.resolution, "original");
    assert_eq!(config.fps, "original");
    assert_eq!(config.pixel_format, "auto");
    assert!(!config.flip_horizontal);
}

#[test]
fn apply_gif_loop_strips_non_digits_and_clamps_to_ffmpeg_range() {
    let mut config = ConversionConfig::default();

    assert!(apply_gif_loop(&mut config, "999999x"));

    assert_eq!(config.gif_loop, 65_535);
}

#[test]
fn vp9_speed_options_are_native_and_legacy_default_is_migrated_once() {
    let mut config = ConversionConfig {
        container: "webm".to_string(),
        video_codec: "vp9".to_string(),
        ..ConversionConfig::default()
    };
    assert_normalizes_once(normalize_video_config, &mut config, None);
    assert_eq!(config.preset, "1");
    let options = video_preset_options(&config, false);
    assert_eq!(options.len(), 6);
    assert_eq!(
        options.iter().filter(|option| option.is_selected).count(),
        1
    );
    for option in options {
        assert!(!option.is_disabled);
        assert!(is_video_preset_allowed("vp9", option.preset));
        apply_video_preset(&mut config, option.preset);
        assert_eq!(config.preset, option.preset);
    }
    assert!(!apply_video_preset(&mut config, "6"));
    assert!(!apply_video_preset(&mut config, "fast"));
}

#[test]
fn prores_normalization_preserves_unused_quality_drafts() {
    let mut config = ConversionConfig {
        container: "mov".to_string(),
        video_codec: "prores".to_string(),
        video_bitrate_mode: "bitrate".to_string(),
        video_bitrate: "unused value".to_string(),
        preset: "unused preset".to_string(),
        crf: 17,
        ..ConversionConfig::default()
    };
    normalize_video_config(&mut config, None);
    assert_eq!(config.video_bitrate, "unused value");
    assert_eq!(config.preset, "unused preset");
    assert_eq!(config.crf, 17);
}
