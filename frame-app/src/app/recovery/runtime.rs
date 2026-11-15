use super::{Context, FrameRoot, RecoveryError, RecoveryStore};
use gpui::AppContext;
use std::time::Duration;

impl FrameRoot {
    pub(in crate::app) fn initialize_workspace(&mut self, cx: &mut Context<Self>) {
        use crate::app::updates::session::UpdateSessionRestore;
        let update = self.restore_pending_update_session(cx);
        if crate::runtime_environment::is_flatpak() {
            return;
        }
        match update {
            UpdateSessionRestore::Absent => self.start_workspace_recovery(false, cx),
            UpdateSessionRestore::Restored => self.start_workspace_recovery(true, cx),
            UpdateSessionRestore::Blocked => {
                self.fail_workspace_recovery(&RecoveryError::Invalid(
                    "the pending update queue could not be restored or cleared",
                ));
                cx.notify();
            }
        }
    }

    pub(in crate::app) fn start_workspace_recovery(
        &mut self,
        updater_restored: bool,
        cx: &mut Context<Self>,
    ) {
        if crate::runtime_environment::is_flatpak() {
            return;
        }
        let Some(persistence) = &self.persistence else {
            return;
        };
        let settings_path = persistence.settings_path().to_path_buf();
        self.workspace_recovery.loading = true;
        cx.observe_self(Self::schedule_workspace_save).detach();
        // GPUI releases the root when its last window closes. The quit hook
        // covers menu/OS quit while the window is still alive. Both serialize
        // with the background writer and capture the latest intent immediately.
        cx.on_release(|root, _cx| root.flush_workspace_recovery())
            .detach();
        cx.on_app_quit(|root, _cx| {
            root.flush_workspace_recovery();
            std::future::ready(())
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut store = RecoveryStore::open(&settings_path)?;
                    let snapshot = store.load()?;
                    Ok::<_, RecoveryError>((store, snapshot))
                })
                .await;
            let _ = this.update(cx, |root, cx| {
                let result = result.and_then(|(store, snapshot)| {
                    root.accept_recovery_store(store, snapshot, updater_restored)
                });
                if let Err(error) = result {
                    root.fail_workspace_recovery(&error);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn schedule_workspace_save(&mut self, cx: &mut Context<Self>) {
        if !self.workspace_recovery.ready {
            return;
        }
        self.workspace_recovery.change_epoch = self.workspace_recovery.change_epoch.wrapping_add(1);
        if self.workspace_recovery.scheduled {
            return;
        }
        self.workspace_recovery.scheduled = true;
        cx.spawn(async move |this, cx| {
            // Coalesce frequent slider/progress notifications without delaying
            // a continuously edited queue indefinitely.
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            let Ok((epoch, write)) = this.update(cx, |root, _cx| {
                (
                    root.workspace_recovery.change_epoch,
                    root.prepare_recovery_write(),
                )
            }) else {
                return;
            };
            let result = match write {
                Ok(Some(write)) => {
                    cx.background_spawn(async move { write.save().map(|()| true) })
                        .await
                }
                Ok(None) => Ok(false),
                Err(error) => Err(error),
            };
            let _ = this.update(cx, |root, cx| {
                root.workspace_recovery.scheduled = false;
                let result = result.and_then(|published| {
                    if published {
                        root.finish_recovered_update()
                    } else {
                        Ok(())
                    }
                });
                if let Err(error) = result {
                    root.fail_workspace_recovery(&error);
                    cx.notify();
                } else if epoch != root.workspace_recovery.change_epoch {
                    root.schedule_workspace_save(cx);
                }
            });
        })
        .detach();
    }

    pub(super) fn flush_workspace_recovery(&mut self) {
        if let Err(error) = self.capture_recovery_write(true).and_then(|write| {
            if let Some(write) = write {
                write.save()?;
                self.finish_recovered_update()?;
            }
            Ok(())
        }) {
            eprintln!("Failed to save workspace before closing: {error}");
            self.fail_workspace_recovery(&error);
        }
    }
}
