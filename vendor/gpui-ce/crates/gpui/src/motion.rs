use std::{
    num::NonZeroU32,
    ops::{Deref, DerefMut, Sub},
    rc::Rc,
    time::Duration,
};

use crate::spring::DEFAULT_SPRING_EPSILON;
use crate::{SpringAnimation, SpringConfig, SpringState, SpringTarget};

/// Creates a duration from a number of whole seconds.
pub const fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// Creates a duration from a number of whole milliseconds.
pub const fn millis(milliseconds: u64) -> Duration {
    Duration::from_millis(milliseconds)
}

/// Animation progress is normalized before easing and may overshoot afterward.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Progress(f32);

impl Progress {
    /// The beginning of an animation.
    pub const START: Self = Self(0.0);

    /// The end of an animation.
    pub const END: Self = Self(1.0);

    /// Returns progress clamped to the normalized range.
    pub fn clamped(value: f32) -> Self {
        assert!(!value.is_nan(), "progress must not be NaN");
        Self(value.clamp(Self::START.0, Self::END.0))
    }

    /// Returns the underlying progress value.
    pub const fn get(self) -> f32 {
        self.0
    }

    /// Returns whether presentation is exactly at one. This does not
    /// indicate whole-run completion; use [`MotionSample::is_complete`].
    pub const fn is_at_end(self) -> bool {
        self.0 == Self::END.0
    }

    pub(crate) fn reversed(self) -> Self {
        Self::eased(Self::END.0 - self.0)
    }

    fn eased(value: f32) -> Self {
        assert!(value.is_finite(), "easing must return a finite value");
        Self(value)
    }
}

/// Creates duration-based motion with easing or a sampled spring.
pub trait MotionDurationExt {
    /// Creates motion with this duration and the supplied easing function.
    fn with_easing(self, easing: impl Fn(f32) -> f32 + 'static) -> Motion;

    /// Samples a spring over this duration, ending when the duration expires.
    fn with_spring(self, config: SpringConfig) -> Motion;
}

impl MotionDurationExt for Duration {
    fn with_easing(self, easing: impl Fn(f32) -> f32 + 'static) -> Motion {
        Motion::new(self).with_easing(easing)
    }

    fn with_spring(self, config: SpringConfig) -> Motion {
        Motion::new(self).with_spring(config)
    }
}

/// The former name of [`MotionDurationExt`].
pub use MotionDurationExt as DurationWithEasing;

/// Maps linear progress to eased progress.
#[derive(Clone)]
pub struct Easing(Rc<dyn Fn(f32) -> f32>);

impl Easing {
    /// Creates an easing function.
    pub fn new(easing: impl Fn(f32) -> f32 + 'static) -> Self {
        Self(Rc::new(easing))
    }

    /// Evaluates normalized progress without clamping the eased result.
    pub fn sample(&self, progress: Progress) -> Progress {
        Progress::eased((self.0)(progress.get()))
    }
}

impl Default for Easing {
    fn default() -> Self {
        Self::new(crate::linear)
    }
}

/// The duration and easing curve for one directional motion pass.
///
/// Local pass time advances from zero to one. The easing function maps that
/// local time to presentation progress and may be non-monotonic or overshoot.
/// On a reverse leg, [`Motion`] maps the eased result back toward the origin.
#[derive(Clone)]
pub struct MotionPass {
    duration: Duration,
    easing: Easing,
}

impl MotionPass {
    /// Creates a linear pass with the supplied duration.
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: Easing::default(),
        }
    }

    /// Configures this pass's easing function.
    pub fn with_easing(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Easing::new(easing);
        self
    }

    /// Returns this pass's duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns this pass's easing function.
    pub fn easing(&self) -> &Easing {
        &self.easing
    }

    fn duration_nanos(&self) -> u128 {
        self.duration.as_nanos()
    }

    fn local_time(&self, elapsed_nanos: u128) -> Progress {
        let duration_nanos = self.duration_nanos();
        debug_assert!(duration_nanos > 0);
        debug_assert!(elapsed_nanos < duration_nanos);

        let linear = elapsed_nanos as f64 / duration_nanos as f64;
        Progress::clamped(linear as f32)
    }
}

impl From<Duration> for MotionPass {
    fn from(duration: Duration) -> Self {
        Self::new(duration)
    }
}

/// The number of passes in a playback run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Iterations {
    /// A nonzero number of passes, including the first.
    Finite(NonZeroU32),
    /// Continue until the owner stops playback.
    Forever,
}

// Derive(Default) cannot select a data-bearing variant.
impl Default for Iterations {
    fn default() -> Self {
        Self::Finite(NonZeroU32::MIN)
    }
}

/// The configured direction of successive iterations.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Direction {
    /// Every iteration travels from origin to target.
    #[default]
    Forward,
    /// Every iteration travels from target to origin.
    Reverse,
    /// Begin forward, then alternate directions.
    Alternate,
    /// Begin in reverse, then alternate directions.
    AlternateReverse,
}

/// The direction of the pass selected for a sample.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassDirection {
    /// Origin to target.
    Forward,
    /// Target to origin.
    Reverse,
}

/// The iteration count and configured direction of a run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Playback {
    iterations: Iterations,
    direction: Direction,
}

impl Playback {
    /// Creates a playback policy.
    pub const fn new(iterations: Iterations, direction: Direction) -> Self {
        Self {
            iterations,
            direction,
        }
    }

