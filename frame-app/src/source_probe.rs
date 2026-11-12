use std::path::Path;
use std::time::Duration;

use frame_core::error::ConversionError;
use frame_core::probe::{ffprobe_json_args, parse_ffprobe_stdout};
use frame_core::types::ProbeMetadata;

use crate::bounded_process::{CommandSpec, ExecutionDomain, Limits, StopReason};

pub fn probe(
    executable: &str,
    path: &Path,
    cancelled: impl Fn() -> bool,
) -> Result<ProbeMetadata, ConversionError> {
    let mut command = CommandSpec::new(executable);
    #[cfg(windows)]
    {
        let (program, interpreter) =
            crate::bounded_process::resolve_windows_program(std::ffi::OsStr::new(executable))?;
        command.program = program;
        command.interpreter = interpreter;
    }
    command.args = ffprobe_json_args(&path.to_string_lossy())
        .into_iter()
        .map(Into::into)
        .collect();
    if let Some(argument) = command.args.last_mut() {
        path.as_os_str().clone_into(argument);
    }
    probe_command(
        &command,
        path,
        Limits {
            timeout: Duration::from_secs(15),
            cleanup_timeout: Duration::from_secs(1),
            stdout_bytes: 4 * 1024 * 1024,
            stderr_bytes: 64 * 1024,
        },
        cancelled,
    )
}

fn probe_command(
    command: &CommandSpec,
    path: &Path,
    limits: Limits,
    cancelled: impl Fn() -> bool,
) -> Result<ProbeMetadata, ConversionError> {
    static SOURCE_PROBES: ExecutionDomain = ExecutionDomain::new();
    let output = SOURCE_PROBES
        .run(command, &[], limits, &cancelled)
        .map_err(|error| {
            let reason = match &error.reason {
                StopReason::TimedOut => "source probe timed out; retry the conversion".to_string(),
                StopReason::Cancelled => "source probe cancelled".to_string(),
                StopReason::CleanupInProgress => {
                    "source probe cleanup is still in progress; retry shortly".to_string()
                }
                StopReason::Io(message) => format!("source probe could not run: {message}"),
            };
            ConversionError::Probe(format!("{reason} (cleanup: {:?})", error.cleanup))
        })?;
    if cancelled() {
        return Err(ConversionError::Probe("source probe cancelled".into()));
    }
    if !output.success {
        return Err(ConversionError::Probe(format!(
            "ffprobe exited with code {:?}: {}",
            output.code,
            String::from_utf8_lossy(&output.stderr.bytes).trim(),
        )));
    }
    if output.stdout.truncated {
        return Err(ConversionError::Probe(format!(
            "source metadata exceeded the {} byte limit",
            limits.stdout_bytes,
        )));
    }
    parse_ffprobe_stdout(
        &path.to_string_lossy(),
        String::from_utf8_lossy(&output.stdout.bytes),
    )
    .map_err(|error| ConversionError::Probe(format!("invalid source metadata: {error}")))
}

