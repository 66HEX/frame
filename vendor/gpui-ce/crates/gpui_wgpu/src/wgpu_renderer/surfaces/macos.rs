use super::*;
use collections::FxHashMap;

pub(in crate::wgpu_renderer) struct SurfaceCache {
    texture_cache: core_video::metal_texture_cache::CVMetalTextureCache,
    surfaces: FxHashMap<usize, CachedSurface>,
    /// Bindings for application-rendered WGPU textures.
    #[cfg(feature = "custom-gpu")]
    textures: FxHashMap<wgpu::Texture, SurfaceBinding>,
}

impl SurfaceCache {
    pub(in crate::wgpu_renderer) fn new(device: &wgpu::Device) -> anyhow::Result<Self> {
        use metal::foreign_types::ForeignTypeRef as _;

        let hal_device = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }
            .ok_or_else(|| anyhow::anyhow!("macOS WGPU device did not expose the Metal HAL"))?;
        let raw_device = objc2::rc::Retained::as_ptr(hal_device.raw_device())
            .cast_mut()
            .cast();
        let metal_device = unsafe { metal::DeviceRef::from_ptr(raw_device) }.to_owned();
        let texture_cache =
            core_video::metal_texture_cache::CVMetalTextureCache::new(None, metal_device, None)
                .map_err(|error| {
                    anyhow::anyhow!("failed to create CoreVideo Metal texture cache: {error}")
                })?;
        Ok(Self {
            texture_cache,
            surfaces: FxHashMap::default(),
            #[cfg(feature = "custom-gpu")]
            textures: FxHashMap::default(),
        })
    }
}

struct CachedSurface {
    _luma_texture: wgpu::Texture,
    _chroma_texture: wgpu::Texture,
    binding: SurfaceBinding,
}

pub(super) fn retain_surface_cache(renderer: &WgpuRenderer, surfaces: &[PaintSurface]) {
    let active_keys = surfaces
        .iter()
        .filter_map(|surface| {
            let gpui::SurfaceSource::Surface(image_buffer) = &surface.source else {
                return None;
            };
            core_video_surface_key(image_buffer).ok()
        })
        .collect::<smallvec::SmallVec<[usize; 4]>>();
    let mut cache = renderer.resources().surface_cache.borrow_mut();
    cache.surfaces.retain(|key, _| active_keys.contains(key));
    #[cfg(feature = "custom-gpu")]
    {
        let active_textures = surfaces
            .iter()
            .filter_map(|surface| match &surface.source {
                gpui::SurfaceSource::Texture { texture, .. } => {
                    texture.downcast_ref::<wgpu::Texture>()
                }
                _ => None,
            })
            .collect::<smallvec::SmallVec<[&wgpu::Texture; 4]>>();
        cache
            .textures
            .retain(|texture, _| active_textures.contains(&texture));
    }
}

pub(super) fn draw_surfaces(
    renderer: &WgpuRenderer,
    surfaces: &[PaintSurface],
    pass: &mut wgpu::RenderPass<'_>,
) -> frame::DrawResult {
    use core_video::pixel_buffer::kCVPixelFormatType_420YpCbCr8BiPlanarFullRange;

    let resources = renderer.resources();
    let mut cache = resources.surface_cache.borrow_mut();

    for surface in surfaces {
        match &surface.source {
            gpui::SurfaceSource::Surface(image_buffer) => {
                if image_buffer.get_pixel_format() != kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
                {
                    log::error!("unsupported CoreVideo surface pixel format");
                    return Err(frame::DrawError::ExternalSurface);
                }
                let key = core_video_surface_key(image_buffer)?;
                let mut imported = cache.surfaces.remove(&key).map(Ok).unwrap_or_else(|| {
                    create_core_video_surface(renderer, &cache.texture_cache, image_buffer)
                })?;
                renderer.draw_surface_binding(
                    surface,
                    SurfaceColorFormat::Yuv,
                    &mut imported.binding,
                    pass,
                )?;
                cache.surfaces.insert(key, imported);
            }
            #[cfg(feature = "custom-gpu")]
            gpui::SurfaceSource::Texture { texture, .. } => {
                let Some(texture) = texture.downcast_ref::<wgpu::Texture>() else {
                    log::error!("surface source is not a WGPU texture");
                    return Err(frame::DrawError::ExternalSurface);
                };
                let binding = cache.textures.entry(texture.clone()).or_insert_with(|| {
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    SurfaceBinding::new(renderer, view.clone(), view)
                });
                renderer.draw_surface_binding(surface, SurfaceColorFormat::Rgba, binding, pass)?;
            }
            _ => {
                log::error!("surface source cannot be imported by the macOS renderer");
                return Err(frame::DrawError::ExternalSurface);
            }
        }
    }
    Ok(())
}

