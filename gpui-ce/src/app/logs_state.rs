use super::*;

impl FrameRoot {
    pub(super) fn update_log_scroll_target(&mut self) {
        if self.active_view != ActiveView::Logs {
            return;
        }

        let Some(file_id) = self.conversion_events.selected_log_file_id() else {
            self.last_log_scroll_target = None;
            return;
        };

        let target = LogScrollTarget {
            file_id: file_id.to_string(),
            line_count: self.conversion_events.logs_for(file_id).len(),
        };
        if target.line_count == 0 {
            self.last_log_scroll_target = Some(target);
            return;
        }

        if self.last_log_scroll_target.as_ref() != Some(&target) {
            self.logs_scroll_handle.scroll_to_bottom();
            self.last_log_scroll_target = Some(target);
        }
    }
}
