use super::*;

pub(super) fn focused_node(cx: &mut VisualTestContext) -> serde_json::Value {
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        let focused = tree["gpui_focus"]
            .as_str()
            .expect("focus must have an accessibility node");
        tree["nodes"][focused].clone()
    })
}

fn tab_to_label(cx: &mut VisualTestContext, label: &str) {
    for _ in 0..180 {
        press(cx, "tab");
        if focused_node(cx)["aria"]["label"] == label {
            return;
        }
    }
    panic!("unreachable tab stop: {label}");
}

#[test]
fn removing_queue_rows_restores_visible_focus_including_the_last_row() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::SettingsVideo,
        ScalePreset::Percent100,
    );
    root.update(cx, |root, cx| {
        root.file_queue
            .add_file(crate::file_queue::FileItem::from_path(
                "keyboard-remove-second",
                "/tmp/keyboard-remove-second.mov",
                1000,
            ));
        cx.notify();
    });
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    tab_to_label(cx, "Remove file");
    press(cx, "enter");
    assert_eq!(focused_node(cx)["aria"]["label"], "File queue");
    tab_to_label(cx, "Remove file");
    press(cx, "enter");
    assert_eq!(focused_node(cx)["aria"]["label"], "Open file");
    press(cx, "shift-tab");
    assert_eq!(focused_node(cx)["aria"]["label"], "Open folder");
    press(cx, "tab");
    assert_eq!(focused_node(cx)["aria"]["label"], "Open file");
}

#[test]
fn queue_action_focus_survives_conversion_status_changes() {
    use crate::file_queue::FileStatus;
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::SettingsVideo,
        ScalePreset::Percent100,
    );
    let id = root.read_with(cx, |root, _| {
        root.file_queue.selected_file_id().unwrap().to_owned()
    });
    root.update(cx, |root, cx| {
        root.file_queue
            .update_status(&id, FileStatus::Converting, 25);
        cx.notify();
    });
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    tab_to_label(cx, "Pause conversion");
    let primary = focus(cx);
    for (status, label) in [
        (FileStatus::Paused, "Resume conversion"),
        (FileStatus::Completed, "Convert again"),
        (FileStatus::Idle, "Remove file"),
        (FileStatus::Queued, "Cancel conversion"),
        (FileStatus::Cancelling, "File queue"),
    ] {
        root.update(cx, |root, cx| {
            root.file_queue.update_status(&id, status, 25);
            cx.notify();
        });
        draw(cx);
        assert_eq!(focused_node(cx)["aria"]["label"], label);
        if matches!(status, FileStatus::Paused | FileStatus::Completed) {
            assert_eq!(focus(cx), primary);
        }
    }
}

