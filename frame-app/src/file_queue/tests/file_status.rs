use super::*;

#[test]
fn locks_settings_while_task_is_active_or_completed() {
    assert!(FileStatus::Converting.locks_settings());
    assert!(FileStatus::Queued.locks_settings());
    assert!(FileStatus::Paused.locks_settings());
    assert!(FileStatus::Cancelling.locks_settings());
    assert!(FileStatus::Completed.locks_settings());
}

#[test]
fn keeps_idle_and_error_files_editable() {
    assert!(!FileStatus::Idle.locks_settings());
    assert!(!FileStatus::Error.locks_settings());
}

#[test]
fn only_idle_and_error_files_are_actionable_for_conversion() {
    assert!(FileStatus::Idle.is_actionable_for_conversion());
    assert!(FileStatus::Error.is_actionable_for_conversion());
    assert!(!FileStatus::Queued.is_actionable_for_conversion());
    assert!(!FileStatus::Converting.is_actionable_for_conversion());
    assert!(!FileStatus::Paused.is_actionable_for_conversion());
    assert!(!FileStatus::Cancelling.is_actionable_for_conversion());
}

#[test]
fn only_settled_files_can_be_removed_from_list() {
    assert!(!FileStatus::Queued.can_be_removed_from_list());
    assert!(!FileStatus::Converting.can_be_removed_from_list());
    assert!(!FileStatus::Paused.can_be_removed_from_list());
    assert!(!FileStatus::Cancelling.can_be_removed_from_list());
    assert!(FileStatus::Idle.can_be_removed_from_list());
    assert!(FileStatus::Completed.can_be_removed_from_list());
    assert!(FileStatus::Error.can_be_removed_from_list());
}
