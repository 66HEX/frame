use crate::{
    AbsoluteLength, App, Bounds, DefiniteLength, Display, Edges, GridTemplate, InlineContent,
    LayoutDirection, Length, Pixels, Point, Position, ResolvedDirection, SharedString, Size, Style,
    UnicodeBidi, VerticalAlign, Window, size,
    util::{
        ceil_to_device_pixel, round_half_toward_zero, round_stroke_to_device_pixel,
        round_to_device_pixel,
    },
};
use collections::{FxHashMap, FxHashSet};
use std::{cell::Cell, fmt::Debug, ops::Range, rc::Rc, sync::Arc};
use taffy::{
    TaffyTree, TraversePartialTree as _,
    geometry::{Point as TaffyPoint, Rect as TaffyRect, Size as TaffySize},
    prelude::{max_content, min_content},
    style::AvailableSpace as TaffyAvailableSpace,
    tree::NodeId,
};

#[cfg(feature = "stacker")]
type StackSafe<T> = stacksafe::StackSafe<T>;
#[cfg(not(feature = "stacker"))]
type StackSafe<T> = T;

type MeasureFn =
    dyn FnMut(Size<Option<Pixels>>, Size<AvailableSpace>, &mut Window, &mut App) -> Size<Pixels>;
type NodeMeasureFn = StackSafe<Box<MeasureFn>>;

struct NodeContext {
    measure: NodeMeasureFn,
    unicode_bidi: UnicodeBidi,
}

#[derive(Clone)]
pub(crate) struct LayoutDirectionHandle(Rc<Cell<ResolvedDirection>>);

impl LayoutDirectionHandle {
    fn new() -> Self {
        Self(Rc::new(Cell::new(ResolvedDirection::LeftToRight)))
    }

    pub(crate) fn get(&self) -> ResolvedDirection {
        self.0.get()
    }

    fn set(&self, direction: ResolvedDirection) {
        self.0.set(direction);
    }
}

struct DirectionMetadata {
    authored: LayoutDirection,
    unicode_bidi: UnicodeBidi,
    source_direction: Option<ResolvedDirection>,
    auto_direction_hint: Option<ResolvedDirection>,
    logical_children: Option<Vec<LayoutId>>,
    resolved: LayoutDirectionHandle,
}

pub struct TaffyLayoutEngine {
    taffy: TaffyTree<NodeContext>,
    absolute_layout_bounds: FxHashMap<LayoutId, Bounds<Pixels>>,
    /// Unrounded absolute border-box top-left per-node coordinate in device pixels.
    absolute_outer_origins: FxHashMap<LayoutId, Point<f32>>,
    computed_available_spaces: FxHashMap<LayoutId, Size<AvailableSpace>>,
    vertical_alignments: FxHashMap<LayoutId, VerticalAlign>,
    pub(crate) inline_content: FxHashMap<LayoutId, Arc<InlineContent>>,
    pub(crate) inline_fragments: FxHashMap<LayoutId, Arc<[Bounds<Pixels>]>>,
    display_and_position: FxHashMap<LayoutId, (Display, Position)>,
    directions: FxHashMap<LayoutId, DirectionMetadata>,
    layout_bounds_scratch_space: Vec<LayoutId>,
}

const EXPECT_MESSAGE: &str = "we should avoid taffy layout errors by construction if possible";

