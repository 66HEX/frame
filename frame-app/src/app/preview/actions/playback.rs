use crate::app::preview::panel::{
    preview_duration_seconds, preview_playback_state, preview_source_media_kind,
};
use crate::app::preview::{timeline_keyboard_time_for_key, timeline_slider_percent_from_bounds};
use crate::app::root::FrameRoot;
use crate::preview::{
    MediaSnapshot, MetadataStatus as PreviewMetadataStatus, PlaybackMediaCommand,
    PreviewControlInput, PreviewMediaKind, PreviewPlaybackState, TimelineDragTarget,
    preview_control_availability,
};
use crate::preview_engine::{PreviewCommand, PreviewEngineError};
use crate::settings::{ConversionConfig, SourceMetadata, apply_trim_times};
use gpui::prelude::*;
use gpui::{Bounds, Context, Pixels, Point};

impl FrameRoot {
    pub(in crate::app) fn sync_preview_playback_for_selection(
        &mut self,
        selected_file_id: Option<&str>,
        metadata: Option<&SourceMetadata>,
        config: &ConversionConfig,
        cx: &Context<Self>,
    ) {
        let media_kind = preview_control_availability(PreviewControlInput {
            metadata_status: if metadata.is_some() {
                PreviewMetadataStatus::Ready
            } else {
                PreviewMetadataStatus::Idle
            },
            source_media_kind: metadata.map(preview_source_media_kind),
            controls_disabled: self.file_queue.selected_file_locked(),
            processing_mode: config.processing_mode,
            container: Some(config.container.as_str()),
        })
        .media_kind;
        let duration_seconds = preview_duration_seconds(metadata);

        if self.preview_ui.playback_file_id.as_deref() != selected_file_id {
            self.preview_ui.trim_preview_seek.reset();
            self.preview_ui.playback_file_id = selected_file_id.map(str::to_string);
            self.preview_ui.playback = preview_playback_state(
                media_kind,
                duration_seconds,
                config.start_time.as_deref(),
                config.end_time.as_deref(),
            );
            return;
        }

        self.preview_ui
            .playback
            .set_is_image(media_kind == PreviewMediaKind::Image);
        self.preview_ui
            .playback
            .sync_initial_values(config.start_time.as_deref(), config.end_time.as_deref());

        if let Some(session) = &self.preview_ui.session {
            let snapshot = session.snapshot();
            if !self.preview_media_snapshot_sync_blocked() {
                self.preview_ui.playback.sync_media(MediaSnapshot {
                    current_time: snapshot.playback.position_seconds,
                    duration: snapshot.playback.duration_seconds,
                    paused: !snapshot.playback.playing,
                });
            }
        } else if media_kind == PreviewMediaKind::Unknown {
            self.preview_ui.playback.clear_media();
        }

        let command = self
            .preview_ui
            .playback
            .handle_time_update(self.preview_ui.playback.current_time());
        self.apply_preview_media_command(command, true, Some(cx));
    }

    pub(in crate::app) fn preview_playback_state(&self) -> PreviewPlaybackState {
        self.preview_ui.playback.clone()
    }

    pub(in crate::app) const fn preview_media_snapshot_sync_blocked(&self) -> bool {
        self.preview_ui.playback.dragging().is_some()
            || self.preview_ui.trim_preview_seek.is_active()
    }

    pub(in crate::app) fn apply_preview_timeline_drag(
        &mut self,
        target: TimelineDragTarget,
        percent: f64,
    ) -> bool {
        self.apply_preview_timeline_drag_internal(target, percent, None)
    }

    pub(in crate::app) fn apply_preview_timeline_drag_with_context(
        &mut self,
        target: TimelineDragTarget,
        percent: f64,
        cx: &Context<Self>,
    ) -> bool {
        self.apply_preview_timeline_drag_internal(target, percent, Some(cx))
    }

