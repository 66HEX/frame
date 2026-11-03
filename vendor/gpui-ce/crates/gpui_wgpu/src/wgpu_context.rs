#[cfg(not(target_family = "wasm"))]
use anyhow::Context as _;
#[cfg(not(target_family = "wasm"))]
use gpui_util::ResultExt;
#[cfg(not(target_family = "wasm"))]
use smallvec::SmallVec;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use wgpu::TextureFormat;

/// Environment variable that forces the HWND swapchain path on Windows.
pub const DISABLE_DIRECT_COMPOSITION_ENV: &str = "GPUI_DISABLE_DIRECT_COMPOSITION";

/// A single native graphics API that can back a [`WgpuContext`].
///
/// Keeping this as an enum, rather than passing a [`wgpu::Backends`] bit-set through the native
/// initialization path, makes it impossible for the renderer to accidentally initialize a
/// fallback API alongside the preferred one. In particular, constructing WGPU's GL backend
/// creates and retains EGL state even when a Vulkan adapter is eventually selected.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg(not(target_family = "wasm"))]
pub(crate) enum NativeBackend {
    #[cfg(target_vendor = "apple")]
    Metal,
    #[cfg(target_os = "windows")]
    Dx12,
    #[cfg(not(target_vendor = "apple"))]
    Vulkan,
    #[cfg(not(target_vendor = "apple"))]
    Gl,
}

/// A WGPU instance paired with the single native API it was created to load.
///
/// Keeping the backend token attached prevents later initialization stages from
/// accidentally widening adapter discovery back to multiple APIs.
#[cfg(not(target_family = "wasm"))]
pub(crate) struct NativeInstance {
    pub(crate) backend: NativeBackend,
    pub(crate) raw: wgpu::Instance,
}

#[cfg(not(target_family = "wasm"))]
impl NativeBackend {
    #[cfg(target_vendor = "apple")]
    const PREFERENCE: &'static [Self] = &[Self::Metal];
    #[cfg(target_os = "windows")]
    const PREFERENCE: &'static [Self] = &[Self::Dx12, Self::Vulkan, Self::Gl];
    #[cfg(not(any(target_vendor = "apple", target_os = "windows")))]
    const PREFERENCE: &'static [Self] = &[Self::Vulkan, Self::Gl];

    pub(crate) fn instance(
        self,
        display: Option<Box<dyn wgpu::wgt::WgpuHasDisplayHandle>>,
    ) -> NativeInstance {
        #[cfg(not(target_os = "windows"))]
        let backend_options = wgpu::BackendOptions::default();
        #[cfg(target_os = "windows")]
        let backend_options = match self {
            Self::Dx12 => {
                let mut options = wgpu::BackendOptions::default();
                let direct_composition_disabled = std::env::var(DISABLE_DIRECT_COMPOSITION_ENV)
                    .is_ok_and(|value| value == "true" || value == "1");
                options.dx12.presentation_system = if direct_composition_disabled {
                    wgpu::Dx12SwapchainKind::DxgiFromHwnd
                } else {
                    wgpu::Dx12SwapchainKind::DxgiFromVisual
                };
                options
            }
            Self::Vulkan | Self::Gl => wgpu::BackendOptions::default(),
        };

        NativeInstance {
            backend: self,
            raw: wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: self.into(),
                flags: wgpu::InstanceFlags::default(),
                backend_options,
                memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
                display,
            }),
        }
    }

    pub(crate) fn try_in_preference_order<T>(
        operation: &'static str,
        mut attempt: impl FnMut(Self) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut failures = SmallVec::<[NativeBackendFailure; 3]>::new();
        for &backend in Self::PREFERENCE {
            match attempt(backend) {
                Ok(value) => return Ok(value),
                Err(source) => failures.push(NativeBackendFailure { backend, source }),
            }
        }
        Err(NativeBackendFallbackError {
            operation,
            failures,
        }
        .into())
    }
}

#[cfg(not(target_family = "wasm"))]
impl From<NativeBackend> for wgpu::Backends {
    fn from(backend: NativeBackend) -> Self {
        match backend {
            #[cfg(target_vendor = "apple")]
            NativeBackend::Metal => Self::METAL,
            #[cfg(target_os = "windows")]
            NativeBackend::Dx12 => Self::DX12,
            #[cfg(not(target_vendor = "apple"))]
            NativeBackend::Vulkan => Self::VULKAN,
            #[cfg(not(target_vendor = "apple"))]
            NativeBackend::Gl => Self::GL,
        }
    }
}

