use super::*;
use crate::ActiveView;
use crate::appearance::ScalePreset;
use crate::file_queue::{FileItem, FileQueue};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, px, size};

fn prepared_snapshot() -> RecoverySnapshot {
    let mut queue = FileQueue::new();
    queue.add_file(FileItem::from_path(
        "file-1",
        "/missing/recovered-source.mp4",
        0,
    ));
    RecoverySnapshot::capture(&queue, ActiveView::Workspace, None, "previous", 1, 42).unwrap()
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn window(
    app: &mut TestAppContext,
    scale: ScalePreset,
) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Recovery dialog test");
        cx.set_reduce_motion(true);
        let mut root = FrameRoot::new();
        root.appearance.ui_scale = scale;
        root.workspace_recovery.pending = Some(prepared_snapshot());
        root
    });
    cx.update(|window, _| {
        window.activate();
        window.set_a11y_forced(true);
    });
    cx.simulate_resize(size(px(1280.0), px(960.0)));
    draw(cx);
    (root, cx)
}

fn press(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    draw(cx);
}

fn focus_label(cx: &mut VisualTestContext) -> String {
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        let id = tree["gpui_focus"]
            .as_str()
            .expect("focused accessible node");
        tree["nodes"][id]["aria"]["label"]
            .as_str()
            .unwrap()
            .to_owned()
    })
}

#[test]
fn recovery_prompt_has_safe_focus_trap_and_escape_defers_without_starting() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = window(&mut app, scale);
        assert!(cx.debug_bounds("recovery-dialog").is_some());
        assert_eq!(focus_label(cx), "Restore queue");
        let bounds = ["recovery-restore", "recovery-discard", "recovery-later"]
            .map(|id| cx.debug_bounds(id).unwrap());
        for bounds in bounds {
            assert!(bounds.left() >= px(0.0) && bounds.right() <= px(1280.0));
            assert!(bounds.top() >= px(0.0) && bounds.bottom() <= px(960.0));
            assert!(bounds.size.height >= px(24.0));
        }
        for pair in bounds.windows(2) {
            assert!(pair[0].right() <= pair[1].left() || pair[0].bottom() <= pair[1].top());
        }
        for expected in ["Discard queue", "Later", "Restore queue"] {
            press(cx, "tab");
            assert_eq!(focus_label(cx), expected);
        }
        press(cx, "shift-tab");
        assert_eq!(focus_label(cx), "Later");
        cx.update(|window, _| {
            let tree = window.a11y_tree().unwrap();
            let (_, dialog) = tree
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some("Restore prepared queue?"))
                .unwrap();
            assert_eq!(dialog.role(), gpui::Role::Dialog);
            assert!(
                dialog
                    .description()
                    .unwrap()
                    .contains("Conversions will not start automatically")
            );
            assert!(dialog.description().unwrap().starts_with("1 prepared file can be restored."));
            assert!(dialog.description().unwrap().contains("Later keeps this queue for the next start; new changes will not be saved for recovery."));
        });
        press(cx, "escape");
        assert!(cx.debug_bounds("recovery-dialog").is_none());
        root.read_with(cx, |root, _| {
            assert!(root.recovery_prompt_count().is_none());
            assert!(!root.workspace_recovery.ready);
            assert!(root.file_queue.files().is_empty());
            assert!(!root.is_processing);
        });
    }
}

#[test]
fn invalid_recovery_notice_offers_only_continue_and_keeps_writer_disabled() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ScalePreset::Percent100);
    root.update(cx, |root, cx| {
        root.workspace_recovery.pending = None;
        root.workspace_recovery.notice =
            Some("The saved queue could not be read. Its file has been kept.".into());
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("recovery-continue").is_some());
    assert!(cx.debug_bounds("recovery-discard").is_none());
    assert!(cx.debug_bounds("recovery-restore").is_none());
    assert_eq!(focus_label(cx), "Continue");
    press(cx, "space");
    assert!(cx.debug_bounds("recovery-dialog").is_none());
    root.read_with(cx, |root, _| assert!(!root.workspace_recovery.ready));
}

#[test]
fn loading_recovery_blocks_background_clicks_and_has_no_destructive_action() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ScalePreset::Percent100);
    root.update(cx, |root, cx| {
        root.workspace_recovery.pending = None;
        root.workspace_recovery.loading = true;
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("recovery-dialog").is_some());
    assert!(cx.debug_bounds("recovery-discard").is_none());
    assert!(cx.debug_bounds("recovery-continue").is_none());
    let bounds = cx.debug_bounds("welcome-open-file").unwrap();
    cx.simulate_click(bounds.center(), Modifiers::default());
    press(cx, "escape");
    assert!(cx.debug_bounds("recovery-dialog").is_some());
    root.read_with(cx, |root, _| {
        assert!(root.recovery_loading());
        assert!(root.file_queue.files().is_empty());
    });
}

#[test]
fn restore_and_discard_keyboard_actions_resolve_the_real_store_without_starting_conversions() {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    for restore in [true, false] {
        let directory = std::env::temp_dir().join(format!(
            "frame-recovery-ui-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let settings = directory.join("settings.json");
        {
            let mut store = RecoveryStore::open(&settings).unwrap();
            store.load().unwrap();
            store.resolve_existing().unwrap();
            store.save(&prepared_snapshot()).unwrap();
        }
        let mut store = RecoveryStore::open(&settings).unwrap();
        let saved = store.load().unwrap();
        {
            let mut app = TestAppContext::single();
            let (root, cx) = window(&mut app, ScalePreset::Percent100);
            root.update(cx, |root, cx| {
                root.workspace_recovery = WorkspaceRecovery::default();
                root.accept_recovery_store(store, saved, false).unwrap();
                cx.notify();
            });
            draw(cx);
            if !restore {
                press(cx, "tab");
            }
            assert_eq!(
                focus_label(cx),
                if restore {
                    "Restore queue"
                } else {
                    "Discard queue"
                }
            );
            press(cx, "enter");
            assert!(cx.debug_bounds("recovery-dialog").is_none());
            root.update(cx, |root, _| {
                assert_eq!(root.file_queue.files().len(), usize::from(restore));
                assert!(!root.is_processing);
                assert!(root.active_conversion_task_ids.is_empty());
                assert!(root.recovery_prompt_count().is_none());
                root.prepare_recovery_write()
                    .unwrap()
                    .unwrap()
                    .save()
                    .unwrap();
            });
            cx.update(|window, cx| {
                window.remove_window();
                cx.defer(|_| drop(root));
            });
            cx.run_until_parked();
        }
        let mut reopened = RecoveryStore::open(&settings).unwrap();
        assert_eq!(
            reopened.load().unwrap().unwrap().file_count(),
            usize::from(restore)
        );
        drop(reopened);
        let stored: RecoverySnapshot =
            serde_json::from_slice(&fs::read(directory.join("workspace-recovery.json")).unwrap())
                .unwrap();
        assert_eq!(stored.file_count(), usize::from(restore));
        fs::remove_dir_all(directory).unwrap();
    }
}
