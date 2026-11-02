use super::{
    capture_size, capture_texture_descriptor, source_descriptor, validate_capture_descriptor,
};
use anyhow::{Context as _, Result};
use windows_061::{
    Win32::Graphics::Direct3D11::{ID3D11DeviceContext4, ID3D11Fence, ID3D11Texture2D},
    core::Interface as _,
};

pub(super) struct SharedTexture {
    pub(super) size: wgpu::Extent3d,
    destination: ID3D11Texture2D,
    context: ID3D11DeviceContext4,
    d3d11_fence: ID3D11Fence,
    d3d12_fence: windows::Win32::Graphics::Direct3D12::ID3D12Fence,
    d3d12_queue: windows::Win32::Graphics::Direct3D12::ID3D12CommandQueue,
    fence_value: u64,
}

impl SharedTexture {
    pub(super) fn new(
        device: &wgpu::Device,
        hal_device: &wgpu::hal::dx12::Device,
        source: &ID3D11Texture2D,
    ) -> Result<(Self, wgpu::Texture)> {
        use windows_061::Win32::{
            Foundation::GENERIC_ALL,
            Graphics::{
                Direct3D11::{
                    D3D11_BIND_SHADER_RESOURCE, D3D11_FENCE_FLAG_SHARED,
                    D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_USAGE_DEFAULT, ID3D11Device5,
                },
                Dxgi::{Common::DXGI_FORMAT_B8G8R8A8_TYPELESS, IDXGIResource1},
            },
        };

        let mut descriptor = source_descriptor(source);
        validate_capture_descriptor(&descriptor)?;
        descriptor.Format = DXGI_FORMAT_B8G8R8A8_TYPELESS;
        descriptor.Usage = D3D11_USAGE_DEFAULT;
        descriptor.BindFlags = D3D11_BIND_SHADER_RESOURCE.0 as u32;
        descriptor.CPUAccessFlags = 0;
        descriptor.MiscFlags = D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 as u32;

        let source_device =
            unsafe { source.GetDevice() }.context("getting capture D3D11 device")?;
        let context: ID3D11DeviceContext4 = unsafe { source_device.GetImmediateContext() }
            .context("getting capture D3D11 context")?
            .cast()
            .context("D3D11 fence signaling is unavailable")?;
        let mut destination = None;
        unsafe {
            source_device.CreateTexture2D(
                &descriptor,
                None,
                Some(std::ptr::addr_of_mut!(destination)),
            )
        }
        .context("creating shared D3D11 capture texture")?;
        let destination = destination.context("D3D11 returned no shared texture")?;

        let device5: ID3D11Device5 = source_device
            .cast()
            .context("D3D11 shared fences are unavailable")?;
        let mut d3d11_fence: Option<ID3D11Fence> = None;
        unsafe {
            device5.CreateFence(
                0,
                D3D11_FENCE_FLAG_SHARED,
                std::ptr::addr_of_mut!(d3d11_fence),
            )
        }
        .context("creating shared D3D11 capture fence")?;
        let d3d11_fence = d3d11_fence.context("D3D11 returned no shared fence")?;

        let resource: IDXGIResource1 = destination
            .cast()
            .context("querying shared DXGI resource")?;
        let resource_handle = unsafe { resource.CreateSharedHandle(None, GENERIC_ALL.0, None) }
            .map(OwnedHandle)
            .context("creating capture resource handle")?;
        let fence_handle = unsafe { d3d11_fence.CreateSharedHandle(None, GENERIC_ALL.0, None) }
            .map(OwnedHandle)
            .context("creating capture fence handle")?;
        let (d3d12_resource, d3d12_fence) =
            open_shared_resources(hal_device, &resource_handle, &fence_handle)?;

        let size = capture_size(&descriptor);
        let hal_texture = unsafe {
            wgpu::hal::dx12::Device::texture_from_raw(
                d3d12_resource,
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureDimension::D2,
                size,
                1,
                1,
            )
        };
        let texture = unsafe {
            device.create_texture_from_hal::<wgpu::hal::api::Dx12>(
                hal_texture,
                &capture_texture_descriptor("windows_capture_shared", size),
            )
        };
        Ok((
            Self {
                size,
                destination,
                context,
                d3d11_fence,
                d3d12_fence,
                d3d12_queue: hal_device.raw_queue().clone(),
                fence_value: 0,
            },
            texture,
        ))
    }

    pub(super) fn update(&mut self, source: &ID3D11Texture2D) -> Result<()> {
        // CachedTexture gives each (native texture pointer, capture timestamp) frame
        // its own destination and initializes it once. That makes this destination
        // immutable while D3D12 samples it; only the producer-to-consumer handoff is
        // needed here.
        let fence_value = self
            .fence_value
            .checked_add(1)
            .context("capture fence value exhausted")?;
        unsafe { self.context.CopyResource(&self.destination, source) };
        unsafe { self.context.Signal(&self.d3d11_fence, fence_value) }
            .context("signaling capture copy completion")?;
        // D3D11 immediate contexts can defer command submission. Flush after the
        // producer signal so the D3D12 queue wait below cannot wait on work that
        // is still sitting in the D3D11 command buffer.
        unsafe { self.context.Flush() };
        unsafe { self.d3d12_queue.Wait(&self.d3d12_fence, fence_value) }
            .context("waiting for D3D11 capture copy")?;
        self.fence_value = fence_value;
        Ok(())
    }
}

fn open_shared_resources(
    hal_device: &wgpu::hal::dx12::Device,
    resource_handle: &OwnedHandle,
    fence_handle: &OwnedHandle,
) -> Result<(
    windows::Win32::Graphics::Direct3D12::ID3D12Resource,
    windows::Win32::Graphics::Direct3D12::ID3D12Fence,
)> {
    use windows::Win32::{
        Foundation::HANDLE,
        Graphics::Direct3D12::{ID3D12Fence, ID3D12Resource},
    };

    let raw_device = hal_device.raw_device();
    let mut resource: Option<ID3D12Resource> = None;
    let mut fence: Option<ID3D12Fence> = None;
    unsafe {
        raw_device.OpenSharedHandle(
            HANDLE(resource_handle.raw()),
            std::ptr::addr_of_mut!(resource),
        )
    }
    .context("opening capture texture on D3D12")?;
    unsafe {
        raw_device.OpenSharedHandle(HANDLE(fence_handle.raw()), std::ptr::addr_of_mut!(fence))
    }
    .context("opening capture fence on D3D12")?;
    Ok((
        resource.context("D3D12 returned no capture texture")?,
        fence.context("D3D12 returned no capture fence")?,
    ))
}

struct OwnedHandle(windows_061::Win32::Foundation::HANDLE);

impl OwnedHandle {
    fn raw(&self) -> *mut std::ffi::c_void {
        self.0.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if let Err(error) = unsafe { windows_061::Win32::Foundation::CloseHandle(self.0) } {
            log::error!("failed to close shared capture handle: {error}");
        }
    }
}
