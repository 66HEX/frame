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
        &crate::capabilities::HardwareCapabilities::default(),
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

    let options = video_codec_options(
        &ConversionConfig::default(),
        &encoders,
        &crate::capabilities::HardwareCapabilities::default(),
        false,
    );

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

#[test]
fn videotoolbox_software_fallback_is_reachable_from_a_software_encoder() {
    use crate::capabilities::{
        EncoderInventory, HardwareCapabilities, HardwareEncoder, HardwareProbePolicy,
        HardwareProbeReport, HardwareReason, HardwareStatus,
    };
    let encoder = HardwareEncoder::H264Videotoolbox;
    let encoders = AvailableEncoders {
        h264_videotoolbox: true,
        ..AvailableEncoders::default()
    };
    let report = |allow, status, reason| HardwareProbeReport {
        policy: HardwareProbePolicy::new(encoder, allow),
        status,
        reason,
        diagnostics: String::new(),
        checked_at: std::time::SystemTime::UNIX_EPOCH,
    };
    let hardware = HardwareCapabilities {
        inventory: EncoderInventory::Complete,
        reports: vec![
            report(
                false,
                HardwareStatus::Unavailable,
                Some(HardwareReason::InitializationFailed),
            ),
            report(true, HardwareStatus::Available, None),
        ],
    };
    let mut config = ConversionConfig::default();
    let options = video_codec_options(&config, &encoders, &hardware, false);
    let strict = options
        .iter()
        .find(|option| option.codec == encoder.name() && !option.allow_software_fallback)
        .unwrap();
    let relaxed = options
        .iter()
        .find(|option| option.codec == encoder.name() && option.allow_software_fallback)
        .unwrap();
    assert!(strict.is_disabled);
    assert!(!relaxed.is_disabled);
    assert!(apply_video_encoder_choice(
        &mut config,
        relaxed.codec,
        relaxed.allow_software_fallback
    ));
    assert_eq!(config.video_codec, encoder.name());
    assert!(config.videotoolbox_allow_sw);
    let options = video_codec_options(&config, &encoders, &hardware, false);
    assert!(options.iter().any(|option| option.codec == encoder.name()
        && option.allow_software_fallback
        && option.is_selected));
    assert!(apply_videotoolbox_allow_sw(&mut config, false));
    assert!(
        video_codec_options(&config, &encoders, &hardware, false)
            .iter()
            .any(|option| option.codec == encoder.name()
                && !option.allow_software_fallback
                && option.is_selected
                && option.is_disabled)
    );
}

#[test]
fn unavailable_selected_hardware_preserves_the_draft_and_distinguishes_inventory_failure() {
    use crate::capabilities::{EncoderInventory, HardwareCapabilities};
    let config = ConversionConfig {
        video_codec: "h264_nvenc".into(),
        preset: "slow".into(),
        hw_decode: true,
        nvenc_spatial_aq: true,
        ..ConversionConfig::default()
    };
    let before = config.clone();
    for (inventory, reason) in [
        (
            EncoderInventory::Unknown,
            "Encoder availability not checked",
        ),
        (EncoderInventory::Failed, "Encoder list could not be read"),
        (
            EncoderInventory::Complete,
            "Encoder is not included in this FFmpeg",
        ),
    ] {
        let options = video_codec_options(
            &config,
            &AvailableEncoders::default(),
            &HardwareCapabilities {
                inventory,
                reports: Vec::new(),
            },
            false,
        );
        let selected = options.iter().find(|option| option.is_selected).unwrap();
        assert_eq!(selected.codec, "h264_nvenc");
        assert!(selected.is_disabled);
        assert_eq!(selected.disabled_reason, Some(reason));
    }
    assert_eq!(config, before);
}
