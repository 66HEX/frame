//! A private sibling output, published only after semantic verification.

use crate::capabilities::runtime::RuntimeIdentity;
use crate::conversion_runner::completion::SetupError;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct StagedOutput {
    destination: PathBuf,
    directory: PathBuf,
    directory_handle: File,
    path: PathBuf,
    file: Option<File>,
    identity: Option<RuntimeIdentity>,
    cleaned: bool,
}

impl StagedOutput {
    pub fn create(destination: &Path) -> Result<Self, SetupError> {
        let name = destination.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "missing output filename")
        })?;
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent = fs::canonicalize(parent)?;
        let destination = parent.join(name);
        let builder = DirBuilder::new();
        #[cfg(unix)]
        let builder = {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = builder;
            builder.mode(0o700);
            builder
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for _ in 0..32 {
            let directory = parent.join(format!(
                ".frame-alac-{}-{timestamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match builder.create(&directory) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
            let directory_handle =
                open_owned_path(&directory, true).map_err(|error| SetupError {
                    error,
                    // Without an owned handle, cleanup cannot prove this basename
                    // still denotes the directory just created. Leave it untouched.
                    diagnostics: vec![format!(
                        "Could not retain private output directory {}",
                        directory.display()
                    )],
                })?;
            let metadata = directory_handle.metadata().map_err(|error| SetupError {
                error,
                diagnostics: vec![format!(
                    "Could not inspect private output directory {}",
                    directory.display()
                )],
            })?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(SetupError {
                    error: io::Error::other("private output directory was replaced"),
                    diagnostics: vec![format!(
                        "Unverified directory entry left untouched: {}",
                        directory.display()
                    )],
                });
            }
            return Ok(Self {
                path: directory.join(name),
                destination,
                directory,
                directory_handle,
                file: None,
                identity: None,
                cleaned: false,
            });
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not create a private output directory",
        )
        .into())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Rechecked after launch diagnostics, before a child may create the file.
    pub fn ensure_unwritten(&self) -> io::Result<()> {
        if !self.directory_path_matches() {
            return Err(io::Error::other(
                "private output directory changed before launch",
            ));
        }
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "private output entry appeared before launch",
            )),
        }
    }

    /// Called after the final child settles, including failure cleanup. The
    /// output cannot be precreated because the actual command retains -n.
    pub fn seal(&mut self) -> io::Result<()> {
        if self.file.is_some() {
            return self.ensure_unchanged();
        }
        if !self.directory_path_matches() {
            return Err(io::Error::other("private output directory changed"));
        }
        let file = open_owned_path(&self.path, false)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(io::Error::other("staged output is not a regular file"));
        }
        let identity = RuntimeIdentity::capture_open_file(&self.path, &file)
            .ok_or_else(|| io::Error::other("could not identify staged output"))?;
        if !identity.matches_paths(std::slice::from_ref(&self.path)) {
            return Err(io::Error::other("staged output path changed while opening"));
        }
        self.file = Some(file);
        self.identity = Some(identity);
        Ok(())
    }

    pub fn ensure_unchanged(&self) -> io::Result<()> {
        let unchanged =
            self.file
                .as_ref()
                .zip(self.identity.as_ref())
                .is_some_and(|(file, identity)| {
                    RuntimeIdentity::capture_open_file(&self.path, file).as_ref() == Some(identity)
                        && identity.matches_paths(std::slice::from_ref(&self.path))
                        && self.directory_path_matches()
                });
        if unchanged {
            Ok(())
        } else {
            Err(io::Error::other(
                "staged output changed during verification",
            ))
        }
    }

    /// The controller must establish its Committing cutoff before this call.
    /// No callback may run until its success/failure decision is recorded.
    pub fn publish(&self) -> io::Result<()> {
        self.publish_using(publish_no_replace)
    }

    fn publish_using(
        &self,
        operation: impl FnOnce(&Path, &Path) -> io::Result<()>,
    ) -> io::Result<()> {
        self.ensure_unchanged()?;
        match operation(&self.path, &self.destination) {
            Ok(()) => Ok(()),
            Err(error) if native_publish_unavailable(&error) => {
                // Hard-link publication itself is atomic/no-replace. Removal of
                // our private link is cleanup after the publication decision.
                fs::hard_link(&self.path, &self.destination)
            }
            Err(error) => Err(error),
        }
    }

    pub fn cleanup(mut self) -> Vec<String> {
        self.cleanup_owned_entries()
    }

    fn cleanup_owned_entries(&mut self) -> Vec<String> {
        if self.cleaned {
            return Vec::new();
        }
        self.cleaned = true;
        let mut diagnostics = Vec::new();
        if !self.directory_path_matches() {
            diagnostics.push(format!(
                "Private output directory changed; left {} untouched",
                self.directory.display()
            ));
            return diagnostics;
        }
        match fs::symlink_metadata(&self.path) {
            Ok(metadata)
                if metadata.is_file()
                    && !metadata.file_type().is_symlink()
                    && self
                        .file
                        .as_ref()
                        .is_some_and(|file| handle_matches_path(&self.path, file)) =>
            {
                // Close the shared file handle before unlinking on Windows.
                drop(self.file.take());
                record_cleanup(fs::remove_file(&self.path), &self.path, &mut diagnostics);
            }
            Ok(_) => diagnostics.push(format!(
                "Unverified private output left untouched: {}",
                self.path.display()
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => diagnostics.push(format!("{}: {error}", self.path.display())),
        }
        // Never recursively remove unknown entries. The directory handle uses
        // delete sharing on Windows, so it can remain owned until Drop.
        record_cleanup(
            fs::remove_dir(&self.directory),
            &self.directory,
            &mut diagnostics,
        );
        diagnostics
    }

    fn directory_path_matches(&self) -> bool {
        // Canonical identity alone also matches a replacement symlink pointing
        // at the same retained inode. The pathname itself must still be a dir.
        fs::symlink_metadata(&self.directory)
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
            && handle_matches_path(&self.directory, &self.directory_handle)
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        let _ = self.cleanup_owned_entries();
    }
}

fn handle_matches_path(path: &Path, file: &File) -> bool {
    RuntimeIdentity::capture_open_file(path, file)
        .is_some_and(|identity| identity.matches_paths(&[path.to_path_buf()]))
}

fn open_owned_path(path: &Path, directory: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | if directory { libc::O_DIRECTORY } else { 0 });
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        };
        options.custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT.0
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS.0
                } else {
                    0
                },
        );
    }
    options.open(path)
}

