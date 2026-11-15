use std::sync::atomic::Ordering;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::bounded_process::{CleanupOutcome, CleanupTask, ProcessReservation};
use crate::preview_engine::PreviewEngineError;

use super::process::{ProcessHandles, RunningPreviewProcess, take_process_handles};
use super::video_stream::SpawnedVideoStream;

pub(super) struct StartupCleanup(pub(super) Vec<ProcessHandles>);

impl CleanupTask for StartupCleanup {
    fn cleanup_step(&mut self) -> bool {
        let mut complete = true;
        for handles in &mut self.0 {
            let already_exited = handles
                .child
                .as_mut()
                .and_then(|child| child.try_wait().ok())
                .flatten();
            if let Some(status) = already_exited
                && let Some(exit) = &handles.audio_exit
            {
                exit.publish(exit.record_exit(status));
            }

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
                if already_exited.is_some() {
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
    pub(super) fn retire_video_for_handoff(
        &self,
        generation: u64,
        stream: SpawnedVideoStream,
    ) -> Result<SpawnedVideoStream, PreviewEngineError> {
        self.retire_for_handoff(generation, stream, take_process_handles(&self.process))
    }

    pub(super) fn retire_for_handoff(
        &self,
        generation: u64,
        stream: SpawnedVideoStream,
        handles: ProcessHandles,
    ) -> Result<SpawnedVideoStream, PreviewEngineError> {
        let mut cleanup = StartupCleanup(vec![handles]);
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if cleanup.cleanup_step() {
                return Ok(stream);
            }
            if self.cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
                let (candidate, reservation) = stream.into_handles();
                cleanup.0.extend(self.detach_failed_video_start(generation));
                cleanup.0.push(candidate);
                let outcome = reservation.cleanup(cleanup, Duration::ZERO);
                return Err(PreviewEngineError::Ffmpeg(format!(
                    "previous preview cleanup did not finish before handoff (cleanup: {outcome:?}); retry shortly"
                )));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

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