    /// Returns the configured iteration policy.
    pub const fn iterations(self) -> Iterations {
        self.iterations
    }

    /// Returns the configured direction, which may alternate between passes.
    pub const fn direction(self) -> Direction {
        self.direction
    }
}

/// The total configured playback extent, including the initial delay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionExtent {
    /// A finite duration that fits in [`Duration`].
    Finite(Duration),
    /// Playback has no configured end.
    Infinite,
}

/// The lifecycle state of a sampled run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionPhase {
    /// The initial delay has not elapsed.
    Delayed,
    /// Playback is in progress.
    Playing,
    /// All finite duration iterations have elapsed, or a spring has reached
    /// its settling cutoff.
    Completed,
    /// Infinite playback has no changing presentation because every applicable pass is zero length.
    Inactive,
}

impl Direction {
    fn pass(self, iteration: u128) -> PassDirection {
        match self {
            Self::Forward => PassDirection::Forward,
            Self::Reverse => PassDirection::Reverse,
            Self::Alternate => {
                if iteration.is_multiple_of(2) {
                    PassDirection::Forward
                } else {
                    PassDirection::Reverse
                }
            }
            Self::AlternateReverse => {
                if iteration.is_multiple_of(2) {
                    PassDirection::Reverse
                } else {
                    PassDirection::Forward
                }
            }
        }
    }
}

/// The result of evaluating motion at a point in time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionSample {
    /// Eased presentation progress, which may overshoot zero through one.
    pub progress: Progress,

    /// Uneased normalized time within a duration pass. For settling springs,
    /// this is elapsed time divided by the estimated settling duration.
    pub local_time: Progress,

    /// Zero-based duration iteration index. Settling springs report zero.
    /// A completed duration run reports its final iteration.
    pub iteration: u128,

    /// Resolved duration pass direction, distinct from configured [`Direction`].
    /// Settling springs report their nominal forward target direction.
    pub direction: PassDirection,

    /// Whole-run lifecycle state.
    pub phase: MotionPhase,

    /// Whether another sample may produce a different value.
    /// True in [`MotionPhase::Delayed`] or [`MotionPhase::Playing`], and false
    /// in [`MotionPhase::Completed`] or [`MotionPhase::Inactive`].
    pub is_active: bool,
}

impl MotionSample {
    /// Whether the entire finite run has completed.
    pub const fn is_complete(self) -> bool {
        matches!(self.phase, MotionPhase::Completed)
    }

    /// Whether eased presentation is exactly at one.
    pub const fn is_at_end(self) -> bool {
        self.progress.is_at_end()
    }
}

/// A deterministic, time-sampled description of duration-based motion.
///
/// A motion has one forward [`MotionPass`], an optional reverse pass, an
/// initial delay, and a typed [`Playback`] policy. The default is one forward
/// iteration with no delay. Delay applies once when a run begins.
///
/// [`Motion::iterations`] counts total iterations, including the first.
/// Restarting motion begins each iteration with local time at zero.
/// [`Motion::alternate`] starts forward, then uses a backward leg on every
/// second iteration. [`Direction::AlternateReverse`] starts backward instead.
/// With conventional easing endpoints, forward-first alternating motion rests
/// at its target after an odd count and at its origin after an even count.
/// Reverse-first alternating motion has the opposite endpoints. Custom easing
/// endpoints may settle elsewhere, including outside zero through one.
///
/// A backward leg uses the configured reverse pass, or reuses the forward
/// pass's duration and easing if no reverse pass is configured. Reverse local
/// time still advances from zero to one and is eased in that direction; the
/// resulting presentation progress is `1 - eased`. For example, reverse
/// ease-in starts the backward leg slowly and accelerates toward the origin.
///
/// Local pass time is normalized. Eased presentation may overshoot, as in
/// upstream spring easing, while pass selection remains deterministic. Finite
/// zero-duration motion resolves immediately once its initial delay is
/// satisfied, with the configured direction and iteration parity determining
/// the final presentation. A zero-length leg in otherwise nonzero alternating
/// motion is instantaneous.
/// Indefinite motion whose applicable passes all have zero duration becomes
/// inactive instead of requesting animation frames forever.
#[derive(Clone)]
pub struct DurationDescription {
    forward: MotionPass,
    reverse: Option<MotionPass>,
    delay: Duration,
    playback: Playback,
}

/// Configuration for motion driven by a settling spring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringDescription {
    /// The spring's physical parameters.
    pub(crate) config: SpringConfig,

    /// The distance and velocity threshold for settling.
    pub(crate) epsilon: f32,

    settle_after: Duration,
}

/// A motion with methods determined by its description type.
#[derive(Clone, Debug)]
pub struct Motion<Description = DurationDescription> {
    description: Description,
}

impl<Description> Deref for Motion<Description> {
    type Target = Description;

    fn deref(&self) -> &Self::Target {
        &self.description
    }
}

impl<Description> DerefMut for Motion<Description> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.description
    }
}

impl Motion<DurationDescription> {
    /// Creates one linear motion pass with the supplied duration.
    pub fn new(duration: Duration) -> Self {
        Self {
            description: DurationDescription {
                forward: MotionPass::new(duration),
                reverse: None,
                delay: Duration::ZERO,
                playback: Playback::default(),
            },
        }
    }

