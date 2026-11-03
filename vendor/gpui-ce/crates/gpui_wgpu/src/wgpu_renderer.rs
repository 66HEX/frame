use crate::{CompositorGpuHint, WgpuAtlas, WgpuContext, WgpuContextHandle, WgpuDeviceRequirements};
use gpui::{DevicePixels, GpuSpecs, Scene, Size};

#[cfg(test)]
use gpui::BackdropFilter;
#[cfg(all(test, feature = "test-support", not(target_family = "wasm")))]
use gpui::{Bounds, ColorExt, Quad, ScaledPixels, Underline, blue, red, white};
#[cfg(test)]
use gpui_render::blur::{BlurKernel, MAX_GAUSSIAN_SAMPLES_PER_SIDE};
#[cfg(test)]
use gpui_render::shaders::interface as shader_interface;
#[cfg(all(test, feature = "test-support", not(target_family = "wasm")))]
use smallvec::{SmallVec, smallvec};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

mod buffers;
mod drawing;
mod filters;
mod frame;
#[cfg(all(feature = "test-support", not(target_family = "wasm")))]
mod headless;
mod path_types;
mod pipelines;
mod platform;
mod resources;
mod settings;
mod surfaces;
mod target;

#[cfg(all(feature = "test-support", not(target_family = "wasm")))]
pub use headless::WgpuHeadlessRenderer;
use pipelines::WgpuPipelines as ShaderPipelines;
use resources::{GlobalBufferLayout, ResourceMetadata, WgpuResources};
use settings::RenderingParameters;
pub use settings::{FontRasterizationSettings, SubpixelOrder, WgpuSurfaceConfig};
use target::RenderTarget;

/// Shared GPU context reference, used to coordinate device recovery across multiple windows.
pub type GpuContext = Rc<RefCell<Option<WgpuContext>>>;

struct GpuFaultState {
    pending_error: Arc<Mutex<Option<String>>>,
    consecutive_failed_frames: u32,
    device_lost: Arc<std::sync::atomic::AtomicBool>,
    #[cfg(not(target_family = "wasm"))]
    recovery_not_before: Option<std::time::Instant>,
}

pub struct WgpuRenderer {
    /// Shared GPU context for device recovery coordination (unused on WASM).
    #[allow(dead_code)]
    context: Option<GpuContext>,
    /// Compositor GPU hint for adapter selection (unused on WASM).
    #[allow(dead_code)]
    compositor_gpu: Option<CompositorGpuHint>,
    /// Application-requested extra wgpu features/limits, stored for device recovery.
    #[allow(dead_code)]
    extra_requirements: Option<WgpuDeviceRequirements>,
    resources: Option<WgpuResources>,
    target: RenderTarget,
    atlas: Arc<WgpuAtlas>,
    globals: GlobalBufferLayout,
    rendering_params: RenderingParameters,
    subpixel_order: SubpixelOrder,
    dual_source_blending: bool,
    adapter_info: wgpu::AdapterInfo,
    uploaded_globals: Option<frame::GlobalUniformState>,
    client_frame_corner_radius: Option<gpui::ScaledPixels>,
    faults: GpuFaultState,
}

impl WgpuRenderer {
    fn resources(&self) -> &WgpuResources {
        self.resources
            .as_ref()
            .expect("GPU resources not available")
    }

    fn resources_mut(&mut self) -> &mut WgpuResources {
        self.resources
            .as_mut()
            .expect("GPU resources not available")
    }

    fn new_internal(
        gpu_context: Option<GpuContext>,
        context: &WgpuContext,
        surface: Option<wgpu::Surface<'static>>,
        config: WgpuSurfaceConfig,
        compositor_gpu: Option<CompositorGpuHint>,
        extra_requirements: Option<WgpuDeviceRequirements>,
        atlas: Arc<WgpuAtlas>,
    ) -> anyhow::Result<Self> {
        let target =
            RenderTarget::new(&context.adapter, &context.device, surface.as_ref(), config)?;
        if let Some(surface) = surface.as_ref() {
            surface.configure(&context.device, target.configuration());
        }

        let dual_source_blending = context.supports_dual_source_blending();
        let rendering_params = RenderingParameters::new(&context.adapter, target.format());
        let adapter_info = context.adapter.get_info();
        let (resources, metadata) = WgpuResources::new(
            context,
            surface,
            target.configuration(),
            &rendering_params,
            dual_source_blending,
        )?;
        let ResourceMetadata {
            globals,
            last_error,
        } = metadata;

        Ok(Self {
            context: gpu_context,
            compositor_gpu,
            extra_requirements,
            resources: Some(resources),
            target,
            atlas,
            globals,
            rendering_params,
            subpixel_order: SubpixelOrder::RedGreenBlue,
            dual_source_blending,
            adapter_info,
            uploaded_globals: None,
            client_frame_corner_radius: None,
            faults: GpuFaultState {
                pending_error: last_error,
                consecutive_failed_frames: 0,
                device_lost: context.device_lost_flag(),
                #[cfg(not(target_family = "wasm"))]
                recovery_not_before: None,
            },
        })
    }

