use crate::conversion::types::{ConversionConfig, VOLUME_EPSILON};

/// Converts a CSS hex color (`#RRGGBB`) to an ASS/SSA color string (`&H00BBGGRR`).
fn hex_to_ass_color(hex: &str) -> Option<String> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(format!("&H00{b:02X}{g:02X}{r:02X}"))
}

fn rounded_i32(value: f64, min_value: f64) -> i32 {
    let clamped = value
        .max(min_value)
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "value is rounded and clamped into i32 range first"
    )]
    let converted = clamped as i32;
    converted
}

pub fn build_video_filters(config: &ConversionConfig, include_scale: bool) -> Vec<String> {
    let mut filters = Vec::new();

    if config.flip_horizontal {
        filters.push("hflip".to_string());
    }
    if config.flip_vertical {
        filters.push("vflip".to_string());
    }

    match config.rotation.as_str() {
        "90" => filters.push("transpose=1".to_string()),
        "180" => filters.push("transpose=1,transpose=1".to_string()),
        "270" => filters.push("transpose=2".to_string()),
        _ => {}
    }

    if let Some(crop) = &config.crop
        && crop.enabled
    {
        let crop_width = rounded_i32(crop.width, 1.0);
        let crop_height = rounded_i32(crop.height, 1.0);
        let crop_x = rounded_i32(crop.x, 0.0);
        let crop_y = rounded_i32(crop.y, 0.0);
        filters.push(format!("crop={crop_width}:{crop_height}:{crop_x}:{crop_y}"));
    }

    if let Some(burn_path) = &config.subtitle_burn_path
        && !burn_path.is_empty()
    {
        let escaped_path = burn_path
            .replace('\\', "/")
            .replace(':', "\\:")
            .replace('\'', "\\'")
            .replace('[', "\\[")
            .replace(']', "\\]")
            .replace(',', "\\,");

        let mut style_parts: Vec<String> = Vec::new();

        if let Some(font) = &config.subtitle_font_name
            && !font.trim().is_empty()
        {
            style_parts.push(format!("FontName={}", font.trim()));
        }

        if let Some(font_size) = &config.subtitle_font_size
            && let Ok(parsed) = font_size.trim().parse::<u16>()
            && (8..=120).contains(&parsed)
        {
            style_parts.push(format!("Fontsize={parsed}"));
        }

        if let Some(color) = &config.subtitle_font_color
            && let Some(ass) = hex_to_ass_color(color)
        {
            style_parts.push(format!("PrimaryColour={ass}"));
        }

        if let Some(color) = &config.subtitle_outline_color
            && let Some(ass) = hex_to_ass_color(color)
        {
            style_parts.push(format!("OutlineColour={ass}"));
        }

        if let Some(pos) = &config.subtitle_position {
            // FFmpeg's subtitles filter interprets force_style Alignment using
            // legacy SSA-style values in this context:
            // - bottom center: 2
            // - top center: 6
            // - middle center: 10
            let alignment = match pos.as_str() {
                "top" => "6",
                "middle" => "10",
                _ => "2",
            };
            style_parts.push(format!("Alignment={alignment}"));
        }

        if style_parts.is_empty() {
            filters.push(format!("subtitles='{escaped_path}'"));
        } else {
            let style = style_parts.join(",");
            filters.push(format!("subtitles='{escaped_path}':force_style='{style}'"));
        }
    }

    if include_scale && (config.resolution != "original" || config.resolution == "custom") {
        let algorithm = match config.scaling_algorithm.as_str() {
            "lanczos" => ":flags=lanczos",
            "bilinear" => ":flags=bilinear",
            "nearest" => ":flags=neighbor",
            "bicubic" => ":flags=bicubic",
            _ => "",
        };

        let scale_filter = if config.resolution == "custom" {
            let w = config.custom_width.as_deref().unwrap_or("-1");
            let h = config.custom_height.as_deref().unwrap_or("-1");
            if w != "-1" && h != "-1" {
                format!(
                    "scale={w}:{h}:force_original_aspect_ratio=decrease{algorithm},pad={w}:{h}:(ow-iw)/2:(oh-ih)/2"
                )
            } else if w == "-1" && h == "-1" {
                "scale=-1:-1".to_string()
            } else {
                format!("scale={w}:{h}{algorithm}")
            }
        } else {
            match config.resolution.as_str() {
                "1080p" => format!("scale=-2:1080{algorithm}"),
                "720p" => format!("scale=-2:720{algorithm}"),
                "480p" => format!("scale=-2:480{algorithm}"),
                _ => "scale=-1:-1".to_string(),
            }
        };

        filters.push(scale_filter);
    }

    filters
}

