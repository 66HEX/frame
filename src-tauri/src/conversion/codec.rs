use crate::conversion::types::ConversionConfig;
use crate::conversion::utils::{is_nvenc_codec, is_videotoolbox_codec, map_nvenc_preset};

pub fn add_video_codec_args(args: &mut Vec<String>, config: &ConversionConfig) {
    let is_still_image_codec = matches!(
        config.video_codec.as_str(),
        "png" | "mjpeg" | "libwebp" | "bmp" | "tiff"
    );

    let is_nvenc = is_nvenc_codec(&config.video_codec);
    let is_videotoolbox = is_videotoolbox_codec(&config.video_codec);

    args.push("-c:v".to_string());
    args.push(config.video_codec.clone());

    if is_still_image_codec {
        if config.video_codec == "mjpeg" || config.video_codec == "libwebp" {
            args.push("-q:v".to_string());
            args.push(config.quality.to_string());
        }
        return;
    }

    if config.video_bitrate_mode == "bitrate" {
        args.push("-b:v".to_string());
        args.push(format!("{}k", config.video_bitrate));
    } else if is_nvenc {
        let cq = 52_u32.saturating_sub(config.quality / 2).clamp(1, 51);
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
    let is_lossless = lossless_audio_codecs.contains(&config.audio_codec.as_str());

    if !is_lossless {
        let use_vbr = config.audio_bitrate_mode == "vbr"
            && audio_codec_supports_vbr(&config.audio_codec);
        if use_vbr {
            add_audio_vbr_args(args, config);
        } else {
            args.push("-b:a".to_string());
            args.push(format!("{}k", config.audio_bitrate));
        }
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

/// Returns true if the encoder supports a quality-based VBR mode exposed by
/// Frame. Native FFmpeg `aac` has an experimental `-q:a` path but produces
/// inconsistent results, so we restrict VBR to the two well-behaved encoders.
pub fn audio_codec_supports_vbr(codec: &str) -> bool {
    matches!(codec, "mp3" | "libmp3lame" | "libfdk_aac")
}

fn add_audio_vbr_args(args: &mut Vec<String>, config: &ConversionConfig) {
    match config.audio_codec.as_str() {
        // libmp3lame: -q:a 0..9  (0 = best, ~245 kbps; 9 = worst, ~65 kbps)
        "mp3" | "libmp3lame" => {
            let q = parse_quality(&config.audio_quality, 0, 9, 4);
            args.push("-q:a".to_string());
            args.push(q.to_string());
        }
        // libfdk_aac: -vbr 1..5  (1 = ~32 kbps/ch, 5 = ~112 kbps/ch)
        "libfdk_aac" => {
            let q = parse_quality(&config.audio_quality, 1, 5, 4);
            args.push("-vbr".to_string());
            args.push(q.to_string());
        }
        _ => {
            // Caller guarantees the codec supports VBR; fall back to CBR defensively.
            args.push("-b:a".to_string());
            args.push(format!("{}k", config.audio_bitrate));
        }
    }
}

fn parse_quality(raw: &str, min: u8, max: u8, fallback: u8) -> u8 {
    raw.trim()
        .parse::<u8>()
        .ok()
        .map(|v| v.clamp(min, max))
        .unwrap_or(fallback)
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
