use super::source::{RawEvent, RawPacket, RawStream};
use super::{
    AlacStreamFacts, ManifestLimits, ObservationError, SourceAudioFrame, SourceAudioPacket,
};
use serde::Deserialize;
use std::collections::BTreeMap;

/// One actual output audio stream. Its index may differ after empty maps vanish.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalAudioTrack {
    pub index: u32,
    pub codec: String,
    pub alac: Option<AlacStreamFacts>,
    pub packets: Vec<SourceAudioPacket>,
}

/// Packet-only `ignore_editlist` observation, in actual output audio order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedPhysicalAudio {
    pub tracks: Vec<PhysicalAudioTrack>,
}

/// Ordinary decoding may produce no frame for a fully skipped preroll packet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlacOutputEvent {
    Packet(SourceAudioPacket),
    Frame(SourceAudioFrame),
}

/// Ordinary edit-list-aware decoding of exactly one output ALAC stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedNormalAlac {
    pub stream: AlacStreamFacts,
    pub events: Vec<AlacOutputEvent>,
}

/// Parses a packet-only observation of every actual output audio stream.
///
/// Syntax alone does not prove successful process EOF, stable file identity or
/// the invocation's input options. The execution owner must establish those.
///
/// # Errors
/// Returns an error for malformed JSON/facts, wrong media types, unknown packet
/// streams, duplicate/out-of-order stream indices or exceeded capture limits.
pub fn parse_physical_audio_scan(
    bytes: &[u8],
    limits: ManifestLimits,
) -> Result<ParsedPhysicalAudio, ObservationError> {
    check_capture(bytes, limits)?;
    let raw: RawPhysical = serde_json::from_slice(bytes)
        .map_err(|_| ObservationError::Invalid("physical output JSON"))?;
    if raw.error.is_some() {
        return Err(ObservationError::Invalid("physical output probe error"));
    }
    if raw.streams.len() > limits.streams || raw.packets.len() > limits.packets {
        return Err(ObservationError::Limit("physical output records"));
    }
    let mut tracks: Vec<PhysicalAudioTrack> = Vec::with_capacity(raw.streams.len());
    let mut ordinals = BTreeMap::new();
    for value in raw.streams {
        let header: RawAudioHeader = serde_json::from_value(value.clone())
            .map_err(|_| ObservationError::Invalid("physical audio header"))?;
        if header.codec_type != "audio"
            || header.codec_name.is_empty()
            || tracks
                .last()
                .is_some_and(|track| track.index >= header.index)
        {
            return Err(ObservationError::Invalid("physical audio stream order"));
        }
        let alac = if header.codec_name == "alac" {
            Some(
                serde_json::from_value::<RawStream>(value)
                    .map_err(|_| ObservationError::Invalid("physical ALAC facts"))?
                    .parse()?,
            )
        } else {
            None
        };
        ordinals.insert(header.index, tracks.len());
        tracks.push(PhysicalAudioTrack {
            index: header.index,
            codec: header.codec_name,
            alac,
            packets: Vec::new(),
        });
    }
    for packet in raw.packets {
        let index = packet.stream_index;
        let ordinal = *ordinals
            .get(&index)
            .ok_or(ObservationError::Invalid("unmapped physical audio packet"))?;
        tracks[ordinal]
            .packets
            .push(packet.parse(index, limits.side_data_per_packet)?);
    }
    Ok(ParsedPhysicalAudio { tracks })
}

/// Parses normal output events without inventing packet/frame cardinality.
///
/// Only the later prepared-clock ledger decides whether a missing frame is a
/// predicted full skip. The execution owner must independently prove EOF.
///
/// # Errors
/// Returns an error for malformed/extra stream facts, unexpected event types,
/// wrong stream indices, explicit probe errors or exceeded capture limits.
pub fn parse_normal_alac_scan(
    bytes: &[u8],
    selected_index: u32,
    limits: ManifestLimits,
) -> Result<ParsedNormalAlac, ObservationError> {
    check_capture(bytes, limits)?;
    let raw: RawNormal = serde_json::from_slice(bytes)
        .map_err(|_| ObservationError::Invalid("normal output JSON"))?;
    if raw.error.is_some() || raw.streams.len() != 1 {
        return Err(ObservationError::Invalid("normal output probe result"));
    }
    let event_limit = limits
        .packets
        .checked_mul(2)
        .ok_or(ObservationError::Limit("normal output events"))?;
    if raw.packets_and_frames.len() > event_limit {
        return Err(ObservationError::Limit("normal output events"));
    }
    let stream = raw
        .streams
        .into_iter()
        .next()
        .ok_or(ObservationError::Invalid("missing normal ALAC stream"))?
        .parse()?;
    if stream.index != selected_index {
        return Err(ObservationError::Invalid("normal output stream index"));
    }
    let mut packet_count = 0_usize;
    let events = raw
        .packets_and_frames
        .into_iter()
        .map(|event| match event {
            RawEvent::Packet(packet) => {
                packet_count += 1;
                if packet_count > limits.packets {
                    return Err(ObservationError::Limit("normal output packets"));
                }
                packet
                    .parse(selected_index, limits.side_data_per_packet)
                    .map(AlacOutputEvent::Packet)
            }
            RawEvent::Frame(frame) => frame
                .parse(selected_index, limits.side_data_per_packet)
                .map(AlacOutputEvent::Frame),
        })
        .collect::<Result<_, _>>()?;
    Ok(ParsedNormalAlac { stream, events })
}

const fn check_capture(bytes: &[u8], limits: ManifestLimits) -> Result<(), ObservationError> {
    if bytes.len() > limits.bytes {
        return Err(ObservationError::Limit("output capture bytes"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct RawPhysical {
    streams: Vec<serde_json::Value>,
    packets: Vec<RawPacket>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawAudioHeader {
    index: u32,
    codec_name: String,
    codec_type: String,
}

#[derive(Deserialize)]
struct RawNormal {
    streams: Vec<RawStream>,
    packets_and_frames: Vec<RawEvent>,
    error: Option<serde_json::Value>,
}
