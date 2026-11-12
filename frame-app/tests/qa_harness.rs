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
