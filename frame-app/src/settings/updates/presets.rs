use crate::settings::model::{ConversionConfig, PresetDefinition, SourceMetadata};
use crate::settings::rules::container_supports_audio;
use crate::settings::updates::normalize::normalize_output_config;

/// Returns the first local track only when applying the preset would enable audio.
#[must_use]
pub fn preset_default_audio_track<'a>(
    target: &ConversionConfig,
    output_supports_audio: bool,
    enable_source_audio: bool,
    metadata: Option<&'a SourceMetadata>,
) -> Option<&'a u32> {
    if output_supports_audio
        && target.selected_audio_tracks.is_empty()
        && (enable_source_audio || !container_supports_audio(&target.container))
    {
        return metadata?.audio_tracks.first().map(|track| &track.index);
    }
    None
}

pub fn apply_preset(
    config: &mut ConversionConfig,
    preset: &PresetDefinition,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    let default_track = preset_default_audio_track(
        config,
        container_supports_audio(&preset.config.container),
        preset.enable_source_audio,
        metadata,
    );
    // Presets describe output settings; stream indices and subtitle paths belong to the source.
    let selected_audio_tracks = std::mem::take(&mut config.selected_audio_tracks);
    let audio_track_overrides = std::mem::take(&mut config.audio_track_overrides);
    let selected_subtitle_tracks = std::mem::take(&mut config.selected_subtitle_tracks);
    let external_subtitle_tracks = std::mem::take(&mut config.external_subtitle_tracks);
    let subtitle_burn_path = config.subtitle_burn_path.take();

    *config = preset.config.clone();
    config.selected_audio_tracks = selected_audio_tracks;
    config.audio_track_overrides = audio_track_overrides;
    config.selected_subtitle_tracks = selected_subtitle_tracks;
    config.external_subtitle_tracks = external_subtitle_tracks;
    config.subtitle_burn_path = subtitle_burn_path;
    if let Some(&index) = default_track {
        config.selected_audio_tracks.push(index);
    }
    normalize_output_config(config, metadata);

    before != *config
}
