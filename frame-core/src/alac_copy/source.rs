use super::numeric::{signed, unsigned};
use super::{HashedData, ManifestLimits, ObservationError, PacketFlags, PacketTimestamp, TimeBase};
use serde::Deserialize;

/// Decoder facts observed for the selected source stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlacStreamFacts {
    pub index: u32,
    pub time_base: TimeBase,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: Option<String>,
    pub sample_format: String,
    pub bits_per_raw_sample: u32,
    pub initial_padding: Option<u64>,
    pub extradata: HashedData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SampleTrimming {
    pub skip: u32,
    pub discard: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceAudioPacket {
    pub pts: PacketTimestamp,
    pub dts: PacketTimestamp,
    pub duration: Option<i64>,
    pub position: Option<u64>,
    pub payload: HashedData,
    pub flags: PacketFlags,
    /// `None` differs from an explicit zero skip, which resets decoder carry.
    pub trimming: Option<SampleTrimming>,
    pub other_side_data: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceAudioFrame {
    pub pts: PacketTimestamp,
    pub position: Option<u64>,
    pub physical_samples: u32,
    pub sample_format: String,
    pub channels: u32,
    pub channel_layout: Option<String>,
    pub side_data_types: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePacketFrame {
    pub packet: SourceAudioPacket,
    pub frame: SourceAudioFrame,
    /// Skip that ordinary decoding would apply, including carry from earlier packets.
    pub effective_skip: u64,
    /// A preceding or current parameter change invalidates a header-only fact binding.
    pub configuration_changed: bool,
}

/// Complete JSON syntax and ordered packet/frame associations, not process EOF proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedAlacSource {
    pub stream: AlacStreamFacts,
    pub pairs: Vec<SourcePacketFrame>,
}

/// Parses one selected ALAC source stream observed with `skip_manual`.
///
/// The execution layer must prove the exact invocation, successful owned EOF,
/// clean error diagnostics, untruncated capture and stable source/runtime identity.
/// Trimming facts are retained rather than rejecting an unaffected copied segment.
///
/// # Errors
/// Returns an error for malformed facts, extra streams, incomplete packet/frame
/// associations, inconsistent corroborating positions or exceeded capture limits.
pub fn parse_alac_source_scan(
    bytes: &[u8],
    selected_index: u32,
    limits: ManifestLimits,
) -> Result<ParsedAlacSource, ObservationError> {
    if bytes.len() > limits.bytes {
        return Err(ObservationError::Limit("source capture bytes"));
    }
    let raw: RawSource =
        serde_json::from_slice(bytes).map_err(|_| ObservationError::Invalid("source JSON"))?;
    if raw.error.is_some() || raw.streams.len() != 1 {
        return Err(ObservationError::Invalid("source probe result"));
    }
    let event_limit = limits
        .packets
        .checked_mul(2)
        .ok_or(ObservationError::Limit("source event count"))?;
    if raw.packets_and_frames.len() > event_limit {
        return Err(ObservationError::Limit("source event count"));
    }
    let stream = raw
        .streams
        .into_iter()
        .next()
        .ok_or(ObservationError::Invalid("missing source stream"))?
        .parse()?;
    if stream.index != selected_index {
        return Err(ObservationError::Invalid("selected source stream"));
    }
    let mut pairs = Vec::new();
    let mut pending = None;
    let mut skip_remaining = stream.initial_padding.unwrap_or(0);
    let mut configuration_changed = false;
    for event in raw.packets_and_frames {
        match event {
            RawEvent::Packet(packet) => {
                if pending.is_some() {
                    return Err(ObservationError::Invalid("multiple packets before a frame"));
                }
                pending = Some(packet.parse(selected_index, limits.side_data_per_packet)?);
            }
            RawEvent::Frame(frame) => {
                if pairs.len() >= limits.packets {
                    return Err(ObservationError::Limit("source packet count"));
                }
                let packet = pending
                    .take()
                    .ok_or(ObservationError::Invalid("frame without a packet"))?;
                let frame = frame.parse(selected_index, limits.side_data_per_packet)?;
                if let (Some(packet_pos), Some(frame_pos)) = (packet.position, frame.position)
                    && packet_pos != frame_pos
                {
                    return Err(ObservationError::Invalid("packet/frame position mismatch"));
                }
                if let (PacketTimestamp::Known(packet_pts), PacketTimestamp::Known(frame_pts)) =
                    (packet.pts, frame.pts)
                    && packet_pts != frame_pts
                {
                    return Err(ObservationError::Invalid("manual frame timestamp mismatch"));
                }
                if let Some(trimming) = packet.trimming {
                    // FFmpeg discard_samples replaces, rather than adds to, its
                    // pending skip when the next frame has explicit side data.
                    skip_remaining = u64::from(trimming.skip);
                }
                let effective_skip = skip_remaining.min(u64::from(frame.physical_samples));
                skip_remaining -= effective_skip;
                configuration_changed |= packet
                    .other_side_data
                    .iter()
                    .any(|kind| matches!(kind.as_str(), "New Extradata" | "Param Change"));
                pairs.push(SourcePacketFrame {
                    packet,
                    frame,
                    effective_skip,
                    configuration_changed,
                });
            }
        }
    }
    if pending.is_some() {
        return Err(ObservationError::Invalid("terminal packet without a frame"));
    }
    Ok(ParsedAlacSource { stream, pairs })
}

#[derive(Deserialize)]
struct RawSource {
    streams: Vec<RawStream>,
    packets_and_frames: Vec<RawEvent>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Integer {
    Number(i64),
    Text(String),
}

impl Integer {
    fn signed(self) -> Result<i64, ObservationError> {
        match self {
            Self::Number(value) => Ok(value),
            Self::Text(value) => signed(&value),
        }
    }

    fn unsigned(self) -> Result<u64, ObservationError> {
        match self {
            Self::Number(value) => u64::try_from(value)
                .map_err(|_| ObservationError::Invalid("unsigned source integer")),
            Self::Text(value) => unsigned(&value),
        }
    }

    fn component(self) -> Result<u32, ObservationError> {
        let value = self.unsigned()?;
        if value == 0 || value > u64::from(i32::MAX.cast_unsigned()) {
            return Err(ObservationError::Invalid("positive source component"));
        }
        u32::try_from(value).map_err(|_| ObservationError::Overflow)
    }

    fn position(self) -> Result<Option<u64>, ObservationError> {
        if matches!(&self, Self::Text(value) if value == "N/A") {
            return Ok(None);
        }
        match self.signed()? {
            -1 => Ok(None),
            value if value >= 0 => Ok(Some(value.cast_unsigned())),
            _ => Err(ObservationError::Invalid("source packet position")),
        }
    }
}

fn timestamp(value: Option<Integer>) -> Result<PacketTimestamp, ObservationError> {
    value.map_or(Ok(PacketTimestamp::Unknown), |value| {
        value.signed().map(PacketTimestamp::from_raw)
    })
}

fn hashed(size: Integer, digest: &str) -> Result<HashedData, ObservationError> {
    Ok(HashedData {
        size: size.unsigned()?,
        digest: super::Sha256Digest::parse_hex(
            digest
                .strip_prefix("SHA256:")
                .ok_or(ObservationError::Invalid("source hash algorithm"))?,
        )?,
    })
}

#[derive(Deserialize)]
pub(super) struct RawStream {
    index: u32,
    codec_name: String,
    codec_type: String,
    sample_rate: Integer,
    channels: Integer,
    channel_layout: Option<String>,
    sample_fmt: String,
    bits_per_raw_sample: Integer,
    time_base: String,
    initial_padding: Option<Integer>,
    extradata_size: Integer,
    extradata_hash: String,
}

impl RawStream {
    pub(super) fn parse(self) -> Result<AlacStreamFacts, ObservationError> {
        if self.codec_name != "alac" || self.codec_type != "audio" || self.sample_fmt.is_empty() {
            return Err(ObservationError::Invalid("ALAC stream facts"));
        }
        Ok(AlacStreamFacts {
            index: self.index,
            time_base: TimeBase::parse(&self.time_base)?,
            sample_rate: self.sample_rate.component()?,
            channels: self.channels.component()?,
            channel_layout: self.channel_layout,
            sample_format: self.sample_fmt,
            bits_per_raw_sample: self.bits_per_raw_sample.component()?,
            initial_padding: self.initial_padding.map(Integer::unsigned).transpose()?,
            extradata: hashed(self.extradata_size, &self.extradata_hash)?,
        })
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub(super) enum RawEvent {
    #[serde(rename = "packet")]
    Packet(RawPacket),
    #[serde(rename = "frame")]
    Frame(RawFrame),
}

#[derive(Deserialize)]
struct RawSideData {
    side_data_type: String,
    skip_samples: Option<Integer>,
    discard_padding: Option<Integer>,
}

#[derive(Deserialize)]
pub(super) struct RawPacket {
    pub(super) stream_index: u32,
    codec_type: String,
    pts: Option<Integer>,
    dts: Option<Integer>,
    duration: Option<Integer>,
    size: Integer,
    pos: Option<Integer>,
    flags: String,
    data_hash: String,
    #[serde(default)]
    side_data_list: Vec<RawSideData>,
}

impl RawPacket {
    pub(super) fn parse(
        self,
        index: u32,
        side_limit: usize,
    ) -> Result<SourceAudioPacket, ObservationError> {
        if self.stream_index != index || self.codec_type != "audio" {
            return Err(ObservationError::Invalid("source packet stream"));
        }
        if self.side_data_list.len() > side_limit {
            return Err(ObservationError::Limit("source packet side data"));
        }
        let mut trimming = None;
        let mut other_side_data = Vec::new();
        for side in self.side_data_list {
            if side.side_data_type == "Skip Samples" {
                let skip = side
                    .skip_samples
                    .ok_or(ObservationError::Invalid("missing skip samples"))?
                    .unsigned()?;
                let discard = side
                    .discard_padding
                    .ok_or(ObservationError::Invalid("missing discard padding"))?
                    .unsigned()?;
                let value = SampleTrimming {
                    skip: u32::try_from(skip).map_err(|_| ObservationError::Overflow)?,
                    discard: u32::try_from(discard).map_err(|_| ObservationError::Overflow)?,
                };
                if trimming.replace(value).is_some() {
                    return Err(ObservationError::Invalid("duplicate sample trimming"));
                }
            } else {
                if side.side_data_type.is_empty() {
                    return Err(ObservationError::Invalid("empty packet side-data type"));
                }
                other_side_data.push(side.side_data_type);
            }
        }
        Ok(SourceAudioPacket {
            pts: timestamp(self.pts)?,
            dts: timestamp(self.dts)?,
            duration: self.duration.map(Integer::signed).transpose()?,
            position: self.pos.map(Integer::position).transpose()?.flatten(),
            payload: hashed(self.size, &self.data_hash)?,
            flags: PacketFlags::parse(&self.flags)?,
            trimming,
            other_side_data,
        })
    }
}

#[derive(Deserialize)]
pub(super) struct RawFrame {
    stream_index: u32,
    media_type: String,
    pts: Option<Integer>,
    pkt_pos: Option<Integer>,
    nb_samples: Integer,
    sample_fmt: String,
    channels: Integer,
    channel_layout: Option<String>,
    #[serde(default)]
    side_data_list: Vec<RawSideData>,
}

impl RawFrame {
    pub(super) fn parse(
        self,
        index: u32,
        side_limit: usize,
    ) -> Result<SourceAudioFrame, ObservationError> {
        if self.stream_index != index || self.media_type != "audio" || self.sample_fmt.is_empty() {
            return Err(ObservationError::Invalid("source frame facts"));
        }
        if self.side_data_list.len() > side_limit {
            return Err(ObservationError::Limit("source frame side data"));
        }
        if self
            .side_data_list
            .iter()
            .any(|side| side.side_data_type.is_empty())
        {
            return Err(ObservationError::Invalid("empty frame side-data type"));
        }
        Ok(SourceAudioFrame {
            pts: timestamp(self.pts)?,
            position: self.pkt_pos.map(Integer::position).transpose()?.flatten(),
            physical_samples: self.nb_samples.component()?,
            sample_format: self.sample_fmt,
            channels: self.channels.component()?,
            channel_layout: self.channel_layout,
            side_data_types: self
                .side_data_list
                .into_iter()
                .map(|side| side.side_data_type)
                .collect(),
        })
    }
}
