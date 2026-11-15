use super::*;
use crate::app::prompts::tests::{answer_prompt, pending_prompt};
use crate::appearance::{ColorTheme, ScalePreset};
use crate::settings::{OutputSection, SettingsTab};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, px, size};

fn hold_preview_request(root: &mut FrameRoot) {
    let metadata = root.selected_source_metadata_entry();
    root.preview_ui.pending_runtime_key = root
        .selected_preview_runtime_request(&metadata)
        .map(|request| request.key);
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn window(
    app: &mut TestAppContext,
    theme: ColorTheme,
    scale: ScalePreset,
) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Apply to all test");
        cx.set_reduce_motion(true);
        let mut root = batch_root();
        add_target(&mut root, "target", FileStatus::Idle, SourceKind::Video);
        root.appearance.color_theme = theme;
        root.appearance.ui_scale = scale;
        root.settings_ui.active_tab = SettingsTab::Output;
        root.settings_ui.expanded_output_sections.clear();
        hold_preview_request(&mut root);
        root
    });
    cx.update(|window, _| {
        window.activate();
        window.set_a11y_forced(true);
    });
    cx.simulate_resize(size(px(2200.0), px(1800.0)));
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

fn click_apply(cx: &mut VisualTestContext) {
    let bounds = cx
        .debug_bounds("settings-apply-all")
        .expect("Apply to all button");
    cx.simulate_click(bounds.center(), Modifiers::default());
    draw(cx);
}

fn assert_reason(cx: &mut VisualTestContext, expected: &str) {
    let button_bounds = cx.debug_bounds("settings-apply-all").unwrap();
    let hint_bounds = cx
        .debug_bounds("settings-apply-all-reason")
        .expect("visible hint");
    assert!(hint_bounds.top() >= button_bounds.bottom());
    assert!(hint_bounds.size.height > px(0.0));
    assert!(hint_bounds.size.width <= button_bounds.size.width);
    cx.update(|window, _| {
        let tree = window.a11y_tree().unwrap();
        let (_, button) = tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.role() == gpui::Role::Button && node.label() == Some("Apply to files…")
            })
            .expect("Apply to all AX button");
        assert!(button.is_disabled());
        assert_eq!(button.description(), Some(expected));
    });
}

#[test]
fn output_button_visibility_keyboard_prompt_cancel_and_render_sync() {
    for theme in ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (root, cx) = window(&mut app, theme, scale);
            assert!(cx.debug_bounds("settings-apply-all").is_some());
            assert!(root.read_with(cx, |root, _| {
                !root
                    .settings_ui
                    .expanded_output_sections
                    .contains(&OutputSection::General)
            }));
            // Reach the real button through the application's tab navigation.
            let mut focused = false;
            for _ in 0..100 {
                press(cx, "tab");
                focused = cx.update(|window, _| {
                    let tree: serde_json::Value =
                        serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
                    tree["gpui_focus"]
                        .as_str()
                        .is_some_and(|id| tree["nodes"][id]["aria"]["label"] == "Apply to files…")
                });
                if focused {
                    break;
                }
            }
            assert!(focused, "keyboard must reach Apply to all");
            press(cx, "enter");
            let (title, detail) = pending_prompt(cx).expect("native prompt opened");
            assert_eq!(title, "Apply output settings?");
            assert!(detail.contains("All files: 1 file; 0 skipped."));
            assert!(detail.contains("subtitle files where the new settings support them"));
            let before = root.read_with(cx, |root, _| root.file_queue.files().to_vec());
            answer_prompt(cx, "Cancel");
            draw(cx);
            assert_eq!(
                root.read_with(cx, |root, _| root.file_queue.files().to_vec()),
                before
            );
            click_apply(cx);
            root.update(cx, |root, _| {
                root.file_queue.select_existing_file("target");
                root.subtitle_ui.external_track_index = Some(99);
                root.file_queue
                    .file_by_id_mut("donor")
                    .unwrap()
                    .config
                    .quality = 22;
                hold_preview_request(root);
            });
            answer_prompt(cx, "Apply to all (1)");
            cx.run_until_parked();
            root.read_with(cx, |root, _| {
                assert_eq!(root.selected_config().unwrap().quality, 80);
            });
            root.update(cx, |root, _| hold_preview_request(root));
            draw(cx);
            root.read_with(cx, |root, _| {
                assert_eq!(root.subtitle_ui.external_track_index, Some(0));
                assert_eq!(root.preview_ui.crop_file_id.as_deref(), Some("target"));
                assert_eq!(
                    root.preview_ui
                        .pending_runtime_key
                        .as_ref()
                        .map(|key| key.file_id.as_str()),
                    Some("target")
                );
            });
            for tab in [SettingsTab::Metadata, SettingsTab::Presets] {
                root.update(cx, |root, cx| {
                    root.settings_ui.active_tab = tab;
                    cx.notify();
                });
                draw(cx);
                assert!(cx.debug_bounds("settings-apply-all").is_none());
            }
            root.update(cx, |root, cx| {
                root.file_queue.remove_file("donor");
                root.settings_ui.active_tab = SettingsTab::Output;
                cx.notify();
            });
            draw(cx);
            assert!(cx.debug_bounds("settings-apply-all").is_none());
        }
    }
}

