//! Keep encoder statistics private and owned by Frame across worker termination.
use std::ffi::OsStr;
use std::io;
use std::path::Path;

use tempfile::TempDir;

pub(super) const ENVIRONMENT: &str = "FRAME_PASS_DIRECTORY";

pub(super) fn for_worker<'a>(
    program: &Path,
    mut args: impl Iterator<Item = &'a OsStr>,
) -> io::Result<Option<TempDir>> {
    if crate::runtime_binaries::worker_subcommand(program, "convert").is_none()
        || args.next() != Some(OsStr::new("convert"))
    {
        return Ok(None);
    }
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if arg == "-frame_two_pass" && args.peek() == Some(&OsStr::new("1")) {
            let mut builder = tempfile::Builder::new();
            builder.prefix("frame-two-pass-");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                builder.permissions(std::fs::Permissions::from_mode(0o700));
            }
            return builder.tempdir().map(Some);
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_workers_receive_unique_directories_and_keep_reference_programs_untouched() {
        let args = ["convert", "-frame_two_pass", "1", "-n", "final.mp4"];
        let create =
            |program: &str| for_worker(Path::new(program), args.iter().map(OsStr::new)).unwrap();
        let first = create("frame-media").unwrap();
        let second = create("frame-media.exe").unwrap();
        assert!(first.path().is_absolute());
        assert_ne!(first.path(), second.path());
        assert!(create("ffmpeg").is_none());
        for args in [
            &["convert", "-n"][..],
            &["convert", "-frame_two_pass", "0"],
            &["preview", "-frame_two_pass", "1"],
        ] {
            assert!(
                for_worker(Path::new("frame-media"), args.iter().map(OsStr::new))
                    .unwrap()
                    .is_none()
            );
        }
        let path = first.path().to_owned();
        std::fs::write(path.join("stats.cutree"), b"encoder data").unwrap();
        drop(first);
        assert!(!path.exists());
        assert!(second.path().exists());
    }
}
