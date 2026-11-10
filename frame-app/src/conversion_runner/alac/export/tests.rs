use super::*;
use crate::conversion_runner::core_config_from_gpui;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "frame-alac-finalize-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn task(&self) -> ConversionTask {
        ConversionTask {
            id: "alac-finalize".into(),
            file_path: "source.mka".into(),
            output_directory: self.0.to_string_lossy().into_owned(),
            output_name: Some("result.m4a".into()),
            config: core_config_from_gpui(&crate::settings::ConversionConfig::default()),
        }
    }

    fn stage(&self) -> StagedOutput {
        let mut stage = StagedOutput::create(&self.0.join("result.m4a")).unwrap();
        fs::write(stage.path(), b"verified candidate").unwrap();
        stage.seal().unwrap();
        stage
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn setup_cleanup_and_failure_logs_precede_unconditional_last_cancellation_consume() {
    let fixture = Fixture::new();
    let task = fixture.task();
    for (checkpoint, saved) in [
        ("[WARN] ALAC", false),
        ("[ERROR] ALAC", false),
        ("[INFO] Task", true),
    ] {
        let controller = ConversionProcessController::default();
        let mut events = Vec::new();
        let result = failed(
            &task,
            &controller,
            &mut |event| {
                if matches!(&event, ConversionEvent::Log(payload) if payload.line.starts_with(checkpoint))
                {
                    assert_eq!(controller.active_process_count().unwrap(), 0);
                    controller.cancel_task(&task.id).unwrap();
                }
                events.push(event);
            },
            &Failure::invalid("synthetic verification refusal"),
            saved,
            vec!["partial setup cleanup warning".into()],
        );
        result.unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, ConversionEvent::Cancelled(_)))
                .count(),
            1
        );
        assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
        assert!(!controller.take_cancelled(&task.id).unwrap());
        controller.begin_attempt(&task.id).unwrap();
        controller.cancel_task(&task.id).unwrap();
        assert!(controller.take_cancelled(&task.id).unwrap());
    }
}

#[test]
fn publication_cutoff_fixes_the_decision_before_cleanup_callbacks() {
    let fixture = Fixture::new();
    let task = fixture.task();
    let stage = fixture.stage();
    let directory = stage.path().parent().unwrap().to_path_buf();
    let extra = directory.join("unowned-extra");
    fs::write(&extra, b"leave this entry").unwrap();
    let controller = ConversionProcessController::default();
    let destination = fixture.0.join("result.m4a");
    let mut events = Vec::new();
    publish(&task, destination.to_str().unwrap(), stage, &controller, &mut |event| {
        if matches!(&event, ConversionEvent::Log(payload) if payload.line.starts_with("[WARN] ALAC verification cleanup:")) {
            assert_eq!(fs::read(&destination).unwrap(), b"verified candidate");
            controller.cancel_task(&task.id).unwrap();
        }
        events.push(event);
    }, false).unwrap();
    assert!(matches!(events.last(), Some(ConversionEvent::Completed(_))));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ConversionEvent::Cancelled(_)))
    );
    assert!(!controller.take_cancelled(&task.id).unwrap());
    assert_eq!(fs::read(extra).unwrap(), b"leave this entry");
}

#[test]
fn cancellation_winning_publication_cutoff_leaves_destination_absent_and_cleans_stage() {
    let fixture = Fixture::new();
    let task = fixture.task();
    let stage = fixture.stage();
    let stage_path = stage.path().to_path_buf();
    let controller = ConversionProcessController::default();
    controller.cancel_task(&task.id).unwrap();
    let mut events = Vec::new();
    publish(
        &task,
        fixture.0.join("result.m4a").to_str().unwrap(),
        stage,
        &controller,
        &mut |event| events.push(event),
        false,
    )
    .unwrap();
    assert!(matches!(events.last(), Some(ConversionEvent::Cancelled(_))));
    assert!(!fixture.0.join("result.m4a").exists());
    assert!(!stage_path.exists());
    assert!(!controller.take_cancelled(&task.id).unwrap());
}

