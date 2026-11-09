//! Request-owned evidence that the final `FFmpeg` invocation reached completion.
//!
//! The bounded tail limits read memory, not the file's lifetime disk usage.
//! This trusted protocol is neither output validation nor executable authentication.

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const TAIL_LIMIT: usize = 64 * 1024;
const CREATE_ATTEMPTS: usize = 32;
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct CompletionFile {
    directory: PathBuf,
    path: PathBuf,
    file: Option<File>,
    owns_file: bool,
    cleaned: bool,
}

#[derive(Debug)]
pub(super) struct SetupError {
    pub error: io::Error,
    pub diagnostics: Vec<String>,
}

impl CompletionFile {
    pub fn create() -> Result<Self, SetupError> {
        Self::create_in(&std::env::temp_dir())
    }

    fn create_in(base: &Path) -> Result<Self, SetupError> {
        let base = if base.is_absolute() {
            base.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(SetupError::from)?
                .join(base)
        };
        let directory_builder = DirBuilder::new();
        #[cfg(unix)]
        let directory_builder = {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = directory_builder;
            builder.mode(0o700);
            builder
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for _ in 0..CREATE_ATTEMPTS {
            let directory = base.join(format!(
                "frame-completion-{}-{timestamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match directory_builder.create(&directory) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
            let mut owner = Self {
                path: directory.join("progress"),
                directory,
                file: None,
                owns_file: false,
                cleaned: false,
            };
            let mut options = OpenOptions::new();
            options.read(true).write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&owner.path) {
                Ok(file) => {
                    owner.file = Some(file);
                    owner.owns_file = true;
                    return Ok(owner);
                }
                Err(error) => {
                    return Err(SetupError {
                        error,
                        diagnostics: owner.cleanup(),
                    });
                }
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not create a unique completion directory",
        )
        .into())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn verify(&mut self) -> Result<(), String> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| "completion file is closed".to_string())?;
        let metadata = file.metadata().map_err(|error| error.to_string())?;
        if !metadata.is_file() {
            return Err("completion handle is not a regular file".to_string());
        }
        let length = metadata.len();
        let offset = length.saturating_sub(TAIL_LIMIT as u64);
        let read_length = usize::try_from(length - offset)
            .map_err(|_| "completion record is too large".to_string())?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
        let mut tail = vec![0; read_length];
        file.read_exact(&mut tail)
            .map_err(|error| error.to_string())?;
        if file.metadata().map_err(|error| error.to_string())?.len() != length {
            return Err("completion file changed while it was read".to_string());
        }
        complete_final_record(&tail, offset).map_err(str::to_string)
    }

    pub fn cleanup(mut self) -> Vec<String> {
        self.cleanup_owned_entries()
    }

    fn cleanup_owned_entries(&mut self) -> Vec<String> {
        if self.cleaned {
            return Vec::new();
        }
        self.cleaned = true;
        // A normal shared handle must close before removal, particularly on Windows.
        drop(self.file.take());
        let mut diagnostics = Vec::new();
        if self.owns_file {
            record_cleanup_error(fs::remove_file(&self.path), &self.path, &mut diagnostics);
        }
        record_cleanup_error(
            fs::remove_dir(&self.directory),
            &self.directory,
            &mut diagnostics,
        );
        diagnostics
    }
}

impl Drop for CompletionFile {
    fn drop(&mut self) {
        let _ = self.cleanup_owned_entries();
    }
}

impl From<io::Error> for SetupError {
    fn from(error: io::Error) -> Self {
        Self {
            error,
            diagnostics: Vec::new(),
        }
    }
}

fn record_cleanup_error(result: io::Result<()>, path: &Path, diagnostics: &mut Vec<String>) {
    if let Err(error) = result
        && error.kind() != io::ErrorKind::NotFound
    {
        diagnostics.push(format!("{}: {error}", path.display()));
    }
}

fn complete_final_record(tail: &[u8], offset: u64) -> Result<(), &'static str> {
    if tail.is_empty() || !tail.ends_with(b"\n") {
        return Err("terminal progress record is missing or incomplete");
    }
    let tail = if offset == 0 {
        tail
    } else {
        // Even a marker-looking first line can be the suffix of an old field.
        // Discard before UTF8 validation: the cut can also bisect an old codepoint.
        let first_newline = tail.iter().position(|byte| *byte == b'\n').unwrap();
        &tail[first_newline + 1..]
    };
    let mut record_start = (offset == 0).then_some(0);
    let mut line_start = 0;
    let mut last_line = &[][..];
    for line in tail.split_inclusive(|byte| *byte == b'\n') {
        let content = progress_line_content(line);
        let line_end = line_start + line.len();
        if line_end < tail.len() && matches!(content, b"progress=continue" | b"progress=end") {
            record_start = Some(line_end);
        }
        last_line = content;
        line_start = line_end;
    }
    if last_line != b"progress=end" {
        return Err("final progress record does not confirm completion");
    }
    let record_start = record_start.ok_or("final progress record exceeds the read limit")?;
    let record = std::str::from_utf8(&tail[record_start..])
        .map_err(|_| "final progress record is not valid UTF-8")?;
    for line in record.split_terminator('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let (key, value) = line
            .split_once('=')
            .ok_or("final progress record contains a malformed field")?;
        if key.is_empty()
            || key.chars().any(|ch| ch.is_whitespace() || ch.is_control())
            || value.contains(['\r', '\0'])
        {
            return Err("final progress record contains a malformed field");
        }
    }
    Ok(())
}

fn progress_line_content(line: &[u8]) -> &[u8] {
    let content = line.strip_suffix(b"\n").unwrap_or(line);
    content.strip_suffix(b"\r").unwrap_or(content)
}

#[cfg(test)]
mod tests;
