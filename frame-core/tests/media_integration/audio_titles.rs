use super::*;

fn packet_and_stream_facts(tools: &Toolchain, path: &Path) -> TestResult<serde_json::Value> {
    let bytes = run_tool_output(
        &tools.ffprobe,
        &args(&[
            "-v",
            "error",
            "-show_packets",
            "-show_streams",
            "-show_data_hash",
            "sha256",
            "-of",
            "json",
            &path_arg(path),
        ]),
    )?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn without_audio_title_tags(mut facts: serde_json::Value) -> serde_json::Value {
    for stream in facts["streams"].as_array_mut().unwrap() {
        if stream["codec_type"] == "audio"
            && let Some(tags) = stream
                .get_mut("tags")
                .and_then(serde_json::Value::as_object_mut)
        {
            tags.remove("name");
            tags.remove("title");
        }
    }
    facts
}

fn generate_named_audio_source(tools: &Toolchain, source: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x48:r=12:d=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=660:sample_rate=48000:duration=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=48000:duration=0.5",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "2:a",
            "-map",
            "3:a",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-c:a",
            "aac",
            "-metadata:s:a:0",
            "title=First = Żółć",
            "-metadata:s:a:0",
            "handler_name=Handler one",
            "-metadata:s:a:1",
            "handler_name=HANDLER ONLY",
            "-metadata:s:a:2",
            "title=Third",
            "-metadata:s:a:2",
            "handler_name=Handler three",
            &path_arg(source),
        ]),
    )
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn genuine_audio_names_should_survive_mp4_mov_copy_without_changing_packets() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio_title_preserve")?;
    let source = sandbox.path("source.mp4");
    generate_named_audio_source(&tools, &source)?;
    let probe = probe_media(&tools, &source)?;
    assert_eq!(probe.audio_tracks.len(), 3);
    assert_eq!(
        probe.audio_tracks[0].metadata_title.as_deref(),
        Some("First = Żółć")
    );
    assert!(probe.audio_tracks[1].metadata_title.is_none());
    assert_eq!(
        probe.audio_tracks[2].metadata_title.as_deref(),
        Some("Third")
    );
    let mut old_probe = probe.clone();
    for track in &mut old_probe.audio_tracks {
        track.metadata_title = None;
    }
    for container in ["mp4", "mov"] {
        let mut config = video_config(container, "libx264", "aac");
        config.processing_mode = "copy".into();
        config.metadata.mode = MetadataMode::Preserve;
        config.selected_audio_tracks = vec![3, 2, 1, 3];
        let baseline = sandbox.path(&format!("baseline.{container}"));
        let actual = sandbox.path(&format!("actual.{container}"));
        for (output, metadata) in [(&baseline, &old_probe), (&actual, &probe)] {
            let arguments =
                build_ffmpeg_args(&path_arg(&source), &path_arg(output), &config, metadata)
                    .map_err(|error| error.to_string())?;
            tools.native("convert", &arguments)?;
        }
        let facts = packet_and_stream_facts(&tools, &actual)?;
        let audio = facts["streams"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|stream| stream["codec_type"] == "audio")
            .collect::<Vec<_>>();
        assert_eq!(audio.len(), 4);
        for (stream, expected) in
            audio
                .iter()
                .zip([Some("Third"), None, Some("First = Żółć"), Some("Third")])
        {
            let title = stream["tags"]
                .get("title")
                .or_else(|| stream["tags"].get("name"));
            assert_eq!(
                title.and_then(serde_json::Value::as_str),
                expected,
                "{container}"
            );
        }
        let expected = packet_and_stream_facts(&tools, &baseline)?;
        assert!(
            facts["packets"] == expected["packets"],
            "{container}: ordered packets and timestamps differ from the same-output baseline"
        );
        assert!(
            without_audio_title_tags(facts) == without_audio_title_tags(expected),
            "{container}: non-title stream facts differ from the same-output baseline"
        );
    }
    Ok(())
}

fn create_disposition_source(tools: &Toolchain, mp4: &Path, source: &Path) -> TestResult {
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-i",
            &path_arg(mp4),
            "-map",
            "0",
            "-c",
            "copy",
            "-metadata:s:a:0",
            "language=eng",
            "-metadata:s:a:0",
            "title=Remove this title",
            "-metadata:s:a:1",
            "language=ita",
            "-metadata:s:a:1",
            "title=Old title",
            "-disposition:a:0",
            "default+comment",
            "-disposition:a:1",
            "hearing_impaired",
            &path_arg(source),
        ]),
    )
}