fn assert_visible_focus(cx: &mut VisualTestContext, viewport: gpui::Size<Pixels>) -> String {
    let node = focused_node(cx);
    let label = node["aria"]["label"]
        .as_str()
        .expect("every tab stop needs a name");
    assert!(!label.is_empty());
    assert_ne!(node["aria"]["disabled"], true);
    let bounds = &node["bounds"];
    let x = bounds["x"].as_f64().unwrap();
    let y = bounds["y"].as_f64().unwrap();
    let w = bounds["width"].as_f64().unwrap();
    let h = bounds["height"].as_f64().unwrap();
    assert!(w > 0.0 && h > 0.0, "empty focus target: {node}");
    assert!(
        x >= -0.01
            && y >= -0.01
            && x + w <= f64::from(viewport.width.as_f32()) + 0.01
            && y + h <= f64::from(viewport.height.as_f32()) + 0.01,
        "offscreen focus: {node}"
    );
    label.to_owned()
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The table-driven traversal test checks the same invariants across all main views."
)]
fn every_view_has_named_visible_tab_stops_and_reversible_traversal() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        for fixture in [
            VisualFixture::SettingsOutput,
            VisualFixture::SettingsVideo,
            VisualFixture::SettingsVideoFilters,
            VisualFixture::SettingsAudio,
            VisualFixture::SettingsAudioFilters,
            VisualFixture::SettingsSubtitles,
            VisualFixture::SettingsMetadata,
            VisualFixture::SettingsPresets,
            VisualFixture::SettingsImages,
            VisualFixture::AppSettings,
            VisualFixture::LogsActive,
            VisualFixture::UpdateAvailable,
            VisualFixture::PreviewCrop,
            VisualFixture::PreviewCropCustom,
            VisualFixture::WorkspaceEmpty,
            VisualFixture::SettingsSource,
            VisualFixture::SettingsVideoAv1,
        ] {
            let mut app = TestAppContext::single();
            let (root, cx) = test_window(&mut app, fixture, scale);
            root.update(cx, |root, cx| {
                root.available_filters = frame_core::capabilities::parse_available_filters(
                    [
                        "eq",
                        "hue",
                        "colortemperature",
                        "unsharp",
                        "gblur",
                        "hqdn3d",
                        "deband",
                        "vignette",
                        "bwdif",
                        "highpass",
                        "lowpass",
                        "afftdn",
                        "deesser",
                        "bass",
                        "treble",
                        "acompressor",
                        "loudnorm",
                        "volume",
                        "stereotools",
                        "alimiter",
                    ]
                    .map(|filter| format!(" TSC {filter} available\n"))
                    .join(""),
                );
                cx.notify();
            });
            let viewport = size(px(1200.0 * scale.factor()), px(800.0 * scale.factor()));
            cx.simulate_resize(viewport);
            cx.update(|window, _| window.set_a11y_forced(true));
            draw(cx);
            let mut seen = Vec::new();
            let mut names = Vec::new();
            let mut wrapped = false;
            for _ in 0..180 {
                press(cx, "tab");
                let handle = focus(cx);
                let name = assert_visible_focus(cx, viewport);
                if seen.contains(&handle) {
                    assert_eq!(
                        seen.first(),
                        Some(&handle),
                        "premature focus cycle: {fixture:?}"
                    );
                    wrapped = true;
                    break;
                }
                seen.push(handle);
                names.push(name);
            }
            assert!(
                wrapped && seen.len() >= 2,
                "incomplete traversal: {fixture:?}"
            );
            // Expected landmarks prevent an accidentally shortened cycle passing.
            let landmarks: &[&str] = match fixture {
                VisualFixture::SettingsOutput => &["Re-encode", "Container", "Output name"],
                VisualFixture::SettingsVideo => &["Custom", "Video CRF"],
                VisualFixture::SettingsPresets => &["Preset name", "Discord"],
                VisualFixture::PreviewCrop | VisualFixture::PreviewCropCustom => &[
                    "Crop aspect ratio",
                    "Move crop selection",
                    "Resize crop top left corner",
                ],
                VisualFixture::AppSettings => &["Close settings", "Theme", "Check now"],
                VisualFixture::UpdateAvailable => &["Close update dialog", "Later", "Download"],
                _ => &[],
            };
            for name in landmarks {
                assert!(
                    names.iter().any(|actual| actual == name),
                    "missing {name} in {fixture:?}: {names:?}"
                );
            }
            // Playback controls appear while focus is inside Preview, then unmount
            // when focus leaves. Their reveal/traversal has a dedicated test; compare
            // the stable controls here, in both directions through that region.
            let contextual = ["Play preview", "Pause preview", "Trim preview"];
            for (expected, name) in seen.iter().zip(&names).rev() {
                if contextual.contains(&name.as_str()) {
                    continue;
                }
                press(cx, "shift-tab");
                for _ in 0..2 {
                    let current = assert_visible_focus(cx, viewport);
                    if !contextual.contains(&current.as_str()) {
                        break;
                    }
                    press(cx, "shift-tab");
                }
                assert_eq!(
                    &focus(cx),
                    expected,
                    "reverse traversal: {fixture:?}, expected {name}, actual {}",
                    focused_node(cx)["aria"]["label"]
                );
                assert_visible_focus(cx, viewport);
            }
        }
    }
}

#[test]
fn settings_escape_returns_to_its_trigger_and_preserves_tab_order() {
    let mut app = TestAppContext::single();
    let (_, cx) = test_window(
        &mut app,
        VisualFixture::SettingsVideo,
        ScalePreset::Percent100,
    );
    click(cx, "titlebar-settings");
    press(cx, "escape");
    let trigger = focus(cx);
    press(cx, "tab");
    press(cx, "shift-tab");
    assert_eq!(focus(cx), trigger);
    press(cx, "enter");
    assert!(
        cx.debug_bounds("app-settings-close").is_some(),
        "Enter must reopen settings"
    );
}

