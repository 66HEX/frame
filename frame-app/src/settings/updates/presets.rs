use crate::settings::model::{ConversionConfig, PresetDefinition, SourceMetadata};
use crate::settings::updates::normalize::normalize_output_config;

pub fn apply_preset(
    config: &mut ConversionConfig,
    preset: &PresetDefinition,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    // Presets describe output settings; stream indices and subtitle paths belong to the source.
    let selected_audio_tracks = std::mem::take(&mut config.selected_audio_tracks);
    let selected_subtitle_tracks = std::mem::take(&mut config.selected_subtitle_tracks);
    let external_subtitle_tracks = std::mem::take(&mut config.external_subtitle_tracks);
    let subtitle_burn_path = config.subtitle_burn_path.take();

    *config = preset.config.clone();
    config.selected_audio_tracks = selected_audio_tracks;
    config.selected_subtitle_tracks = selected_subtitle_tracks;
    config.external_subtitle_tracks = external_subtitle_tracks;
    config.subtitle_burn_path = subtitle_burn_path;
    normalize_output_config(config, metadata);

    before != *config
}
