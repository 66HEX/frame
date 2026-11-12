use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use super::cleanup_broker::CleanupBroker;
use super::video_stream::check_startup_cleanup;

/// Real process fixtures share the same application-wide cleanup admission.
/// Serialize fixture scopes, while preserving all threads and competing sessions
/// within each scenario. Declare this guard before process owners so they drop
/// first and their retained cleanup is checked before the next fixture starts.
pub(in crate::preview_engine) struct ProcessFixtureGuard {
    _exclusive: MutexGuard<'static, ()>,
}

pub(in crate::preview_engine) fn process_fixture() -> ProcessFixtureGuard {
    static FIXTURES: Mutex<()> = Mutex::new(());
    let guard = ProcessFixtureGuard {
        _exclusive: FIXTURES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    };
    assert!(
        cleanup_finished(),
        "previous preview fixture retained cleanup"
    );
    guard
}

impl Drop for ProcessFixtureGuard {
    fn drop(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !cleanup_finished() {
            if Instant::now() >= deadline {
                // Preserve the original assertion on an already-failing test.
                // A successful test must never hide an owner that failed to drain.
                if std::thread::panicking() {
                    eprintln!("preview fixture cleanup did not finish before its deadline");
                    return;
                }
                panic!("preview fixture cleanup did not finish before its deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn cleanup_finished() -> bool {
    CleanupBroker::shared()
        .expect("preview cleanup broker")
        .check_admission()
        .is_ok()
        && check_startup_cleanup().is_ok()
}
