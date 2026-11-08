use super::*;
use crate::file_queue::FileItem;
use crate::preview::aspect_value;
use crate::settings::{SourceKind, SourceMetadata};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, px, size};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn click(cx: &mut VisualTestContext, id: &'static str) {
    let center = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("missing {id}"))
        .center();
    cx.simulate_click(center, Modifiers::default());
    draw(cx);
}

fn crop_window(app: &mut TestAppContext) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Frame crop test");
        let mut root = FrameRoot::new();
        root.apply_visual_fixture(Some(crate::VisualFixture::PreviewCrop));
        // Exercise the real view without starting FFmpeg for the fixture path.
        let metadata = root.selected_source_metadata_entry();
        root.preview_ui.pending_runtime_key = root
            .selected_preview_runtime_request(&metadata)
            .map(|request| request.key);
        root
    });
    cx.update(|window, _| window.activate());
    cx.simulate_resize(size(px(1440.0), px(900.0)));
    draw(cx);
    (root, cx)
}

fn video_root(rotation: &str) -> FrameRoot {
    let mut root = FrameRoot::new();
    root.file_queue
        .add_file(FileItem::from_path("video", "/tmp/crop.mp4", 1));
    root.file_queue.selected_file_mut().unwrap().config.rotation = rotation.into();
    root.source_metadata.mark_ready(
        "video",
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            ..SourceMetadata::default()
        },
    );
    assert!(root.toggle_selected_crop_mode());
    root
}

#[test]
fn auto_crop_is_visible_and_keyboard_reachable_only_for_video() {
    use crate::appearance::ScalePreset;

    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        for custom in [false, true] {
            let mut app = TestAppContext::single();
            let (root, cx) = crop_window(&mut app);
            root.update(cx, |root, cx| {
                root.appearance.ui_scale = scale;
                if custom {
                    root.preview_ui.crop_options.custom = true;
                    root.set_crop_custom_mode(false);
                }
                cx.notify();
            });
            cx.simulate_resize(size(px(1200.0), px(800.0)));
            cx.update(|window, _| window.set_a11y_forced(true));
            draw(cx);
            assert_crop_toolbar_geometry(cx, scale, custom);
            let mut reached_auto_crop = false;
            for _ in 0..80 {
                press_crop_key(cx, "tab");
                if focused_crop_label(cx) == "Auto crop" {
                    reached_auto_crop = true;
                    break;
                }
            }
            assert!(reached_auto_crop, "Auto crop must be a named tab stop");
            assert_crop_control_visible(cx, "preview-crop-auto");
            press_crop_key(cx, "tab");
            assert_eq!(focused_crop_label(cx), "Reset");
            assert_crop_control_visible(cx, "preview-crop-action-reset");
            press_crop_key(cx, "tab");
            assert_eq!(focused_crop_label(cx), "Apply");
            assert_crop_control_visible(cx, "preview-crop-action-apply");
            let mut reached_ratio = false;
            for _ in 0..8 {
                press_crop_key(cx, "shift-tab");
                if focused_crop_label(cx) == "Crop aspect ratio" {
                    reached_ratio = true;
                    break;
                }
            }
            assert!(reached_ratio, "the aspect ratio must remain reachable");
            assert_crop_control_visible(cx, "preview-crop-ratio");

            root.update(cx, |root, cx| {
                let id = root.file_queue.selected_file_id().unwrap().to_owned();
                let mut metadata = root.selected_source_metadata().unwrap();
                metadata.media_kind = Some(SourceKind::Image);
                root.source_metadata.mark_ready(id, metadata);
                cx.notify();
            });
            draw(cx);
            assert!(
                cx.debug_bounds("preview-crop-auto").is_none(),
                "image cropping must not offer video analysis"
            );
        }
    }
}

