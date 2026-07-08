//! Native desktop notifications for app-level events.

use std::{sync::Arc, thread};

#[cfg(target_os = "macos")]
use std::{
    sync::Once,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use notify_rust::{Notification, Timeout};

use crate::{
    app_info::FRAME_APP_NAME,
    file_queue::{FileQueue, FileStatus},
};

const CONVERSION_FINISHED_TITLE: &str = "Queue Finished";
const FRAME_NOTIFICATION_ICON: &str = "frame";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConversionNotificationSummary {
    pub completed_count: usize,
    pub error_count: usize,
}

impl ConversionNotificationSummary {
    #[must_use]
    pub const fn from_counts(completed_count: usize, error_count: usize) -> Option<Self> {
        if completed_count == 0 && error_count == 0 {
            None
        } else {
            Some(Self {
                completed_count,
                error_count,
            })
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        CONVERSION_FINISHED_TITLE
    }

    #[must_use]
    pub fn body(self) -> String {
        format!(
            "Processed {} files with {} errors.",
            self.completed_count, self.error_count
        )
    }
}

#[derive(Clone)]
pub struct AppNotifier {
    conversion_finished_handler: Arc<dyn Fn(ConversionNotificationSummary) + Send + Sync + 'static>,
}

impl AppNotifier {
    #[must_use]
    pub fn disabled() -> Self {
        Self::from_conversion_finished_handler(|_| {})
    }

    #[must_use]
    pub fn system() -> Self {
        Self::from_conversion_finished_handler(send_system_conversion_finished_notification)
    }

    #[must_use]
    pub fn from_conversion_finished_handler(
        handler: impl Fn(ConversionNotificationSummary) + Send + Sync + 'static,
    ) -> Self {
        Self {
            conversion_finished_handler: Arc::new(handler),
        }
    }

    pub fn notify_conversion_finished(&self, summary: ConversionNotificationSummary) {
        (self.conversion_finished_handler)(summary);
    }
}

impl Default for AppNotifier {
    fn default() -> Self {
        Self::disabled()
    }
}

#[must_use]
pub fn conversion_finished_notification_for_task_ids(
    queue: &FileQueue,
    task_ids: &[String],
) -> Option<ConversionNotificationSummary> {
    let mut completed_count = 0;
    let mut error_count = 0;

    for file in queue
        .files()
        .iter()
        .filter(|file| task_ids.contains(&file.id))
    {
        match file.status {
            FileStatus::Completed => completed_count += 1,
            FileStatus::Error => error_count += 1,
            FileStatus::Idle
            | FileStatus::Queued
            | FileStatus::Converting
            | FileStatus::Paused
            | FileStatus::Cancelling => {}
        }
    }

    ConversionNotificationSummary::from_counts(completed_count, error_count)
}

fn send_system_conversion_finished_notification(summary: ConversionNotificationSummary) {
    if let Err(error) = thread::Builder::new()
        .name("frame-notification".to_string())
        .spawn(move || {
            if let Err(error) = show_system_conversion_finished_notification(summary) {
                eprintln!("Failed to show conversion notification: {error}");
            }
        })
    {
        eprintln!("Failed to spawn conversion notification: {error}");
    }
}

#[cfg(not(target_os = "macos"))]
fn show_system_conversion_finished_notification(
    summary: ConversionNotificationSummary,
) -> notify_rust::error::Result<()> {
    Notification::new()
        .appname(FRAME_APP_NAME)
        .summary(summary.title())
        .body(&summary.body())
        .icon(FRAME_NOTIFICATION_ICON)
        .timeout(Timeout::Default)
        .show()?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn show_system_conversion_finished_notification(
    summary: ConversionNotificationSummary,
) -> notify_rust::error::Result<()> {
    initialize_macos_notification_application();

    Notification::new()
        .appname(FRAME_APP_NAME)
        .summary(summary.title())
        .body(&summary.body())
        .icon(FRAME_NOTIFICATION_ICON)
        .timeout(Timeout::Default)
        .schedule_raw(macos_delivery_timestamp())?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn initialize_macos_notification_application() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let bundle_identifier = notify_rust::get_bundle_identifier_or_default(FRAME_APP_NAME);
        if let Err(error) = notify_rust::set_application(&bundle_identifier) {
            eprintln!("Failed to initialize macOS notifications: {error}");
        }
    });
}

#[cfg(target_os = "macos")]
fn macos_delivery_timestamp() -> f64 {
    let delivery_time = SystemTime::now()
        .checked_add(Duration::from_millis(100))
        .unwrap_or_else(SystemTime::now);

    delivery_time
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |duration| duration.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_queue::FileItem;

    fn queue_with_statuses(statuses: &[(&str, FileStatus)]) -> FileQueue {
        let mut queue = FileQueue::new();

        for (id, status) in statuses {
            queue.add_file(FileItem::from_path(*id, format!("/tmp/{id}.mp4"), 1024));
            queue.update_status(id, *status, 0);
        }

        queue
    }

    #[test]
    fn conversion_finished_notification_for_task_ids_counts_active_results_only() {
        let queue = queue_with_statuses(&[
            ("first", FileStatus::Completed),
            ("second", FileStatus::Error),
            ("third", FileStatus::Completed),
        ]);
        let task_ids = vec!["first".to_string(), "second".to_string()];

        let summary = conversion_finished_notification_for_task_ids(&queue, &task_ids);

        assert_eq!(
            summary,
            Some(ConversionNotificationSummary {
                completed_count: 1,
                error_count: 1,
            })
        );
    }

    #[test]
    fn conversion_finished_notification_for_task_ids_skips_empty_results() {
        let queue =
            queue_with_statuses(&[("first", FileStatus::Idle), ("second", FileStatus::Queued)]);
        let task_ids = vec!["first".to_string(), "second".to_string()];

        let summary = conversion_finished_notification_for_task_ids(&queue, &task_ids);

        assert_eq!(summary, None);
    }

    #[test]
    fn conversion_notification_summary_uses_legacy_copy() {
        let summary = ConversionNotificationSummary {
            completed_count: 2,
            error_count: 1,
        };

        assert_eq!(summary.title(), "Queue Finished");
        assert_eq!(summary.body(), "Processed 2 files with 1 errors.");
    }
}