#[test]
fn disabled_button_explains_readiness_status_and_updater_without_opening_prompt() {
    for theme in ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (root, cx) = window(&mut app, theme, scale);
            for reason in [
                DONOR_LOADING,
                DONOR_ERROR,
                DONOR_LOCKED,
                TARGETS_LOADING,
                NO_TARGETS,
                UPDATER,
            ] {
                root.update(cx, |root, cx| {
                    root.update_ui.status = UpdateStatus::Idle;
                    root.file_queue.file_by_id_mut("donor").unwrap().status = FileStatus::Idle;
                    root.source_metadata
                        .mark_ready("donor", metadata(SourceKind::Video, "h264", "aac"));
                    root.source_metadata
                        .mark_ready("target", metadata(SourceKind::Video, "h264", "aac"));
                    match reason {
                        DONOR_LOADING => root.source_metadata.mark_loading("donor"),
                        DONOR_ERROR => root.source_metadata.mark_error("donor", "failed"),
                        DONOR_LOCKED => {
                            root.file_queue.file_by_id_mut("donor").unwrap().status =
                                FileStatus::Queued;
                        }
                        TARGETS_LOADING => root.source_metadata.mark_loading("target"),
                        NO_TARGETS => root.source_metadata.mark_error("target", "failed"),
                        UPDATER => root.update_ui.status = UpdateStatus::Installing,
                        _ => unreachable!(),
                    }
                    hold_preview_request(root);
                    cx.notify();
                });
                draw(cx);
                assert_reason(cx, reason);
                click_apply(cx);
                assert!(!cx.update(|window, _| window.has_active_prompt()));
            }
        }
    }
}

#[test]
fn prompt_keeps_committed_crop_and_skips_targets_locked_while_waiting() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    root.update(cx, |root, cx| {
        root.preview_ui.crop_file_id = Some("donor".into());
        root.preview_ui.crop_mode = true;
        root.preview_ui.draft_crop = Some(crate::preview::CropRect {
            x: 0.1,
            y: 0.1,
            width: 0.5,
            height: 0.5,
        });
        cx.notify();
    });
    draw(cx);
    click_apply(cx);
    root.update(cx, |root, _| {
        root.file_queue.file_by_id_mut("target").unwrap().status = FileStatus::Queued;
    });
    let before = root.read_with(cx, |root, _| root.file_queue.files().to_vec());
    answer_prompt(cx, "Apply to all (1)");
    draw(cx);
    assert_eq!(
        root.read_with(cx, |root, _| root.file_queue.files().to_vec()),
        before
    );
    root.update(cx, |root, cx| {
        root.file_queue.file_by_id_mut("target").unwrap().status = FileStatus::Idle;
        cx.notify();
    });
    draw(cx);
    click_apply(cx);
    answer_prompt(cx, "Apply to all (1)");
    draw(cx);
    assert!(root.read_with(cx, |root, _| {
        root.file_queue
            .file_by_id("target")
            .unwrap()
            .config
            .crop
            .is_none()
    }));
}

