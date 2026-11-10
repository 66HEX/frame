use super::*;

pub(super) fn payload(token: u32) -> HashedData {
    HashedData {
        size: 100,
        digest: Sha256Digest::parse_hex(&format!("{token:064x}")).unwrap(),
    }
}

pub(super) fn source(tokens: &[u32], counts: &[u32]) -> ParsedAlacSource {
    let stream = AlacStreamFacts {
        index: 3,
        time_base: TimeBase::new(1, 1000).unwrap(),
        sample_rate: 48_000,
        channels: 2,
        channel_layout: Some("stereo".into()),
        sample_format: "s16p".into(),
        bits_per_raw_sample: 16,
        initial_padding: Some(0),
        extradata: payload(100),
    };
    let pairs = tokens
        .iter()
        .zip(counts)
        .enumerate()
        .map(|(index, (&token, &samples))| {
            let pts = PacketTimestamp::Known(i64::try_from(index).unwrap() * 85);
            SourcePacketFrame {
                packet: SourceAudioPacket {
                    pts,
                    dts: pts,
                    duration: None,
                    position: Some(200), // Matroska lacing may share a position.
                    payload: payload(token),
                    flags: PacketFlags::parse("K__").unwrap(),
                    trimming: None,
                    other_side_data: Vec::new(),
                },
                frame: SourceAudioFrame {
                    pts,
                    position: None,
                    physical_samples: samples,
                    sample_format: stream.sample_format.clone(),
                    channels: stream.channels,
                    channel_layout: stream.channel_layout.clone(),
                    side_data_types: Vec::new(),
                },
                effective_skip: 0,
                configuration_changed: false,
            }
        })
        .collect();
    ParsedAlacSource { stream, pairs }
}

pub(super) fn mapping() -> AlacOutputMap {
    AlacOutputMap {
        source_stream_index: 3,
        output_stream_index: 0,
        audio_output_ordinal: 0,
    }
}

pub(super) fn manifest(tokens: &[u32]) -> ParsedCopyManifest {
    let facts = source(&[], &[]).stream;
    ParsedCopyManifest {
        streams: vec![ManifestStream {
            index: 0,
            time_base: TimeBase::new(1, facts.sample_rate).unwrap(),
            media_type: "audio".into(),
            codec: "alac".into(),
            extradata: Some(facts.extradata),
            sample_rate: Some(facts.sample_rate),
            channel_layout: facts.channel_layout,
            dimensions: None,
            sample_aspect_ratio: None,
        }],
        packets: tokens
            .iter()
            .enumerate()
            .map(|(index, &token)| {
                let pts = PacketTimestamp::Known(i64::try_from(index).unwrap() * 4096);
                ManifestPacket {
                    stream_index: 0,
                    pts,
                    dts: pts,
                    duration: 0,
                    payload: payload(token),
                    side_data: Vec::new(),
                }
            })
            .collect(),
    }
}

fn plan(
    manifest: &ParsedCopyManifest,
    source: &ParsedAlacSource,
) -> Result<AlacRepairPlan, ObservationError> {
    let candidate = alac_tail_candidates(manifest, &[mapping()])?[0];
    plan_alac_repair(manifest, candidate, source)
}

#[test]
fn positive_and_empty_alac_do_not_require_source_analysis() {
    assert!(
        alac_tail_candidates(&manifest(&[]), &[mapping()])
            .unwrap()
            .is_empty()
    );
    let mut ordinary = manifest(&[1, 2]);
    ordinary.packets[1].duration = 2112;
    assert!(
        alac_tail_candidates(&ordinary, &[mapping()])
            .unwrap()
            .is_empty()
    );
    ordinary.packets[1].duration = -1;
    assert!(alac_tail_candidates(&ordinary, &[mapping()]).is_err());
    assert!(alac_tail_candidates(&ordinary, &[]).is_err());
}

#[test]
fn source_matching_requires_full_sequence_and_equivalent_repeated_occurrences() {
    let copied = manifest(&[1, 2]);
    let repeated = source(&[1, 2, 1, 2], &[4096, 2112, 4096, 2112]);
    assert_eq!(
        plan(&copied, &repeated).unwrap().physical_samples(),
        &[4096, 2112]
    );
    let mut conflicting = repeated.clone();
    conflicting.pairs[3].frame.physical_samples = 2111;
    assert!(plan(&copied, &conflicting).is_err());
    conflicting = repeated;
    conflicting.pairs[2].packet.trimming = Some(SampleTrimming {
        skip: 0,
        discard: 1,
    });
    assert!(plan(&copied, &conflicting).is_err());
    assert!(plan(&copied, &source(&[2], &[2112])).is_err());
    assert!(plan(&copied, &source(&[2, 1], &[2112, 4096])).is_err());
}

