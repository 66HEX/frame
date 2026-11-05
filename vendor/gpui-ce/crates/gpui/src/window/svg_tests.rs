use super::*;
use crate::{ParentElement, Styled, TestAppContext, div, hsla, svg};

const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M0 0h16v16H0z"/></svg>"#;

struct IconView {
    color: Hsla,
}

impl Render for IconView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_4()
            .text_color(self.color)
            .child(svg().data(SVG).size(px(16.0)))
            .child(div().child(svg().data(SVG).size(px(16.0))))
            .child(
                svg()
                    .data(SVG)
                    .size(px(16.0))
                    .text_color(hsla(0.6, 1.0, 0.5, 1.0)),
            )
    }
}

#[crate::test]
fn svg_current_color_inherits_and_updates_while_explicit_colors_win(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, _| IconView {
        color: hsla(0.0, 1.0, 0.5, 1.0),
    });
    for color in [hsla(0.0, 1.0, 0.5, 1.0), hsla(0.3, 0.6, 0.4, 0.7)] {
        view.update(cx, |view, cx| {
            view.color = color;
            cx.notify();
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.update(|window, _| {
            let icons = &window.rendered_frame.scene.monochrome_sprites;
            assert_eq!(
                icons.len(),
                3,
                "an SVG without a local text_color must still paint"
            );
            assert_eq!(icons[0].color, color.into());
            assert_eq!(icons[1].color, color.into());
            assert_eq!(icons[2].color, hsla(0.6, 1.0, 0.5, 1.0).into());
        });
    }
}
