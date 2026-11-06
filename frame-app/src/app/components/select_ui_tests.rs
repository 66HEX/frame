use crate::VisualFixture;
use crate::app::{FrameRoot, SettingsSubtitleMode};
use crate::appearance::{ColorTheme, ScalePreset};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, px, size};

struct SelectCase {
    fixture: VisualFixture,
    trigger: &'static str,
    panel: &'static str,
}

const SELECTS: [SelectCase; 6] = [
    SelectCase {
        fixture: VisualFixture::AppSettings,
        trigger: "app-settings-theme",
        panel: "app-settings-theme-options",
    },
    SelectCase {
        fixture: VisualFixture::AppSettings,
        trigger: "app-settings-ui-scale",
        panel: "app-settings-ui-scale-options",
    },
    SelectCase {
        fixture: VisualFixture::SettingsSubtitles,
        trigger: "settings-subtitle-font-select",
        panel: "settings-subtitle-font-options",
    },
    SelectCase {
        fixture: VisualFixture::SettingsSubtitles,
        trigger: "settings-subtitle-font-size-select",
        panel: "settings-subtitle-font-size-options",
    },
    SelectCase {
        fixture: VisualFixture::PreviewCropCustom,
        trigger: "preview-crop-ratio",
        panel: "crop-options-panel",
    },
    SelectCase {
        fixture: VisualFixture::PreviewCropCustom,
        trigger: "crop-units",
        panel: "crop-units-panel",
    },
];

const OUTPUT_SELECTS: [SelectCase; 4] = [
    SelectCase {
        fixture: VisualFixture::SettingsOutput,
        trigger: "container-select",
        panel: "container-select-options",
    },
    SelectCase {
        fixture: VisualFixture::SettingsVideo,
        trigger: "video-encoder-select",
        panel: "video-encoder-select-options",
    },
    SelectCase {
        fixture: VisualFixture::SettingsVideo,
        trigger: "video-pixel-format-select",
        panel: "video-pixel-format-select-options",
    },
    SelectCase {
        fixture: VisualFixture::SettingsAudio,
        trigger: "audio-codec-select",
        panel: "audio-codec-select-options",
    },
];

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn click(cx: &mut VisualTestContext, id: &'static str) {
    let bounds = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("missing {id}"));
    cx.simulate_click(bounds.center(), Modifiers::default());
    draw(cx);
}

fn press(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    draw(cx);
}

fn select_window<'a>(
    app: &'a mut TestAppContext,
    case: &SelectCase,
    theme: ColorTheme,
    scale: ScalePreset,
) -> (Entity<FrameRoot>, &'a mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Frame select test");
        cx.set_reduce_motion(true);
        let mut root = FrameRoot::new();
        root.apply_visual_fixture(Some(case.fixture));
        root.appearance.color_theme = theme;
        root.appearance.ui_scale = scale;
        root.subtitle_ui.mode = SettingsSubtitleMode::BurnIn;
        let metadata = root.selected_source_metadata_entry();
        root.preview_ui.pending_runtime_key = root
            .selected_preview_runtime_request(&metadata)
            .map(|request| request.key);
        root
    });
    cx.update(|window, _| window.activate());
    cx.simulate_resize(size(px(1800.0), px(1800.0)));
    draw(cx);
    (root, cx)
}

#[test]
fn every_select_opens_navigates_closes_and_reopens_in_both_themes_and_scales() {
    for theme in [ColorTheme::Dark, ColorTheme::Light] {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            for case in &SELECTS {
                let mut app = TestAppContext::single();
                let (root, cx) = select_window(&mut app, case, theme, scale);
                for keyboard in [false, true] {
                    if keyboard {
                        press(cx, "enter");
                    } else {
                        click(cx, case.trigger);
                    }
                    assert!(cx.debug_bounds(case.panel).is_some(), "{}", case.trigger);
                    cx.simulate_keystrokes("down");
                    draw(cx);
                    assert!(
                        cx.debug_bounds(case.panel).is_some(),
                        "down: {}",
                        case.trigger
                    );
                    cx.simulate_keystrokes("end");
                    draw(cx);
                    assert!(
                        cx.debug_bounds(case.panel).is_some(),
                        "end: {}",
                        case.trigger
                    );
                    cx.simulate_keystrokes("escape");
                    draw(cx);
                    assert!(cx.debug_bounds(case.panel).is_none(), "{}", case.trigger);
                    let focused = cx.update(|window, cx| {
                        root.update(cx, |root, cx| {
                            root.ensure_focus(
                                crate::app::accessibility::FrameFocusKey::Control(
                                    if case.trigger == "preview-crop-ratio" {
                                        "preview-crop-action-free"
                                    } else {
                                        case.trigger
                                    }
                                    .into(),
                                ),
                                true,
                                cx,
                            )
                            .is_focused(window)
                        })
                    });
                    assert!(focused, "focus not restored: {}", case.trigger);
                }
            }
        }
    }
}

