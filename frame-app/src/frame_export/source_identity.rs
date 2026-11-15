use crate::capabilities::runtime::RuntimeIdentity;
use frame_core::error::ConversionError;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(super) struct SourceIdentity {
    path: PathBuf,
    identity: RuntimeIdentity,
    // Retain the object across request clones so unlink/recreate cannot reuse
    // its identifier within the filesystem's timestamp resolution.
    _file: Arc<File>,
}

impl PartialEq for SourceIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.identity == other.identity
    }
}

impl Eq for SourceIdentity {}

impl SourceIdentity {
    pub(super) fn capture(path: &Path) -> Result<Self, ConversionError> {
        if !std::fs::metadata(path)?.is_file() {
            return Err(ConversionError::InvalidInput(
                "Choose a source file before exporting a frame".into(),
            ));
        }
        let path = path.to_owned();
        let file = File::open(&path)?;
        if !file.metadata()?.is_file() {
            return Err(ConversionError::InvalidInput(
                "Choose a source file before exporting a frame".into(),
            ));
        }
        let identity = RuntimeIdentity::capture_open_file(&path, &file).ok_or_else(|| {
            ConversionError::Worker("Could not identify the source frame input".into())
        })?;
        let captured = Self {
            path,
            identity,
            _file: Arc::new(file),
        };
        captured.ensure_unchanged()?;
        Ok(captured)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[expect(
        clippy::used_underscore_binding,
        reason = "the lifecycle test observes a guard retained only for ownership in production"
    )]
    fn check_deleted_source_lifecycle() {
        let folder = super::super::tests::Directory::new();
        let source = folder.0.join("source.mov");
        std::fs::write(&source, b"original object").unwrap();
        let captured = SourceIdentity::capture(&source).unwrap();
        let owner = Arc::downgrade(&captured._file);
        let retained = captured.clone();
        drop(captured);

        std::fs::remove_file(&source).unwrap();
        #[cfg(unix)]
        std::fs::write(&source, b"replacement object").unwrap();
        let mut reader = retained._file.try_clone().unwrap();
        let mut original = Vec::new();
        reader.read_to_end(&mut original).unwrap();
        assert_eq!(original, b"original object");
        assert!(retained.ensure_unchanged().is_err());

        drop(reader);
        drop(retained);
        assert!(
            owner.upgrade().is_none(),
            "last request must release its source"
        );
        #[cfg(windows)]
        std::fs::write(&source, b"replacement object").unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), b"replacement object");
    }

    #[cfg(unix)]
    #[test]
    fn cloned_identity_retains_the_unlinked_source_until_last_drop() {
        check_deleted_source_lifecycle();
    }

    #[cfg(windows)]
    #[test]
    fn cloned_identity_retains_the_delete_pending_source_until_last_drop() {
        check_deleted_source_lifecycle();
    }
}
