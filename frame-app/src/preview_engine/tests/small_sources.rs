use super::*;
use frame_core::aspect::{DisplayRatio, SampleAspectRatio};

#[test]
fn source_bounds_are_independent_of_preview_target_bounds() {
    let mut config = real_video_preview_config(PathBuf::from("/tmp/source.mkv"));
    for (width, height) in [
        (1, 1),
        (400, 4),
        (4, 400),
        (15, 15),
        (30_720, 1),
        (1, 30_720),
        (30_720, 30_720),
    ] {
        config.source_width = Some(width);
        config.source_height = Some(height);
        assert!(config.validate().is_ok(), "source {width}x{height}");
    }
    for (width, height) in [(0, 1), (1, 0), (30_721, 1), (1, 30_721), (u32::MAX, 16)] {
        config.source_width = Some(width);
        config.source_height = Some(height);
        assert!(config.validate().is_err(), "source {width}x{height}");
    }
    for (width, height, valid) in [
        (None, None, true),
        (Some(1), None, false),
        (None, Some(1), false),
    ] {
        config.source_width = width;
        config.source_height = height;
        assert_eq!(config.validate().is_ok(), valid);
    }
    config.source_width = Some(1);
    config.source_height = Some(1);
    for dimension in [0, 1, 15, 16, 17, 3840, 3841] {
        config.max_width = dimension;
        config.max_height = 16;
        assert_eq!(config.validate().is_ok(), (16..=3840).contains(&dimension));
        config.max_width = 16;
        config.max_height = dimension;
        assert_eq!(config.validate().is_ok(), (16..=3840).contains(&dimension));
    }
    config.max_width = 17;
    config.max_height = 19;
    assert_eq!(
        config.target_dimensions(),
        PreviewDimensions {
            width: 16,
            height: 18
        }
    );
}

fn assert_small_frame(
    frame: &PreviewRenderedFrame,
    dimensions: (u32, u32),
    sar: DisplayRatio,
    token: u64,
    pixels: &[u8],
) {
    assert_eq!((frame.width, frame.height), dimensions);
    assert_eq!(frame.request_token, token);
    assert_eq!(frame.presentation.pixel_aspect_ratio, sar);
    assert_eq!(
        frame.presentation.output_raster_aspect_ratio,
        DisplayRatio::new(u128::from(dimensions.0), u128::from(dimensions.1))
    );
    assert_eq!(frame.presentation.transform, PreviewTransform::default());
    let image = frame.render_image();
    let actual = image.as_bytes(0).expect("BGRA pixels");
    assert_eq!(actual.len(), pixels.len());
    assert!(
        actual == pixels,
        "tiny frame pixels differ at {:?}",
        actual.iter().zip(pixels).position(|(a, b)| a != b)
    );
}

fn red_padded(width: u32, height: u32, out_width: u32, out_height: u32) -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..out_height {
        for x in 0..out_width {
            pixels.extend_from_slice(if x < width && y < height {
                &[0, 0, 255, 255]
            } else {
                &[0, 0, 0, 255]
            });
        }
    }
    pixels
}

