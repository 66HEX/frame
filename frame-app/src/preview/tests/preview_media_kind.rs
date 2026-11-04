use super::*;

#[test]
fn stays_unknown_until_metadata_is_ready() {
    assert_eq!(
        super::super::preview_media_kind(MetadataStatus::Loading, Some(SourceMediaKind::Video),),
        PreviewMediaKind::Unknown
    );
}

#[test]
fn uses_ready_metadata_kind() {
    assert_eq!(
        super::super::preview_media_kind(MetadataStatus::Ready, Some(SourceMediaKind::Image)),
        PreviewMediaKind::Image
    );
}
