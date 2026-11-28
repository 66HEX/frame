//! Real media written and read through the linked libraries, with no fixture CLI.
use std::path::Path;
use std::process::{Command, Output};
use std::{ffi::OsStr, fs};
use tempfile::tempdir;

fn worker(operation: &str, arguments: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Output {
    Command::new(env!("CARGO_BIN_EXE_frame-media"))
        .arg(operation)
        .args(arguments)
        .env_remove("PATH")
        .output()
        .expect("run the native worker")
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn wav(path: &Path) -> Vec<u8> {
    let mut pcm = Vec::new();
    for sample in 0..400_i16 {
        pcm.extend_from_slice(&(sample * 32).to_le_bytes());
    }
    let size = u32::try_from(pcm.len()).unwrap();
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + size).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes()); // PCM
    bytes.extend(1_u16.to_le_bytes()); // mono
    bytes.extend(8000_u32.to_le_bytes());
    bytes.extend(16000_u32.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(size.to_le_bytes());
    bytes.extend(&pcm);
    fs::write(path, bytes).unwrap();
    pcm
}

#[test]
fn copy_preserves_pcm_payload_and_confirms_completion() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.wav");
    let output = dir.path().join("output.wav");
    let progress = dir.path().join("progress");
    let pcm = wav(&source);
    success(&worker(
        "convert",
        [
            OsStr::new("-progress"),
            progress.as_os_str(),
            "-i".as_ref(),
            source.as_os_str(),
            "-map".as_ref(),
            "0:0".as_ref(),
            "-c".as_ref(),
            "copy".as_ref(),
            "-n".as_ref(),
            output.as_os_str(),
        ],
    ));
    assert!(fs::read(output).unwrap().ends_with(&pcm));
    assert!(
        fs::read_to_string(progress)
            .unwrap()
            .ends_with("progress=end\n")
    );
}

#[test]
fn audio_encoder_and_filter_trim_and_silence_exact_samples() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.wav");
    let output = dir.path().join("output.s16le");
    wav(&source);
    success(&worker(
        "convert",
        [
            OsStr::new("-ss"),
            "0.010".as_ref(),
            "-i".as_ref(),
            source.as_os_str(),
            "-t".as_ref(),
            "0.025".as_ref(),
            "-map".as_ref(),
            "0:0".as_ref(),
            "-af".as_ref(),
            "volume=0".as_ref(),
            "-c:a".as_ref(),
            "pcm_s16le".as_ref(),
            "-f".as_ref(),
            "s16le".as_ref(),
            "-n".as_ref(),
            output.as_os_str(),
        ],
    ));
    assert_eq!(fs::read(output).unwrap(), vec![0; 400]);
}

#[test]
fn encoder_writes_scaled_single_image_and_preserves_existing_output() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.ppm");
    let output = dir.path().join("out mědia.png");
    let mut ppm = b"P6\n2 1\n255\n".to_vec();
    ppm.extend([255, 0, 0, 0, 255, 0]);
    fs::write(&source, ppm).unwrap();
    let args = [
        OsStr::new("-i"),
        source.as_os_str(),
        "-map".as_ref(),
        "0:v:0".as_ref(),
        "-c:v".as_ref(),
        "png".as_ref(),
        "-vf".as_ref(),
        "scale=4:2,hflip".as_ref(),
        "-frames:v".as_ref(),
        "1".as_ref(),
        "-update".as_ref(),
        "1".as_ref(),
        "-n".as_ref(),
        output.as_os_str(),
    ];
    success(&worker("convert", args));
    let before = fs::read(&output).unwrap();
    assert_eq!(&before[0..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&before[16..24], &[0, 0, 0, 4, 0, 0, 0, 2]);
    assert!(!worker("convert", args).status.success());
    assert_eq!(before, fs::read(&output).unwrap());
}

