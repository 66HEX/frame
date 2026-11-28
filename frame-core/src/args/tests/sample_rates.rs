use super::*;
use crate::types::AudioTrackOverride;

#[test]
fn automatic_legacy_rates_and_dormant_copy_edits_survive_serialization() {
    let legacy: AudioTrackOverride =
        serde_json::from_value(serde_json::json!({"title":"original"})).unwrap();
    assert_eq!(legacy.sample_rate, None);
    assert!(
        serde_json::to_value(&legacy)
            .unwrap()
            .get("sampleRate")
            .is_none()
    );
    let mut cfg = sample_config("mkv", "libx264");
    cfg.selected_audio_tracks = vec![1, 1];
    let probe = sample_probe();
    let build = |cfg: &ConversionConfig| build_ffmpeg_args("in.mkv", "out.mkv", cfg, &probe);
    let baseline = build(&cfg).unwrap();
    assert!(!baseline.iter().any(|v| v.starts_with("-ar")));
    cfg.audio_track_overrides.insert(
        8,
        AudioTrackOverride {
            sample_rate: Some(1),
            ..AudioTrackOverride::default()
        },
    );
    assert_eq!(build(&cfg).unwrap(), baseline);
    cfg.audio_track_overrides.insert(
        1,
        AudioTrackOverride {
            sample_rate: Some(1),
            ..AudioTrackOverride::default()
        },
    );
    assert!(build(&cfg).unwrap_err().to_string().contains("1 Hz"));
    for mode in ["copy", "copyAudio"] {
        cfg.processing_mode = mode.into();
        let actual = build(&cfg).unwrap();
        assert!(!actual.iter().any(|v| v.starts_with("-ar")));
    }
    cfg.processing_mode = "copyVideo".into();
    cfg.audio_track_overrides.get_mut(&1).unwrap().sample_rate = Some(32_000);
    let resumed: ConversionConfig =
        serde_json::from_value(serde_json::to_value(&cfg).unwrap()).unwrap();
    let actual = build(&resumed).unwrap();
    for ordinal in [0, 1] {
        assert!(args_contains_pair(
            &actual,
            &format!("-ar:a:{ordinal}"),
            "32000"
        ));
    }
}

#[test]
fn explicit_rate_overrides_mp3_and_transport_defaults() {
    let mut probe = sample_probe();
    probe.audio_tracks[0].sample_rate = Some("8000".into());
    for (container, codec, rate) in [
        ("mp4", "mp3", 32_000),
        ("m2t", "mp2", 24_000),
        ("m2ts", "pcm_bluray", 96_000),
    ] {
        let mut cfg = sample_config(container, "libx264");
        cfg.audio_codec = codec.into();
        cfg.selected_audio_tracks = vec![1];
        let auto = build_ffmpeg_args("in.mov", "out", &cfg, &probe).unwrap();
        if codec == "mp3" {
            assert!(args_contains_pair(&auto, "-ar:a:0", "16000"));
        } else {
            assert!(args_contains_pair(&auto, "-ar", "48000"));
        }
        cfg.audio_track_overrides.insert(
            1,
            AudioTrackOverride {
                sample_rate: Some(rate),
                ..AudioTrackOverride::default()
            },
        );
        let manual = build_ffmpeg_args("in.mov", "out", &cfg, &probe).unwrap();
        assert!(args_contains_pair(&manual, "-ar:a:0", &rate.to_string()));
    }
}
