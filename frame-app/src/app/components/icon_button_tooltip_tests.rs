//! Exercise tooltip coverage through the real icon-button constructors and views.
use super::{
    FrameIconButtonSize, FrameIconButtonVariant, frame_icon_button,
    frame_icon_button_with_motion_key, frame_icon_swap_button,
};
use crate::app::FrameRoot;
use crate::app::accessibility::handle_tab_navigation;
use crate::app::primitives::color;
use crate::appearance::ColorTheme;
use crate::{VisualFixture, assets, theme};
use gpui::prelude::*;
use gpui::{
    AppContext, Context, Entity, FocusHandle, Modifiers, RenderImage, Subscription, TestAppContext,
    VisualTestContext, Window, div, point, px, size,
};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

const HOVER_DELAY: Duration = Duration::from_millis(400);
const ICON_SIZE: FrameIconButtonSize = FrameIconButtonSize {
    button: 32.0,
    icon: 16.0,
};

#[derive(Clone, Copy, Debug)]
enum IconControl {
    Shared,
    Swap,
    StableAction,
}

impl IconControl {
    const ALL: [Self; 3] = [Self::Shared, Self::Swap, Self::StableAction];

    const fn id(self) -> &'static str {
        match self {
            Self::Shared => "tooltip-shared-action",
            Self::Swap => "tooltip-copy-action",
            Self::StableAction => "tooltip-pause-action",
        }
    }

    const fn tooltip_id(self) -> &'static str {
        match self {
            Self::Shared => "tooltip-tooltip-shared-action",
            Self::Swap => "tooltip-tooltip-copy-action",
            Self::StableAction => "tooltip-tooltip-pause-action",
        }
    }
}

struct IconTooltipView {
    root: Entity<FrameRoot>,
    root_focus: FocusHandle,
    action_focus: FocusHandle,
    control: IconControl,
    active: bool,
    clicks: Rc<Cell<usize>>,
    _subscription: Subscription,
}

impl gpui::Render for IconTooltipView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let control = self.control;
        let active = self.active;
        let root_focus = self.root_focus.clone();
        let action_focus = self.action_focus.clone();
        let clicks = self.clicks.clone();
        window.set_rem_size(px(crate::appearance::BASE_REM_PX));
        self.root.update(cx, |root, cx| {
            let enabled = !root.is_processing;
            let palette = theme::palette(ColorTheme::Dark);
            let button = match control {
                IconControl::Shared => frame_icon_button(
                    control.id(),
                    assets::ICON_REFRESH,
                    "Reset filter",
                    FrameIconButtonVariant::Ghost,
                    enabled,
                    ICON_SIZE,
                    palette,
                    window,
                    cx,
                ),
                IconControl::Swap => frame_icon_swap_button(
                    control.id(),
                    assets::ICON_COPY,
                    assets::ICON_CHECK,
                    active,
                    color(palette.success),
                    if active { "Logs copied" } else { "Copy logs" },
                    FrameIconButtonVariant::Ghost,
                    enabled,
                    ICON_SIZE,
                    palette,
                    window,
                    cx,
                ),
                IconControl::StableAction => frame_icon_button_with_motion_key(
                    if active {
                        "tooltip-resume-action"
                    } else {
                        control.id()
                    },
                    "tooltip-queue-primary-slot",
                    if active {
                        assets::ICON_PLAY
                    } else {
                        assets::ICON_PAUSE
                    },
                    if active {
                        "Resume conversion"
                    } else {
                        "Pause conversion"
                    },
                    FrameIconButtonVariant::Ghost,
                    enabled,
                    ICON_SIZE,
                    Some(&action_focus),
                    palette,
                    window,
                    cx,
                ),
            }
            .on_click(cx.listener(move |_, _: &gpui::ClickEvent, _, _| {
                if enabled {
                    clicks.set(clicks.get() + 1);
                }
            }));
            div()
                .size_full()
                .p_16()
                .track_focus(&root_focus)
                .on_key_down(|event, window, cx| {
                    handle_tab_navigation(event, window, cx);
                })
                .child(div().flex().child(button))
        })
    }
}

