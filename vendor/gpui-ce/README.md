# gpui - Community Edition

A community fork of [GPUI](https://gpui.rs), Zed's GPU-accelerated UI framework.

## Usage

```toml
[dependencies]
gpui = { package = "gpui-ce", version = "0.3" }
gpui_platform = { git = "https://github.com/gpui-ce/gpui-ce" }

# for test support...
[dev-dependencies]
gpui = { package = "gpui-ce", version = "0.3", features = ["test-support"] }
```

or for using the git version

```toml
gpui = { package = "gpui", git = "https://github.com/gpui-ce/gpui-ce" }
```

Then use `gpui::{import}` as normal.

## FAQ
#### How does the project compare to other forks in the ecosystem?
Other efforts (namely WGPUI) are actively maintained, but have diverged quite a bit from mainline usage. They typically serve the interests of the projects that they're used within, leading to a diverse yet fragmented ecosystem. GPUI-CE strives to focus on the general use-case first, and over time, grow in the facilities to support the same outside adaptations through a single consistent API.

#### What is the long-term goal of GPUI-CE?
We'd like to be a premiere Rust GUI library! For the time being, we're working incrementally, in an effort to better understand the codebase and where is the right direction to take it, so we're okay being limited by mainline Zed. We will not stay this way forever! The spirit of the project is independence, so "limited" is loose, and we have and will continue to add features that mainline will never have. We will make your contribution work :)

If you'd like to join discussions and help us forge an path forward, please join the discord.

#### Can I use GPUI-CE with gpui-component?
100% Because we're a drop-in for GPUI, any component library or surrounding project should work 1:1 through the use of a [patch block](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html). DO NOTE: We track the latest upstream -- if there's breaking changes and the library you're pulling in hasn't updated yet, gpui-ce cannot help you. Otherwise, we treat any mismatches as bugs.

Example:
```toml
# If they're using the crates release
[patch.crates-io]
gpui = { git = "https://github.com/gpui-ce/gpui-ce", package = "gpui-ce" }

# If they're using the git remote
[patch."https://github.com/zed-industries/zed.git"]
gpui = { git = "https://github.com/gpui-ce/gpui-ce" }
```

#### Is there a community I could... join?
For sure! Join the [discord](https://discord.gg/vNdskJSfWd)

<!-- todo: rewrite below... -->

# Welcome to GPUI!

GPUI is a hybrid immediate and retained mode, GPU accelerated, UI framework
for Rust, designed to support a wide variety of applications.

Everything in GPUI starts with an `Application`. You can create one with `gpui_platform::application()`, and kick off your application by passing a callback to `Application::run()`. Inside this callback, you can create a new window with `App::open_window()`, and register your first root view. See [gpui.rs](https://www.gpui.rs/) for a complete example.

### Dependencies

GPUI has various system dependencies that it needs in order to work.

#### macOS

On macOS, GPUI uses Metal for rendering. In order to use Metal, you need to do the following:

- Install [Xcode](https://apps.apple.com/us/app/xcode/id497799835?mt=12) from the macOS App Store, or from the [Apple Developer](https://developer.apple.com/download/all/) website. Note this requires a developer account.

> Ensure you launch Xcode after installing, and install the macOS components, which is the default option.

- Install [Xcode command line tools](https://developer.apple.com/xcode/resources/)

  ```sh
  xcode-select --install
  ```

- Ensure that the Xcode command line tools are using your newly installed copy of Xcode:

  ```sh
  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
  ```

## The Big Picture

GPUI offers three different [registers](<https://en.wikipedia.org/wiki/Register_(sociolinguistics)>) depending on your needs:

- State management and communication with `Entity`'s. Whenever you need to store application state that communicates between different parts of your application, you'll want to use GPUI's entities. Entities are owned by GPUI and are only accessible through an owned smart pointer similar to an `Rc`. See the `app::context` module for more information.

- High level, declarative UI with views. All UI in GPUI starts with a view. A view is simply an `Entity` that can be rendered, by implementing the `Render` trait. At the start of each frame, GPUI will call this render method on the root view of a given window. Views build a tree of `elements`, lay them out and style them with a tailwind-style API, and then give them to GPUI to turn into pixels. See the `div` element for an all purpose swiss-army knife of rendering.

- Low level, imperative UI with Elements. Elements are the building blocks of UI in GPUI, and they provide a nice wrapper around an imperative API that provides as much flexibility and control as you need. Elements have total control over how they and their child elements are rendered and can be used for making efficient views into large lists, implement custom layouting for a code editor, and anything else you can think of. See the `element` module for more information.

Each of these registers has one or more corresponding contexts that can be accessed from all GPUI services. This context is your main interface to GPUI, and is used extensively throughout the framework.

## Other Resources

In addition to the systems above, GPUI provides a range of smaller services that are useful for building complex applications:

- Actions are user-defined structs that are used for converting keystrokes into logical operations in your UI. Use this for implementing keyboard shortcuts, such as cmd-q. See the `action` module for more information.

- Platform services, such as `quit the app` or `open a URL` are available as methods on the `app::App`.

- An async executor that is integrated with the platform's event loop. See the `executor` module for more information.,

- The `[gpui::test]` macro provides a convenient way to write tests for your GPUI applications. Tests also have their own kind of context, a `TestAppContext` which provides ways of simulating common platform input. See `app::test_context` and `test` modules for more details.

Currently, the best way to learn about these APIs is to read the Zed source code or drop a question in the [Zed Discord](https://zed.dev/community-links). We're working on improving the documentation, creating more examples, and will be publishing more guides to GPUI on our [blog](https://zed.dev/blog).
