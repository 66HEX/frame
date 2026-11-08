use super::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicU64;

fn request() -> AutoCropRequest {
    AutoCropRequest {
        path: "source.mp4".into(),
        video_stream_index: None,
        width: 360,
        height: 240,
        start_seconds: 0.0,
        end_seconds: 100.0,
    }
}

#[test]
fn long_videos_sample_five_distributed_windows_within_trim() {
    let request = AutoCropRequest {
        start_seconds: 20.0,
        end_seconds: 120.0,
        ..request()
    };
    let windows = sample_windows(&request).unwrap();
    assert_eq!(windows.len(), 5);
    let expected = [29.0, 49.0, 69.0, 89.0, 109.0];
    for (window, start) in windows.iter().zip(expected) {
        assert!((window.start - start).abs() < 1e-9);
        assert!((window.duration - 2.0).abs() < f64::EPSILON);
        assert!(window.start >= request.start_seconds);
        assert!(window.start + window.duration <= request.end_seconds);
    }
}

#[test]
fn short_videos_stay_within_the_available_time_range() {
    for duration in [0.04, 0.1, 1.0, 2.0, 2.1, 4.0, 8.3, 10.0] {
        let request = AutoCropRequest {
            start_seconds: 13.0,
            end_seconds: 13.0 + duration,
            ..request()
        };
        let windows = sample_windows(&request).unwrap();
        assert!(!windows.is_empty() && windows.len() <= 5);
        for window in &windows {
            assert!(window.start >= request.start_seconds);
            assert!(window.start + window.duration <= request.end_seconds + 1e-9);
            assert!(window.duration > 0.0);
        }
        assert_eq!(windows.len() == 1, duration <= SAMPLE_SECONDS);
    }
}

#[test]
fn unknown_or_invalid_metadata_is_not_analyzed() {
    for request in [
        AutoCropRequest {
            width: 0,
            ..request()
        },
        AutoCropRequest {
            end_seconds: f64::INFINITY,
            ..request()
        },
        AutoCropRequest {
            start_seconds: f64::NAN,
            ..request()
        },
        AutoCropRequest {
            start_seconds: -1.0,
            ..request()
        },
        AutoCropRequest {
            start_seconds: 100.0,
            ..request()
        },
    ] {
        assert_eq!(sample_windows(&request), Err(AutoCropError::InvalidInput));
    }
}

#[test]
fn crop_parser_rejects_black_frames_bad_geometry_and_tiny_isolated_content() {
    for crop in [
        "-2:-2:180:120",
        "320:180:60:30",
        "0:180:0:30",
        "1:1:0:0",
        "64:32:100:100",
        "320:180:0:30:4",
        "320:180:4294967295:30",
    ] {
        assert_eq!(
            parse_crop(
                &format!("[Parsed_cropdetect_1 @ 0x1] crop={crop}"),
                360,
                240
            ),
            None
        );
    }
    assert_eq!(
        parse_crop("Input metadata crop=320:180:18:30", 360, 240),
        None
    );
    assert_eq!(
        parse_crop(
            "[Parsed_cropdetect_1 @ 0x1] t:4.2 crop=320:180:18:30",
            360,
            240
        ),
        Some(DetectedCrop {
            x: 18,
            y: 30,
            width: 320,
            height: 180
        })
    );
}

#[test]
fn union_preserves_all_content_observed_in_different_samples() {
    let letterbox = DetectedCrop {
        x: 0,
        y: 30,
        width: 360,
        height: 180,
    };
    let pillarbox = DetectedCrop {
        x: 30,
        y: 0,
        width: 300,
        height: 240,
    };
    assert_eq!(
        letterbox.union(pillarbox),
        DetectedCrop {
            x: 0,
            y: 0,
            width: 360,
            height: 240
        }
    );
    assert_eq!(
        DetectedCrop {
            x: 0,
            y: 0,
            width: 360,
            height: 240
        }
        .preserve_source_edges(361, 241),
        DetectedCrop {
            x: 0,
            y: 0,
            width: 361,
            height: 241
        }
    );
}

#[test]
fn sample_command_maps_selected_stream_and_keeps_source_orientation() {
    let request = AutoCropRequest {
        video_stream_index: Some(3),
        ..request()
    };
    let args = sample_args(
        &request,
        SampleWindow {
            start: 12.0,
            duration: 2.0,
        },
    );
    assert!(args.windows(2).any(|pair| pair == ["-map", "0:3"]));
    assert!(args.windows(2).any(|pair| pair == ["-ss", "12.000000"]));
    assert!(
        !args.iter().any(|arg| arg == "-noautorotate"
            || arg.contains("scale=")
            || arg.contains("transpose="))
    );
    assert!(
        args.iter()
            .any(|arg| arg.contains("cropdetect=") && arg.contains("round=2"))
    );
}

#[test]
fn cancelled_requests_do_not_start_a_process() {
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        detect_with_executable(&request(), &cancelled, "/no/ffmpeg"),
        Err(AutoCropError::Cancelled)
    );
}

struct TemporaryFile(PathBuf);

impl TemporaryFile {
    fn new(extension: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "frame-auto-crop-{}-{}.{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed),
            extension
        )))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(unix)]
