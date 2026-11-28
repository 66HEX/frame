use super::super::accessibility::{focus_ring_base, focus_visible_ring};
use crate::app::components::frame_progress_track;
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::file_queue::{FileItem, FileStatus, format_file_size};
use crate::theme;
use frame_core::types::EncodingStats;
use gpui::{Context, Div, Role, Window, div, prelude::*};
use std::fmt::Write as _;
use std::time::Duration;

struct DisplayStats {
    status: String,
    percent: Option<u8>,
    eta: Option<String>,
    metrics: Vec<(&'static str, String)>,
}

fn clock(seconds: f64) -> Option<String> {
    let duration = Duration::try_from_secs_f64(seconds).ok()?.as_secs();
    Some(if duration >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            duration / 3600,
            (duration / 60) % 60,
            duration % 60
        )
    } else {
        format!("{}:{:02}", duration / 60, duration % 60)
    })
}

impl DisplayStats {
    fn for_file(file: &FileItem, sample: Option<&EncodingStats>) -> Option<Self> {
        let state = match file.status {
            FileStatus::Converting => "Encoding",
            FileStatus::Paused => "Paused",
            FileStatus::Cancelling => "Cancelling",
            _ => return None,
        };
        let pass = sample
            .and_then(|s| s.pass)
            .filter(|pass| (1..=2).contains(pass));
        let status = pass.map_or_else(
            || state.into(),
            |pass| format!("{state} · Pass {pass} of 2"),
        );
        let mut display = Self {
            status,
            percent: (file.progress_percent > 0
                || sample
                    .and_then(|s| s.duration_seconds)
                    .is_some_and(|duration| duration.is_finite() && duration > 0.0))
            .then_some(file.progress_percent),
            eta: sample
                .filter(|_| file.status == FileStatus::Converting)
                .and_then(|s| clock(s.eta_seconds?)),
            metrics: Vec::new(),
        };
        if let Some(stats) = sample {
            if let Some(time) = clock(stats.media_seconds) {
                let time = stats
                    .duration_seconds
                    .filter(|&seconds| seconds > 0.0)
                    .and_then(clock)
                    .map_or_else(|| time.clone(), |duration| format!("{time} / {duration}"));
                display.metrics.push(("Media time", time));
            }
            if let Some(elapsed) = clock(stats.elapsed_seconds) {
                display.metrics.push(("Elapsed", elapsed));
            }
            if let Some(speed) = stats.speed.filter(|n| n.is_finite() && *n > 0.0) {
                display.metrics.push(("Speed", format!("{speed:.2}×")));
            }
            if let Some(size) = stats.size_bytes {
                display.metrics.push(("Size", format_file_size(size)));
            }
            if let Some(bitrate) = stats.bitrate_kbps.filter(|n| n.is_finite() && *n >= 0.0) {
                display
                    .metrics
                    .push(("Bitrate", format!("{bitrate:.0} kb/s")));
            }
            if let Some(frame) = stats.frame {
                display.metrics.push(("Frame", frame.to_string()));
            }
            if let Some(fps) = stats.fps.filter(|n| n.is_finite() && *n > 0.0) {
                display.metrics.push(("FPS", format!("{fps:.1}")));
            }
        }
        Some(display)
    }
}

pub(in crate::app) fn panel(
    file: &FileItem,
    sample: Option<&EncodingStats>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<Div> {
    let stats = DisplayStats::for_file(file, sample)?;
    let focus = window
        .use_keyed_state("encoding-stats-focus", cx, |_, cx| {
            cx.focus_handle().tab_stop(true)
        })
        .read(cx)
        .clone();
    let value = stats.percent.map_or_else(
        || "Progress unknown".to_owned(),
        |percent| format!("{percent}%"),
    );
    let mut bar = frame_progress_track(
        f32::from(stats.percent.unwrap_or_default()) / 100.0,
        palette,
    )
    .id("encoding-progress")
    .role(Role::ProgressIndicator)
    .aria_label("Encoding progress")
    .aria_value(value.clone());
    if let Some(percent) = stats.percent {
        bar = bar
            .aria_numeric_value(f64::from(percent))
            .aria_min_numeric_value(0.0)
            .aria_max_numeric_value(100.0);
    }

    let mut header = div()
        .flex()
        .items_center()
        .gap_2()
        .min_w_0()
        .child(
            div()
                .flex_shrink_0()
                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                .child(stats.status.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(color(palette.text_muted))
                .child(file.name.clone()),
        );
    if let Some(eta) = &stats.eta {
        header = header.child(div().flex_shrink_0().child(format!("ETA {eta}")));
    }
    header = header.child(
        div()
            .flex_shrink_0()
            .font_weight(theme::TEXT_WEIGHT_MEDIUM)
            .child(value.clone()),
    );

    let mut summary = format!("{}, {value}", stats.status);
    if let Some(eta) = stats.eta {
        let _ = write!(summary, ", ETA {eta}");
    }
    let mut metrics = div().flex().flex_wrap().items_center().gap_x_4().gap_y_1();
    let has_metrics = !stats.metrics.is_empty();
    for (label, value) in stats.metrics {
        let _ = write!(summary, ", {label} {value}");
        metrics = metrics.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().text_color(color(palette.text_muted)).child(label))
                .child(value),
        );
    }
    let group = focus_ring_base(div().id("encoding-stats"), palette)
        .role(Role::Group)
        .aria_label(format!("Encoding statistics for {}", file.name))
        .aria_description(summary)
        .track_focus(&focus)
        .focus_visible_style(move |style| focus_visible_ring(style, palette))
        .debug_selector(|| "encoding-stats".to_owned())
        .min_w_0()
        .flex()
        .flex_col()
        .gap_2()
        .px(theme::ui_rem(theme::metrics::CARD_PADDING))
        .py_2()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .text_color(color(palette.text_primary))
        .child(header)
        .child(bar)
        .when(has_metrics, |column| column.child(metrics));

    Some(
        div()
            .flex_shrink_0()
            .border_t_1()
            .border_color(color(palette.border_subtle))
            .child(group),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_passes_and_distinguishes_missing_audio_metrics_from_zero() {
        let mut file = FileItem::from_path("a", "/tmp/test.m4a", 0);
        file.status = FileStatus::Converting;
        let stats = EncodingStats {
            media_seconds: 64.5,
            elapsed_seconds: 3604.9,
            eta_seconds: Some(129.0),
            size_bytes: Some(0),
            duration_seconds: None,
            speed: Some(f64::NAN),
            ..Default::default()
        };
        let display = DisplayStats::for_file(&file, Some(&stats)).unwrap();
        assert_eq!(display.percent, None);
        assert_eq!(
            display.metrics,
            [
                ("Media time", "1:04".into()),
                ("Elapsed", "1:00:04".into()),
                ("Size", "0 B".into())
            ]
        );
        file.status = FileStatus::Paused;
        file.progress_percent = 30;
        let paused = DisplayStats::for_file(
            &file,
            Some(&EncodingStats {
                pass: Some(2),
                ..stats
            }),
        )
        .unwrap();
        assert_eq!(paused.status, "Paused · Pass 2 of 2");
        assert_eq!(paused.percent, Some(30));
        assert_eq!(paused.eta, None);
        file.status = FileStatus::Completed;
        assert!(DisplayStats::for_file(&file, None).is_none());
    }
}
