use super::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

mod runtime;

fn serial_export_processes() -> std::sync::MutexGuard<'static, ()> {
    static EXPORT_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());
    EXPORT_FIXTURES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) struct Directory(pub(super) PathBuf);

impl Directory {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "frame-source-export-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn request(folder: &Path) -> SourceFrameExportRequest {
    let source = folder.join("source clip.mov");
    if !source.exists() {
        fs::write(&source, b"source fixture").unwrap();
    }
    SourceFrameExportRequest::new("source-id".into(), source, 40_001, 7, folder.to_path_buf())
        .unwrap()
}

#[test]
fn request_freezes_source_time_stream_and_destination_without_conversion_config() {
    let folder = Directory::new();
    let request = request(&folder.0);
    assert_eq!(request.source_id, "source-id");
    assert_eq!(request.source_path, folder.0.join("source clip.mov"));
    assert_eq!(request.position_microseconds, 40_001);
    assert_eq!(request.video_stream_index, 7);
    assert_eq!(
        request.output_path,
        folder.0.join("source clip_frame_40001.png")
    );
    assert!(!request.output_path.exists());
}

#[test]
fn request_uses_a_new_attempt_id_and_avoids_existing_entries() {
    let folder = Directory::new();
    let first = request(&folder.0);
    fs::write(&first.output_path, b"keep existing").unwrap();
    let second = request(&folder.0);
    assert_ne!(first.id, second.id);
    assert_eq!(
        second.output_path,
        folder.0.join("source clip_frame_40001_2.png")
    );
    assert_eq!(fs::read(&first.output_path).unwrap(), b"keep existing");
}

