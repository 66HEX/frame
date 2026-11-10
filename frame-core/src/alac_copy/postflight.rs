use super::planner::{known, mapped_stream, stream_packets};
use super::{
    AlacOutputEvent, AlacOutputMap, AlacStreamFacts, ObservationError, ParsedCopyManifest,
    ParsedNormalAlac, ParsedPhysicalAudio, PhysicalAudioTrack, PreparedAlacPacket,
    PreparedAlacTrack, SourceAudioFrame, SourceAudioPacket, TimeBase,
};
use std::collections::{BTreeMap, BTreeSet};

/// A checked physical output track, still requiring normal-presentation proof.
#[derive(Clone, Debug)]
pub struct PhysicalAlacProof {
    prepared: PreparedAlacTrack,
    stream: AlacStreamFacts,
    positions: Vec<u64>,
}

impl PhysicalAlacProof {
    #[must_use]
    pub const fn actual_stream_index(&self) -> u32 {
        self.stream.index
    }

    #[must_use]
    pub const fn mapping(&self) -> AlacOutputMap {
        self.prepared.mapping()
    }
}

/// Exact counts from a checked normal decode, not process/identity permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlacPresentationSummary {
    pub packets: usize,
    pub frames: usize,
    pub samples: u64,
}

/// Reconciles nonempty audio maps with physical output order and repaired tails.
///
/// Empty selected streams may disappear during MOV muxing. Physical packet PTS
/// use a media origin distinct from the candidate edit-list clock; compare their
/// exact differences, not absolute PTS. Normal decoding is checked separately.
///
/// # Errors
/// Returns an error for changed mappings/facts/payloads, ambiguous positions,
/// wrong relative clocks or terminal stored duration, or missing repairs.
pub fn validate_alac_physical_output(
    candidate: &ParsedCopyManifest,
    prepared: &[PreparedAlacTrack],
    physical: &ParsedPhysicalAudio,
) -> Result<Vec<PhysicalAlacProof>, ObservationError> {
    let nonempty: BTreeSet<_> = candidate
        .packets
        .iter()
        .map(|packet| packet.stream_index)
        .collect();
    let expected: Vec<_> = candidate
        .streams
        .iter()
        .filter(|stream| stream.media_type == "audio" && nonempty.contains(&stream.index))
        .collect();
    if physical.tracks.len() != expected.len() {
        return Err(ObservationError::Invalid("physical nonempty audio count"));
    }
    let mut repairs = BTreeMap::new();
    for track in prepared {
        let stream = mapped_stream(candidate, track.mapping())?;
        let copied = stream_packets(candidate, stream.index);
        if stream.time_base != TimeBase::new(1, track.source_facts().sample_rate)?
            || copied.len() != track.packets().len()
            || copied.iter().zip(track.packets()).any(|(before, after)| {
                known(before.pts) != Ok(after.pts)
                    || known(before.dts) != Ok(after.dts)
                    || before.duration != after.duration
                    || before.payload != after.payload
            })
            || repairs.insert(stream.index, track).is_some()
        {
            return Err(ObservationError::Invalid(
                "changed prepared postflight track",
            ));
        }
    }
    let mut proofs = Vec::with_capacity(prepared.len());
    for (expected, actual) in expected.into_iter().zip(&physical.tracks) {
        if expected.codec != actual.codec || actual.packets.is_empty() {
            return Err(ObservationError::Invalid("physical audio map order"));
        }
        if expected.codec == "alac"
            && candidate
                .packets
                .iter()
                .filter(|packet| packet.stream_index == expected.index)
                .map(|packet| packet.payload)
                .ne(actual.packets.iter().map(|packet| packet.payload))
        {
            return Err(ObservationError::Invalid("physical copied ALAC payloads"));
        }
        if let Some(prepared) = repairs.remove(&expected.index) {
            proofs.push(check_physical_track(prepared, actual)?);
        }
    }
    if !repairs.is_empty() {
        return Err(ObservationError::Invalid("missing physical repair output"));
    }
    Ok(proofs)
}

fn check_physical_track(
    prepared: &PreparedAlacTrack,
    actual: &PhysicalAudioTrack,
) -> Result<PhysicalAlacProof, ObservationError> {
    let stream = actual
        .alac
        .as_ref()
        .ok_or(ObservationError::Invalid("missing physical ALAC facts"))?;
    let mut expected_facts = prepared.source_facts().clone();
    expected_facts.index = actual.index;
    expected_facts.time_base = TimeBase::new(1, expected_facts.sample_rate)?;
    if stream != &expected_facts || actual.packets.len() != prepared.packets().len() {
        return Err(ObservationError::Invalid("physical ALAC facts/count"));
    }
    let first = actual
        .packets
        .first()
        .ok_or(ObservationError::Invalid("empty repaired physical track"))?;
    let first_timestamp = known(first.pts)?;
    let initial_decode_time = known(first.dts)?;
    let prepared_first = prepared
        .packets()
        .first()
        .ok_or(ObservationError::Invalid("empty prepared track"))?;
    let mut positions = Vec::with_capacity(actual.packets.len());
    let mut seen = BTreeSet::new();
    for (packet, expected) in actual.packets.iter().zip(prepared.packets()) {
        let pts = known(packet.pts)?;
        let dts = known(packet.dts)?;
        let position = packet
            .position
            .ok_or(ObservationError::Invalid("unknown output packet position"))?;
        if packet.payload != expected.payload
            || !packet.flags.key
            || packet.flags.discard
            || packet.flags.corrupt
            || !packet.other_side_data.is_empty()
            || packet
                .trimming
                .is_some_and(|trim| trim.skip != 0 || trim.discard != 0)
            || pts != dts
            || i128::from(pts) - i128::from(first_timestamp)
                != i128::from(expected.pts) - i128::from(prepared_first.pts)
            || i128::from(dts) - i128::from(initial_decode_time)
                != i128::from(expected.dts) - i128::from(prepared_first.dts)
            || !seen.insert(position)
        {
            return Err(ObservationError::Invalid("physical ALAC packet sequence"));
        }
        positions.push(position);
    }
    if actual.packets.last().and_then(|packet| packet.duration)
        != prepared
            .packets()
            .last()
            .map(|packet| i64::from(packet.physical_samples))
    {
        return Err(ObservationError::Invalid("physical ALAC terminal duration"));
    }
    Ok(PhysicalAlacProof {
        prepared: prepared.clone(),
        stream: stream.clone(),
        positions,
    })
}

