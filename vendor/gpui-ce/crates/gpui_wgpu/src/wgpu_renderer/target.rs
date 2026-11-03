use gpui::{DevicePixels, Size};

use super::WgpuSurfaceConfig;

/// Configuration and lifecycle state shared by window, canvas, and headless targets.
pub(super) struct RenderTarget {
    config: wgpu::SurfaceConfiguration,
    transparent_alpha_mode: wgpu::CompositeAlphaMode,
    opaque_alpha_mode: wgpu::CompositeAlphaMode,
    maximum_dimension: u32,
    warned_transparent_alpha_unavailable: bool,
    configured: bool,
    needs_redraw: bool,
    pub(super) clear_color: wgpu::Color,
}

impl RenderTarget {
    pub(super) fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        surface: Option<&wgpu::Surface<'_>>,
        requested: WgpuSurfaceConfig,
    ) -> anyhow::Result<Self> {
        let (format, transparent_alpha_mode, opaque_alpha_mode, present_mode) =
            if let Some(surface) = surface {
                let capabilities = surface.get_capabilities(adapter);
                let format = [
                    wgpu::TextureFormat::Bgra8Unorm,
                    wgpu::TextureFormat::Rgba8Unorm,
                ]
                .into_iter()
                .find(|format| capabilities.formats.contains(format))
                .or_else(|| {
                    capabilities
                        .formats
                        .iter()
                        .find(|format| !format.is_srgb())
                        .copied()
                })
                .or_else(|| capabilities.formats.first().copied())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "surface reports no texture formats for adapter {:?}",
                        adapter.get_info().name
                    )
                })?;
                let (transparent, opaque) = select_alpha_modes(&capabilities.alpha_modes)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "surface reports no alpha modes for adapter {:?}",
                            adapter.get_info().name
                        )
                    })?;
                let present_mode = select_present_mode(
                    requested.preferred_present_mode,
                    &capabilities.present_modes,
                    wgpu::PresentMode::Fifo,
                );
                (format, transparent, opaque, present_mode)
            } else {
                // Native desktop swapchains are BGRA; matching that keeps snapshots comparable.
                (
                    wgpu::TextureFormat::Bgra8Unorm,
                    wgpu::CompositeAlphaMode::PreMultiplied,
                    wgpu::CompositeAlphaMode::Opaque,
                    wgpu::PresentMode::Fifo,
                )
            };
        let maximum_dimension = device.limits().max_texture_dimension_2d;
        let (width, height) = clamped_size(requested.size, maximum_dimension);
        let alpha_mode = if requested.transparent {
            transparent_alpha_mode
        } else {
            opaque_alpha_mode
        };
        let warned_transparent_alpha_unavailable =
            requested.transparent && alpha_mode == wgpu::CompositeAlphaMode::Opaque;
        if warned_transparent_alpha_unavailable {
            log::warn!(
                "Requested a transparent WGPU surface, but the surface only provided opaque alpha mode"
            );
        }
        Ok(Self {
            config: wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode,
                desired_maximum_frame_latency: 2,
                alpha_mode,
                view_formats: Vec::new(),
            },
            transparent_alpha_mode,
            opaque_alpha_mode,
            maximum_dimension,
            warned_transparent_alpha_unavailable,
            configured: surface.is_some(),
            needs_redraw: false,
            clear_color: clear_color(requested.transparent),
        })
    }

    pub(super) fn configuration(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub(super) fn resize(&mut self, size: Size<DevicePixels>) -> bool {
        let (width, height) = clamped_size(size, self.maximum_dimension);
        if (width, height) == (self.config.width, self.config.height) {
            return false;
        }
        self.config.width = width;
        self.config.height = height;
        true
    }

    /// Returns whether blend pipelines must be rebuilt.
    pub(super) fn set_transparent(&mut self, transparent: bool) -> bool {
        self.clear_color = clear_color(transparent);
        let alpha_mode = if transparent {
            self.transparent_alpha_mode
        } else {
            self.opaque_alpha_mode
        };
        if transparent
            && alpha_mode == wgpu::CompositeAlphaMode::Opaque
            && !self.warned_transparent_alpha_unavailable
        {
            log::warn!(
                "Requested a transparent WGPU surface, but the surface only provided opaque alpha mode"
            );
            self.warned_transparent_alpha_unavailable = true;
        }
        if alpha_mode == self.config.alpha_mode {
            return false;
        }
        self.config.alpha_mode = alpha_mode;
        true
    }

    #[cfg(not(target_family = "wasm"))]
    pub(super) fn apply(
        &mut self,
        requested: WgpuSurfaceConfig,
        supported_present_modes: &[wgpu::PresentMode],
    ) -> bool {
        self.resize(requested.size);
        self.config.present_mode = select_present_mode(
            requested.preferred_present_mode,
            supported_present_modes,
            self.config.present_mode,
        );
        self.set_transparent(requested.transparent)
    }

    #[cfg(not(target_family = "wasm"))]
    pub(super) fn recovery_config(&self) -> WgpuSurfaceConfig {
        WgpuSurfaceConfig {
            size: self.viewport_size(),
            transparent: self.config.alpha_mode != wgpu::CompositeAlphaMode::Opaque,
            preferred_present_mode: Some(self.config.present_mode),
        }
    }

    pub(super) fn viewport_size(&self) -> Size<DevicePixels> {
        Size {
            width: DevicePixels(self.config.width as i32),
            height: DevicePixels(self.config.height as i32),
        }
    }

    pub(super) fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub(super) fn width(&self) -> u32 {
        self.config.width
    }

    pub(super) fn height(&self) -> u32 {
        self.config.height
    }

    pub(super) fn alpha_mode(&self) -> wgpu::CompositeAlphaMode {
        self.config.alpha_mode
    }

    pub(super) fn maximum_dimension(&self) -> u32 {
        self.maximum_dimension
    }

    pub(super) fn clear_color(&self) -> wgpu::Color {
        self.clear_color
    }

    pub(super) fn is_configured(&self) -> bool {
        self.configured
    }

    pub(super) fn set_configured(&mut self, configured: bool) {
        self.configured = configured;
    }

    pub(super) fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }

    pub(super) fn take_redraw_request(&mut self) -> bool {
        std::mem::take(&mut self.needs_redraw)
    }
}

