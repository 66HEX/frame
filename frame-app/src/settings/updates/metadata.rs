use crate::settings::model::{ConversionConfig, MetadataField, MetadataMode};

pub fn apply_metadata_mode(config: &mut ConversionConfig, mode: MetadataMode) -> bool {
    if config.metadata.mode == mode {
        return false;
    }

    config.metadata.mode = mode;
    true
}

pub fn apply_metadata_field(
    config: &mut ConversionConfig,
    field: MetadataField,
    value: &str,
) -> bool {
    let value = if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    };

    let target = match field {
        MetadataField::Title => &mut config.metadata.title,
        MetadataField::Artist => &mut config.metadata.artist,
        MetadataField::Album => &mut config.metadata.album,
        MetadataField::Genre => &mut config.metadata.genre,
        MetadataField::Date => &mut config.metadata.date,
        MetadataField::Comment => &mut config.metadata.comment,
        MetadataField::ServiceName => &mut config.metadata.service_name,
        MetadataField::ServiceProvider => &mut config.metadata.service_provider,
    };

    if *target == value {
        return false;
    }

    *target = value;
    true
}
