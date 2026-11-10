use super::numeric::{signed, unsigned};
use super::{ObservationError, PacketTimestamp, Sha256Digest, TimeBase};
use std::collections::BTreeMap;

/// Structural capture limits, independent of a helper's execution deadline.
#[derive(Clone, Copy, Debug)]
pub struct ManifestLimits {
    pub bytes: usize,
    pub packets: usize,
    pub streams: usize,
    pub line_bytes: usize,
    pub side_data_per_packet: usize,
}

impl Default for ManifestLimits {
    fn default() -> Self {
        Self {
            bytes: 32 * 1024 * 1024,
            packets: 1_000_000,
            streams: 256,
            line_bytes: 16 * 1024,
            side_data_per_packet: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HashedData {
    pub size: u64,
    pub digest: Sha256Digest,
}

/// A global output stream, which is not necessarily an audio output ordinal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestStream {
    pub index: u32,
    pub time_base: TimeBase,
    pub media_type: String,
    pub codec: String,
    pub extradata: Option<HashedData>,
    pub sample_rate: Option<u32>,
    pub channel_layout: Option<String>,
    pub dimensions: Option<(u32, u32)>,
    pub sample_aspect_ratio: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestPacket {
    pub stream_index: u32,
    pub dts: PacketTimestamp,
    pub pts: PacketTimestamp,
    pub duration: i64,
    pub payload: HashedData,
    /// Framehash identifies side-data bytes but does not expose their types.
    pub side_data: Vec<HashedData>,
}

/// Parsed syntax only: no claim that the producing process reached EOF.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedCopyManifest {
    pub streams: Vec<ManifestStream>,
    /// Packet order is retained, including duplicate output mappings.
    pub packets: Vec<ManifestPacket>,
}

/// Parses the explicit SHA-256, version-2 framehash protocol.
///
/// A valid prefix ending at a packet boundary is syntactically indistinguishable
/// from a complete file. The caller must separately establish child completion,
/// untruncated capture and unchanged request/source/runtime identity.
///
/// # Errors
/// Returns an error for partial output, unsupported or inconsistent headers,
/// malformed numbers/hashes/side data, undeclared streams or exceeded limits.
pub fn parse_copy_manifest(
    bytes: &[u8],
    limits: ManifestLimits,
) -> Result<ParsedCopyManifest, ObservationError> {
    if bytes.len() > limits.bytes {
        return Err(ObservationError::Limit("capture bytes"));
    }
    if !bytes.ends_with(b"\n") {
        return Err(ObservationError::Invalid("unterminated framehash output"));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| ObservationError::Invalid("framehash UTF-8"))?;
    let mut lines = text.split_terminator('\n').map(|line| {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.len() > limits.line_bytes {
            Err(ObservationError::Limit("line bytes"))
        } else if line.chars().any(char::is_control) {
            Err(ObservationError::Invalid("framehash control character"))
        } else {
            Ok(line)
        }
    });
    for expected in ["#format: frame checksums", "#version: 2", "#hash: SHA256"] {
        if lines.next().transpose()? != Some(expected) {
            return Err(ObservationError::Invalid("framehash protocol header"));
        }
    }
    let mut headers = Headers::default();
    let mut found_columns = false;
    for line in lines.by_ref() {
        let line = line?;
        if line.starts_with("#stream#") {
            if line
                .split(',')
                .map(str::trim)
                .ne(["#stream#", "dts", "pts", "duration", "size", "hash"])
            {
                return Err(ObservationError::Invalid("packet column header"));
            }
            found_columns = true;
            break;
        }
        headers.add(line, limits)?;
    }
    if !found_columns {
        return Err(ObservationError::Invalid("missing packet column header"));
    }
    let streams = headers.finish()?;
    let mut packets = Vec::new();
    for line in lines {
        if packets.len() >= limits.packets {
            return Err(ObservationError::Limit("packet count"));
        }
        let packet = parse_packet(line?, limits.side_data_per_packet)?;
        if usize::try_from(packet.stream_index).map_or(true, |index| index >= streams.len()) {
            return Err(ObservationError::Invalid("undeclared packet stream"));
        }
        packets.push(packet);
    }
    Ok(ParsedCopyManifest { streams, packets })
}

#[derive(Default)]
struct Headers {
    streams: BTreeMap<u32, StreamFields>,
    software: Option<String>,
}

#[derive(Default)]
struct StreamFields {
    values: BTreeMap<String, String>,
    extradata: Option<HashedData>,
}

impl Headers {
    fn stream(
        &mut self,
        index: u32,
        limits: ManifestLimits,
    ) -> Result<&mut StreamFields, ObservationError> {
        if usize::try_from(index).map_or(true, |index| index >= limits.streams) {
            return Err(ObservationError::Limit("stream count"));
        }
        Ok(self.streams.entry(index).or_default())
    }

    fn add(&mut self, line: &str, limits: ManifestLimits) -> Result<(), ObservationError> {
        if let Some(value) = line.strip_prefix("#software: ") {
            if value.is_empty() || self.software.replace(value.to_string()).is_some() {
                return Err(ObservationError::Invalid("software header"));
            }
            return Ok(());
        }
        if let Some(value) = line.strip_prefix("#extradata ") {
            let fields: Vec<_> = value.split(',').map(str::trim).collect();
            let [index, size, digest] = fields.as_slice() else {
                return Err(ObservationError::Invalid("extradata header"));
            };
            let data = HashedData {
                size: unsigned(size)?,
                digest: Sha256Digest::parse_hex(digest)?,
            };
            if self
                .stream(unsigned(index)?, limits)?
                .extradata
                .replace(data)
                .is_some()
            {
                return Err(ObservationError::Invalid("duplicate extradata header"));
            }
            return Ok(());
        }
        let (key, rest) = line
            .strip_prefix('#')
            .and_then(|line| line.split_once(' '))
            .ok_or(ObservationError::Invalid("stream header"))?;
        if !matches!(
            key,
            "tb" | "media_type"
                | "codec_id"
                | "sample_rate"
                | "channel_layout_name"
                | "dimensions"
                | "sar"
        ) {
            return Err(ObservationError::Invalid("unknown stream header"));
        }
        let (index, value) = rest
            .split_once(": ")
            .ok_or(ObservationError::Invalid("stream header separator"))?;
        if value.is_empty()
            || self
                .stream(unsigned(index)?, limits)?
                .values
                .insert(key.to_string(), value.to_string())
                .is_some()
        {
            return Err(ObservationError::Invalid("empty or duplicate stream field"));
        }
        Ok(())
    }

    fn finish(self) -> Result<Vec<ManifestStream>, ObservationError> {
        if self.streams.is_empty() {
            return Err(ObservationError::Invalid("missing mapped streams"));
        }
        self.streams
            .into_iter()
            .enumerate()
            .map(|(expected, (index, fields))| {
                if usize::try_from(index).ok() != Some(expected) {
                    return Err(ObservationError::Invalid(
                        "noncontiguous output stream indices",
                    ));
                }
                fields.finish(index)
            })
            .collect()
    }
}

impl StreamFields {
    fn take(&mut self, key: &str) -> Result<String, ObservationError> {
        self.values
            .remove(key)
            .ok_or(ObservationError::Invalid("missing stream field"))
    }

    fn finish(mut self, index: u32) -> Result<ManifestStream, ObservationError> {
        let time_base = TimeBase::parse(&self.take("tb")?)?;
        let media_type = self.take("media_type")?;
        let codec = self.take("codec_id")?;
        if !codec
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(ObservationError::Invalid("codec identifier"));
        }
        let mut stream = ManifestStream {
            index,
            time_base,
            media_type,
            codec,
            extradata: self.extradata,
            sample_rate: None,
            channel_layout: None,
            dimensions: None,
            sample_aspect_ratio: None,
        };
        match stream.media_type.as_str() {
            "audio" => {
                let rate = positive_component(&self.take("sample_rate")?)?;
                stream.sample_rate = Some(rate);
                stream.channel_layout = Some(self.take("channel_layout_name")?);
            }
            "video" => {
                let dimensions = self.take("dimensions")?;
                let (width, height) = dimensions
                    .split_once('x')
                    .ok_or(ObservationError::Invalid("video dimensions"))?;
                stream.dimensions = Some((positive_component(width)?, positive_component(height)?));
                let sar = self.take("sar")?;
                let (num, den) = sar
                    .split_once('/')
                    .ok_or(ObservationError::Invalid("video sample aspect ratio"))?;
                let numerator: u32 = unsigned(num)?;
                if numerator > i32::MAX.cast_unsigned() {
                    return Err(ObservationError::Invalid("video sample aspect ratio"));
                }
                stream.sample_aspect_ratio = Some((numerator, positive_component(den)?));
            }
            "subtitle" | "data" | "attachment" => {}
            _ => return Err(ObservationError::Invalid("media type")),
        }
        if !self.values.is_empty() {
            return Err(ObservationError::Invalid("inapplicable stream field"));
        }
        Ok(stream)
    }
}

fn positive_component(value: &str) -> Result<u32, ObservationError> {
    let number = unsigned(value)?;
    if number == 0 || number > i32::MAX.cast_unsigned() {
        return Err(ObservationError::Invalid("positive stream component"));
    }
    Ok(number)
}

fn parse_packet(line: &str, side_limit: usize) -> Result<ManifestPacket, ObservationError> {
    let fields: Vec<_> = line.split(',').map(str::trim).collect();
    let [index, dts, pts, duration, size, digest, rest @ ..] = fields.as_slice() else {
        return Err(ObservationError::Invalid("packet fields"));
    };
    let side_data = if rest.is_empty() {
        Vec::new()
    } else {
        let count: usize = unsigned(
            rest[0]
                .strip_prefix("S=")
                .ok_or(ObservationError::Invalid("side-data count"))?,
        )?;
        if count > side_limit {
            return Err(ObservationError::Limit("packet side data"));
        }
        if count == 0 || count.checked_mul(2).and_then(|n| n.checked_add(1)) != Some(rest.len()) {
            return Err(ObservationError::Invalid("side-data fields"));
        }
        rest[1..]
            .chunks_exact(2)
            .map(|fields| {
                Ok(HashedData {
                    size: unsigned(fields[0])?,
                    digest: Sha256Digest::parse_hex(fields[1])?,
                })
            })
            .collect::<Result<_, ObservationError>>()?
    };
    Ok(ManifestPacket {
        stream_index: unsigned(index)?,
        dts: PacketTimestamp::from_raw(signed(dts)?),
        pts: PacketTimestamp::from_raw(signed(pts)?),
        duration: signed(duration)?,
        payload: HashedData {
            size: unsigned(size)?,
            digest: Sha256Digest::parse_hex(digest)?,
        },
        side_data,
    })
}
