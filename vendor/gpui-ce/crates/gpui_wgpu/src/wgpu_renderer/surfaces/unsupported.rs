use super::*;

pub(in crate::wgpu_renderer) struct SurfaceCache;

impl SurfaceCache {
    pub(in crate::wgpu_renderer) fn new(_device: &wgpu::Device) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

pub(super) fn retain_surface_cache(_renderer: &WgpuRenderer, _surfaces: &[PaintSurface]) {}

pub(super) fn draw_surfaces(
    _renderer: &WgpuRenderer,
    surfaces: &[PaintSurface],
    _pass: &mut wgpu::RenderPass<'_>,
) -> frame::DrawResult {
    if surfaces.is_empty() {
        Ok(())
    } else {
        log::error!("native surface primitives are unsupported on this render target");
        Err(frame::DrawError::ExternalSurface)
    }
}
