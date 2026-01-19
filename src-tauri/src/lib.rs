mod conversion;
use tauri::Manager;
use tauri_plugin_store::Builder as StoreBuilder;
use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "macos")]
            apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, Some(16.0))
                .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            app.manage(conversion::ConversionManager::new(app.handle().clone()));

            Ok(())
        })
        .plugin(tauri_plugin_prevent_default::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(StoreBuilder::new().build())
        .invoke_handler(tauri::generate_handler![
            conversion::queue_conversion,
            conversion::probe_media,
            conversion::estimate_output
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
