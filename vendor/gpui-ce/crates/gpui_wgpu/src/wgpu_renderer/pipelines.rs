use crate::RendererTier;
use gpui_render::{
    artifacts::{
        BASE_DOWNLEVEL_WGSL, BASE_WGSL, BLUR_BINDINGS, DOWNLEVEL_BLUR_BINDINGS,
        DOWNLEVEL_INSTANCE_BINDINGS, DOWNLEVEL_RANGE_BINDING, DOWNLEVEL_SURFACE_BINDINGS,
        DOWNLEVEL_TEXTURED_INSTANCE_BINDINGS, GLOBAL_BINDINGS, GeneratedBinding,
        GeneratedBindingKind, INSTANCE_BINDINGS, MONOCHROME_INSTANCE_BINDINGS,
        SUBPIXEL_DUAL_SOURCE_WGSL, SUBPIXEL_INSTANCE_BINDINGS, SURFACE_BINDINGS,
        TEXTURED_INSTANCE_BINDINGS,
    },
    shaders::interface as shader,
};
use std::num::NonZeroU64;

/// Group-1 payload: a storage buffer on modern tiers, a data texture plus per-batch
/// range uniform on downlevel.
pub(super) enum InstanceBindingSource<'a> {
    Buffer(wgpu::BufferBinding<'a>),
    DataTexture {
        texture: &'a wgpu::TextureView,
        range_uniforms: wgpu::BufferBinding<'a>,
    },
}

