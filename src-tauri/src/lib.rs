mod conversion;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_store::Builder as StoreBuilder;
#[cfg(target_os = "windows")]
use window_vibrancy::apply_acrylic;
#[cfg(target_os = "macos")]
use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy};

#[tauri::command]
async fn close_splash(window: tauri::Window) {
    if let Some(splash) = window.get_webview_window("splash") {
        splash.close().unwrap();
    }
    window.get_webview_window("main").unwrap().show().unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    const WINDOWS_ACRYLIC_COLOR: (u8, u8, u8, u8) = (19, 19, 20, 180);

    tauri::Builder::default()
        .setup(|app| {
            let mut builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("Frame")
                    .inner_size(1200.0, 800.0)
                    .min_inner_size(1200.0, 800.0)
                    .resizable(true)
                    .fullscreen(false)
                    .decorations(false)
                    .visible(false);

            #[cfg(target_os = "macos")]
            {
                builder = builder.transparent(true);
            }

            #[cfg(target_os = "windows")]
            {
                builder = builder.transparent(true);
            }

            let window = builder.build().unwrap();

            #[cfg(target_os = "macos")]
            apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, Some(16.0))
                .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            #[cfg(target_os = "windows")]
            if let Err(e) = apply_acrylic(&window, Some(WINDOWS_ACRYLIC_COLOR)) {
                eprintln!("Acrylic effect not available: {e}. Using solid background.");
            }

            let splash = WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("splash".into()))
                .title("Splash")
                .inner_size(300.0, 300.0)
                .resizable(false)
                .decorations(false)
                .always_on_top(true)
                .transparent(true)
                .visible(false)
                .build()
                .unwrap();

            #[cfg(target_os = "macos")]
            apply_vibrancy(&splash, NSVisualEffectMaterial::HudWindow, None, Some(16.0))
                .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            #[cfg(target_os = "windows")]
            if let Err(e) = apply_acrylic(&splash, Some(WINDOWS_ACRYLIC_COLOR)) {
                eprintln!("Acrylic effect not available for splash: {e}. Using solid background.");
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
            conversion::queue_conversion,
            conversion::probe_media,
            conversion::get_max_concurrency,
            conversion::set_max_concurrency,
            close_splash
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
