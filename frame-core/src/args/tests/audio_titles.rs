use super::*;

fn titled_probe() -> ProbeMetadata {
    crate::probe::parse_ffprobe_stdout("source.mp4", serde_json::json!({
        "format":{"tags":{"title":"Global","name":"Not a global title alias"}},
        "streams":[
            {"index":0,"codec_type":"video","codec_name":"h264"},
            {"index":1,"codec_type":"audio","codec_name":"aac","channels":2,"tags":{"name":"First = Żółć","handler_name":"Handler one"}},
            {"index":2,"codec_type":"audio","codec_name":"aac","channels":2,"tags":{"handler_name":"Handler only"}},
            {"index":3,"codec_type":"audio","codec_name":"aac","channels":2,"tags":{"title":"Third","name":"Not preferred","handler_name":"Handler three"}},
            {"index":4,"codec_type":"subtitle","codec_name":"mov_text","tags":{"title":"Captions","language":"pol"}}
        ]
    }).to_string()).unwrap()
}

fn without_titles(probe: &ProbeMetadata) -> ProbeMetadata {
    let mut probe = probe.clone();
    for track in &mut probe.audio_tracks {
        track.metadata_title = None;
    }
    probe
}

fn audio_title_pairs(args: &[String]) -> Vec<(&str, &str)> {
    args.windows(2)
        .filter(|pair| pair[0].starts_with("-metadata:s:a:") && pair[1].starts_with("title="))
        .map(|pair| (pair[0].as_str(), pair[1].as_str()))
        .collect()
}

fn strip_audio_title_pairs(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index].starts_with("-metadata:s:a:")
            && args
                .get(index + 1)
                .is_some_and(|value| value.starts_with("title="))
        {
            index += 2;
        } else {
            result.push(args[index].clone());
            index += 1;
        }
    }
    result
}

#[test]
fn mapped_titles_preserve_duplicates_order_and_all_other_arguments() {
    let probe = titled_probe();
    for processing in ["copy", "reencode"] {
        for container in ["mp4", "mov", "m4a", "mkv"] {
            for mode in [
                MetadataMode::Preserve,
                MetadataMode::Clean,
                MetadataMode::Replace,
            ] {
                let mut config = sample_config(container, "libx264");
                config.processing_mode = processing.into();
                config.metadata.mode = mode;
                config.selected_audio_tracks = vec![3, 1, 3];
                config.selected_subtitle_tracks = vec![4];
                let actual = build_ffmpeg_args("source.mp4", "output", &config, &probe).unwrap();
                let baseline =
                    build_ffmpeg_args("source.mp4", "output", &config, &without_titles(&probe))
                        .unwrap();
                let active = config.metadata.mode == MetadataMode::Preserve
                    && matches!(container, "mp4" | "mov");
                if active {
                    assert_eq!(
                        audio_title_pairs(&actual),
                        [
                            ("-metadata:s:a:0", "title=Third"),
                            ("-metadata:s:a:1", "title=First = Żółć"),
                            ("-metadata:s:a:2", "title=Third"),
                        ]
                    );
                    assert_eq!(strip_audio_title_pairs(&actual), baseline);
                } else {
                    assert!(audio_title_pairs(&actual).is_empty());
                    assert_eq!(actual, baseline);
                }
                let audio_maps = actual
                    .windows(2)
                    .filter(|pair| {
                        pair[0] == "-map" && matches!(pair[1].as_str(), "0:1" | "0:2" | "0:3")
                    })
                    .map(|pair| pair[1].as_str())
                    .collect::<Vec<_>>();
                assert_eq!(audio_maps, ["0:3", "0:1", "0:3"]);
            }
        }
    }
}

#[test]
fn empty_title_clears_its_output_ordinal_without_inventing_handler_titles() {
    let mut probe = titled_probe();
    probe.audio_tracks[2].metadata_title = Some(String::new());
    for processing in ["copy", "reencode"] {
        let mut config = sample_config("mp4", "libx264");
        config.processing_mode = processing.into();
        config.selected_audio_tracks = vec![3, 2, 1];
        config.metadata.mode = MetadataMode::Preserve;
        let actual = build_ffmpeg_args("source.mp4", "output.mp4", &config, &probe).unwrap();
        assert_eq!(
            audio_title_pairs(&actual),
            [
                ("-metadata:s:a:0", "title="),
                ("-metadata:s:a:2", "title=First = Żółć"),
            ]
        );
        assert_eq!(
            strip_audio_title_pairs(&actual),
            build_ffmpeg_args("source.mp4", "output.mp4", &config, &without_titles(&probe))
                .unwrap()
        );
        for selection in [vec![], vec![2]] {
            config.selected_audio_tracks = selection;
            let args = build_ffmpeg_args("source.mp4", "output.mp4", &config, &probe).unwrap();
            assert!(audio_title_pairs(&args).is_empty());
        }
    }
}

#[test]
fn old_label_only_payloads_remain_untitled_and_stale_selection_errors_are_unchanged() {
    let mut probe = titled_probe();
    let old: AudioTrack = serde_json::from_value(serde_json::json!({
        "index":1,"codec":"aac","channels":"2","label":"Old handler-derived label"
    }))
    .unwrap();
    probe.audio_tracks = vec![old];
    for processing in ["copy", "reencode"] {
        let mut config = sample_config("mov", "libx264");
        config.processing_mode = processing.into();
        config.metadata.mode = MetadataMode::Preserve;
        config.selected_audio_tracks = vec![1];
        let args = build_ffmpeg_args("source.mov", "output.mov", &config, &probe).unwrap();
        assert!(audio_title_pairs(&args).is_empty());
        config.selected_audio_tracks = vec![3, 999, 1];
        let titled = titled_probe();
        let actual = build_ffmpeg_args("source.mov", "output.mov", &config, &titled)
            .unwrap_err()
            .to_string();
        let baseline = build_ffmpeg_args(
            "source.mov",
            "output.mov",
            &config,
            &without_titles(&titled),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(actual, baseline);
        assert!(actual.contains("#999"), "{actual}");
    }
}