fn core_video_surface_key(
    image_buffer: &core_video::pixel_buffer::CVPixelBuffer,
) -> Result<usize, frame::DrawError> {
    use core_foundation::base::TCFType as _;

    let io_surface = unsafe {
        core_video::pixel_buffer_io_surface::CVPixelBufferGetIOSurface(
            image_buffer.as_concrete_TypeRef(),
        )
    };
    if io_surface.is_null() {
        log::error!(
            "CoreVideo surface is not IOSurface-backed; allocate it with \
             kCVPixelBufferIOSurfacePropertiesKey and \
             kCVPixelBufferMetalCompatibilityKey"
        );
        return Err(frame::DrawError::ExternalSurface);
    }
    Ok(io_surface as usize)
}

fn create_core_video_surface(
    renderer: &WgpuRenderer,
    texture_cache: &core_video::metal_texture_cache::CVMetalTextureCache,
    image_buffer: &core_video::pixel_buffer::CVPixelBuffer,
) -> Result<CachedSurface, frame::DrawError> {
    use core_foundation::base::TCFType as _;
    use core_video::{image_buffer::TCVImageBuffer, metal_texture::CVMetalTextureGetTexture};

    let resources = renderer.resources();
    let luma = texture_cache
        .create_texture_from_image(
            &image_buffer.as_image_buffer(),
            None,
            metal::MTLPixelFormat::R8Unorm,
            image_buffer.get_width_of_plane(0),
            image_buffer.get_height_of_plane(0),
            0,
        )
        .map_err(|error| {
            log::error!("failed to create CoreVideo luma texture: {error}");
            frame::DrawError::ExternalSurface
        })?;
    let chroma = texture_cache
        .create_texture_from_image(
            &image_buffer.as_image_buffer(),
            None,
            metal::MTLPixelFormat::RG8Unorm,
            image_buffer.get_width_of_plane(1),
            image_buffer.get_height_of_plane(1),
            1,
        )
        .map_err(|error| {
            log::error!("failed to create CoreVideo chroma texture: {error}");
            frame::DrawError::ExternalSurface
        })?;
    let luma_texture = unsafe {
        import_core_video_texture(
            &resources.device,
            CVMetalTextureGetTexture(luma.as_concrete_TypeRef()).cast(),
            wgpu::TextureFormat::R8Unorm,
            plane_size(image_buffer, 0),
        )
    }
    .ok_or(frame::DrawError::ExternalSurface)?;
    let chroma_texture = unsafe {
        import_core_video_texture(
            &resources.device,
            CVMetalTextureGetTexture(chroma.as_concrete_TypeRef()).cast(),
            wgpu::TextureFormat::Rg8Unorm,
            plane_size(image_buffer, 1),
        )
    }
    .ok_or(frame::DrawError::ExternalSurface)?;
    let luma_view = luma_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let chroma_view = chroma_texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok(CachedSurface {
        _luma_texture: luma_texture,
        _chroma_texture: chroma_texture,
        binding: SurfaceBinding::new(renderer, luma_view, chroma_view),
    })
}

fn plane_size(
    image_buffer: &core_video::pixel_buffer::CVPixelBuffer,
    plane: usize,
) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: image_buffer.get_width_of_plane(plane) as u32,
        height: image_buffer.get_height_of_plane(plane) as u32,
        depth_or_array_layers: 1,
    }
}

