use std::path::Path;

use crate::codec::{
    add_audio_codec_args, add_fps_args, add_subtitle_codec_args, add_video_codec_args,
    audio_codec_supports_vbr,
};
use crate::error::ConversionError;
use crate::filters::{
    build_audio_filters, build_encode_overlay_filter_complex, build_encode_video_filters,
    build_overlay_filter_complex, build_video_filters, has_overlay,
};
use crate::media_filters::validate_media_filters;
use crate::media_rules::{
    all_containers, container_supports_audio, container_supports_subtitles, is_audio_codec_allowed,
    is_audio_stream_codec_allowed, is_image_container, is_subtitle_codec_allowed,
    is_video_codec_allowed, is_video_only_container, is_video_pixel_format_allowed,
    is_video_stream_codec_allowed,
};
use crate::types::{
    AudioTrack, ConversionConfig, MetadataConfig, MetadataMode, ProbeMetadata, SubtitleTrack,
    VOLUME_EPSILON,
};
use crate::utils::{get_hwaccel_args, is_audio_only_container, parse_time};

fn is_copy_mode(config: &ConversionConfig) -> bool {
    config.processing_mode == "copy"
}

fn has_custom_pixel_format(config: &ConversionConfig) -> bool {
    let pixel_format = config.pixel_format.trim();
    !pixel_format.is_empty() && pixel_format != "auto"
}

fn collect_selected_audio_tracks<'a>(
    config: &ConversionConfig,
    probe: &'a ProbeMetadata,
) -> Result<Vec<&'a AudioTrack>, ConversionError> {
    if config.selected_audio_tracks.is_empty() {
        return Ok(probe.audio_tracks.iter().collect());
    }

    config
        .selected_audio_tracks
        .iter()
        .map(|index| {
            probe
                .audio_tracks
                .iter()
                .find(|track| track.index == *index)
                .ok_or_else(|| {
                    ConversionError::InvalidInput(format!(
                        "Selected audio track #{index} was not found in source"
                    ))
                })
        })
        .collect()
}

fn collect_selected_subtitle_tracks<'a>(
    config: &ConversionConfig,
    probe: &'a ProbeMetadata,
) -> Result<Vec<&'a SubtitleTrack>, ConversionError> {
    if config.selected_subtitle_tracks.is_empty() {
        return Ok(probe.subtitle_tracks.iter().collect());
    }

    config
        .selected_subtitle_tracks
        .iter()
        .map(|index| {
            probe
                .subtitle_tracks
                .iter()
                .find(|track| track.index == *index)
                .ok_or_else(|| {
                    ConversionError::InvalidInput(format!(
                        "Selected subtitle track #{index} was not found in source"
                    ))
                })
        })
        .collect()
}

fn collect_reencode_subtitle_tracks<'a>(
    config: &ConversionConfig,
    probe: &'a ProbeMetadata,
) -> Result<Vec<&'a SubtitleTrack>, ConversionError> {
    let tracks = collect_selected_subtitle_tracks(config, probe)?;
    if config.selected_subtitle_tracks.is_empty() {
        return Ok(tracks
            .into_iter()
            .filter(|track| subtitle_can_be_encoded_for_container(&config.container, &track.codec))
            .collect());
    }

    for track in &tracks {
        if !subtitle_can_be_encoded_for_container(&config.container, &track.codec) {
            return Err(ConversionError::InvalidInput(format!(
                "Subtitle codec '{}' from source track #{} cannot be converted for container '{}'",
                track.codec, track.index, config.container
            )));
        }
    }

    Ok(tracks)
}

fn subtitle_can_be_encoded_for_container(container: &str, codec: &str) -> bool {
    if container.eq_ignore_ascii_case("mkv") {
        return true;
    }

    matches!(
        container.to_ascii_lowercase().as_str(),
        "mp4" | "mov" | "webm"
    ) && is_text_subtitle_codec(codec)
}

fn is_text_subtitle_codec(codec: &str) -> bool {
    matches!(
        codec.trim().to_ascii_lowercase().as_str(),
        "text"
            | "ssa"
            | "mov_text"
            | "srt"
            | "microdvd"
            | "eia_608"
            | "jacosub"
            | "sami"
            | "realtext"
            | "stl"
            | "subviewer1"
            | "subviewer"
            | "subrip"
            | "webvtt"
            | "mpl2"
            | "vplayer"
            | "pjs"
            | "ass"
            | "hdmv_text_subtitle"
            | "ttml"
    )
}

fn add_track_maps<T>(args: &mut Vec<String>, tracks: &[&T], index: impl Fn(&T) -> u32) {
    for track in tracks {
        args.push("-map".to_string());
        args.push(format!("0:{}", index(track)));
    }
}