#[test]
fn preset_batch_confirmation_reports_selected_all_and_skipped_counts() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    root.update(cx, |root, _| {
        add_target(root, "unchecked", FileStatus::Idle, SourceKind::Video);
        root.file_queue
            .file_by_id_mut("unchecked")
            .unwrap()
            .is_selected_for_conversion = false;
        add_target(root, "loading", FileStatus::Idle, SourceKind::Video);
        root.source_metadata.mark_loading("loading");
    });
    let before = root.read_with(cx, |root, _| root.file_queue.files().to_vec());
    cx.update(|window, cx| {
        root.update(cx, |root, cx| {
            root.confirm_apply_preset_to_all("archive-hq", window, cx);
        });
    });
    let (_, detail) = pending_prompt(cx).expect("preset scope confirmation");
    assert!(
        detail.contains("Selected for conversion: 2 files; 1 skipped."),
        "{detail}"
    );
    assert!(
        detail.contains("All files: 3 files; 1 skipped."),
        "{detail}"
    );
    answer_prompt(cx, "Cancel");
    draw(cx);
    assert_eq!(
        root.read_with(cx, |root, _| root.file_queue.files().to_vec()),
        before
    );
}

#[test]
fn selected_preset_reports_modified_after_fps_edit() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    root.update(cx, |root, cx| {
        root.settings_ui.active_tab = SettingsTab::Presets;
        assert!(root.apply_preset_to_selected("balanced-mp4"));
        root.update_selected_config(|config| {
            config.fps = "24".into();
            true
        });
        hold_preview_request(root);
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("preset-modified-balanced-mp4").is_some());
    cx.update(|window, _| {
        let tree = window.a11y_tree().unwrap();
        let (_, preset) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Balanced MP4"))
            .unwrap();
        assert_eq!(preset.description(), Some("Modified"));
    });
    root.update(cx, |root, cx| {
        assert!(root.apply_preset_to_selected("balanced-mp4"));
        root.update_selected_config(|config| {
            config.selected_audio_tracks = vec![7];
            config.selected_subtitle_tracks.clear();
            config.external_subtitle_tracks[0].path = "/tmp/own-captions.srt".into();
            true
        });
        hold_preview_request(root);
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("preset-modified-balanced-mp4").is_none());
    assert!(cx.debug_bounds("preset-applied-balanced-mp4").is_some());
}

#[test]
fn preset_batch_with_zero_targets_does_not_open_a_confirmation() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    root.update(cx, |root, _| {
        root.source_metadata.mark_loading("donor");
        root.source_metadata.mark_error("target", "probe failed");
    });
    let before = root.read_with(cx, |root, _| root.file_queue.files().to_vec());
    cx.update(|window, cx| {
        root.update(cx, |root, cx| {
            root.confirm_apply_preset_to_all("archive-hq", window, cx);
        });
    });
    assert!(!cx.update(|window, _| window.has_active_prompt()));
    assert_eq!(
        root.read_with(cx, |root, _| root.file_queue.files().to_vec()),
        before
    );
}

#[test]
fn preset_confirmation_freezes_targets_and_rechecks_readiness() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    cx.update(|window, cx| {
        root.update(cx, |root, cx| {
            root.confirm_apply_preset_to_all("archive-hq", window, cx);
        });
    });
    root.update(cx, |root, _| {
        add_target(root, "new", FileStatus::Idle, SourceKind::Video);
        root.source_metadata.mark_loading("target");
    });
    let before = root.read_with(cx, |root, _| root.file_queue.files().to_vec());
    answer_prompt(cx, "Apply to all (2)");
    draw(cx);
    root.read_with(cx, |root, _| {
        for old in before.iter().filter(|file| file.id != "donor") {
            assert_eq!(
                root.file_queue.file_by_id(&old.id).unwrap(),
                old,
                "new files and files no longer ready must remain unchanged"
            );
        }
        assert_eq!(
            root.file_queue
                .file_by_id("donor")
                .unwrap()
                .config
                .video_codec,
            "libx265"
        );
    });
}

#[test]
fn output_confirmation_can_apply_only_to_selected_files() {
    let mut app = TestAppContext::single();
    let (root, cx) = window(&mut app, ColorTheme::Dark, ScalePreset::Percent100);
    root.update(cx, |root, _| {
        add_target(root, "unchecked", FileStatus::Idle, SourceKind::Video);
        root.file_queue
            .file_by_id_mut("unchecked")
            .unwrap()
            .is_selected_for_conversion = false;
    });
    click_apply(cx);
    answer_prompt(cx, "Apply to selected (1)");
    draw(cx);
    root.read_with(cx, |root, _| {
        assert_eq!(
            root.file_queue.file_by_id("target").unwrap().config.quality,
            80
        );
        assert_ne!(
            root.file_queue
                .file_by_id("unchecked")
                .unwrap()
                .config
                .quality,
            80
        );
    });
}
