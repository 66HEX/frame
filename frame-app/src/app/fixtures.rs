use super::*;

use frame_updater::{PlatformAssetKey, UpdateAsset};
use semver::Version;

const UPDATE_AVAILABLE_RELEASE_NOTES: &str = r#"
# Frame 0.1.1

- Native autoupdater flow with signed manifests and platform-specific packages.
- Animated update dialog that presents release notes before the user downloads or installs.
- macOS, Windows, and Linux release assets generated from the release pipeline.
- Bundled GStreamer runtime fixes for packaged builds.
- Update helper validation for replacing installed app bundles safely.

## Notes for visual review

This fixture intentionally uses enough text to exercise the dialog layout, scrolling,
button placement, and close animation. Production release notes should be carried in
`releaseNotesMarkdown` so the user can decide based on visible changes instead of
accepting an update blindly.
"#;

impl FrameRoot {
    pub(super) fn apply_visual_fixture(&mut self, fixture: Option<VisualFixture>) {
        match fixture {
            Some(VisualFixture::AppSettings) => self.open_app_settings(),
            Some(VisualFixture::LogsActive) => self.apply_logs_active_fixture(),
            Some(VisualFixture::PreviewCrop) => self.apply_preview_crop_fixture(),
            Some(VisualFixture::PreviewReady) => self.apply_preview_ready_fixture(),
            Some(VisualFixture::SettingsAudio) => self.apply_settings_audio_fixture(),
            Some(VisualFixture::SettingsImages) => self.apply_settings_images_fixture(),
            Some(VisualFixture::SettingsMetadata) => self.apply_settings_metadata_fixture(),
            Some(VisualFixture::SettingsOutput) => self.apply_settings_output_fixture(),
            Some(VisualFixture::SettingsPresets) => self.apply_settings_presets_fixture(),
            Some(VisualFixture::SettingsSource) => self.apply_settings_source_fixture(),
            Some(VisualFixture::SettingsSubtitles) => self.apply_settings_subtitles_fixture(),
            Some(VisualFixture::SettingsSubtitlesPopover) => {
                self.apply_settings_subtitles_popover_fixture();
            }
            Some(VisualFixture::SettingsVideo) => self.apply_settings_video_fixture(),
            Some(VisualFixture::UpdateAvailable) => self.apply_update_available_fixture(),
            Some(VisualFixture::WorkspaceAudio) => self.apply_workspace_audio_fixture(),
            Some(VisualFixture::WorkspaceEmpty) => self.apply_workspace_empty_fixture(),
            Some(VisualFixture::WorkspaceImage) => self.apply_workspace_image_fixture(),
            None => {}
        }
    }

    pub(super) fn apply_update_available_fixture(&mut self) {
        let Ok(asset_key) = PlatformAssetKey::current() else {
            return;
        };
        let update = Box::new(UpdateInfo {
            version: Version::new(0, 1, 1),
            channel: UpdateChannel::Stable,
            asset_key,
            asset: update_available_fixture_asset(asset_key),
            release_notes_url: Some(
                "https://github.com/66HEX/frame/releases/tag/v0.1.1".to_string(),
            ),
            release_notes_markdown: Some(UPDATE_AVAILABLE_RELEASE_NOTES.to_string()),
        });

        self.update_ui.status = UpdateStatus::Available(update.clone());
        self.update_ui.dialog_info = Some(update);
        self.update_ui.dialog_open = true;
        self.update_ui.dialog_present = true;
    }

