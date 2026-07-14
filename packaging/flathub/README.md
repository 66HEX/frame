# Flathub Distribution

This directory is the source for the production Flathub package
`io.github._66HEX.Frame`.

The Flathub package intentionally differs from the local devel Flatpak built by
`script/bundle-linux --flatpak`:

- it builds from the GitHub release source archive,
- it uses a separate `cargo vendor` archive for offline Rust dependencies,
- it does not install `frame-app/resources/binaries`,
- it does not install copied host libraries from the managed Linux tarball,
- it sets `FRAME_USE_SYSTEM_MEDIA_TOOLS=1`, so Frame resolves `ffmpeg` and
  `ffprobe` from the Flatpak runtime PATH,
- it uses the Freedesktop 25.08 runtime and its automatically installed
  `org.freedesktop.Platform.codecs-extra` extension instead of bundling FFmpeg.
- it does not request static home-directory access; Frame asks the user to
  choose its default output folder through the file chooser portal.
- it sends completion notifications through the notification portal without
  direct access to `org.freedesktop.Notifications`.

## One-Time Flathub Setup

1. Generate the manifest with `cargo xtask flathub-manifest`.
2. Submit the first app review PR to `flathub/flathub` against the `new-pr`
   branch, using `target/flathub/repo/io.github._66HEX.Frame.yml`.
3. After approval, accept write access to the application repository:
   `flathub/io.github._66HEX.Frame`.
4. Add a GitHub token with write access to that repository as
   `FLATHUB_GITHUB_TOKEN` in `66HEX/frame` repository secrets.
5. Ensure the release workflow can push branches and open pull requests in the
   Flathub repository.

After the initial Flathub review, Frame release automation updates the Flathub
manifest in the same way the release workflow updates Homebrew and WinGet.

## Local Artifact Preparation

For a release tag, prepare Flathub inputs with:

```bash
cargo xtask flathub-sources --version 0.30.0
```

This creates:

```text
target/flathub/frame-0.30.0-source.tar.gz
target/flathub/frame-0.30.0-cargo-vendor.tar.gz
target/flathub/checksums.env
```

Render the Flathub repository files with:

```bash
cargo xtask flathub-manifest \
  --version 0.30.0 \
  --release-date 2026-07-10 \
  --source-url https://github.com/66HEX/frame/releases/download/0.30.0/frame-0.30.0-source.tar.gz \
  --source-sha256 "$(grep FRAME_SOURCE_SHA256 target/flathub/checksums.env | cut -d= -f2)" \
  --vendor-url https://github.com/66HEX/frame/releases/download/0.30.0/frame-0.30.0-cargo-vendor.tar.gz \
  --vendor-sha256 "$(grep FRAME_CARGO_VENDOR_SHA256 target/flathub/checksums.env | cut -d= -f2)" \
  --out target/flathub/repo
```

`target/flathub/repo/io.github._66HEX.Frame.yml` belongs in
`flathub/io.github._66HEX.Frame`. The rendered metainfo XML in the same
directory is only a validation aid; the Flathub build installs metainfo from the
upstream source archive.
