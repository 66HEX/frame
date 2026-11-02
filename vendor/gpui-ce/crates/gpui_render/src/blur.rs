pub use crate::shaders::{
    blur::BlurUniforms,
    common::{
        BlurCompositeClip as ShaderBlurCompositeClip, Bounds as ShaderBounds,
        Corners as ShaderCorners, DownsampleMode,
    },
};
use gpui::{Bounds, Corners, ScaledPixels};
use wgsl_rs::std::vec2f;

pub const DOWNSAMPLE_FACTOR: u32 = 2;

/// Texture dimension for the downsampled blur passes; allocations must agree.
pub fn downsampled_dimension(dimension: u32) -> u32 {
    dimension.div_ceil(DOWNSAMPLE_FACTOR).max(1)
}
const RADIUS_TO_STANDARD_DEVIATION: f32 = 0.5;
pub const GAUSSIAN_CUTOFF_STANDARD_DEVIATIONS: f32 = 3.0;
pub const MAX_GAUSSIAN_SAMPLES_PER_SIDE: u32 = 32;

#[derive(Clone, Copy)]
pub enum BlurAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
pub enum FilterCompositeClip {
    RoundedBounds,
    ContentShape,
}

#[derive(Clone, Copy)]
pub struct FilterCompositeParameters {
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: Bounds<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub corner_smoothing: f32,
    pub blur_radius: f32,
    pub opacity: f32,
    pub clip: FilterCompositeClip,
}

#[derive(Clone, Copy)]
pub struct BlurKernel {
    pub standard_deviation: f32,
    pub sample_count: u32,
    pub sample_step: f32,
}

impl BlurKernel {
    pub fn for_radius(blur_radius: f32) -> Option<Self> {
        let standard_deviation = (blur_radius * RADIUS_TO_STANDARD_DEVIATION).max(0.0);
        if standard_deviation <= 0.0 {
            return None;
        }

        let ideal_sample_count = (GAUSSIAN_CUTOFF_STANDARD_DEVIATIONS * standard_deviation).ceil();
        let sample_count = (ideal_sample_count as u32).clamp(1, MAX_GAUSSIAN_SAMPLES_PER_SIDE);
        Some(Self {
            standard_deviation,
            sample_count,
            sample_step: (ideal_sample_count / sample_count as f32).max(1.0),
        })
    }
}

impl BlurUniforms {
    pub fn copy(size: [f32; 2]) -> Self {
        with_texture_sizes(empty_uniforms(DownsampleMode::Copy), size, size)
    }

    pub fn downsample(source_size: [f32; 2], target_size: [f32; 2]) -> Self {
        with_texture_sizes(
            empty_uniforms(DownsampleMode::HalfResolution),
            source_size,
            target_size,
        )
    }

    pub fn gaussian(axis: BlurAxis, texture_size: [f32; 2], kernel: BlurKernel) -> Self {
        let direction = match axis {
            BlurAxis::Horizontal => vec2f(1.0 / texture_size[0], 0.0),
            BlurAxis::Vertical => vec2f(0.0, 1.0 / texture_size[1]),
        };
        Self {
            direction,
            standard_deviation: kernel.standard_deviation,
            sample_count: kernel.sample_count,
            sample_step: kernel.sample_step,
            ..with_texture_sizes(
                empty_uniforms(DownsampleMode::Copy),
                texture_size,
                texture_size,
            )
        }
    }

    pub fn composite(
        bounds: Bounds<ScaledPixels>,
        content_mask: Bounds<ScaledPixels>,
        corner_radii: Corners<ScaledPixels>,
        corner_smoothing: f32,
        opacity: f32,
        clip: FilterCompositeClip,
        source_size: [f32; 2],
        target_size: [f32; 2],
    ) -> Self {
        Self {
            bounds: bounds.into(),
            content_mask: content_mask.into(),
            corner_radii: corner_radii.into(),
            corner_smoothing,
            opacity,
            composite_clip: match clip {
                FilterCompositeClip::RoundedBounds => ShaderBlurCompositeClip::RoundedBounds,
                FilterCompositeClip::ContentShape => ShaderBlurCompositeClip::None,
            },
            ..with_texture_sizes(
                empty_uniforms(DownsampleMode::Copy),
                source_size,
                target_size,
            )
        }
    }
}