    pub(super) fn apply_preview_timeline_drag_internal(
        &mut self,
        target: TimelineDragTarget,
        percent: f64,
        cx: Option<&Context<Self>>,
    ) -> bool {
        if !self.preview_timeline_enabled() {
            return false;
        }

        if self.preview_ui.playback.dragging().is_none() {
            if target == TimelineDragTarget::Scrub {
                self.cancel_trim_preview_seek();
                let command = self.preview_ui.playback.seek_to_percent(percent);
                if command.pause && self.trim_preview_seek_available() {
                    self.preview_ui.trim_preview_seek.pause_before_next_seek();
                }
                self.apply_preview_command_to_local_state(command);
                if let Some(preview_seek_to) = command.seek_to {
                    self.queue_trim_preview_seek(preview_seek_to, cx);
                }
                return true;
            }

            if !self.preview_ui.playback.begin_handle_drag(target) {
                return false;
            }
            let was_playing = self.preview_ui.playback.is_playing();
            if self.trim_preview_seek_available() {
                self.preview_ui
                    .trim_preview_seek
                    .begin_drag(self.preview_ui.playback.current_time());
                if was_playing {
                    self.preview_ui.trim_preview_seek.pause_before_next_seek();
                }
            }
            if was_playing {
                self.apply_preview_media_command(PlaybackMediaCommand::pause(), true, cx);
            }
        }

        let update = self.preview_ui.playback.drag_to_percent(percent);
        self.apply_preview_command_to_local_state(update.command);
        if let Some(preview_seek_to) = update.preview_seek_to {
            self.queue_trim_preview_seek(preview_seek_to, cx);
        }
        true
    }

    pub(in crate::app) const fn set_preview_timeline_track_bounds(
        &mut self,
        bounds: Bounds<Pixels>,
    ) {
        self.preview_ui.timeline_track_bounds = Some(bounds);
    }

    #[cfg(test)]
    pub(in crate::app) fn commit_preview_timeline_seek_at_position(
        &mut self,
        position: Point<Pixels>,
    ) -> bool {
        self.commit_preview_timeline_seek_at_position_internal(position, None)
    }

    pub(in crate::app) fn commit_preview_timeline_seek_at_position_with_context(
        &mut self,
        position: Point<Pixels>,
        cx: &Context<Self>,
    ) -> bool {
        self.commit_preview_timeline_seek_at_position_internal(position, Some(cx))
    }

    pub(super) fn commit_preview_timeline_seek_at_position_internal(
        &mut self,
        position: Point<Pixels>,
        cx: Option<&Context<Self>>,
    ) -> bool {
        let Some(bounds) = self.preview_ui.timeline_track_bounds else {
            return false;
        };
        if !self.preview_timeline_enabled() {
            return false;
        }

        let percent = timeline_slider_percent_from_bounds(position, bounds);
        self.cancel_trim_preview_seek();
        let command = self.preview_ui.playback.seek_once_to_percent(percent);
        self.apply_preview_media_command(command, true, cx)
    }

    #[cfg(test)]
    pub(in crate::app) fn end_preview_timeline_drag(&mut self) -> bool {
        self.end_preview_timeline_drag_internal(None)
    }

    pub(in crate::app) fn end_preview_timeline_drag_with_context(
        &mut self,
        cx: &Context<Self>,
    ) -> bool {
        self.end_preview_timeline_drag_internal(Some(cx))
    }

    pub(super) fn end_preview_timeline_drag_internal(
        &mut self,
        cx: Option<&Context<Self>>,
    ) -> bool {
        if self.preview_ui.playback.dragging().is_none() {
            return false;
        }

        let was_trim_drag = self
            .preview_ui
            .playback
            .dragging()
            .is_some_and(|target| target != TimelineDragTarget::Scrub);
        let end = self.preview_ui.playback.end_drag();
        let mut changed = self.apply_preview_media_command(end.command, true, cx);
        if let Some(trim) = end.trim {
            changed |= self.update_selected_config(|config| {
                apply_trim_times(config, trim.start_time, trim.end_time)
            });
        }
        if was_trim_drag {
            changed |= self.finish_trim_preview_seek(cx);
        }
        changed
    }

    pub(in crate::app) fn adjust_preview_timeline_from_keyboard_with_context(
        &mut self,
        target: TimelineDragTarget,
        key: &str,
        cx: &Context<Self>,
    ) -> bool {
        self.adjust_preview_timeline_from_keyboard_internal(target, key, Some(cx))
    }