/// Validates whether stream-copy mode can preserve the selected source streams.
///
/// # Errors
///
/// Returns [`ConversionError`] when the selected source streams are missing or
/// incompatible with the requested output container.
pub fn validate_stream_copy_compatibility(
    config: &ConversionConfig,
    probe: &ProbeMetadata,
) -> Result<(), ConversionError> {
    if !is_copy_mode(config) {
        return Ok(());
    }

    let is_audio_only = is_audio_only_container(&config.container);

    if is_audio_only {
        let selected_audio = collect_selected_audio_tracks(config, probe)?;
        if selected_audio.is_empty() {
            return Err(ConversionError::InvalidInput(
                "Source has no audio streams to copy into an audio container".to_string(),
            ));
        }
        for track in selected_audio {
            if !is_audio_stream_codec_allowed(&config.container, &track.codec) {
                return Err(ConversionError::InvalidInput(format!(
                    "Audio codec '{}' from source track #{} is incompatible with container '{}'",
                    track.codec, track.index, config.container
                )));
            }
        }
        return Ok(());
    }

    let video_codec = probe.video_codec.as_deref().ok_or_else(|| {
        ConversionError::InvalidInput(
            "Source has no video stream; choose an audio container for stream copy".to_string(),
        )
    })?;
    if !is_video_stream_codec_allowed(&config.container, video_codec) {
        return Err(ConversionError::InvalidInput(format!(
            "Video codec '{}' is incompatible with container '{}'",
            video_codec, config.container
        )));
    }

    if container_supports_audio(&config.container) {
        for track in collect_selected_audio_tracks(config, probe)? {
            if !is_audio_stream_codec_allowed(&config.container, &track.codec) {
                return Err(ConversionError::InvalidInput(format!(
                    "Audio codec '{}' from source track #{} is incompatible with container '{}'",
                    track.codec, track.index, config.container
                )));
            }
        }
    }

    if container_supports_subtitles(&config.container) {
        for track in collect_selected_subtitle_tracks(config, probe)? {
            if !is_subtitle_codec_allowed(&config.container, &track.codec) {
                return Err(ConversionError::InvalidInput(format!(
                    "Subtitle codec '{}' from source track #{} is incompatible with container '{}'",
                    track.codec, track.index, config.container
                )));
            }
        }
    }

    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "FFmpeg command assembly stays in one place to keep ordering guarantees explicit"
)]
/// Builds probe-aware `FFmpeg` arguments for one conversion.
///
/// # Errors
///
/// Returns [`ConversionError`] when a selected source stream is missing or
/// cannot be represented by the requested output configuration.
pub fn build_ffmpeg_args(
    input: &str,
    output: &str,
    config: &ConversionConfig,
    probe: &ProbeMetadata,
) -> Result<Vec<String>, ConversionError> {
    let mut args = Vec::new();

    // Hardware decode acceleration (must be before -i)
    if config.hw_decode {
        args.extend(get_hwaccel_args(&config.video_codec));
    }

    if let Some(start) = &config.start_time
        && !start.is_empty()
    {
        args.push("-ss".to_string());
        args.push(start.clone());
    }

    args.push("-i".to_string());
    args.push(input.to_string());

    if has_overlay(config)
        && let Some(overlay) = &config.overlay
    {
        args.push("-i".to_string());
        args.push(overlay.path.clone());
    }

    if let Some(end_str) = &config.end_time
        && !end_str.is_empty()
    {
        if let Some(start_str) = &config.start_time {
            if start_str.is_empty() {
                args.push("-to".to_string());
                args.push(end_str.clone());
            } else if let (Some(start_t), Some(end_t)) =
                (parse_time(start_str), parse_time(end_str))
            {
                let duration = end_t - start_t;
                if duration > 0.0 {
                    args.push("-t".to_string());
                    args.push(format!("{duration:.3}"));
                }
            }
        } else {
            args.push("-to".to_string());
            args.push(end_str.clone());
        }
    }

    match config.metadata.mode {
        MetadataMode::Clean => {
            args.push("-map_metadata".to_string());
            args.push("-1".to_string());
        }
        MetadataMode::Replace => {
            args.push("-map_metadata".to_string());
            args.push("-1".to_string());
            add_metadata_flags(&mut args, &config.metadata);
        }
        MetadataMode::Preserve => {
            add_metadata_flags(&mut args, &config.metadata);
        }
    }

    let is_audio_only = is_audio_only_container(&config.container);
    let is_video_only = is_video_only_container(&config.container);
    let is_image_output = is_image_container(&config.container);
    let is_gif_output = config.container.eq_ignore_ascii_case("gif");
    let use_overlay = has_overlay(config) && !is_audio_only && !is_gif_output;
    let has_burn_subtitles = config
        .subtitle_burn_path
        .as_ref()
        .is_some_and(|path| !path.trim().is_empty());

    if is_copy_mode(config) {
        validate_stream_copy_compatibility(config, probe)?;

        if !is_audio_only {
            args.push("-map".to_string());
            args.push("0:v?".to_string());
        }

        if container_supports_audio(&config.container) {
            let audio_tracks = collect_selected_audio_tracks(config, probe)?;
            add_track_maps(&mut args, &audio_tracks, |track| track.index);
        }

        if container_supports_subtitles(&config.container) {
            let subtitle_tracks = collect_selected_subtitle_tracks(config, probe)?;
            add_track_maps(&mut args, &subtitle_tracks, |track| track.index);
        }

        args.push("-c".to_string());
        args.push("copy".to_string());
        args.push("-dn".to_string());
        args.push("-n".to_string());
        args.push(output.to_string());
        return Ok(args);
    }

    if is_audio_only {
        args.push("-vn".to_string());

        let audio_tracks = collect_selected_audio_tracks(config, probe)?;
        add_track_maps(&mut args, &audio_tracks, |track| track.index);

        add_audio_codec_args(&mut args, config);
    } else if is_video_only && is_gif_output {
        args.push("-filter_complex".to_string());
        args.push(build_gif_filter_complex(config));

        args.push("-map".to_string());
        args.push("[gif_out]".to_string());
        args.push("-an".to_string());

        args.push("-c:v".to_string());
        args.push("gif".to_string());

        args.push("-loop".to_string());
        args.push(config.gif_loop.to_string());
        args.push("-f".to_string());
        args.push("gif".to_string());
    } else if is_image_output {
        add_video_codec_args(&mut args, config);
        if has_custom_pixel_format(config) {
            args.push("-pix_fmt".to_string());
            args.push(config.pixel_format.trim().to_string());
        }

        if use_overlay {
            args.push("-filter_complex".to_string());
            args.push(build_overlay_filter_complex(config));
        } else {
            let video_filters = build_video_filters(config, true);
            if !video_filters.is_empty() {
                args.push("-vf".to_string());
                args.push(video_filters.join(","));
            }
        }

        args.push("-map".to_string());
        args.push(if use_overlay {
            "[vout]".to_string()
        } else {
            "0:v:0".to_string()
        });
        args.push("-frames:v".to_string());
        args.push("1".to_string());
        args.push("-update".to_string());
        args.push("1".to_string());
    } else {
        add_video_codec_args(&mut args, config);
        if has_custom_pixel_format(config) {
            args.push("-pix_fmt".to_string());
            args.push(config.pixel_format.trim().to_string());
        }

        if use_overlay {
            args.push("-filter_complex".to_string());
            args.push(build_encode_overlay_filter_complex(config));
        } else {
            let video_filters = build_encode_video_filters(config, true);
            if !video_filters.is_empty() {
                args.push("-vf".to_string());
                args.push(video_filters.join(","));
            }
        }

        add_fps_args(&mut args, config);
        args.push("-map".to_string());
        args.push(if use_overlay {
            "[vout]".to_string()
        } else {
            "0:v:0".to_string()
        });

        let audio_tracks = collect_selected_audio_tracks(config, probe)?;
        add_track_maps(&mut args, &audio_tracks, |track| track.index);

        add_audio_codec_args(&mut args, config);

        if !config.selected_subtitle_tracks.is_empty() || !has_burn_subtitles {
            let subtitle_tracks = collect_reencode_subtitle_tracks(config, probe)?;
            if !subtitle_tracks.is_empty() {
                add_track_maps(&mut args, &subtitle_tracks, |track| track.index);
                add_subtitle_codec_args(&mut args, config);
            }
        }
    }

    if !is_video_only && !is_image_output {
        let audio_filters = build_audio_filters(config);
        if !audio_filters.is_empty() {
            args.push("-af".to_string());
            args.push(audio_filters.join(","));
        }
    }

    args.push("-dn".to_string());
    args.push("-n".to_string());
    args.push(output.to_string());

    Ok(args)
}