#[test]
fn crop_toolbar_reaches_resize_handles_and_escape_restores_crop_button() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewCropCustom,
        ScalePreset::Percent100,
    );
    // Opening focuses the aspect selector. Backward traversal must reach the
    // last resize handle, rather than wrapping straight back to Apply.
    let before = root.read_with(cx, |root, _| root.preview_ui.draft_crop);
    press(cx, "shift-tab");
    press(cx, "right");
    assert_ne!(
        root.read_with(cx, |root, _| root.preview_ui.draft_crop),
        before
    );
    press(cx, "escape");
    assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_mode));
    press(cx, "enter");
    assert!(root.read_with(cx, |root, _| root.preview_ui.crop_mode));
    press(cx, "escape");
    assert!(
        !root.read_with(cx, |root, _| root.preview_ui.crop_mode),
        "Escape also works from the toolbar"
    );
}

#[test]
fn virtual_queue_keyboard_selection_reaches_offscreen_files_without_toggling_them() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::WorkspaceLargeQueue,
        ScalePreset::Percent100,
    );
    cx.simulate_resize(size(px(1200.0), px(800.0)));
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    for _ in 0..40 {
        press(cx, "tab");
        if focused_node(cx)["aria"]["label"] == "File queue" {
            break;
        }
    }
    assert_eq!(focused_node(cx)["aria"]["label"], "File queue");
    let selection_before = root.read_with(cx, |root, _| root.file_queue.batch_selection_state());
    let initial_runtime_key =
        root.read_with(cx, |root, _| root.preview_ui.pending_runtime_key.clone());
    press(cx, "end");
    root.read_with(cx, |root, _| {
        assert_eq!(
            root.file_queue.selected_file_id(),
            root.file_queue.files().last().map(|f| f.id.as_str())
        );
        assert_eq!(root.file_queue.batch_selection_state(), selection_before);
    });
    let row = cx
        .debug_bounds("file-list-row-fixture-queue-499")
        .expect("last row must be rendered");
    assert!(row.top() >= px(592.0) && row.bottom() <= px(784.0));
    // Returning to the ready fixture must not start FFmpeg for its dummy /tmp path.
    root.update(cx, |root, _| {
        root.preview_ui.pending_runtime_key = initial_runtime_key;
    });
    press(cx, "home");
    root.read_with(cx, |root, _| {
        assert_eq!(
            root.file_queue.selected_file_id(),
            root.file_queue.files().first().map(|f| f.id.as_str())
        );
    });
    press(cx, "space");
    assert_ne!(
        root.read_with(cx, |root, _| root.file_queue.batch_selection_state()),
        selection_before
    );
}

#[test]
fn overlay_toolbar_does_not_trap_focus_away_from_resize_handles() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::PreviewReady,
        ScalePreset::Percent100,
    );
    root.update(cx, |root, cx| {
        root.update_selected_config(|config| {
            config.overlay = Some(OverlaySettings {
                enabled: true,
                path: "/tmp/keyboard-overlay.png".into(),
                x: 0.5,
                y: 0.5,
                width: 0.3,
                opacity: 1.0,
                anchor: "custom".into(),
            });
            true
        });
        cx.notify();
    });
    draw(cx);
    root.update(cx, |root, cx| {
        root.set_selected_overlay_mode(true);
        cx.notify();
    });
    draw(cx);
    let before = root.read_with(cx, |root, _| root.preview_ui.overlay.overlay().cloned());
    press(cx, "shift-tab");
    press(cx, "right");
    assert_ne!(
        root.read_with(cx, |root, _| root.preview_ui.overlay.overlay().cloned()),
        before
    );
    press(cx, "escape");
    assert!(!root.read_with(cx, |root, _| root.preview_ui.overlay.overlay_mode()));
    press(cx, "enter");
    assert!(root.read_with(cx, |root, _| root.preview_ui.overlay.overlay_mode()));
    press(cx, "escape");
    assert!(!root.read_with(cx, |root, _| root.preview_ui.overlay.overlay_mode()));
}

