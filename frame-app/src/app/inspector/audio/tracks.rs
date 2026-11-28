use crate::app::components::{FrameOption, frame_boolean_field, frame_field, frame_select_field};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::shared::{settings_field_label, settings_hint_text};
use crate::app::root::FrameRoot;
use crate::settings::{
    AudioTrack, ConversionConfig, MetadataMode, OutputSection, SettingsTab, SourceMetadata,
    apply_audio_track_default, audio_track_is_default,
};
use crate::theme;
use frame_core::media_rules::audio_track_metadata_support;
use gpui::{Context, Div, FocusHandle, Window, div, prelude::*};

#[derive(Clone, Copy, Default)]
pub(in crate::app) struct TrackEditor<'a> {
    pub(in crate::app) index: Option<u32>,
    pub(in crate::app) language: Option<&'a FocusHandle>,
    pub(in crate::app) title: Option<&'a FocusHandle>,
}

impl FrameRoot {
    pub(in crate::app) fn sync_audio_editor(
        &mut self,
        config: &ConversionConfig,
        metadata: Option<&SourceMetadata>,
    ) {
        let current = self.settings_ui.audio_editor.as_ref();
        let file_id = self.file_queue.selected_file_id();
        let index = metadata
            .filter(|_| {
                self.settings_ui.active_tab == SettingsTab::Output
                    && self
                        .settings_ui
                        .expanded_output_sections
                        .contains(&OutputSection::Audio)
            })
            .and_then(|metadata| {
                let valid = |index: &u32| metadata.audio_track(*index).is_some();
                current
                    .filter(|(id, index)| {
                        Some(id.as_str()) == file_id
                            && config.selected_audio_tracks.contains(index)
                            && valid(index)
                    })
                    .map(|(_, index)| *index)
                    .or_else(|| {
                        config
                            .selected_audio_tracks
                            .iter()
                            .find(|index| valid(index))
                            .copied()
                    })
            });
        if current.map(|(id, index)| (id.as_str(), *index)) != file_id.zip(index) {
            self.set_audio_editor(index);
        }
    }

    fn set_audio_editor(&mut self, index: Option<u32>) {
        for kind in [
            FrameTextInputKind::AudioTrackLanguage,
            FrameTextInputKind::AudioTrackTitle,
        ] {
            self.reset_text_input(kind);
        }
        self.settings_ui.audio_editor = self
            .file_queue
            .selected_file_id()
            .zip(index)
            .map(|(id, index)| (id.to_owned(), index));
    }

    fn edited_audio_track(&self) -> Option<(&ConversionConfig, &AudioTrack)> {
        let (id, index) = self.settings_ui.audio_editor.as_ref()?;
        let file = self.file_queue.selected_file()?;
        if file.id != *id || !file.config.selected_audio_tracks.contains(index) {
            return None;
        }
        let track = self.source_metadata.metadata_for(id)?.audio_track(*index)?;
        Some((&file.config, track))
    }

    pub(in crate::app) fn audio_track_input_value(
        &self,
        kind: FrameTextInputKind,
    ) -> Option<String> {
        let (config, track) = self.edited_audio_track()?;
        text_value(config, track, kind).map(str::to_owned)
    }

    pub(in crate::app) fn write_audio_track_input(
        &mut self,
        kind: FrameTextInputKind,
        value: &str,
    ) -> Option<String> {
        if self.text_input_disabled(kind) {
            return None;
        }
        let (config, track) = self.edited_audio_track()?;
        let index = track.index;
        let support = audio_track_metadata_support(&config.container);
        let is_language = match kind {
            FrameTextInputKind::AudioTrackLanguage if support.language => true,
            FrameTextInputKind::AudioTrackTitle if support.title => false,
            _ => return None,
        };
        self.update_selected_config(|config| {
            let edit = config.audio_track_overrides.entry(index).or_default();
            let edit = if is_language {
                &mut edit.language
            } else {
                &mut edit.title
            };
            if edit.as_deref() == Some(value) {
                return false;
            }
            *edit = Some(value.to_owned());
            true
        });
        Some(value.to_owned())
    }
}

