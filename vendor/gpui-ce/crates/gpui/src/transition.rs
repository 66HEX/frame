use std::{
    cell::{Ref, RefCell},
    time::Instant,
};

use crate::{Animated, App, Entity, EntityId, Motion, Progress, Window, lerp::Lerp};

#[derive(Clone)]
struct TransitionCache<T> {
    value: Option<T>,
    progress: Progress,
    is_active: bool,
}

impl<T> TransitionCache<T> {
    fn empty() -> Self {
        Self {
            value: None,
            progress: Progress::END,
            is_active: false,
        }
    }

    fn clear(&mut self) {
        self.value = None;
    }
}

/// An animated transition between values of type `T`.
///
/// `Transition` manages the interpolation of a value from a start state to a goal
/// state using the supplied motion. It supports customizable easing functions and
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
    motion: Motion,

    state: Entity<TransitionState<T>>,

    /// The transition sample cached for this render.
    cache: RefCell<TransitionCache<T>>,

    /// Whether to continue the transition from the current value when the goal changes.
    /// If true, transitions smoothly from current animated value to new goal.
    /// If false, restarts from the original start value.
    continuous: bool,
}

impl<T: Lerp + Clone + PartialEq + 'static> Transition<T> {
    /// Create a new transition with the given motion using the specified state.
    pub fn new(state: Entity<TransitionState<T>>, motion: impl Into<Motion>) -> Self {
        Self {
            motion: motion.into(),
            state,
            cache: RefCell::new(TransitionCache::empty()),
            continuous: true,
        }
    }

    /// Sets the forward pass's easing function for this transition.
    ///
    /// The easing function maps normalized local pass time to presentation
    /// progress, which may be non-monotonic or overshoot zero through one.
    pub fn with_easing(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.motion = self.motion.with_easing(easing);
        self.clear_cache();
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

    fn sample(&self, cx: &mut App) -> Ref<'_, TransitionCache<T>> {
        if self.cache.borrow().value.is_none() {
            let mut state = self.state.as_mut(cx);
            let sample = state.sample(Instant::now());

            *self.cache.borrow_mut() = TransitionCache {
                value: Some(sample.value),
                progress: sample.progress,
                is_active: sample.is_active,
            };
        }

        self.cache.borrow()
    }

    fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
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
        let sample = self.sample(cx);
        if sample.is_active {
            window.request_animation_frame();
        }

        Ref::map(sample, |sample| sample.value.as_ref().unwrap())
    }

    /// Reads the end goal of the transitions.
    pub fn read_goal<'b>(&'b self, cx: &'b mut App) -> &'b T {
        self.state.read(cx).value()
    }

    /// Reads the current value of the cached transition, if it exists.
    pub fn read_cache(&self) -> Ref<'_, Option<T>> {
        Ref::map(self.cache.borrow(), |cache| &cache.value)
    }

    /// Returns eased presentation progress, which may overshoot zero through one.
    /// This does not indicate whole-run completion: restarting iterations begin
    /// again and alternating iterations run in reverse.
    pub fn evaluate_delta(&self, cx: &App) -> f32 {
        if self.cache.borrow().value.is_some() {
            return self.cache.borrow().progress.get();
        }

        self.state.read(cx).progress_at(Instant::now()).get()
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
            let mut value = state.value().clone();
            update(&mut value, cx);
            was_updated = if self.continuous {
                state.set(value, &self.motion, Instant::now())
            } else {
                state.restart(value, &self.motion, Instant::now())
            };
        });

        if was_updated {
            self.clear_cache();
        }

        was_updated
    }

    /// Instantly set the transition to the given target value without animation.
    ///
    /// Sets both the start and end goals to `target` so that subsequent evaluations
    /// return `target` immediately. This is useful for transitions that require a
    /// different start value on each update.
    pub fn jump_to(&self, target: T, cx: &mut App) {
        self.state.update(cx, |state, _cx| state.jump_to(target));
        self.clear_cache();
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
        self.state.update(cx, |state, _cx| state.scale_by(ratio));
        self.clear_cache();
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
        self.state.update(cx, |state, _cx| state.reset());
        self.clear_cache();
    }
}

