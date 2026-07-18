use super::*;
use crate::update_session::{UpdateSessionError, UpdateSessionSnapshot, UpdateSessionStore};

#[derive(Debug, thiserror::Error)]
enum UpdateInstallationError {
    #[error(transparent)]
    Updater(#[from] frame_updater::UpdateError),
    #[error(transparent)]
    Session(#[from] UpdateSessionError),
    #[error(transparent)]
    Settings(#[from] crate::app_persistence::AppPersistenceError),
    #[error(transparent)]
    Preview(#[from] crate::preview_engine::PreviewEngineError),
    #[error("{UPDATE_INSTALL_WAIT_MESSAGE}")]
    ConversionStateChanged,
}

impl FrameRoot {
    const SETTINGS_UPDATE_STATUS_DISMISS_DELAY: Duration = Duration::from_secs(4);

    pub(super) fn can_install_downloaded_update(&self) -> bool {
        matches!(self.update_ui.status, UpdateStatus::ReadyToInstall(_))
            && self.conversions_settled_for_update()
    }

    pub(super) const fn update_installation_in_progress(&self) -> bool {
        matches!(self.update_ui.status, UpdateStatus::Installing)
    }

    fn conversions_settled_for_update(&self) -> bool {
        all_conversions_settled(&self.file_queue)
            && self
                .conversion_processes
                .active_process_count()
                .is_ok_and(|count| count == 0)
    }

    fn capture_installation_session(
        &mut self,
        target_version: &str,
        cx: &Context<Self>,
    ) -> Result<(UpdateSessionStore, UpdateSessionSnapshot), UpdateInstallationError> {
        if !self.update_installation_in_progress() || !self.conversions_settled_for_update() {
            return Err(UpdateInstallationError::ConversionStateChanged);
        }

        self.commit_preview_timecode_input(FrameTextInputKind::PreviewStartTime, Some(cx));
        self.commit_preview_timecode_input(FrameTextInputKind::PreviewEndTime, Some(cx));
        self.persist_app_settings()?;
        let store = self.update_session_store()?;
        let snapshot = self.capture_update_session(target_version)?;
        Ok((store, snapshot))
    }

    pub(super) const fn open_update_dialog(&mut self) -> bool {
        let changed = !self.update_ui.dialog_open || !self.update_ui.dialog_present;
        self.update_ui.dialog_open = true;
        self.update_ui.dialog_present = true;
        changed
    }

    pub(super) const fn close_update_dialog(&mut self) -> bool {
        let changed = self.update_ui.dialog_open;
        self.update_ui.dialog_open = false;
        changed
    }

    pub(super) const fn finish_update_dialog_close(&mut self) -> bool {
        if self.update_ui.dialog_open || !self.update_ui.dialog_present {
            return false;
        }
        self.update_ui.dialog_present = false;
        true
    }

    pub(super) fn startup_update_check(&mut self, cx: &mut Context<Self>) {
        if !self.auto_update_check || !update_check_is_due(self.last_update_check_at) {
            return;
        }
        self.check_for_updates(false, cx);
    }

    pub(super) fn check_for_updates(&mut self, manual: bool, cx: &mut Context<Self>) {
        if self.update_ui.status.is_busy() {
            return;
        }

        if let Some(explanation) = updates_disabled_explanation() {
            if manual {
                self.update_ui.status = UpdateStatus::Disabled(explanation);
                self.schedule_settings_update_status_dismiss(cx);
                cx.notify();
            }
            return;
        }

        self.update_ui.status = UpdateStatus::Checking;
        let channel = self.update_channel;
        let skipped_update_version = self.skipped_update_version.clone();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let client = build_update_client(channel)?;
                    client.check()
                })
                .await;

            this.update(cx, move |root, cx| {
                root.last_update_check_at = Some(unix_timestamp());
                match result {
                    Ok(UpdateCheck::Available(info))
                        if !manual
                            && skipped_update_version
                                .as_ref()
                                .is_some_and(|version| version == &info.version.to_string()) =>
                    {
                        root.update_ui.status = UpdateStatus::Idle;
                    }
                    Ok(UpdateCheck::Available(info)) => {
                        root.update_ui.dialog_info = Some(info.clone());
                        root.update_ui.status = UpdateStatus::Available(info);
                        root.open_update_dialog();
                    }
                    Ok(UpdateCheck::UpToDate) => {
                        root.update_ui.dialog_info = None;
                        root.close_update_dialog();
                        root.update_ui.status = if manual {
                            UpdateStatus::UpToDate
                        } else {
                            UpdateStatus::Idle
                        };
                    }
                    Err(error) => {
                        root.close_update_dialog();
                        root.update_ui.status = if manual {
                            UpdateStatus::Error(error.to_string())
                        } else {
                            UpdateStatus::Idle
                        };
                    }
                }
                if let Err(error) = root.persist_app_settings() {
                    root.update_ui.status = UpdateStatus::Error(error.to_string());
                }
                if manual {
                    root.schedule_settings_update_status_dismiss(cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn download_available_update(&mut self, cx: &Context<Self>) {
        if self.update_ui.status.is_busy() {
            return;
        }
        let UpdateStatus::Available(info) = &self.update_ui.status else {
            return;
        };
        let cached_info = info.clone();
        let info = (**info).clone();
        self.update_ui.dialog_info = Some(cached_info);
        self.open_update_dialog();
        let version = info.version.to_string();
        self.update_ui.status = UpdateStatus::Downloading {
            version,
            progress_percent: None,
            received_bytes: 0,
            total_bytes: None,
        };

        let channel = self.update_channel;
        let (progress_tx, progress_rx) = mpsc::channel::<DownloadProgress>();
        let (done_tx, done_rx) = mpsc::channel();
        cx.background_spawn(async move {
            let result = build_update_client(channel).and_then(|client| {
                client.download(&info, |progress| {
                    let _ = progress_tx.send(progress);
                })
            });
            let _ = done_tx.send(result);
        })
        .detach();

        cx.spawn(async move |this, cx| {
            loop {
                while let Ok(progress) = progress_rx.try_recv() {
                    if this
                        .update(cx, move |root, cx| {
                            if let UpdateStatus::Downloading {
                                progress_percent,
                                received_bytes,
                                total_bytes,
                                ..
                            } = &mut root.update_ui.status
                            {
                                *progress_percent = progress.percent();
                                *received_bytes = progress.received_bytes;
                                *total_bytes = progress.total_bytes;
                                cx.notify();
                            }
                        })
                        .is_err()
                    {
                        return;
                    }
                }

                match done_rx.try_recv() {
                    Ok(Ok(package)) => {
                        this.update(cx, |root, cx| {
                            root.update_ui.status = UpdateStatus::ReadyToInstall(Box::new(package));
                            root.open_update_dialog();
                            cx.notify();
                        })
                        .ok();
                        return;
                    }
                    Ok(Err(error)) => {
                        this.update(cx, |root, cx| {
                            root.update_ui.status = UpdateStatus::Error(error.to_string());
                            root.open_update_dialog();
                            cx.notify();
                        })
                        .ok();
                        return;
                    }
                    Err(TryRecvError::Disconnected) => {
                        this.update(cx, |root, cx| {
                            root.update_ui.status = UpdateStatus::Error(
                                "update download worker disconnected".to_string(),
                            );
                            root.open_update_dialog();
                            cx.notify();
                        })
                        .ok();
                        return;
                    }
                    Err(TryRecvError::Empty) => {}
                }

                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        })
        .detach();
    }

    pub(super) fn install_downloaded_update(&mut self, cx: &Context<Self>) {
        if !self.can_install_downloaded_update() {
            return;
        }
        let UpdateStatus::ReadyToInstall(package) = &self.update_ui.status else {
            return;
        };
        let package = (**package).clone();
        let target_version = package.version.to_string();
        self.update_ui.status = UpdateStatus::Installing;
        self.open_update_dialog();
        let channel = self.update_channel;

        cx.spawn(async move |this, cx| {
            let prepared: Result<_, UpdateInstallationError> = cx
                .background_spawn(async move {
                    let client = build_update_client(channel)?;
                    let plan_path = client.prepare_install(&package)?;
                    Ok((client, plan_path))
                })
                .await;

            let (client, plan_path) = match prepared {
                Ok(prepared) => prepared,
                Err(error) => {
                    report_update_installation_error(&this, cx, error.to_string());
                    return;
                }
            };

            let Ok(capture_result) = this.update(cx, |root, cx| {
                root.capture_installation_session(&target_version, cx)
            }) else {
                return;
            };
            let (store, snapshot) = match capture_result {
                Ok(capture) => capture,
                Err(error) => {
                    report_update_installation_error(&this, cx, error.to_string());
                    return;
                }
            };

            let snapshot_store = store.clone();
            let save_result = cx
                .background_spawn(async move { snapshot_store.save(&snapshot) })
                .await;
            if let Err(error) = save_result {
                report_update_installation_error(&this, cx, error.to_string());
                return;
            }

            let preview_result = this
                .update(cx, |root, cx| {
                    if !root.update_installation_in_progress() {
                        return Err(UpdateInstallationError::ConversionStateChanged);
                    }
                    root.clear_preview_runtime(cx)?;
                    cx.notify();
                    Ok(())
                })
                .unwrap_or(Err(UpdateInstallationError::ConversionStateChanged));
            if let Err(error) = preview_result {
                let rollback_result = cx
                    .background_spawn(async move { store.discard_pending() })
                    .await;
                let message = rollback_update_session_message(error.to_string(), rollback_result);
                report_update_installation_error(&this, cx, message);
                return;
            }

            let rollback_store = store.clone();
            let helper_result = cx
                .background_spawn(async move { client.spawn_helper(&plan_path) })
                .await;

            match helper_result {
                Ok(()) => {
                    this.update(cx, |_root, cx| cx.quit()).ok();
                }
                Err(error) => {
                    let rollback_result = cx
                        .background_spawn(async move { rollback_store.discard_pending() })
                        .await;
                    let message =
                        rollback_update_session_message(error.to_string(), rollback_result);
                    report_update_installation_error(&this, cx, message);
                }
            }
        })
        .detach();
    }

    pub(super) fn toggle_auto_update_check(&mut self, cx: &Context<Self>) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        self.auto_update_check = !self.auto_update_check;
        if let Err(error) = self.persist_app_settings() {
            self.update_ui.status = UpdateStatus::Error(error.to_string());
            self.schedule_settings_update_status_dismiss(cx);
            return false;
        }
        true
    }

    pub(super) fn skip_available_update(&mut self, cx: &Context<Self>) -> bool {
        let UpdateStatus::Available(info) = &self.update_ui.status else {
            return false;
        };
        self.skipped_update_version = Some(info.version.to_string());
        self.update_ui.status = UpdateStatus::Idle;
        self.update_ui.dialog_info = None;
        self.close_update_dialog();
        if let Err(error) = self.persist_app_settings() {
            self.update_ui.status = UpdateStatus::Error(error.to_string());
            self.schedule_settings_update_status_dismiss(cx);
            return false;
        }
        true
    }

    pub(super) fn dismiss_update_status(&mut self) {
        if !self.update_ui.status.is_busy() {
            self.update_ui.status = UpdateStatus::Idle;
            self.update_ui.dialog_info = None;
            self.close_update_dialog();
        }
    }

    fn schedule_settings_update_status_dismiss(&mut self, cx: &Context<Self>) {
        self.update_ui.status_dismiss_epoch = self.update_ui.status_dismiss_epoch.wrapping_add(1);
        let dismiss_epoch = self.update_ui.status_dismiss_epoch;

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Self::SETTINGS_UPDATE_STATUS_DISMISS_DELAY)
                .await;

            this.update(cx, |root, cx| {
                if root.update_ui.status_dismiss_epoch == dismiss_epoch
                    && matches!(
                        root.update_ui.status,
                        UpdateStatus::UpToDate | UpdateStatus::Disabled(_) | UpdateStatus::Error(_)
                    )
                {
                    root.dismiss_update_status();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}

fn report_update_installation_error(
    root: &gpui::WeakEntity<FrameRoot>,
    cx: &mut gpui::AsyncApp,
    message: String,
) {
    root.update(cx, |root, cx| {
        root.update_ui.status = UpdateStatus::Error(message);
        root.open_update_dialog();
        cx.notify();
    })
    .ok();
}

fn rollback_update_session_message(
    message: String,
    rollback_result: Result<(), UpdateSessionError>,
) -> String {
    match rollback_result {
        Ok(()) => message,
        Err(rollback_error) => {
            format!("{message}; failed to discard the pending update session: {rollback_error}")
        }
    }
}
