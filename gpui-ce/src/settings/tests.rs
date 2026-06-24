use super::*;

fn tab_ids(tabs: Vec<SettingsTab>) -> Vec<&'static str> {
    tabs.into_iter().map(SettingsTab::id).collect()
}

mod source_metadata {
    use super::*;

    #[test]
    fn source_kind_falls_back_to_audio_when_metadata_has_no_video_codec() {
        let metadata = SourceMetadata::default();

        assert_eq!(metadata.source_kind(), SourceKind::Audio);
    }

    #[test]
    fn source_kind_defaults_to_video_when_metadata_is_missing() {
        assert_eq!(source_kind_for(None), SourceKind::Video);
    }
}

mod media_rules {
    use super::*;

    #[test]
    fn mp4_supports_audio_and_subtitles_like_original_rules() {
        assert!(container_supports_audio("mp4"));
        assert!(container_supports_subtitles("mp4"));
    }

    #[test]
    fn image_containers_do_not_support_audio_or_subtitles() {
        assert!(!container_supports_audio("png"));
        assert!(!container_supports_subtitles("png"));
    }

    #[test]
    fn mp4_rejects_flac_reencode_audio_like_original_rules() {
        assert!(!is_audio_codec_allowed_for_container("mp4", "flac"));
    }

    #[test]
    fn mov_accepts_any_audio_codec_like_original_rules() {
        assert!(is_audio_codec_allowed_for_container("mov", "flac"));
    }

    #[test]
    fn webm_default_audio_codec_matches_shared_rules() {
        assert_eq!(default_audio_codec_for_container("webm"), "libopus");
    }
}

mod output_name {
    use super::*;

    #[test]
    fn sanitize_output_name_keeps_only_last_path_segment() {
        assert_eq!(sanitize_output_name("/tmp/render/final.mp4"), "final.mp4");
    }

    #[test]
    fn sanitize_output_name_handles_windows_separators() {
        assert_eq!(sanitize_output_name("C:\\media\\final.mov"), "final.mov");
    }

    #[test]
    fn sanitize_output_name_rejects_dot_segments() {
        assert_eq!(sanitize_output_name(".."), "");
    }
}

mod output_options {
    use super::*;

    fn audio_metadata(codec: &str) -> SourceMetadata {
        SourceMetadata {
            media_kind: Some(SourceKind::Audio),
            audio_tracks: vec![AudioTrack {
                index: 0,
                codec: codec.to_string(),
                ..AudioTrack::default()
            }],
            ..SourceMetadata::default()
        }
    }

    fn image_metadata() -> SourceMetadata {
        SourceMetadata {
            media_kind: Some(SourceKind::Image),
            video_codec: Some("png".to_string()),
            ..SourceMetadata::default()
        }
    }

    fn video_metadata() -> SourceMetadata {
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            video_codec: Some("h264".to_string()),
            audio_tracks: vec![AudioTrack {
                index: 1,
                codec: "aac".to_string(),
                ..AudioTrack::default()
            }],
            subtitle_tracks: vec![SubtitleTrack {
                index: 2,
                codec: "subrip".to_string(),
            }],
            ..SourceMetadata::default()
        }
    }

    #[test]
    fn visible_output_containers_for_video_exclude_image_formats() {
        assert_eq!(
            visible_output_containers(None),
            vec![
                "mp4", "mkv", "webm", "mov", "gif", "mp3", "m4a", "wav", "flac"
            ]
        );
    }

    #[test]
    fn visible_output_containers_for_images_match_original_image_and_gif_set() {
        assert_eq!(
            visible_output_containers(Some(&image_metadata())),
            vec!["gif", "png", "jpg", "webp", "bmp", "tiff"]
        );
    }

    #[test]
    fn processing_mode_options_disable_copy_for_image_sources() {
        let options = output_processing_mode_options(
            &ConversionConfig::default(),
            Some(&image_metadata()),
            false,
        );

        assert!(options[1].is_disabled);
    }

    #[test]
    fn output_container_options_disable_video_targets_for_audio_sources() {
        let options = output_container_options(
            &ConversionConfig::default(),
            Some(&audio_metadata("aac")),
            false,
        );
        let mp4 = options
            .iter()
            .find(|option| option.container == "mp4")
            .expect("mp4 option should be visible for audio sources");

        assert_eq!(
            mp4.disabled_reason,
            Some("Video container unavailable for audio sources")
        );
    }

    #[test]
    fn stream_copy_audio_target_requires_compatible_audio_codec() {
        let config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            container: "mp3".to_string(),
            ..ConversionConfig::default()
        };

        assert!(!is_container_compatible_for_stream_copy(
            &config,
            Some(&audio_metadata("aac")),
            "mp3"
        ));
    }

    #[test]
    fn stream_copy_video_target_rejects_incompatible_subtitles() {
        let config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            selected_subtitle_tracks: vec![2],
            ..ConversionConfig::default()
        };

        assert!(!is_container_compatible_for_stream_copy(
            &config,
            Some(&video_metadata()),
            "mp4"
        ));
    }

    #[test]
    fn stream_copy_video_target_accepts_mkv_wildcard_rules() {
        let config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            selected_subtitle_tracks: vec![2],
            ..ConversionConfig::default()
        };

        assert!(is_container_compatible_for_stream_copy(
            &config,
            Some(&video_metadata()),
            "mkv"
        ));
    }

    #[test]
    fn stream_copy_without_metadata_keeps_non_image_containers_selectable() {
        let config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            ..ConversionConfig::default()
        };

        assert!(is_container_compatible_for_stream_copy(
            &config, None, "mp4"
        ));
    }
}