fn assert_crop_toolbar_geometry(
    cx: &mut VisualTestContext,
    scale: crate::appearance::ScalePreset,
    custom: bool,
) {
    let viewport = cx.debug_bounds("preview-viewport").unwrap();
    let toolbar = cx.debug_bounds("preview-crop-toolbar").unwrap();
    assert!(
        toolbar.left() >= viewport.left()
            && toolbar.right() <= viewport.right()
            && toolbar.top() > viewport.top()
            && toolbar.bottom() < viewport.bottom(),
        "the crop toolbar must fit inside the preview at {scale:?}, custom={custom}: {toolbar:?} in {viewport:?}"
    );
    for id in [
        "preview-crop-ratio",
        "preview-crop-auto",
        "preview-crop-action-reset",
        "preview-crop-action-apply",
    ] {
        let bounds = cx.debug_bounds(id).unwrap();
        assert!(
            bounds.left() >= viewport.left() && bounds.right() <= viewport.right(),
            "{id} must remain inside the preview at {scale:?}, custom={custom}: {bounds:?} in {viewport:?}"
        );
        for tool in [
            "preview-tool-rotate",
            "preview-tool-flip-horizontal",
            "preview-tool-flip-vertical",
            "preview-tool-crop",
            "preview-tool-overlay",
            "preview-zoom-out",
            "preview-zoom-in",
        ] {
            let tool_bounds = cx.debug_bounds(tool).unwrap();
            assert!(
                bounds.intersect(&tool_bounds).is_empty(),
                "{id} overlaps {tool} at {scale:?}, custom={custom}: {bounds:?} / {tool_bounds:?}"
            );
        }
    }
}

fn assert_crop_control_visible(cx: &mut VisualTestContext, id: &'static str) {
    let toolbar = cx.debug_bounds("preview-crop-toolbar-controls").unwrap();
    let bounds = cx.debug_bounds(id).unwrap();
    assert!(
        bounds.left() >= toolbar.left()
            && bounds.right() <= toolbar.right()
            && bounds.top() >= toolbar.top()
            && bounds.bottom() <= toolbar.bottom(),
        "focused {id} must be revealed inside the toolbar: {bounds:?} in {toolbar:?}"
    );
}

fn press_crop_key(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    draw(cx);
    // GPUI reveals keyboard focus in scroll containers on the following frame.
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
}

fn focused_crop_label(cx: &mut VisualTestContext) -> String {
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        tree["gpui_focus"]
            .as_str()
            .and_then(|id| tree["nodes"][id]["aria"]["label"].as_str())
            .unwrap_or_default()
            .to_owned()
    })
}

#[test]
fn loaded_preview_keeps_timeline_and_queue_inside_window() {
    let mut app = TestAppContext::single();
    let (root, cx) = crop_window(&mut app);
    cx.simulate_resize(size(px(1200.0), px(800.0)));
    root.update(cx, |root, cx| {
        root.apply_visual_fixture(Some(crate::VisualFixture::PreviewCropCustom));
        let metadata = root.selected_source_metadata_entry();
        root.preview_ui.pending_runtime_key = root
            .selected_preview_runtime_request(&metadata)
            .map(|request| request.key);
        let frame = crate::preview_engine::PreviewFrame::bgra(
            1280,
            720,
            1280 * 4,
            0,
            vec![255; 1280 * 720 * 4],
        )
        .unwrap();
        root.preview_ui.render_image =
            Some(crate::preview_engine::render_image_from_frame(&frame).unwrap());
        cx.notify();
    });
    draw(cx);
    let viewport = cx.debug_bounds("preview-viewport").unwrap();
    let queue = cx.debug_bounds("file-list-body").unwrap();
    assert!(
        viewport.bottom() < queue.top(),
        "preview={viewport:?}, queue={queue:?}"
    );
    assert!(
        queue.bottom() <= px(800.0),
        "queue must remain visible: {queue:?}"
    );
    assert!(
        viewport.size.height < px(500.0),
        "preview must retain its workspace row share: {viewport:?}"
    );
}

