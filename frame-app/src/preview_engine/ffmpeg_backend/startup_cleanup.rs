use std::sync::atomic::Ordering;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::bounded_process::{CleanupOutcome, CleanupTask, ProcessReservation};
use crate::preview_engine::PreviewEngineError;

use super::process::{ProcessHandles, RunningPreviewProcess};

struct StartupCleanup(Vec<ProcessHandles>);

impl CleanupTask for StartupCleanup {
    fn cleanup_step(&mut self) -> bool {
        let mut complete = true;
        for handles in &mut self.0 {
            if let Some(stop) = &handles.stop_requested {
                if let Some(start) = &handles.audio_start {
                    start.request_stop(stop);
                } else {
                    stop.store(true, Ordering::Release);
                }
            }
            if let Some(exit) = &handles.audio_exit {
                exit.cancel();
            }
            if let Some(child) = handles.child.as_mut() {
                if let Ok(Some(_)) = child.try_wait() {
                    handles.child = None;
                } else {
                    let _ = child.kill();
                    complete = false;
                }
            }
            complete &= join_finished(&mut handles.stdout_worker);
            complete &= join_finished(&mut handles.stderr_worker);
        }
        complete
    }
}

fn join_finished(worker: &mut Option<JoinHandle<()>>) -> bool {
    if worker.as_ref().is_some_and(JoinHandle::is_finished)
        && let Some(worker) = worker.take()
    {
        let _ = worker.join();
    }
    worker.is_none()
}

impl RunningPreviewProcess {
    pub(super) fn fail_video_start_bounded(
        &self,
        generation: u64,
        error: PreviewEngineError,
        candidate: Option<ProcessHandles>,
        reservation: ProcessReservation,
    ) -> PreviewEngineError {
        let mut handles = self.detach_failed_video_start(generation);
        handles.extend(candidate);
        match reservation.cleanup(StartupCleanup(handles), Duration::from_secs(1)) {
            CleanupOutcome::Complete => error,
            CleanupOutcome::Pending => PreviewEngineError::Ffmpeg(format!(
                "{error}; preview startup cleanup is still in progress; retry shortly"
            )),
        }
    }
}
