use super::*;

#[test]
#[ignore = "requires release codecs, the Frame worker and independent reference tools"]
fn mixed_stream_modes_preserve_packets_and_apply_only_the_selected_encoders() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("mixed-stream-modes")?;
    let source = sandbox.path("input.mkv");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=96x64:r=12:d=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=f=440:r=48000:d=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=f=660:r=48000:d=0.5",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "2:a",
            "-c:v",
            "libx264",
            "-bf",
            "0",
            "-c:a",
            "pcm_s16le",
            &path_arg(&source),
        ]),
    )?;
    let mut config = video_config("mkv", "libx264", "flac");
    config.selected_audio_tracks = vec![2, 1, 2];

    config.set_audio_copy(true);
    config.resolution = "custom".into();
    config.custom_width = Some("64".into());
    config.custom_height = Some("48".into());
    let audio_copy = sandbox.path("audio-copy.mkv");
    convert(&tools, &source, &audio_copy, &config)?;
    let result = probe_media(&tools, &audio_copy)?;
    assert_eq!((result.width, result.height), (Some(64), Some(48)));
    assert_eq!(result.audio_tracks.len(), 3);
    for (ordinal, original) in [1, 0, 1].into_iter().enumerate() {
        let expected = packets(&tools, &source, &format!("a:{original}"))?;
        assert!(expected.as_array().unwrap().len() > 1);
        assert_eq!(
            packets(&tools, &audio_copy, &format!("a:{ordinal}"))?,
            expected
        );
        assert_eq!(result.audio_tracks[ordinal].codec, "pcm_s16le");
    }

    config.set_video_copy(true);
    config.set_audio_copy(false);
    config.resolution = "original".into();
    config.audio_volume = 200.0;
    let video_copy = sandbox.path("video-copy.mkv");
    convert(&tools, &source, &video_copy, &config)?;
    let expected_video = packets(&tools, &source, "v:0")?;
    assert_eq!(expected_video.as_array().unwrap().len(), 6);
    assert_eq!(packets(&tools, &video_copy, "v:0")?, expected_video);
    let result = probe_media(&tools, &video_copy)?;
    assert_eq!((result.width, result.height), (Some(96), Some(64)));
    assert_eq!(result.audio_tracks.len(), 3);
    for (ordinal, original) in [1, 0, 1].into_iter().enumerate() {
        assert_eq!(result.audio_tracks[ordinal].codec, "flac");
        let pcm = |path: &Path, stream: usize, filter: &str| {
            run_tool_output(
                &tools.ffmpeg,
                &args(&[
                    "-v",
                    "error",
                    "-i",
                    &path_arg(path),
                    "-map",
                    &format!("0:a:{stream}"),
                    "-af",
                    filter,
                    "-f",
                    "s16le",
                    "pipe:1",
                ]),
            )
        };
        let expected = pcm(&source, original, "volume=2")?;
        assert_eq!(expected.len(), 48_000);
        assert_bytes_match(
            &pcm(&video_copy, ordinal, "anull")?,
            &expected,
            "filtered audio",
        )?;
    }
    Ok(())
}

#[test]
#[ignore = "requires release codecs, the Frame worker and independent reference tools"]
fn alac_copy_observer_accepts_real_reencoded_video_and_duplicate_audio_maps() -> TestResult {
    use frame_core::{
        alac_copy::{ManifestLimits, parse_copy_manifest},
        args::build_alac_copy_observer,
    };
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("video-encode-alac-copy")?;
    let source = sandbox.path("source.mov");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=96x64:r=12:d=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=f=880:r=48000:d=0.5",
            "-c:v",
            "libx264",
            "-bf",
            "0",
            "-c:a",
            "alac",
            &path_arg(&source),
        ]),
    )?;
    let output = sandbox.path("result.mov");
    let mut config = video_config("mov", "libx264", "aac");
    config.set_audio_copy(true);
    config.flip_vertical = true;
    config.selected_audio_tracks = vec![1, 1];
    config.audio_track_overrides.insert(
        1,
        frame_core::types::AudioTrackOverride {
            sample_rate: None,
            language: Some("pol".into()),
            title: Some("Source mix".into()),
            is_default: Some(true),
        },
    );
    let observer = build_alac_copy_observer(
        &path_arg(&source),
        &path_arg(&output),
        &config,
        &probe_media(&tools, &source)?,
    )
    .map_err(|e| e.to_string())?
    .ok_or("missing ALAC observer")?;
    let observation = tools.native(
        "convert",
        &observer.observer_args(&[]).map_err(|e| e.to_string())?,
    )?;
    let manifest =
        parse_copy_manifest(&observation, ManifestLimits::default()).map_err(|e| e.to_string())?;
    observer
        .validate_manifest(&manifest)
        .map_err(|e| e.to_string())?;
    tools.native(
        "convert",
        &observer.export_args(&[]).map_err(|e| e.to_string())?,
    )?;
    let expected = packets(&tools, &source, "a:0")?;
    assert!(!expected.as_array().unwrap().is_empty());
    assert_eq!(packets(&tools, &output, "a:0")?, expected);
    assert_eq!(packets(&tools, &output, "a:1")?, expected);
    let probe = probe_media(&tools, &output)?;
    for (ordinal, track) in probe.audio_tracks.iter().enumerate() {
        assert_eq!(track.language.as_deref(), Some("pol"));
        assert_eq!(track.metadata_title.as_deref(), Some("Source mix"));
        assert_eq!(track.is_default, ordinal == 0);
    }
    Ok(())
}