mod audio_track_options {
    use super::*;

    fn metadata_with_tracks() -> SourceMetadata {
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            audio_tracks: vec![
                AudioTrack {
                    index: 1,
                    codec: "aac".to_string(),
                    channels: Some("2".to_string()),
                    language: Some("eng".to_string()),
                    label: Some("Main".to_string()),
                    bitrate_kbps: Some(192.0),
                    ..AudioTrack::default()
                },
                AudioTrack {
                    index: 2,
                    codec: "ac3".to_string(),
                    channels: Some("6".to_string()),
                    ..AudioTrack::default()
                },
            ],
            ..SourceMetadata::default()
        }
    }

    #[test]
    fn marks_selected_audio_track() {
        let config = ConversionConfig {
            selected_audio_tracks: vec![2],
            ..ConversionConfig::default()
        };

        let options = audio_track_options(&config, Some(&metadata_with_tracks()), false);

        assert!(!options[0].is_selected);
        assert!(options[1].is_selected);
    }

    #[test]
    fn formats_track_detail_like_original_row_metadata() {
        let options = audio_track_options(
            &ConversionConfig::default(),
            Some(&metadata_with_tracks()),
            false,
        );

        assert_eq!(options[0].detail, "2 channels • eng • Main • 192 kb/s");
    }

    #[test]
    fn propagates_disabled_state_to_all_tracks() {
        let options = audio_track_options(
            &ConversionConfig::default(),
            Some(&metadata_with_tracks()),
            true,
        );

        assert!(options.iter().all(|option| option.is_disabled));
    }

    #[test]
    fn toggle_audio_track_selection_adds_missing_track() {
        let mut config = ConversionConfig::default();

        assert!(toggle_audio_track_selection(&mut config, 1));

        assert_eq!(config.selected_audio_tracks, vec![1]);
    }

    #[test]
    fn toggle_audio_track_selection_removes_selected_track() {
        let mut config = ConversionConfig {
            selected_audio_tracks: vec![1, 2],
            ..ConversionConfig::default()
        };

        assert!(toggle_audio_track_selection(&mut config, 1));

        assert_eq!(config.selected_audio_tracks, vec![2]);
    }
}

mod audio_codec_options {
    use super::*;

    fn codec_option<'a>(options: &'a [AudioCodecOption], codec: &str) -> &'a AudioCodecOption {
        options
            .iter()
            .find(|option| option.codec == codec)
            .unwrap_or_else(|| panic!("{codec} codec option should exist"))
    }

    #[test]
    fn marks_default_aac_selected_for_mp4() {
        let options = audio_codec_options(&ConversionConfig::default(), false);

        assert!(codec_option(&options, "aac").is_selected);
    }

    #[test]
    fn marks_flac_incompatible_for_mp4() {
        let options = audio_codec_options(&ConversionConfig::default(), false);

        assert_eq!(
            codec_option(&options, "flac").disabled_reason,
            Some("Incompatible container")
        );
    }

    #[test]
    fn keeps_mov_flac_codec_enabled() {
        let config = ConversionConfig {
            container: "mov".to_string(),
            ..ConversionConfig::default()
        };

        let options = audio_codec_options(&config, false);

        assert!(!codec_option(&options, "flac").is_disabled);
    }

    #[test]
    fn disables_all_codecs_in_stream_copy_mode() {
        let config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            ..ConversionConfig::default()
        };

        let options = audio_codec_options(&config, false);

        assert!(options.iter().all(|option| option.is_disabled));
    }

    #[test]
    fn apply_audio_codec_updates_allowed_codec() {
        let mut config = ConversionConfig {
            container: "mov".to_string(),
            ..ConversionConfig::default()
        };

        assert!(apply_audio_codec(&mut config, "flac"));

        assert_eq!(config.audio_codec, "flac");
    }

    #[test]
    fn apply_audio_codec_rejects_incompatible_codec() {
        let mut config = ConversionConfig::default();

        assert!(!apply_audio_codec(&mut config, "flac"));

        assert_eq!(config.audio_codec, "aac");
    }

    #[test]
    fn apply_audio_codec_rejects_unknown_codec() {
        let mut config = ConversionConfig::default();

        assert!(!apply_audio_codec(&mut config, "totally_unknown"));

        assert_eq!(config.audio_codec, "aac");
    }
}

