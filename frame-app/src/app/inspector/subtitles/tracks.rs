use crate::app::components::{
    FRAME_ICON_SM_SIZE, FrameIconButtonSize, FrameIconButtonVariant, FrameTrackListItemLayout,
    FrameTrackListItemText, frame_choice_button, frame_icon_button, frame_text_button,
    frame_text_button_with_focus, frame_track_list_item,
};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::panel::settings_section;
use crate::app::inspector::shared::{settings_field_label, settings_hint_text};
use crate::app::inspector::subtitles::view::SettingsSubtitlesTabState;
use crate::app::layout::SETTINGS_CONTROL_HEIGHT;
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, apply_external_subtitle_default, apply_external_subtitle_forced,
    remove_external_subtitle_track, toggle_subtitle_track_selection,
};
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, Window, div};

pub(super) fn settings_external_subtitles_section(
    state: &SettingsSubtitlesTabState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let config = state.config;
    let palette = state.palette;
    let needs_unavailable_dvbsub =
        config.container.eq_ignore_ascii_case("m2t") && !state.available_encoders.dvbsub;
    let enabled = !state.settings_disabled && !needs_unavailable_dvbsub;
    let mut section = settings_section("External files", palette)
        .child(settings_external_subtitle_add_button(
            enabled,
            state.focuses.add_external_files,
            palette,
            window,
            cx,
        ))
        .child(settings_hint_text(
            if needs_unavailable_dvbsub {
                "Selectable .sup/PGS requires the DVB subtitle encoder, which is unavailable in the active FFmpeg runtime. Text subtitles can still be burned in."
            } else {
                "Embedded as switchable tracks in the exported file; they are not shown in the preview."
            },
            palette,
        ));

    if config.external_subtitle_tracks.is_empty() {
        return section.child(settings_hint_text("No external subtitle files", palette));
    }

    let selected_index = state
        .external_track_index
        .filter(|index| *index < config.external_subtitle_tracks.len());
    let mut list = div().flex().flex_col().gap_2();
    for (index, track) in config.external_subtitle_tracks.iter().enumerate() {
        let track_requires_dvbsub = config.container.eq_ignore_ascii_case("m2t")
            && std::path::Path::new(&track.path)
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sup"));
        list = list.child(settings_external_subtitle_track_row(
            index,
            track,
            &config.container,
            selected_index == Some(index),
            enabled && (!track_requires_dvbsub || state.available_encoders.dvbsub),
            palette,
            window,
            cx,
        ));
    }
    section = section.child(list);

    if let Some(index) = selected_index
        && external_subtitle_metadata_visibility(&config.container).has_any()
    {
        section = section.child(settings_external_subtitle_editor(
            config,
            index,
            enabled,
            state.external_language_focus,
            state.external_title_focus,
            palette,
            window,
            cx,
        ));
    }

    section
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExternalSubtitleMetadataVisibility {
    pub(super) language: bool,
    pub(super) descriptive: bool,
}

impl ExternalSubtitleMetadataVisibility {
    pub(super) const fn has_any(self) -> bool {
        self.language || self.descriptive
    }
}

pub(super) const fn external_subtitle_metadata_visibility(
    container: &str,
) -> ExternalSubtitleMetadataVisibility {
    match frame_core::container::transport_stream_profile(container) {
        Some(frame_core::container::TransportStreamProfile::M2ts192) => {
            ExternalSubtitleMetadataVisibility {
                language: false,
                descriptive: false,
            }
        }
        Some(frame_core::container::TransportStreamProfile::MpegTs188) => {
            ExternalSubtitleMetadataVisibility {
                language: true,
                descriptive: false,
            }
        }
        None => ExternalSubtitleMetadataVisibility {
            language: true,
            descriptive: true,
        },
    }
}

pub(super) fn settings_external_subtitle_add_button(
    enabled: bool,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let button = if let Some(focus) = focus {
        frame_text_button_with_focus(
            "settings-subtitle-add-external",
            "Add subtitle files",
            ButtonVariant::Secondary,
            false,
            enabled,
            focus,
            palette,
            window,
            cx,
        )
    } else {
        frame_text_button(
            "settings-subtitle-add-external",
            "Add subtitle files",
            ButtonVariant::Secondary,
            false,
            enabled,
            palette,
            window,
            cx,
        )
    };

    button
        .w_full()
        .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if enabled {
                root.prompt_external_subtitle_files(window, cx);
            }
        }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "The row receives track state, container capability context, and GPUI render handles explicitly."
)]
pub(super) fn settings_external_subtitle_track_row(
    index: usize,
    track: &crate::settings::ExternalSubtitleTrack,
    container: &str,
    selected: bool,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let filename = std::path::Path::new(&track.path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&track.path)
        .to_string();
    let mut details = Vec::new();
    if let Some(language) = track
        .language
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        details.push(language.trim().to_string());
    }
    if track.is_default {
        details.push("Default".to_string());
    }
    if track.is_forced {
        details.push("Forced".to_string());
    }
    let is_sup = std::path::Path::new(&track.path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("sup"));
    if frame_core::container::is_transport_stream_container(container) && !is_sup {
        details.push("Unavailable as selectable; use Burn-in".to_string());
    }
    let detail = details.join(" • ");

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div().flex_1().min_w_0().child(
                frame_track_list_item(
                    format!("external-subtitle-track-{index}"),
                    FrameTrackListItemText {
                        index_label: format!("#{}", index + 1),
                        primary: filename,
                        detail,
                        trailing: String::new(),
                        layout: FrameTrackListItemLayout::Compact,
                    },
                    selected,
                    enabled,
                    palette,
                    window,
                    cx,
                )
                .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if enabled && root.subtitle_ui.external_track_index != Some(index) {
                        root.subtitle_ui.external_track_index = Some(index);
                        cx.notify();
                    }
                })),
            ),
        )
        .child(
            frame_icon_button(
                format!("external-subtitle-remove-{index}"),
                assets::ICON_TRASH,
                format!("Remove selectable subtitle {}", index + 1),
                FrameIconButtonVariant::DestructiveGhost,
                enabled,
                FrameIconButtonSize {
                    button: SETTINGS_CONTROL_HEIGHT,
                    icon: FRAME_ICON_SM_SIZE,
                },
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                let mut remaining = None;
                let changed = root.update_selected_config(|config| {
                    let changed = remove_external_subtitle_track(config, index);
                    remaining = Some(config.external_subtitle_tracks.len());
                    changed
                });
                if changed {
                    let remaining = remaining.unwrap_or_default();
                    root.subtitle_ui.external_track_index =
                        match root.subtitle_ui.external_track_index {
                            Some(_) if remaining == 0 => None,
                            Some(selected) if selected > index => Some(selected - 1),
                            Some(selected) if selected == index => Some(index.min(remaining - 1)),
                            selected => selected,
                        };
                    cx.notify();
                }
            })),
        )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The editor follows the explicit settings-control render contract used across the panel."
)]
#[expect(
    clippy::too_many_lines,
    reason = "The compact sidecar editor keeps its paired fields and track flags in one render unit."
)]
pub(super) fn settings_external_subtitle_editor(
    config: &ConversionConfig,
    index: usize,
    enabled: bool,
    language_focus: Option<&FocusHandle>,
    title_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let track = &config.external_subtitle_tracks[index];
    let language = track.language.as_deref().unwrap_or_default();
    let title = track.title.as_deref().unwrap_or_default();
    let is_default = track.is_default;
    let is_forced = track.is_forced;
    let visibility = external_subtitle_metadata_visibility(&config.container);
    let mut fields = div()
        .grid()
        .grid_cols(if visibility.descriptive { 2 } else { 1 })
        .gap_2();
    if visibility.language {
        fields = fields.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_field_label("Language", palette))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: "settings-external-subtitle-language",
                        value: language,
                        placeholder: "e.g. eng",
                        disabled: !enabled,
                        focus: language_focus,
                        kind: FrameTextInputKind::ExternalSubtitleLanguage,
                    },
                    palette,
                    window,
                    cx,
                )),
        );
    }
    if visibility.descriptive {
        fields = fields.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_field_label("Title", palette))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: "settings-external-subtitle-title",
                        value: title,
                        placeholder: "Optional label",
                        disabled: !enabled,
                        focus: title_focus,
                        kind: FrameTextInputKind::ExternalSubtitleTitle,
                    },
                    palette,
                    window,
                    cx,
                )),
        );
    }

    let mut editor = div()
        .flex()
        .flex_col()
        .gap_3()
        .pt(theme::ui_rem(4.0))
        .child(fields);
    if visibility.descriptive {
        editor = editor.child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(
                    frame_choice_button(
                        "settings-external-subtitle-default",
                        "Default",
                        is_default,
                        enabled,
                        palette,
                        window,
                        cx,
                    )
                    .on_click(cx.listener(
                        move |root, _: &ClickEvent, _window, cx| {
                            cx.stop_propagation();
                            if enabled
                                && root.update_selected_config(|config| {
                                    apply_external_subtitle_default(config, index, !is_default)
                                })
                            {
                                cx.notify();
                            }
                        },
                    )),
                )
                .child(
                    frame_choice_button(
                        "settings-external-subtitle-forced",
                        "Forced",
                        is_forced,
                        enabled,
                        palette,
                        window,
                        cx,
                    )
                    .on_click(cx.listener(
                        move |root, _: &ClickEvent, _window, cx| {
                            cx.stop_propagation();
                            if enabled
                                && root.update_selected_config(|config| {
                                    apply_external_subtitle_forced(config, index, !is_forced)
                                })
                            {
                                cx.notify();
                            }
                        },
                    )),
                ),
        );
    }
    editor
}

pub(in crate::app) fn settings_subtitle_track_button(
    mut option: crate::settings::SubtitleTrackOption,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let index = option.index;
    let is_enabled = !option.is_disabled;
    if let Some(reason) = option.disabled_reason.take() {
        option.detail = if option.detail.is_empty() {
            reason
        } else {
            format!("{} • {reason}", option.detail)
        };
    }
    frame_track_list_item(
        format!("subtitle-track-{index}"),
        FrameTrackListItemText {
            index_label: option.index_label,
            primary: option.codec,
            detail: option.detail,
            trailing: String::new(),
            layout: FrameTrackListItemLayout::Compact,
        },
        option.is_selected,
        is_enabled,
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if !is_enabled {
            return;
        }
        if root.update_selected_config(|config| toggle_subtitle_track_selection(config, index)) {
            cx.notify();
        }
    }))
}
