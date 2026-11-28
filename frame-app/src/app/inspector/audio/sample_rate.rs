use super::TrackEditor;
use crate::app::components::{FrameOption, frame_field, frame_select_field};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::shared::{settings_field_label, settings_hint_text};
use crate::app::root::FrameRoot;
use crate::settings::{
    AudioTrack, ConversionConfig, apply_audio_sample_rate, audio_track_sample_rate_error,
};
use crate::theme;
use frame_core::audio_rules::{audio_sample_rate_custom, audio_sample_rates};
use gpui::{Context, Div, Window, prelude::*};

impl FrameRoot {
    pub(in crate::app) fn audio_sample_rate_input_value(&self) -> String {
        self.edited_audio_track()
            .and_then(|(config, track)| config.audio_track_overrides.get(&track.index)?.sample_rate)
            .map(|rate| rate.to_string())
            .unwrap_or_default()
    }

    pub(in crate::app) fn write_audio_sample_rate_input(
        &mut self,
        candidate: &str,
    ) -> Option<String> {
        if self.text_input_disabled(FrameTextInputKind::AudioSampleRate) {
            return None;
        }
        let (config, track) = self.edited_audio_track()?;
        if config.processing_mode.copies_audio()
            || !audio_sample_rate_custom(&config.container, &config.audio_codec)
        {
            return None;
        }
        let index = track.index;
        let rate = if candidate.is_empty() {
            None
        } else {
            Some(candidate.parse::<u32>().ok()?)
        };
        self.settings_ui.custom_audio_rate = true;
        self.update_selected_config(|config| apply_audio_sample_rate(config, index, rate));
        Some(rate.map(|rate| rate.to_string()).unwrap_or_default())
    }

    fn select_audio_rate(&mut self, index: u32, rate: Option<u32>, cx: &mut Context<Self>) {
        if self.file_queue.selected_file_locked()
            || !self.edited_audio_track().is_some_and(|(config, track)| {
                track.index == index && !config.processing_mode.copies_audio()
            })
        {
            return;
        }
        let was_custom = std::mem::take(&mut self.settings_ui.custom_audio_rate);
        let changed =
            self.update_selected_config(|config| apply_audio_sample_rate(config, index, rate));
        if changed || was_custom {
            self.reset_text_input(FrameTextInputKind::AudioSampleRate);
            cx.notify();
        }
    }
}

pub(super) fn field(
    config: &ConversionConfig,
    track: &AudioTrack,
    editor: TrackEditor<'_>,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Div {
    let copy = config.processing_mode.copies_audio();
    let enabled = enabled && !copy;
    let index = track.index;
    let selected = config
        .audio_track_overrides
        .get(&index)
        .and_then(|edit| edit.sample_rate);
    let rates: Vec<_> = audio_sample_rates(&config.container, &config.audio_codec)
        .iter()
        .copied()
        .filter(|&rate| audio_track_sample_rate_error(config, Some(track), rate).is_none())
        .collect();
    let custom_allowed = audio_sample_rate_custom(&config.container, &config.audio_codec);
    let nonstandard = selected.filter(|rate| !rates.contains(rate));
    let custom = !copy && custom_allowed && (editor.custom_rate || nonstandard.is_some());
    let mut choices = vec![FrameOption::new(
        "audio-rate-auto",
        if copy { "Original" } else { "Automatic" },
        copy || (!custom && selected.is_none()),
        enabled,
        move |root, _, cx| root.select_audio_rate(index, None, cx),
    )];
    if !copy {
        choices.extend(rates.iter().map(|&rate| {
            FrameOption::new(
                format!("audio-rate-{rate}"),
                format!("{rate} Hz"),
                !custom && selected == Some(rate),
                enabled,
                move |root, _, cx| root.select_audio_rate(index, Some(rate), cx),
            )
        }));
        if custom_allowed {
            choices.push(FrameOption::new(
                "audio-rate-custom",
                "Custom",
                custom,
                enabled,
                move |root, _, cx| {
                    root.settings_ui.custom_audio_rate = true;
                    cx.notify();
                },
            ));
        } else if let Some(rate) = nonstandard {
            choices.push(FrameOption::new(
                "audio-rate-unavailable",
                format!("{rate} Hz (unsupported)"),
                true,
                false,
                |_, _, _| {},
            ));
        }
    }
    let mut field = frame_field()
        .child(settings_field_label("Sample rate", palette))
        .child(frame_select_field(
            "audio-track-sample-rate",
            "Sample rate",
            choices,
            palette,
            window,
            cx,
        ));
    if custom {
        let value = selected.map(|rate| rate.to_string()).unwrap_or_default();
        field = field
            .child(settings_field_label("Custom rate (Hz)", palette))
            .child(frame_text_input(
                FrameTextInputSpec {
                    id: "audio-track-custom-rate",
                    value: &value,
                    placeholder: "Automatic",
                    disabled: !enabled,
                    focus: editor.sample_rate,
                    kind: FrameTextInputKind::AudioSampleRate,
                },
                palette,
                window,
                cx,
            ));
    }
    if let Some(rate) = track
        .sample_rate
        .as_deref()
        .and_then(|rate| rate.parse::<u32>().ok())
        .filter(|rate| *rate > 0)
    {
        field = field.child(settings_hint_text(
            &format!("Source rate {rate} Hz"),
            palette,
        ));
    }
    field
}
