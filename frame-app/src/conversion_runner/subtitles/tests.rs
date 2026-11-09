use super::*;

fn config() -> ConversionConfig {
    let mut config = crate::conversion_runner::core_config_from_gpui(
        &crate::settings::ConversionConfig::default(),
    );
    config.container = "mkv".into();
    config.external_subtitle_tracks = vec![frame_core::types::ExternalSubtitleTrack {
        path: "/tmp/captions.srt".into(),
        ..frame_core::types::ExternalSubtitleTrack::default()
    }];
    config
}

#[test]
fn unused_subtitle_trim_never_resolves_or_launches_the_runtime() {
    let mut config = config();
    let missing = "/definitely/missing/ffmpeg";
    for start in [None, Some("0"), Some(" ")] {
        config.start_time = start.map(str::to_string);
        check_subtitle_trim_support(&config, missing, || panic!("unexpected process check"))
            .unwrap();
    }
    config.start_time = Some("8".into());
    config.external_subtitle_tracks[0].path = "/tmp/bitmap.sup".into();
    check_subtitle_trim_support(&config, missing, || panic!("bitmap needs no BSF check")).unwrap();
    config.external_subtitle_tracks.clear();
    config.selected_subtitle_tracks = vec![3];
    config.subtitle_burn_path = Some("/tmp/burn.srt".into());
    check_subtitle_trim_support(&config, missing, || {
        panic!("embedded/burn needs no BSF check")
    })
    .unwrap();
}

#[cfg(unix)]
mod native {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    struct FakeRuntime(PathBuf);

    impl FakeRuntime {
        fn new(body: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "frame-subtitle-runtime-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(
                &path,
                format!("#!/bin/sh\n[ \"$5\" = -bsfs ] || exit 12\n{body}\n"),
            )
            .unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }

        fn executable(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for FakeRuntime {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
            let _ = fs::remove_file(self.0.with_extension("started"));
        }
    }

    #[test]
    fn actual_bounded_listing_distinguishes_missing_from_unverified_subtitle_support() {
        let _serial = crate::bounded_process::serial_test_processes();
        let mut config = config();
        config.start_time = Some("8".into());
        for (body, expected) in [
            ("printf 'Bitstream filters:\\nnoise\\nsetts\\n'", None),
            (
                "printf 'Bitstream filters:\\nnoise_extra\\nsetts\\n'",
                Some("Use a build"),
            ),
            (
                "printf 'Bitstream filters:\\nnoise\\nsetts'",
                Some("Could not verify"),
            ),
            (
                "printf 'Bitstream filters:\\nnoise\\nsetts\\n' >&2",
                Some("Could not verify"),
            ),
            (
                "printf 'Bitstream filters:\\nnoise\\nsetts\\n'; exit 7",
                Some("Could not verify"),
            ),
        ] {
            let runtime = FakeRuntime::new(body);
            let result = check_subtitle_trim_support(&config, runtime.executable(), || false);
            if let Some(expected) = expected {
                let error = result.unwrap_err().to_string();
                assert!(error.contains(expected), "{error}");
            } else {
                result.unwrap();
            }
        }
    }

    #[test]
    fn cancellation_during_subtitle_listing_is_consumed_before_export_can_start() {
        let _serial = crate::bounded_process::serial_test_processes();
        let mut config = config();
        config.start_time = Some("8".into());
        let runtime = FakeRuntime::new(": > \"$0.started\"; exec /bin/sleep 30");
        let marker = runtime.0.with_extension("started");
        let controller = ConversionProcessController::default();
        let worker_controller = controller.clone();
        let cancel = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(3);
            while !marker.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            let started = marker.exists();
            worker_controller.cancel_task("subtitles").unwrap();
            started
        });
        let mut events = Vec::new();
        let proceed = check_subtitle_trim_with_control(
            &config,
            runtime.executable(),
            "subtitles",
            &controller,
            &mut |event| events.push(event),
        )
        .unwrap();
        assert!(cancel.join().unwrap(), "listing fixture never started");
        assert!(!proceed);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, ConversionEvent::Cancelled(_)))
                .count(),
            1
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            ConversionEvent::Started(_) | ConversionEvent::Completed(_)
        )));
        assert!(!controller.is_cancelled("subtitles"));
        assert_eq!(controller.active_pid("subtitles"), None);
        let available = FakeRuntime::new("printf 'Bitstream filters:\\nnoise\\nsetts\\n'");
        check_subtitle_trim_support(&config, available.executable(), || false).unwrap();
    }
}