fn select_present_mode(
    requested: Option<wgpu::PresentMode>,
    supported: &[wgpu::PresentMode],
    current: wgpu::PresentMode,
) -> wgpu::PresentMode {
    // AutoVsync and AutoNoVsync are valid on every surface even though they
    // are intentionally omitted from SurfaceCapabilities::present_modes.
    let candidate = requested.unwrap_or(current);
    if is_supported(candidate, supported) {
        return candidate;
    }
    // FIFO is guaranteed by the surface API, even if a backend omits it from
    // its capability list.
    wgpu::PresentMode::Fifo
}

fn is_supported(mode: wgpu::PresentMode, supported: &[wgpu::PresentMode]) -> bool {
    matches!(
        mode,
        wgpu::PresentMode::AutoVsync | wgpu::PresentMode::AutoNoVsync | wgpu::PresentMode::Fifo
    ) || supported.contains(&mode)
}

fn clamped_size(size: Size<DevicePixels>, maximum: u32) -> (u32, u32) {
    let requested_width = size.width.0.max(1) as u32;
    let requested_height = size.height.0.max(1) as u32;
    let width = requested_width.min(maximum);
    let height = requested_height.min(maximum);
    if (width, height) != (requested_width, requested_height) {
        log::warn!(
            "requested target size ({requested_width}, {requested_height}) exceeds maximum texture dimension {maximum}; clamping to ({width}, {height})"
        );
    }
    (width, height)
}

/// The supported alpha modes for transparent and opaque windows, in that order.
fn select_alpha_modes(
    supported: &[wgpu::CompositeAlphaMode],
) -> Option<(wgpu::CompositeAlphaMode, wgpu::CompositeAlphaMode)> {
    // A non-opaque CAMetalLayer, which WGPU's Metal backend reports as
    // PostMultiplied, composites what gpui draws there as it does with Metal.
    #[cfg(target_os = "macos")]
    const TRANSPARENT: &[wgpu::CompositeAlphaMode] = &[
        wgpu::CompositeAlphaMode::PreMultiplied,
        wgpu::CompositeAlphaMode::Inherit,
        wgpu::CompositeAlphaMode::PostMultiplied,
    ];
    #[cfg(not(target_os = "macos"))]
    const TRANSPARENT: &[wgpu::CompositeAlphaMode] = &[
        wgpu::CompositeAlphaMode::PreMultiplied,
        wgpu::CompositeAlphaMode::Inherit,
    ];
    const OPAQUE: &[wgpu::CompositeAlphaMode] = &[
        wgpu::CompositeAlphaMode::Opaque,
        wgpu::CompositeAlphaMode::Inherit,
    ];
    let pick = |preferences: &[wgpu::CompositeAlphaMode]| {
        preferences
            .iter()
            .find(|mode| supported.contains(mode))
            .or_else(|| supported.first())
            .copied()
    };
    Some((pick(TRANSPARENT)?, pick(OPAQUE)?))
}