#[test]
fn selected_window_rejects_carried_skip_but_not_distinct_unselected_hazards() {
    let copied = manifest(&[1, 2]);
    let mut observed = source(&[9, 1, 2, 8], &[4096, 4096, 2112, 4096]);
    observed.pairs[0].packet.trimming = Some(SampleTrimming {
        skip: 0,
        discard: 200,
    });
    observed.pairs[3].configuration_changed = true;
    assert!(plan(&copied, &observed).is_ok());
    observed.pairs[1].effective_skip = 1;
    assert!(plan(&copied, &observed).is_err());
    observed.pairs[1].effective_skip = 0;
    for padding in [None, Some(1)] {
        observed.stream.initial_padding = padding;
        assert!(plan(&copied, &observed).is_err());
    }
    observed.stream.initial_padding = Some(0);
    observed.pairs[1].frame.channels = 1;
    assert!(plan(&copied, &observed).is_err());
}

#[test]
fn only_corroborated_all_zero_skip_side_data_is_accepted() {
    let mut copied = manifest(&[1]);
    let mut observed = source(&[1], &[1]);
    copied.packets[0].side_data.push(HashedData {
        size: 10,
        digest: Sha256Digest::parse_hex(
            "01d448afd928065458cf670b60f5a594d735af0172c8d67f22a81680132681ca",
        )
        .unwrap(),
    });
    assert!(plan(&copied, &observed).is_err());
    observed.pairs[0].packet.trimming = Some(SampleTrimming {
        skip: 0,
        discard: 0,
    });
    assert!(plan(&copied, &observed).is_ok());
    copied.packets[0].side_data[0].size = 11;
    assert!(plan(&copied, &observed).is_err());
    copied.packets[0].side_data.clear();
    observed.pairs[0]
        .packet
        .other_side_data
        .push("unrecognized".into());
    assert!(plan(&copied, &observed).is_err());
}

#[test]
fn duplicate_source_maps_keep_global_output_and_audio_ordinals_distinct() {
    let base = manifest(&[1, 2]);
    let video = ManifestStream {
        index: 0,
        time_base: TimeBase::new(1, 1000).unwrap(),
        media_type: "video".into(),
        codec: "h264".into(),
        extradata: None,
        sample_rate: None,
        channel_layout: None,
        dimensions: Some((64, 48)),
        sample_aspect_ratio: None,
    };
    let mut copied = ParsedCopyManifest {
        streams: vec![video],
        packets: Vec::new(),
    };
    let mappings: Vec<_> = [3, 1, 3]
        .into_iter()
        .enumerate()
        .map(|(ordinal, index)| {
            let audio_output_ordinal = u32::try_from(ordinal).unwrap();
            let output_stream_index = audio_output_ordinal + 1;
            let mut stream = base.streams[0].clone();
            stream.index = output_stream_index;
            copied.streams.push(stream);
            copied
                .packets
                .extend(base.packets.iter().cloned().map(|mut packet| {
                    packet.stream_index = output_stream_index;
                    packet
                }));
            AlacOutputMap {
                source_stream_index: index,
                output_stream_index,
                audio_output_ordinal,
            }
        })
        .collect();
    let candidates = alac_tail_candidates(&copied, &mappings).unwrap();
    for (ordinal, candidate) in candidates.into_iter().enumerate() {
        let mut observed = source(&[1, 2], &[4096, 17]);
        observed.stream.index = candidate.mapping().source_stream_index;
        let repair = plan_alac_repair(&copied, candidate, &observed).unwrap();
        let args = repair.bitstream_filter_args();
        assert_eq!(args[0], format!("-bsf:a:{ordinal}"));
        assert!(args[1].contains("time_base=1/48000:prescale=1"));
        assert!(args[1].ends_with("if(eq(N,1),17,DURATION))'"));
    }
    let mut wrong = mappings.clone();
    wrong[0].audio_output_ordinal = 1;
    assert!(alac_tail_candidates(&copied, &wrong).is_err());
    assert!(alac_tail_candidates(&copied, &mappings[..2]).is_err());
}