fn normalize_gif_dither(dither: &str) -> &'static str {
    match dither {
        "none" => "none",
        "bayer" => "bayer",
        "floyd_steinberg" => "floyd_steinberg",
        _ => "sierra2_4a",
    }
}

fn build_gif_filter_complex(config: &ConversionConfig) -> String {
    let mut filters = build_video_filters(config, true);
    if config.fps != "original" {
        filters.push(format!("fps={}", config.fps));
    }

    let chain = if filters.is_empty() {
        "split[gif_src][gif_palette_src]".to_string()
    } else {
        format!("{},split[gif_src][gif_palette_src]", filters.join(","))
    };

    let colors = config.gif_colors.clamp(2, 256);
    let dither = normalize_gif_dither(&config.gif_dither);

    format!(
        "[0:v:0]{chain};[gif_palette_src]palettegen=max_colors={colors}:stats_mode=single[gif_palette];[gif_src][gif_palette]paletteuse=dither={dither}:new=1[gif_out]"
    )
}

pub fn add_metadata_flags(args: &mut Vec<String>, metadata: &MetadataConfig) {
    if let Some(v) = &metadata.title
        && !v.is_empty()
    {
        args.push("-metadata".to_string());
        args.push(format!("title={v}"));
    }
    if let Some(v) = &metadata.artist
        && !v.is_empty()
    {
        args.push("-metadata".to_string());
        args.push(format!("artist={v}"));
    }
    if let Some(v) = &metadata.album
        && !v.is_empty()
    {
        args.push("-metadata".to_string());
        args.push(format!("album={v}"));
    }
    if let Some(v) = &metadata.genre
        && !v.is_empty()
    {
        args.push("-metadata".to_string());
        args.push(format!("genre={v}"));
    }
    if let Some(v) = &metadata.date
        && !v.is_empty()
    {
        args.push("-metadata".to_string());
        args.push(format!("date={v}"));
    }
    if let Some(v) = &metadata.comment
        && !v.is_empty()
    {
        args.push("-metadata".to_string());
        args.push(format!("comment={v}"));
    }
}

