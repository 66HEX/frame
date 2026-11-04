use super::*;

#[test]
fn controller_tracks_registered_process_pid() {
    let controller = ConversionProcessController::default();

    controller
        .register_started_process("task-1", 0)
        .expect("pid registration should succeed");

    assert_eq!(controller.active_pid("task-1"), Some(0));
}

#[test]
fn controller_tracks_registered_process_start_time() {
    let controller = ConversionProcessController::default();
    let pid = std::process::id();

    controller
        .register_started_process("task-1", pid)
        .expect("pid registration should succeed");

    assert!(
        controller
            .active_start_time("task-1")
            .is_some_and(|time| time > 0),
        "registered process should keep a start-time guard"
    );
}

#[test]
fn controller_uses_shared_default_max_concurrency() {
    let controller = ConversionProcessController::default();

    assert_eq!(
        controller
            .current_max_concurrency()
            .expect("default max concurrency should be readable"),
        DEFAULT_MAX_CONCURRENCY
    );
}

#[test]
fn controller_update_max_concurrency_rejects_zero() {
    let controller = ConversionProcessController::default();

    let error = controller
        .update_max_concurrency(0)
        .expect_err("zero concurrency should be rejected");

    assert!(error.to_string().contains("at least 1"));
}

#[test]
fn controller_update_max_concurrency_stores_live_limit() {
    let controller = ConversionProcessController::default();

    controller
        .update_max_concurrency(4)
        .expect("valid max concurrency should be stored");

    assert_eq!(
        controller
            .current_max_concurrency()
            .expect("max concurrency should be readable"),
        4
    );
}

#[test]
fn controller_finish_task_reports_cancelled_state() {
    let controller = ConversionProcessController::default();
    controller
        .register_started_process("task-1", 0)
        .expect("pid registration should succeed");
    controller
        .cancel_task("task-1")
        .expect("cancelling pid zero should not signal an OS process");

    let was_cancelled = controller
        .finish_task("task-1")
        .expect("finishing task should succeed");

    assert!(was_cancelled);
}

#[test]
fn controller_register_started_process_reports_pre_cancelled_task() {
    let controller = ConversionProcessController::default();
    controller
        .cancel_task("task-1")
        .expect("pre-cancel should succeed without an active process");

    let was_cancelled = controller
        .register_started_process("task-1", 0)
        .expect("pid registration should succeed");

    assert!(was_cancelled);
    assert!(
        controller
            .finish_task("task-1")
            .expect("finishing task should clean process state")
    );
    assert_eq!(controller.active_pid("task-1"), None);
}

#[test]
fn run_conversion_task_with_control_emits_cancelled_when_cancelled_before_validation() {
    let controller = ConversionProcessController::default();
    controller
        .cancel_task("task-1")
        .expect("pre-cancel should succeed without an active process");
    let task = ConversionTask {
        id: "task-1".to_string(),
        file_path: "/definitely/missing.mov".to_string(),
        output_directory: "/tmp/frame-output".to_string(),
        output_name: None,
        config: core_config_from_gpui(&GpuiConversionConfig::default()),
    };
    let mut events = Vec::new();

    let result = run_conversion_task_with_control(task, &controller, &mut |event| {
        events.push(event);
    });

    assert!(result.is_ok());
    assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
}

#[test]
fn run_conversion_batch_with_control_accepts_empty_batches() {
    let controller = ConversionProcessController::default();
    let mut events = Vec::new();

    let result = run_conversion_batch_with_control(Vec::new(), &controller, |event| {
        events.push(event);
    });

    assert!(result.is_ok());
    assert!(events.is_empty());
}

#[test]
fn next_batch_launch_count_respects_live_concurrency_limit() {
    assert_eq!(next_batch_launch_count(5, 1, 2), 1);
    assert_eq!(next_batch_launch_count(5, 2, 2), 0);
    assert_eq!(next_batch_launch_count(1, 0, 4), 1);
}