    /// Configures the forward pass's easing function.
    ///
    /// An alternating motion without an explicit reverse pass also uses this
    /// easing for its backward iterations.
    pub fn with_easing(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.description.forward.easing = Easing::new(easing);
        self
    }

    /// Replaces easing with a spring sampled over this motion's duration.
    /// Use [`Motion::spring`] for a spring that settles and preserves velocity across retargets.
    pub fn with_spring(self, config: SpringConfig) -> Self {
        let duration = self.forward.duration.as_secs_f32();
        let initial_state = SpringState {
            position: 0.0,
            velocity: 0.0,
        };

        self.with_easing(move |progress| {
            if progress <= 0.0 {
                0.0
            } else if progress >= 1.0 {
                1.0
            } else {
                config
                    .step(initial_state, 1.0, progress * duration)
                    .position
            }
        })
    }

    /// Replaces the forward pass.
    pub fn with_forward_pass(mut self, pass: MotionPass) -> Self {
        self.forward = pass;
        self
    }

    /// Configures the pass used for backward legs.
    ///
    /// This does not infer direction from changes to an [`Animated`](crate::Animated)
    /// target. A newly retargeted run begins with its configured direction. Without an
    /// explicit reverse pass, backward legs reuse the forward pass's duration
    /// and easing.
    ///
    /// Reverse local time advances from zero to one and is eased normally. Its
    /// presentation is then mapped as `1 - eased`, so reverse
    /// ease-in begins slowly and accelerates toward the origin.
    pub fn with_reverse_pass(mut self, pass: MotionPass) -> Self {
        self.reverse = Some(pass);
        self
    }

