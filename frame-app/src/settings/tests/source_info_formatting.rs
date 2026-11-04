use super::*;

#[test]
fn format_source_duration_formats_colon_time_without_fraction() {
    assert_eq!(format_source_duration(Some("01:02:03.450")), "01:02:03");
}

#[test]
fn format_source_duration_formats_numeric_seconds() {
    assert_eq!(format_source_duration(Some("90.4")), "00:01:30");
}

#[test]
fn format_source_duration_keeps_unparseable_values() {
    assert_eq!(format_source_duration(Some("unknown")), "unknown");
}

#[test]
fn format_source_resolution_prefers_dimensions() {
    let metadata = SourceMetadata {
        resolution: Some("1920x1080".to_string()),
        width: Some(3840),
        height: Some(2160),
        ..SourceMetadata::default()
    };

    assert_eq!(format_source_resolution(&metadata), "3840×2160");
}

#[test]
fn format_source_frame_rate_trims_trailing_zeroes() {
    assert_eq!(format_source_frame_rate(Some(29.970)), "29.97 fps");
}

#[test]
fn format_source_bitrate_kbps_uses_megabits_above_threshold() {
    assert_eq!(format_source_bitrate_kbps(Some(2450.0)), "2.45 Mb/s");
}

#[test]
fn format_source_container_bitrate_parses_bits_per_second() {
    assert_eq!(
        format_source_container_bitrate(Some("1250000")),
        "1.25 Mb/s"
    );
}

#[test]
fn format_source_hz_uses_kilohertz_above_threshold() {
    assert_eq!(format_source_hz(Some("48000")), "48 kHz");
}