fn instance_binding_entries(source: InstanceBindingSource<'_>) -> Vec<wgpu::BindGroupEntry<'_>> {
    match source {
        InstanceBindingSource::Buffer(instances) => {
            vec![wgpu::BindGroupEntry {
                binding: shader::DATA_BUFFER_BINDING,
                resource: wgpu::BindingResource::Buffer(instances),
            }]
        }
        InstanceBindingSource::DataTexture {
            texture,
            range_uniforms,
        } => {
            vec![
                wgpu::BindGroupEntry {
                    binding: shader::DATA_BUFFER_BINDING,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: DOWNLEVEL_RANGE_BINDING,
                    resource: wgpu::BindingResource::Buffer(range_uniforms),
                },
            ]
        }
    }
}

pub(super) struct WgpuPipelines {
    pub(super) quads: WgpuRenderPipeline,
    pub(super) smoothed_quads: WgpuRenderPipeline,
    pub(super) shadows: WgpuRenderPipeline,
    pub(super) smoothed_shadows: WgpuRenderPipeline,
    pub(super) path_rasterization: WgpuRenderPipeline,
    pub(super) paths: WgpuRenderPipeline,
    pub(super) underlines: WgpuRenderPipeline,
    pub(super) monochrome_sprites: WgpuRenderPipeline,
    pub(super) subpixel_sprites: Option<WgpuRenderPipeline>,
    pub(super) polychrome_sprites: WgpuRenderPipeline,
    pub(super) smoothed_polychrome_sprites: WgpuRenderPipeline,
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
    pub(super) surfaces: WgpuRenderPipeline,
    pub(super) blur_downsample: WgpuRenderPipeline,
    pub(super) blur: WgpuRenderPipeline,
    pub(super) blur_composite: WgpuRenderPipeline,
    pub(super) smoothed_blur_composite: WgpuRenderPipeline,
}

pub(super) struct WgpuRenderPipeline {
    raw: wgpu::RenderPipeline,
    specification: shader::Pipeline,
}

impl WgpuRenderPipeline {
    pub(super) fn data_layout(&self) -> shader::DataLayout {
        self.specification.data_layout
    }

    pub(super) fn fixed_vertex_count(&self) -> u32 {
        self.specification.vertex_count.fixed().unwrap_or_else(|| {
            panic!(
                "pipeline {} uses a workload-defined vertex count",
                self.specification.label
            )
        })
    }
}

impl std::ops::Deref for WgpuRenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

pub(super) struct WgpuBindGroupLayouts {
    pub(super) globals: wgpu::BindGroupLayout,
    pub(super) instances: wgpu::BindGroupLayout,
    monochrome_sprites: wgpu::BindGroupLayout,
    subpixel_sprites: wgpu::BindGroupLayout,
    textured_instances: wgpu::BindGroupLayout,
    pub(super) surfaces: wgpu::BindGroupLayout,
    pub(super) blur: wgpu::BindGroupLayout,
}

impl WgpuBindGroupLayouts {
    pub(super) fn new(device: &wgpu::Device, tier: RendererTier) -> Self {
        let (
            instance_table,
            monochrome_table,
            subpixel_table,
            textured_table,
            surface_table,
            blur_table,
            instance_dynamic,
        ) = match tier {
            RendererTier::Modern => (
                INSTANCE_BINDINGS,
                MONOCHROME_INSTANCE_BINDINGS,
                SUBPIXEL_INSTANCE_BINDINGS,
                TEXTURED_INSTANCE_BINDINGS,
                SURFACE_BINDINGS,
                BLUR_BINDINGS,
                None,
            ),
            RendererTier::WebGl2 => (
                DOWNLEVEL_INSTANCE_BINDINGS,
                DOWNLEVEL_TEXTURED_INSTANCE_BINDINGS,
                DOWNLEVEL_TEXTURED_INSTANCE_BINDINGS,
                DOWNLEVEL_TEXTURED_INSTANCE_BINDINGS,
                DOWNLEVEL_SURFACE_BINDINGS,
                DOWNLEVEL_BLUR_BINDINGS,
                Some(DOWNLEVEL_RANGE_BINDING),
            ),
        };
        let globals = generated_bind_group_layout(device, "globals_layout", GLOBAL_BINDINGS, None);
        let instances = generated_bind_group_layout(
            device,
            "instances_layout",
            instance_table,
            instance_dynamic,
        );
        let monochrome_sprites = generated_bind_group_layout(
            device,
            "monochrome_sprites_layout",
            monochrome_table,
            instance_dynamic,
        );
        let subpixel_sprites = generated_bind_group_layout(
            device,
            "subpixel_sprites_layout",
            subpixel_table,
            instance_dynamic,
        );
        let textured_instances = generated_bind_group_layout(
            device,
            "textured_instances_layout",
            textured_table,
            instance_dynamic,
        );
        let surfaces = generated_bind_group_layout(
            device,
            "surfaces_layout",
            surface_table,
            Some(shader::DATA_BUFFER_BINDING),
        );
        let blur = generated_bind_group_layout(
            device,
            "blur_layout",
            blur_table,
            Some(shader::DATA_BUFFER_BINDING),
        );
        Self {
            globals,
            instances,
            monochrome_sprites,
            subpixel_sprites,
            textured_instances,
            surfaces,
            blur,
        }
    }

    pub(super) fn create_globals(
        &self,
        device: &wgpu::Device,
        label: &str,
        globals: wgpu::BufferBinding,
        font_rasterization: wgpu::BufferBinding,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.globals,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: shader::GLOBAL_UNIFORMS_BINDING,
                    resource: wgpu::BindingResource::Buffer(globals),
                },
                wgpu::BindGroupEntry {
                    binding: shader::FONT_RASTERIZATION_BINDING,
                    resource: wgpu::BindingResource::Buffer(font_rasterization),
                },
            ],
        })
    }

    /// Creates the group-1 data bind group for the tier's binding source.
    pub(super) fn create_instances(
        &self,
        device: &wgpu::Device,
        source: InstanceBindingSource<'_>,
    ) -> wgpu::BindGroup {
        let entries = instance_binding_entries(source);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("instances"),
            layout: &self.instances,
            entries: &entries,
        })
    }

    pub(super) fn create_textured_instances(
        &self,
        device: &wgpu::Device,
        data_layout: shader::DataLayout,
        source: InstanceBindingSource<'_>,
        texture: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let mut entries = instance_binding_entries(source);
        entries.push(wgpu::BindGroupEntry {
            binding: shader::PRIMARY_TEXTURE_BINDING,
            resource: wgpu::BindingResource::TextureView(texture),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: shader::PRIMARY_SAMPLER_BINDING,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("textured_instances"),
            layout: match data_layout {
                shader::DataLayout::MonochromeSprites => &self.monochrome_sprites,
                shader::DataLayout::SubpixelSprites => &self.subpixel_sprites,
                shader::DataLayout::TexturedInstances => &self.textured_instances,
                _ => panic!("{data_layout:?} does not use a textured instance layout"),
            },
            entries: &entries,
        })
    }

    #[cfg(any(
        all(target_family = "wasm", feature = "custom-gpu"),
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        all(target_os = "windows", feature = "wgpu-surfaces")
    ))]
    pub(super) fn create_surface(
        &self,
        device: &wgpu::Device,
        uniforms: wgpu::BufferBinding,
        color: &wgpu::TextureView,
        chroma: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("surface"),
            layout: &self.surfaces,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: shader::DATA_BUFFER_BINDING,
                    resource: wgpu::BindingResource::Buffer(uniforms),
                },
                wgpu::BindGroupEntry {
                    binding: shader::PRIMARY_TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(color),
                },
                wgpu::BindGroupEntry {
                    binding: shader::SECONDARY_TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(chroma),
                },
                wgpu::BindGroupEntry {
                    binding: shader::SURFACE_SAMPLER_BINDING,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub(super) fn create_blur(
        &self,
        device: &wgpu::Device,
        uniforms: wgpu::BufferBinding,
        texture: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur"),
            layout: &self.blur,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: shader::DATA_BUFFER_BINDING,
                    resource: wgpu::BindingResource::Buffer(uniforms),
                },
                wgpu::BindGroupEntry {
                    binding: shader::PRIMARY_TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: shader::PRIMARY_SAMPLER_BINDING,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

impl WgpuPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        bind_group_layouts: &WgpuBindGroupLayouts,
        surface_format: wgpu::TextureFormat,
        alpha_mode: wgpu::CompositeAlphaMode,
        path_sample_count: u32,
        dual_source_blending: bool,
        tier: RendererTier,
    ) -> Self {
        let dual_source_blending = tier == RendererTier::Modern
            && supported_dual_source_blending(device, dual_source_blending);
        let base_source = match tier {
            RendererTier::Modern => BASE_WGSL,
            // GLES lacks vertex-stage storage; the dialect reads the data texture.
            RendererTier::WebGl2 => BASE_DOWNLEVEL_WGSL,
        };
        let shader_module = create_shader_module(device, "gpui_shaders", base_source);
        let subpixel_shader_module = dual_source_blending.then(|| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("gpui_subpixel_shaders"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                    SUBPIXEL_DUAL_SOURCE_WGSL,
                )),
            })
        });

        let instance_layout = create_pipeline_layout(
            device,
            "instance_pipeline_layout",
            bind_group_layouts,
            &bind_group_layouts.instances,
        );
        let monochrome_layout = create_pipeline_layout(
            device,
            "monochrome_pipeline_layout",
            bind_group_layouts,
            &bind_group_layouts.monochrome_sprites,
        );
        let subpixel_layout = create_pipeline_layout(
            device,
            "subpixel_pipeline_layout",
            bind_group_layouts,
            &bind_group_layouts.subpixel_sprites,
        );
        let textured_layout = create_pipeline_layout(
            device,
            "textured_pipeline_layout",
            bind_group_layouts,
            &bind_group_layouts.textured_instances,
        );
        let surface_layout = create_pipeline_layout(
            device,
            "surface_pipeline_layout",
            bind_group_layouts,
            &bind_group_layouts.surfaces,
        );
        let blur_layout = create_pipeline_layout(
            device,
            "blur_pipeline_layout",
            bind_group_layouts,
            &bind_group_layouts.blur,
        );

        let scene_target = color_target(surface_format, Some(scene_blend_state(alpha_mode)));
        let path_rasterization_target = color_target(
            surface_format,
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let path_target = color_target(surface_format, Some(path_blend_state()));
        let overwrite_target = color_target(surface_format, None);
        let composite_target =
            color_target(surface_format, Some(premultiplied_composite_blend_state()));

        let layout_for = |data_layout| match data_layout {
            shader::DataLayout::Instances => &instance_layout,
            shader::DataLayout::TexturedInstances => &textured_layout,
            shader::DataLayout::MonochromeSprites => &monochrome_layout,
            shader::DataLayout::SubpixelSprites => &subpixel_layout,
            shader::DataLayout::NativeOnly => {
                panic!("native-only shader pipeline cannot be created by wgpu")
            }
            shader::DataLayout::Surface => &surface_layout,
            shader::DataLayout::Blur => &blur_layout,
        };
        let create =
            |specification: shader::Pipeline, target: &wgpu::ColorTargetState, samples, module| {
                create_render_pipeline(
                    device,
                    specification,
                    layout_for(specification.data_layout),
                    target,
                    samples,
                    module,
                )
            };

        Self {
            quads: create(shader::QUADS, &scene_target, 1, &shader_module),
            smoothed_quads: create(shader::SMOOTHED_QUADS, &scene_target, 1, &shader_module),
            shadows: create(shader::SHADOWS, &scene_target, 1, &shader_module),
            smoothed_shadows: create(shader::SMOOTHED_SHADOWS, &scene_target, 1, &shader_module),
            path_rasterization: create(
                shader::PATH_RASTERIZATION,
                &path_rasterization_target,
                path_sample_count,
                &shader_module,
            ),
            paths: create(shader::PATHS, &path_target, 1, &shader_module),
            underlines: create(shader::UNDERLINES, &scene_target, 1, &shader_module),
            monochrome_sprites: create(
                shader::MONOCHROME_SPRITES,
                &scene_target,
                1,
                &shader_module,
            ),
            subpixel_sprites: subpixel_shader_module.as_ref().map(|module| {
                create(
                    shader::SUBPIXEL_SPRITES,
                    &wgpu::ColorTargetState {
                        write_mask: wgpu::ColorWrites::COLOR,
                        ..color_target(surface_format, Some(subpixel_blend_state()))
                    },
                    1,
                    module,
                )
            }),
            polychrome_sprites: create(
                shader::POLYCHROME_SPRITES,
                &scene_target,
                1,
                &shader_module,
            ),
            smoothed_polychrome_sprites: create(
                shader::SMOOTHED_POLYCHROME_SPRITES,
                &scene_target,
                1,
                &shader_module,
            ),
            surfaces: create(shader::SURFACES, &composite_target, 1, &shader_module),
            blur_downsample: create(
                shader::BLUR_DOWNSAMPLE,
                &overwrite_target,
                1,
                &shader_module,
            ),
            blur: create(shader::BLUR, &overwrite_target, 1, &shader_module),
            blur_composite: create(shader::BLUR_COMPOSITE, &composite_target, 1, &shader_module),
            smoothed_blur_composite: create(
                shader::SMOOTHED_BLUR_COMPOSITE,
                &composite_target,
                1,
                &shader_module,
            ),
        }
    }
}

