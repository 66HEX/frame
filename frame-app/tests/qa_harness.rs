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

fn generate_av_source(sandbox: &Sandbox) -> PathBuf {
    let input = sandbox.0.join("źródło test.mkv");
    let generated = Command::new(frame_app::runtime_binaries::ffmpeg_executable())
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=96x64:duration=2",
            "-f",
            "lavfi",
            "-i",
            "sine=duration=2",
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
    input
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn headless_qa_applies_independent_modes_to_video_and_audio() {
    let sandbox = Sandbox::new();
    let input = generate_av_source(&sandbox);

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
        let streams = probe_streams(&sandbox.0.join(output_name));
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0]["codec_name"], expected_video, "{mode}");
        assert_eq!(streams[1]["codec_name"], expected_audio, "{mode}");
    }
}

fn probe_streams(path: &Path) -> Vec<serde_json::Value> {
    let probe = Command::new(frame_app::runtime_binaries::ffprobe_executable())
        .args([
            "-v",
            "error",
            "-count_frames",
            "-show_entries",
            "stream=codec_name,nb_read_frames,sample_rate:stream_tags=title,language:stream_disposition=default,comment",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        probe.status.success(),
        "{}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let mut info: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    serde_json::from_value(info["streams"].take()).unwrap()
}

#[test]
#[ignore = "requires FFmpeg/FFprobe and the Frame media worker; run with --ignored"]
fn headless_two_pass_export_publishes_the_selected_trim_with_audio() {
    let sandbox = Sandbox::new();
    let input = generate_av_source(&sandbox);
    let request = sandbox.0.join("two-pass.json");
    let out = sandbox.0.join("Zażółć two-pass output");
    fs::create_dir(&out).unwrap();
    fs::write(
        &request,
        serde_json::to_vec(&serde_json::json!({
            "input": input,
            "outputDirectory": out,
            "outputName": "render.mkv",
            "config": {
                "container": "mkv", "videoCodec": "libx265", "videoBitrateMode": "bitrate",
                "videoBitrate": "180", "videoTwoPass": true, "selectedAudioTracks": [1],
                "audioCodec": "flac", "startTime": "0.4", "endTime": "1.6"
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let plan = run_qa("--plan", &request);
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert!(
        plan["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|arg| arg == "-frame_two_pass")
    );
    let result = run_qa("--export", &request);
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    let streams = probe_streams(&out.join("render.mkv"));
    assert_eq!(streams.len(), 2);
    assert_eq!(streams[0]["codec_name"], "hevc");
    assert_eq!(streams[0]["nb_read_frames"], "30");
    assert_eq!(streams[1]["codec_name"], "flac");
    assert_eq!(fs::read_dir(out).unwrap().count(), 1);
}

fn generate_multitrack_source(sandbox: &Sandbox) -> PathBuf {
    let input = generate_av_source(sandbox);
    let bilingual = sandbox.0.join("bilingual.mkv");
    let source = Command::new(frame_app::runtime_binaries::ffmpeg_executable())
        .args(["-v", "error", "-i"])
        .arg(input)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "0:a:0",
            "-map",
            "0:a:0",
            "-c",
            "copy",
            "-metadata:s:a:0",
            "title=Old title",
            "-metadata:s:a:0",
            "language=eng",
            "-disposition:a:0",
            "default+comment",
            "-disposition:a:1",
            "0",
        ])
        .arg(&bilingual)
        .output()
        .unwrap();
    assert!(
        source.status.success(),
        "{}",
        String::from_utf8_lossy(&source.stderr)
    );
    bilingual
}

#[test]
#[ignore = "requires FFmpeg/FFprobe and the Frame media worker; run with --ignored"]
fn headless_copy_writes_audio_labels_and_moves_default_to_the_selected_track() {
    let sandbox = Sandbox::new();
    let bilingual = generate_multitrack_source(&sandbox);
    let request = sandbox.0.join("track-names.json");
    fs::write(
        &request,
        serde_json::to_vec(&serde_json::json!({
            "input": bilingual, "outputDirectory": sandbox.0, "outputName": "labelled.mkv",
            "config": {
                "container": "mkv", "processingMode": "copy", "selectedAudioTracks": [2, 1],
                "audioTrackOverrides": {
                    "1": {"title": ""},
                    "2": {"title": "Głos reżysera", "language": "pol", "isDefault": true}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let result = run_qa("--export", &request);
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    let streams = probe_streams(&sandbox.0.join("labelled.mkv"));
    assert_eq!(streams.len(), 3);
    assert_eq!(streams[1]["tags"]["title"], "Głos reżysera");
    assert_eq!(streams[1]["tags"]["language"], "pol");
    assert_eq!(streams[1]["disposition"]["default"], 1);
    assert!(streams[2]["tags"]["title"].is_null());
    assert_eq!(streams[2]["tags"]["language"], "eng");
    assert_eq!(streams[2]["disposition"]["default"], 0);
    assert_eq!(streams[2]["disposition"]["comment"], 1);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe and the Frame media worker; run with --ignored"]
fn headless_export_resamples_each_selected_audio_stream_at_its_requested_rate() {
    let sandbox = Sandbox::new();
    let input = generate_multitrack_source(&sandbox);
    let request = sandbox.0.join("resample.json");
    fs::write(
        &request,
        serde_json::to_vec(&serde_json::json!({
            "input": input, "outputDirectory": sandbox.0, "outputName": "rates.mkv",
            "config": {
                "container": "mkv", "processingMode": "copyVideo", "audioCodec": "flac",
                "selectedAudioTracks": [2, 1, 2],
                "audioTrackOverrides": { "1": {"sampleRate": 96000}, "2": {"sampleRate": 32000} }
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let result = run_qa("--export", &request);
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    let streams = probe_streams(&sandbox.0.join("rates.mkv"));
    assert_eq!(streams.len(), 4);
    for (audio, rate) in streams[1..].iter().zip(["32000", "96000", "32000"]) {
        assert_eq!(audio["codec_name"], "flac");
        assert_eq!(audio["sample_rate"], rate);
    }
}