#[cfg(not(target_family = "wasm"))]
#[derive(Debug)]
struct NativeBackendFailure {
    backend: NativeBackend,
    source: anyhow::Error,
}

#[cfg(not(target_family = "wasm"))]
#[derive(Debug)]
struct NativeBackendFallbackError {
    operation: &'static str,
    failures: SmallVec<[NativeBackendFailure; 3]>,
}

#[cfg(not(target_family = "wasm"))]
impl std::fmt::Display for NativeBackendFallbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "failed to initialize {}; attempted backends in order: ",
            self.operation
        )?;
        for (index, failure) in self.failures.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{:?}: {:#}", failure.backend, failure.source)?;
        }
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
impl std::error::Error for NativeBackendFallbackError {}

#[cfg(not(target_family = "wasm"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SoftwareAdapterPolicy {
    Allow,
    Reject,
}

#[cfg(not(target_family = "wasm"))]
impl SoftwareAdapterPolicy {
    fn accepts(self, device_type: wgpu::DeviceType) -> bool {
        self == Self::Allow || device_type != wgpu::DeviceType::Cpu
    }
}

struct CreatedDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
    dual_source_blending: bool,
    color_texture_format: TextureFormat,
    renderer_tier: RendererTier,
}

#[cfg(not(target_family = "wasm"))]
struct SelectedAdapter {
    adapter: wgpu::Adapter,
    device: CreatedDevice,
}

pub struct WgpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    backend: WgpuBackend,
    dual_source_blending: bool,
    color_texture_format: wgpu::TextureFormat,
    renderer_tier: RendererTier,
    device_lost: Arc<AtomicBool>,
    uncaptured_error: Arc<Mutex<Option<String>>>,
}

/// The resource transport selected for a device.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RendererTier {
    /// WebGPU-class devices use storage buffers for packed scene data.
    Modern,
    /// GLES 3.0 and WebGL2 use a build-generated data-texture transport.
    WebGl2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WgpuBackend {
    BrowserWebGpu,
    Gl,
    Native(wgpu::Backend),
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WebBackendPreference {
    #[default]
    Auto,
    WebGpu,
    WebGl,
}

#[cfg(target_family = "wasm")]
pub struct PreparedWebGraphics {
    pub context: WgpuContext,
    pub surface: wgpu::Surface<'static>,
}

/// wgpu-core refuses to create a surface when neither the instance nor the surface
/// target carries a display handle, and `SurfaceTarget::Canvas` always passes `None`.
/// The WebGL2 backend never reads the handle (WebGPU bypasses wgpu-core entirely), so
/// a unit web display handle on the instance satisfies the check.
#[cfg(target_family = "wasm")]
#[derive(Debug)]
struct WebDisplaySource;

#[cfg(target_family = "wasm")]
impl raw_window_handle::HasDisplayHandle for WebDisplaySource {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(raw_window_handle::DisplayHandle::web())
    }
}

#[derive(Clone, Copy)]
pub struct CompositorGpuHint {
    pub vendor_id: u32,
    pub device_id: u32,
}

/// A typed view of the GPU resources shared with GPUI.
///
/// The device and queue are owned by GPUI and remain valid until
/// [`Self::device_lost`] returns `true`. A control should drop its GPU
/// resources and reacquire this handle after recovery.
#[derive(Clone)]
pub struct WgpuContextHandle {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    texture_format: wgpu::TextureFormat,
    adapter_info: wgpu::AdapterInfo,
    backend: WgpuBackend,
    features: wgpu::Features,
    limits: wgpu::Limits,
    device_lost: Arc<AtomicBool>,
}

impl WgpuContextHandle {
    pub(crate) fn from_resources(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        texture_format: wgpu::TextureFormat,
        adapter_info: wgpu::AdapterInfo,
        backend: WgpuBackend,
        device_lost: Arc<AtomicBool>,
    ) -> Self {
        let features = device.features();
        let limits = device.limits();
        Self {
            device,
            queue,
            texture_format,
            adapter_info,
            backend,
            features,
            limits,
            device_lost,
        }
    }

