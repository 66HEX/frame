use crate::capabilities::runtime::RuntimeIdentity;
use frame_core::error::ConversionError;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SourceIdentity {
    path: PathBuf,
    identity: RuntimeIdentity,
}

impl SourceIdentity {
    pub(super) fn capture(path: &Path) -> Result<Self, ConversionError> {
        if !std::fs::metadata(path)?.is_file() {
            return Err(ConversionError::InvalidInput(
                "Choose a source file before exporting a frame".into(),
            ));
        }
        let path = path.to_owned();
        let identity =
            RuntimeIdentity::capture_paths(std::slice::from_ref(&path)).ok_or_else(|| {
                ConversionError::Worker("Could not identify the source frame input".into())
            })?;
        Ok(Self { path, identity })
    }

    pub(super) fn ensure_unchanged(&self) -> Result<(), ConversionError> {
        if self
            .identity
            .matches_paths(std::slice::from_ref(&self.path))
        {
            Ok(())
        } else {
            Err(ConversionError::Worker(
                "The source changed after the frame export was requested; try again".into(),
            ))
        }
    }
}
