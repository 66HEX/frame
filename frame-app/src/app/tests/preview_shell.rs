use super::*;
use crate::app::preview::PreviewShellStateInput;

fn empty_encoders() -> &'static AvailableEncoders {
    static ENCODERS: AvailableEncoders = AvailableEncoders {
        mpeg2video: false,
        mp2: false,
        dvbsub: false,
        pcm_bluray: false,
        h264_videotoolbox: false,
        h264_nvenc: false,
        hevc_videotoolbox: false,
        hevc_nvenc: false,
        av1_nvenc: false,
        libfdk_aac: false,
        libmp3lame: false,
    };
    &ENCODERS
}

fn empty_filters() -> &'static AvailableFilters {
    static FILTERS: AvailableFilters = AvailableFilters {
        eq: false,
        hue: false,
        colortemperature: false,
        unsharp: false,
        gblur: false,
        hqdn3d: false,
        deband: false,
        vignette: false,
        bwdif: false,
        highpass: false,
        lowpass: false,
        afftdn: false,
        deesser: false,
        bass: false,
        treble: false,
        acompressor: false,
        loudnorm: false,
        volume: false,
        stereotools: false,
        alimiter: false,
    };
    &FILTERS
}

fn settings_state<'a>(
    config: &'a ConversionConfig,
    metadata: Option<&'a SourceMetadata>,
    status: MetadataStatus,
) -> SettingsRenderState<'a> {
    static CLOSED: std::collections::BTreeSet<crate::settings::OutputSection> =
        std::collections::BTreeSet::new();
    let subtitle_font_select_scroll_handle = Box::leak(Box::new(ScrollHandle::new()));
    let subtitle_font_size_select_scroll_handle = Box::leak(Box::new(ScrollHandle::new()));

    SettingsRenderState {
        apply_all: crate::app::inspector::apply_all::ApplyAllAvailability::Hidden,
        crop_editing: false,
        palette: theme::palette(crate::appearance::ColorTheme::Dark),
        active_tab: SettingsTab::Metadata,
        expanded_output_sections: &CLOSED,
        config,
        metadata,
        metadata_status: status,
        metadata_error: None,
        settings_disabled: false,
        output_name: "",
        output_name_focus: None,
        audio_bitrate_focus: None,
        video_width_focus: None,
        video_height_focus: None,
        video_bitrate_focus: None,
        gif_loop_focus: None,
        metadata_focuses: SettingsMetadataInputFocuses {
            title: None,
            artist: None,
            album: None,
            genre: None,
            date: None,
            comment: None,
            service_name: None,
            service_provider: None,
        },
        subtitle_focuses: SettingsSubtitleFocuses::default(),
        external_subtitle_language_focus: None,
        external_subtitle_title_focus: None,
        external_subtitle_track_index: None,
        subtitle_mode: SettingsSubtitleMode::Selectable,
        subtitle_color_focuses: SettingsSubtitleColorInputFocuses {
            font: None,
            outline: None,
        },
        subtitle_popover: None,
        subtitle_rendered_popover: None,
        subtitle_font_select_scroll_handle,
        subtitle_font_size_select_scroll_handle,
        subtitle_font_color_draft: "",
        subtitle_outline_color_draft: "",
        subtitle_font_color_hsv_draft: hex_to_subtitle_hsv(DEFAULT_SUBTITLE_FONT_COLOR),
        subtitle_outline_color_hsv_draft: hex_to_subtitle_hsv(DEFAULT_SUBTITLE_OUTLINE_COLOR),
        preset_name: "",
        preset_name_focus: None,
        presets: &[],
        subtitle_fonts: &[],
        available_encoders: empty_encoders(),
        available_filters: empty_filters(),
    }
}

fn crop_state() -> PreviewCropRenderState {
    PreviewCropRenderState {
        crop_mode: false,
        draft_crop: None,
        applied_crop: None,
        crop_aspect: "free".to_string(),
        has_crop_dimensions: false,
        rotation: "0".to_string(),
        flip_horizontal: false,
        flip_vertical: false,
    }
}

