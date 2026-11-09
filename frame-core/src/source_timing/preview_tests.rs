use super::*;
use crate::types::ConversionConfig;

fn source() -> SourceTiming {
    SourceTiming {
        format_start_microseconds: Some(1_300_000),
        format_start_invalid: false,
        first_video_stream_index: Some(0),
        streams: [
            (0, 145_920),
            (1, 126_000),
            (2, 162_000),
            (3, 117_000),
            (4, 189_000),
        ]
        .map(|(index, start)| SourceStreamTiming {
            index,
            time_base: TimeBase::new(1, 90_000),
            start_pts: Some(start),
        })
        .into(),
    }
}

fn config() -> ConversionConfig {
    ConversionConfig {
        container: "mkv".into(),
        subtitle_burn_path: Some("clock.srt".into()),
        ..serde_json::from_value(serde_json::json!({
            "container": "mp4",
            "videoCodec": "libx264",
            "videoBitrateMode": "crf",
            "videoBitrate": "5000",
            "audioCodec": "aac",
            "audioBitrate": "128",
            "audioChannels": "original",
            "selectedAudioTracks": [],
            "selectedSubtitleTracks": [],
            "subtitleBurnPath": null,
            "resolution": "original",
            "customWidth": null,
            "customHeight": null,
            "scalingAlgorithm": "bicubic",
            "fps": "original",
            "crf": 23,
            "preset": "medium",
            "startTime": null,
            "endTime": null
        }))
        .unwrap()
    }
}

fn resolve(config: &ConversionConfig, source: &SourceTiming) -> PreviewBurnClock {
    resolve_preview_burn_clock(
        config,
        Some(source),
        Some("mpegts"),
        &[1, 2],
        &[(3, "hdmv_pgs_subtitle"), (4, "hdmv_pgs_subtitle")],
    )
    .unwrap()
}

#[test]
fn configured_main_input_maps_define_the_untrimmed_preview_baseline() {
    for (audio, subtitles, zero, seek) in [
        (vec![], vec![], None, Some("round(43/(1500*TB))")),
        (
            vec![1],
            vec![],
            Some("round(83/(375*TB))"),
            Some("round(1/(4*TB))"),
        ),
        (vec![2], vec![], None, Some("round(43/(1500*TB))")),
        (
            vec![2, 1],
            vec![],
            Some("round(83/(375*TB))"),
            Some("round(1/(4*TB))"),
        ),
        (
            vec![1, 2],
            vec![],
            Some("round(83/(375*TB))"),
            Some("round(1/(4*TB))"),
        ),
        (
            vec![],
            vec![3],
            Some("round(241/(750*TB))"),
            Some("round(7/(20*TB))"),
        ),
        (vec![], vec![4], None, Some("round(43/(1500*TB))")),
        (
            vec![2],
            vec![4, 3],
            Some("round(241/(750*TB))"),
            Some("round(7/(20*TB))"),
        ),
        (
            vec![2],
            vec![3, 4],
            Some("round(241/(750*TB))"),
            Some("round(7/(20*TB))"),
        ),
    ] {
        let config = ConversionConfig {
            selected_audio_tracks: audio,
            selected_subtitle_tracks: subtitles,
            ..config()
        };
        let clock = resolve(&config, &source());
        assert_eq!(clock.shift_expression(0).unwrap().as_deref(), zero);
        assert_eq!(clock.shift_expression(350_001).unwrap().as_deref(), seek);
    }
}

#[test]
fn stale_or_unsupported_selection_keeps_known_visual_quantization() {
    let mut source = source();
    source.format_start_microseconds = Some(1_400_000);
    for config in [
        ConversionConfig {
            selected_audio_tracks: vec![99],
            ..config()
        },
        ConversionConfig {
            selected_subtitle_tracks: vec![99],
            ..config()
        },
        ConversionConfig {
            container: "mp4".into(),
            selected_subtitle_tracks: vec![3],
            ..config()
        },
    ] {
        let clock = resolve(&config, &source);
        assert_eq!(clock.export_clock(), ResolvedSourceClock::Unknown);
        assert!(matches!(
            clock.visual_clock(),
            ResolvedSourceClock::Known { .. }
        ));
        assert_eq!(clock.shift_expression(123_456).unwrap(), None);
        assert_eq!(
            clock.shift_expression(350_001).unwrap().as_deref(),
            Some("round(193/(1500*TB))")
        );
    }
    let mut unsupported_settings = config();
    unsupported_settings.video_codec = "draft-encoder".into();
    unsupported_settings.video_bitrate = "draft".into();
    unsupported_settings.selected_audio_tracks = vec![1];
    assert_eq!(
        resolve(&unsupported_settings, &source)
            .shift_expression(0)
            .unwrap()
            .as_deref(),
        Some("round(83/(375*TB))")
    );
}

