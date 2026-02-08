use std::path::PathBuf;

use serde::Deserialize;
use tauri::{Manager, Runtime, State, WebviewWindow, Window, command};
use tauri_plugin_dialog::{Dialog, FileDialogBuilder};
use tauri_plugin_fs::FsExt;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeFileDialogOptions {
    pub title: Option<String>,
    #[serde(default)]
    pub filters: Vec<DialogFilter>,
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub directory: bool,
    pub default_path: Option<PathBuf>,
    #[serde(default)]
    pub recursive: bool,
}

#[cfg(mobile)]
fn set_default_path<R: Runtime>(
    mut dialog_builder: FileDialogBuilder<R>,
    default_path: PathBuf,
) -> FileDialogBuilder<R> {
    if let Some(file_name) = default_path.file_name() {
        dialog_builder = dialog_builder.set_file_name(file_name.to_string_lossy());
    }
    dialog_builder
}

#[cfg(desktop)]
fn set_default_path<R: Runtime>(
    mut dialog_builder: FileDialogBuilder<R>,
    default_path: PathBuf,
) -> FileDialogBuilder<R> {
    let default_path: PathBuf = default_path.components().collect();
    if default_path.is_file() || !default_path.exists() {
        if let (Some(parent), Some(file_name)) = (default_path.parent(), default_path.file_name()) {
            if parent.components().count() > 0 {
                dialog_builder = dialog_builder.set_directory(parent);
            }
            dialog_builder = dialog_builder.set_file_name(file_name.to_string_lossy());
        } else {
            dialog_builder = dialog_builder.set_directory(default_path);
        }
        dialog_builder
    } else {
        dialog_builder.set_directory(default_path)
    }
}

#[command]
pub async fn open_native_file_dialog<R: Runtime>(
    window: Window<R>,
    dialog: State<'_, Dialog<R>>,
    options: NativeFileDialogOptions,
) -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    let dialog_host = prepare_dialog_host(&window);

    let mut dialog_builder = dialog.file();
    if let Some(title) = options.title.clone() {
        dialog_builder = dialog_builder.set_title(title);
    }
    if let Some(default_path) = options.default_path.clone() {
        dialog_builder = set_default_path(dialog_builder, default_path);
    }
    for filter in options.filters.iter() {
        let extensions: Vec<&str> = filter.extensions.iter().map(|s| &**s).collect();
        dialog_builder = dialog_builder.add_filter(filter.name.clone(), &extensions);
    }

    #[cfg(target_os = "macos")]
    if let Some(host) = dialog_host.as_ref() {
        dialog_builder = dialog_builder.set_parent(host);
    }

    let mut selections: Vec<String> = Vec::new();

    if options.directory {
        #[cfg(desktop)]
        {
            let tauri_scope = window.state::<tauri::scope::Scopes>();

            if options.multiple {
                if let Some(folders) = dialog_builder.blocking_pick_folders() {
                    for folder in folders {
                        if let Ok(path) = folder.into_path() {
                            if let Some(scope) = window.try_fs_scope() {
                                scope
                                    .allow_directory(&path, options.recursive)
                                    .map_err(|e| e.to_string())?;
                            }
                            tauri_scope
                                .allow_directory(&path, options.directory)
                                .map_err(|e| e.to_string())?;
                            selections.push(path.to_string_lossy().into_owned());
                        }
                    }
                }
            } else if let Some(folder) = dialog_builder.blocking_pick_folder() {
                if let Ok(path) = folder.into_path() {
                    if let Some(scope) = window.try_fs_scope() {
                        scope
                            .allow_directory(&path, options.recursive)
                            .map_err(|e| e.to_string())?;
                    }
                    tauri_scope
                        .allow_directory(&path, options.directory)
                        .map_err(|e| e.to_string())?;
                    selections.push(path.to_string_lossy().into_owned());
                }
            }
        }
        #[cfg(mobile)]
        {
            let _ = window;
            let _ = dialog;
            return Err("Folder picker is not supported on this platform".into());
        }
    } else if options.multiple {
        let tauri_scope = window.state::<tauri::scope::Scopes>();
        if let Some(files) = dialog_builder.blocking_pick_files() {
            for file in files {
                if let Ok(path) = file.into_path() {
                    if let Some(scope) = window.try_fs_scope() {
                        scope.allow_file(&path).map_err(|e| e.to_string())?;
                    }
                    tauri_scope.allow_file(&path).map_err(|e| e.to_string())?;
                    selections.push(path.to_string_lossy().into_owned());
                }
            }
        }
    } else if let Some(file) = dialog_builder.blocking_pick_file() {
        let tauri_scope = window.state::<tauri::scope::Scopes>();
        if let Ok(path) = file.into_path() {
            if let Some(scope) = window.try_fs_scope() {
                scope.allow_file(&path).map_err(|e| e.to_string())?;
            }
            tauri_scope.allow_file(&path).map_err(|e| e.to_string())?;
            selections.push(path.to_string_lossy().into_owned());
        }
    }

    #[cfg(target_os = "macos")]
    cleanup_dialog_host(&window, dialog_host);

    Ok(selections)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAskDialogOptions {
    pub title: Option<String>,
    pub message: String,
    pub kind: Option<String>,
    pub ok_label: Option<String>,
    pub cancel_label: Option<String>,
}

