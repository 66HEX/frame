use super::*;

fn scan(packets: serde_json::Value) -> Scan {
    let mut value = serde_json::json!({
        "streams": [{"index": 7, "codec_name": "opus", "time_base": "1/48000", "start_pts": 0}],
        "format": {"start_time": "0"}
    });
    value["packets"] = packets;
    serde_json::from_value(value).unwrap()
}

fn window() -> Window {
    Window {
        start: 0,
        end: Some(213_000),
        origin: 0,
    }
}

#[test]
fn packet_timing_must_be_complete_for_the_selected_stream() {
    for packets in [
        serde_json::json!([{"stream_index": 7, "pts": 0}]),
        serde_json::json!([{"stream_index": 7, "duration": 960}]),
        serde_json::json!([{"stream_index": 7, "pts": 0, "duration": 0}]),
        serde_json::json!([{"stream_index": 1, "pts": 0, "duration": 960}]),
    ] {
        assert!(validate_scan(&scan(packets), 7, &window()).is_err());
    }
    assert!(
        validate_scan(
            &scan(serde_json::json!([
                {"stream_index": 7, "pts": -312, "duration": 960},
                {"stream_index": 7, "pts": 648, "duration": 960}
            ])),
            7,
            &window()
        )
        .is_ok()
    );
}

#[test]
fn an_empty_selected_interval_cannot_establish_packet_timing() {
    let empty = scan(serde_json::json!([]));
    for start in [0, 100_000, 457_000] {
        let requested = Window {
            start,
            end: Some(813_000),
            origin: 0,
        };
        assert!(
            validate_scan(&empty, 7, &requested).is_err(),
            "a successful empty scan must not authorize an unobserved Opus trim at {start}"
        );
    }
}

#[test]
fn a_nearby_seek_does_not_prove_required_preroll() {
    let window = Window {
        start: 1_457_000,
        end: Some(1_800_000),
        origin: 0,
    };
    let missed = scan(serde_json::json!([{"stream_index": 7, "pts": 70_000, "duration": 960}]));
    assert!(validate_scan(&missed, 7, &window).is_err());
    let covered = scan(serde_json::json!([{"stream_index": 7, "pts": 60_000, "duration": 960}]));
    assert!(validate_scan(&covered, 7, &window).is_ok());
    assert_eq!(window.interval().unwrap(), "1.257000%1.920000");
}

#[test]
fn a_small_interval_at_a_late_position_does_not_request_the_whole_source() {
    let window = Window {
        start: 36_001_457_000,
        end: Some(36_001_800_000),
        origin: 5_000_000,
    };
    assert_eq!(window.interval().unwrap(), "36001.257000%36001.920000");
    assert_eq!(seconds(-312), "-0.000312");
}

#[test]
fn window_uses_the_existing_export_clock_and_literal_precision() {
    let mut config = crate::conversion_runner::core_config_from_gpui(
        &crate::settings::ConversionConfig::default(),
    );
    let probe = ProbeMetadata {
        source_timing: Some(frame_core::source_timing::SourceTiming {
            format_start_microseconds: Some(5_000_000),
            format_start_invalid: false,
            first_video_stream_index: None,
            streams: Vec::new(),
        }),
        ..ProbeMetadata::default()
    };
    assert_eq!(Window::from_config(&config, &probe).unwrap(), None);
    config.start_time = Some("0".into());
    assert_eq!(Window::from_config(&config, &probe).unwrap(), None);
    config.start_time = None;
    config.end_time = Some("0.2130009".into());
    let end = Window::from_config(&config, &probe).unwrap().unwrap();
    assert_eq!(end.interval().unwrap(), "%5.333000");
    config.start_time = Some("1.4570009".into());
    config.end_time = Some("1.8000009".into());
    let both = Window::from_config(&config, &probe).unwrap().unwrap();
    assert_eq!(both.interval().unwrap(), "6.257000%6.920000");
}

static PROCESS_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
#[ignore = "native Opus probe child fixture"]
fn fake_opus_probe() {
    let Ok(mode) = std::env::var("FRAME_OPUS_PROBE_TEST_MODE") else {
        return;
    };
    if let Some(marker) = std::env::var_os("FRAME_OPUS_PROBE_TEST_MARKER") {
        std::fs::write(marker, b"started").unwrap();
    }
    if mode == "silent" {
        std::thread::sleep(Duration::from_secs(2));
    }
    if mode == "flood" {
        use std::io::Write;
        std::io::stdout().write_all(&vec![b'x'; 64 * 1024]).unwrap();
    }
    print!("{{\"packets\":[]}}");
    std::process::exit(0);
}

fn fixture(mode: &str) -> CommandSpec {
    let mut command = CommandSpec::new(std::env::current_exe().unwrap());
    command.args = [
        "--exact",
        "conversion_runner::opus::tests::fake_opus_probe",
        "--ignored",
        "--nocapture",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    command.env = vec![("FRAME_OPUS_PROBE_TEST_MODE".into(), mode.into())];
    command
}

fn limits() -> Limits {
    Limits {
        timeout: Duration::from_millis(200),
        cleanup_timeout: Duration::from_secs(1),
        stdout_bytes: 1024,
        stderr_bytes: 1024,
    }
}

#[test]
fn packet_scan_timeout_and_output_limit_fail_closed_then_release_the_domain() {
    let _serial = PROCESS_TESTS.lock().unwrap();
    let start = Instant::now();
    let timeout = read_scan(&fixture("silent"), limits(), &|| false).unwrap_err();
    assert!(timeout.to_string().contains("time limit"), "{timeout}");
    assert!(start.elapsed() < Duration::from_secs(1));
    let flood = read_scan(&fixture("flood"), limits(), &|| false).unwrap_err();
    assert!(flood.to_string().contains("output limit"), "{flood}");
    let retry = read_scan(&fixture("valid"), limits(), &|| false).unwrap();
    assert!(String::from_utf8_lossy(&retry).contains("packets"));
}

#[test]
fn packet_scan_cancellation_is_bounded_and_allows_retry() {
    let _serial = PROCESS_TESTS.lock().unwrap();
    let marker =
        std::env::temp_dir().join(format!("frame-opus-probe-started-{}", std::process::id()));
    assert!(!marker.exists());
    let mut command = fixture("silent");
    command.env.push((
        "FRAME_OPUS_PROBE_TEST_MARKER".into(),
        marker.as_os_str().to_owned(),
    ));
    let start = Instant::now();
    let cancelled = read_scan(
        &command,
        Limits {
            timeout: Duration::from_secs(2),
            ..limits()
        },
        &|| marker.exists(),
    )
    .unwrap_err();
    assert!(marker.exists(), "the probe must start before cancellation");
    std::fs::remove_file(marker).unwrap();
    assert!(cancelled.to_string().contains("cancelled"), "{cancelled}");
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(read_scan(&fixture("valid"), limits(), &|| false).is_ok());
}
