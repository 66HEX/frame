// todo("windows"): remove
#![cfg_attr(windows, allow(dead_code))]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    AtlasTextureId, AtlasTile, Background, Bounds, ContentMask, Corners, Edges, Pixels, Point,
    Radians, ScaledFilter, ScaledPixels, Size, bounds_tree::BoundsTree, point,
};
use smallvec::SmallVec;
use std::{
    fmt::Debug,
    iter::Peekable,
    ops::{Add, Range, Sub},
    slice,
};

mod plan;
pub use plan::*;
mod abi;
#[doc(hidden)]
pub use abi::{SCENE_BUFFER_LAYOUTS, SceneBufferLayout};

#[allow(non_camel_case_types, unused)]
#[expect(missing_docs)]
pub type PathVertex_ScaledPixels = PathVertex<ScaledPixels>;

#[expect(missing_docs)]
pub type DrawOrder = u32;

pub(crate) const DEFAULT_BORDER_DASHED_LENGTH: f32 = 2.0;
pub(crate) const DEFAULT_BORDER_DASHED_GAP: f32 = 1.0;

/// A boolean with the same four-byte representation in Rust and WGSL.
/// Scene structs use it over one-byte [`bool`] to keep the storage-buffer ABI explicit.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ShaderBool {
    /// The flag is disabled.
    #[default]
    Disabled = 0,
    /// The flag is enabled.
    Enabled = 1,
}

impl ShaderBool {
    /// Returns this flag as a regular Rust boolean.
    pub fn is_enabled(self) -> bool {
        self == Self::Enabled
    }
}

impl From<bool> for ShaderBool {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

#[derive(Default)]
#[expect(missing_docs)]
pub struct Scene {
    pub(crate) paint_operations: Vec<PaintOperation>,
    primitive_bounds: BoundsTree<ScaledPixels>,
    layer_stack: Vec<Option<DrawOrder>>,
    filter_depth: usize,
    pub shadows: Vec<Shadow>,
    pub quads: Vec<Quad>,
    pub paths: Vec<Path<ScaledPixels>>,
    pub underlines: Vec<Underline>,
    pub monochrome_sprites: Vec<MonochromeSprite>,
    pub subpixel_sprites: Vec<SubpixelSprite>,
    pub polychrome_sprites: Vec<PolychromeSprite>,
    pub surfaces: Vec<PaintSurface>,
    surface_opacities: Vec<f32>,
    pub backdrop_filters: Vec<BackdropFilter>,
    pub filter_boundaries: Vec<FilterBoundary>,
    render_plan: ScenePlan,
    is_finished: bool,
}

#[expect(missing_docs)]
impl Scene {
    pub fn clear(&mut self) {
        self.paint_operations.clear();
        self.primitive_bounds.clear();
        self.layer_stack.clear();
        self.filter_depth = 0;
        self.paths.clear();
        self.shadows.clear();
        self.quads.clear();
        self.underlines.clear();
        self.monochrome_sprites.clear();
        self.subpixel_sprites.clear();
        self.polychrome_sprites.clear();
        self.surfaces.clear();
        self.surface_opacities.clear();
        self.backdrop_filters.clear();
        self.filter_boundaries.clear();
        self.render_plan.clear();
        self.is_finished = false;
    }

    pub fn len(&self) -> usize {
        self.paint_operations.len()
    }

    pub fn push_layer(&mut self, bounds: Bounds<ScaledPixels>) {
        self.is_finished = false;
        let order = self.primitive_bounds.insert(bounds);
        self.layer_stack.push(Some(order));
        self.paint_operations
            .push(PaintOperation::StartLayer(bounds));
    }

    pub fn pop_layer(&mut self) {
        self.is_finished = false;
        self.layer_stack.pop();
        self.paint_operations.push(PaintOperation::EndLayer);
    }

    /// Raise the draw-order floor so every primitive inserted afterwards sorts above everything
    /// inserted before. Called before painting deferred draws so overlays (tooltips, popovers,
    /// drag images) sort above the main scene — and a deferred backdrop's order can't fall inside
    /// a content-filter (`filter`) order range left behind by the main scene.
    pub fn raise_order_floor(&mut self) {
        self.is_finished = false;
        let floor = self.primitive_bounds.max_order() + 1;
        self.primitive_bounds.set_order_floor(floor);
    }

    pub(crate) fn is_in_filter_group(&self) -> bool {
        self.filter_depth > 0
    }

    pub fn insert_primitive(&mut self, primitive: impl Into<Primitive>) {
        let mut primitive = primitive.into();
        self.is_finished = false;

        // Cached glyph batches may have been recorded outside isolation. The monochrome
        // pipeline also accepts RGB atlas masks and provides a scalar-alpha fallback.
        if self.is_in_filter_group()
            && let Primitive::SubpixelSprite(sprite) = primitive
        {
            primitive = Primitive::MonochromeSprite(MonochromeSprite {
                order: sprite.order,
                padding: sprite.padding,
                bounds: sprite.bounds,
                content_mask: sprite.content_mask,
                color: sprite.color,
                tile: sprite.tile,
                transformation: sprite.transformation,
            });
        }

        let clipped_bounds = primitive
            .bounds()
            .intersect(&primitive.content_mask().bounds);

        // Isolation boundaries stay paired even when their clipped region is empty.
        // Each marker sorts above prior content, and its order floor keeps following
        // content inside the intended group or after its closing marker.
        let is_filter_boundary = matches!(primitive, Primitive::FilterBoundary(_));

        if clipped_bounds.is_empty() && !is_filter_boundary {
            return;
        }

        let order = if is_filter_boundary {
            let order_bounds = if clipped_bounds.is_empty() {
                *primitive.bounds()
            } else {
                clipped_bounds
            };
            self.primitive_bounds.insert_above_all(order_bounds)
        } else {
            self.layer_stack
                .last()
                .copied()
                .flatten()
                .unwrap_or_else(|| self.primitive_bounds.insert(clipped_bounds))
        };
        match &mut primitive {
            Primitive::Shadow(shadow) => {
                shadow.order = order;
                self.shadows.push(*shadow);
            }
            Primitive::Quad(quad) => {
                quad.order = order;
                self.quads.push(*quad);
            }
            Primitive::Path(path) => {
                path.order = order;
                path.id = PathId(self.paths.len());
                self.paths.push(path.clone());
            }
            Primitive::Underline(underline) => {
                underline.order = order;
                self.underlines.push(*underline);
            }
            Primitive::MonochromeSprite(sprite) => {
                sprite.order = order;
                self.monochrome_sprites.push(*sprite);
            }
            Primitive::SubpixelSprite(sprite) => {
                sprite.order = order;
                self.subpixel_sprites.push(*sprite);
            }
            Primitive::PolychromeSprite(sprite) => {
                sprite.order = order;
                self.polychrome_sprites.push(*sprite);
            }
            Primitive::Surface(surface) => {
                surface.order = order;
                self.surfaces.push(surface.clone());
                self.surface_opacities.push(1.0);
            }
            Primitive::BackdropFilter(filter) => {
                filter.order = order;
                self.backdrop_filters.push(filter.clone());
            }
            Primitive::FilterBoundary(boundary) => {
                boundary.order = order;
                // Every boundary breaks active layer batching and starts a new order segment.
                self.layer_stack.fill(None);
                self.primitive_bounds.set_order_floor(order + 1);

                if boundary.is_start {
                    self.filter_depth += 1;
                } else {
                    self.filter_depth = self.filter_depth.saturating_sub(1);
                }

                self.filter_boundaries.push(boundary.clone());
            }
        }

        self.paint_operations
            .push(PaintOperation::Primitive(primitive));
    }

