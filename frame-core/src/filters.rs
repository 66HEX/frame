use crate::{
    dimensions::{FilterGeometry, filter_geometry, rotated_source_dimensions},
    media_filters::{
        build_audio_effect_filters, build_video_post_scale_filters, build_video_pre_scale_filters,
    },
    media_rules::is_image_container,
    types::ConversionConfig,
    utils::{format_ffmpeg_seconds, parse_time},
};

pub const EVEN_DIMENSIONS_FILTER: &str = "pad=ceil(iw/2)*2:ceil(ih/2)*2:0:0";
pub const PREVIEW_OUTPUT_LABEL: &str = "preview_v";
pub const VIDEO_OUTPUT_LABEL: &str = "vout";

/// Source geometry used to preserve exact pixels when padding subsampled frames.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VisualFilterContext {
    pub source_dimensions: Option<(u32, u32)>,
}

impl VisualFilterContext {
    #[must_use]
    pub fn new(width: Option<u32>, height: Option<u32>) -> Self {
        Self {
            source_dimensions: width.zip(height).filter(|&(w, h)| w > 0 && h > 0),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisualFilterBase {
    Source,
    Video,
    Image,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VisualFilterProfile {
    ExportVideo,
    ExportImage,
    PreviewLowRes {
        base: VisualFilterBase,
        width: u32,
        height: u32,
        fps: u32,
        source_time_seconds: f64,
    },
}

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

/// Builds filters without source metadata. For exact custom padding, use
/// [`build_visual_filter_chain_with_context`]; unknown geometry retains the
/// legacy dynamic padding expression.
#[must_use]
pub fn build_video_filters(config: &ConversionConfig, include_scale: bool) -> Vec<String> {
    build_video_filters_at_time(
        config,
        include_scale,
        export_subtitle_time(config),
        VisualFilterContext::default(),
    )
}

fn export_subtitle_time(config: &ConversionConfig) -> f64 {
    config
        .start_time
        .as_deref()
        .and_then(parse_time)
        .unwrap_or(0.0)
}

fn build_video_filters_at_time(
    config: &ConversionConfig,
    include_scale: bool,
    source_time_seconds: f64,
    context: VisualFilterContext,
) -> Vec<String> {
    let mut filters = Vec::new();
    let is_image = is_image_container(&config.container);

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
        let source_width = rotated_source_dimensions(config, context.source_dimensions)
            .map(|dimensions| dimensions.0);
        filters.push(exact_crop_filter(
            crop_width,
            crop_height,
            crop_x,
            crop_y,
            source_width,
        ));
    }

    filters.extend(build_video_pre_scale_filters(
        &config.video_filters,
        is_image,
    ));

    if include_scale && (config.resolution != "original" || config.resolution == "custom") {
        filters.push(build_resolution_scale_filter(config, context));
    }

    filters.extend(build_video_post_scale_filters(&config.video_filters));

    if let Some(burn_path) = &config.subtitle_burn_path
        && !burn_path.is_empty()
    {
        let escaped_path = escape_filter_option_value(burn_path);

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
            filters.push(format!("subtitles=filename={escaped_path}"));
        } else {
            let style = escape_filter_option_value(&style_parts.join(","));
            filters.push(format!(
                "subtitles=filename={escaped_path}:force_style={style}"
            ));
        }
    }

    apply_subtitle_timebase(&mut filters, source_time_seconds);
    filters
}

// FFmpeg 8.1.2 crop computes chroma byte offsets before dividing by hsub.
// An unaligned x therefore splits 16-bit samples. Keep exact luma/alpha from
// the requested crop and copy only chroma from a safely aligned second crop.
fn exact_crop_filter(width: i32, height: i32, x: i32, y: i32, source_width: Option<u32>) -> String {
    let crop = format!("crop={width}:{height}:{x}:{y}:exact=1");
    let effective_x =
        source_width.map(|source| i64::from(x).min((i64::from(source) - i64::from(width)).max(0)));
    if x == 0 || effective_x.is_some_and(|value| value % 4 == 0) {
        return crop;
    }
    // Clamp before aligning: crop may clamp an out-of-bounds requested x to an
    // unaligned iw-ow, even if the requested position itself was aligned.
    format!(
        "split[frame_crop_exact_src][frame_crop_chroma_src];[frame_crop_exact_src]{crop}[frame_crop_exact];[frame_crop_chroma_src]crop={width}:{height}:'floor(min({x},iw-ow)/hsub)*hsub':{y}:exact=1[frame_crop_chroma];[frame_crop_exact][frame_crop_chroma]blend=all_mode=normal:c1_opacity=0:c2_opacity=0"
    )
}

// Filter options pass through two parsers: the filtergraph, then AVOption.
// Escape each layer separately; Command passes these arguments without a shell.
fn escape_filter_option_value(value: &str) -> String {
    let escape = |value: &str, separators: &[char]| {
        let mut escaped = String::with_capacity(value.len());
        for character in value.chars() {
            if character.is_whitespace() || separators.contains(&character) {
                escaped.push('\\');
            }
            escaped.push(character);
        }
        escaped
    };
    let option = escape(value, &['\\', '\'', ':']);
    escape(&option, &['\\', '\'', '[', ']', ',', ';'])
}

fn build_resolution_scale_filter(
    config: &ConversionConfig,
    context: VisualFilterContext,
) -> String {
    let algorithm = match config.scaling_algorithm.as_str() {
        "lanczos" => ":flags=lanczos",
        "bilinear" => ":flags=bilinear",
        "nearest" => ":flags=neighbor",
        "bicubic" => ":flags=bicubic",
        _ => "",
    };

    if config.resolution == "custom" {
        let width = config.custom_width.as_deref().unwrap_or("-1");
        let height = config.custom_height.as_deref().unwrap_or("-1");
        return custom_resolution_scale_filter(
            width,
            height,
            algorithm,
            filter_geometry(config, context.source_dimensions, true),
        );
    }

    match config.resolution.as_str() {
        "1080p" => format!("scale=-2:1080{algorithm}"),
        "720p" => format!("scale=-2:720{algorithm}"),
        "480p" => format!("scale=-2:480{algorithm}"),
        _ => "scale=-1:-1".to_string(),
    }
}

fn custom_resolution_scale_filter(
    width: &str,
    height: &str,
    algorithm: &str,
    geometry: Option<FilterGeometry>,
) -> String {
    if width != "-1" && height != "-1" {
        let scale =
            format!("scale={width}:{height}:force_original_aspect_ratio=decrease{algorithm}");
        if let Some(geometry) = geometry {
            if geometry.scaled == geometry.canvas {
                return scale;
            }
            let (w, h) = geometry.canvas;
            let x = (w - geometry.scaled.0) / 2;
            let y = (h - geometry.scaled.1) / 2;
            // pad rounds dimensions and placement to chroma boundaries. Keep
            // its simple path only when every boundary is aligned, including
            // the inner aspect-fit raster and the requested center offset.
            if [w, h, geometry.scaled.0, geometry.scaled.1, x, y]
                .iter()
                .any(|value| value % 4 != 0)
            {
                return format!("{scale},{}", exact_padding_filter(w, h, x, y, "custom"));
            }
        }
        format!(
            "scale={width}:{height}:force_original_aspect_ratio=decrease{algorithm},pad=ceil({width}/hsub)*hsub:ceil({height}/vsub)*vsub:(ow-iw)/2:(oh-ih)/2,crop={width}:{height}:0:0:exact=1"
        )
    } else if width == "-1" && height == "-1" {
        "scale=-1:-1".to_string()
    } else {
        format!("scale={width}:{height}{algorithm}")
    }
}

// xstack copies the entire raster, including the final subsampled row/column.
// A separate pad branch donates frame properties that xstack does not preserve.
// blend at zero top opacity copies bottom pixels while retaining top properties.
fn exact_padding_filter(width: u32, height: u32, x: u32, y: u32, stage: &str) -> String {
    let p = format!("frame_pad_{stage}");
    let dummy_x = u64::from(width) + 4;
    let dummy_y = u64::from(height) + 4;
    format!(
        "split=3[{p}_main][{p}_dummy][{p}_props];[{p}_dummy]scale=4:4:flags=neighbor[{p}_corner];[{p}_main][{p}_corner]xstack=inputs=2:layout={x}_{y}|{dummy_x}_{dummy_y}:fill=black,crop={width}:{height}:0:0:exact=1[{p}_pixels];[{p}_props]pad=ceil({width}/4)*4:ceil({height}/4)*4:0:0,crop={width}:{height}:0:0:exact=1[{p}_meta];[{p}_meta][{p}_pixels]blend=all_mode=normal:all_opacity=0"
    )
}

#[must_use]
pub fn build_encode_video_filters(config: &ConversionConfig, include_scale: bool) -> Vec<String> {
    build_encode_video_filters_at_time(
        config,
        include_scale,
        export_subtitle_time(config),
        VisualFilterContext::default(),
    )
}

fn build_encode_video_filters_at_time(
    config: &ConversionConfig,
    include_scale: bool,
    source_time_seconds: f64,
    context: VisualFilterContext,
) -> Vec<String> {
    let mut filters =
        build_video_filters_at_time(config, include_scale, source_time_seconds, context);
    let geometry = filter_geometry(config, context.source_dimensions, include_scale);
    if let Some(geometry) = geometry
        && (geometry.canvas.0 % 2 != 0 || geometry.canvas.1 % 2 != 0)
        && let (Some(width), Some(height)) = (
            geometry.canvas.0.checked_add(geometry.canvas.0 % 2),
            geometry.canvas.1.checked_add(geometry.canvas.1 % 2),
        )
    {
        filters.push(exact_padding_filter(width, height, 0, 0, "even"));
    } else if geometry.is_none() {
        // Metadata-free callers retain the legacy dynamic guard. Known even
        // rasters need no pad, which also avoids rounding four-pixel chroma.
        filters.push(EVEN_DIMENSIONS_FILTER.to_string());
    }
    filters
}

#[must_use]
pub fn build_visual_filter_chain(
    config: &ConversionConfig,
    profile: VisualFilterProfile,
) -> Vec<String> {
    build_visual_filter_chain_with_context(config, profile, VisualFilterContext::default())
}

/// Builds visual filters using source geometry for exact aspect-fit padding.
#[must_use]
pub fn build_visual_filter_chain_with_context(
    config: &ConversionConfig,
    profile: VisualFilterProfile,
    context: VisualFilterContext,
) -> Vec<String> {
    match profile {
        VisualFilterProfile::ExportVideo => {
            build_encode_video_filters_at_time(config, true, export_subtitle_time(config), context)
        }
        VisualFilterProfile::ExportImage => {
            build_video_filters_at_time(config, true, export_subtitle_time(config), context)
        }
        VisualFilterProfile::PreviewLowRes {
            base,
            width,
            height,
            fps,
            source_time_seconds,
        } => {
            let mut filters = match base {
                VisualFilterBase::Source => Vec::new(),
                VisualFilterBase::Video => {
                    build_encode_video_filters_at_time(config, true, source_time_seconds, context)
                }
                VisualFilterBase::Image => {
                    build_video_filters_at_time(config, true, source_time_seconds, context)
                }
            };
            filters.extend(preview_low_res_filters(width, height, fps));
            filters
        }
    }
}

#[must_use]
pub fn build_visual_filter_complex(
    config: &ConversionConfig,
    profile: VisualFilterProfile,
) -> String {
    build_visual_filter_complex_with_context(config, profile, VisualFilterContext::default())
}

/// Builds a visual graph with the same exact padding geometry as export.
#[must_use]
pub fn build_visual_filter_complex_with_context(
    config: &ConversionConfig,
    profile: VisualFilterProfile,
    context: VisualFilterContext,
) -> String {
    match profile {
        VisualFilterProfile::ExportVideo | VisualFilterProfile::ExportImage => {
            build_export_filter_complex(
                config,
                &build_visual_filter_chain_with_context(config, profile, context),
                VIDEO_OUTPUT_LABEL,
            )
        }
        VisualFilterProfile::PreviewLowRes {
            base,
            width,
            height,
            fps,
            source_time_seconds,
        } => build_preview_filter_complex(
            config,
            base,
            width,
            height,
            fps,
            source_time_seconds,
            context,
        ),
    }
}

#[must_use]
pub fn has_overlay(config: &ConversionConfig) -> bool {
    config
        .overlay
        .as_ref()
        .is_some_and(|overlay| overlay.enabled && !overlay.path.trim().is_empty())
}

#[must_use]
pub fn build_overlay_filter_complex(config: &ConversionConfig) -> String {
    let filters = build_video_filters(config, true);
    build_overlay_filter_complex_with_filters(config, &filters, VIDEO_OUTPUT_LABEL)
}

#[must_use]
pub fn build_encode_overlay_filter_complex(config: &ConversionConfig) -> String {
    let filters = build_encode_video_filters(config, true);
    build_overlay_filter_complex_with_filters(config, &filters, VIDEO_OUTPUT_LABEL)
}

fn build_overlay_filter_complex_with_filters(
    config: &ConversionConfig,
    filters: &[String],
    output_label: &str,
) -> String {
    let Some(overlay) = &config.overlay else {
        return labeled_filter_chain(filters, output_label);
    };

    let base_chain = labeled_filter_chain(filters, "base");
    let x = overlay.x.clamp(0.0, 1.0);
    let y = overlay.y.clamp(0.0, 1.0);
    let width = overlay.width.clamp(0.03, 0.8);
    let opacity = overlay.opacity.clamp(0.0, 1.0);

    format!(
        "{base_chain};[base]split[base_ref][base_out];[1:v:0]format=rgba,colorchannelmixer=aa={opacity:.3}[overlay_src];[overlay_src][base_ref]scale=w='min(rw*{width:.6},rh*iw/ih)':h=-1[overlay_scaled];[base_out][overlay_scaled]overlay=x='min(max(main_w*{x:.6}-overlay_w/2,0),main_w-overlay_w)':y='min(max(main_h*{y:.6}-overlay_h/2,0),main_h-overlay_h)':format=auto[{output_label}]"
    )
}

fn build_export_filter_complex(
    config: &ConversionConfig,
    filters: &[String],
    output_label: &str,
) -> String {
    if has_overlay(config) {
        build_overlay_filter_complex_with_filters(config, filters, output_label)
    } else {
        labeled_filter_chain(filters, output_label)
    }
}

fn build_preview_filter_complex(
    config: &ConversionConfig,
    base: VisualFilterBase,
    width: u32,
    height: u32,
    fps: u32,
    source_time_seconds: f64,
    context: VisualFilterContext,
) -> String {
    let base_filters = match base {
        VisualFilterBase::Source => Vec::new(),
        VisualFilterBase::Video => {
            build_encode_video_filters_at_time(config, true, source_time_seconds, context)
        }
        VisualFilterBase::Image => {
            build_video_filters_at_time(config, true, source_time_seconds, context)
        }
    };
    let preview_filters = preview_low_res_filters(width, height, fps);
    let preview_chain = preview_filters.join(",");

    if base != VisualFilterBase::Source && has_overlay(config) {
        let graph =
            build_overlay_filter_complex_with_filters(config, &base_filters, "preview_export");
        format!("{graph};[preview_export]{preview_chain}[{PREVIEW_OUTPUT_LABEL}]")
    } else {
        let mut filters = base_filters;
        filters.extend(preview_filters);
        labeled_filter_chain(&filters, PREVIEW_OUTPUT_LABEL)
    }
}

fn apply_subtitle_timebase(filters: &mut Vec<String>, source_time_seconds: f64) {
    if !source_time_seconds.is_finite() || source_time_seconds <= 0.0 {
        return;
    }

    let Some(subtitle_index) = filters
        .iter()
        .position(|filter| filter.starts_with("subtitles="))
    else {
        return;
    };

    let offset = format_ffmpeg_seconds(source_time_seconds);
    filters.insert(subtitle_index, format!("setpts=PTS+{offset}/TB"));
    filters.insert(subtitle_index + 2, format!("setpts=PTS-{offset}/TB"));
}

fn labeled_filter_chain(filters: &[String], output_label: &str) -> String {
    if filters.is_empty() {
        format!("[0:v:0]null[{output_label}]")
    } else {
        format!("[0:v:0]{}[{output_label}]", filters.join(","))
    }
}

fn preview_low_res_filters(width: u32, height: u32, fps: u32) -> Vec<String> {
    vec![
        format!("fps={fps}"),
        format!("scale={width}:{height}"),
        "setsar=1".to_string(),
        "format=bgra".to_string(),
    ]
}

#[must_use]
pub fn build_audio_filters(config: &ConversionConfig) -> Vec<String> {
    build_audio_effect_filters(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CropConfig, MetadataConfig, OverlayConfig};

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
            opus_vbr: true,
            audio_quality: "4".to_string(),
            audio_channels: "original".to_string(),
            audio_volume: 100.0,
            audio_normalize: false,
            video_filters: crate::types::VideoFiltersConfig::default(),
            audio_filters: crate::types::AudioFiltersConfig::default(),
            selected_audio_tracks: vec![],
            selected_subtitle_tracks: vec![],
            external_subtitle_tracks: vec![],
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
            crop: None,
            overlay: None,
            nvenc_spatial_aq: false,
            nvenc_temporal_aq: false,
            videotoolbox_allow_sw: false,
            hw_decode: false,
            pixel_format: "auto".to_string(),
            image_jpeg_quality: 85,
            image_jpeg_huffman: "optimal".to_string(),
            image_webp_lossless: false,
            image_webp_quality: 75,
            image_webp_compression: 4,
            image_webp_preset: "default".to_string(),
            image_png_compression: 9,
            image_png_prediction: "paeth".to_string(),
            image_tiff_compression: "packbits".to_string(),
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
    fn encode_video_filters_add_even_dimensions_guard_for_original_resolution() {
        let config = default_config();
        let filters = build_encode_video_filters(&config, true);
        assert_eq!(filters, vec![EVEN_DIMENSIONS_FILTER]);
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
        assert_eq!(filters.len(), 1);
        assert!(filters[0].contains("crop=100:200:10:20:exact=1"));
        assert!(filters[0].contains("floor(min(10,iw-ow)/hsub)*hsub"));
    }

    #[test]
    fn encode_video_filters_append_even_dimensions_guard_after_user_filters() {
        let mut config = default_config();
        config.crop = Some(CropConfig {
            enabled: true,
            x: 10.0,
            y: 20.0,
            width: 101.0,
            height: 201.0,
            source_width: None,
            source_height: None,
            aspect_ratio: None,
        });

        let filters = build_encode_video_filters(&config, true);

        assert_eq!(filters.len(), 2);
        assert!(filters[0].contains("crop=101:201:10:20:exact=1"));
        assert!(filters[1].contains("crop=102:202:0:0:exact=1"));
        assert!(filters[1].ends_with("blend=all_mode=normal:all_opacity=0"));
    }

    #[test]
    fn exact_crop_repair_uses_effective_clamped_origin() {
        assert_eq!(
            exact_crop_filter(128, 100, 4, 5, Some(320)),
            "crop=128:100:4:5:exact=1"
        );
        let graph = exact_crop_filter(127, 100, 400, 5, Some(320));
        assert!(graph.contains("crop=127:100:400:5:exact=1"));
        assert!(graph.contains("floor(min(400,iw-ow)/hsub)*hsub"));
        assert!(graph.ends_with("blend=all_mode=normal:c1_opacity=0:c2_opacity=0"));
        assert!(exact_crop_filter(127, 100, 4, 5, None).contains("blend="));
        assert_eq!(
            exact_crop_filter(127, 100, 0, 5, None),
            "crop=127:100:0:5:exact=1"
        );
    }

    #[test]
    fn known_geometry_preserves_centering_without_changing_even_identity() {
        let mut config = default_config();
        let context = VisualFilterContext::new(Some(320), Some(178));
        assert!(
            build_visual_filter_chain_with_context(
                &config,
                VisualFilterProfile::ExportVideo,
                context,
            )
            .is_empty()
        );
        config.resolution = "custom".to_string();
        config.custom_width = Some("128".to_string());
        config.custom_height = Some("128".to_string());
        let graph = build_visual_filter_chain_with_context(
            &config,
            VisualFilterProfile::ExportVideo,
            context,
        )
        .join(",");
        assert!(graph.contains("layout=0_28|132_132"));
        assert!(graph.contains("crop=128:128:0:0:exact=1"));
        assert_eq!(graph.matches("blend=").count(), 1);

        config.custom_width = Some("127".to_string());
        config.custom_height = Some("129".to_string());
        let graph = build_visual_filter_chain_with_context(
            &config,
            VisualFilterProfile::ExportVideo,
            context,
        )
        .join(",");
        assert!(graph.contains("layout=0_29|131_133"));
        assert!(graph.contains("frame_pad_custom_main"));
        assert!(graph.contains("frame_pad_even_main"));

        // Legacy callers without source metadata cannot infer aspect-fit size.
        let legacy = build_video_filters(&config, true).join(",");
        assert!(!legacy.contains("xstack"));
        assert!(legacy.contains("pad=ceil(127/hsub)"));
    }

    #[test]
    fn test_overlay_filter_complex() {
        let mut config = default_config();
        config.resolution = "720p".to_string();
        config.overlay = Some(OverlayConfig {
            enabled: true,
            path: "/tmp/logo.png".to_string(),
            x: 0.9,
            y: 0.85,
            width: 0.2,
            opacity: 0.75,
            anchor: "custom".to_string(),
        });

        let filter = build_overlay_filter_complex(&config);

        assert!(filter.contains("[0:v:0]scale=-2:720:flags=lanczos[base]"));
        assert!(filter.contains("[1:v:0]format=rgba,colorchannelmixer=aa=0.750"));
        assert!(filter.contains("[base]split[base_ref][base_out]"));
        assert!(filter.contains("[overlay_src][base_ref]scale=w='min(rw*0.200000,rh*iw/ih)':h=-1"));
        assert!(
            filter.contains(
                "overlay=x='min(max(main_w*0.900000-overlay_w/2,0),main_w-overlay_w)':y='min(max(main_h*0.850000-overlay_h/2,0),main_h-overlay_h)'"
            )
        );
        assert!(filter.ends_with("[vout]"));
    }

    #[test]
    fn encode_overlay_filter_complex_pads_base_before_overlay() {
        let mut config = default_config();
        config.overlay = Some(OverlayConfig {
            enabled: true,
            path: "/tmp/logo.png".to_string(),
            x: 0.5,
            y: 0.5,
            width: 0.2,
            opacity: 1.0,
            anchor: "custom".to_string(),
        });

        let filter = build_encode_overlay_filter_complex(&config);

        assert!(filter.contains("[0:v:0]pad=ceil(iw/2)*2:ceil(ih/2)*2:0:0[base]"));
    }

    #[test]
    fn preview_low_res_filters_rebase_subtitle_timestamps_for_seeked_video() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());

        let filters = build_visual_filter_chain(
            &config,
            VisualFilterProfile::PreviewLowRes {
                base: VisualFilterBase::Video,
                width: 640,
                height: 360,
                fps: 24,
                source_time_seconds: 10.0,
            },
        );

        assert_eq!(
            filters,
            vec![
                "setpts=PTS+10.000/TB",
                "subtitles=filename=/tmp/sub.srt",
                "setpts=PTS-10.000/TB",
                EVEN_DIMENSIONS_FILTER,
                "fps=24",
                "scale=640:360",
                "setsar=1",
                "format=bgra",
            ]
        );
    }

