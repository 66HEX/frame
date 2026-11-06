use crate::settings::model::{
    ALL_OUTPUT_SECTIONS, ConversionConfig, OutputSection, ProcessingMode, SourceKind,
    SourceMetadata,
};
use crate::settings::rules::{
    container_supports_audio, container_supports_subtitles, is_audio_only_container,
    source_kind_for,
};

#[must_use]
pub fn visible_output_sections(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> Vec<OutputSection> {
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
    let supports_video_filters_tab = !is_source_audio_only && !is_audio_container && !is_copy_mode;
    let supports_images_tab = is_source_image && !is_audio_container && !is_copy_mode;
    let supports_audio_filters_tab = supports_audio && !is_copy_mode;

    ALL_OUTPUT_SECTIONS
        .into_iter()
        .filter(|tab| match tab {
            OutputSection::Video => supports_video_tab,
            OutputSection::VideoFilters => supports_video_filters_tab,
            OutputSection::Images => supports_images_tab,
            OutputSection::Audio => supports_audio,
            OutputSection::AudioFilters => supports_audio_filters_tab,
            OutputSection::Subtitles => supports_subtitles,
            OutputSection::General | OutputSection::Metadata => true,
        })
        .collect()
}