fn control_window(
    app: &mut TestAppContext,
    control: IconControl,
) -> (Entity<IconTooltipView>, &mut VisualTestContext) {
    let (view, cx) = app.add_window_view(|window, cx| {
        let root_focus = cx.focus_handle();
        root_focus.focus(window, cx);
        let root = cx.new(|_| FrameRoot::new());
        let subscription = cx.observe(&root, |_, _, cx| cx.notify());
        IconTooltipView {
            root,
            root_focus,
            action_focus: cx.focus_handle(),
            control,
            active: false,
            clicks: Rc::default(),
            _subscription: subscription,
        }
    });
    cx.update(|window, cx| {
        window.activate();
        window.set_a11y_forced(true);
        cx.set_reduce_motion(true);
    });
    draw(cx);
    (view, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
}

fn press(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    draw(cx);
}

fn hover(cx: &mut VisualTestContext, id: &'static str) {
    let button = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("missing {id}"));
    cx.simulate_mouse_move(button.center(), None, Modifiers::default());
    cx.executor().advance_clock(HOVER_DELAY);
    draw(cx);
}

fn focused_label(cx: &mut VisualTestContext) -> String {
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        let focused = tree["gpui_focus"].as_str().unwrap();
        tree["nodes"][focused]["aria"]["label"]
            .as_str()
            .unwrap()
            .to_owned()
    })
}

#[test]
fn shared_icon_constructors_show_tooltips_and_preserve_pointer_and_keyboard_activation() {
    for control in IconControl::ALL {
        let mut app = TestAppContext::single();
        let (view, cx) = control_window(&mut app, control);
        assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
        hover(cx, control.id());
        assert!(
            cx.debug_bounds(control.tooltip_id()).is_some(),
            "{control:?}"
        );
        let button = cx.debug_bounds(control.id()).unwrap();
        cx.simulate_click(button.center(), Modifiers::default());
        draw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.clicks.get()), 1);
        assert_eq!(
            cx.debug_bounds("frame-tooltip-bubble").is_some(),
            matches!(control, IconControl::Swap),
            "only feedback buttons keep their visible tooltip after activation"
        );

        cx.simulate_mouse_move(point(px(800.0), px(600.0)), None, Modifiers::default());
        let root_focus = view.read_with(cx, |view, _| view.root_focus.clone());
        cx.update(|window, cx| root_focus.focus(window, cx));
        press(cx, "tab");
        assert!(
            cx.debug_bounds(control.tooltip_id()).is_some(),
            "{control:?}"
        );
        press(cx, "enter");
        assert_eq!(view.read_with(cx, |view, _| view.clicks.get()), 2);
        assert_eq!(
            cx.debug_bounds("frame-tooltip-bubble").is_some(),
            matches!(control, IconControl::Swap)
        );
    }
}

#[test]
fn disabled_shared_icon_constructors_neither_activate_nor_show_tooltips() {
    for control in IconControl::ALL {
        let mut app = TestAppContext::single();
        let (view, cx) = control_window(&mut app, control);
        let root = view.read_with(cx, |view, _| view.root.clone());
        root.update(cx, |root, cx| {
            root.is_processing = true;
            cx.notify();
        });
        draw(cx);
        hover(cx, control.id());
        let button = cx.debug_bounds(control.id()).unwrap();
        cx.simulate_click(button.center(), Modifiers::default());
        press(cx, "tab");
        press(cx, "enter");
        assert!(
            cx.debug_bounds("frame-tooltip-bubble").is_none(),
            "{control:?}"
        );
        assert_eq!(view.read_with(cx, |view, _| view.clicks.get()), 0);
    }
}