    /// Returns the shared device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the shared queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns whether two handles refer to the same device instance.
    pub fn is_same_device(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.device, &other.device)
    }

    /// Returns the texture format used by GPUI's window surface.
    pub fn texture_format(&self) -> wgpu::TextureFormat {
        self.texture_format
    }

    /// Returns information about the selected adapter.
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Returns the backend selected for this window.
    pub fn backend(&self) -> WgpuBackend {
        self.backend
    }

    /// Returns the features enabled on the shared device.
    pub fn features(&self) -> wgpu::Features {
        self.features
    }

    /// Returns the limits used when creating the shared device.
    pub fn limits(&self) -> &wgpu::Limits {
        &self.limits
    }

    /// Returns whether the device has been lost and custom resources must be
    /// recreated after GPUI recovers.
    pub fn device_lost(&self) -> bool {
        self.device_lost.load(Ordering::Relaxed)
    }

    /// Returns the typed wgpu context associated with a GPUI window.
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        all(target_os = "macos", feature = "custom-gpu"),
        all(target_family = "wasm", feature = "custom-gpu")
    ))]
    pub fn from_window(window: &gpui::Window) -> Option<Self> {
        window
            .gpu_context_info()?
            .downcast::<Self>()
            .ok()
            .map(|context| *context)
    }
}

/// A reusable offscreen render target suitable for [`gpui::Window::paint_surface`].
pub struct WgpuRenderTarget {
    texture: Arc<wgpu::Texture>,
    view: wgpu::TextureView,
    size: gpui::Size<gpui::DevicePixels>,
    format: wgpu::TextureFormat,
}

impl WgpuRenderTarget {
    /// Creates a render target using GPUI's surface format.
    pub fn new(context: &WgpuContextHandle, size: gpui::Size<gpui::DevicePixels>) -> Self {
        Self::with_format(context, size, context.texture_format())
    }

    /// Creates a render target with an explicit texture format.
    pub fn with_format(
        context: &WgpuContextHandle,
        size: gpui::Size<gpui::DevicePixels>,
        format: wgpu::TextureFormat,
    ) -> Self {
        let size = normalize_target_size(size);
        let texture = create_render_target_texture(context.device(), size, format);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture: Arc::new(texture),
            view,
            size,
            format,
        }
    }

    /// Resizes the target, preserving its texture format.
    pub fn resize(&mut self, context: &WgpuContextHandle, size: gpui::Size<gpui::DevicePixels>) {
        let size = normalize_target_size(size);
        if self.size == size {
            return;
        }
        let texture = create_render_target_texture(context.device(), size, self.format);
        self.view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.texture = Arc::new(texture);
        self.size = size;
    }

    /// Returns the target texture for use as a GPUI surface.
    pub fn texture(&self) -> Arc<wgpu::Texture> {
        Arc::clone(&self.texture)
    }

    /// Creates a GPUI element that composites this target at its layout bounds.
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        all(target_os = "macos", feature = "custom-gpu"),
        all(target_family = "wasm", feature = "custom-gpu")
    ))]
    pub fn surface(&self) -> gpui::Surface {
        gpui::surface(gpui::SurfaceSource::Texture {
            texture: self.texture(),
            size: self.size,
        })
    }

    /// Returns the target texture view for rendering.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns the target dimensions in device pixels.
    pub fn size(&self) -> gpui::Size<gpui::DevicePixels> {
        self.size
    }

    /// Returns the target texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}

fn normalize_target_size(size: gpui::Size<gpui::DevicePixels>) -> gpui::Size<gpui::DevicePixels> {
    gpui::size(
        gpui::DevicePixels(size.width.0.max(1)),
        gpui::DevicePixels(size.height.0.max(1)),
    )
}

