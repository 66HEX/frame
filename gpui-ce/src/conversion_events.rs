//! GPUI-side reducers for backend conversion events.

use std::{collections::BTreeMap, ops::Range};

use frame_core::events::ConversionEvent;

use crate::file_queue::{FileQueue, FileStatus};

pub const LOG_STICKY_BOTTOM_THRESHOLD_PX: f64 = 25.0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveLogFile {
    pub id: String,
    pub name: String,
    pub status: FileStatus,
    pub line_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogLine {
    pub index: usize,
    pub text: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConversionEventState {
    logs: BTreeMap<String, Vec<String>>,
    selected_log_file_id: Option<String>,
}

impl ConversionEventState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            logs: BTreeMap::new(),
            selected_log_file_id: None,
        }
    }

    #[must_use]
    pub fn selected_log_file_id(&self) -> Option<&str> {
        self.selected_log_file_id.as_deref()
    }

    #[must_use]
    pub fn logs_for(&self, id: &str) -> &[String] {
        self.logs.get(id).map_or(&[], Vec::as_slice)
    }

    pub fn remove_logs(&mut self, id: &str) {
        self.logs.remove(id);
        if self.selected_log_file_id.as_deref() == Some(id) {
            self.selected_log_file_id = None;
        }
    }

    #[must_use]
    pub fn log_lines_for(&self, id: &str) -> Vec<LogLine> {
        self.log_line_window_for(id, 0..self.logs_for(id).len())
    }

    #[must_use]
    pub fn log_line_window_for(&self, id: &str, range: Range<usize>) -> Vec<LogLine> {
        let logs = self.logs_for(id);
        let start = range.start.min(logs.len());
        let end = range.end.min(logs.len());

        logs[start..end]
            .iter()
            .enumerate()
            .map(|(offset, line)| LogLine {
                index: start + offset + 1,
                text: line.clone(),
            })
            .collect()
    }

    #[must_use]
    pub fn active_log_files(&self, queue: &FileQueue) -> Vec<ActiveLogFile> {
        queue
            .files()
            .iter()
            .filter(|file| self.logs.contains_key(&file.id) || file.status != FileStatus::Idle)
            .map(|file| ActiveLogFile {
                id: file.id.clone(),
                name: file.name.clone(),
                status: file.status,
                line_count: self.logs_for(&file.id).len(),
            })
            .collect()
    }

    pub fn select_log_file(&mut self, queue: &FileQueue, id: &str) -> bool {
        if self
            .active_log_files(queue)
            .iter()
            .any(|file| file.id == id)
        {
            self.selected_log_file_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    pub fn ensure_selected_log_file(&mut self, queue: &FileQueue) -> Option<&str> {
        let active_files = self.active_log_files(queue);
        let selected_is_active = self
            .selected_log_file_id
            .as_deref()
            .is_some_and(|selected| active_files.iter().any(|file| file.id == selected));

        if !selected_is_active {
            self.selected_log_file_id = active_files.first().map(|file| file.id.clone());
        }

        self.selected_log_file_id()
    }

    pub fn apply_conversion_event(&mut self, queue: &mut FileQueue, event: ConversionEvent) {
        match event {
            ConversionEvent::Started(payload) => {
                if queue
                    .file_by_id(&payload.id)
                    .is_some_and(|file| file.status == FileStatus::Queued)
                {
                    queue.update_status(&payload.id, FileStatus::Converting, 0);
                }
            }
            ConversionEvent::Progress(payload) => {
                if let Some(file) = queue.file_by_id(&payload.id) {
                    let status = if file.status == FileStatus::Queued {
                        FileStatus::Converting
                    } else {
                        file.status
                    };
                    queue.update_status(&payload.id, status, percent_to_u8(payload.progress));
                }
            }
            ConversionEvent::Completed(payload) => {
                queue.update_status(&payload.id, FileStatus::Completed, 100);
            }
            ConversionEvent::Error(payload) => {
                queue.update_error(&payload.id, payload.error);
            }
            ConversionEvent::Log(payload) => {
                if queue.file_by_id(&payload.id).is_some() {
                    self.logs.entry(payload.id).or_default().push(payload.line);
                }
            }
            ConversionEvent::Cancelled(payload) => {
                queue.update_status(&payload.id, FileStatus::Idle, 0);
                queue.clear_error(&payload.id);
            }
        }

        self.ensure_selected_log_file(queue);
    }
}

#[must_use]
pub fn all_conversions_settled(queue: &FileQueue) -> bool {
    queue.files().iter().all(|file| {
        matches!(
            file.status,
            FileStatus::Completed | FileStatus::Error | FileStatus::Idle
        )
    })
}

#[must_use]
pub fn should_stick_to_bottom(scroll_top: f64, scroll_height: f64, client_height: f64) -> bool {
    if !scroll_top.is_finite() || !scroll_height.is_finite() || !client_height.is_finite() {
        return false;
    }

    scroll_height - scroll_top - client_height < LOG_STICKY_BOTTOM_THRESHOLD_PX
}

fn percent_to_u8(progress: f64) -> u8 {
    if !progress.is_finite() || progress <= 0.0 {
        return 0;
    }
    if progress >= 100.0 {
        return 100;
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "progress is finite and bounded to 0..=100 before rounding"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "negative values are returned before the cast"
    )]
    let rounded = progress.round() as u8;
    rounded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_queue::FileItem;

    fn queue_with_file(status: FileStatus) -> FileQueue {
        let mut queue = FileQueue::new();
        queue.add_file(FileItem::from_path("task-1", "/tmp/source.mp4", 1024));
        queue.update_status("task-1", status, 0);
        queue
    }

    #[test]
    fn apply_conversion_event_started_changes_queued_file_to_converting() {
        let mut queue = queue_with_file(FileStatus::Queued);
        let mut state = ConversionEventState::new();

        state.apply_conversion_event(&mut queue, ConversionEvent::started("task-1"));

        assert_eq!(
            queue.file_by_id("task-1").map(|file| file.status),
            Some(FileStatus::Converting)
        );
    }

    #[test]
    fn apply_conversion_event_progress_preserves_non_queued_status() {
        let mut queue = queue_with_file(FileStatus::Paused);
        let mut state = ConversionEventState::new();

        state.apply_conversion_event(&mut queue, ConversionEvent::progress("task-1", 42.4));

        let file = queue.file_by_id("task-1").expect("file should exist");
        assert_eq!(file.status, FileStatus::Paused);
        assert_eq!(file.progress_percent, 42);
    }

    #[test]
    fn apply_conversion_event_completed_marks_file_ready() {
        let mut queue = queue_with_file(FileStatus::Converting);
        let mut state = ConversionEventState::new();

        state.apply_conversion_event(
            &mut queue,
            ConversionEvent::completed("task-1", "/tmp/output.mp4"),
        );

        let file = queue.file_by_id("task-1").expect("file should exist");
        assert_eq!(file.status, FileStatus::Completed);
        assert_eq!(file.progress_percent, 100);
    }

    #[test]
    fn apply_conversion_event_error_stores_message() {
        let mut queue = queue_with_file(FileStatus::Converting);
        let mut state = ConversionEventState::new();

        state.apply_conversion_event(
            &mut queue,
            ConversionEvent::error("task-1", "ffmpeg failed"),
        );

        let file = queue.file_by_id("task-1").expect("file should exist");
        assert_eq!(file.status, FileStatus::Error);
        assert_eq!(file.conversion_error.as_deref(), Some("ffmpeg failed"));
    }

    #[test]
    fn apply_conversion_event_cancelled_resets_file_to_idle() {
        let mut queue = queue_with_file(FileStatus::Converting);
        queue.update_error("task-1", "cancel path error");
        let mut state = ConversionEventState::new();

        state.apply_conversion_event(&mut queue, ConversionEvent::cancelled("task-1"));

        let file = queue.file_by_id("task-1").expect("file should exist");
        assert_eq!(file.status, FileStatus::Idle);
        assert_eq!(file.progress_percent, 0);
        assert_eq!(file.conversion_error, None);
    }

    #[test]
    fn apply_conversion_event_log_appends_lines_in_order() {
        let mut queue = queue_with_file(FileStatus::Idle);
        let mut state = ConversionEventState::new();

        state.apply_conversion_event(&mut queue, ConversionEvent::log("task-1", "first"));
        state.apply_conversion_event(&mut queue, ConversionEvent::log("task-1", "second"));

        assert_eq!(state.logs_for("task-1"), ["first", "second"]);
        assert_eq!(
            state.log_lines_for("task-1"),
            [
                LogLine {
                    index: 1,
                    text: "first".to_string(),
                },
                LogLine {
                    index: 2,
                    text: "second".to_string(),
                }
            ]
        );
    }

    #[test]
    fn apply_conversion_event_log_ignores_removed_files() {
        let mut queue = FileQueue::new();
        let mut state = ConversionEventState::new();

        state.apply_conversion_event(&mut queue, ConversionEvent::log("task-1", "late line"));

        assert!(state.logs_for("task-1").is_empty());
    }

    #[test]
    fn remove_logs_clears_selected_log_target() {
        let mut queue = queue_with_file(FileStatus::Converting);
        let mut state = ConversionEventState::new();
        state.apply_conversion_event(&mut queue, ConversionEvent::log("task-1", "line"));
        state.select_log_file(&queue, "task-1");

        state.remove_logs("task-1");

        assert_eq!(state.selected_log_file_id(), None);
    }

    #[test]
    fn log_line_window_for_returns_numbered_visible_range() {
        let mut queue = queue_with_file(FileStatus::Idle);
        let mut state = ConversionEventState::new();

        state.apply_conversion_event(&mut queue, ConversionEvent::log("task-1", "first"));
        state.apply_conversion_event(&mut queue, ConversionEvent::log("task-1", "second"));
        state.apply_conversion_event(&mut queue, ConversionEvent::log("task-1", "third"));

        assert_eq!(
            state.log_line_window_for("task-1", 1..3),
            [
                LogLine {
                    index: 2,
                    text: "second".to_string(),
                },
                LogLine {
                    index: 3,
                    text: "third".to_string(),
                }
            ]
        );
    }

    #[test]
    fn active_log_files_include_idle_files_with_logs() {
        let mut queue = queue_with_file(FileStatus::Idle);
        let mut state = ConversionEventState::new();
        state.apply_conversion_event(&mut queue, ConversionEvent::log("task-1", "line"));

        assert_eq!(
            state.active_log_files(&queue),
            [ActiveLogFile {
                id: "task-1".to_string(),
                name: "source.mp4".to_string(),
                status: FileStatus::Idle,
                line_count: 1,
            }]
        );
    }

    #[test]
    fn ensure_selected_log_file_selects_first_active_file() {
        let mut queue = FileQueue::new();
        queue.add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        queue.add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
        queue.update_status("second", FileStatus::Queued, 0);
        let mut state = ConversionEventState::new();

        assert_eq!(state.ensure_selected_log_file(&queue), Some("second"));
    }

    #[test]
    fn select_log_file_preserves_active_selection() {
        let mut queue = FileQueue::new();
        queue.add_file(FileItem::from_path("first", "/tmp/one.mp4", 1));
        queue.add_file(FileItem::from_path("second", "/tmp/two.mp4", 1));
        queue.update_status("first", FileStatus::Queued, 0);
        queue.update_status("second", FileStatus::Queued, 0);
        let mut state = ConversionEventState::new();

        assert!(state.select_log_file(&queue, "second"));
        state.ensure_selected_log_file(&queue);

        assert_eq!(state.selected_log_file_id(), Some("second"));
    }

    #[test]
    fn all_conversions_settled_is_false_for_active_queue_items() {
        let queue = queue_with_file(FileStatus::Queued);

        assert!(!all_conversions_settled(&queue));
    }

    #[test]
    fn all_conversions_settled_accepts_terminal_and_idle_items() {
        let mut queue = FileQueue::new();
        queue.add_file(FileItem::from_path("idle", "/tmp/idle.mp4", 1));
        queue.add_file(FileItem::from_path("done", "/tmp/done.mp4", 1));
        queue.update_status("done", FileStatus::Completed, 100);

        assert!(all_conversions_settled(&queue));
    }

    #[test]
    fn should_stick_to_bottom_matches_original_threshold() {
        assert!(should_stick_to_bottom(475.1, 1000.0, 500.0));
        assert!(!should_stick_to_bottom(474.9, 1000.0, 500.0));
        assert!(!should_stick_to_bottom(f64::NAN, 1000.0, 500.0));
    }

    #[test]
    fn log_lines_for_handles_large_outputs() {
        let mut queue = queue_with_file(FileStatus::Converting);
        let mut state = ConversionEventState::new();

        for index in 0..10_000 {
            state.apply_conversion_event(
                &mut queue,
                ConversionEvent::log("task-1", format!("ffmpeg line {index}")),
            );
        }

        let lines = state.log_lines_for("task-1");
        assert_eq!(lines.len(), 10_000);
        assert_eq!(lines[0].index, 1);
        assert_eq!(lines[9_999].index, 10_000);
        assert_eq!(lines[9_999].text, "ffmpeg line 9999");
    }
}
