use crate::trim::{OutputTrim, SubtitleTrimPlan};
use std::{collections::HashSet, path::Path};

mod alac;
pub use alac::{AlacCopyObserver, CopyAudioOutput, build_alac_copy_observer};

use crate::audio_rules::{
    AudioStreamCopySource, audio_stream_copy_error_with_source, audio_track_count_error,
    effective_audio_channels, mp3_requires_mp4_resampling, opus_encoding_error,
    stereo_width_supports_channels,
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
    FrameRateEofAction, VisualFilterContext, VisualFilterProfile, build_audio_filters,
    build_visual_filter_chain_with_context, build_visual_filter_complex_with_context, has_overlay,
};
use crate::input_map::{embedded_subtitle_action, resolve_reencode_main_input_map};
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
use crate::utils::{get_hwaccel_args, is_audio_only_container};

fn is_copy_mode(config: &ConversionConfig) -> bool {
    config.processing_mode == "copy"
}

/// Returns the configured video encoder only when this task actually uses it.
#[must_use]
pub fn effective_video_encoder<'a>(
    config: &'a ConversionConfig,
    probe: &ProbeMetadata,
) -> Option<&'a str> {
    if is_copy_mode(config)
        || is_audio_only_container(&config.container)
        || is_image_container(&config.container)
        || config.container.eq_ignore_ascii_case("gif")
        || (probe.video_stream_index.is_none() && probe.video_codec.is_none())
    {
        None
    } else {
        Some(&config.video_codec)
    }
}

fn has_custom_pixel_format(config: &ConversionConfig) -> bool {
    let pixel_format = config.pixel_format.trim();
    !pixel_format.is_empty() && pixel_format != "auto"
}

fn collect_selected_audio_tracks<'a>(
    config: &ConversionConfig,
    probe: &'a ProbeMetadata,
) -> Result<Vec<&'a AudioTrack>, ConversionError> {
    collect_audio_tracks(&config.selected_audio_tracks, probe)
}

fn collect_audio_tracks<'a>(
    indices: &[u32],
    probe: &'a ProbeMetadata,
) -> Result<Vec<&'a AudioTrack>, ConversionError> {
    indices
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

fn validate_audio_copy_track(
    container: &str,
    probe: &ProbeMetadata,
    track: &AudioTrack,
) -> Result<(), ConversionError> {
    if let Some(reason) = audio_stream_copy_error_with_source(
        container,
        &track.codec,
        track.sample_rate.as_deref(),
        AudioStreamCopySource {
            format: probe.source_format.as_deref(),
            channels: track.channels.parse().ok(),
            codec_tag: track.codec_tag,
        },
    ) {
        return Err(ConversionError::InvalidInput(format!(
            "Source audio track #{} ('{}') cannot be copied to '{}': {reason}",
            track.index, track.codec, container
        )));
    }
    Ok(())
}

fn collect_selected_subtitle_tracks<'a>(
    config: &ConversionConfig,
    probe: &'a ProbeMetadata,
) -> Result<Vec<&'a SubtitleTrack>, ConversionError> {
    collect_subtitle_tracks(&config.selected_subtitle_tracks, probe)
}

fn collect_subtitle_tracks<'a>(
    indices: &[u32],
    probe: &'a ProbeMetadata,
) -> Result<Vec<&'a SubtitleTrack>, ConversionError> {
    indices
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

fn add_audio_title_metadata(
    args: &mut Vec<String>,
    config: &ConversionConfig,
    tracks: &[&AudioTrack],
) {
    if config.metadata.mode != MetadataMode::Preserve
        || !(config.container.eq_ignore_ascii_case("mp4")
            || config.container.eq_ignore_ascii_case("mov"))
    {
        return;
    }
    for (output_index, track) in tracks.iter().enumerate() {
        if let Some(title) = &track.metadata_title {
            args.extend([
                format!("-metadata:s:a:{output_index}"),
                format!("title={title}"),
            ]);
        }
    }
}

fn add_track_maps<T>(args: &mut Vec<String>, tracks: &[&T], index: impl Fn(&T) -> u32) {
    for track in tracks {
        args.push("-map".to_string());
        args.push(format!("0:{}", index(track)));
    }
}

fn add_external_subtitle_inputs(
    args: &mut Vec<String>,
    config: &ConversionConfig,
    trim: &OutputTrim,
) {
    for track in &config.external_subtitle_tracks {
        trim.append_input_args(args);
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
            validate_audio_copy_track(&config.container, probe, track)?;
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
            validate_audio_copy_track(&config.container, probe, track)?;
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
    build_ffmpeg_args_with_gif_policy(input, output, config, probe, GifFramePolicy::Ordinary)
}

/// A request-local GIF decision. It is never stored in conversion settings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GifFramePolicy {
    #[default]
    Ordinary,
    ConfirmedMinimumFrame,
}