#[test]
fn logs_reveal_distant_tabs_and_keep_the_output_focus_ring_inside_its_clip() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(&mut app, VisualFixture::LogsActive, ScalePreset::Percent100);
    let viewport = size(px(1200.0), px(800.0));
    cx.simulate_resize(viewport);
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    let single_tab = log_tab_widths(cx);
    assert_eq!(single_tab.len(), 1);
    let single_strip = cx.debug_bounds("logs-tab-list").unwrap();
    let edge_padding = f64::from(single_strip.size.width.as_f32()) - single_tab[0];
    assert!(
        edge_padding >= 0.0 && edge_padding < single_tab[0],
        "one tab should determine the strip width, not fill the header: {single_strip:?}"
    );
    let copy_right = cx.debug_bounds("logs-copy").unwrap().right();
    for count in [3, 30] {
        root.update(cx, |root, cx| {
            for index in root.file_queue.files().len()..count {
                let id = format!("keyboard-log-{index:02}");
                root.file_queue
                    .add_file(crate::file_queue::FileItem::from_path(
                        &id,
                        format!("/tmp/long_conversion_filename_{index:02}.mov"),
                        1000,
                    ));
                root.file_queue
                    .update_status(&id, crate::file_queue::FileStatus::Converting, 50);
                root.conversion_events.apply_conversion_event(
                    &mut root.file_queue,
                    frame_core::events::ConversionEvent::log(&id, "Progress"),
                );
            }
            cx.notify();
        });
        draw(cx);
        let widths = log_tab_widths(cx);
        assert_eq!(widths.len(), count);
        let content_width: f64 = widths.iter().sum();
        let strip = cx.debug_bounds("logs-tab-list").unwrap();
        let copy = cx.debug_bounds("logs-copy").unwrap();
        assert_eq!(copy.right(), copy_right, "Copy must keep its right anchor");
        assert!(strip.right() < copy.left(), "tabs must not overlap Copy");
        if count == 3 {
            assert!(
                (f64::from(strip.size.width.as_f32()) - content_width - edge_padding).abs() < 0.5,
                "a short strip should fit the sum of tab widths: {strip:?}, tabs={widths:?}"
            );
        } else {
            assert!(
                content_width > f64::from(strip.size.width.as_f32()),
                "many tabs must overflow inside a constrained strip"
            );
        }
    }
    for _ in 0..10 {
        press(cx, "tab");
        if focused_node(cx)["element_id"]
            .as_str()
            .unwrap()
            .contains("logs-tab-")
        {
            break;
        }
    }
    press(cx, "end");
    assert_visible_focus(cx, viewport);
    let strip = cx.debug_bounds("logs-tab-list").unwrap();
    let focused = focused_node(cx);
    assert_eq!(focused["aria"]["label"], "long_conversion_filename_29.mov");
    let left = focused["bounds"]["x"].as_f64().unwrap();
    let right = left + focused["bounds"]["width"].as_f64().unwrap();
    assert!(
        left >= f64::from(strip.left().as_f32()) - 0.01
            && right <= f64::from(strip.right().as_f32()) + 0.01,
        "End must reveal the final tab inside its scrolling strip: {focused}"
    );
    press(cx, "home");
    assert_visible_focus(cx, viewport);
    press(cx, "tab"); // Copy
    press(cx, "tab"); // Log output
    let node = focused_node(cx);
    assert_eq!(node["aria"]["label"], "Conversion log output");
    let bounds = cx.debug_bounds("logs-output").expect("log output bounds");
    cx.update(|window, _| {
        let ring_bounds = bounds
            .dilate(px(theme::metrics::FOCUS_WIDTH))
            .scale(window.scale_factor());
        let ring = window
            .painted_quads()
            .into_iter()
            .find(|q| q.bounds == ring_bounds)
            .expect("log focus ring");
        assert_eq!(
            ring.bounds.intersect(&ring.content_mask.bounds),
            ring.bounds
        );
    });
}

fn log_tab_widths(cx: &mut VisualTestContext) -> Vec<f64> {
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        tree["nodes"]
            .as_object()
            .unwrap()
            .values()
            .filter(|node| {
                node["aria"]["role"] == "Tab"
                    && node["element_id"]
                        .as_str()
                        .is_some_and(|id| id.contains("logs-tab-"))
            })
            .map(|node| node["bounds"]["width"].as_f64().unwrap())
            .collect()
    })
}

#[test]
fn update_release_notes_can_be_scrolled_and_left_in_both_directions() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::UpdateAvailable,
        ScalePreset::Percent100,
    );
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    press(cx, "tab");
    assert_eq!(focused_node(cx)["aria"]["label"], "Release notes");
    let notes = focus(cx);
    press(cx, "end");
    assert!(
        root.read_with(cx, |root, _| root
            .update_ui
            .release_notes_scroll_handle
            .offset()
            .y)
            < px(0.0)
    );
    press(cx, "home");
    assert_eq!(
        root.read_with(cx, |root, _| root
            .update_ui
            .release_notes_scroll_handle
            .offset()
            .y),
        px(0.0)
    );
    press(cx, "tab");
    press(cx, "shift-tab");
    assert_eq!(focus(cx), notes);
    press(cx, "shift-tab");
    assert_eq!(focused_node(cx)["aria"]["label"], "Close update dialog");
    press(cx, "shift-tab");
    assert_eq!(focused_node(cx)["aria"]["label"], "Download");
}
