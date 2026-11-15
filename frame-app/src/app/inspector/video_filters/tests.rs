use crate::app::root::FrameRoot;
use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn focus_stabilization_with_tab(cx: &mut VisualTestContext) {
    for _ in 0..150 {
        let reached = cx.update(|window, _| {
            let tree: serde_json::Value =
                serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
            tree["gpui_focus"]
                .as_str()
                .is_some_and(|id| tree["nodes"][id]["aria"]["label"] == "Stabilize video")
        });
        if reached {
            return;
        }
        cx.simulate_keystrokes("tab");
        draw(cx);
    }
    panic!("stabilization checkbox must be reachable with Tab");
}

#[test]
fn stabilization_control_requires_runtime_filter_and_can_clear_saved_setting() {
    for (available, checked) in [(true, false), (false, false), (false, true)] {
        let mut app = TestAppContext::single();
        let (root, cx) = app.add_window_view(|_, cx| {
            crate::app::init_app(cx, "Stabilization test");
            cx.set_reduce_motion(true);
            let mut root = FrameRoot::new();
            root.apply_settings_video_filters_fixture();
            root.available_filters.deshake = available;
            root.file_queue
                .selected_file_mut()
                .unwrap()
                .config
                .video_filters
                .stabilization_enabled = checked;
            root
        });
        cx.update(|window, _| {
            window.activate();
            window.set_a11y_forced(true);
        });
        cx.simulate_resize(size(px(1600.0), px(4000.0)));
        draw(cx);
        cx.update(|window, _| {
            let tree = window.a11y_tree().unwrap();
            let (_, checkbox) = tree
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some("Stabilize video"))
                .expect("stabilization checkbox");
            assert_eq!(checkbox.role(), gpui::Role::CheckBox);
            assert_eq!(checkbox.is_disabled(), !available && !checked);
        });
        let bounds = cx
            .debug_bounds("settings-video-stabilization-toggle")
            .unwrap();
        if available || checked {
            focus_stabilization_with_tab(cx);
            cx.simulate_keystrokes("space");
        } else {
            cx.simulate_click(bounds.center(), Modifiers::default());
        }
        draw(cx);
        root.read_with(cx, |root, _| {
            assert_eq!(
                root.file_queue
                    .selected_file()
                    .unwrap()
                    .config
                    .video_filters
                    .stabilization_enabled,
                available && !checked
            );
        });
    }
}

#[test]
fn stabilization_control_is_hidden_for_still_images() {
    let mut app = TestAppContext::single();
    let (_, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Image filter test");
        let mut root = FrameRoot::new();
        root.apply_settings_images_fixture();
        root.settings_ui
            .expanded_output_sections
            .insert(crate::settings::OutputSection::VideoFilters);
        root.available_filters.deshake = true;
        root
    });
    cx.update(|window, _| window.set_a11y_forced(true));
    cx.simulate_resize(size(px(1600.0), px(4000.0)));
    draw(cx);
    cx.update(|window, _| {
        assert!(
            !window
                .a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .any(|(_, node)| node.label() == Some("Stabilize video"))
        );
    });
}