mod audio_encoding_options {
    use super::*;

    fn channel_option<'a>(options: &'a [AudioChannelOption], id: &str) -> &'a AudioChannelOption {
        options
            .iter()
            .find(|option| option.id == id)
            .unwrap_or_else(|| panic!("{id} channel option should exist"))
    }

    #[test]
    fn default_config_matches_original_audio_defaults() {
        let config = ConversionConfig::default();

        assert_eq!(config.audio_bitrate, "128");
        assert_eq!(config.audio_bitrate_mode, "bitrate");
        assert_eq!(config.audio_quality, "4");
        assert_eq!(config.audio_channels, "original");
        assert_eq!(config.audio_volume, 100);
        assert!(!config.audio_normalize);
    }

    #[test]
    fn audio_channel_options_mark_original_selected_by_default() {
        let options = audio_channel_options(&ConversionConfig::default(), false);

        assert!(channel_option(&options, "original").is_selected);
    }

    #[test]
    fn audio_channel_options_disable_in_stream_copy_mode() {
        let config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            ..ConversionConfig::default()
        };

        let options = audio_channel_options(&config, false);

        assert!(options.iter().all(|option| option.is_disabled));
    }

    #[test]
    fn apply_audio_channels_updates_known_channel() {
        let mut config = ConversionConfig::default();

        assert!(apply_audio_channels(&mut config, "stereo"));

        assert_eq!(config.audio_channels, "stereo");
    }

    #[test]
    fn apply_audio_channels_rejects_unknown_channel() {
        let mut config = ConversionConfig::default();

        assert!(!apply_audio_channels(&mut config, "surround"));

        assert_eq!(config.audio_channels, "original");
    }

    #[test]
    fn apply_audio_bitrate_keeps_digits_only() {
        let mut config = ConversionConfig::default();

        assert!(apply_audio_bitrate(&mut config, " 192k "));

        assert_eq!(config.audio_bitrate, "192");
    }

    #[test]
    fn apply_audio_bitrate_mode_rejects_vbr_for_native_aac() {
        let mut config = ConversionConfig::default();

        assert!(!apply_audio_bitrate_mode(&mut config, "vbr"));

        assert_eq!(config.audio_bitrate_mode, "bitrate");
    }

    #[test]
    fn apply_audio_bitrate_mode_accepts_vbr_for_mp3() {
        let mut config = ConversionConfig {
            audio_codec: "mp3".to_string(),
            ..ConversionConfig::default()
        };

        assert!(apply_audio_bitrate_mode(&mut config, "vbr"));

        assert_eq!(config.audio_bitrate_mode, "vbr");
    }

    #[test]
    fn apply_audio_quality_clamps_mp3_quality_range() {
        let mut config = ConversionConfig {
            audio_codec: "mp3".to_string(),
            ..ConversionConfig::default()
        };

        assert!(apply_audio_quality(&mut config, "42"));

        assert_eq!(config.audio_quality, "9");
    }

    #[test]
    fn apply_audio_volume_clamps_to_original_slider_range() {
        let mut config = ConversionConfig::default();

        assert!(apply_audio_volume(&mut config, 250));

        assert_eq!(config.audio_volume, 200);
    }
}

mod output_config {
    use super::*;

    #[test]
    fn default_config_has_no_trim_times() {
        let config = ConversionConfig::default();

        assert_eq!(config.start_time, None);
        assert_eq!(config.end_time, None);
    }

    #[test]
    fn default_config_has_neutral_transform_and_no_crop() {
        let config = ConversionConfig::default();

        assert_eq!(config.rotation, "0");
        assert!(!config.flip_horizontal);
        assert!(!config.flip_vertical);
        assert_eq!(config.crop, None);
    }

    #[test]
    fn normalize_output_config_resets_audio_filter_controls_in_copy_mode() {
        let mut config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            audio_bitrate_mode: "vbr".to_string(),
            audio_volume: 150,
            audio_normalize: true,
            ..ConversionConfig::default()
        };