#[test]
fn both_color_pickers_open_with_mouse_and_keyboard_and_restore_focus_on_escape() {
    for (trigger, panel) in [
        (
            "settings-subtitle-font-color",
            "settings-subtitle-font-color-picker",
        ),
        (
            "settings-subtitle-outline-color",
            "settings-subtitle-outline-color-picker",
        ),
    ] {
        let case = SelectCase {
            fixture: VisualFixture::SettingsSubtitles,
            trigger,
            panel,
        };
        for theme in [ColorTheme::Dark, ColorTheme::Light] {
            for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
                let mut app = TestAppContext::single();
                let (_, cx) = select_window(&mut app, &case, theme, scale);
                click(cx, trigger);
                assert!(cx.debug_bounds(panel).is_some());
                press(cx, "escape");
                assert!(cx.debug_bounds(panel).is_none());
                press(cx, "enter");
                assert!(cx.debug_bounds(panel).is_some());
                press(cx, "escape");
                assert!(cx.debug_bounds(panel).is_none());
            }
        }
    }
}

#[test]
fn closing_font_menus_ignore_clicks_before_the_exit_has_finished() {
    for (index, option) in [(2, "subtitle-font-Arial"), (3, "subtitle-size-24")] {
        let case = &SELECTS[index];
        let mut app = TestAppContext::single();
        let (root, cx) = select_window(&mut app, case, ColorTheme::Dark, ScalePreset::Percent100);
        click(cx, case.trigger);
        root.update(cx, |root, _| {
            root.close_subtitle_popover();
            // A queued click can still target the previous rendered frame.
            let config = &mut root.file_queue.selected_file_mut().unwrap().config;
            config.subtitle_font_name = Some("Test font".into());
            config.subtitle_font_size = Some("36".into());
        });
        click(cx, option);
        root.read_with(cx, |root, _| {
            let config = &root.file_queue.selected_file().unwrap().config;
            assert_eq!(config.subtitle_font_name.as_deref(), Some("Test font"));
            assert_eq!(config.subtitle_font_size.as_deref(), Some("36"));
        });
    }
}

#[test]
fn disabled_font_selects_ignore_pointer_activation() {
    for case in &SELECTS[2..4] {
        let mut app = TestAppContext::single();
        let (root, cx) = select_window(&mut app, case, ColorTheme::Dark, ScalePreset::Percent100);
        root.update(cx, |root, cx| {
            root.file_queue.selected_file_mut().unwrap().status =
                crate::file_queue::FileStatus::Queued;
            cx.notify();
        });
        draw(cx);
        click(cx, case.trigger);
        assert!(root.read_with(cx, |root, _| root.subtitle_ui.popover.is_none()));
        assert!(cx.debug_bounds(case.panel).is_none());
    }
}

#[test]
fn selecting_the_current_font_or_size_still_closes_the_menu() {
    for (index, option) in [(2, "subtitle-font-Arial"), (3, "subtitle-size-24")] {
        let case = &SELECTS[index];
        let mut app = TestAppContext::single();
        let (root, cx) = select_window(&mut app, case, ColorTheme::Dark, ScalePreset::Percent100);
        click(cx, case.trigger);
        click(cx, option);
        assert!(root.read_with(cx, |root, _| root.subtitle_ui.popover.is_none()));
        assert!(cx.debug_bounds(case.panel).is_none());
    }
}

#[test]
fn processing_tabs_activate_on_arrow_focus_and_wrap() {
    let case = SelectCase {
        fixture: VisualFixture::SettingsOutput,
        trigger: "output-mode-reencode",
        panel: "unused",
    };
    let mut app = TestAppContext::single();
    let (root, cx) = select_window(&mut app, &case, ColorTheme::Dark, ScalePreset::Percent100);
    click(cx, case.trigger);
    let original = root.read_with(cx, |root, _| {
        root.selected_config().unwrap().processing_mode
    });
    press(cx, "right");
    assert_ne!(
        root.read_with(cx, |root, _| root
            .selected_config()
            .unwrap()
            .processing_mode),
        original
    );
    press(cx, "enter");
    assert_ne!(
        root.read_with(cx, |root, _| root
            .selected_config()
            .unwrap()
            .processing_mode),
        original
    );
    press(cx, "right");
    press(cx, "enter");
    assert_eq!(
        root.read_with(cx, |root, _| root
            .selected_config()
            .unwrap()
            .processing_mode),
        original
    );
}

