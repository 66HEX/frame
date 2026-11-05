use super::*;
use crate::app::motion::interaction_motion;
use gpui::{Animated, Rgba, RingColor, div};
use std::time::Duration;

fn solid(ring: RingColor) -> Rgba {
    let RingColor::Color(background) = ring else {
        panic!("a focus animation must never fall back to the text color");
    };
    gpui::hsla_to_rgba(background.as_solid().expect("focus ring is a solid color"))
}

fn check_frames(base: RingColor, focused: RingColor) {
    let expected = solid(focused);
    let mut animated = Animated::new(base, interaction_motion());
    // The same Animated<RingColor> path used by GPUI's style transitions.
    // Cover entry, exit, and two reversals before the transition has finished.
    let mut previous = solid(base).alpha;
    for (start_ms, target, frame_count, entering) in [
        (0, focused, 18, true),
        (150, base, 18, false),
        (300, focused, 6, true),
        (350, base, 4, false),
        (384, focused, 18, true),
    ] {
        let start = Duration::from_millis(start_ms);
        let before = animated.sample(start).value;
        animated.set(target, interaction_motion(), start);
        assert_eq!(
            animated.sample(start).value,
            before,
            "retarget must not jump"
        );
        previous = solid(before).alpha;
        for frame in 1..=frame_count {
            let now = start + Duration::from_nanos(frame * 1_000_000_000 / 120);
            let rgba = solid(animated.sample(now).value);
            assert!((rgba.red - expected.red).abs() < 0.0001);
            assert!((rgba.green - expected.green).abs() < 0.0001);
            assert!((rgba.blue - expected.blue).abs() < 0.0001);
            assert!((0.0..=expected.alpha + 0.0001).contains(&rgba.alpha));
            assert!(if entering {
                rgba.alpha >= previous
            } else {
                rgba.alpha <= previous
            });
            previous = rgba.alpha;
        }
    }
    assert!((previous - expected.alpha).abs() < 0.0001);
}

#[test]
fn focus_ring_frames_fade_without_a_text_color_flash() {
    for mode in crate::appearance::ColorTheme::ALL {
        let palette = theme::palette(mode);
        let focused = focus_visible_ring(StyleRefinement::default(), palette)
            .ring
            .color
            .unwrap();
        let composite = composite_focus_ring(StyleRefinement::default(), palette)
            .ring
            .color
            .unwrap();
        for (mut control, target) in [
            (
                apply_accessible_button(div().id("button"), "Button", true, palette),
                focused,
            ),
            (
                apply_accessible_checkbox(
                    div().id("checkbox"),
                    "Checkbox",
                    true,
                    false,
                    false,
                    palette,
                ),
                focused,
            ),
            (
                apply_accessible_toggle_button(div().id("toggle"), "Toggle", true, false, palette),
                composite,
            ),
            (focus_ring_base(div().id("tab"), palette), composite),
        ] {
            // Match GPUI's default when a constructor forgets the base color.
            let base = control.style().ring.color.unwrap_or_default();
            assert!(solid(base).alpha.abs() < f32::EPSILON);
            check_frames(base, target);
        }
    }
}

#[test]
fn focus_ring_base_preserves_explicit_validation_and_surface_rings() {
    for mode in crate::appearance::ColorTheme::ALL {
        let palette = theme::palette(mode);
        for token in [palette.danger.with_opacity(0.4), palette.surface_ring] {
            let mut control = focus_ring_base(
                div()
                    .id("explicit-ring")
                    .ring(px(2.0))
                    .ring_color(color(token)),
                palette,
            );
            assert_eq!(control.style().ring.width, Some(px(2.0)));
            assert_eq!(
                control.style().ring.color,
                Some(RingColor::Color(color(token).into()))
            );
        }
    }
}
