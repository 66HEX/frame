#[cfg(test)]
mod tests {
    use crate::conversion::args::{build_ffmpeg_args, build_output_path, validate_task_input};
    use crate::conversion::types::{ConversionConfig, MetadataConfig};
    use crate::conversion::utils::parse_time;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn contains_args(args: &[String], expected: &[&str]) -> bool {
        expected.iter().all(|e| args.iter().any(|a| a == e))
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
            hw_decode: false,
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
        assert_eq!(parse_time("00:10"), Some(10.0));
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

    #[test]
    fn test_default_stream_mapping_when_tracks_not_selected() {
        let mut config = sample_config("mp4");
        config.selected_audio_tracks = vec![];
        config.selected_subtitle_tracks = vec![];

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        assert!(contains_args(&args, &["-map", "0:v:0"]));
        assert!(contains_args(&args, &["-map", "0:a?"]));
        assert!(contains_args(&args, &["-map", "0:s?"]));
        assert!(contains_args(&args, &["-c:a", "aac"]));
        assert!(contains_args(&args, &["-b:a", "128k"]));
        assert!(contains_args(&args, &["-c:s", "mov_text"]));
    }

    #[test]
    fn test_subtitle_stream_not_mapped_when_burn_in_used_without_track_selection() {
        let mut config = sample_config("mp4");
        config.selected_subtitle_tracks = vec![];
        config.subtitle_burn_path = Some("/tmp/captions.srt".into());

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        assert!(!contains_args(&args, &["-map", "0:s?"]));
        assert!(!contains_args(&args, &["-c:s", "mov_text"]));
    }

    #[test]
    fn test_validate_rejects_ml_upscale_for_audio_only_container() {
        let mut config = sample_config("mp3");
        config.ml_upscale = Some("esrgan-2x".into());

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("frame-validate-{}.tmp", ts));
        fs::write(&path, b"test").unwrap();

        let result = validate_task_input(path.to_str().unwrap(), &config);
        let _ = fs::remove_file(&path);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod parsing_tests {
    use crate::conversion::utils::{DURATION_REGEX, FRAME_REGEX, TIME_REGEX, parse_time};

    #[test]
    fn time_regex_extracts_progress_time() {
        let ffmpeg_output = "frame=  120 fps= 30 q=23.0 size=    1024kB time=00:00:04.00 bitrate= 2097.2kbits/s speed=1.50x";
        let caps = TIME_REGEX.captures(ffmpeg_output).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "00:00:04.00");
    }

    #[test]
    fn time_regex_handles_long_duration() {
        let output = "time=02:15:33.45 bitrate=1024kbits/s";
        let caps = TIME_REGEX.captures(output).unwrap();
        let time_str = caps.get(1).unwrap().as_str();
        assert_eq!(time_str, "02:15:33.45");
        assert_eq!(parse_time(time_str), Some(8133.45));
    }

    #[test]
    fn time_regex_no_match_on_invalid() {
        let invalid_outputs = ["frame=100 fps=30 q=23.0", "time=invalid", ""];
        for output in invalid_outputs {
            assert!(
                TIME_REGEX.captures(output).is_none(),
                "Should not match: {}",
                output
            );
        }
    }

    #[test]
    fn duration_regex_extracts_total_duration() {
        let ffmpeg_output = "  Duration: 00:05:30.50, start: 0.000000, bitrate: 5000 kb/s";
        let caps = DURATION_REGEX.captures(ffmpeg_output).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "00:05:30.50");
    }

    #[test]
    fn duration_regex_real_ffmpeg_output() {
        let output = "Duration: 01:42:15.24, start: 0.000000, bitrate: 8500 kb/s";
        let caps = DURATION_REGEX.captures(output).unwrap();
        let duration_str = caps.get(1).unwrap().as_str();
        assert_eq!(parse_time(duration_str), Some(6135.24));
    }

