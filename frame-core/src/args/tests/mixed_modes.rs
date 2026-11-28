use super::*;

#[test]
fn input_validation_and_encoder_choices_follow_each_stream_type() {
    let path = temporary_input_file("mixed-mode-validation");
    let input = path.to_str().unwrap();
    let mut probe = sample_probe();
    let mut config = sample_config("mp4", "libx264");
    config.selected_audio_tracks = vec![1];
    config.set_audio_copy(true);
    config.flip_horizontal = true;
    config.audio_codec = "unused".into();
    assert!(validate_task_input(input, &config).is_ok());
    assert_eq!(effective_video_encoder(&config, &probe), Some("libx264"));
    config.audio_volume = 65.0;
    assert!(validate_task_input(input, &config).is_err());
    config.audio_volume = 100.0;
    probe.video_codec = Some("not-copyable".into());
    assert!(validate_stream_copy_compatibility(&config, &probe).is_ok());
    probe.audio_tracks[0].codec = "not-copyable".into();
    let err = validate_stream_copy_compatibility(&config, &probe).unwrap_err();
    assert!(err.to_string().contains("audio track #1"), "{err}");

    config.set_video_copy(true);
    config.set_audio_copy(false);
    config.audio_codec = "aac".into();
    config.video_codec = "unused".into();
    config.audio_volume = 65.0;
    assert!(validate_task_input(input, &config).is_err());
    config.flip_horizontal = false;
    assert!(validate_task_input(input, &config).is_ok());
    assert_eq!(effective_video_encoder(&config, &probe), None);
    let err = validate_stream_copy_compatibility(&config, &probe).unwrap_err();
    assert!(err.to_string().contains("Video codec"), "{err}");
    probe.video_codec = Some("h264".into());
    assert!(validate_stream_copy_compatibility(&config, &probe).is_ok());
    fs::remove_file(path).unwrap();
}

#[test]
fn audio_only_output_uses_only_its_audio_mode_and_keeps_copy_repairs() {
    let mut probe = sample_probe();
    probe.video_codec = None;
    probe.audio_tracks[0].codec = "alac".into();
    let mut config = sample_config("m4a", "libx264");
    config.selected_audio_tracks = vec![1];
    config.set_audio_copy(true);
    let args = build_ffmpeg_args("in", "out.m4a", &config, &probe).unwrap();
    assert!(args_contains_pair(&args, "-c:a", "copy"));
    let observer = build_alac_copy_observer("in", "out.m4a", &config, &probe)
        .unwrap()
        .unwrap();
    assert_eq!(observer.alac_mappings()[0].output_stream_index, 0);
    config.set_audio_copy(false);
    config.set_video_copy(true);
    let args = build_ffmpeg_args("in", "out.m4a", &config, &probe).unwrap();
    assert!(args_contains_pair(&args, "-c:a", "aac"));
    assert!(
        build_alac_copy_observer("in", "out.m4a", &config, &probe)
            .unwrap()
            .is_none()
    );
}

#[test]
fn video_reencode_with_alac_copy_tracks_video_dependencies_and_correct_ordinals() {
    let mut probe = sample_probe();
    probe.video_stream_index = Some(0);
    probe.audio_tracks[0].codec = "alac".into();
    let mut config = sample_config("mp4", "libx264");
    config.set_audio_copy(true);
    config.selected_audio_tracks = vec![1, 1];
    config.subtitle_burn_path = Some("captions.srt".into());
    config.start_time = Some("0.2".into());
    config.overlay = Some(crate::types::OverlayConfig {
        enabled: true,
        path: "logo.png".into(),
        x: 0.0,
        y: 0.0,
        width: 0.2,
        opacity: 1.0,
        anchor: "top-left".into(),
    });
    config
        .external_subtitle_tracks
        .push(crate::types::ExternalSubtitleTrack {
            path: "selectable.srt".into(),
            ..Default::default()
        });
    config.video_two_pass = true;
    config.video_bitrate_mode = "bitrate".into();
    let observer = build_alac_copy_observer("in", "out.mp4", &config, &probe)
        .unwrap()
        .unwrap();
    assert_eq!(
        observer.dependencies(),
        &["in", "logo.png", "captions.srt", "selectable.srt"]
    );
    assert_eq!(
        observer
            .alac_mappings()
            .iter()
            .map(|m| m.output_stream_index)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    let args = observer.observer_args(&[]).unwrap();
    assert!(args_contains_pair(&args, "-map", "0:v:0"));
    assert!(args_contains_pair(&args, "-c:v", "copy"));
    assert!(
        !args
            .iter()
            .any(|arg| matches!(arg.as_str(), "-frame_two_pass" | "-vf" | "-filter_complex"))
    );
    let export = observer.export_args(&[]).unwrap();
    assert!(args_contains_pair(&export, "-c:v", "libx264"));
    assert!(args_contains_pair(&export, "-frame_two_pass", "1"));
    assert!(args_contains_pair(&export, "-map", "[vout]"));
    for plan in [&args, &export] {
        assert!(args_contains_pair(plan, "-map", "2:s:0"));
        assert!(args_contains_pair(plan, "-ss", "0.2"));
    }
    assert!(args_contains_pair(&args, "-c:a", "copy"));
}
