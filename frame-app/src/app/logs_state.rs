use super::*;

impl FrameRoot {
    pub(super) fn update_log_scroll_target(&mut self) {
        if self.active_view != ActiveView::Logs {
            self.logs_follow_tail = true;
            self.last_log_scroll_target = None;
            return;
        }

        let Some(file_id) = self.conversion_events.selected_log_file_id() else {
            self.logs_follow_tail = true;
            self.last_log_scroll_target = None;
            return;
        };

        let target = LogScrollTarget {
            file_id: file_id.to_string(),
            line_count: self.conversion_events.logs_for(file_id).len(),
        };
        let target_file_changed = self
            .last_log_scroll_target
            .as_ref()
            .is_none_or(|previous| previous.file_id != target.file_id);
        if target_file_changed {
            self.logs_follow_tail = true;
        }

        if target.line_count == 0 {
            self.last_log_scroll_target = Some(target);
            return;
        }

        if self.logs_follow_tail && self.last_log_scroll_target.as_ref() != Some(&target) {
            self.scroll_logs_to_bottom(target.line_count);
            self.last_log_scroll_target = Some(target);
            return;
        }

        self.last_log_scroll_target = Some(target);
    }

    pub(super) fn select_log_file_for_logs_view(&mut self, file_id: &str) -> bool {
        if !self
            .conversion_events
            .select_log_file(&self.file_queue, file_id)
        {
            return false;
        }

        self.logs_follow_tail = true;
        self.last_log_scroll_target = None;
        true
    }

    pub(super) fn sync_logs_follow_tail_after_user_scroll(&mut self) -> bool {
        let follow_tail = should_follow_logs_tail_after_scroll_position(
            self.logs_scroll_handle.is_scrolled_to_end(),
        );
        if self.logs_follow_tail == follow_tail {
            return false;
        }

        self.logs_follow_tail = follow_tail;
        true
    }

    pub(super) fn scroll_selected_log_to_bottom(&mut self) -> bool {
        let Some(file_id) = self.conversion_events.selected_log_file_id() else {
            return false;
        };

        let line_count = self.conversion_events.logs_for(file_id).len();
        if line_count == 0 {
            return false;
        }

        self.logs_follow_tail = true;
        self.scroll_logs_to_bottom(line_count);
        self.last_log_scroll_target = Some(LogScrollTarget {
            file_id: file_id.to_string(),
            line_count,
        });
        true
    }

    fn scroll_logs_to_bottom(&self, line_count: usize) {
        self.logs_scroll_handle
            .scroll_to_item_strict(line_count.saturating_sub(1), ScrollStrategy::Bottom);
    }
}

#[must_use]
pub(super) fn should_follow_logs_tail_after_scroll_position(scrolled_to_end: Option<bool>) -> bool {
    scrolled_to_end.unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_queue::FileItem;

    fn root_with_logs(line_count: usize) -> FrameRoot {
        let mut root = FrameRoot::new();
        root.active_view = ActiveView::Logs;
        root.file_queue
            .add_file(FileItem::from_path("task-1", "/tmp/source.mp4", 1));
        for index in 0..line_count {
            root.conversion_events.apply_conversion_event(
                &mut root.file_queue,
                ConversionEvent::log("task-1", format!("line {index}")),
            );
        }
        root.conversion_events
            .ensure_selected_log_file(&root.file_queue);
        root
    }

    #[test]
    fn should_follow_logs_tail_after_scroll_position_keeps_non_scrollable_lists_pinned() {
        assert!(should_follow_logs_tail_after_scroll_position(None));
    }

    #[test]
    fn should_follow_logs_tail_after_scroll_position_disables_tail_when_user_left_end() {
        assert!(!should_follow_logs_tail_after_scroll_position(Some(false)));
    }

    #[test]
    fn update_log_scroll_target_resets_tail_following_for_new_selected_file() {
        let mut root = root_with_logs(2);
        root.logs_follow_tail = false;

        root.update_log_scroll_target();

        assert!(root.logs_follow_tail);
    }

    #[test]
    fn sync_logs_follow_tail_after_user_scroll_updates_follow_state() {
        let mut root = FrameRoot::new();
        root.logs_follow_tail = false;

        assert!(root.sync_logs_follow_tail_after_user_scroll());

        assert!(root.logs_follow_tail);
    }

    #[test]
    fn scroll_selected_log_to_bottom_reenables_follow_tail() {
        let mut root = root_with_logs(2);
        root.logs_follow_tail = false;

        assert!(root.scroll_selected_log_to_bottom());

        assert!(root.logs_follow_tail);
    }
}
