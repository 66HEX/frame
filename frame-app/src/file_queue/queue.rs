use crate::settings::sanitize_output_name;

use super::{
    format::derive_output_name,
    item::FileItem,
    status::{BatchSelectionState, FileStatus},
};

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

    pub fn files_mut(&mut self) -> &mut [FileItem] {
        &mut self.files
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
            self.selected_file_id = self
                .files
                .get(index)
                .or_else(|| index.checked_sub(1).and_then(|index| self.files.get(index)))
                .map(|file| file.id.clone());
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

    pub fn select_first_file(&mut self) -> bool {
        let Some(file) = self.files.first() else {
            return false;
        };
        if self.selected_file_id.as_deref() == Some(file.id.as_str()) {
            return false;
        }
        self.selected_file_id = Some(file.id.clone());
        true
    }

    pub fn select_last_file(&mut self) -> bool {
        let Some(file) = self.files.last() else {
            return false;
        };
        if self.selected_file_id.as_deref() == Some(file.id.as_str()) {
            return false;
        }
        self.selected_file_id = Some(file.id.clone());
        true
    }

    pub fn select_next_file(&mut self) -> bool {
        let Some(index) = self.selected_index() else {
            return self.select_first_file();
        };
        let Some(file) = self.files.get(index + 1) else {
            return false;
        };
        self.selected_file_id = Some(file.id.clone());
        true
    }

    pub fn select_previous_file(&mut self) -> bool {
        let Some(index) = self.selected_index() else {
            return self.select_last_file();
        };
        let Some(previous_index) = index.checked_sub(1) else {
            return false;
        };
        let Some(file) = self.files.get(previous_index) else {
            return false;
        };
        self.selected_file_id = Some(file.id.clone());
        true
    }

    fn selected_index(&self) -> Option<usize> {
        let selected_file_id = self.selected_file_id.as_deref()?;
        self.files
            .iter()
            .position(|file| file.id == selected_file_id)
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

    pub fn mark_file_cancelling(&mut self, id: &str) -> bool {
        let Some(file) = self.files.iter_mut().find(|file| file.id == id) else {
            return false;
        };
        if !file.status.can_be_cancelled() {
            return false;
        }

        file.status = FileStatus::Cancelling;
        true
    }

    pub fn prepare_file_for_reconversion(&mut self, id: &str) -> bool {
        let Some(file) = self.files.iter_mut().find(|file| file.id == id) else {
            return false;
        };
        if file.status != FileStatus::Completed {
            return false;
        }

        file.status = FileStatus::Idle;
        file.progress_percent = 0;
        file.conversion_error = None;
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

    pub fn update_selected_output_name(&mut self, value: &str) -> bool {
        let Some(file) = self.selected_file_mut() else {
            return false;
        };

        let sanitized = sanitize_output_name(value);
        let next_output_name = if sanitized.is_empty() {
            derive_output_name(&file.name)
        } else {
            sanitized
        };

        if file.output_name == next_output_name {
            return false;
        }

        file.output_name = next_output_name;
        true
    }

    pub fn set_selected_output_name_from_input(&mut self, value: &str) -> bool {
        let Some(file) = self.selected_file_mut() else {
            return false;
        };

        let next_output_name = sanitize_output_name(value);
        if file.output_name == next_output_name {
            return false;
        }

        file.output_name = next_output_name;
        true
    }

    pub fn queue_selected_pending_conversions(&mut self) -> Vec<FileItem> {
        let mut pending_files = Vec::new();

        for file in &mut self.files {
            if !file.is_selected_for_conversion || !file.status.is_actionable_for_conversion() {
                continue;
            }

            file.status = FileStatus::Queued;
            file.progress_percent = 0;
            file.conversion_error = None;
            pending_files.push(file.clone());
        }

        pending_files
    }
}
