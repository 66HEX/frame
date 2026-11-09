use super::*;

#[test]
fn app_settings_fixture_opens_runtime_settings_drawer() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::AppSettings));

    assert!(root.settings_ui.is_open);
    assert_eq!(
        root.settings_ui.max_concurrency_draft,
        root.max_concurrency.to_string()
    );
}

#[test]
fn ui_scale_fixture_opens_ui_select_at_maximum_scale() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::AppSettingsUiOpen));

    assert!(root.settings_ui.is_open);
    assert_eq!(root.appearance.ui_scale, ScalePreset::Percent200);
    assert!(root.settings_ui.ui_scale_popover.is_open());
}

#[test]
fn theme_fixture_opens_theme_select() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::AppSettingsThemeOpen));

    assert!(root.settings_ui.is_open);
    assert!(root.settings_ui.theme_popover.is_open());
    assert!(!root.settings_ui.ui_scale_popover.is_rendered());
}

#[test]
fn update_available_fixture_opens_update_dialog() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::UpdateAvailable));

    assert!(root.update_ui.dialog_open);
    assert!(root.update_ui.dialog_present);
    assert!(matches!(root.update_ui.status, UpdateStatus::Available(_)));
    assert!(
        root.update_ui
            .dialog_info
            .as_ref()
            .and_then(|info| info.release_notes_markdown.as_deref())
            .is_some_and(|notes| notes.contains("Native GPUI Application"))
    );
}

#[test]
fn preview_ready_fixture_seeds_selected_video_metadata() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::PreviewReady));

    assert_eq!(root.active_view, ActiveView::Workspace);
    assert_eq!(
        root.file_queue
            .selected_file()
            .map(|file| file.name.as_str()),
        Some("source_render.mov")
    );
    assert_eq!(
        root.selected_source_metadata()
            .map(|metadata| metadata.source_kind()),
        Some(SourceKind::Video)
    );
}

#[test]
fn preview_crop_fixture_enters_crop_mode() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::PreviewCrop));

    assert!(root.preview_ui.crop_mode);
    assert!(root.preview_ui.draft_crop.is_some());
    assert_eq!(root.preview_ui.crop_aspect, "1:1");
}

#[test]
fn crop_options_fixture_opens_ratio_list() {
    let mut root = FrameRoot::new();
    root.apply_visual_fixture(Some(VisualFixture::PreviewCropOptions));
    assert!(root.preview_ui.crop_options.popover.is_some());
    assert!(!root.preview_ui.crop_options.custom);
    assert_eq!(root.custom_crop_values(), Some((1, 1)));
    root.apply_visual_fixture(Some(VisualFixture::PreviewCropCustom));
    assert!(root.preview_ui.crop_options.custom);
    assert!(!root.preview_ui.crop_options.is_ratio);
}

#[test]
fn av1_fixture_selects_preset_four_and_ten_bit_pixels() {
    let mut root = FrameRoot::new();
    root.apply_visual_fixture(Some(VisualFixture::SettingsVideoAv1));
    let config = root.selected_config().unwrap();
    assert_eq!(config.video_codec, "libsvtav1");
    assert_eq!(config.preset, "4");
    assert_eq!(config.pixel_format, "yuv420p10le");
}

#[test]
fn codec_fixtures_seed_prores_and_native_vp9_controls() {
    let mut root = FrameRoot::new();
    root.apply_visual_fixture(Some(VisualFixture::SettingsVideoProres));
    let config = root.selected_config().unwrap();
    assert_eq!(config.container, "mov");
    assert_eq!(config.video_codec, "prores");
    assert_eq!(config.pixel_format, "yuv422p10le");
    assert_eq!(config.video_bitrate_mode, "bitrate");
    assert_eq!(config.preset, "slow");

    root.apply_visual_fixture(Some(VisualFixture::SettingsVideoVp9));
    let config = root.selected_config().unwrap();
    assert_eq!(config.container, "webm");
    assert_eq!(config.video_codec, "vp9");
    assert_eq!(config.audio_codec, "libopus");
    assert_eq!(config.preset, "3");
    assert_eq!(config.crf, 0);
}

