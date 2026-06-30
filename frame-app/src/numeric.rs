#![expect(
    clippy::redundant_pub_crate,
    reason = "Numeric helpers live in a private module but must be visible to sibling modules."
)]

#[must_use]
pub(super) const fn f64_to_f32(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "GPUI layout APIs use f32 while layout math is accumulated in f64."
    )]
    {
        value as f32
    }
}

#[must_use]
pub(super) const fn u32_to_f32(value: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "Preview dimensions are bounded by media/UI limits before entering f32 layout math."
    )]
    {
        value as f32
    }
}

#[must_use]
pub(super) const fn usize_to_f32(value: usize) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "UI collection counts are bounded by visible controls before entering f32 layout math."
    )]
    {
        value as f32
    }
}

#[must_use]
pub(super) const fn u64_to_f64(value: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "Formatting and media timestamps tolerate sub-unit precision loss at these magnitudes."
    )]
    {
        value as f64
    }
}

#[must_use]
pub(super) const fn unit_f64_to_f32(value: f64) -> f32 {
    f64_to_f32(value.clamp(0.0, 1.0))
}

#[must_use]
pub(super) fn rounded_f64_to_u8(value: f64) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "The channel value is rounded and clamped to the u8 range before casting."
    )]
    {
        value.round().clamp(0.0, f64::from(u8::MAX)) as u8
    }
}

#[must_use]
pub(super) fn rounded_f64_to_u32(value: f64) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "The value is rounded and clamped to the u32 range before casting."
    )]
    {
        value.round().clamp(0.0, f64::from(u32::MAX)) as u32
    }
}

#[must_use]
pub(super) fn f64_to_u32(value: f64) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "The value is clamped to the u32 range before casting."
    )]
    {
        value.clamp(0.0, f64::from(u32::MAX)) as u32
    }
}

#[must_use]
pub(super) const fn rounded_f64_to_u64(value: f64) -> u64 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "The value is rounded and clamped to the u64 range before casting."
    )]
    {
        value.round().clamp(0.0, u64::MAX as f64) as u64
    }
}

#[must_use]
pub(super) const fn f64_to_u64(value: f64) -> u64 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "The value is clamped to the u64 range before casting."
    )]
    {
        value.clamp(0.0, u64::MAX as f64) as u64
    }
}

#[must_use]
pub(super) fn u32_to_u8(value: u32) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

#[must_use]
pub(super) fn u32_to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