impl TaffyLayoutEngine {
    pub fn new() -> Self {
        let mut taffy = TaffyTree::new();
        taffy.disable_rounding();
        TaffyLayoutEngine {
            taffy,
            absolute_layout_bounds: FxHashMap::default(),
            absolute_outer_origins: FxHashMap::default(),
            computed_available_spaces: FxHashMap::default(),
            vertical_alignments: FxHashMap::default(),
            inline_content: FxHashMap::default(),
            inline_fragments: FxHashMap::default(),
            display_and_position: FxHashMap::default(),
            directions: FxHashMap::default(),
            layout_bounds_scratch_space: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.taffy.clear();
        self.absolute_layout_bounds.clear();
        self.absolute_outer_origins.clear();
        self.computed_available_spaces.clear();
        self.vertical_alignments.clear();
        self.inline_content.clear();
        self.inline_fragments.clear();
        self.display_and_position.clear();
        self.directions.clear();
    }

    pub fn request_layout(
        &mut self,
        style: Style,
        rem_size: Pixels,
        scale_factor: f32,
        children: &[LayoutId],
    ) -> LayoutId {
        let taffy_style = style.to_taffy(rem_size, scale_factor);

        let node_id = if children.is_empty() {
            self.taffy
                .new_leaf(taffy_style)
                .expect(EXPECT_MESSAGE)
                .into()
        } else {
            self.taffy
                // This is safe because LayoutId is repr(transparent) to taffy::tree::NodeId.
                .new_with_children(taffy_style, LayoutId::to_taffy_slice(children))
                .expect(EXPECT_MESSAGE)
                .into()
        };

        self.record_style(node_id, &style);

        node_id
    }

    pub fn request_measured_layout(
        &mut self,
        style: Style,
        rem_size: Pixels,
        scale_factor: f32,
        measure: impl FnMut(
            Size<Option<Pixels>>,
            Size<AvailableSpace>,
            &mut Window,
            &mut App,
        ) -> Size<Pixels>
        + 'static,
    ) -> LayoutId {
        let taffy_style = style.to_taffy(rem_size, scale_factor);
        let measure = Box::new(measure) as Box<MeasureFn>;
        #[cfg(feature = "stacker")]
        let measure = StackSafe::new(measure);

        let node_id = self
            .taffy
            .new_leaf_with_context(
                taffy_style,
                NodeContext {
                    measure,
                    unicode_bidi: style.effective_unicode_bidi(),
                },
            )
            .expect(EXPECT_MESSAGE)
            .into();
        self.record_style(node_id, &style);

        node_id
    }

    fn record_style(&mut self, node_id: LayoutId, style: &Style) {
        if style.vertical_align != VerticalAlign::Baseline {
            self.vertical_alignments
                .insert(node_id, style.vertical_align);
        }

        self.display_and_position
            .insert(node_id, (style.display, style.position));
        self.directions.insert(
            node_id,
            DirectionMetadata {
                authored: style.direction,
                unicode_bidi: style.effective_unicode_bidi(),
                source_direction: None,
                auto_direction_hint: None,
                logical_children: None,
                resolved: LayoutDirectionHandle::new(),
            },
        );
    }

    pub(crate) fn set_logical_children(&mut self, node_id: LayoutId, children: &[LayoutId]) {
        let actual_children = self.taffy.children(node_id.into()).expect(EXPECT_MESSAGE);
        let logical_children = (actual_children.as_slice() != LayoutId::to_taffy_slice(children))
            .then(|| children.to_vec());

        if let Some(metadata) = self.directions.get_mut(&node_id) {
            metadata.logical_children = logical_children;
        }
    }

    pub(crate) fn set_direction_text(&mut self, node_id: LayoutId, text: SharedString) {
        if let Some(metadata) = self.directions.get_mut(&node_id) {
            metadata.source_direction = ResolvedDirection::from_first_strong(&text);
        }
    }

    pub(crate) fn set_auto_direction_hint(
        &mut self,
        node_id: LayoutId,
        direction: Option<ResolvedDirection>,
    ) {
        if let Some(metadata) = self.directions.get_mut(&node_id) {
            metadata.auto_direction_hint = direction;
        }
    }

    pub(crate) fn auto_direction_contribution(
        &self,
        node_id: LayoutId,
    ) -> Option<ResolvedDirection> {
        let metadata = self.directions.get(&node_id)?;

        (metadata.authored == LayoutDirection::Inherit)
            .then(|| self.auto_direction(node_id))
            .flatten()
    }

    pub(crate) fn direction_handle(&self, node_id: LayoutId) -> LayoutDirectionHandle {
        self.directions[&node_id].resolved.clone()
    }

    pub(crate) fn directionality(&self, node_id: LayoutId) -> (ResolvedDirection, UnicodeBidi) {
        let metadata = &self.directions[&node_id];

        (metadata.resolved.get(), metadata.unicode_bidi)
    }

    fn auto_direction(&self, node_id: LayoutId) -> Option<ResolvedDirection> {
        let metadata = self.directions.get(&node_id)?;

        if let Some(direction) = metadata.source_direction.or(metadata.auto_direction_hint) {
            return Some(direction);
        }

        if let Some(children) = &metadata.logical_children {
            return children
                .iter()
                .find_map(|child| self.auto_direction_contribution(*child));
        }

        self.taffy
            .children(node_id.into())
            .expect(EXPECT_MESSAGE)
            .into_iter()
            .find_map(|child| self.auto_direction_contribution(child.into()))
    }

    fn resolve_directions(
        &mut self,
        root_id: LayoutId,
        inherited: ResolvedDirection,
    ) -> Vec<LayoutId> {
        let mut visited = FxHashSet::default();
        let mut changed = Vec::new();
        self.resolve_direction_subtree(root_id, inherited, &mut visited, &mut changed);
        changed
    }

    fn resolve_direction_subtree(
        &mut self,
        node_id: LayoutId,
        inherited: ResolvedDirection,
        visited: &mut FxHashSet<LayoutId>,
        changed: &mut Vec<LayoutId>,
    ) {
        if !visited.insert(node_id) {
            return;
        }

        let Some(metadata) = self.directions.get(&node_id) else {
            return;
        };
        let resolved = match metadata.authored {
            LayoutDirection::Inherit => inherited,
            LayoutDirection::LeftToRight => ResolvedDirection::LeftToRight,
            LayoutDirection::RightToLeft => ResolvedDirection::RightToLeft,
            LayoutDirection::Auto => self
                .auto_direction(node_id)
                .unwrap_or(ResolvedDirection::LeftToRight),
        };
        let actual_children = self.taffy.children(node_id.into()).expect(EXPECT_MESSAGE);
        let logical_children = metadata.logical_children.clone().unwrap_or_default();
        if metadata.resolved.get() != resolved {
            changed.push(node_id);
        }
        metadata.resolved.set(resolved);

        let taffy_direction = match resolved {
            ResolvedDirection::LeftToRight => taffy::style::Direction::Ltr,
            ResolvedDirection::RightToLeft => taffy::style::Direction::Rtl,
        };
        if self
            .taffy
            .style(node_id.into())
            .expect(EXPECT_MESSAGE)
            .direction
            != taffy_direction
        {
            let mut style = self
                .taffy
                .style(node_id.into())
                .expect(EXPECT_MESSAGE)
                .clone();
            style.direction = taffy_direction;
            self.taffy
                .set_style(node_id.into(), style)
                .expect(EXPECT_MESSAGE);
        }

        for child_id in actual_children
            .into_iter()
            .map(LayoutId::from)
            .chain(logical_children)
        {
            self.resolve_direction_subtree(child_id, resolved, visited, changed);
        }
    }

    pub(crate) fn vertical_align(&self, node_id: LayoutId) -> VerticalAlign {
        self.vertical_alignments
            .get(&node_id)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn display_and_position(&self, node_id: LayoutId) -> (Display, Position) {
        self.display_and_position[&node_id]
    }

    /// Places a detached inline box, invalidating cached positions of its descendants.
    pub(crate) fn place_inline(&mut self, node_id: LayoutId, bounds: Bounds<Pixels>, scale: f32) {
        self.clear_cached_bounds(node_id);

        self.absolute_layout_bounds.insert(node_id, bounds);
        self.absolute_outer_origins.insert(
            node_id,
            bounds.origin.map(|position| f32::from(position) * scale),
        );
    }

    /// Treats any `auto` dimension of the given node's style as filling `size`.
    ///
    /// This is applied to window roots before layout so they behave like the
    /// root element on the web, which stretches to fill the initial containing
    /// block (the viewport) unless given an explicit size. Explicitly styled
    /// dimensions are preserved.
    pub fn stretch_auto_size_to_fill(
        &mut self,
        id: LayoutId,
        size: Size<Pixels>,
        scale_factor: f32,
    ) {
        let style = self.taffy.style(id.0).expect(EXPECT_MESSAGE);
        let stretch_width = style.size.width.is_auto();
        let stretch_height = style.size.height.is_auto();
        if !stretch_width && !stretch_height {
            return;
        }
        let mut style = style.clone();
        if stretch_width {
            style.size.width =
                taffy::style::Dimension::length(round_to_device_pixel(size.width.0, scale_factor));
        }
        if stretch_height {
            style.size.height =
                taffy::style::Dimension::length(round_to_device_pixel(size.height.0, scale_factor));
        }
        self.taffy.set_style(id.0, style).expect(EXPECT_MESSAGE);
    }

    // Used to understand performance
    #[allow(dead_code)]
    fn count_all_children(&self, parent: LayoutId) -> anyhow::Result<u32> {
        let mut count = 0;

        for child in self.taffy.children(parent.0)? {
            // Count this child.
            count += 1;

            // Count all of this child's children.
            count += self.count_all_children(LayoutId(child))?
        }

        Ok(count)
    }

    // Used to understand performance
    #[allow(dead_code)]
    fn max_depth(&self, depth: u32, parent: LayoutId) -> anyhow::Result<u32> {
        println!(
            "{parent:?} at depth {depth} has {} children",
            self.taffy.child_count(parent.0)
        );

        let mut max_child_depth = 0;

        for child in self.taffy.children(parent.0)? {
            max_child_depth = std::cmp::max(max_child_depth, self.max_depth(0, LayoutId(child))?);
        }

        Ok(depth + 1 + max_child_depth)
    }

    // Used to understand performance
    #[allow(dead_code)]
    fn get_edges(&self, parent: LayoutId) -> anyhow::Result<Vec<(LayoutId, LayoutId)>> {
        let mut edges = Vec::new();

        for child in self.taffy.children(parent.0)? {
            edges.push((parent, LayoutId(child)));

            edges.extend(self.get_edges(LayoutId(child))?);
        }

        Ok(edges)
    }

    #[cfg_attr(feature = "stacker", stacksafe::stacksafe)]
    pub fn compute_layout(
        &mut self,
        id: LayoutId,
        available_space: Size<AvailableSpace>,
        inherited_direction: ResolvedDirection,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Leaving this here until we have a better instrumentation approach.
        // println!("Laying out {} children", self.count_all_children(id)?);
        // println!("Max layout depth: {}", self.max_depth(0, id)?);

        // Output the edges (branches) of the tree in Mermaid format for visualization.
        // println!("Edges:");
        // for (a, b) in self.get_edges(id)? {
        //     println!("N{} --> N{}", u64::from(a), u64::from(b));
        // }
        //

        let changed_ids = self.resolve_directions(id, inherited_direction);

        for changed_id in changed_ids {
            if changed_id == id {
                continue;
            }

            let Some(available_space) = self.computed_available_spaces.get(&changed_id).copied()
            else {
                continue;
            };

            self.clear_cached_bounds(changed_id);
            self.compute_resolved_layout(changed_id, available_space, window, cx);
        }

        let previous_space = self.computed_available_spaces.insert(id, available_space);

        if previous_space.is_some() {
            self.clear_cached_bounds(id);
        }

        self.compute_resolved_layout(id, available_space, window, cx);
    }

    fn clear_cached_bounds(&mut self, id: LayoutId) {
        let stack = &mut self.layout_bounds_scratch_space;
        stack.push(id);
        while let Some(id) = stack.pop() {
            self.absolute_layout_bounds.remove(&id);
            self.absolute_outer_origins.remove(&id);
            stack.extend(
                self.taffy
                    .children(id.into())
                    .expect(EXPECT_MESSAGE)
                    .into_iter()
                    .map(LayoutId::from),
            );
        }
    }

    fn compute_resolved_layout(
        &mut self,
        id: LayoutId,
        available_space: Size<AvailableSpace>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let scale_factor = window.scale_factor();

        let transform = |v: AvailableSpace| match v {
            AvailableSpace::Definite(pixels) => {
                AvailableSpace::Definite(Pixels(pixels.0 * scale_factor))
            }
            AvailableSpace::MinContent => AvailableSpace::MinContent,
            AvailableSpace::MaxContent => AvailableSpace::MaxContent,
        };
        let available_space = size(
            transform(available_space.width),
            transform(available_space.height),
        );

        self.taffy
            .compute_layout_with_measure(
                id.into(),
                available_space.into(),
                |known_dimensions, available_space, _id, node_context, style| {
                    let Some(node_context) = node_context else {
                        return taffy::geometry::Size::default();
                    };

                    let known_dimensions = Size {
                        width: known_dimensions.width.map(|e| Pixels(e / scale_factor)),
                        height: known_dimensions.height.map(|e| Pixels(e / scale_factor)),
                    };

                    let available_space: Size<AvailableSpace> = available_space.into();
                    let untransform = |ev: AvailableSpace| match ev {
                        AvailableSpace::Definite(pixels) => {
                            AvailableSpace::Definite(Pixels(pixels.0 / scale_factor))
                        }
                        AvailableSpace::MinContent => AvailableSpace::MinContent,
                        AvailableSpace::MaxContent => AvailableSpace::MaxContent,
                    };
                    let available_space = size(
                        untransform(available_space.width),
                        untransform(available_space.height),
                    );

                    let resolved_direction = match style.direction {
                        taffy::style::Direction::Ltr => ResolvedDirection::LeftToRight,
                        taffy::style::Direction::Rtl => ResolvedDirection::RightToLeft,
                    };
                    let previous_direction =
                        std::mem::replace(&mut window.measurement_direction, resolved_direction);
                    let previous_unicode_bidi = std::mem::replace(
                        &mut window.measurement_unicode_bidi,
                        node_context.unicode_bidi,
                    );
                    let measured_size: Size<Pixels> =
                        (node_context.measure)(known_dimensions, available_space, window, cx);
                    window.measurement_direction = previous_direction;
                    window.measurement_unicode_bidi = previous_unicode_bidi;

                    snap_measured_size_to_device_pixels(measured_size, scale_factor).into()
                },
            )
            .expect(EXPECT_MESSAGE);
    }

    // Pixel snapping
    //
    // Painting primitives at non-integer pixel coordinates produces blurry
    // output. Pixel snapping converts layout coordinates into integer
    // device-pixel coordinates so painted edges land exactly on physical
    // pixel boundaries.
    //
    // Non-integer coordinates can arise for several reasons, including:
    //   - flex distribution, percentages, centering, and text measurement
    //     can produce fractional element sizes and positions;
    //   - at fractional scale factors (for example 125% or 150%), integer
    //     logical-pixel values can map to non-integer device-pixel values.
    //
    // We pixel-snap by rounding in device-pixel space, after multiplying
    // by `scale_factor`, so that snapping targets physical pixels. Bounds
    // are divided by `scale_factor` before being returned to GPUI.
    //
    // Midpoints are rounded toward zero. This is a stylistic choice: a
    // 1-logical-pixel line at 150% scale should render as 1 dp rather than
    // 2 dp.
    //
    // Pixel snapping is done in two phases:
    //
    //  1. Pre-layout metric snapping. Before Taffy computes layout, all
    //     authored absolute lengths are rounded in `to_taffy`. This
    //     includes borders, padding, gaps, and explicit sizes.
    //     Custom-measured leaf nodes have their measured sizes rounded up
    //     to integer device-pixel lengths.
    //
    //  2. Post-layout edge snapping. After Taffy resolves the tree, layout
    //     relationships such as flex shares, grid tracks, percentages, and
    //     centering can produce new fractional edge positions. Boxes now
    //     have edges in absolute coordinates, and snapping must decide
    //     where those edges land on the device-pixel grid.
    //
    // Ideally, post-layout snapping would satisfy:
    //
    //  - Edge closure. Two raw layout edges at the same absolute position
    //    should snap to the same pixel column.
    //  - Translation stability. A component's internal geometry should not
    //    change when it moves to a new absolute position.
    //
    // These goals are in tension because rounding is not associative.
    // The simple local schemes make different tradeoffs:
    //
    //  - Absolute edge rounding gives each window coordinate one answer,
    //    so coincident edges always close globally. But a span's snapped
    //    length is `round(far) - round(near)`, which may change by 1 dp
    //    as its absolute origin moves.
    //
    //  - Parent-relative edge rounding rounds each child inside its
    //    parent's coordinate space. This guarantees translation stability,
    //    but a shared edge reached through different parents can
    //    accumulate different rounding, causing non-closure between
    //    cousins.
    //
    //  - Length rounding rounds each width, height, and thickness
    //    independently and then places boxes from those rounded lengths.
    //    Sizes stay stable under translation, but neighboring boxes derive
    //    their shared boundary from different sources, so closure is not
    //    guaranteed.
    //
    // We apply absolute edge rounding for each element's outer box in
    // post-layout rounding to preserve closure. Border and padding widths
    // are not touched by post-layout rounding; they keep their pre-layout
    // rounded value so that they remain stable under translation.
    //
    // This gives both closure and translation stability in the case that
    // all local metrics are integer device-pixel lengths. Pre-layout
    // rounding covers that in most cases. The exception is metrics
    // resolved by layout relationships, such as percentages. Outer box
    // edges will still close globally, and painted border widths are still
    // snapped independently, but the raw content-box origin can carry a
    // 1dp residual into descendants.

    pub fn layout_bounds(&mut self, id: LayoutId, scale_factor: f32) -> Bounds<Pixels> {
        if let Some(layout) = self.absolute_layout_bounds.get(&id).cloned() {
            return layout;
        }

        let layout = self.taffy.layout(id.into()).expect(EXPECT_MESSAGE);
        let layout_location = layout.location;
        let layout_size = layout.size;
        let parent = self.taffy.parent(id.0);

        let absolute_outer_origin = match parent {
            Some(parent_id) => {
                let parent_id = LayoutId::from(parent_id);
                self.layout_bounds(parent_id, scale_factor);
                let parent_origin = *self
                    .absolute_outer_origins
                    .get(&parent_id)
                    .expect("parent absolute outer origin should be cached");
                parent_origin + Point::from(layout_location)
            }
            None => Point::from(layout_location),
        };
        self.absolute_outer_origins
            .insert(id, absolute_outer_origin);

        let absolute_far = absolute_outer_origin + Point::from(Size::from(layout_size));
        let snapped_bounds = Bounds::from_corners(
            absolute_outer_origin.map(round_half_toward_zero),
            absolute_far.map(round_half_toward_zero),
        );

        let bounds = (snapped_bounds / scale_factor).map(Pixels);
        self.absolute_layout_bounds.insert(id, bounds);
        bounds
    }

    /// Returns bounds whose origin and size are snapped in the coordinate space of the parent.
    ///
    /// Text uses this placement so its offset within a containing element remains stable when the
    /// element moves across the device-pixel grid. Other layout boxes continue to use absolute edge
    /// snapping through [`Self::layout_bounds`] so coincident edges still close.
    pub(crate) fn parent_relative_layout_bounds(
        &mut self,
        node_id: LayoutId,
        scale_factor: f32,
    ) -> Bounds<Pixels> {
        if self.inline_fragments.contains_key(&node_id) {
            return self.layout_bounds(node_id, scale_factor);
        }

        let Some(parent_id) = self.taffy.parent(node_id.0).map(LayoutId::from) else {
            return self.layout_bounds(node_id, scale_factor);
        };
        let parent_bounds = self.layout_bounds(parent_id, scale_factor);
        let layout = self.taffy.layout(node_id.into()).expect(EXPECT_MESSAGE);
        let local_origin = Point::from(layout.location).map(round_half_toward_zero);
        let local_size = Size::from(layout.size).map(round_half_toward_zero);

        Bounds::new(
            parent_bounds.origin + (local_origin / scale_factor).map(Pixels),
            (local_size / scale_factor).map(Pixels),
        )
    }
}

/// A unique identifier for a layout node, generated when requesting a layout from Taffy
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(transparent)]
pub struct LayoutId(NodeId);

impl LayoutId {
    fn to_taffy_slice(node_ids: &[Self]) -> &[taffy::NodeId] {
        // SAFETY: LayoutId is repr(transparent) to taffy::tree::NodeId.
        unsafe { std::mem::transmute::<&[LayoutId], &[taffy::NodeId]>(node_ids) }
    }
}

impl std::hash::Hash for LayoutId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        u64::from(self.0).hash(state);
    }
}

