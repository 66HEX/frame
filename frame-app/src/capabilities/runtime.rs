use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::bounded_process::CommandSpec;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct FileIdentity {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
    #[cfg(unix)]
    object: (u64, u64, i64, i64),
    #[cfg(windows)]
    object: (u64, [u8; 16], u64, i64),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeIdentity(Vec<FileIdentity>);

pub struct Runtime {
    pub command: CommandSpec,
    pub identity: Option<RuntimeIdentity>,
    observed_identity: Option<RuntimeIdentity>,
    identity_paths: Vec<PathBuf>,
}

impl RuntimeIdentity {
    pub fn capture_paths(paths: &[PathBuf]) -> Option<Self> {
        fingerprint(paths)
    }

    pub fn matches_paths(&self, paths: &[PathBuf]) -> bool {
        fingerprint(paths).as_ref() == Some(self)
    }

    pub fn capture_open_file(path: &Path, file: &fs::File) -> Option<Self> {
        let path = fs::canonicalize(path).ok()?;
        let metadata = file.metadata().ok()?;
        #[cfg(unix)]
        let object = unix_identity(&metadata);
        #[cfg(windows)]
        let object = {
            use std::os::windows::io::AsRawHandle;
            windows_handle_identity(
                windows::Win32::Foundation::HANDLE(file.as_raw_handle()),
                &metadata,
            )?
        };
        Some(Self(vec![FileIdentity {
            path,
            size: metadata.len(),
            modified: metadata.modified().ok()?,
            object,
        }]))
    }
}

impl Runtime {
    pub fn resolve(requested: &str) -> io::Result<Self> {
        if requested.is_empty() || requested.contains('\0') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid FFmpeg executable path",
            ));
        }
        let mut command = CommandSpec::new(requested);
        #[cfg(windows)]
        {
            let (program, interpreter) =
                crate::bounded_process::resolve_windows_program(std::ffi::OsStr::new(requested))?;
            command.argv0 = Some(requested.into());
            command.program = program;
            command.interpreter = interpreter;
        }
        #[cfg(not(windows))]
        let _ = &mut command;
        // Bare Unix names deliberately keep libc/std exec search, including its
        // PATH permissions and script semantics. Guessing a file for the cache
        // could associate a probe with a different executable.
        let explicit =
            Path::new(requested).is_absolute() || Path::new(requested).components().count() > 1;
        let mut identity_paths = if cfg!(windows) || explicit {
            vec![command.program.clone()]
        } else {
            Vec::new()
        };
        #[cfg(windows)]
        if let Some(interpreter) = &command.interpreter {
            identity_paths.push(interpreter.clone());
        }
        #[cfg(not(windows))]
        let _ = &mut identity_paths;
        let cacheable = identity_paths.len() == 1 && native_header(&command.program);
        let observed_identity = fingerprint(&identity_paths);
        let identity = cacheable.then(|| observed_identity.clone()).flatten();
        Ok(Self {
            command,
            identity,
            observed_identity,
            identity_paths,
        })
    }

    pub fn identity_unchanged(&self) -> bool {
        self.observed_identity
            .as_ref()
            .is_none_or(|identity| fingerprint(&self.identity_paths).as_ref() == Some(identity))
    }
}

