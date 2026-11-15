//! Runtime support for the optional single-pass video stabilizer.

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
    if !config.video_filters.stabilization_enabled {
        return Ok(());
    }
    let available =
        detect_available_filters_with_control(executable, cancelled).map_err(|error| {
            ConversionError::InvalidInput(format!(
                "Could not verify video stabilization support. Try again before exporting. {error}"
            ))
        })?;
    if available.deshake {
        Ok(())
    } else {
        Err(ConversionError::InvalidInput(
            "This FFmpeg does not support video stabilization. Disable Stabilize video or use an FFmpeg build with the deshake filter.".into(),
        ))
    }
}

#[cfg(test)]
mod tests;