impl From<NodeId> for LayoutId {
    fn from(node_id: NodeId) -> Self {
        Self(node_id)
    }
}

impl From<LayoutId> for NodeId {
    fn from(layout_id: LayoutId) -> NodeId {
        layout_id.0
    }
}

fn snap_measured_size_to_device_pixels(size: Size<Pixels>, scale_factor: f32) -> Size<f32> {
    size.map(|d| ceil_to_device_pixel(d.0.max(0.0), scale_factor))
}

fn border_widths_to_taffy(
    widths: &Edges<AbsoluteLength>,
    rem_size: Pixels,
    scale_factor: f32,
) -> TaffyRect<taffy::style::LengthPercentage> {
    let snap = |w: &AbsoluteLength| {
        taffy::style::LengthPercentage::length(round_stroke_to_device_pixel(
            w.to_pixels(rem_size).0,
            scale_factor,
        ))
    };
    TaffyRect {
        top: snap(&widths.top),
        right: snap(&widths.right),
        bottom: snap(&widths.bottom),
        left: snap(&widths.left),
    }
}

// Converts padding, snapping each auto-sized axis as a proportional pair.
fn padding_to_taffy(
    padding: &Edges<DefiniteLength>,
    rem_size: Pixels,
    scale_factor: f32,
    proportional_horizontal: bool,
    proportional_vertical: bool,
) -> TaffyRect<taffy::style::LengthPercentage> {
    // Snaps the combined length, then divides it by the authored ratio.
    fn snap_absolute_pair(
        first: AbsoluteLength,
        second: AbsoluteLength,
        rem_size: Pixels,
        scale_factor: f32,
    ) -> (f32, f32) {
        let first = first.to_pixels(rem_size).0;
        let second = second.to_pixels(rem_size).0;
        let total = first + second;

        if first < 0.0 || second < 0.0 || total == 0.0 {
            return (
                round_to_device_pixel(first, scale_factor),
                round_to_device_pixel(second, scale_factor),
            );
        }

        let snapped_total = round_to_device_pixel(total, scale_factor);
        let snapped_first = snapped_total * (first / total);

        (snapped_first, snapped_total - snapped_first)
    }

    // Keeps normal per-edge conversion for explicit axes and percentage padding.
    fn convert_pair(
        first: &DefiniteLength,
        second: &DefiniteLength,
        rem_size: Pixels,
        scale_factor: f32,
        proportional: bool,
    ) -> (
        taffy::style::LengthPercentage,
        taffy::style::LengthPercentage,
    ) {
        if proportional
            && let (DefiniteLength::Absolute(first), DefiniteLength::Absolute(second)) =
                (*first, *second)
        {
            let (first, second) = snap_absolute_pair(first, second, rem_size, scale_factor);

            (
                taffy::style::LengthPercentage::length(first),
                taffy::style::LengthPercentage::length(second),
            )
        } else {
            (
                first.to_taffy(rem_size, scale_factor),
                second.to_taffy(rem_size, scale_factor),
            )
        }
    }

    let (left, right) = convert_pair(
        &padding.left,
        &padding.right,
        rem_size,
        scale_factor,
        proportional_horizontal,
    );
    let (top, bottom) = convert_pair(
        &padding.top,
        &padding.bottom,
        rem_size,
        scale_factor,
        proportional_vertical,
    );

    TaffyRect {
        top,
        right,
        bottom,
        left,
    }
}

