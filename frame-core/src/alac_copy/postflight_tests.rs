use super::planner_tests::{manifest, mapping, payload, source};
use super::*;
use serde_json::{Value, json};

fn prepared_fixture() -> (ParsedCopyManifest, Vec<PreparedAlacTrack>) {
    let tokens = [1, 2, 3, 4, 5, 6];
    let mut original = manifest(&tokens);
    let mut observed = source(&tokens, &[4, 4, 4, 4, 4, 2]);
    observed.stream.sample_rate = 10;
    original.streams[0].time_base = TimeBase::new(1, 10).unwrap();
    original.streams[0].sample_rate = Some(10);
    for (ordinal, packet) in original.packets.iter_mut().enumerate() {
        let pts = -19 + i64::try_from(ordinal).unwrap() * 4;
        packet.pts = PacketTimestamp::Known(pts);
        packet.dts = packet.pts;
        packet.duration = if ordinal == 5 { 0 } else { 4 };
    }
    let candidates = alac_tail_candidates(&original, &[mapping()]).unwrap();
    let plan = plan_alac_repair(&original, candidates[0], &observed).unwrap();
    let mut candidate = original.clone();
    candidate.packets[5].duration = 2;
    let prepared = prepare_alac_tracks(&original, &[plan], &candidate).unwrap();
    assert_eq!(prepared[0].normal_first_packet(), 2);
    assert_eq!(prepared[0].normal_initial_skip(), 11);
    (candidate, prepared)
}

fn packet(expected: &PreparedAlacPacket, ordinal: usize) -> SourceAudioPacket {
    SourceAudioPacket {
        pts: PacketTimestamp::Known(expected.pts),
        dts: PacketTimestamp::Known(expected.dts),
        duration: Some(expected.duration),
        position: Some(1000 + u64::try_from(ordinal).unwrap() * 100),
        payload: expected.payload,
        flags: PacketFlags::parse("K__").unwrap(),
        trimming: None,
        other_side_data: Vec::new(),
    }
}

fn physical(prepared: &PreparedAlacTrack) -> ParsedPhysicalAudio {
    let mut facts = prepared.source_facts().clone();
    facts.index = 7; // Actual output indices are not pre-mux map ordinals.
    facts.time_base = TimeBase::new(1, facts.sample_rate).unwrap();
    let packets = prepared
        .packets()
        .iter()
        .enumerate()
        .map(|(ordinal, expected)| {
            let mut packet = packet(expected, ordinal);
            packet.pts = PacketTimestamp::Known(expected.pts + 119);
            packet.dts = packet.pts;
            packet
        })
        .collect();
    ParsedPhysicalAudio {
        tracks: vec![PhysicalAudioTrack {
            index: 7,
            codec: "alac".into(),
            alac: Some(facts),
            packets,
        }],
    }
}

fn normal(prepared: &PreparedAlacTrack, first: usize) -> ParsedNormalAlac {
    let mut remaining = prepared.packets()[first].pts.min(0).unsigned_abs();
    let mut events = Vec::new();
    let stream = physical(prepared).tracks.remove(0).alac.unwrap();
    for (ordinal, expected) in prepared.packets().iter().enumerate().skip(first) {
        let mut packet = packet(expected, ordinal);
        if ordinal == first && remaining != 0 {
            packet.trimming = Some(SampleTrimming {
                skip: u32::try_from(remaining).unwrap(),
                discard: 0,
            });
        }
        let consumed = remaining.min(u64::from(expected.physical_samples));
        remaining -= consumed;
        let visible = u64::from(expected.physical_samples) - consumed;
        packet.flags.discard = visible == 0; // Legal ordinary preroll.
        packet.duration = Some(9); // Normal mux duration is not physical sample count.
        let position = packet.position;
        events.push(AlacOutputEvent::Packet(packet));
        if visible != 0 {
            events.push(AlacOutputEvent::Frame(SourceAudioFrame {
                pts: PacketTimestamp::Known(expected.pts + i64::try_from(consumed).unwrap()),
                position,
                physical_samples: u32::try_from(visible).unwrap(),
                sample_format: stream.sample_format.clone(),
                channels: stream.channels,
                channel_layout: stream.channel_layout.clone(),
                side_data_types: Vec::new(),
            }));
        }
    }
    ParsedNormalAlac { stream, events }
}