pub fn build_audio_filters(config: &ConversionConfig) -> Vec<String> {
    let mut filters = Vec::new();

    if config.audio_normalize {
        filters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
    }

    if (config.audio_volume - 100.0).abs() > VOLUME_EPSILON {
        let volume_factor = config.audio_volume / 100.0;
        filters.push(format!("volume={volume_factor:.2}"));
    }

    filters
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversion::types::{CropConfig, MetadataConfig};

    fn default_config() -> ConversionConfig {
        ConversionConfig {
            processing_mode: "reencode".to_string(),
            container: "mp4".to_string(),
            video_codec: "libx264".to_string(),
            video_bitrate_mode: "crf".to_string(),
            video_bitrate: "5000".to_string(),
            audio_codec: "aac".to_string(),
            audio_bitrate: "192".to_string(),
            audio_bitrate_mode: "bitrate".to_string(),
            audio_quality: "4".to_string(),
            audio_channels: "original".to_string(),
            audio_volume: 100.0,
            audio_normalize: false,
            selected_audio_tracks: vec![],
            selected_subtitle_tracks: vec![],
            subtitle_burn_path: None,
            subtitle_font_name: None,
            subtitle_font_size: None,
            subtitle_font_color: None,
            subtitle_outline_color: None,
            subtitle_position: None,
            resolution: "original".to_string(),
            custom_width: None,
            custom_height: None,
            scaling_algorithm: "lanczos".to_string(),
            fps: "original".to_string(),
            crf: 23,
            quality: 50,
            preset: "medium".to_string(),
            start_time: None,
            end_time: None,
            metadata: MetadataConfig::default(),
            rotation: "0".to_string(),
            flip_horizontal: false,
            flip_vertical: false,
            ml_upscale: None,
            crop: None,
            nvenc_spatial_aq: false,
            nvenc_temporal_aq: false,
            videotoolbox_allow_sw: false,
            hw_decode: false,
            pixel_format: "auto".to_string(),
            gif_colors: 256,
            gif_dither: "sierra2_4a".to_string(),
            gif_loop: 0,
        }
    }

    #[test]
    fn test_empty_video_filters() {
        let config = default_config();
        let filters = build_video_filters(&config, true);
        assert!(filters.is_empty());
    }

    #[test]
    fn test_flip_filters() {
        let mut config = default_config();
        config.flip_horizontal = true;
        config.flip_vertical = true;
        let filters = build_video_filters(&config, true);
        assert_eq!(filters, vec!["hflip", "vflip"]);
    }

    #[test]
    fn test_rotation_filter() {
        let mut config = default_config();
        config.rotation = "90".to_string();
        let filters = build_video_filters(&config, true);
        assert_eq!(filters, vec!["transpose=1"]);
    }

    #[test]
    fn test_crop_filter() {
        let mut config = default_config();
        config.crop = Some(CropConfig {
            enabled: true,
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 200.0,
            source_width: None,
            source_height: None,
            aspect_ratio: None,
        });
        let filters = build_video_filters(&config, true);
        assert_eq!(filters, vec!["crop=100:200:10:20"]);
    }

    #[test]
    fn test_audio_normalize_filter() {
        let mut config = default_config();
        config.audio_normalize = true;
        let filters = build_audio_filters(&config);
        assert_eq!(filters, vec!["loudnorm=I=-16:TP=-1.5:LRA=11"]);
    }

    #[test]
    fn test_audio_volume_filter() {
        let mut config = default_config();
        config.audio_volume = 150.0;
        let filters = build_audio_filters(&config);
        assert_eq!(filters, vec!["volume=1.50"]);
    }

    #[test]
    fn test_subtitle_burn_path_escaping() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("C:\\Media\\John's [cut],final.srt".to_string());

        let filters = build_video_filters(&config, true);

        assert_eq!(
            filters,
            vec!["subtitles='C\\:/Media/John\\'s \\[cut\\]\\,final.srt'"]
        );
    }

    #[test]
    fn test_subtitle_position_top_maps_to_alignment_6() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());
        config.subtitle_position = Some("top".to_string());

        let filters = build_video_filters(&config, true);

        assert_eq!(
            filters,
            vec!["subtitles='/tmp/sub.srt':force_style='Alignment=6'"]
        );
    }

    #[test]
    fn test_subtitle_position_middle_maps_to_alignment_10() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());
        config.subtitle_position = Some("middle".to_string());

        let filters = build_video_filters(&config, true);

        assert_eq!(
            filters,
            vec!["subtitles='/tmp/sub.srt':force_style='Alignment=10'"]
        );
    }

    #[test]
    fn test_subtitle_font_size_adds_force_style() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());
        config.subtitle_font_size = Some("28".to_string());

        let filters = build_video_filters(&config, true);

        assert_eq!(
            filters,
            vec!["subtitles='/tmp/sub.srt':force_style='Fontsize=28'"]
        );
    }
}
