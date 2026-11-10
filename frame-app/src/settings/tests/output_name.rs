use super::*;

#[test]
fn sanitize_output_name_keeps_only_last_path_segment() {
    assert_eq!(sanitize_output_name("/tmp/render/final.mp4"), "final.mp4");
}

#[test]
fn sanitize_output_name_handles_windows_separators() {
    assert_eq!(sanitize_output_name("C:\\media\\final.mov"), "final.mov");
}

#[test]
fn sanitize_output_name_rejects_dot_segments() {
    assert_eq!(sanitize_output_name(".."), "");
}
