//! Available inspector options derived from source metadata and conversion settings.

mod audio;
mod images;
mod metadata;
mod output;
mod presets;
mod subtitles;
mod video;

pub use audio::{
    audio_channel_options, audio_codec_options, audio_settings_error, audio_track_options,
    mp2_original_channels_are_unsupported, stereo_width_available,
};
pub use images::{
    image_jpeg_huffman_options, image_png_prediction_options, image_tiff_compression_options,
    image_webp_preset_options,
};
pub use metadata::{
    metadata_field_options, metadata_field_placeholder, metadata_field_value,
    metadata_fields_for_source, metadata_mode_description, metadata_mode_options,
};
pub use output::{
    is_container_compatible_for_stream_copy, output_container_options,
    output_processing_mode_options, visible_output_containers,
};
pub use presets::{
    configs_match, create_custom_preset, default_presets, preset_is_compatible, preset_options,
};
pub use subtitles::{
    normalized_hex_color, subtitle_burn_file_label, subtitle_color_value, subtitle_font_options,
    subtitle_font_size_options, subtitle_position, subtitle_position_options,
    subtitle_track_options,
};
pub use video::{
    first_allowed_video_codec, first_allowed_video_pixel_format, first_allowed_video_preset,
    fps_options, gif_color_options, gif_dither_options, is_hardware_video_codec,
    is_nvenc_video_codec, is_video_preset_allowed, is_videotoolbox_video_codec, resolution_options,
    scaling_algorithm_options, video_codec_options, video_pixel_format_options,
    video_preset_caption, video_preset_label, video_preset_options,
};
