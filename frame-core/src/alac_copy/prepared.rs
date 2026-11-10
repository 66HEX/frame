use super::planner::{exact_integer, known, stream_packets};
use super::{
    AlacOutputMap, AlacRepairPlan, AlacStreamFacts, HashedData, ObservationError,
    ParsedCopyManifest, TimeBase,
};
use std::collections::BTreeMap;

/// The second observer's exact sample clock and the source's physical count.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedAlacPacket {
    pub pts: i64,
    pub dts: i64,
    pub duration: i64,
    pub payload: HashedData,
    pub physical_samples: u32,
}

/// Expected output facts, not permission to publish without both postflight scans.
#[derive(Clone, Debug)]
pub struct PreparedAlacTrack {
    mapping: AlacOutputMap,
    source_facts: AlacStreamFacts,
    packets: Vec<PreparedAlacPacket>,
    normal_first: usize,
    initial_skip: u64,
}

impl PreparedAlacTrack {
    #[must_use]
    pub const fn mapping(&self) -> AlacOutputMap {
        self.mapping
    }

    #[must_use]
    pub const fn source_facts(&self) -> &AlacStreamFacts {
        &self.source_facts
    }

    #[must_use]
    pub fn packets(&self) -> &[PreparedAlacPacket] {
        &self.packets
    }

    /// Predicted from the prepared clock, never inferred from the output suffix.
    #[must_use]
    pub const fn normal_first_packet(&self) -> usize {
        self.normal_first
    }

    #[must_use]
    pub const fn normal_initial_skip(&self) -> u64 {
        self.initial_skip
    }
}

/// Checks a complete candidate observer against the original manifest and plans.
///
/// Other streams remain identical. A repaired stream keeps all payloads and
/// side data; only the declared time base and terminal duration may change.
/// Its actual observed PTS/DTS become the authority for subsequent postflight.
///
/// # Errors
/// Returns an error for omitted/duplicate repairs, changed stream facts, changed
/// packet sequences, unexpected durations or unsupported sample clocks.
pub fn prepare_alac_tracks(
    original: &ParsedCopyManifest,
    plans: &[AlacRepairPlan],
    candidate: &ParsedCopyManifest,
) -> Result<Vec<PreparedAlacTrack>, ObservationError> {
    if original.streams.len() != candidate.streams.len() {
        return Err(ObservationError::Invalid("candidate stream count"));
    }
    let mut by_output = BTreeMap::new();
    for plan in plans {
        if by_output
            .insert(plan.mapping.output_stream_index, plan)
            .is_some()
        {
            return Err(ObservationError::Invalid("duplicate repair plan"));
        }
    }
    let mut prepared = Vec::new();
    for (before, after) in original.streams.iter().zip(&candidate.streams) {
        let before_packets = stream_packets(original, before.index);
        let after_packets = stream_packets(candidate, after.index);
        if let Some(plan) = by_output.remove(&before.index) {
            if before != &plan.original_stream
                || before_packets.iter().copied().ne(&plan.original_packets)
            {
                return Err(ObservationError::Invalid(
                    "changed original repair observation",
                ));
            }
            let sample_tb = TimeBase::new(1, plan.source_facts.sample_rate)?;
            let mut expected = before.clone();
            expected.time_base = sample_tb;
            if &expected != after
                || before_packets.len() != after_packets.len()
                || before_packets.len() != plan.physical_samples.len()
                || before_packets.is_empty()
            {
                return Err(ObservationError::Invalid("candidate stream facts"));
            }
            let mut packets = Vec::with_capacity(after_packets.len());
            for (ordinal, (before_packet, after)) in
                before_packets.iter().zip(after_packets).enumerate()
            {
                let duration = if ordinal + 1 == before_packets.len() {
                    i64::from(plan.physical_samples[ordinal])
                } else {
                    exact_integer(
                        before
                            .time_base
                            .rescale(before_packet.duration, sample_tb)?,
                    )?
                };
                let pts = exact_integer(known(after.pts)?)?;
                let dts = exact_integer(known(after.dts)?)?;
                if before_packet.payload != after.payload
                    || before_packet.side_data != after.side_data
                    || before
                        .time_base
                        .rescale(known(before_packet.pts)?, sample_tb)?
                        != pts
                    || before
                        .time_base
                        .rescale(known(before_packet.dts)?, sample_tb)?
                        != dts
                    || after.duration != duration
                    || pts != dts
                    || packets
                        .last()
                        .is_some_and(|previous: &PreparedAlacPacket| previous.pts >= pts)
                {
                    return Err(ObservationError::Invalid("candidate packet sequence"));
                }
                packets.push(PreparedAlacPacket {
                    pts,
                    dts,
                    duration,
                    payload: after.payload,
                    physical_samples: plan.physical_samples[ordinal],
                });
            }
            prepared.push(with_normal_window(plan, packets));
        } else if before != after
            || before_packets != after_packets
            || (before.codec == "alac"
                && before_packets
                    .last()
                    .is_some_and(|packet| packet.duration == 0))
        {
            return Err(ObservationError::Invalid(
                "unrepaired or changed copy stream",
            ));
        }
    }
    if !by_output.is_empty() {
        return Err(ObservationError::Invalid("missing repair output stream"));
    }
    Ok(prepared)
}

fn with_normal_window(
    plan: &AlacRepairPlan,
    packets: Vec<PreparedAlacPacket>,
) -> PreparedAlacTrack {
    // MOV's standard all-intra audio preroll searches one second before
    // media_time. A crafted stss can select a different suffix despite
    // ffprobe reporting every ALAC packet as key; do not trust that suffix.
    let normal_first = packets
        .iter()
        .rposition(|packet| packet.pts <= -i64::from(plan.source_facts.sample_rate))
        .unwrap_or(0);
    let initial_skip = packets[normal_first].pts.min(0).unsigned_abs();
    PreparedAlacTrack {
        mapping: plan.mapping,
        source_facts: plan.source_facts.clone(),
        packets,
        normal_first,
        initial_skip,
    }
}
