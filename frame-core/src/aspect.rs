//! Canonical source pixel aspect and finite derived display ratios.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RatioFields")]
pub struct SampleAspectRatio {
    numerator: u32,
    denominator: u32,
}

#[derive(Deserialize)]
struct RatioFields {
    numerator: u32,
    denominator: u32,
}

impl TryFrom<RatioFields> for SampleAspectRatio {
    type Error = &'static str;
    fn try_from(value: RatioFields) -> Result<Self, Self::Error> {
        Self::new(value.numerator, value.denominator).ok_or("Invalid sample aspect ratio")
    }
}

impl SampleAspectRatio {
    #[must_use]
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if numerator == 0
            || denominator == 0
            || numerator > i32::MAX.unsigned_abs()
            || denominator > i32::MAX.unsigned_abs()
        {
            return None;
        }
        let divisor = u32::try_from(gcd(u128::from(numerator), u128::from(denominator))).ok()?;
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let (num, den) = value.split_once(':')?;
        if !num.bytes().all(|v| v.is_ascii_digit()) || !den.bytes().all(|v| v.is_ascii_digit()) {
            return None;
        }
        Self::new(num.parse().ok()?, den.parse().ok()?)
    }
    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.numerator
    }
    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
    #[must_use]
    pub const fn inverted(self) -> Self {
        Self {
            numerator: self.denominator,
            denominator: self.numerator,
        }
    }
}

pub(crate) fn deserialize_optional_sample_aspect_ratio<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<SampleAspectRatio>, D::Error> {
    Ok(Option::<serde_json::Value>::deserialize(deserializer)?
        .and_then(|value| serde_json::from_value(value).ok()))
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RatioRepresentation {
    Exact(u128, u128),
    Finite(u64),
}

/// A positive display ratio, retaining exact arithmetic whenever it fits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DisplayRatio {
    representation: RatioRepresentation,
}

impl Default for DisplayRatio {
    fn default() -> Self {
        Self::ONE
    }
}

impl DisplayRatio {
    pub const ONE: Self = Self {
        representation: RatioRepresentation::Exact(1, 1),
    };
    #[must_use]
    pub const fn new(numerator: u128, denominator: u128) -> Option<Self> {
        if numerator == 0 || denominator == 0 {
            return None;
        }
        let divisor = gcd(numerator, denominator);
        Some(Self {
            representation: RatioRepresentation::Exact(numerator / divisor, denominator / divisor),
        })
    }
    #[must_use]
    pub const fn exact_ratio(self) -> Option<(u128, u128)> {
        match self.representation {
            RatioRepresentation::Exact(n, d) => Some((n, d)),
            RatioRepresentation::Finite(_) => None,
        }
    }
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "Presentation ratios use finite f64 only at the display boundary or after checked exact arithmetic overflows."
    )]
    pub fn as_f64(self) -> f64 {
        match self.representation {
            RatioRepresentation::Exact(n, d) => n as f64 / d as f64,
            RatioRepresentation::Finite(bits) => f64::from_bits(bits),
        }
    }
    /// Returns `None` if the result or its inverse cannot remain finite.
    #[must_use]
    pub fn multiplied_by(self, numerator: u128, denominator: u128) -> Option<Self> {
        if numerator == 0 || denominator == 0 {
            return None;
        }
        if let RatioRepresentation::Exact(n, d) = self.representation {
            let a = gcd(n, denominator);
            let b = gcd(numerator, d);
            if let Some((n, d)) = (n / a)
                .checked_mul(numerator / b)
                .zip((d / b).checked_mul(denominator / a))
            {
                return Self::new(n, d);
            }
        }
        // Overflow of a derived ratio does not turn valid source SAR into Unknown.
        let factor = Self::new(numerator, denominator)?.as_f64();
        let value = self.as_f64() * factor;
        (value.is_finite() && value > 0.0 && value.recip().is_finite()).then_some(Self {
            representation: RatioRepresentation::Finite(value.to_bits()),
        })
    }
    #[must_use]
    pub fn inverted(self) -> Self {
        match self.representation {
            RatioRepresentation::Exact(n, d) => Self {
                representation: RatioRepresentation::Exact(d, n),
            },
            RatioRepresentation::Finite(bits) => Self {
                representation: RatioRepresentation::Finite((1.0 / f64::from_bits(bits)).to_bits()),
            },
        }
    }
}

const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let rem = a % b;
        a = b;
        b = rem;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_source_ratio_preserves_unknown_and_inverts_with_source_orientation() {
        for invalid in [
            "N/A",
            "0:1",
            "1:0",
            "-1:2",
            "1:-2",
            "1/2",
            "1:2:3",
            "4294967296:1",
            "2147483648:1",
        ] {
            assert!(SampleAspectRatio::parse(invalid).is_none());
        }
        assert_eq!(
            SampleAspectRatio::parse("2:2"),
            SampleAspectRatio::new(1, 1)
        );
        for (rotation, width, height, ratio) in [
            (0, 320, 180, (2, 1)),
            (90, 180, 320, (1, 2)),
            (180, 320, 180, (2, 1)),
            (270, 180, 320, (1, 2)),
            (-90, 180, 320, (1, 2)),
        ] {
            let probe=crate::probe::parse_ffprobe_stdout("video.mkv",serde_json::json!({"format":{},"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":320,"height":180,"sample_aspect_ratio":"4:2","side_data_list":[{"rotation":rotation}]}]}).to_string()).unwrap();
            assert_eq!((probe.width, probe.height), (Some(width), Some(height)));
            assert_eq!(
                probe.sample_aspect_ratio,
                SampleAspectRatio::new(ratio.0, ratio.1)
            );
            let roundtrip: crate::types::ProbeMetadata =
                serde_json::from_value(serde_json::to_value(&probe).unwrap()).unwrap();
            assert_eq!(roundtrip.sample_aspect_ratio, probe.sample_aspect_ratio);
        }
        let mut old = serde_json::to_value(crate::types::ProbeMetadata::default()).unwrap();
        assert!(old.get("sampleAspectRatio").is_none());
        old["sampleAspectRatio"] = serde_json::json!({"numerator":1,"denominator":0});
        assert!(
            serde_json::from_value::<crate::types::ProbeMetadata>(old)
                .unwrap()
                .sample_aspect_ratio
                .is_none()
        );
    }
    #[test]
    fn repeated_multiplication_never_creates_a_nonfinite_inverse() {
        let mut ratio = DisplayRatio::ONE;
        let mut rejected = false;
        for _ in 0..12 {
            let Some(next) = ratio.multiplied_by(1, u128::MAX) else {
                rejected = true;
                break;
            };
            ratio = next;
            assert!(ratio.as_f64().is_finite() && ratio.as_f64() > 0.0);
            assert!(ratio.inverted().as_f64().is_finite());
        }
        assert!(rejected);
    }

    #[test]
    fn derived_overflow_keeps_finite_positive_display_geometry() {
        let ratio = DisplayRatio::new(u128::MAX, 1)
            .unwrap()
            .multiplied_by(u128::MAX, 1)
            .unwrap();
        assert!(ratio.exact_ratio().is_none());
        assert!(ratio.as_f64().is_finite() && ratio.as_f64() > 0.0);
        assert!(ratio.inverted().as_f64().is_finite() && ratio.inverted().as_f64() > 0.0);
        assert_eq!(
            DisplayRatio::new(u128::MAX, u128::MAX).unwrap(),
            DisplayRatio::ONE
        );
    }
}