#[test]
fn ready_video_metadata_populates_timeline_labels() {
    let config = ConversionConfig::default();
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        duration: Some("90.4".to_string()),
        ..SourceMetadata::default()
    };
    let file = FileItem::from_path("video", "/tmp/render.mov", 1024);
    let settings = settings_state(&config, Some(&metadata), MetadataStatus::Ready);

    let state = preview_shell_state(PreviewShellStateInput {
        selected_file: Some(&file),
        settings: &settings,
        crop: crop_state(),
        overlay: PreviewOverlayRenderState::empty(),
        canvas: PreviewCanvasRenderState::default(),
        playback: preview_playback_state(PreviewMediaKind::Video, 90.4, None, None),
        presentation: PreviewRenderPresentation::default(),
        render_image: None,
        runtime_error: None,
    });
    let labels = preview_timeline_labels(&state);

    assert_eq!(state.availability.media_kind, PreviewMediaKind::Video);
    assert!(preview_trim_enabled(&state));
    assert_eq!(labels.start, "00:00:00.000");
    assert_eq!(labels.end, "00:01:30.400");
    assert_eq!(labels.duration, "00:01:30.400");
}

#[test]
fn ready_video_metadata_uses_configured_trim_bounds() {
    let config = ConversionConfig {
        start_time: Some("00:00:05.000".to_string()),
        end_time: Some("00:00:30.250".to_string()),
        ..ConversionConfig::default()
    };
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        duration: Some("90.4".to_string()),
        ..SourceMetadata::default()
    };
    let file = FileItem::from_path("video", "/tmp/render.mov", 1024);
    let settings = settings_state(&config, Some(&metadata), MetadataStatus::Ready);

    let state = preview_shell_state(PreviewShellStateInput {
        selected_file: Some(&file),
        settings: &settings,
        crop: crop_state(),
        overlay: PreviewOverlayRenderState::empty(),
        canvas: PreviewCanvasRenderState::default(),
        playback: preview_playback_state(
            PreviewMediaKind::Video,
            90.4,
            config.start_time.as_deref(),
            config.end_time.as_deref(),
        ),
        presentation: PreviewRenderPresentation::default(),
        render_image: None,
        runtime_error: None,
    });
    let labels = preview_timeline_labels(&state);

    assert_eq!(labels.start, "00:00:05.000");
    assert_eq!(labels.end, "00:00:30.250");
    assert_eq!(labels.duration, "00:00:25.250");
}

#[test]
fn image_metadata_uses_placeholder_timeline_labels() {
    let config = ConversionConfig::default();
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Image),
        duration: Some("10.0".to_string()),
        ..SourceMetadata::default()
    };
    let file = FileItem::from_path("image", "/tmp/still.png", 1024);
    let settings = settings_state(&config, Some(&metadata), MetadataStatus::Ready);

    let state = preview_shell_state(PreviewShellStateInput {
        selected_file: Some(&file),
        settings: &settings,
        crop: crop_state(),
        overlay: PreviewOverlayRenderState::empty(),
        canvas: PreviewCanvasRenderState::default(),
        playback: PreviewPlaybackState::new(false),
        presentation: PreviewRenderPresentation::default(),
        render_image: None,
        runtime_error: None,
    });
    let labels = preview_timeline_labels(&state);

    assert_eq!(state.availability.media_kind, PreviewMediaKind::Image);
    assert!(state.availability.trim_disabled);
    assert_eq!(labels.start, "--:--:--.---");
    assert_eq!(labels.end, "--:--:--.---");
    assert_eq!(labels.duration, "--:--:--.---");
}

#[test]
fn audio_metadata_hides_visual_controls() {
    let config = ConversionConfig::default();
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Audio),
        duration: Some("00:00:12.500".to_string()),
        ..SourceMetadata::default()
    };
    let settings = settings_state(&config, Some(&metadata), MetadataStatus::Ready);

    let state = preview_shell_state(PreviewShellStateInput {
        selected_file: None,
        settings: &settings,
        crop: crop_state(),
        overlay: PreviewOverlayRenderState::empty(),
        canvas: PreviewCanvasRenderState::default(),
        playback: PreviewPlaybackState::new(false),
        presentation: PreviewRenderPresentation::default(),
        render_image: None,
        runtime_error: None,
    });

    assert_eq!(state.availability.media_kind, PreviewMediaKind::Audio);
    assert!(state.availability.hide_visual_controls);
    assert!(!preview_visual_controls_visible(&state));
    assert_eq!(preview_duration_seconds(Some(&metadata)), 12.5);
}

