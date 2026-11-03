//! Module containing user-input actions that are bound by EditableText elements
use gpui::{InteractiveElement, WeakEntity, Window};
use std::{cell::RefCell, rc::Rc};

/// The key context used for EditableText element keybindings.
pub const DEFAULT_INPUT_CONTEXT: &str = "EditableText";

gpui::actions!(
    actions,
    [
        /// Blur focus from the input.
        Escape,
        /// Insert a newline at the cursor position.
        Enter,
        /// Insert a tab character at the cursor position.
        Tab,
        /// Delete the character before the cursor.
        DeleteLeft,
        /// Delete the character after the cursor.
        DeleteRight,
        /// Delete the word before the cursor.
        DeleteWordLeft,
        /// Delete the word after the cursor.
        DeleteWordRight,
        /// Delete from the cursor to the beginning of the line.
        DeleteToLineStart,
        /// Delete from the cursor to the end of the line.
        DeleteToLineEnd,
        /// Move the cursor one character to the left.
        NavLeft,
        /// Move the cursor one character to the right.
        NavRight,
        /// Move the cursor up one visual line.
        NavUp,
        /// Move the cursor down one visual line.
        NavDown,
        /// Move cursor to the start of the current line.
        NavLineStart,
        /// Move cursor to the end of the current line.
        NavLineEnd,
        /// Move cursor to the beginning of the content.
        NavDocumentStart,
        /// Move cursor to the end of the content.
        NavDocumentEnd,
        /// Move cursor one word to the left.
        NavWordLeft,
        /// Move cursor one word to the right.
        NavWordRight,
        /// Select all text content.
        SelectAll,
        /// Extend selection one character to the left.
        SelectLeft,
        /// Extend selection one character to the right.
        SelectRight,
        /// Extend selection up one visual line.
        SelectUp,
        /// Extend selection down one visual line.
        SelectDown,
        /// Extend selection to the beginning of the content.
        SelectDocumentStart,
        /// Extend selection to the end of the content.
        SelectDocumentEnd,
        /// Extend selection one word to the left.
        SelectWordLeft,
        /// Extend selection one word to the right.
        SelectWordRight,
        /// Cut selected text to clipboard.
        Cut,
        /// Copy selected text to clipboard.
        Copy,
        /// Paste from clipboard at the cursor position.
        Paste,
        /// Undo the last edit.
        Undo,
        /// Redo the last undone edit.
        Redo,
        /// Show the platform character palette.
        ShowCharacterPalette,
    ]
);