#[test]
fn unknown_export_option_fails_before_creating_output() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("input.wav");
    let output = dir.path().join("output.wav");
    wav(&source);
    assert!(
        !worker(
            "convert",
            [
                OsStr::new("-i"),
                source.as_os_str(),
                "-map".as_ref(),
                "0:0".as_ref(),
                "-unimplemented".as_ref(),
                "value".as_ref(),
                output.as_os_str(),
            ]
        )
        .status
        .success()
    );
    assert!(!output.exists());
}

#[cfg(target_os = "linux")] // APFS rejects non-UTF-8 file names.
#[test]
fn copy_and_completion_preserve_non_utf8_paths_and_quiet_output() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempdir().unwrap();
    let source = dir.path().join(OsStr::from_bytes(b"input-\xff.wav"));
    let output = dir.path().join(OsStr::from_bytes(b"output-\xfe.wav"));
    let progress = dir.path().join(OsStr::from_bytes(b"proof-\xfd"));
    let original = wav(&source);
    let result = worker(
        "convert",
        [
            OsStr::new("-progress"),
            progress.as_os_str(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-xerror".as_ref(),
            "-i".as_ref(),
            source.as_os_str(),
            "-map".as_ref(),
            "0:0".as_ref(),
            "-c".as_ref(),
            "copy".as_ref(),
            "-n".as_ref(),
            output.as_os_str(),
        ],
    );
    success(&result);
    assert!(result.stderr.is_empty());
    assert!(fs::read(output).unwrap().ends_with(&original));
    assert!(
        fs::read_to_string(progress)
            .unwrap()
            .ends_with("progress=end\n")
    );
}

#[test]
fn explicit_audio_rate_rejects_invalid_and_unrepresentable_values_before_encoding() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("tone.wav");
    wav(&source);
    for (codec, ext, rate) in [
        ("aac", "m4a", "12345"),
        ("flac", "flac", "1048576"),
        ("libopus", "opus", "24000"),
        ("pcm_s16le", "wav", "2147483647"),
        ("pcm_s16le", "wav", "48000bad"),
    ] {
        let out = dir.path().join(format!("output.{ext}"));
        let result = worker(
            "convert",
            [
                OsStr::new("-i"),
                source.as_os_str(),
                "-map".as_ref(),
                "0:0".as_ref(),
                "-c:a".as_ref(),
                codec.as_ref(),
                "-ar:a:0".as_ref(),
                rate.as_ref(),
                out.as_os_str(),
            ],
        );
        assert!(!result.status.success(), "{codec}/{rate}");
        let msg = String::from_utf8_lossy(&result.stderr);
        assert!(msg.contains(rate), "{msg}");
    }
}

fn last_telemetry(output: &Output) -> std::collections::HashMap<&str, &str> {
    std::str::from_utf8(&output.stderr)
        .unwrap()
        .lines()
        .rfind(|line| line.starts_with("frame="))
        .expect("terminal progress line")
        .split_ascii_whitespace()
        .map(|field| field.split_once('=').expect("key=value"))
        .collect()
}

#[test]
fn audio_telemetry_uses_trimmed_clock_and_final_length_after_backpatching_headers() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("short.wav");
    wav(&src);
    for (n, input, output, expected_us) in [
        ("full", vec![], vec![], 50_000),
        ("seek", vec!["-ss", "0.010"], vec![], 40_000),
        ("trim", vec!["-ss", "0.010"], vec!["-t", "0.025"], 25_000),
        ("past-end", vec!["-ss", "0.010"], vec!["-t", "1"], 40_000),
    ] {
        let dest = dir.path().join(format!("{n}.wav"));
        let mut args: Vec<&OsStr> = input.into_iter().map(OsStr::new).collect();
        args.extend([OsStr::new("-i"), src.as_os_str()]);
        args.extend(output.into_iter().map(OsStr::new));
        args.extend(["-map", "0:0", "-c:a", "pcm_s16le"].map(OsStr::new));
        args.push(dest.as_os_str());
        let result = worker("convert", args);
        success(&result);
        let stats = last_telemetry(&result);
        let expected = expected_us.to_string();
        assert_eq!(stats["duration_us"], expected);
        assert_eq!(stats["media_us"], expected);
        assert_eq!(stats["work_us"], expected);
        assert_eq!(stats["pass"], "0");
        assert_eq!(stats["frame"], "0");
        assert_eq!(stats["video_frames"], "N/A");
        assert_eq!(
            stats["size_bytes"],
            fs::metadata(dest).unwrap().len().to_string()
        );
    }
}

