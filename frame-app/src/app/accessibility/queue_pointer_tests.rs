//! Focus entry must not move a virtual row between pointer down and click.
use super::*;
use gpui::{Bounds, ScrollDelta, ScrollWheelEvent};

fn queue_window(
    app: &mut TestAppContext,
    scale: ScalePreset,
) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (root, cx) = test_window(app, VisualFixture::WorkspaceLargeQueue, scale);
    cx.simulate_resize(size(
        px(1200.0 * scale.factor()),
        px(800.0 * scale.factor()),
    ));
    cx.update(|window, cx| {
        window.set_a11y_forced(true);
        window.blur(cx);
    });
    draw(cx);
    (root, cx)
}

fn wheel(cx: &mut VisualTestContext, delta: f32) {
    let position = cx.debug_bounds("file-list-body").unwrap().center();
    cx.simulate_mouse_move(position, None, Modifiers::default());
    cx.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(delta))),
        ..Default::default()
    });
    draw(cx);
}

fn target_bounds(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.debug_bounds("file-list-row-fixture-queue-042")
        .expect("the target row must remain mounted")
}

fn selected(root: &Entity<FrameRoot>, cx: &VisualTestContext) -> String {
    root.read_with(cx, |root, _| {
        root.file_queue.selected_file_id().unwrap().to_owned()
    })
}

fn pointer_down(cx: &mut VisualTestContext, position: gpui::Point<Pixels>) {
    cx.simulate_mouse_move(position, None, Modifiers::default());
    cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    draw(cx);
}

fn pointer_up(cx: &mut VisualTestContext, position: gpui::Point<Pixels>) {
    cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    draw(cx);
}

fn a11y_focus(cx: &mut VisualTestContext, label: &str) {
    cx.update(|window, cx| {
        let id = window
            .a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some(label))
            .unwrap()
            .0;
        window.perform_a11y_action(id, gpui::AccessibleAction::Focus, None, cx);
    });
    draw(cx);
}

fn checkbox_center(cx: &mut VisualTestContext) -> gpui::Point<Pixels> {
    cx.update(|window, _| {
        let tree = window.a11y_tree().unwrap();
        let id = tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.label() == Some("Select render_queue_item_042.mov for conversion")
            })
            .unwrap()
            .0;
        window.a11y_node_bounds(id).unwrap().center()
    })
}

fn assert_first_row_visible(cx: &mut VisualTestContext) {
    let row = cx.debug_bounds("file-list-row-fixture-preview").unwrap();
    let viewport = cx.debug_bounds("file-list-body").unwrap();
    assert_eq!(row.intersect(&viewport), row);
}

#[test]
fn split_pointer_entry_keeps_the_scrolled_row_until_click_in_both_directions_and_scales() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        for reverse in [false, true] {
            let mut app = TestAppContext::single();
            let (root, cx) = queue_window(&mut app, scale);
            if reverse {
                root.update(cx, |root, cx| {
                    root.file_queue
                        .select_file(Some("fixture-queue-499".into()));
                    cx.notify();
                });
                draw(cx);
                wheel(cx, -8000.0 * scale.factor());
                wheel(cx, 6400.0 * scale.factor());
            } else {
                wheel(cx, -1600.0 * scale.factor());
            }
            let id = "fixture-queue-042";
            let bounds = target_bounds(cx);
            let viewport = cx.debug_bounds("file-list-body").unwrap();
            assert_eq!(bounds.intersect(&viewport), bounds);
            let before = selected(&root, cx);
            let batch = root.read_with(cx, |root, _| root.file_queue.batch_selection_state());
            pointer_down(cx, bounds.center());
            assert_eq!(
                keyboard_tests::focused_node(cx)["aria"]["label"],
                "File queue"
            );
            assert_eq!(
                selected(&root, cx),
                before,
                "selection belongs to the click"
            );
            assert_eq!(
                target_bounds(cx),
                bounds,
                "focus must not scroll before MouseUp"
            );
            pointer_up(cx, bounds.center());
            assert_eq!(selected(&root, cx), id);
            assert_eq!(target_bounds(cx), bounds);
            assert_eq!(
                root.read_with(cx, |root, _| root.file_queue.batch_selection_state()),
                batch
            );
        }
    }
}

