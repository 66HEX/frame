use super::*;

#[test]
fn source_kind_falls_back_to_audio_when_metadata_has_no_video_codec() {
    let metadata = SourceMetadata::default();

    assert_eq!(metadata.source_kind(), SourceKind::Audio);
}

#[test]
fn source_kind_defaults_to_video_when_metadata_is_missing() {
    assert_eq!(source_kind_for(None), SourceKind::Video);
}
