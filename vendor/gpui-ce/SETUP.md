# Development setup

Install Git, then install Rust through [rustup](https://rustup.rs). GPUI-CE requires Rust 1.95 or newer, as specified in [Cargo.toml](crates/gpui/Cargo.toml). Add the formatting and linting tools:

```sh
rustup component add rustfmt clippy
```

## macOS

GPUI uses Metal on macOS and needs Xcode's developer tools.

1. Install [Xcode from the Mac App Store](https://apps.apple.com/us/app/xcode/id497799835?mt=12) or [Apple Developer Downloads](https://developer.apple.com/download/all/). Launch Xcode and install the macOS components.
2. Install the command line tools:

   ```sh
   xcode-select --install
   ```

3. Select your Xcode installation:

   ```sh
   sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
   ```

Adjust the path if you installed Xcode elsewhere. See Apple's [command line tools documentation](https://developer.apple.com/documentation/xcode/command-line-tools) for details.

## Linux

On Ubuntu or Debian, install the build tools and the development libraries used by our [Linux CI job](.github/workflows/ci.yml):

```sh
sudo apt-get update
sudo apt-get install -y \
  build-essential pkg-config \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev libxcb-randr0-dev libxcb-xinput-dev \
  libegl1-mesa-dev libgles2-mesa-dev libglib2.0-dev libfontconfig-dev
```

Use the equivalent packages on other distributions. Desktop examples need a graphical session.

## Windows

Use Rust's MSVC toolchain with the Visual Studio C++ build tools and a Windows SDK. Follow Microsoft's [Rust development setup guide](https://learn.microsoft.com/en-us/windows/dev-environment/rust/setup).