fn sanitize_output_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = trimmed.rsplit(['/', '\\']).next().map_or("", str::trim);

    if candidate.is_empty() || candidate == "." || candidate == ".." {
        return None;
    }

    Some(candidate.to_string())
}

pub fn build_output_path(
    output_directory: &str,
    container: &str,
    output_name: Option<&str>,
) -> String {
    let output_name = output_name
        .and_then(sanitize_output_name)
        .unwrap_or_else(|| "output_converted".to_string());
    let output_stem = output_name
        .rsplit_once('.')
        .filter(|(stem, extension)| {
            !stem.is_empty()
                && all_containers()
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(extension))
        })
        .map_or(output_name.as_str(), |(stem, _)| stem);
    let separator = if output_directory.contains('\\') && !output_directory.contains('/') {
        "\\"
    } else {
        "/"
    };
    let directory = output_directory.trim_end_matches(['/', '\\']);

    format!("{directory}{separator}{output_stem}.{container}")
}

#[expect(
    clippy::too_many_lines,
    reason = "Validation intentionally mirrors UI options in one function for consistent backend guardrails"
)]
/// Validates a source path and conversion configuration before running `FFmpeg`.
///
/// # Errors
///
/// Returns [`ConversionError`] when the input path is invalid, trim bounds are
/// malformed, output settings are incompatible, or referenced sidecar assets do
/// not exist.
pub fn validate_task_input(
    file_path: &str,
    config: &ConversionConfig,
) -> Result<(), ConversionError> {
    let input_path = Path::new(file_path);
    if !input_path.exists() {
        return Err(ConversionError::InvalidInput(format!(
            "Input file does not exist: {file_path}"
        )));
    }
    if !input_path.is_file() {
        return Err(ConversionError::InvalidInput(format!(
            "Input path is not a file: {file_path}"
        )));
    }

    let start_time = config
        .start_time
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let end_time = config
        .end_time
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let processing_mode = config.processing_mode.trim();

    if processing_mode != "reencode" && processing_mode != "copy" {
        return Err(ConversionError::InvalidInput(format!(
            "Invalid processing mode: {processing_mode}"
        )));
    }
    validate_media_filters(config)?;
    let is_copy_mode = processing_mode == "copy";

    if let Some(start) = start_time
        && parse_time(start).is_none()
    {
        return Err(ConversionError::InvalidInput(format!(
            "Invalid start time: {start}"
        )));
    }

    if let Some(end) = end_time
        && parse_time(end).is_none()
    {
        return Err(ConversionError::InvalidInput(format!(
            "Invalid end time: {end}"
        )));
    }

    if let (Some(start), Some(end)) = (start_time, end_time)
        && let (Some(start_t), Some(end_t)) = (parse_time(start), parse_time(end))
        && end_t <= start_t
    {
        return Err(ConversionError::InvalidInput(
            "End time must be greater than start time".to_string(),
        ));
    }

    if !is_copy_mode && config.resolution == "custom" {
        let w_str = config.custom_width.as_deref().unwrap_or("-1");
        let h_str = config.custom_height.as_deref().unwrap_or("-1");

        let w = w_str
            .parse::<i32>()
            .map_err(|_| ConversionError::InvalidInput(format!("Invalid custom width: {w_str}")))?;
        let h = h_str.parse::<i32>().map_err(|_| {
            ConversionError::InvalidInput(format!("Invalid custom height: {h_str}"))
        })?;

        if w == 0 || h == 0 {
            return Err(ConversionError::InvalidInput(
                "Resolution dimensions cannot be zero".to_string(),
            ));
        }
        if w < -1 || h < -1 {
            return Err(ConversionError::InvalidInput(
                "Resolution dimensions cannot be negative (except -1 for auto)".to_string(),
            ));
        }
    }

    if !is_copy_mode
        && config.video_bitrate_mode == "bitrate"
        && !is_audio_only_container(&config.container)
        && !is_video_only_container(&config.container)
    {
        let bitrate = config.video_bitrate.parse::<f64>().map_err(|_| {
            ConversionError::InvalidInput(format!(
                "Invalid video bitrate: {}",
                config.video_bitrate
            ))
        })?;
        if bitrate <= 0.0 {
            return Err(ConversionError::InvalidInput(
                "Video bitrate must be positive".to_string(),
            ));
        }
    }

    let is_audio_only = is_audio_only_container(&config.container);
    let is_video_only = is_video_only_container(&config.container);
    let is_image_output = is_image_container(&config.container);
    let supports_audio = container_supports_audio(&config.container);
    let supports_subtitles = container_supports_subtitles(&config.container);
    if !is_copy_mode
        && !is_audio_only
        && !is_video_codec_allowed(&config.container, &config.video_codec)
    {
        return Err(ConversionError::InvalidInput(format!(
            "Video codec '{}' is not compatible with container '{}'",
            config.video_codec, config.container
        )));
    }

    if !is_copy_mode
        && supports_audio
        && !is_audio_codec_allowed(&config.container, &config.audio_codec)
    {
        return Err(ConversionError::InvalidInput(format!(
            "Audio codec '{}' is not compatible with container '{}'",
            config.audio_codec, config.container
        )));
    }

    if !is_copy_mode && supports_audio {
        let lossless_audio = ["flac", "alac", "pcm_s16le"];
        let is_lossless = lossless_audio.contains(&config.audio_codec.as_str());
        match config.audio_bitrate_mode.as_str() {
            "bitrate" => {
                if !is_lossless {
                    let bitrate = config.audio_bitrate.parse::<f64>().map_err(|_| {
                        ConversionError::InvalidInput(format!(
                            "Invalid audio bitrate: {}",
                            config.audio_bitrate
                        ))
                    })?;
                    if bitrate <= 0.0 {
                        return Err(ConversionError::InvalidInput(
                            "Audio bitrate must be positive".to_string(),
                        ));
                    }
                }
            }
            "vbr" => {
                if is_lossless {
                    return Err(ConversionError::InvalidInput(
                        "VBR is not applicable to lossless audio codecs".to_string(),
                    ));
                }
                if !audio_codec_supports_vbr(&config.audio_codec) {
                    return Err(ConversionError::InvalidInput(format!(
                        "Audio codec '{}' does not support VBR",
                        config.audio_codec
                    )));
                }
                if config.audio_quality.trim().parse::<u8>().is_err() {
                    return Err(ConversionError::InvalidInput(format!(
                        "Invalid audio quality: {}",
                        config.audio_quality
                    )));
                }
            }
            other => {
                return Err(ConversionError::InvalidInput(format!(
                    "Invalid audio bitrate mode: {other}"
                )));
            }
        }
    }

    if (is_audio_only || is_video_only) && has_custom_pixel_format(config) {
        return Err(ConversionError::InvalidInput(
            "Pixel format override is not available for this container".to_string(),
        ));
    }

    if let Some(overlay) = config
        .overlay
        .as_ref()
        .filter(|overlay| overlay.enabled && !overlay.path.trim().is_empty())
    {
        let overlay_path = Path::new(&overlay.path);
        if !overlay_path.exists() {
            return Err(ConversionError::InvalidInput(format!(
                "Overlay image does not exist: {}",
                overlay.path
            )));
        }

        if is_audio_only {
            return Err(ConversionError::InvalidInput(
                "Overlay is not available for audio-only outputs".to_string(),
            ));
        }

        if config.container.eq_ignore_ascii_case("gif") {
            return Err(ConversionError::InvalidInput(
                "Overlay is not available for GIF output yet".to_string(),
            ));
        }
    }

    if !is_copy_mode
        && has_custom_pixel_format(config)
        && !is_video_pixel_format_allowed(
            &config.container,
            &config.video_codec,
            &config.pixel_format,
        )
    {
        return Err(ConversionError::InvalidInput(format!(
            "Pixel format '{}' is not compatible with container '{}' and encoder '{}'",
            config.pixel_format, config.container, config.video_codec
        )));
    }

    if is_copy_mode {
        if is_video_only || is_image_output {
            return Err(ConversionError::InvalidInput(
                "Stream copy mode is not available for image/video-only containers".to_string(),
            ));
        }

        if has_custom_pixel_format(config) {
            return Err(ConversionError::InvalidInput(
                "Pixel format override requires re-encoding mode".to_string(),
            ));
        }

        if config
            .subtitle_burn_path
            .as_ref()
            .is_some_and(|path| !path.trim().is_empty())
        {
            return Err(ConversionError::InvalidInput(
                "Burn-in subtitles are unavailable in stream copy mode".to_string(),
            ));
        }

        if has_overlay(config) {
            return Err(ConversionError::InvalidInput(
                "Overlay requires re-encoding".to_string(),
            ));
        }

        if (config.audio_volume - 100.0).abs() > VOLUME_EPSILON {
            return Err(ConversionError::InvalidInput(
                "Audio volume adjustment requires re-encoding".to_string(),
            ));
        }

        if config.audio_normalize {
            return Err(ConversionError::InvalidInput(
                "Audio normalization requires re-encoding".to_string(),
            ));
        }

        if config.rotation != "0" || config.flip_horizontal || config.flip_vertical {
            return Err(ConversionError::InvalidInput(
                "Video transforms require re-encoding".to_string(),
            ));
        }

        if config.crop.as_ref().is_some_and(|crop| crop.enabled) {
            return Err(ConversionError::InvalidInput(
                "Cropping requires re-encoding".to_string(),
            ));
        }

        if config.resolution != "original" || config.fps != "original" {
            return Err(ConversionError::InvalidInput(
                "Resolution and FPS changes require re-encoding".to_string(),
            ));
        }

        if config.hw_decode {
            return Err(ConversionError::InvalidInput(
                "Hardware decoding is unavailable in stream copy mode".to_string(),
            ));
        }
    }

    if !supports_audio && !config.selected_audio_tracks.is_empty() {
        return Err(ConversionError::InvalidInput(
            "Audio track selection is not available for this container".to_string(),
        ));
    }

    if !supports_subtitles
        && (!config.selected_subtitle_tracks.is_empty()
            || config
                .subtitle_burn_path
                .as_ref()
                .is_some_and(|path| !path.trim().is_empty()))
    {
        return Err(ConversionError::InvalidInput(
            "Subtitle options are not available for this container".to_string(),
        ));
    }

    if is_video_only && config.container.eq_ignore_ascii_case("gif") {
        if !(2..=256).contains(&config.gif_colors) {
            return Err(ConversionError::InvalidInput(format!(
                "GIF palette size must be between 2 and 256 colors: {}",
                config.gif_colors
            )));
        }

        if !matches!(
            config.gif_dither.as_str(),
            "none" | "bayer" | "floyd_steinberg" | "sierra2_4a"
        ) {
            return Err(ConversionError::InvalidInput(format!(
                "Invalid GIF dither mode: {}",
                config.gif_dither
            )));
        }
    }

    if is_image_output {
        validate_image_encoding_settings(config)?;
    }

    Ok(())
}