#[derive(Clone, Copy)]
pub struct ScissorRectangle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ScissorRectangle {
    pub fn for_blurred_bounds(
        bounds: Bounds<ScaledPixels>,
        dilation: f32,
        full_width: u32,
        full_height: u32,
    ) -> Self {
        let downsampled_width = full_width.div_ceil(DOWNSAMPLE_FACTOR).max(1);
        let downsampled_height = full_height.div_ceil(DOWNSAMPLE_FACTOR).max(1);
        let minimum_x = downsampled_coordinate(
            bounds.origin.x.0 - dilation,
            downsampled_width,
            EdgeRounding::OutwardMinimum,
        );
        let minimum_y = downsampled_coordinate(
            bounds.origin.y.0 - dilation,
            downsampled_height,
            EdgeRounding::OutwardMinimum,
        );
        let maximum_x = downsampled_coordinate(
            bounds.origin.x.0 + bounds.size.width.0 + dilation,
            downsampled_width,
            EdgeRounding::OutwardMaximum,
        )
        .max(minimum_x);
        let maximum_y = downsampled_coordinate(
            bounds.origin.y.0 + bounds.size.height.0 + dilation,
            downsampled_height,
            EdgeRounding::OutwardMaximum,
        )
        .max(minimum_y);
        Self {
            x: minimum_x,
            y: minimum_y,
            width: maximum_x - minimum_x,
            height: maximum_y - minimum_y,
        }
    }

    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Clone, Copy)]
enum EdgeRounding {
    OutwardMinimum,
    OutwardMaximum,
}

fn downsampled_coordinate(value: f32, maximum: u32, rounding: EdgeRounding) -> u32 {
    let downsampled = value / DOWNSAMPLE_FACTOR as f32;
    let rounded = match rounding {
        EdgeRounding::OutwardMinimum => downsampled.floor(),
        EdgeRounding::OutwardMaximum => downsampled.ceil(),
    };
    (rounded.max(0.0) as u32).min(maximum)
}

fn with_texture_sizes(
    mut uniforms: BlurUniforms,
    source_size: [f32; 2],
    target_size: [f32; 2],
) -> BlurUniforms {
    uniforms.source_size = vec2f(source_size[0], source_size[1]);
    uniforms.target_size = vec2f(target_size[0], target_size[1]);
    uniforms
}

fn empty_uniforms(downsample_mode: DownsampleMode) -> BlurUniforms {
    let bounds = ShaderBounds {
        origin: vec2f(0.0, 0.0),
        size: vec2f(0.0, 0.0),
    };
    BlurUniforms {
        bounds,
        content_mask: bounds,
        corner_radii: ShaderCorners {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: 0.0,
            bottom_left: 0.0,
        },
        direction: vec2f(0.0, 0.0),
        standard_deviation: 0.0,
        opacity: 0.0,
        sample_count: 0,
        sample_step: 0.0,
        composite_clip: ShaderBlurCompositeClip::None,
        downsample_mode,
        source_size: vec2f(1.0, 1.0),
        target_size: vec2f(1.0, 1.0),
        corner_smoothing: 0.0,
        padding0: 0,
        padding1: 0,
        padding2: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::downsampled_dimension;

    #[test]
    fn downsampled_dimensions_cover_odd_edges() {
        assert_eq!(downsampled_dimension(0), 1);
        assert_eq!(downsampled_dimension(1), 1);
        assert_eq!(downsampled_dimension(2), 1);
        assert_eq!(downsampled_dimension(3), 2);
        assert_eq!(downsampled_dimension(33), 17);
    }
}
