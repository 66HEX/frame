//! Shared, interruptible icon changes. Each glyph keeps its own transition so
//! rapidly changing actions can reverse without replacing the outgoing glyph.
use crate::app::motion::contextual_icon_motion;
use crate::theme;
use gpui::prelude::*;
use gpui::{App, Rgba, Transformation, Window, div, px, size, svg};

#[derive(Clone, Copy)]
struct IconLayer {
    path: &'static str,
    color: Option<Rgba>,
    initially_visible: bool,
}

#[derive(Clone, Default)]
struct IconLayers(Vec<IconLayer>);

pub(in crate::app) fn frame_animated_icon(
    id: impl Into<String>,
    icon: Option<&'static str>,
    icon_size: f32,
    icon_color: Option<Rgba>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Div {
    let id = id.into();
    let layers = window.use_keyed_state(format!("{id}-icon-layers"), cx, |_, _| {
        IconLayers(
            icon.map(|path| IconLayer {
                path,
                color: icon_color,
                initially_visible: true,
            })
            .into_iter()
            .collect(),
        )
    });
    let layers = layers.update(cx, |layers, _| {
        if let Some(path) = icon {
            if let Some(layer) = layers.0.iter_mut().find(|layer| layer.path == path) {
                layer.color = icon_color;
            } else {
                layers.0.push(IconLayer {
                    path,
                    color: icon_color,
                    initially_visible: false,
                });
            }
        }
        layers.0.clone()
    });
    let mut container = div().relative().flex_none().size(theme::ui_rem(icon_size));
    for layer in layers {
        let progress = contextual_icon_motion(
            format!("{id}-icon-motion-{}", layer.path),
            icon == Some(layer.path),
            layer.initially_visible,
            window,
            cx,
        );
        let visuals = contextual_icon_visuals(progress);
        container = container.child(
            svg()
                .absolute()
                .inset_0()
                .path(layer.path)
                .size(theme::ui_rem(icon_size))
                .when_some(layer.color, gpui::Styled::text_color)
                .opacity(visuals.opacity)
                .blur(px(visuals.blur_radius))
                .with_transformation(Transformation::scale(size(visuals.scale, visuals.scale))),
        );
    }
    container
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ContextualIconVisuals {
    opacity: f32,
    scale: f32,
    blur_radius: f32,
}

fn contextual_icon_visuals(progress: f32) -> ContextualIconVisuals {
    let progress = progress.clamp(0.0, 1.0);
    ContextualIconVisuals {
        opacity: progress,
        scale: progress.mul_add(0.75, 0.25),
        blur_radius: (1.0 - progress) * 4.0,
    }
}

#[cfg(test)]
#[path = "animated_icon_tests.rs"]
mod native_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contextual_icon_visuals_use_hidden_endpoint_for_zero_progress() {
        assert_eq!(
            contextual_icon_visuals(0.0),
            ContextualIconVisuals {
                opacity: 0.0,
                scale: 0.25,
                blur_radius: 4.0,
            }
        );
    }

    #[test]
    fn contextual_icon_visuals_use_visible_endpoint_for_full_progress() {
        assert_eq!(
            contextual_icon_visuals(1.0),
            ContextualIconVisuals {
                opacity: 1.0,
                scale: 1.0,
                blur_radius: 0.0,
            }
        );
    }
}