#[test]
fn postflight_accepts_predicted_full_and_partial_skip_with_an_independent_physical_origin() {
    let (candidate, prepared) = prepared_fixture();
    let physical = physical(&prepared[0]);
    assert_ne!(physical.tracks[0].packets[0].pts, candidate.packets[0].pts);
    let proof = validate_alac_physical_output(&candidate, &prepared, &physical)
        .unwrap()
        .remove(0);
    assert_eq!(proof.actual_stream_index(), 7);
    assert_eq!(proof.mapping(), mapping());
    let normal = normal(&prepared[0], 2);
    assert_eq!(
        verify_alac_normal_output(&proof, &normal).unwrap(),
        AlacPresentationSummary {
            packets: 4,
            frames: 2,
            samples: 3
        }
    );
    // A crafted stss can preserve every physical fact yet choose another wholly
    // negative normal prefix. That prefix cannot select its own initial skip.
    assert!(verify_alac_normal_output(&proof, &self::normal(&prepared[0], 0)).is_err());
}

#[test]
fn physical_postflight_rejects_changed_clock_payload_tail_position_and_facts() {
    let (candidate, prepared) = prepared_fixture();
    let valid = physical(&prepared[0]);
    for change in 0..7 {
        let mut observed = valid.clone();
        let track = &mut observed.tracks[0];
        match change {
            0 => track.packets[1].pts = PacketTimestamp::Known(105),
            1 => track.packets[1].payload = payload(99),
            2 => track.packets[5].duration = Some(1),
            3 => track.packets[1].position = track.packets[0].position,
            4 => track.packets[1].position = None,
            5 => track.alac.as_mut().unwrap().extradata = payload(99),
            _ => track.packets[1].other_side_data.push("Param Change".into()),
        }
        assert!(
            validate_alac_physical_output(&candidate, &prepared, &observed).is_err(),
            "change {change}"
        );
    }
    let mut observed = valid;
    for packet in &mut observed.tracks[0].packets {
        let PacketTimestamp::Known(value) = packet.pts else {
            unreachable!()
        };
        packet.pts = PacketTimestamp::Known(value - 200);
        packet.dts = packet.pts;
    }
    assert!(validate_alac_physical_output(&candidate, &prepared, &observed).is_ok());
}

#[test]
fn physical_postflight_keeps_positive_alac_payloads_and_omits_only_empty_maps() {
    let (mut candidate, prepared) = prepared_fixture();
    let mut observed = physical(&prepared[0]);
    let mut empty = candidate.streams[0].clone();
    empty.index = 1;
    candidate.streams.push(empty);
    let mut positive = candidate.streams[0].clone();
    positive.index = 2;
    candidate.streams.push(positive);
    let mut packet = candidate.packets[5].clone();
    packet.stream_index = 2;
    packet.payload = payload(99);
    candidate.packets.push(packet);
    let mut actual = observed.tracks[0].clone();
    actual.index = 9;
    actual.alac.as_mut().unwrap().index = 9;
    actual.packets = vec![actual.packets[5].clone()];
    actual.packets[0].payload = payload(99);
    observed.tracks.push(actual);
    let proofs = validate_alac_physical_output(&candidate, &prepared, &observed).unwrap();
    assert_eq!(proofs.len(), 1); // No extra normal decode for the positive track.
    observed.tracks[1].packets[0].payload = payload(98);
    assert!(validate_alac_physical_output(&candidate, &prepared, &observed).is_err());
    observed.tracks[1].packets[0].payload = payload(99);
    let duplicate = observed.tracks[1].packets[0].clone();
    observed.tracks[1].packets.push(duplicate);
    assert!(validate_alac_physical_output(&candidate, &prepared, &observed).is_err());
}

