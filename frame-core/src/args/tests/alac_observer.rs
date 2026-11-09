use super::*;
use crate::alac_copy::{AlacOutputMap, ManifestStream, ParsedCopyManifest, TimeBase};

fn copy_fixture(container: &str, video: bool) -> (ConversionConfig, ProbeMetadata) {
    let mut config = sample_config(container, "libx264");
    config.processing_mode = "copy".into();
    config.selected_audio_tracks = vec![3, 1, 3];
    config.start_time = Some("0.123456789".into());
    config.end_time = Some("1.23456789".into());
    config.metadata.mode = MetadataMode::Replace;
    config.metadata.title = Some("-n".into());
    let mut probe = sample_probe();
    probe.video_stream_index = video.then_some(0);
    if !video {
        probe.video_codec = None;
    }
    probe.audio_tracks.push(AudioTrack {
        index: 3,
        codec: "alac".into(),
        channels: "2".into(),
        ..AudioTrack::default()
    });
    (config, probe)
}

#[test]
fn alac_observer_changes_only_the_known_sink_and_preserves_duplicate_actual_maps() {
    for (container, video, first) in [
        ("mov", true, 1),
        ("mp4", true, 1),
        ("m4a", false, 0),
        ("m4a", true, 0),
    ] {
        let (config, probe) = copy_fixture(container, video);
        let ordinary = build_ffmpeg_args("-n", "-output.mov", &config, &probe).unwrap();
        let observer = build_alac_copy_observer("-n", "-output.mov", &config, &probe)
            .unwrap()
            .unwrap();
        assert_eq!(observer.export_args(&[]).unwrap(), ordinary);
        assert_eq!(observer.dependencies(), &["-n"]);
        let args = observer.observer_args(&[]).unwrap();
        assert_eq!(&args[5..args.len() - 7], &ordinary[..ordinary.len() - 2]);
        assert_eq!(
            &args[args.len() - 7..],
            [
                "-hash",
                "sha256",
                "-format_version",
                "2",
                "-f",
                "framehash",
                "pipe:1"
            ]
            .map(str::to_owned)
        );
        let mappings = observer
            .audio_outputs()
            .iter()
            .map(|output| output.mapping)
            .collect::<Vec<_>>();
        assert_eq!(
            mappings,
            [3, 1, 3]
                .into_iter()
                .enumerate()
                .map(|(ordinal, source_stream_index)| AlacOutputMap {
                    source_stream_index,
                    output_stream_index: first + u32::try_from(ordinal).unwrap(),
                    audio_output_ordinal: u32::try_from(ordinal).unwrap(),
                })
                .collect::<Vec<_>>()
        );
        assert_eq!(observer.alac_mappings(), [mappings[0], mappings[2]]);
    }
}

#[test]
fn alac_observer_retains_ordinary_error_precedence_and_leaves_other_modes_outside_scope() {
    let (mut config, probe) = copy_fixture("mov", true);
    config.selected_audio_tracks = vec![3, 99];
    let ordinary_error = build_ffmpeg_args("in", "out", &config, &probe)
        .unwrap_err()
        .to_string();
    assert_eq!(
        build_alac_copy_observer("in", "out", &config, &probe)
            .unwrap_err()
            .to_string(),
        ordinary_error
    );
    config.selected_audio_tracks = vec![1];
    assert!(
        build_alac_copy_observer("in", "out", &config, &probe)
            .unwrap()
            .is_none()
    );
    config.selected_audio_tracks = vec![3];
    config.processing_mode = "reencode".into();
    assert!(
        build_alac_copy_observer("in", "out", &config, &probe)
            .unwrap()
            .is_none()
    );
    config.processing_mode = "copy".into();
    config.container = "mkv".into();
    assert!(
        build_alac_copy_observer("in", "out", &config, &probe)
            .unwrap()
            .is_none()
    );
}

#[test]
fn alac_observer_does_not_accept_omitted_or_reordered_manifest_audio_headers() {
    let (config, probe) = copy_fixture("m4a", true);
    let observer = build_alac_copy_observer("in", "out.m4a", &config, &probe)
        .unwrap()
        .unwrap();
    let streams = ["alac", "aac", "alac"]
        .into_iter()
        .enumerate()
        .map(|(index, codec)| ManifestStream {
            index: u32::try_from(index).unwrap(),
            time_base: TimeBase::new(1, 48_000).unwrap(),
            media_type: "audio".into(),
            codec: codec.into(),
            extradata: None,
            sample_rate: Some(48_000),
            channel_layout: Some("stereo".into()),
            dimensions: None,
            sample_aspect_ratio: None,
        })
        .collect();
    let mut manifest = ParsedCopyManifest {
        streams,
        packets: Vec::new(),
    };
    observer.validate_manifest(&manifest).unwrap();
    manifest.streams.swap(0, 1);
    assert!(observer.validate_manifest(&manifest).is_err());
    manifest.streams.swap(0, 1);
    manifest.streams.pop();
    assert!(observer.validate_manifest(&manifest).is_err());
}
