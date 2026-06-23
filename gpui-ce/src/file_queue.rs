//! File queue state shared by Frame workspace, titlebar counters, and conversion reducers.

use std::path::Path;

use crate::settings::ConversionConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileStatus {
    Idle,
    Queued,
    Converting,
    Paused,
    Completed,
    Error,
}

impl FileStatus {
    #[must_use]
    pub const fn locks_settings(self) -> bool {
        matches!(self, Self::Converting | Self::Queued | Self::Completed)
    }

    #[must_use]
    pub const fn can_be_cancelled_before_removal(self) -> bool {
        matches!(self, Self::Converting | Self::Paused | Self::Queued)
    }

    #[must_use]
    pub const fn can_be_removed_from_list(self) -> bool {
        !matches!(self, Self::Converting)
    }

    #[must_use]
    pub const fn is_actionable_for_conversion(self) -> bool {
        !matches!(self, Self::Completed)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Queued => "Queued",
            Self::Converting => "Converting",
            Self::Paused => "Paused",
            Self::Completed => "Ready",
            Self::Error => "Error",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileStateTone {
    Foreground,
    Muted,
    Amber,
    Red,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RowActionAvailability {
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_delete: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BatchSelectionState {
    pub is_checked: bool,
    pub is_indeterminate: bool,
    pub is_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileItem {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub status: FileStatus,
    pub progress_percent: u8,
    pub original_format: String,
    pub output_name: String,
    pub config: ConversionConfig,
    pub path: String,
    pub is_selected_for_conversion: bool,
    pub conversion_error: Option<String>,
}

impl FileItem {
    #[must_use]
    pub fn from_path(id: impl Into<String>, path: impl Into<String>, size_bytes: u64) -> Self {
        let path = path.into();
        let name = file_name_from_path(&path).to_string();
        Self {
            id: id.into(),
            original_format: original_format_from_name(&name).to_string(),
            output_name: derive_output_name(&name),
            config: ConversionConfig::default(),
            name,
            size_bytes,
            status: FileStatus::Idle,
            progress_percent: 0,
            path,
            is_selected_for_conversion: true,
            conversion_error: None,
        }
    }

    #[must_use]
    pub fn from_os_path(id: impl Into<String>, path: &Path) -> Self {
        Self::from_path(id, path.to_string_lossy(), file_size_bytes(path))
    }

    #[must_use]
    pub const fn locks_settings(&self) -> bool {
        self.status.locks_settings()
    }

    #[must_use]
    pub fn row_state_label(&self) -> String {
        match self.status {
            FileStatus::Converting | FileStatus::Paused => {
                format!("{}%", self.progress_percent)
            }
            FileStatus::Completed => "ready".to_string(),
            FileStatus::Queued => "queued".to_string(),
            FileStatus::Error => "error".to_string(),
            FileStatus::Idle => "idle".to_string(),
        }
    }

    #[must_use]
    pub const fn row_state_tone(&self) -> FileStateTone {
        match self.status {
            FileStatus::Converting => FileStateTone::Amber,
            FileStatus::Completed => FileStateTone::Foreground,
            FileStatus::Error => FileStateTone::Red,
            FileStatus::Idle | FileStatus::Queued | FileStatus::Paused => FileStateTone::Muted,
        }
    }

    #[must_use]
    pub const fn row_actions(&self) -> RowActionAvailability {
        RowActionAvailability {
            can_pause: matches!(self.status, FileStatus::Converting),
            can_resume: matches!(self.status, FileStatus::Paused),
            can_delete: self.status.can_be_removed_from_list(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileQueue {
    files: Vec<FileItem>,
    selected_file_id: Option<String>,
}

impl FileQueue {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            files: Vec::new(),
            selected_file_id: None,
        }
    }

    #[must_use]
    pub fn files(&self) -> &[FileItem] {
        &self.files
    }

    #[must_use]
    pub fn selected_file_id(&self) -> Option<&str> {
        self.selected_file_id.as_deref()
    }

    #[must_use]
    pub fn selected_file(&self) -> Option<&FileItem> {
        self.selected_file_id
            .as_deref()
            .and_then(|id| self.files.iter().find(|file| file.id == id))
    }

    pub fn selected_file_mut(&mut self) -> Option<&mut FileItem> {
        let selected_file_id = self.selected_file_id.as_deref()?;
        self.files
            .iter_mut()
            .find(|file| file.id == selected_file_id)
    }

    #[must_use]
    pub fn file_by_id(&self, id: &str) -> Option<&FileItem> {
        self.files.iter().find(|file| file.id == id)
    }

    #[must_use]
    pub fn selected_file_locked(&self) -> bool {
        self.selected_file().is_some_and(FileItem::locks_settings)
    }

    #[must_use]
    pub fn total_size_bytes(&self) -> u64 {
        self.files.iter().map(|file| file.size_bytes).sum()
    }

    #[must_use]
    pub fn selected_count(&self) -> usize {
        self.files
            .iter()
            .filter(|file| file.is_selected_for_conversion)
            .count()
    }

    #[must_use]
    pub fn has_actionable_files(&self) -> bool {
        self.files.iter().any(|file| {
            file.is_selected_for_conversion && file.status.is_actionable_for_conversion()
        })
    }

    #[must_use]
    pub fn all_checked(&self) -> bool {
        !self.files.is_empty()
            && self
                .files
                .iter()
                .all(|file| file.is_selected_for_conversion)
    }

    #[must_use]
    pub fn is_indeterminate(&self) -> bool {
        self.files
            .iter()
            .any(|file| file.is_selected_for_conversion)
            && !self.all_checked()
    }

    #[must_use]
    pub fn batch_selection_state(&self) -> BatchSelectionState {
        BatchSelectionState {
            is_checked: self.all_checked(),
            is_indeterminate: self.is_indeterminate(),
            is_enabled: !self.files.is_empty(),
        }
    }

    pub fn add_file(&mut self, file: FileItem) {
        let should_select = self.selected_file_id.is_none();
        let id = file.id.clone();
        self.files.push(file);
        if should_select {
            self.selected_file_id = Some(id);
        }
    }

    pub fn add_files(&mut self, files: impl IntoIterator<Item = FileItem>) -> usize {
        let mut added_count = 0;
        for file in files {
            self.add_file(file);
            added_count += 1;
        }
        added_count
    }

    pub fn remove_file(&mut self, id: &str) -> Option<FileItem> {
        let index = self.files.iter().position(|file| file.id == id)?;
        let removed = self.files.remove(index);
        if self.selected_file_id.as_deref() == Some(id) {
            self.selected_file_id = None;
        }
        Some(removed)
    }

    pub fn remove_interactive_file(&mut self, id: &str) -> Option<FileItem> {
        let file = self.files.iter().find(|file| file.id == id)?;
        if !file.status.can_be_removed_from_list() {
            return None;
        }

        self.remove_file(id)
    }

    pub fn select_file(&mut self, id: impl Into<Option<String>>) {
        self.selected_file_id = id.into();
    }

    pub fn select_existing_file(&mut self, id: &str) -> bool {
        if self.files.iter().any(|file| file.id == id) {
            self.selected_file_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    pub fn toggle_batch(&mut self, id: &str, is_checked: bool) {
        if let Some(file) = self.files.iter_mut().find(|file| file.id == id) {
            file.is_selected_for_conversion = is_checked;
        }
    }

    pub fn toggle_batch_selection(&mut self, id: &str) -> Option<bool> {
        let file = self.files.iter_mut().find(|file| file.id == id)?;
        file.is_selected_for_conversion = !file.is_selected_for_conversion;
        Some(file.is_selected_for_conversion)
    }

    pub fn toggle_all_batch(&mut self, is_checked: bool) {
        for file in &mut self.files {
            file.is_selected_for_conversion = is_checked;
        }
    }

    pub fn toggle_all_batch_selection(&mut self) -> bool {
        let is_checked = !self.all_checked();
        self.toggle_all_batch(is_checked);
        is_checked
    }

    pub fn pause_file(&mut self, id: &str) -> bool {
        let Some(file) = self.files.iter_mut().find(|file| file.id == id) else {
            return false;
        };
        if file.status != FileStatus::Converting {
            return false;
        }

        file.status = FileStatus::Paused;
        true
    }

    pub fn resume_file(&mut self, id: &str) -> bool {
        let Some(file) = self.files.iter_mut().find(|file| file.id == id) else {
            return false;
        };
        if file.status != FileStatus::Paused {
            return false;
        }

        file.status = FileStatus::Converting;
        true
    }

    pub fn update_status(&mut self, id: &str, status: FileStatus, progress_percent: u8) -> bool {
        if let Some(file) = self.files.iter_mut().find(|file| file.id == id) {
            file.status = status;
            file.progress_percent = progress_percent.min(100);
            true
        } else {
            false
        }
    }

    pub fn update_error(&mut self, id: &str, error: impl Into<String>) -> bool {
        if let Some(file) = self.files.iter_mut().find(|file| file.id == id) {
            file.status = FileStatus::Error;
            file.conversion_error = Some(error.into());
            true
        } else {
            false
        }
    }

    pub fn clear_error(&mut self, id: &str) -> bool {
        if let Some(file) = self.files.iter_mut().find(|file| file.id == id) {
            file.conversion_error = None;
            true
        } else {
            false
        }
    }
}

#[must_use]
pub fn file_size_bytes(path: &Path) -> u64 {
    path.metadata()
        .ok()
        .filter(std::fs::Metadata::is_file)
        .map_or(0, |metadata| metadata.len())
}

#[must_use]
pub fn file_name_from_path(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("unknown")
}

#[must_use]
pub fn original_format_from_name(name: &str) -> &str {
    name.rsplit('.')
        .next()
        .filter(|extension| !extension.is_empty())
        .unwrap_or("unknown")
}

#[must_use]
pub fn derive_output_name(file_name: &str) -> String {
    let base = file_name.rfind('.').map_or(file_name, |dot_index| {
        let extension = &file_name[dot_index + 1..];
        if extension.is_empty() || extension.contains(['/', '\\', '.']) {
            file_name
        } else {
            &file_name[..dot_index]
        }
    });

    if base.is_empty() {
        "output_converted".to_string()
    } else {
        format!("{base}_converted")
    }
}

#[must_use]
pub fn format_file_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }

    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    format!("{} {}", trim_two_decimal_places(value), UNITS[unit_index])
}

fn trim_two_decimal_places(value: f64) -> String {
    let formatted = format!("{value:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_file(id: &str, path: &str, size_bytes: u64) -> FileItem {
        FileItem::from_path(id, path, size_bytes)
    }

    mod file_status {
        use super::*;

        #[test]
        fn locks_settings_for_current_original_locked_statuses() {
            assert!(FileStatus::Converting.locks_settings());
            assert!(FileStatus::Queued.locks_settings());
            assert!(FileStatus::Completed.locks_settings());
        }

        #[test]
        fn keeps_paused_files_editable_like_original_ui() {
            assert!(!FileStatus::Paused.locks_settings());
        }

        #[test]
        fn completed_files_are_not_actionable_for_conversion() {
            assert!(!FileStatus::Completed.is_actionable_for_conversion());
        }

        #[test]
        fn converting_files_are_not_removed_directly_from_list() {
            assert!(!FileStatus::Converting.can_be_removed_from_list());
        }
    }

    mod file_item {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn from_path_derives_name_from_unix_path() {
            let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

            assert_eq!(file.name, "video.mp4");
        }

        #[test]
        fn from_path_derives_name_from_windows_path() {
            let file = FileItem::from_path("1", r"C:\Users\hex\video.mp4", 10);

            assert_eq!(file.name, "video.mp4");
        }

        #[test]
        fn from_path_initializes_conversion_selection_like_original_add_flow() {
            let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

            assert!(file.is_selected_for_conversion);
        }

        #[test]
        fn from_path_initializes_per_file_conversion_config() {
            let file = FileItem::from_path("1", "/tmp/video.mp4", 10);

            assert_eq!(file.config, ConversionConfig::default());
        }

        #[test]
        fn from_os_path_reads_size_from_existing_file() {
            let path = temp_file_path("size-source.mp4");
            std::fs::write(&path, [1_u8, 2, 3]).expect("test file should be written");

            let file = FileItem::from_os_path("1", &path);

            std::fs::remove_file(&path).expect("test file should be removed");
            assert_eq!(file.size_bytes, 3);
        }

        #[test]
        fn from_os_path_uses_zero_size_when_metadata_is_unavailable() {
            let path = temp_file_path("missing-source.mp4");

            let file = FileItem::from_os_path("1", &path);

            assert_eq!(file.size_bytes, 0);
        }

        #[test]
        fn converting_row_state_uses_progress_percent() {
            let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
            file.status = FileStatus::Converting;
            file.progress_percent = 42;

            assert_eq!(file.row_state_label(), "42%");
            assert_eq!(file.row_state_tone(), FileStateTone::Amber);
        }

        #[test]
        fn completed_row_state_matches_ready_label() {
            let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
            file.status = FileStatus::Completed;

            assert_eq!(file.row_state_label(), "ready");
            assert_eq!(file.row_state_tone(), FileStateTone::Foreground);
        }

        #[test]
        fn error_row_state_uses_red_tone() {
            let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
            file.status = FileStatus::Error;

            assert_eq!(file.row_state_label(), "error");
            assert_eq!(file.row_state_tone(), FileStateTone::Red);
        }

        #[test]
        fn converting_row_can_pause_but_not_delete_directly() {
            let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
            file.status = FileStatus::Converting;

            assert_eq!(
                file.row_actions(),
                RowActionAvailability {
                    can_pause: true,
                    can_resume: false,
                    can_delete: false,
                }
            );
        }

        #[test]
        fn paused_row_can_resume_and_delete() {
            let mut file = FileItem::from_path("1", "/tmp/video.mp4", 10);
            file.status = FileStatus::Paused;

            assert_eq!(
                file.row_actions(),
                RowActionAvailability {
                    can_pause: false,
                    can_resume: true,
                    can_delete: true,
                }
            );
        }

        fn temp_file_path(name: &str) -> PathBuf {
            std::env::temp_dir().join(format!("frame-gpui-ce-{}-{name}", std::process::id()))
        }
    }

    mod derive_output_name {
        use super::*;

        #[test]
        fn appends_converted_to_file_stem() {
            assert_eq!(derive_output_name("clip.mp4"), "clip_converted");
        }

        #[test]
        fn removes_only_final_extension() {
            assert_eq!(
                derive_output_name("archive.tar.gz"),
                "archive.tar_converted"
            );
        }

        #[test]
        fn falls_back_when_hidden_file_stem_is_empty() {
            assert_eq!(derive_output_name(".gitignore"), "output_converted");
        }
    }

    mod original_format_from_name {
        use super::*;

        #[test]
        fn uses_final_extension() {
            assert_eq!(original_format_from_name("archive.tar.gz"), "gz");
        }

        #[test]
        fn falls_back_when_trailing_dot_has_no_extension() {
            assert_eq!(original_format_from_name("clip."), "unknown");
        }
    }

    mod format_file_size {
        use super::*;

        #[test]
        fn returns_zero_bytes_label() {
            assert_eq!(format_file_size(0), "0 B");
        }

        #[test]
        fn trims_trailing_decimal_zeroes_like_javascript_parse_float() {
            assert_eq!(format_file_size(1536), "1.5 KB");
        }

        #[test]
        fn formats_megabytes_without_unneeded_decimals() {
            assert_eq!(format_file_size(1024 * 1024), "1 MB");
        }
    }

    mod file_queue {
        use super::*;

        #[test]
        fn add_file_selects_first_file_when_selection_is_empty() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 1));

            assert_eq!(queue.selected_file_id(), Some("first"));
        }

        #[test]
        fn add_file_preserves_existing_selection() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 1));

            assert_eq!(queue.selected_file_id(), Some("first"));
        }

        #[test]
        fn selected_file_mut_updates_selected_file_only() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 1));
            queue.select_existing_file("second");