#[test]
fn workspace_empty_fixture_clears_workspace_state() {
    let mut root = FrameRoot::new();
    root.apply_visual_fixture(Some(VisualFixture::PreviewReady));

    root.apply_visual_fixture(Some(VisualFixture::WorkspaceEmpty));

    assert_eq!(root.active_view, ActiveView::Workspace);
    assert!(root.file_queue.files().is_empty());
    assert_eq!(
        root.selected_source_metadata_entry().status,
        MetadataStatus::Idle
    );
}

#[test]
fn hardware_fixture_keeps_an_unavailable_selection_and_an_explicit_working_fallback() {
    use crate::capabilities::{HardwareEncoder, HardwareProbePolicy};
    let mut root = FrameRoot::new();
    root.apply_visual_fixture(Some(VisualFixture::SettingsVideoHardware));
    assert_eq!(root.selected_config().unwrap().video_codec, "h264_nvenc");
    for (encoder, fallback, available) in [
        (HardwareEncoder::H264Nvenc, false, false),
        (HardwareEncoder::H264Videotoolbox, false, false),
        (HardwareEncoder::H264Videotoolbox, true, true),
    ] {
        assert_eq!(
            root.hardware_capabilities
                .report(HardwareProbePolicy::new(encoder, fallback))
                .unwrap()
                .available(),
            available
        );
    }
}

#[test]
fn workspace_audio_fixture_seeds_selected_audio_source() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::WorkspaceAudio));

    assert_eq!(root.active_view, ActiveView::Workspace);
    assert_eq!(
        root.file_queue
            .selected_file()
            .map(|file| file.name.as_str()),
        Some("source_mix.wav")
    );
    assert_eq!(
        root.selected_source_metadata()
            .map(|metadata| metadata.source_kind()),
        Some(SourceKind::Audio)
    );
}

#[test]
fn workspace_image_fixture_seeds_selected_image_source() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::WorkspaceImage));

    assert_eq!(root.active_view, ActiveView::Workspace);
    assert_eq!(
        root.selected_source_metadata()
            .map(|metadata| metadata.source_kind()),
        Some(SourceKind::Image)
    );
}

#[test]
fn workspace_large_queue_fixture_seeds_many_files_for_virtualization_checks() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::WorkspaceLargeQueue));

    assert_eq!(root.file_queue.files().len(), 500);
}

#[test]
fn settings_source_fixture_opens_source_tab_with_ready_metadata() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsSource));

    let metadata = root
        .selected_source_metadata()
        .expect("source fixture should seed ready metadata");
    assert_eq!(root.settings_ui.active_tab, SettingsTab::Metadata);
    assert_eq!(
        source_info_sections(&metadata)
            .iter()
            .map(|section| match section {
                SourceInfoSection::Rows { title, .. } | SourceInfoSection::Tracks { title, .. } =>
                    *title,
            })
            .collect::<Vec<_>>(),
        ["File information", "Video stream"]
    );
}

#[test]
fn settings_output_fixture_opens_output_tab_with_output_name() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsOutput));

    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert_eq!(
        root.file_queue
            .selected_file()
            .map(|file| file.output_name.as_str()),
        Some("source_render_review.mov")
    );
}

#[test]
fn settings_audio_fixture_opens_audio_tab_with_tracks_and_controls() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsAudio));

    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert!(
        root.settings_ui
            .expanded_output_sections
            .contains(&crate::settings::OutputSection::Audio)
    );
    assert_eq!(
        root.selected_source_metadata()
            .map(|metadata| metadata.audio_tracks.len()),
        Some(2)
    );
    assert_eq!(
        root.file_queue
            .selected_file()
            .map(|file| (file.config.audio_codec.as_str(), file.config.audio_volume)),
        Some(("mp3", 145))
    );
}

