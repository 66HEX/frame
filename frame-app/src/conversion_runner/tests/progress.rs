use super::*;

#[test]
fn ffmpeg_progress_uses_duration_line_before_time_line() {
    let mut duration = None;

    assert_eq!(
        ffmpeg_progress_from_line("Duration: 00:00:20.00, start: 0.000000", 0.0, &mut duration),
        None
    );

    let progress =
        ffmpeg_progress_from_line("frame=12 time=00:00:05.00 speed=1x", 0.0, &mut duration);

    assert_eq!(progress, Some(25.0));
}

#[test]
fn ffmpeg_progress_prefers_trim_expected_duration() {
    let mut duration = Some(100.0);

    let progress =
        ffmpeg_progress_from_line("frame=12 time=00:00:05.00 speed=1x", 10.0, &mut duration);

    assert_eq!(progress, Some(50.0));
}
