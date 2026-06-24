use super::{
    model::{
        ALL_SETTINGS_TABS, ConversionConfig, ProcessingMode, SettingsTab, SourceKind,
        SourceMetadata,
    },
    rules::*,
};

#[must_use]
pub fn visible_settings_tabs(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> Vec<SettingsTab> {
    let source_kind = source_kind_for(metadata);
    let is_source_audio_only = source_kind == SourceKind::Audio;
    let is_source_image = source_kind == SourceKind::Image;
    let is_copy_mode = config.processing_mode == ProcessingMode::Copy;
    let is_audio_container = is_audio_only_container(&config.container);
    let supports_audio = container_supports_audio(&config.container) && !is_source_image;
    let supports_subtitles = !is_source_audio_only
        && !is_source_image
        && container_supports_subtitles(&config.container);
    let supports_video_tab =
        !is_source_audio_only && !is_source_image && !is_audio_container && !is_copy_mode;
    let supports_images_tab = is_source_image && !is_audio_container && !is_copy_mode;

    ALL_SETTINGS_TABS
        .into_iter()
        .filter(|tab| match tab {
            SettingsTab::Video => supports_video_tab,
            SettingsTab::Images => supports_images_tab,
            SettingsTab::Audio => supports_audio,
            SettingsTab::Subtitles => supports_subtitles,
            SettingsTab::Source
            | SettingsTab::Output
            | SettingsTab::Metadata
            | SettingsTab::Presets => true,
        })
        .collect()
}

#[must_use]
pub fn resolve_active_settings_tab(
    active_tab: SettingsTab,
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> SettingsTab {
    if visible_settings_tabs(config, metadata).contains(&active_tab) {
        active_tab
    } else {
        SettingsTab::Output
    }
}
