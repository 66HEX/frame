use crate::settings::model::{
    ConversionConfig, METADATA_FIELDS, METADATA_MODES, MetadataField, MetadataFieldOption,
    MetadataMode, MetadataModeOption, SourceKind, SourceMetadata,
};
use crate::settings::rules::source_kind_for;

#[must_use]
pub fn metadata_mode_options(config: &ConversionConfig, disabled: bool) -> [MetadataModeOption; 3] {
    METADATA_MODES.map(|mode| MetadataModeOption {
        mode,
        label: mode.label(),
        is_selected: config.metadata.mode == mode,
        is_disabled: disabled,
    })
}

#[must_use]
pub fn metadata_field_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> Vec<MetadataFieldOption> {
    let fields = if frame_core::container::is_transport_stream_container(&config.container) {
        vec![MetadataField::ServiceName, MetadataField::ServiceProvider]
    } else {
        metadata_fields_for_source(metadata)
    };
    fields
        .iter()
        .copied()
        .map(|field| MetadataFieldOption {
            field,
            id: field.id(),
            label: field.label(),
            value: metadata_field_value(config, field)
                .unwrap_or_default()
                .to_string(),
            placeholder: metadata_field_placeholder(config, metadata, field),
            is_disabled: disabled,
        })
        .collect()
}

#[must_use]
pub fn metadata_mode_description(config: &ConversionConfig) -> &'static str {
    if frame_core::container::is_transport_stream_container(&config.container) {
        return match config.metadata.mode {
            MetadataMode::Clean => {
                "Removes source tags and writes the neutral MPEG-TS service identity required by the muxer: Service01 / Frame."
            }
            MetadataMode::Replace => {
                "MPEG-TS requires both service identity values. Empty Service name and Service provider fields are replaced with Service01 and Frame."
            }
            MetadataMode::Preserve => config.metadata.mode.description(),
        };
    }

    config.metadata.mode.description()
}

#[must_use]
pub fn metadata_fields_for_source(metadata: Option<&SourceMetadata>) -> Vec<MetadataField> {
    let is_image = source_kind_for(metadata) == SourceKind::Image;
    METADATA_FIELDS
        .iter()
        .copied()
        .filter(|field| !is_image || field.visible_for_image())
        .collect()
}

#[must_use]
pub fn metadata_field_value(config: &ConversionConfig, field: MetadataField) -> Option<&str> {
    match field {
        MetadataField::Title => config.metadata.title.as_deref(),
        MetadataField::Artist => config.metadata.artist.as_deref(),
        MetadataField::Album => config.metadata.album.as_deref(),
        MetadataField::Genre => config.metadata.genre.as_deref(),
        MetadataField::Date => config.metadata.date.as_deref(),
        MetadataField::Comment => config.metadata.comment.as_deref(),
        MetadataField::ServiceName => config.metadata.service_name.as_deref(),
        MetadataField::ServiceProvider => config.metadata.service_provider.as_deref(),
    }
}

#[must_use]
pub fn metadata_field_placeholder(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    field: MetadataField,
) -> String {
    if config.metadata.mode != MetadataMode::Preserve {
        return String::new();
    }

    if field == MetadataField::ServiceName {
        return metadata
            .and_then(|value| value.transport_stream.as_ref())
            .and_then(|value| value.service_name.clone())
            .unwrap_or_else(|| "Leave empty to keep source service name".to_string());
    }
    if field == MetadataField::ServiceProvider {
        return metadata
            .and_then(|value| value.transport_stream.as_ref())
            .and_then(|value| value.service_provider.clone())
            .unwrap_or_else(|| "Leave empty to keep source service provider".to_string());
    }

    metadata
        .and_then(|metadata| metadata.tags.as_ref())
        .and_then(|tags| tags.value(field))
        .filter(|value| !value.trim().is_empty())
        .map_or_else(
            || "Leave empty to keep original".to_string(),
            ToString::to_string,
        )
}