/// Builds export arguments with an independently verified minimum-frame policy.
///
/// # Errors
/// Returns the same validation errors as [`build_ffmpeg_args`].
#[expect(
    clippy::too_many_lines,
    reason = "FFmpeg command assembly stays in one place to keep ordering guarantees explicit"
)]
pub fn build_ffmpeg_args_with_gif_policy(
    input: &str,
    output: &str,
    config: &ConversionConfig,
    probe: &ProbeMetadata,
    gif_policy: GifFramePolicy,
) -> Result<Vec<String>, ConversionError> {
    validate_audio_encoding_tracks(config, probe)?;
    let trim = OutputTrim::from_config(config)?;
    let subtitle_trim = SubtitleTrimPlan::for_window(config, &trim);
    let mut filter_context = source_filter_context(probe);
    let mut args = Vec::new();

    // Hardware decode acceleration (must be before -i)
    if config.hw_decode
        && let Some(encoder) = effective_video_encoder(config, probe)
    {
        args.extend(get_hwaccel_args(encoder));
    }

    trim.append_input_args(&mut args);

    args.push("-i".to_string());
    args.push(input.to_string());

    if has_overlay(config)
        && let Some(overlay) = &config.overlay
    {
        args.push("-i".to_string());
        args.push(overlay.path.clone());
    }

    let first_external_subtitle_input = 1 + usize::from(has_overlay(config));
    add_external_subtitle_inputs(&mut args, config, &trim);

    trim.append_output_args(&mut args);

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
            add_audio_title_metadata(&mut args, config, &audio_tracks);
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
            if let Some(plan) = &subtitle_trim {
                plan.append_args(&mut args);
            }
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

    // These exact lists feed both FFmpeg's -map options and the main-input
    // timestamp clock. Separate overlay/sidecar inputs never enter this set.
    let mapped = resolve_reencode_main_input_map(
        config,
        &probe
            .audio_tracks
            .iter()
            .map(|track| track.index)
            .collect::<Vec<_>>(),
        &probe
            .subtitle_tracks
            .iter()
            .map(|track| (track.index, track.codec.as_str()))
            .collect::<Vec<_>>(),
    )?;
    let mapped_audio = collect_audio_tracks(&mapped.audio, probe)?;
    let mapped_subtitles = collect_subtitle_tracks(&mapped.subtitles, probe)?;
    if !is_audio_only && crate::filters::has_subtitle_burn(config) {
        configure_subtitle_clock(
            config,
            probe,
            &mut filter_context,
            &mapped_audio,
            &mapped_subtitles,
        )?;
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
        let recover = gif_policy == GifFramePolicy::ConfirmedMinimumFrame
            && is_gif_minimum_frame_candidate(config);
        args.push(build_gif_filter_complex_with_eof(
            config,
            filter_context,
            if recover {
                FrameRateEofAction::Pass
            } else {
                FrameRateEofAction::Round
            },
        ));

        args.push("-map".to_string());
        args.push("[gif_out]".to_string());
        args.push("-an".to_string());

        args.push("-c:v".to_string());
        args.push("gif".to_string());
        if recover {
            args.extend(["-frames:v".to_string(), "1".to_string()]);
        }

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

        let audio_tracks = mapped_audio;
        add_track_maps(&mut args, &audio_tracks, |track| track.index);
        add_audio_title_metadata(&mut args, config, &audio_tracks);

        if audio_tracks.is_empty() {
            args.push("-an".to_string());
        } else {
            add_audio_codec_args(&mut args, config);
        }

        let mut subtitle_output_count = 0;
        let subtitle_tracks = mapped_subtitles;
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
            if let Some(plan) = &subtitle_trim {
                plan.append_args(&mut args);
            }
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

fn source_filter_context(probe: &ProbeMetadata) -> VisualFilterContext {
    VisualFilterContext {
        source_is_palette: probe.pixel_format.as_deref() == Some("pal8"),
        ..VisualFilterContext::new(probe.width, probe.height)
    }
}

fn configure_subtitle_clock(
    config: &ConversionConfig,
    probe: &ProbeMetadata,
    context: &mut VisualFilterContext,
    audio: &[&AudioTrack],
    subtitles: &[&SubtitleTrack],
) -> Result<(), ConversionError> {
    let video_index = probe
        .source_timing
        .as_ref()
        .and_then(|timing| timing.first_video_stream_index);
    let enabled = video_index
        .into_iter()
        .chain(audio.iter().map(|track| track.index))
        .chain(subtitles.iter().map(|track| track.index))
        .collect::<Vec<_>>();
    context.source_clock = crate::source_timing::resolve_source_clock(
        probe.source_timing.as_ref(),
        probe.source_format.as_deref(),
        video_index,
        &enabled,
    );
    let seek = config
        .start_time
        .as_deref()
        .and_then(crate::trim::RawFfmpegTime::parse)
        .map_or(0, |time| time.microseconds);
    crate::filters::validate_subtitle_clock(config, context.source_clock, seek)
}

/// Only explicit-rate GIF reencoding needs an optional minimum-frame check.
#[must_use]
pub fn is_gif_minimum_frame_candidate(config: &ConversionConfig) -> bool {
    config.container.eq_ignore_ascii_case("gif")
        && !is_copy_mode(config)
        && config.fps != "original"
}

/// Builds the exact GIF visual prefix with a null output and one-frame limit.
///
/// This low-level helper does not expand application support for GIF subtitles
/// or overlays. Valid tasks are validated before this optional preflight.
///
/// # Errors
/// Returns an error for invalid trim or source-clock settings.
pub fn build_gif_probe_args(
    input: &str,
    config: &ConversionConfig,
    probe: &ProbeMetadata,
    eof_action: FrameRateEofAction,
) -> Result<Option<Vec<String>>, ConversionError> {
    if !is_gif_minimum_frame_candidate(config) {
        return Ok(None);
    }
    let trim = OutputTrim::from_config(config)?;
    let mut context = source_filter_context(probe);
    if crate::filters::has_subtitle_burn(config) {
        configure_subtitle_clock(config, probe, &mut context, &[], &[])?;
    }
    let mut args = ["-nostdin", "-hide_banner", "-loglevel", "error", "-xerror"]
        .map(str::to_owned)
        .to_vec();
    trim.append_input_args(&mut args);
    args.extend(["-i".to_owned(), input.to_owned()]);
    trim.append_output_args(&mut args);
    let prefix = gif_visual_prefix(config, context, eof_action).join(",");
    args.extend([
        "-filter_complex".to_owned(),
        format!("[0:v:0]{prefix}[probe_out]"),
    ]);
    args.extend(
        [
            "-map",
            "[probe_out]",
            "-an",
            "-sn",
            "-dn",
            "-c:v",
            "wrapped_avframe",
            "-fps_mode",
            "passthrough",
            "-frames:v",
            "1",
            "-progress",
            "pipe:1",
            "-nostats",
            "-f",
            "null",
            "-",
        ]
        .map(str::to_owned),
    );
    Ok(Some(args))
}

#[cfg(test)]
fn build_gif_filter_complex(
    config: &ConversionConfig,
    filter_context: VisualFilterContext,
) -> String {
    build_gif_filter_complex_with_eof(config, filter_context, FrameRateEofAction::Round)
}

fn gif_visual_prefix(
    config: &ConversionConfig,
    filter_context: VisualFilterContext,
    eof_action: FrameRateEofAction,
) -> Vec<String> {
    let mut filters = build_visual_filter_chain_with_context(
        config,
        VisualFilterProfile::ExportImage,
        filter_context,
    );
    if config.fps != "original" {
        filters.push(eof_action.filter(&config.fps));
    }
    filters
}

fn build_gif_filter_complex_with_eof(
    config: &ConversionConfig,
    filter_context: VisualFilterContext,
    eof_action: FrameRateEofAction,
) -> String {
    let filters = gif_visual_prefix(config, filter_context, eof_action);
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

    OutputTrim::from_config(config)?;

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
        && !is_audio_only_container(&config.container)
        && !is_video_only_container(&config.container)
        && !is_image_container(&config.container)
        && config.video_codec != "prores"
        && (config.video_codec == "mpeg2video" || config.video_bitrate_mode == "bitrate")
    {
        let bitrate = config.video_bitrate.parse::<f64>().map_err(|_| {
            ConversionError::InvalidInput(format!(
                "Invalid video bitrate: {}",
                config.video_bitrate
            ))
        })?;
        if !bitrate.is_finite() || bitrate <= 0.0 {
            return Err(ConversionError::InvalidInput(
                "Video bitrate must be a finite, positive number".to_string(),
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

    mod alac_observer;
    mod audio_titles;
    use crate::filters::EVEN_DIMENSIONS_FILTER;
    use crate::types::OverlayConfig;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn parsed_track_name_uses_existing_preserve_policy_without_leaking_into_clean_or_replace() {
        let probe = crate::probe::parse_ffprobe_stdout("source.mp4", serde_json::json!({
            "format":{"tags":{"title":"Global title","name":"Global name"}},
            "streams":[
                {"index":0,"codec_type":"video","codec_name":"h264"},
                {"index":4,"codec_type":"subtitle","codec_name":"mov_text","tags":{"name":"PRIVATE SUBTITLE","handler_name":"SubtitleHandler","language":"pol"}}
            ]
        }).to_string()).unwrap();
        for mode in [
            MetadataMode::Preserve,
            MetadataMode::Clean,
            MetadataMode::Replace,
        ] {
            let mut config = sample_config("mkv", "libx264");
            config.selected_subtitle_tracks = vec![4];
            config.metadata.mode = mode;
            if config.metadata.mode == MetadataMode::Replace {
                config.metadata.title = Some("Replacement title".into());
            }
            let args = build_ffmpeg_args("source.mp4", "out.mkv", &config, &probe).unwrap();
            let tags = args
                .windows(2)
                .filter(|pair| pair[0] == "-metadata:s:s:0")
                .map(|pair| pair[1].as_str())
                .collect::<Vec<_>>();
            if config.metadata.mode == MetadataMode::Preserve {
                assert_eq!(tags, ["language=pol", "title=PRIVATE SUBTITLE"]);
            } else {
                assert!(tags.is_empty());
                assert!(args.windows(2).any(|pair| pair == ["-map_metadata", "-1"]));
            }
            assert!(args.windows(2).any(|pair| pair == ["-map", "0:4"]));
            assert!(
                !args
                    .iter()
                    .any(|arg| arg.contains("SubtitleHandler") || arg.contains("Global name"))
            );
            assert_eq!(
                args.iter().any(|arg| arg == "title=Replacement title"),
                config.metadata.mode == MetadataMode::Replace
            );
        }
    }

    #[test]
    fn gif_probe_preserves_structural_geometry_trim_and_only_varies_terminal_fps() {
        let mut config = sample_config("gif", "gif");
        config.fps = "8".into();
        config.start_time = Some(".123456789".into());
        config.end_time = Some("1.5".into());
        config.crop = Some(crate::types::CropConfig {
            enabled: true,
            x: 3.0,
            y: 5.0,
            width: 127.0,
            height: 129.0,
            ..crate::types::CropConfig::default()
        });
        config.resolution = "custom".into();
        config.custom_width = Some("131".into());
        // Leave an odd inner raster and real padding; 131x133 would fit exactly.
        config.custom_height = Some("139".into());
        let mut probe = sample_probe();
        probe.width = Some(320);
        probe.height = Some(180);
        let ordinary = build_ffmpeg_args("source.mkv", "out.gif", &config, &probe).unwrap();
        let normal = build_gif_probe_args("source.mkv", &config, &probe, FrameRateEofAction::Round)
            .unwrap()
            .unwrap();
        let pass = build_gif_probe_args("source.mkv", &config, &probe, FrameRateEofAction::Pass)
            .unwrap()
            .unwrap();
        let value = |args: &[String], flag: &str| {
            args.windows(2)
                .find(|pair| pair[0] == flag)
                .map(|pair| pair[1].clone())
                .unwrap()
        };
        let graph = value(&normal, "-filter_complex");
        let prefix = graph.strip_suffix("[probe_out]").unwrap();
        assert!(prefix.contains("xstack="), "GIF prefix: {prefix}");
        assert!(prefix.contains("blend="), "GIF prefix: {prefix}");
        assert!(prefix.contains("split="), "GIF prefix: {prefix}");
        assert!(prefix.ends_with(",fps=8"));
        assert!(
            value(&ordinary, "-filter_complex")
                .starts_with(&format!("{prefix},split[gif_src][gif_palette_src]"))
        );
        assert_eq!(
            value(&pass, "-filter_complex"),
            format!("{prefix}:eof_action=pass[probe_out]")
        );
        assert!(!graph.contains("palette"));
        for flag in ["-ss", "-i", "-t"] {
            assert_eq!(value(&normal, flag), value(&ordinary, flag));
        }
        assert_eq!(value(&normal, "-ss"), "0.123456789");
        assert_eq!(value(&normal, "-map"), "[probe_out]");
        assert_eq!(value(&normal, "-c:v"), "wrapped_avframe");
        assert_eq!(value(&normal, "-fps_mode"), "passthrough");
        assert_eq!(value(&normal, "-progress"), "pipe:1");
        assert_eq!(normal.last().unwrap(), "-");
        assert!(!normal.contains(&"out.gif".to_string()));
        assert!(normal.contains(&"-xerror".to_string()));
        let recovery = build_ffmpeg_args_with_gif_policy(
            "source.mkv",
            "out.gif",
            &config,
            &probe,
            GifFramePolicy::ConfirmedMinimumFrame,
        )
        .unwrap();
        let mut restored = recovery;
        let cap = restored.iter().position(|arg| arg == "-frames:v").unwrap();
        assert_eq!(restored[cap + 1], "1");
        restored.drain(cap..cap + 2);
        assert!(!restored.contains(&"-frames:v".to_string()));
        let index = restored
            .iter()
            .position(|arg| arg == "-filter_complex")
            .unwrap()
            + 1;
        restored[index] = restored[index].replace("fps=8:eof_action=pass", "fps=8");
        assert_eq!(restored, ordinary);
    }

    #[test]
    fn gif_recovery_is_not_persisted_or_applied_to_unrelated_outputs() {
        for (container, codec, fps) in [
            ("gif", "gif", "original"),
            ("mp4", "libx264", "8"),
            ("png", "png", "8"),
        ] {
            let mut config = sample_config(container, codec);
            config.fps = fps.into();
            let ordinary = build_ffmpeg_args("source", "out", &config, &sample_probe()).unwrap();
            assert!(
                build_gif_probe_args("source", &config, &sample_probe(), FrameRateEofAction::Pass)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                build_ffmpeg_args_with_gif_policy(
                    "source",
                    "out",
                    &config,
                    &sample_probe(),
                    GifFramePolicy::ConfirmedMinimumFrame
                )
                .unwrap(),
                ordinary
            );
        }
        let mut config = sample_config("gif", "gif");
        config.fps = "8".into();
        config.processing_mode = "copy".into();
        assert!(!is_gif_minimum_frame_candidate(&config));
    }

    #[test]
    fn raw_gif_helper_preserves_burn_clock_but_application_policy_still_rejects_it() {
        let mut config = sample_config("gif", "gif");
        config.fps = "8".into();
        config.start_time = Some("-.123456789".into());
        config.subtitle_burn_path = Some("/tmp/captions.srt".into());
        assert!(validate_task_input("source.mkv", &config).is_err());
        let probe = sample_probe();
        let ordinary = build_ffmpeg_args("source.mkv", "out.gif", &config, &probe).unwrap();
        let args = build_gif_probe_args("source.mkv", &config, &probe, FrameRateEofAction::Round)
            .unwrap()
            .unwrap();
        let graph = &args[args
            .iter()
            .position(|arg| arg == "-filter_complex")
            .unwrap()
            + 1];
        let prefix = graph.strip_suffix("[probe_out]").unwrap();
        assert!(prefix.contains("subtitles="));
        assert!(
            prefix.contains("setpts=PTS+(round(-0.123456/TB))"),
            "GIF prefix: {prefix}"
        );
        assert!(
            prefix.contains("setpts=PTS-(round(-0.123456/TB))"),
            "GIF prefix: {prefix}"
        );
        assert!(ordinary.iter().any(|arg| arg.starts_with(prefix)));
        config.start_time = None;
        config.end_time = Some("00:00:00.123456789".into());
        let args = build_gif_probe_args("source", &config, &probe, FrameRateEofAction::Round)
            .unwrap()
            .unwrap();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-to", "00:00:00.123456789"])
        );
    }

    #[test]
    fn burn_clock_uses_the_same_main_input_tracks_as_export_maps() {
        let payload = serde_json::json!({"format":{"format_name":"mpegts","start_time":"1.400000"},"streams":[
            {"index":4,"codec_type":"video","codec_name":"h264","time_base":"1/90000","start_pts":145_920},
            {"index":7,"codec_type":"audio","codec_name":"aac","channels":2,"time_base":"1/90000","start_pts":126_000},
            {"index":9,"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","time_base":"1/90000","start_pts":126_000}
        ]});
        let probe = crate::probe::parse_ffprobe_stdout("source.ts", payload.to_string()).unwrap();
        for (audio, subtitles, shifted) in [
            (vec![], vec![], false),
            (vec![7], vec![], true),
            (vec![], vec![9], true),
            (vec![7], vec![9], true),
        ] {
            let mut config = sample_config("mkv", "libx264");
            config.start_time = Some(".123456789".into());
            config.subtitle_burn_path = Some("/tmp/burn.srt".into());
            config.selected_audio_tracks = audio.clone();
            config.selected_subtitle_tracks = subtitles.clone();
            let argv = build_ffmpeg_args("source.ts", "out.mkv", &config, &probe).unwrap();
            let graph = argv.windows(2).find(|pair| pair[0] == "-vf").unwrap();
            assert_eq!(graph[1].contains("round(11111/(90000*TB))"), shifted);
            for index in audio.iter().chain(&subtitles) {
                assert!(
                    argv.windows(2)
                        .any(|pair| pair[0] == "-map" && pair[1] == format!("0:{index}"))
                );
            }
            assert!(argv.windows(2).any(|pair| pair == ["-ss", "0.123456789"]));
        }
    }

    #[test]
    fn unused_clock_metadata_never_blocks_export_but_used_overflow_does() {
        use crate::source_timing::{SourceStreamTiming, SourceTiming, TimeBase};
        let mut probe = sample_probe();
        probe.source_format = Some("matroska,webm".into());
        probe.source_timing = Some(SourceTiming {
            format_start_microseconds: Some(i64::MAX),
            format_start_invalid: false,
            first_video_stream_index: Some(0),
            streams: vec![SourceStreamTiming {
                index: 0,
                time_base: TimeBase::new(1, 1_000_000),
                start_pts: Some(0),
            }],
        });
        let mut config = sample_config("mkv", "libx264");
        config.start_time = Some("0.000001".into());
        assert!(build_ffmpeg_args("source.mkv", "out.mkv", &config, &probe).is_ok());
        config.subtitle_burn_path = Some("/tmp/burn.srt".into());
        assert!(build_ffmpeg_args("source.mkv", "out.mkv", &config, &probe).is_err());
        config.start_time = Some("0.0000009".into());
        assert!(build_ffmpeg_args("source.mkv", "out.mkv", &config, &probe).is_ok());
        config.start_time = Some("0.000001".into());
        config.processing_mode = "copy".into();
        assert!(build_ffmpeg_args("source.mkv", "out.mkv", &config, &probe).is_ok());
    }

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
    fn copy_preflight_checks_only_selected_source_dependent_audio_tracks() {
        let mut probe = sample_probe();
        probe.source_format = Some("mov,mp4,m4a,3gp,3g2,mj2".into());
        probe.audio_tracks.push(AudioTrack {
            index: 3,
            codec: "adpcm_swf".into(),
            codec_tag: Some(0x4653_736d),
            channels: "1".into(),
            ..AudioTrack::default()
        });
        let mut config = sample_config("mkv", "libx264");
        config.processing_mode = "copy".into();
        for selected in [vec![3], vec![1, 3], vec![3, 1]] {
            config.selected_audio_tracks = selected.clone();
            let error = build_ffmpeg_args("misleading.wav", "output.mkv", &config, &probe)
                .unwrap_err()
                .to_string();
            assert!(error.contains("track #3"), "{error}");
            assert!(error.contains("Choose Re-encode"), "{error}");
            assert_eq!(config.selected_audio_tracks, selected);
        }
        for selected in [vec![], vec![1]] {
            config.selected_audio_tracks = selected;
            assert!(build_ffmpeg_args("input.mov", "output.mkv", &config, &probe).is_ok());
        }
        config.selected_audio_tracks = vec![3];
        config.processing_mode = "reencode".into();
        assert!(build_ffmpeg_args("input.mov", "output.mkv", &config, &probe).is_ok());
        config.processing_mode = "copy".into();
        for tag in [None, Some(0x5346)] {
            probe.audio_tracks[1].codec_tag = tag;
            assert!(build_ffmpeg_args("input.mov", "output.mkv", &config, &probe).is_ok());
        }
    }

    #[test]
    fn hardware_decoding_only_applies_to_an_effective_hardware_video_encoder() {
        let probe = sample_probe();
        let mut config = sample_config("mp4", "h264_nvenc");
        config.hw_decode = true;
        config.selected_audio_tracks = vec![1];
        let args = build_ffmpeg_args("input.mp4", "output.mp4", &config, &probe).unwrap();
        assert!(args_contains_pair(&args, "-hwaccel", "cuda"));
        assert!(!args.iter().any(|arg| arg == "-hwaccel_output_format"));
        for container in ["m4a", "png", "gif"] {
            config.container = container.into();
            let args = build_ffmpeg_args("input.mp4", "output", &config, &probe).unwrap();
            assert!(!args.iter().any(|arg| arg == "-hwaccel"), "{container}");
            assert_eq!(effective_video_encoder(&config, &probe), None);
        }
        config.container = "mp4".into();
        config.processing_mode = "copy".into();
        let args = build_ffmpeg_args("input.mp4", "output.mp4", &config, &probe).unwrap();
        assert!(!args.iter().any(|arg| arg == "-hwaccel"));
        config.processing_mode = "reencode".into();
        let audio_probe = ProbeMetadata {
            video_codec: None,
            ..probe
        };
        assert_eq!(effective_video_encoder(&config, &audio_probe), None);
        let args = build_ffmpeg_args("input.m4a", "output.mp4", &config, &audio_probe).unwrap();
        assert!(!args.iter().any(|arg| arg == "-hwaccel"));
    }

    #[test]
    fn video_encoder_controls_use_codec_specific_ffmpeg_options() {
        let mut config = sample_config("mov", "prores");
        for mode in ["crf", "bitrate"] {
            config.video_bitrate_mode = mode.to_string();
            config.video_bitrate = "unused".to_string();
            config.preset = "unused".to_string();
            let args =
                build_ffmpeg_args("input.mov", "output.mov", &config, &sample_probe()).unwrap();
            assert!(args_contains_pair(&args, "-c:v", "prores"));
            for unused in ["-crf", "-b:v", "-preset"] {
                assert!(!args.iter().any(|arg| arg == unused));
            }
        }
        config = sample_config("webm", "vp9");
        config.audio_codec = "libopus".to_string();
        for speed in ["0", "1", "2", "3", "4", "5", "medium"] {
            config.preset = speed.to_string();
            let args =
                build_ffmpeg_args("input.mov", "output.webm", &config, &sample_probe()).unwrap();
            assert!(args_contains_pair(&args, "-deadline", "good"));
            assert!(args_contains_pair(
                &args,
                "-cpu-used",
                if speed == "medium" { "1" } else { speed }
            ));
            assert!(!args.iter().any(|arg| arg == "-preset"));
        }
        for codec in ["h264_nvenc", "hevc_nvenc", "av1_nvenc"] {
            config = sample_config("mp4", codec);
            config.nvenc_spatial_aq = true;
            config.nvenc_temporal_aq = true;
            let args =
                build_ffmpeg_args("input.mov", "output.mp4", &config, &sample_probe()).unwrap();
            assert!(args_contains_pair(&args, "-spatial-aq", "1"));
            assert!(args_contains_pair(&args, "-temporal-aq", "1"));
            assert!(
                !args
                    .iter()
                    .any(|arg| matches!(arg.as_str(), "-spatial_aq" | "-temporal_aq"))
            );
        }
    }

    #[test]
    fn legacy_nvenc_default_preset_uses_supported_p4_for_each_encoder() {
        for codec in ["h264_nvenc", "hevc_nvenc", "av1_nvenc"] {
            let mut config = sample_config("mp4", codec);
            config.preset = "default".to_string();
            let args =
                build_ffmpeg_args("input.mov", "output.mp4", &config, &sample_probe()).unwrap();
            assert!(args_contains_pair(&args, "-preset", "p4"), "{codec}");
        }
    }

    #[test]
    fn video_bitrate_validation_checks_only_effective_bitrate_controls() {
        let input = temporary_input_file("effective-video-bitrate");
        for (container, codec, mode, used) in [
            ("mp4", "libx264", "bitrate", true),
            ("mkv", "mpeg2video", "crf", true),
            ("mov", "prores", "bitrate", false),
            ("png", "png", "bitrate", false),
            ("gif", "gif", "bitrate", false),
        ] {
            let mut config = sample_config(container, codec);
            config.video_bitrate_mode = mode.to_string();
            // MPEG-2 is offered in transport containers, where MP2 is valid.
            if codec == "mpeg2video" {
                config.container = "m2t".to_string();
                config.audio_codec = "mp2".to_string();
            }
            for value in ["NaN", "inf", "0", "invalid"] {
                config.video_bitrate = value.to_string();
                let result = validate_task_input(&input.to_string_lossy(), &config);
                assert_eq!(
                    result.is_err(),
                    used,
                    "{container}/{codec}/{value}: {result:?}"
                );
                if used {
                    assert!(result.unwrap_err().to_string().contains("bitrate"));
                }
            }
        }
        let _ = fs::remove_file(input);
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
                "setpts=PTS+(round(1.234567/TB)),subtitles=filename=/tmp/sub.srt,setpts=PTS-(round(1.234567/TB))"
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
    fn finite_scientific_start_is_rejected_before_it_can_overflow_trim_duration() {
        let mut config = sample_config("mkv", "libx264");
        config.start_time = Some("-1e308".into());
        config.end_time = Some("1e308".into());
        assert!(
            build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe())
                .unwrap_err()
                .to_string()
                .contains("Invalid start time")
        );
    }

    #[test]
    fn subtitle_trim_plan_skips_nontext_and_noop_windows_and_shares_output_precision() {
        use crate::trim::subtitle_trim_plan;
        let mut config = sample_config("mkv", "libx264");
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: "/tmp/captions.SRT".into(),
            ..ExternalSubtitleTrack::default()
        }];
        for start in [
            None,
            Some(""),
            Some("  "),
            Some("0"),
            Some("00:00:00.000"),
            Some("0.0000009"),
            Some("-1"),
        ] {
            config.start_time = start.map(str::to_string);
            assert!(subtitle_trim_plan(&config).unwrap().is_none());
        }
        config.start_time = Some(" 00:00:08.250001 ".into());
        let plan = subtitle_trim_plan(&config).unwrap().unwrap();
        assert!(plan.bitstream_filter.contains("max(0,DURATION+min(PTS,0))"));
        assert!(!plan.bitstream_filter.contains("/tb"));
        for (start, end, option, length) in [
            (None, " 00:00:01.234567 ", "-to", "1.234567"),
            (Some(" "), "1.234567", "-to", "1.234567"),
            (Some("00:00:08.250001"), "00:00:08.750004", "-t", "0.500003"),
            (Some("0"), "1", "-t", "1.000"),
        ] {
            config.start_time = start.map(str::to_string);
            config.end_time = Some(end.into());
            let plan = subtitle_trim_plan(&config).unwrap().unwrap();
            let args = build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe()).unwrap();
            assert!(args_contains_pair(
                &args,
                option,
                if option == "-to" { end.trim() } else { length }
            ));
            assert!(
                plan.bitstream_filter
                    .contains(&format!("gte(pts,{length}/tb)"))
            );
            assert!(
                plan.bitstream_filter
                    .contains(&format!("min(PTS+DURATION,{length}/TB)"))
            );
        }
        for path in ["/tmp/bitmap.sup", "/tmp/captions.ttml"] {
            config.external_subtitle_tracks[0].path = path.into();
            assert!(subtitle_trim_plan(&config).unwrap().is_none());
        }
        config.external_subtitle_tracks.clear();
        config.selected_subtitle_tracks = vec![7];
        config.subtitle_burn_path = Some("/tmp/burn.srt".into());
        assert!(subtitle_trim_plan(&config).unwrap().is_none());
        config.external_subtitle_tracks.push(ExternalSubtitleTrack {
            path: "/tmp/captions.srt".into(),
            ..ExternalSubtitleTrack::default()
        });
        config.container = "gif".into();
        assert!(subtitle_trim_plan(&config).unwrap().is_none());
    }

    #[test]
    fn raw_time_validation_is_role_aware_and_does_not_rewrite_saved_literals() {
        let mut config = sample_config("mkv", "libx264");
        for (start, literal, duration) in [
            (" .123456789 ", "0.123456789", "1.376543"),
            ("-.5", "-0.5", "2.000"),
            ("+.5", "+0.5", "1.000"),
            ("-00:00:00.5", "-00:00:00.5", "2.000"),
            ("-1:30", "-1:30", "91.500"),
        ] {
            config.start_time = Some(start.into());
            config.end_time = Some("1.5".into());
            let args = build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe()).unwrap();
            assert!(args_contains_pair(&args, "-ss", literal), "{start}");
            assert!(args_contains_pair(&args, "-t", duration), "{start}");
            assert_eq!(config.start_time.as_deref(), Some(start));
        }
        for (end, duration) in [
            ("1e-3", "0.001"),
            ("1:60", "120.000"),
            ("001:2", "62.000"),
            ("1.5:2", "92.000"),
            ("+1:30", "90.000"),
            ("1:-2", "58.000"),
        ] {
            config.start_time = Some("0".into());
            config.end_time = Some(end.into());
            let args = build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe()).unwrap();
            assert!(args_contains_pair(&args, "-t", duration), "{end}");
            config.start_time = None;
            assert!(
                build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe())
                    .unwrap_err()
                    .to_string()
                    .contains("Invalid end time"),
                "{end}"
            );
            config.start_time = Some(end.into());
            config.end_time = None;
            assert!(
                build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe())
                    .unwrap_err()
                    .to_string()
                    .contains("Invalid start time"),
                "{end}"
            );
        }
        config.start_time = None;
        config.end_time = Some(".123456789".into());
        let args = build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe()).unwrap();
        assert!(args_contains_pair(&args, "-to", "0.123456789"));
        assert_eq!(config.end_time.as_deref(), Some(".123456789"));
    }

    #[test]
    fn raw_end_only_time_keeps_ffmpeg_precision_while_generated_duration_keeps_existing_semantics()
    {
        let mut config = sample_config("mkv", "libx264");
        config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
            path: "/tmp/captions.srt".into(),
            ..ExternalSubtitleTrack::default()
        }];
        for start in [None, Some(""), Some("  ")] {
            config.start_time = start.map(str::to_string);
            for (end, length) in [
                ("0.9999999", "0.999999"),
                ("00:00:00.123456789", "0.123456"),
                ("1.0000009", "1.000000"),
                ("0.0000009", "0.000000"),
            ] {
                config.end_time = Some(end.into());
                let args =
                    build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe()).unwrap();
                assert!(args_contains_pair(&args, "-to", end));
                let plan = crate::trim::subtitle_trim_plan(&config).unwrap().unwrap();
                assert!(
                    plan.bitstream_filter
                        .contains(&format!("gte(pts,{length}/tb)"))
                );
            }
        }
        config.end_time = Some("1e-3".into());
        assert!(
            build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe())
                .unwrap_err()
                .to_string()
                .contains("Invalid end time")
        );
        config.start_time = Some("0".into());
        let args = build_ffmpeg_args("input.mkv", "out.mkv", &config, &sample_probe()).unwrap();
        assert!(args_contains_pair(&args, "-ss", "0"));
        assert!(args_contains_pair(&args, "-t", "0.001"));
        assert!(
            crate::trim::subtitle_trim_plan(&config)
                .unwrap()
                .unwrap()
                .bitstream_filter
                .contains("gte(pts,0.001/tb)")
        );
    }

    #[test]
    fn subtitle_trim_plan_keeps_mixed_output_ordinals_actions_and_metadata_independent() {
        let mut probe = sample_probe();
        probe.subtitle_tracks = [(1, "subrip"), (2, "hdmv_pgs_subtitle"), (3, "ass")]
            .map(|(index, codec)| SubtitleTrack {
                index,
                codec: codec.into(),
                ..SubtitleTrack::default()
            })
            .to_vec();
        for mode in ["copy", "reencode"] {
            for metadata_mode in [
                MetadataMode::Preserve,
                MetadataMode::Clean,
                MetadataMode::Replace,
            ] {
                let mut config = sample_config("mkv", "libx264");
                config.processing_mode = mode.into();
                config.start_time = Some("8".into());
                config.end_time = Some("9".into());
                config.selected_subtitle_tracks = vec![3, 2, 1];
                config.metadata.mode = metadata_mode;
                config.external_subtitle_tracks = ["srt", "sup", "ass", "vtt"]
                    .map(|extension| ExternalSubtitleTrack {
                        path: format!("/tmp/captions.{extension}"),
                        is_forced: true,
                        ..ExternalSubtitleTrack::default()
                    })
                    .to_vec();
                if mode == "reencode" {
                    config.overlay = Some(OverlayConfig {
                        enabled: true,
                        path: "/tmp/logo.png".into(),
                        width: 0.25,
                        opacity: 1.0,
                        anchor: "center".into(),
                        ..OverlayConfig::default()
                    });
                }
                let args = build_ffmpeg_args("input.mkv", "out.mkv", &config, &probe).unwrap();
                let ordinals: Vec<_> = args
                    .iter()
                    .filter(|arg| arg.starts_with("-bsf:"))
                    .map(String::as_str)
                    .collect();
                assert_eq!(ordinals, ["-bsf:s:3", "-bsf:s:5", "-bsf:s:6"]);
                // MKV preserves each native subtitle codec, including imported text.
                for ordinal in 0..7 {
                    assert!(args_contains_pair(
                        &args,
                        &format!("-c:s:{ordinal}"),
                        "copy"
                    ));
                }
                assert!(args_contains_pair(&args, "-map", "0:3"));
                assert!(args_contains_pair(
                    &args,
                    "-map",
                    if mode == "copy" { "1:s:0" } else { "2:s:0" }
                ));
                assert!(args_contains_pair(&args, "-disposition:s:3", "forced"));
                let plan = crate::trim::subtitle_trim_plan(&config).unwrap().unwrap();
                assert_eq!(plan.output_ordinals, [3, 5, 6]);
                assert_eq!(plan.required_bitstream_filters(), ["noise", "setts"]);
                let expected = "noise=amount=0:drop='if(gte(duration,0)*not(eq(pts,nopts))*gt(tb,0),gt(duration,0)*lte(pts+duration,0)+gte(pts,1.000/tb),0)',setts=pts='if(gte(DURATION,0)*not(eq(PTS,NOPTS))*gt(TB,0),max(PTS,0),PTS)':dts='if(gte(DURATION,0)*not(eq(PTS,NOPTS))*gt(TB,0)*not(eq(DTS,NOPTS)),max(DTS,0),DTS)':duration='if(gte(DURATION,0)*not(eq(PTS,NOPTS))*gt(TB,0)*gt(DURATION,0),max(0,min(PTS+DURATION,1.000/TB)-max(PTS,0)),DURATION)'";
                assert_eq!(plan.bitstream_filter, expected);
                assert!(args_contains_pair(&args, "-bsf:s:3", expected));
            }
        }
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
    fn build_ffmpeg_args_copies_libx265_frames_after_visual_filters() {
        let probe = ProbeMetadata {
            width: Some(48),
            height: Some(32),
            ..sample_probe()
        };
        for (flip_vertical, expected_filter) in [(false, "copy"), (true, "vflip,copy")] {
            let mut config = sample_config("mkv", "libx265");
            config.flip_vertical = flip_vertical;
            let args = build_ffmpeg_args("input.mp4", "output.mkv", &config, &probe)
                .expect("libx265 arguments should build");

            assert!(args_contains_pair(&args, "-vf", expected_filter));
        }
    }

    #[test]
    fn stream_copy_does_not_add_libx265_frame_copy() {
        let mut config = sample_config("mkv", "libx265");
        config.processing_mode = "copy".into();
        let args = build_ffmpeg_args("input.mp4", "output.mkv", &config, &sample_probe())
            .expect("stream-copy arguments should build");

        assert!(args_contains_pair(&args, "-c", "copy"));
        assert!(
            !args
                .iter()
                .any(|arg| matches!(arg.as_str(), "-vf" | "-filter_complex"))
        );
    }

    #[test]
    fn build_ffmpeg_args_copies_libx265_frames_after_overlay_composition() {
        let probe = ProbeMetadata {
            width: Some(48),
            height: Some(32),
            ..sample_probe()
        };
        for opacity in [0.0, 1.0] {
            let mut config = sample_config("mkv", "libx265");
            config.flip_vertical = true;
            config.overlay = Some(OverlayConfig {
                enabled: true,
                path: "/tmp/logo.png".into(),
                width: 0.25,
                opacity,
                ..OverlayConfig::default()
            });
            let args = build_ffmpeg_args("input.mp4", "output.mkv", &config, &probe)
                .expect("libx265 overlay arguments should build");
            let graph = args
                .windows(2)
                .find(|pair| pair[0] == "-filter_complex")
                .map(|pair| pair[1].as_str())
                .expect("overlay should use a complex graph");

            assert!(graph.ends_with(";[video_composed]copy[vout]"), "{graph}");
            assert_eq!(graph.matches("copy").count(), 1, "{graph}");
            if opacity > 0.0 {
                assert!(graph.contains("overlay=x="), "{graph}");
                assert!(graph.contains(":format=auto[video_composed]"), "{graph}");
            } else {
                assert_eq!(
                    graph,
                    "[0:v:0]vflip[video_composed];[video_composed]copy[vout]"
                );
            }
            assert!(args_contains_pair(&args, "-map", "[vout]"));
        }
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

        assert!(args_contains_pair(&args, "-compression_algo", "8"));
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
