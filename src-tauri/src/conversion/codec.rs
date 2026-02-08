use crate::conversion::types::ConversionConfig;
use crate::conversion::utils::{is_nvenc_codec, is_videotoolbox_codec, map_nvenc_preset};

pub fn add_video_codec_args(args: &mut Vec<String>, config: &ConversionConfig) {
    let is_nvenc = is_nvenc_codec(&config.video_codec);
    let is_videotoolbox = is_videotoolbox_codec(&config.video_codec);

    args.push("-c:v".to_string());
    args.push(config.video_codec.clone());

    if config.video_bitrate_mode == "bitrate" {
        args.push("-b:v".to_string());
        args.push(format!("{}k", config.video_bitrate));
    } else if is_nvenc {
        let cq = (52.0 - (config.quality as f64 / 2.0))
            .round()
            .clamp(1.0, 51.0) as u32;
        args.push("-rc:v".to_string());
        args.push("vbr".to_string());
        args.push("-cq:v".to_string());
        args.push(cq.to_string());
    } else if is_videotoolbox {
        args.push("-q:v".to_string());
        args.push(config.quality.to_string());
    } else {
        args.push("-crf".to_string());
        args.push(config.crf.to_string());
    }

    if !is_videotoolbox {
        args.push("-preset".to_string());
        let preset_value = if is_nvenc {
            map_nvenc_preset(&config.preset)
        } else {
            config.preset.clone()
        };
        args.push(preset_value);
    }

    if is_nvenc {
        if config.nvenc_spatial_aq {
            args.push("-spatial_aq".to_string());
            args.push("1".to_string());
        }
        if config.nvenc_temporal_aq {
            args.push("-temporal_aq".to_string());
            args.push("1".to_string());
        }
    }

    if is_videotoolbox && config.videotoolbox_allow_sw {
        args.push("-allow_sw".to_string());
        args.push("1".to_string());
    }
}

pub fn add_audio_codec_args(args: &mut Vec<String>, config: &ConversionConfig) {
    args.push("-c:a".to_string());
    args.push(config.audio_codec.clone());

    let lossless_audio_codecs = ["flac", "alac", "pcm_s16le"];
    if !lossless_audio_codecs.contains(&config.audio_codec.as_str()) {
        args.push("-b:a".to_string());
        args.push(format!("{}k", config.audio_bitrate));
    }

    match config.audio_channels.as_str() {
        "stereo" => {
            args.push("-ac".to_string());
            args.push("2".to_string());
        }
        "mono" => {
            args.push("-ac".to_string());
            args.push("1".to_string());
        }
        _ => {}
    }
}

pub fn add_subtitle_codec_args(args: &mut Vec<String>, config: &ConversionConfig) {
    let codec = match config.container.as_str() {
        "mkv" => Some("copy"),
        "mp4" | "mov" => Some("mov_text"),
        "webm" => Some("webvtt"),
        _ => None,
    };

    if let Some(codec) = codec {
        args.push("-c:s".to_string());
        args.push(codec.to_string());
    }
}

pub fn add_fps_args(args: &mut Vec<String>, config: &ConversionConfig) {
    if config.fps != "original" {
        args.push("-r".to_string());
        args.push(config.fps.clone());
    }
}
