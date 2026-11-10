use super::*;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "frame-alac-stage-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn destination(&self) -> PathBuf {
        self.0.join("result.m4a")
    }

    fn stage(&self, bytes: &[u8]) -> StagedOutput {
        let mut stage = StagedOutput::create(&self.destination()).unwrap();
        assert!(!stage.path().exists());
        fs::write(stage.path(), bytes).unwrap();
        stage.seal().unwrap();
        stage
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Every entry, including the replacement sentinels below, belongs to
        // this isolated test fixture. Product cleanup is never recursive.
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn stage_is_absent_until_child_writes_and_publishes_only_to_an_absent_destination() {
    let fixture = Fixture::new();
    let stage = fixture.stage(b"verified output");
    let directory = stage.directory.clone();
    assert_eq!(stage.path().file_name(), fixture.destination().file_name());
    let canonical_parent = fs::canonicalize(&fixture.0).unwrap();
    assert_eq!(stage.directory.parent(), Some(canonical_parent.as_path()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    stage.publish().unwrap();
    assert_eq!(fs::read(fixture.destination()).unwrap(), b"verified output");
    assert!(stage.cleanup().is_empty());
    assert!(!directory.exists());
}

#[test]
fn existing_and_racing_destinations_are_never_replaced_or_cleaned() {
    for before_creation in [false, true] {
        let fixture = Fixture::new();
        if before_creation {
            fs::write(fixture.destination(), b"sentinel").unwrap();
        }
        let stage = fixture.stage(b"private candidate");
        if !before_creation {
            fs::write(fixture.destination(), b"sentinel").unwrap();
        }
        assert_eq!(
            stage.publish().unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(fixture.destination()).unwrap(), b"sentinel");
        assert!(stage.cleanup().is_empty());
        assert_eq!(fs::read(fixture.destination()).unwrap(), b"sentinel");
    }
}

#[test]
fn failed_publication_has_a_terminal_decision_before_cleanup_callbacks() {
    use crate::conversion_runner::ConversionProcessController;
    let fixture = Fixture::new();
    let stage = fixture.stage(b"private candidate");
    fs::write(fixture.destination(), b"racing destination").unwrap();
    let controller = ConversionProcessController::default();
    controller.begin_attempt("id").unwrap();
    let owner = controller.begin_publication("id", false).unwrap().unwrap();
    let decision = stage.publish();
    owner.finish().unwrap();
    assert_eq!(decision.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    controller.cancel_task("id").unwrap(); // A post-decision cleanup/log callback.
    assert!(!controller.take_cancelled("id").unwrap());
    assert!(stage.cleanup().is_empty());
    assert_eq!(
        fs::read(fixture.destination()).unwrap(),
        b"racing destination"
    );
    controller.begin_attempt("id").unwrap();
    controller.cancel_task("id").unwrap();
    assert!(controller.take_cancelled("id").unwrap());
}

#[test]
fn replaced_stage_and_directory_are_left_untouched() {
    let fixture = Fixture::new();
    let stage = fixture.stage(b"owned original");
    let stage_path = stage.path().to_path_buf();
    fs::rename(&stage_path, fixture.0.join("original.m4a")).unwrap();
    fs::write(&stage_path, b"unowned replacement").unwrap();
    assert!(stage.ensure_unchanged().is_err());
    assert!(stage.publish().is_err());
    assert!(!stage.cleanup().is_empty());
    assert_eq!(fs::read(&stage_path).unwrap(), b"unowned replacement");
    assert!(!fixture.destination().exists());

    let stage = fixture.stage(b"second original");
    let directory = stage.directory.clone();
    fs::rename(&directory, fixture.0.join("moved-private-directory")).unwrap();
    fs::create_dir(&directory).unwrap();
    fs::write(stage.path(), b"replacement directory file").unwrap();
    let stage_path = stage.path().to_path_buf();
    assert!(stage.publish().is_err());
    assert!(!stage.cleanup().is_empty());
    assert_eq!(fs::read(stage_path).unwrap(), b"replacement directory file");
}

#[test]
fn extra_entries_survive_nonrecursive_cleanup_and_changed_contents_cannot_publish() {
    let fixture = Fixture::new();
    let stage = fixture.stage(b"owned original");
    let stage_path = stage.path().to_path_buf();
    let extra = stage.directory.join("foreign-entry");
    fs::write(&extra, b"do not remove").unwrap();
    stage.ensure_unchanged().unwrap();
    fs::write(stage.path(), b"modified after verification").unwrap();
    assert!(stage.publish().is_err());
    let diagnostics = stage.cleanup();
    assert_eq!(diagnostics.len(), 1);
    assert!(!stage_path.exists()); // Same owned inode, despite changed contents.
    assert_eq!(fs::read(extra).unwrap(), b"do not remove");
    assert!(!fixture.destination().exists());
}

#[cfg(unix)]
#[test]
fn symlink_stage_never_authorizes_publication_or_removal_of_its_target() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let mut stage = StagedOutput::create(&fixture.destination()).unwrap();
    let target = fixture.0.join("outside-private-directory");
    fs::write(&target, b"sentinel").unwrap();
    symlink(&target, stage.path()).unwrap();
    assert!(stage.seal().is_err());
    assert!(stage.publish().is_err());
    assert!(!stage.cleanup().is_empty());
    assert_eq!(fs::read(target).unwrap(), b"sentinel");
}

#[cfg(unix)]
#[test]
fn same_inode_symlink_replacements_are_not_followed_or_removed_during_cleanup() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let stage = fixture.stage(b"retained original inode");
    let original_path = stage.path().to_path_buf();
    let moved_file = fixture.0.join("moved-original.m4a");
    fs::rename(&original_path, &moved_file).unwrap();
    symlink(&moved_file, &original_path).unwrap();
    assert!(stage.publish().is_err());
    assert!(!stage.cleanup().is_empty());
    assert!(
        fs::symlink_metadata(&original_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_link(&original_path).unwrap(), moved_file);
    assert_eq!(fs::read(&moved_file).unwrap(), b"retained original inode");

    let stage = fixture.stage(b"retained original directory");
    let original_directory = stage.directory.clone();
    let child_name = stage.path().file_name().unwrap().to_owned();
    let moved_directory = fixture.0.join("moved-original-directory");
    fs::rename(&original_directory, &moved_directory).unwrap();
    symlink(&moved_directory, &original_directory).unwrap();
    assert!(stage.publish().is_err());
    assert!(!stage.cleanup().is_empty());
    assert!(
        fs::symlink_metadata(&original_directory)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_link(original_directory).unwrap(), moved_directory);
    assert_eq!(
        fs::read(moved_directory.join(child_name)).unwrap(),
        b"retained original directory"
    );
}

#[test]
fn hard_link_fallback_is_limited_to_an_unavailable_native_primitive() {
    let fixture = Fixture::new();
    let stage = fixture.stage(b"verified fallback");
    #[cfg(unix)]
    let unavailable = libc::ENOSYS;
    #[cfg(windows)]
    let unavailable =
        i32::try_from(windows::Win32::Foundation::ERROR_CALL_NOT_IMPLEMENTED.0).unwrap();
    stage
        .publish_using(|_, _| Err(io::Error::from_raw_os_error(unavailable)))
        .unwrap();
    assert_eq!(
        fs::read(fixture.destination()).unwrap(),
        b"verified fallback"
    );
    assert!(stage.cleanup().is_empty());
    assert_eq!(
        fs::read(fixture.destination()).unwrap(),
        b"verified fallback"
    );

    let fixture = Fixture::new();
    let stage = fixture.stage(b"unsupported filesystem");
    assert!(
        stage
            .publish_using(|_, _| Err(io::ErrorKind::Unsupported.into()))
            .is_err()
    );
    assert!(!fixture.destination().exists());
    assert!(stage.cleanup().is_empty());
}

#[cfg(windows)]
#[test]
fn windows_native_errors_preserve_destination_collision_and_fallback_classification() {
    use windows::Win32::Foundation::{
        ERROR_ALREADY_EXISTS, ERROR_CALL_NOT_IMPLEMENTED, ERROR_NOT_SUPPORTED,
    };
    use windows::core::{Error, HRESULT};
    let collision =
        win32_publication_error(Error::from(HRESULT::from_win32(ERROR_ALREADY_EXISTS.0)));
    assert_eq!(collision.kind(), io::ErrorKind::AlreadyExists);
    assert!(!native_publish_unavailable(&collision));
    let unavailable = win32_publication_error(Error::from(HRESULT::from_win32(
        ERROR_CALL_NOT_IMPLEMENTED.0,
    )));
    assert!(native_publish_unavailable(&unavailable));
    let unsupported =
        win32_publication_error(Error::from(HRESULT::from_win32(ERROR_NOT_SUPPORTED.0)));
    assert!(!native_publish_unavailable(&unsupported));
}

#[cfg(unix)]
#[test]
fn staged_paths_preserve_non_utf8_os_components() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let fixture = Fixture::new();
    let destination = fixture.0.join(OsString::from_vec(b"raw-\xff.m4a".to_vec()));
    // Some filesystems (including APFS) cannot represent arbitrary Unix bytes.
    let probe = match File::create(&destination) {
        Ok(file) => file,
        Err(error) if matches!(error.raw_os_error(), Some(libc::EILSEQ | libc::EINVAL)) => return,
        Err(error) => panic!("create non-UTF8 fixture: {error}"),
    };
    drop(probe);
    fs::remove_file(&destination).unwrap();
    let mut stage = StagedOutput::create(&destination).unwrap();
    fs::write(stage.path(), b"raw OS path").unwrap();
    stage.seal().unwrap();
    stage.publish().unwrap();
    assert_eq!(fs::read(destination).unwrap(), b"raw OS path");
    assert!(stage.cleanup().is_empty());
}
