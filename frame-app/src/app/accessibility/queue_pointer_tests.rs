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
