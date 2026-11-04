use super::*;

#[test]
fn parse_time_to_seconds_accepts_full_hh_mm_ss_fraction() {
    assert_close(super::super::parse_time_to_seconds("01:02:03.250"), 3723.25);
}

#[test]
fn parse_time_to_seconds_returns_zero_for_partial_timecodes() {
    assert_eq!(super::super::parse_time_to_seconds("02:03"), 0.0);
}

#[test]
fn format_time_matches_trim_precision() {
    assert_eq!(super::super::format_time(3723.25), "01:02:03.250");
}

#[test]
fn format_time_pads_single_digit_seconds() {
    assert_eq!(super::super::format_time(61.2), "00:01:01.200");
}

#[test]
fn format_time_carries_rounded_seconds_across_timecode_boundaries() {
    assert_eq!(super::super::format_time(59.9999), "00:01:00.000");
    assert_eq!(super::super::format_time(3599.9999), "01:00:00.000");
}
