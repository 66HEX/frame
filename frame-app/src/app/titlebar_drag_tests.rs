use super::*;
use gpui::{
    AppContext, Entity, Modifiers, Subscription, TestAppContext, VisualTestContext, point, size,
};

struct TitlebarTestView {
    root: Entity<FrameRoot>,
    platform: FrameTitlebarPlatform,
    processing: bool,
    focus: FocusHandle,
    _subscription: Subscription,
}

impl gpui::Render for TitlebarTestView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let platform = self.platform;
        let processing = self.processing;
        let focus = self.focus.clone();
        self.root.update(cx, |root, cx| {
            window.set_rem_size(px(crate::appearance::BASE_REM_PX));
            let state = FrameAppState {
                active_view: root.active_view,
                file_count: 1,
                selected_count: 1,
                has_actionable_files: true,
                has_default_output_directory: true,
                is_processing: processing,
                ..FrameAppState::default()
            };
            div()
                .flex()
                .flex_col()
                .size_full()
                .track_focus(&focus)
                .on_key_down(|event, window, cx| {
                    super::super::accessibility::handle_tab_navigation(event, window, cx);
                })
                .child(titlebar_for_platform(
                    platform,
                    state,
                    theme::palette(ColorTheme::Dark),
                    window,
                    cx,
                ))
                .child(
                    div()
                        .id("titlebar-test-content")
                        .flex_1()
                        .w_full()
                        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
                )
        })
    }
}

fn titlebar_window(
    app: &mut TestAppContext,
    platform: FrameTitlebarPlatform,
    processing: bool,
) -> (Entity<FrameRoot>, &mut VisualTestContext) {
    let (view, cx) = app.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let root = cx.new(|_| FrameRoot::new());
        let subscription = cx.observe(&root, |_, _, cx| cx.notify());
        TitlebarTestView {
            root,
            platform,
            processing,
            focus,
            _subscription: subscription,
        }
    });
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(size(px(1800.0), px(500.0)));
    cx.run_until_parked();
    let root = view.read_with(cx, |view, _| view.root.clone());
    draw(cx);
    (root, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear());
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear());
}

fn drag_is_armed(root: &Entity<FrameRoot>, cx: &VisualTestContext) -> bool {
    root.read_with(cx, |root, _| root.titlebar_drag.should_move)
}

fn blank_titlebar_point() -> gpui::Point<gpui::Pixels> {
    // Above the controls on every platform, inside the transparent titlebar surface.
    point(px(600.0), px(2.0))
}

fn control_ids(platform: FrameTitlebarPlatform) -> Vec<&'static str> {
    let mut ids = vec![
        "titlebar-workspace",
        "titlebar-logs",
        "titlebar-settings",
        "titlebar-add-source",
        "titlebar-start",
    ];
    match platform {
        FrameTitlebarPlatform::Macos => {}
        FrameTitlebarPlatform::Windows => ids.extend([
            "titlebar-windows-minimize",
            "titlebar-windows-maximize",
            "titlebar-windows-close",
        ]),
        FrameTitlebarPlatform::Linux => ids.extend([
            "titlebar-linux-minimize",
            "titlebar-linux-maximize",
            "titlebar-linux-close",
        ]),
    }
    ids
}

#[test]
fn presses_on_titlebar_controls_never_start_window_dragging() {
    for platform in [
        FrameTitlebarPlatform::Macos,
        FrameTitlebarPlatform::Linux,
        FrameTitlebarPlatform::Windows,
    ] {
        let mut app = TestAppContext::single();
        let (root, cx) = titlebar_window(&mut app, platform, false);
        for id in control_ids(platform) {
            for click_count in [1, 2] {
                let center = cx
                    .debug_bounds(id)
                    .unwrap_or_else(|| panic!("missing {id}"))
                    .center();
                cx.simulate_event(gpui::MouseDownEvent {
                    position: center,
                    button: MouseButton::Left,
                    modifiers: Modifiers::default(),
                    click_count,
                    first_mouse: false,
                });
                assert!(
                    !drag_is_armed(&root, cx),
                    "{platform:?}: {id}, click {click_count}"
                );
                // Leaving a pressed button must not hand the ongoing gesture to the titlebar.
                // TestWindow::start_window_move panics, so this also catches accidental moves.
                cx.simulate_mouse_move(
                    blank_titlebar_point(),
                    MouseButton::Left,
                    Modifiers::default(),
                );
                cx.simulate_mouse_up(
                    blank_titlebar_point(),
                    MouseButton::Left,
                    Modifiers::default(),
                );
                assert!(!drag_is_armed(&root, cx));
            }
        }
    }
}