fn create_render_target_texture(
    device: &wgpu::Device,
    size: gpui::Size<gpui::DevicePixels>,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gpui-custom-render-target"),
        size: wgpu::Extent3d {
            width: size.width.0 as u32,
            height: size.height.0 as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

/// Extra wgpu features and limits that an application can request on top of
/// gpui's baseline.  Pass an instance to the platform via
/// [`gpui::App::set_gpu_requirements`] *before* opening any windows.
#[derive(Clone, Debug, Default)]
pub struct WgpuDeviceRequirements {
    /// Additional [`wgpu::Features`] to enable.  These are OR-ed with gpui's
    /// own required features.
    pub features: wgpu::Features,
    /// Additional [`wgpu::Limits`] to request.  Each field is merged by taking
    /// `max(gpui_limit, app_limit)` for upper-bound limits and
    /// `min(gpui_limit, app_limit)` for alignment/lower-bound limits.
    pub limits: Option<wgpu::Limits>,
}

impl WgpuContext {
    pub(crate) fn handle(&self, texture_format: wgpu::TextureFormat) -> WgpuContextHandle {
        WgpuContextHandle::from_resources(
            Arc::clone(&self.device),
            Arc::clone(&self.queue),
            texture_format,
            self.adapter.get_info(),
            self.backend,
            Arc::clone(&self.device_lost),
        )
    }

    /// Creates a native device without a presentation surface.
    #[cfg(not(target_family = "wasm"))]
    pub fn new_headless(
        extra_requirements: Option<&WgpuDeviceRequirements>,
    ) -> anyhow::Result<Self> {
        NativeBackend::try_in_preference_order("a headless GPU context", |backend| {
            let instance = backend.instance(None);
            let adapter =
                gpui::block_on(instance.raw.request_adapter(&wgpu::RequestAdapterOptions {
                    // LowPower avoids waking a discrete GPU just for snapshots on dual-GPU
                    // systems.
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                }))
                .map_err(|error| {
                    anyhow::anyhow!("failed to request headless GPU adapter: {error}")
                })?;
            let device = gpui::block_on(Self::create_device(&adapter, extra_requirements))?;
            Self::from_created_device(instance.raw, adapter, device)
        })
    }

    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn new_with_adapter_policy(
        instance: NativeInstance,
        surface: &wgpu::Surface<'_>,
        compositor_gpu: Option<CompositorGpuHint>,
        adapter_policy: SoftwareAdapterPolicy,
        extra_requirements: Option<&WgpuDeviceRequirements>,
    ) -> anyhow::Result<Self> {
        let device_id_filter = match std::env::var("ZED_DEVICE_ID") {
            Ok(val) => parse_pci_id(&val)
                .context("Failed to parse device ID from `ZED_DEVICE_ID` environment variable")
                .log_err(),
            Err(std::env::VarError::NotPresent) => None,
            err => {
                err.context("Failed to read value of `ZED_DEVICE_ID` environment variable")
                    .log_err();
                None
            }
        };

        // Select an adapter by actually testing surface configuration with the real device.
        // This is the only reliable way to determine compatibility on hybrid GPU systems.
        let selection = gpui::block_on(Self::select_adapter_and_device(
            &instance.raw,
            instance.backend,
            device_id_filter,
            surface,
            compositor_gpu.as_ref(),
            adapter_policy,
            extra_requirements,
        ))?;

        Self::from_created_device(instance.raw, selection.adapter, selection.device)
    }

    #[cfg(target_family = "wasm")]
    pub async fn new_web(
        canvas: &web_sys::HtmlCanvasElement,
        preference: WebBackendPreference,
    ) -> anyhow::Result<PreparedWebGraphics> {
        Self::new_web_with_backend(canvas, preference).await
    }

    #[cfg(target_family = "wasm")]
    #[allow(clippy::arc_with_non_send_sync)]
    async fn new_web_with_backend(
        canvas: &web_sys::HtmlCanvasElement,
        preference: WebBackendPreference,
    ) -> anyhow::Result<PreparedWebGraphics> {
        let backends = match preference {
            WebBackendPreference::Auto => wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            WebBackendPreference::WebGpu => wgpu::Backends::BROWSER_WEBGPU,
            WebBackendPreference::WebGl => wgpu::Backends::GL,
        };
        let descriptor = wgpu::InstanceDescriptor {
            backends,
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            display: Some(Box::new(WebDisplaySource)),
        };
        let instance = if preference == WebBackendPreference::Auto {
            wgpu::util::new_instance_with_webgpu_detection(descriptor).await
        } else {
            wgpu::Instance::new(descriptor)
        };
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|error| {
                anyhow::anyhow!("Failed to create browser graphics surface: {error}")
            })?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|error| {
                anyhow::anyhow!(
                    "Failed to request a {preference:?} adapter compatible with the canvas: {error}"
                )
            })?;
        let device = Self::create_device(&adapter, None).await?;
        let context = Self::from_created_device(instance, adapter, device)?;
        log::info!(
            "Browser graphics initialized: requested={preference:?}, selected={backend:?}, \
             adapter={:?}, limits={:?}, dual_source_blending={dual_source_blending}",
            context.adapter.get_info().name,
            context.device.limits(),
            backend = context.backend,
            dual_source_blending = context.dual_source_blending,
        );
        Ok(PreparedWebGraphics { context, surface })
    }

    fn from_created_device(
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        device: CreatedDevice,
    ) -> anyhow::Result<Self> {
        let (device_lost, uncaptured_error) = install_device_callbacks(&device.device);
        let info = adapter.get_info();
        log::info!("Selected GPU adapter: {:?} ({:?})", info.name, info.backend);
        #[cfg(target_family = "wasm")]
        let backend = match info.backend {
            wgpu::Backend::BrowserWebGpu => WgpuBackend::BrowserWebGpu,
            wgpu::Backend::Gl => WgpuBackend::Gl,
            backend => anyhow::bail!(
                "Browser graphics initialization selected unexpected backend {backend:?}"
            ),
        };
        #[cfg(not(target_family = "wasm"))]
        let backend = WgpuBackend::Native(info.backend);

        Ok(Self {
            instance,
            adapter,
            backend,
            device: Arc::new(device.device),
            queue: Arc::new(device.queue),
            dual_source_blending: device.dual_source_blending,
            color_texture_format: device.color_texture_format,
            renderer_tier: device.renderer_tier,
            device_lost,
            uncaptured_error,
        })
    }

    async fn create_device(
        adapter: &wgpu::Adapter,
        extra_requirements: Option<&WgpuDeviceRequirements>,
    ) -> anyhow::Result<CreatedDevice> {
        let renderer_tier = renderer_tier(adapter);
        // Our LCD shader uses storage buffers even when the adapter exposes dual-source
        // blending. Downlevel devices must use grayscale and the data-texture dialect.
        let dual_source_blending = renderer_tier == RendererTier::Modern
            && adapter
                .features()
                .contains(wgpu::Features::DUAL_SOURCE_BLENDING);

        let mut required_features = wgpu::Features::empty();
        if dual_source_blending {
            required_features |= wgpu::Features::DUAL_SOURCE_BLENDING;
        } else {
            log::warn!(
                "Dual-source blending not available on this GPU. \
                Subpixel text antialiasing will be disabled."
            );
        }

        let color_texture_format = Self::select_color_texture_format(adapter)?;

        let baseline = match renderer_tier {
            RendererTier::Modern => wgpu::Limits::downlevel_defaults(),
            RendererTier::WebGl2 => wgpu::Limits::downlevel_webgl2_defaults(),
        };
        let mut required_limits = baseline
            .using_resolution(adapter.limits())
            .using_alignment(adapter.limits());

        // Merge application-requested requirements.
        if let Some(reqs) = extra_requirements {
            required_features |= reqs.features;
            if let Some(limits) = &reqs.limits {
                required_limits = required_limits.or_better_values_from(limits);
            }
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("gpui_device"),
                required_features,
                required_limits,
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create wgpu device: {e}"))?;

        Ok(CreatedDevice {
            device,
            queue,
            dual_source_blending,
            color_texture_format,
            renderer_tier,
        })
    }

    pub fn check_compatible_with_surface(&self, surface: &wgpu::Surface<'_>) -> anyhow::Result<()> {
        let caps = surface.get_capabilities(&self.adapter);
        if caps.formats.is_empty() {
            let info = self.adapter.get_info();
            anyhow::bail!(
                "Adapter {:?} (backend={:?}, device={:#06x}) is not compatible with the \
                 display surface for this window.",
                info.name,
                info.backend,
                info.device,
            );
        }
        Ok(())
    }

    /// Select an adapter and create a device, testing that the surface can actually be configured.
    /// This is the only reliable way to determine compatibility on hybrid GPU systems, where
    /// adapters may report surface compatibility via get_capabilities() but fail when actually
    /// configuring (e.g., NVIDIA reporting Vulkan Wayland support but failing because the
    /// Wayland compositor runs on the Intel GPU).
    #[cfg(not(target_family = "wasm"))]
    async fn select_adapter_and_device(
        instance: &wgpu::Instance,
        backend: NativeBackend,
        device_id_filter: Option<u32>,
        surface: &wgpu::Surface<'_>,
        compositor_gpu: Option<&CompositorGpuHint>,
        adapter_policy: SoftwareAdapterPolicy,
        extra_requirements: Option<&WgpuDeviceRequirements>,
    ) -> anyhow::Result<SelectedAdapter> {
        let mut adapters: Vec<_> = instance.enumerate_adapters(backend.into()).await;

        if adapters.is_empty() {
            anyhow::bail!("No GPU adapters found");
        }

        if let Some(device_id) = device_id_filter {
            log::info!("ZED_DEVICE_ID filter: {:#06x}", device_id);
        }

        // Sort adapters into a single priority order. Tiers (from highest to lowest):
        //
        // 1. ZED_DEVICE_ID match — explicit user override
        // 2. Compositor GPU match — the GPU the display server is rendering on
        // 3. Device type (Discrete > Integrated > Other > Virtual > Cpu).
        //    "Other" ranks above "Virtual" because OpenGL seems to count as "Other".
        // 4. Backend — prefer Vulkan/Metal/Dx12 over GL/etc.
        adapters.sort_by_key(|adapter| {
            let info = adapter.get_info();

            // Backends like OpenGL report device=0 for all adapters, so
            // device-based matching is only meaningful when non-zero.
            let device_known = info.device != 0;

            let user_override: u8 = match device_id_filter {
                Some(id) if device_known && info.device == id => 0,
                _ => 1,
            };

            let compositor_match: u8 = match compositor_gpu {
                Some(hint)
                    if device_known
                        && info.vendor == hint.vendor_id
                        && info.device == hint.device_id =>
                {
                    0
                }
                _ => 1,
            };

            #[cfg(target_vendor = "apple")]
            let type_priority: u8 = match info.device_type {
                // Preserves the native renderer's low-power preference on Intel Macs.
                wgpu::DeviceType::IntegratedGpu => 0,
                wgpu::DeviceType::DiscreteGpu => 1,
                wgpu::DeviceType::Other => 2,
                wgpu::DeviceType::VirtualGpu => 3,
                wgpu::DeviceType::Cpu => 4,
            };
            #[cfg(not(target_vendor = "apple"))]
            let type_priority: u8 = if info.device_type == wgpu::DeviceType::Cpu {
                4
            } else {
                match info.device_type {
                    wgpu::DeviceType::DiscreteGpu => 0,
                    wgpu::DeviceType::IntegratedGpu => 1,
                    wgpu::DeviceType::Other => 2,
                    wgpu::DeviceType::VirtualGpu => 3,
                    wgpu::DeviceType::Cpu => 4,
                }
            };

            let backend_priority: u8 = match info.backend {
                wgpu::Backend::Vulkan | wgpu::Backend::Metal | wgpu::Backend::Dx12 => 0,
                _ => 1,
            };

            (
                user_override,
                compositor_match,
                type_priority,
                backend_priority,
            )
        });

        // Log all available adapters (in sorted order)
        log::info!("Found {} GPU adapter(s):", adapters.len());
        for adapter in &adapters {
            let info = adapter.get_info();
            log::info!(
                "  - {} (vendor={:#06x}, device={:#06x}, backend={:?}, type={:?})",
                info.name,
                info.vendor,
                info.device,
                info.backend,
                info.device_type,
            );
        }

        // Test each adapter by creating a device and configuring the surface
        for adapter in adapters {
            let info = adapter.get_info();

            if !adapter_policy.accepts(info.device_type) {
                log::info!(
                    "Skipping software renderer: {} ({:?})",
                    info.name,
                    info.backend
                );
                continue;
            }

            log::info!("Testing adapter: {} ({:?})...", info.name, info.backend);

            match Self::try_adapter_with_surface(&adapter, surface, extra_requirements).await {
                Ok(device) => {
                    log::info!(
                        "Adapter passed surface configuration test: {} ({:?})",
                        info.name,
                        info.backend
                    );
                    return Ok(SelectedAdapter { adapter, device });
                }
                Err(e) => {
                    log::info!(
                        "  Adapter {} ({:?}) failed: {}, trying next...",
                        info.name,
                        info.backend,
                        e
                    );
                }
            }
        }

        anyhow::bail!("No GPU adapter found that can configure the display surface")
    }

    /// Try to use an adapter with a surface by creating a device and testing configuration.
    /// Returns the device and queue if successful, allowing them to be reused.
    #[cfg(not(target_family = "wasm"))]
    async fn try_adapter_with_surface(
        adapter: &wgpu::Adapter,
        surface: &wgpu::Surface<'_>,
        extra_requirements: Option<&WgpuDeviceRequirements>,
    ) -> anyhow::Result<CreatedDevice> {
        let caps = surface.get_capabilities(adapter);
        if caps.formats.is_empty() {
            anyhow::bail!("no compatible surface formats");
        }
        if caps.alpha_modes.is_empty() {
            anyhow::bail!("no compatible alpha modes");
        }

        let device = Self::create_device(adapter, extra_requirements).await?;
        let error_scope = device
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);

        let test_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: 64,
            height: 64,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device.device, &test_config);

        let error = error_scope.pop().await;
        if let Some(e) = error {
            anyhow::bail!("surface configuration failed: {e}");
        }

        Ok(device)
    }

    fn select_color_texture_format(adapter: &wgpu::Adapter) -> anyhow::Result<wgpu::TextureFormat> {
        let required_usages = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let bgra_features = adapter.get_texture_format_features(wgpu::TextureFormat::Bgra8Unorm);
        let rgba_features = adapter.get_texture_format_features(wgpu::TextureFormat::Rgba8Unorm);
        #[cfg(target_family = "wasm")]
        if adapter.get_info().backend == wgpu::Backend::Gl
            && rgba_features.allowed_usages.contains(required_usages)
        {
            return Ok(wgpu::TextureFormat::Rgba8Unorm);
        }
        if bgra_features.allowed_usages.contains(required_usages) {
            return Ok(wgpu::TextureFormat::Bgra8Unorm);
        }
        if rgba_features.allowed_usages.contains(required_usages) {
            let info = adapter.get_info();
            log::warn!(
                "Adapter {} ({:?}) does not support Bgra8Unorm atlas textures with usages {:?}; \
                 falling back to Rgba8Unorm atlas textures.",
                info.name,
                info.backend,
                required_usages,
            );
            return Ok(wgpu::TextureFormat::Rgba8Unorm);
        }

        let info = adapter.get_info();
        Err(anyhow::anyhow!(
            "Adapter {} ({:?}, device={:#06x}) does not support a usable color atlas texture \
             format with usages {:?}. Bgra8Unorm allowed usages: {:?}; \
             Rgba8Unorm allowed usages: {:?}.",
            info.name,
            info.backend,
            info.device,
            required_usages,
            bgra_features.allowed_usages,
            rgba_features.allowed_usages,
        ))
    }
    pub fn backend(&self) -> WgpuBackend {
        self.backend
    }

    pub fn uses_webgl_instance_data(&self) -> bool {
        matches!(self.backend, WgpuBackend::Gl) && cfg!(target_family = "wasm")
    }

    pub fn supports_dual_source_blending(&self) -> bool {
        self.dual_source_blending
    }

    pub fn color_texture_format(&self) -> wgpu::TextureFormat {
        self.color_texture_format
    }

    pub fn renderer_tier(&self) -> RendererTier {
        self.renderer_tier
    }

    /// Returns true if the GPU device was lost (driver crash, suspend/resume).
    /// When this returns true, the context should be recreated.
    pub fn device_lost(&self) -> bool {
        self.device_lost.load(Ordering::Relaxed)
    }

    /// Returns a clone of the device_lost flag for sharing with renderers.
    pub(crate) fn device_lost_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.device_lost)
    }

    pub(crate) fn uncaptured_error_slot(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.uncaptured_error)
    }
}