    #[test]
    fn frame_regex_extracts_frame_count() {
        let outputs = [
            ("frame=  100 fps=30.0", "100"),
            ("frame=1500 fps=60", "1500"),
            ("frame=    1 fps=24", "1"),
            ("frame=999999 fps=29.97", "999999"),
        ];
        for (output, expected) in outputs {
            let caps = FRAME_REGEX.captures(output).unwrap();
            assert_eq!(caps.get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn frame_regex_upscale_pipeline_output() {
        let decode_output =
            "frame=  450 fps=120 q=-0.0 Lsize=N/A time=00:00:15.00 bitrate=N/A speed=4.00x";
        let caps = FRAME_REGEX.captures(decode_output).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "450");
    }

    #[test]
    fn parse_time_flexible_formats() {
        // Raw seconds
        assert_eq!(parse_time("30.5"), Some(30.5));
        assert_eq!(parse_time("120"), Some(120.0));

        // MM:SS
        assert_eq!(parse_time("01:30"), Some(90.0));
        assert_eq!(parse_time("10:05.5"), Some(605.5));

        // HH:MM:SS
        assert_eq!(parse_time("01:00:00"), Some(3600.0));
        assert_eq!(parse_time("00:01:00.00"), Some(60.0));
    }

    #[test]
    fn parse_time_invalid_formats() {
        assert_eq!(parse_time(""), None);
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time("12:34:56:78"), None);
    }

    #[test]
    fn progress_calculation_realistic() {
        let duration_line = "Duration: 00:10:00.00, start: 0.000000";
        let progress_line = "frame=  900 fps=30 time=00:00:30.00 bitrate=2048kbits/s";

        let total = DURATION_REGEX
            .captures(duration_line)
            .and_then(|c| c.get(1))
            .and_then(|m| parse_time(m.as_str()))
            .unwrap();

        let current = TIME_REGEX
            .captures(progress_line)
            .and_then(|c| c.get(1))
            .and_then(|m| parse_time(m.as_str()))
            .unwrap();

        let progress = (current / total * 100.0).min(100.0);
        assert!((progress - 5.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod utils_tests {
    use std::path::Path;

    use crate::conversion::utils::{
        is_audio_only_container, is_nvenc_codec, is_videotoolbox_codec, map_nvenc_preset,
        parse_frame_rate_string, parse_probe_bitrate, sanitize_external_tool_path,
    };

    #[test]
    fn frame_rate_fractional() {
        assert_eq!(
            parse_frame_rate_string(Some("30000/1001")),
            Some(29.97002997002997)
        );
        assert_eq!(
            parse_frame_rate_string(Some("24000/1001")),
            Some(23.976023976023978)
        );
        assert_eq!(parse_frame_rate_string(Some("60/1")), Some(60.0));
        assert_eq!(parse_frame_rate_string(Some("25/1")), Some(25.0));
    }

    #[test]
    fn frame_rate_direct_value() {
        assert_eq!(parse_frame_rate_string(Some("30")), Some(30.0));
        assert_eq!(parse_frame_rate_string(Some("23.976")), Some(23.976));
        assert_eq!(parse_frame_rate_string(Some("59.94")), Some(59.94));
    }

    #[test]
    fn frame_rate_edge_cases() {
        assert_eq!(parse_frame_rate_string(None), None);
        assert_eq!(parse_frame_rate_string(Some("")), None);
        assert_eq!(parse_frame_rate_string(Some("N/A")), None);
        assert_eq!(parse_frame_rate_string(Some("n/a")), None);
        assert_eq!(parse_frame_rate_string(Some("  ")), None);
        assert_eq!(parse_frame_rate_string(Some("0/0")), None);
    }

    #[test]
    fn bitrate_parsing() {
        assert_eq!(parse_probe_bitrate(Some("5000000")), Some(5000.0));
        assert_eq!(parse_probe_bitrate(Some("128000")), Some(128.0));
        assert_eq!(parse_probe_bitrate(Some("320000")), Some(320.0));
    }

    #[test]
    fn bitrate_edge_cases() {
        assert_eq!(parse_probe_bitrate(None), None);
        assert_eq!(parse_probe_bitrate(Some("N/A")), None);
        assert_eq!(parse_probe_bitrate(Some("")), None);
        assert_eq!(parse_probe_bitrate(Some("0")), None);
        assert_eq!(parse_probe_bitrate(Some("-1000")), None);
    }

    #[test]
    fn audio_only_containers() {
        let audio_containers = ["mp3", "wav", "flac", "aac", "m4a", "MP3", "FLAC"];
        let video_containers = ["mp4", "mkv", "webm", "mov", "avi"];

        for c in audio_containers {
            assert!(is_audio_only_container(c), "{} should be audio-only", c);
        }
        for c in video_containers {
            assert!(
                !is_audio_only_container(c),
                "{} should not be audio-only",
                c
            );
        }
    }

    #[test]
    fn nvenc_codec_detection() {
        assert!(is_nvenc_codec("h264_nvenc"));
        assert!(is_nvenc_codec("hevc_nvenc"));
        assert!(is_nvenc_codec("av1_nvenc"));
        assert!(!is_nvenc_codec("libx264"));
        assert!(!is_nvenc_codec("h264_videotoolbox"));
    }

    #[test]
    fn videotoolbox_codec_detection() {
        assert!(is_videotoolbox_codec("h264_videotoolbox"));
        assert!(is_videotoolbox_codec("hevc_videotoolbox"));
        assert!(!is_videotoolbox_codec("libx264"));
        assert!(!is_videotoolbox_codec("h264_nvenc"));
    }

    #[test]
    fn nvenc_preset_mapping() {
        assert_eq!(map_nvenc_preset("fast"), "fast");
        assert_eq!(map_nvenc_preset("medium"), "medium");
        assert_eq!(map_nvenc_preset("slow"), "slow");
        assert_eq!(map_nvenc_preset("ultrafast"), "fast");
        assert_eq!(map_nvenc_preset("superfast"), "fast");
        assert_eq!(map_nvenc_preset("veryfast"), "fast");
        assert_eq!(map_nvenc_preset("veryslow"), "slow");
        assert_eq!(map_nvenc_preset("slower"), "slow");
        assert_eq!(map_nvenc_preset("p1"), "p1");
        assert_eq!(map_nvenc_preset("p7"), "p7");
        assert_eq!(map_nvenc_preset("unknown"), "medium");
    }

    #[test]
    fn sanitize_external_tool_path_compatible_format() {
        #[cfg(windows)]
        {
            assert_eq!(
                sanitize_external_tool_path(Path::new(r"\\?\C:\Users\Karolina\models")),
                r"C:\Users\Karolina\models"
            );
            assert_eq!(
                sanitize_external_tool_path(Path::new(r"\\?\UNC\server\share\models")),
                r"\\server\share\models"
            );
            assert_eq!(
                sanitize_external_tool_path(Path::new(r"C:\Users\Karolina\models")),
                r"C:\Users\Karolina\models"
            );
        }

        #[cfg(not(windows))]
        {
            assert_eq!(
                sanitize_external_tool_path(Path::new("/tmp/frame/models")),
                "/tmp/frame/models"
            );
        }
    }
}

#[cfg(test)]
mod scenario_tests {
    use crate::conversion::args::build_ffmpeg_args;
    use crate::conversion::types::{ConversionConfig, CropConfig, MetadataConfig, MetadataMode};

    fn base_config() -> ConversionConfig {
        ConversionConfig {
            container: "mp4".into(),
            video_codec: "libx264".into(),
            video_bitrate_mode: "crf".into(),
            video_bitrate: "5000".into(),
            audio_codec: "aac".into(),
            audio_bitrate: "128".into(),
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![],
            selected_subtitle_tracks: vec![],
            subtitle_burn_path: None,
            resolution: "original".into(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "lanczos".into(),
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
            hw_decode: false,
        }
    }

    #[test]
    fn youtube_upload_1080p() {
        let mut config = base_config();
        config.resolution = "1080p".into();
        config.video_codec = "libx264".into();
        config.crf = 18;
        config.preset = "slow".into();
        config.audio_codec = "aac".into();
        config.audio_bitrate = "192".into();

        let args = build_ffmpeg_args("raw_footage.mov", "youtube_upload.mp4", &config);

        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"-crf".to_string()));
        assert!(args.contains(&"18".to_string()));
        assert!(args.contains(&"-preset".to_string()));
        assert!(args.contains(&"slow".to_string()));

        let vf_idx = args.iter().position(|a| a == "-vf").unwrap();
        assert!(args[vf_idx + 1].contains("1080"));
    }

    #[test]
    fn quick_share_720p_compressed() {
        let mut config = base_config();
        config.resolution = "720p".into();
        config.crf = 28;
        config.preset = "veryfast".into();
        config.audio_bitrate = "96".into();
        config.selected_audio_tracks = vec![1];

        let args = build_ffmpeg_args("video.mp4", "share.mp4", &config);

        assert!(args.contains(&"28".to_string()));
        assert!(args.contains(&"veryfast".to_string()));
        assert!(args.contains(&"96k".to_string()));
    }

    #[test]
    fn archive_high_quality_hevc() {
        let mut config = base_config();
        config.container = "mkv".into();
        config.video_codec = "libx265".into();
        config.crf = 16;
        config.preset = "slow".into();
        config.audio_codec = "flac".into();
        config.audio_bitrate = "0".into();
        config.selected_audio_tracks = vec![1];

        let args = build_ffmpeg_args("master.mov", "archive.mkv", &config);

        assert!(args.contains(&"libx265".to_string()));
        assert!(args.contains(&"16".to_string()));
        assert!(args.contains(&"flac".to_string()));
        assert!(args.last().unwrap().ends_with(".mkv"));
    }

    #[test]
    fn social_media_vertical_video() {
        let mut config = base_config();
        config.resolution = "custom".into();
        config.custom_width = Some("1080".into());
        config.custom_height = Some("1920".into());
        config.rotation = "90".into();
        config.crf = 20;

        let args = build_ffmpeg_args("horizontal.mp4", "vertical.mp4", &config);

        let vf_idx = args.iter().position(|a| a == "-vf").unwrap();
        let vf_arg = &args[vf_idx + 1];
        assert!(vf_arg.contains("transpose=1"));
        assert!(vf_arg.contains("1080") && vf_arg.contains("1920"));
    }

    #[test]
    fn clip_extraction_with_trim() {
        let mut config = base_config();
        config.start_time = Some("00:01:30.00".into());
        config.end_time = Some("00:02:45.00".into());
        config.crf = 18;

        let args = build_ffmpeg_args("long_video.mp4", "clip.mp4", &config);

        assert!(args.contains(&"-ss".to_string()));
        assert!(args.contains(&"00:01:30.00".to_string()));
        assert!(args.contains(&"-t".to_string()));
        assert!(args.contains(&"75.000".to_string()));
    }

    #[test]
    fn nvidia_hardware_encoding() {
        let mut config = base_config();
        config.video_codec = "h264_nvenc".into();
        config.quality = 60;
        config.preset = "medium".into();
        config.nvenc_spatial_aq = true;
        config.nvenc_temporal_aq = true;

        let args = build_ffmpeg_args("input.mp4", "output.mp4", &config);

        assert!(args.contains(&"h264_nvenc".to_string()));
        assert!(args.contains(&"-rc:v".to_string()));
        assert!(args.contains(&"vbr".to_string()));
        assert!(args.contains(&"-spatial_aq".to_string()));
        assert!(args.contains(&"1".to_string()));
        assert!(args.contains(&"-temporal_aq".to_string()));
    }

    #[test]
    fn macos_videotoolbox_encoding() {
        let mut config = base_config();
        config.container = "mov".into();
        config.video_codec = "hevc_videotoolbox".into();
        config.quality = 65;
        config.videotoolbox_allow_sw = true;

        let args = build_ffmpeg_args("input.mov", "output.mov", &config);

        assert!(args.contains(&"hevc_videotoolbox".to_string()));
        assert!(args.contains(&"-q:v".to_string()));
        assert!(args.contains(&"65".to_string()));
        assert!(args.contains(&"-allow_sw".to_string()));
    }

    #[test]
    fn audio_normalization_and_volume() {
        let mut config = base_config();
        config.audio_normalize = true;
        config.audio_volume = 120.0;

        let args = build_ffmpeg_args("quiet.mp4", "loud.mp4", &config);

        let af_idx = args.iter().position(|a| a == "-af").unwrap();
        let af_arg = &args[af_idx + 1];
        assert!(af_arg.contains("loudnorm"));
        assert!(af_arg.contains("volume=1.20"));
    }

    #[test]
    fn crop_and_flip_transformation() {
        let mut config = base_config();
        config.crop = Some(CropConfig {
            enabled: true,
            x: 100.0,
            y: 50.0,
            width: 1280.0,
            height: 720.0,
            source_width: Some(1920.0),
            source_height: Some(1080.0),
            aspect_ratio: Some("16:9".into()),
        });
        config.flip_horizontal = true;
        config.flip_vertical = false;

        let args = build_ffmpeg_args("full.mp4", "cropped.mp4", &config);

        let vf_idx = args.iter().position(|a| a == "-vf").unwrap();
        let vf_arg = &args[vf_idx + 1];
        assert!(vf_arg.contains("hflip"));
        assert!(vf_arg.contains("crop=1280:720:100:50"));
    }

    #[test]
    fn metadata_replacement() {
        let mut config = base_config();
        config.metadata = MetadataConfig {
            mode: MetadataMode::Replace,
            title: Some("My Video".into()),
            artist: Some("Creator".into()),
            album: Some("Collection".into()),
            genre: Some("Tutorial".into()),
            date: Some("2026".into()),
            comment: Some("Test comment".into()),
        };

        let args = build_ffmpeg_args("input.mp4", "output.mp4", &config);

        assert!(args.contains(&"-map_metadata".to_string()));
        assert!(args.contains(&"-1".to_string()));
        assert!(args.iter().any(|a| a.contains("title=My Video")));
        assert!(args.iter().any(|a| a.contains("artist=Creator")));
    }

    #[test]
    fn webm_vp9_opus_web_optimization() {
        let mut config = base_config();
        config.container = "webm".into();
        config.video_codec = "libvpx-vp9".into();
        config.audio_codec = "libopus".into();
        config.crf = 30;
        config.audio_bitrate = "96".into();
        config.resolution = "720p".into();
        config.selected_audio_tracks = vec![1];

        let args = build_ffmpeg_args("source.mp4", "web.webm", &config);

        assert!(args.contains(&"libvpx-vp9".to_string()));
        assert!(args.contains(&"libopus".to_string()));
        assert!(args.contains(&"30".to_string()));
        assert!(args.last().unwrap().ends_with(".webm"));
    }

    #[test]
    fn multi_track_audio_selection() {
        let mut config = base_config();
        config.selected_audio_tracks = vec![1, 3];

        let args = build_ffmpeg_args("multi_audio.mkv", "output.mp4", &config);

        let map_positions: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "-map")
            .map(|(i, _)| i)
            .collect();

        assert!(map_positions.len() >= 2);
    }

    #[test]
    fn fps_conversion_60_to_30() {
        let mut config = base_config();
        config.fps = "30".into();

        let args = build_ffmpeg_args("60fps.mp4", "30fps.mp4", &config);

        assert!(args.contains(&"-r".to_string()));
        assert!(args.contains(&"30".to_string()));
    }

    #[test]
    fn av1_modern_codec() {
        let mut config = base_config();
        config.container = "mkv".into();
        config.video_codec = "libsvtav1".into();
        config.crf = 28;
        config.preset = "6".into();

        let args = build_ffmpeg_args("input.mp4", "av1.mkv", &config);

        assert!(args.contains(&"libsvtav1".to_string()));
        assert!(args.contains(&"28".to_string()));
    }
}

#[cfg(test)]
mod hwaccel_tests {
    use crate::conversion::args::build_ffmpeg_args;
    use crate::conversion::types::{ConversionConfig, MetadataConfig};
    use crate::conversion::utils::get_hwaccel_args;

    fn hwaccel_config(codec: &str) -> ConversionConfig {
        ConversionConfig {
            container: "mp4".into(),
            video_codec: codec.into(),
            video_bitrate_mode: "crf".into(),
            video_bitrate: "5000".into(),
            audio_codec: "aac".into(),
            audio_bitrate: "128".into(),
            audio_channels: "original".into(),
            audio_volume: 100.0,
            selected_audio_tracks: vec![],
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
            hw_decode: true,
        }
    }

    #[test]
    fn get_hwaccel_args_nvenc_codecs() {
        for codec in ["h264_nvenc", "hevc_nvenc", "av1_nvenc"] {
            let args = get_hwaccel_args(codec);
            assert_eq!(args.len(), 4);
            assert_eq!(args[0], "-hwaccel");
            assert_eq!(args[1], "cuda");
            assert_eq!(args[2], "-hwaccel_output_format");
            assert_eq!(args[3], "cuda");
        }
    }

    #[test]
    fn get_hwaccel_args_videotoolbox_codecs() {
        for codec in ["h264_videotoolbox", "hevc_videotoolbox"] {
            let args = get_hwaccel_args(codec);
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], "-hwaccel");
            assert_eq!(args[1], "videotoolbox");
        }
    }

    #[test]
    fn get_hwaccel_args_software_codecs() {
        for codec in ["libx264", "libx265", "libsvtav1", "libvpx-vp9"] {
            let args = get_hwaccel_args(codec);
            assert!(args.is_empty());
        }
    }

    #[test]
    fn hwaccel_cuda_in_ffmpeg_args() {
        let config = hwaccel_config("h264_nvenc");
        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);

        // hwaccel args must come BEFORE -i
        let hwaccel_idx = args.iter().position(|a| a == "-hwaccel").unwrap();
        let i_idx = args.iter().position(|a| a == "-i").unwrap();
        assert!(hwaccel_idx < i_idx, "hwaccel must come before -i");
        assert!(args.contains(&"cuda".to_string()));
        assert!(args.contains(&"-hwaccel_output_format".to_string()));
    }

    #[test]
    fn hwaccel_videotoolbox_in_ffmpeg_args() {
        let config = hwaccel_config("hevc_videotoolbox");
        let args = build_ffmpeg_args("in.mov", "out.mov", &config);

        let hwaccel_idx = args.iter().position(|a| a == "-hwaccel").unwrap();
        let i_idx = args.iter().position(|a| a == "-i").unwrap();
        assert!(hwaccel_idx < i_idx);
        assert!(args.contains(&"videotoolbox".to_string()));
    }

    #[test]
    fn hwaccel_disabled() {
        let mut config = hwaccel_config("h264_nvenc");
        config.hw_decode = false;

        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);
        assert!(!args.contains(&"-hwaccel".to_string()));
    }

    #[test]
    fn hwaccel_ignored_for_software_encoder() {
        let config = hwaccel_config("libx264");
        let args = build_ffmpeg_args("in.mp4", "out.mp4", &config);
        assert!(!args.contains(&"-hwaccel".to_string()));
    }
}
