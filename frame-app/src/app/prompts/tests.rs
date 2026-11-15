use super::*;
use crate::app::FrameRoot;
use crate::appearance::{ColorTheme, ScalePreset};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, px, size};
use std::time::{Duration, Instant};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

pub(in crate::app) fn pending_prompt(cx: &mut VisualTestContext) -> Option<(String, String)> {
    draw(cx);
    cx.update(|window, _| {
        if !window.has_active_prompt() {
            return None;
        }
        let tree = window.a11y_tree()?;
        tree.nodes
            .iter()
            .find(|(_, node)| matches!(node.role(), gpui::Role::Dialog | gpui::Role::AlertDialog))
            .map(|(_, node)| {
                (
                    node.label().unwrap_or_default().to_owned(),
                    node.description().unwrap_or_default().to_owned(),
                )
            })
    })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Accessibility coordinates and GPUI pixels describe the same bounded test viewport."
)]
pub(in crate::app) fn answer_prompt(cx: &mut VisualTestContext, label: &str) {
    draw(cx);
    let position = cx.update(|window, _| {
        assert!(window.has_active_prompt());
        let tree = window.a11y_tree().unwrap();
        let bounds = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == gpui::Role::Button && node.label() == Some(label))
            .expect("rendered prompt action")
            .1
            .bounds()
            .expect("prompt action bounds");
        gpui::point(
            px(f64::midpoint(bounds.x0, bounds.x1) as f32 / window.scale_factor()),
            px(f64::midpoint(bounds.y0, bounds.y1) as f32 / window.scale_factor()),
        )
    });
    cx.simulate_click(position, Modifiers::default());
    draw(cx);
}

fn setup(
    app: &mut TestAppContext,
    theme: ColorTheme,
    scale: ScalePreset,
    reduced: bool,
) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Mira prompts");
        cx.set_reduce_motion(reduced);
        let mut root = FrameRoot::new();
        root.appearance.color_theme = theme;
        root.appearance.ui_scale = scale;
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

fn show(cx: &mut VisualTestContext) -> impl FnMut() -> Option<usize> + use<> {
    let mut receiver = cx.update(|window, cx| {
        window.prompt(
            PromptLevel::Warning,
            "Apply output settings?",
            Some("The prepared source is unchanged."),
            &[
                PromptButton::ok("Apply to selected (1)"),
                PromptButton::ok("Apply to all (2)"),
                PromptButton::cancel("Cancel"),
            ],
            cx,
        )
    });
    move || {
        receiver
            .try_recv()
            .expect("prompt response sender remains alive")
    }
}

fn press(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    draw(cx);
}

#[test]
fn mira_prompt_uses_shared_shell_primary_right_and_preserves_response_indices() {
    for theme in [ColorTheme::Dark, ColorTheme::Light] {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            let (_root, cx) = setup(&mut app, theme, scale, true);
            let mut response = show(cx);
            draw(cx);
            let panel = cx
                .debug_bounds("frame-prompt-panel")
                .expect("Mira prompt shell");
            assert!(panel.left() >= px(0.0) && panel.right() <= px(1280.0));
            let primary = cx.debug_bounds("frame-prompt-action-0").unwrap();
            let secondary = cx.debug_bounds("frame-prompt-action-1").unwrap();
            let cancel = cx.debug_bounds("frame-prompt-action-2").unwrap();
            for bounds in [primary, secondary, cancel] {
                assert!(
                    bounds.left() >= panel.left() && bounds.right() <= panel.right(),
                    "all three real batch actions remain within the dialog: {bounds:?} in {panel:?}"
                );
            }
            assert!(
                (primary.left() >= secondary.right() || primary.top() >= secondary.bottom())
                    && (primary.left() >= cancel.right() || primary.top() >= cancel.bottom())
            );
            assert_eq!(response(), None);
            cx.simulate_click(secondary.center(), Modifiers::default());
            draw(cx);
            assert_eq!(response(), Some(1));
            assert!(cx.debug_bounds("frame-prompt").is_none());
        }
    }
}

#[test]
fn mira_prompt_escape_is_cancel_and_restores_previous_focus() {
    let mut app = TestAppContext::single();
    let (_root, cx) = setup(&mut app, ColorTheme::Dark, ScalePreset::Percent100, true);
    let original = cx.update(|window, cx| window.focused(cx).unwrap());
    let mut response = show(cx);
    draw(cx);
    press(cx, "tab");
    press(cx, "shift-tab");
    press(cx, "escape");
    assert_eq!(response(), Some(2));
    cx.update(|window, _| assert!(original.is_focused(window)));
}

