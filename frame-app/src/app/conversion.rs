use super::*;

impl FrameRoot {
    pub(super) fn queue_selected_conversion_tasks(
        &mut self,
    ) -> Vec<frame_core::types::ConversionTask> {
        if self.update_installation_in_progress() {
            return Vec::new();
        }
        let Some(output_directory) = self
            .default_output_directory
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
        else {
            return Vec::new();
        };
        self.normalize_selected_actionable_conversion_configs();

        let mut tasks = self
            .file_queue
            .queue_selected_pending_conversions()
            .iter()
            .map(|file| conversion_task_from_file(file, &output_directory))
            .collect::<Vec<_>>();
        disambiguate_output_paths(&mut tasks);

        for task in &tasks {
            let Some(output_name) = task.output_name.as_ref() else {
                continue;
            };
            let Some(file) = self
                .file_queue
                .files_mut()
                .iter_mut()
                .find(|file| file.id == task.id)
            else {
                continue;
            };
            if file.output_name != *output_name {
                file.output_name.clone_from(output_name);
            }
        }

        tasks
    }
    pub(super) fn start_selected_conversions(&mut self, cx: &mut Context<Self>) {
        if self.is_processing || self.update_installation_in_progress() {
            return;
        }

        let tasks = self.queue_selected_conversion_tasks();
        if tasks.is_empty() {
            self.active_conversion_task_ids.clear();
            return;
        }

        self.active_conversion_task_ids = tasks.iter().map(|task| task.id.clone()).collect();
        self.is_processing = true;
        self.spawn_conversion_batch(tasks, cx);
        cx.notify();
    }
    pub(super) fn spawn_conversion_batch(
        &self,
        tasks: Vec<frame_core::types::ConversionTask>,
        cx: &Context<Self>,
    ) {
        let (tx, rx) = mpsc::channel();
        let controller = self.conversion_processes.clone();

        cx.background_spawn(async move {
            let result = run_conversion_batch_with_control(tasks, &controller, |event| {
                let _ = tx.send(event);
            });
            if let Err(error) = result {
                eprintln!("Conversion batch failed: {error}");
            }
        })
        .detach();

        cx.spawn(async move |this, cx| {
            loop {
                let mut is_disconnected = false;
                loop {
                    match rx.try_recv() {
                        Ok(event) => {
                            if this
                                .update(cx, |root, cx| {
                                    root.apply_conversion_event(event);
                                    cx.notify();
                                })
                                .is_err()
                            {
                                return;
                            }
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            is_disconnected = true;
                            break;
                        }
                    }
                }

                if is_disconnected {
                    this.update(cx, |root, cx| {
                        root.refresh_processing_state_from_queue();
                        cx.notify();
                    })
                    .ok();
                    return;
                }

                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        })
        .detach();
    }
    pub(super) fn pause_conversion_task(&mut self, id: &str) -> bool {
        if !self
            .file_queue
            .file_by_id(id)
            .is_some_and(|file| file.status == FileStatus::Converting)
        {
            return false;
        }

        match self.conversion_processes.pause_task(id) {
            Ok(()) => self.file_queue.pause_file(id),
            Err(error) => {
                self.log_conversion_control_error(id, "pause", &error);
                false
            }
        }
    }
    pub(super) fn resume_conversion_task(&mut self, id: &str) -> bool {
        if !self
            .file_queue
            .file_by_id(id)
            .is_some_and(|file| file.status == FileStatus::Paused)
        {
            return false;
        }

        match self.conversion_processes.resume_task(id) {
            Ok(()) => self.file_queue.resume_file(id),
            Err(error) => {
                self.log_conversion_control_error(id, "resume", &error);
                false
            }
        }
    }
    pub(super) fn cancel_conversion_task(&mut self, id: &str) -> bool {
        if !self
            .file_queue
            .file_by_id(id)
            .is_some_and(|file| file.status.can_be_cancelled())
        {
            return false;
        }

        match self.conversion_processes.cancel_task(id) {
            Ok(()) => self.file_queue.mark_file_cancelling(id),
            Err(error) => {
                self.log_conversion_control_error(id, "cancel", &error);
                false
            }
        }
    }
    pub(super) fn prepare_file_for_reconversion(&mut self, id: &str) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        self.file_queue.prepare_file_for_reconversion(id)
    }
    pub(super) fn remove_file_from_queue(&mut self, id: &str) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        let Some(status) = self.file_queue.file_by_id(id).map(|file| file.status) else {
            return false;
        };

