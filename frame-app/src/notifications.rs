//! Native desktop notifications for app-level events.

use std::{sync::Arc, thread};

#[cfg(target_os = "macos")]
use std::{
    sync::Once,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use notify_rust::{Notification, Timeout};

#[cfg(target_os = "linux")]
use ashpd::{
    Error as PortalError,
    desktop::{
        Icon,
        notification::{
            Notification as PortalNotification, NotificationProxy, Priority as PortalPriority,
        },
    },
};

use crate::{
    app_info::FRAME_APP_NAME,
    file_queue::{FileQueue, FileStatus},
};

#[cfg(target_os = "linux")]
use crate::runtime_environment;

const CONVERSION_FINISHED_TITLE: &str = "Queue Finished";
const FRAME_NOTIFICATION_ICON: &str = "frame";
#[cfg(any(target_os = "linux", test))]
const CONVERSION_FINISHED_NOTIFICATION_ID: &str = "conversion-finished";

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
        let processed_count = self.completed_count + self.error_count;
        let file_suffix = if processed_count == 1 { "" } else { "s" };
        let error_suffix = if self.error_count == 1 { "" } else { "s" };

        format!(
            "Processed {} file{file_suffix} with {} error{error_suffix}.",
            processed_count, self.error_count,
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
        .spawn(move || deliver_system_conversion_finished_notification(summary))
    {
        eprintln!("Failed to spawn conversion notification: {error}");
    }
}

#[cfg(target_os = "linux")]
fn deliver_system_conversion_finished_notification(summary: ConversionNotificationSummary) {
    let runtime = if runtime_environment::is_flatpak() {
        LinuxRuntime::Flatpak
    } else {
        LinuxRuntime::Host
    };

    match deliver_linux_notification(
        runtime,
        || show_portal_conversion_finished_notification(summary),
        || show_direct_conversion_finished_notification(summary),
    ) {
        LinuxDeliveryOutcome::Portal => {}
        LinuxDeliveryOutcome::FreedesktopFallback { portal_error } => {
            eprintln!(
                "Desktop portal notification failed: {portal_error}; delivered through org.freedesktop.Notifications fallback"
            );
        }
        LinuxDeliveryOutcome::PortalFailedInFlatpak { portal_error } => {
            eprintln!(
                "Failed to show conversion notification through the desktop portal: {portal_error}; runtime=flatpak; direct fallback disabled"
            );
        }
        LinuxDeliveryOutcome::BothFailed {
            portal_error,
            fallback_error,
        } => {
            eprintln!(
                "Failed to show conversion notification: portal error: {portal_error}; fallback error: {fallback_error}"
            );
        }
    }
}

#[cfg(target_os = "linux")]
fn show_portal_conversion_finished_notification(
    summary: ConversionNotificationSummary,
) -> Result<(), PortalError> {
    let body = summary.body();
    let flatpak_id = std::env::var("FLATPAK_ID").ok();
    let icon_names = portal_icon_names(flatpak_id.as_deref());

    smol::block_on(async move {
        let proxy = NotificationProxy::new().await?;
        let notification = PortalNotification::new(summary.title())
            .body(body.as_str())
            .priority(PortalPriority::Normal)
            .icon(Icon::with_names(icon_names));

        proxy
            .add_notification(CONVERSION_FINISHED_NOTIFICATION_ID, notification)
            .await
    })
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinuxRuntime {
    Flatpak,
    Host,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Eq, PartialEq)]
enum LinuxDeliveryOutcome<PortalFailure, FallbackFailure> {
    Portal,
    FreedesktopFallback {
        portal_error: PortalFailure,
    },
    PortalFailedInFlatpak {
        portal_error: PortalFailure,
    },
    BothFailed {
        portal_error: PortalFailure,
        fallback_error: FallbackFailure,
    },
}

#[cfg(any(target_os = "linux", test))]
fn deliver_linux_notification<PortalFailure, FallbackFailure>(
    runtime: LinuxRuntime,
    portal: impl FnOnce() -> Result<(), PortalFailure>,
    fallback: impl FnOnce() -> Result<(), FallbackFailure>,
) -> LinuxDeliveryOutcome<PortalFailure, FallbackFailure> {
    let portal_error = match portal() {
        Ok(()) => return LinuxDeliveryOutcome::Portal,
        Err(error) => error,
    };

    if runtime == LinuxRuntime::Flatpak {
        return LinuxDeliveryOutcome::PortalFailedInFlatpak { portal_error };
    }

    match fallback() {
        Ok(()) => LinuxDeliveryOutcome::FreedesktopFallback { portal_error },
        Err(fallback_error) => LinuxDeliveryOutcome::BothFailed {
            portal_error,
            fallback_error,
        },
    }
}

#[cfg(any(target_os = "linux", test))]
fn portal_icon_names(flatpak_id: Option<&str>) -> Vec<String> {
    let mut names = flatpak_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(|id| vec![id.to_string()])
        .unwrap_or_default();

    if !names.iter().any(|name| name == FRAME_NOTIFICATION_ICON) {
        names.push(FRAME_NOTIFICATION_ICON.to_string());
    }

    names
}

#[cfg(not(target_os = "macos"))]
fn show_direct_conversion_finished_notification(
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

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn deliver_system_conversion_finished_notification(summary: ConversionNotificationSummary) {
    if let Err(error) = show_direct_conversion_finished_notification(summary) {
        eprintln!("Failed to show conversion notification: {error}");
    }
}

#[cfg(target_os = "macos")]
fn deliver_system_conversion_finished_notification(summary: ConversionNotificationSummary) {
    initialize_macos_notification_application();

    if let Err(error) = Notification::new()
        .appname(FRAME_APP_NAME)
        .summary(summary.title())
        .body(&summary.body())
        .icon(FRAME_NOTIFICATION_ICON)
        .timeout(Timeout::Default)
        .schedule_raw(macos_delivery_timestamp())
    {
        eprintln!("Failed to show conversion notification: {error}");
    }
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
    use std::cell::Cell;

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
    fn conversion_notification_summary_pluralizes_file_and_error_counts() {
        let cases = [
            (2, 1, "Processed 3 files with 1 error."),
            (1, 2, "Processed 3 files with 2 errors."),
            (1, 1, "Processed 2 files with 1 error."),
            (2, 2, "Processed 4 files with 2 errors."),
            (1, 0, "Processed 1 file with 0 errors."),
            (0, 1, "Processed 1 file with 1 error."),
        ];

        for (completed_count, error_count, expected) in cases {
            let summary = ConversionNotificationSummary {
                completed_count,
                error_count,
            };

            assert_eq!(summary.title(), "Queue Finished");
            assert_eq!(summary.body(), expected);
        }
    }

    #[test]
    fn linux_delivery_uses_portal_without_host_fallback() {
        let fallback_calls = Cell::new(0);

        let outcome = deliver_linux_notification(
            LinuxRuntime::Host,
            || Ok::<(), &str>(()),
            || {
                fallback_calls.set(fallback_calls.get() + 1);
                Ok::<(), &str>(())
            },
        );

        assert_eq!(outcome, LinuxDeliveryOutcome::Portal);
        assert_eq!(fallback_calls.get(), 0);
    }

    #[test]
    fn linux_delivery_uses_portal_without_flatpak_fallback() {
        let fallback_calls = Cell::new(0);

        let outcome = deliver_linux_notification(
            LinuxRuntime::Flatpak,
            || Ok::<(), &str>(()),
            || {
                fallback_calls.set(fallback_calls.get() + 1);
                Ok::<(), &str>(())
            },
        );

        assert_eq!(outcome, LinuxDeliveryOutcome::Portal);
        assert_eq!(fallback_calls.get(), 0);
    }

    #[test]
    fn linux_delivery_falls_back_on_host_after_portal_failure() {
        let fallback_calls = Cell::new(0);

        let outcome = deliver_linux_notification(
            LinuxRuntime::Host,
            || Err::<(), _>("portal unavailable"),
            || {
                fallback_calls.set(fallback_calls.get() + 1);
                Ok::<(), &str>(())
            },
        );

        assert_eq!(
            outcome,
            LinuxDeliveryOutcome::FreedesktopFallback {
                portal_error: "portal unavailable"
            }
        );
        assert_eq!(fallback_calls.get(), 1);
    }

    #[test]
    fn linux_delivery_does_not_fall_back_in_flatpak_after_portal_failure() {
        let fallback_calls = Cell::new(0);

        let outcome = deliver_linux_notification(
            LinuxRuntime::Flatpak,
            || Err::<(), _>("portal unavailable"),
            || {
                fallback_calls.set(fallback_calls.get() + 1);
                Ok::<(), &str>(())
            },
        );

        assert_eq!(
            outcome,
            LinuxDeliveryOutcome::PortalFailedInFlatpak {
                portal_error: "portal unavailable"
            }
        );
        assert_eq!(fallback_calls.get(), 0);
    }

    #[test]
    fn linux_delivery_preserves_portal_and_fallback_failures() {
        let outcome = deliver_linux_notification(
            LinuxRuntime::Host,
            || Err::<(), _>("portal unavailable"),
            || Err::<(), _>("notification daemon unavailable"),
        );

        assert_eq!(
            outcome,
            LinuxDeliveryOutcome::BothFailed {
                portal_error: "portal unavailable",
                fallback_error: "notification daemon unavailable"
            }
        );
    }

    #[test]
    fn portal_icon_names_prefer_flatpak_identity() {
        assert_eq!(
            portal_icon_names(Some("io.github._66HEX.Frame")),
            ["io.github._66HEX.Frame", "frame"]
        );
        assert_eq!(
            portal_icon_names(Some("io.github._66HEX.Frame.Devel")),
            ["io.github._66HEX.Frame.Devel", "frame"]
        );
    }

    #[test]
    fn portal_icon_names_fall_back_to_native_icon() {
        assert_eq!(portal_icon_names(None), ["frame"]);
        assert_eq!(portal_icon_names(Some("  ")), ["frame"]);
        assert_eq!(portal_icon_names(Some("frame")), ["frame"]);
    }

    #[test]
    fn portal_notification_id_is_stable() {
        assert_eq!(CONVERSION_FINISHED_NOTIFICATION_ID, "conversion-finished");
    }
}
