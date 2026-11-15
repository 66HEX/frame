use super::{
    ConversionError, ConversionProcessController, SourceFrameExportOutcome,
    SourceFrameExportRequest,
};
use crate::conversion_runner::{PublicationDecision, StagedOutput, publish_staged_output};
use std::fs::OpenOptions;
use std::io::Write;

pub(super) fn publish_png(
    request: &SourceFrameExportRequest,
    controller: &ConversionProcessController,
    bytes: &[u8],
    before_publish: impl FnOnce(),
) -> Result<SourceFrameExportOutcome, ConversionError> {
    if controller.is_cancelled(&request.id) {
        return Ok(SourceFrameExportOutcome::Cancelled);
    }
    request.ensure_source_unchanged()?;
    let mut stage = StagedOutput::create(&request.output_path)
        .map_err(|setup| with_diagnostics(setup.error.into(), &setup.diagnostics))?;
    let write_result = write_stage(&mut stage, bytes, request, controller);
    if let Err(error) = write_result {
        return Err(with_diagnostics(error, &stage.cleanup()));
    }
    before_publish();
    if let Err(error) = request.ensure_source_unchanged() {
        return Err(with_diagnostics(error, &stage.cleanup()));
    }
    let report = publish_staged_output(stage, &request.id, controller, false);
    match report.result {
        Ok(PublicationDecision::Published) => {
            // Publication is already terminal. Cleanup diagnostics cannot turn a
            // successfully saved PNG into a retry that might create another file.
            for diagnostic in report.diagnostics {
                eprintln!("Source frame export cleanup: {diagnostic}");
            }
            Ok(SourceFrameExportOutcome::Published {
                path: request.output_path.clone(),
            })
        }
        Ok(PublicationDecision::Cancelled) => {
            for diagnostic in report.diagnostics {
                eprintln!("Source frame export cleanup: {diagnostic}");
            }
            Ok(SourceFrameExportOutcome::Cancelled)
        }
        Err(error) => Err(with_diagnostics(error, &report.diagnostics)),
    }
}

fn write_stage(
    stage: &mut StagedOutput,
    bytes: &[u8],
    request: &SourceFrameExportRequest,
    controller: &ConversionProcessController,
) -> Result<(), ConversionError> {
    stage.ensure_unwritten()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(stage.path())?;
    let written = (|| {
        for chunk in bytes.chunks(64 * 1024) {
            if controller.is_cancelled(&request.id) {
                return Err(std::io::Error::other("Source frame export cancelled"));
            }
            file.write_all(chunk)?;
        }
        file.sync_all()
    })();
    drop(file);
    // Seal our created entry even after a partial write, so cleanup retains the
    // same ownership checks used by ordinary native exports.
    let sealed = stage.seal();
    written?;
    sealed?;
    Ok(())
}

fn with_diagnostics(error: ConversionError, diagnostics: &[String]) -> ConversionError {
    if diagnostics.is_empty() {
        error
    } else {
        ConversionError::Worker(format!("{error}; {}", diagnostics.join("; ")))
    }
}
