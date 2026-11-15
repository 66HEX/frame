use super::model::MAX_BYTES;
use super::{RecoveryError, RecoverySnapshot};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct RecoveryStore {
    path: PathBuf,
    _lock: File,
    state: StoreState,
    last_written: Option<(String, u64)>,
    previous_revision: Option<(String, u64)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoreState {
    NotLoaded,
    AwaitingDecision,
    Ready,
    Blocked,
}

impl RecoveryStore {
    pub fn open(settings_path: &Path) -> Result<Self, RecoveryError> {
        Self::open_for_environment(settings_path, crate::runtime_environment::is_flatpak())
    }

    pub(super) fn open_for_environment(
        settings_path: &Path,
        flatpak: bool,
    ) -> Result<Self, RecoveryError> {
        if flatpak {
            return Err(RecoveryError::Unavailable);
        }
        let path = settings_path.with_file_name("workspace-recovery.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("lock"))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(RecoveryError::ProfileInUse),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        Ok(Self {
            path,
            _lock: lock,
            state: StoreState::NotLoaded,
            last_written: None,
            previous_revision: None,
        })
    }

    pub fn load(&mut self) -> Result<Option<RecoverySnapshot>, RecoveryError> {
        check_environment()?;
        if self.state != StoreState::NotLoaded {
            return Err(RecoveryError::Unresolved);
        }
        self.state = StoreState::Blocked;
        let snapshot = read_snapshot(&self.path)?;
        self.previous_revision = snapshot
            .as_ref()
            .map(|snapshot| (snapshot.session_id.clone(), snapshot.revision));
        self.state = StoreState::AwaitingDecision;
        Ok(snapshot)
    }

    pub fn resolve_existing(&mut self) -> Result<(), RecoveryError> {
        if self.state != StoreState::AwaitingDecision {
            return Err(RecoveryError::Unresolved);
        }
        self.state = StoreState::Ready;
        Ok(())
    }

    pub fn save(&mut self, snapshot: &RecoverySnapshot) -> Result<(), RecoveryError> {
        self.save_before_publish(snapshot, |_| Ok(()))
    }

    pub(super) fn save_before_publish(
        &mut self,
        snapshot: &RecoverySnapshot,
        before_publish: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<(), RecoveryError> {
        check_environment()?;
        if self.state != StoreState::Ready {
            return Err(RecoveryError::Unresolved);
        }
        snapshot.validate()?;
        if self
            .last_written
            .as_ref()
            .is_some_and(|(session, revision)| {
                session != &snapshot.session_id || *revision >= snapshot.revision
            })
            || self
                .previous_revision
                .as_ref()
                .is_some_and(|(session, revision)| {
                    session == &snapshot.session_id && *revision >= snapshot.revision
                })
        {
            return Err(RecoveryError::StaleRevision);
        }
        let bytes = serde_json::to_vec(snapshot)?;
        if bytes.len() > MAX_BYTES {
            return Err(RecoveryError::TooLarge);
        }
        let (temporary, mut file) = TemporarySnapshot::create(&self.path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        before_publish(&temporary.0)?;
        replace_file(&temporary.0, &self.path)?;
        #[cfg(unix)]
        if let Some(parent) = self.path.parent() {
            let _ = File::open(parent).and_then(|directory| directory.sync_all());
        }
        self.last_written = Some((snapshot.session_id.clone(), snapshot.revision));
        Ok(())
    }
}

fn check_environment() -> Result<(), RecoveryError> {
    if crate::runtime_environment::is_flatpak() {
        Err(RecoveryError::Unavailable)
    } else {
        Ok(())
    }
}

fn read_snapshot(path: &Path) -> Result<Option<RecoverySnapshot>, RecoveryError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => {
            return Err(RecoveryError::Invalid("snapshot is not a regular file"));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(RecoveryError::Invalid("snapshot is not a regular file"));
    }
    if metadata.len() > MAX_BYTES as u64 {
        return Err(RecoveryError::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(RecoveryError::TooLarge);
    }
    let snapshot: RecoverySnapshot = serde_json::from_slice(&bytes)?;
    snapshot.validate()?;
    Ok(Some(snapshot))
}

struct TemporarySnapshot(PathBuf);

impl TemporarySnapshot {
    fn create(destination: &Path) -> io::Result<(Self, File)> {
        for _ in 0..64 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = destination.with_file_name(format!(
                ".workspace-recovery-{}-{sequence}.tmp",
                std::process::id()
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => return Ok((Self(path), file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Could not reserve a recovery staging file.",
        ))
    }
}

impl Drop for TemporarySnapshot {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        },
        core::PCWSTR,
    };
    let temporary = temporary
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: Both NUL-terminated buffers remain alive for the synchronous call.
    unsafe {
        MoveFileExW(
            PCWSTR(temporary.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(io::Error::other)
}
