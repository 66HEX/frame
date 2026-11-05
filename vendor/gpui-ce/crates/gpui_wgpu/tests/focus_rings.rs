//! Raster regression for animated rings: real Style -> Window -> GPU pixels.
#![cfg(feature = "test-support")]

use gpui::prelude::*;
use gpui::{
    Context, DevicePixels, PlatformHeadlessRenderer, Render, Scene, TestAppContext, Window, div,
    px, rgb, rgba, size,
};
use gpui_ce_wgpu::WgpuHeadlessRenderer;

struct RingView {
    width: f32,
    outline: bool,
}

impl Render for RingView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(rgb(0x171717)).child(
            div()
                .absolute()
                .left(px(24.0))
                .top(px(24.0))
                .size(px(28.0))
                .rounded(px(6.0))
                .ring(px(self.width))
                .ring_color(rgba(0x73737380))
                .when(self.outline, |d| {
                    d.outline(px(1.0)).outline_color(rgb(0x737373))
                }),
        )
    }
}

#[test]
fn fractional_focus_ring_is_smooth_concentric_and_symmetric_in_gpu_pixels() {
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, _| RingView {
        width: 0.0,
        outline: false,
    });
    let mut renderer = WgpuHeadlessRenderer::new().expect("headless GPU renderer");
    let mut previous_mass = 0_u64;
    let mut deltas = Vec::new();
    // Linear widths deliberately isolate raster continuity from the easing curve.
    for step in 0..=60 {
        view.update(cx, |v, cx| {
            v.width = step as f32 / 20.0;
            cx.notify();
        });
        cx.run_until_parked();
        let (mut scene, scale) = cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let mut scene = Scene::default();
            for quad in window.painted_quads() {
                scene.insert_primitive(quad);
            }
            (scene, window.scale_factor())
        });
        assert_eq!(scale, 2.0, "test samples a Retina control");
        scene.finish();
        let image = renderer
            .render_scene_to_image(&scene, size(DevicePixels(152), DevicePixels(152)))
            .unwrap();
        // Transparent center remains untouched: no filled shadow inside the ring.
        assert_eq!(image.get_pixel(76, 76).0, [23, 23, 23, 255]);
        for y in 35..117 {
            for x in 35..117 {
                let value = image.get_pixel(x, y)[0];
                for (rx, ry) in [(151 - x, y), (x, 151 - y), (y, x)] {
                    assert!(
                        value.abs_diff(image.get_pixel(rx, ry)[0]) <= 1,
                        "asymmetric ring at step {step}, ({x}, {y})"
                    );
                }
            }
        }
        let mass: u64 = image
            .pixels()
            .map(|p| u64::from(p[0].saturating_sub(23)))
            .sum();
        if step > 0 {
            assert!(
                mass > previous_mass,
                "ring stuck on a device-pixel step {step}"
            );
            deltas.push(mass - previous_mass);
        }
        previous_mass = mass;
        if let Ok(directory) = std::env::var("FRAME_RING_FRAMES") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(format!("{directory}/ring-{step:02}.rgba"), image.as_raw()).unwrap();
        }
    }
    // No large catch-up frames hidden between the otherwise smooth samples.
    assert!(deltas.iter().max().unwrap() < &(deltas.iter().min().unwrap() * 3));
}

#[test]
fn tab_outline_and_ring_have_the_same_corner_center_in_gpu_pixels() {
    let mut app = TestAppContext::single();
    let (_, cx) = app.add_window_view(|_, _| RingView {
        width: 3.0,
        outline: true,
    });
    cx.run_until_parked();
    let mut scene = cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let mut scene = Scene::default();
        for quad in window.painted_quads() {
            scene.insert_primitive(quad);
        }
        scene
    });
    scene.finish();
    let mut renderer = WgpuHeadlessRenderer::new().unwrap();
    let image = renderer
        .render_scene_to_image(&scene, size(DevicePixels(152), DevicePixels(152)))
        .unwrap();
    for y in 35..117 {
        for x in 35..117 {
            // Independent circular signed-distance model at pixel centers.
            // All three contours share the center and straight-edge lengths.
            let qx = ((x as f32 + 0.5) - 76.0).abs() - 16.0;
            let qy = ((y as f32 + 0.5) - 76.0).abs() - 16.0;
            let distance = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - 12.0;
            let outside = (0.5 + distance).clamp(0.0, 1.0);
            let ring = (6.5 - distance).clamp(0.0, 1.0) * outside * (128.0 / 255.0);
            let outline = (2.5 - distance).clamp(0.0, 1.0) * outside;
            let expected = (23.0 + (115.0 - 23.0) * ring) * (1.0 - outline) + 115.0 * outline;
            assert!(
                (f32::from(image.get_pixel(x, y)[0]) - expected).abs() <= 1.5,
                "non-concentric contour at ({x}, {y}): expected {expected}, got {:?}",
                image.get_pixel(x, y)
            );
        }
    }
    if let Ok(directory) = std::env::var("FRAME_RING_FRAMES") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(format!("{directory}/tab.rgba"), image.as_raw()).unwrap();
    }
}