#[test]
fn settings_audio_filters_fixture_opens_audio_filters_tab_with_enabled_filters() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsAudioFilters));

    let selected = root
        .file_queue
        .selected_file()
        .expect("audio filters fixture should select a file");
    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert!(
        root.settings_ui
            .expanded_output_sections
            .contains(&crate::settings::OutputSection::AudioFilters)
    );
    assert!(selected.config.audio_normalize);
    assert_eq!(selected.config.audio_volume, 132);
    assert!(selected.config.audio_filters.compressor_enabled);
    assert!(selected.config.audio_filters.limiter.enabled);
    assert!(selected.config.audio_filters.high_pass.enabled);
    assert!(selected.config.audio_filters.low_pass.enabled);
    assert!(selected.config.audio_filters.stereo_width.enabled);
}

#[test]
fn settings_metadata_fixture_opens_metadata_tab_with_source_tags() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsMetadata));

    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert_eq!(
        root.selected_source_metadata()
            .and_then(|metadata| metadata.tags)
            .and_then(|tags| tags.title),
        Some("Original Scene 24A".to_string())
    );
}

#[test]
fn settings_video_fixture_opens_video_tab() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsVideo));

    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert!(
        root.settings_ui
            .expanded_output_sections
            .contains(&crate::settings::OutputSection::Video)
    );
    assert_eq!(
        root.file_queue
            .selected_file()
            .and_then(|file| file.config.custom_width.as_deref()),
        Some("1920")
    );
}

#[test]
fn settings_video_filters_fixture_opens_video_filters_tab_with_enabled_filters() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsVideoFilters));

    let selected = root
        .file_queue
        .selected_file()
        .expect("video filters fixture should select a file");
    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert!(
        root.settings_ui
            .expanded_output_sections
            .contains(&crate::settings::OutputSection::VideoFilters)
    );
    assert!(selected.config.video_filters.color.brightness.enabled);
    assert!(selected.config.video_filters.color.contrast.enabled);
    assert!(selected.config.video_filters.temperature.enabled);
    assert!(selected.config.video_filters.sharpen.enabled);
    assert!(selected.config.video_filters.denoise_enabled);
    assert_eq!(
        selected.config.video_filters.deinterlace,
        crate::settings::DeinterlaceMode::Auto
    );
}

#[test]
fn settings_images_fixture_opens_images_tab_for_image_source() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsImages));

    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert!(
        root.settings_ui
            .expanded_output_sections
            .contains(&crate::settings::OutputSection::Images)
    );
    assert_eq!(
        root.selected_source_metadata()
            .map(|metadata| metadata.source_kind()),
        Some(SourceKind::Image)
    );
}

#[test]
fn settings_subtitles_fixture_opens_subtitles_tab_with_tracks() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsSubtitles));

    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert!(
        root.settings_ui
            .expanded_output_sections
            .contains(&crate::settings::OutputSection::Subtitles)
    );
    assert_eq!(
        root.selected_source_metadata()
            .map(|metadata| metadata.subtitle_tracks.len()),
        Some(2)
    );
}

#[test]
fn settings_subtitles_popover_fixture_opens_font_color_picker() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsSubtitlesPopover));

    assert_eq!(root.settings_ui.active_tab, SettingsTab::Output);
    assert!(
        root.settings_ui
            .expanded_output_sections
            .contains(&crate::settings::OutputSection::Subtitles)
    );
    assert_eq!(
        root.subtitle_ui.popover,
        Some(SettingsSubtitlePopover::FontColor)
    );
    assert_eq!(root.subtitle_ui.font_color_draft, "#FFD166");
}

#[test]
fn settings_presets_fixture_opens_presets_tab_with_custom_draft() {
    let mut root = FrameRoot::new();

    root.apply_visual_fixture(Some(VisualFixture::SettingsPresets));

    assert_eq!(root.settings_ui.active_tab, SettingsTab::Presets);
    assert_eq!(root.settings_ui.preset_name_draft, "Client Review MP4");
    assert!(
        root.presets
            .iter()
            .any(|preset| preset.id == "custom-review")
    );
}

