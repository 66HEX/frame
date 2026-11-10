use super::*;
use serde_json::{Value, json};

const HASH: &str = "SHA256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn stream() -> Value {
    json!({
        "index": 3, "codec_name": "alac", "codec_type": "audio",
        "sample_rate": "48000", "channels": 2, "channel_layout": "stereo",
        "sample_fmt": "s16p", "bits_per_raw_sample": "16", "time_base": "1/1000",
        "initial_padding": 0, "extradata_size": 36, "extradata_hash": HASH
    })
}

fn packet(pts: i64) -> Value {
    json!({
        "type": "packet", "stream_index": 3, "codec_type": "audio",
        "pts": pts, "dts": pts, "pos": "100", "size": "50", "flags": "K__",
        "data_hash": HASH
    })
}

fn frame(pts: i64) -> Value {
    json!({
        "type": "frame", "stream_index": 3, "media_type": "audio", "pts": pts,
        "pkt_pos": "100", "nb_samples": 4096, "channels": 2,
        "channel_layout": "stereo", "sample_fmt": "s16p"
    })
}

fn scan(events: &[Value]) -> Value {
    json!({"streams": [stream()], "packets_and_frames": events})
}

fn parse(value: &Value) -> Result<ParsedAlacSource, ObservationError> {
    parse_alac_source_scan(
        &serde_json::to_vec(value).unwrap(),
        3,
        ManifestLimits::default(),
    )
}

#[test]
fn source_scan_preserves_laced_positions_and_does_not_require_source_pos_identity() {
    let source = parse(&scan(&[packet(0), frame(0), packet(85), frame(85)])).unwrap();
    assert_eq!(source.pairs.len(), 2);
    assert_eq!(
        source.pairs[0].packet.position,
        source.pairs[1].packet.position
    );
    assert_eq!(
        source.pairs[0].packet.payload,
        source.pairs[1].packet.payload
    );
    assert_eq!(source.pairs[1].packet.pts, PacketTimestamp::Known(85));
    let mut input = scan(&[packet(0), frame(0)]);
    input["packets_and_frames"][0]
        .as_object_mut()
        .unwrap()
        .remove("pos");
    input["packets_and_frames"][1]["pkt_pos"] = json!("N/A");
    assert_eq!(parse(&input).unwrap().pairs[0].packet.position, None);
    input["streams"][0]
        .as_object_mut()
        .unwrap()
        .remove("initial_padding");
    assert_eq!(parse(&input).unwrap().stream.initial_padding, None);
}

#[test]
fn source_scan_carries_prior_skip_and_respects_an_explicit_zero_reset() {
    let mut first = packet(0);
    first["side_data_list"] = json!([{
        "side_data_type": "Skip Samples", "skip_samples": 6000, "discard_padding": 0
    }]);
    let input = scan(&[
        first,
        frame(0),
        packet(85),
        frame(85),
        packet(171),
        frame(171),
    ]);
    let source = parse(&input).unwrap();
    assert_eq!(
        source
            .pairs
            .iter()
            .map(|pair| pair.effective_skip)
            .collect::<Vec<_>>(),
        vec![4096, 1904, 0]
    );
    assert_eq!(source.pairs[1].packet.trimming, None);

    let mut reset = input;
    reset["packets_and_frames"][2]["side_data_list"] = json!([{
        "side_data_type": "Skip Samples", "skip_samples": 0, "discard_padding": 0
    }]);
    assert_eq!(parse(&reset).unwrap().pairs[1].effective_skip, 0);
}

#[test]
fn source_scan_keeps_parameter_changes_and_unselected_hazards_explicit() {
    let mut changed = packet(85);
    changed["side_data_list"] = json!([{"side_data_type": "New Extradata"}]);
    let mut padded = packet(171);
    padded["side_data_list"] = json!([{
        "side_data_type": "Skip Samples", "skip_samples": 0, "discard_padding": 480
    }]);
    let source = parse(&scan(&[
        packet(0),
        frame(0),
        changed,
        frame(85),
        padded,
        frame(171),
    ]))
    .unwrap();
    assert!(!source.pairs[0].configuration_changed);
    assert!(source.pairs[1].configuration_changed);
    assert!(source.pairs[2].configuration_changed);
    assert_eq!(source.pairs[2].packet.trimming.unwrap().discard, 480);
    // Observing a hazard is not a blanket rejection of every source segment.
    // Only a later planner can decide which pairs the actual copy selects.
}

#[test]
fn source_scan_rejects_incomplete_or_ambiguous_association_and_invalid_facts() {
    let invalid_sequences = [
        vec![packet(0)],
        vec![frame(0)],
        vec![packet(0), packet(85), frame(85)],
        vec![packet(0), frame(0), frame(0)],
        vec![packet(0), frame(1)],
    ];
    for events in invalid_sequences {
        assert!(parse(&scan(&events)).is_err());
    }
    let valid = scan(&[packet(0), frame(0)]);
    for (field, value) in [
        ("nb_samples", json!(0)),
        ("nb_samples", json!(1.5)),
        ("stream_index", json!(4)),
        ("pkt_pos", json!("101")),
    ] {
        let mut input = valid.clone();
        input["packets_and_frames"][1][field] = value;
        assert!(parse(&input).is_err(), "{field}");
    }
    let mut extra = valid.clone();
    extra["streams"].as_array_mut().unwrap().push(stream());
    assert!(parse(&extra).is_err());
    let mut error = valid.clone();
    error["error"] = json!({"code": -1, "string": "decode failed"});
    assert!(parse(&error).is_err());
    let bytes = serde_json::to_vec(&valid).unwrap();
    assert!(
        parse_alac_source_scan(&bytes[..bytes.len() - 1], 3, ManifestLimits::default()).is_err()
    );
    let limits = ManifestLimits {
        packets: 0,
        ..ManifestLimits::default()
    };
    assert!(matches!(
        parse_alac_source_scan(&bytes, 3, limits),
        Err(ObservationError::Limit(_))
    ));
}
