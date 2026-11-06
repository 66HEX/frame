//! Native geometry and modal interactions of the floating settings drawer.
use super::*;
use crate::appearance::ColorTheme;

fn near(actual: Pixels, expected: f32) {
    assert!(
        (actual.as_f32() - expected).abs() < 0.01,
        "expected {expected}, got {actual:?}"
    );
}

#[test]
fn settings_drawer_floats_inside_the_viewport_with_the_card_surface() {
    for mode in ColorTheme::ALL {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            for width in [1200.0_f32, 800.0] {
                let mut app = TestAppContext::single();
                let (root, cx) = test_window(&mut app, VisualFixture::AppSettings, scale);
                root.update(cx, |root, cx| {
                    root.appearance.color_theme = mode;
                    cx.notify();
                });
                let viewport = size(px(width), px(900.0));
                cx.simulate_resize(viewport);
                draw(cx);
                let panel = cx.debug_bounds("app-settings-panel").unwrap();
                let body = cx.debug_bounds("app-settings-scroll-body").unwrap();
                let inset = 8.0 * scale.factor();
                near(panel.top(), inset);
                near(viewport.width - panel.right(), inset);
                near(viewport.height - panel.bottom(), inset);
                near(panel.size.width, (384.0 * scale.factor()).min(width * 0.75));
                assert!(panel.left() > px(0.0));
                assert_eq!(body.intersect(&panel), body);

                cx.update(|window, _| {
                    let native_scale = window.scale_factor();
                    let expected = panel.scale(native_scale);
                    let palette = theme::palette(mode);
                    let background = window
                        .painted_quads()
                        .into_iter()
                        .find(|quad| quad.bounds == expected && !quad.background.is_transparent())
                        .expect("the drawer must paint a visible surface");
                    assert_eq!(
                        background.background,
                        theme::shadows::surface_fill(palette.surface, palette),
                        "settings use the card token including its surface light"
                    );
                    assert_ne!(
                        background.background,
                        theme::shadows::surface_fill(palette.surface_elevated, palette),
                        "the old popover fill must not remain"
                    );
                    let radius =
                        gpui::ScaledPixels(theme::RADIUS_XL * scale.factor() * native_scale);
                    assert_eq!(background.corner_radii, gpui::Corners::all(radius));
                    assert_eq!(
                        background.bounds.intersect(&background.content_mask.bounds),
                        background.bounds,
                        "all four rounded corners must remain inside the window"
                    );
                });
            }
        }
    }
}

#[test]
fn settings_drawer_exposed_margins_dismiss_and_restore_the_settings_trigger() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::SettingsVideo,
        ScalePreset::Percent100,
    );
    let viewport = size(px(1200.0), px(800.0));
    cx.simulate_resize(viewport);
    draw(cx);
    click(cx, "titlebar-settings");
    press(cx, "escape");
    let trigger = focus(cx);
    for edge in ["top", "right", "bottom"] {
        press(cx, "enter");
        assert!(root.read_with(cx, |root, _| root.settings_ui.is_open));
        let panel = cx.debug_bounds("app-settings-panel").unwrap();
        let position = match edge {
            "top" => point(panel.center().x, px(4.0)),
            "right" => point(viewport.width - px(4.0), panel.center().y),
            _ => point(panel.center().x, viewport.height - px(4.0)),
        };
        assert!(!panel.contains(&position));
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
        assert!(!root.read_with(cx, |root, _| root.settings_ui.is_open));
        assert_eq!(
            focus(cx),
            trigger,
            "focus must return after clicking {edge}"
        );
    }
}

#[test]
fn settings_drawer_escape_closes_the_nested_select_before_the_drawer() {
    let mut app = TestAppContext::single();
    let (root, cx) = test_window(
        &mut app,
        VisualFixture::SettingsVideo,
        ScalePreset::Percent100,
    );
    cx.update(|window, _| window.set_a11y_forced(true));
    click(cx, "titlebar-settings");
    press(cx, "escape");
    let trigger = focus(cx);
    press(cx, "enter");
    click(cx, "app-settings-theme");
    assert!(root.read_with(cx, |root, _| root.settings_ui.theme_popover.is_open()));
    press(cx, "escape");
    assert!(root.read_with(cx, |root, _| root.settings_ui.is_open));
    assert!(!root.read_with(cx, |root, _| root.settings_ui.theme_popover.is_open()));
    assert_eq!(keyboard_tests::focused_node(cx)["aria"]["label"], "Theme");
    press(cx, "escape");
    assert!(!root.read_with(cx, |root, _| root.settings_ui.is_open));
    assert_eq!(focus(cx), trigger);
}
