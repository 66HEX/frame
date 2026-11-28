use super::*;
use frame_core::types::AudioTrackOverride;

fn requested_rate(rate: u32) -> AudioTrackOverride {
    AudioTrackOverride {
        sample_rate: Some(rate),
        ..AudioTrackOverride::default()
    }
}

fn audio_pcm(tools: &Toolchain, file: &Path, ordinal: usize) -> TestResult<Vec<u8>> {
    run_tool_output(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-i",
            &path_arg(file),
            "-map",
            &format!("0:a:{ordinal}"),
            "-f",
            "s16le",
            "pipe:1",
        ]),
    )
}

fn tones(tools: &Toolchain, file: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=s=96x64:r=24:d=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100:duration=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=750:sample_rate=48000:duration=1",
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
            &path_arg(file),
        ]),
    )
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn mapped_rates_preserve_tone_order_filter_samples_and_trimmed_duration() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio_rates")?;
    let src = sandbox.path("tones.mov");
    tones(&tools, &src)?;
    let source = probe_media(&tools, &src)?;
    for (codec, container) in [("pcm_s16le", "mov"), ("flac", "mkv"), ("aac", "mp4")] {
        let mut cfg = video_config(container, "libx264", codec);
        cfg.set_video_copy(true);
        cfg.selected_audio_tracks = vec![2, 1, 2];
        cfg.audio_track_overrides.insert(1, requested_rate(24_000));
        cfg.audio_track_overrides.insert(2, requested_rate(32_000));
        cfg.start_time = Some("0.125".into());
        cfg.end_time = Some("0.625".into());
        cfg.audio_volume = 200.;
        let actual = sandbox.path(&format!("worker-{codec}.{container}"));
        let reference = sandbox.path(&format!("oracle-{codec}.{container}"));
        convert(&tools, &src, &actual, &cfg)?;
        let args = build_ffmpeg_args(&path_arg(&src), &path_arg(&reference), &cfg, &source)
            .map_err(|e| e.to_string())?;
        run_tool(&tools.ffmpeg, &args)?;
        let streams = probe_media(&tools, &actual)?.audio_tracks;
        assert_eq!(streams.len(), 3);
        for (ordinal, hz) in [32000usize, 24_000, 32_000].into_iter().enumerate() {
            assert_eq!(
                streams[ordinal].sample_rate.as_ref().unwrap(),
                &hz.to_string()
            );
            let actual_samples = audio_pcm(&tools, &actual, ordinal)?;
            let expected_samples = audio_pcm(&tools, &reference, ordinal)?;
            assert_eq!(
                actual_samples.len(),
                expected_samples.len(),
                "{codec} stream {ordinal} decoded sample count"
            );
            let count = actual_samples.len() / 2;
            if codec == "aac" {
                assert!(
                    (hz / 2..hz / 2 + 1024).contains(&count),
                    "{codec}: {count} samples"
                );
            } else {
                assert_eq!(count, hz / 2, "{codec}: exact trimmed sample count");
                assert_bytes_match(
                    &actual_samples,
                    &expected_samples,
                    &format!("{codec} track {ordinal} filtered PCM"),
                )?;
            }
            // Each muxed packet must move forward on its negotiated track clock.
            let actual_packets = packets(&tools, &actual, &format!("a:{ordinal}"))?;
            let actual_packets = actual_packets.as_array().unwrap();
            assert!(actual_packets.windows(2).all(|pair|pair[0]["pts"].as_i64().unwrap()<pair[1]["pts"].as_i64().unwrap()));
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn custom_hz_and_automatic_copy_should_survive_native_export() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio_custom_rate")?;
    let src = sandbox.path("tones.mov");
    tones(&tools, &src)?;
    for (codec, container) in [
        ("pcm_s16le", "wav"),
        ("flac", "flac"),
        ("alac", "m4a"),
        ("libopus", "opus"),
    ] {
        let mut cfg = audio_config(container, codec);
        cfg.selected_audio_tracks = vec![1];
        let rate = if codec == "libopus" { 48_000 } else { 12_345 };
        cfg.audio_track_overrides.insert(1, requested_rate(rate));
        let output = sandbox.path(&format!("custom.{container}"));
        convert(&tools, &src, &output, &cfg)?;
        assert_eq!(
            probe_media(&tools, &output)?.audio_tracks[0].sample_rate,
            Some(rate.to_string())
        );
        assert_eq!(
            audio_pcm(&tools, &output, 0)?.len() / 2,
            rate as usize,
            "{codec} one-second sample count"
        );
    }
    let mut cfg = video_config("mov", "libx264", "pcm_s16le");
    cfg.processing_mode = "copy".into();
    cfg.selected_audio_tracks = vec![2, 1, 2];
    cfg.audio_track_overrides
        .insert(1, requested_rate(u32::MAX));
    cfg.audio_track_overrides.insert(2, requested_rate(12_345));
    let out = sandbox.path("copy.mov");
    convert(&tools, &src, &out, &cfg)?;
    let tracks = probe_media(&tools, &out)?.audio_tracks;
    for (ordinal, source_index) in [1, 0, 1].into_iter().enumerate() {
        let expected = audio_pcm(&tools, &src, source_index)?;
        assert_bytes_match(
            &audio_pcm(&tools, &out, ordinal)?,
            &expected,
            "copied audio samples",
        )?;
        assert_eq!(
            tracks[ordinal].sample_rate.as_deref(),
            Some(if source_index == 0 { "44100" } else { "48000" })
        );
    }
    Ok(())
}