fn validate_image_encoding_settings(config: &ConversionConfig) -> Result<(), ConversionError> {
    match config.video_codec.as_str() {
        "mjpeg" => {
            if !(1..=100).contains(&config.image_jpeg_quality) {
                return Err(ConversionError::InvalidInput(format!(
                    "JPEG quality must be between 1 and 100: {}",
                    config.image_jpeg_quality
                )));
            }
            if !matches!(config.image_jpeg_huffman.as_str(), "default" | "optimal") {
                return Err(ConversionError::InvalidInput(format!(
                    "Invalid JPEG Huffman mode: {}",
                    config.image_jpeg_huffman
                )));
            }
        }
        "libwebp" => {
            if config.image_webp_quality > 100 {
                return Err(ConversionError::InvalidInput(format!(
                    "WebP quality must be between 0 and 100: {}",
                    config.image_webp_quality
                )));
            }
            if config.image_webp_compression > 6 {
                return Err(ConversionError::InvalidInput(format!(
                    "WebP compression effort must be between 0 and 6: {}",
                    config.image_webp_compression
                )));
            }
            if !matches!(
                config.image_webp_preset.as_str(),
                "default" | "picture" | "photo" | "drawing" | "icon" | "text"
            ) {
                return Err(ConversionError::InvalidInput(format!(
                    "Invalid WebP preset: {}",
                    config.image_webp_preset
                )));
            }
        }
        "png" => {
            if config.image_png_compression > 9 {
                return Err(ConversionError::InvalidInput(format!(
                    "PNG compression level must be between 0 and 9: {}",
                    config.image_png_compression
                )));
            }
            if !matches!(
                config.image_png_prediction.as_str(),
                "none" | "sub" | "up" | "avg" | "paeth" | "mixed"
            ) {
                return Err(ConversionError::InvalidInput(format!(
                    "Invalid PNG prediction mode: {}",
                    config.image_png_prediction
                )));
            }
        }
        "tiff"
            if !matches!(
                config.image_tiff_compression.as_str(),
                "packbits" | "raw" | "lzw" | "deflate"
            ) =>
        {
            return Err(ConversionError::InvalidInput(format!(
                "Invalid TIFF compression mode: {}",
                config.image_tiff_compression
            )));
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::EVEN_DIMENSIONS_FILTER;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn sample_config(container: &str, video_codec: &str) -> ConversionConfig {
        ConversionConfig {
            processing_mode: "reencode".to_string(),
            container: container.to_string(),
            video_codec: video_codec.to_string(),
            video_bitrate_mode: "crf".to_string(),
            video_bitrate: "5000".to_string(),
            audio_codec: "aac".to_string(),
            audio_bitrate: "128".to_string(),
            audio_bitrate_mode: "bitrate".to_string(),
            audio_quality: "4".to_string(),
            audio_channels: "original".to_string(),
            audio_volume: 100.0,
            audio_normalize: false,
            video_filters: crate::types::VideoFiltersConfig::default(),
            audio_filters: crate::types::AudioFiltersConfig::default(),
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
            scaling_algorithm: "bicubic".to_string(),
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

    fn sample_probe() -> ProbeMetadata {
        ProbeMetadata {
            media_kind: "video".to_string(),
            video_codec: Some("h264".to_string()),
            audio_tracks: vec![AudioTrack {
                index: 1,
                codec: "aac".to_string(),
                channels: "2".to_string(),
                ..AudioTrack::default()
            }],
            ..ProbeMetadata::default()
        }
    }

    #[test]
    fn build_ffmpeg_args_adds_even_dimensions_guard_for_default_video_reencode() {
        let config = sample_config("mp4", "libx264");

        let args = build_ffmpeg_args("input.mov", "output.mp4", &config, &sample_probe())
            .expect("arguments should build");

        let vf_index = args.iter().position(|arg| arg == "-vf").unwrap();
        assert_eq!(args[vf_index + 1], EVEN_DIMENSIONS_FILTER);
    }

    #[test]
    fn build_ffmpeg_args_does_not_add_even_dimensions_guard_for_image_output() {
        let config = sample_config("png", "png");

        let args = build_ffmpeg_args("input.mov", "output.png", &config, &sample_probe())
            .expect("arguments should build");

        assert!(!args.iter().any(|arg| arg == EVEN_DIMENSIONS_FILTER));
    }

    #[test]
    fn build_output_path_preserves_periods_in_output_name_on_unc_share() {
        let output = build_output_path(
            r"\\myserver.domain.com\share\movies\Really Funny Home Video Vol.1 (2026)",
            "mp4",
            Some("Really Funny Home Video Vol.1 (2026)"),
        );

        assert_eq!(
            output,
            r"\\myserver.domain.com\share\movies\Really Funny Home Video Vol.1 (2026)\Really Funny Home Video Vol.1 (2026).mp4"
        );
    }

    #[test]
    fn build_output_path_replaces_known_container_extension() {
        let output = build_output_path("/tmp", "mp4", Some("render.mov"));

        assert_eq!(output, "/tmp/render.mp4");
    }

    #[test]
    fn build_output_path_uses_selected_output_directory() {
        let output = build_output_path("/exports", "mp4", Some("render"));

        assert_eq!(output, "/exports/render.mp4");
    }

    #[test]
    fn build_ffmpeg_args_disables_output_overwrite_for_reencode() {
        let config = sample_config("mp4", "libx264");

        let args = build_ffmpeg_args("input.mov", "output.mp4", &config, &sample_probe())
            .expect("re-encode arguments should build");

        assert_eq!(
            (
                args.iter().any(|arg| arg == "-n"),
                args.iter().any(|arg| arg == "-y")
            ),
            (true, false)
        );
    }

    #[test]
    fn build_ffmpeg_args_disables_output_overwrite_for_stream_copy() {
        let mut config = sample_config("mp4", "libx264");
        config.processing_mode = "copy".to_string();

        let args = build_ffmpeg_args("input.mov", "output.mp4", &config, &sample_probe())
            .expect("stream-copy arguments should build");

        assert_eq!(
            (
                args.iter().any(|arg| arg == "-n"),
                args.iter().any(|arg| arg == "-y")
            ),
            (true, false)
        );
    }

    #[test]
    fn build_ffmpeg_args_adds_png_compression_options() {
        let mut config = sample_config("png", "png");
        config.image_png_compression = 3;
        config.image_png_prediction = "mixed".to_string();

        let args = build_ffmpeg_args("input.mov", "output.png", &config, &sample_probe())
            .expect("arguments should build");

        assert!(args_contains_pair(&args, "-compression_level", "3"));
        assert!(args_contains_pair(&args, "-pred", "mixed"));
    }

    #[test]
    fn build_ffmpeg_args_adds_jpeg_quality_and_huffman_options() {
        let mut config = sample_config("jpg", "mjpeg");
        config.image_jpeg_quality = 100;
        config.image_jpeg_huffman = "default".to_string();

        let args = build_ffmpeg_args("input.mov", "output.jpg", &config, &sample_probe())
            .expect("arguments should build");

        assert!(args_contains_pair(&args, "-q:v", "2"));
        assert!(args_contains_pair(&args, "-huffman", "default"));
    }

    #[test]
    fn build_ffmpeg_args_adds_webp_quality_and_compression_options() {
        let mut config = sample_config("webp", "libwebp");
        config.image_webp_lossless = true;
        config.image_webp_quality = 88;
        config.image_webp_compression = 6;
        config.image_webp_preset = "photo".to_string();

        let args = build_ffmpeg_args("input.mov", "output.webp", &config, &sample_probe())
            .expect("arguments should build");

        assert!(args_contains_pair(&args, "-lossless", "1"));
        assert!(args_contains_pair(&args, "-quality", "88"));
        assert!(args_contains_pair(&args, "-compression_level", "6"));
        assert!(args_contains_pair(&args, "-preset", "photo"));
    }

    #[test]
    fn build_ffmpeg_args_adds_tiff_compression_option() {
        let mut config = sample_config("tiff", "tiff");
        config.image_tiff_compression = "deflate".to_string();

        let args = build_ffmpeg_args("input.mov", "output.tiff", &config, &sample_probe())
            .expect("arguments should build");

        assert!(args_contains_pair(&args, "-compression_algo", "deflate"));
    }

    #[test]
    fn build_ffmpeg_args_maps_only_audio_tracks_returned_by_probe() {
        let config = sample_config("mp4", "libx264");
        let probe = sample_probe();

        let args = build_ffmpeg_args("spatial.mov", "output.mp4", &config, &probe)
            .expect("recognized AAC track should be mapped");

        assert!(args_contains_pair(&args, "-map", "0:1"));
        assert!(!args.iter().any(|arg| arg == "0:a?"));
        assert!(!args.iter().any(|arg| arg == "0:2"));
        assert!(args.iter().any(|arg| arg == "-dn"));
    }

    #[test]
    fn build_ffmpeg_args_skips_bitmap_subtitles_for_mp4_by_default() {
        let config = sample_config("mp4", "libx264");
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![
            SubtitleTrack {
                index: 2,
                codec: "hdmv_pgs_subtitle".to_string(),
                ..SubtitleTrack::default()
            },
            SubtitleTrack {
                index: 3,
                codec: "subrip".to_string(),
                ..SubtitleTrack::default()
            },
        ];

        let args = build_ffmpeg_args("subtitles.mkv", "output.mp4", &config, &probe)
            .expect("compatible text subtitle should be mapped");

        assert!(!args.iter().any(|arg| arg == "0:s?"));
        assert!(!args.iter().any(|arg| arg == "0:2"));
        assert!(args_contains_pair(&args, "-map", "0:3"));
        assert!(args_contains_pair(&args, "-c:s", "mov_text"));
    }

    #[test]
    fn build_ffmpeg_args_omits_subtitle_codec_when_mp4_source_has_only_pgs() {
        let config = sample_config("mp4", "libx264");
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }];

        let args = build_ffmpeg_args("pgs.mkv", "output.mp4", &config, &probe)
            .expect("default PGS subtitle should be skipped");

        assert!(!args.iter().any(|arg| arg == "0:2"));
        assert!(!args.iter().any(|arg| arg == "-c:s"));
    }

    #[test]
    fn build_ffmpeg_args_rejects_explicit_pgs_selection_for_mp4() {
        let mut config = sample_config("mp4", "libx264");
        config.selected_subtitle_tracks = vec![2];
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }];

        let error = build_ffmpeg_args("pgs.mkv", "output.mp4", &config, &probe)
            .expect_err("explicit PGS selection should fail before FFmpeg starts");

        assert!(error.to_string().contains("hdmv_pgs_subtitle"));
        assert!(error.to_string().contains("track #2"));
        assert!(error.to_string().contains("mp4"));
    }

    #[test]
    fn build_ffmpeg_args_keeps_pgs_subtitles_for_mkv() {
        let config = sample_config("mkv", "libx264");
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }];

        let args = build_ffmpeg_args("pgs.mkv", "output.mkv", &config, &probe)
            .expect("Matroska should preserve PGS subtitles");

        assert!(args_contains_pair(&args, "-map", "0:2"));
        assert!(args_contains_pair(&args, "-c:s", "copy"));
    }

    #[test]
    fn validate_task_input_rejects_invalid_webp_compression_level() {
        let path = temporary_input_file("invalid-webp-compression");
        let mut config = sample_config("webp", "libwebp");
        config.image_webp_compression = 7;

        let error = validate_task_input(&path.to_string_lossy(), &config)
            .expect_err("invalid webp compression should be rejected");

        let _ = fs::remove_file(path);
        assert!(error.to_string().contains("WebP compression effort"));
    }

    fn args_contains_pair(args: &[String], key: &str, value: &str) -> bool {
        args.windows(2)
            .any(|window| window[0] == key && window[1] == value)
    }

    fn temporary_input_file(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "frame-core-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        fs::write(&path, b"").expect("temporary input should be written");
        path
    }
}
