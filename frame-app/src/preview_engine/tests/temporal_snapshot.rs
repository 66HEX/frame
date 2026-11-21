use super::*;
use crate::bounded_process::{CommandSpec, ExecutionDomain, Limits};
use frame_core::types::DeinterlaceMode;
use std::ffi::OsString;
use std::path::Path;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "frame-temporal-snapshot-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(program: &str, args: Vec<OsString>) -> Vec<u8> {
    static DOMAIN: ExecutionDomain = ExecutionDomain::new();
    let mut command = CommandSpec::new(program);
    command.args = args;
    let result = DOMAIN
        .run(
            &command,
            &[],
            Limits {
                timeout: Duration::from_secs(10),
                cleanup_timeout: Duration::from_secs(1),
                stdout_bytes: 2 * 1024 * 1024,
                stderr_bytes: 64 * 1024,
            },
            || false,
        )
        .expect("bounded fixture process");
    assert!(
        result.success,
        "fixture process failed: {}",
        String::from_utf8_lossy(&result.stderr.bytes)
    );
    result.stdout.bytes
}

fn dynamic_source(directory: &Path, vfr: bool) -> PathBuf {
    for (index, (width, height)) in [(128, 96), (160, 80), (96, 64)].into_iter().enumerate() {
        image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([
                u8::try_from((x * 7 + y * 3) % 256).unwrap(),
                if y % 2 == 0 { 240 } else { 15 },
                u8::try_from(index * 90 + 20).unwrap(),
                255,
            ])
        })
        .save(directory.join(format!("frame-{index}.png")))
        .unwrap();
    }
    let input = directory.join("dynamic.mov");
    let mut args: Vec<OsString> = ["-v", "error"].map(Into::into).to_vec();
    if vfr {
        let list = directory.join("frames.ffconcat");
        std::fs::write(
            &list,
            "ffconcat version 1.0\nfile frame-0.png\nduration 0.12\nfile frame-1.png\nduration 0.20\nfile frame-2.png\n",
        )
        .unwrap();
        args.extend([
            "-f".into(),
            "concat".into(),
            "-i".into(),
            list.into_os_string(),
        ]);
    } else {
        args.extend([
            "-framerate".into(),
            "25".into(),
            "-i".into(),
            directory.join("frame-%d.png").into_os_string(),
        ]);
    }
    args.extend(["-c:v".into(), "copy".into(), input.clone().into_os_string()]);
    run(&crate::runtime_binaries::ffmpeg_executable(), args);
    let packets: serde_json::Value = serde_json::from_slice(&run(
        &crate::runtime_binaries::ffprobe_executable(),
        [
            "-v",
            "error",
            "-show_packets",
            "-show_entries",
            "packet=pts_time,pos,size",
            "-of",
            "json",
        ]
        .map(OsString::from)
        .into_iter()
        .chain([input.clone().into_os_string()])
        .collect(),
    ))
    .unwrap();
    let packets = packets["packets"].as_array().unwrap();
    assert_eq!(packets.len(), 3);
    let bytes = std::fs::read(&input).unwrap();
    let times = if vfr {
        ["0.000000", "0.120000", "0.320000"]
    } else {
        ["0.000000", "0.040000", "0.080000"]
    };
    for (index, packet) in packets.iter().enumerate() {
        assert_eq!(packet["pts_time"].as_str(), Some(times[index]));
        let position: usize = packet["pos"].as_str().unwrap().parse().unwrap();
        let size: usize = packet["size"].as_str().unwrap().parse().unwrap();
        let original = std::fs::read(directory.join(format!("frame-{index}.png"))).unwrap();
        assert!(
            bytes[position..position + size] == original,
            "packet {index} changed"
        );
    }
    input
}

fn oracle(png: &Path, deinterlace: bool) -> Vec<u8> {
    let filter = if deinterlace {
        "bwdif=mode=send_frame:parity=auto:deint=all,scale=128:96,setsar=1,format=bgra"
    } else {
        "scale=128:96,setsar=1,format=bgra"
    };
    oracle_filter(png, filter)
}

fn oracle_filter(png: &Path, filter: &str) -> Vec<u8> {
    run(
        &crate::runtime_binaries::ffmpeg_executable(),
        ["-v", "error", "-loop", "1", "-framerate", "25", "-i"]
            .map(OsString::from)
            .into_iter()
            .chain([png.as_os_str().to_owned()])
            .chain(
                [
                    "-vf",
                    filter,
                    "-frames:v",
                    "1",
                    "-pix_fmt",
                    "bgra",
                    "-f",
                    "rawvideo",
                    "pipe:1",
                ]
                .map(Into::into),
            )
            .collect(),
    )
}

