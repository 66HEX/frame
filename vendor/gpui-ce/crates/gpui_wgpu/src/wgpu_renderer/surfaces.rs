use gpui::PaintSurface;
#[cfg(any(
    all(target_family = "wasm", feature = "custom-gpu"),
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    all(target_os = "windows", feature = "wgpu-surfaces")
))]
use gpui_render::shaders::{
    common::SurfaceColorFormat, interface as shader_interface, surface::SurfaceUniforms,
};

use super::{WgpuRenderer, frame};

#[cfg(target_os = "macos")]
#[path = "surfaces/macos.rs"]
mod platform;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    all(target_family = "wasm", feature = "custom-gpu")
))]
#[path = "surfaces/texture.rs"]
mod platform;
#[cfg(all(target_os = "windows", feature = "wgpu-surfaces"))]
#[path = "surfaces/windows.rs"]
mod platform;
#[cfg(not(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    all(target_family = "wasm", feature = "custom-gpu"),
    all(target_os = "windows", feature = "wgpu-surfaces")
)))]
#[path = "surfaces/unsupported.rs"]
mod platform;

pub(super) use platform::SurfaceCache;

#[cfg(any(
    all(target_family = "wasm", feature = "custom-gpu"),
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    all(target_os = "windows", feature = "wgpu-surfaces")
))]
struct SurfaceBinding {
    color_view: wgpu::TextureView,
    chroma_view: wgpu::TextureView,
    uniform_generation: u64,
    bind_group: wgpu::BindGroup,
}

#[cfg(any(
    all(target_family = "wasm", feature = "custom-gpu"),
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    all(target_os = "windows", feature = "wgpu-surfaces")
))]
impl SurfaceBinding {
    fn new(
        renderer: &WgpuRenderer,
        color_view: wgpu::TextureView,
        chroma_view: wgpu::TextureView,
    ) -> Self {
        Self {
            bind_group: renderer.create_surface_bind_group(&color_view, &chroma_view),
            uniform_generation: renderer.resources().surface_uniforms.generation(),
            color_view,
            chroma_view,
        }
    }

    fn refresh(&mut self, renderer: &WgpuRenderer) {
        let generation = renderer.resources().surface_uniforms.generation();
        if self.uniform_generation != generation {
            self.bind_group =
                renderer.create_surface_bind_group(&self.color_view, &self.chroma_view);
            self.uniform_generation = generation;
        }
    }
}

impl WgpuRenderer {
    pub(super) fn retain_surface_cache(&self, surfaces: &[PaintSurface]) {
        platform::retain_surface_cache(self, surfaces);
    }

    #[cfg(any(
        all(target_family = "wasm", feature = "custom-gpu"),
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        all(target_os = "windows", feature = "wgpu-surfaces")
    ))]
    fn create_surface_bind_group(
        &self,
        color_view: &wgpu::TextureView,
        chroma_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        let resources = self.resources();
        resources.bind_group_layouts.create_surface(
            &resources.device,
            wgpu::BufferBinding {
                buffer: &resources.surface_uniforms.buffer,
                offset: 0,
                size: std::num::NonZeroU64::new(std::mem::size_of::<SurfaceUniforms>() as u64),
            },
            color_view,
            chroma_view,
            &resources.surface_sampler,
        )
    }

    #[cfg(any(
        all(target_family = "wasm", feature = "custom-gpu"),
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        all(target_os = "windows", feature = "wgpu-surfaces")
    ))]
    fn draw_surface_binding(
        &self,
        surface: &PaintSurface,
        color_format: SurfaceColorFormat,
        binding: &mut SurfaceBinding,
        pass: &mut wgpu::RenderPass<'_>,
    ) -> frame::DrawResult {
        binding.refresh(self);
        let uniforms = SurfaceUniforms {
            bounds: surface.bounds.into(),
            content_mask: surface.content_mask.into(),
            color_format,
            opacity: 1.0,
            padding0: 0,
            padding1: 0,
            padding2: 0,
            padding3: 0,
            padding4: 0,
            padding5: 0,
        };
        let resources = self.resources();
        let uniform_offset = resources.surface_uniforms.write(&uniforms);

        pass.set_pipeline(&resources.pipelines.surfaces);
        pass.set_bind_group(
            shader_interface::DATA_BIND_GROUP,
            &binding.bind_group,
            &[uniform_offset],
        );
        pass.draw(0..resources.pipelines.surfaces.fixed_vertex_count(), 0..1);
        Ok(())
    }

    pub(super) fn draw_surfaces(
        &self,
        surfaces: &[PaintSurface],
        pass: &mut wgpu::RenderPass<'_>,
    ) -> frame::DrawResult {
        platform::draw_surfaces(self, surfaces, pass)
    }
}