trait ToTaffy<Output> {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> Output;
}

impl ToTaffy<taffy::style::Style> for Style {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> taffy::style::Style {
        use taffy::style_helpers::{fr, length, minmax, repeat};

        fn to_grid_line(
            placement: &Range<crate::GridPlacement>,
        ) -> taffy::Line<taffy::GridPlacement> {
            taffy::Line {
                start: placement.start.into(),
                end: placement.end.into(),
            }
        }

        fn to_grid_repeat<T: taffy::style::CheapCloneStr>(
            unit: &Option<GridTemplate>,
        ) -> Vec<taffy::GridTemplateComponent<T>> {
            unit.map(|template| {
                match template.min_size {
                    // grid-template-*: repeat(<number>, minmax(0, 1fr));
                    crate::GridTemplateMinSize::Zero => {
                        vec![repeat(
                            template.repeat,
                            vec![minmax(length(0.0_f32), fr(1.0_f32))],
                        )]
                    }
                    // grid-template-*: repeat(<number>, minmax(min-content, 1fr));
                    crate::GridTemplateMinSize::MinContent => {
                        vec![repeat(
                            template.repeat,
                            vec![minmax(min_content(), fr(1.0_f32))],
                        )]
                    }
                    // grid-template-*: repeat(<number>, minmax(0, max-content))
                    crate::GridTemplateMinSize::MaxContent => {
                        vec![repeat(
                            template.repeat,
                            vec![minmax(length(0.0_f32), max_content())],
                        )]
                    }
                }
            })
            .unwrap_or_default()
        }