#[test]
fn cancelled_pointer_entry_preserves_selection_and_an_already_focused_click_still_works() {
    let mut app = TestAppContext::single();
    let (root, cx) = queue_window(&mut app, ScalePreset::Percent100);
    wheel(cx, -1600.0);
    let bounds = target_bounds(cx);
    let before = selected(&root, cx);
    pointer_down(cx, bounds.center());
    pointer_up(cx, point(px(0.0), px(0.0)));
    assert_eq!(selected(&root, cx), before);
    assert_eq!(target_bounds(cx), bounds);
    pointer_down(cx, bounds.center());
    assert_eq!(selected(&root, cx), before);
    pointer_up(cx, bounds.center());
    assert_eq!(selected(&root, cx), "fixture-queue-042");
    assert_eq!(target_bounds(cx), bounds);
    wheel(cx, -800.0);
    assert!(cx.debug_bounds("file-list-row-fixture-queue-042").is_none());
    assert_eq!(selected(&root, cx), "fixture-queue-042");
}

#[test]
fn child_controls_preserve_the_viewport_and_do_not_suppress_later_a11y_focus() {
    for checkbox in [true, false] {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (root, cx) = queue_window(&mut app, scale);
            wheel(cx, -1600.0 * scale.factor());
            let bounds = target_bounds(cx);
            let before = selected(&root, cx);
            let position = if checkbox {
                checkbox_center(cx)
            } else {
                cx.debug_bounds("file-row-action-delete-fixture-queue-042")
                    .unwrap()
                    .center()
            };
            assert!(
                bounds.contains(&position),
                "the child hit point must be inside the target row"
            );
            pointer_down(cx, position);
            assert_eq!(target_bounds(cx), bounds);
            assert_eq!(selected(&root, cx), before);
            cx.update(|window, _| {
                let tree: serde_json::Value =
                    serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
                assert!(
                    tree["gpui_focus"]
                        .as_str()
                        .is_none_or(|id| { tree["nodes"][id]["aria"]["label"] != "File queue" })
                );
            });
            if checkbox {
                pointer_up(cx, position);
                root.read_with(cx, |root, _| {
                    assert_eq!(
                        root.file_queue.selected_file_id(),
                        Some("fixture-queue-042")
                    );
                    assert!(
                        !root
                            .file_queue
                            .file_by_id("fixture-queue-042")
                            .unwrap()
                            .is_selected_for_conversion
                    );
                    assert_eq!(root.file_queue.selected_count(), 499);
                });
                // Move the viewport away from the new selection before Focus.
                wheel(cx, 1600.0 * scale.factor());
                a11y_focus(cx, "File queue");
                let row = target_bounds(cx);
                let viewport = cx.debug_bounds("file-list-body").unwrap();
                assert_eq!(row.intersect(&viewport), row);
            } else {
                // Cancel the destructive action; its MouseDown must not leak a marker.
                pointer_up(cx, point(px(0.0), px(0.0)));
                assert_eq!(
                    root.read_with(cx, |root, _| root.file_queue.files().len()),
                    500
                );
                a11y_focus(cx, "File queue");
                assert_first_row_visible(cx);
            }
        }
    }
}

#[test]
fn keyboard_programmatic_and_a11y_focus_after_wheel_still_reveal_selection() {
    for origin in ["tab", "programmatic", "a11y"] {
        let mut app = TestAppContext::single();
        let (root, cx) = queue_window(&mut app, ScalePreset::Percent100);
        a11y_focus(cx, "File queue");
        let list_focus = focus(cx);
        cx.update(gpui::Window::blur);
        draw(cx);
        wheel(cx, -1600.0);
        assert!(cx.debug_bounds("file-list-row-fixture-preview").is_none());
        let before = selected(&root, cx);
        match origin {
            "tab" => {
                a11y_focus(cx, "Add source");
                for _ in 0..40 {
                    press(cx, "tab");
                    if keyboard_tests::focused_node(cx)["aria"]["label"] == "File queue" {
                        break;
                    }
                }
            }
            "programmatic" => {
                cx.update(|window, cx| list_focus.focus(window, cx));
                draw(cx);
            }
            _ => a11y_focus(cx, "File queue"),
        }
        assert_eq!(
            keyboard_tests::focused_node(cx)["aria"]["label"],
            "File queue"
        );
        assert_eq!(selected(&root, cx), before);
        assert_first_row_visible(cx);
    }
}

