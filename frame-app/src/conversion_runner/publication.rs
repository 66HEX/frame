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
    previously_cancelled: bool,
) -> PublicationReport {
    let result = (|| {
        let Some(cutoff) = controller.begin_publication(id, previously_cancelled)? else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn saved_child_cancellation_prevents_publication_and_allows_retry() {
        let root = std::env::temp_dir().join(format!(
            "frame-publication-saved-cancel-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let output = root.join("result.mp4");
        let controller = ConversionProcessController::default();
        controller.begin_queued_attempts(&["id".into()]).unwrap();
        controller.cancel_task("id").unwrap();
        let was_cancelled = controller.finish_task("id").unwrap();
        assert!(was_cancelled);
        assert!(!controller.is_cancelled("id"));

        let cancelled =
            publish_staged_output(sealed_stage(&output), "id", &controller, was_cancelled);
        let decision = cancelled.result.unwrap();
        let cancelled_output_existed = output.exists();
        if cancelled_output_existed {
            fs::remove_file(&output).unwrap();
        }
        controller.begin_queued_attempts(&["id".into()]).unwrap();
        let retry = publish_staged_output(sealed_stage(&output), "id", &controller, false);
        let retry_bytes = fs::read(&output).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(decision, PublicationDecision::Cancelled);
        assert!(!cancelled_output_existed);
        assert!(cancelled.diagnostics.is_empty());
        assert_eq!(retry.result.unwrap(), PublicationDecision::Published);
        assert!(retry.diagnostics.is_empty());
        assert_eq!(retry_bytes, b"verified output");
    }

    fn sealed_stage(output: &std::path::Path) -> StagedOutput {
        let mut stage = StagedOutput::create(output).unwrap();
        fs::write(stage.path(), b"verified output").unwrap();
        stage.seal().unwrap();
        stage
    }
}