#[test]
fn anamorphic_fixtures_seed_real_pixels_and_an_explicit_file_scoped_static_frame() {
    for (key, fixture) in [
        (
            "preview-anamorphic-crop",
            VisualFixture::PreviewAnamorphicCrop,
        ),
        (
            "preview-anamorphic-overlay",
            VisualFixture::PreviewAnamorphicOverlay,
        ),
    ] {
        assert_eq!(
            crate::app::model::visual_fixture_from_env_value(Some(key)),
            Some(fixture)
        );
        let mut root = FrameRoot::new();
        root.apply_visual_fixture(Some(fixture));
        assert_eq!(
            root.preview_ui.static_fixture_file_id.as_deref(),
            Some("fixture-preview")
        );
        assert_eq!(root.preview_ui.accepted_request_token, 0);
        let image = root.preview_ui.render_image.as_ref().unwrap();
        assert_eq!((image.size(0).width.0, image.size(0).height.0), (320, 180));
        assert!(image.as_bytes(0).unwrap().iter().any(|byte| *byte != 0));
        assert_eq!(
            root.preview_ui
                .rendered_presentation
                .pixel_aspect_ratio
                .exact_ratio(),
            Some((2, 1))
        );
        assert_eq!(
            root.selected_source_metadata().unwrap().sample_aspect_ratio,
            frame_core::aspect::SampleAspectRatio::new(2, 1)
        );
        assert_eq!(
            root.preview_ui.crop_mode,
            fixture == VisualFixture::PreviewAnamorphicCrop
        );
        assert_eq!(
            root.preview_ui.overlay.overlay_mode(),
            fixture == VisualFixture::PreviewAnamorphicOverlay
        );
    }
}

#[test]
fn native_anamorphic_fixture_bounds_keep_crop_and_overlay_in_raster_coordinates() {
    use gpui::{TestAppContext, VisualTestContext};
    fn draw(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.simulate_next_frame(cx);
                window.draw(cx).clear(cx);
            });
        }
    }
    for (fixture, selection, width_fraction, height_fraction) in [
        (
            VisualFixture::PreviewAnamorphicCrop,
            "preview-crop-move-handle",
            128.0 / 320.0,
            128.0 / 180.0,
        ),
        (
            VisualFixture::PreviewAnamorphicOverlay,
            "preview-overlay-layer",
            80.0 / 320.0,
            80.0 / 180.0,
        ),
    ] {
        for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
            let mut app = TestAppContext::single();
            if fixture == VisualFixture::PreviewAnamorphicOverlay {
                preload_overlay_test_image(
                    &app,
                    std::path::Path::new(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/resources/app-icons/64x64.png"
                    )),
                    (64, 64),
                );
            }
            let (root, cx) = app.add_window_view(|_, cx| {
                crate::app::init_app(cx, "Anamorphic preview test");
                cx.set_reduce_motion(true);
                let mut root = FrameRoot::new();
                root.apply_visual_fixture(Some(fixture));
                root.appearance.ui_scale = scale;
                root
            });
            cx.simulate_resize(size(px(1800.0), px(1200.0)));
            draw(cx);
            let stage = cx.debug_bounds("preview-media-stage").unwrap();
            let selected = cx.debug_bounds(selection).unwrap();
            let width = f64::from(stage.size.width.as_f32());
            let height = f64::from(stage.size.height.as_f32());
            assert!((width / height - 32.0 / 9.0).abs() < 0.001, "{stage:?}");
            assert!(
                width
                    .mul_add(-width_fraction, f64::from(selected.size.width.as_f32()))
                    .abs()
                    < 1.0
            );
            assert!(
                height
                    .mul_add(-height_fraction, f64::from(selected.size.height.as_f32()))
                    .abs()
                    < 1.0
            );
            assert!(
                (selected.size.width.as_f32() / selected.size.height.as_f32() - 2.0).abs() < 0.01
            );
            if fixture == VisualFixture::PreviewAnamorphicOverlay {
                // Check the loaded image itself, not merely its clipping parent:
                // an intrinsic square previously lost its entire lower half.
                let image = cx.debug_bounds("preview-overlay-image").unwrap();
                assert_eq!(image, selected);
                let painted_images = cx.update(|window, _| window.rendered_primitive_counts().3);
                assert!(painted_images >= 2, "preview and overlay must both paint");
            }
            root.read_with(cx, |root, _| {
                assert!(root.preview_edit_frame_ready());
                assert!(root.preview_ui.session.is_none());
            });
        }
    }
}