fn assert_snapshot(frame: &PreviewRenderedFrame, expected: &[u8], time: f64) {
    assert_eq!((frame.width, frame.height), (128, 96));
    let image = frame.render_image();
    let actual = image.as_bytes(0).unwrap();
    assert_eq!(actual.len(), expected.len());
    let mismatches = actual.iter().zip(expected).filter(|(a, b)| a != b).count();
    assert_eq!(
        mismatches, 0,
        "snapshot at {time}s selected/processed the wrong pixels"
    );
}

fn exercise_snapshot(vfr: bool) {
    let _guard = crate::preview_engine::ffmpeg_backend::test_support::process_fixture();
    let directory = Fixture::new();
    let input = dynamic_source(&directory.0, vfr);
    let mut config = real_video_preview_config(input);
    config.source_width = Some(128);
    config.source_height = Some(96);
    config.max_width = 128;
    config.max_height = 96;
    config.conversion_config.video_filters.deinterlace = DeinterlaceMode::On;
    let expected = oracle(&directory.0.join("frame-0.png"), true);
    assert!(
        expected != oracle(&directory.0.join("frame-0.png"), false),
        "oracle must exercise the effect"
    );
    let session = PreviewSession::start(config).unwrap();
    if !vfr {
        assert_snapshot(&session.latest_frame().unwrap().frame, &expected, 0.0);
    }
    for (time, index) in if vfr {
        [(0.05, 1), (0.32, 2)]
    } else {
        [(0.04, 1), (0.08, 2)]
    } {
        session.command(PreviewCommand::SeekPrecise(time)).unwrap();
        let png = directory.0.join(format!("frame-{index}.png"));
        let expected = oracle(&png, true);
        assert!(
            expected != oracle(&png, false),
            "oracle must exercise the effect"
        );
        assert_snapshot(&session.latest_frame().unwrap().frame, &expected, time);
    }
    session.stop().unwrap();
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; pixel oracle for dynamic geometry and temporal snapshot"]
fn ffmpeg_temporal_snapshot_keeps_first_frame_and_effect_across_geometry_changes() {
    exercise_snapshot(false);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; VFR pixel oracle for precise temporal snapshot"]
fn ffmpeg_temporal_snapshot_keeps_precisely_selected_vfr_frame_and_effect() {
    exercise_snapshot(true);
}

#[test]
#[ignore = "requires FFmpeg/FFprobe/libass; temporal VFR subtitle clock pixel oracle"]
fn ffmpeg_temporal_snapshot_preserves_selected_vfr_subtitle_time() {
    let _guard = crate::preview_engine::ffmpeg_backend::test_support::process_fixture();
    let directory = Fixture::new();
    let input = dynamic_source(&directory.0, true);
    let subtitles = directory.0.join("cue.ass");
    std::fs::write(&subtitles, concat!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: 160\nPlayResY: 80\n",
        "[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n",
        "Style: Default,Arial,12,&H0000FFFF,&H0000FFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,7,0,0,0,1\n",
        "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        "Dialogue: 0,0:00:00.10,0:00:00.14,Default,,0,0,0,,{\\p1\\pos(4,4)\\an7}m 0 0 l 24 0 24 16 0 16\n",
    )).unwrap();
    let mut config = real_video_preview_config(input);
    config.source_width = Some(128);
    config.source_height = Some(96);
    config.max_width = 128;
    config.max_height = 96;
    config.conversion_config.video_filters.deinterlace = DeinterlaceMode::On;
    config.conversion_config.subtitle_burn_path = Some(subtitles.to_string_lossy().into_owned());
    config.conversion_config.subtitle_font_name = None;
    config.conversion_config.subtitle_font_size = None;
    config.conversion_config.subtitle_font_color = None;
    config.conversion_config.subtitle_outline_color = None;
    config.conversion_config.subtitle_position = None;
    let mut escaped = subtitles.to_string_lossy().into_owned();
    for separators in [
        &['\\', '\'', ':'][..],
        &['\\', '\'', '[', ']', ',', ';'][..],
    ] {
        escaped = escaped
            .chars()
            .flat_map(|character| {
                let prefix =
                    (character.is_whitespace() || separators.contains(&character)).then_some('\\');
                prefix.into_iter().chain([character])
            })
            .collect();
    }
    let png = directory.0.join("frame-1.png");
    let expected = oracle_filter(
        &png,
        &format!(
            "bwdif=mode=send_frame:parity=auto:deint=all,setpts=PTS+0.12/TB,subtitles=filename={escaped},scale=128:96,setsar=1,format=bgra"
        ),
    );
    assert!(
        expected != oracle(&png, true),
        "subtitle oracle must contain the active cue"
    );
    let session = PreviewSession::start(config).unwrap();
    session.command(PreviewCommand::SeekPrecise(0.05)).unwrap();
    assert_snapshot(&session.latest_frame().unwrap().frame, &expected, 0.05);
    session.stop().unwrap();
}