        normalize_output_config(&mut config, None);

        assert_eq!(config.audio_bitrate_mode, "bitrate");
        assert_eq!(config.audio_volume, 100);
        assert!(!config.audio_normalize);
    }

    #[test]
    fn apply_trim_times_stores_trim_bounds() {
        let mut config = ConversionConfig::default();

        assert!(apply_trim_times(
            &mut config,
            Some(" 00:00:05.000 ".to_string()),
            Some("00:00:30.250".to_string())
        ));

        assert_eq!(config.start_time.as_deref(), Some("00:00:05.000"));
        assert_eq!(config.end_time.as_deref(), Some("00:00:30.250"));
    }

    #[test]
    fn apply_trim_times_clears_blank_trim_bounds() {
        let mut config = ConversionConfig {
            start_time: Some("00:00:05.000".to_string()),
            end_time: Some("00:00:30.250".to_string()),
            ..ConversionConfig::default()
        };

        assert!(apply_trim_times(
            &mut config,
            Some(" ".to_string()),
            Some(String::new())
        ));

        assert_eq!(config.start_time, None);
        assert_eq!(config.end_time, None);
    }

    #[test]
    fn apply_trim_times_reports_no_change_for_same_bounds() {
        let mut config = ConversionConfig {
            start_time: Some("00:00:05.000".to_string()),
            end_time: Some("00:00:30.250".to_string()),
            ..ConversionConfig::default()
        };

        assert!(!apply_trim_times(
            &mut config,
            Some("00:00:05.000".to_string()),
            Some("00:00:30.250".to_string())
        ));
    }

    #[test]
    fn normalize_output_config_forces_audio_sources_to_audio_container() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Audio),
            ..SourceMetadata::default()
        };
        let mut config = ConversionConfig::default();

        normalize_output_config(&mut config, Some(&metadata));

        assert_eq!(config.container, "mp3");
    }

    #[test]
    fn normalize_output_config_forces_image_sources_to_image_container() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Image),
            ..SourceMetadata::default()
        };
        let mut config = ConversionConfig::default();

        normalize_output_config(&mut config, Some(&metadata));

        assert_eq!(config.container, "png");
    }

    #[test]
    fn normalize_output_config_clears_trim_for_image_sources() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Image),
            ..SourceMetadata::default()
        };
        let mut config = ConversionConfig {
            start_time: Some("00:00:05.000".to_string()),
            end_time: Some("00:00:30.250".to_string()),
            ..ConversionConfig::default()
        };

        assert!(normalize_output_config(&mut config, Some(&metadata)));

        assert_eq!(config.start_time, None);
        assert_eq!(config.end_time, None);
    }

    #[test]
    fn normalize_output_config_preserves_trim_for_video_sources() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Video),
            ..SourceMetadata::default()
        };
        let mut config = ConversionConfig {
            start_time: Some("00:00:05.000".to_string()),
            end_time: Some("00:00:30.250".to_string()),
            ..ConversionConfig::default()
        };

        normalize_output_config(&mut config, Some(&metadata));

        assert_eq!(config.start_time.as_deref(), Some("00:00:05.000"));
        assert_eq!(config.end_time.as_deref(), Some("00:00:30.250"));
    }

    #[test]
    fn normalize_output_config_clears_crop_for_audio_sources() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Audio),
            ..SourceMetadata::default()
        };
        let mut config = ConversionConfig {
            crop: Some(CropSettings {
                enabled: true,
                x: 100,
                y: 100,
                width: 200,
                height: 200,
                source_width: Some(1920),
                source_height: Some(1080),
                aspect_ratio: None,
            }),
            ..ConversionConfig::default()
        };

        assert!(normalize_output_config(&mut config, Some(&metadata)));

        assert_eq!(config.crop, None);
    }

    #[test]
    fn normalize_output_config_reencodes_image_sources() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Image),
            ..SourceMetadata::default()
        };
        let mut config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            container: "png".to_string(),
            ..ConversionConfig::default()
        };

        normalize_output_config(&mut config, Some(&metadata));

        assert_eq!(config.processing_mode, ProcessingMode::Reencode);
    }

    #[test]
    fn normalize_output_config_reencodes_gif_outputs() {
        let mut config = ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            container: "gif".to_string(),
            ..ConversionConfig::default()
        };

        normalize_output_config(&mut config, None);

        assert_eq!(config.processing_mode, ProcessingMode::Reencode);
    }

    #[test]
    fn apply_output_container_falls_back_to_default_audio_codec_when_needed() {
        let mut config = ConversionConfig {
            audio_codec: "flac".to_string(),
            ..ConversionConfig::default()
        };

        apply_output_container(&mut config, "webm");

        assert_eq!(config.audio_codec, "libopus");
    }

    #[test]
    fn apply_processing_mode_rejects_copy_for_image_sources() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Image),
            ..SourceMetadata::default()
        };
        let mut config = ConversionConfig::default();

        assert!(!apply_processing_mode(
            &mut config,
            Some(&metadata),
            ProcessingMode::Copy
        ));
    }
}