fn renderer_tier(adapter: &wgpu::Adapter) -> RendererTier {
    let limits = adapter.limits();
    let flags = adapter.get_downlevel_capabilities().flags;
    if limits.max_storage_buffers_per_shader_stage > 0
        && flags.contains(wgpu::DownlevelFlags::VERTEX_STORAGE)
        && flags.contains(wgpu::DownlevelFlags::FRAGMENT_STORAGE)
    {
        RendererTier::Modern
    } else {
        RendererTier::WebGl2
    }
}

fn install_device_callbacks(
    device: &wgpu::Device,
) -> (Arc<AtomicBool>, Arc<Mutex<Option<String>>>) {
    let device_lost = Arc::new(AtomicBool::new(false));
    device.set_device_lost_callback({
        let device_lost = Arc::clone(&device_lost);
        move |reason, message| {
            log::error!("wgpu device lost: reason={reason:?}, message={message}");
            if reason != wgpu::DeviceLostReason::Destroyed {
                device_lost.store(true, Ordering::Relaxed);
            }
        }
    });
    let uncaptured_error = Arc::new(Mutex::new(None));
    device.on_uncaptured_error(Arc::new({
        let uncaptured_error = Arc::clone(&uncaptured_error);
        move |error| {
            let message = error.to_string();
            log::error!("uncaptured wgpu error: {message}");
            *uncaptured_error.lock().unwrap() = Some(message);
        }
    }));
    (device_lost, uncaptured_error)
}

