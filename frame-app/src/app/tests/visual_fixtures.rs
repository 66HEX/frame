use super::*;

#[test]
fn app_settings_fixture_opens_runtime_settings_sheet() {
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
