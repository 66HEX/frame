use super::*;

impl FrameRoot {
    pub(super) fn queue_selected_conversion_tasks(
        &mut self,
    ) -> Vec<frame_core::types::ConversionTask> {
        self.file_queue
            .queue_selected_pending_conversions()
            .iter()
            .map(conversion_task_from_file)
            .collect()
    }
    pub(super) fn start_selected_conversions(&mut self, cx: &mut Context<Self>) {
        if self.is_processing {
            return;
        }

        let tasks = self.queue_selected_conversion_tasks();
        if tasks.is_empty() {
            return;
        }

        self.is_processing = true;
        self.spawn_conversion_batch(tasks, cx);
        cx.notify();
    }
    pub(super) fn spawn_conversion_batch(
        &mut self,
        tasks: Vec<frame_core::types::ConversionTask>,
        cx: &mut Context<Self>,
    ) {
        let (tx, rx) = mpsc::channel();
        let controller = self.conversion_processes.clone();

        cx.background_spawn(async move {
            let result = run_conversion_batch_with_control(tasks, controller, |event| {
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
                        root.is_processing = !all_conversions_settled(&root.file_queue);
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
    pub(super) fn remove_file_from_queue(&mut self, id: &str) -> bool {
        let Some(status) = self.file_queue.file_by_id(id).map(|file| file.status) else {
            return false;
        };

        if status.can_be_cancelled_before_removal()
            && let Err(error) = self.conversion_processes.cancel_task(id)
        {
            self.log_conversion_control_error(id, "cancel", &error);
            return false;
        }

        let removed = self.file_queue.remove_file(id).is_some();
        if removed {
            self.source_metadata.remove(id);
            self.conversion_events.remove_logs(id);
            self.is_processing = !all_conversions_settled(&self.file_queue);
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
        self.is_processing = !all_conversions_settled(&self.file_queue);
    }
}
