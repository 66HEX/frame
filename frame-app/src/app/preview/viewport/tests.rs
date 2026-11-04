use super::*;
use crate::app::preview::*;
use crate::app::*;

#[test]
fn preview_image_transformation_converts_degrees_to_radians() {
    let transformation = preview_image_transformation(
        PreviewTransform {
            rotation_degrees: 90,
            flip_horizontal: false,
            flip_vertical: false,
        },
        point(px(50.0), px(25.0)),
        1.0,
    );

    assert!((transformation.rotation_scale[0][0]).abs() < 0.000_001);
    assert!((transformation.rotation_scale[0][1] + 1.0).abs() < 0.000_001);
    assert!((transformation.rotation_scale[1][0] - 1.0).abs() < 0.000_001);
    assert!((transformation.rotation_scale[1][1]).abs() < 0.000_001);
}

#[test]
fn preview_canvas_layout_ready_requires_measured_viewport() {
    assert!(!preview_canvas_layout_ready(
        PreviewCanvasRenderState::default()
    ));

    assert!(preview_canvas_layout_ready(PreviewCanvasRenderState {
        viewport_width: 640.0,
        viewport_height: 360.0,
        ..PreviewCanvasRenderState::default()
    }));
}

#[test]
fn preview_waiting_for_visual_render_only_hides_visual_media_without_frame() {
    let mut state = PreviewShellState {
        tooltip_visible_id: None,
        palette: theme::palette(crate::appearance::ColorTheme::Dark),
        selected_file_name: Some("sample.mov".to_string()),
        metadata_status: PreviewMetadataStatus::Ready,
        metadata_error: None,
        controls_disabled: false,
        availability: PreviewControlAvailability {
            media_kind: PreviewMediaKind::Video,
            hide_visual_controls: false,
            trim_disabled: false,
            overlay_available: true,
        },
        playback: PreviewPlaybackState::new(false),
        duration_seconds: 1.0,
        canvas: PreviewCanvasRenderState::default(),
        crop: PreviewCropRenderState {
            crop_mode: false,
            draft_crop: None,
            applied_crop: None,
            crop_aspect: "free".to_string(),
            has_crop_dimensions: false,
            rotation: "0".to_string(),
            flip_horizontal: false,
            flip_vertical: false,
        },
        overlay: PreviewOverlayRenderState {
            overlay_mode: false,
            has_overlay: false,
            overlay: None,
            image_dimensions: None,
        },
        presentation: PreviewRenderPresentation::default(),
        media: None,
        render_image: None,
        runtime_error: None,
    };

    assert!(preview_waiting_for_visual_render(&state));

    state.availability.media_kind = PreviewMediaKind::Audio;

    assert!(!preview_waiting_for_visual_render(&state));
}
