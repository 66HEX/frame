use super::*;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "This regression test intentionally keeps a full conversion config fixture in one assertion flow."
)]
fn core_config_from_gpui_preserves_active_conversion_fields() {
    let config = GpuiConversionConfig {
        processing_mode: ProcessingMode::Copy,
        container: "mov".to_string(),
        audio_codec: "aac".to_string(),
        audio_bitrate: "192".to_string(),
        audio_bitrate_mode: "bitrate".to_string(),
        opus_vbr: true,
        audio_quality: "4".to_string(),
        audio_channels: "stereo".to_string(),
        audio_volume: 125,
        audio_normalize: true,
        external_subtitle_tracks: vec![ExternalSubtitleTrack {
            path: "/tmp/dialogue.srt".to_string(),
            language: Some("eng".to_string()),
            title: Some("English".to_string()),
            is_default: true,
            is_forced: false,
        }],
        video_filters: VideoFiltersConfig {
            color: VideoColorFiltersConfig {
                brightness: FilterValue {
                    enabled: true,
                    value: 15,
                },
                contrast: FilterValue {
                    enabled: true,
                    value: 120,
                },
                saturation: FilterValue {
                    enabled: true,
                    value: 80,
                },
                gamma: FilterValue {
                    enabled: true,
                    value: 110,
                },
            },
            hue: FilterValue {
                enabled: true,
                value: -30,
            },
            temperature: FilterValue {
                enabled: true,
                value: 4200,
            },
            sharpen: FilterValue {
                enabled: true,
                value: 40,
            },
            gaussian_blur: FilterValue {
                enabled: true,
                value: 10,
            },
            denoise_enabled: true,
            denoise_strength: FilterStrength::High,
            deband: FilterValue {
                enabled: true,
                value: 50,
            },
            vignette: FilterValue {
                enabled: true,
                value: 35,
            },
            grayscale: true,
            deinterlace: DeinterlaceMode::Auto,
        },
        audio_filters: AudioFiltersConfig {
            compressor_enabled: true,
            compressor_strength: FilterStrength::Low,
            limiter: FilterValue {
                enabled: true,
                value: -3,
            },
            bass: FilterValue {
                enabled: true,
                value: 6,
            },
            treble: FilterValue {
                enabled: true,
                value: -4,
            },
            high_pass: FilterValue {
                enabled: true,
                value: 120,
            },
            low_pass: FilterValue {
                enabled: true,
                value: 15_000,
            },
            noise_reduction: FilterValue {
                enabled: true,
                value: 10,
            },
            de_esser: FilterValue {
                enabled: true,
                value: 45,
            },
            stereo_width: FilterValue {
                enabled: true,
                value: 125,
            },
        },
        start_time: Some("00:00:05.000".to_string()),
        end_time: Some("00:00:15.000".to_string()),
        metadata: MetadataConfig {
            mode: MetadataMode::Replace,
            title: Some("Render Title".to_string()),
            artist: Some("Frame".to_string()),
            service_name: Some("Frame Service".to_string()),
            service_provider: Some("Frame Provider".to_string()),
            ..MetadataConfig::default()
        },
        subtitle_burn_path: Some("/tmp/dialogue.srt".to_string()),
        subtitle_font_name: Some("Arial".to_string()),
        subtitle_font_size: Some("24".to_string()),
        subtitle_font_color: Some("#ffffff".to_string()),
        subtitle_outline_color: Some("#000000".to_string()),
        subtitle_position: Some("bottom".to_string()),
        rotation: "90".to_string(),
        flip_horizontal: true,
        flip_vertical: true,
        crop: Some(CropSettings {
            enabled: true,
            x: 10,
            y: 20,
            width: 300,
            height: 200,
            source_width: Some(1920),
            source_height: Some(1080),
            aspect_ratio: Some("16:9".to_string()),
        }),
        overlay: None,
        selected_audio_tracks: vec![1, 2],
        selected_subtitle_tracks: vec![3],
        video_codec: "libx265".to_string(),
        video_bitrate_mode: "bitrate".to_string(),
        video_bitrate: "9000".to_string(),
        resolution: "custom".to_string(),
        custom_width: Some("1920".to_string()),
        custom_height: Some("1080".to_string()),
        scaling_algorithm: "lanczos".to_string(),
        fps: "30".to_string(),
        crf: 18,
        quality: 60,
        preset: "slow".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        image_jpeg_quality: 92,
        image_jpeg_huffman: "optimal".to_string(),
        image_webp_lossless: true,
        image_webp_quality: 88,
        image_webp_compression: 6,
        image_webp_preset: "photo".to_string(),
        image_png_compression: 8,
        image_png_prediction: "mixed".to_string(),
        image_tiff_compression: "deflate".to_string(),
        gif_colors: 128,
        gif_dither: "floyd_steinberg".to_string(),
        gif_loop: 3,
        nvenc_spatial_aq: false,
        nvenc_temporal_aq: false,
        videotoolbox_allow_sw: false,
        hw_decode: false,
    };

    let core = core_config_from_gpui(&config);

    assert_eq!(core.processing_mode, "copy");
    assert_eq!(core.container, "mov");
    assert_eq!(core.audio_bitrate, "192");
    assert_eq!(core.audio_channels, "stereo");
    assert_eq!(core.audio_volume, 125.0);
    assert!(core.audio_normalize);
    assert_eq!(core.video_codec, "libx265");
    assert_eq!(core.video_bitrate_mode, "bitrate");
    assert_eq!(core.video_bitrate, "9000");
    assert_eq!(core.resolution, "custom");
    assert_eq!(core.custom_width.as_deref(), Some("1920"));
    assert_eq!(core.custom_height.as_deref(), Some("1080"));
    assert_eq!(core.scaling_algorithm, "lanczos");
    assert_eq!(core.fps, "30");
    assert_eq!(core.crf, 18);
    assert_eq!(core.quality, 60);
    assert_eq!(core.preset, "slow");
    assert_eq!(core.pixel_format, "yuv420p10le");
    assert_eq!(core.image_jpeg_quality, 92);
    assert_eq!(core.image_jpeg_huffman, "optimal");
    assert!(core.image_webp_lossless);
    assert_eq!(core.image_webp_quality, 88);
    assert_eq!(core.image_webp_compression, 6);
    assert_eq!(core.image_webp_preset, "photo");
    assert_eq!(core.image_png_compression, 8);
    assert_eq!(core.image_png_prediction, "mixed");
    assert_eq!(core.image_tiff_compression, "deflate");
    assert_eq!(core.gif_colors, 128);
    assert_eq!(core.gif_dither, "floyd_steinberg");
    assert_eq!(core.gif_loop, 3);
    assert_eq!(core.start_time.as_deref(), Some("00:00:05.000"));
    assert_eq!(core.end_time.as_deref(), Some("00:00:15.000"));
    assert_eq!(core.metadata.service_name.as_deref(), Some("Frame Service"));
    assert_eq!(
        core.metadata.service_provider.as_deref(),
        Some("Frame Provider")
    );
    assert_eq!(core.rotation, "90");
    assert!(core.flip_horizontal);
    assert!(core.flip_vertical);
    assert_eq!(core.selected_audio_tracks, [1, 2]);
    assert_eq!(core.selected_subtitle_tracks, [3]);
    assert_eq!(core.external_subtitle_tracks.len(), 1);
    assert_eq!(core.external_subtitle_tracks[0].path, "/tmp/dialogue.srt");
    assert_eq!(
        core.external_subtitle_tracks[0].language.as_deref(),
        Some("eng")
    );
    assert!(core.external_subtitle_tracks[0].is_default);
    assert_eq!(
        core.subtitle_burn_path.as_deref(),
        Some("/tmp/dialogue.srt")
    );
    assert_eq!(core.subtitle_font_name.as_deref(), Some("Arial"));
    assert_eq!(core.subtitle_font_size.as_deref(), Some("24"));
    assert_eq!(core.subtitle_font_color.as_deref(), Some("#ffffff"));
    assert_eq!(core.subtitle_outline_color.as_deref(), Some("#000000"));
    assert_eq!(core.subtitle_position.as_deref(), Some("bottom"));
    assert_eq!(core.crop.as_ref().map(|crop| crop.width), Some(300.0));
    assert_eq!(core.metadata.mode, frame_core::types::MetadataMode::Replace);
    assert_eq!(core.metadata.title.as_deref(), Some("Render Title"));
    assert_eq!(core.metadata.artist.as_deref(), Some("Frame"));
}

#[test]
fn opus_vbr_legacy_default_and_explicit_values_survive_app_core_mapping() {
    let legacy = serde_json::json!({"container": "webm", "audioCodec": "libopus"});
    let config: GpuiConversionConfig = serde_json::from_value(legacy).unwrap();
    assert!(config.opus_vbr);
    assert!(core_config_from_gpui(&config).opus_vbr);
    for opus_vbr in [false, true] {
        let config = GpuiConversionConfig {
            opus_vbr,
            ..config.clone()
        };
        let encoded = serde_json::to_value(&config).unwrap();
        assert_eq!(encoded["opusVbr"], opus_vbr);
        let restored: GpuiConversionConfig = serde_json::from_value(encoded).unwrap();
        assert_eq!(restored, config);
        let core = core_config_from_gpui(&restored);
        assert_eq!(core.opus_vbr, opus_vbr);
        let core_json = serde_json::to_value(core).unwrap();
        let core: frame_core::types::ConversionConfig = serde_json::from_value(core_json).unwrap();
        assert_eq!(core.opus_vbr, opus_vbr);
    }
}
