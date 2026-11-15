use super::*;

fn config() -> ConversionConfig {
    crate::conversion_runner::core_config_from_gpui(&crate::settings::ConversionConfig::default())
}

#[test]
fn stabilization_disabled_never_starts_a_capability_process() {
    check_support(&config(), "/definitely/missing/ffmpeg", || panic!("unused")).unwrap();
}

#[test]
fn stabilization_enabled_fails_closed_if_runtime_cannot_be_verified() {
    let mut config = config();
    config.video_filters.stabilization_enabled = true;
    let error = check_support(&config, "/definitely/missing/ffmpeg", || false).unwrap_err();
    assert!(error.to_string().contains("Could not verify"));
}

#[cfg(unix)]
#[test]
fn stabilization_checks_actual_complete_filter_listing_before_export() {
    use std::os::unix::fs::PermissionsExt;
    let _serial = crate::bounded_process::serial_test_processes();
    let path = std::env::temp_dir().join(format!(
        "frame-stabilization-listing-{}",
        std::process::id()
    ));
    let mut config = config();
    config.video_filters.stabilization_enabled = true;
    for (body, expected) in [
        (
            "printf 'Filters:\n .. deshake V->V Stabilize video.\n'",
            None,
        ),
        (
            "printf 'Filters:\n .. null V->V Pass video.\n'",
            Some("does not support"),
        ),
        ("printf 'Filters:\n .. deshake V'", Some("Could not verify")),
        (
            "printf 'Filters:\n .. deshake V->V Stabilize video.\n'; exit 3",
            Some("Could not verify"),
        ),
    ] {
        std::fs::write(
            &path,
            format!("#!/bin/sh\n[ \"$1\" = -filters ] || exit 12\n{body}\n"),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let result = check_support(&config, path.to_str().unwrap(), || false);
        if let Some(expected) = expected {
            let error = result.unwrap_err().to_string();
            assert!(error.contains(expected), "{error}");
        } else {
            result.unwrap();
        }
    }
    std::fs::remove_file(path).unwrap();
}

mod runtime;

#[cfg(unix)]
mod controlled {
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
                "frame-stabilization-controlled-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(
                &path,
                format!("#!/bin/sh\n[ \"$1\" = -filters ] || exit 12\n{body}\n"),
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
    fn stabilization_missing_capability_does_not_emit_export_lifecycle_events() {
        let _serial = crate::bounded_process::serial_test_processes();
        let runtime = FakeRuntime::new("printf 'Filters:\\n .. null V->V Pass video.\\n'");
        let mut config = config();
        config.video_filters.stabilization_enabled = true;
        let controller = ConversionProcessController::default();
        let mut events = Vec::new();
        let result = check_with_control(
            &config,
            runtime.executable(),
            "stabilization",
            &controller,
            &mut |event| events.push(event),
        );
        assert!(result.unwrap_err().to_string().contains("does not support"));
        assert!(events.is_empty());
        assert_eq!(controller.active_pid("stabilization"), None);
    }

    #[test]
    fn stabilization_cancelled_listing_cannot_proceed_and_frees_the_next_check() {
        let _serial = crate::bounded_process::serial_test_processes();
        let mut config = config();
        config.video_filters.stabilization_enabled = true;
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
            let cancellation = Instant::now();
            worker_controller.cancel_task("stabilization").unwrap();
            (started, cancellation)
        });
        let mut events = Vec::new();
        let result = check_with_control(
            &config,
            runtime.executable(),
            "stabilization",
            &controller,
            &mut |event| events.push(event),
        );
        let (started, cancellation) = cancel.join().unwrap();
        assert!(started, "listing fixture never started");
        assert!(cancellation.elapsed() < Duration::from_secs(2));
        assert!(!result.unwrap());
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, ConversionEvent::Cancelled(_)))
                .count(),
            1
        );
        assert!(events.iter().all(|event| {
            event.id() == "stabilization"
                && matches!(
                    event,
                    ConversionEvent::Log(_) | ConversionEvent::Cancelled(_)
                )
        }));
        assert!(!controller.is_cancelled("stabilization"));
        assert_eq!(controller.active_pid("stabilization"), None);

        let available =
            FakeRuntime::new("printf 'Filters:\\n .. deshake V->V Stabilize video.\\n'");
        let mut retry_events = Vec::new();
        assert!(
            check_with_control(
                &config,
                available.executable(),
                "stabilization",
                &controller,
                &mut |event| retry_events.push(event),
            )
            .unwrap()
        );
        assert!(retry_events.is_empty());
    }
}
