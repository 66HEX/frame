use std::{collections::HashSet, path::Path};

use crate::audio_rules::{
    audio_stream_copy_error, audio_track_count_error, effective_audio_channels,
    mp3_requires_mp4_resampling, opus_encoding_error, stereo_width_supports_channels,
};
use crate::codec::{
    add_audio_codec_args, add_fps_args, add_video_codec_args, audio_codec_supports_vbr,
    subtitle_output_codec,
};
use crate::container::{
    TransportStreamProfile, is_transport_stream_container, transport_stream_profile,
};
use crate::error::ConversionError;
use crate::filters::{
    VisualFilterContext, VisualFilterProfile, build_audio_filters,
    build_visual_filter_chain_with_context, build_visual_filter_complex_with_context, has_overlay,
};
use crate::media_filters::validate_media_filters;
use crate::media_rules::{
    SubtitleStreamAction as SubtitleAction, all_containers, container_supports_audio,
    container_supports_subtitles, is_audio_codec_allowed, is_image_container,
    is_video_codec_allowed, is_video_only_container, is_video_pixel_format_allowed,
    is_video_stream_codec_allowed,
};
use crate::types::{
    AudioTrack, ConversionConfig, ExternalSubtitleTrack, MIN_GIF_COLORS, MetadataConfig,
    MetadataMode, ProbeMetadata, SubtitleTrack, VOLUME_EPSILON,
};
use crate::utils::{format_ffmpeg_seconds, get_hwaccel_args, is_audio_only_container, parse_time};

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

fn validate_audio_track_count(config: &ConversionConfig) -> Result<(), ConversionError> {
    if let Some(reason) =
        audio_track_count_error(&config.container, config.selected_audio_tracks.len())
    {
        return Err(ConversionError::InvalidInput(reason));
    }
    Ok(())
}

fn validate_audio_encoding_tracks(
    config: &ConversionConfig,
    probe: &ProbeMetadata,
) -> Result<(), ConversionError> {
    validate_audio_track_count(config)?;
    if is_copy_mode(config) || !container_supports_audio(&config.container) {
        return Ok(());
    }
    for track in collect_selected_audio_tracks(config, probe)? {
        let source_channels = track.channels.parse::<u16>().ok();
        if config.audio_codec == "libopus" {
            let channels = effective_audio_channels(&config.audio_channels, source_channels);
            let layout = (config.audio_channels == "original")
                .then_some(track.channel_layout.as_deref())
                .flatten();
            if let Some(reason) = opus_encoding_error(&config.audio_bitrate, channels, layout) {
                return Err(ConversionError::InvalidInput(format!(
                    "Audio track #{}: {reason}",
                    track.index
                )));
            }
        }
        let width = config.audio_filters.stereo_width;
        if width.enabled
            && width.value != 100
            && !stereo_width_supports_channels(&config.audio_channels, source_channels)
        {
            return Err(ConversionError::InvalidInput(format!(
                "Stereo width needs two channels in every selected audio track. Track #{} is not stereo; choose Stereo in Channels or turn off Stereo width.",
                track.index
            )));
        }
    }
    Ok(())
}

fn validate_audio_copy_track(container: &str, track: &AudioTrack) -> Result<(), ConversionError> {
    if let Some(reason) =
        audio_stream_copy_error(container, &track.codec, track.sample_rate.as_deref())
    {
        return Err(ConversionError::InvalidInput(format!(
            "Audio codec '{}' from source track #{} is incompatible with container '{}': {reason}",
            track.codec, track.index, container
        )));
    }
    Ok(())
}

fn collect_selected_subtitle_tracks<'a>(
    config: &ConversionConfig,
    probe: &'a ProbeMetadata,
) -> Result<Vec<&'a SubtitleTrack>, ConversionError> {
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
    for track in &tracks {
        if let Err(reason) = embedded_subtitle_action(&config.container, &track.codec) {
            return Err(ConversionError::InvalidInput(format!(
                "Subtitle codec '{}' from source track #{} cannot be represented by '{}': {reason}",
                track.codec, track.index, config.container,
            )));
        }
    }

    Ok(tracks)
}

fn validate_external_subtitle_tracks(config: &ConversionConfig) -> Result<(), ConversionError> {
    let mut paths = HashSet::new();
    let mut default_count = 0;

    for track in &config.external_subtitle_tracks {
        let path = track.path.trim();
        if path.is_empty() {
            return Err(ConversionError::InvalidInput(
                "External subtitle path cannot be empty".to_string(),
            ));
        }

        let subtitle_path = Path::new(path);
        if !subtitle_path.exists() {
            return Err(ConversionError::InvalidInput(format!(
                "External subtitle file does not exist: {path}"
            )));
        }
        if !subtitle_path.is_file() {
            return Err(ConversionError::InvalidInput(format!(
                "External subtitle path is not a file: {path}"
            )));
        }
        let supported = subtitle_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "srt" | "ass" | "vtt" | "sup"
                )
            });
        if !supported {
            return Err(ConversionError::InvalidInput(format!(
                "Unsupported external subtitle format: {path}"
            )));
        }
        let canonical_path = std::fs::canonicalize(subtitle_path).map_err(|error| {
            ConversionError::InvalidInput(format!(
                "External subtitle path cannot be resolved ({path}): {error}"
            ))
        })?;
        if !paths.insert(canonical_path) {
            return Err(ConversionError::InvalidInput(format!(
                "External subtitle file was added more than once: {path}"
            )));
        }

        for (field, value) in [("language", &track.language), ("title", &track.title)] {
            if config.metadata.mode == MetadataMode::Preserve
                && value
                    .as_deref()
                    .is_some_and(|value| value.chars().any(char::is_control))
            {
                return Err(ConversionError::InvalidInput(format!(
                    "External subtitle {field} contains control characters: {path}"
                )));
            }
        }
        default_count += usize::from(track.is_default);
    }

    if default_count > 1 {
        return Err(ConversionError::InvalidInput(
            "Only one external subtitle track can be marked as default".to_string(),
        ));
    }

    Ok(())
}

fn embedded_subtitle_action(container: &str, codec: &str) -> Result<SubtitleAction, &'static str> {
    crate::media_rules::subtitle_stream_action(container, codec).ok_or(
        "subtitle codec cannot be copied or decoded for the selected container; choose another output or leave this track unselected",
    )
}

fn external_subtitle_action(
    container: &str,
    track: &ExternalSubtitleTrack,
) -> Result<SubtitleAction, &'static str> {
    let extension = Path::new(&track.path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("sup") {
        return match transport_stream_profile(container) {
            Some(TransportStreamProfile::MpegTs188) => Ok(SubtitleAction::Encode("dvbsub")),
            Some(TransportStreamProfile::M2ts192) => Ok(SubtitleAction::Copy),
            None if container.eq_ignore_ascii_case("mkv") => Ok(SubtitleAction::Copy),
            None => Err("PGS sidecars are selectable only in MKV, MTS, M2TS, or as DVB in M2T"),
        };
    }
    if is_transport_stream_container(container) {
        return Err(
            "text sidecars cannot be authored as standard selectable MPEG-TS subtitles; use Burn-in",
        );
    }
    subtitle_output_codec(container).map_or(
        Err("selectable subtitles are unavailable for this container"),
        |codec| Ok(SubtitleAction::Encode(codec)),
    )
}

fn add_subtitle_output_actions(
    args: &mut Vec<String>,
    container: &str,
    preserve_text_metadata: bool,
    embedded: &[&SubtitleTrack],
    external: &[ExternalSubtitleTrack],
) -> Result<(), ConversionError> {
    for (output_index, track) in embedded.iter().enumerate() {
        let action = embedded_subtitle_action(container, &track.codec).map_err(|reason| {
            ConversionError::InvalidInput(format!(
                "Subtitle codec '{}' from source track #{} is incompatible with '{}': {reason}",
                track.codec, track.index, container
            ))
        })?;
        add_subtitle_action_arg(args, output_index, action);
        if preserve_text_metadata
            && transport_stream_profile(container) != Some(TransportStreamProfile::M2ts192)
            && let Some(language) = track
                .language
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        {
            args.push(format!("-metadata:s:s:{output_index}"));
            args.push(format!("language={language}"));
        }
        if preserve_text_metadata
            && !is_transport_stream_container(container)
            && let Some(label) = track
                .label
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        {
            args.push(format!("-metadata:s:s:{output_index}"));
            args.push(format!("title={label}"));
        }
    }
    for (offset, track) in external.iter().enumerate() {
        let action = external_subtitle_action(container, track).map_err(|reason| {
            ConversionError::InvalidInput(format!(
                "External subtitle '{}' is incompatible with '{}': {reason}",
                track.path, container
            ))
        })?;
        add_subtitle_action_arg(args, embedded.len() + offset, action);
    }
    Ok(())
}

