//! Conversion-setting mutations and normalization, grouped by inspector domain.

mod audio;
mod images;
mod metadata;
mod normalize;
mod output;
mod presets;
mod subtitles;
mod validation;
mod video;

pub use audio::{
    apply_audio_bitrate, apply_audio_bitrate_mode, apply_audio_channels, apply_audio_codec,
    apply_audio_normalize, apply_audio_quality, apply_audio_volume, apply_opus_vbr,
    audio_codec_supports_vbr, audio_quality_range, toggle_audio_track_selection,
};
pub use images::{
    apply_image_jpeg_huffman, apply_image_jpeg_quality, apply_image_png_compression,
    apply_image_png_prediction, apply_image_tiff_compression, apply_image_webp_compression,
    apply_image_webp_lossless, apply_image_webp_preset, apply_image_webp_quality,
};
pub use metadata::{apply_metadata_field, apply_metadata_mode};
pub use normalize::{initialize_output_config, normalize_output_config, normalize_video_config};
pub use output::{
    apply_output_container, apply_processing_mode, apply_trim_times, sanitize_output_name,
};
pub use presets::apply_preset;
pub use subtitles::{
    add_external_subtitle_tracks, apply_external_subtitle_default, apply_external_subtitle_forced,
    apply_external_subtitle_language, apply_external_subtitle_title, apply_subtitle_burn_path,
    apply_subtitle_font_color, apply_subtitle_font_name, apply_subtitle_font_size,
    apply_subtitle_outline_color, apply_subtitle_position, remove_external_subtitle_track,
    toggle_subtitle_track_selection,
};
pub use video::{
    apply_crf, apply_custom_height, apply_custom_width, apply_fps, apply_gif_colors,
    apply_gif_dither, apply_gif_loop, apply_hw_decode, apply_nvenc_spatial_aq,
    apply_nvenc_temporal_aq, apply_pixel_format, apply_quality, apply_resolution,
    apply_scaling_algorithm, apply_video_bitrate, apply_video_bitrate_mode, apply_video_codec,
    apply_video_preset, apply_videotoolbox_allow_sw,
};