            queue
                .selected_file_mut()
                .expect("selected file should exist")
                .config
                .container = "webm".to_string();

            assert_eq!(
                queue
                    .file_by_id("first")
                    .map(|file| file.config.container.as_str()),
                Some("mp4")
            );
            assert_eq!(
                queue
                    .file_by_id("second")
                    .map(|file| file.config.container.as_str()),
                Some("webm")
            );
        }

        #[test]
        fn add_files_returns_added_count() {
            let mut queue = FileQueue::new();

            let added_count = queue.add_files([
                sample_file("first", "/tmp/one.mp4", 1),
                sample_file("second", "/tmp/two.mp4", 1),
            ]);

            assert_eq!(added_count, 2);
        }

        #[test]
        fn add_files_selects_first_import_when_queue_was_empty() {
            let mut queue = FileQueue::new();

            queue.add_files([
                sample_file("first", "/tmp/one.mp4", 1),
                sample_file("second", "/tmp/two.mp4", 1),
            ]);

            assert_eq!(queue.selected_file_id(), Some("first"));
        }

        #[test]
        fn add_files_preserves_existing_selection() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("existing", "/tmp/existing.mp4", 1));

            queue.add_files([
                sample_file("first", "/tmp/one.mp4", 1),
                sample_file("second", "/tmp/two.mp4", 1),
            ]);

            assert_eq!(queue.selected_file_id(), Some("existing"));
        }

        #[test]
        fn remove_file_clears_selection_when_selected_file_is_removed() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
            queue.remove_file("first");

            assert_eq!(queue.selected_file_id(), None);
        }

        #[test]
        fn remove_interactive_file_keeps_converting_file_in_queue() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
            queue.update_status("first", FileStatus::Converting, 20);

            assert!(queue.remove_interactive_file("first").is_none());
            assert_eq!(queue.files().len(), 1);
        }

        #[test]
        fn remove_interactive_file_removes_paused_file_after_cancel_path() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
            queue.update_status("first", FileStatus::Paused, 20);

            assert!(queue.remove_interactive_file("first").is_some());
            assert!(queue.files().is_empty());
        }

        #[test]
        fn select_existing_file_ignores_unknown_ids() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 1));

            assert!(!queue.select_existing_file("missing"));
            assert_eq!(queue.selected_file_id(), Some("first"));
        }

        #[test]
        fn select_existing_file_updates_selection_for_known_id() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 1));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 1));

            assert!(queue.select_existing_file("second"));
            assert_eq!(queue.selected_file_id(), Some("second"));
        }

        #[test]
        fn total_size_bytes_sums_all_files() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 15));

            assert_eq!(queue.total_size_bytes(), 25);
        }

        #[test]
        fn selected_count_counts_batch_selected_files() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 15));
            queue.toggle_batch("second", false);

            assert_eq!(queue.selected_count(), 1);
        }

        #[test]
        fn toggle_batch_selection_inverts_single_file_selection() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

            assert_eq!(queue.toggle_batch_selection("first"), Some(false));
            assert_eq!(queue.selected_count(), 0);
        }

        #[test]
        fn toggle_batch_selection_returns_none_for_unknown_file() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

            assert_eq!(queue.toggle_batch_selection("missing"), None);
        }

        #[test]
        fn has_actionable_files_ignores_completed_files() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.update_status("first", FileStatus::Completed, 100);

            assert!(!queue.has_actionable_files());
        }

        #[test]
        fn all_checked_is_false_for_empty_queue() {
            let queue = FileQueue::new();

            assert!(!queue.all_checked());
        }

        #[test]
        fn is_indeterminate_matches_partial_batch_selection() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 15));
            queue.toggle_batch("second", false);

            assert!(queue.is_indeterminate());
        }

        #[test]
        fn batch_selection_state_is_disabled_for_empty_queue() {
            let queue = FileQueue::new();

            assert_eq!(
                queue.batch_selection_state(),
                BatchSelectionState {
                    is_checked: false,
                    is_indeterminate: false,
                    is_enabled: false,
                }
            );
        }

        #[test]
        fn batch_selection_state_reports_all_files_checked() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 15));

            assert_eq!(
                queue.batch_selection_state(),
                BatchSelectionState {
                    is_checked: true,
                    is_indeterminate: false,
                    is_enabled: true,
                }
            );
        }

        #[test]
        fn batch_selection_state_reports_partial_selection() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 15));
            queue.toggle_batch("second", false);

            assert_eq!(
                queue.batch_selection_state(),
                BatchSelectionState {
                    is_checked: false,
                    is_indeterminate: true,
                    is_enabled: true,
                }
            );
        }

        #[test]
        fn toggle_all_batch_selection_selects_all_from_partial_state() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 15));
            queue.toggle_batch("second", false);

            assert!(queue.toggle_all_batch_selection());
            assert!(queue.all_checked());
        }

        #[test]
        fn toggle_all_batch_selection_unchecks_all_when_all_are_checked() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 15));

            assert!(!queue.toggle_all_batch_selection());
            assert_eq!(queue.selected_count(), 0);
        }

        #[test]
        fn selected_file_locked_uses_selected_file_status() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.update_status("first", FileStatus::Queued, 0);

            assert!(queue.selected_file_locked());
        }

        #[test]
        fn update_status_clamps_progress_to_percent_range() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.update_status("first", FileStatus::Converting, 250);

            assert_eq!(
                queue.selected_file().map(|file| file.progress_percent),
                Some(100)
            );
        }

        #[test]
        fn file_by_id_returns_matching_file_without_changing_selection() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.add_file(sample_file("second", "/tmp/two.mp4", 10));

            assert_eq!(
                queue.file_by_id("second").map(|file| file.name.as_str()),
                Some("two.mp4")
            );
            assert_eq!(queue.selected_file_id(), Some("first"));
        }

        #[test]
        fn update_error_stores_conversion_error_message() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

            assert!(queue.update_error("first", "ffmpeg failed"));

            let file = queue.file_by_id("first").expect("file should exist");
            assert_eq!(file.status, FileStatus::Error);
            assert_eq!(file.conversion_error.as_deref(), Some("ffmpeg failed"));
        }

        #[test]
        fn clear_error_removes_previous_conversion_error() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.update_error("first", "ffmpeg failed");

            assert!(queue.clear_error("first"));

            assert_eq!(
                queue
                    .file_by_id("first")
                    .and_then(|file| file.conversion_error.as_deref()),
                None
            );
        }

        #[test]
        fn pause_file_changes_only_converting_file_to_paused() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.update_status("first", FileStatus::Converting, 40);

            assert!(queue.pause_file("first"));
            assert_eq!(
                queue.selected_file().map(|file| file.status),
                Some(FileStatus::Paused)
            );
        }

        #[test]
        fn pause_file_ignores_non_converting_file() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

            assert!(!queue.pause_file("first"));
            assert_eq!(
                queue.selected_file().map(|file| file.status),
                Some(FileStatus::Idle)
            );
        }

        #[test]
        fn resume_file_changes_only_paused_file_to_converting() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));
            queue.update_status("first", FileStatus::Paused, 40);

            assert!(queue.resume_file("first"));
            assert_eq!(
                queue.selected_file().map(|file| file.status),
                Some(FileStatus::Converting)
            );
        }

        #[test]
        fn resume_file_ignores_non_paused_file() {
            let mut queue = FileQueue::new();
            queue.add_file(sample_file("first", "/tmp/one.mp4", 10));

            assert!(!queue.resume_file("first"));
            assert_eq!(
                queue.selected_file().map(|file| file.status),
                Some(FileStatus::Idle)
            );
        }
    }
}