#[cfg(test)]
pub fn serial_test_probes() -> std::sync::MutexGuard<'static, ()> {
    static PROBES: std::sync::Mutex<()> = std::sync::Mutex::new(());
    PROBES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "frame-source-probe-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn fixture(mode: &str, marker: &Path) -> CommandSpec {
        let mut command = CommandSpec::new(std::env::current_exe().unwrap());
        command.args = [
            "--exact",
            "source_probe::tests::fake_probe",
            "--ignored",
            "--nocapture",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        command.env = vec![
            ("FRAME_SOURCE_PROBE_MODE".into(), mode.into()),
            (
                "FRAME_SOURCE_PROBE_MARKER".into(),
                marker.as_os_str().to_owned(),
            ),
        ];
        command
    }
    fn limits() -> Limits {
        Limits {
            timeout: Duration::from_millis(200),
            cleanup_timeout: Duration::from_secs(1),
            stdout_bytes: 2048,
            stderr_bytes: 1024,
        }
    }

    #[test]
    #[ignore = "native child fixture"]
    fn fake_probe() {
        let Ok(mode) = std::env::var("FRAME_SOURCE_PROBE_MODE") else {
            return;
        };
        std::fs::write(
            std::env::var_os("FRAME_SOURCE_PROBE_MARKER").unwrap(),
            std::process::id().to_string(),
        )
        .unwrap();
        if mode == "silent" {
            std::thread::sleep(Duration::from_secs(2));
        }
        if mode == "flood" || mode == "failure" {
            use std::io::Write;
            let bytes = vec![b'x'; 256 * 1024];
            std::io::stdout().write_all(&bytes).unwrap();
            std::io::stderr().write_all(&bytes).unwrap();
            if mode == "failure" {
                std::process::exit(7);
            }
        }
        println!("{{\"streams\":[],\"format\":{{}}}}");
        std::process::exit(0);
    }

    #[test]
    fn cancellation_is_observed_before_probe_spawn() {
        let _serial = serial_test_probes();
        let directory = Directory::new();
        let marker = directory.0.join("started");
        let result = probe_command(
            &fixture("empty", &marker),
            Path::new("source.mp4"),
            limits(),
            || true,
        );
        assert!(result.is_err());
        assert!(
            !marker.exists(),
            "pre-cancelled probe must not launch a child"
        );
    }

    #[test]
    fn silent_probe_has_a_deadline_and_releases_its_process_budget() {
        let _serial = serial_test_probes();
        let directory = Directory::new();
        let marker = directory.0.join("started");
        let started = Instant::now();
        let error = probe_command(
            &fixture("silent", &marker),
            Path::new("source.mp4"),
            limits(),
            || false,
        )
        .unwrap_err();
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "silent probe exceeded its deadline"
        );
        assert!(error.to_string().contains("timed out"), "{error}");
        let error = probe_command(
            &fixture("empty", &marker),
            Path::new("source.mp4"),
            limits(),
            || false,
        )
        .unwrap_err();
        assert!(
            !error.to_string().contains("timed out"),
            "next probe could not acquire the budget: {error}"
        );
    }
    #[test]
    fn running_probe_cancel_is_prompt_and_releases_the_budget() {
        let _serial = serial_test_probes();
        let directory = Directory::new();
        let marker = directory.0.join("started");
        let started = Instant::now();
        let error = probe_command(
            &fixture("silent", &marker),
            Path::new("source.mp4"),
            Limits {
                timeout: Duration::from_secs(3),
                ..limits()
            },
            || marker.exists(),
        )
        .unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(error.to_string().contains("cancelled"), "{error}");
        #[cfg(unix)]
        {
            let pid: i32 = std::fs::read_to_string(&marker).unwrap().parse().unwrap();
            // SAFETY: signal zero only queries our recorded child PID.
            assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        }
    }

    #[test]
    fn oversized_metadata_is_rejected_and_stderr_is_bounded() {
        let _serial = serial_test_probes();
        let directory = Directory::new();
        let marker = directory.0.join("started");
        let budget = Limits {
            timeout: Duration::from_secs(3),
            ..limits()
        };
        let overflow = probe_command(
            &fixture("flood", &marker),
            Path::new("source.mp4"),
            budget,
            || false,
        )
        .unwrap_err();
        assert!(
            overflow
                .to_string()
                .contains("exceeded the 2048 byte limit"),
            "{overflow}"
        );
        let failure = probe_command(
            &fixture("failure", &marker),
            Path::new("source.mp4"),
            budget,
            || false,
        )
        .unwrap_err();
        assert!(failure.to_string().contains("code Some(7)"), "{failure}");
        assert!(failure.to_string().len() < 2048);
    }

    #[test]
    #[ignore = "requires the selected FFprobe runtime"]
    fn real_probe_preserves_audio_metadata() {
        let _serial = serial_test_probes();
        let directory = Directory::new();
        let path = directory.0.join("probe fixture.wav");
        let sample_bytes = 960_u32;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + sample_bytes).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&48_000_u32.to_le_bytes());
        wav.extend_from_slice(&96_000_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&sample_bytes.to_le_bytes());
        wav.extend(vec![0; sample_bytes as usize]);
        std::fs::write(&path, wav).unwrap();
        let metadata = probe(
            &crate::runtime_binaries::ffprobe_executable(),
            &path,
            || false,
        )
        .unwrap();
        assert_eq!(metadata.media_kind, "audio");
        assert_eq!(metadata.audio_codec.as_deref(), Some("pcm_s16le"));
        assert_eq!(metadata.audio_tracks.len(), 1);
        assert_eq!(metadata.source_format.as_deref(), Some("wav"));
    }
}