#[test]
fn mira_prompt_response_waits_for_exit_and_blocks_repeated_activation() {
    let mut app = TestAppContext::single();
    let mut now = Instant::now();
    app.set_transition_time(now);
    let (_root, cx) = setup(&mut app, ColorTheme::Dark, ScalePreset::Percent100, false);
    let mut response = show(cx);
    draw(cx);
    now += Duration::from_millis(100);
    cx.set_transition_time(now);
    draw(cx);
    let cancel = cx.debug_bounds("frame-prompt-action-2").unwrap();
    cx.simulate_click(cancel.center(), Modifiers::default());
    draw(cx);
    assert_eq!(response(), None, "receiver must wait for the exit");
    assert!(cx.debug_bounds("frame-prompt").is_some());
    press(cx, "enter");
    now += Duration::from_millis(50);
    cx.set_transition_time(now);
    draw(cx);
    assert_eq!(response(), None);
    now += Duration::from_millis(50);
    cx.set_transition_time(now);
    draw(cx);
    assert_eq!(response(), Some(2));
    assert!(cx.debug_bounds("frame-prompt").is_none());
}

#[test]
fn update_dialog_matches_mira_geometry() {
    for theme in [ColorTheme::Dark, ColorTheme::Light] {
        let mut app = TestAppContext::single();
        let (root, cx) = setup(&mut app, theme, ScalePreset::Percent100, true);
        root.update(cx, |root, cx| {
            root.apply_update_available_fixture();
            cx.notify();
        });
        draw(cx);
        let panel = cx.debug_bounds("update-dialog-panel").unwrap();
        assert_eq!(panel.size.width, px(384.0), "Mira Dialog max width");
        let later = cx.debug_bounds("update-dialog-later").unwrap();
        let primary = cx.debug_bounds("update-dialog-download").unwrap();
        assert!(primary.left() >= later.right());
        press(cx, "escape");
    }
}

#[test]
fn settings_drawer_preserves_version_034_edge_insets() {
    for theme in [ColorTheme::Dark, ColorTheme::Light] {
        let mut app = TestAppContext::single();
        let (root, cx) = setup(&mut app, theme, ScalePreset::Percent100, true);
        root.update(cx, |root, cx| {
            root.settings_ui.is_open = true;
            root.settings_ui.is_present = true;
            cx.notify();
        });
        draw(cx);
        let panel = cx.debug_bounds("app-settings-panel").unwrap();
        assert_eq!(panel.top(), px(8.0), "the settings drawer keeps its inset");
        assert_eq!(panel.bottom(), px(952.0));
        assert_eq!(panel.right(), px(1272.0));
        press(cx, "escape");
        assert!(cx.debug_bounds("app-settings-panel").is_none());
    }
}

#[test]
fn prompt_close_and_focus_survive_background_update_at_both_scales() {
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        let (root, cx) = setup(&mut app, ColorTheme::Dark, scale, true);
        root.update(cx, |root, cx| {
            root.apply_update_available_fixture();
            cx.notify();
        });
        draw(cx);
        let mut response = show(cx);
        draw(cx);
        press(cx, "tab");
        let focus = cx.update(|window, cx| window.focused(cx).unwrap());
        root.update(cx, |_, cx| cx.notify());
        draw(cx);
        cx.update(|window, _| {
            assert!(
                focus.is_focused(window),
                "background modal must not steal prompt focus"
            );
        });
        let close = cx.debug_bounds("frame-prompt-close").unwrap();
        cx.simulate_click(close.center(), Modifiers::default());
        draw(cx);
        assert_eq!(response(), Some(2));
        assert!(cx.debug_bounds("frame-prompt").is_none());
        assert!(cx.debug_bounds("update-dialog-panel").is_some());
    }
}

#[test]
fn update_downloading_content_precedes_its_footer() {
    let mut app = TestAppContext::single();
    let (root, cx) = setup(&mut app, ColorTheme::Dark, ScalePreset::Percent100, true);
    root.update(cx, |root, cx| {
        root.apply_update_available_fixture();
        root.update_ui.status = crate::app::updates::state::UpdateStatus::Downloading {
            version: "1.0.0".into(),
            progress_percent: Some(40),
            received_bytes: 40,
            total_bytes: Some(100),
        };
        cx.notify();
    });
    draw(cx);
    let busy = cx.debug_bounds("update-dialog-busy").unwrap();
    cx.update(|window, _| {
        let tree = window.a11y_tree().unwrap();
        let progress = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == gpui::Role::ProgressIndicator)
            .unwrap()
            .1
            .bounds()
            .unwrap();
        assert!(
            progress.y1 <= f64::from(busy.top().as_f32() * window.scale_factor()),
            "progress belongs above the footer"
        );
    });
    press(cx, "escape");
    assert!(root.read_with(cx, |root, _| root.update_ui.dialog_open));
}

#[test]
fn prompt_without_cancel_requires_an_explicit_action() {
    let mut app = TestAppContext::single();
    let (_root, cx) = setup(&mut app, ColorTheme::Light, ScalePreset::Percent100, true);
    let mut response = cx.update(|window, cx| {
        window.prompt(
            PromptLevel::Info,
            "Notice",
            Some("Acknowledgement required"),
            &[PromptButton::ok("Continue")],
            cx,
        )
    });
    draw(cx);
    assert!(cx.debug_bounds("frame-prompt-close").is_none());
    press(cx, "escape");
    assert_eq!(response.try_recv().unwrap(), None);
    press(cx, "enter");
    assert_eq!(response.try_recv().unwrap(), Some(0));
}