fn preload_overlay_test_image(
    app: &gpui::TestAppContext,
    path: &std::path::Path,
    expected: (i32, i32),
) {
    let resource = gpui::Resource::Path(path.into());
    let image = app.update(|cx| cx.fetch_asset::<gpui::ImgResourceLoader>(&resource).0);
    let image = app
        .foreground_executor()
        .block_test(image)
        .expect("load overlay asset");
    assert_eq!((image.size(0).width.0, image.size(0).height.0), expected);
}

fn rounded_overlay_test_root(path: &std::path::Path) -> FrameRoot {
    use crate::preview_engine::{PreviewRenderPresentation, rendered_frame_from_bgra_payload};
    use frame_core::aspect::SampleAspectRatio;
    let mut root = FrameRoot::new();
    root.apply_visual_fixture(Some(VisualFixture::PreviewAnamorphicOverlay));
    let mut metadata = root.selected_source_metadata().unwrap();
    metadata.width = Some(400);
    metadata.height = Some(4);
    metadata.sample_aspect_ratio = SampleAspectRatio::new(1, 1);
    root.source_metadata.mark_ready("fixture-preview", metadata);
    let mut overlay = root.preview_ui.overlay.overlay().unwrap().clone();
    overlay.path = path.to_string_lossy().into_owned();
    overlay.width = 0.05;
    root.preview_ui.overlay = crate::preview::PreviewOverlayState::new();
    root.preview_ui.overlay.sync_initial_overlay(Some(&overlay));
    root.preview_ui.overlay.set_overlay_mode(true, false);
    root.preview_ui.overlay_dimensions_key = Some(overlay.path.clone());
    root.preview_ui.overlay_image_dimensions =
        Some(crate::app::preview::state::PreviewOverlayImageDimensions {
            width: 100,
            height: 10,
        });
    let config = crate::conversion_runner::core_config_from_gpui(root.selected_config().unwrap());
    let plan = frame_core::preview::build_ffmpeg_preview_args(
        "source",
        &config,
        &frame_core::preview::PreviewFfmpegOptions {
            start_seconds: 0.0,
            end_seconds: None,
            source_width: Some(400),
            source_height: Some(4),
            source_is_palette: false,
            source_sample_aspect_ratio: SampleAspectRatio::new(1, 1),
            source_clock: frame_core::source_timing::ResolvedSourceClock::Unknown,
            max_width: 240,
            max_height: 240,
            fps: 30,
            realtime: false,
            precise_seek: true,
            source_is_image: false,
        },
    )
    .unwrap();
    assert_eq!((plan.width, plan.height), (240, 2));
    let frame = rendered_frame_from_bgra_payload(
        plan.width,
        plan.height,
        plan.width * 4,
        0,
        vec![0; plan.frame_bytes],
    )
    .unwrap()
    .with_presentation(
        PreviewRenderPresentation {
            pixel_aspect_ratio: plan.pixel_aspect_ratio,
            output_raster_aspect_ratio: plan.output_raster_aspect_ratio,
            ..Default::default()
        },
        0,
    );
    root.preview_ui.render_image = Some(frame.render_image());
    root.preview_ui.rendered_presentation = frame.presentation;
    root
}

fn normalized_test_overlay_size(root: &FrameRoot) -> (f64, f64) {
    let metadata = root.selected_source_metadata();
    let config = root.selected_config().unwrap();
    let settings =
        super::preview_shell::settings_state(config, metadata.as_ref(), MetadataStatus::Ready);
    let state = preview_shell_state(PreviewShellStateInput {
        selected_file: root.file_queue.selected_file(),
        settings: &settings,
        crop: root.preview_crop_render_state(metadata.as_ref(), config),
        overlay: root.preview_overlay_render_state(),
        canvas: root.preview_canvas_render_state(),
        playback: root.preview_playback_state(),
        presentation: root.preview_ui.rendered_presentation,
        render_image: root.preview_ui.render_image.clone(),
        runtime_error: None,
    });
    preview_overlay_test_render_size(&state)
}

