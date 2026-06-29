use super::*;

pub fn init_app(cx: &mut App, name: impl Into<SharedString>) {
    cx.activate(true);
    cx.set_text_rendering_mode(TextRenderingMode::PlatformDefault);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new(
            "backspace",
            TextInputBackspace,
            Some(FRAME_TEXT_INPUT_CONTEXT),
        ),
        KeyBinding::new("delete", TextInputDelete, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("left", TextInputLeft, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("right", TextInputRight, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new(
            "shift-left",
            TextInputSelectLeft,
            Some(FRAME_TEXT_INPUT_CONTEXT),
        ),
        KeyBinding::new(
            "shift-right",
            TextInputSelectRight,
            Some(FRAME_TEXT_INPUT_CONTEXT),
        ),
        KeyBinding::new("home", TextInputHome, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("end", TextInputEnd, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("cmd-left", TextInputHome, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("cmd-right", TextInputEnd, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("cmd-a", TextInputSelectAll, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("cmd-c", TextInputCopy, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("cmd-x", TextInputCut, Some(FRAME_TEXT_INPUT_CONTEXT)),
        KeyBinding::new("cmd-v", TextInputPaste, Some(FRAME_TEXT_INPUT_CONTEXT)),
    ]);
    cx.set_menus(vec![Menu {
        name: name.into(),
        items: vec![MenuItem::action("Quit", Quit)],
        disabled: false,
    }]);
    cx.on_window_closed(|cx, _| {
        if cx.windows().is_empty() {
            cx.quit();
        }
    })
    .detach();
}

pub fn open_frame_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT)), cx);
    cx.open_window(frame_window_options(bounds), |_, cx| {
        cx.new(|cx| {
            let mut root = FrameRoot::new_with_platform_persistence();
            root.load_runtime_capabilities(cx);
            root.startup_update_check(cx);
            root
        })
    })
    .expect("failed to open Frame GPUI window");
}

pub fn frame_window_options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some(FRAME_APP_ID.to_owned()),
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        icon: frame_window_icon(),
        ..Default::default()
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn frame_window_icon() -> Option<std::sync::Arc<image::RgbaImage>> {
    use std::{io::Cursor, sync::LazyLock};

    static APP_ICON: LazyLock<Option<std::sync::Arc<image::RgbaImage>>> = LazyLock::new(|| {
        const BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/app_icon.png"));
        image::ImageReader::new(Cursor::new(BYTES))
            .with_guessed_format()
            .ok()?
            .decode()
            .ok()
            .map(image::DynamicImage::into_rgba8)
            .map(std::sync::Arc::new)
    });

    APP_ICON.as_ref().cloned()
}

#[cfg(target_os = "macos")]
pub(super) fn hide_native_macos_titlebar_controls(window: &Window) -> bool {
    let Ok(window_handle) = HasWindowHandle::window_handle(window) else {
        return false;
    };

    let RawWindowHandle::AppKit(appkit_handle) = window_handle.as_raw() else {
        return true;
    };

    // SAFETY: GPUI exposes a valid AppKit NSView handle for the live window.
    let ns_view = unsafe { &*appkit_handle.ns_view.as_ptr().cast::<NSView>() };
    let Some(ns_window) = ns_view.window() else {
        return false;
    };

    for button_kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        if let Some(button) = ns_window.standardWindowButton(button_kind) {
            button.setHidden(true);
        }
    }

    true
}

#[cfg(not(target_os = "macos"))]
pub(super) fn hide_native_macos_titlebar_controls(_window: &Window) -> bool {
    true
}