#[test]
fn every_select_and_color_picker_finishes_enter_and_exit_before_unmounting() {
    let colors = [
        SelectCase {
            fixture: VisualFixture::SettingsSubtitles,
            trigger: "settings-subtitle-font-color",
            panel: "settings-subtitle-font-color-picker",
        },
        SelectCase {
            fixture: VisualFixture::SettingsSubtitles,
            trigger: "settings-subtitle-outline-color",
            panel: "settings-subtitle-outline-color-picker",
        },
    ];
    for case in SELECTS.iter().chain(&OUTPUT_SELECTS).chain(&colors) {
        let mut app = TestAppContext::single();
        let (_, cx) = select_window(&mut app, case, ColorTheme::Dark, ScalePreset::Percent100);
        cx.update(|_, cx| cx.set_reduce_motion(false));
        click(cx, case.trigger);
        let entering = cx.debug_bounds(case.panel).expect(case.trigger);
        std::thread::sleep(crate::app::motion::SURFACE_MOTION_DURATION);
        draw(cx);
        let settled = cx.debug_bounds(case.panel).expect(case.trigger);
        assert!(
            entering.top() < settled.top(),
            "{} must slide into place",
            case.trigger
        );
        press(cx, "escape");
        assert!(
            cx.debug_bounds(case.panel).is_some(),
            "{} must retain its exit frame",
            case.trigger
        );
        std::thread::sleep(crate::app::motion::SURFACE_MOTION_DURATION);
        draw(cx);
        assert!(
            cx.debug_bounds(case.panel).is_none(),
            "{} must unmount after its exit",
            case.trigger
        );
        click(cx, case.trigger);
        assert!(
            cx.debug_bounds(case.panel).is_some(),
            "{} must reopen",
            case.trigger
        );
    }
}

#[test]
fn output_selects_restore_focus_and_preserve_keyboard_selection_on_reopen() {
    for theme in [ColorTheme::Dark, ColorTheme::Light] {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            for case in &OUTPUT_SELECTS {
                let mut app = TestAppContext::single();
                let (_, cx) = select_window(&mut app, case, theme, scale);
                click(cx, case.trigger);
                assert!(cx.debug_bounds(case.panel).is_some(), "{}", case.trigger);
                press(cx, "escape");
                let trigger = cx.update(|window, cx| window.focused(cx).unwrap());
                press(cx, "enter");
                assert!(cx.debug_bounds(case.panel).is_some());
                press(cx, "end");
                let selected_option = cx.update(|window, cx| window.focused(cx).unwrap());
                press(cx, "enter");
                assert!(cx.debug_bounds(case.panel).is_none());
                assert!(cx.update(|window, _| trigger.is_focused(window)));
                press(cx, "enter");
                assert!(cx.debug_bounds(case.panel).is_some());
                assert!(
                    cx.update(|window, _| selected_option.is_focused(window)),
                    "selection not retained: {}",
                    case.trigger
                );
                press(cx, "escape");
                press(cx, "shift-tab");
                assert!(!cx.update(|window, _| trigger.is_focused(window)));
                press(cx, "tab");
                assert!(cx.update(|window, _| trigger.is_focused(window)));
            }
        }
    }
}