#[test]
fn pointer_marker_is_consumed_even_when_focus_leaves_and_returns_between_frames() {
    let mut app = TestAppContext::single();
    let (_, cx) = queue_window(&mut app, ScalePreset::Percent100);
    a11y_focus(cx, "File queue");
    wheel(cx, -1600.0);
    let bounds = target_bounds(cx);
    // The previous render still records list focus: entered will be false.
    cx.update(gpui::Window::blur);
    pointer_down(cx, bounds.center());
    assert_eq!(target_bounds(cx), bounds);
    pointer_up(cx, point(px(0.0), px(0.0)));
    cx.update(gpui::Window::blur);
    draw(cx);
    a11y_focus(cx, "File queue");
    assert_first_row_visible(cx);
}

#[test]
fn first_scrollbar_drag_from_an_unfocused_queue_keeps_scrolling() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = queue_window(&mut app, scale);
        wheel(cx, -1600.0 * scale.factor());
        let before = selected(&root, cx);
        let row = target_bounds(cx);
        let scrollbar = cx.debug_bounds("file-list-scrollbar").unwrap();
        let from = point(
            scrollbar.center().x,
            scrollbar.top() + px(20.0 * scale.factor()),
        );
        pointer_down(cx, from);
        assert_eq!(
            target_bounds(cx),
            row,
            "first scrollbar press must not snap to selection"
        );
        let to = point(from.x, scrollbar.top() + scrollbar.size.height * 0.75);
        for position in [
            from + point(px(0.0), px(8.0)),
            to,
            to + point(px(0.0), px(2.0)),
        ] {
            cx.simulate_mouse_move(position, MouseButton::Left, Modifiers::default());
            draw(cx);
        }
        assert!(cx.update(|_, cx| cx.has_active_drag()));
        assert!(cx.debug_bounds("file-list-row-fixture-queue-042").is_none());
        assert!(
            root.read_with(cx, |root, _| root
                .file_list_scroll_handle
                .0
                .borrow()
                .base_handle
                .offset()
                .y)
                < px(-1600.0 * scale.factor())
        );
        pointer_up(cx, to);
        assert_eq!(selected(&root, cx), before);
    }
}

#[test]
fn header_and_row_columns_align_horizontally_across_scales_and_widths() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (_root, cx) = queue_window(&mut app, scale);

        for width in [960.0_f32, 1200.0, 1440.0] {
            cx.simulate_resize(size(px(width * scale.factor()), px(800.0 * scale.factor())));
            draw(cx);

            for (column, header_selector, row_selector) in [
                (
                    "checkbox",
                    "file-list-header-col-checkbox",
                    "file-list-row-fixture-preview-col-checkbox",
                ),
                (
                    "name",
                    "file-list-header-col-name",
                    "file-list-row-fixture-preview-col-name",
                ),
                (
                    "size",
                    "file-list-header-col-size",
                    "file-list-row-fixture-preview-col-size",
                ),
                (
                    "target",
                    "file-list-header-col-target",
                    "file-list-row-fixture-preview-col-target",
                ),
                (
                    "state",
                    "file-list-header-col-state",
                    "file-list-row-fixture-preview-col-state",
                ),
                (
                    "actions",
                    "file-list-header-col-actions",
                    "file-list-row-fixture-preview-col-actions",
                ),
            ] {
                let header = cx
                    .debug_bounds(header_selector)
                    .unwrap_or_else(|| panic!("missing header bounds for column `{column}`"));
                let row = cx
                    .debug_bounds(row_selector)
                    .unwrap_or_else(|| panic!("missing row bounds for column `{column}`"));

                assert!(
                    (header.left() - row.left()).abs() <= px(0.5),
                    "header and row left edges must align for every column"
                );
                assert!(
                    (header.right() - row.right()).abs() <= px(0.5),
                    "header and row right edges must align for every column"
                );
            }
        }
    }
}

const STATS_TRIGGER: &str = "file-row-action-stats-fixture-preview";
const STATS_DIALOG: &str = "encoding-stats-dialog";
const STATS_CLOSE: &str = "encoding-stats-close";

