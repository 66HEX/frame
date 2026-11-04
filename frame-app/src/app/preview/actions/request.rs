use crate::app::preview::actions::overlay::valid_preview_dimensions;
use crate::app::preview::panel::preview_duration_seconds;
use crate::app::preview::state::{
    PREVIEW_ADAPTIVE_SCALE, PREVIEW_DIMENSION_QUANTUM, PREVIEW_MIN_ADAPTIVE_HEIGHT,
    PREVIEW_MIN_ADAPTIVE_WIDTH, PreviewRuntimeDimensions, PreviewRuntimeKey, PreviewRuntimeRequest,
};
use crate::conversion_runner::core_config_from_gpui;
use crate::file_queue::FileItem;
use crate::numeric::rounded_f64_to_u64;
use crate::preview_engine::{
    DEFAULT_PREVIEW_FPS, DEFAULT_PREVIEW_MAX_HEIGHT, DEFAULT_PREVIEW_MAX_WIDTH,
    PreviewRenderPresentation, PreviewSessionConfig, PreviewSourceKind as EnginePreviewSourceKind,
};
use crate::settings::{
    AudioFiltersConfig, ConversionConfig, CropSettings, FilterValue, OverlaySettings, SourceKind,
    SourceMetadata, VideoFiltersConfig,
};
use crate::source_metadata::{MetadataStatus, SourceMetadataEntry};
use gpui::{Bounds, Pixels};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub(super) fn preview_runtime_request(
    selected_file: &FileItem,
    metadata_entry: &SourceMetadataEntry,
    include_applied_crop: bool,
    include_committed_overlay: bool,
    preview_dimensions: PreviewRuntimeDimensions,
) -> Option<PreviewRuntimeRequest> {
    if metadata_entry.status != MetadataStatus::Ready {
        return None;
    }

    let metadata = metadata_entry.metadata.as_ref()?;
    let source_kind = engine_source_kind(metadata);
    let duration_seconds = preview_duration_seconds(Some(metadata));
    let (source_width, source_height) = valid_preview_dimensions(metadata.width, metadata.height);
    let mut preview_config = selected_file.config.clone();
    if !include_applied_crop {
        preview_config.crop = None;
        preview_config.resolution = "original".to_string();
        preview_config.custom_width = None;
        preview_config.custom_height = None;
    }
    if !include_committed_overlay {
        preview_config.overlay = None;
    }
    let core_config = core_config_from_gpui(&preview_config);
    let presentation = PreviewRenderPresentation::default();
    let selected_audio_track = preview_selected_audio_track(&preview_config, metadata);
    let key = PreviewRuntimeKey {
        file_id: selected_file.id.clone(),
        path: selected_file.path.clone(),
        source_kind,
        source_width,
        source_height,
        duration_millis: rounded_f64_to_u64(duration_seconds * 1000.0),
        preview_dimensions,
        visual_hash: preview_visual_hash(&preview_config),
        audio_hash: preview_audio_hash(&preview_config, selected_audio_track),
    };
    let config = PreviewSessionConfig {
        file_id: key.file_id.clone(),
        path: PathBuf::from(&selected_file.path),
        source_kind,
        source_width,
        source_height,
        selected_audio_track,
        duration_seconds,
        max_width: preview_dimensions.max_width,
        max_height: preview_dimensions.max_height,
        fps: DEFAULT_PREVIEW_FPS,
        conversion_config: core_config,
    };

    Some(PreviewRuntimeRequest {
        key,
        config,
        presentation,
    })
}

pub(in crate::app) fn preview_runtime_dimensions(
    canvas_bounds: Option<Bounds<Pixels>>,
    target_zoom: f64,
) -> PreviewRuntimeDimensions {
    let Some(bounds) = canvas_bounds else {
        return default_preview_runtime_dimensions();
    };
    let canvas_width = f64::from(bounds.size.width.as_f32());
    let canvas_height = f64::from(bounds.size.height.as_f32());
    if canvas_width <= 0.0 || canvas_height <= 0.0 {
        return default_preview_runtime_dimensions();
    }

    let zoom = if target_zoom.is_finite() {
        target_zoom.max(1.0)
    } else {
        1.0
    };
    PreviewRuntimeDimensions {
        max_width: quantized_preview_dimension(canvas_width * zoom * PREVIEW_ADAPTIVE_SCALE)
            .clamp(PREVIEW_MIN_ADAPTIVE_WIDTH, DEFAULT_PREVIEW_MAX_WIDTH),
        max_height: quantized_preview_dimension(canvas_height * zoom * PREVIEW_ADAPTIVE_SCALE)
            .clamp(PREVIEW_MIN_ADAPTIVE_HEIGHT, DEFAULT_PREVIEW_MAX_HEIGHT),
    }
}

pub(super) const fn default_preview_runtime_dimensions() -> PreviewRuntimeDimensions {
    PreviewRuntimeDimensions {
        max_width: DEFAULT_PREVIEW_MAX_WIDTH,
        max_height: DEFAULT_PREVIEW_MAX_HEIGHT,
    }
}

pub(super) fn quantized_preview_dimension(value: f64) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    let quantum = f64::from(PREVIEW_DIMENSION_QUANTUM);
    let quantized = (value / quantum).ceil() * quantum;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "preview dimensions are finite and clamped before use"
    )]
    {
        quantized.min(f64::from(u32::MAX)) as u32
    }
}

