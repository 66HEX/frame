//! Intersection of a rounded scrollbar thumb and its rounded scroll viewport.
//! GPUI only supports rectangular content masks; drawing the intersected outline
//! avoids painting over whatever is behind a floating popover's transparent corners.
use crate::numeric::usize_to_f32;
use gpui::{Bounds, Path, PathBuilder, Pixels, Point, point, px};

// Cosine sampling keeps the polygon dense at the caps' vertical tangents.
// 64 subdivisions across a 7px thumb remain subpixel even at 200% UI scale.
const SEGMENTS: usize = 64;

pub(super) fn thumb_path(
    thumb: Bounds<Pixels>,
    viewport: Bounds<Pixels>,
    radius: Pixels,
) -> Option<Path<Pixels>> {
    let outline = thumb_outline(thumb, viewport, radius);
    let mut builder = PathBuilder::fill();
    builder.move_to(*outline.first()?);
    for point in outline.into_iter().skip(1) {
        builder.line_to(point);
    }
    builder.close();
    builder.build().ok()
}

fn thumb_outline(
    thumb: Bounds<Pixels>,
    viewport: Bounds<Pixels>,
    radius: Pixels,
) -> Vec<Point<Pixels>> {
    let left = thumb.left().max(viewport.left()).as_f32();
    let right = thumb.right().min(viewport.right()).as_f32();
    if left >= right || thumb.intersect(&viewport).is_empty() {
        return Vec::new();
    }
    let thumb_radius = thumb.size.width / 2.0;
    let mut top = Vec::with_capacity(SEGMENTS + 1);
    let mut bottom = Vec::with_capacity(SEGMENTS + 1);
    for index in 0..=SEGMENTS {
        let angle = std::f32::consts::PI * usize_to_f32(index) / usize_to_f32(SEGMENTS);
        let x = (right - left).mul_add((1.0 - angle.cos()) / 2.0, left);
        let (thumb_top, thumb_bottom) = vertical_span(thumb, thumb_radius, x);
        let (clip_top, clip_bottom) = vertical_span(viewport, radius, x);
        let y0 = thumb_top.max(clip_top);
        let y1 = thumb_bottom.min(clip_bottom);
        if y0 <= y1 {
            top.push(point(px(x), px(y0)));
            bottom.push(point(px(x), px(y1)));
        }
    }
    top.extend(bottom.into_iter().rev());
    top
}

fn vertical_span(bounds: Bounds<Pixels>, radius: Pixels, x: f32) -> (f32, f32) {
    let radius = radius
        .min(bounds.size.width / 2.0)
        .min(bounds.size.height / 2.0)
        .max(px(0.0))
        .as_f32();
    let distance = (bounds.left().as_f32() + radius - x)
        .max(x - (bounds.right().as_f32() - radius))
        .max(0.0);
    let inset = radius - radius.mul_add(radius, -distance * distance).max(0.0).sqrt();
    (
        bounds.top().as_f32() + inset,
        bounds.bottom().as_f32() - inset,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::size;

    #[test]
    fn both_thumb_extremes_follow_the_popover_curve_instead_of_protruding() {
        for scale in [1.0, 2.0] {
            let viewport = Bounds::new(
                point(px(40.0), px(60.0)),
                size(px(180.0 * scale), px(192.0 * scale)),
            );
            let radius = px(10.0 * scale);
            for bottom in [false, true] {
                let height = px(72.0 * scale);
                let thumb = Bounds::new(
                    point(
                        viewport.right() - px(8.5 * scale),
                        if bottom {
                            viewport.bottom() - height
                        } else {
                            viewport.top()
                        },
                    ),
                    size(px(7.0 * scale), height),
                );
                let outline = thumb_outline(thumb, viewport, radius);
                assert!(!outline.is_empty());
                let center = point(
                    viewport.right() - radius,
                    if bottom {
                        viewport.bottom() - radius
                    } else {
                        viewport.top() + radius
                    },
                );
                for point in &outline {
                    let dy = point.y - center.y;
                    if (bottom && dy > px(0.0)) || (!bottom && dy < px(0.0)) {
                        let dx = point.x - center.x;
                        assert!(
                            dx.as_f32().mul_add(dx.as_f32(), dy.as_f32().powi(2))
                                <= radius.as_f32().mul_add(radius.as_f32(), 0.01)
                        );
                    }
                }
                assert!(outline.iter().all(|p| if bottom {
                    p.y < viewport.bottom()
                } else {
                    p.y > viewport.top()
                }));
            }
        }
    }
}