/// The animated value stored by the legacy transition hooks.
pub type TransitionState<T> = Animated<T, Instant>;

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use crate::{AppContext, Context, IntoElement, Render, canvas, px, size};

    use super::*;
    use gpui::TestAppContext;

    /// Helper to create a Transition directly without using window hooks.
    /// This bypasses the render-phase restriction of use_transition/use_keyed_transition.
    fn create_transition<T: Lerp + Clone + PartialEq + 'static>(
        cx: &mut App,
        motion: impl Into<Motion>,
        initial: T,
    ) -> Transition<T> {
        let motion = motion.into();
        let state = cx.new(|_| TransitionState::new(initial, motion.clone()));
        Transition::new(state, motion)
    }

    struct TransitionTestView {
        transition: Transition<f32>,
        rendered_values: Rc<RefCell<Vec<f32>>>,
    }

    impl Render for TransitionTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let transition = self.transition.clone();
            let rendered_values = self.rendered_values.clone();
            canvas(
                move |_, window, cx| {
                    rendered_values
                        .borrow_mut()
                        .push(*transition.evaluate(window, cx));
                },
                |_, _, _, _| {},
            )
        }
    }

    #[gpui::test]
    fn transition_exercises_state_updates_and_cache(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let transition =
                create_transition(cx, Duration::from_secs(1), 0.0_f32).with_easing(|_| 0.5);

            assert!(transition.read_cache().is_none());
            assert!(transition.update(cx, |value, _| *value = 100.0));
            assert_eq!(*transition.read_goal(cx), 100.0);
            let entity_id = transition.entity_id();
            assert_eq!(entity_id, transition.entity_id());

            let value1 = *transition.sample(cx).value.as_ref().unwrap();
            let value2 = *transition.sample(cx).value.as_ref().unwrap();
            assert_eq!((value1, value2), (50.0, 50.0));
            assert_eq!(transition.evaluate_delta(cx), 0.5);

            assert!(!transition.update(cx, |value, _| *value = 100.0));
            assert!(transition.read_cache().is_some());
            assert!(transition.update(cx, |value, _| *value = 200.0));
            assert!(transition.read_cache().is_none());

            let transition = transition.with_easing(|_| 0.75);
            assert!(transition.read_cache().is_none());
            assert_eq!(*transition.sample(cx).value.as_ref().unwrap(), 125.0);
            assert_eq!(transition.evaluate_delta(cx), 0.5);

            let motion = Motion::new(Duration::from_secs(1)).with_easing(|_| 0.5);
            let continuous = create_transition(cx, motion.clone(), 0.0_f32);
            let restarting = create_transition(cx, motion, 0.0_f32).continuous(false);

            for transition in [&continuous, &restarting] {
                assert!(transition.update(cx, |value, _| *value = 100.0));
                assert_eq!(*transition.sample(cx).value.as_ref().unwrap(), 50.0);
            }

            assert!(continuous.update(cx, |value, _| *value = 200.0));
            assert!(restarting.update(cx, |value, _| *value = 200.0));

            assert_eq!(*continuous.sample(cx).value.as_ref().unwrap(), 125.0);
            assert_eq!(*restarting.sample(cx).value.as_ref().unwrap(), 100.0);

            let motion = Motion::new(Duration::from_secs(1)).with_easing(|_| 0.5);
            let mutators = create_transition(cx, motion, 2.0_f32);

            assert!(mutators.update(cx, |value, _| *value = 10.0));
            assert_eq!(*mutators.sample(cx).value.as_ref().unwrap(), 6.0);

            mutators.scale_by(2.0, cx);
            assert_eq!(*mutators.read_goal(cx), 20.0);
            assert_eq!(*mutators.sample(cx).value.as_ref().unwrap(), 12.0);

            mutators.jump_to(7.0, cx);
            assert_eq!(*mutators.read_goal(cx), 7.0);
            assert_eq!(*mutators.sample(cx).value.as_ref().unwrap(), 7.0);

            mutators.reset(cx);
            assert_eq!(*mutators.read_goal(cx), 2.0);
            assert!(mutators.read_cache().is_none());

            let alternating = create_transition(
                cx,
                Motion::new(Duration::ZERO).iterations(2).alternate(),
                0.0_f32,
            );
            assert!(alternating.update(cx, |value, _| *value = 100.0));
            let sample = alternating.sample(cx);
            assert_eq!(sample.value, Some(0.0));
            assert_eq!(sample.progress, Progress::START);
            assert!(!sample.is_active);
            drop(sample);
            assert_eq!(alternating.evaluate_delta(cx), 0.0);
        });
    }

    #[gpui::test]
    fn evaluate_uses_samples_and_requests_frames_while_active(cx: &mut TestAppContext) {
        let transition = cx.update(|cx| {
            create_transition(
                cx,
                Motion::new(Duration::from_secs(1)).with_easing(|_| 0.5),
                0.0_f32,
            )
        });
        cx.update(|cx| {
            assert!(transition.update(cx, |value, _| *value = 100.0));
        });
        let rendered_values = Rc::new(RefCell::new(Vec::new()));
        let window = cx.open_window(size(px(100.0), px(100.0)), {
            let transition = transition.clone();
            let rendered_values = rendered_values.clone();
            move |_, _| TransitionTestView {
                transition,
                rendered_values,
            }
        });
        cx.run_until_parked();

        assert_eq!(*rendered_values.borrow(), vec![50.0]);
        cx.update(|cx| transition.jump_to(100.0, cx));
        assert_eq!(
            window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap(),
            1
        );
        cx.run_until_parked();

        assert_eq!(*rendered_values.borrow(), vec![50.0, 100.0]);
        assert_eq!(
            window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap(),
            0
        );
    }
}
