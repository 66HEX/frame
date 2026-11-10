use super::{CropRect, FrameRoot, SourceKind, SourceMetadata};
use crate::app::preview::state::PreviewOverlayImageDimensions;
use crate::preview::PreviewOverlay;
use crate::preview_engine::{PreviewRenderPresentation, rendered_frame_from_bgra_payload};
use frame_core::aspect::{DisplayRatio, SampleAspectRatio};

impl FrameRoot {
    pub(super) fn apply_anamorphic_fixture(&mut self, overlay: bool) {
        self.apply_preview_ready_fixture();
        self.source_metadata.mark_ready(
            "fixture-preview",
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(320),
                height: Some(180),
                resolution: Some("320x180".into()),
                sample_aspect_ratio: SampleAspectRatio::new(2, 1),
                duration: Some("90.4".into()),
                ..SourceMetadata::default()
            },
        );
        let frame = rendered_frame_from_bgra_payload(320, 180, 1280, 0, anamorphic_grid())
            .expect("valid deterministic fixture raster")
            .with_presentation(
                PreviewRenderPresentation {
                    pixel_aspect_ratio: DisplayRatio::new(2, 1).unwrap(),
                    output_raster_aspect_ratio: DisplayRatio::new(16, 9),
                    ..Default::default()
                },
                0,
            );
        self.preview_ui.render_image = Some(frame.render_image());
        self.preview_ui.rendered_presentation = frame.presentation;
        self.preview_ui.static_fixture_file_id = Some("fixture-preview".into());
        if overlay {
            self.seed_anamorphic_overlay();
        } else {
            self.preview_ui.crop_file_id = Some("fixture-preview".into());
            self.preview_ui.crop_mode = true;
            self.preview_ui.crop_aspect = "1:1".into();
            self.preview_ui.draft_crop = Some(CropRect {
                x: 16.0 / 320.0,
                y: 8.0 / 180.0,
                width: 128.0 / 320.0,
                height: 128.0 / 180.0,
            });
        }
    }

    fn seed_anamorphic_overlay(&mut self) {
        let overlay = PreviewOverlay {
            enabled: true,
            path: concat!(env!("CARGO_MANIFEST_DIR"), "/resources/app-icons/64x64.png").into(),
            x: 0.5,
            y: 0.5,
            width: 0.25,
            opacity: 1.0,
            anchor: "custom".into(),
        };
        self.preview_ui.overlay_file_id = Some("fixture-preview".into());
        self.preview_ui.overlay.sync_initial_overlay(Some(&overlay));
        self.preview_ui.overlay.set_overlay_mode(true, false);
        self.preview_ui.overlay_dimensions_key = Some(overlay.path.clone());
        self.preview_ui.overlay_image_dimensions = Some(PreviewOverlayImageDimensions {
            width: 64,
            height: 64,
        });
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.overlay = Some(crate::settings::OverlaySettings {
                enabled: overlay.enabled,
                path: overlay.path,
                x: overlay.x,
                y: overlay.y,
                width: overlay.width,
                opacity: overlay.opacity,
                anchor: overlay.anchor,
            });
        }
    }
}

fn anamorphic_grid() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(320 * 180 * 4);
    for y in 0..180 {
        for x in 0..320 {
            let color = if x % 16 == 0 || y % 16 == 0 {
                [200, 210, 220, 255]
            } else if (x / 16 + y / 16) % 2 == 0 {
                [88, 65, 39, 255]
            } else {
                [42, 36, 29, 255]
            };
            pixels.extend_from_slice(&color);
        }
    }
    pixels
}
