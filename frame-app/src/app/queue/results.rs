use crate::app::root::FrameRoot;
use gpui::{Context, PromptButton, PromptLevel, Window};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) enum ResultAction {
    Open,
    Reveal,
}

impl ResultAction {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Open => "Open result",
            Self::Reveal => "Show result in folder",
        }
    }

    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Open => "open-result",
            Self::Reveal => "reveal-result",
        }
    }

    fn dispatch(self, path: &Path, cx: &gpui::App) {
        #[cfg(test)]
        {
            tests::record_result_action(
                cx,
                match self {
                    Self::Open => "open",
                    Self::Reveal => "reveal",
                },
                path,
            );
        }
        #[cfg(not(test))]
        match self {
            Self::Open => cx.open_with_system(path),
            Self::Reveal => cx.reveal_path(path),
        }
    }
}

impl FrameRoot {
    pub(super) fn open_completed_result(
        &self,
        file_id: &str,
        action: ResultAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self
            .file_queue
            .file_by_id(file_id)
            .and_then(crate::file_queue::FileItem::completed_output_path)
            .map(PathBuf::from)
        else {
            return;
        };
        let (title, detail) = match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                action.dispatch(&path, cx);
                return;
            }
            Ok(_) => (
                "Result is not a file",
                format!(
                    "The saved result path is no longer a file:\n{}\nChoose Convert again to create a new result.",
                    path.display()
                ),
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
                "Result file not found",
                format!(
                    "The result may have been moved or deleted:\n{}\nChoose Convert again to create a new result.",
                    path.display()
                ),
            ),
            Err(error) => (
                "Unable to access result",
                format!(
                    "Unable to access {}: {error}\nCheck that the drive is available and you have permission to open the file.",
                    path.display()
                ),
            ),
        };
        let response = window.prompt(
            PromptLevel::Warning,
            title,
            Some(&detail),
            &[PromptButton::cancel("Cancel")],
            cx,
        );
        cx.spawn(async move |_, _| {
            let _ = response.await;
        })
        .detach();
    }
}

#[cfg(test)]
mod tests;
