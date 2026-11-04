use crate::settings::model::{
    ConversionConfig, IMAGE_JPEG_HUFFMAN_OPTIONS, IMAGE_PNG_PREDICTION_OPTIONS,
    IMAGE_TIFF_COMPRESSION_OPTIONS, IMAGE_WEBP_PRESET_OPTIONS, ImageEncodingOption,
};

#[must_use]
pub fn image_jpeg_huffman_options(
    config: &ConversionConfig,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    image_encoding_options(
        &IMAGE_JPEG_HUFFMAN_OPTIONS,
        &config.image_jpeg_huffman,
        disabled,
    )
}

#[must_use]
pub fn image_webp_preset_options(
    config: &ConversionConfig,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    image_encoding_options(
        &IMAGE_WEBP_PRESET_OPTIONS,
        &config.image_webp_preset,
        disabled,
    )
}

#[must_use]
pub fn image_png_prediction_options(
    config: &ConversionConfig,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    image_encoding_options(
        &IMAGE_PNG_PREDICTION_OPTIONS,
        &config.image_png_prediction,
        disabled,
    )
}

#[must_use]
pub fn image_tiff_compression_options(
    config: &ConversionConfig,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    image_encoding_options(
        &IMAGE_TIFF_COMPRESSION_OPTIONS,
        &config.image_tiff_compression,
        disabled,
    )
}

pub(super) fn image_encoding_options(
    definitions: &[crate::settings::model::ImageEncodingOptionDefinition],
    selected_id: &str,
    disabled: bool,
) -> Vec<ImageEncodingOption> {
    definitions
        .iter()
        .map(|definition| ImageEncodingOption {
            id: definition.id,
            label: definition.label,
            caption: definition.caption,
            is_selected: selected_id == definition.id,
            is_disabled: disabled,
        })
        .collect()
}
