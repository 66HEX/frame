use super::crop::clamp;

const PLAYBACK_END_EPSILON_SECONDS: f64 = 0.05;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineDragTarget {
    Start,
    End,
    Scrub,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaSnapshot {
    pub current_time: f64,
    pub duration: f64,
    pub paused: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlaybackMediaCommand {
    pub seek_to: Option<f64>,
    pub pause: bool,
    pub play: bool,
}

impl PlaybackMediaCommand {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            seek_to: None,
            pause: false,
            play: false,
        }
    }

    #[must_use]
    pub const fn seek(time: f64) -> Self {
        Self {
            seek_to: Some(time),
            pause: false,
            play: false,
        }
    }

    #[must_use]
    pub const fn pause() -> Self {
        Self {
            seek_to: None,
            pause: true,
            play: false,
        }
    }

    #[must_use]
    pub const fn play() -> Self {
        Self {
            seek_to: None,
            pause: false,
            play: true,
        }
    }

    #[must_use]
    pub const fn pause_and_seek(time: f64) -> Self {
        Self {
            seek_to: Some(time),
            pause: true,
            play: false,
        }
    }

    #[must_use]
    pub const fn seek_and_play(time: f64) -> Self {
        Self {
            seek_to: Some(time),
            pause: false,
            play: true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TrimSelection {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineDragUpdate {
    pub command: PlaybackMediaCommand,
    pub trim: Option<TrimSelection>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineDragEnd {
    pub command: PlaybackMediaCommand,
    pub trim: Option<TrimSelection>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "Playback state exposes independent UI capabilities and button states."
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewPlaybackState {
    is_image: bool,
    has_media: bool,
    is_playing: bool,
    current_time: f64,
    duration: f64,
    start_value: f64,
    end_value: f64,
    dragging: Option<TimelineDragTarget>,
    was_playing_before_scrub: bool,
    previous_initial_start: Option<String>,
    previous_initial_end: Option<String>,
}

impl PreviewPlaybackState {
    #[must_use]
    pub const fn new(is_image: bool) -> Self {
        Self {
            is_image,
            has_media: false,
            is_playing: false,
            current_time: 0.0,
            duration: 0.0,
            start_value: 0.0,
            end_value: 0.0,
            dragging: None,
            was_playing_before_scrub: false,
            previous_initial_start: None,
            previous_initial_end: None,
        }
    }

    #[must_use]
    pub const fn is_playing(&self) -> bool {
        self.is_playing
    }

    #[must_use]
    pub const fn current_time(&self) -> f64 {
        self.current_time
    }

    #[must_use]
    pub const fn duration(&self) -> f64 {
        self.duration
    }

    #[must_use]
    pub const fn start_value(&self) -> f64 {
        self.start_value
    }

    #[must_use]
    pub const fn end_value(&self) -> f64 {
        self.end_value
    }

    #[must_use]
    pub const fn dragging(&self) -> Option<TimelineDragTarget> {
        self.dragging
    }

    pub const fn set_is_image(&mut self, is_image: bool) {
        self.is_image = is_image;
    }

    pub const fn clear_media(&mut self) {
        self.has_media = false;
        self.is_playing = false;
        self.current_time = 0.0;
        self.duration = 0.0;
        self.start_value = 0.0;
        self.end_value = 0.0;
    }

    pub fn sync_media(&mut self, snapshot: MediaSnapshot) {
        self.has_media = true;
        self.is_playing = !snapshot.paused;
        self.current_time = finite_or_zero(snapshot.current_time);
        self.sync_from_media(snapshot);
    }

    pub fn sync_initial_values(&mut self, initial_start: Option<&str>, initial_end: Option<&str>) {
        if self.previous_initial_start.as_deref() != initial_start {
            self.previous_initial_start = initial_start.map(str::to_string);
            self.start_value = initial_start.map_or(0.0, parse_time_to_seconds);
        }

        if self.previous_initial_end.as_deref() != initial_end {
            self.previous_initial_end = initial_end.map(str::to_string);
            if let Some(initial_end) = initial_end {
                self.end_value = parse_time_to_seconds(initial_end);
            } else if self.duration > 0.0 {
                self.end_value = self.duration;
            }
        } else if initial_end.is_none() && self.duration > 0.0 && self.end_value == 0.0 {
            self.end_value = self.duration;
        }
    }

    pub fn sync_from_media(&mut self, snapshot: MediaSnapshot) {
        if self.is_image {
            return;
        }

        self.duration = finite_or_zero(snapshot.duration);
        self.current_time = finite_or_zero(snapshot.current_time);
        self.end_value = self
            .previous_initial_end
            .as_deref()
            .map_or(self.duration, parse_time_to_seconds);

        if self.start_value > self.duration {
            self.start_value = 0.0;
        }
        if self.end_value > self.duration {
            self.end_value = self.duration;
        }
    }

    pub const fn handle_play(&mut self) {
        self.is_playing = true;
    }

    pub const fn handle_pause(&mut self) {
        self.is_playing = false;
    }

    #[must_use]
    pub fn handle_time_update(&mut self, current_time: f64) -> PlaybackMediaCommand {
        if self.is_image || !self.has_media {
            return PlaybackMediaCommand::none();
        }

        self.current_time = finite_or_zero(current_time);
        if self.dragging.is_some() {
            return PlaybackMediaCommand::none();
        }

        if playback_time_reached_end(self.current_time, self.end_value)
            && self.end_value > self.start_value
        {
            self.is_playing = false;
            self.current_time = self.start_value;
            return PlaybackMediaCommand::pause_and_seek(self.start_value);
        }

        PlaybackMediaCommand::none()
    }

    #[must_use]
    pub fn toggle_play(&mut self) -> PlaybackMediaCommand {
        if self.is_image || !self.has_media {
            return PlaybackMediaCommand::none();
        }

        if self.is_playing {
            return PlaybackMediaCommand::pause();
        }

        if self.current_time < self.start_value
            || playback_time_reached_end(self.current_time, self.end_value)
        {
            self.current_time = self.start_value;
            return PlaybackMediaCommand::seek_and_play(self.start_value);
        }

        PlaybackMediaCommand::play()
    }

    #[must_use]
    pub fn commit_trim_values(&self) -> Option<TrimSelection> {
        if self.is_image {
            return None;
        }

        Some(TrimSelection {
            start_time: (self.start_value > 0.0).then(|| format_time(self.start_value)),
            end_time: (self.duration > 0.0 && self.end_value < self.duration)
                .then(|| format_time(self.end_value)),
        })
    }

    #[must_use]
    pub fn set_start_from_input(&mut self, value: f64) -> Option<PlaybackMediaCommand> {
        if value >= 0.0 && value < self.end_value {
            self.start_value = value;
            return Some(PlaybackMediaCommand::seek(value));
        }

        None
    }

    #[must_use]
    pub fn set_end_from_input(&mut self, value: f64) -> Option<PlaybackMediaCommand> {
        if value > self.start_value && value <= self.duration {
            self.end_value = value;
            return Some(PlaybackMediaCommand::seek(value));
        }

        None
    }

    pub const fn begin_handle_drag(&mut self, target: TimelineDragTarget) -> bool {
        if self.is_image {
            return false;
        }

        self.dragging = Some(target);
        true
    }

    #[must_use]
    pub fn seek_to_percent(&mut self, percent: f64) -> PlaybackMediaCommand {
        if self.is_image {
            return PlaybackMediaCommand::none();
        }

        let time = self.time_from_slider_percent(percent);
        self.current_time = time;
        self.dragging = Some(TimelineDragTarget::Scrub);
        self.was_playing_before_scrub = self.is_playing;

        if self.is_playing {
            PlaybackMediaCommand::pause_and_seek(time)
        } else {
            PlaybackMediaCommand::seek(time)
        }
    }

    #[must_use]
    pub fn seek_once_to_percent(&mut self, percent: f64) -> PlaybackMediaCommand {
        if self.is_image {
            return PlaybackMediaCommand::none();
        }

        let time = self.time_from_slider_percent(percent);
        self.current_time = time;
        PlaybackMediaCommand::seek(time)
    }

    #[must_use]
    pub fn drag_to_percent(&mut self, percent: f64) -> TimelineDragUpdate {
        if self.is_image {
            return TimelineDragUpdate {
                command: PlaybackMediaCommand::none(),
                trim: None,
            };
        }

        let Some(dragging) = self.dragging else {
            return TimelineDragUpdate {
                command: PlaybackMediaCommand::none(),
                trim: None,
            };
        };

        let time = self.time_from_slider_percent(percent);
        match dragging {
            TimelineDragTarget::Scrub => {
                self.current_time = time;
                TimelineDragUpdate {
                    command: PlaybackMediaCommand::seek(time),
                    trim: None,
                }
            }
            TimelineDragTarget::Start => {
                self.start_value = time.min(self.end_value - 1.0);
                TimelineDragUpdate {
                    command: PlaybackMediaCommand::seek(self.start_value),
                    trim: self.commit_trim_values(),
                }
            }
            TimelineDragTarget::End => {
                self.end_value = time.max(self.start_value + 1.0);
                TimelineDragUpdate {
                    command: PlaybackMediaCommand::seek(self.end_value),
                    trim: self.commit_trim_values(),
                }
            }
        }
    }

    #[must_use]
    pub fn end_drag(&mut self) -> TimelineDragEnd {
        let command = if self.dragging == Some(TimelineDragTarget::Scrub) {
            if self.was_playing_before_scrub {
                PlaybackMediaCommand::seek_and_play(self.current_time)
            } else {
                PlaybackMediaCommand::seek(self.current_time)
            }
        } else {
            PlaybackMediaCommand::none()
        };
        let trim = if self
            .dragging
            .is_some_and(|target| target != TimelineDragTarget::Scrub)
        {
            self.commit_trim_values()
        } else {
            None
        };

        self.dragging = None;
        self.was_playing_before_scrub = false;

        TimelineDragEnd { command, trim }
    }

    #[must_use]
    pub fn to_timeline_percent(&self, value: f64) -> f64 {
        if !self.duration.is_finite() || self.duration <= 0.0 {
            return 0.0;
        }

        (value / self.duration) * 100.0
    }

    fn time_from_slider_percent(&self, percent: f64) -> f64 {
        clamp(percent, 0.0, 1.0) * self.duration
    }
}

#[must_use]
pub fn parse_time_to_seconds(time: &str) -> f64 {
    let mut parts = time.split(':');
    let (Some(hours), Some(minutes), Some(seconds), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return 0.0;
    };

    let Ok(hours) = hours.parse::<f64>() else {
        return 0.0;
    };
    let Ok(minutes) = minutes.parse::<f64>() else {
        return 0.0;
    };
    let Ok(seconds) = seconds.parse::<f64>() else {
        return 0.0;
    };

    hours.mul_add(3600.0, minutes * 60.0) + seconds
}

#[must_use]
pub fn format_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor();
    let minutes = ((seconds % 3600.0) / 60.0).floor();
    let seconds = seconds % 60.0;

    format!("{hours:02.0}:{minutes:02.0}:{seconds:06.3}")
}

const fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

fn playback_time_reached_end(current_time: f64, end_time: f64) -> bool {
    current_time + PLAYBACK_END_EPSILON_SECONDS >= end_time
}
