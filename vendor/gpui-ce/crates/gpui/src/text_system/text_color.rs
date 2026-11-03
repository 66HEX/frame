/// Eight-bit sRGB color stored in a raster cache key when a native raster path needs it.
pub type Rgba8 = palette::Srgba<u8>;

/// Converts a scene color to the bytes used by a raster cache key.
pub trait RasterColorExt {
    /// Clamps and rounds each channel to the nearest byte.
    fn quantize_raster_color(self) -> Rgba8;
}

impl RasterColorExt for crate::Rgba {
    fn quantize_raster_color(self) -> Rgba8 {
        fn channel(value: f32) -> u8 {
            (value.clamp(0.0, 1.0) * 255.0).round() as u8
        }

        Rgba8::new(
            channel(self.red),
            channel(self.green),
            channel(self.blue),
            channel(self.alpha),
        )
    }
}
