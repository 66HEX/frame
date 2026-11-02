use gpui::{Bounds, Corners, ScaledPixels, point, size};
use gpui_render::shaders::interface as shader_interface;
use gpui_render::{
    blur::{
        BlurAxis, BlurKernel, FilterCompositeClip, FilterCompositeParameters,
        GAUSSIAN_CUTOFF_STANDARD_DEVIATIONS, ScissorRectangle, downsampled_dimension,
    },
    shaders::blur::BlurUniforms,
};

use super::{WgpuRenderer, begin_color_render_pass, pipelines};

pub(super) const FILTER_UNIFORMS_PER_COMPOSITE: u64 = 4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct FrameUniformRequirements {
    pub(super) filter_count: u64,
    pub(super) surface_count: u64,
}

const _: () = assert!(std::mem::size_of::<BlurUniforms>() == 112);

impl WgpuRenderer {
    fn draw_filter_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        label: &str,
        pipeline: &pipelines::WgpuRenderPipeline,
        target: &wgpu::TextureView,
        source: &wgpu::TextureView,
        uniforms: BlurUniforms,
        load: wgpu::LoadOp<wgpu::Color>,
        scissor: Option<ScissorRectangle>,
    ) {
        let resources = self.resources();
        let uniform_offset = resources.filter_uniforms.write(&uniforms);
        let bind_group = resources.blur_bind_group(source);
        let mut pass = begin_color_render_pass(encoder, label, target, load);
        pass.set_pipeline(pipeline);
        pass.set_bind_group(
            shader_interface::GLOBAL_BIND_GROUP,
            &resources.globals_bind_group,
            &[],
        );
        pass.set_bind_group(
            shader_interface::DATA_BIND_GROUP,
            &bind_group,
            &[uniform_offset],
        );

        if let Some(scissor) = scissor {
            pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
        }

        pass.draw(0..pipeline.fixed_vertex_count(), 0..1);
    }

    pub(super) fn blur_and_composite(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
        parameters: FilterCompositeParameters,
    ) {
        if parameters.blur_radius <= 0.0
            && matches!(parameters.clip, FilterCompositeClip::RoundedBounds)
        {
            return;
        }

        let full_size = [self.target.width() as f32, self.target.height() as f32];

        let Some(kernel) = BlurKernel::for_radius(parameters.blur_radius) else {
            self.composite_texture(encoder, source, target, parameters, full_size);

            return;
        };
        let full_width = self.target.width();
        let full_height = self.target.height();
        let blur_size = [
            downsampled_dimension(full_width) as f32,
            downsampled_dimension(full_height) as f32,
        ];
        let dilation = GAUSSIAN_CUTOFF_STANDARD_DEVIATIONS * parameters.blur_radius;
        let scissor = ScissorRectangle::for_blurred_bounds(
            parameters.bounds,
            dilation,
            full_width,
            full_height,
        );
        if scissor.is_empty() {
            return;
        }

        let (horizontal_target, vertical_target) = {
            let resources = self.resources();
            match (
                resources.blur_ping_view.as_ref(),
                resources.blur_pong_view.as_ref(),
            ) {
                (Some(horizontal), Some(vertical)) => (horizontal.clone(), vertical.clone()),
                _ => return,
            }
        };

        self.draw_filter_pass(
            encoder,
            "blur_downsample",
            &self.resources().pipelines.blur_downsample,
            &horizontal_target,
            source,
            BlurUniforms::downsample([full_width as f32, full_height as f32], blur_size),
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            Some(scissor),
        );
        self.draw_filter_pass(
            encoder,
            "blur_horizontal",
            &self.resources().pipelines.blur,
            &vertical_target,
            &horizontal_target,
            BlurUniforms::gaussian(BlurAxis::Horizontal, blur_size, kernel),
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            Some(scissor),
        );
        self.draw_filter_pass(
            encoder,
            "blur_vertical",
            &self.resources().pipelines.blur,
            &horizontal_target,
            &vertical_target,
            BlurUniforms::gaussian(BlurAxis::Vertical, blur_size, kernel),
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            Some(scissor),
        );

        self.composite_texture(encoder, &horizontal_target, target, parameters, blur_size);
    }

    fn composite_texture(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
        parameters: FilterCompositeParameters,
        source_size: [f32; 2],
    ) {
        let clips_to_bounds = matches!(parameters.clip, FilterCompositeClip::RoundedBounds);
        let composite_bounds = if clips_to_bounds {
            parameters.bounds
        } else {
            parameters.bounds.dilate(ScaledPixels(
                GAUSSIAN_CUTOFF_STANDARD_DEVIATIONS * parameters.blur_radius,
            ))
        };
        let uniforms = BlurUniforms::composite(
            composite_bounds,
            parameters.content_mask,
            parameters.corner_radii,
            parameters.corner_smoothing,
            parameters.opacity,
            parameters.clip,
            source_size,
            [self.target.width() as f32, self.target.height() as f32],
        );
        let resources = self.resources();
        let pipeline = if uniforms.corner_smoothing > 0.0 {
            &resources.pipelines.smoothed_blur_composite
        } else {
            &resources.pipelines.blur_composite
        };

        self.draw_filter_pass(
            encoder,
            "blur_composite",
            pipeline,
            target,
            source,
            uniforms,
            wgpu::LoadOp::Load,
            None,
        );
    }

    pub(super) fn snapshot_backdrop<'a>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        sources: impl Iterator<Item = &'a wgpu::TextureView>,
    ) -> wgpu::TextureView {
        let target = self
            .resources()
            .backdrop_snapshot_view
            .as_ref()
            .expect("backdrop snapshot was prepared")
            .clone();
        let full_size = [self.target.width() as f32, self.target.height() as f32];
        let bounds = Bounds::new(
            point(ScaledPixels(0.0), ScaledPixels(0.0)),
            size(ScaledPixels(full_size[0]), ScaledPixels(full_size[1])),
        );
        drop(begin_color_render_pass(
            encoder,
            "clear_backdrop_snapshot",
            &target,
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
        ));

        for source in sources {
            self.composite_texture(
                encoder,
                source,
                &target,
                FilterCompositeParameters {
                    bounds,
                    content_mask: bounds,
                    corner_radii: Corners::default(),
                    corner_smoothing: 0.0,
                    blur_radius: 0.0,
                    opacity: 1.0,
                    clip: FilterCompositeClip::ContentShape,
                },
                full_size,
            );
        }

        target
    }

    pub(super) fn blit_to_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        frame_view: &wgpu::TextureView,
    ) {
        let size = [self.target.width() as f32, self.target.height() as f32];
        if let Some(radius) = self.client_frame_corner_radius {
            let bounds = Bounds {
                origin: point(ScaledPixels(0.0), ScaledPixels(0.0)),
                size: gpui::size(ScaledPixels(size[0]), ScaledPixels(size[1])),
            };
            self.draw_filter_pass(
                encoder,
                "client_frame_clip",
                &self.resources().pipelines.blur_composite,
                frame_view,
                source,
                BlurUniforms::composite(
                    bounds,
                    bounds,
                    Corners::all(radius),
                    0.0,
                    1.0,
                    FilterCompositeClip::RoundedBounds,
                    size,
                    size,
                ),
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                None,
            );
            return;
        }

        self.draw_filter_pass(
            encoder,
            "scene_blit",
            &self.resources().pipelines.blur_downsample,
            frame_view,
            source,
            BlurUniforms::copy(size),
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            None,
        );
    }
}
