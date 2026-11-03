//! A minimal custom GPU control.
//!
//! The control owns its buffers, pipeline, and offscreen target. GPUI owns the
//! device and queue, so those resources are recreated after device loss.
//!
//! Run with `--features custom-gpu` on supported targets. macOS windows also
//! need the WGPU renderer:
//!
//! ```sh
//! cargo run -p gpui_ce_wgpu --example custom_gpu --features custom-gpu,gpui_platform/macos-wgpu
//! ```

#[cfg(any(
    target_family = "wasm",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "macos"
))]
mod custom_gpu {
    use std::borrow::Cow;
    #[cfg(target_family = "wasm")]
    use std::cell::RefCell;

    #[cfg(target_family = "wasm")]
    use gpui::ApplicationHandle;
    use gpui::{
        App, AppContext, Bounds, Context, ParentElement, Render, Styled, TitlebarOptions, Window,
        WindowBounds, WindowOptions, div, px, size,
    };
    use gpui_ce_wgpu::{WgpuContextHandle, WgpuRenderTarget};
    use gpui_platform::application;
    use wgpu::util::DeviceExt;

    #[cfg(target_family = "wasm")]
    thread_local! {
        static APPLICATION: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
    }

    const SHADER: &str = r#"
struct Uniforms {
    time: f32,
    aspect: f32,
    pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let angle = uniforms.time;
    let rotation = mat2x2<f32>(
        cos(angle), -sin(angle),
        sin(angle), cos(angle),
    );
    let position = rotation * vec2<f32>(input.position.x / uniforms.aspect, input.position.y);

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Uniforms {
        time: f32,
        aspect: f32,
        _pad: [f32; 2],
    }

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct Vertex {
        position: [f32; 2],
        color: [f32; 3],
    }

    const VERTICES: &[Vertex] = &[
        Vertex {
            position: [0.0, 0.75],
            color: [1.0, 0.2, 0.2],
        },
        Vertex {
            position: [-0.75, -0.5],
            color: [0.2, 1.0, 0.2],
        },
        Vertex {
            position: [0.75, -0.5],
            color: [0.2, 0.4, 1.0],
        },
    ];

    struct CustomGpuControl {
        context: Option<WgpuContextHandle>,
        target: Option<WgpuRenderTarget>,
        pipeline: Option<wgpu::RenderPipeline>,
        bind_group: Option<wgpu::BindGroup>,
        uniform_buffer: Option<wgpu::Buffer>,
        vertex_buffer: Option<wgpu::Buffer>,
        time: f32,
    }

    impl CustomGpuControl {
        fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
            Self {
                context: None,
                target: None,
                pipeline: None,
                bind_group: None,
                uniform_buffer: None,
                vertex_buffer: None,
                time: 0.0,
            }
        }

        fn clear_gpu_resources(&mut self) {
            self.context = None;
            self.target = None;
            self.pipeline = None;
            self.bind_group = None;
            self.uniform_buffer = None;
            self.vertex_buffer = None;
        }

        fn ensure_gpu_resources(&mut self, window: &Window, context: &WgpuContextHandle) {
            let viewport = window.viewport_size();
            let scale_factor = window.scale_factor();
            let target_size = size(
                gpui::DevicePixels((f32::from(viewport.width) * scale_factor).round() as i32),
                gpui::DevicePixels((f32::from(viewport.height) * scale_factor).round() as i32),
            );
            let target_size = size(
                target_size.width.max(gpui::DevicePixels(1)),
                target_size.height.max(gpui::DevicePixels(1)),
            );

            if self
                .target
                .as_ref()
                .is_none_or(|target| target.size() != target_size)
            {
                self.target = Some(WgpuRenderTarget::new(context, target_size));
            }

            if self.pipeline.is_some() {
                return;
            }

            let device = context.device();
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("custom-gpu-shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
            });
            let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("custom-gpu-uniforms"),
                size: std::mem::size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("custom-gpu-bind-group-layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("custom-gpu-bind-group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("custom-gpu-pipeline-layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("custom-gpu-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: std::mem::size_of::<[f32; 2]>() as u64,
                                shader_location: 1,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: context.texture_format(),
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("custom-gpu-vertices"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });

            self.pipeline = Some(pipeline);
            self.bind_group = Some(bind_group);
            self.uniform_buffer = Some(uniform_buffer);
            self.vertex_buffer = Some(vertex_buffer);
        }

        fn render_gpu(&mut self, window: &mut Window) {
            let Some(context) = WgpuContextHandle::from_window(window) else {
                self.clear_gpu_resources();
                return;
            };
            if self
                .context
                .as_ref()
                .is_some_and(|previous| !context.is_same_device(previous))
            {
                self.clear_gpu_resources();
            }
            if context.device_lost() {
                self.clear_gpu_resources();
                return;
            }
            self.context = Some(context.clone());

            self.ensure_gpu_resources(window, &context);
            self.time += 0.02;
            let target = self.target.as_ref().unwrap();
            let uniform_buffer = self.uniform_buffer.as_ref().unwrap();
            let pipeline = self.pipeline.as_ref().unwrap();
            let bind_group = self.bind_group.as_ref().unwrap();
            let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
            let aspect = target.size().width.0 as f32 / target.size().height.0 as f32;
            context.queue().write_buffer(
                uniform_buffer,
                0,
                bytemuck::bytes_of(&Uniforms {
                    time: self.time,
                    aspect,
                    _pad: [0.0; 2],
                }),
            );

            let mut encoder =
                context
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("custom-gpu-encoder"),
                    });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("custom-gpu-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target.view(),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(0..VERTICES.len() as u32, 0..1);
            }
            context.queue().submit([encoder.finish()]);
        }
    }

    impl Render for CustomGpuControl {
        fn render(
            &mut self,
            window: &mut Window,
            _cx: &mut Context<Self>,
        ) -> impl gpui::IntoElement {
            window.request_animation_frame();
            self.render_gpu(window);
            let mut root = div().size_full();
            if let Some(target) = self.target.as_ref() {
                root = root.child(target.surface());
            }
            root
        }
    }

    pub fn run() {
        env_logger::init();
        let application_handle = application().run(|cx: &mut App| {
            cx.open_window(
                WindowOptions::new()
                    .titlebar(Some(TitlebarOptions {
                        title: Some("Custom GPU rendering".into()),
                        ..Default::default()
                    }))
                    .window_bounds(Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(800.0), px(600.0)),
                        cx,
                    )))),
                |window, cx| cx.new(|cx| CustomGpuControl::new(window, cx)),
            )
            .unwrap();
            cx.activate(true);
        });

        #[cfg(target_family = "wasm")]
        APPLICATION.set(Some(application_handle));
        #[cfg(not(target_family = "wasm"))]
        drop(application_handle);
    }
}

#[cfg(any(
    target_family = "wasm",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "macos"
))]
fn main() {
    custom_gpu::run();
}

#[cfg(not(any(
    target_family = "wasm",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "macos"
)))]
fn main() {
    eprintln!("custom_gpu is supported on Linux, FreeBSD, macOS, and WASM targets only");
}
