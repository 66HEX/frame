#[wgsl_rs::wgsl]
pub mod emoji_rasterization {
    use super::super::common::*;
    use wgsl_rs::std::*;

    #[repr(C)]
    #[derive(Clone, Copy, Wgsl)]
    pub struct GlyphLayerTextureParams {
        pub run_color: Vec4f,
        pub gamma_ratios: Vec4f,
        pub grayscale_enhanced_contrast: f32,
    }

    uniform!(group(0), binding(0), GLYPH_LAYER_PARAMS: GlyphLayerTextureParams);
    texture!(group(0), binding(1), GLYPH_LAYER_TEXTURE: Texture2D<f32>);
    sampler!(group(0), binding(2), GLYPH_LAYER_SAMPLER: Sampler);

    #[derive(Wgsl)]
    pub struct EmojiRasterizationVarying {
        #[builtin(position)]
        pub position: Vec4f,
        #[location(0)]
        pub texture_coordinates: Vec2f,
    }

    #[vertex]
    pub fn vertex_emoji_rasterization(
        #[builtin(vertex_index)] vertex_id: u32,
    ) -> EmojiRasterizationVarying {
        let texture_coordinates = vec2f(
            ((vertex_id << 1u32) & 2u32) as f32,
            (vertex_id & 2u32) as f32,
        );
        EmojiRasterizationVarying {
            position: vec4f(
                texture_coordinates.x * 2.0 - 1.0,
                1.0 - texture_coordinates.y * 2.0,
                0.0,
                1.0,
            ),
            texture_coordinates,
        }
    }

    #[fragment]
    pub fn fragment_emoji_rasterization(input: EmojiRasterizationVarying) -> Vec4f {
        let params = get!(GLYPH_LAYER_PARAMS);
        let sample = texture_sample_level(
            GLYPH_LAYER_TEXTURE,
            GLYPH_LAYER_SAMPLER,
            input.texture_coordinates,
            0.0,
        )
        .x;
        let alpha = apply_contrast_and_gamma_correction(
            sample,
            params.run_color.rgb(),
            params.grayscale_enhanced_contrast,
            params.gamma_ratios,
        ) * params.run_color.w;
        vec4f(
            params.run_color.x * alpha,
            params.run_color.y * alpha,
            params.run_color.z * alpha,
            alpha,
        )
    }
}
