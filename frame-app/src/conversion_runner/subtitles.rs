//! Preflight for mandatory packet clipping of imported text subtitles.

use super::controller::ConversionProcessController;
use crate::capabilities::detect_available_bitstream_filters_with_executable;
use frame_core::error::ConversionError;
use frame_core::events::ConversionEvent;
use frame_core::trim::subtitle_trim_plan;
use frame_core::types::ConversionConfig;

/// Returns false after emitting cancellation, before the output process starts.
pub(super) fn check_subtitle_trim_with_control(
    config: &ConversionConfig,
    executable: &str,
    id: &str,
    controller: &ConversionProcessController,
    emit: &mut impl FnMut(ConversionEvent),
) -> Result<bool, ConversionError> {
    let result = check_subtitle_trim_support(config, executable, || controller.is_cancelled(id));
    // Consume the marker only once the owned helper has returned/handed off cleanup.
    if controller.take_cancelled(id)? {
        super::runner::emit_cancelled_task(id, emit);
        return Ok(false);
    }
    result?;
    Ok(true)
}

/// Verifies the filters used by the exact shared subtitle trim plan.
///
/// # Errors
///
/// Returns an error for invalid trim settings, missing filters, or an incomplete
/// runtime check. No check is made when no mapped text subtitle needs clipping.
pub fn check_subtitle_trim_support(
    config: &ConversionConfig,
    executable: &str,
    cancelled: impl Fn() -> bool,
) -> Result<(), ConversionError> {
    let Some(plan) = subtitle_trim_plan(config)? else {
        return Ok(());
    };
    let available = detect_available_bitstream_filters_with_executable(executable, cancelled)
        .map_err(|error| {
            ConversionError::InvalidInput(format!(
                "Could not verify subtitle trimming support. Try again before exporting. {error}"
            ))
        })?;
    let missing: Vec<_> = plan
        .required_bitstream_filters()
        .into_iter()
        .filter(|required| !available.iter().any(|name| name == required))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(ConversionError::InvalidInput(format!(
            "This FFmpeg cannot trim imported text subtitles. Use a build with these bitstream filters: {}.",
            missing.join(", ")
        )))
    }
}

#[cfg(test)]
mod tests;