fn record_cleanup(result: io::Result<()>, path: &Path, diagnostics: &mut Vec<String>) {
    if let Err(error) = result
        && error.kind() != io::ErrorKind::NotFound
    {
        diagnostics.push(format!("{}: {error}", path.display()));
    }
}

#[cfg(unix)]
fn native_publish_unavailable(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ENOSYS)
}

#[cfg(windows)]
fn native_publish_unavailable(error: &io::Error) -> bool {
    use windows::Win32::Foundation::ERROR_CALL_NOT_IMPLEMENTED;
    error.raw_os_error() == i32::try_from(ERROR_CALL_NOT_IMPLEMENTED.0).ok()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn publish_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let destination = CString::new(destination.as_os_str().as_bytes())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    #[cfg(target_os = "macos")]
    // SAFETY: both C strings remain alive and flags forbid replacement.
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    // SAFETY: syscall takes these documented renameat2 arguments. Avoid requiring
    // a newer libc symbol; ENOSYS alone enables the hard-link fallback.
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn publish_no_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic output publication is not supported on this platform",
    ))
}

#[cfg(windows)]
fn publish_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{MOVE_FILE_FLAGS, MoveFileExW};
    use windows::core::PCWSTR;
    let source: Vec<_> = source.as_os_str().encode_wide().chain([0]).collect();
    let destination: Vec<_> = destination.as_os_str().encode_wide().chain([0]).collect();
    if source[..source.len() - 1].contains(&0) || destination[..destination.len() - 1].contains(&0)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output path contains NUL",
        ));
    }
    // SAFETY: terminated strings live throughout the call. No REPLACE_EXISTING
    // or COPY_ALLOWED flag is supplied; both entries share the same filesystem.
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVE_FILE_FLAGS(0),
        )
    }
    .map_err(win32_publication_error)
}

#[cfg(windows)]
fn win32_publication_error(error: windows::core::Error) -> io::Error {
    let code = error.code().0;
    // windows-result's generic io conversion retains HRESULT, not the Win32
    // code. Recover FACILITY_WIN32 so AlreadyExists/ENOSYS policy stays exact.
    if code.cast_unsigned() & 0xffff_0000 == 0x8007_0000 {
        io::Error::from_raw_os_error(code & 0xffff)
    } else {
        io::Error::other(error)
    }
}

#[cfg(test)]
mod tests;