    pub fn replay(&mut self, range: Range<usize>, prev_scene: &Scene) {
        for operation in &prev_scene.paint_operations[range] {
            match operation {
                PaintOperation::Primitive(primitive) => self.insert_primitive(primitive.clone()),
                PaintOperation::StartLayer(bounds) => self.push_layer(*bounds),
                PaintOperation::EndLayer => self.pop_layer(),
            }
        }
    }

    pub fn finish(&mut self) {
        self.shadows.sort_by_key(|shadow| shadow.order);
        self.quads.sort_by_key(|quad| quad.order);
        self.paths.sort_by_key(|path| path.order);
        self.underlines.sort_by_key(|underline| underline.order);
        self.monochrome_sprites
            .sort_by_key(|sprite| (sprite.order, sprite.tile.tile_id));
        self.subpixel_sprites
            .sort_by_key(|sprite| (sprite.order, sprite.tile.tile_id));
        self.polychrome_sprites
            .sort_by_key(|sprite| (sprite.order, sprite.tile.tile_id));
        self.surfaces.sort_by_key(|surface| surface.order);
        self.surface_opacities.resize(self.surfaces.len(), 1.0);
        self.backdrop_filters.sort_by_key(|filter| filter.order);
        // Markers normally get distinct, monotonically-increasing orders (children overlap
        // their group bounds and so sort strictly between the start and end). The `!is_start`
        // tiebreak only matters for a degenerate empty group whose start and end tie: it keeps
        // the start (false = 0) ahead of the end (true = 1) so the pair stays well-formed.
        self.filter_boundaries
            .sort_by_key(|boundary| (boundary.order, !boundary.is_start));
        let commands = std::mem::take(&mut self.render_plan.commands);
        self.render_plan = ScenePlan::build(self, commands);
        self.is_finished = true;
    }

    #[cfg_attr(
        all(
            any(target_os = "linux", target_os = "freebsd"),
            not(any(feature = "x11", feature = "wayland"))
        ),
        allow(dead_code)
    )]
    pub fn batches(&self) -> impl Iterator<Item = PrimitiveBatch> + '_ {
        self.render_commands().iter().map(|command| match command {
            RenderCommand::Batch(batch) => batch.clone(),
            RenderCommand::BeginFilter { boundary_index, .. } => {
                PrimitiveBatch::FilterBoundary(*boundary_index)
            }
            RenderCommand::EndFilter {
                closing_boundary_index,
                ..
            } => PrimitiveBatch::FilterBoundary(*closing_boundary_index),
        })
    }

    /// Returns the backend-neutral sequence of work needed to render this scene.
    ///
    /// In addition to preserving primitive batch order, this pairs content-filter boundaries and
    /// assigns the bounded offscreen target used by each isolated group. GPU backends only need to
    /// manage their target handles; nesting and overflow behavior stay consistent everywhere.
    pub fn render_commands(&self) -> &[RenderCommand] {
        self.render_plan().commands()
    }

    /// Returns the compiled render plan for this finished scene.
    pub fn render_plan(&self) -> &ScenePlan {
        debug_assert!(
            self.is_finished,
            "Scene::finish must be called before rendering"
        );
        self.render_plan.assert_matches(self);
        &self.render_plan
    }

    /// Returns the intrinsic opacity of each surface in [`Self::surfaces`].
    /// Surfaces are fully opaque here; group opacity is applied when compositing.
    pub fn surface_opacities(&self) -> &[f32] {
        &self.surface_opacities
    }

    /// Whether rendering needs an offscreen scene target for filters or grouped opacity.
    pub fn requires_offscreen_rendering(&self) -> bool {
        self.render_plan().requirements().uses_offscreen_target
    }
}

/// Internal representation of [`palette::Hsla`] which is layout sensitive, as its provided to the renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[repr(C)]
pub struct SceneHsla {
    /// Hue, in a range from 0 to 1
    pub(crate) h: f32,
    /// Saturation, in a range from 0 to 1
    pub(crate) s: f32,
    /// Lightness, in a range from 0 to 1
    pub(crate) l: f32,
    /// Alpha, in a range from 0 to 1
    pub(crate) a: f32,
}
impl Into<palette::Hsla> for SceneHsla {
    fn into(self) -> palette::Hsla {
        palette::Hsla::new(self.h * 360.0, self.s, self.l, self.a)
    }
}
impl From<palette::Hsla> for SceneHsla {
    fn from(hsla: palette::Hsla) -> Self {
        Self {
            h: hsla.hue.into_positive_degrees() / 360.0,
            s: hsla.saturation,
            l: hsla.lightness,
            a: hsla.alpha,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Default)]
#[cfg_attr(
    all(
        any(target_os = "linux", target_os = "freebsd"),
        not(any(feature = "x11", feature = "wayland"))
    ),
    allow(dead_code)
)]
pub(crate) enum PrimitiveKind {
    // Lowest discriminant: at an equal order, a content-filter group-start is emitted before
    // the group's own content so the renderer redirects rendering before any child draws.
    FilterBoundaryStart,
    Shadow,
    #[default]
    Quad,
    Path,
    Underline,
    MonochromeSprite,
    SubpixelSprite,
    PolychromeSprite,
    Surface,
    BackdropFilter,
    // Highest discriminant: at an equal order, a group-end is emitted after the group's content
    // so the renderer composites the filtered group only once every child has been drawn.
    FilterBoundaryEnd,
}

pub(crate) enum PaintOperation {
    Primitive(Primitive),
    StartLayer(Bounds<ScaledPixels>),
    EndLayer,
}

