use crate::numeric::u64_to_f64;
use frame_core::types::EncodingStats;
use std::time::Duration;

#[derive(Default)]
pub(super) struct WorkerProgress {
    last_pass: Option<u8>,
    last_elapsed: Duration,
    pass_start: Duration,
}

impl WorkerProgress {
    pub(super) fn read(
        &mut self,
        line: &str,
        elapsed: impl FnOnce() -> Duration,
    ) -> Option<(f64, EncodingStats)> {
        // Codec diagnostics and older workers may also contain a human-readable time.
        let field = |name: &str| {
            line.split_ascii_whitespace()
                .filter_map(|word| word.split_once('='))
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value)
        };
        let seconds = |name: &str| {
            let us = field(name)?.parse::<u64>().ok()?;
            Some(Duration::from_micros(us).as_secs_f64())
        };
        let media_seconds = seconds("media_us")?;
        let work = seconds("work_us")?;
        let duration_seconds = seconds("duration_us").filter(|duration| *duration > 0.0);
        let pass = match field("pass")? {
            "0" => None,
            "1" => Some(1),
            "2" => Some(2),
            _ => return None,
        };
        let frame = field("video_frames")?.parse::<u64>().ok();
        let size_bytes = field("size_bytes")?.parse::<u64>().ok();
        let elapsed = elapsed();
        if pass == Some(2) && self.last_pass == Some(1) {
            self.pass_start = self.last_elapsed;
        }
        self.last_pass = pass;
        self.last_elapsed = elapsed;
        let elapsed_seconds = elapsed.as_secs_f64();
        let pass_seconds = elapsed.saturating_sub(self.pass_start).as_secs_f64();
        let fps = frame
            .filter(|frame| *frame > 0 && pass_seconds > 0.0)
            .map(|frame| u64_to_f64(frame) / pass_seconds);
        let speed =
            (media_seconds > 0.0 && pass_seconds > 0.0).then(|| media_seconds / pass_seconds);
        let bitrate_kbps = size_bytes
            .filter(|bytes| *bytes > 0 && media_seconds > 0.0)
            .map(|bytes| u64_to_f64(bytes) * 8.0 / 1000.0 / media_seconds);
        let progress =
            duration_seconds.map_or(0.0, |duration| (100.0 * work / duration).clamp(0.0, 100.0));
        let eta_seconds = duration_seconds
            .filter(|_| work > 0.0 && elapsed_seconds > 0.0)
            .map(|duration| elapsed_seconds * (duration / work - 1.0).max(0.0));

        Some((
            progress,
            EncodingStats {
                frame,
                fps,
                size_bytes,
                bitrate_kbps,
                speed,
                media_seconds,
                elapsed_seconds,
                eta_seconds,
                duration_seconds,
                pass,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_pass_local_rates_and_total_work_for_eta() {
        let mut stats = WorkerProgress::default();
        let read = |stats: &mut WorkerProgress, pass, work, media, frame, bytes, elapsed| {
            let line = format!(
                "frame={frame} time=00:00:00.0 media_us={media} work_us={work} duration_us=10000000 pass={pass} size_bytes={bytes} video_frames={frame}"
            );
            stats.read(&line, || Duration::from_secs(elapsed)).unwrap()
        };
        let (pct, first) = read(&mut stats, 1, 5_000_000, 10_000_000, 300, "N/A", 5);
        assert_eq!(
            (pct, first.fps, first.speed, first.eta_seconds),
            (50.0, Some(60.0), Some(2.0), Some(5.0))
        );
        assert_eq!(first.bitrate_kbps, None);
        let (pct, second) = read(&mut stats, 2, 7_500_000, 5_000_000, 150, "1000000", 10);
        assert_eq!(
            (pct, second.fps, second.speed, second.bitrate_kbps),
            (75.0, Some(30.0), Some(1.0), Some(1600.0))
        );
        assert!((second.elapsed_seconds - 10.0).abs() < f64::EPSILON);
        assert!((second.eta_seconds.unwrap() - 10.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn unavailable_samples_preserve_unknown_values_and_ignore_diagnostics() {
        let mut stats = WorkerProgress::default();
        assert!(stats.read("frame=32 time=00:00:01", || panic!()).is_none());
        let (pct, sample) = stats
            .read(
                "media_us=0 work_us=0 duration_us=0 pass=0 size_bytes=0 video_frames=N/A",
                || Duration::ZERO,
            )
            .unwrap();
        assert!(pct.abs() < f64::EPSILON);
        assert_eq!(
            sample,
            EncodingStats {
                size_bytes: Some(0),
                ..EncodingStats::default()
            }
        );
    }
}
