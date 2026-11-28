use super::FrameStyle;
use super::frame_dialog_close_button;
use crate::app::primitives::color;
use crate::{appearance::ColorTheme, theme};
use gpui::{Styled, div, px};

#[test]
fn dialogs_use_the_settings_drawer_shadow_and_highlight() {
    for mode in ColorTheme::ALL {
        let palette = theme::palette(mode);
        let mut panel = div().dialog_surface(palette);
        let style = panel.style();
        assert_eq!(
            style.box_shadow.as_deref(),
            Some(theme::shadows::ShadowLevel::Lg.shadows().as_slice())
        );
        assert_eq!(style.ring.width, None);
        assert_eq!(
            style.background,
            Some(theme::shadows::surface_fill(palette.surface, palette).into())
        );
    }
}

use super::{frame_dialog_footer, frame_dialog_header, frame_dialog_overlay, frame_dialog_panel};
use crate::app::motion::{DialogPresence, surface_motion};
use gpui::prelude::*;
use gpui::{Context, FocusHandle, Modifiers, TestAppContext, VisualTestContext, Window, size};
use std::time::{Duration, Instant};

struct DialogProbe {
    presence: DialogPresence,
    focus: FocusHandle,
    primary_focus: FocusHandle,
    close_focus: FocusHandle,
    closes: usize,
    title: String,
    description: String,
    progress: f32,
    group_opacity: f32,
    activations: usize,
    theme: ColorTheme,
    rem: f32,
}

impl gpui::Render for DialogProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(px(self.rem));
        if !self.presence.is_present() {
            return div().size_full().into_any_element();
        }
        let open = self.presence.is_open();
        self.progress = surface_motion("mira-dialog-probe-motion", open, window, cx);
        if self.presence.finish_close(self.progress) {
            return div().size_full().into_any_element();
        }
        let palette = theme::palette(self.theme);
        let primary = div()
            .id("mira-primary")
            .debug_selector(|| "mira-primary".into())
            .role(gpui::Role::Button)
            .aria_label("Primary")
            .track_focus(&self.primary_focus)
            .tab_stop(open)
            .disabled(!open)
            .min_w(theme::ui_rem(60.0))
            .h(theme::ui_rem(28.0))
            .on_click(cx.listener(|view, _, _, _| view.activations += 1))
            .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, _, _| {
                if event.keystroke.key == "enter" {
                    view.activations += 1;
                }
            }));
        let panel = frame_dialog_panel(
            "mira-panel",
            &self.title,
            Some(&self.description),
            &self.focus,
            palette,
            window,
        )
        .child(
            frame_dialog_header(&self.title, Some(&self.description), palette)
                .id("mira-header")
                .debug_selector(|| "mira-header".into()),
        )
        .child(
            div()
                .id("mira-body")
                .debug_selector(|| "mira-body".into())
                .h(theme::ui_rem(40.0)),
        )
        .child(
            frame_dialog_footer(palette, window)
                .id("mira-footer")
                .debug_selector(|| "mira-footer".into())
                .child(
                    div()
                        .id("mira-secondary")
                        .debug_selector(|| "mira-secondary".into())
                        .min_w(theme::ui_rem(60.0))
                        .h(theme::ui_rem(28.0)),
                )
                .child(primary),
        )
        .child(
            frame_dialog_close_button(
                "mira-close",
                "Close dialog",
                open,
                &self.close_focus,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|view, _, _, _| {
                view.closes += 1;
                view.presence.close();
            })),
        );

        let mut overlay =
            frame_dialog_overlay("mira-overlay", panel, self.progress, open, palette, window);
        self.group_opacity = overlay.style().opacity.unwrap_or(1.0);
        div().size_full().child(overlay).into_any_element()
    }
}

fn setup_dialog(
    app: &mut TestAppContext,
    mode: ColorTheme,
    rem: f32,
) -> (gpui::Entity<DialogProbe>, &mut VisualTestContext, Instant) {
    let now = Instant::now();
    app.set_transition_time(now);
    let (view, cx) = app.add_window_view(|_, cx| DialogProbe {
        presence: DialogPresence::default(),
        focus: cx.focus_handle(),
        primary_focus: cx.focus_handle(),
        close_focus: cx.focus_handle(),
        closes: 0,
        title: "Example".into(),
        description: "Description".into(),
        progress: 0.0,
        group_opacity: 0.0,
        activations: 0,
        theme: mode,
        rem,
    });
    cx.update(|window, _| window.activate());
    (view, cx, now)
}

