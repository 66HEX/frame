use std::{
    cell::RefCell,
    num::NonZeroU64,
    sync::{Arc, Mutex},
};

use collections::FxHashMap;
use gpui::ScenePlanRequirements;

use crate::WgpuContext;
use gpui_render::blur::downsampled_dimension;
use gpui_render::shaders::{
    blur::BlurUniforms,
    common::{FontRasterizationUniforms, GlobalUniforms},
    surface::SurfaceUniforms,
};

use super::{
    WgpuRenderer,
    buffers::{DynamicUniformBuffer, InstanceBufferArena},
    filters::FrameUniformRequirements,
    pipelines::{WgpuBindGroupLayouts, WgpuPipelines},
    settings::RenderingParameters,
    surfaces::SurfaceCache,
};

const INITIAL_FILTER_UNIFORM_CAPACITY: u64 = 16;
const INITIAL_SURFACE_UNIFORM_CAPACITY: u64 = 8;

/// Device-owned state that is replaced atomically during GPU recovery.
pub(super) struct WgpuResources {
    pub(super) device: Arc<wgpu::Device>,
    pub(super) queue: Arc<wgpu::Queue>,
    pub(super) renderer_tier: crate::RendererTier,
    pub(super) surface: Option<wgpu::Surface<'static>>,
    pub(super) pipelines: WgpuPipelines,
    pub(super) bind_group_layouts: WgpuBindGroupLayouts,
    pub(super) atlas_sampler: wgpu::Sampler,
    pub(super) surface_sampler: wgpu::Sampler,
    pub(super) surface_uniforms: DynamicUniformBuffer<SurfaceUniforms>,
    #[cfg_attr(
        not(any(
            all(target_family = "wasm", feature = "custom-gpu"),
            target_os = "macos",
            target_os = "linux",
            target_os = "freebsd",
            all(target_os = "windows", feature = "wgpu-surfaces")
        )),
        allow(dead_code)
    )]
    pub(super) surface_cache: RefCell<SurfaceCache>,
    pub(super) filter_uniforms: DynamicUniformBuffer<BlurUniforms>,
    blur_bind_groups: RefCell<BlurBindGroups>,
    pub(super) globals_buffer: wgpu::Buffer,
    pub(super) globals_bind_group: wgpu::BindGroup,
    pub(super) path_globals_bind_group: wgpu::BindGroup,
    pub(super) instances: InstanceBufferArena,
    pub(super) path_intermediate_texture: Option<wgpu::Texture>,
    pub(super) path_intermediate_view: Option<wgpu::TextureView>,
    pub(super) path_msaa_texture: Option<wgpu::Texture>,
    pub(super) path_msaa_view: Option<wgpu::TextureView>,
    pub(super) scene_color_texture: Option<wgpu::Texture>,
    pub(super) scene_color_view: Option<wgpu::TextureView>,
    pub(super) backdrop_snapshot_texture: Option<wgpu::Texture>,
    pub(super) backdrop_snapshot_view: Option<wgpu::TextureView>,
    pub(super) blur_ping_texture: Option<wgpu::Texture>,
    pub(super) blur_ping_view: Option<wgpu::TextureView>,
    pub(super) blur_pong_texture: Option<wgpu::Texture>,
    pub(super) blur_pong_view: Option<wgpu::TextureView>,
    pub(super) filter_group_textures: Vec<wgpu::Texture>,
    pub(super) filter_group_views: Vec<wgpu::TextureView>,
}

#[derive(Default)]
struct BlurBindGroups {
    uniform_generation: u64,
    groups: FxHashMap<wgpu::TextureView, wgpu::BindGroup>,
}

pub(super) struct ResourceMetadata {
    pub(super) globals: GlobalBufferLayout,
    pub(super) last_error: Arc<Mutex<Option<String>>>,
}

pub(super) struct GlobalBufferLayout {
    pub(super) path_offset: u64,
    pub(super) font_offset: u64,
    pub(super) maximum_uniform_buffer_size: u64,
}