        taffy::style::Style {
            display: self.display.into(),
            direction: taffy::style::Direction::Ltr,
            overflow: self.overflow.into(),
            scrollbar_width: self.scrollbar_width.to_taffy(rem_size, scale_factor),
            position: self.position.into(),
            inset: self.inset.to_taffy(rem_size, scale_factor),
            size: self.size.to_taffy(rem_size, scale_factor),
            min_size: self.min_size.to_taffy(rem_size, scale_factor),
            max_size: self.max_size.to_taffy(rem_size, scale_factor),
            aspect_ratio: self.aspect_ratio,
            margin: self.margin.to_taffy(rem_size, scale_factor),
            padding: padding_to_taffy(
                &self.padding,
                rem_size,
                scale_factor,
                self.size.width == Length::Auto,
                self.size.height == Length::Auto,
            ),
            border: border_widths_to_taffy(&self.border_widths, rem_size, scale_factor),
            align_items: self.align_items.map(|x| x.into()),
            align_self: self.align_self.map(|x| x.into()),
            align_content: self.align_content.map(|x| x.into()),
            justify_content: self.justify_content.map(|x| x.into()),
            gap: self.gap.to_taffy(rem_size, scale_factor),
            flex_direction: self.flex_direction.into(),
            flex_wrap: self.flex_wrap.into(),
            flex_basis: self.flex_basis.to_taffy(rem_size, scale_factor),
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            grid_template_rows: to_grid_repeat(&self.grid_rows),
            grid_template_columns: to_grid_repeat(&self.grid_cols),
            grid_row: self
                .grid_location
                .as_ref()
                .map(|location| to_grid_line(&location.row))
                .unwrap_or_default(),
            grid_column: self
                .grid_location
                .as_ref()
                .map(|location| to_grid_line(&location.column))
                .unwrap_or_default(),
            ..Default::default()
        }
    }
}