#[expect(
    clippy::suboptimal_flops,
    reason = "The native edge oracle preserves GPUI's separate f32 layout operations before midpoint snapping; fused arithmetic can change the selected pixel."
)]
fn snapped_overlay_bounds(stage: Bounds<Pixels>, device_scale: f32) -> Bounds<Pixels> {
    // GPUI rounds absolute near/far edges in device pixels, with midpoint ties
    // toward zero. Keep this independent oracle in exact normalized geometry.
    let snap = |value: f32| px((value * device_scale - 0.5).ceil() / device_scale);
    Bounds::from_corners(
        point(
            snap(stage.origin.x.as_f32() + stage.size.width.as_f32() * 0.475),
            snap(stage.origin.y.as_f32() + stage.size.height.as_f32() * 0.25),
        ),
        point(
            snap(stage.origin.x.as_f32() + stage.size.width.as_f32() * 0.525),
            snap(stage.origin.y.as_f32() + stage.size.height.as_f32() * 0.75),
        ),
    )
}

#[test]
fn rounded_preview_texture_keeps_output_overlay_geometry_in_native_layout_and_keyboard() {
    use gpui::TestAppContext;
    let path = std::env::temp_dir().join(format!(
        "frame-r7-rounded-overlay-{}.png",
        std::process::id()
    ));
    image::RgbaImage::from_pixel(100, 10, image::Rgba([255, 0, 0, 255]))
        .save(&path)
        .unwrap();
    for scale in [ScalePreset::Percent100, ScalePreset::Percent200] {
        let mut app = TestAppContext::single();
        preload_overlay_test_image(&app, &path, (100, 10));
        let (root, cx) = app.add_window_view(|_, cx| {
            crate::app::init_app(cx, "Rounded overlay test");
            cx.set_reduce_motion(true);
            let mut root = rounded_overlay_test_root(&path);
            root.appearance.ui_scale = scale;
            root
        });
        cx.simulate_resize(size(px(1800.0), px(1200.0)));
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.simulate_next_frame(cx);
                window.draw(cx).clear(cx);
            });
        }
        let stage = cx.debug_bounds("preview-media-stage").unwrap();
        let selected = cx.debug_bounds("preview-overlay-layer").unwrap();
        let image = cx.debug_bounds("preview-overlay-image").unwrap();
        let normalized = root.read_with(cx, |root, _| normalized_test_overlay_size(root));
        assert_eq!(normalized, (0.05, 0.5));
        assert_eq!(normalized.0 / normalized.1 * 100.0, 10.0);
        let device_scale = cx.update(|window, _| window.scale_factor());
        let expected_bounds = snapped_overlay_bounds(stage, device_scale);
        root.read_with(cx, |root, _| {
            eprintln!(
                "rounded-overlay scale={scale:?} stage={stage:?} selected={selected:?} image={image:?} draft={:?} presentation={:?}",
                root.preview_ui.overlay.render_overlay(), root.preview_ui.rendered_presentation
            );
        });
        assert_eq!(image, selected);
        assert_eq!(
            selected, expected_bounds,
            "scale={scale:?}, device_scale={device_scale}"
        );
        root.update(cx, |root, _| {
            let media = preview_media_render_state(
                root.preview_ui.render_image.as_ref(),
                root.preview_ui.rendered_presentation,
            )
            .unwrap();
            assert_eq!((media.width, media.height), (240, 2));
            assert_eq!(media.raster_aspect_ratio(), 100.0);
            for _ in 0..8 {
                root.adjust_preview_overlay_from_keyboard_with_step(
                    crate::preview::OverlayDragHandle::Move,
                    "down",
                    Some(media),
                    true,
                );
            }
            assert!((root.preview_ui.overlay.overlay().unwrap().y - 0.75).abs() < 0.000_001);
            assert!(root.nudge_selected_overlay_size(
                crate::preview::OverlaySizeDirection::Increase,
                Some(media)
            ));
            let overlay = root.preview_ui.overlay.overlay().unwrap();
            assert!((overlay.y - (1.0 - overlay.width * 10.0 / 2.0)).abs() < 0.000_001);
        });
    }
    std::fs::remove_file(path).unwrap();
}