#[test]
fn copied_icon_updates_the_visible_tooltip_together_with_its_accessible_label() {
    let mut app = TestAppContext::single();
    let (view, cx) = control_window(&mut app, IconControl::Swap);
    press(cx, "tab");
    assert_eq!(focused_label(cx), "Copy logs");
    let original = cx.debug_bounds("frame-tooltip-bubble").unwrap();
    view.update(cx, |view, cx| {
        view.active = true;
        cx.notify();
    });
    draw(cx);
    assert_eq!(focused_label(cx), "Logs copied");
    let copied = cx.debug_bounds("frame-tooltip-bubble").unwrap();
    assert!(
        copied.size.width > original.size.width,
        "the tooltip must use the updated label"
    );
}

#[test]
fn queue_action_tooltip_uses_the_current_action_while_preserving_its_registered_focus() {
    let mut app = TestAppContext::single();
    let (view, cx) = control_window(&mut app, IconControl::StableAction);
    press(cx, "tab");
    let expected_focus = view.read_with(cx, |view, _| view.action_focus.clone());
    assert!(cx.update(|window, _| expected_focus.is_focused(window)));
    assert_eq!(focused_label(cx), "Pause conversion");
    assert!(cx.debug_bounds("tooltip-tooltip-pause-action").is_some());
    view.update(cx, |view, cx| {
        view.active = true;
        cx.notify();
    });
    draw(cx);
    assert!(cx.update(|window, _| expected_focus.is_focused(window)));
    assert_eq!(focused_label(cx), "Resume conversion");
    assert!(cx.debug_bounds("tooltip-tooltip-pause-action").is_none());
    assert!(cx.debug_bounds("tooltip-tooltip-resume-action").is_some());
    press(cx, "enter");
    assert_eq!(view.read_with(cx, |view, _| view.clicks.get()), 1);
}

fn replace_queue_action_without_pointer_exit(
    view: &Entity<IconTooltipView>,
    cx: &mut VisualTestContext,
) {
    view.update(cx, |view, cx| {
        view.active = true;
        cx.notify();
    });
    // Let both retained frames release the old action and its tooltip state.
    draw(cx);
    draw(cx);
    assert!(cx.debug_bounds("tooltip-pause-action").is_none());
    assert!(cx.debug_bounds("tooltip-tooltip-pause-action").is_none());
    cx.simulate_mouse_move(point(px(800.0), px(600.0)), None, Modifiers::default());
    draw(cx);
}

#[test]
fn disappearing_queue_action_does_not_leave_other_tooltips_permanently_warm() {
    let mut app = TestAppContext::single();
    let (view, cx) = control_window(&mut app, IconControl::StableAction);
    hover(cx, "tooltip-pause-action");
    assert!(cx.debug_bounds("tooltip-tooltip-pause-action").is_some());

    replace_queue_action_without_pointer_exit(&view, cx);
    let resume = cx.debug_bounds("tooltip-resume-action").unwrap();
    cx.simulate_mouse_move(resume.center(), None, Modifiers::default());
    draw(cx);
    assert!(
        cx.debug_bounds("frame-tooltip-bubble").is_none(),
        "the removed owner's visible state must not bypass a new hover delay"
    );
    cx.executor()
        .advance_clock(HOVER_DELAY.checked_sub(Duration::from_millis(1)).unwrap());
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    cx.executor().advance_clock(Duration::from_millis(1));
    draw(cx);
    assert!(cx.debug_bounds("tooltip-tooltip-resume-action").is_some());
}

#[test]
fn queue_action_removed_during_hover_delay_cannot_reopen_its_tooltip() {
    let mut app = TestAppContext::single();
    let (view, cx) = control_window(&mut app, IconControl::StableAction);
    let pause = cx.debug_bounds("tooltip-pause-action").unwrap();
    cx.simulate_mouse_move(pause.center(), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(200));
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());

    replace_queue_action_without_pointer_exit(&view, cx);
    cx.executor().advance_clock(HOVER_DELAY);
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());

    let resume = cx.debug_bounds("tooltip-resume-action").unwrap();
    cx.simulate_mouse_move(resume.center(), None, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    cx.executor().advance_clock(HOVER_DELAY);
    draw(cx);
    assert!(cx.debug_bounds("tooltip-tooltip-resume-action").is_some());
}

