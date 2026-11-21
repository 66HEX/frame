use super::*;
use std::ffi::OsString;
use std::process::Command;

fn ffmpeg(args: &[OsString]) {
    let result = crate::process_launch::output(
        Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args(["-hide_banner", "-nostdin", "-loglevel", "error", "-n"])
            .args(args),
    )
    .unwrap();
    assert!(
        result.status.success(),
        "fixture: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn probe(path: &Path) -> serde_json::Value {
    let output = crate::process_launch::output(
        Command::new(crate::runtime_binaries::ffprobe_executable())
            .args([
                "-v",
                "error",
                "-show_streams",
                "-show_frames",
                "-show_format",
                "-of",
                "json",
            ])
            .arg(path),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "probe: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn export(path: &Path, folder: &Path, time: u64) -> PathBuf {
    let request = SourceFrameExportRequest::new(
        "fixture".into(),
        path.to_owned(),
        time,
        0,
        folder.to_owned(),
    )
    .unwrap();
    let controller = ConversionProcessController::default();
    controller
        .begin_queued_attempts(std::slice::from_ref(&request.id))
        .unwrap();
    match export_source_frame(&request, &controller).unwrap() {
        SourceFrameExportOutcome::Published { path } => path,
        SourceFrameExportOutcome::Cancelled => panic!("fixture export was cancelled"),
    }
}

fn vfr_source(folder: &Path) -> (PathBuf, Vec<image::RgbaImage>) {
    let originals: Vec<_> = (0..5_u8)
        .map(|index| {
            image::RgbaImage::from_fn(12, 8, |x, y| {
                image::Rgba([
                    index * 43,
                    u8::try_from(x * 17).unwrap(),
                    u8::try_from(y * 29).unwrap(),
                    255,
                ])
            })
        })
        .collect();
    for (index, pixels) in originals.iter().enumerate() {
        pixels
            .save(folder.join(format!("oracle-{index}.png")))
            .unwrap();
    }
    let source = folder.join("vfr.mov");
    ffmpeg(&[
        "-framerate".into(),
        "1000".into(),
        "-i".into(),
        folder.join("oracle-%d.png").into_os_string(),
        "-vf".into(),
        "setpts='if(eq(N,0),0,if(eq(N,1),40,if(eq(N,2),110,if(eq(N,3),320,470))))'".into(),
        "-frames:v".into(),
        "5".into(),
        "-fps_mode".into(),
        "passthrough".into(),
        "-c:v".into(),
        "png".into(),
        "-pix_fmt".into(),
        "rgba".into(),
        "-threads".into(),
        "1".into(),
        "-video_track_timescale".into(),
        "1000000".into(),
        source.clone().into_os_string(),
    ]);
    (source, originals)
}

#[test]
#[ignore = "requires selected FFmpeg runtime; source PNG pixel oracle"]
fn real_source_frame_export_selects_first_at_or_after_vfr_time_with_nonzero_origin() {
    let _serial = serial_export_processes();
    let directory = Directory::new();
    let (source, originals) = vfr_source(&directory.0);
    let offset = directory.0.join("offset.mov");
    ffmpeg(&[
        "-i".into(),
        source.clone().into_os_string(),
        "-c".into(),
        "copy".into(),
        "-output_ts_offset".into(),
        "5".into(),
        offset.clone().into_os_string(),
    ]);
    for input in [&source, &offset] {
        let metadata = probe(input);
        let origin = if input == &offset { 5_000_000 } else { 0 };
        let expected_pts: Vec<_> = [0, 40_000, 110_000, 320_000, 470_000]
            .map(|time| time + origin)
            .into();
        let observed_pts: Vec<_> = metadata["frames"]
            .as_array()
            .unwrap()
            .iter()
            .map(|frame| frame["pts"].as_i64().unwrap())
            .collect();
        assert_eq!(metadata["streams"][0]["time_base"], "1/1000000");
        assert_eq!(
            observed_pts, expected_pts,
            "fixture must retain the exact VFR clock"
        );
        assert_eq!(
            metadata["format"]["start_time"],
            if origin == 0 { "0.000000" } else { "5.000000" }
        );
        for (time, index) in [
            (0, 0),
            (40_000, 1),
            (40_001, 2),
            (100_000, 2),
            (110_000, 2),
            (320_000, 3),
        ] {
            let output = export(input, &directory.0, time);
            let actual = image::open(output).unwrap().to_rgba8();
            assert_eq!(
                actual,
                originals[index],
                "source={} time={time}",
                input.display()
            );
        }
        let request = SourceFrameExportRequest::new(
            "eof".into(),
            input.clone(),
            470_001,
            0,
            directory.0.clone(),
        )
        .unwrap();
        let controller = ConversionProcessController::default();
        controller
            .begin_queued_attempts(std::slice::from_ref(&request.id))
            .unwrap();
        assert!(export_source_frame(&request, &controller).is_err());
        assert!(!request.output_path.exists());
    }
}

#[test]
#[ignore = "requires selected FFmpeg runtime; source PNG full-resolution alpha oracle"]
fn real_source_frame_export_preserves_odd_full_resolution_and_alpha() {
    let _serial = serial_export_processes();
    let directory = Directory::new();
    let input = directory.0.join("large.png");
    let pixels = image::RgbaImage::from_fn(2579, 1441, |x, y| {
        image::Rgba([
            u8::try_from(x % 256).unwrap(),
            u8::try_from(y % 256).unwrap(),
            91,
            143,
        ])
    });
    pixels.save(&input).unwrap();
    let result = export(&input, &directory.0, 0);
    let actual = image::open(result).unwrap().to_rgba8();
    assert_eq!(actual.dimensions(), (2579, 1441));
    assert_eq!(actual, pixels);
}

#[test]
#[ignore = "requires selected FFmpeg runtime; source PNG SAR and orientation oracle"]
fn real_source_frame_export_normalizes_sar_after_source_rotation() {
    let _serial = serial_export_processes();
    let directory = Directory::new();
    let png = directory.0.join("source.png");
    let pixels = image::RgbaImage::from_fn(720, 480, |x, y| {
        if x < 360 && y < 240 {
            image::Rgba([255, 0, 0, 255])
        } else if y < 240 {
            image::Rgba([0, 255, 0, 255])
        } else if x < 360 {
            image::Rgba([0, 0, 255, 255])
        } else {
            image::Rgba([255, 255, 0, 255])
        }
    });
    pixels.save(&png).unwrap();
    let source = directory.0.join("sar.mov");
    ffmpeg(&[
        "-i".into(),
        png.into_os_string(),
        "-vf".into(),
        "setsar=8/9".into(),
        "-frames:v".into(),
        "1".into(),
        "-c:v".into(),
        "png".into(),
        "-pix_fmt".into(),
        "rgba".into(),
        source.clone().into_os_string(),
    ]);
    let rotated = directory.0.join("rotated.mov");
    ffmpeg(&[
        "-display_rotation".into(),
        "90".into(),
        "-i".into(),
        source.clone().into_os_string(),
        "-c".into(),
        "copy".into(),
        rotated.clone().into_os_string(),
    ]);
    let metadata = probe(&rotated);
    assert_eq!(metadata["streams"][0]["sample_aspect_ratio"], "8:9");
    assert!(
        metadata["streams"][0]["side_data_list"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["rotation"].as_i64() == Some(90)),
        "fixture must carry actual display rotation: {metadata}"
    );
    let normal = image::open(export(&source, &directory.0, 0))
        .unwrap()
        .to_rgba8();
    assert_eq!(normal.dimensions(), (720, 540));
    let turned = image::open(export(&rotated, &directory.0, 0))
        .unwrap()
        .to_rgba8();
    assert_eq!(turned.dimensions(), (540, 720));
    assert_eq!(*turned.get_pixel(20, 20), image::Rgba([0, 255, 0, 255]));
    assert_eq!(*turned.get_pixel(520, 20), image::Rgba([255, 255, 0, 255]));
    assert_eq!(*turned.get_pixel(20, 700), image::Rgba([255, 0, 0, 255]));
    assert_eq!(*turned.get_pixel(520, 700), image::Rgba([0, 0, 255, 255]));
}

#[test]
#[ignore = "requires selected FFmpeg runtime; source PNG sixteen-bit pixel oracle"]
fn real_source_frame_export_preserves_sixteen_bit_rgba_samples() {
    let _serial = serial_export_processes();
    let directory = Directory::new();
    let input = directory.0.join("sixteen-bit.png");
    let pixels = image::ImageBuffer::<image::Rgba<u16>, Vec<u16>>::from_fn(3, 1, |x, _| {
        [
            image::Rgba([1, 65534, 257, 4097]),
            image::Rgba([50000, 12345, 42, 65535]),
            image::Rgba([0, 32123, 65535, 32769]),
        ][usize::try_from(x).unwrap()]
    });
    pixels.save(&input).unwrap();
    let actual = image::open(export(&input, &directory.0, 0)).unwrap();
    assert_eq!(actual.color(), image::ColorType::Rgba16);
    assert_eq!(actual.to_rgba16(), pixels);
}

#[test]
#[ignore = "requires selected FFmpeg runtime; source PNG dynamic-size pixel oracle"]
fn real_source_frame_export_keeps_the_selected_dynamic_size_packet() {
    let _serial = serial_export_processes();
    let directory = Directory::new();
    let originals: Vec<_> = [(128, 96), (160, 80)]
        .into_iter()
        .enumerate()
        .map(|(index, (width, height))| {
            image::RgbaImage::from_fn(width, height, |x, y| {
                image::Rgba([
                    u8::try_from(x).unwrap(),
                    u8::try_from(y).unwrap(),
                    if index == 0 { 25 } else { 220 },
                    255,
                ])
            })
        })
        .collect();
    for (index, original) in originals.iter().enumerate() {
        original
            .save(directory.0.join(format!("dynamic-{index}.png")))
            .unwrap();
    }
    let input = directory.0.join("dynamic.mov");
    ffmpeg(&[
        "-framerate".into(),
        "25".into(),
        "-i".into(),
        directory.0.join("dynamic-%d.png").into_os_string(),
        "-c:v".into(),
        "copy".into(),
        input.clone().into_os_string(),
    ]);
    let metadata = probe(&input);
    let dimensions: Vec<_> = metadata["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| {
            (
                frame["width"].as_u64().unwrap(),
                frame["height"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(dimensions, [(128, 96), (160, 80)]);
    for (time, original) in [0, 40_000].into_iter().zip(originals) {
        let actual = image::open(export(&input, &directory.0, time))
            .unwrap()
            .to_rgba8();
        assert_eq!(actual, original, "source time {time}");
    }
}