fn add_subtitle_action_arg(args: &mut Vec<String>, output_index: usize, action: SubtitleAction) {
    args.push(format!("-c:s:{output_index}"));
    args.push(
        match action {
            SubtitleAction::Copy => "copy",
            SubtitleAction::Encode(codec) => codec,
        }
        .to_string(),
    );
}

fn add_track_maps<T>(args: &mut Vec<String>, tracks: &[&T], index: impl Fn(&T) -> u32) {
    for track in tracks {
        args.push("-map".to_string());
        args.push(format!("0:{}", index(track)));
    }
}

fn add_external_subtitle_inputs(args: &mut Vec<String>, config: &ConversionConfig) {
    for track in &config.external_subtitle_tracks {
        if let Some(start) = config
            .start_time
            .as_deref()
            .map(str::trim)
            .filter(|start| !start.is_empty())
        {
            args.push("-ss".to_string());
            args.push(start.to_string());
        }
        args.push("-i".to_string());
        args.push(track.path.clone());
    }
}

fn add_external_subtitle_maps(
    args: &mut Vec<String>,
    config: &ConversionConfig,
    first_input_index: usize,
) {
    for input_index in first_input_index..first_input_index + config.external_subtitle_tracks.len()
    {
        args.push("-map".to_string());
        args.push(format!("{input_index}:s:0"));
    }
}

fn add_external_subtitle_output_args(
    args: &mut Vec<String>,
    config: &ConversionConfig,
    first_output_index: usize,
    override_codec: bool,
) {
    let codec = subtitle_output_codec(&config.container);
    for (offset, track) in config.external_subtitle_tracks.iter().enumerate() {
        let output_index = first_output_index + offset;
        if override_codec && let Some(codec) = codec {
            args.push(format!("-c:s:{output_index}"));
            args.push(codec.to_string());
        }
        add_external_subtitle_metadata(
            args,
            track,
            output_index,
            &config.container,
            config.metadata.mode == MetadataMode::Preserve,
        );
    }
}

fn add_external_subtitle_metadata(
    args: &mut Vec<String>,
    track: &ExternalSubtitleTrack,
    output_index: usize,
    container: &str,
    preserve_text_metadata: bool,
) {
    let transport_profile = transport_stream_profile(container);
    if preserve_text_metadata
        && let Some(language) = track
            .language
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        && transport_profile != Some(TransportStreamProfile::M2ts192)
    {
        args.push(format!("-metadata:s:s:{output_index}"));
        args.push(format!("language={language}"));
    }
    if preserve_text_metadata
        && transport_profile.is_none()
        && let Some(title) = track
            .title
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    {
        args.push(format!("-metadata:s:s:{output_index}"));
        args.push(format!("title={title}"));
        if matches!(container, "mp4" | "mov") {
            args.push(format!("-metadata:s:s:{output_index}"));
            args.push(format!("handler_name={title}"));
        }
    }

    if transport_profile.is_some() {
        return;
    }
    let disposition = match (track.is_default, track.is_forced) {
        (true, true) => "default+forced",
        (true, false) => "default",
        (false, true) => "forced",
        (false, false) => "0",
    };
    args.push(format!("-disposition:s:{output_index}"));
    args.push(disposition.to_string());
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
    validate_audio_track_count(config)?;

    let is_audio_only = is_audio_only_container(&config.container);

    if is_audio_only {
        let selected_audio = collect_selected_audio_tracks(config, probe)?;
        if selected_audio.is_empty() {
            return Err(ConversionError::InvalidInput(
                "Select at least one audio track for an audio-only output".to_string(),
            ));
        }
        for track in selected_audio {
            validate_audio_copy_track(&config.container, track)?;
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
            validate_audio_copy_track(&config.container, track)?;
        }
    }

    if container_supports_subtitles(&config.container) {
        for track in collect_selected_subtitle_tracks(config, probe)? {
            if embedded_subtitle_action(&config.container, &track.codec).is_err() {
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
    validate_audio_encoding_tracks(config, probe)?;
    let filter_context = VisualFilterContext::new(probe.width, probe.height);
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

    let first_external_subtitle_input = 1 + usize::from(has_overlay(config));
    add_external_subtitle_inputs(&mut args, config);

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
                    args.push(format_ffmpeg_seconds(duration));
                }
            }
        } else {
            args.push("-to".to_string());
            args.push(end_str.clone());
        }
    }

    add_container_metadata_flags(&mut args, config, probe);

    let is_audio_only = is_audio_only_container(&config.container);
    let is_video_only = is_video_only_container(&config.container);
    let is_image_output = is_image_container(&config.container);
    let is_gif_output = config.container.eq_ignore_ascii_case("gif");
    let use_overlay = has_overlay(config) && !is_audio_only && !is_gif_output;
    if is_copy_mode(config) {
        validate_stream_copy_compatibility(config, probe)?;

        if !is_audio_only {
            args.push("-map".to_string());
            args.push(
                probe
                    .video_stream_index
                    .map_or_else(|| "0:v:0?".to_string(), |index| format!("0:{index}")),
            );
        }

        if container_supports_audio(&config.container) {
            let audio_tracks = collect_selected_audio_tracks(config, probe)?;
            add_track_maps(&mut args, &audio_tracks, |track| track.index);
            for (output_index, track) in audio_tracks.iter().enumerate() {
                if config.container.eq_ignore_ascii_case("mov") && track.codec == "g723_1" {
                    // G.723.1 has 240 samples per packet. Demuxed streams can
                    // lose this value, causing MOV's edit list to omit the tail.
                    args.extend([format!("-frame_size:a:{output_index}"), "240".to_string()]);
                }
                if config.container.eq_ignore_ascii_case("mkv") && track.codec == "dfpwm" {
                    // Demuxers can omit the keyframe flag for DFPWM packets;
                    // otherwise stream copy silently emits an empty track.
                    args.push(format!("-copyinkf:a:{output_index}"));
                }
                if let Some(tag) =
                    crate::media_rules::audio_stream_copy_tag(&config.container, &track.codec)
                {
                    args.extend([format!("-tag:a:{output_index}"), tag.to_string()]);
                }
            }
            if audio_tracks.is_empty() {
                args.push("-an".to_string());
            }
        }

        if container_supports_subtitles(&config.container) {
            let subtitle_tracks = collect_selected_subtitle_tracks(config, probe)?;
            add_track_maps(&mut args, &subtitle_tracks, |track| track.index);
            add_external_subtitle_maps(&mut args, config, first_external_subtitle_input);

            args.push("-c".to_string());
            args.push("copy".to_string());
            add_subtitle_output_actions(
                &mut args,
                &config.container,
                config.metadata.mode == MetadataMode::Preserve,
                &subtitle_tracks,
                &config.external_subtitle_tracks,
            )?;
            add_external_subtitle_output_args(&mut args, config, subtitle_tracks.len(), false);
        } else {
            args.push("-c".to_string());
            args.push("copy".to_string());
        }
        args.push("-dn".to_string());
        if config.container.eq_ignore_ascii_case("mkv")
            && probe.video_codec.as_deref() == Some("rawvideo")
        {
            // Matroska needs its VFW representation for untagged raw RGB.
            // This option also preserves the already supported tagged YUV path.
            args.extend(["-allow_raw_vfw".to_string(), "1".to_string()]);
        }
        add_container_output_args(&mut args, &config.container);
        args.push("-n".to_string());
        args.push(output.to_string());
        return Ok(args);
    }

    if is_audio_only {
        args.push("-vn".to_string());

        let audio_tracks = collect_selected_audio_tracks(config, probe)?;
        if audio_tracks.is_empty() {
            return Err(ConversionError::InvalidInput(
                "Select at least one audio track for an audio-only output".to_string(),
            ));
        }
        add_track_maps(&mut args, &audio_tracks, |track| track.index);

        add_audio_codec_args(&mut args, config);
    } else if is_video_only && is_gif_output {
        args.push("-filter_complex".to_string());
        args.push(build_gif_filter_complex(config, filter_context));

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
            args.push(build_visual_filter_complex_with_context(
                config,
                VisualFilterProfile::ExportImage,
                filter_context,
            ));
        } else {
            let video_filters = build_visual_filter_chain_with_context(
                config,
                VisualFilterProfile::ExportImage,
                filter_context,
            );
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
            args.push(build_visual_filter_complex_with_context(
                config,
                VisualFilterProfile::ExportVideo,
                filter_context,
            ));
        } else {
            let video_filters = build_visual_filter_chain_with_context(
                config,
                VisualFilterProfile::ExportVideo,
                filter_context,
            );
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

        if audio_tracks.is_empty() {
            args.push("-an".to_string());
        } else {
            add_audio_codec_args(&mut args, config);
        }

        let mut subtitle_output_count = 0;
        let subtitle_tracks = collect_reencode_subtitle_tracks(config, probe)?;
        if !subtitle_tracks.is_empty() {
            add_track_maps(&mut args, &subtitle_tracks, |track| track.index);
            subtitle_output_count = subtitle_tracks.len();
        }
        add_external_subtitle_maps(&mut args, config, first_external_subtitle_input);
        if subtitle_output_count > 0 || !config.external_subtitle_tracks.is_empty() {
            add_subtitle_output_actions(
                &mut args,
                &config.container,
                config.metadata.mode == MetadataMode::Preserve,
                &subtitle_tracks,
                &config.external_subtitle_tracks,
            )?;
            add_external_subtitle_output_args(&mut args, config, subtitle_output_count, false);
        }
    }

    if !is_video_only && !is_image_output && !config.selected_audio_tracks.is_empty() {
        for (output_index, track) in collect_selected_audio_tracks(config, probe)?
            .iter()
            .enumerate()
        {
            if mp3_requires_mp4_resampling(
                &config.container,
                &config.audio_codec,
                track.sample_rate.as_deref(),
            ) {
                args.extend([format!("-ar:a:{output_index}"), "16000".to_string()]);
            }
        }
        let audio_filters = build_audio_filters(config);
        if !audio_filters.is_empty() {
            args.push("-af".to_string());
            args.push(audio_filters.join(","));
        }
    }

    args.push("-dn".to_string());
    add_container_output_args(&mut args, &config.container);
    args.push("-n".to_string());
    args.push(output.to_string());

    Ok(args)
}