    #[test]
    fn preview_low_res_filter_complex_keeps_overlay_before_fps_and_scale() {
        let mut config = default_config();
        config.overlay = Some(OverlayConfig {
            enabled: true,
            path: "/tmp/logo.png".to_string(),
            x: 0.5,
            y: 0.5,
            width: 0.2,
            opacity: 1.0,
            anchor: "custom".to_string(),
        });

        let filter = build_visual_filter_complex(
            &config,
            VisualFilterProfile::PreviewLowRes {
                base: VisualFilterBase::Video,
                width: 640,
                height: 360,
                fps: 24,
                source_time_seconds: 0.0,
            },
        );

        assert!(filter.contains("[preview_export]fps=24,scale=640:360"));
    }

    #[test]
    fn preview_low_res_filters_keep_subtitle_timestamps_for_zero_start() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());

        let filters = build_visual_filter_chain(
            &config,
            VisualFilterProfile::PreviewLowRes {
                base: VisualFilterBase::Video,
                width: 640,
                height: 360,
                fps: 24,
                source_time_seconds: 0.0,
            },
        );

        assert!(!filters.iter().any(|filter| filter.starts_with("setpts=")));
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
        assert_eq!(filters, vec!["volume=1.500"]);
    }

    #[test]
    fn test_subtitle_burn_path_escaping() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("C:\\Media\\John's [cut],final.srt".to_string());

        let filters = build_video_filters(&config, true);

        assert_eq!(
            filters,
            vec![r"subtitles=filename=C\\:\\\\Media\\\\John\\\'s\\\ \[cut\]\,final.srt"]
        );
    }

    #[test]
    fn subtitle_clock_uses_export_trim_or_preview_seek_without_double_offset() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());
        config.start_time = Some("00:00:02.500".to_string());
        for profile in [
            VisualFilterProfile::ExportVideo,
            VisualFilterProfile::ExportImage,
        ] {
            let filters = build_visual_filter_chain(&config, profile).join(",");
            assert!(filters.contains(
                "setpts=PTS+2.500/TB,subtitles=filename=/tmp/sub.srt,setpts=PTS-2.500/TB"
            ));
        }
        for base in [VisualFilterBase::Video, VisualFilterBase::Image] {
            let profile = VisualFilterProfile::PreviewLowRes {
                base,
                width: 64,
                height: 48,
                fps: 10,
                source_time_seconds: 3.0,
            };
            let filters = build_visual_filter_chain(&config, profile).join(",");
            assert!(filters.contains(
                "setpts=PTS+3.000/TB,subtitles=filename=/tmp/sub.srt,setpts=PTS-3.000/TB"
            ));
            assert!(!filters.contains("2.500/TB"));
            let graph = build_visual_filter_complex(&config, profile);
            assert!(graph.contains("setpts=PTS+3.000/TB"));
            assert!(!graph.contains("2.500/TB"));
        }
    }

    #[test]
    fn test_subtitle_position_top_maps_to_alignment_6() {
        let mut config = default_config();
        config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());
        config.subtitle_position = Some("top".to_string());

        let filters = build_video_filters(&config, true);

        assert_eq!(
            filters,
            vec!["subtitles=filename=/tmp/sub.srt:force_style=Alignment=6"]
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
            vec!["subtitles=filename=/tmp/sub.srt:force_style=Alignment=10"]
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
            vec!["subtitles=filename=/tmp/sub.srt:force_style=Fontsize=28"]
        );
    }
}
