use std::{ops::Sub, time::Duration};

use crate::{AnyMotion, Lerp, Progress};

/// A sampled animated value and its activity state.
#[derive(Clone)]
pub struct AnimatedSample<T> {
    /// The interpolated value at the sampled time.
    pub value: T,

    /// Whether another sample may produce a different value.
    pub is_active: bool,

    /// Eased presentation progress between the interruption anchor and logical
    /// target, which may overshoot zero through one.
    pub progress: Progress,
}

/// A logical value together with the state required to animate its changes.
#[derive(Clone)]
pub struct Animated<T, Time = std::time::Instant> {
    value: T,
    initial_value: T,
    last_value: T,
    motion: AnyMotion,
    reverse_progress: bool,
    settled_progress: Option<Progress>,
    started_at: Option<Time>,
}

impl<T, Time> Animated<T, Time>
where
    T: Lerp + Clone + PartialEq,
    Time: Copy + Sub<Time, Output = Duration>,
{
    /// Creates a completed animated value.
    pub fn new(value: T, motion: impl Into<AnyMotion>) -> Self {
        Self {
            initial_value: value.clone(),
            last_value: value.clone(),
            value,
            motion: motion.into(),
            reverse_progress: false,
            settled_progress: None,
            started_at: None,
        }
    }

    /// Returns the logical target value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Returns eased presentation progress without changing state.
    /// The result may overshoot zero through one.
    ///
    /// A value with no active or previously settled run is already presenting
    /// its logical target and therefore reports [`Progress::END`].
    pub(crate) fn progress_at(&self, now: Time) -> Progress {
        self.started_at.map_or_else(
            || self.settled_progress.unwrap_or(Progress::END),
            |started_at| {
                self.presentation_progress(self.motion.sample_at(started_at, now).progress)
            },
        )
    }

    fn presentation_progress(&self, progress: Progress) -> Progress {
        if self.reverse_progress {
            progress.reversed()
        } else {
            progress
        }
    }

    /// Updates the logical target using the current presentation as the
    /// interruption anchor.
    ///
    /// Reverse-first playback flips motion progress into anchor-to-target
    /// progress. A custom easing curve that starts away from zero can still
    /// cause an initial jump.
    pub fn set(&mut self, value: T, motion: impl Into<AnyMotion>, now: Time) -> bool {
        self.retarget(value, motion, now, true)
    }

    /// Updates the logical value and restarts from the initial value.
    pub(crate) fn restart(&mut self, value: T, motion: impl Into<AnyMotion>, now: Time) -> bool {
        self.retarget(value, motion, now, false)
    }

    fn retarget(
        &mut self,
        value: T,
        motion: impl Into<AnyMotion>,
        now: Time,
        continuous: bool,
    ) -> bool {
        if self.value == value {
            return false;
        }

        let current = self.sample(now);
        self.last_value = if continuous {
            current.value
        } else {
            self.initial_value.clone()
        };
        self.value = value;
        self.motion = motion.into();
        // Reverse-first playback uses the opposite interpolation axis. Flip
        // its progress so easing that starts at zero begins at the anchor.
        self.reverse_progress = self.motion.starts_in_reverse();
        self.settled_progress = None;
        self.started_at = Some(now);
        true
    }

    /// Sets the logical and sampled value without animation.
    pub fn jump_to(&mut self, value: T) {
        self.value = value.clone();
        self.last_value = value;
        self.settled_progress = None;
        self.started_at = None;
    }

    /// Aligns an inactive END presentation with its logical target.
    ///
    /// Style transitions present the authored target at this endpoint, even if
    /// interpolation produced a different value. Keeping the same value as the
    /// next interruption anchor prevents a jump on a later retarget.
    pub(crate) fn adopt_completed_target(&mut self) {
        if self.started_at.is_none() && self.settled_progress == Some(Progress::END) {
            self.last_value = self.value.clone();
            self.settled_progress = None;
        }
    }

    /// Restores the value used to initialize this animation.
    pub fn reset(&mut self) {
        self.value = self.initial_value.clone();
        self.last_value = self.initial_value.clone();
        self.settled_progress = None;
        self.started_at = None;
    }

    /// Evaluates the interpolated value at the supplied time.
    pub fn sample(&mut self, now: Time) -> AnimatedSample<T> {
        let Some(started_at) = self.started_at else {
            return AnimatedSample {
                value: self.last_value.clone(),
                is_active: false,
                progress: self.settled_progress.unwrap_or(Progress::END),
            };
        };

        let sample = self.motion.sample_at(started_at, now);
        let progress = self.presentation_progress(sample.progress);
        let value = self.last_value.lerp(&self.value, progress.get());

        if !sample.is_active {
            self.last_value = value.clone();
            self.settled_progress = Some(progress);
            self.started_at = None;
        }

        AnimatedSample {
            value,
            is_active: sample.is_active,
            progress,
        }
    }

    pub(crate) fn scale_by(&mut self, ratio: f32)
    where
        T: std::ops::Mul<f32, Output = T>,
    {
        self.last_value = self.last_value.clone() * ratio;
        self.value = self.value.clone() * ratio;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{Direction, Motion, SpringConfig};

    fn assert_sample(sample: AnimatedSample<f32>, value: f32, progress: Progress, is_active: bool) {
        assert_eq!(
            (sample.value, sample.progress, sample.is_active),
            (value, progress, is_active)
        );
    }

    #[test]
    fn animated_value_supports_a_complete_lifecycle() {
        let motion = Motion::new(Duration::from_secs(1));
        let mut animated = Animated::<f32, Duration>::new(2.0, motion.clone());

        assert_eq!(animated.value(), &2.0);
        assert_eq!(animated.progress_at(Duration::ZERO), Progress::END);
        assert_sample(animated.sample(Duration::ZERO), 2.0, Progress::END, false);

        assert!(animated.set(10.0, &motion, Duration::ZERO));
        assert_eq!(animated.sample(Duration::from_millis(500)).value, 6.0);
        assert_eq!(
            animated.progress_at(Duration::from_millis(500)),
            Progress::clamped(0.5)
        );

        assert_sample(
            animated.sample(Duration::from_secs(1)),
            10.0,
            Progress::END,
            false,
        );
        assert!(!animated.set(10.0, &motion, Duration::from_secs(2)));

        animated.jump_to(8.0);
        assert_sample(
            animated.sample(Duration::from_secs(2)),
            8.0,
            Progress::END,
            false,
        );

        animated.scale_by(2.0);
        assert_eq!(animated.value(), &16.0);

        let immediate = Motion::default();
        assert!(animated.set(12.0, &immediate, Duration::from_secs(2)));
        assert_sample(
            animated.sample(Duration::from_secs(2)),
            12.0,
            Progress::END,
            false,
        );
        animated.reset();

        assert_eq!(animated.value(), &2.0);
        assert_sample(
            animated.sample(Duration::from_millis(500)),
            2.0,
            Progress::END,
            false,
        );
    }

    #[test]
    fn animated_exercises_retargeting_and_non_monotonic_motion() {
        let motion = Motion::new(Duration::from_secs(1));
        let mut continuous = Animated::<f32, Duration>::new(0.0, motion.clone());
        let mut restarting = Animated::<f32, Duration>::new(0.0, motion.clone());

        assert!(continuous.set(10.0, &motion, Duration::ZERO));
        assert!(restarting.set(10.0, &motion, Duration::ZERO));
        assert_eq!(continuous.sample(Duration::from_millis(500)).value, 5.0);
        assert_eq!(restarting.sample(Duration::from_millis(500)).value, 5.0);

        assert!(continuous.set(20.0, &motion, Duration::from_millis(500)));
        assert!(restarting.restart(20.0, &motion, Duration::from_millis(500)));

        let continuous_anchor = continuous.sample(Duration::from_millis(500));
        let restarting_anchor = restarting.sample(Duration::from_millis(500));
        assert_eq!(continuous_anchor.value, 5.0);
        assert_eq!(restarting_anchor.value, 0.0);
        assert!(continuous_anchor.is_active);
        assert!(restarting_anchor.is_active);

        assert_eq!(continuous.sample(Duration::from_secs(1)).value, 12.5);
        assert_eq!(restarting.sample(Duration::from_secs(1)).value, 10.0);
        assert!(!continuous.set(20.0, &motion, Duration::from_secs(1)));
        assert!(!restarting.restart(20.0, &motion, Duration::from_secs(1)));

        let non_monotonic = Motion::new(Duration::from_secs(1)).with_easing(|progress| {
            if progress < 0.5 {
                progress * 2.0
            } else {
                (1.0 - progress) * 2.0
            }
        });
        let mut animated = Animated::<f32, Duration>::new(0.0, non_monotonic.clone());

        assert!(animated.set(1.0, &non_monotonic, Duration::ZERO));
        assert_sample(animated.sample(Duration::ZERO), 0.0, Progress::START, true);
        assert_sample(
            animated.sample(Duration::from_millis(500)),
            1.0,
            Progress::END,
            true,
        );
        assert_sample(
            animated.sample(Duration::from_secs(1)),
            0.0,
            Progress::START,
            false,
        );
        assert_sample(
            animated.sample(Duration::from_secs(2)),
            0.0,
            Progress::START,
            false,
        );
        assert_eq!(
            animated.progress_at(Duration::from_secs(2)),
            Progress::START
        );
        assert_eq!(animated.value(), &1.0);

        animated.jump_to(2.0);
        assert_sample(
            animated.sample(Duration::from_secs(3)),
            2.0,
            Progress::END,
            false,
        );
        let spring =
            Motion::new(Duration::from_secs(1)).with_spring(SpringConfig::new(100.0, 6.0, 1.0));
        let mut animated = Animated::<f32, Duration>::new(0.0, spring.clone());
        assert!(animated.set(1.0, &spring, Duration::ZERO));

        assert!(
            (1..100).any(|step| { animated.sample(Duration::from_millis(step * 10)).value > 1.0 })
        );
        assert_eq!(animated.sample(Duration::from_secs(1)).value, 1.0);
    }

    #[test]
    fn animated_keeps_the_motion_that_started_each_run() {
        let one_second = Motion::new(Duration::from_secs(1));
        let two_seconds = Motion::new(Duration::from_secs(2));
        let mut animated = Animated::<f32, Duration>::new(0.0, one_second.clone());

        assert!(animated.set(10.0, &one_second, Duration::ZERO));
        assert_eq!(animated.sample(Duration::from_millis(500)).value, 5.0);
        assert_eq!(
            animated.progress_at(Duration::from_millis(750)),
            Progress::clamped(0.75)
        );

        assert!(animated.set(20.0, &two_seconds, Duration::from_millis(500)));
        assert_eq!(animated.sample(Duration::from_millis(500)).value, 5.0);
        assert_eq!(animated.sample(Duration::from_millis(1_500)).value, 12.5);
    }

    #[test]
    fn delayed_retargeting_holds_the_interruption_anchor() {
        let motion = Motion::new(Duration::from_secs(1)).with_delay(Duration::from_millis(100));
        let mut animated = Animated::<f32, Duration>::new(0.0, motion.clone());

        assert!(animated.set(10.0, &motion, Duration::ZERO));
        assert_sample(
            animated.sample(Duration::from_millis(100)),
            0.0,
            Progress::START,
            true,
        );
        assert_eq!(animated.sample(Duration::from_millis(600)).value, 5.0);

        assert!(animated.set(20.0, &motion, Duration::from_millis(600)));
        for milliseconds in [600, 650, 700] {
            assert_sample(
                animated.sample(Duration::from_millis(milliseconds)),
                5.0,
                Progress::START,
                true,
            );
        }
        assert_eq!(animated.sample(Duration::from_millis(1_200)).value, 12.5);
    }

    #[test]
    fn reverse_first_retargeting_starts_at_the_current_presentation() {
        let motion = Motion::new(Duration::from_secs(1)).direction(Direction::Reverse);
        let mut animated = Animated::<f32, Duration>::new(0.0, motion.clone());

        assert!(animated.set(10.0, &motion, Duration::ZERO));
        assert_sample(animated.sample(Duration::ZERO), 0.0, Progress::START, true);
        assert_sample(
            animated.sample(Duration::from_millis(500)),
            5.0,
            Progress::clamped(0.5),
            true,
        );

        assert!(animated.set(20.0, &motion, Duration::from_millis(500)));
        assert_sample(
            animated.sample(Duration::from_millis(500)),
            5.0,
            Progress::START,
            true,
        );
        assert_eq!(animated.sample(Duration::from_secs(1)).value, 12.5);
        assert_sample(
            animated.sample(Duration::from_millis(1_500)),
            20.0,
            Progress::END,
            false,
        );

        let alternating = Motion::new(Duration::from_secs(1))
            .iterations(2)
            .direction(Direction::AlternateReverse);
        assert!(animated.set(30.0, &alternating, Duration::from_secs(2)));
        assert_eq!(animated.sample(Duration::from_secs(2)).value, 20.0);
        assert_eq!(animated.sample(Duration::from_secs(3)).value, 30.0);
        assert_sample(
            animated.sample(Duration::from_secs(4)),
            20.0,
            Progress::START,
            false,
        );
    }

    #[test]
    fn finite_iterations_settle_at_their_sampled_presentation() {
        let restarting_motion = Motion::new(Duration::from_secs(1)).iterations(3);
        let mut restarting = Animated::<f32, Duration>::new(0.0, restarting_motion.clone());
        assert!(restarting.set(10.0, &restarting_motion, Duration::ZERO));
        assert_sample(
            restarting.sample(Duration::from_secs(1)),
            10.0,
            Progress::END,
            true,
        );
        assert_sample(
            restarting.sample(Duration::from_millis(1_500)),
            5.0,
            Progress::clamped(0.5),
            true,
        );
        assert_sample(
            restarting.sample(Duration::from_secs(3)),
            10.0,
            Progress::END,
            false,
        );

        let alternating_motion = Motion::new(Duration::from_secs(1))
            .iterations(2)
            .alternate();
        let mut idle = Animated::<f32, Duration>::new(4.0, alternating_motion.clone());
        assert_sample(idle.sample(Duration::ZERO), 4.0, Progress::END, false);
        idle.jump_to(6.0);
        assert_sample(idle.sample(Duration::ZERO), 6.0, Progress::END, false);

        let mut alternating = Animated::<f32, Duration>::new(0.0, alternating_motion.clone());
        assert!(alternating.set(10.0, &alternating_motion, Duration::ZERO));
        assert_sample(
            alternating.sample(Duration::from_secs(1)),
            10.0,
            Progress::END,
            true,
        );
        assert_sample(
            alternating.sample(Duration::from_secs(2)),
            0.0,
            Progress::START,
            false,
        );
        assert_sample(
            alternating.sample(Duration::from_secs(20)),
            0.0,
            Progress::START,
            false,
        );

        let zero_forever = Motion::new(Duration::ZERO).repeat_forever();
        assert!(alternating.set(20.0, &zero_forever, Duration::from_secs(21)));
        assert_sample(
            alternating.sample(Duration::from_secs(21)),
            0.0,
            Progress::START,
            false,
        );
    }
}