#[test]
fn crop_selector_keyboard_and_pointer_work_in_rendered_window() {
    let mut app = TestAppContext::single();
    let (root, cx) = crop_window(&mut app);
    click(cx, "preview-crop-ratio");
    assert!(root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
    click(cx, "crop-ratio-4:3");
    assert_eq!(
        root.read_with(cx, |root, _| root.preview_ui.crop_aspect.clone()),
        "4:3"
    );
    assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
    cx.simulate_keystrokes("down");
    draw(cx);
    assert!(root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
    cx.simulate_keystrokes("end");
    draw(cx);
    cx.simulate_keystrokes("up");
    draw(cx);
    cx.simulate_keystrokes("enter");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    draw(cx);
    assert_eq!(
        root.read_with(cx, |root, _| root.preview_ui.crop_aspect.clone()),
        "1:2"
    );
    click(cx, "preview-crop-ratio");
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
}

#[test]
fn crop_custom_fields_accept_typing_and_tab_navigation() {
    let mut app = TestAppContext::single();
    let (root, cx) = crop_window(&mut app);
    click(cx, "preview-crop-ratio");
    cx.simulate_keystrokes("end");
    draw(cx);
    cx.simulate_keystrokes("enter");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    draw(cx);
    click(cx, "crop-units");
    click(cx, "crop-units-pixels");
    click(cx, "crop-custom-width");
    cx.simulate_keystrokes("cmd-a");
    cx.simulate_input("640");
    cx.simulate_keystrokes("tab");
    draw(cx);
    assert_eq!(
        cx.update(|window, cx| root.read(cx).focused_text_input_kind(window)),
        Some(FrameTextInputKind::CropHeight)
    );
    cx.simulate_keystrokes("cmd-a");
    cx.simulate_input("480");
    draw(cx);
    let live_crop = root.read_with(cx, |root, _| root.current_draft_crop_settings().unwrap());
    assert_eq!((live_crop.width, live_crop.height), (640, 480));
    assert!(root.read_with(cx, |root, _| root.selected_config().unwrap().crop.is_none()));
    cx.simulate_keystrokes("enter");
    draw(cx);
    assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
    let crop = root.read_with(cx, |root, _| {
        root.selected_config().unwrap().crop.clone().unwrap()
    });
    assert_eq!((crop.width, crop.height), (640, 480));
    assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_mode));
    root.update(cx, |root, cx| {
        assert!(root.toggle_selected_crop_mode());
        cx.notify();
    });
    draw(cx);
    click(cx, "preview-crop-ratio");
    // Clicking the trigger a second time closes it without reopening on mouse-up.
    assert!(
        !cx.debug_bounds("crop-options-panel")
            .unwrap()
            .contains(&cx.debug_bounds("preview-crop-ratio").unwrap().center())
    );
    click(cx, "preview-crop-ratio");
    assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
}

#[test]
fn crop_picker_fits_small_windows_at_large_interface_scales() {
    for scale in [
        crate::appearance::ScalePreset::Percent100,
        crate::appearance::ScalePreset::Percent200,
    ] {
        let mut app = TestAppContext::single();
        let (root, cx) = crop_window(&mut app);
        root.update(cx, |root, cx| {
            root.appearance.ui_scale = scale;
            cx.notify();
        });
        cx.simulate_resize(size(px(1200.0), px(800.0)));
        draw(cx);
        click(cx, "preview-crop-ratio");
        let panel = cx.debug_bounds("crop-options-panel").unwrap();
        assert!(
            panel.left() >= px(0.0) && panel.top() >= px(0.0),
            "{scale:?}: {panel:?}"
        );
        assert!(
            panel.right() <= px(1200.0) && panel.bottom() <= px(800.0),
            "{scale:?}: {panel:?}"
        );
        let trigger = cx.debug_bounds("preview-crop-ratio").unwrap();
        assert!(
            !panel.contains(&trigger.center()),
            "{scale:?}: panel={panel:?}, trigger={trigger:?}"
        );
        cx.simulate_keystrokes("end");
        draw(cx);
        cx.simulate_keystrokes("up");
        draw(cx);
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        draw(cx);
        assert_eq!(
            root.read_with(cx, |root, _| root.preview_ui.crop_aspect.clone()),
            "1:2"
        );
        click(cx, "preview-crop-ratio");
        cx.simulate_click(gpui::point(px(1100.0), px(700.0)), Modifiers::default());
        draw(cx);
        assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
    }
}

#[test]
fn custom_ratio_updates_preview_while_typing_without_cumulative_shrinking() {
    let mut root = video_root("0");
    root.select_preview_crop_aspect("16:9");
    root.preview_ui.crop_options.custom = true;
    root.set_crop_custom_mode(true);
    root.write_text_input_value(FrameTextInputKind::CropWidth, "9");
    root.write_text_input_value(FrameTextInputKind::CropHeight, "1");
    root.write_text_input_value(FrameTextInputKind::CropHeight, "16");
    assert_eq!(root.preview_ui.crop_aspect, "9:16");
    let live_crop = root.current_draft_crop_settings().unwrap();
    assert_eq!((live_crop.width, live_crop.height), (608, 1080));
    assert!(root.selected_config().unwrap().crop.is_none());
    assert!(root.apply_selected_crop());
    assert_eq!(root.selected_config().unwrap().crop, Some(live_crop));
}

