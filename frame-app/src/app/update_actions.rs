use super::*;

impl FrameRoot {
    pub(super) fn open_update_dialog(&mut self) -> bool {
        let changed = !self.update_ui.dialog_open || !self.update_ui.dialog_present;
        self.update_ui.dialog_open = true;
        self.update_ui.dialog_present = true;
        changed
    }

    pub(super) fn close_update_dialog(&mut self) -> bool {
        let changed = self.update_ui.dialog_open;
        self.update_ui.dialog_open = false;
        changed
    }

    pub(super) fn finish_update_dialog_close(&mut self) -> bool {
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
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn download_available_update(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn install_downloaded_update(&mut self, cx: &mut Context<Self>) {
        if self.update_ui.status.is_busy() {
            return;
        }
        let UpdateStatus::ReadyToInstall(package) = &self.update_ui.status else {
            return;
        };
        let package = (**package).clone();
        self.update_ui.status = UpdateStatus::Installing;
        self.open_update_dialog();
        let channel = self.update_channel;

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let client = build_update_client(channel)?;
                    let plan_path = client.prepare_install(&package)?;
                    client.spawn_helper(&plan_path)
                })
                .await;

            this.update(cx, |root, cx| match result {
                Ok(()) => cx.quit(),
                Err(error) => {
                    root.update_ui.status = UpdateStatus::Error(error.to_string());
                    root.open_update_dialog();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn toggle_auto_update_check(&mut self) -> bool {
        self.auto_update_check = !self.auto_update_check;
        if let Err(error) = self.persist_app_settings() {
            self.update_ui.status = UpdateStatus::Error(error.to_string());
            return false;
        }
        true
    }

    pub(super) fn skip_available_update(&mut self) -> bool {
        let UpdateStatus::Available(info) = &self.update_ui.status else {
            return false;
        };
        self.skipped_update_version = Some(info.version.to_string());
        self.update_ui.status = UpdateStatus::Idle;
        self.update_ui.dialog_info = None;
        self.close_update_dialog();
        if let Err(error) = self.persist_app_settings() {
            self.update_ui.status = UpdateStatus::Error(error.to_string());
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
}
