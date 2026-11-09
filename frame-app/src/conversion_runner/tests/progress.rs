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

#[test]
fn progress_duration_uses_the_global_negative_start_sign() {
    for (start, end, expected, log_time) in [
        ("-00:00:00.5", ".5", 1.0, "00:00:00.50"),
        ("-1:30", "0", 90.0, "00:00:45.00"),
    ] {
        let mut config = core_config_from_gpui(&GpuiConversionConfig::default());
        config.start_time = Some(start.into());
        config.end_time = Some(end.into());
        let duration = expected_duration_seconds(&config);
        assert_eq!(duration, expected);
        assert_eq!(
            ffmpeg_progress_from_line(
                &format!("frame=12 time={log_time} speed=1x"),
                duration,
                &mut None
            ),
            Some(50.0)
        );
    }
}
