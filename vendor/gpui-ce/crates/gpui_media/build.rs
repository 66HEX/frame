#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]
fn main() {
    use std::{env, path::PathBuf, process::Command};

    // A build script compiles for the host, so `cfg!(target_os)` here would
    // describe the machine running cargo. The platform being compiled for is
    // only visible through the environment cargo sets for build scripts.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    // An explicit SDKROOT (a cross build, or a pinned SDK) wins over asking
    // Xcode, which a non-Apple host does not have.
    println!("cargo:rerun-if-env-changed=SDKROOT");
    let sdk_path = match env::var("SDKROOT") {
        Ok(root) if !root.is_empty() => root,
        _ => String::from_utf8(
            Command::new("xcrun")
                .args(["--sdk", "macosx", "--show-sdk-path"])
                .output()
                .expect("neither SDKROOT nor xcrun can locate the macOS SDK")
                .stdout,
        )
        .unwrap(),
    };
    let sdk_path = sdk_path.trim_end();

    println!("cargo:rerun-if-changed=src/bindings.h");
    let bindings = bindgen::Builder::default()
        .header("src/bindings.h")
        .clang_arg(format!("-isysroot{}", sdk_path))
        .clang_arg("-xobjective-c")
        .allowlist_type("CMItemIndex")
        .allowlist_type("CMSampleTimingInfo")
        .allowlist_type("CMVideoCodecType")
        .allowlist_type("VTEncodeInfoFlags")
        .allowlist_function("CMTimeMake")
        .allowlist_var("kCVPixelFormatType_.*")
        .allowlist_var("kCVReturn.*")
        .allowlist_var("VTEncodeInfoFlags_.*")
        .allowlist_var("kCMVideoCodecType_.*")
        .allowlist_var("kCMTime.*")
        .allowlist_var("kCMSampleAttachmentKey_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .expect("unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("couldn't write dispatch bindings");
}
