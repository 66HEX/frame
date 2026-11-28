//! Exercise the app's raw streams with locally generated, lossless media.
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn worker(source: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_frame-media"))
        .env("PATH", "")
        .args(["preview", "-i"])
        .arg(source)
        .args(args)
        .output()
        .unwrap()
}

fn successful(result: Output) -> Vec<u8> {
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}

#[test]
fn filtered_preview_keeps_bgra_channel_order_and_removes_line_padding() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("two pixels.ppm");
    fs::write(&path, b"P6\n2 1\n255\n\xff\0\0\0\x80\xff").unwrap();
    let pixels = successful(worker(
        &path,
        &[
            "-filter_complex",
            "[0:v:0]hflip[preview_v]",
            "-map",
            "[preview_v]",
            "-an",
            "-sn",
            "-dn",
            "-pix_fmt",
            "bgra",
            "-f",
            "rawvideo",
            "-frames:v",
            "1",
            "pipe:1",
        ],
    ));
    assert_eq!(pixels, [255, 128, 0, 255, 0, 0, 255, 255]);
}

#[test]
fn audio_graph_applies_volume_and_accurate_sample_trim() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("constant.wav");
    let mut wav = b"RIFF".to_vec();
    let count = 800_u32;
    wav.extend_from_slice(&(36 + count * 2).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    wav.extend_from_slice(&8000_u32.to_le_bytes());
    wav.extend_from_slice(&16000_u32.to_le_bytes());
    wav.extend_from_slice(b"\x02\0\x10\0data");
    wav.extend_from_slice(&(count * 2).to_le_bytes());
    for _ in 0..count {
        wav.extend_from_slice(&8192_i16.to_le_bytes());
    }
    fs::write(&path, wav).unwrap();
    let samples = successful(worker(
        &path,
        &[
            "-ss",
            "0.010",
            "-t",
            "0.015",
            "-map",
            "0:0",
            "-af",
            "volume=0.5",
            "-ac",
            "1",
            "-ar",
            "8000",
            "-f",
            "f32le",
            "pipe:1",
        ],
    ));
    assert_eq!(samples.len(), 120 * 4);
    assert!(
        samples
            .chunks_exact(4)
            .all(|s| s == 0.125_f32.to_le_bytes())
    );
}

#[test]
fn analysis_reports_zero_frames_and_preview_errors_propagate() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../frame-core/tests/fixtures/hevc-runtime-comparison/source.mp4");
    let counts = successful(worker(
        &source,
        &[
            "-filter_complex",
            "[0:v:0]select=0[out]",
            "-map",
            "[out]",
            "-progress",
            "pipe:1",
            "-f",
            "null",
            "-",
        ],
    ));
    assert_eq!(counts, b"frame=0\nprogress=end\n");
    let failed = worker(
        &source,
        &["-vf", "no_such_frame_filter", "-f", "rawvideo", "pipe:1"],
    );
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
}

#[test]
fn lists_the_linked_registries_without_cli_tools_on_path() {
    for (kind, name) in [
        ("-encoders", " png "),
        ("-filters", " scale "),
        ("-bsfs", "setts"),
    ] {
        let listing = successful(
            Command::new(env!("CARGO_BIN_EXE_frame-media"))
                .env("PATH", "")
                .args(["capabilities", kind])
                .output()
                .unwrap(),
        );
        assert!(std::str::from_utf8(&listing).unwrap().contains(name));
    }
}

#[test]
fn midstream_orientation_sei_applies_to_following_frames_until_replaced() {
    // Two 4-picture H.264 clips of a blue/red split, concatenated without
    // re-encoding. The second clip carries a single 90-degree orientation SEI.
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../frame-core/tests/fixtures/changing-orientation.mkv");
    let raw = successful(worker(
        &source,
        &[
            "-threads",
            "1",
            "-vf",
            "scale=96:64,format=bgra",
            "-fps_mode",
            "passthrough",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "bgra",
            "pipe:1",
        ],
    ));
    let pictures: Vec<_> = raw.chunks_exact(96 * 64 * 4).collect();
    assert_eq!(pictures.len(), 8);
    for (index, picture) in pictures.iter().enumerate() {
        let sample = &picture[(12 * 96 + 16) * 4..][..4];
        let blue = sample[0] > 240 && sample[2] < 8;
        let red = sample[2] > 240 && sample[0] < 8;
        assert!(
            if index < 4 { blue } else { red },
            "frame {index}: {sample:?}"
        );
    }
}
