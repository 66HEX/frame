//! Small UI previews for the motion playback controls.
//!
//! Run with `cargo run -p gpui-ce --example motion_showcase --locked`.
//! Each panel names the API that drives its animation. No network or service is used.

use std::time::{Duration, Instant};

#[path = "../shared/prelude.rs"]
mod example_prelude;

use gpui::{
    App, AppContext, Bounds, Context, FontWeight, Motion, MotionPass, MotionSample, Window,
    WindowBounds, WindowOptions, div, ease_in_out, millis, prelude::*, px, relative, rgb, size,
};

const BACKGROUND: u32 = 0x141a26;
const SURFACE: u32 = 0x1e293b;
const INSET: u32 = 0x101522;
const BORDER: u32 = 0x35435a;
const TEXT: u32 = 0xe5edff;
const MUTED: u32 = 0xbac6db;
const ACCENT: u32 = 0x9bbcff;

struct Motions {
    progress: Motion,
    notification: Motion,
    items: [Motion; 3],
    attention: Motion,
    pulse: Motion,
}

#[derive(Clone, Copy, PartialEq)]
struct Settings {
    progress_ms: u64,
    notification_reverse_ms: u64,
    stagger_gap_ms: u64,
    attention_count: u32,
    pulse_pass_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            progress_ms: 1_500,
            notification_reverse_ms: 1_700,
            stagger_gap_ms: 180,
            attention_count: 3,
            pulse_pass_ms: 700,
        }
    }
}

impl Motions {
    fn new(settings: Settings) -> Self {
        Self {
            progress: Motion::new(millis(settings.progress_ms)).with_easing(ease_in_out),
            notification: Motion::new(millis(280))
                .with_forward_pass(MotionPass::new(millis(280)).with_easing(ease_out_cubic))
                .with_reverse_pass(
                    MotionPass::new(millis(settings.notification_reverse_ms))
                        .with_easing(hold_then_exit),
                )
                .iterations(2)
                .alternate(),
            items: std::array::from_fn(|index| {
                Motion::new(millis(420))
                    .with_delay(millis(120 + index as u64 * settings.stagger_gap_ms))
                    .with_easing(ease_out_cubic)
            }),
            // One pass is one nudge, so the badge returns to rest after n passes.
            attention: Motion::new(millis(500))
                .iterations(settings.attention_count)
                .with_easing(attention_nudge),
            pulse: Motion::new(millis(settings.pulse_pass_ms))
                .repeat_forever()
                .alternate()
                .with_easing(ease_in_out),
        }
    }
}

#[derive(Clone, Copy)]
enum Setting {
    Progress,
    Notification,
    Stagger,
    Attention,
    Pulse,
}

struct MotionShowcase {
    settings: Settings,
    motions: Motions,
    progress: Timeline,
    progress_from: f32,
    progress_to: f32,
    progress_played: bool,
    notification: Timeline,
    items: [Timeline; 3],
    attention: Timeline,
    attention_run_count: u32,
    pulse: Timeline,
}

#[derive(Clone, Copy, Default)]
struct Timeline {
    elapsed: Duration,
    last_tick: Option<Instant>,
    value: f32,
    last_sample: Option<MotionSample>,
}

impl Timeline {
    fn play(&mut self, now: Instant) {
        self.elapsed = Duration::ZERO;
        self.last_tick = Some(now);
        self.value = 0.;
        self.last_sample = None;
    }

    fn stop(&mut self) {
        self.elapsed = Duration::ZERO;
        self.last_tick = None;
        self.value = 0.;
        self.last_sample = None;
    }

    fn is_active(&self) -> bool {
        self.last_tick.is_some()
    }

    fn advance(&mut self, motion: &Motion, now: Instant) -> bool {
        let Some(last_tick) = self.last_tick else {
            return false;
        };

        self.elapsed += now.saturating_duration_since(last_tick);
        let sample = motion.sample(self.elapsed);
        self.value = sample.progress.get();
        self.last_sample = Some(sample);
        self.last_tick = sample.is_active.then_some(now);
        sample.is_active
    }

    fn retime(&mut self, motion: &Motion, elapsed: Duration, now: Instant) {
        self.elapsed = elapsed;
        let sample = motion.sample(elapsed);
        self.value = sample.progress.get();
        self.last_sample = Some(sample);
        self.last_tick = sample.is_active.then_some(now);
    }
}

