//! Pointer corridor used by Radix Tooltip's hoverable content.
//! See @radix-ui/react-tooltip (MIT), getPaddedExitPoints/getHull.
use gpui::{Bounds, Pixels, Point, point};

pub(super) struct TooltipGraceArea(Vec<Point<f32>>);

impl TooltipGraceArea {
    pub fn new(exit: Point<Pixels>, source: Bounds<Pixels>, target: Bounds<Pixels>) -> Self {
        let exit = point(exit.x.as_f32(), exit.y.as_f32());
        let distances = [
            (exit.x - source.left().as_f32()).abs(),
            (exit.x - source.right().as_f32()).abs(),
            (exit.y - source.top().as_f32()).abs(),
            (exit.y - source.bottom().as_f32()).abs(),
        ];
        let side = (0..4)
            .min_by(|a, b| distances[*a].total_cmp(&distances[*b]))
            .unwrap();
        let padding = 5.0;
        let padded = match side {
            0 => [
                point(exit.x + padding, exit.y - padding),
                point(exit.x + padding, exit.y + padding),
            ],
            1 => [
                point(exit.x - padding, exit.y - padding),
                point(exit.x - padding, exit.y + padding),
            ],
            2 => [
                point(exit.x - padding, exit.y + padding),
                point(exit.x + padding, exit.y + padding),
            ],
            _ => [
                point(exit.x - padding, exit.y - padding),
                point(exit.x + padding, exit.y - padding),
            ],
        };
        let mut points = vec![
            padded[0],
            padded[1],
            point(target.left().as_f32(), target.top().as_f32()),
            point(target.right().as_f32(), target.top().as_f32()),
            point(target.right().as_f32(), target.bottom().as_f32()),
            point(target.left().as_f32(), target.bottom().as_f32()),
        ];
        points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        let mut hull = Vec::with_capacity(8);
        for order in [points.clone(), points.into_iter().rev().collect()] {
            let start = hull.len();
            for p in order {
                while hull.len() >= start + 2 {
                    let q: Point<f32> = hull[hull.len() - 1];
                    let r: Point<f32> = hull[hull.len() - 2];
                    if (q.x - r.x) * (p.y - r.y) < (q.y - r.y) * (p.x - r.x) {
                        break;
                    }
                    hull.pop();
                }
                hull.push(p);
            }
            hull.pop();
        }
        Self(hull)
    }

    pub fn contains(&self, position: Point<Pixels>) -> bool {
        let x = position.x.as_f32();
        let y = position.y.as_f32();
        let mut inside = false;
        let mut previous = *self
            .0
            .last()
            .expect("a tooltip corridor has six source points");
        for current in &self.0 {
            if (current.y > y) != (previous.y > y)
                && x < (previous.x - current.x) * (y - current.y) / (previous.y - current.y)
                    + current.x
            {
                inside = !inside;
            }
            previous = *current;
        }
        inside
    }
}
