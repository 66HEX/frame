use super::*;

#[test]
fn returns_zero_bytes_label() {
    assert_eq!(format_file_size(0), "0 B");
}

#[test]
fn trims_trailing_decimal_zeroes_like_javascript_parse_float() {
    assert_eq!(format_file_size(1536), "1.5 KB");
}

#[test]
fn formats_megabytes_without_unneeded_decimals() {
    assert_eq!(format_file_size(1024 * 1024), "1 MB");
}
