use crate::app::accessibility::{
    APP_ROOT_FOCUS_ID, FrameFocusKey, focus_ring_base, focus_visible_ring,
    handle_focus_scope_tab_navigation,
};
use crate::app::components::{
    frame_dialog_close_button, frame_dialog_header, frame_dialog_overlay, frame_dialog_panel,
    frame_progress_track,
};
use crate::app::motion::{DialogPresence, surface_motion};
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::file_queue::{FileItem, FileStatus, format_file_size};
use crate::theme;
use frame_core::events::ConversionEvent;
use frame_core::types::EncodingStats;
use gpui::{
    ClickEvent, Context, Div, FocusHandle, KeyDownEvent, Role, Stateful, WeakFocusHandle, Window,
    div, prelude::*,
};
use std::fmt::Write as _;
use std::time::Duration;

#[derive(Default)]
pub(in crate::app) struct EncodingStatsDialog {
    pub(in crate::app) presence: DialogPresence,
    job: Option<DisplayedJob>,
}

impl EncodingStatsDialog {
    pub(super) fn follows_event(&self, event: &ConversionEvent) -> bool {
        let Some(job) = &self.job else {
            return false;
        };
        let id = match event {
            ConversionEvent::Progress(p) => &p.id,
            ConversionEvent::Completed(p) => &p.id,
            ConversionEvent::Cancelled(p) => &p.id,
            ConversionEvent::Error(p) => &p.id,
            ConversionEvent::Started(_) | ConversionEvent::Log(_) => return false,
        };
        job.status.has_encoding_statistics() && id == &job.id
    }
}

struct DisplayedJob {
    id: String,
    row_index: usize,
    name: String,
    status: FileStatus,
    percent: u8,
    sample: Option<EncodingStats>,
    trigger: Option<WeakFocusHandle>,
    list: WeakFocusHandle,
}

impl DisplayedJob {
    fn refresh(&mut self, file: &FileItem, sample: Option<&EncodingStats>) {
        // A second export of the same file is a different job. An open dialog
        // keeps the completed, failed or cancelled attempt until dismissed.
        if self.status.has_encoding_statistics() {
            self.status = file.status;
            if self.status.has_encoding_statistics() || self.status == FileStatus::Completed {
                self.percent = file.progress_percent;
            }
            if let Some(sample) = sample {
                self.sample = Some(sample.clone());
            }
        }
    }
}

impl FrameRoot {
    pub(in crate::app) fn open_encoding_stats(
        &mut self,
        id: &str,
        list: &FocusHandle,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(row_index) = self
            .file_queue
            .files()
            .iter()
            .position(|file| file.id == id)
        else {
            return;
        };
        let file = &self.file_queue.files()[row_index];
        if !file.status.has_encoding_statistics() {
            return;
        }
        self.encoding_stats_dialog.job = Some(DisplayedJob {
            id: id.to_owned(),
            row_index,
            name: file.name.clone(),
            status: file.status,
            percent: file.progress_percent,
            sample: self.conversion_events.encoding_stats_for(id).cloned(),
            trigger: window.focused(cx).map(|handle| handle.downgrade()),
            list: list.downgrade(),
        });
        self.encoding_stats_dialog.presence.open();
        cx.notify();
    }

    pub(in crate::app) fn refresh_encoding_stats_dialog(&mut self) {
        let Some(job) = self.encoding_stats_dialog.job.as_mut() else {
            return;
        };
        let rows = self.file_queue.files();
        // Virtualized queues can contain thousands of jobs. Reuse this position
        // on each redraw and re-resolve the stable ID only if rows moved.
        let file = if let Some(file) = rows.get(job.row_index).filter(|f| f.id == job.id) {
            file
        } else if let Some((index, file)) = rows.iter().enumerate().find(|(_, f)| f.id == job.id) {
            job.row_index = index;
            file
        } else {
            self.encoding_stats_dialog.presence.close();
            return;
        };
        job.refresh(file, self.conversion_events.encoding_stats_for(&job.id));
    }

    fn close_encoding_stats(&mut self, cx: &mut Context<Self>) {
        if self.encoding_stats_dialog.presence.close() {
            cx.notify();
        }
    }