    /// Delays the start of playback once per run.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Sets the total number of iterations, including the first.
    ///
    /// The default is one. Restarting motion restarts local timing
    /// on every iteration. In forward-first alternating motion, odd-numbered
    /// iterations are forward and even-numbered iterations are backward. With conventional
    /// easing endpoints, counts of one and three rest at the target while counts
    /// of two and four rest at the origin.
    ///
    /// # Panics
    ///
    /// Panics if `iterations` is zero.
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.playback.iterations = Iterations::Finite(
            NonZeroU32::new(iterations).expect("motion iterations must be at least 1"),
        );
        self
    }

    /// Repeats this motion indefinitely.
    ///
    /// A motion whose applicable passes all have zero duration becomes inactive
    /// at a deterministic resting presentation.
    pub fn repeat_forever(mut self) -> Self {
        self.playback.iterations = Iterations::Forever;
        self
    }

    /// Alternates forward and backward iterations.
    ///
    /// Odd-numbered iterations use the forward pass. Even-numbered iterations
    /// use the configured reverse pass, falling back to the forward pass when
    /// none is configured.
    pub fn alternate(mut self) -> Self {
        self.playback.direction = Direction::Alternate;
        self
    }

    /// Configures the direction of playback iterations.
    pub fn direction(mut self, direction: Direction) -> Self {
        self.playback.direction = direction;
        self
    }

    /// Replaces the playback policy.
    pub fn with_playback(mut self, playback: Playback) -> Self {
        self.playback = playback;
        self
    }

    /// Returns the forward pass.
    pub fn forward_pass(&self) -> &MotionPass {
        &self.forward
    }

    /// Returns the pass explicitly configured for backward legs, if any.
    pub fn reverse_pass(&self) -> Option<&MotionPass> {
        self.reverse.as_ref()
    }

    /// Returns the initial delay applied once per run.
    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// Returns the strongly typed iteration and direction policy.
    pub const fn playback(&self) -> Playback {
        self.description.playback
    }

    fn reverse_pass_or_forward(&self) -> &MotionPass {
        self.reverse.as_ref().unwrap_or(&self.forward)
    }

    fn finite_duration_nanos(&self, iterations: NonZeroU32) -> Option<u128> {
        let iterations = u128::from(iterations.get());
        let forward = self.forward.duration_nanos();
        let reverse = self.reverse_pass_or_forward().duration_nanos();
        match self.playback.direction {
            Direction::Forward => forward.checked_mul(iterations),
            Direction::Reverse => reverse.checked_mul(iterations),
            Direction::Alternate => (forward.checked_add(reverse)?)
                .checked_mul(iterations / 2)?
                .checked_add(forward.checked_mul(iterations % 2)?),
            Direction::AlternateReverse => (forward.checked_add(reverse)?)
                .checked_mul(iterations / 2)?
                .checked_add(reverse.checked_mul(iterations % 2)?),
        }
    }

    /// Returns `Some(Infinite)` for unbounded playback and `None` only when a
    /// finite extent cannot fit in [`Duration`].
    pub fn checked_extent(&self) -> Option<MotionExtent> {
        let Iterations::Finite(iterations) = self.playback.iterations else {
            return Some(MotionExtent::Infinite);
        };
        let nanos = self
            .delay
            .as_nanos()
            .checked_add(self.finite_duration_nanos(iterations)?)?;
        let seconds = u64::try_from(nanos / 1_000_000_000).ok()?;
        let subsec_nanos = (nanos % 1_000_000_000) as u32;
        Some(MotionExtent::Finite(Duration::new(seconds, subsec_nanos)))
    }

    fn pass(&self, direction: PassDirection) -> &MotionPass {
        match direction {
            PassDirection::Forward => &self.forward,
            PassDirection::Reverse => self.reverse_pass_or_forward(),
        }
    }

    fn active_pass(&self, elapsed_nanos: u128) -> Option<(&MotionPass, u128, PassDirection, u128)> {
        let forward_duration = self.forward.duration_nanos();
        let reverse_duration = self.reverse_pass_or_forward().duration_nanos();
        match self.playback.direction {
            Direction::Forward | Direction::Reverse => {
                let direction = self.playback.direction.pass(0);
                let pass = self.pass(direction);
                let duration = pass.duration_nanos();
                if duration == 0 {
                    return None;
                }
                let iteration = elapsed_nanos / duration;
                let local = elapsed_nanos % duration;
                Some((pass, local, direction, iteration))
            }
            Direction::Alternate | Direction::AlternateReverse => {
                let cycle = forward_duration + reverse_duration;
                if cycle == 0 {
                    return None;
                }
                let first = self.playback.direction.pass(0);
                let first_duration = self.pass(first).duration_nanos();
                let cycle_index = elapsed_nanos / cycle;
                let in_cycle = elapsed_nanos % cycle;
                let iteration = cycle_index * 2;
                if in_cycle < first_duration {
                    Some((self.pass(first), in_cycle, first, iteration))
                } else {
                    let second = self.playback.direction.pass(1);
                    Some((
                        self.pass(second),
                        in_cycle - first_duration,
                        second,
                        iteration + 1,
                    ))
                }
            }
        }
    }

    fn start_progress(&self) -> Progress {
        match self.playback.direction.pass(0) {
            PassDirection::Forward => Progress::START,
            PassDirection::Reverse => Progress::END,
        }
    }

    fn terminal_sample(
        &self,
        direction: PassDirection,
        iteration: u128,
        phase: MotionPhase,
        zero_duration: bool,
    ) -> MotionSample {
        let progress = if zero_duration {
            match direction {
                PassDirection::Forward => Progress::END,
                PassDirection::Reverse => Progress::START,
            }
        } else {
            self.presentation(self.pass(direction), Progress::END, direction)
        };
        MotionSample {
            progress,
            local_time: Progress::END,
            iteration,
            direction,
            phase,
            is_active: false,
        }
    }

    fn presentation(
        &self,
        pass: &MotionPass,
        local_time: Progress,
        direction: PassDirection,
    ) -> Progress {
        let eased = pass.easing.sample(local_time);
        if direction == PassDirection::Reverse {
            eased.reversed()
        } else {
            eased
        }
    }

    /// Evaluates this motion after the supplied elapsed time.
    ///
    /// Pass selection, iteration parity, and completion use integer nanosecond
    /// arithmetic. Only the fraction within the selected pass is converted to
    /// floating point for easing.
    pub fn sample(&self, elapsed: Duration) -> MotionSample {
        if elapsed < self.delay {
            return MotionSample {
                progress: self.start_progress(),
                local_time: Progress::START,
                iteration: 0,
                direction: self.playback.direction.pass(0),
                phase: MotionPhase::Delayed,
                is_active: true,
            };
        }

        let elapsed_nanos = (elapsed - self.delay).as_nanos();
        if let Iterations::Finite(iterations) = self.playback.iterations {
            let total_duration = self
                .finite_duration_nanos(iterations)
                .expect("finite Motion extent fits in u128 nanoseconds");
            if elapsed_nanos >= total_duration {
                let last = u128::from(iterations.get() - 1);
                return self.terminal_sample(
                    self.playback.direction.pass(last),
                    last,
                    MotionPhase::Completed,
                    total_duration == 0,
                );
            }
        }

        let Some((pass, elapsed_in_pass, direction, iteration)) = self.active_pass(elapsed_nanos)
        else {
            let direction = self.playback.direction.pass(0);
            return MotionSample {
                progress: self.start_progress(),
                local_time: Progress::START,
                iteration: 0,
                direction,
                phase: MotionPhase::Inactive,
                is_active: false,
            };
        };

        // A restarting pass presents its endpoint at the exact boundary while
        // the run itself remains active. Its next sample begins the new pass.
        if matches!(
            self.playback.direction,
            Direction::Forward | Direction::Reverse
        ) && elapsed_nanos > 0
            && elapsed_in_pass == 0
        {
            return MotionSample {
                progress: self.presentation(pass, Progress::END, direction),
                local_time: Progress::END,
                iteration: iteration - 1,
                direction,
                phase: MotionPhase::Playing,
                is_active: true,
            };
        }

        let local_time = pass.local_time(elapsed_in_pass);

        MotionSample {
            progress: self.presentation(pass, local_time, direction),
            local_time,
            iteration,
            direction,
            phase: MotionPhase::Playing,
            is_active: true,
        }
    }

    /// Evaluates this motion between two timestamps.
    pub fn sample_at<Time>(&self, started_at: Time, now: Time) -> MotionSample
    where
        Time: Sub<Time, Output = Duration>,
    {
        self.sample(now - started_at)
    }

    pub(crate) fn resting_progress(&self) -> Progress {
        match self.playback.iterations {
            Iterations::Finite(iterations) => {
                let last = u128::from(iterations.get() - 1);
                match self.playback.direction.pass(last) {
                    PassDirection::Forward => Progress::END,
                    PassDirection::Reverse => Progress::START,
                }
            }
            Iterations::Forever => self.start_progress(),
        }
    }
}