/// Creates a collection of default keystroke bindings for EditableText actions.
/// See [`ActionBindingCollection`](gpui::ActionBindingCollection) docs on how to override these bindings.
///
/// Apple keyboards dont have Home or End keys, so there are common bindings that replace those keys.
/// | Action                | All         | Linux & Windows           | MacOS           |
/// | --------------------- | ----------- | ------------------------- | --------------- |
/// | Escape                | escape      |                           |                 |
/// | Enter                 | enter       |                           |                 |
/// | Tab                   | tab         |                           |                 |
/// | DeleteLeft            | backspace   |                           |                 |
/// | DeleteRight           | delete      |                           |                 |
/// | DeleteWordLeft        |             | ctrl + backspace          | alt + backspace |
/// | DeleteWordRight       |             | ctrl + delete             | alt + delete    |
/// | DeleteToLineStart     |             | ctrl + shift + backspace  | cmd + backspace |
/// | DeleteToLineEnd       |             | ctrl + shift + delete     | ctrl + k        |
/// | NavLeft               | 🡄          |                           |                  |
/// | NavRight              | 🡆          |                           |                  |
/// | NavUp                 | 🡅          |                           |                  |
/// | NavDown               | 🡇          |                           |                  |
/// | NavLineStart          | shift + home | home                      | cmd + 🡄, cmd + shift + 🡄 |
/// | NavLineEnd            | shift + end  | end                       | cmd + 🡆, cmd + shift + 🡆 |
/// | NavDocumentStart      |             | ctrl + home               | cmd + 🡅         |
/// | NavDocumentEnd        |             | ctrl + end                | cmd + 🡇         |
/// | NavWordLeft           |             | ctrl + 🡄                 | alt + 🡄         |
/// | NavWordRight          |             | ctrl + 🡆                 | alt + 🡆         |
/// | SelectAll             |             | ctrl + a                  | cmd + a         |
/// | SelectLeft            | shift + 🡄  |                           |                  |
/// | SelectRight           | shift + 🡆  |                           |                  |
/// | SelectUp              | shift + 🡅  |                           |                  |
/// | SelectDown            | shift + 🡇  |                           |                  |
/// | SelectDocumentStart   |             | ctrl + shift + home       | cmd + shift + 🡅 |
/// | SelectDocumentEnd     |             | ctrl + shift + end        | cmd + shift + 🡇 |
/// | SelectWordLeft        |             | ctrl + shift + 🡄         | alt + shift + 🡄 |
/// | SelectWordRight       |             | ctrl + shift + 🡆         | alt + shift + 🡆 |
/// | Cut                   |             | ctrl + x                  | cmd + x          |
/// | Copy                  |             | ctrl + c                  | cmd + c          |
/// | Paste                 |             | ctrl + v                  | cmd + v          |
/// | Undo                  |             | ctrl + z                  | cmd + z          |
/// | Redo                  |             | ctrl + shift + z          | cmd + shift + z  |
/// | ShowCharacterPalette  |             | ctrl + space              | cmd + space      |
///
/// TODO: Collection does not supply a way to unbind a default keystroke
pub fn default_bindings() -> gpui::ActionBindingCollection {
    let mut bindings = gpui::ActionBindingCollection::default()
        .with::<DeleteLeft>("backspace")
        .with::<DeleteRight>("delete")
        .with::<Tab>("tab")
        .with::<Enter>("enter")
        .with::<NavLeft>("left")
        .with::<NavRight>("right")
        .with::<NavUp>("up")
        .with::<NavDown>("down")
        .with::<SelectAll>("secondary-a")
        .with::<SelectLeft>("shift-left")
        .with::<SelectRight>("shift-right")
        .with::<SelectUp>("shift-up")
        .with::<SelectDown>("shift-down")
        // Shift+Home and Shift+End intentionally use navigation actions so they collapse the
        // selection at the requested line boundary.
        .with::<NavLineStart>("shift-home")
        .with::<NavLineEnd>("shift-end")
        .with::<Copy>("secondary-c")
        .with::<Cut>("secondary-x")
        .with::<Paste>("secondary-v")
        .with::<Undo>("secondary-z")
        .with::<Redo>("secondary-shift-z")
        .with::<Escape>("escape")
        .with::<ShowCharacterPalette>("secondary-space");

    #[cfg(target_os = "macos")]
    {
        bindings = bindings
            .with::<DeleteWordLeft>("alt-backspace")
            .with::<DeleteWordRight>("alt-delete")
            .with::<DeleteToLineStart>("cmd-backspace")
            .with::<DeleteToLineEnd>("ctrl-k")
            // Mac keyboards don't have Home/End keys, so cmd-left/right are standard
            .with::<NavLineStart>("cmd-left")
            .with::<NavLineEnd>("cmd-right")
            .with::<NavDocumentStart>("cmd-up")
            .with::<NavDocumentEnd>("cmd-down")
            .with::<NavLineStart>("cmd-shift-left")
            .with::<NavLineEnd>("cmd-shift-right")
            .with::<SelectDocumentStart>("cmd-shift-up")
            .with::<SelectDocumentEnd>("cmd-shift-down")
            .with::<NavWordLeft>("alt-left")
            .with::<NavWordRight>("alt-right")
            .with::<SelectWordLeft>("alt-shift-left")
            .with::<SelectWordRight>("alt-shift-right");
    }

    #[cfg(not(target_os = "macos"))]
    {
        bindings = bindings
            .with::<DeleteWordLeft>("ctrl-backspace")
            .with::<DeleteWordRight>("ctrl-delete")
            .with::<DeleteToLineStart>("ctrl-shift-backspace")
            .with::<DeleteToLineEnd>("ctrl-shift-delete")
            .with::<NavLineStart>("home")
            .with::<NavLineEnd>("end")
            .with::<NavDocumentStart>("ctrl-home")
            .with::<NavDocumentEnd>("ctrl-end")
            .with::<SelectDocumentStart>("ctrl-shift-home")
            .with::<SelectDocumentEnd>("ctrl-shift-end")
            .with::<NavWordLeft>("ctrl-left")
            .with::<NavWordRight>("ctrl-right")
            .with::<SelectWordLeft>("ctrl-shift-left")
            .with::<SelectWordRight>("ctrl-shift-right");
    }

    bindings
}