#[derive(Clone)]
#[expect(missing_docs)]
pub enum Primitive {
    Shadow(Shadow),
    Quad(Quad),
    Path(Path<ScaledPixels>),
    Underline(Underline),
    MonochromeSprite(MonochromeSprite),
    SubpixelSprite(SubpixelSprite),
    PolychromeSprite(PolychromeSprite),
    Surface(PaintSurface),
    BackdropFilter(BackdropFilter),
    FilterBoundary(FilterBoundary),
}

#[expect(missing_docs)]
impl Primitive {
    pub fn bounds(&self) -> &Bounds<ScaledPixels> {
        match self {
            Primitive::Shadow(shadow) => &shadow.bounds,
            Primitive::Quad(quad) => &quad.bounds,
            Primitive::Path(path) => &path.bounds,
            Primitive::Underline(underline) => &underline.bounds,
            Primitive::MonochromeSprite(sprite) => &sprite.bounds,
            Primitive::SubpixelSprite(sprite) => &sprite.bounds,
            Primitive::PolychromeSprite(sprite) => &sprite.bounds,
            Primitive::Surface(surface) => &surface.bounds,
            Primitive::BackdropFilter(filter) => &filter.bounds,
            Primitive::FilterBoundary(boundary) => &boundary.bounds,
        }
    }

    pub fn content_mask(&self) -> &ContentMask<ScaledPixels> {
        match self {
            Primitive::Shadow(shadow) => &shadow.content_mask,
            Primitive::Quad(quad) => &quad.content_mask,
            Primitive::Path(path) => &path.content_mask,
            Primitive::Underline(underline) => &underline.content_mask,
            Primitive::MonochromeSprite(sprite) => &sprite.content_mask,
            Primitive::SubpixelSprite(sprite) => &sprite.content_mask,
            Primitive::PolychromeSprite(sprite) => &sprite.content_mask,
            Primitive::Surface(surface) => &surface.content_mask,
            Primitive::BackdropFilter(filter) => &filter.content_mask,
            Primitive::FilterBoundary(boundary) => &boundary.content_mask,
        }
    }
}

#[cfg_attr(
    all(
        any(target_os = "linux", target_os = "freebsd"),
        not(any(feature = "x11", feature = "wayland"))
    ),
    allow(dead_code)
)]
struct BatchIterator<'a> {
    shadows_start: usize,
    shadows_iter: Peekable<slice::Iter<'a, Shadow>>,
    quads_start: usize,
    quads_iter: Peekable<slice::Iter<'a, Quad>>,
    paths_start: usize,
    paths: &'a [Path<ScaledPixels>],
    paths_iter: Peekable<slice::Iter<'a, Path<ScaledPixels>>>,
    underlines_start: usize,
    underlines_iter: Peekable<slice::Iter<'a, Underline>>,
    monochrome_sprites_start: usize,
    monochrome_sprites_iter: Peekable<slice::Iter<'a, MonochromeSprite>>,
    subpixel_sprites_start: usize,
    subpixel_sprites_iter: Peekable<slice::Iter<'a, SubpixelSprite>>,
    polychrome_sprites_start: usize,
    polychrome_sprites_iter: Peekable<slice::Iter<'a, PolychromeSprite>>,
    surfaces_start: usize,
    surfaces_iter: Peekable<slice::Iter<'a, PaintSurface>>,
    backdrop_filters_start: usize,
    backdrop_filters_iter: Peekable<slice::Iter<'a, BackdropFilter>>,
    filter_boundaries_start: usize,
    filter_boundaries_iter: Peekable<slice::Iter<'a, FilterBoundary>>,
}

impl<'a> BatchIterator<'a> {
    fn new(scene: &'a Scene) -> Self {
        Self {
            shadows_start: 0,
            shadows_iter: scene.shadows.iter().peekable(),
            quads_start: 0,
            quads_iter: scene.quads.iter().peekable(),
            paths_start: 0,
            paths: &scene.paths,
            paths_iter: scene.paths.iter().peekable(),
            underlines_start: 0,
            underlines_iter: scene.underlines.iter().peekable(),
            monochrome_sprites_start: 0,
            monochrome_sprites_iter: scene.monochrome_sprites.iter().peekable(),
            subpixel_sprites_start: 0,
            subpixel_sprites_iter: scene.subpixel_sprites.iter().peekable(),
            polychrome_sprites_start: 0,
            polychrome_sprites_iter: scene.polychrome_sprites.iter().peekable(),
            surfaces_start: 0,
            surfaces_iter: scene.surfaces.iter().peekable(),
            backdrop_filters_start: 0,
            backdrop_filters_iter: scene.backdrop_filters.iter().peekable(),
            filter_boundaries_start: 0,
            filter_boundaries_iter: scene.filter_boundaries.iter().peekable(),
        }
    }
}

fn precedes_limit(
    order: DrawOrder,
    kind: PrimitiveKind,
    limit: Option<(DrawOrder, PrimitiveKind)>,
) -> bool {
    limit.is_none_or(|limit| (order, kind) < limit)
}

fn has_corner_smoothing(corner_smoothing: f32) -> bool {
    corner_smoothing > 0.0
}

impl<'a> Iterator for BatchIterator<'a> {
    type Item = PrimitiveBatch;

