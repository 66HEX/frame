use super::*;

#[test]
fn disables_trim_for_unknown_metadata() {
    assert!(availability(MetadataStatus::Idle, Some(SourceMediaKind::Video), "mp4").trim_disabled);
}

#[test]
fn disables_trim_for_image_sources() {
    assert!(availability(MetadataStatus::Ready, Some(SourceMediaKind::Image), "png").trim_disabled);
}

#[test]
fn keeps_visual_controls_hidden_for_audio_sources() {
    assert!(
        availability(MetadataStatus::Ready, Some(SourceMediaKind::Audio), "mp3")
            .hide_visual_controls
    );
}

#[test]
fn hides_visual_controls_for_video_to_audio_output() {
    assert!(
        availability(MetadataStatus::Ready, Some(SourceMediaKind::Video), "mp3")
            .hide_visual_controls
    );
}

#[test]
fn enables_overlay_for_reencoded_video_output() {
    assert!(
        availability(MetadataStatus::Ready, Some(SourceMediaKind::Video), "mp4").overlay_available
    );
}

#[test]
fn disables_overlay_for_copy_mode() {
    let availability = super::super::preview_control_availability(PreviewControlInput {
        metadata_status: MetadataStatus::Ready,
        source_media_kind: Some(SourceMediaKind::Video),
        controls_disabled: false,
        processing_mode: ProcessingMode::Copy,
        container: Some("mp4"),
    });

    assert!(!availability.overlay_available);
}

#[test]
fn disables_overlay_for_gif_output() {
    assert!(
        !availability(MetadataStatus::Ready, Some(SourceMediaKind::Video), "gif").overlay_available
    );
}

#[test]
fn respects_global_control_disabled_state_for_trim() {
    let availability = super::super::preview_control_availability(PreviewControlInput {
        metadata_status: MetadataStatus::Ready,
        source_media_kind: Some(SourceMediaKind::Video),
        controls_disabled: true,
        processing_mode: ProcessingMode::Reencode,
        container: Some("mp4"),
    });

    assert!(availability.trim_disabled);
}

fn availability(
    metadata_status: MetadataStatus,
    source_media_kind: Option<SourceMediaKind>,
    container: &str,
) -> PreviewControlAvailability {
    super::super::preview_control_availability(PreviewControlInput {
        metadata_status,
        source_media_kind,
        controls_disabled: false,
        processing_mode: ProcessingMode::Reencode,
        container: Some(container),
    })
}
