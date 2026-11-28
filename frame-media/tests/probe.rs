//! Exercise Frame's existing metadata and proof readers against bytes produced
//! by the shipped worker. Fixtures include real edit lists and display matrices.
use frame_core::alac_copy::{
    AlacOutputEvent, ManifestLimits, PacketTimestamp, Sha256Digest, parse_alac_source_scan,
    parse_normal_alac_scan, parse_physical_audio_scan,
};
use frame_core::probe::parse_ffprobe_stdout;
use frame_core::types::ProbeMetadata;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn probe(args: &[&str], path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_frame-media"))
        .arg("probe")
        .args(args)
        .arg(path)
        .output()
        .unwrap()
}

fn observed(args: &[&str], path: &Path) -> Vec<u8> {
    let output = probe(args, path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn metadata(path: &Path) -> ProbeMetadata {
    parse_ffprobe_stdout(
        &path.to_string_lossy(),
        String::from_utf8(observed(&["metadata"], path)).unwrap(),
    )
    .unwrap()
}

#[test]
fn metadata_preserves_display_rotation_audio_and_subtitle_mapping() {
    let info = metadata(&fixture("rotated.mp4"));
    assert_eq!(info.video_stream_index, Some(0));
    assert_eq!(info.pixel_format.as_deref(), Some("yuv420p"));
    assert_eq!(info.profile.as_deref(), Some("Constrained Baseline"));
    assert_eq!((info.width, info.height), (Some(32), Some(48)));
    assert_eq!(info.frame_rate, Some(12.0));
    let [audio] = info.audio_tracks.as_slice() else {
        panic!("expected one audio track")
    };
    assert_eq!(
        (audio.index, audio.codec.as_str(), audio.codec_tag),
        (1, "aac", Some(0x6134_706d))
    );
    let [sub] = info.subtitle_tracks.as_slice() else {
        panic!("expected one subtitle track")
    };
    assert_eq!(
        (
            sub.index,
            sub.codec.as_str(),
            sub.language.as_deref(),
            sub.label.as_deref()
        ),
        (2, "mov_text", Some("pol"), Some("Captions"))
    );
}

#[test]
fn transport_stream_retains_program_packet_size_clock_and_sar() {
    let info = metadata(&fixture("service.ts"));
    let ts = info.transport_stream.unwrap();
    assert_eq!((ts.program_id, ts.packet_size), (Some(1), Some(188)));
    assert_eq!(ts.service_name.as_deref(), Some("Frame service"));
    assert_eq!(ts.service_provider.as_deref(), Some("Frame provider"));
    let timing = info.source_timing.unwrap();
    assert_eq!(timing.format_start_microseconds, Some(1_525_000));
    assert_eq!(timing.streams[0].start_pts, Some(137_250));
    assert_eq!(
        info.sample_aspect_ratio,
        frame_core::aspect::SampleAspectRatio::new(4, 3)
    );
}

#[test]
fn metadata_decodes_quotes_and_unicode_from_format_tags() {
    let info = metadata(&fixture("tone-alac.m4a"));
    assert_eq!(info.media_kind, "audio");
    assert_eq!(info.tags.unwrap().title.as_deref(), Some("Próba \"audio\""));
    assert_eq!(info.audio_tracks[0].sample_rate.as_deref(), Some("48000"));
    assert_eq!(info.audio_tracks[0].language.as_deref(), Some("pol"));
}

#[test]
fn alac_packet_frame_scans_distinguish_edit_list_and_decoder_trimming() {
    let path = fixture("tone-alac.m4a");
    let limits = ManifestLimits::default();
    let source =
        parse_alac_source_scan(&observed(&["alac-source", "0"], &path), 0, limits).unwrap();
    let physical =
        parse_physical_audio_scan(&observed(&["physical-audio"], &path), limits).unwrap();
    let normal =
        parse_normal_alac_scan(&observed(&["alac-output", "0"], &path), 0, limits).unwrap();
    assert_eq!(source.stream.sample_format, "s16p");
    assert_eq!(source.stream.bits_per_raw_sample, 16);
    assert_eq!(
        source.stream.extradata.digest,
        Sha256Digest::parse_hex("50da6ff305e47725d70832f9b5abc1faf5ca86aa5ccc3aed51fc85a21aff2305")
            .unwrap()
    );
    assert_eq!(source.pairs.len(), 2);
    let first = &source.pairs[0];
    assert_eq!(first.packet.pts, PacketTimestamp::Known(-1920));
    assert_eq!(first.effective_skip, 1920);
    assert_eq!(
        first.packet.payload.digest,
        Sha256Digest::parse_hex("b859ecfa31a8e42644c8f2e36ecb453f0a80b8bc98e2ddc9374ce79086b56954")
            .unwrap()
    );
    assert_eq!(first.frame.position, Some(44));
    assert_eq!(first.frame.physical_samples, 4096);
    assert_eq!(physical.tracks[0].packets[0].pts, PacketTimestamp::Known(0));
    assert_eq!(physical.tracks[0].packets[0].payload, first.packet.payload);
    assert_eq!(normal.events.len(), 4);
    let AlacOutputEvent::Frame(frame) = &normal.events[1] else {
        panic!("expected decoded frame")
    };
    assert_eq!(frame.physical_samples, 2176);
    assert_eq!(frame.pts, PacketTimestamp::Known(0));
}

#[test]
fn opus_timing_returns_actual_preroll_and_applies_exclusive_end() {
    let scan: serde_json::Value = serde_json::from_slice(&observed(
        &["opus-timing", "0", "200000", "250000"],
        &fixture("silence.opus"),
    ))
    .unwrap();
    assert_eq!(scan["streams"][0]["codec_name"], "opus");
    assert_eq!(scan["streams"][0]["time_base"], "1/48000");
    assert_eq!(scan["format"]["start_time"], "0.000000");
    let packets = scan["packets"].as_array().unwrap();
    assert_eq!(packets[0]["pts"], -312);
    assert_eq!(packets.last().unwrap()["pts"], 11208);
    assert!(packets.iter().all(|packet| packet["duration"] == 960));
}

#[test]
fn invalid_stream_clock_input_and_mode_return_failure_status() {
    let path = fixture("tone-alac.m4a");
    for args in [
        &["alac-output", "9"][..],
        &["opus-timing", "0", "", "broken"],
        &["unsupported"],
    ] {
        assert!(!probe(args, &path).status.success());
    }
    assert!(!probe(&["metadata"], &fixture("missing")).status.success());
}

#[test]
fn metadata_opens_unicode_paths_with_spaces() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("źródło 01.m4a");
    std::fs::copy(fixture("tone-alac.m4a"), &path).unwrap();
    assert_eq!(metadata(&path).audio_codec.as_deref(), Some("alac"));
}

// Linux filesystems allow non-UTF-8 names. APFS rejects their creation.
#[cfg(target_os = "linux")]
#[test]
fn metadata_opens_raw_unix_paths() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(OsStr::from_bytes(b"track-\xff.m4a"));
    std::fs::copy(fixture("tone-alac.m4a"), &path).unwrap();
    assert_eq!(metadata(&path).audio_codec.as_deref(), Some("alac"));
}