    fn next(&mut self) -> Option<Self::Item> {
        let orders_and_kinds = [
            (
                self.shadows_iter.peek().map(|s| s.order),
                PrimitiveKind::Shadow,
            ),
            (self.quads_iter.peek().map(|q| q.order), PrimitiveKind::Quad),
            (self.paths_iter.peek().map(|q| q.order), PrimitiveKind::Path),
            (
                self.underlines_iter.peek().map(|u| u.order),
                PrimitiveKind::Underline,
            ),
            (
                self.monochrome_sprites_iter.peek().map(|s| s.order),
                PrimitiveKind::MonochromeSprite,
            ),
            (
                self.subpixel_sprites_iter.peek().map(|s| s.order),
                PrimitiveKind::SubpixelSprite,
            ),
            (
                self.polychrome_sprites_iter.peek().map(|s| s.order),
                PrimitiveKind::PolychromeSprite,
            ),
            (
                self.surfaces_iter.peek().map(|s| s.order),
                PrimitiveKind::Surface,
            ),
            (
                self.backdrop_filters_iter.peek().map(|f| f.order),
                PrimitiveKind::BackdropFilter,
            ),
            (
                self.filter_boundaries_iter.peek().map(|b| b.order),
                // The same vec yields both start and end markers; the discriminant decides
                // where the next marker sorts relative to draw batches at an equal order
                // (start before content, end after).
                match self.filter_boundaries_iter.peek() {
                    Some(boundary) if boundary.is_start => PrimitiveKind::FilterBoundaryStart,
                    _ => PrimitiveKind::FilterBoundaryEnd,
                },
            ),
        ];
        // Find the two lowest live cursors in one fixed-size scan.
        let mut first = None;
        let mut second = None;
        for (order, kind) in orders_and_kinds {
            let Some(order) = order else {
                continue;
            };
            let candidate = (order, kind);
            if first.is_none_or(|current| candidate < current) {
                second = first;
                first = Some(candidate);
            } else if second.is_none_or(|current| candidate < current) {
                second = Some(candidate);
            }
        }
        let (_, batch_kind) = first?;
        let max_order_and_kind = second;

        match batch_kind {
            PrimitiveKind::Shadow => {
                let smoothed =
                    has_corner_smoothing(self.shadows_iter.peek().unwrap().corner_smoothing);
                let shadows_start = self.shadows_start;
                let mut shadows_end = shadows_start + 1;
                self.shadows_iter.next();
                while self
                    .shadows_iter
                    .next_if(|shadow| {
                        precedes_limit(shadow.order, batch_kind, max_order_and_kind)
                            && has_corner_smoothing(shadow.corner_smoothing) == smoothed
                    })
                    .is_some()
                {
                    shadows_end += 1;
                }
                self.shadows_start = shadows_end;
                Some(PrimitiveBatch::Shadows {
                    range: shadows_start..shadows_end,
                    smoothed,
                })
            }
            PrimitiveKind::Quad => {
                let smoothed =
                    has_corner_smoothing(self.quads_iter.peek().unwrap().corner_smoothing);
                let quads_start = self.quads_start;
                let mut quads_end = quads_start + 1;
                self.quads_iter.next();
                while self
                    .quads_iter
                    .next_if(|quad| {
                        precedes_limit(quad.order, batch_kind, max_order_and_kind)
                            && has_corner_smoothing(quad.corner_smoothing) == smoothed
                    })
                    .is_some()
                {
                    quads_end += 1;
                }
                self.quads_start = quads_end;
                Some(PrimitiveBatch::Quads {
                    range: quads_start..quads_end,
                    smoothed,
                })
            }
            PrimitiveKind::Path => {
                let paths_start = self.paths_start;
                let mut paths_end = paths_start + 1;
                self.paths_iter.next();
                while self
                    .paths_iter
                    .next_if(|path| precedes_limit(path.order, batch_kind, max_order_and_kind))
                    .is_some()
                {
                    paths_end += 1;
                }
                self.paths_start = paths_end;
                let range = paths_start..paths_end;
                let paths = &self.paths[range.clone()];
                let rasterization_vertex_count = paths.iter().map(|path| path.vertices.len()).sum();
                let sprite_count = if paths
                    .last()
                    .is_some_and(|path| path.order == paths[0].order)
                {
                    paths.len()
                } else {
                    1
                };
                Some(PrimitiveBatch::Paths {
                    range,
                    rasterization_vertex_count,
                    sprite_count,
                })
            }
            PrimitiveKind::Underline => {
                let underlines_start = self.underlines_start;
                let mut underlines_end = underlines_start + 1;
                self.underlines_iter.next();
                while self
                    .underlines_iter
                    .next_if(|underline| {
                        precedes_limit(underline.order, batch_kind, max_order_and_kind)
                    })
                    .is_some()
                {
                    underlines_end += 1;
                }
                self.underlines_start = underlines_end;
                Some(PrimitiveBatch::Underlines(underlines_start..underlines_end))
            }
            PrimitiveKind::MonochromeSprite => {
                let texture_id = self.monochrome_sprites_iter.peek().unwrap().tile.texture_id;
                let sprites_start = self.monochrome_sprites_start;
                let mut sprites_end = sprites_start + 1;
                self.monochrome_sprites_iter.next();
                while self
                    .monochrome_sprites_iter
                    .next_if(|sprite| {
                        precedes_limit(sprite.order, batch_kind, max_order_and_kind)
                            && sprite.tile.texture_id == texture_id
                    })
                    .is_some()
                {
                    sprites_end += 1;
                }
                self.monochrome_sprites_start = sprites_end;
                Some(PrimitiveBatch::MonochromeSprites {
                    texture_id,
                    range: sprites_start..sprites_end,
                })
            }
            PrimitiveKind::SubpixelSprite => {
                let texture_id = self.subpixel_sprites_iter.peek().unwrap().tile.texture_id;
                let sprites_start = self.subpixel_sprites_start;
                let mut sprites_end = sprites_start + 1;
                self.subpixel_sprites_iter.next();
                while self
                    .subpixel_sprites_iter
                    .next_if(|sprite| {
                        precedes_limit(sprite.order, batch_kind, max_order_and_kind)
                            && sprite.tile.texture_id == texture_id
                    })
                    .is_some()
                {
                    sprites_end += 1;
                }
                self.subpixel_sprites_start = sprites_end;
                Some(PrimitiveBatch::SubpixelSprites {
                    texture_id,
                    range: sprites_start..sprites_end,
                })
            }
            PrimitiveKind::PolychromeSprite => {
                let first_sprite = self.polychrome_sprites_iter.peek().unwrap();
                let texture_id = first_sprite.tile.texture_id;
                let smoothed = has_corner_smoothing(first_sprite.corner_smoothing);
                let sprites_start = self.polychrome_sprites_start;
                let mut sprites_end = sprites_start + 1;
                self.polychrome_sprites_iter.next();
                while self
                    .polychrome_sprites_iter
                    .next_if(|sprite| {
                        precedes_limit(sprite.order, batch_kind, max_order_and_kind)
                            && sprite.tile.texture_id == texture_id
                            && has_corner_smoothing(sprite.corner_smoothing) == smoothed
                    })
                    .is_some()
                {
                    sprites_end += 1;
                }
                self.polychrome_sprites_start = sprites_end;
                Some(PrimitiveBatch::PolychromeSprites {
                    texture_id,
                    range: sprites_start..sprites_end,
                    smoothed,
                })
            }
            PrimitiveKind::Surface => {
                let surfaces_start = self.surfaces_start;
                let mut surfaces_end = surfaces_start + 1;
                self.surfaces_iter.next();
                while self
                    .surfaces_iter
                    .next_if(|surface| {
                        precedes_limit(surface.order, batch_kind, max_order_and_kind)
                    })
                    .is_some()
                {
                    surfaces_end += 1;
                }
                self.surfaces_start = surfaces_end;
                Some(PrimitiveBatch::Surfaces(surfaces_start..surfaces_end))
            }
            PrimitiveKind::BackdropFilter => {
                let backdrop_filters_start = self.backdrop_filters_start;
                let mut backdrop_filters_end = backdrop_filters_start + 1;
                self.backdrop_filters_iter.next();
                while self
                    .backdrop_filters_iter
                    .next_if(|filter| precedes_limit(filter.order, batch_kind, max_order_and_kind))
                    .is_some()
                {
                    backdrop_filters_end += 1;
                }
                self.backdrop_filters_start = backdrop_filters_end;
                Some(PrimitiveBatch::BackdropFilters(
                    backdrop_filters_start..backdrop_filters_end,
                ))
            }
            // Boundaries are emitted one at a time (never merged) so the renderer can switch
            // render targets at exactly the right point in the batch stream.
            PrimitiveKind::FilterBoundaryStart | PrimitiveKind::FilterBoundaryEnd => {
                let index = self.filter_boundaries_start;
                self.filter_boundaries_iter.next();
                self.filter_boundaries_start = index + 1;
                Some(PrimitiveBatch::FilterBoundary(index))
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
#[expect(missing_docs)]
pub struct Quad {
    pub order: DrawOrder,
    pub border_style: BorderStyle,
    pub border_dashed_length: f32,
    pub border_dashed_gap: f32,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub background: Background,
    pub border_color: Background,
    pub corner_radii: Corners<ScaledPixels>,
    pub border_widths: Edges<ScaledPixels>,
    pub corner_smoothing: f32,
    pub padding: u32,
}

impl Default for Quad {
    fn default() -> Self {
        Self {
            order: Default::default(),
            border_style: Default::default(),
            border_dashed_length: DEFAULT_BORDER_DASHED_LENGTH,
            border_dashed_gap: DEFAULT_BORDER_DASHED_GAP,
            bounds: Default::default(),
            content_mask: Default::default(),
            background: Default::default(),
            border_color: Default::default(),
            corner_radii: Default::default(),
            border_widths: Default::default(),
            corner_smoothing: Default::default(),
            padding: Default::default(),
        }
    }
}

impl From<Quad> for Primitive {
    fn from(quad: Quad) -> Self {
        Primitive::Quad(quad)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
#[expect(missing_docs)]
pub struct Underline {
    pub order: DrawOrder,
    pub padding: u32,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub color: SceneHsla,
    pub thickness: ScaledPixels,
    pub wavy: ShaderBool,
}

impl From<Underline> for Primitive {
    fn from(underline: Underline) -> Self {
        Primitive::Underline(underline)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
#[expect(missing_docs)]
pub struct Shadow {
    pub order: DrawOrder,
    pub blur_radius: ScaledPixels,
    pub bounds: Bounds<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub color: Background,
    pub element_bounds: Bounds<ScaledPixels>,
    pub element_corner_radii: Corners<ScaledPixels>,
    /// Whether this shadow is rendered inside the element instead of outside it.
    pub inset: ShaderBool,
    pub corner_smoothing: f32,
}

impl From<Shadow> for Primitive {
    fn from(shadow: Shadow) -> Self {
        Primitive::Shadow(shadow)
    }
}

/// A backdrop filter blurs (and may otherwise filter) the content already rendered behind
/// `bounds`, compositing the result into a rounded rectangle — the frosted-glass effect.
/// Emitted by [`crate::Window::paint_backdrop_filter`]; produces the CSS `backdrop-filter` effect.
#[derive(Default, Debug, Clone)]
#[expect(missing_docs)]
pub struct BackdropFilter {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub corner_smoothing: f32,
    /// The filter chain applied to the backdrop, in scene (device-pixel) space. Identity filters
    /// are dropped at paint time, so a `BackdropFilter` is only emitted when this is non-empty.
    ///
    /// Inline capacity is 4: a `SmallVec<[ScaledFilter; 4]>` is the same size as capacity 1 here
    /// (the heap repr already occupies that space), so chains up to 4 filters avoid allocating
    /// at no extra struct size.
    pub filters: SmallVec<[ScaledFilter; 4]>,
    /// Element opacity captured at paint time, multiplied into the composited result.
    pub opacity: f32,
}

impl BackdropFilter {
    /// Largest gaussian blur radius in this filter chain, in device pixels.
    pub fn max_blur_radius(&self) -> f32 {
        max_blur_radius(&self.filters)
    }
}

impl From<BackdropFilter> for Primitive {
    fn from(filter: BackdropFilter) -> Self {
        Primitive::BackdropFilter(filter)
    }
}

/// The start or end marker of a content-filter or opacity isolation group. The element's
/// subtree is painted between a matched start/end pair; the renderer redirects that span into
/// an offscreen target, filters it, and composites it back at `bounds`. Produces the CSS
/// `filter` effect (e.g. blurring the element and its children as a single group).
#[derive(Debug, Clone)]
#[expect(missing_docs)]
pub struct FilterBoundary {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub corner_smoothing: f32,
    /// The filter chain applied to the isolated group, in scene (device-pixel) space. Identity
    /// filters are dropped at paint time. An empty chain represents an opacity-only group.
    /// Inline capacity 4 (same struct size as 1 here — see [`BackdropFilter::filters`]).
    pub filters: SmallVec<[ScaledFilter; 4]>,
    /// Opacity applied once when compositing the completed group into its parent.
    pub opacity: f32,
    /// `true` for the start marker (opens the group), `false` for the end marker (closes it).
    pub is_start: bool,
}

impl FilterBoundary {
    /// Largest gaussian blur radius in this filter chain, in device pixels.
    pub fn max_blur_radius(&self) -> f32 {
        max_blur_radius(&self.filters)
    }
}

/// Returns the largest blur radius in a scene-space filter chain.
///
/// This match is deliberately exhaustive so adding a filter requires one shared scheduling
/// decision instead of three backend-specific implementations that can drift.
fn max_blur_radius(filters: &[ScaledFilter]) -> f32 {
    filters.iter().fold(0.0, |radius, filter| match filter {
        ScaledFilter::Blur(filter_radius) => radius.max(filter_radius.0),
    })
}

impl From<FilterBoundary> for Primitive {
    fn from(boundary: FilterBoundary) -> Self {
        Primitive::FilterBoundary(boundary)
    }
}

/// The style of a border.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[repr(u32)]
pub enum BorderStyle {
    /// A solid border.
    #[default]
    Solid = 0,
    /// A dashed border.
    Dashed = 1,
}

/// A data type representing a 2 dimensional transformation that can be applied to an element.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct TransformationMatrix {
    /// 2x2 matrix containing rotation and scale,
    /// stored row-major
    pub rotation_scale: [[f32; 2]; 2],
    /// translation vector
    pub translation: [f32; 2],
}

impl Eq for TransformationMatrix {}

impl TransformationMatrix {
    /// The unit matrix, has no effect.
    pub fn unit() -> Self {
        Self {
            rotation_scale: [[1.0, 0.0], [0.0, 1.0]],
            translation: [0.0, 0.0],
        }
    }

    /// Move the origin by a given point
    pub fn translate(mut self, point: Point<ScaledPixels>) -> Self {
        self.compose(Self {
            rotation_scale: [[1.0, 0.0], [0.0, 1.0]],
            translation: [point.x.0, point.y.0],
        })
    }

    /// Clockwise rotation in radians around the origin
    pub fn rotate(self, angle: Radians) -> Self {
        self.compose(Self {
            rotation_scale: [
                [angle.0.cos(), -angle.0.sin()],
                [angle.0.sin(), angle.0.cos()],
            ],
            translation: [0.0, 0.0],
        })
    }

    /// Scale around the origin
    pub fn scale(self, size: Size<f32>) -> Self {
        self.compose(Self {
            rotation_scale: [[size.width, 0.0], [0.0, size.height]],
            translation: [0.0, 0.0],
        })
    }

    /// Perform matrix multiplication with another transformation
    /// to produce a new transformation that is the result of
    /// applying both transformations: first, `other`, then `self`.
    #[inline]
    pub fn compose(self, other: TransformationMatrix) -> TransformationMatrix {
        if other == Self::unit() {
            return self;
        }
        // Perform matrix multiplication
        TransformationMatrix {
            rotation_scale: [
                [
                    self.rotation_scale[0][0] * other.rotation_scale[0][0]
                        + self.rotation_scale[0][1] * other.rotation_scale[1][0],
                    self.rotation_scale[0][0] * other.rotation_scale[0][1]
                        + self.rotation_scale[0][1] * other.rotation_scale[1][1],
                ],
                [
                    self.rotation_scale[1][0] * other.rotation_scale[0][0]
                        + self.rotation_scale[1][1] * other.rotation_scale[1][0],
                    self.rotation_scale[1][0] * other.rotation_scale[0][1]
                        + self.rotation_scale[1][1] * other.rotation_scale[1][1],
                ],
            ],
            translation: [
                self.translation[0]
                    + self.rotation_scale[0][0] * other.translation[0]
                    + self.rotation_scale[0][1] * other.translation[1],
                self.translation[1]
                    + self.rotation_scale[1][0] * other.translation[0]
                    + self.rotation_scale[1][1] * other.translation[1],
            ],
        }
    }

    /// Apply transformation to a point, mainly useful for debugging
    pub fn apply(&self, point: Point<Pixels>) -> Point<Pixels> {
        let input = [point.x.0, point.y.0];
        let mut output = self.translation;
        for (i, output_cell) in output.iter_mut().enumerate() {
            for (k, input_cell) in input.iter().enumerate() {
                *output_cell += self.rotation_scale[i][k] * *input_cell;
            }
        }
        Point::new(output[0].into(), output[1].into())
    }
}

impl Default for TransformationMatrix {
    fn default() -> Self {
        Self::unit()
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
#[expect(missing_docs)]
pub struct MonochromeSprite {
    pub order: DrawOrder,
    pub padding: u32,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub color: SceneHsla,
    pub tile: AtlasTile,
    pub transformation: TransformationMatrix,
}

impl From<MonochromeSprite> for Primitive {
    fn from(sprite: MonochromeSprite) -> Self {
        Primitive::MonochromeSprite(sprite)
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
#[expect(missing_docs)]
pub struct SubpixelSprite {
    pub order: DrawOrder,
    pub padding: u32,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub color: SceneHsla,
    pub tile: AtlasTile,
    pub transformation: TransformationMatrix,
}

impl From<SubpixelSprite> for Primitive {
    fn from(sprite: SubpixelSprite) -> Self {
        Primitive::SubpixelSprite(sprite)
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
#[expect(missing_docs)]
pub struct PolychromeSprite {
    pub order: DrawOrder,
    pub grayscale: ShaderBool,
    pub opacity: f32,
    pub corner_smoothing: f32,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub tile: AtlasTile,
    pub transformation: TransformationMatrix,
}

impl From<PolychromeSprite> for Primitive {
    fn from(sprite: PolychromeSprite) -> Self {
        Primitive::PolychromeSprite(sprite)
    }
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct PaintSurface {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub source: crate::SurfaceSource,
}

impl From<PaintSurface> for Primitive {
    fn from(surface: PaintSurface) -> Self {
        Primitive::Surface(surface)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[expect(missing_docs)]
pub struct PathId(pub usize);

/// A line made up of a series of vertices and control points.
#[derive(Clone, Debug)]
#[expect(missing_docs)]
pub struct Path<P: Clone + Debug + Default + PartialEq> {
    pub id: PathId,
    pub order: DrawOrder,
    pub bounds: Bounds<P>,
    pub content_mask: ContentMask<P>,
    pub vertices: Vec<PathVertex<P>>,
    pub color: Background,
    start: Point<P>,
    current: Point<P>,
    contour_count: usize,
}

impl Path<Pixels> {
    /// Create a new path with the given starting point.
    pub fn new(start: Point<Pixels>) -> Self {
        Self {
            id: PathId(0),
            order: DrawOrder::default(),
            vertices: Vec::new(),
            start,
            current: start,
            bounds: Bounds {
                origin: start,
                size: Default::default(),
            },
            content_mask: Default::default(),
            color: Default::default(),
            contour_count: 0,
        }
    }

    /// Scale this path by the given factor.
    pub fn scale(&self, factor: f32) -> Path<ScaledPixels> {
        Path {
            id: self.id,
            order: self.order,
            bounds: self.bounds.scale(factor),
            content_mask: self.content_mask.scale(factor),
            vertices: self
                .vertices
                .iter()
                .map(|vertex| vertex.scale(factor))
                .collect(),
            start: self.start.map(|start| start.scale(factor)),
            current: self.current.scale(factor),
            contour_count: self.contour_count,
            color: self.color,
        }
    }

    /// Move the start, current point to the given point.
    pub fn move_to(&mut self, to: Point<Pixels>) {
        self.contour_count += 1;
        self.start = to;
        self.current = to;
    }

    /// Draw a straight line from the current point to the given point.
    pub fn line_to(&mut self, to: Point<Pixels>) {
        self.contour_count += 1;
        if self.contour_count > 1 {
            self.push_triangle(
                (self.start, self.current, to),
                (point(0., 1.), point(0., 1.), point(0., 1.)),
            );
        }
        self.current = to;
    }

    /// Draw a curve from the current point to the given point, using the given control point.
    pub fn curve_to(&mut self, to: Point<Pixels>, ctrl: Point<Pixels>) {
        self.contour_count += 1;
        if self.contour_count > 1 {
            self.push_triangle(
                (self.start, self.current, to),
                (point(0., 1.), point(0., 1.), point(0., 1.)),
            );
        }

        self.push_triangle(
            (self.current, ctrl, to),
            (point(0., 0.), point(0.5, 0.), point(1., 1.)),
        );
        self.current = to;
    }

    /// Push a triangle to the Path.
    pub fn push_triangle(
        &mut self,
        xy: (Point<Pixels>, Point<Pixels>, Point<Pixels>),
        st: (Point<f32>, Point<f32>, Point<f32>),
    ) {
        self.bounds = self
            .bounds
            .union(&Bounds {
                origin: xy.0,
                size: Default::default(),
            })
            .union(&Bounds {
                origin: xy.1,
                size: Default::default(),
            })
            .union(&Bounds {
                origin: xy.2,
                size: Default::default(),
            });

        self.vertices.push(PathVertex {
            xy_position: xy.0,
            st_position: st.0,
            content_mask: Default::default(),
        });
        self.vertices.push(PathVertex {
            xy_position: xy.1,
            st_position: st.1,
            content_mask: Default::default(),
        });
        self.vertices.push(PathVertex {
            xy_position: xy.2,
            st_position: st.2,
            content_mask: Default::default(),
        });
    }
}

impl<T> Path<T>
where
    T: Clone + Debug + Default + PartialEq + PartialOrd + Add<T, Output = T> + Sub<Output = T>,
{
    #[allow(unused)]
    #[expect(missing_docs)]
    pub fn clipped_bounds(&self) -> Bounds<T> {
        self.bounds.intersect(&self.content_mask.bounds)
    }
}

impl From<Path<ScaledPixels>> for Primitive {
    fn from(path: Path<ScaledPixels>) -> Self {
        Primitive::Path(path)
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
#[expect(missing_docs)]
pub struct PathVertex<P: Clone + Debug + Default + PartialEq> {
    pub xy_position: Point<P>,
    pub st_position: Point<f32>,
    pub content_mask: ContentMask<P>,
}

#[expect(missing_docs)]
impl PathVertex<Pixels> {
    pub fn scale(&self, factor: f32) -> PathVertex<ScaledPixels> {
        PathVertex {
            xy_position: self.xy_position.scale(factor),
            st_position: self.st_position,
            content_mask: self.content_mask.scale(factor),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtlasTextureKind, DevicePixels, Point, ShaderBool, Size, SurfaceSource, TileId};

    fn scaled(value: f32) -> ScaledPixels {
        ScaledPixels(value)
    }

    /// Default primitives overlap so their draw orders follow insertion order.
    fn full_bounds() -> Bounds<ScaledPixels> {
        Bounds::new(Point::default(), Size::new(scaled(100.0), scaled(100.0)))
    }

    fn mask() -> ContentMask<ScaledPixels> {
        ContentMask {
            bounds: full_bounds(),
            ..Default::default()
        }
    }

    fn quad() -> Quad {
        Quad {
            bounds: full_bounds(),
            content_mask: mask(),
            ..Default::default()
        }
    }

    fn shadow() -> Shadow {
        Shadow {
            order: 0,
            blur_radius: scaled(0.0),
            bounds: full_bounds(),
            corner_radii: Corners::default(),
            content_mask: mask(),
            color: Default::default(),
            element_bounds: full_bounds(),
            element_corner_radii: Corners::default(),
            inset: ShaderBool::Disabled,
            corner_smoothing: 0.0,
        }
    }

    fn polychrome_sprite(texture_index: u32) -> PolychromeSprite {
        PolychromeSprite {
            transformation: crate::TransformationMatrix::unit(),
            order: 0,
            grayscale: ShaderBool::Disabled,
            opacity: 1.0,
            corner_smoothing: 0.0,
            bounds: full_bounds(),
            content_mask: mask(),
            corner_radii: Corners::default(),
            tile: AtlasTile {
                texture_id: AtlasTextureId {
                    index: texture_index,
                    kind: AtlasTextureKind::Polychrome,
                },
                tile_id: TileId(0),
                padding: 0,
                bounds: Bounds::<DevicePixels>::default(),
            },
        }
    }

    fn batches(scene: &mut Scene) -> Vec<PrimitiveBatch> {
        scene.finish();
        scene.batches().collect()
    }

    #[test]
    fn smoothing_and_texture_changes_define_batches() {
        let mut scene = Scene::default();
        for smoothing in [0.0, 0.0, 0.5, 1.0, 0.0] {
            let mut quad = quad();
            quad.corner_smoothing = smoothing;
            scene.insert_primitive(quad);
        }

        for smoothing in [0.0, 0.0, 0.5, 1.0, 0.0] {
            let mut shadow = shadow();
            shadow.corner_smoothing = smoothing;
            scene.insert_primitive(shadow);
        }

        for (texture, smoothing) in [
            (0, 0.0),
            (0, 0.0),
            (0, 0.5),
            (0, 1.0),
            (1, 1.0),
            (1, 0.5),
            (1, 0.0),
        ] {
            let mut sprite = polychrome_sprite(texture);
            sprite.corner_smoothing = smoothing;
            scene.insert_primitive(sprite);
        }

        let batch_signatures = batches(&mut scene)
            .into_iter()
            .map(|batch| match batch {
                PrimitiveBatch::Quads { range, smoothed } => ("quad", None, range, smoothed),
                PrimitiveBatch::Shadows { range, smoothed } => ("shadow", None, range, smoothed),
                PrimitiveBatch::PolychromeSprites {
                    texture_id,
                    range,
                    smoothed,
                } => ("polychrome", Some(texture_id.index), range, smoothed),
                other => panic!("unexpected batch: {other:?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            batch_signatures,
            vec![
                ("quad", None, 0..2, false),
                ("quad", None, 2..4, true),
                ("quad", None, 4..5, false),
                ("shadow", None, 0..2, false),
                ("shadow", None, 2..4, true),
                ("shadow", None, 4..5, false),
                ("polychrome", Some(0), 0..2, false),
                ("polychrome", Some(0), 2..4, true),
                ("polychrome", Some(1), 4..6, true),
                ("polychrome", Some(1), 6..7, false),
            ]
        );
    }

    /// A 100x100 quad whose bounds don't overlap `full_bounds()` (used to exercise the
    /// order-reuse path: non-overlapping content reuses low draw-orders).
    fn detached_quad() -> Quad {
        let bounds = Bounds::new(point(scaled(200.0), scaled(200.0)), full_bounds().size);

        Quad {
            bounds,
            content_mask: ContentMask {
                bounds,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn boundary(is_start: bool) -> FilterBoundary {
        FilterBoundary {
            order: 0,
            bounds: full_bounds(),
            content_mask: mask(),
            corner_radii: Corners::default(),
            corner_smoothing: 0.0,
            filters: smallvec::smallvec![ScaledFilter::Blur(scaled(8.0))],
            opacity: 1.0,
            is_start,
        }
    }

    fn backdrop() -> BackdropFilter {
        BackdropFilter {
            bounds: full_bounds(),
            content_mask: mask(),
            corner_radii: Corners::default(),
            filters: smallvec::smallvec![ScaledFilter::Blur(scaled(20.0))],
            opacity: 1.0,
            ..Default::default()
        }
    }

    fn surface() -> PaintSurface {
        PaintSurface {
            order: 0,
            bounds: full_bounds(),
            content_mask: mask(),
            source: SurfaceSource::Unsupported(Size::default()),
        }
    }

    #[test]
    fn backdrop_filter_sorts_before_a_later_overlapping_quad() {
        let mut scene = Scene::default();
        scene.insert_primitive(quad());
        scene.insert_primitive(backdrop());
        scene.insert_primitive(quad());

        assert!(matches!(
            batches(&mut scene).as_slice(),
            [
                PrimitiveBatch::Quads { .. },
                PrimitiveBatch::BackdropFilters(_),
                PrimitiveBatch::Quads { .. }
            ]
        ));
    }

    #[test]
    fn isolation_commands_preserve_order_pairing_and_replay() {
        for active_layer in [false, true] {
            let mut scene = Scene::default();

            if active_layer {
                scene.push_layer(full_bounds());
            }

            scene.insert_primitive(quad());
            scene.insert_primitive(boundary(true));
            scene.insert_primitive(detached_quad());
            scene.insert_primitive(boundary(true));
            scene.insert_primitive(quad());
            scene.insert_primitive(boundary(true));
            scene.insert_primitive(surface());
            scene.insert_primitive(boundary(false));
            scene.insert_primitive(boundary(false));
            scene.insert_primitive(detached_quad());
            scene.insert_primitive(boundary(false));
            scene.insert_primitive(detached_quad());

            if active_layer {
                scene.pop_layer();
            }

            scene.finish();
            assert_eq!(scene.render_plan().requirements().isolated_target_count, 3);

            let commands: Vec<_> = scene
                .render_commands()
                .iter()
                .map(|command| match command {
                    RenderCommand::BeginFilter {
                        boundary_index,
                        target: FilterRenderTarget::Isolated(target),
                    } => ("start", Some((*boundary_index, target.as_usize()))),
                    RenderCommand::EndFilter {
                        boundary_index,
                        target: FilterRenderTarget::Isolated(target),
                        ..
                    } => ("end", Some((*boundary_index, target.as_usize()))),
                    RenderCommand::Batch(PrimitiveBatch::Quads { .. }) => ("quad", None),
                    RenderCommand::Batch(PrimitiveBatch::Surfaces(_)) => ("surface", None),
                    other => panic!("unexpected command: {other:?}"),
                })
                .collect();
            assert_eq!(
                commands,
                [
                    ("quad", None),
                    ("start", Some((0, 0))),
                    ("quad", None),
                    ("start", Some((1, 1))),
                    ("quad", None),
                    ("start", Some((2, 2))),
                    ("surface", None),
                    ("end", Some((2, 2))),
                    ("end", Some((1, 1))),
                    ("quad", None),
                    ("end", Some((0, 0))),
                    ("quad", None),
                ]
            );

            let mut replay = Scene::default();
            replay.replay(0..scene.len(), &scene);
            replay.finish();
            assert_eq!(replay.render_commands(), scene.render_commands());
            assert_eq!(replay.surface_opacities(), &[1.0]);
            assert!(replay.layer_stack.is_empty());
            assert!(!replay.is_in_filter_group());
        }
    }

    #[test]
    fn unmatched_filter_start_renders_inline() {
        let mut scene = Scene::default();
        scene.insert_primitive(boundary(true));
        scene.insert_primitive(quad());
        scene.finish();

        let mut commands = scene.render_commands().iter();
        assert!(matches!(
            commands.next(),
            Some(RenderCommand::BeginFilter {
                target: FilterRenderTarget::Inline,
                ..
            })
        ));
        assert!(matches!(
            commands.next(),
            Some(RenderCommand::Batch(PrimitiveBatch::Quads { .. }))
        ));
        assert!(commands.next().is_none());
        assert!(!scene.requires_offscreen_rendering());
    }

    #[test]
    fn finish_rebuilds_the_plan_after_direct_primitive_mutation() {
        let mut scene = Scene::default();
        scene.insert_primitive(quad());
        scene.finish();
        let commands = scene.render_commands().to_vec();

        // Primitive storage is public, so direct mutation must change the compiled plan.
        scene.quads.push(quad());
        scene.finish();
        assert_ne!(scene.render_commands(), commands);
        assert_eq!(scene.render_plan().requirements().instance_batch_count, 1);
    }

    #[test]
    fn render_plan_reuses_its_command_allocation_across_frames() {
        let mut scene = Scene::default();
        for _ in 0..32 {
            scene.insert_primitive(quad());
        }
        scene.finish();
        let capacity = scene.render_plan.commands.capacity();
        assert!(capacity >= 32);

        scene.clear();
        assert_eq!(scene.render_plan.commands.capacity(), capacity);
        scene.insert_primitive(quad());
        scene.finish();
        assert_eq!(scene.render_plan.commands.capacity(), capacity);
    }

    #[test]
    fn maximum_draw_order_is_not_treated_as_an_empty_batch_cursor() {
        let mut scene = Scene::default();
        let mut quad = quad();
        quad.order = DrawOrder::MAX;
        scene.quads.push(quad);
        scene.finish();

        assert!(matches!(
            scene.render_commands(),
            [RenderCommand::Batch(PrimitiveBatch::Quads { range, smoothed: false })]
                if range == &(0..1)
        ));
    }

    #[test]
    fn filter_requirements_distinguish_opacity_from_blur() {
        for (name, group, content_blur, backdrop_blur) in [
            ("ordinary quad", false, false, false),
            ("opacity", true, false, false),
            ("content blur", true, true, false),
            ("backdrop blur", false, false, true),
            ("backdrop inside opacity", true, false, true),
        ] {
            let mut scene = Scene::default();
            let mut start = boundary(true);

            if !content_blur {
                start.filters.clear();
                start.opacity = 0.5;
            }

            if group {
                scene.insert_primitive(start.clone());
            }

            if backdrop_blur {
                scene.insert_primitive(backdrop());
            }

            scene.insert_primitive(quad());

            if group {
                scene.insert_primitive(FilterBoundary {
                    is_start: false,
                    ..start
                });
            }

            scene.finish();

            let requirements = scene.render_plan().requirements();
            assert_eq!(
                (
                    requirements.uses_offscreen_target,
                    requirements.uses_blur_target
                ),
                (group || backdrop_blur, content_blur || backdrop_blur),
                "{name}"
            );
        }
    }
}