fn text_value<'a>(
    config: &'a ConversionConfig,
    track: &'a AudioTrack,
    kind: FrameTextInputKind,
) -> Option<&'a str> {
    let edit = config.audio_track_overrides.get(&track.index);
    match kind {
        FrameTextInputKind::AudioTrackLanguage => edit
            .and_then(|e| e.language.as_deref())
            .or(track.language.as_deref()),
        FrameTextInputKind::AudioTrackTitle => edit
            .and_then(|e| e.title.as_deref())
            .or(track.metadata_title.as_deref()),
        _ => None,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The track editor shares the audio tab's configuration, metadata, focus and render context."
)]
pub(super) fn track_editor(
    mut section: Div,
    editor: TrackEditor<'_>,
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Div {
    let Some(metadata) = metadata else {
        return section;
    };
    let Some(track) = editor.index.and_then(|index| metadata.audio_track(index)) else {
        return section;
    };
    let support = audio_track_metadata_support(&config.container);
    if !(support.language || support.title || support.default_flag) {
        return section;
    }
    let index = track.index;
    if let Some(selector) = track_selector(metadata, config, index, enabled, palette, window, cx) {
        section = section.child(selector);
    }
    let mut fields = div()
        .grid()
        .grid_cols(if support.language && support.title {
            2
        } else {
            1
        })
        .gap_2();
    for (visible, kind, id, label, placeholder, focus) in [
        (
            support.language,
            FrameTextInputKind::AudioTrackLanguage,
            "audio-track-language",
            "Language",
            "e.g. eng",
            editor.language,
        ),
        (
            support.title,
            FrameTextInputKind::AudioTrackTitle,
            "audio-track-title",
            "Title",
            "Optional label",
            editor.title,
        ),
    ] {
        if visible {
            fields = fields.child(
                frame_field()
                    .child(settings_field_label(label, palette))
                    .child(frame_text_input(
                        FrameTextInputSpec {
                            id,
                            value: text_value(config, track, kind).unwrap_or_default(),
                            placeholder,
                            disabled: !enabled,
                            focus,
                            kind,
                        },
                        palette,
                        window,
                        cx,
                    )),
            );
        }
    }
    if support.language || support.title {
        section = section.child(fields);
        if config.metadata.mode != MetadataMode::Preserve {
            section = section.child(settings_hint_text(
                "Track tags are exported only when Metadata is set to Preserve.",
                palette,
            ));
        }
    }
    if support.default_flag {
        section = section.child(
            frame_field()
                .child(settings_field_label("Default", palette))
                .child(frame_boolean_field(
                    "audio-track-default",
                    "Default",
                    audio_track_is_default(config, metadata, track),
                    enabled,
                    ["No", "Yes"],
                    move |root, selected, _, cx| {
                        if root.update_selected_config(|config| {
                            apply_audio_track_default(config, index, selected)
                        }) {
                            cx.notify();
                        }
                    },
                    palette,
                    window,
                    cx,
                )),
        );
    }
    section
}

fn track_selector(
    metadata: &SourceMetadata,
    config: &ConversionConfig,
    current: u32,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<Div> {
    let choices: Vec<_> = metadata
        .audio_tracks
        .iter()
        .filter(|t| config.selected_audio_tracks.contains(&t.index))
        .map(|t| {
            let index = t.index;
            FrameOption::new(
                format!("audio-metadata-track-{index}"),
                format!("Track #{index}"),
                current == index,
                enabled,
                move |root, _, cx| {
                    root.set_audio_editor(Some(index));
                    cx.notify();
                },
            )
        })
        .collect();
    (choices.len() > 1).then(|| {
        frame_field()
            .child(settings_field_label("Edit audio track", palette))
            .child(frame_select_field(
                "audio-metadata-track",
                "Edit audio track",
                choices,
                palette,
                window,
                cx,
            ))
    })
}
