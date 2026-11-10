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
            run_tool(&tools.ffmpeg, &arguments)?;
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
