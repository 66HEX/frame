use super::*;

#[test]
fn returns_free_as_unconstrained() {
    assert_eq!(aspect_value("free"), None);
}

#[test]
fn returns_original_wide_ratio() {
    assert_close(aspect_value("16:9").unwrap(), 16.0 / 9.0);
}

#[test]
fn returns_none_for_unknown_ratio() {
    for invalid in ["unknown", "0:1", "1:0", "-1:2", "65536:1", "1:2:3"] {
        assert_eq!(aspect_value(invalid), None);
    }
    assert_close(aspect_value("2:1").unwrap(), 2.0);
    assert_close(aspect_value("239:100").unwrap(), 2.39);
}