#[command]
pub async fn ask_native_dialog<R: Runtime>(
    window: Window<R>,
    dialog: State<'_, Dialog<R>>,
    options: NativeAskDialogOptions,
) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    let dialog_host = prepare_dialog_host(&window);

    let mut dialog_builder = dialog.message(options.message);

    if let Some(title) = options.title {
        dialog_builder = dialog_builder.title(title);
    }

    if let Some(kind) = options.kind {
        let message_kind = match kind.as_str() {
            "info" => tauri_plugin_dialog::MessageDialogKind::Info,
            "warning" => tauri_plugin_dialog::MessageDialogKind::Warning,
            "error" => tauri_plugin_dialog::MessageDialogKind::Error,
            _ => tauri_plugin_dialog::MessageDialogKind::Info,
        };
        dialog_builder = dialog_builder.kind(message_kind);
    }

    match (options.ok_label, options.cancel_label) {
        (Some(ok), Some(cancel)) => {
            dialog_builder = dialog_builder.buttons(
                tauri_plugin_dialog::MessageDialogButtons::OkCancelCustom(ok, cancel),
            );
        }
        (Some(ok), None) => {
            dialog_builder =
                dialog_builder.buttons(tauri_plugin_dialog::MessageDialogButtons::OkCustom(ok));
        }
        (None, Some(cancel)) => {
            dialog_builder = dialog_builder.buttons(
                tauri_plugin_dialog::MessageDialogButtons::OkCancelCustom("Ok".to_string(), cancel),
            );
        }
        (None, None) => {}
    }

    #[cfg(target_os = "macos")]
    if let Some(host) = dialog_host.as_ref() {
        dialog_builder = dialog_builder.parent(host);
    }

    let result = dialog_builder.blocking_show();

    #[cfg(target_os = "macos")]
    cleanup_dialog_host(&window, dialog_host);

    Ok(result)
}

#[cfg(target_os = "macos")]
fn prepare_dialog_host<R: Runtime>(window: &Window<R>) -> Option<WebviewWindow<R>> {
    let app_handle = window.app_handle();
    let dialog_host = app_handle.get_webview_window("dialog-host");

    if let Some(host) = dialog_host.as_ref() {
        let _ = host.set_shadow(false);
        let _ = host.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(1, 1)));
    }

    dialog_host
}

#[cfg(target_os = "macos")]
fn cleanup_dialog_host<R: Runtime>(window: &Window<R>, host: Option<WebviewWindow<R>>) {
    if let Some(dialog_window) = host {
        let _ = dialog_window.hide();
    }
    let _ = window.set_focus();
}
