<img src="assets/readme/banner.png" alt="GPUI Community Edition banner" width="100%">

<p align="center">
  <a href="https://gpui-ce.github.io">Website</a> ​ · ​ <a href="crates/gpui/examples/learn">Examples</a> ​ · ​ <a href="https://discord.gg/ENGHGjrYEn">Discord</a>
</p>

# GPUI - Community Edition

GPUI-CE is a fork of Zed's [GPUI](https://gpui.rs) UI Framework.

It's mostly API compatible with upstream, but that is changing!

## Overview:

- Web-inspired Styling & Layout

  Build views with familiar elements, flex layouts, and Tailwind-style methods:

  ```rust
  div()
      .id("some_id_123")
      .flex()
      .items_center()
      .gap_2()
      .rounded_lg()
      .rounded_smoothing(0.8)
      .bg(rgba(0xffffff30))
      .backdrop_blur(px(12.0))
      .transitions(|transitions| transitions.bg(millis(200)))
      .hover_style(|style| style.bg(rgba(0xffffff60)))
      .child("Hello, GPUI")
  ```

  [Layout example](crates/gpui/examples/learn/layout.rs) ​ · ​ [Styling example](crates/gpui/examples/learn/styling.rs)

- State and events

  Use `Entity<T>` to access view and shared application state. Observe changes and call `cx.notify()` when state changes to notify observers and update the view. For a view with a `count` field:

  ```rust
  div()
      .id("counter")
      .child(format!("Count: {}", self.count))
      .on_click(cx.listener(|this, _event, _window, cx| {
          this.count += 1;
          cx.notify();
      }))
  ```

  [Interaction example](crates/gpui/examples/learn/interactive_elements.rs)

- Actions and keybinds

  Define typed actions and bind them to keyboard shortcuts. Call `register_keybinds` during app setup to bind Space and Backspace in the focused counter:

  ```rust
  use gpui::{App, KeyBinding, actions};

  actions!(keybinds_example, [Increment, Reset]);

  fn register_keybinds(cx: &mut App) {
      cx.bind_keys([
          KeyBinding::new("space", Increment, Some("Counter")),
          KeyBinding::new("backspace", Reset, Some("Counter")),
      ]);
  }
  ```

  Connect the view's focus handle and action handlers:

  ```rust
  div()
      .key_context("Counter")
      .track_focus(&self.focus_handle)
      .on_action(cx.listener(Self::increment))
      .on_action(cx.listener(Self::reset))
      .child(format!("Count: {}", self.count))
  ```

  [Keybind example](crates/gpui/examples/learn/actions_and_keybinds.rs)

- Virtualized lists

  Use `uniform_list` for large collections of equal-height rows. GPUI requests the item ranges needed for the visible area as you scroll.

  ```rust
  uniform_list("items", 10_000, |range, _window, _cx| {
      range
          .map(|index| {
              div()
                  .h(px(24.0))
                  .child(format!("Item {index}"))
          })
          .collect()
  })
  .h(px(300.0))
  ```

  [List example](crates/gpui/examples/learn/uniform_list.rs)

- Custom drawing

  Use `canvas` to paint directly within a view. Implement `Element` when you need control over layout and rendering, such as for a code editor or custom widget.

  ```rust
  canvas(
      |_bounds, _window, _cx| {},
      |bounds, _state, window, _cx| {
          window.paint_quad(PaintQuad::new(bounds).background(rgb(0x5078f0)));
      },
  )
  .size(px(80.0))
  ```

  [Drawing example](crates/gpui/examples/learn/custom_drawing.rs)

## FAQ

- Q: What is our AI Policy?
  A: See the [genAI/LLM Policy document](AI_POLICY.md). GPUI-CE is a framework "Made by Humans". We value community participation and take time to understand contributors' intentions and offer guidance.

- Q: What is the long-term goal of GPUI-CE?
  A: To become the go-to Rust GUI library for applications of any size. We want reusable components, native platform integration, and control over performance. We build on Zed's work and continue to bring in upstream fixes.

- Q: How does the project compare to other forks in the ecosystem?
  A: Other forks often develop around the applications that use them. GPUI-CE aims to support a broad range of applications, with a focus on stability.

## Contributing

Read the [contributing](CONTRIBUTING.md) and [genAI/LLM Policy](AI_POLICY.md) documents, which fully outline everything you need to know about how this project is maintained and what maintainers expect from contributors and contributions.

While you will need to read both documents in full to contribute, they can be summarized as:
- use github discussions to discuss potential bugs or feature requests
- issue tracker is all actionable work for anyone to pick up
- there is a vouch system for new contributors to get maintainer approval for changes
- issues and PRs from non-vouched contributors will be automatically closed
- all commits/PRs must update the changelog if they affect functionality
- there are commit content and description guidelines (see contributing doc)
- do not use genAI/LLMs to create code or prose, and all other forms of usage must be disclosed

Then read the [setup guide](SETUP.md) for installation instructions.
