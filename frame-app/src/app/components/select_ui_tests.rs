use crate::{
    VisualFixture,
    app::{FrameRoot, SettingsSubtitleMode},
    appearance::{ColorTheme, ScalePreset},
};
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