    pub fn set_client_frame_corner_radius(&mut self, radius: Option<gpui::ScaledPixels>) {
        if self.client_frame_corner_radius != radius {
            self.client_frame_corner_radius = radius;
            self.target.request_redraw();
        }
    }

    pub fn update_drawable_size(&mut self, size: Size<DevicePixels>) {
        if !self.target.resize(size) {
            return;
        }
        let config = self.target.configuration().clone();
        let Some(resources) = self.resources.as_mut() else {
            return;
        };
        resources.invalidate_intermediate_textures();
        if let Some(surface) = resources.surface.as_ref() {
            surface.configure(&resources.device, &config);
        }
    }

    /// Selects the physical LCD component order used by subpixel glyph correction.
    pub fn set_subpixel_order(&mut self, order: SubpixelOrder) {
        self.subpixel_order = order;
    }

    /// Updates platform-specific text rasterization parameters.
    pub fn set_font_rasterization_settings(&mut self, settings: FontRasterizationSettings) {
        self.rendering_params.font_rasterization = settings;
        self.subpixel_order = settings.subpixel_order;
    }

    /// Compatibility wrapper for callers that still report the layout as a boolean.
    pub fn set_subpixel_layout(&mut self, is_bgr: bool) {
        self.set_subpixel_order(if is_bgr {
            SubpixelOrder::BlueGreenRed
        } else {
            SubpixelOrder::RedGreenBlue
        });
    }

    pub fn update_transparency(&mut self, transparent: bool) {
        if !self.target.set_transparent(transparent) {
            return;
        }
        let config = self.target.configuration().clone();
        if let Some(resources) = self.resources.as_ref() {
            if let Some(surface) = resources.surface.as_ref() {
                surface.configure(&resources.device, &config);
            }
        }
        self.rebuild_pipelines();
    }

    pub(super) fn rebuild_pipelines(&mut self) {
        let config = self.target.configuration();
        let Some(resources) = self.resources.as_mut() else {
            return;
        };
        resources.pipelines = ShaderPipelines::new(
            &resources.device,
            &resources.bind_group_layouts,
            config.format,
            config.alpha_mode,
            self.rendering_params.path_sample_count,
            self.dual_source_blending,
            resources.renderer_tier,
        );
    }

    #[allow(dead_code)]
    pub fn viewport_size(&self) -> Size<DevicePixels> {
        self.target.viewport_size()
    }

    pub fn sprite_atlas(&self) -> &Arc<WgpuAtlas> {
        &self.atlas
    }

    pub fn supports_dual_source_blending(&self) -> bool {
        self.dual_source_blending
    }

    pub fn gpu_context(&self) -> (Arc<wgpu::Device>, Arc<wgpu::Queue>) {
        let resources = self.resources();
        (resources.device.clone(), resources.queue.clone())
    }

    pub fn gpu_context_info(&self) -> Option<WgpuContextHandle> {
        if let Some(context) = self.context.as_ref().and_then(|context| {
            context
                .borrow()
                .as_ref()
                .map(|context| context.handle(self.target.format()))
        }) {
            return Some(context);
        }
        self.resources.as_ref().map(|resources| {
            WgpuContextHandle::from_resources(
                resources.device.clone(),
                resources.queue.clone(),
                self.target.format(),
                self.adapter_info.clone(),
                match self.adapter_info.backend {
                    wgpu::Backend::BrowserWebGpu => crate::WgpuBackend::BrowserWebGpu,
                    wgpu::Backend::Gl => crate::WgpuBackend::Gl,
                    backend => crate::WgpuBackend::Native(backend),
                },
                self.faults.device_lost.clone(),
            )
        })
    }

    pub fn gpu_specs(&self) -> GpuSpecs {
        GpuSpecs {
            is_software_emulated: self.adapter_info.device_type == wgpu::DeviceType::Cpu,
            device_name: self.adapter_info.name.clone(),
            driver_name: self.adapter_info.driver.clone(),
            driver_info: self.adapter_info.driver_info.clone(),
        }
    }

