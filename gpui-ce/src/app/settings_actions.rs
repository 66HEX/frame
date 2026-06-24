use super::*;

impl FrameRoot {
    pub(super) fn open_app_settings(&mut self) {
        self.is_settings_open = true;
        self.max_concurrency_draft = self.max_concurrency.to_string();
        self.max_concurrency_error = None;
    }
    pub(super) fn close_app_settings(&mut self) {
        self.is_settings_open = false;
        self.max_concurrency_error = None;
        self.app_settings_value_focus = None;
        if self.active_text_input == Some(FrameTextInputKind::MaxConcurrency) {
            self.stop_text_input_cursor();
        }
    }
    pub(super) fn apply_max_concurrency_draft(&mut self) -> bool {
        let Some(value) = self.parsed_max_concurrency_draft() else {
            self.max_concurrency_error =
                Some("Enter a whole number greater than zero.".to_string());
            return false;
        };

        match self.conversion_processes.update_max_concurrency(value) {
            Ok(()) => {
                self.max_concurrency = value;
                self.max_concurrency_draft = value.to_string();
                self.max_concurrency_error = None;
                true
            }
            Err(error) => {
                self.max_concurrency_error = Some(error.to_string());
                false
            }
        }
    }
    pub(super) fn parsed_max_concurrency_draft(&self) -> Option<usize> {
        let trimmed = self.max_concurrency_draft.trim();
        let value = trimmed.parse::<usize>().ok()?;
        (value > 0).then_some(value)
    }
}