fn draw_dialog(cx: &mut VisualTestContext, now: Instant) {
    cx.set_transition_time(now);
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn mira_dialog_layout_preserves_reference_spacing_and_primary_right() {
    for mode in ColorTheme::ALL {
        for rem in [16.0, 20.0] {
            let mut app = TestAppContext::single();
            let (view, cx, now) = setup_dialog(&mut app, mode, rem);
            cx.simulate_resize(size(px(1000.0), px(800.0)));
            view.update(cx, |view, cx| {
                cx.notify();
                view.presence.open();
            });
            draw_dialog(cx, now);
            draw_dialog(cx, now + Duration::from_millis(100));
            let panel = cx.debug_bounds("mira-panel").unwrap();
            let header = cx.debug_bounds("mira-header").unwrap();
            let body = cx.debug_bounds("mira-body").unwrap();
            let footer = cx.debug_bounds("mira-footer").unwrap();
            let primary = cx.debug_bounds("mira-primary").unwrap();
            let secondary = cx.debug_bounds("mira-secondary").unwrap();
            let scale = rem / 16.0;
            near(panel.size.width.as_f32(), 384.0 * scale);
            near((header.left() - panel.left()).as_f32(), 16.0 * scale);
            near((header.top() - panel.top()).as_f32(), 16.0 * scale);
            assert!(
                43.5_f32.mul_add(-scale, header.size.height.as_f32()).abs() <= 0.5,
                "native text line boxes may round by at most half a logical pixel"
            );
            near((body.top() - header.bottom()).as_f32(), 16.0 * scale);
            near((footer.top() - body.bottom()).as_f32(), 16.0 * scale);
            near((panel.right() - primary.right()).as_f32(), 16.0 * scale);
            near((primary.left() - secondary.right()).as_f32(), 8.0 * scale);
            near((panel.bottom() - footer.bottom()).as_f32(), 0.0);
            near((footer.bottom() - primary.bottom()).as_f32(), 16.0 * scale);
            near((footer.left() - panel.left()).as_f32(), 0.0);
            near(footer.size.width.as_f32(), panel.size.width.as_f32());
            assert!(primary.left() > secondary.left());
            cx.simulate_resize(size(px(500.0), px(800.0)));
            draw_dialog(cx, now + Duration::from_millis(100));
            let panel = cx.debug_bounds("mira-panel").unwrap();
            let primary = cx.debug_bounds("mira-primary").unwrap();
            let secondary = cx.debug_bounds("mira-secondary").unwrap();
            near(panel.size.width.as_f32(), 32.0_f32.mul_add(-scale, 500.0));
            near((secondary.top() - primary.bottom()).as_f32(), 8.0 * scale);
        }
    }
}

#[test]
fn mira_dialog_retains_exit_and_reverses_without_restarting_motion() {
    let mut app = TestAppContext::single();
    let (view, cx, now) = setup_dialog(&mut app, ColorTheme::Dark, 16.0);
    view.update(cx, |view, cx| {
        cx.notify();
        view.presence.open();
    });
    draw_dialog(cx, now);
    near(view.read_with(cx, |view, _| view.progress), 0.0);
    draw_dialog(cx, now + Duration::from_millis(50));
    near(view.read_with(cx, |view, _| view.progress), 0.802_403_4);
    draw_dialog(cx, now + Duration::from_millis(100));
    near(view.read_with(cx, |view, _| view.progress), 1.0);
    view.update(cx, |view, cx| {
        cx.notify();
        view.presence.close();
    });
    draw_dialog(cx, now + Duration::from_millis(100));
    draw_dialog(cx, now + Duration::from_millis(150));
    near(
        view.read_with(cx, |view, _| view.progress),
        1.0 - 0.802_403_4,
    );
    assert!(cx.debug_bounds("mira-panel").is_some());
    view.update(cx, |view, cx| {
        cx.notify();
        view.presence.open();
    });
    draw_dialog(cx, now + Duration::from_millis(150));
    assert!(view.read_with(cx, |view, _| view.progress) > 0.0);
    draw_dialog(cx, now + Duration::from_millis(250));
    near(view.read_with(cx, |view, _| view.progress), 1.0);
    view.update(cx, |view, cx| {
        cx.notify();
        view.presence.close();
    });
    draw_dialog(cx, now + Duration::from_millis(250));
    draw_dialog(cx, now + Duration::from_millis(350));
    assert!(!view.read_with(cx, |view, _| view.presence.is_present()));
    assert!(cx.debug_bounds("mira-panel").is_none());
}

#[test]
fn mira_dialog_exit_blocks_mouse_keyboard_and_accessible_actions() {
    let mut app = TestAppContext::single();
    let (view, cx, now) = setup_dialog(&mut app, ColorTheme::Dark, 16.0);
    view.update(cx, |view, cx| {
        cx.notify();
        view.presence.open();
    });
    draw_dialog(cx, now);
    draw_dialog(cx, now + Duration::from_millis(100));
    let position = cx.debug_bounds("mira-primary").unwrap().center();
    cx.simulate_click(position, Modifiers::default());
    assert_eq!(view.read_with(cx, |view, _| view.activations), 1);
    view.update(cx, |view, cx| {
        cx.notify();
        view.presence.close();
    });
    draw_dialog(cx, now + Duration::from_millis(100));
    cx.simulate_click(position, Modifiers::default());
    cx.simulate_keystrokes("enter");
    assert_eq!(view.read_with(cx, |view, _| view.activations), 1);
    cx.update(|window, cx| {
        window.set_a11y_forced(true);
        window.draw(cx).clear(cx);
        let tree = window.a11y_tree().unwrap();
        let (_, primary) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Primary"))
            .unwrap();
        assert!(primary.is_disabled());
        assert!(!primary.supports_action(gpui::accesskit::Action::Click));
    });
}

#[test]
fn mira_dialog_reduced_motion_settles_open_and_close_in_the_same_draw() {
    let mut app = TestAppContext::single();
    let (view, cx, now) = setup_dialog(&mut app, ColorTheme::Dark, 16.0);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    for _ in 0..2 {
        view.update(cx, |view, cx| {
            cx.notify();
            view.presence.open();
        });
        draw_dialog(cx, now);
        near(view.read_with(cx, |view, _| view.progress), 1.0);
        assert!(cx.debug_bounds("mira-panel").is_some());
        view.update(cx, |view, cx| {
            cx.notify();
            view.presence.close();
        });
        draw_dialog(cx, now);
        near(view.read_with(cx, |view, _| view.progress), 0.0);
        assert!(!view.read_with(cx, |view, _| view.presence.is_present()));
        assert!(cx.debug_bounds("mira-panel").is_none());
    }
}

#[test]
fn mira_dialog_footer_matches_requested_muted_full_bleed_strip() {
    let mut app = TestAppContext::single();
    let (_, cx, _) = setup_dialog(&mut app, ColorTheme::Dark, 16.0);
    for mode in ColorTheme::ALL {
        let palette = theme::palette(mode);
        cx.update(|window, _| {
            let mut footer = frame_dialog_footer(palette, window);
            assert_eq!(
                footer.style().background,
                Some(color(palette.token("muted").with_opacity(0.5)).into())
            );
            assert_eq!(footer.style().border_widths.top, Some(px(1.0).into()));
        });
    }
}

#[test]
fn mira_dialog_close_stays_at_its_anchor_while_pressed_and_activates_once() {
    for rem in [16.0, 32.0] {
        let mut app = TestAppContext::single();
        let (view, cx, now) = setup_dialog(&mut app, ColorTheme::Dark, rem);
        cx.simulate_resize(size(px(1000.0), px(800.0)));
        view.update(cx, |view, cx| {
            cx.notify();
            view.presence.open();
        });
        draw_dialog(cx, now);
        draw_dialog(cx, now + Duration::from_millis(100));
        let original = cx.debug_bounds("mira-close").unwrap();
        cx.simulate_mouse_move(original.center(), None, Modifiers::default());
        cx.simulate_mouse_down(
            original.center(),
            gpui::MouseButton::Left,
            Modifiers::default(),
        );
        draw_dialog(cx, now + Duration::from_millis(100));
        let pressed = cx.debug_bounds("mira-close").unwrap();
        near(pressed.left().as_f32(), original.left().as_f32());
        // The control's top transition may still be at its initial sample. It
        // must never lose its absolute anchor and jump into the flex flow.
        assert!((pressed.top() - original.top()).as_f32().abs() <= 1.0);
        cx.simulate_mouse_up(
            original.center(),
            gpui::MouseButton::Left,
            Modifiers::default(),
        );
        draw_dialog(cx, now + Duration::from_millis(100));
        assert_eq!(view.read_with(cx, |view, _| view.closes), 1);
        assert!(!view.read_with(cx, |view, _| view.presence.is_open()));
        cx.simulate_click(original.center(), Modifiers::default());
        assert_eq!(view.read_with(cx, |view, _| view.closes), 1);
    }
}

#[test]
fn mira_dialog_long_text_container_is_bounded_and_reserves_close_area() {
    let mut app = TestAppContext::single();
    let (view, cx, now) = setup_dialog(&mut app, ColorTheme::Dark, 32.0);
    cx.simulate_resize(size(px(700.0), px(500.0)));
    view.update(cx, |view, cx| {
        cx.notify();
        view.title = "very_long_filename_".repeat(18);
        view.description = "A long diagnostic that must remain inside the dialog. ".repeat(20);
        view.presence.open();
    });
    draw_dialog(cx, now);
    draw_dialog(cx, now + Duration::from_millis(100));
    let panel = cx.debug_bounds("mira-panel").unwrap();
    let header = cx.debug_bounds("mira-header").unwrap();
    let close = cx.debug_bounds("mira-close").unwrap();
    assert!(panel.left() >= px(32.0) && panel.right() <= px(668.0));
    assert!(panel.top() >= px(32.0) && panel.bottom() <= px(468.0));
    // TestTextSystem ignores wrap_width and emits one VisualLine for any
    // string. This test proves container geometry, not native text wrapping.
    assert!(header.left() >= panel.left() && header.right() <= panel.right());
    assert!(header.right() - px(48.0) < close.left());
    assert!(close.top() >= panel.top() && close.bottom() <= panel.bottom());
}

#[test]
fn mira_dialog_painted_backdrop_and_panel_follow_the_same_fade_and_scale() {
    let mut app = TestAppContext::single();
    let (view, cx, now) = setup_dialog(&mut app, ColorTheme::Dark, 16.0);
    cx.simulate_resize(size(px(1000.0), px(800.0)));
    view.update(cx, |view, cx| {
        cx.notify();
        view.presence.open();
    });
    draw_dialog(cx, now);
    draw_dialog(cx, now + Duration::from_millis(50));
    near(
        view.read_with(cx, |view, _| view.group_opacity),
        0.802_403_4,
    );
    let center = cx.debug_bounds("mira-panel").unwrap().center();
    assert!((center.y.as_f32() - 400.0).abs() <= 0.5);
    cx.update(|window, _| {
        let scale = window.scale_factor();
        let quads = window.painted_quads();
        let backdrop = quads
            .iter()
            .find(|quad| 1000.0_f32.mul_add(-scale, quad.bounds.size.width.0).abs() < 0.01)
            .unwrap();
        let rgba = gpui::hsla_to_rgba(backdrop.background.as_solid().unwrap());
        // GPUI composites the entire overlay through an opacity boundary; raw
        // scene quads retain their unmultiplied alpha.
        near(rgba.alpha, 0.8);
        let expected_width = 384.0 * 0.05_f32.mul_add(0.802_403_4, 0.95) * scale;
        let panel = quads
            .iter()
            .find(|quad| (quad.bounds.size.width.0 - expected_width).abs() < 0.01)
            .unwrap();
        near(panel.bounds.center().x.0 / scale, center.x.as_f32());
        near(panel.bounds.center().y.0 / scale, center.y.as_f32());
        let palette = theme::palette(ColorTheme::Dark);
        assert_eq!(
            panel.background,
            theme::shadows::surface_fill(palette.surface, palette)
        );
    });
}