    pub(super) fn apply_workspace_empty_fixture(&mut self) {
        self.active_view = ActiveView::Workspace;
        self.file_queue = FileQueue::new();
        self.source_metadata = SourceMetadataStore::default();
        self.settings_ui.active_tab = SettingsTab::Source;
    }
    pub(super) fn apply_workspace_audio_fixture(&mut self) {
        self.seed_audio_source_fixture();
        self.settings_ui.active_tab = SettingsTab::Source;
    }
    pub(super) fn apply_workspace_image_fixture(&mut self) {
        self.seed_image_source_fixture();
        self.settings_ui.active_tab = SettingsTab::Source;
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
        self.preview_ui.crop_file_id = Some("fixture-preview".to_string());
        self.preview_ui.crop_mode = true;
        self.preview_ui.draft_crop = Some(CropRect {
            x: 0.18,
            y: 0.16,
            width: 0.64,
            height: 0.64,
        });
        self.preview_ui.crop_aspect = "1:1".to_string();
    }
    pub(super) fn apply_settings_source_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_ui.active_tab = SettingsTab::Source;
    }
    pub(super) fn apply_settings_output_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_ui.active_tab = SettingsTab::Output;
        self.file_queue
            .update_selected_output_name("source_render_review.mov");
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.container = "mov".to_string();
        }
    }
    pub(super) fn apply_settings_video_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_ui.active_tab = SettingsTab::Video;
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.resolution = "custom".to_string();
            file.config.custom_width = Some("1920".to_string());
            file.config.custom_height = Some("1080".to_string());
            file.config.video_bitrate_mode = "crf".to_string();
            file.config.crf = 18;
        }
    }
    pub(super) fn apply_settings_audio_fixture(&mut self) {
        self.seed_audio_source_fixture();
        self.settings_ui.active_tab = SettingsTab::Audio;
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.container = "mp3".to_string();
            file.config.audio_codec = "mp3".to_string();
            file.config.audio_bitrate_mode = "vbr".to_string();
            file.config.audio_quality = "2".to_string();
            file.config.audio_channels = "stereo".to_string();
            file.config.audio_volume = 145;
            file.config.audio_normalize = true;
            file.config.selected_audio_tracks = vec![1];
        }
    }
    pub(super) fn apply_settings_images_fixture(&mut self) {
        self.seed_image_source_fixture();
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.container = "png".to_string();
            file.config.resolution = "custom".to_string();
            file.config.custom_width = Some("2048".to_string());
            file.config.custom_height = Some("1080".to_string());
        }
        self.settings_ui.active_tab = SettingsTab::Images;
    }
    pub(super) fn apply_settings_metadata_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_ui.active_tab = SettingsTab::Metadata;
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
        self.settings_ui.active_tab = SettingsTab::Subtitles;
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
        self.subtitle_ui.popover = Some(SettingsSubtitlePopover::FontColor);
        self.subtitle_ui.rendered_popover = Some(SettingsSubtitlePopover::FontColor);
        self.subtitle_ui.font_color_draft = "#FFD166".to_string();
        self.subtitle_ui.font_color_hsv_draft = settings_panel::hex_to_subtitle_hsv("#ffd166");
    }
    pub(super) fn apply_settings_presets_fixture(&mut self) {
        self.apply_preview_ready_fixture();
        self.settings_ui.active_tab = SettingsTab::Presets;
        self.settings_ui.preset_name_draft = "Client Review MP4".to_string();
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

    fn seed_audio_source_fixture(&mut self) {
        self.active_view = ActiveView::Workspace;
        self.file_queue.add_file(FileItem::from_path(
            "fixture-audio",
            "/tmp/source_mix.wav",
            96_468_480,
        ));
        self.source_metadata.mark_ready(
            "fixture-audio",
            SourceMetadata {
                media_kind: Some(SourceKind::Audio),
                duration: Some("184.250000".to_string()),
                bitrate: Some("1536000".to_string()),
                audio_codec: Some("pcm_s16le".to_string()),
                audio_tracks: vec![
                    crate::settings::AudioTrack {
                        index: 0,
                        codec: "pcm_s16le".to_string(),
                        channels: Some("2".to_string()),
                        language: Some("eng".to_string()),
                        label: Some("Main mix".to_string()),
                        bitrate_kbps: Some(1536.0),
                        sample_rate: Some("48000".to_string()),
                    },
                    crate::settings::AudioTrack {
                        index: 1,
                        codec: "aac".to_string(),
                        channels: Some("2".to_string()),
                        language: Some("eng".to_string()),
                        label: Some("Reference".to_string()),
                        bitrate_kbps: Some(192.0),
                        sample_rate: Some("48000".to_string()),
                    },
                ],
                tags: Some(SourceTags {
                    title: Some("Source Mix".to_string()),
                    artist: Some("Frame Audio".to_string()),
                    album: None,
                    genre: None,
                    date: Some("2026".to_string()),
                    comment: Some("Stereo master".to_string()),
                }),
                ..SourceMetadata::default()
            },
        );
        if let Some(file) = self.file_queue.selected_file_mut() {
            file.config.container = "mp3".to_string();
            file.config.audio_codec = "mp3".to_string();
        }
    }

    fn seed_image_source_fixture(&mut self) {
        self.active_view = ActiveView::Workspace;
        self.file_queue.add_file(FileItem::from_path(
            "fixture-image",
            "/tmp/source_frame.png",
            8_388_608,
        ));
        self.source_metadata.mark_ready(
            "fixture-image",
            SourceMetadata {
                media_kind: Some(SourceKind::Image),
                video_codec: Some("png".to_string()),
                resolution: Some("4096x2160".to_string()),
                width: Some(4096),
                height: Some(2160),
                pixel_format: Some("rgba".to_string()),
                color_space: Some("bt709".to_string()),
                color_range: Some("pc".to_string()),
                ..SourceMetadata::default()
            },
        );
    }
}

fn update_available_fixture_asset(asset_key: PlatformAssetKey) -> UpdateAsset {
    let file_name = match asset_key {
        PlatformAssetKey::MacosAarch64 | PlatformAssetKey::MacosX8664 => "Frame-fixture.app.zip",
        PlatformAssetKey::WindowsX8664 => "Frame-fixture.exe",
        PlatformAssetKey::LinuxX8664 | PlatformAssetKey::LinuxAarch64 => {
            "frame-fixture-linux.tar.gz"
        }
    };

    UpdateAsset {
        target_triple: asset_key.target_triple().to_string(),
        kind: asset_key.asset_kind(),
        file_name: file_name.to_string(),
        url: format!("https://example.com/{file_name}"),
        size_bytes: 1,
        sha256: "0".repeat(64),
        installer_args: Vec::new(),
    }
}
