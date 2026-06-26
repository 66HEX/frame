use super::*;

impl FrameRoot {
    pub(super) fn apply_visual_fixture(&mut self, fixture: Option<VisualFixture>) {
        match fixture {
            Some(VisualFixture::AppSettings) => self.open_app_settings(),
            Some(VisualFixture::LogsActive) => self.apply_logs_active_fixture(),
            Some(VisualFixture::PreviewCrop) => self.apply_preview_crop_fixture(),
            Some(VisualFixture::PreviewReady) => self.apply_preview_ready_fixture(),
            Some(VisualFixture::SettingsImages) => self.apply_settings_images_fixture(),
            Some(VisualFixture::SettingsMetadata) => self.apply_settings_metadata_fixture(),
            Some(VisualFixture::SettingsPresets) => self.apply_settings_presets_fixture(),
            Some(VisualFixture::SettingsSubtitles) => self.apply_settings_subtitles_fixture(),
            Some(VisualFixture::SettingsSubtitlesPopover) => {
                self.apply_settings_subtitles_popover_fixture();
            }
            Some(VisualFixture::SettingsVideo) => self.apply_settings_video_fixture(),
            None => {}
        }
    }
    pub(super) fn apply_logs_active_fixture(&mut self) {
        self.active_view = ActiveView::Logs;
        self.file_queue.add_file(FileItem::from_path(
            "fixture-video",
            "/tmp/source_render.mov",
            1_572_864_000,
        ));
        self.file_queue
            .update_status("fixture-video", FileStatus::Converting, 64);

        for line in [
            "ffmpeg version 7.1.1 Copyright (c) 2000-2025 the FFmpeg developers",
            "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'source_render.mov':",
            "Stream #0:0: Video: prores (HQ), yuv422p10le, 3840x2160, 24 fps",
            "Stream mapping:",
            "frame=  148 fps= 27 q=-0.0 size=   65536kB time=00:00:06.16 bitrate=87145.2kbits/s speed=1.12x",
            "frame=  296 fps= 28 q=-0.0 size=  131072kB time=00:00:12.33 bitrate=87042.7kbits/s speed=1.14x",
            "frame=  444 fps= 29 q=-0.0 size=  196608kB time=00:00:18.50 bitrate=87054.9kbits/s speed=1.16x",
        ] {
            self.conversion_events.apply_conversion_event(
                &mut self.file_queue,
                ConversionEvent::log("fixture-video", line),
            );
        }
    }
    pub(super) fn apply_preview_ready_fixture(&mut self) {
        self.active_view = ActiveView::Workspace;
        self.file_queue.add_file(FileItem::from_path(
            "fixture-preview",
            "/tmp/source_render.mov",
            1_572_864_000,
        ));
        self.source_metadata.mark_ready(
            "fixture-preview".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.400000".to_string()),
                bitrate: Some("12000000".to_string()),
                video_codec: Some("prores".to_string()),
                audio_codec: Some("aac".to_string()),
                resolution: Some("3840x2160".to_string()),
                frame_rate: Some(24.0),
                width: Some(3840),
                height: Some(2160),
                video_bitrate_kbps: Some(12_000.0),
                ..SourceMetadata::default()
            },
        );
    }
    pub(super) fn apply_preview_crop_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.preview_crop_file_id = Some("fixture-preview".to_string());
        self.preview_crop_mode = true;
        self.preview_draft_crop = Some(CropRect {
            x: 0.18,
            y: 0.16,
            width: 0.64,
            height: 0.64,
        });
        self.preview_crop_aspect = "1:1".to_string();
    }
    pub(super) fn apply_settings_video_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_active_tab = SettingsTab::Video;
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.resolution = "custom".to_string();
            file.config.custom_width = Some("1920".to_string());
            file.config.custom_height = Some("1080".to_string());
            file.config.video_bitrate_mode = "crf".to_string();
            file.config.crf = 18;
        }
    }
    pub(super) fn apply_settings_images_fixture(&mut self) {
        self.active_view = ActiveView::Workspace;
        self.file_queue.add_file(FileItem::from_path(
            "fixture-image",
            "/tmp/source_frame.png",
            8_388_608,
        ));
        self.source_metadata.mark_ready(
            "fixture-image".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Image),
                video_codec: Some("png".to_string()),
                resolution: Some("4096x2160".to_string()),
                width: Some(4096),
                height: Some(2160),
                pixel_format: Some("rgba".to_string()),
                ..SourceMetadata::default()
            },
        );
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.container = "png".to_string();
            file.config.resolution = "custom".to_string();
            file.config.custom_width = Some("2048".to_string());
            file.config.custom_height = Some("1080".to_string());
        }
        self.settings_active_tab = SettingsTab::Images;
    }
    pub(super) fn apply_settings_metadata_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_active_tab = SettingsTab::Metadata;
        self.source_metadata.mark_ready(
            "fixture-preview".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.400000".to_string()),
                bitrate: Some("12000000".to_string()),
                video_codec: Some("prores".to_string()),
                audio_codec: Some("aac".to_string()),
                resolution: Some("3840x2160".to_string()),
                frame_rate: Some(24.0),
                width: Some(3840),
                height: Some(2160),
                video_bitrate_kbps: Some(12_000.0),
                tags: Some(SourceTags {
                    title: Some("Original Scene 24A".to_string()),
                    artist: Some("Frame Camera".to_string()),
                    album: Some("Dailies".to_string()),
                    genre: Some("Editorial".to_string()),
                    date: Some("2026".to_string()),
                    comment: Some("Camera roll A014".to_string()),
                }),
                ..SourceMetadata::default()
            },
        );
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.metadata.title = Some("Render Scene 24A".to_string());
            file.config.metadata.comment = Some("Color pass".to_string());
        }
    }
    pub(super) fn apply_settings_subtitles_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_active_tab = SettingsTab::Subtitles;
        self.subtitle_font_families = vec![
            "Arial".to_string(),
            "Helvetica Neue".to_string(),
            "Inter".to_string(),
            "Noto Sans".to_string(),
            "SF Pro".to_string(),
        ];
        self.source_metadata.mark_ready(
            "fixture-preview".to_string(),
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("90.400000".to_string()),
                bitrate: Some("12000000".to_string()),
                video_codec: Some("h264".to_string()),
                audio_codec: Some("aac".to_string()),
                resolution: Some("1920x1080".to_string()),
                frame_rate: Some(24.0),
                width: Some(1920),
                height: Some(1080),
                subtitle_tracks: vec![
                    crate::settings::SubtitleTrack {
                        index: 2,
                        codec: "subrip".to_string(),
                        language: Some("eng".to_string()),
                        label: Some("Dialogue".to_string()),
                    },
                    crate::settings::SubtitleTrack {
                        index: 3,
                        codec: "ass".to_string(),
                        language: Some("jpn".to_string()),
                        label: Some("Signs".to_string()),
                    },
                ],
                ..SourceMetadata::default()
            },
        );
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.subtitle_burn_path = Some("/tmp/dialogue-final.srt".to_string());
            file.config.subtitle_font_name = Some("Arial".to_string());
            file.config.subtitle_font_size = Some("24".to_string());
            file.config.subtitle_font_color = Some("#ffd166".to_string());
            file.config.subtitle_outline_color = Some("#1d3557".to_string());
            file.config.subtitle_position = Some("bottom".to_string());
            file.config.selected_subtitle_tracks = vec![2];
        }
    }
    pub(super) fn apply_settings_subtitles_popover_fixture(&mut self) {
        self.apply_settings_subtitles_fixture();
        self.settings_subtitle_popover = Some(SettingsSubtitlePopover::FontColor);
        self.subtitle_font_color_draft = "#FFD166".to_string();
    }
    pub(super) fn apply_settings_presets_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_active_tab = SettingsTab::Presets;
        self.preset_name_draft = "Client Review MP4".to_string();
        self.presets.push(PresetDefinition::custom(
            "custom-review".to_string(),
            "Client Review MP4".to_string(),
            ConversionConfig {
                video_bitrate_mode: "bitrate".to_string(),
                video_bitrate: "7500".to_string(),
                audio_bitrate: "192".to_string(),
                audio_channels: "stereo".to_string(),
                resolution: "1080p".to_string(),
                preset: "fast".to_string(),
                ..ConversionConfig::default()
            },
        ));
    }
}