#[test]
fn active_invalid_cross_clock_is_not_swallowed_at_zero_or_by_unknown() {
    let unknown = ResolvedSourceClock::Unknown;
    let invalid = ResolvedSourceClock::Invalid;
    for (visual, export) in [(unknown, invalid), (invalid, unknown), (invalid, invalid)] {
        let clock = PreviewBurnClock { visual, export };
        assert!(clock.shift_expression(0).is_err());
        assert!(clock.shift_expression(-1).is_err());
    }
    assert_eq!(invalid.shift_expression(0).unwrap(), None);
    let mut invalid_source = source();
    invalid_source.streams[0].time_base = TimeBase::new(1, 1);
    invalid_source.streams[0].start_pts = Some(i64::MAX);
    let stale = ConversionConfig {
        selected_audio_tracks: vec![99],
        ..config()
    };
    let observed = resolve(&stale, &invalid_source);
    assert_eq!(observed.visual_clock(), invalid);
    assert_eq!(observed.export_clock(), unknown);
    assert!(observed.shift_expression(0).is_err());
    let source = source();
    let inactive = ConversionConfig {
        subtitle_burn_path: None,
        ..config()
    };
    assert!(
        resolve_preview_burn_clock(&inactive, Some(&source), Some("mpegts"), &[], &[]).is_none()
    );
    for (mode, container) in [("copy", "mkv"), ("reencode", "wav")] {
        let config = ConversionConfig {
            processing_mode: mode.into(),
            container: container.into(),
            ..config()
        };
        assert!(
            resolve_preview_burn_clock(&config, Some(&source), Some("mpegts"), &[], &[]).is_none()
        );
    }
}

#[test]
fn incompatible_known_context_falls_back_to_the_visual_clock_without_replacing_its_minimum() {
    let visual = ResolvedSourceClock::Known {
        video_time_base: TimeBase::new(1, 90_000).unwrap(),
        origin_microseconds: 1_400_000,
        minimum_start_microseconds: Some(1_621_333),
    };
    let export = ResolvedSourceClock::Known {
        video_time_base: TimeBase::new(1, 1000).unwrap(),
        origin_microseconds: 1_400_000,
        minimum_start_microseconds: None,
    };
    // Production creates the private pair from one source. This defensive
    // branch must retain the visual clock if that invariant ever changes.
    let clock = PreviewBurnClock { visual, export };
    assert_eq!(clock.shift_expression(123_456).unwrap(), None);
    assert_eq!(
        clock.shift_expression(350_001).unwrap(),
        visual.shift_expression(350_001).unwrap()
    );
}

#[test]
fn cross_clock_reuses_signed_rescaling_and_precision_guards() {
    for (origin, seek, expected) in [
        (-501_000, 123_500, "round(123/(1000*TB))"),
        (0, -123_500, "round(-31/(250*TB))"),
        (9_000_000_000_000_000_000, 1, "round(1/(1000000*TB))"),
    ] {
        let base = if origin > 0 {
            TimeBase::new(1, 1_000_000)
        } else {
            TimeBase::new(1, 1000)
        }
        .unwrap();
        let known = ResolvedSourceClock::Known {
            video_time_base: base,
            origin_microseconds: origin,
            minimum_start_microseconds: None,
        };
        let clock = PreviewBurnClock {
            visual: known,
            export: known,
        };
        assert_eq!(
            clock.shift_expression(seek).unwrap().as_deref(),
            Some(expected)
        );
    }
    let known = ResolvedSourceClock::Known {
        video_time_base: TimeBase::new(1, 1_000_000).unwrap(),
        origin_microseconds: 0,
        minimum_start_microseconds: None,
    };
    let clock = PreviewBurnClock {
        visual: known,
        export: known,
    };
    assert!(clock.shift_expression(1_i64 << 52).is_ok());
    assert!(clock.shift_expression((1_i64 << 52) + 1).is_err());
    assert!(clock.shift_expression_for_filter(1_i64 << 51, true).is_ok());
    assert!(
        clock
            .shift_expression_for_filter((1_i64 << 51) + 1, true)
            .is_err()
    );
    let overflow = ResolvedSourceClock::Known {
        video_time_base: TimeBase::new(1, 1_000_000).unwrap(),
        origin_microseconds: i64::MAX,
        minimum_start_microseconds: None,
    };
    assert!(
        PreviewBurnClock {
            visual: overflow,
            export: overflow
        }
        .shift_expression(1)
        .is_err()
    );
}