fn png_bytes() -> Vec<u8> {
    let pixels = image::RgbaImage::from_pixel(3, 2, image::Rgba([30, 60, 90, 120]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    pixels
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

#[test]
fn png_validation_rejects_empty_invalid_and_incomplete_images() {
    let complete = png_bytes();
    let mut invalid_crc = complete.clone();
    *invalid_crc.last_mut().unwrap() ^= 1;
    let mut trailing = complete.clone();
    trailing.extend_from_slice(b"unexpected trailing payload");
    for invalid in [
        &[][..],
        b"not an image",
        &complete[..complete.len() / 2],
        &complete[..complete.len() - 12],
        &invalid_crc,
        &trailing,
    ] {
        assert!(
            validation::validate_png(invalid, 1024).is_err(),
            "invalid PNG was accepted"
        );
    }
}

#[test]
fn png_validation_checks_decoded_memory_before_pixel_allocation() {
    let complete = png_bytes();
    assert!(
        validation::validate_png(&complete, 23).is_err(),
        "3x2 RGBA exceeds 23 decoded bytes"
    );
    validation::validate_png(&complete, 1024 * 1024).unwrap();
}

#[test]
fn source_frame_export_rejects_flatpak_before_process_or_stage_work() {
    assert!(check_environment(true).is_err());
    check_environment(false).unwrap();
}

#[test]
fn pre_cancelled_source_frame_export_is_cancelled_without_creating_output() {
    let folder = Directory::new();
    let request = SourceFrameExportRequest::for_test(
        "pre-cancel".into(),
        "source-id".into(),
        "missing-source.mov".into(),
        0,
        0,
        folder.0.join("frame.png"),
    );
    let controller = ConversionProcessController::default();
    controller
        .begin_queued_attempts(std::slice::from_ref(&request.id))
        .unwrap();
    controller.cancel_task(&request.id).unwrap();
    assert_eq!(
        export_source_frame(&request, &controller).unwrap(),
        SourceFrameExportOutcome::Cancelled
    );
    assert!(!request.output_path.exists());
    assert!(!controller.is_cancelled(&request.id));
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 0);
}

fn child_command(mode: &str, marker: &Path) -> CommandSpec {
    let mut command = CommandSpec::new(std::env::current_exe().unwrap());
    command.args = [
        "--exact",
        "frame_export::tests::fake_export",
        "--ignored",
        "--nocapture",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    command.env = vec![
        ("FRAME_EXPORT_MODE".into(), mode.into()),
        ("FRAME_EXPORT_MARKER".into(), marker.as_os_str().to_owned()),
    ];
    command
}

fn test_request(folder: &Path, id: &str) -> SourceFrameExportRequest {
    let mut request = request(folder);
    request.id = id.into();
    request.output_path = folder.join("frame.png");
    request
}

#[test]
#[ignore = "native source frame export fixture"]
fn fake_export() {
    use std::io::Write;
    let Ok(mode) = std::env::var("FRAME_EXPORT_MODE") else {
        return;
    };
    fs::write(
        std::env::var_os("FRAME_EXPORT_MARKER").unwrap(),
        std::process::id().to_string(),
    )
    .unwrap();
    match mode.as_str() {
        "silent" => std::thread::sleep(Duration::from_secs(3)),
        "flood" => std::io::stdout().write_all(&vec![37; 1024 * 1024]).unwrap(),
        _ => (),
    }
    std::process::exit(0);
}

#[test]
fn running_source_frame_export_observes_cancel_and_reaps_the_child() {
    let _serial = serial_export_processes();
    let folder = Directory::new();
    let request = test_request(&folder.0, "running-cancel");
    let marker = folder.0.join("started");
    let controller = ConversionProcessController::default();
    controller
        .begin_queued_attempts(std::slice::from_ref(&request.id))
        .unwrap();
    let cancellation = controller.clone();
    let watched = marker.clone();
    let id = request.id.clone();
    let cancel = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !watched.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        cancellation.cancel_task(&id).unwrap();
    });
    let started = Instant::now();
    let result = export_with_command(
        &request,
        &controller,
        &child_command("silent", &marker),
        Limits {
            timeout: Duration::from_secs(5),
            ..Limits::default()
        },
    );
    cancel.join().unwrap();
    assert!(
        marker.exists(),
        "cancellation must interrupt a child that actually started"
    );
    assert_eq!(result.unwrap(), SourceFrameExportOutcome::Cancelled);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!controller.is_cancelled(&request.id));
    assert!(!request.output_path.exists());
    #[cfg(unix)]
    {
        let pid: i32 = fs::read_to_string(marker).unwrap().parse().unwrap();
        // SAFETY: signal zero only queries the recorded fixture child.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    }
}

#[test]
fn source_frame_timeout_and_truncation_never_create_a_stage_or_result() {
    let _serial = serial_export_processes();
    let folder = Directory::new();
    let request = test_request(&folder.0, "bounded-output");
    let marker = folder.0.join("started");
    for (mode, timeout, expected) in [
        ("silent", Duration::from_millis(200), "timed out"),
        ("flood", Duration::from_secs(3), "limit"),
    ] {
        let controller = ConversionProcessController::default();
        controller
            .begin_queued_attempts(std::slice::from_ref(&request.id))
            .unwrap();
        let result = export_with_command(
            &request,
            &controller,
            &child_command(mode, &marker),
            Limits {
                timeout,
                stdout_bytes: 4096,
                ..Limits::default()
            },
        )
        .unwrap_err();
        assert!(result.to_string().contains(expected), "{mode}: {result}");
        assert!(marker.exists());
        assert!(!request.output_path.exists());
        assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 2);
    }
}

#[test]
fn source_frame_publication_saves_the_png_and_late_cancel_preserves_success() {
    let folder = Directory::new();
    let request = request(&folder.0);
    let controller = ConversionProcessController::default();
    controller
        .begin_queued_attempts(std::slice::from_ref(&request.id))
        .unwrap();
    let png = png_bytes();
    let result = publication::publish_png(&request, &controller, &png, || {}).unwrap();
    assert_eq!(
        result,
        SourceFrameExportOutcome::Published {
            path: request.output_path.clone()
        }
    );
    controller.cancel_task(&request.id).unwrap();
    assert!(!controller.is_cancelled(&request.id));
    assert_eq!(fs::read(&request.output_path).unwrap(), png);
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 2);
}

