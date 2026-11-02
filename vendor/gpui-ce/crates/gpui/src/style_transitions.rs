use scheduler::Instant;

use crate::{
    AbsoluteLength, Animated, AnimatedSample, AnyMotion, Background, Bounds, DefiniteLength, Fill,
    Hsla, Length, Lerp, Pixels, Progress, RingColor,
};

#[derive(Clone, Copy)]
pub(crate) struct StyleTransitionContext {
    pub(crate) bounds: Option<Bounds<Pixels>>,
    pub(crate) containing_bounds: Option<Bounds<Pixels>>,
    pub(crate) rem_size: Pixels,
}

impl StyleTransitionContext {
    pub(crate) fn new(bounds: Option<Bounds<Pixels>>, rem_size: Pixels) -> Self {
        Self {
            bounds,
            containing_bounds: None,
            rem_size,
        }
    }

    pub(crate) fn with_containing_bounds(
        mut self,
        containing_bounds: Option<Bounds<Pixels>>,
    ) -> Self {
        self.containing_bounds = containing_bounds;
        self
    }
}

pub(crate) struct StyleTransitionPropertyState<T: Lerp + Clone + PartialEq> {
    animated: Option<Animated<T, Instant>>,
}

struct StyleTransitionEvaluation<T> {
    value: Option<T>,
    progress: Progress,
    is_active: bool,
}

impl<T> StyleTransitionEvaluation<T> {
    fn from_sample(sample: AnimatedSample<T>) -> Self {
        Self {
            value: Some(sample.value),
            progress: sample.progress,
            is_active: sample.is_active,
        }
    }

    fn at_target(value: Option<T>) -> Self {
        Self {
            value,
            progress: Progress::END,
            is_active: false,
        }
    }

    /// An inactive sample at Progress::END presents the authored target.
    /// Other inactive progress retains the sampled presentation, such as an
    /// alternating run that settles at its origin or a custom easing endpoint.
    fn presents_authored_target(&self) -> bool {
        !self.is_active && self.progress == Progress::END
    }
}

struct SizeTransitionState<T: Lerp + Clone + PartialEq> {
    width: Option<StyleTransitionPropertyState<T>>,
    height: Option<StyleTransitionPropertyState<T>>,
}

#[derive(Clone, Copy)]
pub(crate) enum StyleTransitionAxis {
    Width,
    Height,
}

#[derive(Clone, Copy)]
pub(crate) enum StyleTransitionEdge {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Default)]
struct AutoSizeTransitionPropertyState {
    authored_goal: Option<Length>,
    resolved_auto: Option<Pixels>,
    pending_auto_capture: bool,
    animated: Option<Animated<Pixels, Instant>>,
}

#[derive(Default)]
struct AutoSizeTransitionState {
    width: Option<AutoSizeTransitionPropertyState>,
    height: Option<AutoSizeTransitionPropertyState>,
}

#[derive(Default)]
struct InsetTransitionPropertyState {
    authored_goal: Option<Length>,
    resolved_auto: Option<Pixels>,
    containing_size: Option<Pixels>,
    active: bool,
    animated: Option<Animated<Pixels, Instant>>,
}

#[derive(Default)]
struct InsetTransitionState {
    top: Option<InsetTransitionPropertyState>,
    right: Option<InsetTransitionPropertyState>,
    bottom: Option<InsetTransitionPropertyState>,
    left: Option<InsetTransitionPropertyState>,
}

impl<T: Lerp + Clone + PartialEq> Default for SizeTransitionState<T> {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
        }
    }
}

struct EdgesTransitionState<T: Lerp + Clone + PartialEq> {
    top: Option<StyleTransitionPropertyState<T>>,
    right: Option<StyleTransitionPropertyState<T>>,
    bottom: Option<StyleTransitionPropertyState<T>>,
    left: Option<StyleTransitionPropertyState<T>>,
}

impl<T: Lerp + Clone + PartialEq> Default for EdgesTransitionState<T> {
    fn default() -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: None,
        }
    }
}

struct CornersTransitionState<T: Lerp + Clone + PartialEq> {
    top_left: Option<StyleTransitionPropertyState<T>>,
    top_right: Option<StyleTransitionPropertyState<T>>,
    bottom_right: Option<StyleTransitionPropertyState<T>>,
    bottom_left: Option<StyleTransitionPropertyState<T>>,
}

impl<T: Lerp + Clone + PartialEq> Default for CornersTransitionState<T> {
    fn default() -> Self {
        Self {
            top_left: None,
            top_right: None,
            bottom_right: None,
            bottom_left: None,
        }
    }
}

#[derive(Default)]
struct TextStyleTransitionState {
    color: Option<StyleTransitionPropertyState<Hsla>>,
    background_color: Option<StyleTransitionPropertyState<Hsla>>,
    font_size: Option<StyleTransitionPropertyState<AbsoluteLength>>,
    line_height: Option<StyleTransitionPropertyState<DefiniteLength>>,
    letter_spacing: Option<StyleTransitionPropertyState<Pixels>>,
    line_clamp: Option<StyleTransitionPropertyState<usize>>,
}

#[derive(Default)]
struct RingStyleTransitionState {
    width: Option<StyleTransitionPropertyState<Pixels>>,
    color: Option<StyleTransitionPropertyState<RingColor>>,
}

#[derive(Default)]
pub(crate) struct StyleTransitionState {
    inset: InsetTransitionState,
    size: AutoSizeTransitionState,
    min_size: SizeTransitionState<Length>,
    max_size: SizeTransitionState<Length>,
    margin: EdgesTransitionState<Length>,
    padding: EdgesTransitionState<DefiniteLength>,
    border_widths: EdgesTransitionState<AbsoluteLength>,
    gap: SizeTransitionState<DefiniteLength>,
    corner_radii: CornersTransitionState<AbsoluteLength>,
    corner_smoothing: Option<StyleTransitionPropertyState<f32>>,
    scrollbar_width: Option<StyleTransitionPropertyState<AbsoluteLength>>,
    aspect_ratio: Option<StyleTransitionPropertyState<f32>>,
    flex_basis: Option<StyleTransitionPropertyState<Length>>,
    flex_grow: Option<StyleTransitionPropertyState<f32>>,
    flex_shrink: Option<StyleTransitionPropertyState<f32>>,
    background: Option<StyleTransitionPropertyState<Fill>>,
    border_color: Option<StyleTransitionPropertyState<Background>>,
    ring: RingStyleTransitionState,
    inset_ring: RingStyleTransitionState,
    text: TextStyleTransitionState,
    opacity: Option<StyleTransitionPropertyState<f32>>,
}