        if status.can_be_cancelled()
            && let Err(error) = self.conversion_processes.cancel_task(id)
        {
            self.log_conversion_control_error(id, "cancel", &error);
            return false;
        }

        let removed = self.file_queue.remove_file(id).is_some();
        if removed {
            self.source_metadata.remove(id);
            self.conversion_events.remove_logs(id);
            self.refresh_processing_state_from_queue();
        }

        removed
    }
    pub(super) fn log_conversion_control_error(
        &mut self,
        id: &str,
        action: &str,
        error: &frame_core::error::ConversionError,
    ) {
        self.conversion_events.apply_conversion_event(
            &mut self.file_queue,
            ConversionEvent::log(
                id.to_string(),
                format!("[ERROR] Failed to {action}: {error}"),
            ),
        );
    }
    pub(super) fn apply_conversion_event(&mut self, event: ConversionEvent) {
        self.conversion_events
            .apply_conversion_event(&mut self.file_queue, event);
        self.refresh_processing_state_from_queue();
    }

    fn refresh_processing_state_from_queue(&mut self) {
        let was_processing = self.is_processing;
        self.is_processing = !all_conversions_settled(&self.file_queue);

        if was_processing && !self.is_processing {
            self.notify_active_conversion_batch_finished();
        }
    }

    fn notify_active_conversion_batch_finished(&mut self) {
        let summary = conversion_finished_notification_for_task_ids(
            &self.file_queue,
            &self.active_conversion_task_ids,
        );
        self.active_conversion_task_ids.clear();

        if let Some(summary) = summary {
            self.notifier.notify_conversion_finished(summary);
        }
    }

    fn normalize_selected_actionable_conversion_configs(&mut self) {
        let metadata_by_file = self
            .file_queue
            .files()
            .iter()
            .filter(|file| {
                file.is_selected_for_conversion && file.status.is_actionable_for_conversion()
            })
            .map(|file| {
                (
                    file.id.clone(),
                    conversion_metadata_for_file(
                        file,
                        self.source_metadata.metadata_for(&file.id).cloned(),
                    ),
                )
            })
            .collect::<Vec<_>>();

        for file in self.file_queue.files_mut() {
            if !file.is_selected_for_conversion || !file.status.is_actionable_for_conversion() {
                continue;
            }

            let metadata = metadata_by_file
                .iter()
                .find(|(id, _)| id == &file.id)
                .and_then(|(_, metadata)| metadata.as_ref());
            normalize_output_config(&mut file.config, metadata);
        }
    }
}

fn conversion_metadata_for_file(
    file: &FileItem,
    metadata: Option<SourceMetadata>,
) -> Option<SourceMetadata> {
    metadata.or_else(|| {
        source_kind_from_file_extension(file).map(|media_kind| SourceMetadata {
            media_kind: Some(media_kind),
            ..SourceMetadata::default()
        })
    })
}

fn source_kind_from_file_extension(file: &FileItem) -> Option<SourceKind> {
    if extension_matches(&file.original_format, IMAGE_FILE_EXTENSIONS) {
        Some(SourceKind::Image)
    } else if extension_matches(&file.original_format, AUDIO_FILE_EXTENSIONS) {
        Some(SourceKind::Audio)
    } else {
        None
    }
}

fn extension_matches(extension: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
}