fn workspace_regions(cx: &mut VisualTestContext) -> [Bounds<Pixels>; 4] {
    [
        "preview-viewport",
        "file-list-header-col-name",
        "file-list-body",
        "file-list-row-fixture-preview",
    ]
    .map(|id| {
        cx.debug_bounds(id)
            .unwrap_or_else(|| panic!("missing {id}"))
    })
}

#[test]
fn encoding_stats_never_resize_the_preview_or_queue_during_conversion() {
    use crate::file_queue::FileStatus;
    use frame_core::events::ConversionEvent;
    use frame_core::types::{EncodingStats, ProgressPayload};
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = test_window(&mut app, VisualFixture::PreviewReady, scale);
        cx.simulate_resize(size(px(1200.0), px(800.0)));
        draw(cx);
        let idle = workspace_regions(cx);
        assert_first_row_visible(cx);
        assert!(cx.debug_bounds(STATS_TRIGGER).is_none());

        root.update(cx, |root, cx| {
            root.file_queue
                .update_status("fixture-preview", FileStatus::Queued, 0);
            root.apply_conversion_event(ConversionEvent::started("fixture-preview"));
            root.apply_conversion_event(ConversionEvent::progress("fixture-preview", 42.0));
            cx.notify();
        });
        draw(cx);
        assert_eq!(workspace_regions(cx), idle);
        assert!(cx.debug_bounds("encoding-stats").is_none());

        click(cx, STATS_TRIGGER);
        let dialog = cx
            .debug_bounds(STATS_DIALOG)
            .expect("opened statistics dialog");
        assert!(dialog.top() >= px(0.0) && dialog.left() >= px(0.0));
        assert!(dialog.bottom() <= px(800.0) && dialog.right() <= px(1200.0));
        assert_eq!(workspace_regions(cx), idle);
        let close_button = cx.debug_bounds(STATS_CLOSE).unwrap();
        root.update(cx, |root, cx| {
            root.apply_conversion_event(ConversionEvent::Progress(ProgressPayload {
                id: "fixture-preview".into(),
                progress: 73.0,
                stats: Some(EncodingStats {
                    frame: Some(3712),
                    fps: Some(38.4),
                    size_bytes: Some(100_000_000),
                    bitrate_kbps: Some(5200.0),
                    speed: Some(1.6),
                    media_seconds: 108.0,
                    elapsed_seconds: 67.5,
                    eta_seconds: Some(24.0),
                    duration_seconds: Some(148.0),
                    pass: Some(2),
                }),
            }));
            cx.notify();
        });
        draw(cx);
        assert_eq!(cx.debug_bounds(STATS_DIALOG), Some(dialog));
        assert_eq!(cx.debug_bounds(STATS_CLOSE), Some(close_button));
        assert_eq!(workspace_regions(cx), idle);

        click(cx, STATS_CLOSE);
        assert!(cx.debug_bounds(STATS_DIALOG).is_none());
        assert_eq!(workspace_regions(cx), idle);
        click(cx, STATS_TRIGGER);
        let overlay = cx.debug_bounds("encoding-stats-overlay").unwrap();
        let outside = overlay.origin + point(px(2.0), px(2.0));
        assert!(!dialog.contains(&outside));
        cx.simulate_click(outside, Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds(STATS_DIALOG).is_none());
        assert_eq!(workspace_regions(cx), idle);
        root.update(cx, |root, cx| {
            root.apply_conversion_event(ConversionEvent::cancelled("fixture-preview"));
            cx.notify();
        });
        draw(cx);
        assert!(cx.debug_bounds(STATS_TRIGGER).is_none());
        assert_eq!(workspace_regions(cx), idle);
    }
}

fn assert_fixture_stats(cx: &mut VisualTestContext) {
    cx.update(|window, _| {
        let tree = window.a11y_tree().unwrap();
        assert!(
            tree.nodes
                .iter()
                .any(|(_, node)| node.role() == gpui::Role::Dialog
                    && node.is_modal()
                    && node.label() == Some("Encoding statistics"))
        );
        let (_, group) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Encoding statistics for source_render.mov"))
            .expect("statistics for the requested file");
        let description = group.description().expect("spoken metrics");
        for detail in [
            "ETA 0:16",
            "Media time 0:57 / 1:30",
            "Frame 1389",
            "FPS 48.2",
            "Speed 2.01×",
            "Bitrate 5820 kb/s",
            "Size ",
            "Elapsed 0:28",
        ] {
            assert!(description.contains(detail), "{description}");
        }
        let (_, bar) = tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.role() == gpui::Role::ProgressIndicator
                    && node.label() == Some("Encoding progress")
            })
            .expect("accessible progress indicator");
        assert_eq!(bar.numeric_value(), Some(64.0));
    });
}

