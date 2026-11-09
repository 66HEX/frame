use super::matching::match_source_samples;
use super::{
    AlacStreamFacts, ManifestPacket, ManifestStream, ObservationError, PacketTimestamp,
    ParsedAlacSource, ParsedCopyManifest, TimeBase,
};
use std::collections::BTreeSet;

pub(super) const EXACT_EXPRESSION_INTEGER: u64 = 1 << 53;

/// Three separate namespaces; duplicate source selections retain distinct outputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlacOutputMap {
    pub source_stream_index: u32,
    pub output_stream_index: u32,
    pub audio_output_ordinal: u32,
}

/// A terminal zero observed in a complete copy manifest, not a repair proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlacTailCandidate {
    mapping: AlacOutputMap,
    packet_count: usize,
}

impl AlacTailCandidate {
    #[must_use]
    pub const fn mapping(self) -> AlacOutputMap {
        self.mapping
    }
}

/// A proposed terminal-only BSF, still requiring a second complete observer.
#[derive(Clone, Debug)]
pub struct AlacRepairPlan {
    pub(super) mapping: AlacOutputMap,
    pub(super) source_facts: AlacStreamFacts,
    pub(super) physical_samples: Vec<u32>,
    pub(super) original_stream: ManifestStream,
    pub(super) original_packets: Vec<ManifestPacket>,
}

impl AlacRepairPlan {
    #[must_use]
    pub const fn mapping(&self) -> AlacOutputMap {
        self.mapping
    }

    #[must_use]
    pub fn physical_samples(&self) -> &[u32] {
        &self.physical_samples
    }

    /// The caller must reject an existing BSF at this audio output ordinal.
    #[must_use]
    pub fn bitstream_filter_args(&self) -> [String; 2] {
        let terminal = self.physical_samples.len() - 1;
        let samples = self.physical_samples[terminal];
        let rate = self.source_facts.sample_rate;
        [
            format!("-bsf:a:{}", self.mapping.audio_output_ordinal),
            format!(
                "setts=pts=PTS:dts=DTS:time_base=1/{rate}:prescale=1:duration='if(gt(DURATION,0)+not(eq(NEXT_DTS,NOPTS)),DURATION,if(eq(N,{terminal}),{samples},DURATION))'"
            ),
        ]
    }
}

/// Finds only zero-duration tails; positive and empty tracks remain ordinary.
///
/// The caller supplies the validated actual Copy maps and independently proves
/// process completion and request identity. Every observed ALAC output must map.
///
/// # Errors
/// Returns an error for missing, repeated or inconsistent output mappings or a
/// negative terminal duration. A failed observation is not ordinary passthrough.
pub fn alac_tail_candidates(
    manifest: &ParsedCopyManifest,
    mappings: &[AlacOutputMap],
) -> Result<Vec<AlacTailCandidate>, ObservationError> {
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();
    for &mapping in mappings {
        mapped_stream(manifest, mapping)?;
        if !seen.insert(mapping.output_stream_index) {
            return Err(ObservationError::Invalid("duplicate ALAC output mapping"));
        }
        let packets = stream_packets(manifest, mapping.output_stream_index);
        if let Some(terminal) = packets.last() {
            if terminal.duration < 0 {
                return Err(ObservationError::Invalid("negative terminal duration"));
            }
            if terminal.duration == 0 {
                candidates.push(AlacTailCandidate {
                    mapping,
                    packet_count: packets.len(),
                });
            }
        }
    }
    if manifest
        .streams
        .iter()
        .any(|stream| stream.codec == "alac" && !seen.contains(&stream.index))
    {
        return Err(ObservationError::Invalid("unmapped ALAC output"));
    }
    Ok(candidates)
}

/// Binds the complete copied sequence to equivalent decoded source sample facts.
///
/// Source PTS/DTS must be known, but they are not rescaled into a guessed copy
/// clock. The second actual BSF observer supplies the final packet clock.
///
/// # Errors
/// Returns an error for incomplete coverage, ambiguous counts/configuration,
/// affected trimming or parameter changes, or inexact expression integers.
pub fn plan_alac_repair(
    manifest: &ParsedCopyManifest,
    candidate: AlacTailCandidate,
    source: &ParsedAlacSource,
) -> Result<AlacRepairPlan, ObservationError> {
    let mapping = candidate.mapping;
    let stream = mapped_stream(manifest, mapping)?;
    let packets = stream_packets(manifest, mapping.output_stream_index);
    if packets.len() != candidate.packet_count
        || packets.last().is_none_or(|packet| packet.duration != 0)
    {
        return Err(ObservationError::Invalid("changed terminal candidate"));
    }
    if source.stream.index != mapping.source_stream_index
        || stream.extradata != Some(source.stream.extradata)
        || stream.sample_rate != Some(source.stream.sample_rate)
        || stream.channel_layout != source.stream.channel_layout
        || source.stream.initial_padding != Some(0)
    {
        return Err(ObservationError::Invalid("source decoder configuration"));
    }
    let output_tb = TimeBase::new(1, source.stream.sample_rate)?;
    exact_integer(i64::try_from(packets.len() - 1).map_err(|_| ObservationError::Overflow)?)?;
    for packet in &packets {
        for value in [known(packet.pts)?, known(packet.dts)?, packet.duration] {
            exact_integer(value)?;
            exact_integer(stream.time_base.rescale(value, output_tb)?)?;
        }
        if packet.duration < 0 {
            return Err(ObservationError::Invalid("negative copied duration"));
        }
    }
    let physical_samples = match_source_samples(&packets, source)?;
    Ok(AlacRepairPlan {
        mapping,
        source_facts: source.stream.clone(),
        physical_samples,
        original_stream: stream.clone(),
        original_packets: packets.into_iter().cloned().collect(),
    })
}

pub(super) fn stream_packets(manifest: &ParsedCopyManifest, index: u32) -> Vec<&ManifestPacket> {
    manifest
        .packets
        .iter()
        .filter(|packet| packet.stream_index == index)
        .collect()
}

pub(super) fn mapped_stream(
    manifest: &ParsedCopyManifest,
    mapping: AlacOutputMap,
) -> Result<&ManifestStream, ObservationError> {
    let stream = manifest
        .streams
        .get(usize::try_from(mapping.output_stream_index).map_err(|_| ObservationError::Overflow)?)
        .ok_or(ObservationError::Invalid("missing ALAC output stream"))?;
    let ordinal = manifest
        .streams
        .iter()
        .filter(|stream| stream.media_type == "audio")
        .position(|stream| stream.index == mapping.output_stream_index);
    if stream.index != mapping.output_stream_index
        || stream.media_type != "audio"
        || stream.codec != "alac"
        || ordinal != usize::try_from(mapping.audio_output_ordinal).ok()
    {
        return Err(ObservationError::Invalid("ALAC output stream mapping"));
    }
    Ok(stream)
}

pub(super) const fn known(value: PacketTimestamp) -> Result<i64, ObservationError> {
    match value {
        PacketTimestamp::Known(value) if value != i64::MIN => Ok(value),
        _ => Err(ObservationError::Invalid("unknown packet timestamp")),
    }
}

pub(super) const fn exact_integer(value: i64) -> Result<i64, ObservationError> {
    if value.unsigned_abs() > EXACT_EXPRESSION_INTEGER {
        return Err(ObservationError::Invalid("inexact BSF integer"));
    }
    Ok(value)
}