fn apply_auto_size(
    state: &mut Option<AutoSizeTransitionPropertyState>,
    value: &mut Length,
    axis: StyleTransitionAxis,
    motion: Option<&AnyMotion>,
    context: StyleTransitionContext,
    now: Instant,
    reduce_motion: bool,
) -> bool {
    let Some(motion) = motion else {
        *state = None;
        return false;
    };

    let authored_goal = *value;
    let state = state.get_or_insert_with(Default::default);
    let bounds_value = context.bounds.map(|bounds| match axis {
        StyleTransitionAxis::Width => bounds.size.width,
        StyleTransitionAxis::Height => bounds.size.height,
    });
    let endpoint = match authored_goal {
        Length::Definite(DefiniteLength::Absolute(length)) => {
            Some(length.to_pixels(context.rem_size))
        }
        Length::Definite(DefiniteLength::Relative(_)) => None,
        Length::Auto => state.resolved_auto,
    };

    let Some(endpoint) = endpoint else {
        if authored_goal == Length::Auto {
            if let Some(bounds_value) = bounds_value {
                state.resolved_auto = Some(bounds_value);
                state.pending_auto_capture = false;
                match state.animated.as_mut() {
                    Some(animated) => animated.jump_to(bounds_value),
                    None => state.animated = Some(Animated::new(bounds_value, motion.clone())),
                }
            } else {
                state.pending_auto_capture = true;
            }
        } else {
            state.animated = None;
            state.pending_auto_capture = false;
        }
        state.authored_goal = Some(authored_goal);
        return false;
    };

    if state.pending_auto_capture {
        if let Some(bounds_value) = bounds_value {
            state.resolved_auto = Some(bounds_value);
            state.pending_auto_capture = false;
            match state.animated.as_mut() {
                Some(animated) => animated.jump_to(bounds_value),
                None => state.animated = Some(Animated::new(bounds_value, motion.clone())),
            }
        }
        state.authored_goal = Some(authored_goal);
        return false;
    }

    let animated = state
        .animated
        .get_or_insert_with(|| Animated::new(endpoint, motion.clone()));

    if reduce_motion {
        animated.jump_to(endpoint);
    } else if state.authored_goal != Some(authored_goal) || animated.value() != &endpoint {
        animated.set(endpoint, motion, now);
    }

    let evaluation = StyleTransitionEvaluation::from_sample(animated.sample(now));
    state.authored_goal = Some(authored_goal);

    if !evaluation.presents_authored_target() {
        let sampled_value = evaluation
            .value
            .expect("an Animated sample always contains a value");
        *value = Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(
            sampled_value,
        )));
    } else if authored_goal == Length::Auto
        && let Some(bounds_value) = bounds_value
    {
        state.resolved_auto = Some(bounds_value);
        animated.jump_to(bounds_value);
    } else {
        animated.adopt_completed_target();
    }

    evaluation.is_active
}

fn evaluate<T>(
    state: &mut Option<StyleTransitionPropertyState<T>>,
    target: Option<T>,
    motion: &AnyMotion,
    now: Instant,
    reduce_motion: bool,
) -> StyleTransitionEvaluation<T>
where
    T: Lerp + Clone + PartialEq,
{
    state
        .get_or_insert_with(|| StyleTransitionPropertyState::new(target.clone(), motion))
        .evaluate(target, motion, now, reduce_motion)
}

fn apply_required<T>(
    state: &mut Option<StyleTransitionPropertyState<T>>,
    value: &mut T,
    motion: Option<&AnyMotion>,
    now: Instant,
    reduce_motion: bool,
) -> bool
where
    T: Lerp + Clone + PartialEq,
{
    let target = value.clone();
    apply_required_target(state, value, Some(target), motion, now, reduce_motion)
}

fn apply_required_target<T>(
    state: &mut Option<StyleTransitionPropertyState<T>>,
    value: &mut T,
    target: Option<T>,
    motion: Option<&AnyMotion>,
    now: Instant,
    reduce_motion: bool,
) -> bool
where
    T: Lerp + Clone + PartialEq,
{
    let Some(motion) = motion else {
        *state = None;
        return false;
    };
    let Some(target) = target else {
        return false;
    };

    let evaluation = evaluate(state, Some(target), motion, now, reduce_motion);
    if !evaluation.presents_authored_target()
        && let Some(evaluated_value) = evaluation.value
    {
        *value = evaluated_value;
    }
    evaluation.is_active
}

fn apply_optional<T>(
    state: &mut Option<StyleTransitionPropertyState<T>>,
    value: &mut Option<T>,
    motion: Option<&AnyMotion>,
    now: Instant,
    reduce_motion: bool,
) -> bool
where
    T: Lerp + Clone + Default + PartialEq,
{
    let Some(motion) = motion else {
        *state = None;
        return false;
    };

    let target = value.clone().unwrap_or_default();
    let evaluation = evaluate(state, Some(target), motion, now, reduce_motion);
    if !evaluation.presents_authored_target() {
        *value = evaluation.value;
    }
    evaluation.is_active
}