    pub fn max_texture_size(&self) -> u32 {
        self.target.maximum_dimension()
    }

    /// Encodes and submits a complete scene into an arbitrary color target.
    /// Swapchain drawing, window capture, and headless rendering all use this path so
    /// platform integrations can't diverge in batching, filters, or pipeline state.
    fn render_to_view(&mut self, scene: &Scene, frame_view: &wgpu::TextureView) -> bool {
        frame::render_to_view(self, scene, frame_view, None).is_some()
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    fn render_to_view_with_readback(
        &mut self,
        scene: &Scene,
        frame_view: &wgpu::TextureView,
        readback: frame::ReadbackCopy<'_>,
    ) -> Option<wgpu::SubmissionIndex> {
        frame::render_to_view(self, scene, frame_view, Some(readback))
    }
}

fn begin_color_render_pass<'encoder>(
    encoder: &'encoder mut wgpu::CommandEncoder,
    label: &'encoder str,
    target: &'encoder wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) -> wgpu::RenderPass<'encoder> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgpu_renderer::filters::FrameUniformRequirements;

    // Native shader backends can differ slightly when quantizing transcendental math to UNORM8.
    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    const MAX_CROSS_BACKEND_CHANNEL_DELTA: u8 = 2;

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    fn dashed_border_scene(dash_length: f32, dash_gap: f32) -> Scene {
        let full_bounds = test_bounds(0.0, 0.0, 40.0, 20.0);
        let quad_bounds = |left| test_bounds(left, 4.0, 12.0, 12.0);
        let mut scene = Scene::default();

        for (bounds, corner_smoothing) in [(quad_bounds(4.0), 0.0), (quad_bounds(24.0), 0.6)] {
            scene.insert_primitive(Quad {
                bounds,
                content_mask: gpui::ContentMask {
                    bounds: full_bounds,
                    ..Default::default()
                },
                background: gpui::solid_background(gpui::hsla(0.05, 0.8, 0.45, 1.0)),
                border_style: gpui::BorderStyle::Dashed,
                border_dashed_length: dash_length,
                border_dashed_gap: dash_gap,
                border_color: gpui::hsla(0.6, 0.9, 0.7, 1.0).into(),
                corner_radii: gpui::Corners::all(ScaledPixels(4.0)),
                border_widths: gpui::Edges::all(ScaledPixels(2.0)),
                corner_smoothing,
                ..Default::default()
            });
        }

        scene.finish();

        scene
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    fn images_differ_in_region(
        first: &image::RgbaImage,
        second: &image::RgbaImage,
        left: u32,
        right: u32,
    ) -> bool {
        (4..16).any(|y| (left..right).any(|x| first.get_pixel(x, y) != second.get_pixel(x, y)))
    }

    #[test]
    fn rust_storage_types_match_shader_strides() {
        fn module_layout(source: &wgsl_rs::Source) -> (naga::Module, naga::proc::Layouter) {
            let source = source.wgsl_source().expect("shader must generate WGSL");
            let module =
                naga::front::wgsl::parse_str(&format!("enable dual_source_blending;\n{source}"))
                    .expect("generated shader must parse as WGSL");
            let mut layouter = naga::proc::Layouter::default();
            layouter
                .update(module.to_ctx())
                .expect("generated shader types must have valid layouts");
            (module, layouter)
        }

        let (base, base_layout) = module_layout(&gpui_render::shaders::base::WGSL_SOURCE);
        let (subpixel, subpixel_layout) =
            module_layout(&gpui_render::shaders::subpixel_sprite::WGSL_SOURCE);

        for abi in shader_interface::SCENE_STORAGE_ABI
            .iter()
            .chain(path_types::STORAGE_ABI)
        {
            let base_handle = base
                .types
                .iter()
                .find(|(_, ty)| ty.name.as_deref() == Some(abi.wgsl_type))
                .map(|(handle, _)| handle);
            let shader_stride = if let Some(handle) = base_handle {
                base_layout[handle].to_stride()
            } else {
                let (handle, _) = subpixel
                    .types
                    .iter()
                    .find(|(_, ty)| ty.name.as_deref() == Some(abi.wgsl_type))
                    .unwrap_or_else(|| panic!("missing WGSL type {}", abi.wgsl_type));
                subpixel_layout[handle].to_stride()
            };
            assert_eq!(
                shader_stride as usize, abi.rust_stride,
                "Rust and WGSL layouts differ for {}",
                abi.wgsl_type,
            );
        }
    }

    #[test]
    fn large_filter_scenes_reserve_unique_uniform_slots() {
        let mut scene = Scene::default();
        scene
            .backdrop_filters
            .resize_with(100, BackdropFilter::default);
        scene.finish();

        assert_eq!(
            frame::FrameRequirements::for_scene(
                &scene,
                super::buffers::InstanceTransport::StorageBuffer
            )
            .uniforms,
            FrameUniformRequirements {
                filter_count: 401,
                surface_count: 0,
            }
        );
    }

    #[test]
    fn large_blur_radii_expand_tap_spacing_instead_of_truncating_the_kernel() {
        let kernel = BlurKernel::for_radius(100.0).expect("positive radii have a blur kernel");
        assert_eq!(kernel.standard_deviation, 50.0);
        assert_eq!(kernel.sample_count, MAX_GAUSSIAN_SAMPLES_PER_SIDE);
        assert!(kernel.sample_step > 4.0);
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn desktop_blending_preserves_source_over_alpha() {
        let straight = pipelines::desktop_scene_blend_state(wgpu::CompositeAlphaMode::Opaque);
        assert_eq!(straight.color.src_factor, wgpu::BlendFactor::SrcAlpha);
        assert_eq!(
            straight.alpha.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );

        let premultiplied =
            pipelines::desktop_scene_blend_state(wgpu::CompositeAlphaMode::PreMultiplied);
        assert_eq!(premultiplied.color.src_factor, wgpu::BlendFactor::One);
        assert_eq!(
            premultiplied.alpha.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn underline_opacity_is_applied_once() -> anyhow::Result<()> {
        let context = WgpuContext::new_headless(None)?;
        let size = gpui::size(DevicePixels(3), DevicePixels(3));
        let mut renderer = WgpuRenderer::new_headless(&context, size)?;
        let bounds = test_bounds(0.0, 0.0, 3.0, 3.0);
        let mut scene = Scene::default();
        // Forces a non-zero first-instance index in the mixed-type arena.
        scene.insert_primitive(Quad::default());
        scene.insert_primitive(Underline {
            order: 0,
            padding: 0,
            bounds,
            content_mask: gpui::ContentMask {
                bounds,
                ..Default::default()
            },
            color: gpui::hsla(0.0, 1.0, 0.5, 0.5).into(),
            thickness: ScaledPixels(1.0),
            wavy: false.into(),
        });
        scene.finish();

        let image = renderer.render_to_image(&scene)?;
        let center = image.get_pixel(1, 1).0;
        assert_eq!(
            center,
            [128, 0, 0, 255],
            "must match the retired Metal renderer's mixed primitive baseline"
        );
        Ok(())
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn configurable_dashes_reach_both_wgpu_quad_pipelines() -> anyhow::Result<()> {
        let context = WgpuContext::new_headless(None)?;
        let size = gpui::size(DevicePixels(40), DevicePixels(20));
        let mut renderer = WgpuRenderer::new_headless(&context, size)?;
        let default_image = renderer.render_to_image(&dashed_border_scene(2.0, 1.0))?;
        let custom_image = renderer.render_to_image(&dashed_border_scene(4.0, 0.5))?;

        assert!(
            images_differ_in_region(&default_image, &custom_image, 4, 16),
            "custom dash length and gap must change the ordinary quad pipeline"
        );
        assert!(
            images_differ_in_region(&default_image, &custom_image, 24, 36),
            "custom dash length and gap must change the smoothed quad pipeline"
        );

        Ok(())
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn quad_backgrounds_match_legacy_metal_pixels() -> anyhow::Result<()> {
        const LEGACY: &[u8] = &[
            255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 0, 255, 38, 110, 217, 255, 255, 0, 0, 255, 255,
            0, 0, 255, 38, 110, 217, 255, 0, 0, 0, 255, 10, 34, 4, 255, 57, 196, 22, 255, 193, 114,
            152, 255, 223, 155, 105, 255, 57, 196, 22, 255, 10, 34, 4, 255, 165, 60, 182, 255, 193,
            114, 151, 255,
        ];
        let bounds = |x, y| test_bounds(x, y, 2.0, 2.0);
        let mut scene = Scene::default();
        for (order, (bounds, background)) in [
            (
                bounds(0.0, 0.0),
                gpui::solid_background(gpui::hsla(0.0, 1.0, 0.5, 1.0)),
            ),
            (
                bounds(2.0, 0.0),
                gpui::checkerboard(gpui::hsla(0.6, 0.7, 0.5, 1.0), 1.0),
            ),
            (
                bounds(0.0, 2.0),
                gpui::pattern_slash(gpui::hsla(0.3, 0.8, 0.5, 1.0), 1.0, 1.0),
            ),
            (
                bounds(2.0, 2.0),
                gpui::linear_gradient(
                    45.0,
                    gpui::linear_color_stop(gpui::hsla(0.8, 0.9, 0.4, 1.0), 0.0),
                    gpui::linear_color_stop(gpui::hsla(0.1, 0.8, 0.6, 1.0), 1.0),
                ),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            scene.insert_primitive(Quad {
                order: order as u32,
                bounds,
                content_mask: gpui::ContentMask {
                    bounds,
                    ..Default::default()
                },
                background,
                ..Default::default()
            });
        }
        scene.finish();
        let context = WgpuContext::new_headless(None)?;
        let mut renderer =
            WgpuRenderer::new_headless(&context, gpui::size(DevicePixels(4), DevicePixels(4)))?;
        let actual = renderer.render_to_image(&scene)?;
        for (index, (actual, expected)) in actual.as_raw().iter().zip(LEGACY).enumerate() {
            assert!(
                actual.abs_diff(*expected) <= MAX_CROSS_BACKEND_CHANNEL_DELTA,
                "legacy mismatch at pixel ({}, {}), channel {}: got {}, expected {}",
                (index / 4) % 4,
                (index / 4) / 4,
                index % 4,
                actual,
                expected,
            );
        }
        assert_eq!(actual.as_raw().len(), LEGACY.len());
        Ok(())
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn quad_border_backgrounds_use_the_fill_coordinate_space() -> anyhow::Result<()> {
        let context = WgpuContext::new_headless(None)?;
        let size = gpui::size(DevicePixels(12), DevicePixels(12));
        let bounds = test_bounds(0.0, 0.0, 12.0, 12.0);
        let render = |quad| -> anyhow::Result<image::RgbaImage> {
            let mut scene = Scene::default();
            scene.insert_primitive(quad);
            scene.finish();
            let mut renderer = WgpuRenderer::new_headless(&context, size)?;
            renderer.render_to_image(&scene)
        };
        let backgrounds = [
            gpui::solid_background(gpui::hsla(0.0, 1.0, 0.5, 1.0)),
            gpui::checkerboard(gpui::hsla(0.6, 0.7, 0.5, 1.0), 2.0),
            gpui::pattern_slash(gpui::hsla(0.3, 0.8, 0.5, 1.0), 2.0, 2.0),
            gpui::linear_gradient(
                90.0,
                gpui::linear_color_stop(gpui::hsla(0.8, 0.9, 0.4, 1.0), 0.0),
                gpui::linear_color_stop(gpui::hsla(0.1, 0.8, 0.6, 1.0), 1.0),
            ),
        ];
        let border_samples = [
            (1, 1),
            (6, 1),
            (10, 1),
            (1, 6),
            (10, 6),
            (1, 10),
            (6, 10),
            (10, 10),
        ];

        for background in backgrounds {
            let filled = render(Quad {
                bounds,
                content_mask: gpui::ContentMask {
                    bounds,
                    ..Default::default()
                },
                background,
                ..Default::default()
            })?;
            let bordered = render(Quad {
                bounds,
                content_mask: gpui::ContentMask {
                    bounds,
                    ..Default::default()
                },
                border_color: background,
                border_widths: gpui::Edges::all(ScaledPixels(4.0)),
                ..Default::default()
            })?;

            for (x, y) in border_samples {
                let border_pixel = bordered.get_pixel(x, y).0;
                let fill_pixel = filled.get_pixel(x, y).0;
                for (channel, (border, fill)) in
                    border_pixel.iter().zip(fill_pixel.iter()).enumerate()
                {
                    assert!(
                        border.abs_diff(*fill) <= MAX_CROSS_BACKEND_CHANNEL_DELTA,
                        "border background must sample like a fill at ({x}, {y}), channel {channel}, for {background:?}: got {border_pixel:?}, expected {fill_pixel:?}"
                    );
                }
            }
            assert_eq!(
                bordered.get_pixel(6, 6).0,
                [0, 0, 0, 255],
                "border background must not paint the quad interior for {background:?}"
            );
        }
        Ok(())
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn cached_filter_bindings_accept_frame_local_dynamic_offsets() -> anyhow::Result<()> {
        let context = WgpuContext::new_headless(None)?;
        let size = gpui::size(DevicePixels(4), DevicePixels(4));
        let mut renderer = WgpuRenderer::new_headless(&context, size)?;
        let bounds = test_bounds(0.0, 0.0, 4.0, 4.0);
        let mut scene = Scene::default();
        scene.insert_primitive(BackdropFilter {
            bounds,
            content_mask: gpui::ContentMask {
                bounds,
                ..Default::default()
            },
            filters: smallvec::smallvec![gpui::ScaledFilter::Blur(ScaledPixels(1.0))],
            opacity: 1.0,
            ..BackdropFilter::default()
        });
        scene.finish();

        renderer.render_to_image(&scene)?;
        renderer.render_to_image(&scene)?;
        Ok(())
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn odd_sized_blur_preserves_edge_symmetry() -> anyhow::Result<()> {
        let context = WgpuContext::new_headless(None)?;
        let size = gpui::size(DevicePixels(5), DevicePixels(5));
        let mut renderer = WgpuRenderer::new_headless(&context, size)?;
        let full_bounds = test_bounds(0.0, 0.0, 5.0, 5.0);
        let center_bounds = test_bounds(2.0, 2.0, 1.0, 1.0);
        let mut scene = Scene::default();
        scene.insert_primitive(Quad {
            order: 0,
            bounds: full_bounds,
            content_mask: gpui::ContentMask {
                bounds: full_bounds,
                ..Default::default()
            },
            background: gpui::solid_background(gpui::hsla(0.0, 0.0, 0.0, 1.0)),
            ..Default::default()
        });
        scene.insert_primitive(Quad {
            order: 1,
            bounds: center_bounds,
            content_mask: gpui::ContentMask {
                bounds: center_bounds,
                ..Default::default()
            },
            background: gpui::solid_background(gpui::hsla(0.0, 0.0, 1.0, 1.0)),
            ..Default::default()
        });
        scene.insert_primitive(BackdropFilter {
            order: 2,
            bounds: full_bounds,
            content_mask: gpui::ContentMask {
                bounds: full_bounds,
                ..Default::default()
            },
            filters: smallvec::smallvec![gpui::ScaledFilter::Blur(ScaledPixels(2.0))],
            opacity: 1.0,
            ..Default::default()
        });
        scene.finish();

        let image = renderer.render_to_image(&scene)?;
        for y in 0..5 {
            for x in 0..5 {
                let actual = image.get_pixel(x, y).0;
                let horizontal = image.get_pixel(4 - x, y).0;
                let vertical = image.get_pixel(x, 4 - y).0;
                for channel in 0..4 {
                    assert!(
                        actual[channel].abs_diff(horizontal[channel]) <= 1,
                        "horizontal blur asymmetry at ({x}, {y}), channel {channel}: {actual:?} vs {horizontal:?}"
                    );
                    assert!(
                        actual[channel].abs_diff(vertical[channel]) <= 1,
                        "vertical blur asymmetry at ({x}, {y}), channel {channel}: {actual:?} vs {vertical:?}"
                    );
                }
            }
        }
        assert!(
            image.get_pixel(1, 2).0[0] > 0,
            "blur must spread from the center pixel"
        );
        Ok(())
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn alternating_instance_batches_fit_the_exact_upload_arena() -> anyhow::Result<()> {
        let context = WgpuContext::new_headless(None)?;
        let size = gpui::size(DevicePixels(1), DevicePixels(1));
        let mut renderer = WgpuRenderer::new_headless(&context, size)?;
        let bounds = test_bounds(0.0, 0.0, 1.0, 1.0);
        let mut scene = Scene::default();
        for index in 0..512 {
            scene.insert_primitive(Quad {
                order: index * 2,
                bounds,
                content_mask: gpui::ContentMask {
                    bounds,
                    ..Default::default()
                },
                background: gpui::solid_background(gpui::hsla(0.0, 0.0, 0.0, 1.0)),
                ..Default::default()
            });
            scene.insert_primitive(Underline {
                order: index * 2 + 1,
                padding: 0,
                bounds,
                content_mask: gpui::ContentMask {
                    bounds,
                    ..Default::default()
                },
                color: gpui::hsla(0.0, 0.0, 1.0, 1.0).into(),
                thickness: ScaledPixels(1.0),
                wavy: false.into(),
            });
        }
        scene.finish();
        assert_eq!(
            scene
                .render_commands()
                .iter()
                .filter(|command| matches!(command, gpui::RenderCommand::Batch(_)))
                .count(),
            1024
        );

        renderer.render_to_image(&scene)?;
        Ok(())
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    fn test_bounds(left: f32, top: f32, width: f32, height: f32) -> Bounds<ScaledPixels> {
        Bounds::new(
            gpui::point(ScaledPixels(left), ScaledPixels(top)),
            gpui::size(ScaledPixels(width), ScaledPixels(height)),
        )
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    fn test_quad(
        bounds: Bounds<ScaledPixels>,
        mask: Bounds<ScaledPixels>,
        color: gpui::Hsla,
    ) -> Quad {
        Quad {
            bounds,
            content_mask: gpui::ContentMask {
                bounds: mask,
                ..Default::default()
            },
            background: gpui::solid_background(color),
            ..Default::default()
        }
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    fn test_group(
        scene: &mut Scene,
        bounds: Bounds<ScaledPixels>,
        opacity: f32,
        filters: SmallVec<[gpui::ScaledFilter; 4]>,
        paint: impl FnOnce(&mut Scene),
    ) {
        let boundary = gpui::FilterBoundary {
            order: 0,
            bounds,
            content_mask: gpui::ContentMask {
                bounds,
                ..Default::default()
            },
            corner_radii: gpui::Corners::default(),
            corner_smoothing: 0.0,
            filters,
            opacity,
            is_start: true,
        };
        scene.insert_primitive(boundary.clone());
        paint(scene);
        scene.insert_primitive(gpui::FilterBoundary {
            is_start: false,
            ..boundary
        });
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    fn test_renderers() -> anyhow::Result<Vec<Box<dyn gpui::PlatformHeadlessRenderer>>> {
        let mut renderers: Vec<Box<dyn gpui::PlatformHeadlessRenderer>> =
            vec![Box::new(WgpuHeadlessRenderer::new()?)];

        if let Some(native) = gpui_platform::current_headless_renderer() {
            renderers.push(native);
        }

        Ok(renderers)
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn opacity_groups_preserve_overlap_alpha_and_deep_nesting() -> anyhow::Result<()> {
        let bounds = test_bounds(0.0, 0.0, 16.0, 8.0);
        let target_size = gpui::size(DevicePixels(16), DevicePixels(8));
        let green = gpui::hsla(1.0 / 3.0, 1.0, 0.5, 1.0);
        let mut slider = Scene::default();
        test_group(&mut slider, bounds, 0.5, smallvec![], |scene| {
            scene.insert_primitive(test_quad(test_bounds(0.0, 3.0, 8.0, 2.0), bounds, red()));
            scene.insert_primitive(test_quad(test_bounds(4.0, 1.0, 4.0, 6.0), bounds, white()));
        });

        let mut translucent = Scene::default();
        translucent.insert_primitive(test_quad(bounds, bounds, green));
        test_group(&mut translucent, bounds, 0.5, smallvec![], |scene| {
            scene.insert_primitive(test_quad(bounds, bounds, red().opacity(0.5)));
            scene.insert_primitive(test_quad(bounds, bounds, blue().opacity(0.5)));
        });

        let mut nested = Scene::default();
        test_group(&mut nested, bounds, 0.5, smallvec![], |scene| {
            scene.insert_primitive(test_quad(bounds, bounds, red()));
            test_group(scene, bounds, 0.5, smallvec![], |scene| {
                scene.insert_primitive(test_quad(bounds, bounds, blue()));
                test_group(scene, bounds, 0.5, smallvec![], |scene| {
                    scene.insert_primitive(test_quad(bounds, bounds, white()));
                });
            });
        });
        nested.insert_primitive(test_quad(test_bounds(12.0, 0.0, 4.0, 8.0), bounds, green));

        for scene in [&mut slider, &mut translucent, &mut nested] {
            scene.finish();
        }

        let cases = [
            (
                "overlap",
                &slider,
                &[
                    (6, 4, [128, 128, 128, 255]),
                    (1, 4, [128, 0, 0, 255]),
                    (6, 1, [128, 128, 128, 255]),
                    (8, 4, [0, 0, 0, 255]),
                ][..],
            ),
            (
                "intrinsic alpha",
                &translucent,
                &[(6, 4, [32, 159, 64, 255])][..],
            ),
            (
                "nesting",
                &nested,
                &[(6, 4, [96, 32, 64, 255]), (14, 4, [0, 255, 0, 255])][..],
            ),
        ];

        for (renderer_idx, renderer) in test_renderers()?.iter_mut().enumerate() {
            for (name, scene, samples) in cases {
                let image = renderer.render_scene_to_image(scene, target_size)?;

                for &(x, y, expected) in samples {
                    let actual = image.get_pixel(x, y).0;
                    assert!(
                        actual
                            .iter()
                            .zip(expected)
                            .all(|(actual, expected)| actual.abs_diff(expected) <= 2),
                        "renderer {renderer_idx}, {name}, pixel ({x}, {y}): {actual:?}, expected {expected:?}"
                    );
                }
            }
        }

        Ok(())
    }

    #[cfg(all(feature = "test-support", not(target_family = "wasm")))]
    #[test]
    fn opacity_groups_preserve_blur_and_inherited_backdrops() -> anyhow::Result<()> {
        let bounds = test_bounds(0.0, 0.0, 16.0, 8.0);
        let target_size = gpui::size(DevicePixels(16), DevicePixels(8));
        let green = gpui::hsla(1.0 / 3.0, 1.0, 0.5, 1.0);
        let blur = smallvec![gpui::ScaledFilter::Blur(ScaledPixels(2.0))];
        let backdrop = BackdropFilter {
            bounds,
            content_mask: gpui::ContentMask {
                bounds,
                ..Default::default()
            },
            filters: blur.clone(),
            opacity: 1.0,
            ..Default::default()
        };
        let paint_background = |scene: &mut Scene| {
            scene.insert_primitive(test_quad(bounds, bounds, red()));
            scene.insert_primitive(test_quad(test_bounds(8.0, 0.0, 8.0, 8.0), bounds, blue()));
        };

        let mut original = Scene::default();
        paint_background(&mut original);
        original.finish();

        for (renderer_idx, renderer) in test_renderers()?.iter_mut().enumerate() {
            let background = renderer.render_scene_to_image(&original, target_size)?;

            for (name, depth, with_local_quad, content_blur) in [
                ("inherited backdrop", 2, false, false),
                ("local backdrop", 1, true, false),
                ("content blur", 1, false, true),
            ] {
                let paint = |scene: &mut Scene, blur_enabled: bool| {
                    if with_local_quad {
                        scene.insert_primitive(test_quad(
                            test_bounds(6.0, 0.0, 4.0, 8.0),
                            bounds,
                            green,
                        ));
                    }

                    if content_blur {
                        let filters = if blur_enabled {
                            blur.clone()
                        } else {
                            smallvec![]
                        };
                        test_group(scene, bounds, 1.0, filters, |scene| {
                            scene.insert_primitive(test_quad(
                                test_bounds(6.0, 2.0, 4.0, 4.0),
                                bounds,
                                white(),
                            ));
                        });
                    } else if blur_enabled {
                        scene.insert_primitive(backdrop.clone());
                    }
                };

                let mut reference = Scene::default();
                let mut grouped = Scene::default();
                let mut unfiltered = Scene::default();

                if !content_blur {
                    paint_background(&mut reference);
                    paint_background(&mut grouped);
                    paint_background(&mut unfiltered);
                }

                paint(&mut reference, true);
                paint(&mut unfiltered, false);
                test_group(&mut grouped, bounds, 0.5, smallvec![], |scene| {
                    if depth == 2 {
                        test_group(scene, bounds, 0.5, smallvec![], |scene| paint(scene, true));
                    } else {
                        paint(scene, true);
                    }
                });

                reference.finish();
                grouped.finish();
                unfiltered.finish();

                let reference = renderer.render_scene_to_image(&reference, target_size)?;
                let actual = renderer.render_scene_to_image(&grouped, target_size)?;
                let unfiltered = renderer.render_scene_to_image(&unfiltered, target_size)?;
                let opacity = 0.5_f32.powi(depth);

                assert!(
                    reference
                        .pixels()
                        .zip(unfiltered.pixels())
                        .any(|(blurred, plain)| {
                            (0..3).any(|channel_idx| {
                                blurred[channel_idx].abs_diff(plain[channel_idx]) > 8
                            })
                        }),
                    "renderer {renderer_idx}, {name}: blur must visibly change the unfiltered scene"
                );

                if content_blur {
                    let falloff = reference.get_pixel(5, 4).0;
                    assert!(
                        falloff[0] > 8 && falloff[1] > 8 && falloff[2] > 8,
                        "renderer {renderer_idx}: content blur must spread outside the white quad: {falloff:?}"
                    );
                }

                for (pixel_idx, ((actual, reference), background)) in actual
                    .pixels()
                    .zip(reference.pixels())
                    .zip(background.pixels())
                    .enumerate()
                {
                    for channel_idx in 0..3 {
                        let original = if content_blur {
                            0.0
                        } else {
                            background[channel_idx] as f32
                        };
                        let expected = (reference[channel_idx] as f32 * opacity
                            + original * (1.0 - opacity))
                            .round() as u8;
                        assert!(
                            actual[channel_idx].abs_diff(expected) <= 3,
                            "renderer {renderer_idx}, {name}, pixel {pixel_idx}, channel {channel_idx}: {}, expected {expected}",
                            actual[channel_idx]
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
