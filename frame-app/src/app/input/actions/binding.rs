use crate::app::input::runtime::{FrameTextInputKind, FrameTimecodeInputDraft};
use crate::app::input::text::{sanitize_hex_draft, sanitize_number_input};
use crate::app::inspector::subtitles::state::SettingsSubtitleColorTarget;
use crate::app::root::FrameRoot;
use crate::file_queue::FileQueue;
use crate::preview::format_time;
use crate::settings::{
    MetadataField, apply_audio_bitrate, apply_custom_height, apply_custom_width,
    apply_external_subtitle_language, apply_external_subtitle_title, apply_gif_loop,
    apply_metadata_field, apply_video_bitrate, metadata_field_value, normalized_hex_color,
    sanitize_output_name,
};

impl FrameRoot {
    pub(in crate::app) fn text_input_value(&self, kind: FrameTextInputKind) -> String {
        match kind {
            FrameTextInputKind::CropWidth => self.preview_ui.crop_options.width.clone(),
            FrameTextInputKind::CropHeight => self.preview_ui.crop_options.height.clone(),
            FrameTextInputKind::MaxConcurrency => self.settings_ui.max_concurrency_draft.clone(),
            FrameTextInputKind::OutputName => self
                .file_queue
                .selected_file()
                .map_or_else(String::new, |file| file.output_name.clone()),
            FrameTextInputKind::AudioBitrate => self
                .file_queue
                .selected_file()
                .map_or_else(String::new, |file| file.config.audio_bitrate.clone()),
            FrameTextInputKind::VideoCustomWidth => self
                .file_queue
                .selected_file()
                .and_then(|file| file.config.custom_width.clone())
                .unwrap_or_default(),
            FrameTextInputKind::VideoCustomHeight => self
                .file_queue
                .selected_file()
                .and_then(|file| file.config.custom_height.clone())
                .unwrap_or_default(),
            FrameTextInputKind::VideoBitrate => self
                .file_queue
                .selected_file()
                .map_or_else(String::new, |file| file.config.video_bitrate.clone()),
            FrameTextInputKind::GifLoop => self
                .file_queue
                .selected_file()
                .map_or_else(String::new, |file| file.config.gif_loop.to_string()),
            FrameTextInputKind::PreviewStartTime | FrameTextInputKind::PreviewEndTime => self
                .text_input_runtime(kind)
                .timecode_draft
                .as_ref()
                .filter(|draft| self.file_queue.selected_file_id() == Some(draft.file_id.as_str()))
                .map_or_else(
                    || match kind {
                        FrameTextInputKind::PreviewStartTime => {
                            format_time(self.preview_ui.playback.start_value())
                        }
                        FrameTextInputKind::PreviewEndTime => {
                            format_time(self.preview_ui.playback.end_value())
                        }
                        _ => unreachable!("matched preview timecode text input variants"),
                    },
                    |draft| draft.value.clone(),
                ),
            FrameTextInputKind::MetadataTitle
            | FrameTextInputKind::MetadataArtist
            | FrameTextInputKind::MetadataAlbum
            | FrameTextInputKind::MetadataGenre
            | FrameTextInputKind::MetadataDate
            | FrameTextInputKind::MetadataComment
            | FrameTextInputKind::MetadataServiceName
            | FrameTextInputKind::MetadataServiceProvider => self
                .file_queue
                .selected_file()
                .and_then(|file| {
                    metadata_field_for_text_input(kind).and_then(|field| {
                        metadata_field_value(&file.config, field).map(str::to_string)
                    })
                })
                .unwrap_or_default(),
            FrameTextInputKind::PresetName => self.settings_ui.preset_name_draft.clone(),
            FrameTextInputKind::ExternalSubtitleLanguage
            | FrameTextInputKind::ExternalSubtitleTitle => self
                .subtitle_ui
                .external_track_index
                .and_then(|index| {
                    self.file_queue
                        .selected_file()
                        .and_then(|file| file.config.external_subtitle_tracks.get(index))
                })
                .and_then(|track| match kind {
                    FrameTextInputKind::ExternalSubtitleLanguage => track.language.clone(),
                    FrameTextInputKind::ExternalSubtitleTitle => track.title.clone(),
                    _ => None,
                })
                .unwrap_or_default(),
            FrameTextInputKind::SubtitleFontColorHex => self.subtitle_ui.font_color_draft.clone(),
            FrameTextInputKind::SubtitleOutlineColorHex => {
                self.subtitle_ui.outline_color_draft.clone()
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Text input routing centralizes per-field normalization so field behavior stays auditable in one place."
    )]
    pub(in crate::app) fn write_text_input_value(
        &mut self,
        kind: FrameTextInputKind,
        candidate: &str,
    ) -> Option<String> {
        match kind {
            FrameTextInputKind::CropWidth | FrameTextInputKind::CropHeight => {
                if self.text_input_disabled(kind) || !self.preview_ui.crop_mode {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                if !self.preview_ui.crop_options.custom_dirty {
                    self.preview_ui.crop_options.custom_edit_base = self.preview_ui.draft_crop;
                }
                if kind == FrameTextInputKind::CropWidth {
                    self.preview_ui.crop_options.width.clone_from(&next);
                } else {
                    self.preview_ui.crop_options.height.clone_from(&next);
                }
                self.preview_ui.crop_options.custom_dirty = true;
                self.preview_custom_crop_values();
                Some(next)
            }
            FrameTextInputKind::MaxConcurrency => {
                let next = sanitize_number_input(candidate);
                if self.settings_ui.max_concurrency_draft != next {
                    self.settings_ui.max_concurrency_draft.clone_from(&next);
                    self.settings_ui.max_concurrency_error = None;
                }
                Some(next)
            }
            FrameTextInputKind::OutputName => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_output_name(candidate);
                self.file_queue.set_selected_output_name_from_input(&next);
                Some(next)
            }
            FrameTextInputKind::AudioBitrate => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_audio_bitrate(&mut file.config, &next);
                })?;
                Some(next)
            }
            FrameTextInputKind::VideoCustomWidth => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_custom_width(&mut file.config, &next);
                })?;
                Some(next)
            }
            FrameTextInputKind::VideoCustomHeight => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_custom_height(&mut file.config, &next);
                })?;
                Some(next)
            }
            FrameTextInputKind::VideoBitrate => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_video_bitrate(&mut file.config, &next);
                })?;
                Some(next)
            }
            FrameTextInputKind::GifLoop => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_number_input(candidate);
                self.file_queue.selected_file_mut().map(|file| {
                    apply_gif_loop(&mut file.config, &next);
                })?;
                Some(file_gif_loop_value(&self.file_queue))
            }
            FrameTextInputKind::PreviewStartTime | FrameTextInputKind::PreviewEndTime => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let file_id = self.file_queue.selected_file_id()?.to_string();
                let next = candidate.to_string();
                self.text_input_runtime_mut(kind).timecode_draft = Some(FrameTimecodeInputDraft {
                    file_id,
                    value: next.clone(),
                });
                Some(next)
            }
            FrameTextInputKind::MetadataTitle
            | FrameTextInputKind::MetadataArtist
            | FrameTextInputKind::MetadataAlbum
            | FrameTextInputKind::MetadataGenre
            | FrameTextInputKind::MetadataDate
            | FrameTextInputKind::MetadataComment
            | FrameTextInputKind::MetadataServiceName
            | FrameTextInputKind::MetadataServiceProvider => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let field = metadata_field_for_text_input(kind)?;
                self.file_queue.selected_file_mut().map(|file| {
                    apply_metadata_field(&mut file.config, field, candidate);
                })?;
                Some(candidate.to_string())
            }
            FrameTextInputKind::PresetName => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next: String = candidate.chars().filter(|ch| !ch.is_control()).collect();
                self.settings_ui.preset_name_draft.clone_from(&next);
                self.settings_ui.preset_notice = None;
                Some(next)
            }
            FrameTextInputKind::ExternalSubtitleLanguage
            | FrameTextInputKind::ExternalSubtitleTitle => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let index = self.subtitle_ui.external_track_index?;
                let next: String = candidate.chars().filter(|ch| !ch.is_control()).collect();
                self.file_queue.selected_file_mut().map(|file| match kind {
                    FrameTextInputKind::ExternalSubtitleLanguage => {
                        apply_external_subtitle_language(&mut file.config, index, &next);
                    }
                    FrameTextInputKind::ExternalSubtitleTitle => {
                        apply_external_subtitle_title(&mut file.config, index, &next);
                    }
                    _ => unreachable!("matched external subtitle text input variants"),
                })?;
                Some(next)
            }
            FrameTextInputKind::SubtitleFontColorHex
            | FrameTextInputKind::SubtitleOutlineColorHex => {
                if self.file_queue.selected_file_locked() {
                    return None;
                }
                let next = sanitize_hex_draft(candidate);
                let target = match kind {
                    FrameTextInputKind::SubtitleFontColorHex => SettingsSubtitleColorTarget::Font,
                    FrameTextInputKind::SubtitleOutlineColorHex => {
                        SettingsSubtitleColorTarget::Outline
                    }
                    _ => unreachable!("matched subtitle color text input variants"),
                };
                self.set_subtitle_color_draft(target, next.clone());
                if let Some(normalized) = normalized_hex_color(&next) {
                    self.commit_subtitle_color(target, &normalized);
                }
                Some(next)
            }
        }
    }
}

pub(super) fn file_gif_loop_value(file_queue: &FileQueue) -> String {
    file_queue
        .selected_file()
        .map_or_else(String::new, |file| file.config.gif_loop.to_string())
}

pub(super) const fn metadata_field_for_text_input(
    kind: FrameTextInputKind,
) -> Option<MetadataField> {
    match kind {
        FrameTextInputKind::MetadataTitle => Some(MetadataField::Title),
        FrameTextInputKind::MetadataArtist => Some(MetadataField::Artist),
        FrameTextInputKind::MetadataAlbum => Some(MetadataField::Album),
        FrameTextInputKind::MetadataGenre => Some(MetadataField::Genre),
        FrameTextInputKind::MetadataDate => Some(MetadataField::Date),
        FrameTextInputKind::MetadataComment => Some(MetadataField::Comment),
        FrameTextInputKind::MetadataServiceName => Some(MetadataField::ServiceName),
        FrameTextInputKind::MetadataServiceProvider => Some(MetadataField::ServiceProvider),
        _ => None,
    }
}
