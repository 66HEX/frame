use super::*;

pub fn init_app(cx: &mut App, name: impl Into<SharedString>) {
    cx.activate(true);
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
        ..Default::default()
    }
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
