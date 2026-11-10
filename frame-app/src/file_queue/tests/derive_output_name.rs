use super::*;

#[test]
fn appends_converted_to_file_stem() {
    assert_eq!(derive_output_name("clip.mp4"), "clip_converted");
}

#[test]
fn removes_only_final_extension() {
    assert_eq!(
        derive_output_name("archive.tar.gz"),
        "archive.tar_converted"
    );
}

#[test]
fn falls_back_when_hidden_file_stem_is_empty() {
    assert_eq!(derive_output_name(".gitignore"), "output_converted");
}
