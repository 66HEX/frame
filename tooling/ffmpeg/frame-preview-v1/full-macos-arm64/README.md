# Complete macOS ARM64 runtime

This opt-in recipe builds FFmpeg 8.1.2 and FFprobe with the Frame preview transport patch, the codec, format and filter coverage of the existing macOS ARM64 bundle, static third-party libraries, and a macOS 12 deployment target. It writes only to a new private output directory. It does not install into Homebrew, change the app bundle, sign, publish, or commit binaries.

The smaller sibling configuration remains a protocol test build. It cannot replace the complete runtime.

## Requirements

Use an ARM64 Mac with Xcode command-line tools and its macOS SDK, Python 3.12+, CMake, Ninja, NASM, pkg-config, Autoconf, GNU Libtool (`glibtoolize` and `glibtool`), and the already-installed Rust toolchain `1.95.0-aarch64-apple-darwin`. The script checks for tools and never installs a system toolchain. The initial successful build used Apple clang and SDK 26.4, CMake 4.3.1, Autoconf 2.72, and Rust 1.95.0; their command output is part of the build record.

Automake 1.17, Meson 1.9.1 and cargo-c 0.10.24+cargo-0.98.0 are built or unpacked into the private tools directory. cargo-c requires Rust 1.95. The only Cargo operations build this tool and rav1e, with a separate Cargo home and target; this recipe does not invoke repository Cargo. For rav1e, the installed Cargo fetches dependencies with `--locked` first; cargo-c then builds with `--locked --offline` from that same private cache.

Start with at least 9–12 GiB free. The default reserve is 6 GiB. Projects build sequentially with at most four build jobs; the recipe checks free space during commands, terminates its process group if the reserve is breached, and retains failure logs. It removes only its own new intermediate trees after preserving configuration, licenses, hashes, logs, and installed artifacts. Archives, the private prefix, Cargo cache, and the final runtime build remain. A failed run is retained; retry in a new output directory.

Use an output path without whitespace because several upstream pkg-config/autotools recipes do not escape such prefixes consistently. Its ancestors must contain neither a Cargo workspace nor `.cargo/config`/`.cargo/config.toml`; a private Cargo home alone does not disable ancestor configuration. The build environment is allowlisted: inherited compiler/Cargo/DESTDIR/toolchain overrides are excluded, SDKROOT comes from the selected macOS SDK, CONFIG_SITE is `/dev/null`, and CMake package registries are disabled. Compiler commands and generated configurations are retained before cleanup.

## Commands

From the repository root, print the locked plan without downloading or building:

```sh
python3 tooling/ffmpeg/frame-preview-v1/full-macos-arm64/build_full.py
```

Build in a new directory:

```sh
python3 tooling/ffmpeg/frame-preview-v1/full-macos-arm64/build_full.py \
  --build --output /tmp/frame-ffmpeg-full --jobs 4
```

An optional read-only archive cache uses the names printed by the source manifest. Every cached file must match the lock before use:

```sh
python3 tooling/ffmpeg/frame-preview-v1/full-macos-arm64/build_full.py \
  --verify-cache /path/to/verified-archives

python3 tooling/ffmpeg/frame-preview-v1/full-macos-arm64/build_full.py \
  --build --output /tmp/frame-ffmpeg-full-cached --jobs 4 \
  --archive-cache /path/to/verified-archives
```

The FFmpeg archive is named `ffmpeg-n8.1.2.tar.gz`; its source and patch are checked against the sibling runtime lock. The final step delegates to `../build_pinned.py --configure-json ... --build`, retaining its exact source/patch/file checks. Final binaries are under `runtime/artifacts/`. `--runtime-recipe` can explicitly locate that sibling helper when reviewing this directory outside the repository.

To run the development app with the resulting runtime, select both tools explicitly:

```sh
FRAME_FFMPEG_PATH=/tmp/frame-ffmpeg-full/runtime/artifacts/ffmpeg \
FRAME_FFPROBE_PATH=/tmp/frame-ffmpeg-full/runtime/artifacts/ffprobe \
cargo xtask run
```

This selection does not replace the existing bundled files. Release packaging
must supply the patched runtime separately; Frame checks its transport support
before enabling per-frame SAR.

## Pins and portability

`sources.lock.json` locks 34 archives: 31 libraries and three build tools. Rust transitive dependencies also use the upstream Cargo locks and registry checksums with `--locked`. Each build saves the archives, source hashes, effective environment, compiler commands, logs, generated build configuration, licenses, and installed archive hashes.