/// Checks the exact predicted normal suffix and every packet's visible samples.
///
/// The one-second preroll boundary and initial skip come only from the prepared
/// clock. An output-chosen negative suffix cannot authorize its own edit list.
/// Fully skipped packets must have no frame; every other packet must have one.
///
/// # Errors
/// Returns an error for changed facts, wrong suffix/skip/positions/timestamps,
/// missing or extra frames, mismatched sample counts or unexplained remaining skip.
pub fn verify_alac_normal_output(
    physical: &PhysicalAlacProof,
    normal: &ParsedNormalAlac,
) -> Result<AlacPresentationSummary, ObservationError> {
    if normal.stream != physical.stream {
        return Err(ObservationError::Invalid("normal ALAC stream facts"));
    }
    let prepared = &physical.prepared;
    let first = prepared.normal_first_packet();
    let mut events = normal.events.iter();
    let mut remaining = prepared.normal_initial_skip();
    let mut summary = AlacPresentationSummary {
        packets: 0,
        frames: 0,
        samples: 0,
    };
    for (ordinal, expected) in prepared.packets()[first..].iter().enumerate() {
        let Some(AlacOutputEvent::Packet(packet)) = events.next() else {
            return Err(ObservationError::Invalid("normal ALAC packet order"));
        };
        let expected_skip = if ordinal == 0 {
            prepared.normal_initial_skip()
        } else {
            0
        };
        let position = physical.positions[first + ordinal];
        check_normal_packet(packet, expected, position, expected_skip)?;
        let consumed = remaining.min(u64::from(expected.physical_samples));
        remaining -= consumed;
        let visible = u64::from(expected.physical_samples) - consumed;
        if visible != 0 {
            let Some(AlacOutputEvent::Frame(frame)) = events.next() else {
                return Err(ObservationError::Invalid("missing normal ALAC frame"));
            };
            check_normal_frame(frame, expected, &normal.stream, position, consumed, visible)?;
            summary.frames += 1;
            summary.samples = summary
                .samples
                .checked_add(visible)
                .ok_or(ObservationError::Overflow)?;
        }
        summary.packets += 1;
    }
    if remaining != 0 || events.next().is_some() {
        return Err(ObservationError::Invalid(
            "incomplete normal ALAC presentation",
        ));
    }
    Ok(summary)
}

fn check_normal_packet(
    packet: &SourceAudioPacket,
    expected: &PreparedAlacPacket,
    position: u64,
    expected_skip: u64,
) -> Result<(), ObservationError> {
    if known(packet.pts)? != expected.pts
        || known(packet.dts)? != expected.dts
        || packet.payload != expected.payload
        || packet.position != Some(position)
        || !packet.flags.key
        || packet.flags.corrupt
        || !packet.other_side_data.is_empty()
        || u64::from(packet.trimming.map_or(0, |trim| trim.skip)) != expected_skip
        || packet.trimming.is_some_and(|trim| trim.discard != 0)
    {
        return Err(ObservationError::Invalid("normal ALAC packet/skip"));
    }
    // A normal packet may carry discard/preroll flags and a mux-rounded duration.
    // Neither changes the expected decoded frame count or exact sample clock.
    Ok(())
}

fn check_normal_frame(
    frame: &SourceAudioFrame,
    expected: &PreparedAlacPacket,
    stream: &AlacStreamFacts,
    position: u64,
    consumed: u64,
    visible: u64,
) -> Result<(), ObservationError> {
    let consumed = i64::try_from(consumed).map_err(|_| ObservationError::Overflow)?;
    let pts = expected
        .pts
        .checked_add(consumed)
        .ok_or(ObservationError::Overflow)?;
    if known(frame.pts)? != pts
        || frame.position != Some(position)
        || u64::from(frame.physical_samples) != visible
        || frame.sample_format != stream.sample_format
        || frame.channels != stream.channels
        || frame.channel_layout != stream.channel_layout
        || !frame.side_data_types.is_empty()
    {
        return Err(ObservationError::Invalid("normal ALAC decoded samples"));
    }
    Ok(())
}
