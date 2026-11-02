//! Hit testing for rectangular resize handles.

use crate::{Bounds, Edges, Pixels, Point, ResizeEdge, Tiling};

/// Resize handles around a rectangle.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ResizeRegion {
    border_width: Pixels,
    corner_size: Pixels,
    enabled_edges: Edges<bool>,
}

impl ResizeRegion {
    /// Creates handles with the given border width.
    /// Invalid widths become zero; hits must lie inside the bounds.
    pub fn new(border_width: Pixels) -> Self {
        Self {
            border_width: sane(border_width),
            corner_size: sane(border_width),
            enabled_edges: Edges::all(true),
        }
    }

    /// Sets diagonal reach; invalid sizes become zero.
    pub fn corner_size(mut self, corner_size: Pixels) -> Self {
        self.corner_size = sane(corner_size);
        self
    }

    /// Sets which sides can resize.
    pub fn enabled_edges(mut self, enabled_edges: Edges<bool>) -> Self {
        self.enabled_edges = enabled_edges;
        self
    }

    /// Disables handles on tiled sides.
    pub fn without_tiled_edges(mut self, tiling: Tiling) -> Self {
        self.enabled_edges.top &= !tiling.top;
        self.enabled_edges.right &= !tiling.right;
        self.enabled_edges.bottom &= !tiling.bottom;
        self.enabled_edges.left &= !tiling.left;
        self
    }