#[test]
fn loading_metadata_keeps_preview_unknown() {
    let config = ConversionConfig::default();
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        duration: Some("90.0".to_string()),
        ..SourceMetadata::default()
    };
    let settings = settings_state(&config, Some(&metadata), MetadataStatus::Loading);

    let state = preview_shell_state(PreviewShellStateInput {
        selected_file: None,
        settings: &settings,
        crop: crop_state(),
        overlay: PreviewOverlayRenderState::empty(),
        canvas: PreviewCanvasRenderState::default(),
        playback: PreviewPlaybackState::new(false),
        presentation: PreviewRenderPresentation::default(),
        render_image: None,
        runtime_error: None,
    });

    assert_eq!(state.availability.media_kind, PreviewMediaKind::Unknown);
    assert!(state.availability.trim_disabled);
}

#[test]
fn centered_offset_never_returns_negative_values() {
    assert_eq!(centered_offset(30.0, 6.0), 12.0);
    assert_eq!(centered_offset(6.0, 30.0), 0.0);
}

#[test]
fn timeline_fraction_from_percent_clamps_to_track_range() {
    assert_eq!(timeline_fraction_from_percent(-25.0), 0.0);
    assert_eq!(timeline_fraction_from_percent(50.0), 0.5);
    assert_eq!(timeline_fraction_from_percent(125.0), 1.0);
}

#[test]
fn timeline_slider_percent_from_bounds_clamps_pointer_to_track() {
    let bounds = Bounds {
        origin: point(px(10.0), px(0.0)),
        size: size(px(100.0), px(30.0)),
    };

    assert_eq!(
        timeline_slider_percent_from_bounds(point(px(60.0), px(0.0)), bounds),
        0.5
    );
    assert_eq!(
        timeline_slider_percent_from_bounds(point(px(-10.0), px(0.0)), bounds),
        0.0
    );
    assert_eq!(
        timeline_slider_percent_from_bounds(point(px(140.0), px(0.0)), bounds),
        1.0
    );
}

#[test]
fn preview_canvas_layout_metrics_preserve_media_aspect_when_zooming() {
    let metrics = preview_canvas_layout_metrics(1000.0, 500.0, 1920.0, 1080.0, 1.18, 0.0, 0.0)
        .expect("metrics");

    assert!(((metrics.width / metrics.height) - (16.0 / 9.0)).abs() < 0.000_001);
    assert!(metrics.width > 1000.0);
    assert!((metrics.top - -45.0).abs() < 0.000_001);
}

#[test]
fn preview_canvas_pan_limits_allow_original_overscroll_window() {
    let (max_x, max_y) =
        preview_canvas_pan_limits(1000.0, 500.0, 1920.0, 1080.0, 0.25).expect("limits");

    assert_eq!(max_x, 1000.0);
    assert_eq!(max_y, 500.0);
}

#[test]
fn preview_canvas_initial_zoom_starts_inset_from_object_contain() {
    let zoom = preview_canvas_initial_zoom(1000.0, 500.0, 1920.0, 1080.0).expect("initial zoom");

    assert!((zoom - 0.9).abs() < 0.000_001);

    let metrics = preview_canvas_layout_metrics(1000.0, 500.0, 1920.0, 1080.0, zoom, 0.0, 0.0)
        .expect("metrics");
    assert!(metrics.width < 1000.0);
    assert!(metrics.height < 500.0);
    assert!(metrics.left > 0.0);
    assert!(metrics.top > 0.0);
}

#[test]
fn subtitle_hsv_helpers_round_trip_primary_colors() {
    assert_eq!(subtitle_hsv_to_hex(0.0, 1.0, 1.0), "#ff0000");
    assert_eq!(subtitle_hsv_to_hex(120.0, 1.0, 1.0), "#00ff00");

    let hsv = hex_to_subtitle_hsv("#00f");
    assert_eq!(hsv.h, 240.0);
    assert_eq!(hsv.s, 1.0);
    assert_eq!(hsv.v, 1.0);
}
