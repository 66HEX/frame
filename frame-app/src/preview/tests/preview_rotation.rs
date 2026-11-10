use super::*;

#[test]
fn parses_original_rotation_strings() {
    assert_eq!(PreviewRotation::from("90"), PreviewRotation::Deg90);
    assert_eq!(PreviewRotation::from("180"), PreviewRotation::Deg180);
    assert_eq!(PreviewRotation::from("270"), PreviewRotation::Deg270);
}

#[test]
fn treats_unknown_rotation_as_zero() {
    assert_eq!(PreviewRotation::from(""), PreviewRotation::Deg0);
    assert_eq!(PreviewRotation::from("45"), PreviewRotation::Deg0);
}