#[test]
fn normal_postflight_rejects_each_unexplained_sample_or_event_change() {
    let (candidate, prepared) = prepared_fixture();
    let proof = validate_alac_physical_output(&candidate, &prepared, &physical(&prepared[0]))
        .unwrap()
        .remove(0);
    let valid = normal(&prepared[0], 2);
    for change in 0..9 {
        let mut observed = valid.clone();
        match change {
            0 => {
                observed.events.pop();
            }
            1 => observed
                .events
                .push(observed.events.last().unwrap().clone()),
            2 => {
                observed.events.remove(0);
            }
            3 => {
                if let AlacOutputEvent::Packet(packet) = &mut observed.events[0] {
                    packet.trimming.as_mut().unwrap().skip -= 1;
                }
            }
            4 => {
                if let AlacOutputEvent::Packet(packet) = &mut observed.events[1] {
                    packet.trimming = Some(SampleTrimming {
                        skip: 1,
                        discard: 0,
                    });
                }
            }
            5 => {
                if let AlacOutputEvent::Packet(packet) = &mut observed.events[0] {
                    packet.trimming.as_mut().unwrap().discard = 1;
                }
            }
            6 => {
                if let AlacOutputEvent::Frame(frame) = &mut observed.events[3] {
                    frame.physical_samples += 1;
                }
            }
            7 => {
                if let AlacOutputEvent::Frame(frame) = &mut observed.events[3] {
                    frame.pts = PacketTimestamp::Known(1);
                }
            }
            _ => {
                if let AlacOutputEvent::Frame(frame) = &mut observed.events[3] {
                    frame.position = None;
                }
            }
        }
        assert!(
            verify_alac_normal_output(&proof, &observed).is_err(),
            "change {change}"
        );
    }
}

fn stream_json() -> Value {
    json!({"index":7,"codec_name":"alac","codec_type":"audio","sample_rate":"10","channels":2,
        "channel_layout":"stereo","sample_fmt":"s16p","bits_per_raw_sample":"16","initial_padding":0,
        "time_base":"1/10","extradata_size":36,"extradata_hash":format!("SHA256:{:064x}",100)})
}

fn packet_json() -> Value {
    json!({"type":"packet","stream_index":7,"codec_type":"audio","pts":-11,"dts":-11,"duration":4,
        "pos":"1200","size":"100","flags":"KD_","data_hash":format!("SHA256:{:064x}",3),
        "side_data_list":[{"side_data_type":"Skip Samples","skip_samples":11,"discard_padding":0}]})
}

#[test]
fn output_parsers_keep_packet_only_and_zero_frame_normal_observations_distinct() {
    let limits = ManifestLimits::default();
    let physical = json!({"streams":[stream_json()],"packets":[packet_json()]});
    let parsed =
        parse_physical_audio_scan(&serde_json::to_vec(&physical).unwrap(), limits).unwrap();
    assert_eq!(parsed.tracks[0].packets[0].trimming.unwrap().skip, 11);
    let normal = json!({"streams":[stream_json()],"packets_and_frames":[packet_json()]});
    let parsed = parse_normal_alac_scan(&serde_json::to_vec(&normal).unwrap(), 7, limits).unwrap();
    assert_eq!(parsed.events.len(), 1); // Syntax is not a normal-presentation proof.
    assert!(parse_physical_audio_scan(&serde_json::to_vec(&normal).unwrap(), limits).is_err());
    assert!(parse_normal_alac_scan(&serde_json::to_vec(&physical).unwrap(), 7, limits).is_err());
    for invalid in [
        json!({"streams":[stream_json(),stream_json()],"packets":[]}),
        json!({"streams":[],"packets":[packet_json()]}),
        json!({"streams":[stream_json()],"packets":[],"error":{"code":-1}}),
    ] {
        assert!(parse_physical_audio_scan(&serde_json::to_vec(&invalid).unwrap(), limits).is_err());
    }
    let bytes = serde_json::to_vec(&normal).unwrap();
    assert!(parse_normal_alac_scan(&bytes[..bytes.len() - 1], 7, limits).is_err());
    assert!(
        parse_normal_alac_scan(
            &bytes,
            7,
            ManifestLimits {
                bytes: bytes.len() - 1,
                ..limits
            }
        )
        .is_err()
    );
    assert!(parse_normal_alac_scan(&bytes, 8, limits).is_err());
}
