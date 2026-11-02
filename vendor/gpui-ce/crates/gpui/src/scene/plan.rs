use super::{AtlasTextureId, BatchIterator, FilterBoundary, Scene};
use smallvec::SmallVec;
use std::ops::Range;

/// Inline capacity of renderer isolation stacks. Deeper groups allocate additional targets.
pub const MAX_FILTER_GROUP_DEPTH: usize = 2;

/// Index of an offscreen texture reserved for an isolated content-filter group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FilterTargetIndex(usize);

impl FilterTargetIndex {
    /// Returns the index into the renderer's content-filter target pool.
    pub fn as_usize(self) -> usize {
        self.0
    }
}

/// Where the contents of a filter group are rendered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterRenderTarget {
    /// Render an unmatched group directly into the current target.
    Inline,
    /// Render into a dedicated offscreen target and composite it into the parent.
    Isolated(FilterTargetIndex),
}

impl FilterRenderTarget {
    fn is_isolated(self) -> bool {
        matches!(self, Self::Isolated(_))
    }
}

/// Resource totals computed while a scene's render plan is compiled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[expect(missing_docs)]
pub struct ScenePlanRequirements {
    pub command_count: usize,
    pub instance_batch_count: usize,
    pub path_rasterization_vertex_count: usize,
    pub path_sprite_count: usize,
    pub surface_count: usize,
    pub backdrop_filter_count: usize,
    pub isolated_filter_count: usize,
    pub isolated_target_count: usize,
    pub uses_path_target: bool,
    pub uses_offscreen_target: bool,
    pub uses_blur_target: bool,
}

/// A compiled scene command stream, built by [`Scene::finish`] and shared by every renderer.
#[derive(Debug, Default)]
pub struct ScenePlan {
    // Retain this allocation across `Scene::clear`/`Scene::finish`; scenes are rebuilt every frame.
    pub(super) commands: Vec<RenderCommand>,
    requirements: ScenePlanRequirements,
    scene_lengths: SceneLengths,
}

impl ScenePlan {
    pub(super) fn clear(&mut self) {
        self.commands.clear();
        self.requirements = ScenePlanRequirements::default();
        self.scene_lengths = SceneLengths::default();
    }

    pub(super) fn build(scene: &Scene, mut commands: Vec<RenderCommand>) -> Self {
        commands.clear();
        commands.reserve(scene.len());
        let mut matched_starts =
            SmallVec::<[bool; 8]>::from_elem(false, scene.filter_boundaries.len());
        let mut pending_starts = SmallVec::<[usize; 4]>::new();
        for (index, boundary) in scene.filter_boundaries.iter().enumerate() {
            if boundary.is_start {
                pending_starts.push(index);
            } else if let Some(start_index) = pending_starts.pop() {
                matched_starts[start_index] = true;
            }
        }

        let mut filter_stack = SmallVec::<[(usize, FilterRenderTarget); 4]>::new();
        let mut isolated_depth = 0;
        let mut requirements = ScenePlanRequirements::default();

        for batch in BatchIterator::new(scene) {
            match batch {
                PrimitiveBatch::FilterBoundary(boundary_index) => {
                    let boundary = &scene.filter_boundaries[boundary_index];
                    if boundary.is_start {
                        let target = if matched_starts[boundary_index] {
                            FilterRenderTarget::Isolated(FilterTargetIndex(isolated_depth))
                        } else {
                            FilterRenderTarget::Inline
                        };
                        if target.is_isolated() {
                            isolated_depth += 1;
                            requirements.uses_offscreen_target = true;
                            requirements.uses_blur_target |= boundary.max_blur_radius() > 0.0;
                            requirements.isolated_target_count =
                                requirements.isolated_target_count.max(isolated_depth);
                        }
                        filter_stack.push((boundary_index, target));
                        commands.push(RenderCommand::BeginFilter {
                            boundary_index,
                            target,
                        });
                    } else if let Some((start_index, target)) = filter_stack.pop() {
                        if target.is_isolated() {
                            isolated_depth -= 1;
                            requirements.isolated_filter_count += 1;
                        }
                        commands.push(RenderCommand::EndFilter {
                            boundary_index: start_index,
                            closing_boundary_index: boundary_index,
                            target,
                        });
                    } else {
                        debug_assert!(false, "content-filter end boundary has no matching start");
                        commands.push(RenderCommand::EndFilter {
                            boundary_index,
                            closing_boundary_index: boundary_index,
                            target: FilterRenderTarget::Inline,
                        });
                    }
                }
                batch => {
                    if let PrimitiveBatch::BackdropFilters(range) = &batch {
                        requirements.uses_blur_target |= scene.backdrop_filters[range.clone()]
                            .iter()
                            .any(|filter| filter.max_blur_radius() > 0.0);
                    }

                    requirements.include_batch(&batch);
                    commands.push(RenderCommand::Batch(batch));
                }
            }
        }

        requirements.command_count = commands.len();
        Self {
            commands,
            requirements,
            scene_lengths: SceneLengths::for_scene(scene),
        }
    }

