use crate::settings::model::{
    ConversionConfig, IMAGE_JPEG_HUFFMAN_OPTIONS, IMAGE_PNG_PREDICTION_OPTIONS,
    IMAGE_TIFF_COMPRESSION_OPTIONS, IMAGE_WEBP_PRESET_OPTIONS, MAX_IMAGE_JPEG_QUALITY,
    MAX_IMAGE_PNG_COMPRESSION, MAX_IMAGE_WEBP_COMPRESSION, MAX_IMAGE_WEBP_QUALITY,
};

pub fn apply_image_jpeg_quality(config: &mut ConversionConfig, quality: u32) -> bool {
    let quality = quality.clamp(1, MAX_IMAGE_JPEG_QUALITY);
    if config.image_jpeg_quality == quality {
        return false;
    }

    config.image_jpeg_quality = quality;
    true
}

pub fn apply_image_jpeg_huffman(config: &mut ConversionConfig, huffman: &str) -> bool {
    let huffman = huffman.to_ascii_lowercase();
    if !image_option_is_known(&IMAGE_JPEG_HUFFMAN_OPTIONS, &huffman) {
        return false;
    }

    if config.image_jpeg_huffman == huffman {
        return false;
    }

    config.image_jpeg_huffman = huffman;
    true
}

pub const fn apply_image_webp_lossless(config: &mut ConversionConfig, enabled: bool) -> bool {
    if config.image_webp_lossless == enabled {
        return false;
    }

    config.image_webp_lossless = enabled;
    true
}

pub fn apply_image_webp_quality(config: &mut ConversionConfig, quality: u32) -> bool {
    let quality = quality.min(MAX_IMAGE_WEBP_QUALITY);
    if config.image_webp_quality == quality {
        return false;
    }

    config.image_webp_quality = quality;
    true
}

pub fn apply_image_webp_compression(config: &mut ConversionConfig, compression: u32) -> bool {
    let compression = compression.min(MAX_IMAGE_WEBP_COMPRESSION);
    if config.image_webp_compression == compression {
        return false;
    }

    config.image_webp_compression = compression;
    true
}

pub fn apply_image_webp_preset(config: &mut ConversionConfig, preset: &str) -> bool {
    let preset = preset.to_ascii_lowercase();
    if !image_option_is_known(&IMAGE_WEBP_PRESET_OPTIONS, &preset) {
        return false;
    }

    if config.image_webp_preset == preset {
        return false;
    }

    config.image_webp_preset = preset;
    true
}

pub fn apply_image_png_compression(config: &mut ConversionConfig, compression: u32) -> bool {
    let compression = compression.min(MAX_IMAGE_PNG_COMPRESSION);
    if config.image_png_compression == compression {
        return false;
    }

    config.image_png_compression = compression;
    true
}

pub fn apply_image_png_prediction(config: &mut ConversionConfig, prediction: &str) -> bool {
    let prediction = prediction.to_ascii_lowercase();
    if !image_option_is_known(&IMAGE_PNG_PREDICTION_OPTIONS, &prediction) {
        return false;
    }

    if config.image_png_prediction == prediction {
        return false;
    }

    config.image_png_prediction = prediction;
    true
}

pub fn apply_image_tiff_compression(config: &mut ConversionConfig, compression: &str) -> bool {
    let compression = compression.to_ascii_lowercase();
    if !image_option_is_known(&IMAGE_TIFF_COMPRESSION_OPTIONS, &compression) {
        return false;
    }

    if config.image_tiff_compression == compression {
        return false;
    }

    config.image_tiff_compression = compression;
    true
}

pub(super) fn image_option_is_known(
    definitions: &[crate::settings::model::ImageEncodingOptionDefinition],
    value: &str,
) -> bool {
    definitions
        .iter()
        .any(|definition| definition.id.eq_ignore_ascii_case(value))
}
