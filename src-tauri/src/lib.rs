mod conversion;
mod capabilities;
use tauri::window::{Color, EffectState};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_store::Builder as StoreBuilder;

#[tauri::command]
async fn close_splash(window: tauri::Window) {
    if let Some(splash) = window.get_webview_window("splash") {
        splash.close().unwrap();
    }
    window.get_webview_window("main").unwrap().show().unwrap();
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
        .expect("Unsupported platform! 'HudWindow' effect is only supported on macOS");
}

#[cfg(target_os = "windows")]
fn apply_window_effect(window: &tauri::WebviewWindow) {
    use tauri::window::{Effect, EffectsBuilder};

    window
        .set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build())
        .expect("Unsupported platform! 'Acrylic' effect is only supported on Windows");
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

            let window = builder.build().unwrap();

            apply_window_effect(&window);

            let splash = WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("splash".into()))
                .title("Splash")
                .inner_size(300.0, 300.0)
                .resizable(false)
                .decorations(false)
                .always_on_top(true)
                .transparent(true)
                .background_color(Color(0, 0, 0, 0))
                .visible(false)
                .build()
                .unwrap();

            apply_window_effect(&splash);

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
            conversion::queue_conversion,
            conversion::pause_conversion,
            conversion::resume_conversion,
            conversion::cancel_conversion,
            conversion::probe_media,
            conversion::get_max_concurrency,
            conversion::set_max_concurrency,
            capabilities::get_available_encoders,
            close_splash
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
