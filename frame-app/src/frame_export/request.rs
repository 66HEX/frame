use frame_core::error::ConversionError;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFrameExportRequest {
    pub id: String,
    pub source_id: String,
    pub source_path: PathBuf,
    pub position_microseconds: u64,
    pub video_stream_index: u32,
    pub output_path: PathBuf,
    source_identity: Option<super::source_identity::SourceIdentity>,
}

impl SourceFrameExportRequest {
    #[cfg(test)]
    pub(crate) const fn for_test(
        id: String,
        source_id: String,
        source_path: PathBuf,
        position_microseconds: u64,
        video_stream_index: u32,
        output_path: PathBuf,
    ) -> Self {
        Self {
            id,
            source_id,
            source_path,
            position_microseconds,
            video_stream_index,
            output_path,
            source_identity: None,
        }
    }

    pub fn new(
        source_id: String,
        source_path: PathBuf,
        position_microseconds: u64,
        video_stream_index: u32,
        output_directory: PathBuf,
    ) -> Result<Self, ConversionError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        super::check_environment(crate::runtime_environment::is_flatpak())?;
        if source_id.is_empty() || source_path.file_name().is_none() {
            return Err(ConversionError::InvalidInput(
                "Choose a source before exporting a frame".into(),
            ));
        }
        if position_microseconds > i64::MAX.unsigned_abs() {
            return Err(ConversionError::InvalidInput(
                "Source frame position is too large".into(),
            ));
        }
        let source_identity = super::source_identity::SourceIdentity::capture(&source_path)?;
        let output_directory = std::fs::canonicalize(output_directory)?;
        if !output_directory.is_dir() {
            return Err(ConversionError::InvalidInput(
                "Choose an output folder before exporting a frame".into(),
            ));
        }
        let mut stem = source_path
            .file_stem()
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| std::ffi::OsStr::new("source"))
            .to_owned();
        stem.push(format!("_frame_{position_microseconds}"));
        for suffix in 1..=10_000 {
            let mut name = stem.clone();
            if suffix > 1 {
                name.push(format!("_{suffix}"));
            }
            name.push(".png");
            let output_path = output_directory.join(name);
            match std::fs::symlink_metadata(&output_path) {
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            return Ok(Self {
                id: format!(
                    "source-frame-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ),
                source_id,
                source_path,
                position_microseconds,
                video_stream_index,
                output_path,
                source_identity: Some(source_identity),
            });
        }
        Err(ConversionError::InvalidInput(
            "Too many source frame exports have this name; choose another output folder".into(),
        ))
    }

    pub(super) fn ensure_source_unchanged(&self) -> Result<(), ConversionError> {
        self.source_identity
            .as_ref()
            .ok_or_else(|| {
                ConversionError::Worker(
                    "Source frame request has no captured source identity".into(),
                )
            })?
            .ensure_unchanged()
    }
}
