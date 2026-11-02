//! Shared pixel-level contracts for Frame's downstream preview patches.
use gpui::{
    AtlasKey, Bounds, ContentMask, Corners, DevicePixels, ImageId, PlatformHeadlessRenderer,
    PolychromeSprite, RenderImageParams, ScaledPixels, Scene, ShaderBool, TransformationMatrix,
    point, size,
};
use std::borrow::Cow;

pub fn check_preview_transforms_and_updates(renderer: &mut dyn PlatformHeadlessRenderer) {
    let atlas = renderer.sprite_atlas();
    let key = AtlasKey::Image(RenderImageParams {
        image_id: ImageId(900),
        frame_index: 0,
    });
    // BGRA: red, green / blue, white.
    let mut pixels = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            pixels.extend_from_slice(&match (x < 4, y < 4) {
                (true, true) => [0, 0, 255, 255],
                (false, true) => [0, 255, 0, 255],
                (true, false) => [255, 0, 0, 255],
                (false, false) => [255; 4],
            });
        }
    }
    let tile = atlas
        .get_or_update_with(&key, 1, &mut || {
            Ok(Some((
                size(DevicePixels(8), DevicePixels(8)),
                Cow::Borrowed(&pixels),
            )))
        })
        .unwrap()
        .unwrap();
    let bounds = Bounds::new(
        point(ScaledPixels(8.0), ScaledPixels(8.0)),
        size(ScaledPixels(16.0), ScaledPixels(16.0)),
    );
    let mask = Bounds::new(
        point(ScaledPixels(0.0), ScaledPixels(0.0)),
        size(ScaledPixels(32.0), ScaledPixels(32.0)),
    );
    let cases = [
        (
            TransformationMatrix::unit(),
            [
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [0, 0, 255, 255],
                [255, 255, 255, 255],
            ],
        ),
        (
            TransformationMatrix {
                rotation_scale: [[0.0, -1.0], [1.0, 0.0]],
                translation: [32.0, 0.0],
            },
            [
                [0, 0, 255, 255],
                [255, 0, 0, 255],
                [255, 255, 255, 255],
                [0, 255, 0, 255],
            ],
        ),
        (
            TransformationMatrix {
                rotation_scale: [[-1.0, 0.0], [0.0, 1.0]],
                translation: [32.0, 0.0],
            },
            [
                [0, 255, 0, 255],
                [255, 0, 0, 255],
                [255, 255, 255, 255],
                [0, 0, 255, 255],
            ],
        ),
    ];
    let render = |renderer: &mut dyn PlatformHeadlessRenderer, transformation, smoothing, clip| {
        let mut scene = Scene::default();
        scene.insert_primitive(PolychromeSprite {
            order: 0,
            grayscale: ShaderBool::Disabled,
            opacity: 1.0,
            corner_smoothing: smoothing,
            bounds,
            content_mask: ContentMask {
                bounds: clip,
                ..Default::default()
            },
            corner_radii: Corners::all(ScaledPixels(4.0)),
            tile,
            transformation,
        });
        scene.finish();
        renderer
            .render_scene_to_image(&scene, size(DevicePixels(32), DevicePixels(32)))
            .expect("Frame preview must render")
    };
    for smoothing in [0.0, 0.6] {
        for (transform, expected) in cases {
            let image = render(renderer, transform, smoothing, mask);
            for ((x, y), expected) in [(12, 12), (20, 12), (12, 20), (20, 20)]
                .into_iter()
                .zip(expected)
            {
                let actual = image.get_pixel(x, y).0;
                assert!(
                    actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) < 40),
                    "transform={transform:?}, smoothing={smoothing}, ({x},{y}): {actual:?} != {expected:?}"
                );
            }
            assert!(
                image.get_pixel(8, 8).0[..3].iter().all(|v| *v < 40),
                "rounded corners must follow the transformed image"
            );
        }
    }
    let clipped = render(
        renderer,
        cases[1].0,
        0.0,
        Bounds {
            size: size(ScaledPixels(16.0), ScaledPixels(32.0)),
            ..mask
        },
    );
    assert!(
        clipped.get_pixel(20, 12).0[..3].iter().all(|v| *v < 5),
        "content mask remains in window coordinates"
    );
    let next = atlas
        .get_or_update_with(&key, 2, &mut || {
            Ok(Some((
                size(DevicePixels(8), DevicePixels(8)),
                Cow::Owned([0, 255, 255, 255].repeat(64)),
            )))
        })
        .unwrap()
        .unwrap();
    assert_eq!(tile, next, "new preview frames reuse the tile");
    let image = render(renderer, TransformationMatrix::unit(), 0.0, mask);
    assert_eq!(
        image.get_pixel(12, 12).0,
        [255, 255, 0, 255],
        "reused tiles must display new pixels"
    );
}