#[test]
fn final_command_uses_observed_program_and_replaces_only_known_last_sink() {
    let executable = std::env::current_exe().unwrap();
    let runtime = Runtime::resolve(executable.to_str().unwrap()).unwrap();
    let args = ["-i", "-n", "-metadata", "title=-n", "-n", "original.m4a"].map(str::to_owned);
    let ordinary = command(&runtime, &args, Path::new("proof"), None).unwrap();
    assert_eq!(ordinary.get_program(), executable.as_os_str());
    let ordinary_args = ordinary
        .get_args()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let expected_args = args
        .iter()
        .map(std::ffi::OsString::from)
        .collect::<Vec<_>>();
    assert_eq!(&ordinary_args[2..], expected_args.as_slice());
    let stage = Path::new("private path/kept extension.m4a");
    let staged = command(&runtime, &args, Path::new("proof"), Some(stage)).unwrap();
    let staged_args = staged.get_args().map(ToOwned::to_owned).collect::<Vec<_>>();
    assert_eq!(
        &staged_args[..staged_args.len() - 1],
        &ordinary_args[..ordinary_args.len() - 1]
    );
    assert_eq!(staged_args.last().unwrap(), stage.as_os_str());
    assert!(
        command(
            &runtime,
            &["wrong-sink".into()],
            Path::new("proof"),
            Some(stage)
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn private_output_and_progress_paths_keep_non_utf8_bytes() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let runtime = Runtime::resolve("/bin/sh").unwrap();
    let stage = OsString::from_vec(b"/private-\xff/out.m4a".to_vec());
    let proof = OsString::from_vec(b"/proof-\xfe/progress".to_vec());
    let args = ["-n".into(), "out.m4a".into()];
    let command = command(&runtime, &args, Path::new(&proof), Some(Path::new(&stage))).unwrap();
    let args = command.get_args().collect::<Vec<_>>();
    assert_eq!(args[1], proof);
    assert_eq!(args.last().copied(), Some(stage.as_os_str()));
}

#[test]
fn running_callback_entry_is_not_adopted_when_no_child_was_started() {
    use crate::settings::ProcessingMode;
    use frame_core::alac_copy::ParsedCopyManifest;
    use frame_core::args::build_alac_copy_observer;
    use frame_core::types::{AudioTrack, ProbeMetadata};
    for cancel in [false, true] {
        let fixture = Fixture::new();
        let mut task = fixture.task();
        let source = fixture.0.join("source.mka");
        fs::write(&source, b"source identity").unwrap();
        task.file_path = source.to_str().unwrap().to_owned();
        task.config = core_config_from_gpui(&crate::settings::ConversionConfig {
            processing_mode: ProcessingMode::Copy,
            container: "m4a".into(),
            selected_audio_tracks: vec![0],
            ..crate::settings::ConversionConfig::default()
        });
        let destination = fixture.0.join("result.m4a");
        let probe = ProbeMetadata {
            audio_tracks: vec![AudioTrack {
                codec: "alac".into(),
                ..AudioTrack::default()
            }],
            ..ProbeMetadata::default()
        };
        let observer = build_alac_copy_observer(
            &task.file_path,
            destination.to_str().unwrap(),
            &task.config,
            &probe,
        )
        .unwrap()
        .unwrap();
        // This ownership test stops before spawn; it deliberately supplies no
        // semantic postflight proof and cannot authorize a successful export.
        let prepared = PreparedRepair {
            bindings: Bindings::new(
                &observer,
                std::env::current_exe().unwrap().to_str().unwrap(),
            )
            .unwrap(),
            args: observer.export_args(&[]).unwrap(),
            candidate: ParsedCopyManifest {
                streams: Vec::new(),
                packets: Vec::new(),
            },
            tracks: Vec::new(),
            captured_bytes: 0,
        };
        let controller = ConversionProcessController::default();
        let mut inserted = None;
        let mut events = Vec::new();
        let result = repaired(
            &task,
            destination.to_str().unwrap(),
            &prepared,
            &controller,
            &mut |event| {
                if matches!(&event, ConversionEvent::Log(payload) if payload.line.starts_with("[INFO] Running "))
                {
                    let directory = fs::read_dir(&fixture.0)
                        .unwrap()
                        .map(Result::unwrap)
                        .find(|entry| {
                            entry
                                .file_name()
                                .to_string_lossy()
                                .starts_with(".frame-alac-")
                        })
                        .unwrap();
                    let path = directory.path().join("result.m4a");
                    fs::write(&path, b"unowned callback entry").unwrap();
                    inserted = Some(path);
                    if cancel {
                        controller.cancel_task(&task.id).unwrap();
                    }
                }
                events.push(event);
            },
        );
        assert_eq!(result.is_ok(), cancel);
        assert_eq!(
            fs::read(inserted.unwrap()).unwrap(),
            b"unowned callback entry"
        );
        assert!(!destination.exists());
        assert!(!events.iter().any(|event| matches!(
            event,
            ConversionEvent::Started(_) | ConversionEvent::Completed(_)
        )));
        assert_eq!(
            events
                .iter()
                .any(|event| matches!(event, ConversionEvent::Cancelled(_))),
            cancel
        );
        assert_eq!(controller.active_process_count().unwrap(), 0);
        assert!(!controller.take_cancelled(&task.id).unwrap());
    }
}
