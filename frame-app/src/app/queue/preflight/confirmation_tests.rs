use super::*;
use crate::app::prompts::tests::{answer_prompt, pending_prompt};
use crate::file_queue::FileItem;
use gpui::{Entity, TestAppContext, VisualTestContext};
use std::fs;
use std::time::Duration;

fn window(app: &mut TestAppContext) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Preflight confirmation");
        cx.set_reduce_motion(true);
        FrameRoot::new()
    });
    cx.update(|window, _| {
        window.activate();
        window.set_a11y_forced(true);
    });
    (root, cx)
}

#[test]
fn confirmation_ignores_stale_cancelled_and_removed_files() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app);
    cx.update(|window, cx| {
        root.update(cx, |root, cx| {
            root.default_output_directory = Some("exports".into());
            for id in ["removed", "cancelled", "active"] {
                root.file_queue.add_file(FileItem::from_path(id, id, 0));
            }
            let tasks = root.queue_selected_conversion_tasks();
            let mut batch = PlannedBatch {
                tasks,
                ..PlannedBatch::default()
            };
            for id in ["removed", "cancelled", "active"] {
                batch
                    .fallbacks
                    .insert(id.into(), format!("Fallback for {id}"));
            }
            root.is_processing = true;
            assert!(root.remove_file_from_queue("removed"));
            assert!(root.cancel_conversion_task("cancelled"));
            root.confirm_planned_batch(0, batch, window, cx);
        });
    });
    let (_, description) = pending_prompt(cx).expect("remaining queued file requires consent");
    assert_eq!(description, "Fallback for active");
    root.update(cx, |root, _| {
        assert_eq!(
            root.file_queue.file_by_id("cancelled").unwrap().status,
            FileStatus::Idle
        );
        assert!(!root.conversion_processes.is_cancelled("cancelled"));
        assert!(!root.conversion_processes.is_cancelled("removed"));
        // Replace the batch while the old confirmation is open.
        root.conversion_batch_generation += 1;
        assert!(root.cancel_conversion_task("active"));
    });
    answer_prompt(cx, "Continue export");
    root.read_with(cx, |root, _| {
        assert_eq!(
            root.file_queue.file_by_id("active").unwrap().status,
            FileStatus::Cancelling
        );
        assert!(root.conversion_processes.is_cancelled("active"));
    });
}

#[test]
#[ignore = "requires the built media worker"]
fn hardware_preflight_cancel_and_continue_control_real_batch_publication() {
    let dir = tempfile::tempdir().unwrap();
    let exports = dir.path().join("exports");
    fs::create_dir(&exports).unwrap();
    let mut fixture = b"YUV4MPEG2 W160 H96 F24:1 Ip A1:1 C420jpeg\n".to_vec();
    for luma in [64, 128, 192] {
        fixture.extend_from_slice(b"FRAME\n");
        fixture.extend(std::iter::repeat_n(luma, 160 * 96));
        fixture.extend(std::iter::repeat_n(128, 160 * 96 / 2));
    }
    let encoder = if cfg!(target_os = "macos") {
        "h264_nvenc"
    } else {
        "h264_videotoolbox"
    };
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app);
    root.update(cx, |root, _| {
        root.default_output_directory = Some(exports.clone());
        for id in ["export", "cancelled", "removed", "missing"] {
            let path = dir.path().join(format!("{id}.y4m"));
            if id != "missing" {
                fs::write(&path, &fixture).unwrap();
            }
            let mut file = FileItem::from_os_path(id, &path);
            "mp4".clone_into(&mut file.config.container);
            encoder.clone_into(&mut file.config.video_codec);
            "bitrate".clone_into(&mut file.config.video_bitrate_mode);
            "1200".clone_into(&mut file.config.video_bitrate);
            root.file_queue.add_file(file);
        }
    });
    for action in ["Cancel batch", "Continue export"] {
        cx.update(|window, cx| {
            root.update(cx, |root, cx| root.start_selected_conversions(window, cx));
        });
        cx.run_until_parked();
        let (_, description) = pending_prompt(cx).expect("software encoder requires confirmation");
        assert!(
            description.contains("export.y4m")
                && description.contains("libx264")
                && description.contains("1200")
        );
        assert!(!description.contains("missing.y4m"));
        assert_eq!(fs::read_dir(&exports).unwrap().count(), 0);
        root.update(cx, |root, _| {
            assert_eq!(
                root.file_queue.file_by_id("missing").unwrap().status,
                FileStatus::Error
            );
            for id in ["export", "cancelled", "removed"] {
                assert_eq!(
                    root.file_queue.file_by_id(id).unwrap().status,
                    FileStatus::Queued
                );
            }
            if action == "Continue export" {
                assert!(root.cancel_conversion_task("cancelled"));
                assert!(root.remove_file_from_queue("removed"));
            }
        });
        answer_prompt(cx, action);
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        root.read_with(cx, |root, _| {
            assert!(!root.is_processing);
            assert!(root.active_conversion_task_ids.is_empty());
            let file = root.file_queue.file_by_id("export").unwrap();
            assert_eq!(file.config.video_codec, encoder);
            assert_eq!(
                root.file_queue.file_by_id("cancelled").unwrap().status,
                FileStatus::Idle
            );
            assert!(!root.conversion_processes.is_cancelled("cancelled"));
            if action == "Cancel batch" {
                assert_eq!(file.status, FileStatus::Idle);
                assert_eq!(fs::read_dir(&exports).unwrap().count(), 0);
            } else {
                assert_eq!(
                    file.status,
                    FileStatus::Completed,
                    "{:?}",
                    file.conversion_error
                );
                let meta = crate::source_metadata::probe_source_metadata(
                    file.completed_output_path().unwrap(),
                )
                .unwrap();
                assert_eq!(meta.video_codec.as_deref(), Some("h264"));
                assert_eq!(fs::read_dir(&exports).unwrap().count(), 1);
                assert!(root.file_queue.file_by_id("removed").is_none());
                assert!(!root.conversion_processes.is_cancelled("removed"));
            }
        });
    }
}