impl Default for Motion<DurationDescription> {
    fn default() -> Self {
        Self::new(Duration::ZERO)
    }
}

impl From<Duration> for Motion<DurationDescription> {
    fn from(duration: Duration) -> Self {
        Self::new(duration)
    }
}

/// The former name of [`Motion`].
#[deprecated(note = "use Motion")]
pub type MotionInfo = Motion;

impl Motion<SpringDescription> {
    /// Creates a spring motion that runs until it settles.
    pub fn spring(config: SpringConfig) -> Self {
        let epsilon = DEFAULT_SPRING_EPSILON;

        Self {
            description: SpringDescription {
                config,
                epsilon,
                settle_after: config.settle_time(SpringState::default(), 1.0, epsilon),
            },
        }
    }

    /// Returns this spring's physical parameters.
    pub fn config(&self) -> SpringConfig {
        self.config
    }

    /// Returns this spring's settling tolerance.
    pub fn epsilon(&self) -> f32 {
        self.epsilon
    }

    /// Sets the spring's settling tolerance.
    pub fn with_epsilon(mut self, epsilon: f32) -> Self {
        self.description.epsilon = epsilon;
        self.description.settle_after =
            self.config
                .settle_time(SpringState::default(), 1.0, epsilon);
        self
    }

    /// Evaluates this spring after the supplied elapsed time, starting from rest at zero progress.
    pub fn sample(&self, elapsed: Duration) -> MotionSample {
        if elapsed >= self.settle_after {
            return MotionSample {
                progress: Progress::END,
                local_time: Progress::END,
                iteration: 0,
                direction: PassDirection::Forward,
                phase: MotionPhase::Completed,
                is_active: false,
            };
        }

        let state = self
            .config
            .step(SpringState::default(), 1.0, elapsed.as_secs_f32());

        MotionSample {
            progress: Progress::eased(state.position),
            local_time: Progress::clamped(
                (elapsed.as_secs_f64() / self.settle_after.as_secs_f64()) as f32,
            ),
            iteration: 0,
            direction: PassDirection::Forward,
            phase: MotionPhase::Playing,
            is_active: true,
        }
    }

    /// Targets a value or projected path with this spring.
    pub fn to<T: SpringTarget>(self, target: T) -> SpringAnimation<T> {
        SpringAnimation {
            motion: self,
            target,
            initial: None,
            playback: crate::SpringPlayback::Running,
        }
    }
}

impl From<SpringConfig> for Motion<SpringDescription> {
    fn from(config: SpringConfig) -> Self {
        Self::spring(config)
    }
}

/// A duration or spring motion that can be sampled through one interface.
/// Each spring sample starts from rest, so retargeting an animated value resets its velocity.
#[derive(Clone)]
pub enum AnyMotion {
    /// Motion that runs for a fixed duration.
    Duration(Motion<DurationDescription>),

    /// Motion that runs until its spring settles.
    Spring(Motion<SpringDescription>),
}

impl AnyMotion {
    pub(crate) fn starts_in_reverse(&self) -> bool {
        matches!(self, Self::Duration(motion) if motion.playback.direction.pass(0) == PassDirection::Reverse)
    }

    /// Evaluates this motion after the supplied elapsed time.
    pub fn sample(&self, elapsed: Duration) -> MotionSample {
        match self {
            Self::Duration(motion) => motion.sample(elapsed),
            Self::Spring(motion) => motion.sample(elapsed),
        }
    }

    /// Evaluates this motion between two timestamps.
    pub fn sample_at<Time>(&self, started_at: Time, now: Time) -> MotionSample
    where
        Time: Sub<Time, Output = Duration>,
    {
        self.sample(now - started_at)
    }
}

impl From<Motion<DurationDescription>> for AnyMotion {
    fn from(motion: Motion<DurationDescription>) -> Self {
        Self::Duration(motion)
    }
}

impl From<Motion<SpringDescription>> for AnyMotion {
    fn from(motion: Motion<SpringDescription>) -> Self {
        Self::Spring(motion)
    }
}

impl From<Duration> for AnyMotion {
    fn from(duration: Duration) -> Self {
        Self::Duration(duration.into())
    }
}

impl From<SpringConfig> for AnyMotion {
    fn from(config: SpringConfig) -> Self {
        Self::Spring(config.into())
    }
}

impl From<&Motion<DurationDescription>> for AnyMotion {
    fn from(motion: &Motion<DurationDescription>) -> Self {
        Self::Duration(motion.clone())
    }
}

impl From<&Motion<SpringDescription>> for AnyMotion {
    fn from(motion: &Motion<SpringDescription>) -> Self {
        Self::Spring(motion.clone())
    }
}

