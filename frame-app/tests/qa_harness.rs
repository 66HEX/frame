#![cfg(feature = "qa-harness")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "frame-qa-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_qa(flag: &str, path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_frame-qa"))
        .arg(flag)
        .arg(path)
        .output()
        .unwrap()
}

#[test]
fn headless_qa_rejects_malformed_requests_without_starting_the_ui() {
    let sandbox = Sandbox::new();
    let request = sandbox.0.join("invalid.json");
    fs::write(&request, b"{not valid JSON}").unwrap();

    let output = run_qa("--plan", &request);

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("request JSON"), "{error}");
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 1);
}

#[test]
fn headless_source_frame_rejects_malformed_requests_without_starting_the_ui() {
    let sandbox = Sandbox::new();
    let request = sandbox.0.join("invalid-frame.json");
    fs::write(&request, b"{not valid JSON}").unwrap();

    let output = run_qa("--export-frame", &request);

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("request JSON"), "{error}");
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 1);
}

#[test]
#[ignore = "requires FFmpeg; run with --ignored"]
fn headless_source_frame_publishes_original_pixels_without_replacing_a_result() {
    let sandbox = Sandbox::new();
    let source = sandbox.0.join("Zażółć source.png");
    let original = image::RgbaImage::from_fn(33, 19, |x, y| {
        image::Rgba([
            u8::try_from(x * 7).unwrap(),
            u8::try_from(y * 11).unwrap(),
            u8::try_from(x + y).unwrap(),
            127,
        ])
    });
    original.save(&source).unwrap();
    let source_before = fs::read(&source).unwrap();
    let output_directory = sandbox.0.join("output's space");
    fs::create_dir(&output_directory).unwrap();
    let sentinel = output_directory.join("Zażółć source_frame_0.png");
    fs::write(&sentinel, b"existing frame must survive").unwrap();
    let request = sandbox.0.join("frame-request.json");
    fs::write(
        &request,
        serde_json::to_vec(&serde_json::json!({
            "input": source,
            "outputDirectory": output_directory,
            "positionMicroseconds": 0,
            "videoStreamIndex": 0
        }))
        .unwrap(),
    )
    .unwrap();

    let exported = run_qa("--export-frame", &request);

    assert!(
        exported.status.success(),
        "{}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&exported.stdout).unwrap();
    assert_eq!(result["status"], "published");
    let output = Path::new(result["outputPath"].as_str().unwrap());
    assert_ne!(output, sentinel);
    assert_eq!(image::open(output).unwrap().into_rgba8(), original);
    assert_eq!(fs::read(&source).unwrap(), source_before);
    assert_eq!(fs::read(&sentinel).unwrap(), b"existing frame must survive");
    assert_eq!(fs::read_dir(&output_directory).unwrap().count(), 2);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn headless_qa_exports_through_the_app_runner_and_reports_the_actual_result() {
    let sandbox = Sandbox::new();
    let source = sandbox.0.join("Zażółć source.wav");
    let generated = Command::new(frame_app::runtime_binaries::ffmpeg_executable())
        .args([
            "-v",
            "error",
            "-n",
            "-f",
            "lavfi",
            "-i",
            "sine=duration=0.25",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let before = fs::read(&source).unwrap();
    let output_directory = sandbox.0.join("output's space");
    fs::create_dir(&output_directory).unwrap();
    let sentinel = output_directory.join("result.wav");
    fs::write(&sentinel, b"existing result must survive").unwrap();
    let request = sandbox.0.join("request.json");
    fs::write(&request, serde_json::to_vec(&serde_json::json!({
        "input": source,
        "outputDirectory": output_directory,
        "outputName": "result.wav",
        "config": {"container": "wav", "audioCodec": "copy", "processingMode": "copy", "selectedAudioTracks": [0]}
    })).unwrap()).unwrap();

    let planned = run_qa("--plan", &request);
    assert!(
        planned.status.success(),
        "{}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&planned.stdout).unwrap();
    assert_eq!(plan["normalizedConfig"]["processingMode"], "copy");
    assert_eq!(
        fs::read_dir(&output_directory).unwrap().count(),
        1,
        "planning must not create an output"
    );

    let exported = run_qa("--export", &request);
    assert!(
        exported.status.success(),
        "{}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let events: Vec<serde_json::Value> = String::from_utf8(exported.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let completed: Vec<_> = events
        .iter()
        .filter(|event| event["event"] == "conversion-completed")
        .collect();
    assert_eq!(completed.len(), 1, "{events:?}");
    let result = Path::new(completed[0]["payload"]["output_path"].as_str().unwrap());
    assert_ne!(result, sentinel);
    assert!(result.is_file());
    assert_eq!(fs::read(&source).unwrap(), before);
    assert_eq!(
        fs::read(&sentinel).unwrap(),
        b"existing result must survive"
    );
    assert_eq!(
        fs::read_dir(&output_directory).unwrap().count(),
        2,
        "only sentinel and completed result remain"
    );
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn headless_qa_applies_independent_modes_to_video_and_audio() {
    let sandbox = Sandbox::new();
    let input = sandbox.0.join("source.mkv");
    let generated = Command::new(frame_app::runtime_binaries::ffmpeg_executable())
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=96x64:duration=0.6",
            "-f",
            "lavfi",
            "-i",
            "sine=duration=0.6",
            "-c:v",
            "mpeg2video",
            "-c:a",
            "mp2",
        ])
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );

    for (mode, expected_video, expected_audio) in [
        ("copyAudio", "h264", "mp2"),
        ("copyVideo", "mpeg2video", "flac"),
    ] {
        let request = sandbox.0.join(format!("{mode}.json"));
        let output_name = format!("{mode}.mkv");
        fs::write(
            &request,
            serde_json::to_vec(&serde_json::json!({
                "input": input,
                "outputDirectory": sandbox.0,
                "outputName": output_name,
                "config": {
                    "container": "mkv", "processingMode": mode, "videoCodec": "libx264",
                    "audioCodec": "flac", "selectedAudioTracks": [1]
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let run = run_qa("--export", &request);
        assert!(
            run.status.success(),
            "{mode}: {} {}",
            String::from_utf8_lossy(&run.stderr),
            String::from_utf8_lossy(&run.stdout)
        );
        let probe = Command::new(frame_app::runtime_binaries::ffprobe_executable())
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_name",
                "-of",
                "json",
            ])
            .arg(sandbox.0.join(output_name))
            .output()
            .unwrap();
        assert!(
            probe.status.success(),
            "{}",
            String::from_utf8_lossy(&probe.stderr)
        );
        let info: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
        let streams = info["streams"].as_array().unwrap();
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0]["codec_name"], expected_video, "{mode}");
        assert_eq!(streams[1]["codec_name"], expected_audio, "{mode}");
    }
}