#[test]
#[ignore = "requires the Frame worker and independent reference tools"]
fn edited_audio_metadata_round_trips_without_losing_original_dispositions() -> TestResult {
    use frame_core::types::AudioTrackOverride;
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio-metadata-edits")?;
    let mp4 = sandbox.path("source.mp4");
    generate_named_audio_source(&tools, &mp4)?;
    let source = sandbox.path("source.mkv");
    create_disposition_source(&tools, &mp4, &source)?;

    for processing in ["copy", "reencode"] {
        for mode in [
            MetadataMode::Preserve,
            MetadataMode::Clean,
            MetadataMode::Replace,
        ] {
            let mut config = video_config("mkv", "libx264", "flac");
            config.processing_mode = processing.into();
            config.selected_audio_tracks = vec![1, 2, 2];
            config.metadata.mode.clone_from(&mode);
            if mode == MetadataMode::Replace {
                config.metadata.title = Some("Global title".into());
            }
            config.audio_track_overrides.insert(
                1,
                AudioTrackOverride {
                    title: Some(String::new()),
                    language: Some(String::new()),
                    is_default: None,
                },
            );
            config.audio_track_overrides.insert(
                2,
                AudioTrackOverride {
                    title: Some("Nowy ♥ = mix".into()),
                    language: Some("pl-PL".into()),
                    is_default: Some(true),
                },
            );

            let output = sandbox.path(&format!("{processing}-{mode:?}.mkv"));
            convert(&tools, &source, &output, &config)?;
            let facts = packet_and_stream_facts(&tools, &output)?;
            let audio: Vec<_> = facts["streams"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|s| s["codec_type"] == "audio")
                .collect();
            assert_eq!(audio.len(), 3);
            for (ordinal, stream) in audio.into_iter().enumerate() {
                let expected = mode == MetadataMode::Preserve && ordinal > 0;
                assert_eq!(
                    stream["tags"]["language"].as_str(),
                    expected.then_some("pl-PL")
                );
                assert_eq!(
                    stream["tags"]["title"].as_str(),
                    expected.then_some("Nowy ♥ = mix")
                );
                assert!(
                    stream["tags"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .all(|key| !key.eq_ignore_ascii_case("name"))
                );
                assert_eq!(stream["disposition"]["default"], usize::from(ordinal == 1));
                assert_eq!(stream["disposition"]["comment"], usize::from(ordinal == 0));
                assert_eq!(
                    stream["disposition"]["hearing_impaired"],
                    usize::from(ordinal != 0)
                );
            }
            let native = tools.native("probe", &["metadata".into(), path_arg(&output)])?;
            let probed =
                parse_ffprobe_stdout(&path_arg(&output), std::str::from_utf8(&native).unwrap())
                    .map_err(|e| e.to_string())?;
            assert_eq!(
                probed
                    .audio_tracks
                    .iter()
                    .map(|t| t.is_default)
                    .collect::<Vec<_>>(),
                [false, true, false]
            );
            if processing == "copy" {
                for (ordinal, original) in [0, 1, 1].into_iter().enumerate() {
                    let source_packets = packets(&tools, &source, &format!("a:{original}"))?;
                    assert!(!source_packets.as_array().unwrap().is_empty());
                    assert_eq!(
                        packets(&tools, &output, &format!("a:{ordinal}"))?,
                        source_packets
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires the Frame worker and independent reference tools"]
fn mp4_family_stores_edited_track_names_and_selects_an_eligible_default() -> TestResult {
    use frame_core::types::AudioTrackOverride;
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("mov-audio-edits")?;
    let source = sandbox.path("source.mp4");
    generate_named_audio_source(&tools, &source)?;
    for container in ["mp4", "mov", "m4a"] {
        let mut config = video_config(container, "libx264", "aac");
        config.processing_mode = "copy".into();
        config.selected_audio_tracks = vec![1, 3];
        config.audio_track_overrides.insert(
            1,
            AudioTrackOverride {
                language: Some("deu".into()),
                title: Some(String::new()),
                is_default: Some(false),
            },
        );
        config.audio_track_overrides.insert(
            3,
            AudioTrackOverride {
                language: Some("pol".into()),
                title: Some("Dialog ♥".into()),
                is_default: None,
            },
        );
        let output = sandbox.path(&format!("edited.{container}"));
        convert(&tools, &source, &output, &config)?;
        let result = probe_media(&tools, &output)?;
        assert_eq!(result.audio_tracks.len(), 2);
        assert!(!result.audio_tracks[0].is_default);
        assert!(result.audio_tracks[1].is_default);
        assert_eq!(result.audio_tracks[0].metadata_title, None);
        assert_eq!(
            result.audio_tracks[0].language.as_deref(),
            Some(if container == "mov" { "ger" } else { "deu" })
        );
        assert_eq!(result.audio_tracks[1].language.as_deref(), Some("pol"));
        assert_eq!(
            result.audio_tracks[1].metadata_title.as_deref(),
            (container != "m4a").then_some("Dialog ♥")
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires the Frame worker and independent reference tools"]
fn webm_ogg_transport_and_single_audio_muxers_apply_their_supported_track_fields() -> TestResult {
    use frame_core::types::AudioTrackOverride;
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio-edit-containers")?;
    let source = sandbox.path("source.mp4");
    generate_named_audio_source(&tools, &source)?;
    for (container, video, audio, lang, title, default) in [
        ("webm", "vp9", "libopus", true, true, true),
        ("opus", "libx264", "libopus", true, true, false),
        ("m2t", "libx264", "mp2", true, false, false),
        ("mts", "libx264", "ac3", true, false, false),
        ("m2ts", "libx264", "ac3", true, false, false),
        ("flac", "libx264", "flac", false, false, false),
        ("wav", "libx264", "pcm_s16le", false, false, false),
        ("mp3", "libx264", "mp3", false, false, false),
    ] {
        let mut config = video_config(container, video, audio);
        config.selected_audio_tracks = vec![1];
        config.audio_track_overrides.insert(
            1,
            AudioTrackOverride {
                language: Some("pol".into()),
                title: Some("Commentary".into()),
                is_default: Some(true),
            },
        );
        let output = sandbox.path(&format!("tracks.{container}"));
        convert(&tools, &source, &output, &config)?;
        let result = probe_media(&tools, &output)?;
        assert_eq!(result.audio_tracks.len(), 1, "{container}");
        let track = &result.audio_tracks[0];
        assert_eq!(
            track.language.as_deref(),
            lang.then_some("pol"),
            "{container}"
        );
        assert_eq!(
            track.metadata_title.as_deref(),
            title.then_some("Commentary"),
            "{container}"
        );
        assert_eq!(track.is_default, default, "{container}");
    }
    Ok(())
}
