use super::playback::preview_commands_from_playback;
use crate::app::preview::state::TrimPreviewSeekRequest;
use crate::app::root::FrameRoot;
use crate::preview::PlaybackMediaCommand;
use crate::preview_engine::PreviewEngineError;
use gpui::Context;
use gpui::prelude::*;
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
struct QueuedMediaCommand {
    command: PlaybackMediaCommand,
    precise_seek: bool,
    generation: u64,
    origin: MediaCommandOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MediaCommandOrigin {
    Playback,
    Trim { generation: u64, seconds: f64 },
}

impl QueuedMediaCommand {
    const fn is_seek_only(self) -> bool {
        matches!(self.origin, MediaCommandOrigin::Playback)
            && self.command.seek_to.is_some()
            && !self.command.pause
            && !self.command.play
    }
}

/// A single worker preserves transport ordering while pending seeks keep only
/// the latest target. A seek never replaces a Play/Pause boundary or work that
/// has already started; in particular the final precise seek remains precise.
#[derive(Default)]
pub(in crate::app) struct PreviewMediaCommandQueue {
    pending: VecDeque<QueuedMediaCommand>,
    worker_active: bool,
    epoch: u64,
}

impl PreviewMediaCommandQueue {
    pub(in crate::app) fn reset(&mut self) {
        self.pending.clear();
        self.worker_active = false;
        self.epoch = self.epoch.wrapping_add(1);
    }

    pub(in crate::app) const fn is_active(&self) -> bool {
        self.worker_active
    }

    fn push(&mut self, request: QueuedMediaCommand) {
        if request.command == PlaybackMediaCommand::none() {
            return;
        }
        if request.is_seek_only()
            && let Some(previous) = self.pending.back_mut()
            && previous.is_seek_only()
        {
            *previous = request;
            return;
        }
        self.pending.push_back(request);
    }

    fn start_worker(&mut self) -> Option<u64> {
        if self.worker_active || self.pending.is_empty() {
            return None;
        }
        self.worker_active = true;
        Some(self.epoch)
    }

    const fn is_current_worker(&self, epoch: u64) -> bool {
        self.worker_active && self.epoch == epoch
    }

    fn take_next(&mut self, epoch: u64) -> Option<QueuedMediaCommand> {
        if !self.is_current_worker(epoch) {
            return None;
        }
        let next = self.pending.pop_front();
        if next.is_none() {
            self.worker_active = false;
        }
        next
    }

    fn take_next_current(
        &mut self,
        epoch: u64,
        trim_generation: u64,
    ) -> Option<QueuedMediaCommand> {
        while let Some(request) = self.take_next(epoch) {
            if let MediaCommandOrigin::Trim { generation, .. } = request.origin
                && generation != trim_generation
            {
                continue;
            }
            return Some(request);
        }
        None
    }
}

impl FrameRoot {
    pub(super) fn queue_preview_media_command(
        &mut self,
        command: PlaybackMediaCommand,
        precise_seek: bool,
        cx: &Context<Self>,
    ) {
        self.preview_ui.media_command_generation =
            self.preview_ui.media_command_generation.wrapping_add(1);
        self.enqueue_preview_media_command(
            QueuedMediaCommand {
                command,
                precise_seek,
                generation: self.preview_ui.media_command_generation,
                origin: MediaCommandOrigin::Playback,
            },
            cx,
        );
    }

    pub(super) fn queue_trim_media_command(
        &mut self,
        request: TrimPreviewSeekRequest,
        generation: u64,
        cx: &Context<Self>,
    ) {
        self.enqueue_preview_media_command(
            QueuedMediaCommand {
                command: PlaybackMediaCommand {
                    seek_to: Some(request.seconds),
                    pause: request.pause_first,
                    play: false,
                },
                precise_seek: true,
                generation: self.preview_ui.media_command_generation,
                origin: MediaCommandOrigin::Trim {
                    generation,
                    seconds: request.seconds,
                },
            },
            cx,
        );
    }

    fn enqueue_preview_media_command(&mut self, request: QueuedMediaCommand, cx: &Context<Self>) {
        let Some(session) = self.preview_ui.session.clone() else {
            return;
        };
        self.preview_ui.media_commands.push(request);
        let Some(epoch) = self.preview_ui.media_commands.start_worker() else {
            return;
        };

        cx.spawn(async move |this, cx| {
            loop {
                let request = this
                    .update(cx, |root, cx| {
                        if !root.preview_ui.media_commands.is_current_worker(epoch) {
                            return None;
                        }
                        let request = root
                            .preview_ui
                            .media_commands
                            .take_next_current(epoch, root.preview_ui.trim_preview_seek.generation);
                        if request.is_none() {
                            // Unblock snapshot synchronization and present the final
                            // frame even when the paused-preview ticker is asleep.
                            root.schedule_preview_frame_tick(cx);
                            cx.notify();
                        }
                        request
                    })
                    .ok()
                    .flatten();
                let Some(request) = request else {
                    break;
                };

                let result = cx
                    .background_spawn({
                        let session = Arc::clone(&session);
                        async move {
                            for command in preview_commands_from_playback(
                                request.command,
                                request.precise_seek,
                            ) {
                                session.command(command)?;
                            }
                            Ok::<(), PreviewEngineError>(())
                        }
                    })
                    .await;

                let current = this
                    .update(cx, |root, cx| {
                        if !root.preview_ui.media_commands.is_current_worker(epoch) {
                            return false;
                        }
                        match request.origin {
                            MediaCommandOrigin::Trim {
                                generation,
                                seconds,
                            } => {
                                if !root.finish_trim_preview_seek_request(
                                    generation, seconds, result, cx,
                                ) {
                                    Self::schedule_next_trim_preview_seek(generation, cx);
                                }
                            }
                            MediaCommandOrigin::Playback => {
                                if root.preview_ui.media_command_generation == request.generation {
                                    match result {
                                        Ok(()) => {
                                            if root.preview_ui.failed_runtime_key.is_none() {
                                                root.preview_ui.runtime_error = None;
                                            }
                                            root.schedule_preview_frame_tick(cx);
                                        }
                                        Err(error) => {
                                            root.preview_ui.runtime_error = Some(error.to_string());
                                        }
                                    }
                                    cx.notify();
                                }
                            }
                        }
                        true
                    })
                    .unwrap_or(false);
                if !current {
                    break;
                }
            }
            // This may be the last session reference after a source/window
            // closes. Process teardown must not run on the UI executor.
            cx.background_spawn(async move { drop(session) }).detach();
        })
        .detach();
    }
}

#[cfg(test)]
#[path = "media_commands_tests.rs"]
mod tests;
