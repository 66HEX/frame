use crate::conversion::types::{ConversionConfig, VOLUME_EPSILON};

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

    if let Some(crop) = &config.crop {
        if crop.enabled {
            let crop_width = crop.width.max(1.0).round() as i32;
            let crop_height = crop.height.max(1.0).round() as i32;
            let crop_x = crop.x.max(0.0).round() as i32;
            let crop_y = crop.y.max(0.0).round() as i32;
            filters.push(format!(
                "crop={}:{}:{}:{}",
                crop_width, crop_height, crop_x, crop_y
            ));
        }
    }

    if let Some(burn_path) = &config.subtitle_burn_path {
        if !burn_path.is_empty() {
            let escaped_path = burn_path.replace('\\', "/").replace(':', "\\:");
            filters.push(format!("subtitles='{}'", escaped_path));
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
                    "scale={w}:{h}:force_original_aspect_ratio=decrease{algo},pad={w}:{h}:(ow-iw)/2:(oh-ih)/2",
                    w = w,
                    h = h,
                    algo = algorithm
                )
            } else if w == "-1" && h == "-1" {
                "scale=-1:-1".to_string()
            } else {
                format!("scale={}:{}{}", w, h, algorithm)
            }
        } else {
            match config.resolution.as_str() {
                "1080p" => format!("scale=-2:1080{}", algorithm),
                "720p" => format!("scale=-2:720{}", algorithm),
                "480p" => format!("scale=-2:480{}", algorithm),
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
        filters.push(format!("volume={:.2}", volume_factor));
    }

    filters
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversion::types::CropConfig;

    fn default_config() -> ConversionConfig {
        ConversionConfig {
            container: "mp4".to_string(),
            video_codec: "libx264".to_string(),
            video_bitrate_mode: "crf".to_string(),
            video_bitrate: "5000".to_string(),
            audio_codec: "aac".to_string(),
            audio_bitrate: "192".to_string(),
            audio_channels: "original".to_string(),
            audio_volume: 100.0,
            audio_normalize: false,
            selected_audio_tracks: vec![],
            selected_subtitle_tracks: vec![],
            subtitle_burn_path: None,
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
            metadata: Default::default(),
            rotation: "0".to_string(),
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
}