    /// Returns the nearest enabled handle under `position`.
    /// Bounds use half-open edges.
    pub fn hit_test(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> Option<ResizeEdge> {
        let left = bounds.origin.x.0;
        let top = bounds.origin.y.0;
        let width = bounds.size.width.0;
        let height = bounds.size.height.0;
        let right = left + width;
        let bottom = top + height;
        let x = position.x.0;
        let y = position.y.0;
        let bw = self.border_width.0;
        let corner = self.corner_size.0;

        if !(left.is_finite()
            && top.is_finite()
            && width.is_finite()
            && height.is_finite()
            && x.is_finite()
            && y.is_finite())
            || width <= 0.0
            || height <= 0.0
            || bw <= 0.0
            || !right.is_finite()
            || !bottom.is_finite()
        {
            return None;
        }

        if x < left || x >= right || y < top || y >= bottom {
            return None;
        }

        let top_hit = self.enabled_edges.top
            && y >= top
            && y < (top + bw).min(bottom)
            && x >= left
            && x < right;
        let right_hit = self.enabled_edges.right
            && x >= (right - bw).max(left)
            && x < right
            && y >= top
            && y < bottom;
        let bottom_hit = self.enabled_edges.bottom
            && y >= (bottom - bw).max(top)
            && y < bottom
            && x >= left
            && x < right;
        let left_hit = self.enabled_edges.left
            && x >= left
            && x < (left + bw).min(right)
            && y >= top
            && y < bottom;

        let near_left = x - left < right - x && x - left < corner;
        let near_right = right - x <= x - left && right - x <= corner;
        let near_top = y - top < bottom - y && y - top < corner;
        let near_bottom = bottom - y <= y - top && bottom - y <= corner;
        let in_left_strip = x - left < bw;
        let in_right_strip = right - x <= bw;
        let in_top_strip = y - top < bw;
        let in_bottom_strip = bottom - y <= bw;
        let corner_hit = if near_top
            && near_left
            && (in_top_strip || in_left_strip)
            && self.enabled_edges.top
            && self.enabled_edges.left
        {
            Some(ResizeEdge::TopLeft)
        } else if near_top
            && near_right
            && (in_top_strip || in_right_strip)
            && self.enabled_edges.top
            && self.enabled_edges.right
        {
            Some(ResizeEdge::TopRight)
        } else if near_bottom
            && near_left
            && (in_bottom_strip || in_left_strip)
            && self.enabled_edges.bottom
            && self.enabled_edges.left
        {
            Some(ResizeEdge::BottomLeft)
        } else if near_bottom
            && near_right
            && (in_bottom_strip || in_right_strip)
            && self.enabled_edges.bottom
            && self.enabled_edges.right
        {
            Some(ResizeEdge::BottomRight)
        } else {
            None
        };
        if corner_hit.is_some() {
            return corner_hit;
        }

        // Choose the nearest side when strips overlap.
        let mut best: Option<(f32, ResizeEdge)> = None;
        for (hit, distance, edge) in [
            (top_hit, y - top, ResizeEdge::Top),
            (right_hit, right - x, ResizeEdge::Right),
            (bottom_hit, bottom - y, ResizeEdge::Bottom),
            (left_hit, x - left, ResizeEdge::Left),
        ] {
            if hit && best.map_or(true, |(best_distance, _)| distance < best_distance) {
                best = Some((distance, edge));
            }
        }
        best.map(|(_, edge)| edge)
    }
}

fn sane(value: Pixels) -> Pixels {
    if value.0.is_finite() && value.0 > 0.0 {
        value
    } else {
        Pixels(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CursorStyle, point, px, size};
    use proptest::prelude::*;

    fn bounds() -> Bounds<Pixels> {
        Bounds::new(point(px(10.), px(20.)), size(px(100.), px(80.)))
    }

    fn region() -> ResizeRegion {
        ResizeRegion::new(px(4.)).corner_size(px(8.))
    }

    #[test]
    fn resize_handles_and_cursor_directions_agree() {
        // Offset bounds catch origin-relative hit tests.
        for (x, y, expected) in [
            (60., 20., Some((ResizeEdge::Top, CursorStyle::ResizeUp))),
            (
                109.,
                60.,
                Some((ResizeEdge::Right, CursorStyle::ResizeRight)),
            ),
            (
                60.,
                99.,
                Some((ResizeEdge::Bottom, CursorStyle::ResizeDown)),
            ),
            (10., 60., Some((ResizeEdge::Left, CursorStyle::ResizeLeft))),
            (
                10.,
                20.,
                Some((ResizeEdge::TopLeft, CursorStyle::ResizeUpLeft)),
            ),
            (
                109.,
                20.,
                Some((ResizeEdge::TopRight, CursorStyle::ResizeUpRight)),
            ),
            (
                10.,
                99.,
                Some((ResizeEdge::BottomLeft, CursorStyle::ResizeDownLeft)),
            ),
            (
                109.,
                99.,
                Some((ResizeEdge::BottomRight, CursorStyle::ResizeDownRight)),
            ),
            (60., 60., None),
            (9.75, 60., None),
            (110., 60., None),
            (60., 19.75, None),
            (60., 100., None),
        ] {
            let actual = region().hit_test(point(px(x), px(y)), bounds());
            assert_eq!(actual, expected.map(|(edge, _)| edge), "({x}, {y})");
            assert_eq!(
                actual.map(CursorStyle::from),
                expected.map(|(_, cursor)| cursor)
            );
        }
    }

    #[test]
    fn strip_and_corner_boundaries_follow_half_open_bounds() {
        for (x, y, expected) in [
            (60., 23.75, Some(ResizeEdge::Top)),
            (60., 24., None),
            (106., 60., Some(ResizeEdge::Right)),
            (105.75, 60., None),
            (60., 96., Some(ResizeEdge::Bottom)),
            (60., 95.75, None),
            (13.75, 60., Some(ResizeEdge::Left)),
            (14., 60., None),
            (17.75, 21., Some(ResizeEdge::TopLeft)),
            (18., 21., Some(ResizeEdge::Top)),
            (102., 21., Some(ResizeEdge::TopRight)),
            (101.75, 21., Some(ResizeEdge::Top)),
            (11., 27.75, Some(ResizeEdge::TopLeft)),
            (11., 28., Some(ResizeEdge::Left)),
            (11., 92., Some(ResizeEdge::BottomLeft)),
            (11., 91.75, Some(ResizeEdge::Left)),
            (17., 27., None), // Inside the corner square, outside both strips.
            (103., 93., None),
        ] {
            assert_eq!(
                region().hit_test(point(px(x), px(y)), bounds()),
                expected,
                "({x}, {y})"
            );
        }
    }

    fn edges(mask: u8) -> Edges<bool> {
        Edges {
            top: mask & 1 != 0,
            right: mask & 2 != 0,
            bottom: mask & 4 != 0,
            left: mask & 8 != 0,
        }
    }

    #[test]
    fn every_edge_mask_disables_sides_and_demotes_corners() {
        for mask in 0..16 {
            let region = region().enabled_edges(edges(mask));
            for (x, y, bit, edge) in [
                (60., 21., 1, ResizeEdge::Top),
                (109., 60., 2, ResizeEdge::Right),
                (60., 99., 4, ResizeEdge::Bottom),
                (11., 60., 8, ResizeEdge::Left),
            ] {
                assert_eq!(
                    region.hit_test(point(px(x), px(y)), bounds()),
                    (mask & bit != 0).then_some(edge),
                    "mask={mask}, {edge:?}"
                );
            }
            // These samples hit only the horizontal strip; disabling it leaves a hole.
            for (x, y, horizontal, vertical, side, corner) in [
                (16., 21., 1, 8, ResizeEdge::Top, ResizeEdge::TopLeft),
                (104., 21., 1, 2, ResizeEdge::Top, ResizeEdge::TopRight),
                (16., 99., 4, 8, ResizeEdge::Bottom, ResizeEdge::BottomLeft),
                (104., 99., 4, 2, ResizeEdge::Bottom, ResizeEdge::BottomRight),
            ] {
                let expected = match (mask & horizontal != 0, mask & vertical != 0) {
                    (true, true) => Some(corner),
                    (true, false) => Some(side),
                    (false, _) => None,
                };
                assert_eq!(
                    region.hit_test(point(px(x), px(y)), bounds()),
                    expected,
                    "mask={mask}, {corner:?}"
                );
            }
        }
    }

    #[test]
    fn tiling_intersects_enabled_edges_and_never_reenables_a_side() {
        for enabled in 0..16 {
            for tiled in 0..16 {
                let e = edges(tiled);
                let tiling = Tiling {
                    top: e.top,
                    right: e.right,
                    bottom: e.bottom,
                    left: e.left,
                };
                let actual = region()
                    .enabled_edges(edges(enabled))
                    .without_tiled_edges(tiling);
                let expected = region().enabled_edges(edges(enabled & !tiled));
                for (x, y) in [
                    (60., 21.),
                    (109., 60.),
                    (60., 99.),
                    (11., 60.),
                    (11., 21.),
                    (109., 21.),
                    (11., 99.),
                    (109., 99.),
                ] {
                    let p = point(px(x), px(y));
                    assert_eq!(
                        actual.hit_test(p, bounds()),
                        expected.hit_test(p, bounds()),
                        "enabled={enabled}, tiled={tiled}, {p:?}"
                    );
                }
                assert_eq!(actual.without_tiled_edges(tiling), actual);
            }
        }
    }

    #[test]
    fn overlapping_strips_and_corners_choose_the_nearest_boundaries() {
        let b = Bounds::new(point(px(10.), px(20.)), size(px(2.), px(2.)));
        let sides = ResizeRegion::new(px(5.)).corner_size(px(0.));
        for (x, y, side, corner) in [
            (10.25, 20.5, ResizeEdge::Left, ResizeEdge::TopLeft),
            (11.75, 20.5, ResizeEdge::Right, ResizeEdge::TopRight),
            (10.5, 20.25, ResizeEdge::Top, ResizeEdge::TopLeft),
            (10.5, 21.75, ResizeEdge::Bottom, ResizeEdge::BottomLeft),
            (11.75, 21.5, ResizeEdge::Right, ResizeEdge::BottomRight),
            (10.25, 20.25, ResizeEdge::Top, ResizeEdge::TopLeft),
            (11., 21., ResizeEdge::Top, ResizeEdge::BottomRight),
        ] {
            let p = point(px(x), px(y));
            assert_eq!(sides.hit_test(p, b), Some(side), "{p:?}");
            assert_eq!(
                sides.corner_size(px(5.)).hit_test(p, b),
                Some(corner),
                "{p:?}"
            );
        }
    }

    #[test]
    fn invalid_dimensions_and_nonfinite_coordinates_never_activate_handles() {
        for invalid in [0., -1., f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                ResizeRegion::new(px(invalid)).hit_test(point(px(10.), px(20.)), bounds()),
                None
            );
            // Invalid corner sizes disable diagonals but keep side handles.
            assert_eq!(
                region()
                    .corner_size(px(invalid))
                    .hit_test(point(px(11.), px(21.)), bounds()),
                Some(ResizeEdge::Top)
            );
            for b in [
                Bounds::new(bounds().origin, size(px(invalid), px(80.))),
                Bounds::new(bounds().origin, size(px(100.), px(invalid))),
            ] {
                assert_eq!(region().hit_test(bounds().origin, b), None, "{b:?}");
            }
        }
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for p in [point(px(invalid), px(20.)), point(px(10.), px(invalid))] {
                assert_eq!(region().hit_test(p, bounds()), None);
                assert_eq!(
                    region().hit_test(bounds().origin, Bounds::new(p, bounds().size)),
                    None
                );
            }
        }
        for b in [
            Bounds::new(point(px(f32::MAX), px(0.)), size(px(f32::MAX), px(80.))),
            Bounds::new(point(px(0.), px(f32::MAX)), size(px(100.), px(f32::MAX))),
        ] {
            assert_eq!(
                region().hit_test(b.origin, b),
                None,
                "overflowed bounds: {b:?}"
            );
        }
    }

    proptest! {
        #[test]
        fn translating_a_rectangle_preserves_all_hits(
            x in -8i32..=108, y in -8i32..=88,
            dx in -1000i32..=1000, dy in -1000i32..=1000,
            border in 0u8..=120, corner in 0u8..=120, mask in 0u8..16,
        ) {
            // Quarter pixels keep boundary samples exact.
            let p = point(px(x as f32 / 4.),px(y as f32 / 4.));
            let offset = point(px(dx as f32),px(dy as f32));
            let b = Bounds::new(Point::default(),size(px(25.),px(20.)));
            let r = ResizeRegion::new(px(border as f32 / 4.))
                .corner_size(px(corner as f32 / 4.))
                .enabled_edges(edges(mask));
            prop_assert_eq!(r.hit_test(p,b),r.hit_test(p+offset,Bounds::new(offset,b.size)));
        }
    }
}
