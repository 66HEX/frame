use crate::app_persistence::{AppPersistence, AppSettings};
use gpui::AppContext;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub mod export;

#[derive(Debug)]
pub struct QaProfile {
    directory: PathBuf,
}

impl QaProfile {
    /// # Errors
    /// Returns an error if the path is relative, already exists, or cannot be written.
    pub fn create(directory: &Path) -> io::Result<Self> {
        if !directory.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "QA directory must be absolute",
            ));
        }
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(directory)?;
        let profile = Self {
            directory: directory.to_path_buf(),
        };
        let output = directory.join("output");
        fs::create_dir(&output)?;
        profile
            .persistence()
            .save(&AppSettings {
                auto_update_check: false,
                default_output_directory: Some(output),
                ..AppSettings::default()
            })
            .map_err(io::Error::other)?;
        Ok(profile)
    }

    #[must_use]
    pub fn persistence(&self) -> AppPersistence {
        AppPersistence::from_settings_path(self.directory.join("settings.json"))
    }
}

/// # Errors
/// Returns an error when the platform cannot create the QA window.
pub fn open_qa_window(cx: &mut gpui::App, profile: &QaProfile) -> Result<(), String> {
    use crate::app::layout::{WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};
    use crate::app::root::FrameRoot;
    use gpui::{Bounds, px, size};
    let bounds = Bounds::centered(None, size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT)), cx);
    let mut options = super::runtime::frame_window_options(bounds);
    if let Some(titlebar) = options.titlebar.as_mut() {
        titlebar.title = Some("Frame — local QA".into());
    }
    cx.open_window(options, |_, cx| {
        cx.new(|cx| {
            let mut root = FrameRoot::new_with_persistence(profile.persistence());
            root.load_runtime_capabilities(cx);
            root
        })
    })
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::FrameRoot;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_SANDBOX: AtomicU64 = AtomicU64::new(0);

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "frame-qa-profile-{}-{}",
                std::process::id(),
                NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("unique sandbox should be created");
            Self(path)
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn qa_profile_rejects_a_relative_directory() {
        assert!(QaProfile::create(Path::new("relative-profile")).is_err());
    }

    #[test]
    fn qa_profile_creates_isolated_settings_and_output() {
        let sandbox = Sandbox::new();
        let directory = sandbox.0.join("run");

        let profile = QaProfile::create(&directory).expect("fresh profile should be created");

        assert!(profile.persistence().settings_path().is_file());
        let settings = profile.persistence().load().expect("settings should load");
        assert!(!settings.auto_update_check);
        assert_eq!(
            settings.default_output_directory,
            Some(directory.join("output"))
        );
        assert!(directory.join("output").is_dir());
    }

    #[test]
    fn qa_profile_rejects_an_existing_directory_without_overwriting_it() {
        let sandbox = Sandbox::new();
        let marker = sandbox.0.join("settings.json");
        fs::write(&marker, b"existing preferences").unwrap();

        assert!(QaProfile::create(&sandbox.0).is_err());
        assert_eq!(fs::read(marker).unwrap(), b"existing preferences");
    }

    #[test]
    fn qa_profile_settings_changes_stay_inside_the_new_directory() {
        let sandbox = Sandbox::new();
        let normal_settings = sandbox.0.join("settings.json");
        fs::write(&normal_settings, b"normal profile canary").unwrap();
        let profile = QaProfile::create(&sandbox.0.join("run")).unwrap();
        let persistence = profile.persistence();
        let mut root = FrameRoot::new_with_persistence(persistence.clone());
        root.max_concurrency = 2;
        root.persist_app_settings().unwrap();

        assert_eq!(fs::read(normal_settings).unwrap(), b"normal profile canary");
        assert_eq!(persistence.load().unwrap().max_concurrency, 2);
    }

    #[test]
    fn qa_build_disables_update_client_before_accessing_user_cache() {
        let result =
            crate::update_runtime::build_update_client(frame_updater::UpdateChannel::Stable);
        assert!(
            matches!(result, Err(frame_updater::UpdateError::Disabled(ref message)) if message.contains("QA build")),
            "QA builds must reject every updater entry point"
        );
    }
}