    pub(super) fn adjust_preview_timeline_from_keyboard_internal(
        &mut self,
        target: TimelineDragTarget,
        key: &str,
        cx: Option<&Context<Self>>,
    ) -> bool {
        if !self.preview_timeline_enabled() {
            return false;
        }

        let duration = self.preview_ui.playback.duration();
        let current_time = match target {
            TimelineDragTarget::Start => self.preview_ui.playback.start_value(),
            TimelineDragTarget::End => self.preview_ui.playback.end_value(),
            TimelineDragTarget::Scrub => self.preview_ui.playback.current_time(),
        };
        let Some(next_time) = timeline_keyboard_time_for_key(current_time, duration, key) else {
            return false;
        };
        let percent = next_time / duration;

        if target == TimelineDragTarget::Scrub {
            self.cancel_trim_preview_seek();
            let command = self.preview_ui.playback.seek_once_to_percent(percent);
            return self.apply_preview_media_command(command, true, cx);
        }

        self.apply_preview_timeline_drag(target, percent)
            | self.end_preview_timeline_drag_internal(cx)
    }

    pub(in crate::app) fn toggle_preview_playback_with_context(
        &mut self,
        cx: &Context<Self>,
    ) -> bool {
        self.toggle_preview_playback_internal(Some(cx))
    }

    pub(super) fn toggle_preview_playback_internal(&mut self, cx: Option<&Context<Self>>) -> bool {
        if !self.preview_timeline_enabled() {
            return false;
        }

        let command = self.preview_ui.playback.toggle_play();
        self.apply_preview_media_command(command, true, cx)
    }

    pub(in crate::app) fn apply_preview_media_command(
        &mut self,
        command: PlaybackMediaCommand,
        precise_seek: bool,
        cx: Option<&Context<Self>>,
    ) -> bool {
        let Some(session) = self.preview_ui.session.clone() else {
            return self.apply_preview_command_to_local_state(command);
        };

        let commands = preview_commands_from_playback(command, precise_seek);
        if !commands.is_empty() {
            if let Some(cx) = cx {
                self.preview_ui.media_command_generation =
                    self.preview_ui.media_command_generation.saturating_add(1);
                let command_generation = self.preview_ui.media_command_generation;
                cx.spawn(async move |this, cx| {
                    let result = cx
                        .background_spawn(async move {
                            for command in commands {
                                session.command(command)?;
                            }
                            Ok::<(), PreviewEngineError>(())
                        })
                        .await;

                    this.update(cx, move |root, cx| {
                        if root.preview_ui.media_command_generation != command_generation {
                            return;
                        }
                        match result {
                            Ok(()) => {
                                root.preview_ui.runtime_error = None;
                                root.schedule_preview_frame_tick(cx);
                            }
                            Err(error) => {
                                root.preview_ui.runtime_error = Some(error.to_string());
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            } else {
                for command in commands {
                    if let Err(error) = session.command(command) {
                        self.preview_ui.runtime_error = Some(error.to_string());
                        return false;
                    }
                }
                self.preview_ui.runtime_error = None;
            }
        }

        self.apply_preview_command_to_local_state(command)
    }

    pub(super) const fn apply_preview_command_to_local_state(
        &mut self,
        command: PlaybackMediaCommand,
    ) -> bool {
        if command.pause {
            self.preview_ui.playback.handle_pause();
        }
        if command.play {
            self.preview_ui.playback.handle_play();
        }
        command.pause || command.play || command.seek_to.is_some()
    }

    pub(super) fn preview_timeline_enabled(&self) -> bool {
        let metadata = self.selected_source_metadata();
        let Some(config) = self.selected_config() else {
            return false;
        };
        let availability = preview_control_availability(PreviewControlInput {
            metadata_status: if metadata.is_some() {
                PreviewMetadataStatus::Ready
            } else {
                PreviewMetadataStatus::Idle
            },
            source_media_kind: metadata.as_ref().map(preview_source_media_kind),
            controls_disabled: self.file_queue.selected_file_locked(),
            processing_mode: config.processing_mode,
            container: Some(config.container.as_str()),
        });

        !availability.trim_disabled && self.preview_ui.playback.duration() > 0.0
    }
}

pub(super) fn preview_commands_from_playback(
    command: PlaybackMediaCommand,
    precise_seek: bool,
) -> Vec<PreviewCommand> {
    let mut commands = Vec::with_capacity(3);
    if command.pause {
        commands.push(PreviewCommand::Pause);
    }
    if let Some(seconds) = command.seek_to {
        commands.push(if precise_seek {
            PreviewCommand::SeekPrecise(seconds)
        } else {
            PreviewCommand::SeekFast(seconds)
        });
    }
    if command.play {
        commands.push(PreviewCommand::Play);
    }
    commands
}