    /// Returns the ordered rendering commands.
    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    /// Returns the resource totals collected while compiling this plan.
    pub fn requirements(&self) -> &ScenePlanRequirements {
        &self.requirements
    }

    pub(super) fn assert_matches(&self, scene: &Scene) {
        debug_assert_eq!(
            self.scene_lengths,
            SceneLengths::for_scene(scene),
            "scene primitive storage changed after Scene::finish"
        );
    }
}

impl ScenePlanRequirements {
    fn include_batch(&mut self, batch: &PrimitiveBatch) {
        match batch {
            PrimitiveBatch::Shadows { range, .. }
            | PrimitiveBatch::Quads { range, .. }
            | PrimitiveBatch::Underlines(range) => {
                self.instance_batch_count += usize::from(!range.is_empty());
            }
            PrimitiveBatch::Paths {
                rasterization_vertex_count,
                sprite_count,
                ..
            } => {
                self.path_rasterization_vertex_count += rasterization_vertex_count;
                if *rasterization_vertex_count > 0 {
                    self.path_sprite_count += sprite_count;
                    self.uses_path_target = true;
                    self.instance_batch_count += 2;
                }
            }
            PrimitiveBatch::MonochromeSprites { range, .. }
            | PrimitiveBatch::SubpixelSprites { range, .. }
            | PrimitiveBatch::PolychromeSprites { range, .. } => {
                self.instance_batch_count += usize::from(!range.is_empty());
            }
            PrimitiveBatch::Surfaces(range) => self.surface_count += range.len(),
            PrimitiveBatch::BackdropFilters(range) => {
                self.backdrop_filter_count += range.len();
                self.uses_offscreen_target |= !range.is_empty();
            }
            PrimitiveBatch::FilterBoundary(_) => {
                unreachable!("filter boundaries are compiled before requirements are collected")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SceneLengths {
    shadows: usize,
    quads: usize,
    paths: usize,
    underlines: usize,
    monochrome_sprites: usize,
    subpixel_sprites: usize,
    polychrome_sprites: usize,
    surfaces: usize,
    backdrop_filters: usize,
    filter_boundaries: usize,
}

impl SceneLengths {
    fn for_scene(scene: &Scene) -> Self {
        Self {
            shadows: scene.shadows.len(),
            quads: scene.quads.len(),
            paths: scene.paths.len(),
            underlines: scene.underlines.len(),
            monochrome_sprites: scene.monochrome_sprites.len(),
            subpixel_sprites: scene.subpixel_sprites.len(),
            polychrome_sprites: scene.polychrome_sprites.len(),
            surfaces: scene.surfaces.len(),
            backdrop_filters: scene.backdrop_filters.len(),
            filter_boundaries: scene.filter_boundaries.len(),
        }
    }
}

/// A contiguous range of one primitive type drawn by a single pipeline invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum PrimitiveBatch {
    Shadows {
        range: Range<usize>,
        smoothed: bool,
    },
    Quads {
        range: Range<usize>,
        smoothed: bool,
    },
    Paths {
        range: Range<usize>,
        rasterization_vertex_count: usize,
        sprite_count: usize,
    },
    Underlines(Range<usize>),
    MonochromeSprites {
        texture_id: AtlasTextureId,
        range: Range<usize>,
    },
    SubpixelSprites {
        texture_id: AtlasTextureId,
        range: Range<usize>,
    },
    PolychromeSprites {
        texture_id: AtlasTextureId,
        range: Range<usize>,
        smoothed: bool,
    },
    Surfaces(Range<usize>),
    BackdropFilters(Range<usize>),
    FilterBoundary(usize),
}

/// Backend-neutral rendering work derived from a [`Scene`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderCommand {
    /// A normal primitive batch.
    Batch(PrimitiveBatch),
    /// Begin rendering a content-filter group.
    BeginFilter {
        /// Index of the opening boundary in [`Scene::filter_boundaries`].
        boundary_index: usize,
        /// Whether this group renders inline or into an isolated target.
        target: FilterRenderTarget,
    },
    /// Finish and composite a content-filter group.
    EndFilter {
        /// Index of the matched opening boundary in [`Scene::filter_boundaries`].
        boundary_index: usize,
        /// Index of the closing marker in [`Scene::filter_boundaries`].
        closing_boundary_index: usize,
        /// The same target selected by the matching begin command.
        target: FilterRenderTarget,
    },
}

impl RenderCommand {
    /// Returns the opening filter boundary carried by a filter command.
    pub fn boundary<'a>(&self, scene: &'a Scene) -> Option<&'a FilterBoundary> {
        match self {
            Self::BeginFilter { boundary_index, .. } | Self::EndFilter { boundary_index, .. } => {
                Some(&scene.filter_boundaries[*boundary_index])
            }
            Self::Batch(_) => None,
        }
    }