    pub(in crate::app) fn render_encoding_stats_overlay(
        &mut self,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        if !self.encoding_stats_dialog.presence.is_present() {
            return None;
        }
        self.refresh_encoding_stats_dialog();
        let open = self.encoding_stats_dialog.presence.is_open();
        let progress = surface_motion("encoding-stats-motion", open, window, cx);
        if self.encoding_stats_dialog.presence.finish_close(progress) {
            if let Some(job) = self.encoding_stats_dialog.job.take() {
                if let Some(list) = job
                    .list
                    .upgrade()
                    .filter(|_| !self.file_queue.files().is_empty())
                {
                    let focus = job
                        .trigger
                        .and_then(|handle| handle.upgrade())
                        .filter(|handle| {
                            self.file_queue
                                .file_by_id(&job.id)
                                .is_some_and(|f| f.status.has_encoding_statistics())
                                && list.contains(handle, window)
                        });
                    focus.as_ref().unwrap_or(&list).focus(window, cx);
                } else {
                    self.focus_registered_control(APP_ROOT_FOCUS_ID, window, cx);
                }
            }
            cx.notify();
            return None;
        }
        let scope = self.ensure_focus(
            FrameFocusKey::Control("encoding-stats-dialog".into()),
            false,
            cx,
        );
        let content = self.ensure_focus(FrameFocusKey::Control("encoding-stats".into()), open, cx);
        let close = self.ensure_focus(
            FrameFocusKey::Control("encoding-stats-close".into()),
            open,
            cx,
        );
        if open && !scope.contains_focused(window, cx) {
            close.focus(window, cx);
        }
        let job = self.encoding_stats_dialog.job.as_ref()?;
        let title = "Encoding statistics";
        let panel = frame_dialog_panel(
            "encoding-stats-dialog",
            title,
            Some(&job.name),
            &scope,
            palette,
            window,
        )
        .child(
            frame_dialog_header(title, None, palette).child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(color(palette.text_muted))
                    .child(job.name.clone()),
            ),
        )
        .child(metrics_group(job, &content, palette))
        .child(
            frame_dialog_close_button(
                "encoding-stats-close",
                "Close encoding statistics",
                open,
                &close,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _, cx| root.close_encoding_stats(cx))),
        );
        Some(
            frame_dialog_overlay(
                "encoding-stats-overlay",
                panel,
                progress,
                open,
                palette,
                window,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _, cx| root.close_encoding_stats(cx)))
            .on_key_down(cx.listener(move |root, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => root.close_encoding_stats(cx),
                    "tab" => {
                        handle_focus_scope_tab_navigation(event, &scope, window, cx);
                    }
                    _ => {}
                }
                cx.stop_propagation();
            })),
        )
    }
}

fn clock(seconds: f64) -> Option<String> {
    let duration = Duration::try_from_secs_f64(seconds).ok()?.as_secs();
    Some(if duration >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            duration / 3600,
            duration / 60 % 60,
            duration % 60
        )
    } else {
        format!("{}:{:02}", duration / 60, duration % 60)
    })
}

