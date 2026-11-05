//! Uniform subtree scaling for native enter/exit animations. Layout stays unchanged.
use super::*;

#[derive(Clone, Copy)]
pub(crate) struct PaintScale {
    pub origin: Point<ScaledPixels>,
    pub factor: f32,
    pub outer_mask: Bounds<ScaledPixels>,
}

impl PaintScale {
    fn point(self, p: Point<ScaledPixels>) -> Point<ScaledPixels> {
        self.origin + (p - self.origin) * self.factor
    }

    pub(super) fn bounds(self, b: Bounds<ScaledPixels>) -> Bounds<ScaledPixels> {
        Bounds::new(self.point(b.origin), b.size.map(|v| v * self.factor))
    }

    fn mask(self, mask: &mut ContentMask<ScaledPixels>) {
        // Ancestor clipping is in window coordinates, as with a CSS transform.
        if mask.bounds != self.outer_mask {
            mask.bounds = self.bounds(mask.bounds).intersect(&self.outer_mask);
        }
    }

    fn sprite(self, matrix: &mut TransformationMatrix) {
        let transform = TransformationMatrix::unit()
            .translate(self.origin)
            .scale(Size::new(self.factor, self.factor))
            .translate(self.origin * -1.0);
        let inverse = TransformationMatrix::unit()
            .translate(self.origin)
            .scale(Size::new(1.0 / self.factor, 1.0 / self.factor))
            .translate(self.origin * -1.0);
        // Bounds are scaled too; conjugation preserves existing icon/image rotations.
        *matrix = transform.compose(*matrix).compose(inverse);
    }

    pub(super) fn apply(self, primitive: &mut Primitive) {
        macro_rules! geometry {
            ($p:expr) => {{
                $p.bounds = self.bounds($p.bounds);
                self.mask(&mut $p.content_mask);
            }};
        }
        macro_rules! filter {
            ($p:expr) => {{
                geometry!($p);
                $p.corner_radii = $p.corner_radii.map(|v| *v * self.factor);
                for filter in &mut $p.filters {
                    match filter {
                        ScaledFilter::Blur(radius) => *radius = *radius * self.factor,
                    }
                }
            }};
        }
        match primitive {
            Primitive::Quad(p) => {
                geometry!(p);
                p.corner_radii = p.corner_radii.map(|v| *v * self.factor);
                p.border_widths = p.border_widths.map(|v| *v * self.factor);
                p.border_dashed_length *= self.factor;
                p.border_dashed_gap *= self.factor;
            }
            Primitive::Shadow(p) => {
                geometry!(p);
                p.corner_radii = p.corner_radii.map(|v| *v * self.factor);
                p.element_bounds = self.bounds(p.element_bounds);
                p.element_corner_radii = p.element_corner_radii.map(|v| *v * self.factor);
                p.blur_radius = p.blur_radius * self.factor;
            }
            Primitive::Path(p) => {
                geometry!(p);
                for vertex in &mut p.vertices {
                    vertex.xy_position = self.point(vertex.xy_position);
                    self.mask(&mut vertex.content_mask);
                }
                p.start = self.point(p.start);
                p.current = self.point(p.current);
            }
            Primitive::Underline(p) => {
                geometry!(p);
                p.thickness = p.thickness * self.factor;
            }
            Primitive::MonochromeSprite(p) => {
                geometry!(p);
                self.sprite(&mut p.transformation);
            }
            Primitive::SubpixelSprite(p) => {
                geometry!(p);
                self.sprite(&mut p.transformation);
            }
            Primitive::PolychromeSprite(p) => {
                geometry!(p);
                p.corner_radii = p.corner_radii.map(|v| *v * self.factor);
                self.sprite(&mut p.transformation);
            }
            Primitive::Surface(p) => geometry!(p),
            Primitive::BackdropFilter(p) => filter!(p),
            Primitive::FilterBoundary(p) => filter!(p),
        }
    }
}
