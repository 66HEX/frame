//! Native geometry and modal interactions of the Mira settings sheet.
use super::*;
use crate::appearance::ColorTheme;

fn near(actual: Pixels, expected: f32) {
    assert!(
        (actual.as_f32() - expected).abs() < 0.01,
        "expected {expected}, got {actual:?}"
    );
}

#[test]
fn settings_sheet_meets_viewport_edges_with_the_popover_surface() {
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
                let inset = 0.0;
                near(panel.top(), inset);
                near(viewport.width - panel.right(), inset);
                near(viewport.height - panel.bottom(), inset);
                near(
                    panel.size.width,
                    if width >= 640.0 * scale.factor() {
                        (384.0 * scale.factor()).min(width * 0.75)
                    } else {
                        width * 0.75
                    },
                );
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
                        gpui::Background::from(crate::app::primitives::color(
                            palette.token("popover")
                        )),
                        "Mira Sheet uses flat popover fill"
                    );
                    assert_eq!(
                        background.corner_radii,
                        gpui::Corners::all(gpui::ScaledPixels(0.0))
                    );
                    assert_eq!(
                        background.bounds.intersect(&background.content_mask.bounds),
                        background.bounds,
                        "the full-height sheet remains inside the window"
                    );
                });
                let close = cx.debug_bounds("app-settings-close").unwrap();
                near(close.top() - panel.top(), 16.0 * scale.factor());
                near(panel.right() - close.right(), 16.0 * scale.factor());
                cx.simulate_click(close.center(), Modifiers::default());
                draw(cx);
                assert!(!root.read_with(cx, |root, _| root.settings_ui.is_open));
            }
        }
    }
}

#[test]
fn settings_sheet_backdrop_dismisses_and_restores_the_settings_trigger() {
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
    press(cx, "enter");
    assert!(root.read_with(cx, |root, _| root.settings_ui.is_open));
    let panel = cx.debug_bounds("app-settings-panel").unwrap();
    let position = point(panel.left() / 2.0, panel.center().y);
    assert!(!panel.contains(&position));
    cx.simulate_click(position, Modifiers::default());
    draw(cx);
    assert!(!root.read_with(cx, |root, _| root.settings_ui.is_open));
    assert_eq!(focus(cx), trigger, "focus returns after backdrop click");
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
