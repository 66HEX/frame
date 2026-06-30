use super::{
    ClickEvent, Context, ConversionConfig, FocusHandle, FrameRoot, FrameTextInputKind,
    FrameTextInputSpec, ParentElement, SourceMetadata, StatefulInteractiveElement, Styled, Window,
    apply_output_container, apply_processing_mode, div, frame_choice_button, frame_text_input,
    normalize_output_config, output_container_options, output_processing_mode_options,
    settings_hint_text, settings_section,
};

pub(in crate::app) fn settings_output_tab(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    output_name: &str,
    output_name_focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            settings_section("Processing mode")
                .child(settings_processing_mode_grid(
                    config,
                    metadata,
                    settings_disabled,
                    window,
                    cx,
                ))
                .child(settings_hint_text(config.processing_mode.hint())),
        )
        .child(
            settings_section("Output name")
                .child(settings_output_name_field(
                    output_name,
                    settings_disabled,
                    output_name_focus,
                    window,
                    cx,
                ))
                .child(settings_hint_text(
                    "Output stays next to the original file.",
                )),
        )
        .child(
            settings_section("Output container").child(settings_container_grid(
                config,
                metadata,
                settings_disabled,
                window,
                cx,
            )),
        )
}

pub(in crate::app) fn settings_processing_mode_grid(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for option in output_processing_mode_options(config, metadata, settings_disabled) {
        let mode = option.mode;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            frame_choice_button(
                format!("output-mode-{}", option.mode.id()),
                option.label,
                option.is_selected,
                is_enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }

                let metadata = root.selected_source_metadata();
                if root.update_selected_config(|config| {
                    apply_processing_mode(config, metadata.as_ref(), mode)
                }) {
                    root.resolve_selected_settings_tab(metadata.as_ref());
                    cx.notify();
                }
            })),
        );
    }
    grid
}

pub(in crate::app) fn settings_container_grid(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(2).gap_2();
    for option in output_container_options(config, metadata, settings_disabled) {
        let container = option.container;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            frame_choice_button(
                format!("output-container-{container}"),
                container.as_str(),
                option.is_selected,
                is_enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }

                let metadata = root.selected_source_metadata();
                let changed = root.update_selected_config(|config| {
                    apply_output_container(config, &container)
                        | normalize_output_config(config, metadata.as_ref())
                });
                if changed {
                    root.resolve_selected_settings_tab(metadata.as_ref());
                    cx.notify();
                }
            })),
        );
    }
    grid
}

pub(in crate::app) fn settings_output_name_field(
    output_name: &str,
    disabled: bool,
    output_name_focus: Option<&FocusHandle>,
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
        window,
        cx,
    )
}