fn add_container_output_args(args: &mut Vec<String>, container: &str) {
    let Some(profile) = transport_stream_profile(container) else {
        return;
    };
    args.push("-f".to_string());
    args.push("mpegts".to_string());
    args.push("-mpegts_m2ts_mode".to_string());
    args.push(profile.ffmpeg_m2ts_mode().to_string());
}

fn add_container_metadata_flags(
    args: &mut Vec<String>,
    config: &ConversionConfig,
    probe: &ProbeMetadata,
) {
    if !is_transport_stream_container(&config.container) {
        match config.metadata.mode {
            MetadataMode::Clean => {
                args.extend(["-map_metadata".to_string(), "-1".to_string()]);
            }
            MetadataMode::Replace => {
                args.extend(["-map_metadata".to_string(), "-1".to_string()]);
                add_metadata_flags(args, &config.metadata);
            }
            MetadataMode::Preserve => add_metadata_flags(args, &config.metadata),
        }
        return;
    }

    args.extend(["-map_metadata".to_string(), "-1".to_string()]);
    let source_transport = probe.transport_stream.as_ref();
    let source_tags = probe.tags.as_ref();
    let manual_name = non_empty(config.metadata.service_name.as_deref());
    let manual_provider = non_empty(config.metadata.service_provider.as_deref());
    let (service_name, service_provider) = match config.metadata.mode {
        MetadataMode::Preserve => (
            manual_name
                .or_else(|| {
                    source_transport.and_then(|value| non_empty(value.service_name.as_deref()))
                })
                .or_else(|| source_tags.and_then(|value| non_empty(value.title.as_deref())))
                .unwrap_or("Service01"),
            manual_provider
                .or_else(|| {
                    source_transport.and_then(|value| non_empty(value.service_provider.as_deref()))
                })
                .or_else(|| source_tags.and_then(|value| non_empty(value.artist.as_deref())))
                .unwrap_or("Frame"),
        ),
        MetadataMode::Replace => (
            manual_name.unwrap_or("Service01"),
            manual_provider.unwrap_or("Frame"),
        ),
        MetadataMode::Clean => ("Service01", "Frame"),
    };
    for value in [
        format!("service_name={service_name}"),
        format!("service_provider={service_provider}"),
    ] {
        args.push("-metadata".to_string());
        args.push(value);
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn normalize_gif_dither(dither: &str) -> &'static str {
    match dither {
        "none" => "none",
        "bayer" => "bayer",
        "floyd_steinberg" => "floyd_steinberg",
        _ => "sierra2_4a",
    }
}

fn build_gif_filter_complex(
    config: &ConversionConfig,
    filter_context: VisualFilterContext,
) -> String {
    let mut filters = build_visual_filter_chain_with_context(
        config,
        VisualFilterProfile::ExportImage,
        filter_context,
    );
    if config.fps != "original" {
        filters.push(format!("fps={}", config.fps));
    }

    let chain = if filters.is_empty() {
        "split[gif_src][gif_palette_src]".to_string()
    } else {
        format!("{},split[gif_src][gif_palette_src]", filters.join(","))
    };

    let colors = config.gif_colors.clamp(MIN_GIF_COLORS, 256);
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
    validate_audio_track_count(config)?;

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
    if !container_supports_subtitles(&config.container)
        && (!config.selected_subtitle_tracks.is_empty()
            || !config.external_subtitle_tracks.is_empty()
            || config
                .subtitle_burn_path
                .as_ref()
                .is_some_and(|path| !path.trim().is_empty()))
    {
        return Err(ConversionError::InvalidInput(
            "Subtitle options are not available for this container".to_string(),
        ));
    }
    validate_external_subtitle_tracks(config)?;
    for track in &config.external_subtitle_tracks {
        external_subtitle_action(&config.container, track).map_err(|reason| {
            ConversionError::InvalidInput(format!(
                "External subtitle '{}' is incompatible with '{}': {reason}",
                track.path, config.container
            ))
        })?;
        if let Some(profile) = transport_stream_profile(&config.container) {
            let preserve_text_metadata = config.metadata.mode == MetadataMode::Preserve;
            let has_language =
                preserve_text_metadata && non_empty(track.language.as_deref()).is_some();
            let has_title = preserve_text_metadata && non_empty(track.title.as_deref()).is_some();
            let unsupported_metadata = match profile {
                TransportStreamProfile::MpegTs188 => {
                    has_title || track.is_default || track.is_forced
                }
                TransportStreamProfile::M2ts192 => {
                    has_language || has_title || track.is_default || track.is_forced
                }
            };
            if unsupported_metadata {
                return Err(ConversionError::InvalidInput(format!(
                    "External subtitle metadata cannot be represented by '{}'; clear unsupported language/title/default/forced values",
                    config.container
                )));
            }
        }
    }
    let is_copy_mode = processing_mode == "copy";

    if let Some(start) = start_time
        && parse_time(start).is_none_or(|seconds| !seconds.is_finite())
    {
        return Err(ConversionError::InvalidInput(format!(
            "Invalid start time: {start}"
        )));
    }

    if let Some(end) = end_time
        && parse_time(end).is_none_or(|seconds| !seconds.is_finite())
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
        let lossless_audio = ["flac", "alac", "pcm_s16le", "pcm_bluray"];
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
                    if !bitrate.is_finite() || bitrate <= 0.0 {
                        return Err(ConversionError::InvalidInput(
                            "Audio bitrate must be a finite, positive number".to_string(),
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
        if config.audio_codec == "libopus"
            && !config.selected_audio_tracks.is_empty()
            && matches!(config.audio_channels.as_str(), "mono" | "stereo")
            && let Some(reason) = opus_encoding_error(
                &config.audio_bitrate,
                effective_audio_channels(&config.audio_channels, None),
                None,
            )
        {
            return Err(ConversionError::InvalidInput(reason));
        }
        if config.audio_codec == "mp2"
            && !matches!(
                config.audio_bitrate.as_str(),
                "64" | "96" | "112" | "128" | "160" | "192" | "224" | "256" | "320" | "384"
            )
        {
            return Err(ConversionError::InvalidInput(format!(
                "MP2 bitrate must be one of 64, 96, 112, 128, 160, 192, 224, 256, 320, or 384 kbps: {}",
                config.audio_bitrate
            )));
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

    if is_video_only && config.container.eq_ignore_ascii_case("gif") {
        if !(MIN_GIF_COLORS..=256).contains(&config.gif_colors) {
            return Err(ConversionError::InvalidInput(format!(
                "GIF palette size must be between {MIN_GIF_COLORS} and 256 colors: {}",
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
    fn raw_vfw_is_enabled_only_for_matroska_raw_video_copy() {
        for (container, mode, source_codec, expected) in [
            ("mkv", "copy", "rawvideo", true),
            ("mov", "copy", "rawvideo", false),
            ("mkv", "reencode", "rawvideo", false),
            ("mkv", "copy", "h264", false),
        ] {
            let mut config = sample_config(container, "libx264");
            config.processing_mode = mode.to_string();
            let mut probe = sample_probe();
            probe.video_codec = Some(source_codec.to_string());
            let args = build_ffmpeg_args(
                "source.avi",
                &format!("output.{container}"),
                &config,
                &probe,
            )
            .unwrap();
            assert_eq!(args_contains_pair(&args, "-allow_raw_vfw", "1"), expected);
        }
    }

    #[test]
    fn vorbis_copy_uses_the_mov_audio_tag_for_each_output_track() {
        let mut config = sample_config("mov", "libx264");
        config.processing_mode = "copy".to_string();
        config.selected_audio_tracks = vec![2, 1];
        let mut probe = sample_probe();
        probe.audio_tracks.push(AudioTrack {
            index: 2,
            codec: "vorbis".to_string(),
            ..AudioTrack::default()
        });
        let args = build_ffmpeg_args("source.nut", "output.mov", &config, &probe).unwrap();
        assert!(args_contains_pair(&args, "-tag:a:0", "0x6f56736d"));
        assert!(!args.iter().any(|arg| arg == "-tag:a:1"));
        config.container = "mkv".to_string();
        let args = build_ffmpeg_args("source.nut", "output.mkv", &config, &probe).unwrap();
        assert!(!args.iter().any(|arg| arg.starts_with("-tag:a:")));
    }

    #[test]
    fn dfpwm_matroska_copy_preserves_packets_without_keyframe_flags() {
        let mut config = sample_config("mkv", "libx264");
        config.processing_mode = "copy".to_string();
        config.selected_audio_tracks = vec![1];
        let mut probe = sample_probe();
        probe.audio_tracks[0].codec = "dfpwm".to_string();
        let args = build_ffmpeg_args("source.nut", "output.mkv", &config, &probe).unwrap();
        assert!(args.iter().any(|arg| arg == "-copyinkf:a:0"));
        config.processing_mode = "reencode".to_string();
        let args = build_ffmpeg_args("source.nut", "output.mkv", &config, &probe).unwrap();
        assert!(!args.iter().any(|arg| arg.starts_with("-copyinkf")));
    }

    #[test]
    fn g723_mov_copy_restores_frame_size_only_for_the_selected_output_stream() {
        let mut config = sample_config("mov", "libx264");
        config.processing_mode = "copy".to_string();
        config.selected_audio_tracks = vec![2, 1];
        let mut probe = sample_probe();
        probe.audio_tracks.push(AudioTrack {
            index: 2,
            codec: "g723_1".to_string(),
            channels: "1".to_string(),
            ..AudioTrack::default()
        });
        let args = build_ffmpeg_args("input.nut", "output.mov", &config, &probe).unwrap();
        assert!(args_contains_pair(&args, "-frame_size:a:0", "240"));
        assert!(!args.iter().any(|arg| arg == "-frame_size:a:1"));
        for (container, mode) in [("mkv", "copy"), ("mov", "reencode")] {
            config.container = container.to_string();
            config.processing_mode = mode.to_string();
            let args = build_ffmpeg_args("input.nut", "output.video", &config, &probe).unwrap();
            assert!(!args.iter().any(|arg| arg.starts_with("-frame_size")));
        }
    }

    #[test]
    fn single_track_audio_outputs_reject_saved_multiple_selections_in_both_modes() {
        let input = temporary_input_file_with_extension("single-track-limit", "mkv");
        for container in ["mp3", "wav", "flac"] {
            for mode in ["copy", "reencode"] {
                let mut config = sample_config(container, "libx264");
                config.processing_mode = mode.to_string();
                config.selected_audio_tracks = vec![1, 2];
                assert!(
                    validate_task_input(&input.to_string_lossy(), &config)
                        .unwrap_err()
                        .to_string()
                        .contains("one audio track")
                );
                assert!(
                    build_ffmpeg_args("input.mkv", "output.audio", &config, &sample_probe())
                        .unwrap_err()
                        .to_string()
                        .contains("one audio track")
                );
            }
        }
        fs::remove_file(input).unwrap();
    }

    #[test]
    fn mp3_mp4_resamples_only_selected_low_rate_streams() {
        let mut config = sample_config("mp4", "libx264");
        config.audio_codec = "mp3".to_string();
        config.selected_audio_tracks = vec![2, 1];
        let mut probe = sample_probe();
        probe.audio_tracks[0].codec = "mp3".to_string();
        probe.audio_tracks[0].sample_rate = Some("8000".to_string());
        probe.audio_tracks.push(AudioTrack {
            index: 2,
            codec: "mp3".to_string(),
            channels: "2".to_string(),
            sample_rate: Some("48000".to_string()),
            ..AudioTrack::default()
        });
        let args = build_ffmpeg_args("input.mkv", "output.mp4", &config, &probe).unwrap();
        assert!(args_contains_pair(&args, "-ar:a:1", "16000"));
        assert!(!args.iter().any(|arg| arg == "-ar:a:0" || arg == "-ar"));
        config.processing_mode = "copy".to_string();
        assert!(
            build_ffmpeg_args("input.mkv", "output.mp4", &config, &probe)
                .unwrap_err()
                .to_string()
                .contains("16 kHz")
        );
        config.selected_audio_tracks = vec![2];
        assert!(build_ffmpeg_args("input.mkv", "output.mp4", &config, &probe).is_ok());
        config.processing_mode = "reencode".to_string();
        config.selected_audio_tracks = vec![1];
        for container in ["mov", "mkv", "mp3"] {
            config.container = container.to_string();
            let args = build_ffmpeg_args("input.mkv", "output.audio", &config, &probe).unwrap();
            assert!(!args.iter().any(|arg| arg.starts_with("-ar")));
        }
    }

    #[test]
    fn opus_checks_every_selected_track_and_explicit_channel_override() {
        let mut config = sample_config("mkv", "libx264");
        config.audio_codec = "libopus".to_string();
        config.audio_bitrate = "320".to_string();
        let mut probe = sample_probe();
        probe.audio_tracks.push(AudioTrack {
            index: 2,
            codec: "flac".to_string(),
            channels: "1".to_string(),
            ..AudioTrack::default()
        });
        for selected in [vec![1, 2], vec![2, 1]] {
            config.selected_audio_tracks = selected;
            for (mode, allowed) in [("original", false), ("stereo", true), ("mono", false)] {
                config.audio_channels = mode.to_string();
                assert_eq!(
                    build_ffmpeg_args("input.mkv", "output.mkv", &config, &probe).is_ok(),
                    allowed,
                    "{mode}"
                );
            }
        }
        config.audio_channels = "original".to_string();
        config.audio_bitrate = "128".to_string();
        config.selected_audio_tracks = vec![2];
        probe.audio_tracks[1].channels = "6".to_string();
        probe.audio_tracks[1].channel_layout = Some("5.1(side)".to_string());
        assert!(
            build_ffmpeg_args("input.mkv", "output.mkv", &config, &probe)
                .unwrap_err()
                .to_string()
                .contains("channel layout")
        );
        probe.audio_tracks[1].channel_layout = Some("5.1".to_string());
        assert!(build_ffmpeg_args("input.mkv", "output.mkv", &config, &probe).is_ok());
    }

    #[test]
    fn stereo_width_never_implicitly_downmixes_original_audio() {
        let mut config = sample_config("mkv", "libx264");
        config.audio_codec = "flac".to_string();
        config.selected_audio_tracks = vec![1];
        config.audio_filters.stereo_width.enabled = true;
        config.audio_filters.stereo_width.value = 150;
        let mut probe = sample_probe();
        probe.audio_tracks[0].channels = "6".to_string();
        assert!(
            build_ffmpeg_args("input.mkv", "output.mkv", &config, &probe)
                .unwrap_err()
                .to_string()
                .contains("Stereo width")
        );
        config.audio_channels = "stereo".to_string();
        assert!(build_ffmpeg_args("input.mkv", "output.mkv", &config, &probe).is_ok());
        config.audio_channels = "original".to_string();
        config.audio_filters.stereo_width.value = 100;
        assert!(build_ffmpeg_args("input.mkv", "output.mkv", &config, &probe).is_ok());
        config.audio_filters.stereo_width.value = 150;
        config.selected_audio_tracks.clear();
        let args = build_ffmpeg_args("input.mkv", "output.mkv", &config, &probe).unwrap();
        assert!(!args.iter().any(|arg| arg == "-af"));
    }

    #[test]
    fn subtitle_copy_and_conversion_decisions_match_in_both_processing_modes() {
        for mode in ["copy", "reencode"] {
            for (container, codec, expected) in [
                ("mkv", "mov_text", Some("srt")),
                ("mov", "subrip", Some("mov_text")),
                ("mp4", "ttml", Some("copy")),
                ("mov", "ttml", Some("copy")),
                ("webm", "ttml", None),
                ("mkv", "ttml", None),
            ] {
                let mut config = sample_config(
                    container,
                    if container == "webm" {
                        "vp9"
                    } else {
                        "libx264"
                    },
                );
                config.processing_mode = mode.to_string();
                config.selected_subtitle_tracks = vec![2];
                let mut probe = sample_probe();
                probe.video_codec =
                    Some(if container == "webm" { "vp9" } else { "h264" }.to_string());
                probe.subtitle_tracks = vec![SubtitleTrack {
                    index: 2,
                    codec: codec.to_string(),
                    ..SubtitleTrack::default()
                }];
                let result = build_ffmpeg_args(
                    "source.mkv",
                    &format!("output.{container}"),
                    &config,
                    &probe,
                );
                if let Some(expected) = expected {
                    assert!(args_contains_pair(&result.unwrap(), "-c:s:0", expected));
                } else {
                    assert!(result.unwrap_err().to_string().contains("ttml"));
                }
            }
        }
    }

    #[test]
    fn clean_and_replace_omit_subtitle_text_tags_but_keep_playback_flags() {
        for mode in ["copy", "reencode"] {
            for metadata_mode in [
                MetadataMode::Preserve,
                MetadataMode::Clean,
                MetadataMode::Replace,
            ] {
                let preserve = metadata_mode == MetadataMode::Preserve;
                let mut config = sample_config("mov", "libx264");
                config.processing_mode = mode.to_string();
                config.metadata.mode = metadata_mode;
                config.selected_subtitle_tracks = vec![2];
                config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
                    path: "/tmp/external.srt".to_string(),
                    language: Some("pol".to_string()),
                    title: Some("Private external title".to_string()),
                    is_default: true,
                    is_forced: true,
                }];
                let mut probe = sample_probe();
                probe.subtitle_tracks = vec![SubtitleTrack {
                    index: 2,
                    codec: "subrip".to_string(),
                    language: Some("eng".to_string()),
                    label: Some("Private source title".to_string()),
                }];
                let args = build_ffmpeg_args("source.mkv", "output.mov", &config, &probe).unwrap();
                for tag in [
                    "language=eng",
                    "language=pol",
                    "title=Private source title",
                    "title=Private external title",
                    "handler_name=Private external title",
                ] {
                    assert_eq!(args.iter().any(|arg| arg == tag), preserve, "{mode}: {tag}");
                }
                assert!(args_contains_pair(
                    &args,
                    "-disposition:s:1",
                    "default+forced"
                ));
            }
        }
    }

    #[test]
    fn burn_clock_is_shared_by_image_gif_and_overlay_graphs() {
        for (container, codec, overlay) in [
            ("png", "png", false),
            ("gif", "gif", false),
            ("mp4", "libx264", true),
        ] {
            let mut config = sample_config(container, codec);
            config.subtitle_burn_path = Some("/tmp/sub.srt".to_string());
            config.start_time = Some("1.234567".to_string());
            config.end_time = Some("2.345678".to_string());
            if overlay {
                config.overlay = Some(crate::types::OverlayConfig {
                    enabled: true,
                    path: "/tmp/overlay.png".to_string(),
                    x: 0.5,
                    y: 0.5,
                    width: 0.25,
                    opacity: 1.0,
                    anchor: "center".to_string(),
                });
            }
            // These builders share burn construction even where the product's
            // current UI does not expose subtitles for image/GIF outputs.
            let args =
                build_ffmpeg_args("input.mp4", "output.media", &config, &sample_probe()).unwrap();
            assert!(args.iter().any(|arg| arg.contains(
                "setpts=PTS+1.234567/TB,subtitles=filename=/tmp/sub.srt,setpts=PTS-1.234567/TB"
            )));
            assert!(args_contains_pair(&args, "-t", "1.111111"));
        }
    }

    #[test]
    fn gif_palette_requires_space_for_opaque_and_transparent_colors() {
        let input = temporary_input_file_with_extension("gif-palette-input", "png");
        let mut config = sample_config("gif", "gif");
        config.gif_colors = 2;
        assert!(validate_task_input(&input.to_string_lossy(), &config).is_err());
        config.gif_colors = MIN_GIF_COLORS;
        assert!(validate_task_input(&input.to_string_lossy(), &config).is_ok());
        let graph = build_gif_filter_complex(&config, VisualFilterContext::default());
        assert!(graph.contains("palettegen=max_colors=3"));
        assert!(!graph.contains("reserve_transparent=0"));
        fs::remove_file(input).unwrap();
    }

    #[test]
    fn nonfinite_trim_times_are_rejected_before_starting_ffmpeg() {
        let input = temporary_input_file_with_extension("nonfinite-trim-input", "mp4");
        for mode in ["copy", "reencode"] {
            for value in ["NaN", "inf", "-inf", "1e309", "00:00:NaN"] {
                let mut config = sample_config("mp4", "libx264");
                config.processing_mode = mode.to_string();
                config.start_time = Some(value.to_string());
                let error = validate_task_input(&input.to_string_lossy(), &config).unwrap_err();
                assert!(error.to_string().contains("Invalid start time"));
                config.start_time = None;
                config.end_time = Some(value.to_string());
                let error = validate_task_input(&input.to_string_lossy(), &config).unwrap_err();
                assert!(error.to_string().contains("Invalid end time"));
            }
        }
        fs::remove_file(input).unwrap();
    }

    #[test]
    fn ignored_transport_subtitle_text_tags_do_not_block_clean_or_replace() {
        let input = temporary_input_file_with_extension("transport-metadata-input", "mkv");
        let subtitle = temporary_input_file_with_extension("transport-metadata-sidecar", "sup");
        for container in ["m2t", "m2ts"] {
            let mut config = sample_config(container, "libx264");
            config.audio_codec = "ac3".to_string();
            config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
                path: subtitle.to_string_lossy().into_owned(),
                language: Some("eng".to_string()),
                title: Some("Private title".to_string()),
                ..ExternalSubtitleTrack::default()
            }];
            assert!(validate_task_input(&input.to_string_lossy(), &config).is_err());
            for metadata_mode in [MetadataMode::Clean, MetadataMode::Replace] {
                config.metadata.mode = metadata_mode;
                assert!(validate_task_input(&input.to_string_lossy(), &config).is_ok());
                config.external_subtitle_tracks[0].is_forced = true;
                assert!(validate_task_input(&input.to_string_lossy(), &config).is_err());
                config.external_subtitle_tracks[0].is_forced = false;
            }
        }
        fs::remove_file(input).unwrap();
        fs::remove_file(subtitle).unwrap();
    }

    #[test]
    fn hevc_reencode_uses_hvc1_for_mp4_and_mov() {
        for container in ["mp4", "mov"] {
            for codec in ["libx265", "hevc_videotoolbox", "hevc_nvenc"] {
                let config = sample_config(container, codec);
                let args = build_ffmpeg_args(
                    "input.mp4",
                    &format!("output.{container}"),
                    &config,
                    &sample_probe(),
                )
                .unwrap();

                assert!(args_contains_pair(&args, "-c:v", codec));
                assert!(args_contains_pair(&args, "-tag:v", "hvc1"));
            }
        }
    }

    #[test]
    fn video_tag_is_not_overridden_for_other_codecs_or_containers() {
        let mut probe = sample_probe();
        probe.video_codec = Some("hevc".to_string());
        for (container, codec) in [
            ("mp4", "libx264"),
            ("mov", "h264_videotoolbox"),
            ("mp4", "libsvtav1"),
            ("mov", "prores"),
            ("mkv", "libx265"),
            ("mkv", "hevc_videotoolbox"),
            ("m2ts", "hevc_nvenc"),
            ("m4a", "hevc_videotoolbox"),
        ] {
            let mut config = sample_config(container, codec);
            config.selected_audio_tracks = vec![1];
            let args =
                build_ffmpeg_args("input.mp4", &format!("output.{container}"), &config, &probe)
                    .unwrap();

            assert!(
                !args.iter().any(|arg| arg == "-tag:v"),
                "{container}/{codec}"
            );
        }
    }

    #[test]
    fn stream_copy_keeps_video_tag_selection_with_the_muxer() {
        for container in ["mp4", "mov"] {
            for source_codec in ["h264", "hevc"] {
                let mut config = sample_config(container, "hevc_videotoolbox");
                config.processing_mode = "copy".to_string();
                let mut probe = sample_probe();
                probe.video_codec = Some(source_codec.to_string());
                let args =
                    build_ffmpeg_args("input.mp4", &format!("output.{container}"), &config, &probe)
                        .unwrap();

                assert!(args_contains_pair(&args, "-c", "copy"));
                assert!(!args.iter().any(|arg| arg == "-tag:v"));
            }
        }
    }

    #[test]
    fn build_ffmpeg_args_skips_padding_for_known_even_video_raster() {
        let config = sample_config("mp4", "libx264");
        let probe = ProbeMetadata {
            width: Some(1920),
            height: Some(1080),
            ..sample_probe()
        };
        let args = build_ffmpeg_args("input.mov", "output.mp4", &config, &probe)
            .expect("known even source should need no visual filter");
        assert!(!args.iter().any(|arg| arg == "-vf"));
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
    fn transport_stream_output_paths_preserve_public_suffixes() {
        for container in ["m2t", "mts", "m2ts"] {
            assert_eq!(
                build_output_path("/exports", container, Some("camera.M2T")),
                format!("/exports/camera.{container}")
            );
        }
    }

    #[test]
    fn transport_stream_profiles_emit_exact_muxer_tail_for_reencode_and_copy() {
        for (container, mode) in [("m2t", "0"), ("mts", "1"), ("m2ts", "1")] {
            for processing_mode in ["reencode", "copy"] {
                let mut config = sample_config(container, "libx264");
                config.processing_mode = processing_mode.to_string();
                config.audio_codec = if container == "m2t" { "mp2" } else { "ac3" }.to_string();
                let mut probe = sample_probe();
                probe.audio_tracks[0].codec =
                    if container == "m2t" { "mp2" } else { "ac3" }.to_string();
                let args =
                    build_ffmpeg_args("input.ts", &format!("output.{container}"), &config, &probe)
                        .unwrap();
                let expected = [
                    "-f".to_string(),
                    "mpegts".to_string(),
                    "-mpegts_m2ts_mode".to_string(),
                    mode.to_string(),
                    "-n".to_string(),
                    format!("output.{container}"),
                ];
                assert_eq!(&args[args.len() - 6..], expected);
            }
        }
    }

    #[test]
    fn stream_copy_maps_only_the_video_selected_during_probe() {
        let mut config = sample_config("m2t", "libx264");
        config.processing_mode = "copy".to_string();
        config.selected_audio_tracks = vec![1];
        let mut probe = sample_probe();
        probe.video_stream_index = Some(4);

        let args = build_ffmpeg_args("input.m2t", "output.m2t", &config, &probe).unwrap();

        assert!(args_contains_pair(&args, "-map", "0:4"));
        assert!(!args.iter().any(|arg| arg == "0:v?"));
    }

    #[test]
    fn mpeg2video_uses_bitrate_without_generic_crf_or_preset() {
        let mut config = sample_config("m2t", "mpeg2video");
        config.video_bitrate_mode = "bitrate".to_string();
        config.video_bitrate = "18000".to_string();
        config.audio_codec = "mp2".to_string();

        let args = build_ffmpeg_args("input.ts", "output.m2t", &config, &sample_probe()).unwrap();

        assert!(args_contains_pair(&args, "-b:v", "18000k"));
        assert!(!args.iter().any(|arg| arg == "-crf"));
        assert!(!args.iter().any(|arg| arg == "-preset"));
    }

    #[test]
    fn mpegts_metadata_preserve_prefers_service_tags_and_manual_overrides() {
        let mut config = sample_config("m2t", "mpeg2video");
        config.audio_codec = "mp2".to_string();
        config.metadata.service_name = Some("Manual".to_string());
        let mut probe = sample_probe();
        probe.transport_stream = Some(crate::types::TransportStreamMetadata {
            service_name: Some("Source".to_string()),
            service_provider: Some("Provider".to_string()),
            ..crate::types::TransportStreamMetadata::default()
        });

        let args = build_ffmpeg_args("input.m2t", "output.m2t", &config, &probe).unwrap();

        assert!(args_contains_pair(
            &args,
            "-metadata",
            "service_name=Manual"
        ));
        assert!(args_contains_pair(
            &args,
            "-metadata",
            "service_provider=Provider"
        ));
    }

    #[test]
    fn bitmap_subtitles_use_per_track_actions_for_transport_profiles() {
        let mut m2t = sample_config("m2t", "mpeg2video");
        m2t.audio_codec = "mp2".to_string();
        m2t.selected_subtitle_tracks = vec![2];
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }];
        let args = build_ffmpeg_args("input.mkv", "output.m2t", &m2t, &probe).unwrap();
        assert!(args_contains_pair(&args, "-c:s:0", "dvbsub"));
        assert!(!args.iter().any(|arg| arg == "-c:s"));

        let mut m2ts = sample_config("m2ts", "libx264");
        m2ts.audio_codec = "ac3".to_string();
        m2ts.selected_subtitle_tracks = vec![2];
        let args = build_ffmpeg_args("input.mkv", "output.m2ts", &m2ts, &probe).unwrap();
        assert!(args_contains_pair(&args, "-c:s:0", "copy"));
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
    fn opus_output_keeps_target_bitrate_in_both_vbr_modes() {
        let input = temporary_input_file_with_extension("opus-input", "flac");
        for bitrate in ["64", "128"] {
            for opus_vbr in [false, true] {
                let mut config = sample_config("opus", "libx264");
                config.audio_codec = "libopus".to_string();
                config.audio_bitrate = bitrate.to_string();
                config.opus_vbr = opus_vbr;
                config.selected_audio_tracks = vec![1];
                validate_task_input(&input.to_string_lossy(), &config).unwrap();
                let mut probe = sample_probe();
                probe.media_kind = "audio".to_string();
                probe.video_codec = None;
                probe.audio_tracks[0].codec = "flac".to_string();
                let args = build_ffmpeg_args("input.flac", "output.opus", &config, &probe).unwrap();
                assert!(args_contains_pair(&args, "-c:a", "libopus"));
                assert!(args_contains_pair(&args, "-b:a", &format!("{bitrate}k")));
                assert!(args_contains_pair(
                    &args,
                    "-vbr",
                    if opus_vbr { "on" } else { "off" }
                ));
                assert!(args_contains_pair(&args, "-map", "0:1"));
                assert!(args.iter().any(|arg| arg == "-vn"));
                for absent in ["-q:a", "-ar", "-c:v", "-c", "-c:s"] {
                    assert!(!args.iter().any(|arg| arg == absent), "{absent}: {args:?}");
                }
            }
        }
        fs::remove_file(input).unwrap();
    }

    #[test]
    fn legacy_opus_video_configs_keep_vbr_on() {
        for (container, video_codec) in [("webm", "vp9"), ("mp4", "libx264")] {
            let mut config = sample_config(container, video_codec);
            config.audio_codec = "libopus".to_string();
            config.selected_audio_tracks = vec![1];
            let mut legacy = serde_json::to_value(config).unwrap();
            legacy.as_object_mut().unwrap().remove("opusVbr");
            let config = serde_json::from_value(legacy).unwrap();
            let args = build_ffmpeg_args(
                "input.mov",
                &format!("output.{container}"),
                &config,
                &sample_probe(),
            )
            .unwrap();
            assert!(args_contains_pair(&args, "-b:a", "128k"));
            assert!(args_contains_pair(&args, "-vbr", "on"));
            assert!(!args.iter().any(|arg| arg == "-q:a"));
        }
    }

    #[test]
    fn opus_stream_copy_requires_opus_source_and_omits_encoder_options() {
        let mut config = sample_config("opus", "libx264");
        config.processing_mode = "copy".to_string();
        config.audio_codec = "libopus".to_string();
        config.selected_audio_tracks = vec![1];
        for opus_vbr in [false, true] {
            config.opus_vbr = opus_vbr;
            for codec in ["opus", "flac", "aac"] {
                let mut probe = sample_probe();
                probe.audio_tracks[0].codec = codec.to_string();
                let result = build_ffmpeg_args("input.mkv", "output.opus", &config, &probe);
                if codec != "opus" {
                    assert!(result.is_err(), "{codec} must not copy into Ogg Opus");
                    continue;
                }
                let args = result.unwrap();
                assert!(args_contains_pair(&args, "-c", "copy"));
                for absent in ["-b:a", "-vbr", "-q:a", "-ar", "-c:a"] {
                    assert!(!args.iter().any(|arg| arg == absent), "{absent}: {args:?}");
                }
            }
        }
    }

    #[test]
    fn opus_vbr_does_not_change_other_audio_encoder_arguments() {
        for (codec, mode, expected) in [
            ("aac", "bitrate", vec!["-c:a", "aac", "-b:a", "128k"]),
            ("mp3", "bitrate", vec!["-c:a", "mp3", "-b:a", "128k"]),
            ("mp3", "vbr", vec!["-c:a", "mp3", "-q:a", "4"]),
            (
                "libfdk_aac",
                "bitrate",
                vec!["-c:a", "libfdk_aac", "-b:a", "128k"],
            ),
            ("libfdk_aac", "vbr", vec!["-c:a", "libfdk_aac", "-vbr", "4"]),
            ("flac", "bitrate", vec!["-c:a", "flac"]),
            ("pcm_s16le", "bitrate", vec!["-c:a", "pcm_s16le"]),
            (
                "mp2",
                "bitrate",
                vec!["-c:a", "mp2", "-b:a", "128k", "-ar", "48000"],
            ),
        ] {
            for opus_vbr in [false, true] {
                let mut config = sample_config("mkv", "libx264");
                config.audio_codec = codec.to_string();
                config.audio_bitrate_mode = mode.to_string();
                config.opus_vbr = opus_vbr;
                let mut args = Vec::new();
                add_audio_codec_args(&mut args, &config);
                assert_eq!(args, expected, "{codec}, {mode}, opus_vbr={opus_vbr}");
            }
        }
    }

    #[test]
    fn build_ffmpeg_args_maps_only_audio_tracks_returned_by_probe() {
        let mut config = sample_config("mp4", "libx264");
        config.selected_audio_tracks = vec![1];
        let probe = sample_probe();

        let args = build_ffmpeg_args("spatial.mov", "output.mp4", &config, &probe)
            .expect("recognized AAC track should be mapped");

        assert!(args_contains_pair(&args, "-map", "0:1"));
        assert!(!args.iter().any(|arg| arg == "0:a?"));
        assert!(!args.iter().any(|arg| arg == "0:2"));
        assert!(args.iter().any(|arg| arg == "-dn"));
    }

    #[test]
    fn build_ffmpeg_args_drops_only_unselected_bitmap_subtitles_for_mp4() {
        let mut config = sample_config("mp4", "libx264");
        config.selected_subtitle_tracks = vec![3];
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
        assert!(args_contains_pair(&args, "-c:s:0", "mov_text"));
    }

    #[test]
    fn build_ffmpeg_args_omits_unselected_pgs_for_mp4() {
        let config = sample_config("mp4", "libx264");
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }];

        let args = build_ffmpeg_args("pgs.mkv", "output.mp4", &config, &probe)
            .expect("unselected PGS should not participate in the output");

        assert!(!args_contains_pair(&args, "-map", "0:2"));
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
        let mut config = sample_config("mkv", "libx264");
        config.selected_subtitle_tracks = vec![2];
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }];

        let args = build_ffmpeg_args("pgs.mkv", "output.mkv", &config, &probe)
            .expect("Matroska should preserve PGS subtitles");

        assert!(args_contains_pair(&args, "-map", "0:2"));
        assert!(args_contains_pair(&args, "-c:s:0", "copy"));
    }

    #[test]
    fn build_ffmpeg_args_omits_audio_when_no_track_is_selected() {
        let mut config = sample_config("mp4", "libx264");
        config.selected_audio_tracks.clear();

        let args = build_ffmpeg_args("input.mov", "output.mp4", &config, &sample_probe())
            .expect("video-only MP4 should build");

        assert!(args.iter().any(|arg| arg == "-an"));
        assert!(!args_contains_pair(&args, "-map", "0:1"));
    }

    #[test]
    fn build_ffmpeg_args_omits_subtitles_when_no_track_is_selected() {
        let config = sample_config("mkv", "libx264");
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "hdmv_pgs_subtitle".to_string(),
            ..SubtitleTrack::default()
        }];

        let args = build_ffmpeg_args("input.mkv", "output.mkv", &config, &probe)
            .expect("unselected subtitles should not participate in the output");

        assert!(!args_contains_pair(&args, "-map", "0:2"));
    }

    #[test]
    fn build_ffmpeg_args_rejects_audio_only_output_without_selected_track() {
        let mut config = sample_config("mp3", "libx264");
        config.selected_audio_tracks.clear();
        config.audio_codec = "mp3".to_string();

        let error = build_ffmpeg_args("input.mov", "output.mp3", &config, &sample_probe())
            .expect_err("audio-only output requires an explicit audio selection");

        assert!(
            error
                .to_string()
                .contains("Select at least one audio track")
        );
    }

    #[test]
    fn build_ffmpeg_args_embeds_external_subtitles_with_metadata_and_dispositions() {
        let mut config = sample_config("mp4", "libx264");
        config.start_time = Some("00:00:05.000".to_string());
        config.overlay = Some(crate::types::OverlayConfig {
            enabled: true,
            path: "logo.png".to_string(),
            ..crate::types::OverlayConfig::default()
        });
        config.external_subtitle_tracks = vec![
            ExternalSubtitleTrack {
                path: "english.srt".to_string(),
                language: Some("eng".to_string()),
                title: Some("English".to_string()),
                is_default: true,
                is_forced: false,
            },
            ExternalSubtitleTrack {
                path: "signs.vtt".to_string(),
                language: Some("eng".to_string()),
                title: Some("Signs & songs".to_string()),
                is_default: false,
                is_forced: true,
            },
        ];

        let args = build_ffmpeg_args("input.mov", "output.mp4", &config, &sample_probe())
            .expect("external subtitle arguments should build");

        assert!(
            args.windows(4)
                .any(|window| window == ["-ss", "00:00:05.000", "-i", "english.srt"])
        );
        assert!(
            args.windows(4)
                .any(|window| window == ["-ss", "00:00:05.000", "-i", "signs.vtt"])
        );
        assert!(args_contains_pair(&args, "-map", "2:s:0"));
        assert!(args_contains_pair(&args, "-map", "3:s:0"));
        assert!(args_contains_pair(&args, "-c:s:0", "mov_text"));
        assert!(args_contains_pair(&args, "-c:s:1", "mov_text"));
        assert!(args_contains_pair(&args, "-metadata:s:s:0", "language=eng"));
        assert!(args_contains_pair(
            &args,
            "-metadata:s:s:0",
            "title=English"
        ));
        assert!(args_contains_pair(&args, "-disposition:s:0", "default"));
        assert!(args_contains_pair(
            &args,
            "-metadata:s:s:1",
            "title=Signs & songs"
        ));
        assert!(args_contains_pair(&args, "-disposition:s:1", "forced"));
    }

    #[test]
    fn stream_copy_only_transcodes_the_added_external_subtitle_stream() {
        let mut config = sample_config("mp4", "libx264");
        config.processing_mode = "copy".to_string();
        config.selected_subtitle_tracks = vec![2];
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: "commentary.ass".to_string(),
            ..ExternalSubtitleTrack::default()
        }];
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "mov_text".to_string(),
            ..SubtitleTrack::default()
        }];

        let args = build_ffmpeg_args("input.mp4", "output.mp4", &config, &probe)
            .expect("copy arguments should build");

        assert!(args_contains_pair(&args, "-map", "0:2"));
        assert!(args_contains_pair(&args, "-map", "1:s:0"));
        assert!(args_contains_pair(&args, "-c", "copy"));
        assert!(args_contains_pair(&args, "-c:s:1", "mov_text"));
        assert!(args_contains_pair(&args, "-c:s:0", "copy"));
        assert!(args_contains_pair(&args, "-disposition:s:1", "0"));
    }

    #[test]
    fn burn_in_and_external_selectable_subtitles_remain_independent() {
        let mut config = sample_config("mp4", "libx264");
        config.subtitle_burn_path = Some("burn.srt".to_string());
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: "selectable.srt".to_string(),
            ..ExternalSubtitleTrack::default()
        }];
        let mut probe = sample_probe();
        probe.subtitle_tracks = vec![SubtitleTrack {
            index: 2,
            codec: "subrip".to_string(),
            ..SubtitleTrack::default()
        }];

        let args = build_ffmpeg_args("input.mov", "output.mp4", &config, &probe)
            .expect("combined subtitle modes should build");

        assert!(args_contains_pair(&args, "-map", "1:s:0"));
        assert!(!args_contains_pair(&args, "-map", "0:2"));
        assert!(args_contains_pair(&args, "-c:s:0", "mov_text"));
    }

    #[test]
    fn validate_task_input_accepts_supported_external_subtitle_file() {
        let input = temporary_input_file("external-subtitle-input");
        let subtitle = temporary_input_file_with_extension("external-subtitle", "srt");
        let mut config = sample_config("mp4", "libx264");
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: subtitle.to_string_lossy().to_string(),
            language: Some("eng".to_string()),
            title: Some("English".to_string()),
            is_default: true,
            is_forced: true,
        }];

        let result = validate_task_input(&input.to_string_lossy(), &config);

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(subtitle);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_task_input_rejects_selectable_text_for_transport_stream_without_deleting_draft() {
        let input = temporary_input_file("transport-text-subtitle-input");
        let subtitle = temporary_input_file_with_extension("transport-text-subtitle", "srt");
        let mut config = sample_config("m2t", "mpeg2video");
        config.audio_codec = "mp2".to_string();
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: subtitle.to_string_lossy().to_string(),
            ..ExternalSubtitleTrack::default()
        }];

        let error = validate_task_input(&input.to_string_lossy(), &config).unwrap_err();

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(subtitle);
        assert!(error.to_string().contains("use Burn-in"));
        assert_eq!(config.external_subtitle_tracks.len(), 1);
    }

    #[test]
    fn validate_task_input_accepts_sup_for_both_transport_profiles() {
        let input = temporary_input_file("transport-sup-input");
        let subtitle = temporary_input_file_with_extension("transport-sup", "sup");
        for (container, video_codec, audio_codec) in
            [("m2t", "mpeg2video", "mp2"), ("m2ts", "libx264", "ac3")]
        {
            let mut config = sample_config(container, video_codec);
            config.audio_codec = audio_codec.to_string();
            config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
                path: subtitle.to_string_lossy().to_string(),
                ..ExternalSubtitleTrack::default()
            }];
            assert!(validate_task_input(&input.to_string_lossy(), &config).is_ok());
        }

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(subtitle);
    }

    #[test]
    fn validate_task_input_rejects_duplicate_external_subtitle_files() {
        let input = temporary_input_file("duplicate-subtitle-input");
        let subtitle = temporary_input_file_with_extension("duplicate-subtitle", "vtt");
        let track = ExternalSubtitleTrack {
            path: subtitle.to_string_lossy().to_string(),
            ..ExternalSubtitleTrack::default()
        };
        let mut config = sample_config("mp4", "libx264");
        config.external_subtitle_tracks = vec![track.clone(), track];

        let error = validate_task_input(&input.to_string_lossy(), &config)
            .expect_err("duplicate sidecars should be rejected");

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(subtitle);
        assert!(error.to_string().contains("more than once"));
    }

    #[test]
    fn validate_task_input_rejects_multiple_default_external_subtitles() {
        let input = temporary_input_file("default-subtitle-input");
        let first = temporary_input_file_with_extension("default-subtitle-first", "srt");
        let second = temporary_input_file_with_extension("default-subtitle-second", "ass");
        let mut config = sample_config("mkv", "libx264");
        config.external_subtitle_tracks = vec![
            ExternalSubtitleTrack {
                path: first.to_string_lossy().to_string(),
                is_default: true,
                ..ExternalSubtitleTrack::default()
            },
            ExternalSubtitleTrack {
                path: second.to_string_lossy().to_string(),
                is_default: true,
                ..ExternalSubtitleTrack::default()
            },
        ];

        let error = validate_task_input(&input.to_string_lossy(), &config)
            .expect_err("multiple default sidecars should be rejected");

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(first);
        let _ = fs::remove_file(second);
        assert!(error.to_string().contains("Only one"));
    }

    #[test]
    fn validate_task_input_rejects_unsupported_external_subtitle_format() {
        let input = temporary_input_file("unsupported-subtitle-input");
        let subtitle = temporary_input_file_with_extension("unsupported-subtitle", "txt");
        let mut config = sample_config("mp4", "libx264");
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: subtitle.to_string_lossy().to_string(),
            ..ExternalSubtitleTrack::default()
        }];

        let error = validate_task_input(&input.to_string_lossy(), &config)
            .expect_err("unsupported sidecar format should be rejected");

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(subtitle);
        assert!(error.to_string().contains("Unsupported external subtitle"));
    }

    #[test]
    fn validate_task_input_rejects_external_subtitles_for_audio_output() {
        let input = temporary_input_file("audio-subtitle-input");
        let subtitle = temporary_input_file_with_extension("audio-subtitle", "srt");
        let mut config = sample_config("mp3", "libx264");
        config.audio_codec = "mp3".to_string();
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: subtitle.to_string_lossy().to_string(),
            ..ExternalSubtitleTrack::default()
        }];

        let error = validate_task_input(&input.to_string_lossy(), &config)
            .expect_err("audio output cannot carry selectable subtitles");

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(subtitle);
        assert!(
            error
                .to_string()
                .contains("Subtitle options are not available")
        );
    }

    #[test]
    fn validate_task_input_rejects_control_characters_in_subtitle_metadata() {
        let input = temporary_input_file("metadata-subtitle-input");
        let subtitle = temporary_input_file_with_extension("metadata-subtitle", "ass");
        let mut config = sample_config("mkv", "libx264");
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: subtitle.to_string_lossy().to_string(),
            title: Some("Unsafe\nTitle".to_string()),
            ..ExternalSubtitleTrack::default()
        }];

        let error = validate_task_input(&input.to_string_lossy(), &config)
            .expect_err("control characters should be rejected");

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(subtitle);
        assert!(error.to_string().contains("control characters"));
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

    fn temporary_input_file_with_extension(name: &str, extension: &str) -> PathBuf {
        let path = temporary_input_file(name);
        let destination = path.with_extension(extension);
        fs::rename(path, &destination).expect("temporary subtitle should be renamed");
        destination
    }
}