#[test]
fn still_image_keeps_unknown_duration_and_quiet_helpers_write_only_completion() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("source.ppm");
    fs::write(&src, b"P6\n1 1\n255\n\x10\x20\x30").unwrap();
    for quiet in [false, true] {
        let proof = dir.path().join("progress");
        let mut args = Vec::new();
        if quiet {
            args.extend(["-loglevel", "error"].map(OsStr::new));
        }
        args.extend([
            OsStr::new("-progress"),
            proof.as_os_str(),
            OsStr::new("-i"),
            src.as_os_str(),
        ]);
        args.extend(["-map", "0:v:0", "-c:v", "copy", "-f", "framehash", "pipe:1"].map(OsStr::new));
        let result = worker("convert", args);
        success(&result);
        assert!(
            fs::read_to_string(proof)
                .unwrap()
                .ends_with("progress=end\n")
        );
        if quiet {
            assert!(result.stderr.is_empty());
        } else {
            let stats = last_telemetry(&result);
            assert_eq!(stats["duration_us"], "0");
            assert_eq!(stats["video_frames"], "1");
            assert_eq!(stats["size_bytes"], "N/A");
            assert_eq!(stats["media_us"], stats["work_us"]);
        }
    }
}

#[test]
fn preflight_negotiates_each_decoder_before_any_output_is_opened() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.ppm");
    let mut ppm = b"P6\n3 3\n255\n".to_vec();
    ppm.extend([128; 27]);
    fs::write(&source, ppm).unwrap();
    let output = dir.path().join("encoded.mp4");
    let progress = dir.path().join("progress");
    for device in ["cuda", "d3d11va", "vaapi"] {
        let args = [
            "-hwaccel",
            device,
            "-i",
            source.to_str().unwrap(),
            "-map",
            "0:0",
            "-vf",
            "pad=4:4",
            "-pix_fmt",
            "yuv420p",
            "-c:v",
            "libx264",
            "-progress",
            progress.to_str().unwrap(),
            "-n",
        ];
        let dest = output.to_str().unwrap();
        let probe = worker("preflight", args.into_iter().chain([dest]));
        success(&probe);
        let route: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
        assert_eq!(route["encoder"], "libx264");
        assert_eq!(route["decoder"], "cpu");
        assert_eq!(route["filters"], "cpu");
        assert_eq!(route["format"], "yuv420p");
        assert_eq!(route["upload"], false);
        assert!(route["fallback"].as_str().is_some());
        assert!(!output.exists() && !progress.exists());

        for pins in [
            ["-frame_decoder", device],
            ["-frame_filters", device],
            ["-frame_format", "p010le"],
            ["-frame_device", "removed-adapter"],
        ] {
            assert!(
                !worker("convert", args.into_iter().chain(pins).chain([dest]))
                    .status
                    .success()
            );
            assert!(!output.exists());
        }
        let pins = [
            "-frame_decoder",
            "cpu",
            "-frame_filters",
            "cpu",
            "-frame_device",
            "default",
            "-frame_format",
            "yuv420p",
        ];
        success(&worker(
            "convert",
            args.into_iter().chain(pins).chain([dest]),
        ));
        assert!(fs::read(&output).unwrap().len() > 100);
        fs::remove_file(&output).unwrap();
        fs::remove_file(&progress).unwrap();
    }
}
