mod capabilities;
mod conversion;
mod dialog;
use std::time::Duration;
use tauri::window::{Color, EffectState};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_store::Builder as StoreBuilder;
use tokio::time::sleep;

#[tauri::command]
async fn close_splash(window: tauri::Window) {
    if let Some(splash) = window.get_webview_window("splash")
        && let Err(error) = splash.close()
    {
        eprintln!("Failed to close splash window: {}", error);
    }

    if let Some(main) = window.get_webview_window("main") {
        if let Err(error) = main.show() {
            eprintln!("Failed to show main window: {}", error);
        }
    } else {
        eprintln!("Main window is not available while closing splash");
    }
}

#[cfg(target_os = "macos")]
fn apply_window_effect(window: &tauri::WebviewWindow) {
    use tauri::window::{Effect, EffectsBuilder};

    window
        .set_effects(
            EffectsBuilder::new()
                .effect(Effect::HudWindow)
                .state(EffectState::Active)
                .radius(16.0)
                .build(),
        )
        .unwrap_or_else(|error| eprintln!("Failed to apply macOS window effect: {}", error));
}

#[cfg(target_os = "windows")]
fn apply_window_effect(window: &tauri::WebviewWindow) {
    use tauri::window::{Effect, EffectsBuilder};

    window
        .set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build())
        .unwrap_or_else(|error| eprintln!("Failed to apply Windows window effect: {}", error));
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn apply_window_effect(_window: &tauri::WebviewWindow) {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("Frame")
                    .inner_size(1200.0, 800.0)
                    .min_inner_size(1200.0, 800.0)
                    .resizable(true)
                    .fullscreen(false)
                    .decorations(false)
                    .visible(false)
                    .background_color(Color(0, 0, 0, 0))
                    .transparent(true);

            let window = builder.build()?;

            apply_window_effect(&window);
            {
                let event_window = window.clone();
                window.on_window_event(move |event| {
                    if matches!(event, WindowEvent::Focused(_)) {
                        let target = event_window.clone();
                        tauri::async_runtime::spawn(async move {
                            sleep(Duration::from_millis(10)).await;
                            apply_window_effect(&target);
                        });
                    }
                    if let WindowEvent::CloseRequested { .. } = event {
                        event_window.app_handle().exit(0);
                    }
                });
            }

            let splash = WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("splash".into()))
                .title("Splash")
                .inner_size(300.0, 300.0)
                .resizable(false)
                .decorations(false)
                .always_on_top(true)
                .transparent(true)
                .background_color(Color(0, 0, 0, 0))
                .visible(false)
                .build()?;

            apply_window_effect(&splash);

            #[cfg(target_os = "macos")]
            {
                match WebviewWindowBuilder::new(
                    app,
                    "dialog-host",
                    WebviewUrl::App("dialog-host.html".into()),
                )
                .title("Dialog Host")
                .inner_size(1.0, 1.0)
                .resizable(false)
                .decorations(false)
                .fullscreen(false)
                .visible(false)
                .parent(&window)
                {
                    Ok(dialog_builder) => {
                        match dialog_builder
                            .transparent(true)
                            .background_color(Color(0, 0, 0, 0))
                            .skip_taskbar(true)
                            .shadow(false)
                            .build()
                        {
                            Ok(dialog_host) => {
                                let _ = dialog_host.hide();
                            }
                            Err(error) => {
                                eprintln!("Failed to build macOS dialog host window: {}", error);
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!(
                            "Failed to configure macOS dialog host parent window: {}",
                            error
                        );
                    }
                }
            }

            app.manage(conversion::ConversionManager::new(app.handle().clone()));

            Ok(())
        })
        .plugin(tauri_plugin_prevent_default::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(StoreBuilder::new().build())
        .invoke_handler(tauri::generate_handler![
            conversion::commands::queue_conversion,
            conversion::commands::pause_conversion,
            conversion::commands::resume_conversion,
            conversion::commands::cancel_conversion,
            conversion::commands::probe_media,
            conversion::commands::get_max_concurrency,
            conversion::commands::set_max_concurrency,
            capabilities::get_available_encoders,
            dialog::open_native_file_dialog,
            dialog::ask_native_dialog,
            close_splash,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