#[test]
fn cancellation_before_source_frame_publication_cleans_the_private_stage() {
    let folder = Directory::new();
    let request = request(&folder.0);
    let controller = ConversionProcessController::default();
    controller
        .begin_queued_attempts(std::slice::from_ref(&request.id))
        .unwrap();
    let result = publication::publish_png(&request, &controller, &png_bytes(), || {
        controller.cancel_task(&request.id).unwrap();
    })
    .unwrap();
    assert_eq!(result, SourceFrameExportOutcome::Cancelled);
    assert!(!request.output_path.exists());
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 1);
}

#[test]
fn source_frame_destination_collision_at_publication_preserves_existing_bytes() {
    let folder = Directory::new();
    let request = request(&folder.0);
    let controller = ConversionProcessController::default();
    controller
        .begin_queued_attempts(std::slice::from_ref(&request.id))
        .unwrap();
    let result = publication::publish_png(&request, &controller, &png_bytes(), || {
        fs::write(&request.output_path, b"keep this file").unwrap();
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&request.output_path).unwrap(), b"keep this file");
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 2);
}

#[test]
fn source_frame_identity_rejects_modified_or_replaced_input() {
    let folder = Directory::new();
    let source = folder.0.join("source.mov");
    fs::write(&source, b"original source").unwrap();
    let captured = source_identity::SourceIdentity::capture(&source).unwrap();
    captured.ensure_unchanged().unwrap();
    fs::write(&source, b"modified source with different size").unwrap();
    assert!(captured.ensure_unchanged().is_err());

    let captured = source_identity::SourceIdentity::capture(&source).unwrap();
    let retained = captured.clone();
    drop(captured);
    fs::remove_file(&source).unwrap();
    fs::write(&source, b"modified source with different size").unwrap();
    assert!(retained.ensure_unchanged().is_err());
}

#[test]
fn source_frame_request_rejects_missing_and_non_file_sources() {
    let folder = Directory::new();
    for source in [folder.0.join("missing.mov"), folder.0.clone()] {
        assert!(
            SourceFrameExportRequest::new("source".into(), source, 0, 0, folder.0.clone()).is_err()
        );
    }
}

#[test]
fn changed_source_is_rejected_before_decode_starts() {
    let _serial = serial_export_processes();
    let folder = Directory::new();
    let request = request(&folder.0);
    fs::write(&request.source_path, b"a different input").unwrap();
    let marker = folder.0.join("started");
    let controller = ConversionProcessController::default();
    controller
        .begin_queued_attempts(std::slice::from_ref(&request.id))
        .unwrap();
    let error = export_with_command(
        &request,
        &controller,
        &child_command("silent", &marker),
        Limits {
            timeout: Duration::from_millis(100),
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("source changed"), "{error}");
    assert!(
        !marker.exists(),
        "a stale accepted request must not start decoding"
    );
    assert!(!request.output_path.exists());
}

#[test]
fn changed_source_before_publication_cleans_the_private_stage() {
    let folder = Directory::new();
    let request = request(&folder.0);
    let controller = ConversionProcessController::default();
    controller
        .begin_queued_attempts(std::slice::from_ref(&request.id))
        .unwrap();
    let result = publication::publish_png(&request, &controller, &png_bytes(), || {
        fs::write(&request.source_path, b"replacement input").unwrap();
    });
    assert!(
        result.is_err(),
        "an export must not publish after its source changes"
    );
    assert!(!request.output_path.exists());
    assert_eq!(
        fs::read_dir(&folder.0).unwrap().count(),
        1,
        "only the source remains"
    );
}