impl ToTaffy<f32> for AbsoluteLength {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> f32 {
        round_to_device_pixel(self.to_pixels(rem_size).0, scale_factor)
    }
}

impl ToTaffy<taffy::style::LengthPercentageAuto> for Length {
    fn to_taffy(
        &self,
        rem_size: Pixels,
        scale_factor: f32,
    ) -> taffy::prelude::LengthPercentageAuto {
        match self {
            Length::Definite(length) => length.to_taffy(rem_size, scale_factor),
            Length::Auto => taffy::prelude::LengthPercentageAuto::auto(),
        }
    }
}

impl ToTaffy<taffy::style::Dimension> for Length {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> taffy::prelude::Dimension {
        match self {
            Length::Definite(length) => length.to_taffy(rem_size, scale_factor),
            Length::Auto => taffy::prelude::Dimension::auto(),
        }
    }
}

impl ToTaffy<taffy::style::LengthPercentage> for DefiniteLength {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> taffy::style::LengthPercentage {
        match self {
            DefiniteLength::Absolute(length) => length.to_taffy(rem_size, scale_factor),
            DefiniteLength::Relative(relative) => {
                taffy::style::LengthPercentage::percent(relative.0)
            }
        }
    }
}

impl ToTaffy<taffy::style::LengthPercentageAuto> for DefiniteLength {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> taffy::style::LengthPercentageAuto {
        match self {
            DefiniteLength::Absolute(length) => length.to_taffy(rem_size, scale_factor),
            DefiniteLength::Relative(relative) => {
                taffy::style::LengthPercentageAuto::percent(relative.0)
            }
        }
    }
}

impl ToTaffy<taffy::style::Dimension> for DefiniteLength {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> taffy::style::Dimension {
        match self {
            DefiniteLength::Absolute(length) => length.to_taffy(rem_size, scale_factor),
            DefiniteLength::Relative(relative) => taffy::style::Dimension::percent(relative.0),
        }
    }
}

impl ToTaffy<taffy::style::LengthPercentage> for AbsoluteLength {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> taffy::style::LengthPercentage {
        taffy::style::LengthPercentage::length(self.to_taffy(rem_size, scale_factor))
    }
}

impl ToTaffy<taffy::style::LengthPercentageAuto> for AbsoluteLength {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> taffy::style::LengthPercentageAuto {
        taffy::style::LengthPercentageAuto::length(self.to_taffy(rem_size, scale_factor))
    }
}

