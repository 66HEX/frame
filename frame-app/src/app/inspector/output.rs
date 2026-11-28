use crate::app::components::{FrameOption, frame_option_field, frame_select_field};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::panel::settings_field;
use crate::app::inspector::shared::settings_hint_text;
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, ProcessingStream, SourceMetadata, apply_output_container,
    apply_processing_mode, container_supports_audio, normalize_output_config,
    output_container_options, output_processing_mode_options, processing_stream_available,
};
use crate::theme;
use gpui::prelude::{FluentBuilder, ParentElement};
use gpui::{Context, FocusHandle, Window};

#[expect(
    clippy::too_many_arguments,
    reason = "The output tab explicitly receives conversion state, editable input, focus, palette, and render context."
)]
pub(in crate::app) fn settings_output_tab(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    output_name: &str,
    output_name_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let dimensions = metadata.and_then(|source| {
        frame_core::dimensions::output_dimensions(
            &crate::conversion_runner::core_config_from_gpui(config),
            source.width?,
            source.height?,
            source.pixel_format.as_deref(),
        )
    });
    let mut fields = crate::app::components::frame_field_group();
    for stream in [ProcessingStream::Video, ProcessingStream::Audio] {
        if processing_stream_available(config, metadata, stream) {
            fields = fields.child(settings_processing_mode_grid(
                stream,
                config,
                metadata,
                settings_disabled,
                palette,
                window,
                cx,
            ));
        }
    }
    fields
        .child(
            crate::app::inspector::panel::settings_fields_grid()
                .child(
                    settings_field("Output name", palette).child(settings_output_name_field(
                        output_name,
                        settings_disabled,
                        output_name_focus,
                        palette,
                        window,
                        cx,
                    )),
                )
                .child(
                    settings_field("Container", palette).child(settings_container_grid(
                        config,
                        metadata,
                        settings_disabled,
                        palette,
                        window,
                        cx,
                    )),
                ),
        )
        .child(settings_hint_text(
            "Output is saved to the default folder selected in Settings.",
            palette,
        ))
        .when_some(dimensions, |panel, (width, height)| {
            panel.child(crate::app::inspector::shared::settings_value_row(
                "Output dimensions",
                format!("{width} × {height} px"),
                palette,
            ))
        })
}

fn settings_processing_mode_grid(
    stream: ProcessingStream,
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let (id, label) = match stream {
        ProcessingStream::Video => ("video", "Video mode"),
        ProcessingStream::Audio => ("audio", "Audio mode"),
    };
    let mut grid = Vec::new();
    for option in output_processing_mode_options(config, metadata, stream, settings_disabled) {
        let copy = option.copy;
        let action = if copy { "copy" } else { "reencode" };
        let is_enabled = !option.is_disabled;
        grid.push(FrameOption::new(
            format!("output-{id}-mode-{action}"),
            option.label,
            option.is_selected,
            is_enabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }

                let metadata = root.selected_source_metadata();
                if root.update_selected_config(|config| {
                    apply_processing_mode(
                        config,
                        metadata.as_ref(),
                        config.processing_mode.with_copy(stream, copy),
                    )
                }) {
                    root.sync_inspector_availability(metadata.as_ref());
                    cx.notify();
                }
            },
        ));
    }
    let field = settings_field(label, palette).child(frame_option_field(
        format!("{id}-processing-mode-select"),
        label,
        grid,
        palette,
        window,
        cx,
    ));
    if stream == ProcessingStream::Video && config.processing_mode.copies_video() {
        field.child(settings_hint_text(
            "Video cuts depend on source keyframes.",
            palette,
        ))
    } else {
        field
    }
}

pub(in crate::app) fn settings_container_grid(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = Vec::new();
    for option in output_container_options(config, metadata, settings_disabled) {
        let container = option.container;
        let is_enabled = !option.is_disabled;
        grid.push(FrameOption::new(
            format!("output-container-{container}"),
            container.clone(),
            option.is_selected,
            is_enabled,
            move |root, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }

                let metadata = root.selected_source_metadata();
                let changed = root.update_selected_config(|config| {
                    let had_audio_container = container_supports_audio(&config.container);
                    let container_changed = apply_output_container(config, &container);
                    if !had_audio_container
                        && container_supports_audio(&config.container)
                        && config.selected_audio_tracks.is_empty()
                        && let Some(first_track) = metadata
                            .as_ref()
                            .and_then(|metadata| metadata.audio_tracks.first())
                    {
                        config.selected_audio_tracks.push(first_track.index);
                    }
                    container_changed | normalize_output_config(config, metadata.as_ref())
                });
                if changed {
                    root.sync_inspector_availability(metadata.as_ref());
                    cx.notify();
                }
            },
        ));
    }
    frame_select_field("container-select", "Container", grid, palette, window, cx)
}

pub(in crate::app) fn settings_output_name_field(
    output_name: &str,
    disabled: bool,
    output_name_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_input(
        FrameTextInputSpec {
            id: "settings-output-name-field",
            value: output_name,
            placeholder: "my_render_final",
            disabled,
            focus: output_name_focus,
            kind: FrameTextInputKind::OutputName,
        },
        palette,
        window,
        cx,
    )
}