fn generated_bind_group_layout(
    device: &wgpu::Device,
    label: &str,
    bindings: &[GeneratedBinding],
    dynamic_uniform_binding: Option<u32>,
) -> wgpu::BindGroupLayout {
    let entries = bindings
        .iter()
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding: binding.binding,
            visibility: wgpu::ShaderStages::from_bits_retain(binding.visibility),
            ty: match binding.kind {
                GeneratedBindingKind::Uniform(min_size) => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: dynamic_uniform_binding == Some(binding.binding),
                    min_binding_size: NonZeroU64::new(min_size),
                },
                GeneratedBindingKind::StorageRead(min_size) => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(min_size),
                },
                GeneratedBindingKind::Texture2dFloat => wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                GeneratedBindingKind::FilteringSampler => {
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                }
                GeneratedBindingKind::DataTexture => wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                GeneratedBindingKind::RangeUniform => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    // The generated `DATA_RANGE` batch base, padded to satisfy
                    // browser/WebGL downlevel uniform-binding alignment.
                    min_binding_size: NonZeroU64::new(16),
                },
            },
            count: None,
        })
        .collect::<Vec<_>>();

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &entries,
    })
}

fn create_shader_module(
    device: &wgpu::Device,
    label: &str,
    source: &'static str,
) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source)),
    })
}