struct DisplayStats {
    status: String,
    percent: Option<u8>,
    metrics: [(&'static str, Option<String>); 8],
}

impl DisplayStats {
    fn new(status: FileStatus, progress: u8, sample: Option<&EncodingStats>) -> Self {
        let state = match status {
            FileStatus::Converting => "Encoding",
            FileStatus::Completed => "Completed",
            FileStatus::Idle => "Cancelled",
            FileStatus::Error => "Failed",
            other => other.label(),
        };
        let pass = sample
            .and_then(|s| s.pass)
            .filter(|pass| (1..=2).contains(pass) && status.has_encoding_statistics());
        let status_text = pass.map_or_else(
            || state.into(),
            |pass| format!("{state} · Pass {pass} of 2"),
        );
        let duration = sample
            .and_then(|s| s.duration_seconds)
            .filter(|n| n.is_finite() && *n > 0.0);
        let media = sample.and_then(|s| clock(s.media_seconds)).map(|time| {
            match duration.and_then(clock) {
                Some(end) => format!("{time} / {end}"),
                None => time,
            }
        });
        Self {
            status: status_text,
            percent: (status == FileStatus::Completed || progress > 0 || duration.is_some())
                .then_some(progress),
            metrics: [
                (
                    "ETA",
                    sample
                        .filter(|_| status == FileStatus::Converting)
                        .and_then(|s| clock(s.eta_seconds?)),
                ),
                ("Elapsed", sample.and_then(|s| clock(s.elapsed_seconds))),
                ("Media time", media),
                (
                    "Speed",
                    sample
                        .and_then(|s| s.speed)
                        .filter(|n| n.is_finite() && *n > 0.)
                        .map(|n| format!("{n:.2}×")),
                ),
                (
                    "Size",
                    sample.and_then(|s| s.size_bytes).map(format_file_size),
                ),
                (
                    "Bitrate",
                    sample
                        .and_then(|s| s.bitrate_kbps)
                        .filter(|n| n.is_finite() && *n >= 0.)
                        .map(|n| format!("{n:.0} kb/s")),
                ),
                ("Frame", sample.and_then(|s| s.frame).map(|n| n.to_string())),
                (
                    "FPS",
                    sample
                        .and_then(|s| s.fps)
                        .filter(|n| n.is_finite() && *n > 0.)
                        .map(|n| format!("{n:.1}")),
                ),
            ],
        }
    }
}

fn metrics_group(
    job: &DisplayedJob,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
) -> Stateful<Div> {
    let stats = DisplayStats::new(job.status, job.percent, job.sample.as_ref());
    let value = stats.percent.map_or_else(
        || "Progress unknown".to_owned(),
        |percent| format!("{percent}%"),
    );
    let mut bar = frame_progress_track(
        f32::from(stats.percent.unwrap_or_default()) / 100.0,
        palette,
    )
    .id("encoding-progress")
    .debug_selector(|| "encoding-progress".to_string())
    .role(Role::ProgressIndicator)
    .aria_label("Encoding progress")
    .aria_value(value.clone());
    if let Some(percent) = stats.percent {
        bar = bar
            .aria_numeric_value(f64::from(percent))
            .aria_min_numeric_value(0.0)
            .aria_max_numeric_value(100.0);
    }
    let mut summary = format!("{}, {value}", stats.status);
    let header = div()
        .flex()
        .justify_between()
        .items_center()
        .gap_2()
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .child(
            div()
                .id("encoding-stats-status")
                .role(Role::Status)
                .aria_live(gpui::accesskit::Live::Polite)
                .child(stats.status),
        )
        .child(value);
    let mut rows = div().flex().flex_col().w_full().gap_1();
    for (label, reading) in stats.metrics {
        if let Some(reading) = &reading {
            let _ = write!(summary, ", {label} {reading}");
        }
        rows = rows.child(
            div()
                .min_w_0()
                .flex()
                .gap_2()
                .justify_between()
                .items_center()
                .child(div().text_color(color(palette.text_muted)).child(label))
                .child(
                    div()
                        .text_right()
                        .child(reading.unwrap_or_else(|| "—".into())),
                ),
        );
    }
    focus_ring_base(div().id("encoding-stats"), palette)
        .debug_selector(|| "encoding-stats".to_owned())
        .role(Role::Group)
        .aria_label(format!("Encoding statistics for {}", job.name))
        .aria_description(summary)
        .track_focus(focus)
        .focus_visible_style(move |style| focus_visible_ring(style, palette))
        .min_w_0()
        .flex()
        .flex_col()
        .gap_2()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .text_color(color(palette.text_primary))
        .font_features(crate::assets::frame_tabular_number_font_features())
        .child(header)
        .child(bar)
        .child(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_values_keep_their_rows_and_terminal_state_keeps_measured_values() {
        let sample = EncodingStats {
            media_seconds: 64.5,
            elapsed_seconds: 3604.9,
            eta_seconds: Some(129.),
            size_bytes: Some(0),
            speed: Some(f64::NAN),
            pass: Some(2),
            ..Default::default()
        };
        let active = DisplayStats::new(FileStatus::Converting, 0, Some(&sample));
        let complete = DisplayStats::new(FileStatus::Completed, 100, Some(&sample));
        assert_eq!(active.status, "Encoding · Pass 2 of 2");
        assert_eq!(complete.status, "Completed");
        assert_eq!(active.metrics[0].1.as_deref(), Some("2:09"));
        assert_eq!(complete.metrics[0].1, None);
        assert_eq!(complete.metrics[1].1.as_deref(), Some("1:00:04"));
        assert_eq!(complete.metrics[2].1.as_deref(), Some("1:04"));
        assert_eq!(complete.metrics[4].1.as_deref(), Some("0 B"));
        assert_eq!(complete.metrics[6].1, None);
        assert_eq!(complete.percent, Some(100));
        assert_eq!(active.percent, None);
    }
}
