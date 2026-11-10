use super::*;
use std::io::Cursor;

const STATIC: &[u8] = include_bytes!("../../../tooling/ffmpeg/frame-preview-v1/probes/static.fpv");
const DYNAMIC: &[u8] =
    include_bytes!("../../../tooling/ffmpeg/frame-preview-v1/probes/dynamic.fpv");

struct Fragmented<'a> {
    bytes: &'a [u8],
    maximum: usize,
}

impl Read for Fragmented<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let length = out.len().min(self.maximum).min(self.bytes.len());
        out[..length].copy_from_slice(&self.bytes[..length]);
        self.bytes = &self.bytes[length..];
        Ok(length)
    }
}

#[test]
fn real_wire_records_survive_every_header_fragmentation() {
    let spec = FramedBgraSpec::new(4, 2).unwrap();
    for maximum in 1..=HEADER_BYTES + 1 {
        let mut input = Fragmented {
            bytes: STATIC,
            maximum,
        };
        for sequence in 0..2 {
            let frame = spec.read_frame(&mut input, sequence).unwrap().unwrap();
            assert_eq!(frame.pixels.len(), 32);
            assert_eq!(frame.metadata.sequence, sequence);
            assert_eq!(frame.metadata.pts, Some(i64::try_from(sequence).unwrap()));
            assert_eq!(frame.metadata.time_base, (1, 2));
            assert_eq!(frame.metadata.decoder_index, Some(sequence));
            assert_eq!(
                spec.pixel_aspect_ratio(frame.metadata).exact_ratio(),
                Some((3, 2))
            );
        }
        assert!(spec.read_frame(&mut input, 2).unwrap().is_none());
    }
}

#[test]
fn only_record_boundary_eof_is_success() {
    let spec = FramedBgraSpec::new(4, 2).unwrap();
    for end in 0..STATIC.len() {
        let mut input = Cursor::new(&STATIC[..end]);
        if end == 0 {
            assert!(spec.read_frame(&mut input, 0).unwrap().is_none());
        } else if end < 112 {
            assert_eq!(
                spec.read_frame(&mut input, 0).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof
            );
        } else {
            spec.read_frame(&mut input, 0).unwrap().unwrap();
            if end == 112 {
                assert!(spec.read_frame(&mut input, 1).unwrap().is_none());
            } else {
                assert_eq!(
                    spec.read_frame(&mut input, 1).unwrap_err().kind(),
                    io::ErrorKind::UnexpectedEof
                );
            }
        }
    }
}

#[test]
fn identical_pixels_carry_their_own_dynamic_sar() {
    let spec = FramedBgraSpec::new(4, 2).unwrap();
    let mut input = Cursor::new(DYNAMIC);
    let first_pixels = DYNAMIC[80..112].to_vec();
    for sequence in 0..36 {
        let frame = spec.read_frame(&mut input, sequence).unwrap().unwrap();
        assert_eq!(frame.pixels, first_pixels);
        let sar = if (12..24).contains(&sequence) { 2 } else { 1 };
        assert_eq!(
            frame.metadata.sample_aspect_ratio,
            SampleAspectRatio::new(sar, 1)
        );
        assert_eq!(
            frame
                .metadata
                .output_raster_aspect_ratio()
                .unwrap()
                .exact_ratio(),
            Some((4, 3))
        );
        assert_eq!(
            spec.pixel_aspect_ratio(frame.metadata).exact_ratio(),
            Some((u128::from(sar) * 2, 3))
        );
    }
    assert!(spec.read_frame(&mut input, 36).unwrap().is_none());
}

#[test]
fn invalid_headers_are_rejected_before_reading_pixels() {
    let spec = FramedBgraSpec::new(4, 2).unwrap();
    let original: [u8; HEADER_BYTES] = STATIC[..HEADER_BYTES].try_into().unwrap();
    for (offset, value) in [
        (0, 0),
        (7, b'2'),
        (8, 81),
        (10, 15),
        (12, 2),
        (16, 31),
        (24, 1),
        (40, 0),
        (44, 0),
        (48, 5),
        (52, 3),
        (56, 0),
        (60, 0),
        (64, 0),
        (68, 0),
    ] {
        let mut header = original;
        header[offset] = value;
        assert!(spec.parse_header(&header, 0).is_err(), "offset {offset}");
    }
    for offset in [16, 24, 40, 44, 48, 52, 56, 60, 64, 68] {
        let mut header = original;
        header[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(
            spec.parse_header(&header, 0).is_err(),
            "overflow at {offset}"
        );
    }
    assert!(spec.parse_header(&original, 1).is_err());
    assert!(FramedBgraSpec::new(0, 1).is_err());
    assert!(FramedBgraSpec::new(3841, 1).is_err());
}

#[test]
fn unknown_values_require_exact_sentinels_and_never_reuse_initial_sar() {
    let spec = FramedBgraSpec::new(4, 2).unwrap();
    let mut header: [u8; HEADER_BYTES] = STATIC[..HEADER_BYTES].try_into().unwrap();
    header[10] = 0;
    assert!(spec.parse_header(&header, 0).is_err());
    header[32..40].copy_from_slice(&i64::MIN.to_le_bytes());
    header[40..48].fill(0);
    header[72..80].copy_from_slice(&u64::MAX.to_le_bytes());
    header[64..68].copy_from_slice(&0_u32.to_le_bytes());
    header[68..72].copy_from_slice(&1_u32.to_le_bytes());
    let unknown = spec.parse_header(&header, 0).unwrap();
    assert_eq!(unknown.pts, None);
    assert_eq!(unknown.time_base, (0, 0));
    assert_eq!(unknown.decoder_index, None);
    assert_eq!(unknown.sample_aspect_ratio, None);
    assert_eq!(spec.pixel_aspect_ratio(unknown), DisplayRatio::ONE);
    let mut odd_raster = unknown;
    odd_raster.output_width = 400;
    odd_raster.output_height = 4;
    assert_eq!(
        FramedBgraSpec::new(240, 2)
            .unwrap()
            .pixel_aspect_ratio(odd_raster)
            .exact_ratio(),
        Some((5, 6))
    );
    header[40..44].copy_from_slice(&1_u32.to_le_bytes());
    header[44..48].copy_from_slice(&2_u32.to_le_bytes());
    assert!(spec.parse_header(&header, 0).is_err());
    header[40..48].fill(0);
    for flag in [1, 2, 4] {
        header[10] = flag;
        assert!(spec.parse_header(&header, 0).is_err());
    }
}

#[test]
fn malformed_stream_never_resynchronizes_at_a_later_magic_marker() {
    let spec = FramedBgraSpec::new(4, 2).unwrap();
    let mut input = vec![0; HEADER_BYTES];
    input.extend_from_slice(STATIC);
    assert!(spec.read_frame(&mut Cursor::new(input), 0).is_err());
}