impl ToTaffy<taffy::style::Dimension> for AbsoluteLength {
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> taffy::style::Dimension {
        taffy::style::Dimension::length(self.to_taffy(rem_size, scale_factor))
    }
}

impl<T, T2> From<TaffyPoint<T>> for Point<T2>
where
    T: Into<T2>,
    T2: Clone + Debug + Default + PartialEq,
{
    fn from(point: TaffyPoint<T>) -> Point<T2> {
        Point {
            x: point.x.into(),
            y: point.y.into(),
        }
    }
}

impl<T, T2> From<Point<T>> for TaffyPoint<T2>
where
    T: Into<T2> + Clone + Debug + Default + PartialEq,
{
    fn from(val: Point<T>) -> Self {
        TaffyPoint {
            x: val.x.into(),
            y: val.y.into(),
        }
    }
}

impl<T, U> ToTaffy<TaffySize<U>> for Size<T>
where
    T: ToTaffy<U> + Clone + Debug + Default + PartialEq,
{
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> TaffySize<U> {
        TaffySize {
            width: self.width.to_taffy(rem_size, scale_factor),
            height: self.height.to_taffy(rem_size, scale_factor),
        }
    }
}

impl<T, U> ToTaffy<TaffyRect<U>> for Edges<T>
where
    T: ToTaffy<U> + Clone + Debug + Default + PartialEq,
{
    fn to_taffy(&self, rem_size: Pixels, scale_factor: f32) -> TaffyRect<U> {
        TaffyRect {
            top: self.top.to_taffy(rem_size, scale_factor),
            right: self.right.to_taffy(rem_size, scale_factor),
            bottom: self.bottom.to_taffy(rem_size, scale_factor),
            left: self.left.to_taffy(rem_size, scale_factor),
        }
    }
}

impl<T, U> From<TaffySize<T>> for Size<U>
where
    T: Into<U>,
    U: Clone + Debug + Default + PartialEq,
{
    fn from(taffy_size: TaffySize<T>) -> Self {
        Size {
            width: taffy_size.width.into(),
            height: taffy_size.height.into(),
        }
    }
}

impl<T, U> From<Size<T>> for TaffySize<U>
where
    T: Into<U> + Clone + Debug + Default + PartialEq,
{
    fn from(size: Size<T>) -> Self {
        TaffySize {
            width: size.width.into(),
            height: size.height.into(),
        }
    }
}

/// The space available for an element to be laid out in
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub enum AvailableSpace {
    /// The amount of space available is the specified number of pixels
    Definite(Pixels),
    /// The amount of space available is indefinite and the node should be laid out under a min-content constraint
    #[default]
    MinContent,
    /// The amount of space available is indefinite and the node should be laid out under a max-content constraint
    MaxContent,
}

impl AvailableSpace {
    /// Returns a `Size` with both width and height set to `AvailableSpace::MinContent`.
    ///
    /// This function is useful when you want to create a `Size` with the minimum content constraints
    /// for both dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use gpui::AvailableSpace;
    /// let min_content_size = AvailableSpace::min_size();
    /// assert_eq!(min_content_size.width, AvailableSpace::MinContent);
    /// assert_eq!(min_content_size.height, AvailableSpace::MinContent);
    /// ```
    pub const fn min_size() -> Size<Self> {
        Size {
            width: Self::MinContent,
            height: Self::MinContent,
        }
    }
}

impl From<AvailableSpace> for TaffyAvailableSpace {
    fn from(space: AvailableSpace) -> TaffyAvailableSpace {
        match space {
            AvailableSpace::Definite(Pixels(value)) => TaffyAvailableSpace::Definite(value),
            AvailableSpace::MinContent => TaffyAvailableSpace::MinContent,
            AvailableSpace::MaxContent => TaffyAvailableSpace::MaxContent,
        }
    }
}

impl From<TaffyAvailableSpace> for AvailableSpace {
    fn from(space: TaffyAvailableSpace) -> AvailableSpace {
        match space {
            TaffyAvailableSpace::Definite(value) => AvailableSpace::Definite(Pixels(value)),
            TaffyAvailableSpace::MinContent => AvailableSpace::MinContent,
            TaffyAvailableSpace::MaxContent => AvailableSpace::MaxContent,
        }
    }
}

impl From<Pixels> for AvailableSpace {
    fn from(pixels: Pixels) -> Self {
        AvailableSpace::Definite(pixels)
    }
}

