use crate::app::prompts::tests::{answer_prompt, pending_prompt};
use crate::app::root::FrameRoot;
use crate::appearance::{ColorTheme, ScalePreset};
use crate::file_queue::{FileItem, FileStatus};
use crate::settings::{SourceKind, SourceMetadata};
use gpui::{Entity, Global, Modifiers, TestAppContext, VisualTestContext, px, size};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "frame-result-actions-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Request {
    action: &'static str,
    path: PathBuf,
}

#[derive(Default)]
struct ResultRequests(RefCell<Vec<Request>>);
impl Global for ResultRequests {}

pub(super) fn record_result_action(cx: &gpui::App, action: &'static str, path: &Path) {
    if let Some(requests) = cx.try_global::<ResultRequests>() {
        requests.0.borrow_mut().push(Request {
            action,
            path: path.to_path_buf(),
        });
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn window<'a>(
    app: &'a mut TestAppContext,
    output: &Path,
    theme: ColorTheme,
    scale: ScalePreset,
) -> (Entity<FrameRoot>, &'a mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Completed result test");
        cx.set_reduce_motion(true);
        cx.set_global(ResultRequests::default());
        let mut root = FrameRoot::new();
        root.appearance.color_theme = theme;
        root.appearance.ui_scale = scale;
        let mut file = FileItem::from_path("source", "/tmp/original-source.mov", 100);
        file.status = FileStatus::Completed;
        file.output_path = Some(output.to_string_lossy().into_owned());
        file.output_name = "renamed-after-export".into();
        file.config.container = "avi".into();
        root.file_queue.add_file(file);
        root.default_output_directory = Some(PathBuf::from("/tmp/changed-output-folder"));
        root.source_metadata.mark_ready(
            "source",
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                video_codec: Some("h264".into()),
                resolution: Some("1920x1080".into()),
                ..SourceMetadata::default()
            },
        );
        let metadata = root.selected_source_metadata_entry();
        root.preview_ui.pending_runtime_key = root
            .selected_preview_runtime_request(&metadata)
            .map(|request| request.key);
        root
    });
    cx.update(|window, _| {
        window.activate();
        window.set_a11y_forced(true);
    });
    cx.simulate_resize(size(px(1200.0), px(800.0)));
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

fn focused_label(cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        tree["gpui_focus"].as_str().and_then(|id| {
            tree["nodes"][id]["aria"]["label"]
                .as_str()
                .map(str::to_string)
        })
    })
}

fn tab_to(cx: &mut VisualTestContext, label: &str) {
    for _ in 0..100 {
        press(cx, "tab");
        if focused_label(cx).as_deref() == Some(label) {
            return;
        }
    }
    panic!("keyboard must reach {label}");
}

#[test]
fn completed_actions_open_the_published_path_and_preserve_existing_controls() {
    let directory = TestDirectory::new();
    let output = directory.path().join("résultat final (2).mkv");
    std::fs::write(&output, b"completed test output").unwrap();
    for theme in ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (root, cx) = window(&mut app, &output, theme, scale);
            let open = cx
                .debug_bounds("file-row-action-open-result-source")
                .expect("Open result control");
            let reveal = cx
                .debug_bounds("file-row-action-reveal-result-source")
                .expect("Show result in folder control");
            let reconvert = cx
                .debug_bounds("file-row-action-reconvert-source")
                .expect("Convert again remains");
            let remove = cx
                .debug_bounds("file-row-action-delete-source")
                .expect("Remove file remains");
            assert!(open.right() <= reveal.left());
            assert!(reveal.right() <= reconvert.left());
            assert!(reconvert.right() <= remove.left());
            let row = cx.debug_bounds("file-list-row-source").unwrap();
            for control in [open, reveal, reconvert, remove] {
                assert_eq!(control.intersect(&row), control);
            }
            let before = root.read_with(cx, |root, _| root.file_queue.files().to_vec());
            tab_to(cx, "Open result");
            press(cx, "enter");
            let reveal = cx
                .debug_bounds("file-row-action-reveal-result-source")
                .unwrap();
            cx.simulate_click(reveal.center(), Modifiers::default());
            draw(cx);
            assert_eq!(
                cx.update(|_, cx| cx.global::<ResultRequests>().0.borrow().clone()),
                [
                    Request {
                        action: "open",
                        path: output.clone()
                    },
                    Request {
                        action: "reveal",
                        path: output.clone()
                    },
                ]
            );
            assert_eq!(
                root.read_with(cx, |root, _| root.file_queue.files().to_vec()),
                before
            );
        }
    }
}

#[test]
fn missing_result_reports_its_path_without_changing_completion_or_losing_focus() {
    let directory = TestDirectory::new();
    let output = directory.path().join("removed-result.mkv");
    std::fs::write(&output, b"completed test output").unwrap();
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, &output, ColorTheme::Dark, ScalePreset::Percent100);
    assert!(
        cx.debug_bounds("file-row-action-open-result-source")
            .is_some()
    );
    std::fs::remove_file(&output).unwrap();
    let before = root.read_with(cx, |root, _| root.file_queue.files().to_vec());
    tab_to(cx, "Open result");
    press(cx, "enter");
    let (title, detail) = pending_prompt(cx).expect("visible missing-result error");
    assert_eq!(title, "Result file not found");
    assert!(detail.contains(output.to_str().unwrap()));
    assert!(detail.contains("Convert again"));
    answer_prompt(cx, "Close");
    draw(cx);
    assert_eq!(focused_label(cx).as_deref(), Some("Open result"));
    assert!(cx.update(|_, cx| cx.global::<ResultRequests>().0.borrow().is_empty()));
    assert_eq!(
        root.read_with(cx, |root, _| root.file_queue.files().to_vec()),
        before
    );
}

#[test]
fn retry_hides_result_controls_and_returns_focus_to_available_row_action() {
    let directory = TestDirectory::new();
    let output = directory.path().join("completed.mkv");
    std::fs::write(&output, b"completed test output").unwrap();
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, &output, ColorTheme::Dark, ScalePreset::Percent100);
    assert!(
        cx.debug_bounds("file-row-action-open-result-source")
            .is_some()
    );
    tab_to(cx, "Show result in folder");
    root.update(cx, |root, cx| {
        assert!(root.prepare_file_for_reconversion("source"));
        cx.notify();
    });
    draw(cx);
    assert!(
        cx.debug_bounds("file-row-action-open-result-source")
            .is_none()
    );
    assert!(
        cx.debug_bounds("file-row-action-reveal-result-source")
            .is_none()
    );
    assert_eq!(focused_label(cx).as_deref(), Some("Remove file"));
    assert!(cx.update(|_, cx| cx.global::<ResultRequests>().0.borrow().is_empty()));
}