impl WgpuResources {
    pub(super) fn new(
        context: &WgpuContext,
        surface: Option<wgpu::Surface<'static>>,
        surface_config: &wgpu::SurfaceConfiguration,
        rendering: &RenderingParameters,
        dual_source_blending: bool,
    ) -> anyhow::Result<(Self, ResourceMetadata)> {
        let device = Arc::clone(&context.device);
        let queue = Arc::clone(&context.queue);
        let renderer_tier = context.renderer_tier();
        let bind_group_layouts = WgpuBindGroupLayouts::new(&device, renderer_tier);
        let pipelines = WgpuPipelines::new(
            &device,
            &bind_group_layouts,
            surface_config.format,
            surface_config.alpha_mode,
            rendering.path_sample_count,
            dual_source_blending,
            renderer_tier,
        );
        let linear_sampler = |label| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
        };
        let atlas_sampler = linear_sampler("atlas_sampler");
        let surface_sampler = linear_sampler("surface_sampler");

        let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let surface_uniforms = DynamicUniformBuffer::new(
            &device,
            "surface_uniforms",
            INITIAL_SURFACE_UNIFORM_CAPACITY,
            uniform_alignment,
        );
        let filter_uniforms = DynamicUniformBuffer::new(
            &device,
            "filter_uniforms",
            INITIAL_FILTER_UNIFORM_CAPACITY,
            uniform_alignment,
        );
        let globals_size = std::mem::size_of::<GlobalUniforms>() as u64;
        let gamma_size = std::mem::size_of::<FontRasterizationUniforms>() as u64;
        let path_globals_offset = globals_size.next_multiple_of(uniform_alignment);
        let gamma_offset = (path_globals_offset + globals_size).next_multiple_of(uniform_alignment);
        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals_buffer"),
            size: gamma_offset + gamma_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let create_globals = |label, offset| {
            bind_group_layouts.create_globals(
                &device,
                label,
                wgpu::BufferBinding {
                    buffer: &globals_buffer,
                    offset,
                    size: NonZeroU64::new(globals_size),
                },
                wgpu::BufferBinding {
                    buffer: &globals_buffer,
                    offset: gamma_offset,
                    size: NonZeroU64::new(gamma_size),
                },
            )
        };
        let globals_bind_group = create_globals("globals_bind_group", 0);
        let path_globals_bind_group =
            create_globals("path_globals_bind_group", path_globals_offset);
        let last_error = context.uncaptured_error_slot();
        let surface_cache = SurfaceCache::new(&device)?;

        let metadata = ResourceMetadata {
            globals: GlobalBufferLayout {
                path_offset: path_globals_offset,
                font_offset: gamma_offset,
                maximum_uniform_buffer_size: device.limits().max_buffer_size.min(u32::MAX as u64),
            },
            last_error,
        };
        let resources = Self {
            instances: InstanceBufferArena::new(&device, &bind_group_layouts, renderer_tier),
            renderer_tier,
            device,
            queue,
            surface,
            pipelines,
            bind_group_layouts,
            atlas_sampler,
            surface_sampler,
            surface_uniforms,
            surface_cache: RefCell::new(surface_cache),
            filter_uniforms,
            blur_bind_groups: RefCell::default(),
            globals_buffer,
            globals_bind_group,
            path_globals_bind_group,
            path_intermediate_texture: None,
            path_intermediate_view: None,
            path_msaa_texture: None,
            path_msaa_view: None,
            scene_color_texture: None,
            scene_color_view: None,
            backdrop_snapshot_texture: None,
            backdrop_snapshot_view: None,
            blur_ping_texture: None,
            blur_ping_view: None,
            blur_pong_texture: None,
            blur_pong_view: None,
            filter_group_textures: Vec::new(),
            filter_group_views: Vec::new(),
        };
        Ok((resources, metadata))
    }

    pub(super) fn invalidate_intermediate_textures(&mut self) {
        self.instances.invalidate_texture_bindings();
        self.blur_bind_groups.get_mut().groups.clear();
        self.path_intermediate_texture = None;
        self.path_intermediate_view = None;
        self.path_msaa_texture = None;
        self.path_msaa_view = None;
        self.scene_color_texture = None;
        self.scene_color_view = None;
        self.backdrop_snapshot_texture = None;
        self.backdrop_snapshot_view = None;
        self.blur_ping_texture = None;
        self.blur_ping_view = None;
        self.blur_pong_texture = None;
        self.blur_pong_view = None;
        self.filter_group_textures.clear();
        self.filter_group_views.clear();
    }

    pub(super) fn finish_frame_uploads(&self) {
        self.filter_uniforms.finish_upload();
        self.surface_uniforms.finish_upload();
    }

    pub(super) fn blur_bind_group(&self, source: &wgpu::TextureView) -> wgpu::BindGroup {
        let mut cache = self.blur_bind_groups.borrow_mut();
        let uniform_generation = self.filter_uniforms.generation();
        if cache.uniform_generation != uniform_generation {
            cache.groups.clear();
            cache.uniform_generation = uniform_generation;
        }
        cache
            .groups
            .entry(source.clone())
            .or_insert_with(|| {
                self.bind_group_layouts.create_blur(
                    &self.device,
                    wgpu::BufferBinding {
                        buffer: &self.filter_uniforms.buffer,
                        offset: 0,
                        size: NonZeroU64::new(std::mem::size_of::<BlurUniforms>() as u64),
                    },
                    source,
                    &self.surface_sampler,
                )
            })
            .clone()
    }
}