#[test]
fn invalid_custom_input_keeps_last_valid_preview_and_resolution() {
    let mut root = video_root("0");
    root.preview_ui.crop_options.custom = true;
    root.set_crop_custom_mode(true);
    root.write_text_input_value(FrameTextInputKind::CropWidth, "7");
    root.write_text_input_value(FrameTextInputKind::CropHeight, "5");
    let valid_rect = root.preview_ui.draft_crop;
    let valid_settings = root.crop_settings_preview().unwrap();
    for invalid in ["", "0", "65536"] {
        root.write_text_input_value(FrameTextInputKind::CropHeight, invalid);
        assert_eq!(root.preview_ui.draft_crop, valid_rect);
        assert_eq!(root.crop_settings_preview(), Some(valid_settings.clone()));
        assert!(!root.apply_selected_crop());
    }
    root.write_text_input_value(FrameTextInputKind::CropHeight, "5");
    assert_eq!(root.preview_ui.draft_crop, valid_rect);
    assert!(root.toggle_selected_crop_mode());
    assert!(root.selected_config().unwrap().crop.is_none());
}

#[test]
fn moving_a_live_custom_crop_is_preserved_on_apply() {
    let mut root = video_root("0");
    root.preview_ui.crop_options.custom = true;
    root.set_crop_custom_mode(false);
    root.write_text_input_value(FrameTextInputKind::CropWidth, "640");
    root.write_text_input_value(FrameTextInputKind::CropHeight, "480");
    let before = root.current_draft_crop_settings().unwrap();
    root.apply_preview_crop_drag(
        crate::preview::DragHandle::Move,
        crate::preview::Point { x: 0.5, y: 0.5 },
    );
    assert!(root.apply_preview_crop_drag(
        crate::preview::DragHandle::Move,
        crate::preview::Point { x: 0.6, y: 0.55 }
    ));
    root.end_preview_crop_drag();
    let moved = root.current_draft_crop_settings().unwrap();
    assert!(moved.x > before.x && moved.y > before.y);
    assert_eq!((moved.width, moved.height), (640, 480));
    assert!(root.apply_selected_crop());
    assert_eq!(root.selected_config().unwrap().crop, Some(moved));
}

#[test]
fn ratio_mode_uses_the_visible_free_crop_proportions() {
    let mut root = video_root("0");
    root.preview_ui.crop_options.custom = true;
    root.set_crop_custom_mode(true);
    assert_eq!(root.custom_crop_values(), Some((16, 9)));
    root.preview_custom_crop_values();
    assert_eq!(root.preview_ui.crop_aspect, "16:9");
    assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
}

#[test]
fn crop_resolution_preview_commits_on_apply_and_preserves_settings_on_cancel() {
    let mut root = video_root("0");
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .resolution = "720p".into();
    assert!(root.select_preview_crop_aspect("4:3"));
    let preview = root.crop_settings_preview().unwrap();
    assert_eq!(preview.resolution, "custom");
    assert_eq!(preview.custom_width.as_deref(), Some("1440"));
    assert_eq!(preview.custom_height.as_deref(), Some("1080"));
    assert_eq!(root.selected_config().unwrap().resolution, "720p");
    assert!(root.toggle_selected_crop_mode());
    assert!(root.crop_settings_preview().is_none());
    assert_eq!(root.selected_config().unwrap().resolution, "720p");
    assert!(root.toggle_selected_crop_mode());
    assert!(root.select_preview_crop_aspect("4:3"));
    assert!(root.apply_selected_crop());
    let config = root.selected_config().unwrap();
    assert_eq!(config.resolution, "custom");
    assert_eq!(config.custom_width.as_deref(), Some("1440"));
    assert_eq!(config.custom_height.as_deref(), Some("1080"));
    assert!(root.toggle_selected_crop_mode());
    let metadata = root.selected_source_metadata_entry();
    let request = root.selected_preview_runtime_request(&metadata).unwrap();
    assert_eq!(request.config.conversion_config.resolution, "original");
    assert!(request.config.conversion_config.crop.is_none());
}

