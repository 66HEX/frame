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
