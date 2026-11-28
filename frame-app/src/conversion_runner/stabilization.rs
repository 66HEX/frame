//! Checks optional libav filters required by the selected export.

use super::controller::ConversionProcessController;
use crate::capabilities::detect_available_filters_with_control;
use frame_core::{error::ConversionError, events::ConversionEvent, types::ConversionConfig};

pub(super) fn check_with_control(
    config: &ConversionConfig,
    executable: &str,
    id: &str,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<bool, ConversionError> {
    let result = check_support(config, executable, || controller.is_cancelled(id));
    if controller.take_cancelled(id)? {
        super::runner::emit_cancelled_task(id, emit);
        return Ok(false);
    }
    result?;
    Ok(true)
}

fn check_support(
    config: &ConversionConfig,
    executable: &str,
    cancelled: impl Fn() -> bool,
) -> Result<(), ConversionError> {
    let subtitles = !frame_core::utils::is_audio_only_container(&config.container)
        && config
            .subtitle_burn_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty());
    if !config.video_filters.stabilization_enabled && !subtitles {
        return Ok(());
    }
    let available =
        detect_available_filters_with_control(executable, cancelled).map_err(|error| {
            ConversionError::InvalidInput(format!(
                "Could not verify export filter support. Try again before exporting. {error}"
            ))
        })?;
    if subtitles && !available.subtitles {
        return Err(ConversionError::InvalidInput(
            "This media runtime does not support subtitle burn-in. Remove the burn-in subtitle file before exporting.".into(),
        ));
    }
    if config.video_filters.stabilization_enabled && !available.deshake {
        return Err(ConversionError::InvalidInput(
            "This build of Frame does not support video stabilization. Turn off Stabilize video before exporting.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