fn clear_color(transparent: bool) -> wgpu::Color {
    if transparent {
        return wgpu::Color::TRANSPARENT;
    }
    #[cfg(target_os = "windows")]
    return wgpu::Color::WHITE;
    #[cfg(target_os = "macos")]
    return wgpu::Color::BLACK;
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    wgpu::Color::TRANSPARENT
}

#[cfg(test)]
mod tests {
    use super::{select_alpha_modes, select_present_mode};

    #[cfg(target_os = "macos")]
    #[test]
    fn metal_transparent_windows_are_not_opaque() {
        // What WGPU's Metal backend reports for a CAMetalLayer.
        let metal = [
            wgpu::CompositeAlphaMode::Opaque,
            wgpu::CompositeAlphaMode::PostMultiplied,
        ];
        assert_eq!(
            select_alpha_modes(&metal),
            Some((
                wgpu::CompositeAlphaMode::PostMultiplied,
                wgpu::CompositeAlphaMode::Opaque,
            )),
        );
    }

    #[test]
    fn alpha_modes_follow_preference_then_the_first_supported() {
        use wgpu::CompositeAlphaMode::{Inherit, Opaque, PostMultiplied, PreMultiplied};

        assert_eq!(
            select_alpha_modes(&[PostMultiplied, Opaque, PreMultiplied]),
            Some((PreMultiplied, Opaque)),
        );
        assert_eq!(select_alpha_modes(&[Inherit]), Some((Inherit, Inherit)));
        // Neither opaque preference is offered, so opaque windows take the
        // first mode the surface supports.
        assert_eq!(
            select_alpha_modes(&[PostMultiplied]),
            Some((PostMultiplied, PostMultiplied)),
        );
    }

    #[test]
    fn no_alpha_modes_select_nothing() {
        assert_eq!(select_alpha_modes(&[]), None);
    }

    #[test]
    fn unsupported_present_mode_falls_back_to_fifo() {
        assert_eq!(
            select_present_mode(
                Some(wgpu::PresentMode::Immediate),
                &[wgpu::PresentMode::Fifo],
                wgpu::PresentMode::Fifo,
            ),
            wgpu::PresentMode::Fifo,
        );
    }

    #[test]
    fn missing_capabilities_keep_current_mode() {
        assert_eq!(
            select_present_mode(None, &[], wgpu::PresentMode::Fifo,),
            wgpu::PresentMode::Fifo,
        );
    }

    #[test]
    fn none_preserves_supported_current_mode() {
        assert_eq!(
            select_present_mode(
                None,
                &[wgpu::PresentMode::Fifo, wgpu::PresentMode::Mailbox],
                wgpu::PresentMode::Mailbox,
            ),
            wgpu::PresentMode::Mailbox,
        );
    }

    #[test]
    fn none_falls_back_when_replacement_surface_drops_current_mode() {
        assert_eq!(
            select_present_mode(None, &[wgpu::PresentMode::Fifo], wgpu::PresentMode::Mailbox,),
            wgpu::PresentMode::Fifo,
        );
    }

    #[test]
    fn none_preserves_unlisted_auto_current_mode() {
        assert_eq!(
            select_present_mode(
                None,
                &[wgpu::PresentMode::Fifo],
                wgpu::PresentMode::AutoVsync,
            ),
            wgpu::PresentMode::AutoVsync,
        );
    }

    #[test]
    fn auto_modes_are_valid_without_capability_listing() {
        assert_eq!(
            select_present_mode(
                Some(wgpu::PresentMode::AutoNoVsync),
                &[wgpu::PresentMode::Fifo],
                wgpu::PresentMode::Fifo,
            ),
            wgpu::PresentMode::AutoNoVsync,
        );
    }
}