fn tiny_video(path: &std::path::Path, png: &std::path::Path) {
    let status = crate::process_launch::status(
        Command::new(crate::runtime_binaries::ffmpeg_executable()).args([
            "-v",
            "error",
            "-y",
            "-loop",
            "1",
            "-framerate",
            "4",
            "-i",
            png.to_str().unwrap(),
            "-frames:v",
            "4",
            "-vf",
            "setsar=2/1",
            "-c:v",
            "ffv1",
            "-pix_fmt",
            "bgra",
            path.to_str().unwrap(),
        ]),
    )
    .unwrap();
    assert!(status.success());
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_tiny_sources_keep_exact_pixels_and_presentation_through_seek_and_rotation() {
    for (width, height) in [(1, 1), (400, 4), (4, 400)] {
        let root = std::env::temp_dir().join(format!(
            "frame-tiny-preview-{}-{width}x{height}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let png = root.join("source.png");
        image::RgbaImage::from_pixel(width, height, image::Rgba([255, 0, 0, 255]))
            .save(&png)
            .unwrap();
        let mut config = real_video_preview_config(png.clone());
        config.source_kind = PreviewSourceKind::Image;
        config.source_width = Some(width);
        config.source_height = Some(height);
        config.max_width = 640;
        config.max_height = 480;
        config.conversion_config.container = "png".into();
        config.conversion_config.video_codec = "png".into();
        config.request_token = 51;
        let image_session = PreviewSession::start(config.clone()).unwrap();
        let image_frame = image_session.latest_frame().unwrap().frame;
        assert_small_frame(
            &image_frame,
            (width, height),
            DisplayRatio::ONE,
            51,
            &red_padded(width, height, width, height),
        );
        image_session.stop().unwrap();

        let video = root.join("source.mkv");
        tiny_video(&video, &png);
        config.path = video;
        config.source_kind = PreviewSourceKind::Video;
        config.source_sample_aspect_ratio = SampleAspectRatio::new(2, 1);
        config.conversion_config = default_core_config();
        config.duration_seconds = 1.0;
        config.request_token = 52;
        let out_width = width + width % 2;
        let out_height = height + height % 2;
        let expected = red_padded(width, height, out_width, out_height);
        let session = PreviewSession::start(config.clone()).unwrap();
        assert_small_frame(
            &session.latest_frame().unwrap().frame,
            (out_width, out_height),
            DisplayRatio::new(2, 1).unwrap(),
            52,
            &expected,
        );
        session.command(PreviewCommand::SeekPrecise(0.5)).unwrap();
        assert_small_frame(
            &session.latest_frame().unwrap().frame,
            (out_width, out_height),
            DisplayRatio::new(2, 1).unwrap(),
            52,
            &expected,
        );
        config.conversion_config.rotation = "90".into();
        config.request_token = 53;
        session.reconfigure(config).unwrap();
        let rotated = red_padded(height, width, out_height, out_width);
        assert_small_frame(
            &session.latest_frame().unwrap().frame,
            (out_height, out_width),
            DisplayRatio::new(1, 2).unwrap(),
            53,
            &rotated,
        );
        session.stop().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
#[ignore = "requires FFmpeg/FFprobe; run with --ignored"]
fn ffmpeg_tiny_source_crop_and_overlay_keep_the_accepted_output_raster() {
    let root = std::env::temp_dir().join(format!("frame-tiny-composition-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.png");
    let logo = root.join("logo.png");
    image::RgbaImage::from_pixel(400, 4, image::Rgba([255, 0, 0, 255]))
        .save(&source)
        .unwrap();
    image::RgbaImage::from_pixel(100, 10, image::Rgba([0, 0, 255, 255]))
        .save(&logo)
        .unwrap();
    let video = root.join("source.mkv");
    tiny_video(&video, &source);
    let mut config = real_video_preview_config(video);
    config.source_width = Some(400);
    config.source_height = Some(4);
    config.source_sample_aspect_ratio = SampleAspectRatio::new(2, 1);
    config.max_width = 640;
    config.max_height = 480;
    config.request_token = 81;
    config.conversion_config.crop = Some(frame_core::types::CropConfig {
        enabled: true,
        x: 1.0,
        y: 1.0,
        width: 200.0,
        height: 3.0,
        ..frame_core::types::CropConfig::default()
    });
    let session = PreviewSession::start(config.clone()).unwrap();
    assert_small_frame(
        &session.latest_frame().unwrap().frame,
        (200, 4),
        DisplayRatio::new(2, 1).unwrap(),
        81,
        &red_padded(200, 3, 200, 4),
    );
    config.conversion_config.crop = None;
    config.conversion_config.overlay = Some(frame_core::types::OverlayConfig {
        enabled: true,
        path: logo.to_str().unwrap().into(),
        x: 0.5,
        y: 0.5,
        width: 0.05,
        opacity: 1.0,
        anchor: "center".into(),
    });
    config.request_token = 82;
    session.reconfigure(config).unwrap();
    let mut expected = red_padded(400, 4, 400, 4);
    for y in 1..3 {
        for x in 190..210 {
            let offset = (y * 400 + x) * 4;
            expected[offset..offset + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
    }
    assert_small_frame(
        &session.latest_frame().unwrap().frame,
        (400, 4),
        DisplayRatio::new(2, 1).unwrap(),
        82,
        &expected,
    );
    session.stop().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
