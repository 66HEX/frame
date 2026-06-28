use std::{
    borrow::BorrowMut,
    cell::{Ref, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{App, Entity, EntityId, Window, lerp::Lerp, linear};

/// An animated transition between values of type `T`.
///
/// `Transition` manages the interpolation of a value from a start state to a goal
/// state over a specified duration. It supports customizable easing functions and
/// can operate in continuous or non-continuous mode.
///
/// # Type Parameters
///
/// * `T` - The type of value being transitioned. Must implement [`Lerp`], [`Clone`],
///   and [`PartialEq`].
///
/// # Continuous vs Non-Continuous Mode
///
/// By default, transitions operate in continuous mode. When the goal is updated:
/// - **Continuous mode** (`continuous = true`): The transition smoothly continues
///   from the current interpolated value to the new goal.
/// - **Non-continuous mode** (`continuous = false`): The transition restarts from
///   the initial value to the new goal.
///
/// # Example
///
/// ```ignore
/// let transition = window.use_transition(cx, Duration::from_millis(300), |_, _| 0.0_f32)
///     .with_easing(ease_in_out);
///
/// // Get the current interpolated value
/// let value = transition.evaluate(window, cx);
///
/// // Update the goal
/// transition.update(cx, |val, cx| {
///     *val = 1.0;
///     cx.notify();
/// });
/// ```
#[derive(Clone)]
pub struct Transition<T: Lerp + Clone + PartialEq + 'static> {
    /// The amount of time for which this transition should run.
    duration_secs: f32,

    /// A function that takes a delta between 0 and 1 and returns a new delta
    /// between 0 and 1 based on the given easing function.
    easing: Rc<dyn Fn(f32) -> f32>,

    state: Entity<TransitionState<T>>,

    /// A cached version of the transition's value.
    cached_value: RefCell<Option<T>>,

    /// Whether to continue the transition from the current value when the goal changes.
    /// If true, transitions smoothly from current animated value to new goal.
    /// If false, restarts from the original start value.
    continuous: bool,
}

impl<T: Lerp + Clone + PartialEq + 'static> Transition<T> {
    /// Create a new transition with the given duration using the specified state.
    pub fn new(state: Entity<TransitionState<T>>, duration: Duration) -> Self {
        Self {
            duration_secs: duration.as_secs_f32(),
            easing: Rc::new(linear),
            state,
            cached_value: RefCell::new(None),
            continuous: true,
        }
    }

    /// Set the easing function to use for this transition.
    /// The easing function will take a time delta between 0 and 1 and return a new delta
    /// between 0 and 1
    pub fn with_easing(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Rc::new(easing);
        self
    }

    /// Sets whether the transition should be continuous.
    ///
    /// On goal updates, transitions continue from the current value by default.
    /// If `continuous` is set to false, the transition will restart from its initial value.
    pub fn continuous(mut self, continuous: bool) -> Self {
        self.continuous = continuous;
        self
    }

    fn default_goal_updated_at(&self) -> Instant {
        Instant::now() - Duration::from_secs_f32(self.duration_secs)
    }

    /// Evaluates the value of the transition without using the cache.
    /// Returns if the transition is finished (bool) and the evaluated value (T).
    fn raw_evaluate(&self, cx: &mut App) -> (bool, T) {
        let mut state_entity = self.state.as_mut(cx);
        let state: &mut TransitionState<T> = state_entity.borrow_mut();

        let elapsed_secs = state
            .goal_last_updated_at
            .unwrap_or_else(|| self.default_goal_updated_at())
            .elapsed()
            .as_secs_f32();
        let delta = (self.easing)((elapsed_secs / self.duration_secs).min(1.));

        debug_assert!(
            (0.0..=1.0).contains(&delta),
            "delta should always be between 0 and 1"
        );

        state.last_delta = delta;

        let evaluated_value = state.start_goal.lerp(&state.end_goal, delta);

        (delta != 1., evaluated_value)
    }

    /// Evaluates and returns the current interpolated value of the transition.
    ///
    /// This method calculates the value based on the elapsed time since the last
    /// goal update, applies the easing function, and caches the result. If the
    /// transition is still in progress, it automatically requests an animation
    /// frame to continue the animation.
    ///
    /// The returned value is cached for the duration of the current frame to avoid
    /// redundant calculations when called multiple times.
    pub fn evaluate(&self, window: &mut Window, cx: &mut App) -> Ref<'_, T> {
        if self.cached_value.borrow().is_none() {
            let (in_progress, evaluated_value) = self.raw_evaluate(cx);

            if in_progress {
                window.request_animation_frame();
            }

            *self.cached_value.borrow_mut() = Some(evaluated_value);
        }

        Ref::map(self.cached_value.borrow(), |opt| opt.as_ref().unwrap())
    }

    /// Reads the end goal of the transitions.
    pub fn read_goal<'b>(&'b self, cx: &'b mut App) -> &'b T {
        &self.state.read(cx).end_goal
    }

    /// Reads the current value of the cached transition, if it exists.
    pub fn read_cache(&self) -> Ref<'_, Option<T>> {
        self.cached_value.borrow()
    }

    /// Evaluates and returns the current progress delta of the transition.
    ///
    /// Returns a value between 0.0 and 1.0 representing how far the transition
    /// has progressed, after applying the easing function. A value of 0.0 means
    /// the transition just started, and 1.0 means it has completed.
    pub fn evaluate_delta<'b>(&'b self, cx: &'b App) -> f32 {
        let goal_last_updated_at = self
            .state
            .read(cx)
            .goal_last_updated_at
            .unwrap_or_else(|| self.default_goal_updated_at());

        let elapsed_secs = goal_last_updated_at.elapsed().as_secs_f32();
        (self.easing)((elapsed_secs / self.duration_secs).min(1.))
    }

    /// Updates the goal value for the transition.
    ///
    /// The provided closure receives a mutable reference to the current goal value
    /// and can modify it. If the goal changes (and continuous mode is enabled),
    /// a new animation will begin from the current interpolated value toward the
    /// new goal.
    ///
    /// Returns `true` if the goal was actually updated (i.e., the new value differs
    /// from the previous goal), `false` otherwise.
    ///
    /// Note: This method does not automatically notify GPUI of changes. You should
    /// call `cx.notify()` within the closure if you want to trigger a re-render.
    pub fn update<R>(
        &self,
        cx: &mut App,
        update: impl FnOnce(&mut T, &mut crate::Context<TransitionState<T>>) -> R,
    ) -> bool {
        let mut was_updated = false;

        self.state.update(cx, |state, cx| {
            let last_end_goal = state.end_goal.clone();

            update(&mut state.end_goal, cx);

            if self.continuous && state.end_goal == last_end_goal {
                return;
            };

            state.goal_last_updated_at = Some(Instant::now());

            if self.continuous {
                state.start_goal = state.start_goal.lerp(&last_end_goal, state.last_delta);
            }

            was_updated = true;
        });

        was_updated
    }

    /// Instantly set the transition to the given target value without animation.
    ///
    /// Sets both the start and end goals to `target` so that subsequent evaluations
    /// return `target` immediately. This is useful for transitions that require a
    /// different start value on each update.
    pub fn jump_to(&self, target: T, cx: &mut App) {
        self.state.update(cx, |state, _cx| {
            state.start_goal = target.clone();
            state.end_goal = target;
            state.goal_last_updated_at = Some(Instant::now());
        });
        self.cached_value.borrow_mut().take();
    }

    /// Scale the transition's start and end goals by the given ratio.
    ///
    /// This preserves the relative progress of an in-flight animation when the
    /// coordinate space changes (e.g. on window resize).  Both the start and
    /// end goal are multiplied by `ratio` so the interpolated value remains
    /// proportionally correct.
    pub fn scale_by(&self, ratio: f32, cx: &mut App)
    where
        T: std::ops::Mul<f32, Output = T>,
    {
        self.state.update(cx, |state, _cx| {
            state.start_goal = state.start_goal.clone() * ratio;
            state.end_goal = state.end_goal.clone() * ratio;
        });
        self.cached_value.borrow_mut().take();
    }

    /// Returns the entity ID associated with this transition's state.
    ///
    /// This can be useful for tracking or comparing transitions.
    pub fn entity_id(&self) -> EntityId {
        self.state.entity_id()
    }

    /// Resets the transition to its initial state.
    ///
    /// This clears all progress and sets both the start and end goals back to
    /// the initial value that was provided when the transition was created.
    /// The cache is also cleared.
    pub fn reset(&self, cx: &mut App) {
        self.state.update(cx, |state, _cx| {
            state.goal_last_updated_at = None;
            state.start_goal = state.initial_goal.clone();
            state.end_goal = state.initial_goal.clone();
            state.last_delta = 0.0;
        });
        *self.cached_value.borrow_mut() = None;
    }
}