pub(super) fn preview_visual_hash(config: &ConversionConfig) -> u64 {
    let mut state = DefaultHasher::new();
    config.rotation.hash(&mut state);
    config.flip_horizontal.hash(&mut state);
    config.flip_vertical.hash(&mut state);
    config.resolution.hash(&mut state);
    config.custom_width.hash(&mut state);
    config.custom_height.hash(&mut state);
    config.scaling_algorithm.hash(&mut state);
    config.fps.hash(&mut state);
    hash_crop(config.crop.as_ref(), &mut state);
    config.subtitle_burn_path.hash(&mut state);
    config.subtitle_font_name.hash(&mut state);
    config.subtitle_font_size.hash(&mut state);
    config.subtitle_font_color.hash(&mut state);
    config.subtitle_outline_color.hash(&mut state);
    config.subtitle_position.hash(&mut state);
    hash_overlay(config.overlay.as_ref(), &mut state);
    hash_video_filters(&config.video_filters, &mut state);
    config.gif_colors.hash(&mut state);
    config.gif_dither.hash(&mut state);
    state.finish()
}

pub(super) fn preview_audio_hash(
    config: &ConversionConfig,
    selected_audio_track: Option<u32>,
) -> u64 {
    let mut state = DefaultHasher::new();
    selected_audio_track.hash(&mut state);
    config.audio_volume.hash(&mut state);
    config.audio_normalize.hash(&mut state);
    hash_audio_filters(&config.audio_filters, &mut state);
    state.finish()
}

pub(super) const fn preview_runtime_hash_changed(
    current: &PreviewRuntimeKey,
    next: &PreviewRuntimeKey,
) -> bool {
    current.visual_hash != next.visual_hash || current.audio_hash != next.audio_hash
}

pub(super) fn hash_video_filters(filters: &VideoFiltersConfig, state: &mut DefaultHasher) {
    hash_filter_value(&filters.color.brightness, state);
    hash_filter_value(&filters.color.contrast, state);
    hash_filter_value(&filters.color.saturation, state);
    hash_filter_value(&filters.color.gamma, state);
    hash_filter_value(&filters.hue, state);
    hash_filter_value(&filters.temperature, state);
    hash_filter_value(&filters.sharpen, state);
    hash_filter_value(&filters.gaussian_blur, state);
    filters.denoise_enabled.hash(state);
    if filters.denoise_enabled {
        filters.denoise_strength.hash(state);
    }
    hash_filter_value(&filters.deband, state);
    hash_filter_value(&filters.vignette, state);
    filters.grayscale.hash(state);
    filters.deinterlace.hash(state);
}

pub(super) fn hash_audio_filters(filters: &AudioFiltersConfig, state: &mut DefaultHasher) {
    filters.compressor_enabled.hash(state);
    if filters.compressor_enabled {
        filters.compressor_strength.hash(state);
    }
    hash_filter_value(&filters.limiter, state);
    hash_filter_value(&filters.bass, state);
    hash_filter_value(&filters.treble, state);
    hash_filter_value(&filters.high_pass, state);
    hash_filter_value(&filters.low_pass, state);
    hash_filter_value(&filters.noise_reduction, state);
    hash_filter_value(&filters.de_esser, state);
    hash_filter_value(&filters.stereo_width, state);
}

pub(super) fn hash_filter_value<T: Hash>(filter: &FilterValue<T>, state: &mut DefaultHasher) {
    filter.enabled.hash(state);
    if filter.enabled {
        filter.value.hash(state);
    }
}

pub(super) fn hash_crop(crop: Option<&CropSettings>, state: &mut DefaultHasher) {
    let Some(crop) = crop else {
        false.hash(state);
        return;
    };

    crop.enabled.hash(state);
    crop.x.hash(state);
    crop.y.hash(state);
    crop.width.hash(state);
    crop.height.hash(state);
    crop.source_width.hash(state);
    crop.source_height.hash(state);
    crop.aspect_ratio.hash(state);
}

pub(super) fn hash_overlay(overlay: Option<&OverlaySettings>, state: &mut DefaultHasher) {
    let Some(overlay) = overlay else {
        false.hash(state);
        return;
    };

    overlay.enabled.hash(state);
    overlay.path.hash(state);
    overlay.x.to_bits().hash(state);
    overlay.y.to_bits().hash(state);
    overlay.width.to_bits().hash(state);
    overlay.opacity.to_bits().hash(state);
    overlay.anchor.hash(state);
}

pub(super) fn engine_source_kind(metadata: &SourceMetadata) -> EnginePreviewSourceKind {
    match metadata.source_kind() {
        SourceKind::Video => EnginePreviewSourceKind::Video,
        SourceKind::Audio => EnginePreviewSourceKind::Audio,
        SourceKind::Image => EnginePreviewSourceKind::Image,
    }
}

pub(super) fn preview_selected_audio_track(
    config: &ConversionConfig,
    metadata: &SourceMetadata,
) -> Option<u32> {
    config.selected_audio_tracks.iter().copied().find(|index| {
        metadata
            .audio_tracks
            .iter()
            .any(|track| track.index == *index)
    })
}