The provider recipe reference is commit `bb1d6db29cee948f9685bcd69e6caf17d960662b`. It is the last provider commit before the existing bundle timestamp, but the provider's historical dependency manifest was unavailable. This recipe does not claim to reproduce that binary byte for byte. It pins the provider's release versions where available and replaces its moving x264 `master` URL with the recorded upstream stable commit. Graphite2 is included explicitly. The VMAF v3.1.0 archive reports pkg-config version 3.0.0 in its own source; the archive pin remains v3.1.0.

Libraries use baseline ARM64, macOS 12, and runtime CPU dispatch where supported. No `-march=native` or `-mcpu=native` is requested. Homebrew package discovery is excluded from target builds; host build tools may still use their installed Homebrew libraries. The final runtime must link only Apple/system libraries. x265 is built for 8, 10 and 12 bits and combined with Apple's archive tool. OpenH264 declares libc++ through its upstream `STATIC_LDFLAGS` variable. zimg uses `STL_LIBS=-lc++`; static VMAF requires the explicit FFmpeg `--extra-libs=-lc++`. Vorbis 1.3.7's obsolete Darwin linker flag is removed from effective make flags without changing codec source.

This recipe disables FreeType's optional PNG, Brotli and HarfBuzz integrations, and HarfBuzz's GLib, GObject and ICU integrations. It enables Graphite2 support in HarfBuzz. The registry comparison and font tests below do not establish parity for every optional font feature; the original bundle's complete dependency configuration was unavailable.

## Required verification before delivery

A successful compile is not release acceptance. For both FFmpeg and FFprobe:

- Compare encoder, decoder, filter, demuxer/muxer aliases, both protocol directions, bitstream filters, devices, hardware APIs, pixel/sample formats, and relevant encoder pixel-format support with the actual bundled binaries. Reject missing features.
- Inspect `otool -L`, load commands and `vtool -show-build`; reject private/Homebrew dynamic linkage, private rpaths, or minimum OS above 12. Inspect linked archive architecture and generated CPU flags.
- Copy the binaries to a separate test directory, temporarily rename only this build's private prefix, and restore it in a `finally`/trap. Run actual libass/CoreText subtitle burn, drawtext/fontconfig, zscale and embedded-model VMAF. Compare the font samples with the bundle. This catches resource dependencies that a dylib check misses.
- Encode real high-depth samples with x264, x265 at 8/10/12 bits, AOM, SVT-AV1, rav1e and the other enabled external encoders. Run the Frame transport/default-off/forged-metadata controls and application media tests with this explicitly selected runtime.
- Test on a physical macOS 12/M1 machine before claiming that platform has been exercised. A deployment-target and instruction audit on a newer Mac is a portability check, not physical old-OS execution.

The maintained recipe completed a fresh build with all 34 pinned dependencies and tools. Its resulting binaries passed 12 registry comparisons with no missing features, 22 encoder configurations, 3 paired font/plain comparisons and 2 paired external-filter comparisons with the private prefix hidden, plus 126 transport cases, 7 rejection controls and 3 forged-source controls. The application passed 111 media tests using these exact binaries. The build retained actual compiler commands for all 31 target libraries and FFmpeg, closing the earlier research build's command-record gaps.

Both binaries had minOS 12 and system-only dynamic linkage. All 44 static archives contained ARM64 objects with minimum OS at most 12. Fontconfig printed the same missing-default-config diagnostics as the existing bundle, while drawtext and CoreText results matched that bundle exactly. These are checks on the current development Mac, not physical macOS 12/M1 execution.

Two earlier fresh builds stopped during cargo-c's registry access for rav1e. Their logs were retained. The maintained recipe's locked Cargo fetch followed by offline cargo-c resolved that observed failure without changing dependency pins or compiler flags. The exact underlying HTTP failure was not established.

## License and provenance

The FFmpeg configuration retains `--enable-gpl --enable-version3` and does not enable `nonfree`; FFmpeg reports GPL version 3 or later. The patch, source archive, build options, and dependency license files must accompany the normal runtime licensing/release process. Dependency versions, notices, and the source offer must be checked for an actual distribution. The provider's build-script license is separate from FFmpeg and its libraries. Nothing in this recipe performs distribution or changes the repository's license.

Primary references: [FFmpeg 8.1.2 license](https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.2/LICENSE.md), [provider source recipe](https://git.martin-riedl.de/ffmpeg/build-script), and the upstream release URLs in the lock.
