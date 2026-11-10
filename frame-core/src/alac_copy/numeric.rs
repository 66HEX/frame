use super::ObservationError;

/// A positive, reduced time base with the bounds of an `FFmpeg` `AVRational`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeBase {
    numerator: u32,
    denominator: u32,
}

impl TimeBase {
    /// Creates a time base without accepting zero or overflowing signed fields.
    ///
    /// # Errors
    /// Returns an error when either component is outside `1..=i32::MAX`.
    pub const fn new(numerator: u32, denominator: u32) -> Result<Self, ObservationError> {
        if numerator == 0
            || denominator == 0
            || numerator > i32::MAX.cast_unsigned()
            || denominator > i32::MAX.cast_unsigned()
        {
            return Err(ObservationError::Invalid("time base"));
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// Parses the exact integer `numerator/denominator` representation.
    ///
    /// # Errors
    /// Returns an error for malformed, zero, negative or oversized components.
    pub fn parse(value: &str) -> Result<Self, ObservationError> {
        let (numerator, denominator) = value
            .split_once('/')
            .ok_or(ObservationError::Invalid("time base"))?;
        Self::new(unsigned(numerator)?, unsigned(denominator)?)
    }

    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.denominator
    }

    /// Rescales a known timestamp, rounding half away from zero like `NEAR_INF`.
    ///
    /// # Errors
    /// Returns an error for `AV_NOPTS_VALUE` or an unrepresentable result.
    pub fn rescale(self, value: i64, destination: Self) -> Result<i64, ObservationError> {
        if value == i64::MIN {
            return Err(ObservationError::Invalid("unknown timestamp"));
        }
        let numerator = i128::from(value)
            .checked_mul(i128::from(self.numerator))
            .and_then(|value| value.checked_mul(i128::from(destination.denominator)))
            .ok_or(ObservationError::Overflow)?;
        let denominator = i128::from(self.denominator) * i128::from(destination.numerator);
        let magnitude = numerator.abs();
        let quotient = magnitude / denominator;
        let remainder = magnitude % denominator;
        let rounded = quotient + i128::from(remainder * 2 >= denominator);
        let signed = if numerator < 0 { -rounded } else { rounded };
        let result = i64::try_from(signed).map_err(|_| ObservationError::Overflow)?;
        if result == i64::MIN {
            return Err(ObservationError::Overflow);
        }
        Ok(result)
    }
}

const fn gcd(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

pub(super) fn unsigned<T: std::str::FromStr>(value: &str) -> Result<T, ObservationError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ObservationError::Invalid("unsigned integer"));
    }
    value
        .parse()
        .map_err(|_| ObservationError::Invalid("unsigned integer"))
}

pub(super) fn signed(value: &str) -> Result<i64, ObservationError> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ObservationError::Invalid("signed integer"));
    }
    value
        .parse()
        .map_err(|_| ObservationError::Invalid("signed integer"))
}

/// A missing timestamp is distinct from every ordinary numeric timestamp.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PacketTimestamp {
    Unknown,
    Known(i64),
}

impl PacketTimestamp {
    #[must_use]
    pub const fn from_raw(value: i64) -> Self {
        if value == i64::MIN {
            Self::Unknown
        } else {
            Self::Known(value)
        }
    }
}

/// Binary SHA-256, so textual case cannot change a packet association.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Parses exactly 64 hexadecimal digits, without an algorithm prefix.
    ///
    /// # Errors
    /// Returns an error for any missing, extra or non-hexadecimal byte.
    pub fn parse_hex(value: &str) -> Result<Self, ObservationError> {
        if value.len() != 64 {
            return Err(ObservationError::Invalid("SHA-256 digest"));
        }
        let mut digest = [0; 32];
        for (destination, pair) in digest.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
            *destination = hex_digit(pair[0])? * 16 + hex_digit(pair[1])?;
        }
        Ok(Self(digest))
    }

    #[must_use]
    pub const fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

const fn hex_digit(value: u8) -> Result<u8, ObservationError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(ObservationError::Invalid("SHA-256 digest")),
    }
}

/// The three packet flags emitted by the pinned `FFprobe` JSON protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PacketFlags {
    pub key: bool,
    pub discard: bool,
    pub corrupt: bool,
}

impl PacketFlags {
    /// Parses `K`, `D`, `C` or `_` in each of their respective positions.
    ///
    /// # Errors
    /// Returns an error for omitted, partial, reordered or unknown flags.
    pub fn parse(value: &str) -> Result<Self, ObservationError> {
        let [key, discard, corrupt] = value.as_bytes() else {
            return Err(ObservationError::Invalid("packet flags"));
        };
        if !matches!(key, b'K' | b'_')
            || !matches!(discard, b'D' | b'_')
            || !matches!(corrupt, b'C' | b'_')
        {
            return Err(ObservationError::Invalid("packet flags"));
        }
        Ok(Self {
            key: *key == b'K',
            discard: *discard == b'D',
            corrupt: *corrupt == b'C',
        })
    }
}
