//! Run inside a Wayland session with Vulkan available:
//! `cargo test -p gpui_ce_wgpu --lib failed_frames_release_surface_images -- --ignored`
use std::{ptr::NonNull, sync::Arc};

use gpui::{DevicePixels, Scene, Size};
use raw_window_handle::{RawWindowHandle, WaylandWindowHandle};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_compositor, wl_registry, wl_surface},
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

use super::{
    NativeBackend, NativeSurfaceTarget, SoftwareAdapterPolicy, WgpuAtlas, WgpuContext,
    create_surface,
};
use crate::{WgpuRenderer, WgpuSurfaceConfig};

#[derive(Default)]
struct WindowState {
    configured: bool,
}

delegate_noop!(WindowState: ignore wl_compositor::WlCompositor);
delegate_noop!(WindowState: ignore wl_surface::WlSurface);
delegate_noop!(WindowState: ignore xdg_toplevel::XdgToplevel);

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WindowState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WindowState {
    fn event(
        _: &mut Self,
        shell: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            shell.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for WindowState {
    fn event(
        state: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            state.configured = true;
        }
    }
}

#[test]
#[ignore = "requires a Wayland compositor and Vulkan; opens a temporary test window"]
fn failed_frames_release_surface_images() {
    let connection = Connection::connect_to_env().expect("Wayland connection");
    let (globals, mut events) = registry_queue_init::<WindowState>(&connection).unwrap();
    let queue = events.handle();
    let compositor: wl_compositor::WlCompositor = globals.bind(&queue, 1..=4, ()).unwrap();
    let shell: xdg_wm_base::XdgWmBase = globals.bind(&queue, 1..=1, ()).unwrap();
    let surface = compositor.create_surface(&queue, ());
    let shell_surface = shell.get_xdg_surface(&surface, &queue, ());
    let toplevel = shell_surface.get_toplevel(&queue, ());
    toplevel.set_title("GPUI surface recovery test".into());
    surface.commit();
    let mut state = WindowState::default();
    events.roundtrip(&mut state).unwrap();
    assert!(
        state.configured,
        "compositor must configure the test window"
    );
    connection.flush().unwrap();

    // Use a real Vulkan swapchain: a headless texture cannot expose exhausted
    // presentation images. Keep the Wayland objects alive until after rendering.
    let instance = NativeBackend::Vulkan.instance(Some(Box::new(connection.backend())));
    let handle = WaylandWindowHandle::new(NonNull::new(surface.id().as_ptr().cast()).unwrap());
    let gpu_surface = create_surface(
        &instance.raw,
        NativeSurfaceTarget::Window(RawWindowHandle::Wayland(handle)),
    )
    .unwrap();
    let context = WgpuContext::new_with_adapter_policy(
        instance,
        &gpu_surface,
        None,
        SoftwareAdapterPolicy::Allow,
        None,
    )
    .unwrap();
    let mut renderer = WgpuRenderer::new_internal(
        None,
        &context,
        Some(gpu_surface),
        WgpuSurfaceConfig {
            size: Size {
                width: DevicePixels(128),
                height: DevicePixels(128),
            },
            transparent: false,
            preferred_present_mode: None,
        },
        None,
        None,
        Arc::new(WgpuAtlas::from_context(&context)),
    )
    .unwrap();
    let mut scene = Scene::default();
    scene.finish();
    assert!(renderer.draw(&scene), "initial frame must present");
    events.roundtrip(&mut state).unwrap();

    for attempt in 0..8 {
        // Exercise the existing failure path in begin_frame, after draw has
        // acquired an image. Do not mock acquisition or call recovery ourselves.
        renderer.faults.consecutive_failed_frames = 5;
        *renderer.faults.pending_error.lock().unwrap() = Some("injected frame error".into());
        assert!(!renderer.draw(&scene), "injected frame {attempt} must fail");
        assert!(
            renderer.faults.pending_error.lock().unwrap().is_none(),
            "frame {attempt} never reached rendering: surface images were exhausted"
        );
        events.roundtrip(&mut state).unwrap();
    }
    assert!(
        renderer.draw(&scene),
        "healthy frame must present after repeated failures"
    );
    drop(renderer);
    toplevel.destroy();
    shell_surface.destroy();
    surface.destroy();
    connection.flush().unwrap();
}