#[test]
fn selects_and_color_pickers_stay_in_view_when_opened_near_the_bottom() {
    let colors = [
        SelectCase {
            fixture: VisualFixture::SettingsSubtitles,
            trigger: "settings-subtitle-font-color",
            panel: "settings-subtitle-font-color-picker",
        },
        SelectCase {
            fixture: VisualFixture::SettingsSubtitles,
            trigger: "settings-subtitle-outline-color",
            panel: "settings-subtitle-outline-color-picker",
        },
    ];
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        for case in SELECTS.iter().chain(&OUTPUT_SELECTS).chain(&colors) {
            let mut app = TestAppContext::single();
            let (_, cx) = select_window(&mut app, case, ColorTheme::Dark, scale);
            click(cx, case.trigger);
            let natural_height = cx.debug_bounds(case.panel).unwrap().size.height;
            press(cx, "escape");
            let trigger = cx.debug_bounds(case.trigger).unwrap();
            let viewport = size(px(1800.0), trigger.bottom() + px(40.0));
            cx.simulate_resize(viewport);
            draw(cx);
            click(cx, case.trigger);
            let trigger = cx.debug_bounds(case.trigger).unwrap();
            let panel = cx.debug_bounds(case.panel).unwrap();
            assert!(panel.top() >= px(0.0), "{}: {panel:?}", case.trigger);
            assert!(
                panel.bottom() <= viewport.height,
                "{}: {panel:?}, viewport={viewport:?}",
                case.trigger
            );
            assert!(
                panel.left() >= px(0.0) && panel.right() <= viewport.width,
                "{}: {panel:?}",
                case.trigger
            );
            if viewport.height - trigger.bottom() < natural_height && trigger.top() > natural_height
            {
                assert!(
                    panel.bottom() < trigger.top(),
                    "{} must flip above: {panel:?}, trigger={trigger:?}",
                    case.trigger
                );
            }
            cx.simulate_resize(size(px(1800.0), px(1800.0)));
            draw(cx);
            let trigger = cx.debug_bounds(case.trigger).unwrap();
            let panel = cx.debug_bounds(case.panel).unwrap();
            assert!(
                panel.top() > trigger.bottom(),
                "{} must return below after resize",
                case.trigger
            );
            press(cx, "escape");
            assert!(cx.debug_bounds(case.panel).is_none());
        }
    }
}

#[test]
fn select_scrollbars_overlay_full_width_options_without_reserving_a_gutter() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let case = &SELECTS[3];
        let (_, cx) = select_window(&mut app, case, ColorTheme::Dark, scale);
        click(cx, case.trigger);
        for key in ["home", "end"] {
            press(cx, key);
            let panel = cx.debug_bounds(case.panel).unwrap();
            let item = cx.debug_bounds("subtitle-size-8").unwrap();
            let scrollbar = cx
                .debug_bounds("settings-subtitle-font-size-options-scrollbar")
                .unwrap();
            let left = item.left() - panel.left();
            let right = panel.right() - item.right();
            assert!(
                (left - right).abs() < px(0.1),
                "asymmetric item inset: {item:?}, {panel:?}"
            );
            assert!(
                item.right() > scrollbar.left(),
                "scrollbar must overlay the options"
            );
            assert_eq!(scrollbar.right(), panel.right());
            assert_eq!(scrollbar.top(), panel.top());
            assert_eq!(scrollbar.bottom(), panel.bottom());
        }
    }
}

#[test]
fn flipped_select_and_color_picker_animate_toward_the_trigger_on_exit() {
    for case in [
        SelectCase {
            fixture: VisualFixture::SettingsSubtitles,
            trigger: "settings-subtitle-font-select",
            panel: "settings-subtitle-font-options",
        },
        SelectCase {
            fixture: VisualFixture::SettingsSubtitles,
            trigger: "settings-subtitle-font-color",
            panel: "settings-subtitle-font-color-picker",
        },
    ] {
        let mut app = TestAppContext::single();
        let (_, cx) = select_window(&mut app, &case, ColorTheme::Light, ScalePreset::Percent100);
        let trigger = cx.debug_bounds(case.trigger).unwrap();
        cx.simulate_resize(size(px(1800.0), trigger.bottom() + px(40.0)));
        draw(cx);
        cx.update(|_, cx| cx.set_reduce_motion(false));
        click(cx, case.trigger);
        let entering = cx.debug_bounds(case.panel).unwrap();
        std::thread::sleep(crate::app::motion::SURFACE_MOTION_DURATION);
        draw(cx);
        let settled = cx.debug_bounds(case.panel).unwrap();
        let trigger = cx.debug_bounds(case.trigger).unwrap();
        assert!(settled.bottom() < trigger.top());
        assert!(
            entering.top() > settled.top(),
            "{} must enter upward",
            case.trigger
        );
        press(cx, "escape");
        std::thread::sleep(crate::app::motion::SURFACE_MOTION_DURATION / 2);
        draw(cx);
        let exiting = cx.debug_bounds(case.panel).unwrap();
        assert!(
            exiting.top() > settled.top(),
            "{} must exit downward",
            case.trigger
        );
        std::thread::sleep(crate::app::motion::SURFACE_MOTION_DURATION);
        draw(cx);
        assert!(cx.debug_bounds(case.panel).is_none());
    }
}