fn scale_elapsed(elapsed: Duration, old: Duration, new: Duration) -> Duration {
    let nanos = elapsed.as_nanos().saturating_mul(new.as_nanos()) / old.as_nanos();
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

impl MotionShowcase {
    fn new() -> Self {
        let settings = Settings::default();
        Self {
            settings,
            motions: Motions::new(settings),
            progress: Timeline::default(),
            progress_from: 0.,
            progress_to: 1.,
            progress_played: false,
            notification: Timeline::default(),
            items: [Timeline::default(); 3],
            attention: Timeline::default(),
            attention_run_count: settings.attention_count,
            pulse: Timeline::default(),
        }
    }

    fn change_setting(&mut self, setting: Setting, direction: i32, cx: &mut Context<Self>) {
        let previous = self.settings;
        let adjust = |value: &mut u64, step: i64, min: u64, max: u64| {
            *value =
                (*value as i64 + step * i64::from(direction)).clamp(min as i64, max as i64) as u64;
        };

        match setting {
            Setting::Progress => adjust(&mut self.settings.progress_ms, 200, 500, 3_000),
            Setting::Notification => {
                adjust(&mut self.settings.notification_reverse_ms, 300, 800, 3_200)
            }
            Setting::Stagger => adjust(&mut self.settings.stagger_gap_ms, 60, 60, 360),
            Setting::Attention => {
                self.settings.attention_count =
                    (self.settings.attention_count as i32 + direction).clamp(1, 5) as u32;
            }
            Setting::Pulse => adjust(&mut self.settings.pulse_pass_ms, 100, 300, 1_200),
        }

        if self.settings != previous {
            let now = Instant::now();
            let mut updated = Motions::new(self.settings);

            match setting {
                Setting::Progress => {
                    if self.progress.advance(&self.motions.progress, now) {
                        let elapsed = scale_elapsed(
                            self.progress.elapsed,
                            millis(previous.progress_ms),
                            millis(self.settings.progress_ms),
                        );
                        self.progress.retime(&updated.progress, elapsed, now);
                    }
                }
                Setting::Notification => {
                    if self.notification.advance(&self.motions.notification, now) {
                        let forward = millis(280);
                        let elapsed = if self.notification.elapsed <= forward {
                            self.notification.elapsed
                        } else {
                            forward
                                + scale_elapsed(
                                    self.notification.elapsed - forward,
                                    millis(previous.notification_reverse_ms),
                                    millis(self.settings.notification_reverse_ms),
                                )
                        };
                        self.notification
                            .retime(&updated.notification, elapsed, now);
                    }
                }
                Setting::Stagger => {
                    for (index, item) in self.items.iter_mut().enumerate() {
                        if item.advance(&self.motions.items[index], now) {
                            let old_delay = millis(120 + index as u64 * previous.stagger_gap_ms);
                            let new_delay =
                                millis(120 + index as u64 * self.settings.stagger_gap_ms);
                            let elapsed = if item.elapsed < old_delay {
                                scale_elapsed(item.elapsed, old_delay, new_delay)
                            } else {
                                new_delay + (item.elapsed - old_delay)
                            };
                            item.retime(&updated.items[index], elapsed, now);
                        }
                    }
                }
                Setting::Attention => {
                    if self.attention.advance(&self.motions.attention, now) {
                        // Reducing the count never cuts off the current pass.
                        let current_pass =
                            (self.attention.elapsed.as_nanos() / millis(500).as_nanos()) as u32;
                        self.attention_run_count =
                            self.settings.attention_count.max(current_pass + 1);
                        updated.attention = Motion::new(millis(500))
                            .iterations(self.attention_run_count)
                            .with_easing(attention_nudge);
                        self.attention
                            .retime(&updated.attention, self.attention.elapsed, now);
                    }
                }
                Setting::Pulse => {
                    if self.pulse.advance(&self.motions.pulse, now) {
                        let elapsed = scale_elapsed(
                            self.pulse.elapsed,
                            millis(previous.pulse_pass_ms),
                            millis(self.settings.pulse_pass_ms),
                        );
                        self.pulse.retime(&updated.pulse, elapsed, now);
                    }
                }
            }

            if self.attention.is_active() {
                updated.attention = Motion::new(millis(500))
                    .iterations(self.attention_run_count)
                    .with_easing(attention_nudge);
            }
            self.motions = updated;
            cx.notify();
        }
    }
}

fn ease_out_cubic(progress: f32) -> f32 {
    1. - (1. - progress).powi(3)
}

fn attention_nudge(progress: f32) -> f32 {
    (std::f32::consts::PI * progress).sin().max(0.)
}

fn hold_then_exit(progress: f32) -> f32 {
    let exit = ((progress - 0.6) / 0.4).clamp(0., 1.);
    exit * exit * (3. - 2. * exit)
}

fn button(id: &'static str, label: &'static str, primary: bool) -> gpui::Stateful<gpui::Div> {
    let fill = if primary { ACCENT } else { SURFACE };
    div()
        .id(id)
        .px(px(14.))
        .py(px(9.))
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(if primary { ACCENT } else { BORDER }))
        .bg(rgb(fill))
        .text_color(rgb(if primary { SURFACE } else { TEXT }))
        .cursor_pointer()
        .hover_style(move |style| style.bg(rgb(if primary { 0xc2d5ff } else { 0x2a3850 })))
        .child(label)
}

