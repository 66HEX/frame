//! Example Prelude
//!
//! Common helpers for GPUI examples. Import with:
//!
//! ```ignore
//! #[path = "../shared/prelude.rs"]
//! mod example_prelude;
//! use example_prelude::init_example;
//! ```

use gpui::{App, KeyBinding, Menu, MenuItem, SharedString, actions};

actions!(example, [Quit, CloseWindow]);

/// Sets up common example boilerplate:
/// - Activates the application
/// - Sets up an app menu with the example name and a Quit action (cmd-q)
/// - Configures the app to quit when all windows are closed
pub fn init_example(cx: &mut App, name: impl Into<SharedString>) {
    // Bring the example window to the front
    cx.activate(true);

    // Define the quit action...
    cx.on_action(|_: &Quit, cx| cx.quit());
    // ...then bind it to cmd+q
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

    // Set up an app menu with the example name and a Quit action (cmd-q)
    cx.set_menus(vec![Menu {
        name: name.into(),
        items: vec![MenuItem::action("Quit", Quit)],
        disabled: false,
    }]);

    // Quit the app when all windows are closed
    cx.on_window_closed(|cx, _| {
        if cx.windows().is_empty() {
            cx.quit();
        }
    })
    .detach();
}