fn fingerprint(paths: &[PathBuf]) -> Option<RuntimeIdentity> {
    if paths.is_empty() {
        return None;
    }
    let files = paths
        .iter()
        .map(|path| {
            let path = fs::canonicalize(path).ok()?;
            let metadata = fs::metadata(&path).ok()?;
            #[cfg(unix)]
            let object = unix_identity(&metadata);
            #[cfg(windows)]
            let object = windows_identity(&path, &metadata)?;
            Some(FileIdentity {
                path,
                size: metadata.len(),
                modified: metadata.modified().ok()?,
                object,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(RuntimeIdentity(files))
}

#[cfg(unix)]
fn unix_identity(metadata: &fs::Metadata) -> (u64, u64, i64, i64) {
    use std::os::unix::fs::MetadataExt;
    (
        metadata.dev(),
        metadata.ino(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

fn native_header(path: &Path) -> bool {
    if !fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return false;
    }
    let mut header = [0; 4];
    if fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .is_err()
    {
        // Executable-only files remain runnable; inability to inspect them only
        // prevents cache reuse. Scripts can delegate to changing dependencies.
        return false;
    }
    cfg!(target_os = "linux") && &header == b"\x7fELF"
        || cfg!(windows) && header.starts_with(b"MZ")
        || cfg!(target_os = "macos")
            && matches!(
                &header,
                b"\xfe\xed\xfa\xce"
                    | b"\xce\xfa\xed\xfe"
                    | b"\xfe\xed\xfa\xcf"
                    | b"\xcf\xfa\xed\xfe"
                    | b"\xca\xfe\xba\xbe"
                    | b"\xbe\xba\xfe\xca"
                    | b"\xca\xfe\xba\xbf"
                    | b"\xbf\xba\xfe\xca"
            )
}

#[cfg(windows)]
fn windows_identity(path: &Path, metadata: &fs::Metadata) -> Option<(u64, [u8; 16], u64, i64)> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::core::PCWSTR;
    let path: Vec<_> = path.as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: a null-terminated path and noninheritable metadata-only handle.
    // ReFS needs full FileIdInfo rather than the legacy 64-bit file index.
    let raw = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            FILE_READ_ATTRIBUTES.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }
    .ok()?;
    // SAFETY: ownership is transferred once from the successful native call.
    let owned = unsafe { OwnedHandle::from_raw_handle(raw.0) };
    windows_handle_identity(HANDLE(owned.as_raw_handle()), metadata)
}

#[cfg(windows)]
fn windows_handle_identity(
    handle: windows::Win32::Foundation::HANDLE,
    metadata: &fs::Metadata,
) -> Option<(u64, [u8; 16], u64, i64)> {
    use std::os::windows::fs::MetadataExt;
    use windows::Win32::Storage::FileSystem::{
        FILE_BASIC_INFO, FILE_ID_INFO, FileBasicInfo, FileIdInfo, GetFileInformationByHandleEx,
    };
    let mut info = FILE_ID_INFO::default();
    // SAFETY: the live handle and destination match this information class.
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&raw mut info).cast(),
            u32::try_from(size_of_val(&info)).ok()?,
        )
    }
    .ok()?;
    let mut basic = FILE_BASIC_INFO::default();
    // SAFETY: the same owned metadata handle and correctly sized destination.
    // ChangeTime also detects in-place writes followed by restoring LastWriteTime.
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileBasicInfo,
            (&raw mut basic).cast(),
            u32::try_from(size_of_val(&basic)).ok()?,
        )
    }
    .ok()?;
    Some((
        info.VolumeSerialNumber,
        info.FileId.Identifier,
        metadata.creation_time(),
        basic.ChangeTime,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "frame-runtime-identity-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn atomic_replacement_changes_identity_even_with_preserved_size_and_mtime() {
        let directory = Directory::new();
        let path = directory.0.join("runtime.exe");
        let replacement = directory.0.join("replacement.exe");
        let header: &[u8] = if cfg!(windows) {
            b"MZ\0\0data"
        } else if cfg!(target_os = "macos") {
            b"\xcf\xfa\xed\xfedata"
        } else {
            b"\x7fELFdata"
        };
        fs::write(&path, header).unwrap();
        let runtime = Runtime::resolve(path.to_str().unwrap()).unwrap();
        assert!(runtime.identity.is_some());
        let original = fs::metadata(&path).unwrap();
        fs::write(&replacement, header).unwrap();
        fs::File::options()
            .write(true)
            .open(&replacement)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(original.modified().unwrap()))
            .unwrap();
        fs::rename(&replacement, &path).unwrap();
        let after = fs::metadata(&path).unwrap();
        assert_eq!(original.len(), after.len());
        assert_eq!(original.modified().unwrap(), after.modified().unwrap());
        assert!(!runtime.identity_unchanged());
        assert_ne!(
            runtime.identity,
            Runtime::resolve(path.to_str().unwrap()).unwrap().identity
        );
    }

    #[cfg(unix)]
    #[test]
    fn scripts_and_mz_assignments_keep_the_actual_path_without_persistent_cache() {
        let directory = Directory::new();
        for (name, source) in [
            ("wrapper", b"#!/bin/sh\nexit 0\n".as_slice()),
            ("mz-wrapper", b"MZ=wrapper\nexit 0\n".as_slice()),
        ] {
            let path = directory.0.join(name);
            fs::write(&path, source).unwrap();
            let runtime = Runtime::resolve(path.to_str().unwrap()).unwrap();
            assert_eq!(runtime.command.program, path);
            assert!(runtime.identity.is_none());
            assert!(runtime.identity_unchanged());
            fs::write(&path, b"changed wrapper").unwrap();
            assert!(!runtime.identity_unchanged());
        }
    }

    #[cfg(windows)]
    #[test]
    fn in_place_rewrite_with_restored_mtime_invalidates_windows_identity() {
        use std::io::Write;
        let directory = Directory::new();
        let path = directory.0.join("runtime.exe");
        fs::write(&path, b"MZ\0\0original").unwrap();
        let runtime = Runtime::resolve(path.to_str().unwrap()).unwrap();
        let original = fs::metadata(&path).unwrap();
        // Give filesystems with coarse timestamp updates a distinct change tick.
        std::thread::sleep(std::time::Duration::from_millis(30));
        let mut file = fs::File::options().write(true).open(&path).unwrap();
        file.write_all(b"MZ\0\0modified").unwrap();
        file.set_times(fs::FileTimes::new().set_modified(original.modified().unwrap()))
            .unwrap();
        drop(file);
        let after = fs::metadata(&path).unwrap();
        assert_eq!(original.len(), after.len());
        assert_eq!(original.modified().unwrap(), after.modified().unwrap());
        assert!(!runtime.identity_unchanged());
    }

    #[cfg(unix)]
    #[test]
    fn bare_unix_name_retains_std_search_without_cache_identity() {
        let runtime = Runtime::resolve("ffmpeg").unwrap();
        assert_eq!(runtime.command.program, Path::new("ffmpeg"));
        assert!(runtime.identity.is_none());
    }

    #[test]
    fn explicit_native_test_executable_has_a_stable_identity() {
        let path = std::env::current_exe().unwrap();
        let runtime = Runtime::resolve(path.to_str().unwrap()).unwrap();
        assert!(runtime.identity.is_some());
        assert!(runtime.identity_unchanged());
    }
}