fn apply_inset(
    state: &mut Option<InsetTransitionPropertyState>,
    value: &mut Length,
    edge: StyleTransitionEdge,
    motion: Option<&AnyMotion>,
    context: StyleTransitionContext,
    now: Instant,
    reduce_motion: bool,
) -> bool {
    let Some(motion) = motion else {
        *state = None;
        return false;
    };

    let authored_goal = *value;
    let state = state.get_or_insert_with(Default::default);

    if let (Some(bounds), Some(containing_bounds)) = (context.bounds, context.containing_bounds) {
        state.containing_size = Some(match edge {
            StyleTransitionEdge::Top | StyleTransitionEdge::Bottom => containing_bounds.size.height,
            StyleTransitionEdge::Right | StyleTransitionEdge::Left => containing_bounds.size.width,
        });

        if authored_goal == Length::Auto && !state.active {
            let resolved_auto = match edge {
                StyleTransitionEdge::Top => bounds.origin.y - containing_bounds.origin.y,
                StyleTransitionEdge::Right => {
                    containing_bounds.bottom_right().x - bounds.bottom_right().x
                }
                StyleTransitionEdge::Bottom => {
                    containing_bounds.bottom_right().y - bounds.bottom_right().y
                }
                StyleTransitionEdge::Left => bounds.origin.x - containing_bounds.origin.x,
            };
            state.resolved_auto = Some(resolved_auto);
            if let Some(animated) = state.animated.as_mut() {
                animated.jump_to(resolved_auto);
            }
        }
    }

    let endpoint = match authored_goal {
        Length::Auto => state.resolved_auto,
        Length::Definite(length) => match length {
            DefiniteLength::Absolute(length) => Some(length.to_pixels(context.rem_size)),
            DefiniteLength::Relative(_) => state.containing_size.map(|containing_size| {
                length.to_pixels(AbsoluteLength::Pixels(containing_size), context.rem_size)
            }),
        },
    };

    let Some(endpoint) = endpoint else {
        state.authored_goal = Some(authored_goal);
        state.active = false;
        state.animated = None;
        return false;
    };

    let previous_goal = state.authored_goal;
    let animated = state.animated.get_or_insert_with(|| {
        Animated::new(
            if previous_goal == Some(Length::Auto) {
                state.resolved_auto.unwrap_or(endpoint)
            } else {
                endpoint
            },
            motion.clone(),
        )
    });

    if previous_goal == Some(Length::Auto) && authored_goal != Length::Auto {
        animated.jump_to(state.resolved_auto.unwrap_or(endpoint));
    }

    if reduce_motion {
        animated.jump_to(endpoint);
    } else if previous_goal != Some(authored_goal) || animated.value() != &endpoint {
        animated.set(endpoint, motion, now);
    }

    let evaluation = StyleTransitionEvaluation::from_sample(animated.sample(now));
    state.authored_goal = Some(authored_goal);
    state.active = evaluation.is_active;

    if !evaluation.presents_authored_target() {
        let sampled_value = evaluation
            .value
            .expect("an Animated sample always contains a value");
        *value = Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(
            sampled_value,
        )));
    } else {
        animated.adopt_completed_target();
    }

    evaluation.is_active
}

impl<T> StyleTransitionPropertyState<T>
where
    T: Lerp + Clone + PartialEq,
{
    fn new(initial_goal: Option<T>, motion: &AnyMotion) -> Self {
        Self {
            animated: initial_goal.map(|goal| Animated::new(goal, motion.clone())),
        }
    }

    fn jump_to(&mut self, goal: Option<T>, motion: &AnyMotion) {
        match (self.animated.as_mut(), goal) {
            (Some(animated), Some(goal)) => animated.jump_to(goal),
            (_, Some(goal)) => {
                self.animated = Some(Animated::new(goal, motion.clone()));
            }
            (_, None) => self.animated = None,
        }
    }

    fn evaluate(
        &mut self,
        goal: Option<T>,
        motion: &AnyMotion,
        now: Instant,
        reduce_motion: bool,
    ) -> StyleTransitionEvaluation<T> {
        if reduce_motion {
            self.jump_to(goal, motion);
            return StyleTransitionEvaluation::at_target(
                self.animated
                    .as_ref()
                    .map(|animated| animated.value().clone()),
            );
        }

        let Some(goal) = goal else {
            self.animated = None;
            return StyleTransitionEvaluation::at_target(None);
        };

        let Some(animated) = self.animated.as_mut() else {
            self.animated = Some(Animated::new(goal.clone(), motion.clone()));
            return StyleTransitionEvaluation::at_target(Some(goal));
        };

        animated.set(goal, motion, now);
        let evaluation = StyleTransitionEvaluation::from_sample(animated.sample(now));
        if evaluation.presents_authored_target() {
            animated.adopt_completed_target();
        }
        evaluation
    }
}

