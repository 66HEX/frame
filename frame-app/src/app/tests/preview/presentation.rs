use super::*;

#[test]
fn preview_presentation_maps_rotation_and_crop_without_rewriting_frame() {
    let frame = PreviewFrame::bgra(
        4,
        2,
        16,
        0,
        vec![
            0, 0, 0, 255, 1, 0, 0, 255, 2, 0, 0, 255, 3, 0, 0, 255, 4, 0, 0, 255, 5, 0, 0, 255, 6,
            0, 0, 255, 7, 0, 0, 255,
        ],
    )
    .expect("frame");
    let render_image = render_image_from_frame(&frame).expect("render image");

    let presented = preview_presented_frame(
        &render_image,
        PreviewRenderPresentation {
            pixel_aspect_ratio: frame_core::aspect::DisplayRatio::ONE,
            output_raster_aspect_ratio: None,
            transform: PreviewTransform {
                rotation_degrees: 90,
                flip_horizontal: false,
                flip_vertical: false,
            },
            crop: Some(EnginePreviewCrop {
                x: 0,
                y: 1,
                width: 2,
                height: 2,
            }),
            crop_source_width: Some(2),
            crop_source_height: Some(4),
        },
    )
    .expect("presented frame");

    assert_eq!(render_image.size(0).width.0, 4);
    assert_eq!(render_image.size(0).height.0, 2);
    assert_eq!(presented.full_width, 2);
    assert_eq!(presented.full_height, 4);
    assert_eq!(presented.visible_x, 0);
    assert_eq!(presented.visible_y, 1);
    assert_eq!(presented.visible_width, 2);
    assert_eq!(presented.visible_height, 2);
}

#[test]
fn sample_aspect_changes_layout_once_without_changing_native_crop_indices() {
    let root = root_with_preview_canvas_media(320, 180);
    let image = root.preview_ui.render_image.as_ref().unwrap();
    for (rotation, expected_raster, expected_ratio) in [
        (0, (320, 180), (2, 1)),
        (90, (180, 320), (1, 2)),
        (180, (320, 180), (2, 1)),
        (270, (180, 320), (1, 2)),
    ] {
        let presentation = PreviewRenderPresentation {
            pixel_aspect_ratio: frame_core::aspect::DisplayRatio::new(2, 1).unwrap(),
            output_raster_aspect_ratio: frame_core::aspect::DisplayRatio::new(16, 9),
            transform: PreviewTransform {
                rotation_degrees: rotation,
                ..Default::default()
            },
            ..Default::default()
        };
        let media = preview_media_render_state(Some(image), presentation).unwrap();
        assert_eq!((media.width, media.height), expected_raster);
        assert_eq!(media.pixel_aspect_ratio.exact_ratio(), Some(expected_ratio));
        assert_eq!(
            media.output_raster_aspect_ratio.unwrap().exact_ratio(),
            Some(if rotation == 90 || rotation == 270 {
                (9, 16)
            } else {
                (16, 9)
            })
        );
        let crop_presentation = PreviewRenderPresentation {
            crop: Some(EnginePreviewCrop {
                x: 16,
                y: 8,
                width: 128,
                height: 128,
            }),
            crop_source_width: Some(expected_raster.0),
            crop_source_height: Some(expected_raster.1),
            ..presentation
        };
        let crop = preview_presented_frame(image, crop_presentation).unwrap();
        assert_eq!(
            (
                crop.visible_x,
                crop.visible_y,
                crop.visible_width,
                crop.visible_height
            ),
            (16, 8, 128, 128)
        );
        let media = preview_media_render_state(Some(image), crop_presentation).unwrap();
        assert_eq!(
            media.output_raster_aspect_ratio.unwrap().exact_ratio(),
            Some((1, 1))
        );
        assert_eq!(
            media.display_dimensions(),
            if rotation == 90 || rotation == 270 {
                (64.0, 128.0)
            } else {
                (256.0, 128.0)
            }
        );
    }
    assert_eq!(image.size(0).width.0, 320);
    assert_eq!(image.size(0).height.0, 180);
}

#[test]
fn output_raster_aspect_follows_native_axes_and_visible_crop_with_legacy_fallback() {
    use frame_core::aspect::DisplayRatio;
    let root = root_with_preview_canvas_media(4, 2);
    let image = root.preview_ui.render_image.as_ref();
    let legacy = preview_media_render_state(image, PreviewRenderPresentation::default()).unwrap();
    assert_eq!(legacy.raster_aspect_ratio(), 2.0);
    let presentation = PreviewRenderPresentation {
        pixel_aspect_ratio: DisplayRatio::new(5, 1).unwrap(),
        output_raster_aspect_ratio: DisplayRatio::new(10, 1),
        transform: PreviewTransform {
            rotation_degrees: 90,
            ..Default::default()
        },
        crop: Some(EnginePreviewCrop {
            x: 0,
            y: 1,
            width: 2,
            height: 2,
        }),
        crop_source_width: Some(2),
        crop_source_height: Some(4),
    };
    let media = preview_media_render_state(image, presentation).unwrap();
    assert_eq!((media.width, media.height), (2, 2));
    assert_eq!(media.pixel_aspect_ratio.exact_ratio(), Some((1, 5)));
    assert_eq!(
        media.output_raster_aspect_ratio.unwrap().exact_ratio(),
        Some((1, 5))
    );
    assert_eq!(media.display_dimensions(), (0.4, 2.0));
}

#[test]
fn anamorphic_canvas_fit_and_pan_use_the_visible_pair() {
    let mut root = root_with_preview_canvas_media(320, 180);
    root.preview_ui.rendered_presentation.pixel_aspect_ratio =
        frame_core::aspect::DisplayRatio::new(2, 1).unwrap();
    let media = preview_media_render_state(
        root.preview_ui.render_image.as_ref(),
        root.preview_ui.rendered_presentation,
    )
    .unwrap();
    assert_eq!((media.width, media.height), (320, 180));
    let (width, height) = media.display_dimensions();
    assert_eq!((width, height), (640.0, 180.0));
    let layout = preview_canvas_layout_metrics(800.0, 600.0, width, height, 1.0, 0.0, 0.0).unwrap();
    assert_eq!(
        (layout.width, layout.height, layout.left, layout.top),
        (800.0, 225.0, 0.0, 187.5)
    );
    let pan = preview_canvas_pan_limits(800.0, 600.0, width, height, 4.0).unwrap();
    assert_eq!(pan, (1600.0, 600.0));
    // The overlay remains 80x80 in raster space, hence 160x80 in display space.
    let overlay_width = 0.25;
    let overlay_height = overlay_width * f64::from(media.width) / f64::from(media.height);
    assert_eq!(
        (width * overlay_width, height * overlay_height),
        (160.0, 80.0)
    );
}
