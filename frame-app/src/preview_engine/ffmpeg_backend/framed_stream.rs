//! Native framed-stream cleanup, preserving generation and child ownership.

use super::audio_stream::PendingAudioStart;
use super::clock::{PreviewPlaybackClock, lock_playback, mark_playback_ended};
use super::decode::{DecodedPreviewFrame, RenderImageIdentity, read_stream_frame};
use super::process::{RunningProcessState, lock_process, push_stderr_line, stop_process_handles};
use super::video_stream::SpawnedVideoStream;
use crate::preview_engine::PreviewEngineError;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

pub(super) struct FramedStreamOwner {
    pub(super) process: Arc<Mutex<RunningProcessState>>,
    pub(super) audio: Arc<Mutex<RunningProcessState>>,
    pub(super) pending_audio: Arc<Mutex<Option<PendingAudioStart>>>,
    pub(super) stderr: Arc<Mutex<VecDeque<String>>>,
}

impl FramedStreamOwner {
    #[expect(
        clippy::significant_drop_tightening,
        reason = "The playback guard serializes generation checks with detaching that generation's children and pending audio."
    )]
    pub(super) fn finish(
        &self,
        generation: u64,
        playback: &Mutex<PreviewPlaybackClock>,
        changed: &Condvar,
        stop: &Arc<AtomicBool>,
        error: Option<String>,
    ) {
        let (child, stderr_worker, audio) = {
            let mut clock = lock_playback(playback);
            if !clock.generation_matches(generation) || stop.load(Ordering::Acquire) {
                return;
            }
            let mut process = lock_process(&self.process);
            if !process
                .stop_requested
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, stop))
            {
                return;
            }
            if error.is_some() {
                clock.fail_generation(generation);
            }
            *self
                .pending_audio
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
            let audio = lock_process(&self.audio).take_handles();
            // Leave our own stdout-worker handle for the owner to join later.
            (process.child.take(), process.stderr_worker.take(), audio)
        };
        changed.notify_all();
        let outcome = finish_child(child, stop, error.is_some());
        if let Some(worker) = stderr_worker {
            let _ = worker.join();
        }
        let _ = stop_process_handles(audio);
        if stop.load(Ordering::Acquire) {
            return;
        }
        if let Some(error) = error.or_else(|| outcome.err()) {
            lock_playback(playback).fail_generation(generation);
            changed.notify_all();
            push_stderr_line(&self.stderr, format!("framed preview stopped: {error}"));
        } else {
            mark_playback_ended(playback, changed, generation);
        }
    }
}

fn finish_child(
    child: Option<std::process::Child>,
    stop: &AtomicBool,
    failed: bool,
) -> Result<(), String> {
    let Some(mut child) = child else {
        return Err("preview child ownership was lost".into());
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!("FFmpeg exited with {status}"))
                };
            }
            Ok(None) if !failed && !stop.load(Ordering::Acquire) && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(2));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("FFmpeg did not complete the framed stream".into());
            }
        }
    }
}

/// The temporary reader returns the same pipe to the streaming worker. There
/// is no second decode or restart that could choose a different first frame.
pub(super) fn read_first_frame(
    stream: &mut SpawnedVideoStream,
    seconds: f64,
    image_identity: RenderImageIdentity,
    cancelled: &AtomicBool,
) -> Result<DecodedPreviewFrame, PreviewEngineError> {
    if !stream.spec.framed {
        return read_stream_frame(&mut stream.stdout, &stream.spec, seconds, image_identity);
    }
    let mut stdout = std::mem::replace(&mut stream.stdout, Box::new(std::io::empty()));
    let spec = stream.spec;
    let (sender, receiver) = mpsc::channel();
    let reader = thread::Builder::new()
        .name("frame-preview-first-frame".into())
        .spawn(move || {
            let frame = read_stream_frame(&mut stdout, &spec, seconds, image_identity);
            let _ = sender.send((stdout, frame));
        })
        .map_err(PreviewEngineError::Io)?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
            let _ = stream.child.kill();
            let _ = stream.child.wait();
            let _ = reader.join();
            return Err(PreviewEngineError::Ffmpeg(
                "first preview frame cancelled or timed out".into(),
            ));
        }
        match receiver.recv_timeout(Duration::from_millis(5)) {
            Ok((stdout, result)) => {
                stream.stdout = stdout;
                let _ = reader.join();
                if cancelled.load(Ordering::Acquire) {
                    return Err(PreviewEngineError::Ffmpeg(
                        "preview request cancelled".into(),
                    ));
                }
                return result;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = stream.child.kill();
                let _ = stream.child.wait();
                let _ = reader.join();
                return Err(PreviewEngineError::Ffmpeg(
                    "first preview frame reader stopped".into(),
                ));
            }
        }
    }
}
