use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Command, Output};

fn variable_motion(path: &Path) {
    let (width, height) = (128_usize, 80_usize);
    let mut out = BufWriter::new(File::create(path).unwrap());
    writeln!(out, "YUV4MPEG2 W{width} H{height} F24:1 Ip A1:1 C420jpeg").unwrap();
    let mut noise = 71_u32;
    for frame in 0..72 {
        out.write_all(b"FRAME\n").unwrap();
        let luma = (0..width * height)
            .map(|i| {
                noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let pattern = (i % width + i / width + frame) % 64;
                if frame < 24 {
                    100
                } else if frame < 48 {
                    16 + u8::try_from(noise >> 25).unwrap()
                } else {
                    32 + u8::try_from(pattern).unwrap() * 3
                }
            })
            .collect::<Vec<_>>();
        out.write_all(&luma).unwrap();
        out.write_all(&vec![128; width * height / 2]).unwrap();
    }
}

fn video(source: &Path, scratch: &Path, codec: &str, two_pass: bool) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_frame-media"));
    cmd.env("FRAME_PASS_DIRECTORY", scratch)
        .args(["convert", "-i"])
        .arg(source)
        .args([
            "-map", "0:v:0", "-c:v", codec, "-b:v", "130k", "-vf", "hflip",
        ]);
    if codec == "vp9" {
        cmd.args(["-deadline", "good", "-cpu-used", "4"]);
    } else {
        cmd.args(["-preset", "fast"]);
    }
    if two_pass {
        cmd.args(["-frame_two_pass", "1"]);
    }
    cmd
}

fn sizes(output: &Output) -> Vec<usize> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let contents = std::str::from_utf8(&output.stdout).unwrap();
    contents
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split(',').nth(4).unwrap().trim().parse().unwrap())
        .collect()
}

#[test]
fn two_pass_abr_changes_packet_allocation_for_each_frame_encoder() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("variable-motion.y4m");
    // Colons are valid POSIX filenames, and Windows paths already contain a
    // drive separator. This path must survive the x265 options dictionary.
    let scratch = tmp.path().join(if cfg!(windows) {
        "média stats"
    } else {
        "média:stats"
    });
    fs::create_dir(&scratch).unwrap();
    variable_motion(&source);

    for codec in ["libx264", "libx265", "vp9"] {
        let run = |two_pass| {
            video(&source, &scratch, codec, two_pass)
                .args(["-f", "framehash", "pipe:1"])
                .output()
                .unwrap()
        };
        let ordinary = run(false);
        let planned = run(true);
        let first = sizes(&ordinary);
        let second = sizes(&planned);
        assert_eq!((first.len(), second.len()), (72, 72), "{codec}");
        // Compare allocation across the easy and busy sections of the same
        // movie. An encoder-specific SEI or a different header cannot account
        // for changes across these non-initial packets.
        assert!(
            first[12..]
                .iter()
                .zip(&second[12..])
                .filter(|(a, b)| a != b)
                .count()
                > 8,
            "{codec}"
        );
        let logs = String::from_utf8_lossy(&planned.stderr);
        let (analysis, final_pass) = logs.split_once("Video pass 2/2").unwrap();
        assert!(analysis.contains("Video pass 1/2"));
        let times = analysis
            .lines()
            .filter_map(|line| line.split_once(" time=").map(|(_, t)| t));
        let mut observed = false;
        for time in times {
            assert!(time <= "00:00:01.500", "{codec}: {time}");
            observed = true;
        }
        assert!(observed, "{codec} did not report analysis progress");
        assert!(final_pass.contains("time=00:00:03.000"), "{logs}");
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
    }
}

#[test]
fn invalid_two_pass_request_does_not_create_the_destination() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source.y4m");
    let output = tmp.path().join("out.mkv");
    variable_motion(&source);
    for (codec, rate) in [("ffv1", "100k"), ("copy", "100k"), ("libx264", "0")] {
        let result = Command::new(env!("CARGO_BIN_EXE_frame-media"))
            .args(["convert", "-i"])
            .arg(&source)
            .args([
                "-map",
                "0:0",
                "-c:v",
                codec,
                "-b:v",
                rate,
                "-frame_two_pass",
                "1",
                "-n",
            ])
            .arg(&output)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{codec}, {rate}");
        assert!(!output.exists());
    }
}

#[test]
fn simultaneous_two_pass_jobs_clean_separate_stats_after_success_and_publication_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source.y4m");
    let destination = tmp.path().join("same-destination.mkv");
    let scratch = tmp.path().join("scratch");
    fs::create_dir(&scratch).unwrap();
    variable_motion(&source);

    let start = || {
        video(&source, &scratch, "libx265", true)
            .arg("-n")
            .arg(&destination)
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let children = [start(), start()];
    let mut written = 0;
    for child in children {
        let result = child.wait_with_output().unwrap();
        if result.status.success() {
            written += 1;
        } else {
            let logs = String::from_utf8_lossy(&result.stderr);
            assert!(logs.contains("Video pass 2/2"), "{logs}");
        }
    }
    assert_eq!(written, 1);
    assert!(destination.metadata().unwrap().len() > 1024);
    assert_eq!(fs::read_dir(scratch).unwrap().count(), 0);
}
