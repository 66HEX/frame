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
        for (pass, section) in [(1, analysis), (2, final_pass)] {
            let mut final_frames = None;
            let mut final_time = 0;
            for line in section.lines().filter(|line| line.starts_with("frame=")) {
                let fields = line
                    .split_ascii_whitespace()
                    .map(|field| field.split_once('=').unwrap());
                let recorded = fields.collect::<std::collections::HashMap<_, _>>();
                assert_eq!(recorded["duration_us"], "3000000", "{codec}: {line}");
                assert_eq!(recorded["pass"], pass.to_string());
                assert_eq!(recorded["size_bytes"], "N/A"); // pass one or output pipe
                let media: u64 = recorded["media_us"].parse().unwrap();
                let work: u64 = recorded["work_us"].parse().unwrap();
                assert_eq!(
                    work,
                    u64::midpoint(if pass == 1 { 0 } else { 3_000_000 }, media.min(3_000_000))
                );
                final_frames = Some(recorded["video_frames"]);
                final_time = media;
            }
            assert_eq!(final_frames, Some("72"), "{codec} pass {pass}");
            assert_eq!(final_time, 3_000_000, "{codec} pass {pass}");
        }
        assert!(analysis.contains("time=00:00:01.500"), "{logs}");
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

fn tiny_movie(path: &Path, seconds: u32) {
    let mut out = BufWriter::new(File::create(path).unwrap());
    writeln!(out, "YUV4MPEG2 W64 H32 F2:1 Ip A1:1 C420jpeg").unwrap();
    for n in 0..seconds * 2 {
        out.write_all(b"FRAME\n").unwrap();
        out.write_all(&vec![u8::try_from(n).unwrap() + 64; 64 * 32 * 3 / 2])
            .unwrap();
    }
}

fn silence(path: &Path, seconds: u32) {
    let data_bytes = seconds * 8000 * 2;
    let mut out = BufWriter::new(File::create(path).unwrap());
    out.write_all(b"RIFF").unwrap();
    out.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
    out.write_all(b"WAVEfmt \x10\0\0\0\x01\0\x01\0").unwrap();
    out.write_all(&8000_u32.to_le_bytes()).unwrap();
    out.write_all(&16000_u32.to_le_bytes()).unwrap();
    out.write_all(b"\x02\0\x10\0data").unwrap();
    out.write_all(&data_bytes.to_le_bytes()).unwrap();
    out.write_all(&vec![0; data_bytes as usize]).unwrap();
}

fn number(line: &str, name: &str) -> u64 {
    line.split_ascii_whitespace()
        .filter_map(|field| field.split_once('='))
        .find(|(key, _)| *key == name)
        .unwrap()
        .1
        .parse()
        .unwrap()
}

#[test]
fn unequal_video_and_audio_durations_keep_progress_continuous_between_passes() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("frames.y4m");
    let audio = dir.path().join("tone.wav");
    let movie = dir.path().join("source.mov");
    for (video_secs, audio_secs) in [(2, 20), (20, 2)] {
        tiny_movie(&raw, video_secs);
        silence(&audio, audio_secs);
        let mut create = Command::new(env!("CARGO_BIN_EXE_frame-media"));
        create
            .args(["convert", "-loglevel", "error", "-y", "-i"])
            .arg(&raw)
            .arg("-i")
            .arg(&audio);
        let maps = if video_secs < audio_secs {
            ["1:a:0", "0:v:0"]
        } else {
            ["0:v:0", "1:a:0"]
        };
        for map in maps {
            create.args(["-map", map]);
        }
        let fixture = create
            .args(["-c:v", "libx264", "-c:a", "copy"])
            .arg(&movie)
            .output()
            .unwrap();
        assert!(
            fixture.status.success(),
            "{}",
            String::from_utf8_lossy(&fixture.stderr)
        );
        let output = Command::new(env!("CARGO_BIN_EXE_frame-media"))
            .args(["convert", "-i"])
            .arg(&movie)
            .args([
                "-map",
                "0:a:0",
                "-map",
                "0:v:0",
                "-c:a",
                "copy",
                "-c:v",
                "libx264",
                "-b:v",
                "30k",
                "-frame_two_pass",
                "1",
                "-f",
                "framehash",
                "pipe:1",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let log = std::str::from_utf8(&output.stderr).unwrap();
        let (analysis, mux) = log.split_once("Video pass 2/2").unwrap();
        let last = analysis
            .lines()
            .rfind(|line| line.starts_with("frame="))
            .unwrap();
        let total = 20_000_000_u64;
        let video = u64::from(video_secs) * 1_000_000;
        let anchor = video * total / (video + total);
        assert_eq!(number(last, "media_us"), video);
        assert_eq!(number(last, "duration_us"), total);
        assert_eq!(number(last, "work_us"), anchor);
        let mut previous = anchor;
        for row in mux.lines().filter(|line| line.starts_with("frame=")) {
            let media = number(row, "media_us");
            let work = number(row, "work_us");
            assert_eq!(
                work,
                anchor + media.min(total) * (total - anchor) / total,
                "{row}"
            );
            assert!(work >= previous, "{row}");
            previous = work;
        }
        assert_eq!(previous, total);
    }
}

#[test]
fn selected_matroska_stream_uses_its_own_end_or_reports_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("frames.y4m");
    let audio = dir.path().join("tone.wav");
    let tagged = dir.path().join("tags.mkv");
    let untagged = dir.path().join("untagged.mkv");
    tiny_movie(&raw, 2);
    silence(&audio, 20);
    let status = Command::new(env!("CARGO_BIN_EXE_frame-media"))
        .args(["convert", "-i"])
        .arg(&raw)
        .arg("-i")
        .arg(&audio)
        .args([
            "-map", "0:v:0", "-map", "1:a:0", "-c:v", "libx264", "-c:a", "copy",
        ])
        .arg(&tagged)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    // Change only the names of the string-valued per-stream tags. The
    // binary Matroska Segment Duration and every packet remain untouched.
    let mut bytes = fs::read(&tagged).unwrap();
    let indexes: Vec<_> = bytes
        .windows(8)
        .enumerate()
        .filter(|(_, b)| *b == b"DURATION")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(indexes.len(), 2);
    for i in indexes {
        bytes[i] = b'X';
    }
    fs::write(&untagged, bytes).unwrap();
    for (input, selects_audio, expected_media, expected_duration) in [
        (&tagged, false, 2_000_000, 2_000_000),
        (&untagged, false, 2_000_000, 0),
        (&untagged, true, 20_000_000, 20_000_000),
    ] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_frame-media"));
        cmd.args(["convert", "-i"])
            .arg(input)
            .args(["-map", "0:v:0"]);
        if selects_audio {
            cmd.args(["-map", "0:a:0"]);
        }
        let result = cmd
            .args(["-c", "copy", "-f", "framehash", "pipe:1"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let log = std::str::from_utf8(&result.stderr).unwrap();
        let final_row = log
            .lines()
            .rfind(|line| line.starts_with("frame="))
            .unwrap();
        assert_eq!(number(final_row, "duration_us"), expected_duration, "{log}");
        assert_eq!(number(final_row, "media_us"), expected_media, "{log}");
    }
}