mod source_info_formatting {
    use super::*;

    #[test]
    fn format_source_duration_formats_colon_time_without_fraction() {
        assert_eq!(format_source_duration(Some("01:02:03.450")), "01:02:03");
    }

    #[test]
    fn format_source_duration_formats_numeric_seconds() {
        assert_eq!(format_source_duration(Some("90.4")), "00:01:30");
    }

    #[test]
    fn format_source_duration_keeps_unparseable_values() {
        assert_eq!(format_source_duration(Some("unknown")), "unknown");
    }

    #[test]
    fn format_source_resolution_prefers_dimensions() {
        let metadata = SourceMetadata {
            resolution: Some("1920x1080".to_string()),
            width: Some(3840),
            height: Some(2160),
            ..SourceMetadata::default()
        };

        assert_eq!(format_source_resolution(&metadata), "3840×2160");
    }

    #[test]
    fn format_source_frame_rate_trims_trailing_zeroes() {
        assert_eq!(format_source_frame_rate(Some(29.970)), "29.97 fps");
    }

    #[test]
    fn format_source_bitrate_kbps_uses_megabits_above_threshold() {
        assert_eq!(format_source_bitrate_kbps(Some(2450.0)), "2.45 Mb/s");
    }

    #[test]
    fn format_source_container_bitrate_parses_bits_per_second() {
        assert_eq!(
            format_source_container_bitrate(Some("1250000")),
            "1.25 Mb/s"
        );
    }

    #[test]
    fn format_source_hz_uses_kilohertz_above_threshold() {
        assert_eq!(format_source_hz(Some("48000")), "48 kHz");
    }
}

mod source_info_sections {
    use super::*;

    fn row_value<'a>(rows: &'a [SourceInfoRow], label: &str) -> Option<&'a str> {
        rows.iter()
            .find(|row| row.label == label)
            .map(|row| row.value.as_str())
    }

    #[test]
    fn source_info_sections_for_images_use_file_information_only() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Image),
            video_codec: Some("png".to_string()),
            width: Some(640),
            height: Some(480),
            pixel_format: Some("rgba".to_string()),
            ..SourceMetadata::default()
        };

        let sections = source_info_sections(&metadata);

        assert_eq!(
            sections,
            vec![SourceInfoSection::Rows {
                title: "FILE INFORMATION",
                rows: vec![
                    SourceInfoRow {
                        label: "IMAGE CODEC",
                        value: "png".to_string(),
                    },
                    SourceInfoRow {
                        label: "DIMENSIONS",
                        value: "640×480".to_string(),
                    },
                    SourceInfoRow {
                        label: "PIXEL FORMAT",
                        value: "rgba".to_string(),
                    },
                ],
            }]
        );
    }

    #[test]
    fn source_info_sections_for_video_include_file_and_video_sections() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Video),
            duration: Some("00:00:10.50".to_string()),
            bitrate: Some("2500000".to_string()),
            video_codec: Some("h264".to_string()),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(59.940),
            video_bitrate_kbps: Some(2200.0),
            ..SourceMetadata::default()
        };

        let sections = source_info_sections(&metadata);

        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn source_info_sections_for_audio_tracks_include_track_rows() {
        let metadata = SourceMetadata {
            media_kind: Some(SourceKind::Audio),
            audio_tracks: vec![AudioTrack {
                index: 3,
                codec: "aac".to_string(),
                channels: Some("stereo".to_string()),
                sample_rate: Some("48000".to_string()),
                bitrate_kbps: Some(192.0),
                language: Some("eng".to_string()),
                ..AudioTrack::default()
            }],
            ..SourceMetadata::default()
        };

        let sections = source_info_sections(&metadata);
        let SourceInfoSection::Tracks { tracks, .. } = &sections[0] else {
            panic!("audio metadata should render audio tracks");
        };

        assert_eq!(row_value(&tracks[0].rows, "SAMPLE RATE"), Some("48 kHz"));
    }
}

mod visible_settings_tabs {
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
                "audio",
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
            vec!["source", "output", "audio", "metadata", "presets"]
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
            vec!["source", "output", "images", "metadata", "presets"]
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
}
