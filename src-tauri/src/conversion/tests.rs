#[cfg(test)]
mod tests {
    use crate::conversion::ffmpeg::{build_ffmpeg_args, build_output_path};
    use crate::conversion::types::{ConversionConfig, MetadataConfig};

    fn contains_args(args: &[String], expected: &[&str]) -> bool {
        expected.iter().all(|e| args.iter().any(|a| a == e))
    }

    fn parse_time(time_str: &str) -> Option<f64> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 3 {
            return None;
        }
        let h: f64 = parts[0].parse().ok()?;
        let m: f64 = parts[1].parse().ok()?;
        let s: f64 = parts[2].parse().ok()?;
        Some(h * 3600.0 + m * 60.0 + s)
    }

    fn sample_config(container: &str) -> ConversionConfig {
        ConversionConfig {
            container: container.into(),
            video_codec: "libx264".into(),
            video_bitrate_mode: "crf".into(),
            video_bitrate: "5000".into(),
            audio_codec: "aac".into(),
            audio_bitrate: "128".into(),
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![1],
            selected_subtitle_tracks: vec![],
            subtitle_burn_path: None,
            resolution: "original".into(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "bicubic".into(),
            fps: "original".into(),
            crf: 23,
            quality: 50,
            preset: "medium".into(),
            start_time: None,
            end_time: None,
            audio_normalize: false,
            metadata: MetadataConfig::default(),
            rotation: "0".into(),
            flip_horizontal: false,
            flip_vertical: false,
            ml_upscale: None,
            crop: None,
            nvenc_spatial_aq: false,
            nvenc_temporal_aq: false,
            videotoolbox_allow_sw: false,
        }
    }

    #[test]
    fn test_default_mp4_h264() {
        let config = sample_config("mp4");
        let args = build_ffmpeg_args("input.mov", "output.mp4", &config);

        assert_eq!(args[0], "-i");
        assert_eq!(args[1], "input.mov");

        assert!(contains_args(&args, &["-c:v", "libx264"]));
        assert!(contains_args(&args, &["-c:a", "aac"]));

        assert!(contains_args(&args, &["-crf", "23"]));
        assert!(contains_args(&args, &["-preset", "medium"]));

        assert!(!args.iter().any(|a| a == "-vf"));
    }

    #[test]
    fn test_resolution_scaling_1080p() {
        let mut config = sample_config("mp4");
        config.resolution = "1080p".into();

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_index = args.iter().position(|r| r == "-vf").unwrap();
        assert_eq!(args[vf_index + 1], "scale=-2:1080:flags=bicubic");
    }

    #[test]
    fn test_resolution_scaling_720p() {
        let mut config = sample_config("mp4");
        config.resolution = "720p".into();

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_index = args.iter().position(|r| r == "-vf").unwrap();
        assert_eq!(args[vf_index + 1], "scale=-2:720:flags=bicubic");
    }

    #[test]
    fn test_resolution_scaling_480p() {
        let mut config = sample_config("mp4");
        config.resolution = "480p".into();

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_index = args.iter().position(|r| r == "-vf").unwrap();
        assert_eq!(args[vf_index + 1], "scale=-2:480:flags=bicubic");
    }

    #[test]
    fn test_high_quality_h265() {
        let mut config = sample_config("mkv");
        config.video_codec = "libx265".into();
        config.audio_codec = "ac3".into();
        config.audio_bitrate = "192".into();
        config.crf = 18;
        config.preset = "slow".into();

        let args = build_ffmpeg_args("raw.mov", "archive.mkv", &config);

        assert!(contains_args(&args, &["-c:v", "libx265"]));
        assert!(contains_args(&args, &["-crf", "18"]));
        assert!(contains_args(&args, &["-preset", "slow"]));
        assert!(contains_args(&args, &["-c:a", "ac3"]));
        assert_eq!(args.last().unwrap(), "archive.mkv");
    }

    #[test]
    fn test_web_optimization_vp9() {
        let mut config = sample_config("webm");
        config.video_codec = "libvpx-vp9".into();
        config.audio_codec = "libopus".into();
        config.audio_bitrate = "96".into();
        config.crf = 30;

        let args = build_ffmpeg_args("clip.mp4", "web.webm", &config);

        assert!(contains_args(&args, &["-c:v", "libvpx-vp9"]));
        assert!(contains_args(&args, &["-c:a", "libopus"]));
        assert!(args.last().unwrap().ends_with(".webm"));
    }

    #[test]
    fn test_time_parsing() {
        assert_eq!(parse_time("00:00:10.50"), Some(10.5));
        assert_eq!(parse_time("01:00:00.00"), Some(3600.0));
        assert_eq!(parse_time("00:01:05.10"), Some(65.1));

        assert_eq!(parse_time("invalid"), None);
        assert_eq!(parse_time("00:10"), None);
    }

    #[test]
    fn test_build_output_path_with_custom_name() {
        let custom = build_output_path(
            "/Users/hex/Videos/clip.mov",
            "mp4",
            Some("final_render".into()),
        );
        assert_eq!(custom, "/Users/hex/Videos/final_render.mp4");

        let default = build_output_path("/tmp/sample.mov", "mp4", None);
        assert_eq!(default, "/tmp/sample.mov_converted.mp4");
    }

    #[test]
    fn test_custom_resolution_and_fps() {
        let mut config = sample_config("mp4");
        config.resolution = "custom".into();
        config.custom_width = Some("1280".into());
        config.custom_height = Some("720".into());
        config.scaling_algorithm = "lanczos".into();
        config.fps = "60".into();

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        let vf_arg = args.iter().find(|a| a.starts_with("scale=")).unwrap();
        assert_eq!(
            vf_arg,
            "scale=1280:720:force_original_aspect_ratio=decrease:flags=lanczos,pad=1280:720:(ow-iw)/2:(oh-ih)/2"
        );

        let fps_index = args.iter().position(|a| a == "-r").unwrap();
        assert_eq!(args[fps_index + 1], "60");
    }

    #[test]
    fn test_video_bitrate_mode() {
        let mut config = sample_config("mp4");
        config.video_bitrate_mode = "bitrate".into();
        config.video_bitrate = "2500".into();

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        assert!(contains_args(&args, &["-b:v", "2500k"]));
        assert!(!args.iter().any(|a| a == "-crf"));
    }

    #[test]
    fn test_av1_codec() {
        let mut config = sample_config("mkv");
        config.video_codec = "libsvtav1".into();

        let args = build_ffmpeg_args("in.mp4", "out.mkv", &config);

        assert!(contains_args(&args, &["-c:v", "libsvtav1"]));
    }

    #[test]
    fn test_hardware_encoder_nvenc() {
        let mut config = sample_config("mp4");
        config.video_codec = "h264_nvenc".into();
        config.quality = 50; // Should map to CQ ~27 (52 - 25)

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        assert!(contains_args(&args, &["-c:v", "h264_nvenc"]));
        assert!(contains_args(&args, &["-rc:v", "vbr"]));
        assert!(contains_args(&args, &["-cq:v", "27"]));
        assert!(contains_args(&args, &["-preset", "medium"]));
        assert!(!args.iter().any(|a| a == "-crf"));

        config.video_codec = "hevc_nvenc".into();
        let args_hevc = build_ffmpeg_args("in.mp4", "out.mp4", &config);
        assert!(contains_args(&args_hevc, &["-c:v", "hevc_nvenc"]));
        assert!(contains_args(&args_hevc, &["-cq:v", "27"]));

        config.preset = "veryslow".into();
        let args_remapped = build_ffmpeg_args("in.mp4", "out.mp4", &config);
        assert!(contains_args(&args_remapped, &["-preset", "slow"]));
    }

    #[test]
    fn test_hardware_encoder_videotoolbox() {
        let mut config = sample_config("mov");
        config.video_codec = "h264_videotoolbox".into();
        config.quality = 55;

        let args = build_ffmpeg_args("in.mov", "out.mov", &config);

        assert!(contains_args(&args, &["-c:v", "h264_videotoolbox"]));
        assert!(contains_args(&args, &["-q:v", "55"]));
        assert!(!args.iter().any(|a| a == "-crf"));
        assert!(!args.iter().any(|a| a == "-preset"));

        config.video_codec = "hevc_videotoolbox".into();
        let args_hevc = build_ffmpeg_args("in.mov", "out.mov", &config);
        assert!(contains_args(&args_hevc, &["-c:v", "hevc_videotoolbox"]));
        assert!(contains_args(&args_hevc, &["-q:v", "55"]));
    }

    #[test]
    fn test_nvenc_option_flags() {
        let mut config = sample_config("mp4");
        config.video_codec = "h264_nvenc".into();
        config.nvenc_spatial_aq = true;
        config.nvenc_temporal_aq = true;

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);
        assert!(contains_args(&args, &["-spatial_aq", "1"]));
        assert!(contains_args(&args, &["-temporal_aq", "1"]));
    }

    #[test]
    fn test_videotoolbox_option_flags() {
        let mut config = sample_config("mov");
        config.video_codec = "h264_videotoolbox".into();
        config.videotoolbox_allow_sw = true;

        let args = build_ffmpeg_args("in.mov", "out.mov", &config);
        assert!(contains_args(&args, &["-allow_sw", "1"]));
    }

    #[test]
    fn test_scaling_algorithms() {
        let algos = vec![
            ("lanczos", ":flags=lanczos"),
            ("bicubic", ":flags=bicubic"),
            ("nearest", ":flags=neighbor"),
        ];

        for (algo_name, expected_flag) in algos {
            let mut config = sample_config("mp4");
            config.resolution = "720p".into();
            config.scaling_algorithm = algo_name.into();

            let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);
            let vf_arg = args.iter().find(|a| a.starts_with("scale=")).unwrap();
            assert!(
                vf_arg.ends_with(expected_flag),
                "Algorithm {} expected flag {}, got {}",
                algo_name,
                expected_flag,
                vf_arg
            );
        }
    }

    #[test]
    fn test_audio_volume_filter() {
        let config = sample_config("mp4");
        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);
        assert!(!args.iter().any(|a| a == "-af"), "no -af at 100% volume");

        let mut config_reduced = sample_config("mp4");
        config_reduced.audio_volume = 50.0;
        let args_reduced = build_ffmpeg_args("in.mp4", "out.mp4", &config_reduced);
        let af_index = args_reduced.iter().position(|r| r == "-af").unwrap();
        assert_eq!(args_reduced[af_index + 1], "volume=0.50");

        let mut config_boosted = sample_config("mp4");
        config_boosted.audio_volume = 150.0;
        let args_boosted = build_ffmpeg_args("in.mp4", "out.mp4", &config_boosted);
        let af_index = args_boosted.iter().position(|r| r == "-af").unwrap();
        assert_eq!(args_boosted[af_index + 1], "volume=1.50");
    }
}
