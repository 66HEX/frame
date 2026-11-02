use super::{
    capture_size, capture_texture_descriptor, source_descriptor, validate_capture_descriptor,
};
use anyhow::{Context as _, Result};
use windows_061::Win32::Graphics::Direct3D11::ID3D11Texture2D;

pub(super) struct UploadedTexture {
    pub(super) size: wgpu::Extent3d,
    context: windows_061::Win32::Graphics::Direct3D11::ID3D11DeviceContext,
    staging: ID3D11Texture2D,
    pixels: Vec<u8>,
    texture: wgpu::Texture,
}

impl UploadedTexture {
    pub(super) fn new(
        device: &wgpu::Device,
        source: &ID3D11Texture2D,
    ) -> Result<(Self, wgpu::Texture)> {
        use windows_061::Win32::Graphics::Direct3D11::{
            D3D11_CPU_ACCESS_READ, D3D11_USAGE_STAGING,
        };

        let mut descriptor = source_descriptor(source);
        validate_capture_descriptor(&descriptor)?;
        descriptor.Usage = D3D11_USAGE_STAGING;
        descriptor.BindFlags = 0;
        descriptor.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        descriptor.MiscFlags = 0;
        let size = capture_size(&descriptor);

        let source_device =
            unsafe { source.GetDevice() }.context("getting capture D3D11 device")?;
        let context = unsafe { source_device.GetImmediateContext() }
            .context("getting capture D3D11 context")?;
        let mut staging = None;
        unsafe {
            source_device.CreateTexture2D(&descriptor, None, Some(std::ptr::addr_of_mut!(staging)))
        }
        .context("creating D3D11 capture staging texture")?;
        let staging = staging.context("D3D11 returned no staging texture")?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            ..capture_texture_descriptor("windows_capture_uploaded", size)
        });
        Ok((
            Self {
                size,
                context,
                staging,
                pixels: vec![0; size.width as usize * size.height as usize * 4],
                texture: texture.clone(),
            },
            texture,
        ))
    }

    pub(super) fn update(&mut self, queue: &wgpu::Queue, source: &ID3D11Texture2D) -> Result<()> {
        unsafe { self.context.CopyResource(&self.staging, source) };
        let mapped = MappedTexture::new(&self.context, &self.staging)?;
        let bytes_per_row = self.size.width as usize * 4;
        for row in 0..self.size.height as usize {
            let source = mapped.row(row, bytes_per_row);
            self.pixels[row * bytes_per_row..(row + 1) * bytes_per_row].copy_from_slice(source);
        }
        drop(mapped);

        queue.write_texture(
            self.texture.as_image_copy(),
            &self.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row as u32),
                rows_per_image: Some(self.size.height),
            },
            self.size,
        );
        Ok(())
    }
}

struct MappedTexture<'a> {
    context: &'a windows_061::Win32::Graphics::Direct3D11::ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
    mapped: windows_061::Win32::Graphics::Direct3D11::D3D11_MAPPED_SUBRESOURCE,
}

impl<'a> MappedTexture<'a> {
    fn new(
        context: &'a windows_061::Win32::Graphics::Direct3D11::ID3D11DeviceContext,
        texture: &'a ID3D11Texture2D,
    ) -> Result<Self> {
        use windows_061::Win32::Graphics::Direct3D11::{D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE};

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe { context.Map(texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }
            .context("mapping D3D11 capture texture")?;
        Ok(Self {
            context,
            texture,
            mapped,
        })
    }

    fn row(&self, row: usize, bytes_per_row: usize) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.mapped
                    .pData
                    .cast::<u8>()
                    .add(row * self.mapped.RowPitch as usize),
                bytes_per_row,
            )
        }
    }
}

impl Drop for MappedTexture<'_> {
    fn drop(&mut self) {
        unsafe { self.context.Unmap(self.texture, 0) };
    }
}