#[test]
fn encoding_stats_dialog_exposes_live_metrics_and_restores_keyboard_focus() {
    use frame_core::events::ConversionEvent;
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::WorkspaceEncodingStats,
        ScalePreset::Percent100,
    );
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    assert!(cx.debug_bounds(STATS_DIALOG).is_none());
    a11y_focus(cx, "File queue");
    for _ in 0..20 {
        press(cx, "tab");
        if keyboard_tests::focused_node(cx)["aria"]["label"] == "Show encoding statistics" {
            break;
        }
    }
    assert_eq!(
        keyboard_tests::focused_node(cx)["aria"]["label"],
        "Show encoding statistics"
    );
    let trigger_focus = focus(cx);
    for key in ["enter", "space"] {
        press(cx, key);
        assert!(cx.debug_bounds(STATS_DIALOG).is_some());
        let close_focus = focus(cx);
        assert_ne!(close_focus, trigger_focus);
        for key in ["tab", "shift-tab"] {
            press(cx, key);
            assert_eq!(
                keyboard_tests::focused_node(cx)["aria"]["label"],
                "Encoding statistics for source_render.mov"
            );
            press(cx, key);
            assert_eq!(focus(cx), close_focus);
        }
        assert_fixture_stats(cx);
        press(cx, "escape");
        assert!(cx.debug_bounds(STATS_DIALOG).is_none());
        assert_eq!(focus(cx), trigger_focus);
    }
    press(cx, "enter");
    root.update(cx, |root, cx| {
        root.apply_conversion_event(ConversionEvent::completed(
            "fixture-preview",
            "/tmp/done.mov",
        ));
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds(STATS_DIALOG).is_some());
    assert!(cx.debug_bounds(STATS_TRIGGER).is_none());
    cx.update(|window, _| {
        let (_, group) = window
            .a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Encoding statistics for source_render.mov"))
            .unwrap();
        let description = group.description().unwrap();
        assert!(description.contains("Completed"), "{description}");
        assert!(description.contains("Frame 1389"), "{description}");
    });
    click(cx, STATS_CLOSE);
    assert!(cx.debug_bounds(STATS_DIALOG).is_none());
}

#[test]
fn encoding_stats_open_for_the_requested_queue_row() {
    use crate::file_queue::{FileItem, FileStatus};
    use frame_core::events::ConversionEvent;
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::WorkspaceEncodingStats,
        ScalePreset::Percent100,
    );
    root.update(cx, |root, cx| {
        root.file_queue
            .add_file(FileItem::from_path("other", "/tmp/second.mov", 10));
        root.file_queue
            .update_status("other", FileStatus::Queued, 0);
        root.apply_conversion_event(ConversionEvent::started("other"));
        root.apply_conversion_event(ConversionEvent::progress("other", 17.0));
        cx.notify();
    });
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    assert_eq!(selected(&root, cx), "fixture-preview");
    click(cx, "file-row-action-stats-other");
    assert_eq!(selected(&root, cx), "fixture-preview");
    cx.update(|window, _| {
        let tree = window.a11y_tree().unwrap();
        assert!(
            tree.nodes
                .iter()
                .any(|(_, node)| node.label() == Some("Encoding statistics for second.mov"))
        );
        let (_, bar) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Encoding progress"))
            .unwrap();
        assert_eq!(bar.numeric_value(), Some(17.0));
    });
    root.update(cx, |root, cx| {
        root.apply_conversion_event(ConversionEvent::progress("other", 31.0));
        cx.notify();
    });
    draw(cx);
    cx.update(|window, _| {
        let (_, bar) = window
            .a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Encoding progress"))
            .unwrap();
        assert_eq!(bar.numeric_value(), Some(31.0));
    });
}
