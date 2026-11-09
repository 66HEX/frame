#![cfg(unix)]

use super::*;
use crate::conversion_runner::core_config_from_gpui;
use crate::settings::{ConversionConfig, ProcessingMode};
use frame_core::args::build_alac_copy_observer;
use frame_core::types::{AudioTrack, ProbeMetadata};
use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    root: PathBuf,
    runtime: PathBuf,
    source: PathBuf,
    observer: AlacCopyObserver,
}

impl Fixture {
    fn new(duration: Option<i64>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "frame-alac-preflight-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let runtime = root.join("ffmpeg-fixture");
        fs::write(&runtime, b"#!/bin/sh\nparent=$(dirname \"$0\")\nprintf x >> \"$parent/calls\"\ncat \"$parent/manifest\"\nprintf 'progress=end\\n' > \"$2\"\n").unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
        let source = root.join("source.mka");
        fs::write(&source, b"immutable input identity").unwrap();
        let config = core_config_from_gpui(&ConversionConfig {
            container: "m4a".into(),
            processing_mode: ProcessingMode::Copy,
            selected_audio_tracks: vec![3],
            ..ConversionConfig::default()
        });
        let probe = ProbeMetadata {
            audio_tracks: vec![AudioTrack {
                index: 3,
                codec: "alac".into(),
                channels: "2".into(),
                ..AudioTrack::default()
            }],
            ..ProbeMetadata::default()
        };
        let observer = build_alac_copy_observer(
            source.to_str().unwrap(),
            root.join("out.m4a").to_str().unwrap(),
            &config,
            &probe,
        )
        .unwrap()
        .unwrap();
        let fixture = Self {
            root,
            runtime,
            source,
            observer,
        };
        fixture.write_manifest(duration);
        fixture
    }

    fn write_manifest(&self, duration: Option<i64>) {
        const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let packet = duration.map_or_else(String::new, |duration| {
            format!("0, 0, 0, {duration}, 128, {HASH}\n")
        });
        fs::write(self.root.join("manifest"), format!("#format: frame checksums\n#version: 2\n#hash: SHA256\n#software: Lavf62.12.102\n#extradata 0, 36, {HASH}\n#tb 0: 1/48000\n#media_type 0: audio\n#codec_id 0: alac\n#sample_rate 0: 48000\n#channel_layout_name 0: stereo\n#stream#, dts, pts, duration, size, hash\n{packet}")).unwrap();
    }

    fn run(&self, cancelled: &dyn Fn() -> bool, log: &mut dyn FnMut(String)) -> Preflight {
        prepare(
            &self.observer,
            &self.source,
            self.runtime.to_str().unwrap(),
            "/nonexistent/ffprobe",
            cancelled,
            log,
        )
    }

    fn calls(&self) -> usize {
        fs::read(self.root.join("calls")).map_or(0, |bytes| bytes.len())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn positive_and_empty_observations_keep_ordinary_arguments_and_do_not_need_ffprobe() {
    let _serial = crate::bounded_process::serial_test_processes();
    for duration in [Some(1024), None] {
        let fixture = Fixture::new(duration);
        let ordinary = fixture.observer.export_args(&[]).unwrap();
        let mut logs = Vec::new();
        let report = fixture.run(&|| false, &mut |line| logs.push(line));
        let Preparation::Ordinary(bindings) = report.result.unwrap() else {
            panic!("positive/empty must be ordinary")
        };
        assert!(
            bindings.ffmpeg.identity.is_none(),
            "shell script is intentionally uncacheable"
        );
        assert!(bindings.ffprobe.is_none());
        bindings.check(&|| false).unwrap();
        assert_eq!(fixture.observer.export_args(&[]).unwrap(), ordinary);
        assert_eq!(fixture.calls(), 1);
        assert_eq!(
            logs,
            [
                "[INFO] ALAC verification: observing copied packets",
                "[INFO] ALAC verification: copied packet observation complete"
            ]
        );
        assert!(report.diagnostics.is_empty());
        fs::write(&fixture.source, b"changed input after ordinary observation").unwrap();
        assert!(bindings.check(&|| false).is_err());
    }
}

#[test]
fn zero_tail_requires_native_repair_identity_and_unknown_never_becomes_ordinary() {
    let _serial = crate::bounded_process::serial_test_processes();
    let fixture = Fixture::new(Some(0));
    let report = fixture.run(&|| false, &mut |_| {});
    assert!(
        matches!(&report.result, Err(Failure::Invalid(detail)) if detail.contains("native FFmpeg"))
    );
    assert_eq!(fixture.calls(), 1);
    fs::write(fixture.root.join("manifest"), b"partial protocol\n").unwrap();
    assert!(matches!(
        fixture.run(&|| false, &mut |_| {}).result,
        Err(Failure::Invalid(_))
    ));
    assert_eq!(fixture.calls(), 2);
}

#[test]
fn checkpoint_mutations_cannot_authorize_the_next_phase_or_ordinary_export() {
    let _serial = crate::bounded_process::serial_test_processes();
    for after_observer in [false, true] {
        let fixture = Fixture::new(Some(1024));
        let expected = if after_observer {
            "copied packet observation complete"
        } else {
            "observing copied packets"
        };
        let report = fixture.run(&|| false, &mut |line| {
            if line.ends_with(expected) {
                fs::write(&fixture.source, b"replacement source content").unwrap();
            }
        });
        assert!(matches!(report.result, Err(Failure::Invalid(_))));
        assert_eq!(fixture.calls(), usize::from(after_observer));
    }
    let fixture = Fixture::new(Some(1024));
    let report = fixture.run(&|| false, &mut |line| {
        if line.ends_with("copied packet observation complete") {
            fs::write(&fixture.runtime, b"#!/bin/sh\nexit 42\n").unwrap();
        }
    });
    assert!(matches!(report.result, Err(Failure::Invalid(_))));
}

#[test]
fn cancellation_before_and_after_observation_stops_without_a_new_child() {
    let _serial = crate::bounded_process::serial_test_processes();
    for after_observer in [false, true] {
        let fixture = Fixture::new(Some(0));
        let cancelled = Cell::new(false);
        let expected = if after_observer {
            "copied packet observation complete"
        } else {
            "observing copied packets"
        };
        let report = fixture.run(&|| cancelled.get(), &mut |line| {
            if line.ends_with(expected) {
                cancelled.set(true);
            }
        });
        assert!(matches!(report.result, Err(Failure::Cancelled)));
        assert_eq!(fixture.calls(), usize::from(after_observer));
        assert!(report.diagnostics.is_empty());
    }
}
