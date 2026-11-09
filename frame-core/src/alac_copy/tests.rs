use super::*;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn audio_header(index: u32) -> String {
    format!(
        "#extradata {index}, 36, {HASH}\n#tb {index}: 1/48000\n#media_type {index}: audio\n#codec_id {index}: alac\n#sample_rate {index}: 48000\n#channel_layout_name {index}: stereo\n"
    )
}

fn manifest(headers: &str, packets: &str) -> String {
    format!(
        "#format: frame checksums\n#version: 2\n#hash: SHA256\n#software: Lavf62.12.102\n{headers}#stream#, dts, pts, duration, size, hash\n{packets}"
    )
}

fn parse(text: &str) -> Result<ParsedCopyManifest, ObservationError> {
    parse_copy_manifest(text.as_bytes(), ManifestLimits::default())
}

#[test]
fn copy_manifest_retains_global_indices_duplicate_streams_and_all_side_data() {
    let video = "#tb 0: 1/1000\n#media_type 0: video\n#codec_id 0: h264\n#dimensions 0: 320x180\n#sar 0: 0/1\n";
    let headers = format!("{video}{}{}", audio_header(1), audio_header(2));
    let packet = format!("0, 0, 0, 40, 100, {HASH}\n");
    let packet = format!(
        "{packet}1, -353, -353, 0, 200, {HASH}, S=2, 10, {HASH}, 12, {HASH}\n2, -353, -353, 0, 200, {HASH}\n1, 3743, 3743, 0, 201, {HASH}\n"
    );
    let parsed = parse(&manifest(&headers, &packet)).unwrap();
    assert_eq!(parsed.streams.len(), 3);
    assert_eq!(parsed.streams[0].sample_aspect_ratio, Some((0, 1)));
    assert_eq!(parsed.streams[1].sample_rate, Some(48_000));
    assert_eq!(parsed.streams[2].extradata, parsed.streams[1].extradata);
    assert_eq!(parsed.packets.len(), 4);
    assert_eq!(parsed.packets[1].stream_index, 1);
    assert_eq!(parsed.packets[2].stream_index, 2);
    assert_eq!(parsed.packets[1].payload, parsed.packets[2].payload);
    assert_eq!(parsed.packets[1].side_data.len(), 2);
    assert_eq!(parsed.packets[1].side_data[1].size, 12);
    assert_eq!(parsed.packets[3].pts, PacketTimestamp::Known(3743));
}

#[test]
fn copy_manifest_retains_empty_stream_and_unknown_timestamp_as_observations() {
    let text = manifest(&audio_header(0), "");
    let empty = parse(&text).unwrap();
    assert_eq!(empty.streams.len(), 1);
    assert!(empty.packets.is_empty());
    let text = manifest(
        &audio_header(0),
        &format!("0, {}, {}, -1, 1, {HASH}\n", i64::MIN, i64::MIN),
    );
    let packet = &parse(&text).unwrap().packets[0];
    assert_eq!(packet.pts, PacketTimestamp::Unknown);
    assert_eq!(packet.dts, PacketTimestamp::Unknown);
    assert_eq!(packet.duration, -1);
    // A parsed observation is not a repair authorization: the planner must reject
    // unknown timestamps and negative durations in the affected sequence.
}

#[test]
fn copy_manifest_rejects_partial_side_data_and_protocol_changes() {
    let original = manifest(&audio_header(0), &format!("0, 0, 0, 0, 1, {HASH}\n"));
    let cases = [
        original.trim_end_matches('\n').to_string(),
        original.replace("#version: 2", "#version: 1"),
        original.replace("#hash: SHA256", "#hash: MD5"),
        original.replace("#codec_id 0: alac", "#codec_id 0: alac\n#codec_id 0: alac"),
        original.replace("#tb 0: 1/48000", "#tb 0: 0/48000"),
        original.replace("#tb 0: 1/48000", "#tb 0: 1/48000\n#unknown 0: data"),
        format!("{original}#software: unexpected late header\n"),
        manifest(&audio_header(1), ""),
        manifest(&audio_header(0), &format!("1, 0, 0, 0, 1, {HASH}\n")),
        manifest(&audio_header(0), &format!("0, 0.5, 0, 0, 1, {HASH}\n")),
        manifest(&audio_header(0), &format!("0, 0, 0, 0, 1, {HASH}, S=1\n")),
        manifest(&audio_header(0), &format!("0, 0, 0, 0, 1, {HASH}, S=0\n")),
        manifest(
            &audio_header(0),
            &format!("0, 0, 0, 0, 1, {HASH}, S=1, 10, {HASH}, extra\n"),
        ),
    ];
    for (index, text) in cases.iter().enumerate() {
        assert!(parse(text).is_err(), "malformed case {index}");
    }
    let mut bytes = original.into_bytes();
    bytes[0] = 0xff;
    assert!(parse_copy_manifest(&bytes, ManifestLimits::default()).is_err());
}