#[test]
fn expression_guards_reject_unknown_negative_and_inexact_values() {
    let observed = source(&[1], &[17]);
    let mut copied = manifest(&[1]);
    copied.packets[0].pts = PacketTimestamp::Known(1 << 53);
    copied.packets[0].dts = copied.packets[0].pts;
    assert!(plan(&copied, &observed).is_ok());
    copied.packets[0].pts = PacketTimestamp::Known((1 << 53) + 1);
    assert!(plan(&copied, &observed).is_err());
    copied.packets[0].pts = PacketTimestamp::Unknown;
    assert!(plan(&copied, &observed).is_err());
    copied.packets[0].pts = PacketTimestamp::Known(1 << 52);
    copied.packets[0].dts = copied.packets[0].pts;
    copied.streams[0].time_base = TimeBase::new(1, 1).unwrap();
    assert!(plan(&copied, &observed).is_err());
}

#[test]
fn second_observer_prepares_exact_clock_and_predicts_normal_preroll() {
    let tokens: Vec<_> = (1..=21).collect();
    let mut counts = vec![4096; 21];
    counts[20] = 17;
    let mut copied = manifest(&tokens);
    for packet in &mut copied.packets {
        let PacketTimestamp::Known(pts) = packet.pts else {
            unreachable!()
        };
        packet.pts = PacketTimestamp::Known(pts - 60_000);
        packet.dts = packet.pts;
    }
    let repair = plan(&copied, &source(&tokens, &counts)).unwrap();
    let mut candidate = copied.clone();
    candidate.packets[20].duration = 17;
    let plans = [repair];
    let prepared = prepare_alac_tracks(&copied, &plans, &candidate).unwrap();
    assert_eq!(prepared[0].normal_first_packet(), 2);
    assert_eq!(prepared[0].normal_initial_skip(), 51_808);
    assert_eq!(prepared[0].packets()[20].physical_samples, 17);
    assert_eq!(prepared[0].packets()[20].pts, 21_920);
    // A wrong observed suffix (such as a crafted stss retaining packet zero)
    // cannot become the expected ledger: its start is already fixed at two.
    assert_ne!(prepared[0].normal_first_packet(), 0);
    let mut wrong = candidate.clone();
    wrong.packets[3].duration = 1;
    assert!(prepare_alac_tracks(&copied, &plans, &wrong).is_err());
    wrong = candidate.clone();
    wrong.packets[3].pts = PacketTimestamp::Known(-47_711);
    wrong.packets[3].dts = wrong.packets[3].pts;
    assert!(prepare_alac_tracks(&copied, &plans, &wrong).is_err());
    wrong = candidate.clone();
    wrong.packets[3].payload = payload(999);
    assert!(prepare_alac_tracks(&copied, &plans, &wrong).is_err());
    wrong = candidate.clone();
    wrong.packets[3].pts = wrong.packets[2].pts;
    wrong.packets[3].dts = wrong.packets[3].pts;
    assert!(prepare_alac_tracks(&copied, &plans, &wrong).is_err());
    let mut changed_original = copied.clone();
    changed_original.packets[0].payload = payload(999);
    assert!(prepare_alac_tracks(&changed_original, &plans, &candidate).is_err());
}

#[test]
fn candidate_clock_rescales_actual_first_observer_with_signed_integer_ties() {
    let mut copied = manifest(&[1, 2]);
    copied.streams[0].time_base = TimeBase::new(1, 2000).unwrap();
    copied.streams[0].sample_rate = Some(44_100);
    copied.packets[0].pts = PacketTimestamp::Known(-10);
    copied.packets[0].dts = copied.packets[0].pts;
    copied.packets[1].pts = PacketTimestamp::Known(10);
    copied.packets[1].dts = copied.packets[1].pts;
    let mut observed = source(&[1, 2], &[4096, 17]);
    observed.stream.sample_rate = 44_100;
    let repair = plan(&copied, &observed).unwrap();
    let mut candidate = copied.clone();
    candidate.streams[0].time_base = TimeBase::new(1, 44_100).unwrap();
    candidate.packets[0].pts = PacketTimestamp::Known(-221);
    candidate.packets[0].dts = candidate.packets[0].pts;
    candidate.packets[1].pts = PacketTimestamp::Known(221);
    candidate.packets[1].dts = candidate.packets[1].pts;
    candidate.packets[1].duration = 17;
    let plans = [repair];
    assert!(prepare_alac_tracks(&copied, &plans, &candidate).is_ok());
    candidate.packets[0].pts = PacketTimestamp::Known(-220);
    candidate.packets[0].dts = candidate.packets[0].pts;
    assert!(prepare_alac_tracks(&copied, &plans, &candidate).is_err());
}