impl From<Size<Pixels>> for Size<AvailableSpace> {
    fn from(size: Size<Pixels>) -> Self {
        Size {
            width: AvailableSpace::Definite(size.width),
            height: AvailableSpace::Definite(size.height),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::px;

    fn direction_style(direction: LayoutDirection) -> Style {
        Style {
            direction,
            ..Default::default()
        }
    }

    #[test]
    fn direction_defaults_to_ltr_and_inherits_through_overrides() {
        let mut engine = TaffyLayoutEngine::new();
        let inherited_grandchild = engine.request_layout(
            direction_style(LayoutDirection::Inherit),
            px(16.0),
            1.0,
            &[],
        );
        let rtl_child = engine.request_layout(
            direction_style(LayoutDirection::RightToLeft),
            px(16.0),
            1.0,
            &[inherited_grandchild],
        );
        let root = engine.request_layout(
            direction_style(LayoutDirection::Inherit),
            px(16.0),
            1.0,
            &[rtl_child],
        );

        let _ = engine.resolve_directions(root, ResolvedDirection::LeftToRight);

        assert_eq!(
            engine.direction_handle(root).get(),
            ResolvedDirection::LeftToRight
        );
        assert_eq!(
            engine.direction_handle(rtl_child).get(),
            ResolvedDirection::RightToLeft
        );
        assert_eq!(
            engine.taffy.style(rtl_child.into()).unwrap().direction,
            taffy::style::Direction::Rtl
        );
        assert_eq!(
            engine.direction_handle(inherited_grandchild).get(),
            ResolvedDirection::RightToLeft
        );
    }

    #[test]
    fn auto_direction_uses_first_eligible_strong_descendant() {
        let mut engine = TaffyLayoutEngine::new();
        let neutral = engine.request_layout(
            direction_style(LayoutDirection::Inherit),
            px(16.0),
            1.0,
            &[],
        );
        engine.set_direction_text(neutral, "123 …".into());
        let arabic = engine.request_layout(
            direction_style(LayoutDirection::Inherit),
            px(16.0),
            1.0,
            &[],
        );
        engine.set_direction_text(arabic, "مرحبا".into());
        let root = engine.request_layout(
            direction_style(LayoutDirection::Auto),
            px(16.0),
            1.0,
            &[neutral, arabic],
        );

        let _ = engine.resolve_directions(root, ResolvedDirection::LeftToRight);

        assert_eq!(
            engine.direction_handle(root).get(),
            ResolvedDirection::RightToLeft
        );
        assert_eq!(
            engine.direction_handle(neutral).get(),
            ResolvedDirection::RightToLeft
        );
    }

    #[test]
    fn auto_direction_excludes_directional_subtrees_and_falls_back_to_ltr() {
        let mut engine = TaffyLayoutEngine::new();
        let explicit_child = engine.request_layout(
            direction_style(LayoutDirection::RightToLeft),
            px(16.0),
            1.0,
            &[],
        );
        engine.set_direction_text(explicit_child, "مرحبا".into());
        let root = engine.request_layout(
            direction_style(LayoutDirection::Auto),
            px(16.0),
            1.0,
            &[explicit_child],
        );

        let _ = engine.resolve_directions(root, ResolvedDirection::LeftToRight);

        assert_eq!(
            engine.direction_handle(root).get(),
            ResolvedDirection::LeftToRight
        );
        assert_eq!(
            engine.direction_handle(explicit_child).get(),
            ResolvedDirection::RightToLeft
        );
    }

    #[test]
    fn auto_direction_updates_when_source_text_changes() {
        let mut engine = TaffyLayoutEngine::new();
        let text = engine.request_layout(
            direction_style(LayoutDirection::Inherit),
            px(16.0),
            1.0,
            &[],
        );
        let root = engine.request_layout(
            direction_style(LayoutDirection::Auto),
            px(16.0),
            1.0,
            &[text],
        );
        engine.set_direction_text(text, "English".into());
        let _ = engine.resolve_directions(root, ResolvedDirection::LeftToRight);
        assert_eq!(
            engine.direction_handle(root).get(),
            ResolvedDirection::LeftToRight
        );

        engine.set_direction_text(text, "مرحبا".into());
        let changed = engine.resolve_directions(root, ResolvedDirection::LeftToRight);
        assert!(changed.contains(&root));
        assert_eq!(
            engine.direction_handle(root).get(),
            ResolvedDirection::RightToLeft
        );
    }

    #[test]
    fn auto_sized_axes_snap_padding_proportionally() {
        use taffy::style::LengthPercentage;

        let mut style = Style {
            padding: Edges {
                top: Pixels(6.5).into(),
                right: Pixels(5.0).into(),
                bottom: Pixels(6.5).into(),
                left: Pixels(2.5).into(),
            },
            ..Default::default()
        };

        let padding = style.to_taffy(Pixels(16.0), 1.0).padding;
        let expected_left = 7.0 * (2.5 / 7.5);
        assert_eq!(padding.left, LengthPercentage::length(expected_left));
        assert_eq!(padding.right, LengthPercentage::length(7.0 - expected_left));
        assert_eq!(padding.top, LengthPercentage::length(6.5));
        assert_eq!(padding.bottom, LengthPercentage::length(6.5));

        style.size.width = Length::Definite(Pixels(100.0).into());
        let padding = style.to_taffy(Pixels(16.0), 1.0).padding;
        assert_eq!(padding.left, LengthPercentage::length(2.0));
        assert_eq!(padding.right, LengthPercentage::length(5.0));
        assert_eq!(padding.top, LengthPercentage::length(6.5));
        assert_eq!(padding.bottom, LengthPercentage::length(6.5));

        style.size.width = Length::Auto;
        style.padding.top = Pixels(6.75).into();
        style.padding.bottom = Pixels(6.75).into();
        let padding = style.to_taffy(Pixels(16.0), 2.0).padding;
        assert_eq!(padding.top, LengthPercentage::length(13.5));
        assert_eq!(padding.bottom, LengthPercentage::length(13.5));
    }

    #[test]
    fn border_widths_to_taffy_use_stroke_snapping() {
        let border_widths = Edges {
            top: Pixels(0.0).into(),
            right: Pixels(0.4).into(),
            bottom: Pixels(0.5).into(),
            left: Pixels(1.6).into(),
        };
        let taffy_border = border_widths_to_taffy(&border_widths, Pixels(16.0), 1.0);

        assert_eq!(
            taffy_border.top,
            taffy::style::LengthPercentage::length(0.0)
        );
        assert_eq!(
            taffy_border.right,
            taffy::style::LengthPercentage::length(1.0)
        );
        assert_eq!(
            taffy_border.bottom,
            taffy::style::LengthPercentage::length(1.0)
        );
        assert_eq!(
            taffy_border.left,
            taffy::style::LengthPercentage::length(2.0)
        );
    }
}