#[test]
fn copy_manifest_bounds_bytes_lines_streams_packets_and_side_data() {
    let text = manifest(
        &audio_header(0),
        &format!("0, 0, 0, 0, 1, {HASH}, S=1, 10, {HASH}\n"),
    );
    let limits = [
        ManifestLimits {
            bytes: text.len() - 1,
            ..ManifestLimits::default()
        },
        ManifestLimits {
            line_bytes: 20,
            ..ManifestLimits::default()
        },
        ManifestLimits {
            streams: 0,
            ..ManifestLimits::default()
        },
        ManifestLimits {
            packets: 0,
            ..ManifestLimits::default()
        },
        ManifestLimits {
            side_data_per_packet: 0,
            ..ManifestLimits::default()
        },
    ];
    for limit in limits {
        assert!(matches!(
            parse_copy_manifest(text.as_bytes(), limit),
            Err(ObservationError::Limit(_))
        ));
    }
    assert_eq!(parse(&text), parse(&text.replace('\n', "\r\n")));
}

#[test]
fn exact_rescale_rounds_signed_half_ties_and_rejects_overflow() {
    let half = TimeBase::new(1, 2).unwrap();
    let whole = TimeBase::new(1, 1).unwrap();
    assert_eq!(half.rescale(1, whole), Ok(1));
    assert_eq!(half.rescale(-1, whole), Ok(-1));
    assert_eq!(half.rescale(3, whole), Ok(2));
    assert_eq!(half.rescale(-3, whole), Ok(-2));
    let source = TimeBase::new(1, 44_100).unwrap();
    let destination = TimeBase::new(1, 48_000).unwrap();
    assert_eq!(source.rescale(441, destination), Ok(480));
    assert_eq!(source.rescale(-441, destination), Ok(-480));
    assert_eq!(whole.rescale(i64::MAX, whole), Ok(i64::MAX));
    assert_eq!(
        whole.rescale(i64::MAX, half),
        Err(ObservationError::Overflow)
    );
    assert!(whole.rescale(i64::MIN, whole).is_err());
    assert_eq!(TimeBase::new(2, 96_000), TimeBase::new(1, 48_000));
    for value in [
        "0/1",
        "1/0",
        "-1/1000",
        "1/-1",
        "1/1/1",
        "1/2147483648",
        "1/1.0",
    ] {
        assert!(TimeBase::parse(value).is_err(), "{value}");
    }
}

#[test]
fn packet_flags_and_hashes_cannot_silently_drop_unknown_fields() {
    assert_eq!(
        PacketFlags::parse("K__"),
        Ok(PacketFlags {
            key: true,
            discard: false,
            corrupt: false
        })
    );
    assert_eq!(
        PacketFlags::parse("_DC"),
        Ok(PacketFlags {
            key: false,
            discard: true,
            corrupt: true
        })
    );
    for flags in ["", "K_", "K___", "_K_", "K?_"] {
        assert!(PacketFlags::parse(flags).is_err());
    }
    assert_eq!(
        Sha256Digest::parse_hex(HASH),
        Sha256Digest::parse_hex(&HASH.to_ascii_uppercase())
    );
    for invalid in [
        "0".repeat(63),
        "0".repeat(65),
        "x".repeat(64),
        "é".repeat(32),
    ] {
        assert!(Sha256Digest::parse_hex(&invalid).is_err());
    }
}
