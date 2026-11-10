use super::*;

fn decoded_audio(tools: &Toolchain, path: &Path, audio_ordinal: usize) -> TestResult<Vec<u8>> {
    run_tool_output(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-xerror",
            "-i",
            &path_arg(path),
            "-map",
            &format!("0:a:{audio_ordinal}"),
            "-c:a",
            "pcm_s32le",
            "-f",
            "s32le",
            "pipe:1",
        ]),
    )
}

fn av_source(
    tools: &Toolchain,
    path: &Path,
    codec: &str,
    channels: u16,
    muxer: &str,
) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x90:rate=10:duration=0.3",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100:duration=0.3",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "1:a",
            "-c:v",
            "libx264",
            "-c:a:0",
            codec,
            "-ac:a:0",
            &channels.to_string(),
            "-c:a:1",
            "pcm_s16le",
            "-f",
            muxer,
            &path_arg(path),
        ]),
    )
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn mov_adpcm_copy_guard_should_preserve_other_selected_tracks_and_reencode() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("mov_adpcm_copy_source")?;
    for channels in [1, 2] {
        let input = sandbox.path(&format!("source-{channels}.mov"));
        av_source(&tools, &input, "adpcm_swf", channels, "mov")?;
        let metadata = probe_media(&tools, &input)?;
        assert!(
            metadata
                .source_format
                .as_deref()
                .unwrap()
                .split(',')
                .any(|name| name == "mov")
        );
        assert_eq!(metadata.audio_tracks[0].codec, "adpcm_swf");
        assert_eq!(metadata.audio_tracks[0].codec_tag, Some(0x4653_736d));
        for selected_ordinal in [0, 1] {
            let source_track = &metadata.audio_tracks[selected_ordinal];
            let source_audio = decoded_audio(&tools, &input, selected_ordinal)?;
            assert!(!source_audio.is_empty());
            let output = sandbox.path(&format!("copy-{channels}-{selected_ordinal}.mkv"));
            let mut config = video_config("mkv", "libx264", "flac");
            config.processing_mode = "copy".into();
            config.selected_audio_tracks = vec![source_track.index];
            if channels == 1 && selected_ordinal == 0 {
                let error = convert(&tools, &input, &output, &config).unwrap_err();
                assert!(error.contains("Choose Re-encode"), "{error}");
                assert!(!output.exists());
                config.processing_mode = "reencode".into();
            }
            convert(&tools, &input, &output, &config)?;
            let output_metadata = probe_media(&tools, &output)?;
            assert_eq!(output_metadata.audio_tracks.len(), 1);
            assert_eq!(
                output_metadata.audio_tracks[0].channels,
                source_track.channels
            );
            assert_bytes_match(
                &decoded_audio(&tools, &output, 0)?,
                &source_audio,
                "selected audio samples",
            )?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ms_adpcm_copy_guard_should_use_actual_nut_format_and_keep_native_mov_working() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("ms_adpcm_copy_source")?;
    for muxer in ["nut", "mov"] {
        // Both extensions are accepted source paths. Only the probe distinguishes them.
        let input = sandbox.path(&format!("source-{muxer}.mov"));
        av_source(&tools, &input, "adpcm_ms", 2, muxer)?;
        let metadata = probe_media(&tools, &input)?;
        assert_eq!(metadata.audio_tracks[0].codec, "adpcm_ms");
        let source_audio = decoded_audio(&tools, &input, 0)?;
        assert!(!source_audio.is_empty());
        let output = sandbox.path(&format!("copy-{muxer}.mkv"));
        let mut config = video_config("mkv", "libx264", "flac");
        config.processing_mode = "copy".into();
        config.selected_audio_tracks = vec![metadata.audio_tracks[0].index];
        if muxer == "nut" {
            assert_eq!(metadata.source_format.as_deref(), Some("nut"));
            let error = convert(&tools, &input, &output, &config).unwrap_err();
            assert!(error.contains("Choose Re-encode"), "{error}");
            assert!(!output.exists());
            config.processing_mode = "reencode".into();
        }
        convert(&tools, &input, &output, &config)?;
        assert_bytes_match(
            &decoded_audio(&tools, &output, 0)?,
            &source_audio,
            "MS ADPCM decoded samples",
        )?;
    }
    Ok(())
}
