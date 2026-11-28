use crate::settings::model::{ConversionConfig, PresetDefinition, SourceMetadata};
use crate::settings::rules::container_supports_audio;
use crate::settings::updates::normalize::normalize_output_config;

pub fn apply_preset(
    config: &mut ConversionConfig,
    preset: &PresetDefinition,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    let had_audio_container = container_supports_audio(&before.container);
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
    if !had_audio_container
        && container_supports_audio(&config.container)
        && config.selected_audio_tracks.is_empty()
        && let Some(first_track) = metadata.and_then(|metadata| metadata.audio_tracks.first())
    {
        config.selected_audio_tracks.push(first_track.index);
    }
    normalize_output_config(config, metadata);

    before != *config
}