#[test]
fn stalled_process_is_reaped_on_timeout_and_cancellation() {
    use std::os::unix::fs::PermissionsExt;
    let script = TemporaryFile::new("sh");
    std::fs::write(script.path(), "#!/bin/sh\nexec sleep 10\n").unwrap();
    std::fs::set_permissions(script.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let executable = script.path().to_str().unwrap();
    let request = request();
    let window = SampleWindow {
        start: 0.0,
        duration: 2.0,
    };
    let started = Instant::now();
    let result = process::analyze_window(
        executable,
        &request,
        window,
        &AtomicBool::new(false),
        Instant::now() + Duration::from_millis(100),
    );
    assert_eq!(result, Err(AutoCropError::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(3));

    let cancelled = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(100));
            cancelled.store(true, Ordering::Relaxed);
        });
        let started = Instant::now();
        assert_eq!(
            process::analyze_window(
                executable,
                &request,
                window,
                &cancelled,
                Instant::now() + Duration::from_secs(5)
            ),
            Err(AutoCropError::Cancelled)
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    });
}

fn generate_fixture(filter: &str, output: &Path) {
    let result = crate::process_launch::output(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
                filter,
                "-c:v",
                "ffv1",
            ])
            .arg(output),
    )
    .expect("run fixture FFmpeg");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
#[ignore = "requires FFmpeg with lavfi and FFV1; run explicitly with --ignored"]
fn real_ffmpeg_detects_asymmetric_bars_and_handles_no_bars_and_black_video() {
    for (filter, expected) in [
        (
            "color=white:size=320x180:rate=10:duration=6,pad=360:240:18:30:black",
            Ok(DetectedCrop {
                x: 18,
                y: 30,
                width: 320,
                height: 180,
            }),
        ),
        (
            "color=white:size=360x240:rate=10:duration=6",
            Ok(DetectedCrop {
                x: 0,
                y: 0,
                width: 360,
                height: 240,
            }),
        ),
        (
            "color=black:size=360x240:rate=10:duration=6",
            Err(AutoCropError::NoReliableCrop),
        ),
    ] {
        let file = TemporaryFile::new("mkv");
        generate_fixture(filter, file.path());
        let request = AutoCropRequest {
            path: file.path().to_string_lossy().into_owned(),
            end_seconds: 6.0,
            ..request()
        };
        assert_eq!(
            detect_auto_crop(&request, &AtomicBool::new(false)),
            expected
        );
    }
}

#[test]
#[ignore = "requires FFmpeg with lavfi and FFV1; run explicitly with --ignored"]
fn real_ffmpeg_preserves_content_when_borders_change_and_ignores_dark_intro() {
    for (filter, expected) in [
        (
            "color=black:size=360x240:rate=10:duration=12,drawbox=x=18:y=30:w=320:h=180:color=white:t=fill:enable='gte(t,4)'",
            DetectedCrop {
                x: 18,
                y: 30,
                width: 320,
                height: 180,
            },
        ),
        (
            "color=black:size=360x240:rate=10:duration=12,drawbox=x=18:y=30:w=320:h=180:color=white:t=fill,drawbox=x=0:y=0:w=360:h=240:color=white:t=fill:enable='gte(t,6)'",
            DetectedCrop {
                x: 0,
                y: 0,
                width: 360,
                height: 240,
            },
        ),
    ] {
        let file = TemporaryFile::new("mkv");
        generate_fixture(filter, file.path());
        let request = AutoCropRequest {
            path: file.path().to_string_lossy().into_owned(),
            end_seconds: 12.0,
            ..request()
        };
        assert_eq!(
            detect_auto_crop(&request, &AtomicBool::new(false)),
            Ok(expected)
        );
    }
}

#[test]
#[ignore = "requires FFmpeg with lavfi and libx264; run explicitly with --ignored"]
fn real_ffmpeg_uses_display_orientation_and_the_selected_video_stream() {
    let source = TemporaryFile::new("mp4");
    let rotated = TemporaryFile::new("mp4");
    let executable = crate::runtime_binaries::ffmpeg_executable();
    let output = crate::process_launch::output(
        Command::new(&executable)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
                "color=white:size=360x240:rate=10:duration=6",
                "-f",
                "lavfi",
                "-i",
                "color=white:size=320x180:rate=10:duration=6,pad=360:240:20:30:black",
                "-map",
                "0:v",
                "-map",
                "1:v",
                "-c:v",
                "libx264",
                "-crf",
                "0",
            ])
            .arg(source.path()),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let selected = AutoCropRequest {
        path: source.path().to_string_lossy().into_owned(),
        video_stream_index: Some(1),
        end_seconds: 6.0,
        ..request()
    };
    assert_eq!(
        detect_auto_crop(&selected, &AtomicBool::new(false)),
        Ok(DetectedCrop {
            x: 20,
            y: 30,
            width: 320,
            height: 180
        })
    );

    let output = crate::process_launch::output(
        Command::new(executable)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-display_rotation",
                "90",
                "-i",
            ])
            .arg(source.path())
            .args(["-map", "0:1", "-c", "copy"])
            .arg(rotated.path()),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rotated_request = AutoCropRequest {
        path: rotated.path().to_string_lossy().into_owned(),
        video_stream_index: Some(0),
        width: 240,
        height: 360,
        end_seconds: 6.0,
        ..request()
    };
    assert_eq!(
        detect_auto_crop(&rotated_request, &AtomicBool::new(false)),
        Ok(DetectedCrop {
            x: 30,
            y: 20,
            width: 180,
            height: 320
        })
    );
}
