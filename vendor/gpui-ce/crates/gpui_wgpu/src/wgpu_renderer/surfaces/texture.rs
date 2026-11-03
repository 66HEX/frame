use super::*;
use collections::FxHashMap;

pub(in crate::wgpu_renderer) struct SurfaceCache {
    textures: FxHashMap<wgpu::Texture, SurfaceBinding>,
}

impl SurfaceCache {
    pub(in crate::wgpu_renderer) fn new(_device: &wgpu::Device) -> anyhow::Result<Self> {
        Ok(Self {
            textures: FxHashMap::default(),
        })
    }
}

pub(super) fn retain_surface_cache(renderer: &WgpuRenderer, surfaces: &[PaintSurface]) {
    let mut textures = smallvec::SmallVec::<[&wgpu::Texture; 4]>::new();
    for surface in surfaces {
        let gpui::SurfaceSource::Texture { texture, .. } = &surface.source else {
            continue;
        };
        if let Some(texture) = texture.downcast_ref::<wgpu::Texture>() {
            textures.push(texture);
        }
    }
    renderer
        .resources()
        .surface_cache
        .borrow_mut()
        .textures
        .retain(|texture, _| textures.contains(&texture));
}

pub(super) fn draw_surfaces(
    renderer: &WgpuRenderer,
    surfaces: &[PaintSurface],
    pass: &mut wgpu::RenderPass<'_>,
) -> frame::DrawResult {
    let mut textures = smallvec::SmallVec::<[(&PaintSurface, &wgpu::Texture); 4]>::new();
    for surface in surfaces {
        let gpui::SurfaceSource::Texture { texture, .. } = &surface.source else {
            log::error!("surface source cannot be imported by the WGPU texture renderer");
            return Err(frame::DrawError::ExternalSurface);
        };
        let Some(texture) = texture.downcast_ref::<wgpu::Texture>() else {
            log::error!("surface source is not a WGPU texture");
            return Err(frame::DrawError::ExternalSurface);
        };
        textures.push((surface, texture));
    }

    let resources = renderer.resources();
    let mut cache = resources.surface_cache.borrow_mut();

    for (surface, texture) in textures {
        if !cache.textures.contains_key(texture) {
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            cache.textures.insert(
                texture.clone(),
                SurfaceBinding::new(renderer, view.clone(), view),
            );
        }
        let Some(cached) = cache.textures.get_mut(texture) else {
            log::error!("surface texture cache insertion failed");
            return Err(frame::DrawError::ExternalSurface);
        };
        renderer.draw_surface_binding(surface, SurfaceColorFormat::Rgba, cached, pass)?;
    }
    Ok(())
}