unsafe fn import_core_video_texture(
    device: &wgpu::Device,
    raw_texture: *mut objc2::runtime::AnyObject,
    format: wgpu::TextureFormat,
    size: wgpu::Extent3d,
) -> Option<wgpu::Texture> {
    use objc2::{rc::Retained, runtime::ProtocolObject};

    // SAFETY: CoreVideo returned a live MTLTexture; the retain count transfers into the
    // HAL texture so it outlives the CVMetalTexture wrapper.
    let object = unsafe { Retained::retain(raw_texture) }?;
    let raw =
        unsafe { Retained::cast_unchecked::<ProtocolObject<dyn objc2_metal::MTLTexture>>(object) };
    let hal_texture = unsafe {
        wgpu::hal::metal::Device::texture_from_raw(
            raw,
            format,
            objc2_metal::MTLTextureType::Type2D,
            1,
            1,
            size.into(),
        )
    };
    Some(unsafe {
        device.create_texture_from_hal::<wgpu::hal::api::Metal>(
            hal_texture,
            &wgpu::TextureDescriptor {
                label: Some("core_video_surface_plane"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        )
    })
}

#[cfg(all(test, feature = "test-support", feature = "custom-gpu"))]
mod tests {
    use std::sync::Arc;

    use gpui::{
        Bounds, ContentMask, DevicePixels, Point, ScaledPixels, Scene, Size, SurfaceSource,
    };

    use super::*;
    use crate::WgpuContext;

    const RED: [u8; 4] = [255, 0, 0, 255];

    /// A 4x2 renderer: a surface over its left half leaves the right half clear.
    fn renderer() -> anyhow::Result<WgpuRenderer> {
        let context = WgpuContext::new_headless(None)?;
        WgpuRenderer::new_headless(
            &context,
            Size {
                width: DevicePixels(4),
                height: DevicePixels(2),
            },
        )
    }

    fn solid_texture(renderer: &WgpuRenderer, color: [u8; 4]) -> Arc<wgpu::Texture> {
        let (device, queue) = renderer.gpu_context();
        let size = wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("application_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &color.repeat(4),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: Some(2),
            },
            size,
        );
        Arc::new(texture)
    }

    /// A scene drawing each source over the renderer's left half.
    fn scene(sources: impl IntoIterator<Item = SurfaceSource>) -> Scene {
        let bounds = |width| Bounds {
            origin: Point {
                x: ScaledPixels(0.0),
                y: ScaledPixels(0.0),
            },
            size: Size {
                width: ScaledPixels(width),
                height: ScaledPixels(2.0),
            },
        };
        let mut scene = Scene::default();
        for (order, source) in sources.into_iter().enumerate() {
            scene.insert_primitive(PaintSurface {
                order: order as u32,
                bounds: bounds(2.0),
                content_mask: ContentMask {
                    bounds: bounds(4.0),
                },
                source,
            });
        }
        scene.finish();
        scene
    }

    fn texture_source(texture: &Arc<wgpu::Texture>) -> SurfaceSource {
        SurfaceSource::Texture {
            texture: texture.clone(),
            size: Size {
                width: DevicePixels(2),
                height: DevicePixels(2),
            },
        }
    }

    fn cached_textures(renderer: &WgpuRenderer) -> Vec<wgpu::Texture> {
        let cache = renderer.resources().surface_cache.borrow();
        cache.textures.keys().cloned().collect()
    }

    #[test]
    fn application_textures_composite_within_their_bounds() -> anyhow::Result<()> {
        let mut renderer = renderer()?;
        let clear = renderer.render_to_image(&scene([]))?.get_pixel(3, 1).0;
        assert_ne!(clear, RED);
        let texture = solid_texture(&renderer, RED);

        let image = renderer.render_to_image(&scene([texture_source(&texture)]))?;

        assert_eq!(image.get_pixel(0, 0).0, RED);
        assert_eq!(image.get_pixel(1, 1).0, RED);
        assert_eq!(image.get_pixel(2, 0).0, clear);
        assert_eq!(image.get_pixel(3, 1).0, clear);
        Ok(())
    }

    #[test]
    fn texture_bindings_last_only_while_their_texture_is_drawn() -> anyhow::Result<()> {
        let mut renderer = renderer()?;
        let first = solid_texture(&renderer, RED);
        let second = solid_texture(&renderer, RED);

        // One binding per texture, reused across frames.
        renderer.render_to_image(&scene([texture_source(&first)]))?;
        renderer.render_to_image(&scene([texture_source(&first)]))?;
        assert_eq!(cached_textures(&renderer), [(*first).clone()]);

        // A texture the scene stops drawing is released, so an application
        // that replaces its target (on resize) doesn't accumulate them.
        renderer.render_to_image(&scene([texture_source(&second)]))?;
        assert_eq!(cached_textures(&renderer), [(*second).clone()]);
        renderer.render_to_image(&scene([]))?;
        assert_eq!(cached_textures(&renderer), []);
        Ok(())
    }

    #[test]
    fn sources_that_are_not_wgpu_textures_fail_the_frame() -> anyhow::Result<()> {
        let mut renderer = renderer()?;
        let not_a_texture = SurfaceSource::Texture {
            texture: Arc::new("not a texture"),
            size: Size {
                width: DevicePixels(2),
                height: DevicePixels(2),
            },
        };

        assert!(renderer.render_to_image(&scene([not_a_texture])).is_err());
        Ok(())
    }
}