#[test]
fn custom_crop_resolution_preserves_selected_pixels_before_encoder_padding() {
    let mut root = video_root("90");
    root.source_metadata.mark_ready(
        "video",
        SourceMetadata {
            media_kind: Some(SourceKind::Video),
            width: Some(1920),
            height: Some(1080),
            pixel_format: Some("yuv420p".into()),
            ..SourceMetadata::default()
        },
    );
    root.preview_ui.crop_options.custom = true;
    root.set_crop_custom_mode(false);
    root.write_text_input_value(FrameTextInputKind::CropWidth, "481");
    root.write_text_input_value(FrameTextInputKind::CropHeight, "359");
    let preview = root.crop_settings_preview().unwrap();
    assert_eq!(preview.custom_width.as_deref(), Some("481"));
    assert_eq!(preview.custom_height.as_deref(), Some("359"));
    assert_eq!(
        preview.crop.as_ref().map(|crop| (crop.width, crop.height)),
        Some((481, 359))
    );
    assert_eq!(
        frame_core::dimensions::output_dimensions(
            &crate::conversion_runner::core_config_from_gpui(&preview),
            1920,
            1080,
            Some("yuv420p"),
        ),
        Some((482, 360))
    );
    assert!(root.apply_selected_crop());
    let config = root.selected_config().unwrap();
    assert_eq!(config.custom_width, preview.custom_width);
    assert_eq!(config.custom_height, preview.custom_height);
    assert_eq!(config.crop, preview.crop);
    assert_eq!(
        frame_core::dimensions::output_dimensions(
            &crate::conversion_runner::core_config_from_gpui(config),
            1920,
            1080,
            Some("yuv420p"),
        ),
        Some((482, 360))
    );
}

#[test]
fn reset_discards_pending_custom_dimensions() {
    let mut root = video_root("0");
    root.preview_ui.crop_options.custom = true;
    root.set_crop_custom_mode(false);
    root.write_text_input_value(FrameTextInputKind::CropWidth, "640");
    root.write_text_input_value(FrameTextInputKind::CropHeight, "480");
    assert!(root.reset_preview_crop_selection());
    assert!(!root.preview_ui.crop_options.custom);
    assert!(root.apply_selected_crop());
    assert!(root.selected_config().unwrap().crop.is_none());
}

#[test]
fn crop_four_three_matches_issue_example() {
    let mut root = video_root("0");
    assert!(root.select_preview_crop_aspect("4:3"));
    assert!(root.apply_selected_crop());
    let crop = root.selected_config().unwrap().crop.as_ref().unwrap();
    assert_eq!(
        (crop.x, crop.y, crop.width, crop.height),
        (240, 0, 1440, 1080)
    );
}

#[test]
fn crop_exact_size_survives_all_rotations_and_flips() {
    for rotation in ["0", "90", "180", "270"] {
        for flip in [false, true] {
            let mut root = video_root(rotation);
            root.file_queue
                .selected_file_mut()
                .unwrap()
                .config
                .flip_horizontal = flip;
            root.preview_ui.crop_options.is_ratio = false;
            root.preview_ui.crop_options.width = "640".into();
            root.preview_ui.crop_options.height = "480".into();
            assert!(root.apply_custom_crop_values());
            assert!(root.apply_selected_crop());
            let crop = root.selected_config().unwrap().crop.as_ref().unwrap();
            assert_eq!(
                (crop.width, crop.height),
                (640, 480),
                "{rotation}, flip={flip}"
            );
        }
    }
}

#[test]
fn crop_rejects_invalid_custom_values_without_changing_selection() {
    let mut root = video_root("90");
    root.preview_ui.crop_options.is_ratio = false;
    let original = root.preview_ui.draft_crop;
    for (width, height) in [
        ("", "480"),
        ("0", "480"),
        ("1081", "480"),
        ("640", "1921"),
        ("999999999999", "480"),
    ] {
        root.preview_ui.crop_options.width = width.into();
        root.preview_ui.crop_options.height = height.into();
        assert!(!root.apply_custom_crop_values());
        assert_eq!(root.preview_ui.draft_crop, original);
    }
}

#[test]
fn crop_custom_ratio_is_preserved_on_apply_and_reopen() {
    let mut root = video_root("0");
    root.preview_ui.crop_options.width = "7".into();
    root.preview_ui.crop_options.height = "5".into();
    assert!(root.apply_custom_crop_values());
    assert!(root.apply_selected_crop());
    assert!(root.toggle_selected_crop_mode());
    assert_eq!(root.preview_ui.crop_aspect, "7:5");
    assert!(aspect_value(&root.preview_ui.crop_aspect).is_some());
}