/// Declares stubs for all editable-text actions that an element's state entity can implement.
pub trait EditableTextActionHandler<Context>: Sized {
    /// Blur focus from the input.
    fn escape(&mut self, _: &Escape, _w: &mut Window, _cx: &mut Context) {}

    /// Insert a newline at the cursor position.
    fn insert_enter(&mut self, _: &Enter, _w: &mut Window, _cx: &mut Context) {}
    /// Insert a tab character at the cursor position.
    fn insert_tab(&mut self, _: &Tab, _w: &mut Window, _cx: &mut Context) {}

    /// Delete the character before the cursor.
    fn delete_left(&mut self, _: &DeleteLeft, _w: &mut Window, _cx: &mut Context) {}
    /// Delete the character after the cursor.
    fn delete_right(&mut self, _: &DeleteRight, _w: &mut Window, _cx: &mut Context) {}
    /// Delete the word before the cursor.
    fn delete_word_left(&mut self, _: &DeleteWordLeft, _w: &mut Window, _cx: &mut Context) {}
    /// Delete the word after the cursor.
    fn delete_word_right(&mut self, _: &DeleteWordRight, _w: &mut Window, _cx: &mut Context) {}
    /// Delete from the cursor to the beginning of the line.
    fn delete_to_line_start(&mut self, _: &DeleteToLineStart, _w: &mut Window, _cx: &mut Context) {}
    /// Delete from the cursor to the end of the line.
    fn delete_to_line_end(&mut self, _: &DeleteToLineEnd, _w: &mut Window, _cx: &mut Context) {}

    /// Move the cursor one character to the left.
    fn nav_left(&mut self, _: &NavLeft, _w: &mut Window, _cx: &mut Context) {}
    /// Move the cursor one character to the right.
    fn nav_right(&mut self, _: &NavRight, _w: &mut Window, _cx: &mut Context) {}
    /// Move the cursor up one visual line.
    fn nav_up(&mut self, _: &NavUp, _w: &mut Window, _cx: &mut Context) {}
    /// Move the cursor down one visual line.
    fn nav_down(&mut self, _: &NavDown, _w: &mut Window, _cx: &mut Context) {}
    /// Move cursor to the start of the current line.
    fn nav_line_start(&mut self, _: &NavLineStart, _w: &mut Window, _cx: &mut Context) {}
    /// Move cursor to the end of the current line.
    fn nav_line_end(&mut self, _: &NavLineEnd, _w: &mut Window, _cx: &mut Context) {}
    /// Move cursor to the start of the document.
    fn nav_start(&mut self, _: &NavDocumentStart, _w: &mut Window, _cx: &mut Context) {}
    /// Move cursor to the end of the document.
    fn nav_end(&mut self, _: &NavDocumentEnd, _w: &mut Window, _cx: &mut Context) {}
    /// Move cursor one word to the left.
    fn nav_left_word(&mut self, _: &NavWordLeft, _w: &mut Window, _cx: &mut Context) {}
    /// Move cursor one word to the right.
    fn nav_right_word(&mut self, _: &NavWordRight, _w: &mut Window, _cx: &mut Context) {}

    /// Select the entire document.
    fn select_all(&mut self, _: &SelectAll, _w: &mut Window, _cx: &mut Context) {}
    /// Extend selection one character to the left.
    fn select_left(&mut self, _: &SelectLeft, _w: &mut Window, _cx: &mut Context) {}
    /// Extend selection one character to the right.
    fn select_right(&mut self, _: &SelectRight, _w: &mut Window, _cx: &mut Context) {}
    /// Extend selection up one visual line.
    fn select_up(&mut self, _: &SelectUp, _w: &mut Window, _cx: &mut Context) {}
    /// Extend selection down one visual line.
    fn select_down(&mut self, _: &SelectDown, _w: &mut Window, _cx: &mut Context) {}
    /// Extend selection to the beginning of the document.
    fn select_start(&mut self, _: &SelectDocumentStart, _w: &mut Window, _cx: &mut Context) {}
    /// Extend selection to the end of the document.
    fn select_end(&mut self, _: &SelectDocumentEnd, _w: &mut Window, _cx: &mut Context) {}
    /// Extend selection one word to the left.
    fn select_left_word(&mut self, _: &SelectWordLeft, _w: &mut Window, _cx: &mut Context) {}
    /// Extend selection one word to the right.
    fn select_right_word(&mut self, _: &SelectWordRight, _w: &mut Window, _cx: &mut Context) {}