fn fixture_window(
    app: &mut TestAppContext,
    fixture: VisualFixture,
) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (root, cx) = app.add_window_view(|_, cx| {
        crate::app::init_app(cx, "Frame icon tooltip coverage");
        cx.set_reduce_motion(true);
        let mut root = FrameRoot::new();
        root.apply_visual_fixture(Some(fixture));
        if fixture == VisualFixture::SettingsPresets {
            root.presets.retain(|preset| preset.id == "custom-review");
        }
        root.available_filters.lutyuv = true;
        root.preview_ui.render_image = Some(Arc::new(RenderImage::new(smallvec::smallvec![
            image::Frame::new(image::RgbaImage::new(400, 225))
        ])));
        let metadata = root.selected_source_metadata_entry();
        root.preview_ui.pending_runtime_key = root
            .selected_preview_runtime_request(&metadata)
            .map(|request| request.key);
        root
    });
    cx.update(|window, _| window.activate());
    cx.simulate_resize(size(px(2200.0), px(1800.0)));
    draw(cx);
    (root, cx)
}

#[test]
fn real_settings_logs_presets_and_filter_actions_expose_their_tooltips() {
    for (fixture, id, tooltip_id) in [
        (
            VisualFixture::SettingsVideo,
            "titlebar-settings",
            "tooltip-titlebar-settings",
        ),
        (VisualFixture::LogsActive, "logs-copy", "tooltip-logs-copy"),
        (
            VisualFixture::SettingsPresets,
            "settings-preset-apply-all-custom-review",
            "tooltip-settings-preset-apply-all-custom-review",
        ),
        (
            VisualFixture::SettingsPresets,
            "settings-preset-delete-custom-review",
            "tooltip-settings-preset-delete-custom-review",
        ),
        (
            VisualFixture::SettingsVideoFilters,
            "settings-video-filter-brightness-reset",
            "tooltip-settings-video-filter-brightness-reset",
        ),
    ] {
        let mut app = TestAppContext::single();
        let (_, cx) = fixture_window(&mut app, fixture);
        hover(cx, id);
        assert!(cx.debug_bounds(tooltip_id).is_some(), "{fixture:?}: {id}");
    }
}

fn assert_logs_copy_feedback(root: &Entity<FrameRoot>, cx: &mut VisualTestContext) {
    let expected = root.read_with(cx, |root, _| {
        let file_id = root.conversion_events.selected_log_file_id().unwrap();
        assert_eq!(root.copied_log_file_id.as_deref(), Some(file_id));
        root.conversion_events.logs_for(file_id).join("\n")
    });
    let clipboard = cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(clipboard.as_deref(), Some(expected.as_str()));
    assert!(cx.debug_bounds("tooltip-logs-copy").is_some());
    cx.update(|window, _| {
        let tree: serde_json::Value =
            serde_json::from_str(&window.debug_a11y_tree_json().unwrap()).unwrap();
        assert!(
            tree["nodes"]
                .as_object()
                .unwrap()
                .values()
                .any(|node| node["aria"]["label"] == "Logs copied")
        );
    });
}