impl From<&AnyMotion> for AnyMotion {
    fn from(motion: &AnyMotion) -> Self {
        motion.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(value: f32) -> Progress {
        Progress::clamped(value)
    }

    fn assert_sample(motion: &Motion, elapsed: Duration, value: f32, is_active: bool) {
        let sample = motion.sample(elapsed);
        assert_eq!(sample.progress, progress(value));
        assert_eq!(sample.is_active, is_active);
    }

    #[test]
    fn creates_durations() {
        assert_eq!(secs(2), Duration::from_secs(2));
        assert_eq!(millis(250), Duration::from_millis(250));
    }

    #[test]
    fn samples_default_one_shot_and_custom_easing() {
        let motion = Duration::from_secs(2).with_easing(|progress| progress * progress);
        assert_eq!(motion.forward_pass().duration(), Duration::from_secs(2));
        assert!(motion.reverse_pass().is_none());
        assert_eq!(motion.delay(), Duration::ZERO);
        assert_eq!(motion.playback(), Playback::default());

        assert_sample(&motion, Duration::ZERO, 0.0, true);
        assert_sample(&motion, Duration::from_secs(1), 0.25, true);
        assert_sample(&motion, Duration::from_secs(3), 1.0, false);
        assert_eq!(
            motion
                .sample_at(Duration::from_secs(3), Duration::from_secs(5))
                .progress,
            Progress::END
        );

        assert_eq!(Progress::clamped(-1.0), Progress::START);
        assert_eq!(Progress::clamped(2.0), Progress::END);
    }

    #[test]
    fn delay_applies_once_at_the_start_of_a_run() {
        let motion = Motion::new(Duration::from_secs(1))
            .with_delay(Duration::from_millis(100))
            .iterations(3);

        assert_sample(&motion, Duration::ZERO, 0.0, true);
        assert_sample(&motion, Duration::from_millis(99), 0.0, true);
        assert_sample(&motion, Duration::from_millis(100), 0.0, true);
        assert_sample(&motion, Duration::from_millis(600), 0.5, true);
        assert_sample(&motion, Duration::from_millis(1_100), 1.0, true);
        assert_sample(&motion, Duration::from_millis(3_100), 1.0, false);
    }

    #[test]
    fn restarting_iterations_have_exact_boundaries_and_stable_completion() {
        let motion = Motion::new(Duration::from_secs(1)).iterations(3);

        for (milliseconds, value, active) in [
            (0, 0.0, true),
            (500, 0.5, true),
            (1_000, 1.0, true),
            (2_000, 1.0, true),
            (2_750, 0.75, true),
            (3_000, 1.0, false),
            (30_000, 1.0, false),
        ] {
            assert_sample(&motion, Duration::from_millis(milliseconds), value, active);
        }
    }

    #[test]
    fn alternating_iterations_reverse_and_settle_by_parity() {
        for (iterations, expected_final) in [(1, 1.0), (2, 0.0), (3, 1.0), (4, 0.0)] {
            let motion = Motion::new(Duration::from_secs(1))
                .iterations(iterations)
                .alternate();
            assert_sample(
                &motion,
                Duration::from_secs(u64::from(iterations)),
                expected_final,
                false,
            );
            assert_sample(&motion, Duration::from_secs(100), expected_final, false);
        }

        let motion = Motion::new(Duration::from_secs(1))
            .iterations(4)
            .alternate();
        for (milliseconds, value) in [
            (0, 0.0),
            (500, 0.5),
            (1_000, 1.0),
            (1_500, 0.5),
            (2_000, 0.0),
            (3_000, 1.0),
        ] {
            assert_sample(&motion, Duration::from_millis(milliseconds), value, true);
        }
    }

    #[test]
    fn asymmetric_alternating_passes_use_local_easing() {
        let motion = Motion::new(Duration::from_secs(1))
            .with_easing(|value| value)
            .with_reverse_pass(
                MotionPass::new(Duration::from_secs(2)).with_easing(|value| value * value),
            )
            .iterations(5)
            .alternate();

        assert_eq!(
            motion.reverse_pass().unwrap().duration(),
            Duration::from_secs(2)
        );
        assert_sample(&motion, Duration::from_millis(500), 0.5, true);
        assert_sample(&motion, Duration::from_secs(1), 1.0, true);
        assert_sample(&motion, Duration::from_secs(2), 0.75, true);
        assert_sample(&motion, Duration::from_secs(3), 0.0, true);
        assert_sample(&motion, Duration::from_millis(6_500), 0.5, true);
        assert_sample(&motion, Duration::from_secs(7), 1.0, false);
    }

    #[test]
    fn infinite_motion_restarts_or_alternates_without_completing() {
        let restarting = Motion::new(Duration::from_secs(1)).repeat_forever();
        assert_sample(&restarting, Duration::from_millis(250), 0.25, true);
        assert_sample(&restarting, Duration::from_secs(1), 1.0, true);
        assert_sample(&restarting, Duration::from_millis(2_500), 0.5, true);

        let alternating = Motion::new(Duration::from_secs(1))
            .repeat_forever()
            .alternate();
        assert_sample(&alternating, Duration::from_millis(1_250), 0.75, true);
        assert_sample(&alternating, Duration::from_secs(2), 0.0, true);
    }

    #[test]
    fn zero_duration_motion_resolves_after_delay_without_spinning() {
        for (iterations, alternate, expected) in [
            (1, false, 1.0),
            (2, false, 1.0),
            (2, true, 0.0),
            (3, true, 1.0),
        ] {
            let mut motion = Motion::new(Duration::ZERO)
                .with_delay(Duration::from_millis(10))
                .iterations(iterations);
            if alternate {
                motion = motion.alternate();
            }
            assert_sample(&motion, Duration::from_millis(9), 0.0, true);
            assert_sample(&motion, Duration::from_millis(10), expected, false);
        }

        let forever = Motion::new(Duration::ZERO).repeat_forever().alternate();
        assert_sample(&forever, Duration::ZERO, 0.0, false);
        assert_sample(&forever, Duration::MAX, 0.0, false);
    }

    #[test]
    fn zero_length_passes_are_skipped_without_iteration() {
        let zero_forward = Motion::new(Duration::ZERO)
            .with_reverse_pass(MotionPass::new(Duration::from_secs(2)))
            .iterations(2)
            .alternate();
        assert_sample(&zero_forward, Duration::ZERO, 1.0, true);
        assert_sample(&zero_forward, Duration::from_secs(1), 0.5, true);
        assert_sample(&zero_forward, Duration::from_secs(2), 0.0, false);

        let zero_reverse = Motion::new(Duration::from_secs(1))
            .with_reverse_pass(MotionPass::new(Duration::ZERO))
            .iterations(3)
            .alternate();
        assert_sample(&zero_reverse, Duration::from_secs(1), 0.0, true);
        assert_sample(&zero_reverse, Duration::from_secs(2), 1.0, false);
    }

    #[test]
    fn nanosecond_and_large_duration_sampling_keep_exact_pass_selection() {
        let nanos = Motion::new(Duration::from_nanos(3))
            .iterations(3)
            .alternate();
        assert_sample(&nanos, Duration::from_nanos(3), 1.0, true);
        assert_sample(&nanos, Duration::from_nanos(6), 0.0, true);
        assert_sample(&nanos, Duration::from_nanos(9), 1.0, false);

        let enormous = Motion::new(Duration::MAX).iterations(u32::MAX);
        assert_sample(&enormous, Duration::MAX, 1.0, true);
    }

    #[test]
    fn playback_model_and_sample_distinguish_end_from_completion() {
        assert_eq!(Iterations::default(), Iterations::Finite(NonZeroU32::MIN));
        assert_eq!(Playback::default().direction(), Direction::Forward);
        let motion = Motion::new(secs(1)).iterations(3);
        assert_eq!(
            motion.playback().iterations(),
            Iterations::Finite(NonZeroU32::new(3).unwrap())
        );
        let boundary = motion.sample(secs(1));
        assert_eq!(boundary.progress, Progress::END);
        assert_eq!(boundary.local_time, Progress::END);
        assert_eq!(boundary.iteration, 0);
        assert!(boundary.is_at_end());
        assert!(!boundary.is_complete());
        let next = motion.sample(secs(1) + Duration::from_nanos(1));
        assert_eq!(next.iteration, 1);
        assert_eq!(next.phase, MotionPhase::Playing);

        let alternating = Motion::new(secs(1)).iterations(2).alternate();
        let end = alternating.sample(secs(2));
        assert_eq!(end.progress, Progress::START);
        assert!(!end.is_at_end());
        assert!(end.is_complete());
        assert_eq!(end.iteration, 1);
    }

    #[test]
    fn overshoot_is_not_the_presentation_endpoint() {
        let motion =
            Motion::new(secs(1)).with_easing(|progress| if progress < 1.0 { 1.2 } else { 1.0 });
        let overshoot = motion.sample(millis(500));
        assert_eq!(overshoot.progress.get(), 1.2);
        assert!(!overshoot.progress.is_at_end());
        assert!(!overshoot.is_at_end());
        assert!(!overshoot.is_complete());
        assert!(motion.sample(secs(1)).is_at_end());
    }

    #[test]
    fn all_directions_resolve_passes_and_reverse_first_delay() {
        for (configured, first, second) in [
            (
                Direction::Forward,
                PassDirection::Forward,
                PassDirection::Forward,
            ),
            (
                Direction::Reverse,
                PassDirection::Reverse,
                PassDirection::Reverse,
            ),
            (
                Direction::Alternate,
                PassDirection::Forward,
                PassDirection::Reverse,
            ),
            (
                Direction::AlternateReverse,
                PassDirection::Reverse,
                PassDirection::Forward,
            ),
        ] {
            let motion = Motion::new(secs(1))
                .with_delay(millis(100))
                .iterations(2)
                .direction(configured);
            let delayed = motion.sample(Duration::ZERO);
            assert_eq!(delayed.phase, MotionPhase::Delayed);
            assert_eq!(delayed.direction, first);
            assert_eq!(
                delayed.progress,
                if first == PassDirection::Reverse {
                    Progress::END
                } else {
                    Progress::START
                }
            );
            let active = motion.sample(millis(600));
            assert_eq!(active.direction, first);
            assert_eq!(active.iteration, 0);
            assert_eq!(active.local_time, progress(0.5));
            let boundary = motion.sample(millis(1_100));
            assert_eq!(boundary.phase, MotionPhase::Playing);
            assert_eq!(
                boundary.progress,
                if first == PassDirection::Forward {
                    Progress::END
                } else {
                    Progress::START
                }
            );
            let later = motion.sample(millis(1_600));
            assert_eq!(later.direction, second);
            assert_eq!(later.iteration, 1);
            assert_eq!(later.local_time, progress(0.5));
            let completed = motion.sample(millis(2_100));
            assert_eq!(completed.direction, second);
            assert!(completed.is_complete());
        }
    }

    #[test]
    fn reverse_pass_easing_is_distinct_from_uneased_local_time() {
        let motion = Motion::new(secs(1))
            .with_reverse_pass(MotionPass::new(secs(2)).with_easing(|t| t * t))
            .direction(Direction::Reverse);
        let sample = motion.sample(secs(1));
        assert_eq!(sample.local_time, progress(0.5));
        assert_eq!(sample.progress, progress(0.75));
        assert_eq!(sample.direction, PassDirection::Reverse);
    }

    #[test]
    fn checked_extent_counts_delay_and_asymmetric_passes() {
        let base = Motion::new(millis(300))
            .with_reverse_pass(MotionPass::new(millis(700)))
            .with_delay(millis(200))
            .iterations(3);
        for (direction, expected) in [
            (Direction::Forward, 1_100),
            (Direction::Reverse, 2_300),
            (Direction::Alternate, 1_500),
            (Direction::AlternateReverse, 1_900),
        ] {
            assert_eq!(
                base.clone().direction(direction).checked_extent(),
                Some(MotionExtent::Finite(millis(expected)))
            );
        }
        for direction in [Direction::Alternate, Direction::AlternateReverse] {
            assert_eq!(
                base.clone()
                    .iterations(2)
                    .direction(direction)
                    .checked_extent(),
                Some(MotionExtent::Finite(millis(1_200)))
            );
        }
        assert_eq!(
            base.repeat_forever().checked_extent(),
            Some(MotionExtent::Infinite)
        );
        assert_eq!(
            Motion::new(Duration::MAX).iterations(2).checked_extent(),
            None
        );
        assert_eq!(
            Motion::new(Duration::MAX)
                .with_delay(Duration::from_nanos(1))
                .checked_extent(),
            None
        );
    }

    #[test]
    fn zero_length_passes_work_in_both_reverse_first_modes() {
        let finite = Motion::new(Duration::ZERO)
            .with_reverse_pass(MotionPass::new(secs(1)))
            .iterations(2)
            .direction(Direction::AlternateReverse);
        assert_eq!(
            finite.sample(Duration::ZERO).direction,
            PassDirection::Reverse
        );
        assert_eq!(finite.sample(Duration::ZERO).progress, Progress::END);
        assert_eq!(finite.sample(secs(1)).phase, MotionPhase::Completed);
        let forever = Motion::new(Duration::ZERO)
            .with_reverse_pass(MotionPass::new(Duration::ZERO))
            .repeat_forever()
            .direction(Direction::AlternateReverse);
        assert_eq!(forever.sample(Duration::MAX).phase, MotionPhase::Inactive);
        assert_eq!(forever.sample(Duration::MAX).progress, Progress::END);
    }

    #[test]
    fn zero_duration_combinations_terminate_or_stay_active_as_configured() {
        for direction in [
            Direction::Forward,
            Direction::Reverse,
            Direction::Alternate,
            Direction::AlternateReverse,
        ] {
            for (forward, reverse) in [
                (Duration::ZERO, Duration::ZERO),
                (Duration::ZERO, secs(1)),
                (secs(1), Duration::ZERO),
            ] {
                let motion = Motion::new(forward)
                    .with_reverse_pass(MotionPass::new(reverse))
                    .iterations(3)
                    .direction(direction);
                let MotionExtent::Finite(extent) = motion.checked_extent().unwrap() else {
                    panic!("finite playback must have finite extent");
                };
                assert_eq!(motion.sample(extent).phase, MotionPhase::Completed);
                assert!(!motion.sample(extent).is_active);

                let forever = motion.repeat_forever();
                let sample = forever.sample(Duration::MAX);
                let applicable_duration = match direction {
                    Direction::Forward => forward,
                    Direction::Reverse => reverse,
                    Direction::Alternate | Direction::AlternateReverse => forward + reverse,
                };
                assert_eq!(
                    sample.phase == MotionPhase::Inactive,
                    applicable_duration.is_zero()
                );
            }
        }

        let fallback = Motion::new(secs(1)).direction(Direction::Reverse);
        assert_eq!(fallback.sample(millis(500)).progress, progress(0.5));
        assert_eq!(
            fallback.checked_extent(),
            Some(MotionExtent::Finite(secs(1)))
        );
    }

    #[test]
    #[should_panic(expected = "motion iterations must be at least 1")]
    fn zero_iterations_are_invalid() {
        let _ = Motion::new(Duration::from_secs(1)).iterations(0);
    }
    #[test]
    fn spring_motion_preserves_overshoot_and_rich_sample_metadata() {
        let config = SpringConfig::new(100.0, 6.0, 1.0);
        let sampled = Motion::new(secs(1)).with_spring(config);
        assert!((1..100).any(|step| { sampled.sample(millis(step * 10)).progress.get() > 1.0 }));

        let native: AnyMotion = config.into();
        let loose = Motion::spring(config).with_epsilon(0.1);
        let cutoff = loose.settle_after;
        assert!(native.sample(cutoff).is_active);
        let completed = AnyMotion::from(loose).sample_at(Duration::ZERO, cutoff);
        assert!(completed.is_complete());
        assert_eq!(completed.progress, Progress::END);
        assert_eq!(completed.local_time, Progress::END);
        assert_eq!(completed.direction, PassDirection::Forward);
    }

    #[test]
    fn resting_progress_uses_the_configured_endpoint() {
        let forward = Motion::new(secs(1)).with_easing(|_| 0.5);
        assert_eq!(forward.resting_progress(), Progress::END);
        assert_eq!(
            forward
                .clone()
                .direction(Direction::Reverse)
                .resting_progress(),
            Progress::START
        );
        assert_eq!(
            forward
                .repeat_forever()
                .direction(Direction::Reverse)
                .resting_progress(),
            Progress::END
        );
    }
}