fn stepper_button(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    button(id, label, false)
        .w(px(32.))
        .px(px(0.))
        .py(px(6.))
        .flex()
        .justify_center()
}

fn panel(title: &'static str, api: &'static str, content: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(16.))
        .rounded(px(12.))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(content)
        .child(div().text_xs().text_color(rgb(ACCENT)).child(api))
}

fn setting_stepper(
    cx: &mut Context<MotionShowcase>,
    setting: Setting,
    label: &'static str,
    value: String,
    minus_id: &'static str,
    plus_id: &'static str,
) -> impl IntoElement + use<> {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .child(div().text_sm().text_color(rgb(MUTED)).child(label))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(stepper_button(minus_id, "−").on_click(
                    cx.listener(move |this, _, _, cx| this.change_setting(setting, -1, cx)),
                ))
                .child(div().w(px(72.)).text_center().text_sm().child(value))
                .child(stepper_button(plus_id, "+").on_click(
                    cx.listener(move |this, _, _, cx| this.change_setting(setting, 1, cx)),
                )),
        )
}

impl Render for MotionShowcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = f32::from(window.viewport_size().width) < 700.;
        let now = Instant::now();
        let mut active = self.progress.advance(&self.motions.progress, now);
        active |= self.notification.advance(&self.motions.notification, now);
        for (index, item) in self.items.iter_mut().enumerate() {
            active |= item.advance(&self.motions.items[index], now);
        }
        active |= self.attention.advance(&self.motions.attention, now);
        active |= self.pulse.advance(&self.motions.pulse, now);
        if active {
            window.request_animation_frame();
        }

        let progress_value =
            self.progress_from + (self.progress_to - self.progress_from) * self.progress.value;
        let notification_value = self.notification.value;
        let notification_sample = self.notification.last_sample.map_or_else(
            || "Press Play to inspect the sample".to_string(),
            |sample| {
                format!(
                    "Pass {} · {:?} · local {:.2} · {:?} · at end: {} · complete: {}",
                    sample.iteration + 1,
                    sample.direction,
                    sample.local_time.get(),
                    sample.phase,
                    sample.is_at_end(),
                    sample.is_complete(),
                )
            },
        );
        let item_values: [f32; 3] = std::array::from_fn(|index| self.items[index].value);
        let attention_value = self.attention.value;
        let pulse_value = self.pulse.value;

        let progress_stepper = setting_stepper(
            cx,
            Setting::Progress,
            "Duration",
            format!("{} ms", self.settings.progress_ms),
            "progress-less",
            "progress-more",
        );
        let notification_stepper = setting_stepper(
            cx,
            Setting::Notification,
            "Reverse pass",
            format!("{} ms", self.settings.notification_reverse_ms),
            "notification-less",
            "notification-more",
        );
        let stagger_stepper = setting_stepper(
            cx,
            Setting::Stagger,
            "Stagger gap",
            format!("{} ms", self.settings.stagger_gap_ms),
            "stagger-less",
            "stagger-more",
        );
        let attention_stepper = setting_stepper(
            cx,
            Setting::Attention,
            "Nudges",
            format!("{}×", self.settings.attention_count),
            "attention-less",
            "attention-more",
        );
        let pulse_stepper = setting_stepper(
            cx,
            Setting::Pulse,
            "Pass duration",
            format!("{} ms", self.settings.pulse_pass_ms),
            "pulse-less",
            "pulse-more",
        );

        let progress_played = self.progress_played;
        let pulse_running = self.pulse.is_active();

        let progress_panel = panel(
            "Progress",
            "Motion::sample(...) — retarget from the current value",
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .text_sm()
                        .text_color(rgb(MUTED))
                        .child("Task progress")
                        .child(format!("{:.0}%", progress_value * 100.)),
                )
                .child(
                    div()
                        .h(px(18.))
                        .w_full()
                        .rounded(px(9.))
                        .overflow_hidden()
                        .bg(rgb(INSET))
                        .child(
                            div()
                                .h_full()
                                .w(relative(progress_value))
                                .rounded(px(9.))
                                .bg(rgb(ACCENT)),
                        ),
                )
                .child(progress_stepper)
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(8.))
                        .child(button("progress-play", "Play", true).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.progress_from = 0.;
                                this.progress_to = 1.;
                                this.progress_played = true;
                                this.progress.play(Instant::now());
                                cx.notify();
                            },
                        )))
                        .child(
                            button("progress-retarget", "Retarget", false)
                                .when(!progress_played, |button| {
                                    button.opacity(0.45).cursor_not_allowed()
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if !this.progress_played {
                                        return;
                                    }
                                    let now = Instant::now();
                                    this.progress.advance(&this.motions.progress, now);
                                    let current = this.progress_from
                                        + (this.progress_to - this.progress_from)
                                            * this.progress.value;
                                    this.progress_from = current;
                                    this.progress_to = if this.progress_to < 0.5 { 1. } else { 0. };
                                    this.progress.play(now);
                                    cx.notify();
                                })),
                        ),
                ),
        );

        let items_panel = panel(
            "Recent items",
            ".with_delay(...) — separate transitions",
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(div().flex().flex_col().gap(px(8.)).children(
                    item_values.into_iter().enumerate().map(|(index, value)| {
                        div()
                            .relative()
                            .top(px((1. - value) * 8.))
                            .opacity(0.5 + value * 0.5)
                            .p(px(10.))
                            .rounded(px(7.))
                            .bg(rgb(INSET))
                            .child(format!("ITEM {}", index + 1))
                    }),
                ))
                .child(stagger_stepper)
                .child(button("items-play", "Play", true).on_click(cx.listener(
                    move |this, _, _, cx| {
                        let now = Instant::now();
                        for item in &mut this.items {
                            item.play(now);
                        }
                        cx.notify();
                    },
                ))),
        );

        let status_panel = panel(
            "Status",
            ".repeat_forever().alternate()",
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(
                            div()
                                .size(px(20.))
                                .rounded(px(10.))
                                .bg(rgb(ACCENT))
                                .opacity(if pulse_running {
                                    0.35 + 0.65 * pulse_value
                                } else {
                                    0.35
                                }),
                        )
                        .child(if pulse_running { "Active" } else { "Idle" }),
                )
                .child(pulse_stepper)
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(button("pulse-play", "Play", true).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.pulse.play(Instant::now());
                                cx.notify();
                            },
                        )))
                        .child(button("pulse-stop", "Stop", false).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.pulse.stop();
                                cx.notify();
                            },
                        ))),
                ),
        );

        let attention_panel = panel(
            "Attention",
            ".iterations(n) — finite nudges",
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .relative()
                        .left(px(attention_value * 9.))
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(7.))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(INSET))
                        .text_color(rgb(ACCENT))
                        .child("ATTENTION"),
                )
                .child(attention_stepper)
                .child(button("attention-play", "Play", true).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.attention_run_count = this.settings.attention_count;
                        this.motions.attention = Motion::new(Duration::from_millis(500))
                            .iterations(this.attention_run_count)
                            .with_easing(attention_nudge);
                        this.attention.play(Instant::now());
                        cx.notify();
                    },
                ))),
        );

        let notification_panel = panel(
            "Notifications",
            ".with_forward_pass(...)\n.with_reverse_pass(...)\n.iterations(2).alternate()",
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .relative()
                        .h(px(46.))
                        .w_full()
                        .rounded(px(9.))
                        .bg(rgb(INSET))
                        .child(
                            div()
                                .absolute()
                                .left(px(12.))
                                .top(px(14.))
                                .opacity(1. - notification_value)
                                .text_sm()
                                .text_color(rgb(MUTED))
                                .child("No notifications"),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(px(12.))
                                .top(px(8. + (1. - notification_value) * 8.))
                                .opacity(notification_value)
                                .px(px(12.))
                                .py(px(7.))
                                .rounded(px(7.))
                                .border_1()
                                .border_color(rgb(BORDER))
                                .bg(rgb(SURFACE))
                                .child("NOTIFICATION"),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(notification_sample),
                )
                .child(notification_stepper)
                .child(
                    button("notification-play", "Play", true).on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.notification.play(Instant::now());
                            cx.notify();
                        },
                    )),
                ),
        );

        div()
            .id("motion-showcase")
            .size_full()
            .overflow_y_scroll()
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Motion showcase"),
            )
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .when(compact, |layout| layout.flex_col())
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(16.))
                            .child(progress_panel)
                            .child(items_panel),
                    )
                    .child(
                        div()
                            .when(compact, |rail| rail.w_full())
                            .when(!compact, |rail| rail.w(px(300.)))
                            .flex()
                            .flex_col()
                            .gap(px(16.))
                            .child(status_panel)
                            .child(attention_panel)
                            .child(notification_panel),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_pass_duration_preserves_an_active_pulse_phase() {
        let now = Instant::now();
        let old = Motion::new(millis(700))
            .repeat_forever()
            .alternate()
            .with_easing(ease_in_out);
        let new = Motion::new(millis(900))
            .repeat_forever()
            .alternate()
            .with_easing(ease_in_out);
        let mut pulse = Timeline::default();
        pulse.play(now);
        assert!(pulse.advance(&old, now + millis(1_750)));
        let before = pulse.value;

        let elapsed = scale_elapsed(pulse.elapsed, millis(700), millis(900));
        pulse.retime(&new, elapsed, now + millis(1_750));

        assert!(pulse.is_active());
        assert!((pulse.value - before).abs() < 0.000_001);
    }

    #[test]
    fn changing_reverse_duration_preserves_notification_phase() {
        let old = Motions::new(Settings::default());
        let new = Motions::new(Settings {
            notification_reverse_ms: 2_300,
            ..Settings::default()
        });
        let forward = millis(280);
        let old_elapsed = forward + millis(850);
        let new_elapsed = forward + scale_elapsed(millis(850), millis(1_700), millis(2_300));

        let before = old.notification.sample(old_elapsed);
        let after = new.notification.sample(new_elapsed);
        assert!(before.is_active && after.is_active);
        assert!((before.progress.get() - after.progress.get()).abs() < 0.000_001);
    }

    #[test]
    fn changing_stagger_preserves_delay_or_forward_phase() {
        let old_delay = millis(300);
        let new_delay = millis(420);
        let old = Motion::new(millis(420))
            .with_delay(old_delay)
            .with_easing(ease_out_cubic);
        let new = Motion::new(millis(420))
            .with_delay(new_delay)
            .with_easing(ease_out_cubic);

        for elapsed in [millis(150), millis(510)] {
            let remapped = if elapsed < old_delay {
                scale_elapsed(elapsed, old_delay, new_delay)
            } else {
                new_delay + (elapsed - old_delay)
            };
            let before = old.sample(elapsed);
            let after = new.sample(remapped);
            assert!(before.is_active && after.is_active);
            assert!((before.progress.get() - after.progress.get()).abs() < 0.000_001);
        }
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.), px(820.)), cx);
        cx.open_window(
            WindowOptions::new().window_bounds(Some(WindowBounds::Windowed(bounds))),
            |window, cx| {
                window.set_window_title("Motion showcase");
                cx.new(|_| MotionShowcase::new())
            },
        )
        .expect("Failed to open window");

        example_prelude::init_example(cx, "Motion showcase");
    });
}
