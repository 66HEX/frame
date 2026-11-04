use super::*;

#[test]
fn uses_final_extension() {
    assert_eq!(original_format_from_name("archive.tar.gz"), "gz");
}

#[test]
fn falls_back_when_trailing_dot_has_no_extension() {
    assert_eq!(original_format_from_name("clip."), "unknown");
}
