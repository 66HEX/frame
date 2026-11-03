//! Implementation for editable-text elements (gpui equivalent of html
//! [`<input>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/input) and
//! [`<textarea>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/textarea)).
//!
//! Both [`text_input`] and [`text_area`] create an [`EditableTextElement`]. This element supports:
//! - navigating via keyboard & mouse (by character, word, line, and document)
//! - highlight selection via keyboard & mouse (holding shift, double/triple click mouse, mouse drag)
//! - typing using an InputMethodEditor (IME) for writing Chinese, Japanese, and Korean utf-16
//! - inserting newlines (`\n`) and tabs (`\t`)
//! - cut/copy/paste
//! - caret / text cursor that can blink
//! - simple undo/redo within a single field
//!
//! For all input actions, see documentation in the [`actions`] module.
//!
//! Editable text elements will default to using [`String`] as the storage medium (see [`StringStorage`]).
//! Standard library strings are not ideal though for large text documents. For such uses,
//! it is encouraged that implementers consider rolling their own [`UnicodeTextStorage`] medium.
//!
//! Unlike other elements, editable text internally owns its [`FocusHandle`](gpui::FocusHandle).
//! This is required due to limitations of the [`Interactivity`](gpui::Interactivity) api and
//! that a user cannot interact with a text-input field if it cannot be focused.
//!
//! ### Usage Samples
//!
//! A single-line text input with a fixed width and text that does not wrap
//! (overflow text is clipped and does not scroll).
//! ```
//! # use gpui::prelude::*;
//! # fn test() -> gpui_ce_elements::editable_text::EditableTextElement {
//! use gpui_ce_elements::editable_text::text_input;
//! text_input("my_input")
//!     .placeholder("empty text")
//!     .w_5()
//!     .min_h_auto()
//!     .whitespace_nowrap()
//! # }
//! ```
//!
//! A single-line text input with a flexible width and text that does not wrap, but will scroll if overflowing.
//! ```
//! # use gpui::{prelude::*, Hsla};
//! # fn test() -> gpui_ce_elements::editable_text::EditableTextElement {
//! use gpui_ce_elements::editable_text::text_input;
//! text_input("my_input")
//!     .placeholder("empty text")
//!     .border_1().rounded_lg().border_color(gpui::white()) // has a border
//!     .p_2() // padding between the text and border
//!     .min_w_10().max_w_128()
//!     .min_h_auto()
//!     .whitespace_nowrap()
//!     .overflow_x_scroll()
//! # }
//! ```
//!
//! A multi-line text area with flexible height, wrapping text, and scrolling overflow on both axes.
//! ```
//! # use gpui::{prelude::*, Hsla};
//! # fn test() -> gpui_ce_elements::editable_text::EditableTextElement {
//! use gpui_ce_elements::editable_text::text_area;
//! text_area("message")
//!     .placeholder("empty text")
//!     .border_1().rounded_lg().border_color(gpui::white()) // has a border
//!     .p_2() // padding between the text and border
//!     .min_w_10().max_w_128()
//!     .min_h_24().max_h_128()
//!     .whitespace_normal() // default
//!     .overflow_y_scroll()
//! # }
//! ```
//!
//! The user-inputted text can be accessed via event callbacks on the element.
//! There is no callback representing the concept of "user is done editing". Its recommended that
//! users write a [debounce](https://developer.mozilla.org/en-US/docs/Glossary/Debounce)
//! or some way to detect "focus lost" to signify the user leaving the field.
//! ```
//! # use gpui::{prelude::*, App, Entity, Window, AppContext, ElementId};
//! # fn test(window: &mut Window, cx: &mut App) -> gpui_ce_elements::editable_text::EditableTextElement {
//! use gpui_ce_elements::editable_text::{text_input, EditableTextState, TextChanged};
//!
//! // A unique id to the editable text element within the outer scope.
//! let id = ElementId::from("my_input");
//!
//! // Find or lazily create the state entity backing the element.
//! // Then attach the entity to the element, thereby keeping it alive across consecutive frames.
//! let state = EditableTextState::use_keyed(id.clone(), window, cx);
//!
//! // This will trigger on every character input or other mutation to the underlying string
//! cx.subscribe(&state, |state, _: &TextChanged, cx| {
//! 	println!("{:?}", state.read(cx).as_str());
//! }).detach();
//!
//! // Using state explicitly attaches the state we already have attached to the ElementId.
//! text_input(id).state(state.downgrade())
//! # }
//! ```
//!
//! You can configure the default value of the editable text by using [`use_keyed_init`]:
//! ```
//! # use gpui::{prelude::*, App, Entity, Window, AppContext, ElementId};
//! # use gpui_ce_elements::editable_text::{text_input, EditableTextState, StringStorage};
//! # fn test(window: &mut Window, cx: &mut App) -> gpui_ce_elements::editable_text::EditableTextElement {
//! let id = ElementId::from("my_input");
//!
//! // The function parameter will only be called when the state is created/initialized.
//! // All successive renders across consecutive frames will re-use the existing state.
//! let _state = EditableTextState::use_keyed_init(id.clone(), window, cx,
//!     |_window, _cx| StringStorage::from("this is some default text content"));
//!
//! // Its also plausible to omit the state function call. The element will try to find the state
//! // according to its id (which we are trusting here was guaranteed to be at that id above).
//! // Despite this functionality, its recommended that callers which construct a state explicitly
//! // provide it to the element, at least for clarity and debugging.
//! text_input(id)
//! # }
//! ```
//!
//! To use a blinking caret, you can use one of the templated functions:
//! ```
//! # use gpui::{prelude::*, App, Entity, Window, AppContext, ElementId};
//! # fn test(window: &mut Window, cx: &mut App) -> gpui_ce_elements::editable_text::EditableTextElement {
//! use gpui_ce_elements::editable_text::{text_input};
//! let id = ElementId::from("my_input");
//! text_input(id)
//!     .caret_blink_interval_500ms()
//!     // or use the parameterized one, e.g. 200ms
//!     .caret_blink_interval(std::time::Duration::from_millis(200))
//! # }
//! ```
//!
//! or construct a caret entity with a blinking interval when constructing the state:
//! ```
//! # use gpui::{prelude::*, App, Entity, Window, AppContext, ElementId};
//! # fn test(window: &mut Window, cx: &mut App) -> gpui_ce_elements::editable_text::EditableTextElement {
//! use gpui_ce_elements::editable_text::{text_input, EditableTextState, TextChanged, Caret};
//! let id = ElementId::from("my_input");
//!
//! let state = EditableTextState::use_keyed(id.clone(), window, cx);
//!
//! // Ensure the caret exists, linked to the input element by id.
//! window.use_keyed_state(id.clone(), cx, |window, cx| {
//!     // using the default interval of 500ms
//!     let mut caret = Caret::default().with_blink_interval_500ms();
//!     // ensures the caret receives events from the input state during typing & other actions
//!     caret.subscribe_to(&state, cx);
//!     caret
//! });
//!
//! text_input(id).state(state.downgrade())
//! # }
//! ```
//!
//! Full-text examples can be found in the [examples folder](https://github.com/gpui-ce/gpui-ce/tree/main/crates/gpui_elements/examples)
//!
//! ### Backlog of not-yet implemented features:
//! - detecting focus being lost on an EditableText field
//! - text sanitation & validation (see no-op implementation of [`EditableTextState::validate_incoming_text`])
//! - nav & select via PageUp/PageDown
//! - screen reader support via a11y
//! - masking text (e.g. for passwords)
//! - disabling `insert_tab` if favor of tab being used to change focus between elements (i.e. escaping the field)
//!

pub mod actions;
mod caret;
mod element;
mod history;
mod layout;
mod state;
mod storage;

pub use caret::*;
pub use element::*;
pub use state::*;
pub use storage::*;
