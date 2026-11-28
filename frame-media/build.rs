use std::{env, fs, path::PathBuf};

const LIBRARIES: &[&str] = &[
    "avfilter",
    "avformat",
    "avcodec",
    "swresample",
    "swscale",
    "avutil",
];

fn main() {
    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
    println!("cargo:rerun-if-changed=native");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=dxgi");
    }
    let mut build = cc::Build::new();
    build.std("c11");
    let sdk = env::var_os("FFMPEG_DIR").map(PathBuf::from);
    if let Some(prefix) = &sdk {
        build.include(prefix.join("include"));
    } else {
        for name in LIBRARIES {
            let library = pkg_config::Config::new()
                .cargo_metadata(false)
                .probe(&format!("lib{name}"))
                .expect("Install the libav development SDK or set FFMPEG_DIR");
            build.includes(library.include_paths);
        }
    }
    let mut sources: Vec<_> = fs::read_dir("native")
        .expect("native directory")
        .map(|entry| entry.expect("source entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "c"))
        .collect();
    sources.sort();
    build.files(sources).compile("frame_media_native");
    if let Some(prefix) = &sdk {
        // macOS strips DYLD_LIBRARY_PATH from subprocesses launched through a
        // protected system binary, including /bin/sh in the descriptor test.
        if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
            && env::var("PROFILE").as_deref() == Ok("debug")
        {
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{}",
                prefix.join("lib").display()
            );
        }
        println!(
            "cargo:rustc-link-search=native={}",
            prefix.join("lib").display()
        );
        for name in LIBRARIES {
            println!("cargo:rustc-link-lib={name}");
        }
    } else {
        for name in LIBRARIES {
            pkg_config::probe_library(&format!("lib{name}")).expect("libav link settings");
        }
    }
}