#[cfg(not(target_family = "wasm"))]
fn parse_pci_id(id: &str) -> anyhow::Result<u32> {
    let mut id = id.trim();

    if id.starts_with("0x") || id.starts_with("0X") {
        id = &id[2..];
    }
    let is_hex_string = id.chars().all(|c| c.is_ascii_hexdigit());
    let is_4_chars = id.len() == 4;
    anyhow::ensure!(
        is_4_chars && is_hex_string,
        "Expected a 4 digit PCI ID in hexadecimal format"
    );

    u32::from_str_radix(id, 16).context("parsing PCI ID as hex")
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{NativeBackend, SoftwareAdapterPolicy, parse_pci_id};

    #[test]
    fn native_backend_fallbacks_are_individually_initialized() {
        let backends = NativeBackend::PREFERENCE;
        assert!(!backends.is_empty());
        assert!(
            backends
                .iter()
                .all(|backend| wgpu::Backends::from(*backend).bits().count_ones() == 1),
            "each fallback step must initialize exactly one WGPU backend"
        );

        #[cfg(target_os = "linux")]
        assert_eq!(backends, &[NativeBackend::Vulkan, NativeBackend::Gl]);
        #[cfg(target_os = "windows")]
        assert_eq!(
            backends,
            &[
                NativeBackend::Dx12,
                NativeBackend::Vulkan,
                NativeBackend::Gl,
            ]
        );
        #[cfg(target_os = "macos")]
        assert_eq!(backends, &[NativeBackend::Metal]);
    }

    #[test]
    fn native_backend_fallback_preserves_ordered_failures() {
        let mut attempted = Vec::new();
        let error = NativeBackend::try_in_preference_order::<()>("test context", |backend| {
            attempted.push(backend);
            anyhow::bail!("unavailable")
        })
        .unwrap_err();

        assert_eq!(attempted, NativeBackend::PREFERENCE);
        let message = format!("{error:#}");
        let mut previous = 0;
        for backend in NativeBackend::PREFERENCE {
            let index = message[previous..]
                .find(&format!("{backend:?}: unavailable"))
                .expect("each backend failure should be reported in preference order");
            previous += index;
        }
    }

    #[test]
    fn software_adapter_policy_is_explicit() {
        assert!(SoftwareAdapterPolicy::Allow.accepts(wgpu::DeviceType::Cpu));
        assert!(!SoftwareAdapterPolicy::Reject.accepts(wgpu::DeviceType::Cpu));
        assert!(
            SoftwareAdapterPolicy::Reject.accepts(wgpu::DeviceType::IntegratedGpu),
            "rejecting software must not reject lower-power hardware adapters"
        );
    }

    #[test]
    fn test_parse_device_id() {
        assert!(parse_pci_id("0xABCD").is_ok());
        assert!(parse_pci_id("ABCD").is_ok());
        assert!(parse_pci_id("abcd").is_ok());
        assert!(parse_pci_id("1234").is_ok());
        assert!(parse_pci_id("123").is_err());
        assert_eq!(
            parse_pci_id(&format!("{:x}", 0x1234)).unwrap(),
            parse_pci_id(&format!("{:X}", 0x1234)).unwrap(),
        );

        assert_eq!(
            parse_pci_id(&format!("{:#x}", 0x1234)).unwrap(),
            parse_pci_id(&format!("{:#X}", 0x1234)).unwrap(),
        );
    }
}