impl WgpuRenderer {
    pub(super) fn ensure_path_textures(&mut self) {
        if self.resources().path_intermediate_texture.is_some() {
            return;
        }
        let format = self.target.format();
        let width = self.target.width();
        let height = self.target.height();
        let sample_count = self.rendering_params.path_sample_count;
        let resources = self.resources_mut();
        let (texture, view) = sampled_render_texture(&resources.device, format, width, height);
        resources.path_intermediate_texture = Some(texture);
        resources.path_intermediate_view = Some(view);
        if let Some((texture, view)) =
            msaa_texture(&resources.device, format, width, height, sample_count)
        {
            resources.path_msaa_texture = Some(texture);
            resources.path_msaa_view = Some(view);
        }
    }

    pub(super) fn ensure_filter_textures(&mut self, requirements: &ScenePlanRequirements) {
        let format = self.target.format();
        let width = self.target.width();
        let height = self.target.height();
        let blur_width = downsampled_dimension(width);
        let blur_height = downsampled_dimension(height);
        let resources = self.resources_mut();

        if resources.scene_color_texture.is_none() {
            let (texture, view) = sampled_render_texture(&resources.device, format, width, height);
            resources.scene_color_texture = Some(texture);
            resources.scene_color_view = Some(view);
        }

        if requirements.uses_blur_target && resources.blur_ping_texture.is_none() {
            let (texture, view) =
                sampled_render_texture(&resources.device, format, blur_width, blur_height);
            resources.blur_ping_texture = Some(texture);
            resources.blur_ping_view = Some(view);
            let (texture, view) =
                sampled_render_texture(&resources.device, format, blur_width, blur_height);
            resources.blur_pong_texture = Some(texture);
            resources.blur_pong_view = Some(view);
        }

        if requirements.backdrop_filter_count > 0
            && requirements.isolated_target_count > 0
            && resources.backdrop_snapshot_texture.is_none()
        {
            let (texture, view) = sampled_render_texture(&resources.device, format, width, height);
            resources.backdrop_snapshot_texture = Some(texture);
            resources.backdrop_snapshot_view = Some(view);
        }

        while resources.filter_group_views.len() < requirements.isolated_target_count {
            let (texture, view) = sampled_render_texture(&resources.device, format, width, height);
            resources.filter_group_textures.push(texture);
            resources.filter_group_views.push(view);
        }
    }

    pub(super) fn ensure_uniform_capacity(
        &mut self,
        requirements: FrameUniformRequirements,
    ) -> bool {
        let maximum_buffer_size = self.globals.maximum_uniform_buffer_size;
        let resources = self.resources_mut();
        let filters = resources.filter_uniforms.ensure_capacity(
            &resources.device,
            requirements.filter_count,
            maximum_buffer_size,
        );
        let surfaces = resources.surface_uniforms.ensure_capacity(
            &resources.device,
            requirements.surface_count,
            maximum_buffer_size,
        );
        let capacity_available = filters && surfaces;
        if !capacity_available {
            log::error!(
                "scene uniform data exceeds the GPU buffer limit: {} filter uniforms and {} surface uniforms",
                requirements.filter_count,
                requirements.surface_count,
            );
            return false;
        }
        let filters = resources
            .filter_uniforms
            .begin_upload(&resources.queue, requirements.filter_count);
        let surfaces = resources
            .surface_uniforms
            .begin_upload(&resources.queue, requirements.surface_count);
        if !(filters && surfaces) {
            resources.finish_frame_uploads();
            log::error!("failed to map frame uniform staging memory");
            return false;
        }
        true
    }
}

fn sampled_render_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sampled_render_texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn msaa_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    sample_count: u32,
) -> Option<(wgpu::Texture, wgpu::TextureView)> {
    if sample_count <= 1 {
        return None;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("path_msaa"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Some((texture, view))
}
