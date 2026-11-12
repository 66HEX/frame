use super::{ConversionProcessController, StagedOutput};
use frame_core::error::ConversionError;

#[derive(Debug, Eq, PartialEq)]
pub enum PublicationDecision {
    Published,
    Cancelled,
}

pub struct PublicationReport {
    pub result: Result<PublicationDecision, ConversionError>,
    pub diagnostics: Vec<String>,
}

/// Publishes a sealed, verified output after a controller-serialized cutoff.
/// Cleanup completes before callers receive diagnostics or emit terminal events.
pub fn publish_staged_output(
    stage: StagedOutput,
    id: &str,
    controller: &ConversionProcessController,
) -> PublicationReport {
    let result = (|| {
        let Some(cutoff) = controller.begin_publication(id, false)? else {
            return Ok(PublicationDecision::Cancelled);
        };
        // No caller callback may interleave publication and its recorded decision.
        let publication = stage.publish();
        cutoff.finish()?;
        publication.map_err(|error| {
            ConversionError::Worker(format!(
                "Could not publish the completed export: {error}. Choose another output name or folder."
            ))
        })?;
        Ok(PublicationDecision::Published)
    })();
    PublicationReport {
        result,
        diagnostics: stage.cleanup(),
    }
}
