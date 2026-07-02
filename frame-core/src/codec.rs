use crate::types::ConversionConfig;
use crate::utils::{
    is_nvenc_codec, is_svt_av1_codec, is_videotoolbox_codec, map_nvenc_preset, map_svt_av1_preset,
};

pub fn add_video_codec_args(args: &mut Vec<String>, config: &ConversionConfig) {
    let is_still_image_codec = matches!(
        config.video_codec.as_str(),
        "png" | "mjpeg" | "libwebp" | "bmp" | "tiff"
    );

    let is_nvenc = is_nvenc_codec(&config.video_codec);
    let is_svt_av1 = is_svt_av1_codec(&config.video_codec);
    let is_videotoolbox = is_videotoolbox_codec(&config.video_codec);

    args.push("-c:v".to_string());
    args.push(config.video_codec.clone());

    if is_still_image_codec {
        add_still_image_codec_args(args, config);
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
        } else if is_svt_av1 {
            map_svt_av1_preset(&config.preset)
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

fn add_still_image_codec_args(args: &mut Vec<String>, config: &ConversionConfig) {
    match config.video_codec.as_str() {
        "mjpeg" => {
            args.push("-q:v".to_string());
            args.push(jpeg_quality_to_qscale(config.image_jpeg_quality).to_string());
            args.push("-huffman".to_string());
            args.push(normalize_jpeg_huffman(&config.image_jpeg_huffman).to_string());
        }
        "libwebp" => {
            args.push("-lossless".to_string());
            args.push(if config.image_webp_lossless { "1" } else { "0" }.to_string());
            args.push("-quality".to_string());
            args.push(config.image_webp_quality.min(100).to_string());
            args.push("-compression_level".to_string());
            args.push(config.image_webp_compression.min(6).to_string());
            args.push("-preset".to_string());
            args.push(normalize_webp_preset(&config.image_webp_preset).to_string());
        }
        "png" => {
            args.push("-compression_level".to_string());
            args.push(config.image_png_compression.min(9).to_string());
            args.push("-pred".to_string());
            args.push(normalize_png_prediction(&config.image_png_prediction).to_string());
        }
        "tiff" => {
            args.push("-compression_algo".to_string());
            args.push(normalize_tiff_compression(&config.image_tiff_compression).to_string());
        }
        _ => {}
    }
}

#[must_use]
pub fn jpeg_quality_to_qscale(quality: u32) -> u32 {
    let quality = quality.clamp(1, 100);
    2 + ((100 - quality) * 29 + 49) / 99
}

fn normalize_jpeg_huffman(value: &str) -> &'static str {
    match value {
        "default" => "default",
        _ => "optimal",
    }
}

fn normalize_webp_preset(value: &str) -> &'static str {
    match value {
        "picture" => "picture",
        "photo" => "photo",
        "drawing" => "drawing",
        "icon" => "icon",
        "text" => "text",
        _ => "default",
    }
}

fn normalize_png_prediction(value: &str) -> &'static str {
    match value {
        "none" => "none",
        "sub" => "sub",
        "up" => "up",
        "avg" => "avg",
        "mixed" => "mixed",
        _ => "paeth",
    }
}

fn normalize_tiff_compression(value: &str) -> &'static str {
    match value {
        "raw" => "raw",
        "lzw" => "lzw",
        "deflate" => "deflate",
        _ => "packbits",
    }
}

pub fn add_audio_codec_args(args: &mut Vec<String>, config: &ConversionConfig) {
    args.push("-c:a".to_string());
    args.push(config.audio_codec.clone());

    let lossless_audio_codecs = ["flac", "alac", "pcm_s16le"];
    let is_lossless = lossless_audio_codecs.contains(&config.audio_codec.as_str());

    if !is_lossless {
        let use_vbr =
            config.audio_bitrate_mode == "vbr" && audio_codec_supports_vbr(&config.audio_codec);
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

/// Returns true if the encoder supports Frame's quality-based VBR mode.
///
/// Native `FFmpeg` `aac` has an experimental `-q:a` path but produces
/// inconsistent results, so Frame restricts VBR to well-behaved encoders.
#[must_use]
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
        .map_or(fallback, |v| v.clamp(min, max))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_quality_to_qscale_maps_best_quality_to_low_quantizer() {
        assert_eq!(jpeg_quality_to_qscale(100), 2);
    }

    #[test]
    fn jpeg_quality_to_qscale_maps_lowest_quality_to_high_quantizer() {
        assert_eq!(jpeg_quality_to_qscale(1), 31);
    }
}