#[test]
fn clicking_logs_copy_keeps_the_updated_feedback_tooltip_until_leave_or_escape() {
    let mut app = TestAppContext::single();
    let (root, cx) = fixture_window(&mut app, VisualFixture::LogsActive);
    cx.update(|window, _| window.set_a11y_forced(true));
    draw(cx);
    hover(cx, "logs-copy");
    let before = cx.debug_bounds("frame-tooltip-bubble").unwrap();
    let button = cx.debug_bounds("logs-copy").unwrap();
    cx.simulate_click(button.center(), Modifiers::default());
    draw(cx);
    assert_logs_copy_feedback(&root, cx);
    let after = cx.debug_bounds("frame-tooltip-bubble").unwrap();
    assert!(
        after.size.width > before.size.width,
        "the visible label must change to Logs copied"
    );

    cx.simulate_mouse_move(point(px(200.0), px(800.0)), None, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    hover(cx, "logs-copy");
    assert!(cx.debug_bounds("tooltip-logs-copy").is_some());
    press(cx, "escape");
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
}

#[test]
fn keyboard_copy_keeps_the_updated_feedback_tooltip_and_escape_dismisses_it() {
    for key in ["enter", "space"] {
        let mut app = TestAppContext::single();
        let (root, cx) = fixture_window(&mut app, VisualFixture::LogsActive);
        cx.update(|window, _| window.set_a11y_forced(true));
        draw(cx);
        for _ in 0..40 {
            press(cx, "tab");
            if focused_label(cx) == "Copy logs" {
                break;
            }
        }
        assert_eq!(focused_label(cx), "Copy logs");
        let before = cx.debug_bounds("frame-tooltip-bubble").unwrap();
        press(cx, key);
        assert_logs_copy_feedback(&root, cx);
        assert_eq!(focused_label(cx), "Logs copied");
        let after = cx.debug_bounds("frame-tooltip-bubble").unwrap();
        assert!(after.size.width > before.size.width);
        press(cx, "escape");
        assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
    }
}

#[test]
fn tooltip_warm_switching_is_shared_between_settings_and_filter_actions() {
    let mut app = TestAppContext::single();
    let (_, cx) = fixture_window(&mut app, VisualFixture::SettingsVideoFilters);
    hover(cx, "titlebar-settings");
    assert!(cx.debug_bounds("tooltip-titlebar-settings").is_some());

    let reset = cx
        .debug_bounds("settings-video-filter-brightness-reset")
        .unwrap();
    cx.simulate_mouse_move(reset.center(), None, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("tooltip-titlebar-settings").is_none());
    assert!(
        cx.debug_bounds("tooltip-settings-video-filter-brightness-reset")
            .is_some(),
        "switching between different constructor families must reuse the warm delay"
    );
}

#[test]
fn labelled_titlebar_action_does_not_gain_a_redundant_tooltip() {
    let mut app = TestAppContext::single();
    let (_, cx) = fixture_window(&mut app, VisualFixture::SettingsVideo);
    hover(cx, "titlebar-add-source");
    assert!(cx.debug_bounds("frame-tooltip-bubble").is_none());
}

#[test]
fn overlay_toolbar_icon_actions_expose_tooltips_in_the_real_preview() {
    let mut app = TestAppContext::single();
    let (root, cx) = fixture_window(&mut app, VisualFixture::PreviewReady);
    root.update(cx, |root, cx| {
        root.update_selected_config(|config| {
            config.overlay = Some(crate::settings::OverlaySettings {
                enabled: true,
                path: "/tmp/tooltip-overlay.png".into(),
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
    for id in ["preview-overlay-decrease", "preview-overlay-increase"] {
        assert!(cx.debug_bounds(id).is_none(), "{id} should not be rendered");
    }
    for id in ["preview-zoom-out", "preview-zoom-in"] {
        assert!(cx.debug_bounds(id).is_some(), "{id} must remain available");
    }
    for (id, tooltip_id) in [
        ("preview-overlay-replace", "tooltip-preview-overlay-replace"),
        ("preview-overlay-remove", "tooltip-preview-overlay-remove"),
        ("preview-overlay-done", "tooltip-preview-overlay-done"),
    ] {
        hover(cx, id);
        assert!(cx.debug_bounds(tooltip_id).is_some(), "{id}");
    }
}
