use super::{
    ClickEvent, Context, ConversionConfig, FocusHandle, FrameRoot, FrameTextInputKind,
    FrameTextInputSpec, MetadataField, ParentElement, SettingsMetadataInputFocuses, SourceMetadata,
    StatefulInteractiveElement, Styled, Window, apply_metadata_mode, div, frame_choice_button,
    frame_text_input, metadata_field_options, metadata_mode_description, metadata_mode_options,
    settings_field_label, settings_hint_text, settings_section, theme,
};

pub(in crate::app) fn settings_metadata_tab(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    focuses: SettingsMetadataInputFocuses<'_>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut content = div().flex().flex_col().gap_4().child(
        settings_section("Metadata mode", palette)
            .child(settings_metadata_mode_grid(
                config,
                settings_disabled,
                palette,
                window,
                cx,
            ))
            .child(settings_hint_text(
                metadata_mode_description(config),
                palette,
            )),
    );

    if config.metadata.mode != crate::settings::MetadataMode::Clean {
        content = content.child(settings_section("Metadata fields", palette).child(
            settings_metadata_fields(
                config,
                metadata,
                settings_disabled,
                focuses,
                palette,
                window,
                cx,
            ),
        ));
    }

    content
}

fn settings_metadata_mode_grid(
    config: &ConversionConfig,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(3).gap_2();
    for option in metadata_mode_options(config, settings_disabled) {
        let mode = option.mode;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            frame_choice_button(
                format!("metadata-mode-{}", mode.id()),
                option.label,
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
                if root.update_selected_config(|config| apply_metadata_mode(config, mode)) {
                    cx.notify();
                }
            })),
        );
    }

    grid
}

fn settings_metadata_fields(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    settings_disabled: bool,
    focuses: SettingsMetadataInputFocuses<'_>,
    palette: &'static theme::ThemePalette,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    let mut fields = div().flex().flex_col().gap_3();
    for option in metadata_field_options(config, metadata, settings_disabled) {
        let value = option.value;
        let placeholder = option.placeholder;
        let field = option.field;
        fields = fields.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_field_label(option.label, palette))
                .child(frame_text_input(
                    FrameTextInputSpec {
                        id: metadata_field_input_id(field),
                        value: &value,
                        placeholder: &placeholder,
                        disabled: option.is_disabled,
                        focus: metadata_field_focus(field, focuses),
                        kind: metadata_field_input_kind(field),
                    },
                    palette,
                    window,
                    cx,
                )),
        );
    }

    fields
}

const fn metadata_field_input_id(field: MetadataField) -> &'static str {
    match field {
        MetadataField::Title => "metadata-title-field",
        MetadataField::Artist => "metadata-artist-field",
        MetadataField::Album => "metadata-album-field",
        MetadataField::Genre => "metadata-genre-field",
        MetadataField::Date => "metadata-date-field",
        MetadataField::Comment => "metadata-comment-field",
        MetadataField::ServiceName => "metadata-service-name-field",
        MetadataField::ServiceProvider => "metadata-service-provider-field",
    }
}

const fn metadata_field_input_kind(field: MetadataField) -> FrameTextInputKind {
    match field {
        MetadataField::Title => FrameTextInputKind::MetadataTitle,
        MetadataField::Artist => FrameTextInputKind::MetadataArtist,
        MetadataField::Album => FrameTextInputKind::MetadataAlbum,
        MetadataField::Genre => FrameTextInputKind::MetadataGenre,
        MetadataField::Date => FrameTextInputKind::MetadataDate,
        MetadataField::Comment => FrameTextInputKind::MetadataComment,
        MetadataField::ServiceName => FrameTextInputKind::MetadataServiceName,
        MetadataField::ServiceProvider => FrameTextInputKind::MetadataServiceProvider,
    }
}

const fn metadata_field_focus(
    field: MetadataField,
    focuses: SettingsMetadataInputFocuses<'_>,
) -> Option<&FocusHandle> {
    match field {
        MetadataField::Title => focuses.title,
        MetadataField::Artist => focuses.artist,
        MetadataField::Album => focuses.album,
        MetadataField::Genre => focuses.genre,
        MetadataField::Date => focuses.date,
        MetadataField::Comment => focuses.comment,
        MetadataField::ServiceName => focuses.service_name,
        MetadataField::ServiceProvider => focuses.service_provider,
    }
}