// Internal state container for a [`Transition`](crate::Transition).
///
/// This struct holds the data necessary to track a transition's progress,
/// including the start and end goals, timing information, and the last
/// computed delta value.
#[derive(Clone)]
pub struct TransitionState<T: Lerp + Clone + PartialEq + 'static> {
    pub(crate) goal_last_updated_at: Option<Instant>,
    pub(crate) initial_goal: T,
    pub(crate) start_goal: T,
    pub(crate) end_goal: T,
    pub(crate) last_delta: f32,
}

impl<T: Lerp + Clone + PartialEq + 'static> TransitionState<T> {
    /// Creates a new transition state with the given initial goal.
    ///
    /// The start goal, end goal, and initial goal are all set to the provided value.
    /// The transition begins in a "completed" state (delta = 1.0) until the goal
    /// is updated.
    pub fn new(initial_goal: T) -> Self {
        Self {
            goal_last_updated_at: None,
            initial_goal: initial_goal.clone(),
            start_goal: initial_goal.clone(),
            end_goal: initial_goal,
            last_delta: 1.,
        }
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use crate::AppContext;

    use super::*;
    use gpui::{Point, Rgba, TestAppContext, px};

    /// Helper to create a Transition directly without using window hooks.
    /// This bypasses the render-phase restriction of use_transition/use_keyed_transition.
    fn create_transition<T: Lerp + Clone + PartialEq + 'static>(
        cx: &mut App,
        duration: Duration,
        initial: T,
    ) -> Transition<T> {
        let state = cx.new(|_| TransitionState::new(initial));
        Transition::new(state, duration)
    }

    #[gpui::test]
    fn test_transition_creation(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            // Read the goal - should be the initial value
            let goal = transition.read_goal(cx);
            assert_eq!(*goal, 0.0);
        });
    }

    #[gpui::test]
    fn test_transition_read_goal(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 42.0_f32);

            let goal = transition.read_goal(cx);
            assert_eq!(*goal, 42.0);
        });
    }

    #[gpui::test]
    fn test_transition_update_returns_true_on_change(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            let was_updated = transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            assert!(was_updated);
        });
    }

    #[gpui::test]
    fn test_transition_update_returns_false_on_no_change(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 50.0_f32);

            // Update to the same value
            let was_updated = transition.update(cx, |val, _cx| {
                *val = 50.0;
            });

            assert!(!was_updated);
        });
    }

    #[gpui::test]
    fn test_transition_goal_updated_after_update(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            transition.update(cx, |val, _cx| {
                *val = 200.0;
            });

            let goal = transition.read_goal(cx);
            assert_eq!(*goal, 200.0);
        });
    }

    #[gpui::test]
    fn test_transition_entity_id(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition1 = create_transition(cx, Duration::from_millis(300), 0.0_f32);
            let transition2 = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            // Different transitions should have different entity IDs
            assert_ne!(transition1.entity_id(), transition2.entity_id());
        });
    }

    #[gpui::test]
    fn test_transition_reset(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 10.0_f32);

            // Update the goal
            transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            assert_eq!(*transition.read_goal(cx), 100.0);

            // Reset the transition
            transition.reset(cx);

            // Goal should be back to initial value
            assert_eq!(*transition.read_goal(cx), 10.0);
        });
    }

    #[gpui::test]
    fn test_transition_cache_cleared_on_reset(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 25.0_f32);

            // Manually populate the cache using raw_evaluate
            let (_, value) = transition.raw_evaluate(cx);
            *transition.cached_value.borrow_mut() = Some(value);
            assert!(transition.read_cache().is_some());

            // Reset
            transition.reset(cx);

            // Cache should be cleared
            assert!(transition.read_cache().is_none());
        });
    }

    #[gpui::test]
    fn test_transition_with_custom_easing(cx: &mut TestAppContext) {
        cx.update(|cx| {
            // Custom easing that always returns 0.5
            let transition =
                create_transition(cx, Duration::from_millis(300), 0.0_f32).with_easing(|_| 0.5);

            // Update goal to trigger animation
            transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            // With our custom easing, the delta should be 0.5
            // So the value should be lerp(0, 100, 0.5) = 50
            let (_, value) = transition.raw_evaluate(cx);
            assert_eq!(value, 50.0);
        });
    }

    #[gpui::test]
    fn test_transition_with_point(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let initial: Point<f32> = Point { x: 0.0, y: 0.0 };
            let transition = create_transition(cx, Duration::from_millis(300), initial);

            let goal = transition.read_goal(cx);
            assert_eq!(goal.x, 0.0);
            assert_eq!(goal.y, 0.0);

            transition.update(cx, |point, _cx| {
                point.x = 100.0;
                point.y = 200.0;
            });

            let goal = transition.read_goal(cx);
            assert_eq!(goal.x, 100.0);
            assert_eq!(goal.y, 200.0);
        });
    }

    #[gpui::test]
    fn test_transition_with_rgba(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let initial = Rgba {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            };
            let transition = create_transition(cx, Duration::from_millis(300), initial);

            let goal = transition.read_goal(cx);
            assert_eq!(goal.r, 1.0);
            assert_eq!(goal.g, 0.0);
            assert_eq!(goal.b, 0.0);
        });
    }

    #[gpui::test]
    fn test_transition_continuous_mode_default(cx: &mut TestAppContext) {
        cx.update(|cx| {
            // By default, transitions are continuous
            let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            // First update
            transition.update(cx, |val, _cx| {
                *val = 50.0;
            });

            // Second update (in continuous mode, this should work from current interpolated position)
            let was_updated = transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            assert!(was_updated);
            assert_eq!(*transition.read_goal(cx), 100.0);
        });
    }

    #[gpui::test]
    fn test_transition_non_continuous_mode(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition =
                create_transition(cx, Duration::from_millis(300), 0.0_f32).continuous(false);

            // Update the goal
            transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            assert_eq!(*transition.read_goal(cx), 100.0);
        });
    }

    #[gpui::test]
    fn test_evaluate_delta_initial(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            // Without any update, the transition should be "complete" (delta = 1.0)
            // because it starts in completed state
            let delta = transition.evaluate_delta(cx);
            assert_eq!(delta, 1.0);
        });
    }

    #[gpui::test]
    fn test_evaluate_delta_after_update(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            // Immediately after update, delta should be close to 0
            let delta = transition.evaluate_delta(cx);
            assert!(
                delta < 0.1,
                "delta should be small immediately after update"
            );
        });
    }

    #[gpui::test]
    fn test_transition_clone(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 42.0_f32);

            let cloned = transition.clone();

            // Both should reference the same entity
            assert_eq!(transition.entity_id(), cloned.entity_id());

            // Updating one should affect the other
            transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            assert_eq!(*cloned.read_goal(cx), 100.0);
        });
    }

    #[gpui::test]
    fn test_transition_cache_consistency(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition =
                create_transition(cx, Duration::from_millis(300), 0.0_f32).with_easing(|_| 0.5); // Always return 0.5 for deterministic testing

            transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            // First evaluation using raw_evaluate
            let (_, value1) = transition.raw_evaluate(cx);

            // Second evaluation should return the same value
            let (_, value2) = transition.raw_evaluate(cx);

            assert_eq!(value1, value2);
        });
    }

    #[gpui::test]
    fn test_multiple_transitions_independent(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition_a = create_transition(cx, Duration::from_millis(300), 0.0_f32);
            let transition_b = create_transition(cx, Duration::from_millis(300), 100.0_f32);

            // Update only transition_a
            transition_a.update(cx, |val, _cx| {
                *val = 50.0;
            });

            // transition_b should remain unchanged
            assert_eq!(*transition_a.read_goal(cx), 50.0);
            assert_eq!(*transition_b.read_goal(cx), 100.0);
        });
    }

    #[gpui::test]
    fn test_transition_with_pixels(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), px(0.0));

            let goal = transition.read_goal(cx);
            assert_eq!(*goal, px(0.0));

            transition.update(cx, |val, _cx| {
                *val = px(100.0);
            });

            let goal = transition.read_goal(cx);
            assert_eq!(*goal, px(100.0));
        });
    }

    #[gpui::test]
    fn test_transition_rapid_updates(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            // Rapidly update multiple times
            for i in 1..=10 {
                transition.update(cx, |val, _cx| {
                    *val = i as f32 * 10.0;
                });
            }

            // Final goal should be 100.0
            assert_eq!(*transition.read_goal(cx), 100.0);
        });
    }

    #[gpui::test]
    fn test_raw_evaluate_in_progress(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32);

            transition.update(cx, |val, _cx| {
                *val = 100.0;
            });

            // Immediately after update, the transition should be in progress
            let (in_progress, _value) = transition.raw_evaluate(cx);
            assert!(
                in_progress,
                "transition should be in progress immediately after update"
            );
        });
    }

    #[gpui::test]
    fn test_raw_evaluate_completed(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition = create_transition(cx, Duration::from_millis(300), 50.0_f32);

            // Without any update, the transition starts as "complete"
            let (in_progress, value) = transition.raw_evaluate(cx);
            assert!(
                !in_progress,
                "transition should be complete without updates"
            );
            assert_eq!(value, 50.0);
        });
    }

    #[gpui::test]
    fn test_transition_interpolation_with_easing(cx: &mut TestAppContext) {
        cx.update(|cx| {
            // Test with different easing values
            for expected_delta in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let transition = create_transition(cx, Duration::from_millis(300), 0.0_f32)
                    .with_easing(move |_| expected_delta);

                transition.update(cx, |val, _cx| {
                    *val = 100.0;
                });

                let (_, value) = transition.raw_evaluate(cx);
                let expected_value = 0.0_f32.lerp(&100.0_f32, expected_delta);
                assert_eq!(value, expected_value);
            }
        });
    }

    #[test]
    fn test_new_state_initialization() {
        let state = TransitionState::new(42.0_f32);

        assert_eq!(state.initial_goal, 42.0);
        assert_eq!(state.start_goal, 42.0);
        assert_eq!(state.end_goal, 42.0);
        assert_eq!(state.last_delta, 1.0);
        assert!(state.goal_last_updated_at.is_none());
    }

    #[test]
    fn test_state_with_point() {
        let initial: Point<f32> = Point { x: 10.0, y: 20.0 };
        let state = TransitionState::new(initial);

        assert_eq!(state.initial_goal.x, 10.0);
        assert_eq!(state.initial_goal.y, 20.0);
        assert_eq!(state.start_goal.x, 10.0);
        assert_eq!(state.end_goal.x, 10.0);
    }

    #[test]
    fn test_state_with_rgba() {
        let initial = Rgba {
            r: 1.0,
            g: 0.5,
            b: 0.0,
            a: 1.0,
        };
        let state = TransitionState::new(initial);

        assert_eq!(state.initial_goal.r, 1.0);
        assert_eq!(state.initial_goal.g, 0.5);
        assert_eq!(state.initial_goal.b, 0.0);
        assert_eq!(state.initial_goal.a, 1.0);
    }

    #[test]
    fn test_state_clone() {
        let state = TransitionState::new(100.0_f32);
        let cloned = state.clone();

        assert_eq!(state.initial_goal, cloned.initial_goal);
        assert_eq!(state.start_goal, cloned.start_goal);
        assert_eq!(state.end_goal, cloned.end_goal);
        assert_eq!(state.last_delta, cloned.last_delta);
    }

    #[test]
    fn test_state_with_integer() {
        let state = TransitionState::new(50_i32);

        assert_eq!(state.initial_goal, 50);
        assert_eq!(state.start_goal, 50);
        assert_eq!(state.end_goal, 50);
    }

    #[test]
    fn test_state_starts_completed() {
        let state = TransitionState::new(0.0_f32);

        // last_delta should be 1.0 indicating the transition is "complete"
        assert_eq!(state.last_delta, 1.0);
    }
}