fn create_pipeline_layout(
    device: &wgpu::Device,
    label: &str,
    layouts: &WgpuBindGroupLayouts,
    data_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(&layouts.globals), Some(data_layout)],
        immediate_size: 0,
    })
}

fn create_render_pipeline(
    device: &wgpu::Device,
    specification: shader::Pipeline,
    layout: &wgpu::PipelineLayout,
    color_target: &wgpu::ColorTargetState,
    sample_count: u32,
    module: &wgpu::ShaderModule,
) -> WgpuRenderPipeline {
    let raw = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(specification.label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some(specification.vertex_entry),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(specification.fragment_entry),
            targets: &[Some(color_target.clone())],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: match specification.topology {
                shader::PrimitiveTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
                shader::PrimitiveTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
            },
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    });
    WgpuRenderPipeline { raw, specification }
}

fn color_target(
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn supported_dual_source_blending(device: &wgpu::Device, requested: bool) -> bool {
    let supported = device
        .features()
        .contains(wgpu::Features::DUAL_SOURCE_BLENDING);
    if requested && !supported {
        log::error!(
            "dual-source blending was requested but is absent from device features {:?}; falling \
             back to monochrome text rendering",
            device.features(),
        );
    }
    requested && supported
}

fn scene_blend_state(alpha_mode: wgpu::CompositeAlphaMode) -> wgpu::BlendState {
    let source_color_factor = if alpha_mode == wgpu::CompositeAlphaMode::PreMultiplied {
        wgpu::BlendFactor::One
    } else {
        wgpu::BlendFactor::SrcAlpha
    };
    source_over_blend_state(source_color_factor)
}

fn premultiplied_composite_blend_state() -> wgpu::BlendState {
    source_over_blend_state(wgpu::BlendFactor::One)
}

fn source_over_blend_state(source_color_factor: wgpu::BlendFactor) -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: source_color_factor,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

fn path_blend_state() -> wgpu::BlendState {
    source_over_blend_state(wgpu::BlendFactor::One)
}

fn subpixel_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Src1,
            dst_factor: wgpu::BlendFactor::OneMinusSrc1,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
pub(super) fn desktop_scene_blend_state(alpha_mode: wgpu::CompositeAlphaMode) -> wgpu::BlendState {
    scene_blend_state(alpha_mode)
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::WgpuContext;

    #[test]
    fn generated_layouts_create_every_pipeline_and_bind_group() -> anyhow::Result<()> {
        let context = WgpuContext::new_headless(None)?;
        let device = &context.device;
        let tier = context.renderer_tier();
        let layouts = WgpuBindGroupLayouts::new(device, tier);
        let _pipelines = WgpuPipelines::new(
            device,
            &layouts,
            context.color_texture_format(),
            wgpu::CompositeAlphaMode::Opaque,
            1,
            context.supports_dual_source_blending(),
            tier,
        );

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader_interface_test_buffer"),
            size: 256,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shader_interface_test_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let binding = || wgpu::BufferBinding {
            buffer: &buffer,
            offset: 0,
            size: NonZeroU64::new(256),
        };

        let _globals = layouts.create_globals(device, "test_globals", binding(), binding());
        let _instances = layouts.create_instances(device, InstanceBindingSource::Buffer(binding()));
        let _textured = layouts.create_textured_instances(
            device,
            shader::DataLayout::MonochromeSprites,
            InstanceBindingSource::Buffer(binding()),
            &view,
            &sampler,
        );
        #[cfg(any(
            all(target_family = "wasm", feature = "custom-gpu"),
            target_os = "macos",
            target_os = "linux",
            target_os = "freebsd",
            all(target_os = "windows", feature = "wgpu-surfaces")
        ))]
        let _surface = layouts.create_surface(device, binding(), &view, &view, &sampler);
        let _blur = layouts.create_blur(device, binding(), &view, &sampler);
        Ok(())
    }

    /// The downlevel artifacts must also build and bind against a real device.
    #[test]
    fn downlevel_tier_builds_pipelines_and_data_texture_arena() -> anyhow::Result<()> {
        use super::super::buffers::{InstanceBufferArena, InstanceTransport};
        use gpui::Quad;

        let context = WgpuContext::new_headless(None)?;
        let device = &context.device;
        let tier = crate::RendererTier::WebGl2;
        let layouts = WgpuBindGroupLayouts::new(device, tier);
        let pipelines = WgpuPipelines::new(
            device,
            &layouts,
            context.color_texture_format(),
            wgpu::CompositeAlphaMode::Opaque,
            1,
            false,
            tier,
        );
        assert!(pipelines.quads.fixed_vertex_count() > 0);

        let mut arena = InstanceBufferArena::new(device, &layouts, tier);
        assert_eq!(arena.transport(), InstanceTransport::DataTexture);
        arena
            .ensure_capacity(device, &layouts, 4096, 8)
            .then_some(())
            .expect("small downlevel capacities must always be available");
        let mut upload = arena
            .begin_upload(&context.queue, 4096, 8)
            .expect("mapped downlevel staging");

        let custom_dash = Quad {
            border_dashed_length: 4.0,
            border_dashed_gap: 0.5,
            ..Quad::default()
        };
        let slice = upload
            .write(&[custom_dash, Quad::default()])
            .expect("small batch must fit");
        // Downlevel draws start at instance zero; the base travels in the range offset.
        assert_eq!(slice.range(), 0..2);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("downlevel_test_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let _textured = layouts.create_textured_instances(
            device,
            shader::DataLayout::MonochromeSprites,
            arena.binding_source(),
            &view,
            &sampler,
        );

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        upload.finish(&mut encoder);
        let _commands = encoder.finish();
        Ok(())
    }
}