#[test]
fn disabled_start_cannot_reuse_an_armed_titlebar_drag() {
    for platform in [
        FrameTitlebarPlatform::Macos,
        FrameTitlebarPlatform::Linux,
        FrameTitlebarPlatform::Windows,
    ] {
        let mut app = TestAppContext::single();
        let (root, cx) = titlebar_window(&mut app, platform, true);
        cx.simulate_mouse_down(
            blank_titlebar_point(),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert!(drag_is_armed(&root, cx));
        // Model a missed release outside the window followed by a new press on a control.
        let center = cx.debug_bounds("titlebar-start").unwrap().center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        assert!(!drag_is_armed(&root, cx));
        cx.simulate_mouse_move(
            blank_titlebar_point(),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            blank_titlebar_point(),
            MouseButton::Left,
            Modifiers::default(),
        );
    }
}

#[test]
fn blank_titlebar_press_is_cancelled_by_release_inside_or_outside() {
    let mut app = TestAppContext::single();
    let (root, cx) = titlebar_window(&mut app, FrameTitlebarPlatform::Macos, false);
    for release in [blank_titlebar_point(), point(px(600.0), px(200.0))] {
        cx.simulate_mouse_down(
            blank_titlebar_point(),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert!(drag_is_armed(&root, cx));
        cx.simulate_mouse_up(release, MouseButton::Left, Modifiers::default());
        assert!(!drag_is_armed(&root, cx));
        cx.simulate_mouse_move(blank_titlebar_point(), None, Modifiers::default());
    }
}

#[test]
fn macos_blank_titlebar_double_click_does_not_arm_a_drag() {
    let mut app = TestAppContext::single();
    let (root, cx) = titlebar_window(&mut app, FrameTitlebarPlatform::Macos, false);
    cx.simulate_event(gpui::MouseDownEvent {
        position: blank_titlebar_point(),
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 2,
        first_mouse: false,
    });
    assert!(!drag_is_armed(&root, cx));
    cx.simulate_mouse_move(
        blank_titlebar_point(),
        MouseButton::Left,
        Modifiers::default(),
    );
}

#[test]
fn titlebar_hover_without_a_pressed_button_cancels_a_missed_release() {
    let mut app = TestAppContext::single();
    let (root, cx) = titlebar_window(&mut app, FrameTitlebarPlatform::Macos, false);
    cx.simulate_mouse_down(
        blank_titlebar_point(),
        MouseButton::Left,
        Modifiers::default(),
    );
    assert!(drag_is_armed(&root, cx));
    cx.simulate_mouse_move(blank_titlebar_point(), None, Modifiers::default());
    assert!(!drag_is_armed(&root, cx));
    cx.simulate_mouse_down(
        blank_titlebar_point(),
        MouseButton::Right,
        Modifiers::default(),
    );
    assert!(!drag_is_armed(&root, cx));
}

#[test]
fn titlebar_buttons_and_tabs_keep_click_and_keyboard_activation() {
    let mut app = TestAppContext::single();
    let (root, cx) = titlebar_window(&mut app, FrameTitlebarPlatform::Macos, false);
    let settings = cx.debug_bounds("titlebar-settings").unwrap().center();
    cx.simulate_click(settings, Modifiers::default());
    assert!(root.read_with(cx, |root, _| root.settings_ui.is_open));
    assert!(!drag_is_armed(&root, cx));
    cx.simulate_click(settings, Modifiers::default());
    assert!(!root.read_with(cx, |root, _| root.settings_ui.is_open));
    let logs = cx.debug_bounds("titlebar-logs").unwrap().center();
    cx.simulate_click(logs, Modifiers::default());
    assert_eq!(
        root.read_with(cx, |root, _| root.active_view),
        ActiveView::Logs
    );
    assert!(!drag_is_armed(&root, cx));
    draw(cx);
    // Start keyboard navigation from a fresh window, independent of mouse focus policy.
    let mut app = TestAppContext::single();
    let (root, cx) = titlebar_window(&mut app, FrameTitlebarPlatform::Macos, false);
    cx.simulate_keystrokes("tab end");
    assert_eq!(
        root.read_with(cx, |root, _| root.active_view),
        ActiveView::Logs
    );
    cx.simulate_keystrokes("home");
    assert_eq!(
        root.read_with(cx, |root, _| root.active_view),
        ActiveView::Workspace
    );
    cx.simulate_keystrokes("end");
    assert_eq!(
        root.read_with(cx, |root, _| root.active_view),
        ActiveView::Logs
    );
}