    /// A diagnostic label suitable for GPU debug annotations.
    pub fn label(&self) -> String {
        match self {
            Self::Batch(batch) => batch.label(),
            Self::BeginFilter { target, .. } => match target {
                FilterRenderTarget::Isolated(index) => {
                    format!("begin filter group ({})", index.as_usize())
                }
                FilterRenderTarget::Inline => "begin inline filter group".into(),
            },
            Self::EndFilter { target, .. } => match target {
                FilterRenderTarget::Isolated(index) => {
                    format!("end filter group ({})", index.as_usize())
                }
                FilterRenderTarget::Inline => "end inline filter group".into(),
            },
        }
    }
}

impl PrimitiveBatch {
    /// A diagnostic label suitable for GPU debug annotations.
    pub fn label(&self) -> String {
        match self {
            Self::Shadows { range, smoothed } => format!(
                "{}shadows ({})",
                if *smoothed { "smoothed " } else { "" },
                range.len()
            ),
            Self::Quads { range, smoothed } => format!(
                "{}quads ({})",
                if *smoothed { "smoothed " } else { "" },
                range.len()
            ),
            Self::Paths { range, .. } => format!("paths ({})", range.len()),
            Self::Underlines(range) => format!("underlines ({})", range.len()),
            Self::MonochromeSprites { texture_id, range } => format!(
                "monochrome sprites ({}) on atlas {}",
                range.len(),
                texture_id.index
            ),
            Self::SubpixelSprites { texture_id, range } => format!(
                "subpixel sprites ({}) on atlas {}",
                range.len(),
                texture_id.index
            ),
            Self::PolychromeSprites {
                texture_id,
                range,
                smoothed,
            } => format!(
                "{}polychrome sprites ({}) on atlas {}",
                if *smoothed { "smoothed " } else { "" },
                range.len(),
                texture_id.index
            ),
            Self::Surfaces(range) => format!("surfaces ({})", range.len()),
            Self::BackdropFilters(range) => format!("backdrop filters ({})", range.len()),
            Self::FilterBoundary(index) => format!("filter boundary ({index})"),
        }
    }
}