gpui_macros::style_transitions!();

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use super::*;
    use crate::{
        AbsoluteLength, AnyWindowHandle, Bounds, Corners, DefiniteLength, Edges, FocusHandle,
        InputEvent as _, Length, Motion, MotionDurationExt, MouseButton, MouseDownEvent,
        MouseUpEvent, Pixels, SpringConfig, Style, TestAppContext, Window, blue, canvas, div,
        ease_in_out, point, prelude::*, px, red, relative, rems, size,
    };

    #[derive(Clone, Debug, Default, PartialEq)]
    struct EndpointBiased(f32);

    impl Lerp for EndpointBiased {
        fn lerp(&self, to: &Self, delta: f32) -> Self {
            let interpolated = self.0 + (to.0 - self.0) * delta;
            Self(if delta == 1.0 {
                interpolated + 0.25
            } else {
                interpolated
            })
        }
    }

    fn length(value: f32) -> Length {
        Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(value))))
    }

    fn corners(value: f32) -> Corners<AbsoluteLength> {
        let value = AbsoluteLength::Pixels(px(value));
        Corners {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }

    fn size_transition_after(transitions: StyleTransitions, elapsed: Duration) -> (bool, Style) {
        let started_at = Instant::now();
        let mut state = StyleTransitionState::default();
        let mut style = Style {
            size: size(length(10.0), length(10.0)),
            ..Style::default()
        };

        let context = StyleTransitionContext::new(None, px(16.0));
        assert!(!transitions.apply(&mut style, &mut state, context, started_at, false));

        style.size = size(length(20.0), length(20.0));
        assert!(transitions.apply(&mut style, &mut state, context, started_at, false));

        style.size = size(length(20.0), length(20.0));
        let in_progress =
            transitions.apply(&mut style, &mut state, context, started_at + elapsed, false);

        (in_progress, style)
    }

    #[test]
    fn alternating_style_motion_keeps_its_origin_after_completion() {
        let started_at = Instant::now();
        let motion = Motion::new(Duration::from_secs(1))
            .with_delay(Duration::from_millis(500))
            .iterations(2)
            .alternate();
        let transitions = StyleTransitions::new()
            .flex_grow(motion.clone())
            .opacity(motion);
        let context = StyleTransitionContext::new(None, px(16.0));
        let mut state = StyleTransitionState::default();
        let mut style = Style {
            flex_grow: 4.0,
            opacity: Some(1.0),
            ..Style::default()
        };

        assert!(!transitions.apply(&mut style, &mut state, context, started_at, false));

        for (milliseconds, expected_grow, expected_opacity, active) in [
            (0, 4.0, Some(1.0), true),
            (500, 4.0, Some(1.0), true),
            (1_000, 7.0, Some(0.5), true),
            (1_500, 10.0, Some(0.0), true),
            (2_000, 7.0, Some(0.5), true),
            (2_500, 4.0, Some(1.0), false),
            (3_000, 4.0, Some(1.0), false),
        ] {
            let mut style = Style {
                flex_grow: 10.0,
                opacity: None,
                ..Style::default()
            };
            assert_eq!(
                transitions.apply(
                    &mut style,
                    &mut state,
                    context,
                    started_at + Duration::from_millis(milliseconds),
                    false,
                ),
                active
            );
            assert_eq!(style.flex_grow, expected_grow);
            assert_eq!(style.opacity, expected_opacity);
        }

        let mut style = Style {
            flex_grow: 20.0,
            opacity: None,
            ..Style::default()
        };
        assert!(!transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + Duration::from_millis(3_100),
            true,
        ));
        assert_eq!((style.flex_grow, style.opacity), (20.0, None));

        assert!(!transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + Duration::from_millis(3_200),
            false,
        ));
        assert_eq!((style.flex_grow, style.opacity), (20.0, None));
    }

    #[test]
    fn required_completion_uses_motion_progress_not_value_equality() {
        let started_at = Instant::now();
        let motion: AnyMotion = Motion::new(Duration::from_secs(1)).into();
        let mut state = None;
        let mut value = EndpointBiased(10.0);

        assert!(!apply_required(
            &mut state,
            &mut value,
            Some(&motion),
            started_at,
            false,
        ));

        value = EndpointBiased(20.0);
        assert!(apply_required(
            &mut state,
            &mut value,
            Some(&motion),
            started_at,
            false,
        ));
        assert_eq!(value, EndpointBiased(10.0));

        value = EndpointBiased(20.0);
        assert!(apply_required(
            &mut state,
            &mut value,
            Some(&motion),
            started_at + Duration::from_millis(500),
            false,
        ));
        assert_eq!(value, EndpointBiased(15.0));

        value = EndpointBiased(20.0);
        assert!(!apply_required(
            &mut state,
            &mut value,
            Some(&motion),
            started_at + Duration::from_secs(1),
            false,
        ));
        assert_eq!(value, EndpointBiased(20.0));

        value = EndpointBiased(30.0);
        assert!(apply_required(
            &mut state,
            &mut value,
            Some(&motion),
            started_at + Duration::from_secs(2),
            false,
        ));
        assert_eq!(value, EndpointBiased(20.0));

        value = EndpointBiased(30.0);
        assert!(apply_required(
            &mut state,
            &mut value,
            Some(&motion),
            started_at + Duration::from_millis(2_500),
            false,
        ));
        assert_eq!(value, EndpointBiased(25.0));

        let nonstandard_endpoint: AnyMotion = Motion::new(Duration::from_secs(1))
            .with_easing(|_| 0.5)
            .into();
        let mut state = None;
        let mut value = 10.0_f32;
        assert!(!apply_required(
            &mut state,
            &mut value,
            Some(&nonstandard_endpoint),
            started_at,
            false,
        ));

        value = 20.0;
        assert!(apply_required(
            &mut state,
            &mut value,
            Some(&nonstandard_endpoint),
            started_at,
            false,
        ));
        assert_eq!(value, 15.0);

        value = 20.0;
        assert!(!apply_required(
            &mut state,
            &mut value,
            Some(&nonstandard_endpoint),
            started_at + Duration::from_secs(1),
            false,
        ));
        assert_eq!(value, 15.0);
    }

    #[test]
    fn optional_completion_uses_motion_progress_not_value_equality() {
        let started_at = Instant::now();
        let motion: AnyMotion = Motion::new(Duration::from_secs(1)).into();
        let mut state = None;
        let mut value = Some(EndpointBiased(10.0));

        assert!(!apply_optional(
            &mut state,
            &mut value,
            Some(&motion),
            started_at,
            false,
        ));

        value = None;
        assert!(apply_optional(
            &mut state,
            &mut value,
            Some(&motion),
            started_at,
            false,
        ));
        assert_eq!(value, Some(EndpointBiased(10.0)));

        value = None;
        assert!(!apply_optional(
            &mut state,
            &mut value,
            Some(&motion),
            started_at + Duration::from_secs(1),
            false,
        ));
        assert_eq!(value, None);

        value = None;
        assert!(!apply_optional(
            &mut state,
            &mut value,
            Some(&motion),
            started_at + Duration::from_secs(2),
            false,
        ));
        assert_eq!(value, None);

        let nonstandard_endpoint: AnyMotion = Motion::new(Duration::from_secs(1))
            .with_easing(|_| 0.5)
            .into();
        let mut state = None;
        let mut value = Some(10.0_f32);
        assert!(!apply_optional(
            &mut state,
            &mut value,
            Some(&nonstandard_endpoint),
            started_at,
            false,
        ));

        value = None;
        assert!(apply_optional(
            &mut state,
            &mut value,
            Some(&nonstandard_endpoint),
            started_at,
            false,
        ));
        assert_eq!(value, Some(5.0));

        value = None;
        assert!(!apply_optional(
            &mut state,
            &mut value,
            Some(&nonstandard_endpoint),
            started_at + Duration::from_secs(1),
            false,
        ));
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn optional_some_completion_and_retarget_use_the_authored_endpoint() {
        let started_at = Instant::now();
        let motion: AnyMotion = Motion::new(Duration::from_secs(1)).into();
        let mut state = None;
        let mut value = Some(EndpointBiased(10.0));

        assert!(!apply_optional(
            &mut state,
            &mut value,
            Some(&motion),
            started_at,
            false,
        ));

        value = Some(EndpointBiased(20.0));
        assert!(apply_optional(
            &mut state,
            &mut value,
            Some(&motion),
            started_at,
            false,
        ));
        assert_eq!(value, Some(EndpointBiased(10.0)));

        value = Some(EndpointBiased(20.0));
        assert!(!apply_optional(
            &mut state,
            &mut value,
            Some(&motion),
            started_at + Duration::from_secs(1),
            false,
        ));
        assert_eq!(value, Some(EndpointBiased(20.0)));

        value = Some(EndpointBiased(30.0));
        assert!(apply_optional(
            &mut state,
            &mut value,
            Some(&motion),
            started_at + Duration::from_secs(2),
            false,
        ));
        assert_eq!(value, Some(EndpointBiased(20.0)));
    }

    #[test]
    fn alternating_auto_size_and_inset_keep_resolved_origins() {
        let started_at = Instant::now();
        let motion = Motion::new(Duration::from_secs(1))
            .iterations(2)
            .alternate();
        let transitions = StyleTransitions::new().w(motion.clone()).top(motion);
        let layout_context = StyleTransitionContext::new(None, px(16.0));
        let prepaint_context = StyleTransitionContext::new(
            Some(Bounds {
                origin: point(px(0.0), px(15.0)),
                size: size(px(100.0), px(40.0)),
            }),
            px(16.0),
        )
        .with_containing_bounds(Some(Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(300.0), px(200.0)),
        }));
        let mut state = StyleTransitionState::default();
        let mut style = Style::default();

        assert!(!transitions.apply(&mut style, &mut state, layout_context, started_at, false));
        assert!(!transitions.apply(&mut style, &mut state, prepaint_context, started_at, false,));

        for (elapsed, expected_width, expected_top, active) in [
            (Duration::ZERO, 100.0, 15.0, true),
            (Duration::from_secs(1), 200.0, 5.0, true),
            (Duration::from_secs(2), 100.0, 15.0, false),
            (Duration::from_secs(3), 100.0, 15.0, false),
        ] {
            let mut style = Style::default();
            style.size.width = length(200.0);
            style.inset.top = length(5.0);
            assert_eq!(
                transitions.apply(
                    &mut style,
                    &mut state,
                    layout_context,
                    started_at + elapsed,
                    false,
                ),
                active
            );
            assert_eq!(style.size.width, length(expected_width));
            assert_eq!(style.inset.top, length(expected_top));
        }

        let mut style = Style::default();
        style.size.width = length(240.0);
        style.inset.top = length(8.0);
        assert!(!transitions.apply(
            &mut style,
            &mut state,
            layout_context,
            started_at + Duration::from_millis(3_100),
            true,
        ));
        assert_eq!(style.size.width, length(240.0));
        assert_eq!(style.inset.top, length(8.0));
    }

    #[test]
    fn spring_style_transitions_overshoot_retarget_and_settle() {
        let config = SpringConfig::new(100.0, 6.0, 1.0);
        let transitions = StyleTransitions::new()
            .flex_grow(config)
            .opacity(Motion::spring(config).with_epsilon(0.01));
        let context = StyleTransitionContext::new(None, px(16.0));
        let started_at = Instant::now();
        let mut state = StyleTransitionState::default();
        let mut style = Style::default();

        assert!(!transitions.apply(&mut style, &mut state, context, started_at, false));

        style.flex_grow = 10.0;
        style.opacity = Some(1.0);
        assert!(transitions.apply(&mut style, &mut state, context, started_at, false));
        assert_eq!(style.flex_grow, 0.0);

        let overshoot_at = started_at + Duration::from_millis(350);
        style.flex_grow = 10.0;
        style.opacity = Some(1.0);
        assert!(transitions.apply(&mut style, &mut state, context, overshoot_at, false));
        assert!(style.flex_grow > 10.0);
        let overshoot = style.flex_grow;

        style.flex_grow = 20.0;
        style.opacity = Some(1.0);
        assert!(transitions.apply(&mut style, &mut state, context, overshoot_at, false));
        assert_eq!(style.flex_grow, overshoot);

        style.flex_grow = 20.0;
        style.opacity = Some(1.0);
        assert!(!transitions.apply(
            &mut style,
            &mut state,
            context,
            overshoot_at + Duration::from_secs(5),
            false,
        ));
        assert_eq!(style.flex_grow, 20.0);
        assert_eq!(style.opacity, Some(1.0));
    }

    #[test]
    fn style_transitions_follow_a_complete_lifecycle() {
        let started_at = Instant::now();
        let duration = Duration::from_secs(1);
        let transitions = StyleTransitions::new()
            .flex_grow(duration)
            .opacity(duration);
        let context = StyleTransitionContext::new(None, px(16.0));
        let mut state = StyleTransitionState::default();
        let mut style = Style::default();

        assert!(!transitions.apply(&mut style, &mut state, context, started_at, false));
        assert_eq!((style.flex_grow, style.opacity), (0.0, None));

        style.flex_grow = 10.0;
        style.opacity = Some(1.0);
        assert!(transitions.apply(&mut style, &mut state, context, started_at, false));
        assert_eq!((style.flex_grow, style.opacity), (0.0, Some(0.0)));

        style.flex_grow = 10.0;
        style.opacity = Some(1.0);
        assert!(transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + duration / 2,
            false,
        ));
        assert_eq!((style.flex_grow, style.opacity), (5.0, Some(0.5)));

        style.flex_grow = 20.0;
        style.opacity = None;
        assert!(transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + duration / 2,
            false,
        ));
        assert_eq!((style.flex_grow, style.opacity), (5.0, Some(0.5)));

        style.flex_grow = 20.0;
        style.opacity = None;
        assert!(transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + duration,
            false,
        ));
        assert_eq!((style.flex_grow, style.opacity), (12.5, Some(0.25)));

        style.flex_grow = 20.0;
        style.opacity = None;
        assert!(!transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + duration + duration / 2,
            false,
        ));
        assert_eq!((style.flex_grow, style.opacity), (20.0, None));

        style.flex_grow = 30.0;
        style.opacity = Some(1.0);
        assert!(!transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + duration + duration / 2,
            true,
        ));
        assert_eq!((style.flex_grow, style.opacity), (30.0, Some(1.0)));
    }

    #[test]
    fn solid_ring_color_transitions_survive_background_storage() {
        let started_at = Instant::now();
        let duration = Duration::from_secs(1);
        let transitions = StyleTransitions::new().ring_color(duration);
        let context = StyleTransitionContext::new(None, px(16.0));
        let mut state = StyleTransitionState::default();
        let mut style = Style::default();
        style.ring.color = RingColor::Color(red().into());

        assert!(!transitions.apply(&mut style, &mut state, context, started_at, false));

        style.ring.color = RingColor::Color(blue().into());
        assert!(transitions.apply(&mut style, &mut state, context, started_at, false));

        style.ring.color = RingColor::Color(blue().into());
        assert!(transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + duration / 2,
            false,
        ));
        assert_eq!(
            style.ring.color,
            RingColor::Color(red().lerp(&blue(), 0.5).into())
        );
    }

    #[test]
    fn inset_edge_transitions_start_from_resolved_layout_position() {
        #[derive(Clone, Copy)]
        enum Edge {
            Top,
            Right,
            Bottom,
            Left,
        }

        impl Edge {
            fn transitions(self, duration: Duration) -> StyleTransitions {
                match self {
                    Self::Top => StyleTransitions::new().top(duration),
                    Self::Right => StyleTransitions::new().right(duration),
                    Self::Bottom => StyleTransitions::new().bottom(duration),
                    Self::Left => StyleTransitions::new().left(duration),
                }
            }

            fn set(self, style: &mut Style, value: Length) {
                match self {
                    Self::Top => style.inset.top = value,
                    Self::Right => style.inset.right = value,
                    Self::Bottom => style.inset.bottom = value,
                    Self::Left => style.inset.left = value,
                }
            }

            fn get(self, style: &Style) -> Length {
                match self {
                    Self::Top => style.inset.top,
                    Self::Right => style.inset.right,
                    Self::Bottom => style.inset.bottom,
                    Self::Left => style.inset.left,
                }
            }
        }

        let started_at = Instant::now();
        let duration = Duration::from_secs(1);
        let layout_context = StyleTransitionContext::new(None, px(16.0));
        let prepaint_context = StyleTransitionContext::new(
            Some(Bounds {
                origin: point(px(20.0), px(15.0)),
                size: size(px(30.0), px(20.0)),
            }),
            px(16.0),
        )
        .with_containing_bounds(Some(Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(100.0), px(80.0)),
        }));

        for (edge, resolved_auto, target, midpoint) in [
            (Edge::Top, 15.0, length(5.0), 10.0),
            (
                Edge::Right,
                50.0,
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Rems(rems(1.0)))),
                33.0,
            ),
            (Edge::Bottom, 45.0, relative(0.25).into(), 32.5),
            (Edge::Left, 20.0, length(10.0), 15.0),
        ] {
            let transitions = edge.transitions(duration);
            let mut state = StyleTransitionState::default();
            let mut style = Style::default();

            assert!(!transitions.apply(&mut style, &mut state, layout_context, started_at, false,));
            assert!(!transitions.apply(
                &mut style,
                &mut state,
                prepaint_context,
                started_at,
                false,
            ));

            edge.set(&mut style, target);
            assert!(transitions.apply(&mut style, &mut state, layout_context, started_at, false,));
            assert_eq!(edge.get(&style), length(resolved_auto));

            edge.set(&mut style, target);
            assert!(transitions.apply(
                &mut style,
                &mut state,
                layout_context,
                started_at + duration / 2,
                false,
            ));
            assert_eq!(edge.get(&style), length(midpoint));

            edge.set(&mut style, target);
            assert!(!transitions.apply(
                &mut style,
                &mut state,
                layout_context,
                started_at + duration,
                false,
            ));
            assert_eq!(edge.get(&style), target);
        }
    }

    #[test]
    fn auto_size_transitions_use_stable_prepaint_bounds() {
        let started_at = Instant::now();
        let duration = Duration::from_secs(1);
        let transitions = StyleTransitions::new().w(duration);
        let layout_context = StyleTransitionContext::new(None, px(16.0));
        let prepaint_context = |width| {
            StyleTransitionContext::new(
                Some(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(width), px(40.0)),
                }),
                px(16.0),
            )
        };
        let mut state = StyleTransitionState::default();
        let mut style = Style::default();

        assert!(!transitions.apply(&mut style, &mut state, layout_context, started_at, false,));
        assert!(!transitions.apply(
            &mut style,
            &mut state,
            prepaint_context(120.0),
            started_at,
            false,
        ));

        style.size.width = length(220.0);
        assert!(transitions.apply(&mut style, &mut state, layout_context, started_at, false,));
        assert_eq!(style.size.width, length(120.0));

        style.size.width = length(220.0);
        assert!(transitions.apply(
            &mut style,
            &mut state,
            layout_context,
            started_at + duration / 2,
            false,
        ));
        assert_eq!(style.size.width, length(170.0));

        style.size.width = length(220.0);
        assert!(!transitions.apply(
            &mut style,
            &mut state,
            layout_context,
            started_at + duration,
            false,
        ));
        assert_eq!(style.size.width, length(220.0));

        style.size.width = Length::Auto;
        assert!(transitions.apply(
            &mut style,
            &mut state,
            layout_context,
            started_at + duration,
            false,
        ));
        assert_eq!(style.size.width, length(220.0));

        style.size.width = Length::Auto;
        assert!(!transitions.apply(
            &mut style,
            &mut state,
            layout_context,
            started_at + duration * 2,
            false,
        ));
        assert_eq!(style.size.width, Length::Auto);

        assert!(!transitions.apply(
            &mut style,
            &mut state,
            prepaint_context(140.0),
            started_at + duration * 2,
            false,
        ));

        style.size.width = length(240.0);
        assert!(transitions.apply(
            &mut style,
            &mut state,
            layout_context,
            started_at + duration * 2,
            false,
        ));
        assert_eq!(style.size.width, length(140.0));
    }

    #[test]
    fn grouped_transition_builders_interpolate_selected_properties_and_respect_order() {
        let (in_progress, style) = size_transition_after(
            StyleTransitions::new().w(Duration::from_secs(1)),
            Duration::from_millis(500),
        );
        assert!(in_progress);
        assert_eq!(style.size, size(length(15.0), length(20.0)));

        let (in_progress, style) = size_transition_after(
            StyleTransitions::new()
                .size(Duration::from_secs(1))
                .w(Duration::from_secs(2)),
            Duration::from_secs(1),
        );
        assert!(in_progress);
        assert_eq!(style.size, size(length(15.0), length(20.0)));

        let (in_progress, style) = size_transition_after(
            StyleTransitions::new()
                .w(Duration::from_secs(2))
                .size(Duration::from_secs(1)),
            Duration::from_secs(1),
        );
        assert!(!in_progress);
        assert_eq!(style.size, size(length(20.0), length(20.0)));

        let started_at = Instant::now();
        let mut state = StyleTransitionState::default();
        let transitions = StyleTransitions::new()
            .opacity(Duration::from_secs(1))
            .rounded(Duration::from_secs(1))
            .rounded_smoothing(Duration::from_secs(1))
            .p(Duration::from_secs(1))
            .border(Duration::from_secs(1))
            .gap(Duration::from_secs(1))
            .text_size(Duration::from_secs(1));
        let context = StyleTransitionContext::new(
            Some(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(100.0), px(60.0)),
            }),
            px(16.0),
        );
        let mut style = Style {
            corner_radii: corners(30.0),
            corner_smoothing: Some(0.0),
            ..Style::default()
        };

        assert!(!transitions.apply(&mut style, &mut state, context, started_at, false,));
        assert_eq!(style.opacity, None);
        assert_eq!(style.corner_radii, corners(30.0));
        assert_eq!(style.corner_smoothing, Some(0.0));

        style.opacity = Some(0.5);
        style.corner_radii = corners(0.0);
        style.corner_smoothing = Some(1.0);
        style.padding = Edges::all(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(20.0))));
        style.border_widths = Edges::all(AbsoluteLength::Pixels(px(10.0)));
        style.gap = size(
            DefiniteLength::Absolute(AbsoluteLength::Pixels(px(30.0))),
            DefiniteLength::Absolute(AbsoluteLength::Pixels(px(30.0))),
        );
        style.text.font_size = Some(AbsoluteLength::Pixels(px(24.0)));
        assert!(transitions.apply(&mut style, &mut state, context, started_at, false,));

        style.opacity = Some(0.5);
        style.corner_radii = corners(0.0);
        style.corner_smoothing = Some(1.0);
        style.padding = Edges::all(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(20.0))));
        style.border_widths = Edges::all(AbsoluteLength::Pixels(px(10.0)));
        style.gap = size(
            DefiniteLength::Absolute(AbsoluteLength::Pixels(px(30.0))),
            DefiniteLength::Absolute(AbsoluteLength::Pixels(px(30.0))),
        );
        style.text.font_size = Some(AbsoluteLength::Pixels(px(24.0)));
        assert!(transitions.apply(
            &mut style,
            &mut state,
            context,
            started_at + Duration::from_millis(500),
            false,
        ));
        assert_eq!(style.opacity, Some(0.25));
        assert_eq!(style.corner_radii, corners(15.0));
        assert_eq!(style.corner_smoothing, Some(0.5));
        assert_eq!(
            style.padding,
            Edges::all(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(10.0))))
        );
        assert_eq!(
            style.border_widths,
            Edges::all(AbsoluteLength::Pixels(px(5.0)))
        );
        assert_eq!(
            style.gap,
            size(
                DefiniteLength::Absolute(AbsoluteLength::Pixels(px(15.0))),
                DefiniteLength::Absolute(AbsoluteLength::Pixels(px(15.0))),
            )
        );
        assert_eq!(style.text.font_size, Some(AbsoluteLength::Pixels(px(12.0))));
    }

    struct InsetTransitionTestView {
        enabled: bool,
        presented_bounds: Rc<Cell<Bounds<Pixels>>>,
    }

    impl Render for InsetTransitionTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let presented_bounds = self.presented_bounds.clone();

            div().size_full().child(
                div()
                    .id("track")
                    .w(px(100.0))
                    .h(px(40.0))
                    .p(px(10.0))
                    .child(
                        div()
                            .id("knob")
                            .absolute()
                            .w(px(20.0))
                            .h(px(20.0))
                            .when(self.enabled, |knob| knob.right(px(10.0)))
                            .transitions(|transitions| transitions.right(Duration::from_secs(1)))
                            .child(canvas(
                                move |bounds, _, _| presented_bounds.set(bounds),
                                |_, _, _, _| {},
                            )),
                    ),
            )
        }
    }

    #[gpui::test]
    fn auto_to_right_inset_moves_from_the_rendered_position_without_jumping(
        cx: &mut TestAppContext,
    ) {
        let presented_bounds = Rc::new(Cell::new(Bounds::default()));
        let window = cx.add_window({
            let presented_bounds = presented_bounds.clone();
            move |_, _| InsetTransitionTestView {
                enabled: false,
                presented_bounds,
            }
        });
        let any_window = AnyWindowHandle::from(window);

        let draw = |cx: &mut TestAppContext| {
            cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
                .expect("failed to draw inset transition test window");
        };
        let assert_x_near = |expected: f32| {
            let actual = presented_bounds.get().origin.x.0;
            assert!(
                (actual - expected).abs() < 0.01,
                "expected knob x near {expected}, got {actual}",
            );
        };

        draw(cx);
        assert_x_near(10.0);

        window
            .update(cx, |view, _, cx| {
                view.enabled = true;
                cx.notify();
            })
            .expect("failed to enable inset transition test view");
        draw(cx);
        assert_x_near(10.0);

        cx.executor().advance_clock(Duration::from_millis(500));
        draw(cx);
        assert_x_near(40.0);

        cx.executor().advance_clock(Duration::from_millis(500));
        draw(cx);
        assert_x_near(70.0);

        window
            .update(cx, |view, _, cx| {
                view.enabled = false;
                cx.notify();
            })
            .expect("failed to disable inset transition test view");
        draw(cx);
        assert_x_near(70.0);

        cx.executor().advance_clock(Duration::from_millis(500));
        draw(cx);
        assert_x_near(40.0);

        cx.executor().advance_clock(Duration::from_millis(500));
        draw(cx);
        assert_x_near(10.0);
    }

    struct StyleTransitionTestView {
        transitions_enabled: bool,
        base_width: Pixels,
        presented_width: Rc<Cell<Pixels>>,
    }

    impl Render for StyleTransitionTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let presented_width = self.presented_width.clone();

            div().size_full().child(
                div()
                    .id("transition-target")
                    .h(px(50.0))
                    .w(self.base_width)
                    .when(self.transitions_enabled, |element| {
                        element.transitions(|transitions| transitions.w(Duration::from_millis(200)))
                    })
                    .hover_style(|style| style.w(px(200.0)))
                    .active_style(|style| style.w(px(50.0)))
                    .child(
                        canvas(
                            move |bounds, _, _| presented_width.set(bounds.size.width),
                            |_, _, _, _| {},
                        )
                        .size_full(),
                    ),
            )
        }
    }

    #[gpui::test]
    fn style_transitions_follow_interaction_and_reset_persistent_state(cx: &mut TestAppContext) {
        let presented_width = Rc::new(Cell::new(px(0.0)));
        let window = cx.add_window({
            let presented_width = presented_width.clone();
            move |_, _| StyleTransitionTestView {
                transitions_enabled: true,
                base_width: px(100.0),
                presented_width,
            }
        });
        let any_window = AnyWindowHandle::from(window);
        let mouse_position = point(px(10.0), px(10.0));

        let draw = |cx: &mut TestAppContext| {
            if let Err(error) =
                cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
            {
                panic!("failed to draw transition test window: {error:#}");
            }
        };
        let assert_width_near = |expected: f32| {
            assert!(
                (presented_width.get().0 - expected).abs() < 0.01,
                "expected width near {expected}, got {}",
                presented_width.get().0,
            );
        };

        draw(cx);
        assert_width_near(100.0);

        if let Err(error) = cx.update_window(any_window, |_, window, cx| {
            window.simulate_mouse_move(mouse_position, cx);
            window.draw(cx).clear(cx);
        }) {
            panic!("failed to move the mouse into the transition target: {error:#}");
        }
        assert_width_near(100.0);

        cx.executor().advance_clock(Duration::from_millis(100));
        draw(cx);
        assert_width_near(150.0);

        if let Err(error) = cx.update_window(any_window, |_, window, cx| {
            window.dispatch_event(
                MouseDownEvent {
                    position: mouse_position,
                    button: MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
            window.draw(cx).clear(cx);
        }) {
            panic!("failed to press the transition target: {error:#}");
        }
        assert_width_near(150.0);

        cx.executor().advance_clock(Duration::from_millis(100));
        draw(cx);
        assert_width_near(100.0);
        cx.executor().advance_clock(Duration::from_millis(100));
        draw(cx);
        assert_width_near(50.0);

        if let Err(error) = cx.update_window(any_window, |_, window, cx| {
            window.dispatch_event(
                MouseUpEvent {
                    position: mouse_position,
                    button: MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            window.simulate_mouse_move(point(px(300.0), px(300.0)), cx);
        }) {
            panic!("failed to release the transition target: {error:#}");
        }
        if let Err(error) = window.update(cx, |view, _, cx| {
            view.transitions_enabled = false;
            view.base_width = px(240.0);
            cx.notify();
        }) {
            panic!("failed to disable transitions: {error:#}");
        }
        draw(cx);
        assert_width_near(240.0);

        if let Err(error) = window.update(cx, |view, _, cx| {
            view.transitions_enabled = true;
            cx.notify();
        }) {
            panic!("failed to re-enable transitions: {error:#}");
        }
        draw(cx);
        assert_width_near(240.0);
    }

    struct HoverTransitionFocusTestView {
        target_focus: FocusHandle,
        next_focus: FocusHandle,
        presented_width: Rc<Cell<Pixels>>,
    }

    impl Render for HoverTransitionFocusTestView {
        fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let presented_width = self.presented_width.clone();
            let hover_width = if self.target_focus.is_focused(window) {
                px(200.0)
            } else {
                px(150.0)
            };

            div()
                .size_full()
                .child(
                    div()
                        .id("transition-target")
                        .w(px(100.0))
                        .h(px(50.0))
                        .track_focus(&self.target_focus)
                        .hover_style(move |style| style.w(hover_width))
                        .transitions(|transitions| {
                            transitions.w(Duration::from_millis(120).with_easing(ease_in_out))
                        })
                        .on_key_down(|event, window, cx| {
                            if event.keystroke.key == "tab" {
                                window.focus_next(cx);
                            }
                        })
                        .child(
                            canvas(
                                move |bounds, _, _| presented_width.set(bounds.size.width),
                                |_, _, _, _| {},
                            )
                            .size_full(),
                        ),
                )
                .child(div().w(px(50.0)).h(px(50.0)).track_focus(&self.next_focus))
        }
    }

    #[gpui::test]
    fn hover_transitions_follow_input_modality_across_focus_changes(cx: &mut TestAppContext) {
        let target_focus = cx.update(|cx| cx.focus_handle().tab_stop(true));
        let next_focus = cx.update(|cx| cx.focus_handle().tab_stop(true));
        let presented_width = Rc::new(Cell::new(px(0.0)));
        let window = cx.add_window({
            let target_focus = target_focus.clone();
            let next_focus = next_focus.clone();
            let presented_width = presented_width.clone();
            move |_, _| HoverTransitionFocusTestView {
                target_focus,
                next_focus,
                presented_width,
            }
        });
        let any_window = AnyWindowHandle::from(window);

        let simulate_next_frame = |cx: &mut TestAppContext| {
            let callback_count = cx
                .update_window(any_window, |_, window, cx| window.simulate_next_frame(cx))
                .expect("failed to deliver the next transition frame");
            cx.run_until_parked();
            callback_count
        };
        let assert_width_near = |expected: f32| {
            assert!(
                (presented_width.get().0 - expected).abs() < 0.01,
                "expected width near {expected}, got {}",
                presented_width.get().0,
            );
        };

        cx.update_window(any_window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.focus(&target_focus, cx);
            window.simulate_mouse_move(point(px(10.0), px(10.0)), cx);
        })
        .expect("failed to focus and hover the transition target");
        cx.run_until_parked();

        cx.executor().advance_clock(Duration::from_millis(120));
        assert!(simulate_next_frame(cx) > 0);
        assert_width_near(200.0);

        cx.update_window(any_window, |_, window, cx| {
            window.focus(&next_focus, cx);
        })
        .expect("failed to move focus while retaining mouse modality");
        cx.run_until_parked();

        cx.executor().advance_clock(Duration::from_millis(120));
        assert!(simulate_next_frame(cx) > 0);
        assert_width_near(150.0);

        cx.update_window(any_window, |_, window, cx| {
            window.focus(&target_focus, cx);
        })
        .expect("failed to restore focus while retaining mouse modality");
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(120));
        assert!(simulate_next_frame(cx) > 0);
        assert_width_near(200.0);

        cx.simulate_keystrokes(any_window, "tab");
        assert!(
            cx.update_window(any_window, |_, window, _| next_focus.is_focused(window))
                .expect("transition test window should remain open")
        );

        cx.executor().advance_clock(Duration::from_millis(120));
        assert!(simulate_next_frame(cx) > 0);
        assert_width_near(100.0);
        assert_eq!(
            simulate_next_frame(cx),
            0,
            "the completed hover transition must stop requesting frames"
        );
    }
}
