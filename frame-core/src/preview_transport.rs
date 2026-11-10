//! Strict reader for Frame's opt-in, packet-bound `FFmpeg` preview transport.
//!
//! A record is one 80-byte little-endian header followed by its BGRA pixels.
//! Never search for a later magic marker or substitute out-of-band metadata.

use crate::aspect::{DisplayRatio, SampleAspectRatio};
use std::io::{self, Read};

pub const HEADER_BYTES: usize = 80;
const MAX_TEXTURE_DIMENSION: u32 = 3840;
const MAX_OUTPUT_DIMENSION: u32 = 30_720;

/// The dimensions are prepared before spawning the child, not read from it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramedBgraSpec {
    width: u32,
    height: u32,
    payload_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameMetadata {
    pub sequence: u64,
    pub pts: Option<i64>,
    pub time_base: (u32, u32),
    pub decoder_index: Option<u64>,
    pub output_width: u32,
    pub output_height: u32,
    pub sample_aspect_ratio: Option<SampleAspectRatio>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FramedBgraFrame {
    pub metadata: FrameMetadata,
    pub pixels: Vec<u8>,
}

impl FramedBgraSpec {
    /// # Errors
    /// Rejects zero, oversized or overflowing texture dimensions.
    pub fn new(width: u32, height: u32) -> io::Result<Self> {
        if !(1..=MAX_TEXTURE_DIMENSION).contains(&width)
            || !(1..=MAX_TEXTURE_DIMENSION).contains(&height)
        {
            return Err(invalid(
                "preview texture dimensions are outside the supported range",
            ));
        }
        let payload_bytes = usize::try_from(u64::from(width) * u64::from(height) * 4)
            .map_err(|_| invalid("preview payload size overflow"))?;
        Ok(Self {
            width,
            height,
            payload_bytes,
        })
    }

    #[must_use]
    pub const fn payload_bytes(self) -> usize {
        self.payload_bytes
    }

    /// Reads one record, returning `None` only at a clean record boundary.
    ///
    /// # Errors
    /// Rejects malformed headers, unexpected sequence/dimensions and any
    /// truncated header or payload. The caller must stop this child on error.
    pub fn read_frame(
        self,
        reader: &mut (impl Read + ?Sized),
        sequence: u64,
    ) -> io::Result<Option<FramedBgraFrame>> {
        let mut header = [0; HEADER_BYTES];
        loop {
            match reader.read(&mut header[..1]) {
                Ok(0) => return Ok(None),
                Ok(_) => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        reader.read_exact(&mut header[1..])?;
        let metadata = self.parse_header(&header, sequence)?;
        let mut pixels = vec![0; self.payload_bytes];
        reader.read_exact(&mut pixels)?;
        Ok(Some(FramedBgraFrame { metadata, pixels }))
    }

    /// Validates a complete fixed-size header without allocating pixel storage.
    ///
    /// # Errors
    /// Rejects any unsupported version, flags, format, sizes or sentinels.
    pub fn parse_header(
        self,
        header: &[u8; HEADER_BYTES],
        sequence: u64,
    ) -> io::Result<FrameMetadata> {
        if &header[..8] != b"FRMPV001"
            || u16::from_le_bytes([header[8], header[9]]) != 80
            || u32_at(header, 12) != 1
        {
            return Err(invalid("unsupported preview frame header"));
        }
        let flags = u16::from_le_bytes([header[10], header[11]]);
        if flags & !7 != 0
            || u64_at(header, 16) != u64::try_from(self.payload_bytes).unwrap_or(u64::MAX)
            || u64_at(header, 24) != sequence
            || u32_at(header, 48) != self.width
            || u32_at(header, 52) != self.height
        {
            return Err(invalid("preview frame does not match the prepared stream"));
        }
        let time_base = (u32_at(header, 40), u32_at(header, 44));
        let output_width = u32_at(header, 56);
        let output_height = u32_at(header, 60);
        let valid_time_base = if flags & 2 != 0 {
            valid_rational(time_base.0, time_base.1)
        } else {
            time_base == (0, 0)
        };
        if !valid_time_base
            || !(1..=MAX_OUTPUT_DIMENSION).contains(&output_width)
            || !(1..=MAX_OUTPUT_DIMENSION).contains(&output_height)
        {
            return Err(invalid("invalid preview frame geometry or time base"));
        }
        let pts = i64::from_le_bytes(u64_at(header, 32).to_le_bytes());
        let decoder_index = u64_at(header, 72);
        if (flags & 2 != 0) == (pts == i64::MIN) || (flags & 4 != 0) == (decoder_index == u64::MAX)
        {
            return Err(invalid(
                "inconsistent preview frame timestamp or index sentinel",
            ));
        }
        let sar = (u32_at(header, 64), u32_at(header, 68));
        let sample_aspect_ratio = if flags & 1 != 0 {
            Some(
                SampleAspectRatio::new(sar.0, sar.1)
                    .ok_or_else(|| invalid("invalid known preview sample aspect ratio"))?,
            )
        } else if sar == (0, 1) {
            None
        } else {
            return Err(invalid("invalid unknown preview sample aspect ratio"));
        };
        Ok(FrameMetadata {
            sequence,
            pts: (flags & 2 != 0).then_some(pts),
            time_base,
            decoder_index: (flags & 4 != 0).then_some(decoder_index),
            output_width,
            output_height,
            sample_aspect_ratio,
        })
    }

    /// Texture PAR derived from this frame's output raster and known SAR.
    /// Unknown SAR retains the existing output-raster presentation policy;
    /// the metadata still records that the source SAR is unknown.
    #[must_use]
    pub fn pixel_aspect_ratio(self, metadata: FrameMetadata) -> DisplayRatio {
        let (numerator, denominator) = metadata
            .sample_aspect_ratio
            .map_or((1, 1), |sar| (sar.numerator(), sar.denominator()));
        DisplayRatio::new(
            u128::from(metadata.output_width) * u128::from(numerator) * u128::from(self.height),
            u128::from(metadata.output_height) * u128::from(denominator) * u128::from(self.width),
        )
        .unwrap_or_default()
    }
}

impl FrameMetadata {
    #[must_use]
    pub const fn output_raster_aspect_ratio(self) -> Option<DisplayRatio> {
        DisplayRatio::new(self.output_width as u128, self.output_height as u128)
    }
}

const fn valid_rational(numerator: u32, denominator: u32) -> bool {
    numerator > 0
        && denominator > 0
        && numerator <= i32::MAX as u32
        && denominator <= i32::MAX as u32
}

fn u32_at(bytes: &[u8; HEADER_BYTES], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn u64_at(bytes: &[u8; HEADER_BYTES], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
#[path = "preview_transport/tests.rs"]
mod tests;