    /// Cut selected text to clipboard.
    fn cut(&mut self, _: &Cut, _w: &mut Window, _cx: &mut Context) {}
    /// Copy selected text to clipboard.
    fn copy(&mut self, _: &Copy, _w: &mut Window, _cx: &mut Context) {}
    /// Paste from clipboard at the cursor position.
    fn paste(&mut self, _: &Paste, _w: &mut Window, _cx: &mut Context) {}

    /// Undo the last edit.
    fn undo(&mut self, _: &Undo, _w: &mut Window, _cx: &mut Context) {}
    /// Redo the last undone edit.
    fn redo(&mut self, _: &Redo, _w: &mut Window, _cx: &mut Context) {}

    /// Show the platform character palette.
    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _cx: &mut Context,
    ) {
        window.show_character_palette();
    }

    fn on_mouse_down(
        &mut self,
        _event: &gpui::MouseDownEvent,
        _text_position: gpui::Point<gpui::Pixels>,
        _w: &mut Window,
        _cx: &mut Context,
    ) {
    }
    fn on_mouse_up(&mut self, _event: &gpui::MouseUpEvent, _w: &mut Window, _cx: &mut Context) {}
    fn on_mouse_move(
        &mut self,
        _event: &gpui::MouseMoveEvent,
        _text_position: gpui::Point<gpui::Pixels>,
        _w: &mut Window,
        _cx: &mut Context,
    ) {
    }
}

/// Registers an handler function of [`EditableTextActionHandler`]
/// which is processed via the return value of [`EditableTextActionElement::state_entity_rc`].
macro_rules! register_action {
    ($action_element:expr, $func:ident) => {{
        let entity_rc = $action_element.state_entity_rc().clone();
        $action_element
            .interactivity()
            .on_action(move |action, window, cx| {
                let weak_entity = entity_rc.borrow();
                if let Some(entity) = weak_entity.upgrade() {
                    entity.update(cx, |state, cx| {
                        state.$func(action, window, cx);
                    });
                }
            });
    }};
}

/// Generic trait to support an element backed by an internal state entity to bind to all editable-text input actions.
pub(super) trait EditableTextActionElement<State> {
    fn state_entity_rc(&self) -> &Rc<RefCell<WeakEntity<State>>>;

    fn register_actions(&mut self)
    where
        Self: InteractiveElement,
        State: for<'app> EditableTextActionHandler<gpui::Context<'app, State>>,
        State: 'static,
    {
        register_action!(self, escape);
        register_action!(self, insert_enter);
        register_action!(self, insert_tab);
        register_action!(self, delete_left);
        register_action!(self, delete_right);
        register_action!(self, delete_word_left);
        register_action!(self, delete_word_right);
        register_action!(self, delete_to_line_start);
        register_action!(self, delete_to_line_end);
        register_action!(self, nav_left);
        register_action!(self, nav_right);
        register_action!(self, nav_up);
        register_action!(self, nav_down);
        register_action!(self, nav_line_start);
        register_action!(self, nav_line_end);
        register_action!(self, nav_start);
        register_action!(self, nav_end);
        register_action!(self, nav_left_word);
        register_action!(self, nav_right_word);
        register_action!(self, select_all);
        register_action!(self, select_left);
        register_action!(self, select_right);
        register_action!(self, select_up);
        register_action!(self, select_down);
        register_action!(self, select_start);
        register_action!(self, select_end);
        register_action!(self, select_left_word);
        register_action!(self, select_right_word);
        register_action!(self, cut);
        register_action!(self, copy);
        register_action!(self, paste);
        register_action!(self, undo);
        register_action!(self, redo);
        register_action!(self, show_character_palette);
    }
}
