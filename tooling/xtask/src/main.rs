use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fmt, fs, io,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use frame_updater::{
    PlatformAssetKey, UpdateAsset, UpdateAssetKind, UpdateChannel, UpdateManifest, file_sha256_hex,
    sign_manifest_bytes,
};
use sha2::{Digest, Sha256};

const CI_WORKFLOW_PATH: &str = ".github/workflows/ci.yml";
const DEPENDENCY_REVIEW_WORKFLOW_PATH: &str = ".github/workflows/dependency-review.yml";
const CODEQL_WORKFLOW_PATH: &str = ".github/workflows/codeql.yml";
const RUN_BUNDLING_WORKFLOW_PATH: &str = ".github/workflows/run_bundling.yml";
const RELEASE_WORKFLOW_PATH: &str = ".github/workflows/release.yml";
const PUBLISH_NIXPKGS_WORKFLOW_PATH: &str = ".github/workflows/publish_nixpkgs.yml";
const FLATHUB_MANIFEST_TEMPLATE_PATH: &str = "packaging/flathub/io.github._66HEX.Frame.yml.in";
const FLATHUB_METAINFO_PATH: &str = "packaging/flathub/io.github._66HEX.Frame.metainfo.xml";
const FFMPEG_VERSION: &str = "8.1.2";
const RUST_VERSION: &str = "1.95.0";
const CARGO_BUNDLE_VERSION: &str = "0.11.0";
const CARGO_DENY_VERSION: &str = "0.20.2";
const CARGO_AUDIT_VERSION: &str = "0.22.2";
const CARGO_CYCLONEDX_VERSION: &str = "0.5.9";
const INNO_SETUP_VERSION: &str = "6.7.1";
const NIX_VERSION: &str = "2.35.1";
const APPIMAGETOOL_VERSION: &str = "1.9.1";
const APPIMAGETOOL_X86_64_SHA256: &str =
    "ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0";
const APPIMAGETOOL_AARCH64_SHA256: &str =
    "f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158";
const TAURI_BRIDGE_LATEST_JSON_SHA256: &str =
    "5cb46d105add55f71c24cbdf14433207ca377b11be4acf685acdb84244e98b3c";
const KOMAC_VERSION: &str = "2.16.0";
const KOMAC_LINUX_X86_64_SHA256: &str =
    "7d2707fa6210f2789a3702de49fbd150b736dbf426ee0b9bc8e098736f9fd82d";
const ACTIONLINT_VERSION: &str = "1.7.12";
const ACTIONLINT_LINUX_X86_64_SHA256: &str =
    "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8";

const ACTION_CHECKOUT_SHA: &str = "9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0";
const ACTION_UPLOAD_ARTIFACT_SHA: &str = "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a";
const ACTION_DOWNLOAD_ARTIFACT_SHA: &str = "3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c";
const ACTION_DEPENDENCY_REVIEW_SHA: &str = "a1d282b36b6f3519aa1f3fc636f609c47dddb294";
const ACTION_CODEQL_SHA: &str = "7188fc363630916deb702c7fdcf4e481b751f97a";
const ACTION_ATTEST_BUILD_PROVENANCE_SHA: &str = "0f67c3f4856b2e3261c31976d6725780e5e4c373";
const ACTION_IMPORT_CODESIGN_CERTS_SHA: &str = "5142e029c445c10ffc7149d172e540235a065466";
const ACTION_INSTALL_NIX_SHA: &str = "630ae543ea3a38a9a4166f03376c02c50f408342";

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

type Result<T> = std::result::Result<T, XtaskError>;

fn main() -> ExitCode {
    match run_xtask() {
        Ok(()) | Err(XtaskError::Help) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_xtask() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "build" => build_frame_app(args.collect()),
        "bundle" => {
            let args = args.collect::<Vec<_>>();
            bundle(&args)
        }
        "ci" => ci(),
        "run" => run_frame_app(args.collect()),
        "setup-ffmpeg" => {
            let args = args.collect::<Vec<_>>();
            setup_ffmpeg(&args)
        }
        "update-manifest" => {
            let args = args.collect::<Vec<_>>();
            update_manifest(&args)
        }
        "sign-update-manifest" => {
            let args = args.collect::<Vec<_>>();
            sign_update_manifest(&args)
        }
        "flathub-sources" => {
            let args = args.collect::<Vec<_>>();
            flathub_sources(&args)
        }
        "flathub-manifest" => {
            let args = args.collect::<Vec<_>>();
            flathub_manifest(&args)
        }
        "workflows" => {
            let args = args.collect::<Vec<_>>();
            match args.as_slice() {
                [] => write_workflows(),
                [flag] if flag == "--check" => check_workflows(),
                _ => Err(XtaskError::Usage(
                    "usage: cargo xtask workflows [--check]".to_string(),
                )),
            }
        }
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        _ => Err(XtaskError::Usage(format!("unknown command `{command}`"))),
    }
}

fn print_help() {
    println!(
        "\
Usage: cargo xtask <command>

Commands:
  run               Run the native Frame app
  build             Build frame-app
  bundle macos      Build the macOS .app and .dmg package
  bundle linux      Build the Linux tarball package
  bundle linux --all Build Linux tarball, AppImage, and Flatpak test packages
  bundle windows    Build the Windows Inno Setup installer
  setup-ffmpeg      Download FFmpeg and FFprobe runtime binaries
  update-manifest   Generate a signed-update manifest from release artifacts
  sign-update-manifest Sign update-manifest.json with FRAME_UPDATE_SIGNING_KEY
  flathub-sources   Create Flathub source and cargo-vendor release archives
  flathub-manifest  Render Flathub repository manifest files
  ci                Run local formatting, tests, lints, and script checks
  workflows         Regenerate GitHub Actions workflows
  workflows --check Verify checked-in workflows match the generator exactly
"
    );
}

fn run_frame_app(args: Vec<String>) -> Result<()> {
    let mut cargo_args = vec![
        "run".to_string(),
        "--manifest-path".to_string(),
        "frame-app/Cargo.toml".to_string(),
    ];
    cargo_args.extend(args);
    run_command_path("cargo", &cargo_args)
}

fn build_frame_app(args: Vec<String>) -> Result<()> {
    let mut cargo_args = vec![
        "build".to_string(),
        "--manifest-path".to_string(),
        "frame-app/Cargo.toml".to_string(),
    ];
    cargo_args.extend(args);
    run_command_path("cargo", &cargo_args)
}

fn bundle(args: &[String]) -> Result<()> {
    let Some(platform) = args.first() else {
        return Err(XtaskError::Usage(
            "missing bundle platform: expected macos, linux, or windows".to_string(),
        ));
    };
    let script_args = args.iter().skip(1).map(String::as_str).collect::<Vec<_>>();

    match platform.as_str() {
        "macos" | "mac" | "darwin" => run_script("./script/bundle-mac", &script_args),
        "linux" => run_script("./script/bundle-linux", &script_args),
        "windows" | "win" => run_script("./script/bundle-windows.ps1", &script_args),
        other => Err(XtaskError::Usage(format!(
            "unknown bundle platform `{other}`"
        ))),
    }
}

fn setup_ffmpeg(args: &[String]) -> Result<()> {
    let options = SetupFfmpegOptions::parse(args)?;
    let target = ffmpeg_target_for(
        options
            .platform
            .as_deref()
            .unwrap_or_else(|| host_platform()),
        options.arch.as_deref().unwrap_or(host_arch()),
    )?;
    let binary_dir = repo_root()?.join("frame-app/resources/binaries");

    fs::create_dir_all(&binary_dir)?;
    println!(
        "Detected {}. Preparing FFmpeg {FFMPEG_VERSION} binaries...",
        target.label()
    );

    match target {
        FfmpegTarget::Individual { binaries, .. } => {
            for entry in binaries {
                process_ffmpeg_entry(entry, &binary_dir, options.force)?;
            }
        }
        FfmpegTarget::SharedArchive {
            archive, entries, ..
        } => {
            process_ffmpeg_shared_archive(archive, entries, &binary_dir, options.force)?;
        }
    }

    println!("All binaries are ready in frame-app/resources/binaries.");
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct SetupFfmpegOptions {
    force: bool,
    platform: Option<String>,
    arch: Option<String>,
}

impl SetupFfmpegOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut force = false;
        let mut platform = None;
        let mut arch = None;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--force" => {
                    force = true;
                    index += 1;
                }
                "--platform" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(XtaskError::Usage(
                            "missing value for --platform".to_string(),
                        ));
                    };
                    platform = Some(value.clone());
                    index += 2;
                }
                "--arch" => {
                    let Some(value) = args.get(index + 1) else {
                        return Err(XtaskError::Usage("missing value for --arch".to_string()));
                    };
                    arch = Some(value.clone());
                    index += 2;
                }
                "-h" | "--help" => {
                    println!(
                        "\
Usage: cargo xtask setup-ffmpeg [options]

Options:
  --force              Re-download binaries even when they already exist
  --platform <name>    Override platform: darwin, linux, or win32
  --arch <name>        Override architecture: x64, x86_64, arm64, or aarch64
"
                    );
                    return Err(XtaskError::Help);
                }
                other => {
                    return Err(XtaskError::Usage(format!(
                        "unknown setup-ffmpeg option `{other}`"
                    )));
                }
            }
        }

        Ok(Self {
            force,
            platform,
            arch,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FfmpegBinaryEntry {
    id: &'static str,
    archive: Option<FfmpegArchive>,
    expected_names: &'static [&'static str],
    destination_name: &'static str,
    sha256: &'static str,
    make_executable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FfmpegArchive {
    url: &'static str,
    sha256: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FfmpegTarget {
    Individual {
        label: &'static str,
        binaries: &'static [FfmpegBinaryEntry],
    },
    SharedArchive {
        label: &'static str,
        archive: FfmpegArchive,
        entries: &'static [FfmpegBinaryEntry],
    },
}

impl FfmpegTarget {
    const fn label(&self) -> &'static str {
        match self {
            Self::Individual { label, .. } | Self::SharedArchive { label, .. } => label,
        }
    }
}

const MACOS_X86_64_BINARIES: &[FfmpegBinaryEntry] = &[
    FfmpegBinaryEntry {
        id: "ffmpeg",
        archive: Some(FfmpegArchive {
            url: "https://ffmpeg.martin-riedl.de/download/macos/amd64/1783018342_8.1.2/ffmpeg.zip",
            sha256: "a52ef43883f44c219766d4b3bdde4e635b35465d0b704c01c3a0566b59775df9",
        }),
        expected_names: &["ffmpeg"],
        destination_name: "ffmpeg-x86_64-apple-darwin",
        sha256: "1ca59dda73668c59898a0b305afd8a88817a989187f222ec62d64e775d614d23",
        make_executable: true,
    },
    FfmpegBinaryEntry {
        id: "ffprobe",
        archive: Some(FfmpegArchive {
            url: "https://ffmpeg.martin-riedl.de/download/macos/amd64/1783018342_8.1.2/ffprobe.zip",
            sha256: "5408ca588c8c72b0dde3afe676d0a7acf25ef97e55ae6eba5c7bede1cda42695",
        }),
        expected_names: &["ffprobe"],
        destination_name: "ffprobe-x86_64-apple-darwin",
        sha256: "bdb6aff0f1f414382effd97040f7862dc85e67996ac296cb4288beed0e06498f",
        make_executable: true,
    },
];

const MACOS_AARCH64_BINARIES: &[FfmpegBinaryEntry] = &[
    FfmpegBinaryEntry {
        id: "ffmpeg",
        archive: Some(FfmpegArchive {
            url: "https://ffmpeg.martin-riedl.de/download/macos/arm64/1783011502_8.1.2/ffmpeg.zip",
            sha256: "ef1aa60006c7b77ce170c1608c08d8e4ba1c30c5746f2ac986ded932d0ac2c3c",
        }),
        expected_names: &["ffmpeg"],
        destination_name: "ffmpeg-aarch64-apple-darwin",
        sha256: "eaf91238e104dd0e262bc6510e25061855cc99a6955a721b0ac99660d58c473d",
        make_executable: true,
    },
    FfmpegBinaryEntry {
        id: "ffprobe",
        archive: Some(FfmpegArchive {
            url: "https://ffmpeg.martin-riedl.de/download/macos/arm64/1783011502_8.1.2/ffprobe.zip",
            sha256: "c39787f4af7a3932502d2d48db6f6feaaa836b48a73ef78c32cc3285df61dfaf",
        }),
        expected_names: &["ffprobe"],
        destination_name: "ffprobe-aarch64-apple-darwin",
        sha256: "ed9dc5871914b466b96b402c9ec0ba68ce4f836e72faa464b1b4e279835bd4a6",
        make_executable: true,
    },
];

const LINUX_X86_64_BINARIES: &[FfmpegBinaryEntry] = &[
    FfmpegBinaryEntry {
        id: "ffmpeg",
        archive: Some(FfmpegArchive {
            url: "https://ffmpeg.martin-riedl.de/download/linux/amd64/1783011670_8.1.2/ffmpeg.zip",
            sha256: "56452c0bfc4ee0325cd615d62f46ba8264f62eed34f727c2224c6c84fa7b8719",
        }),
        expected_names: &["ffmpeg"],
        destination_name: "ffmpeg-x86_64-unknown-linux-gnu",
        sha256: "bea0dfb96f7223b1be497cf11ccda9ddd9a39103b948b342bb6db1c60a56be12",
        make_executable: true,
    },
    FfmpegBinaryEntry {
        id: "ffprobe",
        archive: Some(FfmpegArchive {
            url: "https://ffmpeg.martin-riedl.de/download/linux/amd64/1783011670_8.1.2/ffprobe.zip",
            sha256: "c6f2d36e98f9a4445fad0b0be539f4c4faf13fd502116bf131becd53f56cd390",
        }),
        expected_names: &["ffprobe"],
        destination_name: "ffprobe-x86_64-unknown-linux-gnu",
        sha256: "f0a9c3c87d45fe323ae893fe9820150a46f5af9fc5f75066712097f160befac5",
        make_executable: true,
    },
];

const LINUX_AARCH64_BINARIES: &[FfmpegBinaryEntry] = &[
    FfmpegBinaryEntry {
        id: "ffmpeg",
        archive: Some(FfmpegArchive {
            url: "https://ffmpeg.martin-riedl.de/download/linux/arm64/1783010599_8.1.2/ffmpeg.zip",
            sha256: "ab9e16864b6bf4ae7e13bbdbdc29621be11a5c547c57af8d4250e9fa2f5e6461",
        }),
        expected_names: &["ffmpeg"],
        destination_name: "ffmpeg-aarch64-unknown-linux-gnu",
        sha256: "93a3684e7467d33881f8fa39e3b8408248d4f95fb2e9f6b18383edcdbd70f163",
        make_executable: true,
    },
    FfmpegBinaryEntry {
        id: "ffprobe",
        archive: Some(FfmpegArchive {
            url: "https://ffmpeg.martin-riedl.de/download/linux/arm64/1783010599_8.1.2/ffprobe.zip",
            sha256: "fb78317b81cdeb614533be59e489019b754afd199670666af28f0e9574be395b",
        }),
        expected_names: &["ffprobe"],
        destination_name: "ffprobe-aarch64-unknown-linux-gnu",
        sha256: "7a4103c64cd78c7c634a5610ea3ae5dd3a97b3714cc831407c668decf6a34c6d",
        make_executable: true,
    },
];

// BtbN retains month-end snapshots for two years; ordinary daily builds expire after 14 days.
const WINDOWS_X86_64_ARCHIVE: FfmpegArchive = FfmpegArchive {
    url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-06-30-13-34/ffmpeg-n8.1.2-21-gce3c09c101-win64-gpl-8.1.zip",
    sha256: "682361e32c9631caec09e5d9f09077101c9ed90c14e275f62014fefa6d397990",
};

const WINDOWS_X86_64_BINARIES: &[FfmpegBinaryEntry] = &[
    FfmpegBinaryEntry {
        id: "ffmpeg",
        archive: None,
        expected_names: &["ffmpeg.exe"],
        destination_name: "ffmpeg-x86_64-pc-windows-msvc.exe",
        sha256: "c47e9e15e76897778915ba16e36c8002b0a3f2f9e7c0a71aa1d41459ac1d02d1",
        make_executable: false,
    },
    FfmpegBinaryEntry {
        id: "ffprobe",
        archive: None,
        expected_names: &["ffprobe.exe"],
        destination_name: "ffprobe-x86_64-pc-windows-msvc.exe",
        sha256: "2864c7a71b820b07d3a9666bb4389c8af4bb9449876b07a75b3b7f15adbdafaa",
        make_executable: false,
    },
];

fn ffmpeg_target_for(platform: &str, arch: &str) -> Result<FfmpegTarget> {
    match (platform, arch) {
        ("darwin", "x64" | "x86_64") => {
            Ok(martin_ffmpeg_target("macOS (Intel)", MACOS_X86_64_BINARIES))
        }
        ("darwin", "arm64" | "aarch64") => Ok(martin_ffmpeg_target(
            "macOS (Apple Silicon)",
            MACOS_AARCH64_BINARIES,
        )),
        ("linux", "x64" | "x86_64" | "amd64") => {
            Ok(martin_ffmpeg_target("Linux x86_64", LINUX_X86_64_BINARIES))
        }
        ("linux", "arm64" | "aarch64") => {
            Ok(martin_ffmpeg_target("Linux ARM64", LINUX_AARCH64_BINARIES))
        }
        ("win32" | "windows", "x64" | "x86_64") => Ok(windows_ffmpeg_target()),
        _ => Err(XtaskError::Usage(format!(
            "unsupported platform or architecture: {platform}/{arch}"
        ))),
    }
}

const fn martin_ffmpeg_target(
    label: &'static str,
    binaries: &'static [FfmpegBinaryEntry],
) -> FfmpegTarget {
    FfmpegTarget::Individual { label, binaries }
}

const fn windows_ffmpeg_target() -> FfmpegTarget {
    FfmpegTarget::SharedArchive {
        label: "Windows x86_64",
        archive: WINDOWS_X86_64_ARCHIVE,
        entries: WINDOWS_X86_64_BINARIES,
    }
}

fn required_option_value(args: &[String], index: &mut usize, flag: &str) -> Result<String> {
    let Some(value) = args.get(*index + 1) else {
        return Err(XtaskError::Usage(format!("missing value for {flag}")));
    };
    if value.starts_with("--") {
        return Err(XtaskError::Usage(format!("missing value for {flag}")));
    }
    *index += 2;
    Ok(value.clone())
}

fn update_manifest(args: &[String]) -> Result<()> {
    let options = UpdateManifestOptions::parse(args)?;
    let mut assets = BTreeMap::new();

    for artifact in &options.artifacts {
        let file_name = artifact
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                XtaskError::Usage(format!(
                    "artifact path has no file name: {}",
                    artifact.path.display()
                ))
            })?
            .to_string();
        let metadata = fs::metadata(&artifact.path)?;
        if !metadata.is_file() {
            return Err(XtaskError::Usage(format!(
                "artifact is not a file: {}",
                artifact.path.display()
            )));
        }

        assets.insert(
            artifact.platform.as_str().to_string(),
            UpdateAsset {
                target_triple: artifact.platform.target_triple().to_string(),
                kind: artifact.kind,
                file_name: file_name.clone(),
                url: options.asset_url(&file_name),
                size_bytes: metadata.len(),
                sha256: file_sha256_hex(&artifact.path)?,
                installer_args: installer_args_for(artifact.kind),
            },
        );
    }

    let manifest = UpdateManifest {
        schema_version: 1,
        app_id: "Frame".to_string(),
        channel: options.channel,
        version: options.version,
        published_at: options.published_at,
        min_supported_version: options.min_supported_version,
        release_notes_url: options.release_notes_url.or_else(|| {
            Some(format!(
                "https://github.com/66HEX/frame/releases/tag/{}",
                options.release_tag
            ))
        }),
        release_notes_markdown: options.release_notes_markdown,
        assets,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    write_atomic(&options.out, &bytes)?;
    println!("Created {}", options.out.display());

    Ok(())
}

fn sign_update_manifest(args: &[String]) -> Result<()> {
    let options = SignUpdateManifestOptions::parse(args)?;
    let signing_key = env::var("FRAME_UPDATE_SIGNING_KEY").map_err(|_| {
        XtaskError::Usage(
            "FRAME_UPDATE_SIGNING_KEY must contain the base64 Ed25519 seed".to_string(),
        )
    })?;
    let manifest_bytes = fs::read(&options.manifest)?;
    let signature = sign_manifest_bytes(&manifest_bytes, &signing_key)?;
    write_atomic(&options.out, signature.as_bytes())?;
    println!("Created {}", options.out.display());

    Ok(())
}

fn flathub_sources(args: &[String]) -> Result<()> {
    let options = FlathubSourcesOptions::parse(args)?;
    let root = repo_root()?;
    let output_dir = root.join(&options.out_dir);
    let source_archive = output_dir.join(format!("frame-{}-source.tar.gz", options.version));
    let vendor_dir = output_dir.join("cargo-vendor");
    let vendor_archive = output_dir.join(format!("frame-{}-cargo-vendor.tar.gz", options.version));
    let checksums_path = output_dir.join("checksums.env");

    fs::create_dir_all(&output_dir)?;
    if vendor_dir.exists() {
        fs::remove_dir_all(&vendor_dir)?;
    }
    for archive in [&source_archive, &vendor_archive] {
        if archive.exists() {
            fs::remove_file(archive)?;
        }
    }

    run_command_path(
        "git",
        &[
            "archive".to_string(),
            "--format=tar.gz".to_string(),
            format!("--prefix=frame-{}/", options.version),
            "-o".to_string(),
            path_to_string_lossy(&source_archive),
            "HEAD".to_string(),
        ],
    )?;
    let vendor_config = run_command_capture(
        "cargo",
        &[
            "vendor".to_string(),
            "--locked".to_string(),
            path_to_string_lossy(&vendor_dir),
        ],
    )?;
    let normalized_vendor_config =
        vendor_config.replace(&path_to_string_lossy(&vendor_dir), "cargo-vendor");
    fs::write(
        vendor_dir.join("cargo-config.toml"),
        normalized_vendor_config,
    )?;
    run_command_path(
        "tar",
        &[
            "-czf".to_string(),
            path_to_string_lossy(&vendor_archive),
            "-C".to_string(),
            path_to_string_lossy(&vendor_dir),
            ".".to_string(),
        ],
    )?;

    let source_sha256 = file_sha256_hex(&source_archive)?;
    let vendor_sha256 = file_sha256_hex(&vendor_archive)?;
    fs::write(
        &checksums_path,
        format!(
            "FRAME_SOURCE_ARCHIVE={}\nFRAME_SOURCE_SHA256={source_sha256}\nFRAME_CARGO_VENDOR_ARCHIVE={}\nFRAME_CARGO_VENDOR_SHA256={vendor_sha256}\n",
            source_archive
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or_default(),
            vendor_archive
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or_default(),
        ),
    )?;

    println!("Wrote {}", source_archive.display());
    println!("Wrote {}", vendor_archive.display());
    println!("Wrote {}", checksums_path.display());
    Ok(())
}

fn flathub_manifest(args: &[String]) -> Result<()> {
    let options = FlathubManifestOptions::parse(args)?;
    let root = repo_root()?;
    let out_dir = root.join(&options.out);
    fs::create_dir_all(&out_dir)?;

    let manifest = render_flathub_manifest_template(
        &fs::read_to_string(root.join(FLATHUB_MANIFEST_TEMPLATE_PATH))?,
        &options,
    );
    let metainfo = fs::read_to_string(root.join(FLATHUB_METAINFO_PATH))?;
    validate_flathub_metainfo_version(&metainfo, &options.version)?;

    fs::write(out_dir.join("io.github._66HEX.Frame.yml"), manifest)?;
    fs::write(
        out_dir.join("io.github._66HEX.Frame.metainfo.xml"),
        metainfo,
    )?;
    println!("Wrote {}", out_dir.display());
    Ok(())
}

fn render_flathub_manifest_template(template: &str, options: &FlathubManifestOptions) -> String {
    template
        .replace("@@FRAME_SOURCE_URL@@", &options.source_url)
        .replace("@@FRAME_SOURCE_SHA256@@", &options.source_sha256)
        .replace("@@FRAME_CARGO_VENDOR_URL@@", &options.vendor_url)
        .replace("@@FRAME_CARGO_VENDOR_SHA256@@", &options.vendor_sha256)
}

fn validate_flathub_metainfo_version(metainfo: &str, version: &str) -> Result<()> {
    let release_prefix = format!(r#"<release version="{version}" date=""#);
    if metainfo.contains(&release_prefix) {
        return Ok(());
    }

    Err(XtaskError::Usage(format!(
        "{FLATHUB_METAINFO_PATH} does not contain release metadata for version {version}"
    )))
}

fn path_to_string_lossy(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[derive(Clone, Debug)]
struct FlathubSourcesOptions {
    version: String,
    out_dir: PathBuf,
}

impl FlathubSourcesOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut version = None;
        let mut out_dir = PathBuf::from("target/flathub");
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--version" => {
                    version = Some(required_option_value(args, &mut index, "--version")?);
                }
                "--out-dir" => {
                    out_dir = PathBuf::from(required_option_value(args, &mut index, "--out-dir")?);
                }
                "-h" | "--help" => {
                    println!(
                        "\
Usage: cargo xtask flathub-sources --version <semver> [--out-dir <path>]
"
                    );
                    return Err(XtaskError::Help);
                }
                other => {
                    return Err(XtaskError::Usage(format!(
                        "unknown flathub-sources option `{other}`"
                    )));
                }
            }
        }

        let version = version.ok_or_else(|| XtaskError::Usage("missing --version".to_string()))?;
        validate_semver(&version, "--version")?;
        Ok(Self { version, out_dir })
    }
}

#[derive(Clone, Debug)]
struct FlathubManifestOptions {
    version: String,
    source_url: String,
    source_sha256: String,
    vendor_url: String,
    vendor_sha256: String,
    out: PathBuf,
}

impl FlathubManifestOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut version = None;
        let mut source_url = None;
        let mut source_sha256 = None;
        let mut vendor_url = None;
        let mut vendor_sha256 = None;
        let mut out = None;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--version" => {
                    version = Some(required_option_value(args, &mut index, "--version")?);
                }
                "--source-url" => {
                    source_url = Some(required_option_value(args, &mut index, "--source-url")?);
                }
                "--source-sha256" => {
                    source_sha256 =
                        Some(required_option_value(args, &mut index, "--source-sha256")?);
                }
                "--vendor-url" => {
                    vendor_url = Some(required_option_value(args, &mut index, "--vendor-url")?);
                }
                "--vendor-sha256" => {
                    vendor_sha256 =
                        Some(required_option_value(args, &mut index, "--vendor-sha256")?);
                }
                "--out" => {
                    out = Some(PathBuf::from(required_option_value(
                        args, &mut index, "--out",
                    )?));
                }
                "-h" | "--help" => {
                    println!(
                        "\
Usage: cargo xtask flathub-manifest [options]

Required:
  --version <semver>
  --source-url <url>
  --source-sha256 <sha256>
  --vendor-url <url>
  --vendor-sha256 <sha256>
  --out <path>
"
                    );
                    return Err(XtaskError::Help);
                }
                other => {
                    return Err(XtaskError::Usage(format!(
                        "unknown flathub-manifest option `{other}`"
                    )));
                }
            }
        }

        let options = Self {
            version: version.ok_or_else(|| XtaskError::Usage("missing --version".to_string()))?,
            source_url: source_url
                .ok_or_else(|| XtaskError::Usage("missing --source-url".to_string()))?,
            source_sha256: source_sha256
                .ok_or_else(|| XtaskError::Usage("missing --source-sha256".to_string()))?,
            vendor_url: vendor_url
                .ok_or_else(|| XtaskError::Usage("missing --vendor-url".to_string()))?,
            vendor_sha256: vendor_sha256
                .ok_or_else(|| XtaskError::Usage("missing --vendor-sha256".to_string()))?,
            out: out.ok_or_else(|| XtaskError::Usage("missing --out".to_string()))?,
        };
        validate_semver(&options.version, "--version")?;
        validate_sha256(&options.source_sha256, "--source-sha256")?;
        validate_sha256(&options.vendor_sha256, "--vendor-sha256")?;
        Ok(options)
    }
}

fn validate_semver(value: &str, flag: &str) -> Result<()> {
    semver::Version::parse(value)
        .map(|_| ())
        .map_err(|error| XtaskError::Usage(format!("invalid {flag} `{value}`: {error}")))
}

fn validate_sha256(value: &str, flag: &str) -> Result<()> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(XtaskError::Usage(format!(
            "{flag} must be a 64-character SHA-256 hex digest"
        )))
    }
}

#[derive(Clone, Debug, Default)]
struct UpdateManifestOptions {
    version: String,
    channel: UpdateChannel,
    release_tag: String,
    release_notes_url: Option<String>,
    release_notes_markdown: Option<String>,
    published_at: Option<String>,
    min_supported_version: Option<String>,
    base_url: Option<String>,
    artifacts: Vec<ReleaseArtifactSpec>,
    out: PathBuf,
}

impl UpdateManifestOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut options = Self::default();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--version" => {
                    options.version = required_option_value(args, &mut index, "--version")?;
                }
                "--channel" => {
                    options.channel =
                        required_option_value(args, &mut index, "--channel")?.parse()?;
                }
                "--release-tag" => {
                    options.release_tag = required_option_value(args, &mut index, "--release-tag")?;
                }
                "--release-notes-url" => {
                    options.release_notes_url = Some(required_option_value(
                        args,
                        &mut index,
                        "--release-notes-url",
                    )?);
                }
                "--release-notes-markdown" => {
                    options.release_notes_markdown = Some(required_option_value(
                        args,
                        &mut index,
                        "--release-notes-markdown",
                    )?);
                }
                "--published-at" => {
                    options.published_at =
                        Some(required_option_value(args, &mut index, "--published-at")?);
                }
                "--min-supported-version" => {
                    options.min_supported_version = Some(required_option_value(
                        args,
                        &mut index,
                        "--min-supported-version",
                    )?);
                }
                "--base-url" => {
                    options.base_url = Some(required_option_value(args, &mut index, "--base-url")?);
                }
                "--artifact" => {
                    let spec = required_option_value(args, &mut index, "--artifact")?;
                    options.artifacts.push(ReleaseArtifactSpec::parse(&spec)?);
                }
                "--out" => {
                    options.out = PathBuf::from(required_option_value(args, &mut index, "--out")?);
                }
                "-h" | "--help" => {
                    println!(
                        "\
Usage: cargo xtask update-manifest [options]

Required:
  --version <semver>
  --release-tag <tag>
  --artifact <path:platformKey:assetKind>
  --out <path>

Options:
  --channel <stable>                  Defaults to stable
  --base-url <url>                    Defaults to GitHub release URL for tag
  --min-supported-version <semver>
  --release-notes-url <url>
  --release-notes-markdown <text>
  --published-at <iso8601>
"
                    );
                    return Err(XtaskError::Help);
                }
                other => {
                    return Err(XtaskError::Usage(format!(
                        "unknown update-manifest option `{other}`"
                    )));
                }
            }
        }

        if options.version.trim().is_empty() {
            return Err(XtaskError::Usage("missing --version".to_string()));
        }
        if options.release_tag.trim().is_empty() {
            return Err(XtaskError::Usage("missing --release-tag".to_string()));
        }
        if options.artifacts.is_empty() {
            return Err(XtaskError::Usage("missing --artifact".to_string()));
        }
        if options.out.as_os_str().is_empty() {
            return Err(XtaskError::Usage("missing --out".to_string()));
        }
        semver::Version::parse(&options.version).map_err(|error| {
            XtaskError::Usage(format!("invalid --version `{}`: {error}", options.version))
        })?;
        if let Some(min_supported_version) = &options.min_supported_version {
            semver::Version::parse(min_supported_version).map_err(|error| {
                XtaskError::Usage(format!(
                    "invalid --min-supported-version `{min_supported_version}`: {error}"
                ))
            })?;
        }

        Ok(options)
    }

    fn asset_url(&self, file_name: &str) -> String {
        let base_url = self.base_url.clone().unwrap_or_else(|| {
            format!(
                "https://github.com/66HEX/frame/releases/download/{}",
                self.release_tag
            )
        });
        format!("{}/{file_name}", base_url.trim_end_matches('/'))
    }
}

#[derive(Clone, Debug)]
struct ReleaseArtifactSpec {
    path: PathBuf,
    platform: PlatformAssetKey,
    kind: UpdateAssetKind,
}

impl ReleaseArtifactSpec {
    fn parse(value: &str) -> Result<Self> {
        let mut parts = value.rsplitn(3, ':');
        let kind = parts
            .next()
            .ok_or_else(|| XtaskError::Usage(format!("invalid artifact spec `{value}`")))?
            .parse::<UpdateAssetKind>()?;
        let platform = parse_platform_asset_key(
            parts
                .next()
                .ok_or_else(|| XtaskError::Usage(format!("invalid artifact spec `{value}`")))?,
        )?;
        let path = PathBuf::from(
            parts
                .next()
                .ok_or_else(|| XtaskError::Usage(format!("invalid artifact spec `{value}`")))?,
        );

        if platform.asset_kind() != kind {
            return Err(XtaskError::Usage(format!(
                "artifact kind `{kind}` does not match platform `{}`",
                platform.as_str()
            )));
        }

        Ok(Self {
            path,
            platform,
            kind,
        })
    }
}

#[derive(Clone, Debug)]
struct SignUpdateManifestOptions {
    manifest: PathBuf,
    out: PathBuf,
}

impl SignUpdateManifestOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut manifest = None;
        let mut out = None;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(required_option_value(
                        args,
                        &mut index,
                        "--manifest",
                    )?));
                }
                "--out" => {
                    out = Some(PathBuf::from(required_option_value(
                        args, &mut index, "--out",
                    )?));
                }
                "-h" | "--help" => {
                    println!(
                        "\
Usage: cargo xtask sign-update-manifest --manifest <path> --out <path>

Requires FRAME_UPDATE_SIGNING_KEY to contain the base64 Ed25519 seed.
"
                    );
                    return Err(XtaskError::Help);
                }
                other => {
                    return Err(XtaskError::Usage(format!(
                        "unknown sign-update-manifest option `{other}`"
                    )));
                }
            }
        }

        Ok(Self {
            manifest: manifest
                .ok_or_else(|| XtaskError::Usage("missing --manifest".to_string()))?,
            out: out.ok_or_else(|| XtaskError::Usage("missing --out".to_string()))?,
        })
    }
}

fn parse_platform_asset_key(value: &str) -> Result<PlatformAssetKey> {
    match value {
        "macos-aarch64" => Ok(PlatformAssetKey::MacosAarch64),
        "macos-x86_64" => Ok(PlatformAssetKey::MacosX8664),
        "windows-x86_64" => Ok(PlatformAssetKey::WindowsX8664),
        "linux-x86_64" => Ok(PlatformAssetKey::LinuxX8664),
        "linux-aarch64" => Ok(PlatformAssetKey::LinuxAarch64),
        other => Err(XtaskError::Usage(format!(
            "unsupported platform asset key `{other}`"
        ))),
    }
}

fn installer_args_for(kind: UpdateAssetKind) -> Vec<String> {
    match kind {
        UpdateAssetKind::WindowsInno => vec![
            "/SP-".to_string(),
            "/VERYSILENT".to_string(),
            "/SUPPRESSMSGBOXES".to_string(),
            "/NORESTART".to_string(),
        ],
        UpdateAssetKind::MacosAppZip | UpdateAssetKind::LinuxManagedTar => Vec::new(),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, bytes)?;
    match fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(_) if path.exists() => {
            fs::remove_file(path)?;
            fs::rename(&temp_path, path)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn process_ffmpeg_entry(entry: &FfmpegBinaryEntry, binary_dir: &Path, force: bool) -> Result<()> {
    let destination = binary_dir.join(entry.destination_name);
    if !ffmpeg_binary_needs_download(entry, &destination, force)? {
        return Ok(());
    }

    let Some(archive) = entry.archive else {
        return Err(XtaskError::Usage(format!(
            "missing download URL for {}",
            entry.id
        )));
    };

    println!("Downloading {} from {}...", entry.id, archive.url);
    let archive_bytes = download_file(archive.url)?;
    verify_bytes_sha256(&archive_bytes, archive.sha256, archive.url)?;
    extract_expected_file(&archive_bytes, entry, &destination)
}

fn process_ffmpeg_shared_archive(
    archive: FfmpegArchive,
    entries: &[FfmpegBinaryEntry],
    binary_dir: &Path,
    force: bool,
) -> Result<()> {
    let destinations = entries
        .iter()
        .map(|entry| binary_dir.join(entry.destination_name))
        .collect::<Vec<_>>();
    let needs_download = entries
        .iter()
        .zip(&destinations)
        .map(|(entry, destination)| ffmpeg_binary_needs_download(entry, destination, force))
        .collect::<Result<Vec<_>>>()?;

    if !needs_download.iter().any(|needs_download| *needs_download) {
        return Ok(());
    }

    println!("Downloading Windows bundle from {}...", archive.url);
    let archive_bytes = download_file(archive.url)?;
    verify_bytes_sha256(&archive_bytes, archive.sha256, archive.url)?;

    for ((entry, destination), needs_download) in
        entries.iter().zip(&destinations).zip(needs_download)
    {
        if !needs_download {
            continue;
        }
        extract_expected_file(&archive_bytes, entry, destination)?;
    }

    Ok(())
}

fn ffmpeg_binary_needs_download(
    entry: &FfmpegBinaryEntry,
    destination: &Path,
    force: bool,
) -> Result<bool> {
    if force || !destination.is_file() {
        return Ok(true);
    }

    let actual = file_sha256_hex(destination)?;
    if actual == entry.sha256 {
        if entry.make_executable {
            make_file_executable(destination)?;
        }
        println!("Verified cached {} (SHA-256).", entry.destination_name);
        Ok(false)
    } else {
        println!(
            "Cached {} has an unexpected SHA-256; downloading it again.",
            entry.destination_name
        );
        Ok(true)
    }
}

fn download_file(url: &str) -> Result<Vec<u8>> {
    let response = ureq::get(url)
        .call()
        .map_err(|source| XtaskError::Download {
            url: url.to_string(),
            source: Box::new(source),
        })?;
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn verify_bytes_sha256(bytes: &[u8], expected: &str, subject: &str) -> Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    verify_sha256(&actual, expected, subject)
}

fn verify_file_sha256(path: &Path, expected: &str) -> Result<()> {
    let actual = file_sha256_hex(path)?;
    verify_sha256(&actual, expected, &path.display().to_string())
}

fn verify_sha256(actual: &str, expected: &str, subject: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(XtaskError::HashMismatch {
            subject: subject.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

fn extract_expected_file(
    archive_bytes: &[u8],
    entry: &FfmpegBinaryEntry,
    destination: &Path,
) -> Result<()> {
    let reader = Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(reader)?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        if file.is_file() && archive_entry_name_matches(file.name(), entry.expected_names) {
            write_archive_file(&mut file, destination, entry.sha256, entry.make_executable)?;
            println!("Placed {}.", entry.destination_name);
            return Ok(());
        }
    }

    Err(XtaskError::ArchiveEntryMissing {
        expected_names: entry.expected_names.join(", "),
    })
}

fn archive_entry_name_matches(name: &str, expected_names: &[&str]) -> bool {
    let file_name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    expected_names.contains(&file_name)
}

fn write_archive_file(
    reader: &mut impl Read,
    destination: &Path,
    expected_sha256: &str,
    make_executable: bool,
) -> Result<()> {
    let Some(file_name) = destination.file_name().and_then(|name| name.to_str()) else {
        return Err(XtaskError::Usage(format!(
            "invalid destination path `{}`",
            destination.display()
        )));
    };

    let temporary_destination = destination.with_file_name(format!(".{file_name}.download"));
    {
        let mut output = fs::File::create(&temporary_destination)?;
        io::copy(reader, &mut output)?;
    }

    if let Err(error) = verify_file_sha256(&temporary_destination, expected_sha256) {
        let _ = fs::remove_file(&temporary_destination);
        return Err(error);
    }

    if make_executable {
        make_file_executable(&temporary_destination)?;
    }

    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary_destination, destination)?;
    Ok(())
}

#[cfg(unix)]
fn make_file_executable(path: &Path) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_file_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn host_platform() -> &'static str {
    match env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

const fn host_arch() -> &'static str {
    env::consts::ARCH
}

fn ci() -> Result<()> {
    const MANIFESTS: [&str; 5] = [
        "frame-core/Cargo.toml",
        "frame-app/Cargo.toml",
        "frame-updater/Cargo.toml",
        "tooling/manifest-signer/Cargo.toml",
        "tooling/xtask/Cargo.toml",
    ];

    for manifest in MANIFESTS {
        run_command("cargo", &["fmt", "--manifest-path", manifest, "--check"])?;
    }
    for manifest in MANIFESTS {
        run_command("cargo", &["test", "--manifest-path", manifest])?;
    }
    for manifest in MANIFESTS {
        run_command(
            "cargo",
            &[
                "clippy",
                "--manifest-path",
                manifest,
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
        )?;
    }
    run_command("bash", &["-n", "script/bundle-mac"])?;
    run_command("bash", &["-n", "script/bundle-linux"])?;
    check_workflows()?;
    run_command("git", &["diff", "--check"])?;
    Ok(())
}

fn write_workflows() -> Result<()> {
    let root = repo_root()?;
    for (relative_path, content) in generated_workflows() {
        let path = root.join(relative_path);
        fs::create_dir_all(path.parent().expect("workflow path should have a parent"))?;
        fs::write(&path, content)?;
        println!("Wrote {}", path.display());
    }
    Ok(())
}

fn check_workflows() -> Result<()> {
    let root = repo_root()?;
    for (relative_path, expected) in generated_workflows() {
        let path = root.join(relative_path);
        let actual = fs::read_to_string(&path).map_err(|error| {
            XtaskError::Usage(format!(
                "failed to read generated workflow `{}`: {error}",
                path.display()
            ))
        })?;
        if actual != expected {
            return Err(XtaskError::GeneratedWorkflowOutOfDate {
                path: relative_path.to_string(),
            });
        }
    }
    println!("Checked-in GitHub Actions workflows match the xtask generator.");
    Ok(())
}

fn generated_workflows() -> Vec<(&'static str, String)> {
    vec![
        (CI_WORKFLOW_PATH, ci_workflow()),
        (
            DEPENDENCY_REVIEW_WORKFLOW_PATH,
            dependency_review_workflow(),
        ),
        (CODEQL_WORKFLOW_PATH, codeql_workflow()),
        (RUN_BUNDLING_WORKFLOW_PATH, run_bundling_workflow()),
        (RELEASE_WORKFLOW_PATH, release_workflow()),
        (PUBLISH_NIXPKGS_WORKFLOW_PATH, publish_nixpkgs_workflow()),
    ]
}

fn run_script(script: &str, args: &[&str]) -> Result<()> {
    let is_powershell_script = Path::new(script)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ps1"));

    if is_powershell_script && !cfg!(target_os = "windows") {
        return Err(XtaskError::Usage(
            "Windows bundles must be built on Windows.".to_string(),
        ));
    }

    if cfg!(target_os = "windows") && is_powershell_script {
        let mut command_args = vec!["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script];
        command_args.extend_from_slice(args);
        run_command("powershell.exe", &command_args)
    } else {
        let mut command_args = Vec::with_capacity(args.len() + 1);
        command_args.push(script);
        command_args.extend_from_slice(args);
        run_command("bash", &command_args)
    }
}

fn run_command(program: &str, args: &[&str]) -> Result<()> {
    let root = repo_root()?;
    println!("$ {} {}", program, args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|source| XtaskError::CommandSpawn {
            program: program.to_string(),
            source,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(XtaskError::CommandFailed {
            program: program.to_string(),
            status,
        })
    }
}

fn run_command_path(program: impl AsRef<OsStr>, args: &[String]) -> Result<()> {
    let program = program.as_ref();
    println!(
        "$ {} {}",
        program.to_string_lossy(),
        args.iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    );
    let status = Command::new(program)
        .args(args)
        .current_dir(repo_root()?)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|source| XtaskError::CommandSpawn {
            program: program.to_string_lossy().into_owned(),
            source,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(XtaskError::CommandFailed {
            program: program.to_string_lossy().into_owned(),
            status,
        })
    }
}

fn run_command_capture(program: &str, args: &[String]) -> Result<String> {
    println!("$ {} {}", program, args.join(" "));
    let output = Command::new(program)
        .args(args)
        .current_dir(repo_root()?)
        .output()
        .map_err(|source| XtaskError::CommandSpawn {
            program: program.to_string(),
            source,
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        eprint!("{}", String::from_utf8_lossy(&output.stdout));
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        Err(XtaskError::CommandFailed {
            program: program.to_string(),
            status: output.status,
        })
    }
}

fn render_workflow(template: &str) -> String {
    let replacements = [
        ("__RUST_VERSION__", RUST_VERSION),
        ("__CARGO_BUNDLE_VERSION__", CARGO_BUNDLE_VERSION),
        ("__CARGO_DENY_VERSION__", CARGO_DENY_VERSION),
        ("__CARGO_AUDIT_VERSION__", CARGO_AUDIT_VERSION),
        ("__CARGO_CYCLONEDX_VERSION__", CARGO_CYCLONEDX_VERSION),
        ("__INNO_SETUP_VERSION__", INNO_SETUP_VERSION),
        ("__NIX_VERSION__", NIX_VERSION),
        ("__APPIMAGETOOL_VERSION__", APPIMAGETOOL_VERSION),
        ("__APPIMAGETOOL_X86_64_SHA256__", APPIMAGETOOL_X86_64_SHA256),
        (
            "__APPIMAGETOOL_AARCH64_SHA256__",
            APPIMAGETOOL_AARCH64_SHA256,
        ),
        (
            "__TAURI_BRIDGE_LATEST_JSON_SHA256__",
            TAURI_BRIDGE_LATEST_JSON_SHA256,
        ),
        ("__KOMAC_VERSION__", KOMAC_VERSION),
        ("__KOMAC_LINUX_X86_64_SHA256__", KOMAC_LINUX_X86_64_SHA256),
        ("__ACTIONLINT_VERSION__", ACTIONLINT_VERSION),
        (
            "__ACTIONLINT_LINUX_X86_64_SHA256__",
            ACTIONLINT_LINUX_X86_64_SHA256,
        ),
    ];
    let mut workflow = template.to_string();
    for (placeholder, value) in replacements {
        workflow = workflow.replace(placeholder, value);
    }

    pin_workflow_tools(&mut workflow);
    pin_workflow_actions(&mut workflow);
    disable_checkout_credentials(&workflow)
}

fn pin_workflow_tools(workflow: &mut String) {
    let rust_install = format!(
        "      run: rustup toolchain install {RUST_VERSION} --profile minimal --component clippy,rustfmt"
    );
    *workflow = workflow.replace("      uses: dtolnay/rust-toolchain@stable", &rust_install);
    *workflow = workflow.replace(
        &rust_install,
        &format!(
            "      run: |\n        rustup toolchain install {RUST_VERSION} --profile minimal --component clippy,rustfmt\n        rustup default {RUST_VERSION}\n        test \"$(rustc --version | awk '{{print $2}}')\" = \"{RUST_VERSION}\""
        ),
    );
    *workflow = workflow.replace(
        "cargo install cargo-bundle --locked",
        &format!("cargo install cargo-bundle --version {CARGO_BUNDLE_VERSION} --locked"),
    );
    *workflow = workflow.replace(
        "choco install innosetup --no-progress -y",
        &format!("choco install innosetup --version {INNO_SETUP_VERSION} --no-progress --yes"),
    );
}

fn pin_workflow_actions(workflow: &mut String) {
    let action_replacements = [
        (
            "actions/checkout@v4",
            format!("actions/checkout@{ACTION_CHECKOUT_SHA} # v7.0.0"),
        ),
        (
            "actions/checkout@v7.0.0",
            format!("actions/checkout@{ACTION_CHECKOUT_SHA} # v7.0.0"),
        ),
        (
            "actions/upload-artifact@v4",
            format!("actions/upload-artifact@{ACTION_UPLOAD_ARTIFACT_SHA} # v7.0.1"),
        ),
        (
            "actions/upload-artifact@v7.0.1",
            format!("actions/upload-artifact@{ACTION_UPLOAD_ARTIFACT_SHA} # v7.0.1"),
        ),
        (
            "actions/download-artifact@v4",
            format!("actions/download-artifact@{ACTION_DOWNLOAD_ARTIFACT_SHA} # v8.0.1"),
        ),
        (
            "actions/download-artifact@v8.0.1",
            format!("actions/download-artifact@{ACTION_DOWNLOAD_ARTIFACT_SHA} # v8.0.1"),
        ),
        (
            "actions/dependency-review-action@v5.0.0",
            format!("actions/dependency-review-action@{ACTION_DEPENDENCY_REVIEW_SHA} # v5.0.0"),
        ),
        (
            "actions/attest-build-provenance@v4.1.1",
            format!(
                "actions/attest-build-provenance@{ACTION_ATTEST_BUILD_PROVENANCE_SHA} # v4.1.1"
            ),
        ),
        (
            "Apple-Actions/import-codesign-certs@v3",
            format!(
                "Apple-Actions/import-codesign-certs@{ACTION_IMPORT_CODESIGN_CERTS_SHA} # v7.0.0"
            ),
        ),
        (
            "Apple-Actions/import-codesign-certs@v7.0.0",
            format!(
                "Apple-Actions/import-codesign-certs@{ACTION_IMPORT_CODESIGN_CERTS_SHA} # v7.0.0"
            ),
        ),
        (
            "cachix/install-nix-action@v31.11.0",
            format!("cachix/install-nix-action@{ACTION_INSTALL_NIX_SHA} # v31.11.0"),
        ),
        (
            "cachix/install-nix-action@v31",
            format!("cachix/install-nix-action@{ACTION_INSTALL_NIX_SHA} # v31.11.0"),
        ),
        (
            "github/codeql-action/init@v4.37.1",
            format!("github/codeql-action/init@{ACTION_CODEQL_SHA} # v4.37.1"),
        ),
        (
            "github/codeql-action/analyze@v4.37.1",
            format!("github/codeql-action/analyze@{ACTION_CODEQL_SHA} # v4.37.1"),
        ),
    ];
    for (reference, pinned) in action_replacements {
        *workflow = workflow.replace(reference, &pinned);
    }
}

fn disable_checkout_credentials(workflow: &str) -> String {
    let lines = workflow.lines().collect::<Vec<_>>();
    let mut rendered = String::with_capacity(workflow.len() + 256);
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        rendered.push_str(line);
        rendered.push('\n');

        if line.trim_start().starts_with("uses: actions/checkout@") {
            let step_indent = line.len() - line.trim_start().len();
            if lines
                .get(index + 1)
                .is_some_and(|next| next.trim() == "with:")
            {
                let with_line = lines[index + 1];
                rendered.push_str(with_line);
                rendered.push('\n');
                let with_indent = with_line.len() - with_line.trim_start().len();
                let has_setting = lines[index + 2..]
                    .iter()
                    .take_while(|candidate| {
                        let trimmed = candidate.trim();
                        trimmed.is_empty()
                            || candidate.len() - candidate.trim_start().len() > with_indent
                    })
                    .any(|candidate| candidate.trim_start().starts_with("persist-credentials:"));
                if !has_setting {
                    rendered.push_str(&" ".repeat(with_indent + 2));
                    rendered.push_str("persist-credentials: false\n");
                }
                index += 2;
                continue;
            }

            rendered.push_str(&" ".repeat(step_indent));
            rendered.push_str("with:\n");
            rendered.push_str(&" ".repeat(step_indent + 2));
            rendered.push_str("persist-credentials: false\n");
        }
        index += 1;
    }

    rendered
}

fn ci_workflow() -> String {
    render_workflow(
        r"# Generated from xtask::workflows::ci
# Rebuild with `cargo xtask workflows`.
name: ci
on:
  pull_request:
  push:
    branches:
      - master
permissions:
  contents: read
concurrency:
  group: ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true
jobs:
  cargo_xtask_ci:
    name: cargo xtask ci
    runs-on: ubuntu-22.04
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v7.0.0
      with:
        persist-credentials: false
    - name: steps::setup_rust
      run: rustup toolchain install __RUST_VERSION__ --profile minimal --component clippy,rustfmt
    - name: steps::setup_linux
      run: |
        sudo apt-get update
        sudo apt-get install -y clang desktop-file-utils libasound2-dev libdrm-dev libfontconfig1-dev libfreetype6-dev libx11-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev pkg-config
    - name: security::validate_workflow_syntax
      run: |
        mkdir -p target/actionlint
        curl --fail --location --proto '=https' --tlsv1.2 \
          --output target/actionlint/actionlint.tar.gz \
          https://github.com/rhysd/actionlint/releases/download/v__ACTIONLINT_VERSION__/actionlint___ACTIONLINT_VERSION___linux_amd64.tar.gz
        echo '__ACTIONLINT_LINUX_X86_64_SHA256__  target/actionlint/actionlint.tar.gz' | sha256sum --check --strict
        tar --extract --gzip --file target/actionlint/actionlint.tar.gz --directory target/actionlint actionlint
        target/actionlint/actionlint -color
    - name: security::install_cargo_deny
      run: cargo install cargo-deny --version __CARGO_DENY_VERSION__ --locked
    - name: security::cargo_deny
      run: cargo deny check
    - name: security::install_cargo_audit
      run: cargo install cargo-audit --version __CARGO_AUDIT_VERSION__ --locked
    - name: security::cargo_audit
      run: |
        cargo audit --deny unmaintained --deny unsound --no-yanked \
          --ignore RUSTSEC-2024-0384 \
          --ignore RUSTSEC-2024-0436 \
          --ignore RUSTSEC-2026-0192 \
          --ignore RUSTSEC-2026-0194 \
          --ignore RUSTSEC-2026-0195 \
          --ignore RUSTSEC-2026-0206
    - name: ci::run_documented_gate
      run: cargo xtask ci
    timeout-minutes: 60
",
    )
}

fn dependency_review_workflow() -> String {
    render_workflow(
        r"# Generated from xtask::workflows::dependency_review
# Rebuild with `cargo xtask workflows`.
name: dependency review
on:
  pull_request:
permissions:
  contents: read
jobs:
  dependency_review:
    name: dependency review
    runs-on: ubuntu-22.04
    steps:
    - name: security::review_dependency_changes
      uses: actions/dependency-review-action@v5.0.0
      with:
        fail-on-severity: moderate
        comment-summary-in-pr: never
    timeout-minutes: 10
",
    )
}

fn codeql_workflow() -> String {
    render_workflow(
        r"# Generated from xtask::workflows::codeql
# Rebuild with `cargo xtask workflows`.
name: codeql
on:
  pull_request:
  push:
    branches:
      - master
  schedule:
    - cron: '23 4 * * 3'
permissions:
  contents: read
  security-events: write
concurrency:
  group: codeql-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true
jobs:
  analyze:
    name: codeql rust
    runs-on: ubuntu-22.04
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v7.0.0
      with:
        persist-credentials: false
    - name: security::initialize_codeql
      uses: github/codeql-action/init@v4.37.1
      with:
        languages: rust
        build-mode: none
        queries: security-extended
    - name: security::analyze_code
      uses: github/codeql-action/analyze@v4.37.1
      with:
        category: /language:rust
    timeout-minutes: 30
",
    )
}

fn run_bundling_workflow() -> String {
    let header = "\
# Generated from xtask::workflows::run_bundling
# Rebuild with `cargo xtask workflows`.
name: run_bundling
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: '1'
on:
  workflow_dispatch:
  pull_request:
    types:
      - labeled
permissions:
  contents: read
";

    let jobs = [
        linux_job("x86_64", "ubuntu-22.04"),
        linux_job("aarch64", "ubuntu-22.04-arm"),
        macos_job("x86_64", "x86_64-apple-darwin", "macos-26-intel"),
        macos_job("aarch64", "aarch64-apple-darwin", "macos-26"),
        windows_job("x86_64", "windows-2022"),
    ]
    .join("");

    render_workflow(&format!("{header}jobs:\n{jobs}"))
}

const fn bundle_if_expression() -> &'static str {
    "      github.event_name == 'workflow_dispatch' ||\n      (github.event.action == 'labeled' && github.event.label.name == 'run-bundling')"
}

const fn checkout_step() -> &'static str {
    r"    - name: steps::checkout_repo
      uses: actions/checkout@v7.0.0
      with:
        persist-credentials: false
"
}

const fn setup_rust_step() -> &'static str {
    r"    - name: steps::setup_rust
      run: rustup toolchain install __RUST_VERSION__ --profile minimal --component clippy,rustfmt
"
}

fn linux_job(arch: &str, runner: &str) -> String {
    let appimagetool_arch = arch;
    let linux_packages = "clang curl desktop-file-utils file libfontconfig1-dev libfreetype6-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libdrm-dev pkg-config patchelf zsync";
    let bundle_args = "--tarball --appimage";
    let timeout_minutes = 90;

    format!(
        r"  bundle_linux_{arch}:
    if: |-
{if_expression}
    runs-on: {runner}
    environment: run-bundling
    env:
      CARGO_INCREMENTAL: 0
    steps:
{checkout}{rust}    - name: steps::setup_linux
      run: |
        sudo apt-get update
        sudo apt-get install -y {linux_packages}
    - name: steps::setup_appimagetool
      run: |
        curl --fail --location --proto '=https' --tlsv1.2 --output /tmp/appimagetool.AppImage https://github.com/AppImage/appimagetool/releases/download/__APPIMAGETOOL_VERSION__/appimagetool-{appimagetool_arch}.AppImage
        echo '__APPIMAGETOOL_{appimagetool_sha_name}_SHA256__  /tmp/appimagetool.AppImage' | sha256sum --check --strict
        chmod +x /tmp/appimagetool.AppImage
    - name: ./script/bundle-linux
      env:
        APPIMAGETOOL: /tmp/appimagetool.AppImage
      run: ./script/bundle-linux {bundle_args}
    - name: run_bundling::verify_appimage_update_information
      shell: bash
      run: |
        expected='gh-releases-zsync|66HEX|frame|latest|Frame-{arch}.AppImage.zsync'
        actual=$(target/release/Frame-{arch}.AppImage --appimage-updateinformation)
        test $actual = $expected
        test -s target/release/Frame-{arch}.AppImage.zsync
    - name: run_bundling::upload_artifact
      uses: actions/upload-artifact@v7.0.1
      with:
        name: frame-linux-{arch}.tar.gz
        path: target/release/frame-linux-{arch}.tar.gz
        if-no-files-found: error
    - name: run_bundling::upload_appimage_artifact
      uses: actions/upload-artifact@v7.0.1
      with:
        name: Frame-{arch}.AppImage
        path: target/release/Frame-{arch}.AppImage
        if-no-files-found: error
    - name: run_bundling::upload_appimage_zsync_artifact
      uses: actions/upload-artifact@v7.0.1
      with:
        name: Frame-{arch}.AppImage.zsync
        path: target/release/Frame-{arch}.AppImage.zsync
        if-no-files-found: error
    timeout-minutes: {timeout_minutes}
",
        if_expression = bundle_if_expression(),
        checkout = checkout_step(),
        rust = setup_rust_step(),
        appimagetool_arch = appimagetool_arch,
        appimagetool_sha_name = arch.to_ascii_uppercase(),
        linux_packages = linux_packages,
        bundle_args = bundle_args,
        timeout_minutes = timeout_minutes,
    )
}

fn macos_job(arch: &str, target: &str, runner: &str) -> String {
    format!(
        r"  bundle_macos_{arch}:
    if: |-
{if_expression}
    runs-on: {runner}
    environment: run-bundling
    env:
      CARGO_INCREMENTAL: 0
    steps:
{checkout}{rust}    - name: steps::install_cargo_bundle
      run: cargo install cargo-bundle --version __CARGO_BUNDLE_VERSION__ --locked
    - name: ./script/bundle-mac
      run: ./script/bundle-mac {target}
    - name: run_bundling::upload_artifact
      uses: actions/upload-artifact@v7.0.1
      with:
        name: Frame-{arch}.dmg
        path: target/{target}/release/Frame-{arch}.dmg
        if-no-files-found: error
    - name: run_bundling::upload_update_artifact
      uses: actions/upload-artifact@v7.0.1
      with:
        name: Frame-{arch}.app.zip
        path: target/{target}/release/Frame-{arch}.app.zip
        if-no-files-found: error
    timeout-minutes: 60
",
        if_expression = bundle_if_expression(),
        checkout = checkout_step(),
        rust = setup_rust_step(),
    )
}

fn windows_job(arch: &str, runner: &str) -> String {
    format!(
        r"  bundle_windows_{arch}:
    if: |-
{if_expression}
    runs-on: {runner}
    environment: run-bundling
    env:
      CARGO_INCREMENTAL: 0
    steps:
{checkout}{rust}    - name: steps::setup_inno
      shell: pwsh
      run: choco install innosetup --version __INNO_SETUP_VERSION__ --no-progress --yes
    - name: ./script/bundle-windows.ps1
      shell: pwsh
      run: ./script/bundle-windows.ps1 -Architecture {arch}
    - name: run_bundling::upload_artifact
      uses: actions/upload-artifact@v7.0.1
      with:
        name: Frame-{arch}.exe
        path: target/Frame-{arch}.exe
        if-no-files-found: error
    timeout-minutes: 60
",
        if_expression = bundle_if_expression(),
        checkout = checkout_step(),
        rust = setup_rust_step(),
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "The generated GitHub Actions workflow is kept as one raw template for easier diffing against YAML output."
)]
fn release_workflow() -> String {
    let workflow = r#"# Generated from xtask::workflows::release
# Rebuild with `cargo xtask workflows`.
name: release
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: '1'
on:
  push:
    tags:
      - '[0-9]*.[0-9]*.[0-9]*'
      - '!*-*'
  workflow_dispatch:
    inputs:
      tag:
        description: Release tag to publish, without a v prefix.
        required: true
permissions:
  contents: read
concurrency:
  group: release-${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref_name }}
  cancel-in-progress: false
jobs:
  prepare_release:
    runs-on: ubuntu-22.04
    outputs:
      commit_sha: ${{ steps.release.outputs.commit_sha }}
      tag: ${{ steps.release.outputs.tag }}
      tag_object_sha: ${{ steps.release.outputs.tag_object_sha }}
      version: ${{ steps.release.outputs.version }}
    steps:
    - name: release::resolve_and_verify_signed_tag
      id: release
      env:
        GH_TOKEN: ${{ github.token }}
        REQUESTED_TAG: ${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref_name }}
      shell: bash
      run: |
        set -euo pipefail
        tag="$REQUESTED_TAG"
        if [[ ! "$tag" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
          echo "::error::Release tag must be stable semver without a v prefix: $tag" >&2
          exit 1
        fi
        if [[ "$GITHUB_EVENT_NAME" == workflow_dispatch && "$GITHUB_REF" != "refs/tags/$tag" ]]; then
          echo "::error::A manual release must be dispatched from the exact requested tag ref." >&2
          exit 1
        fi

        ref_json="$(gh api "repos/$GITHUB_REPOSITORY/git/ref/tags/$tag")"
        if [[ "$(jq -r '.object.type' <<<"$ref_json")" != "tag" ]]; then
          echo "::error::Release tag must be an annotated, signed tag." >&2
          exit 1
        fi
        tag_object_sha="$(jq -r '.object.sha' <<<"$ref_json")"
        tag_json="$(gh api "repos/$GITHUB_REPOSITORY/git/tags/$tag_object_sha")"
        if [[ "$(jq -r '.verification.verified' <<<"$tag_json")" != "true" ]]; then
          reason="$(jq -r '.verification.reason' <<<"$tag_json")"
          echo "::error::GitHub did not verify the tag signature: $reason" >&2
          exit 1
        fi
        if [[ "$(jq -r '.object.type' <<<"$tag_json")" != "commit" ]]; then
          echo "::error::Signed tag does not point directly to a commit." >&2
          exit 1
        fi
        commit_sha="$(jq -r '.object.sha' <<<"$tag_json")"

        if gh api "repos/$GITHUB_REPOSITORY/releases/tags/$tag" >/dev/null 2>&1; then
          echo "::error::A GitHub release already exists for $tag; release assets are immutable." >&2
          exit 1
        fi
        echo "commit_sha=$commit_sha" >> "$GITHUB_OUTPUT"
        echo "tag=$tag" >> "$GITHUB_OUTPUT"
        echo "tag_object_sha=$tag_object_sha" >> "$GITHUB_OUTPUT"
        echo "version=$tag" >> "$GITHUB_OUTPUT"
    - name: steps::checkout_verified_commit
      uses: actions/checkout@v7.0.0
      with:
        ref: ${{ steps.release.outputs.commit_sha }}
        fetch-depth: 0
    - name: release::verify_commit_source_and_ci
      env:
        COMMIT_SHA: ${{ steps.release.outputs.commit_sha }}
        GH_TOKEN: ${{ github.token }}
        TAG: ${{ steps.release.outputs.tag }}
      shell: bash
      run: |
        set -euo pipefail
        git fetch --no-tags origin refs/heads/master:refs/remotes/origin/master
        git merge-base --is-ancestor "$COMMIT_SHA" refs/remotes/origin/master || {
          echo "::error::Release commit is not an ancestor of origin/master." >&2
          exit 1
        }
        test "$(git rev-parse HEAD)" = "$COMMIT_SHA"
        version="$(sed -n '/^\[package\]$/,/^\[/s/^version = "\([^"]*\)"$/\1/p' frame-app/Cargo.toml | head -n1)"
        if [[ "$version" != "$TAG" ]]; then
          echo "::error::frame-app version $version does not match signed tag $TAG." >&2
          exit 1
        fi
        ci_runs="$(gh api --method GET "repos/$GITHUB_REPOSITORY/actions/workflows/ci.yml/runs" \
          -f head_sha="$COMMIT_SHA" -f branch=master -f status=completed -f per_page=100)"
        if ! jq -e --arg sha "$COMMIT_SHA" \
          '[.workflow_runs[] | select(.head_sha == $sha and .head_branch == "master" and .event == "push" and .conclusion == "success")] | length > 0' \
          <<<"$ci_runs" >/dev/null; then
          echo "::error::The verified commit has no successful ci.yml run from a push to master." >&2
          exit 1
        fi
    timeout-minutes: 10

  build_linux_x86_64:
    runs-on: ubuntu-22.04
    needs: prepare_release
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: release::check_public_key
      run: test -n "$FRAME_UPDATE_PUBLIC_KEY"
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: steps::setup_linux
      run: |
        sudo apt-get update
        sudo apt-get install -y clang curl desktop-file-utils file libfontconfig1-dev libfreetype6-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libdrm-dev pkg-config patchelf zsync
    - name: steps::setup_appimagetool
      run: |
        curl --fail --location --proto '=https' --tlsv1.2 --output /tmp/appimagetool.AppImage https://github.com/AppImage/appimagetool/releases/download/__APPIMAGETOOL_VERSION__/appimagetool-x86_64.AppImage
        echo '__APPIMAGETOOL_X86_64_SHA256__  /tmp/appimagetool.AppImage' | sha256sum --check --strict
        chmod +x /tmp/appimagetool.AppImage
    - name: ./script/bundle-linux
      env:
        APPIMAGETOOL: /tmp/appimagetool.AppImage
      run: ./script/bundle-linux --tarball --appimage
    - name: release::verify_linux_x86_64_appimage_update_information
      shell: bash
      run: |
        expected='gh-releases-zsync|66HEX|frame|latest|Frame-x86_64.AppImage.zsync'
        actual="$(target/release/Frame-x86_64.AppImage --appimage-updateinformation)"
        test "$actual" = "$expected"
        test -s target/release/Frame-x86_64.AppImage.zsync
    - name: release::upload_linux_x86_64
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-frame-linux-x86_64
        path: target/release/frame-linux-x86_64.tar.gz
        if-no-files-found: error
    - name: release::upload_linux_x86_64_appimage
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-x86_64-AppImage
        path: target/release/Frame-x86_64.AppImage
        if-no-files-found: error
    - name: release::upload_linux_x86_64_appimage_zsync
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-x86_64-AppImage-zsync
        path: target/release/Frame-x86_64.AppImage.zsync
        if-no-files-found: error
    timeout-minutes: 90

  build_linux_aarch64:
    runs-on: ubuntu-22.04-arm
    needs: prepare_release
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: release::check_public_key
      run: test -n "$FRAME_UPDATE_PUBLIC_KEY"
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: steps::setup_linux
      run: |
        sudo apt-get update
        sudo apt-get install -y clang curl desktop-file-utils file libfontconfig1-dev libfreetype6-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libdrm-dev pkg-config patchelf zsync
    - name: steps::setup_appimagetool
      run: |
        curl --fail --location --proto '=https' --tlsv1.2 --output /tmp/appimagetool.AppImage https://github.com/AppImage/appimagetool/releases/download/__APPIMAGETOOL_VERSION__/appimagetool-aarch64.AppImage
        echo '__APPIMAGETOOL_AARCH64_SHA256__  /tmp/appimagetool.AppImage' | sha256sum --check --strict
        chmod +x /tmp/appimagetool.AppImage
    - name: ./script/bundle-linux
      env:
        APPIMAGETOOL: /tmp/appimagetool.AppImage
      run: ./script/bundle-linux --tarball --appimage
    - name: release::verify_linux_aarch64_appimage_update_information
      shell: bash
      run: |
        expected='gh-releases-zsync|66HEX|frame|latest|Frame-aarch64.AppImage.zsync'
        actual="$(target/release/Frame-aarch64.AppImage --appimage-updateinformation)"
        test "$actual" = "$expected"
        test -s target/release/Frame-aarch64.AppImage.zsync
    - name: release::upload_linux_aarch64
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-frame-linux-aarch64
        path: target/release/frame-linux-aarch64.tar.gz
        if-no-files-found: error
    - name: release::upload_linux_aarch64_appimage
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-aarch64-AppImage
        path: target/release/Frame-aarch64.AppImage
        if-no-files-found: error
    - name: release::upload_linux_aarch64_appimage_zsync
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-aarch64-AppImage-zsync
        path: target/release/Frame-aarch64.AppImage.zsync
        if-no-files-found: error
    timeout-minutes: 90

  build_macos_x86_64:
    runs-on: macos-26-intel
    needs: prepare_release
    outputs:
      unsigned_sha256: ${{ steps.unsigned.outputs.sha256 }}
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: release::check_public_key
      run: test -n "$FRAME_UPDATE_PUBLIC_KEY"
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: steps::install_cargo_bundle
      run: cargo install cargo-bundle --locked
    - name: release::build_macos_x86_64_unsigned
      run: ./script/bundle-mac -b x86_64-apple-darwin
    - name: release::archive_macos_x86_64_unsigned
      id: unsigned
      run: |
        ditto -c -k --sequesterRsrc --keepParent \
          target/x86_64-apple-darwin/release/bundle/osx/Frame.app \
          target/Frame-x86_64-unsigned.app.zip
        echo "sha256=$(shasum -a 256 target/Frame-x86_64-unsigned.app.zip | awk '{print $1}')" >> "$GITHUB_OUTPUT"
    - name: release::upload_macos_x86_64_unsigned
      uses: actions/upload-artifact@v4
      with:
        name: signing-input-Frame-x86_64-app
        path: target/Frame-x86_64-unsigned.app.zip
        if-no-files-found: error
        retention-days: 3
    timeout-minutes: 90

  sign_macos_x86_64:
    runs-on: macos-26-intel
    needs:
      - prepare_release
      - build_macos_x86_64
    environment: production-release
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: release::download_macos_x86_64_unsigned
      uses: actions/download-artifact@v4
      with:
        name: signing-input-Frame-x86_64-app
        path: target/signing-input
    - name: release::extract_macos_x86_64_unsigned
      run: |
        echo '${{ needs.build_macos_x86_64.outputs.unsigned_sha256 }}  target/signing-input/Frame-x86_64-unsigned.app.zip' | shasum -a 256 --check
        mkdir -p target/x86_64-apple-darwin/release/bundle/osx
        ditto -x -k target/signing-input/Frame-x86_64-unsigned.app.zip \
          target/x86_64-apple-darwin/release/bundle/osx
        test -d target/x86_64-apple-darwin/release/bundle/osx/Frame.app
    - name: release::check_macos_signing_credentials
      env:
        MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
        MACOS_CERTIFICATES_P12: ${{ secrets.MACOS_CERTIFICATES_P12 }}
        MACOS_CERTIFICATES_PASSWORD: ${{ secrets.MACOS_CERTIFICATES_PASSWORD }}
        APPLE_NOTARIZATION_KEY: ${{ secrets.APPLE_NOTARIZATION_KEY }}
        APPLE_NOTARIZATION_KEY_ID: ${{ secrets.APPLE_NOTARIZATION_KEY_ID }}
        APPLE_NOTARIZATION_ISSUER_ID: ${{ secrets.APPLE_NOTARIZATION_ISSUER_ID }}
      run: |
        test -n "$MACOS_SIGNING_IDENTITY"
        test -n "$MACOS_CERTIFICATES_P12"
        test -n "$MACOS_CERTIFICATES_PASSWORD"
        test -n "$APPLE_NOTARIZATION_KEY"
        test -n "$APPLE_NOTARIZATION_KEY_ID"
        test -n "$APPLE_NOTARIZATION_ISSUER_ID"
    - name: release::import_macos_signing_certificate
      uses: Apple-Actions/import-codesign-certs@v3
      with:
        p12-file-base64: ${{ secrets.MACOS_CERTIFICATES_P12 }}
        p12-password: ${{ secrets.MACOS_CERTIFICATES_PASSWORD }}
    - name: release::sign_and_package_macos_x86_64
      env:
        MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
        APPLE_NOTARIZATION_KEY: ${{ secrets.APPLE_NOTARIZATION_KEY }}
        APPLE_NOTARIZATION_KEY_ID: ${{ secrets.APPLE_NOTARIZATION_KEY_ID }}
        APPLE_NOTARIZATION_ISSUER_ID: ${{ secrets.APPLE_NOTARIZATION_ISSUER_ID }}
      run: ./script/bundle-mac -p x86_64-apple-darwin
    - name: release::verify_macos_x86_64_signing
      run: |
        codesign --verify --strict --verbose=2 target/x86_64-apple-darwin/release/Frame-x86_64.dmg
        xcrun stapler validate target/x86_64-apple-darwin/release/Frame-x86_64.dmg
    - name: release::upload_macos_x86_64_dmg
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-x86_64-dmg
        path: target/x86_64-apple-darwin/release/Frame-x86_64.dmg
        if-no-files-found: error
    - name: release::upload_macos_x86_64_update
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-x86_64-app-zip
        path: target/x86_64-apple-darwin/release/Frame-x86_64.app.zip
        if-no-files-found: error
    timeout-minutes: 90

  build_macos_aarch64:
    runs-on: macos-26
    needs: prepare_release
    outputs:
      unsigned_sha256: ${{ steps.unsigned.outputs.sha256 }}
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: release::check_public_key
      run: test -n "$FRAME_UPDATE_PUBLIC_KEY"
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: steps::install_cargo_bundle
      run: cargo install cargo-bundle --locked
    - name: release::build_macos_aarch64_unsigned
      run: ./script/bundle-mac -b aarch64-apple-darwin
    - name: release::archive_macos_aarch64_unsigned
      id: unsigned
      run: |
        ditto -c -k --sequesterRsrc --keepParent \
          target/aarch64-apple-darwin/release/bundle/osx/Frame.app \
          target/Frame-aarch64-unsigned.app.zip
        echo "sha256=$(shasum -a 256 target/Frame-aarch64-unsigned.app.zip | awk '{print $1}')" >> "$GITHUB_OUTPUT"
    - name: release::upload_macos_aarch64_unsigned
      uses: actions/upload-artifact@v4
      with:
        name: signing-input-Frame-aarch64-app
        path: target/Frame-aarch64-unsigned.app.zip
        if-no-files-found: error
        retention-days: 3
    timeout-minutes: 90

  sign_macos_aarch64:
    runs-on: macos-26
    needs:
      - prepare_release
      - build_macos_aarch64
    environment: production-release
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: release::download_macos_aarch64_unsigned
      uses: actions/download-artifact@v4
      with:
        name: signing-input-Frame-aarch64-app
        path: target/signing-input
    - name: release::extract_macos_aarch64_unsigned
      run: |
        echo '${{ needs.build_macos_aarch64.outputs.unsigned_sha256 }}  target/signing-input/Frame-aarch64-unsigned.app.zip' | shasum -a 256 --check
        mkdir -p target/aarch64-apple-darwin/release/bundle/osx
        ditto -x -k target/signing-input/Frame-aarch64-unsigned.app.zip \
          target/aarch64-apple-darwin/release/bundle/osx
        test -d target/aarch64-apple-darwin/release/bundle/osx/Frame.app
    - name: release::check_macos_signing_credentials
      env:
        MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
        MACOS_CERTIFICATES_P12: ${{ secrets.MACOS_CERTIFICATES_P12 }}
        MACOS_CERTIFICATES_PASSWORD: ${{ secrets.MACOS_CERTIFICATES_PASSWORD }}
        APPLE_NOTARIZATION_KEY: ${{ secrets.APPLE_NOTARIZATION_KEY }}
        APPLE_NOTARIZATION_KEY_ID: ${{ secrets.APPLE_NOTARIZATION_KEY_ID }}
        APPLE_NOTARIZATION_ISSUER_ID: ${{ secrets.APPLE_NOTARIZATION_ISSUER_ID }}
      run: |
        test -n "$MACOS_SIGNING_IDENTITY"
        test -n "$MACOS_CERTIFICATES_P12"
        test -n "$MACOS_CERTIFICATES_PASSWORD"
        test -n "$APPLE_NOTARIZATION_KEY"
        test -n "$APPLE_NOTARIZATION_KEY_ID"
        test -n "$APPLE_NOTARIZATION_ISSUER_ID"
    - name: release::import_macos_signing_certificate
      uses: Apple-Actions/import-codesign-certs@v3
      with:
        p12-file-base64: ${{ secrets.MACOS_CERTIFICATES_P12 }}
        p12-password: ${{ secrets.MACOS_CERTIFICATES_PASSWORD }}
    - name: release::sign_and_package_macos_aarch64
      env:
        MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
        APPLE_NOTARIZATION_KEY: ${{ secrets.APPLE_NOTARIZATION_KEY }}
        APPLE_NOTARIZATION_KEY_ID: ${{ secrets.APPLE_NOTARIZATION_KEY_ID }}
        APPLE_NOTARIZATION_ISSUER_ID: ${{ secrets.APPLE_NOTARIZATION_ISSUER_ID }}
      run: ./script/bundle-mac -p aarch64-apple-darwin
    - name: release::verify_macos_aarch64_signing
      run: |
        codesign --verify --strict --verbose=2 target/aarch64-apple-darwin/release/Frame-aarch64.dmg
        xcrun stapler validate target/aarch64-apple-darwin/release/Frame-aarch64.dmg
    - name: release::upload_macos_aarch64_dmg
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-aarch64-dmg
        path: target/aarch64-apple-darwin/release/Frame-aarch64.dmg
        if-no-files-found: error
    - name: release::upload_macos_aarch64_update
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-aarch64-app-zip
        path: target/aarch64-apple-darwin/release/Frame-aarch64.app.zip
        if-no-files-found: error
    timeout-minutes: 90

  build_windows_x86_64:
    runs-on: windows-2022
    needs: prepare_release
    outputs:
      unsigned_sha256: ${{ steps.unsigned.outputs.sha256 }}
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: release::check_public_key
      shell: pwsh
      run: |
        if (-not $env:FRAME_UPDATE_PUBLIC_KEY) { throw "FRAME_UPDATE_PUBLIC_KEY is required" }
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: release::build_windows_x86_64_unsigned
      shell: pwsh
      run: ./script/bundle-windows.ps1 -Architecture x86_64 -PrepareOnly
    - name: release::archive_windows_x86_64_unsigned
      id: unsigned
      shell: pwsh
      run: |
        Compress-Archive -Path target/inno/x86_64/* -DestinationPath target/Frame-x86_64-inno-inputs.zip
        $sha256 = (Get-FileHash target/Frame-x86_64-inno-inputs.zip -Algorithm SHA256).Hash.ToLowerInvariant()
        "sha256=$sha256" >> $env:GITHUB_OUTPUT
    - name: release::upload_windows_x86_64_unsigned
      uses: actions/upload-artifact@v4
      with:
        name: signing-input-Frame-x86_64-windows
        path: target/Frame-x86_64-inno-inputs.zip
        if-no-files-found: error
        retention-days: 3
    timeout-minutes: 90

  sign_windows_x86_64:
    runs-on: windows-2022
    needs:
      - prepare_release
      - build_windows_x86_64
    environment: production-release
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: release::download_windows_x86_64_unsigned
      uses: actions/download-artifact@v4
      with:
        name: signing-input-Frame-x86_64-windows
        path: target/signing-input
    - name: release::extract_windows_x86_64_unsigned
      shell: pwsh
      run: |
        $expected = '${{ needs.build_windows_x86_64.outputs.unsigned_sha256 }}'
        $actual = (Get-FileHash target/signing-input/Frame-x86_64-inno-inputs.zip -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne $expected) { throw "Unsigned Windows input digest mismatch: $actual != $expected" }
        Expand-Archive -Path target/signing-input/Frame-x86_64-inno-inputs.zip -DestinationPath target/inno/x86_64
    - name: steps::setup_inno
      shell: pwsh
      run: choco install innosetup --no-progress -y
    - name: release::sign_and_package_windows_x86_64
      env:
        WINDOWS_SIGNTOOL: ${{ secrets.WINDOWS_SIGNTOOL }}
      shell: pwsh
      run: |
        if (-not $env:WINDOWS_SIGNTOOL) { throw "WINDOWS_SIGNTOOL is required for production releases" }
        ./script/bundle-windows.ps1 -Architecture x86_64 -PackageOnly
        $signature = Get-AuthenticodeSignature target/Frame-x86_64.exe
        if ($signature.Status -ne 'Valid') { throw "Installer signature is not valid: $($signature.Status)" }
    - name: release::upload_windows_x86_64
      uses: actions/upload-artifact@v4
      with:
        name: release-asset-Frame-x86_64-exe
        path: target/Frame-x86_64.exe
        if-no-files-found: error
    timeout-minutes: 60

  generate_release_metadata:
    runs-on: ubuntu-22.04
    needs:
      - prepare_release
      - build_linux_x86_64
      - build_linux_aarch64
      - sign_macos_x86_64
      - sign_macos_aarch64
      - sign_windows_x86_64
    env:
      TAURI_BRIDGE_RELEASE_TAG: '0.29.3'
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: release::download_build_artifacts
      uses: actions/download-artifact@v4
      with:
        pattern: release-asset-*
        path: target/release-artifacts
        merge-multiple: true
    - name: release::extract_release_notes
      shell: bash
      run: |
        mkdir -p target/release-files
        version="${{ needs.prepare_release.outputs.version }}"
        awk -v ver="[$version]" '/^## / { if (p) { exit }; if ($2 == ver) { p=1; next } } p' CHANGELOG.md > target/release-files/release-notes.md
        if [[ ! -s target/release-files/release-notes.md ]]; then
          echo "::error::CHANGELOG.md has no release notes section for [$version]" >&2
          exit 1
        fi
    - name: release::prepare_flathub_sources
      run: |
        cargo xtask flathub-sources --version "${{ needs.prepare_release.outputs.version }}"
        cp "target/flathub/frame-${{ needs.prepare_release.outputs.version }}-source.tar.gz" target/release-files/
        cp "target/flathub/frame-${{ needs.prepare_release.outputs.version }}-cargo-vendor.tar.gz" target/release-files/
    - name: release::download_tauri_latest_json
      env:
        GH_TOKEN: ${{ github.token }}
      run: |
        gh release download "$TAURI_BRIDGE_RELEASE_TAG" \
          --repo 66HEX/frame \
          --pattern latest.json \
          --dir target/release-files
        echo '__TAURI_BRIDGE_LATEST_JSON_SHA256__  target/release-files/latest.json' | sha256sum --check --strict
    - name: release::generate_update_manifest
      run: |
        cargo xtask update-manifest \
          --version "${{ needs.prepare_release.outputs.version }}" \
          --release-tag "${{ needs.prepare_release.outputs.tag }}" \
          --artifact target/release-artifacts/Frame-aarch64.app.zip:macos-aarch64:macos_app_zip \
          --artifact target/release-artifacts/Frame-x86_64.app.zip:macos-x86_64:macos_app_zip \
          --artifact target/release-artifacts/Frame-x86_64.exe:windows-x86_64:windows_inno \
          --artifact target/release-artifacts/frame-linux-x86_64.tar.gz:linux-x86_64:linux_managed_tar \
          --artifact target/release-artifacts/frame-linux-aarch64.tar.gz:linux-aarch64:linux_managed_tar \
          --release-notes-markdown "$(< target/release-files/release-notes.md)" \
          --out target/release-files/update-manifest.json
    - name: security::generate_cyclonedx_sbom
      run: |
        cargo install cargo-cyclonedx --version __CARGO_CYCLONEDX_VERSION__ --locked
        cargo cyclonedx \
          --manifest-path frame-app/Cargo.toml \
          --format json \
          --target all \
          --override-filename Frame.cdx \
          --spec-version 1.5
        cp frame-app/Frame.cdx.json target/release-files/Frame.cdx.json
    - name: release::build_isolated_manifest_signer
      run: cargo build --manifest-path tooling/manifest-signer/Cargo.toml --release --locked
    - name: release::upload_unsigned_metadata
      uses: actions/upload-artifact@v4
      with:
        name: unsigned-release-metadata
        path: target/release-files
        if-no-files-found: error
        retention-days: 3
    - name: release::upload_manifest_signer
      uses: actions/upload-artifact@v4
      with:
        name: manifest-signer
        path: target/release/frame-manifest-signer
        if-no-files-found: error
        retention-days: 3
    timeout-minutes: 90

  sign_release_metadata:
    runs-on: ubuntu-22.04
    needs:
      - prepare_release
      - generate_release_metadata
    environment: production-release
    steps:
    - name: release::download_unsigned_metadata
      uses: actions/download-artifact@v4
      with:
        name: unsigned-release-metadata
        path: target/release-files
    - name: release::download_manifest_signer
      uses: actions/download-artifact@v4
      with:
        name: manifest-signer
        path: target/signer
    - name: release::sign_update_manifest
      env:
        FRAME_UPDATE_SIGNING_KEY: ${{ secrets.FRAME_UPDATE_SIGNING_KEY }}
        FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY }}
      shell: bash
      run: |
        set -euo pipefail
        test -n "$FRAME_UPDATE_SIGNING_KEY"
        chmod +x target/signer/frame-manifest-signer
        target/signer/frame-manifest-signer \
          --manifest target/release-files/update-manifest.json \
          --out target/release-files/update-manifest.json.sig
    - name: release::upload_manifest_signature
      uses: actions/upload-artifact@v4
      with:
        name: signed-update-manifest
        path: target/release-files/update-manifest.json.sig
        if-no-files-found: error
        retention-days: 3
    timeout-minutes: 10

  assemble_release:
    runs-on: ubuntu-22.04
    needs:
      - prepare_release
      - build_linux_x86_64
      - build_linux_aarch64
      - sign_macos_x86_64
      - sign_macos_aarch64
      - sign_windows_x86_64
      - generate_release_metadata
      - sign_release_metadata
    permissions:
      contents: read
      id-token: write
      attestations: write
    steps:
    - name: release::download_build_artifacts
      uses: actions/download-artifact@v4
      with:
        pattern: release-asset-*
        path: target/release-package
        merge-multiple: true
    - name: release::download_unsigned_metadata
      uses: actions/download-artifact@v4
      with:
        name: unsigned-release-metadata
        path: target/metadata
    - name: release::download_manifest_signature
      uses: actions/download-artifact@v4
      with:
        name: signed-update-manifest
        path: target/signature
    - name: release::assemble_exact_asset_set
      env:
        VERSION: ${{ needs.prepare_release.outputs.version }}
      shell: bash
      run: |
        set -euo pipefail
        cp target/metadata/Frame.cdx.json target/release-package/
        cp target/metadata/latest.json target/release-package/
        cp target/metadata/update-manifest.json target/release-package/
        cp target/metadata/frame-"$VERSION"-source.tar.gz target/release-package/
        cp target/metadata/frame-"$VERSION"-cargo-vendor.tar.gz target/release-package/
        cp target/signature/update-manifest.json.sig target/release-package/
        cp target/metadata/release-notes.md target/release-package/

        expected=(
          Frame-aarch64.AppImage
          Frame-aarch64.AppImage.zsync
          Frame-aarch64.app.zip
          Frame-aarch64.dmg
          Frame-x86_64.AppImage
          Frame-x86_64.AppImage.zsync
          Frame-x86_64.app.zip
          Frame-x86_64.dmg
          Frame-x86_64.exe
          Frame.cdx.json
          frame-linux-aarch64.tar.gz
          frame-linux-x86_64.tar.gz
          frame-"$VERSION"-cargo-vendor.tar.gz
          frame-"$VERSION"-source.tar.gz
          latest.json
          release-notes.md
          update-manifest.json
          update-manifest.json.sig
        )
        mapfile -t actual < <(find target/release-package -maxdepth 1 -type f -printf '%f\n' | LC_ALL=C sort)
        mapfile -t wanted < <(printf '%s\n' "${expected[@]}" | LC_ALL=C sort)
        diff <(printf '%s\n' "${wanted[@]}") <(printf '%s\n' "${actual[@]}")

        (
          cd target/release-package
          for asset in "${wanted[@]}"; do
            if [[ "$asset" != release-notes.md ]]; then
              sha256sum "$asset"
            fi
          done > SHA256SUMS
        )
        (cd target/release-package && sha256sum --check --strict SHA256SUMS)
    - name: security::attest_release_assets
      uses: actions/attest-build-provenance@v4.1.1
      with:
        subject-checksums: target/release-package/SHA256SUMS
    - name: release::upload_complete_release_package
      uses: actions/upload-artifact@v4
      with:
        name: complete-release-package
        path: target/release-package
        if-no-files-found: error
        retention-days: 3
    timeout-minutes: 20

  publish_release:
    runs-on: ubuntu-22.04
    needs:
      - prepare_release
      - assemble_release
    environment: production-release
    permissions:
      contents: write
    env:
      COMMIT_SHA: ${{ needs.prepare_release.outputs.commit_sha }}
      TAG: ${{ needs.prepare_release.outputs.tag }}
      TAG_OBJECT_SHA: ${{ needs.prepare_release.outputs.tag_object_sha }}
      VERSION: ${{ needs.prepare_release.outputs.version }}
    steps:
    - name: release::download_complete_release_package
      uses: actions/download-artifact@v4
      with:
        name: complete-release-package
        path: target/release-package
    - name: release::create_verified_draft_then_publish
      env:
        GH_TOKEN: ${{ github.token }}
      shell: bash
      run: |
        set -euo pipefail
        cd target/release-package
        sha256sum --check --strict SHA256SUMS
        mapfile -t checksummed < <(awk '{print $2}' SHA256SUMS | LC_ALL=C sort)
        mapfile -t packaged < <(find . -maxdepth 1 -type f ! -name release-notes.md ! -name SHA256SUMS -printf '%f\n' | LC_ALL=C sort)
        diff <(printf '%s\n' "${checksummed[@]}") <(printf '%s\n' "${packaged[@]}")

        ref_json="$(gh api "repos/$GITHUB_REPOSITORY/git/ref/tags/$TAG")"
        test "$(jq -r '.object.type' <<<"$ref_json")" = tag
        test "$(jq -r '.object.sha' <<<"$ref_json")" = "$TAG_OBJECT_SHA"
        tag_json="$(gh api "repos/$GITHUB_REPOSITORY/git/tags/$TAG_OBJECT_SHA")"
        test "$(jq -r '.verification.verified' <<<"$tag_json")" = true
        test "$(jq -r '.object.type' <<<"$tag_json")" = commit
        test "$(jq -r '.object.sha' <<<"$tag_json")" = "$COMMIT_SHA"
        comparison="$(gh api "repos/$GITHUB_REPOSITORY/compare/$COMMIT_SHA...master")"
        comparison_status="$(jq -r '.status' <<<"$comparison")"
        if [[ "$comparison_status" != ahead && "$comparison_status" != identical ]]; then
          echo "::error::Verified release commit is no longer an ancestor of master." >&2
          exit 1
        fi
        if gh release view "$TAG" --repo "$GITHUB_REPOSITORY" >/dev/null 2>&1; then
          echo "::error::Release $TAG already exists; refusing to overwrite it." >&2
          exit 1
        fi
        mapfile -t assets < <(find . -maxdepth 1 -type f ! -name release-notes.md -printf '%f\n' | LC_ALL=C sort)
        gh release create "$TAG" "${assets[@]}" \
          --repo "$GITHUB_REPOSITORY" \
          --target "$COMMIT_SHA" \
          --verify-tag \
          --draft \
          --title "Frame $VERSION" \
          --notes-file release-notes.md

        release_id="$(gh api --paginate "repos/$GITHUB_REPOSITORY/releases?per_page=100" --jq ".[] | select(.tag_name == \"$TAG\") | .id" | head -n1)"
        test -n "$release_id"
        release_json="$(gh api "repos/$GITHUB_REPOSITORY/releases/$release_id")"
        test "$(jq -r '.draft' <<<"$release_json")" = true
        mapfile -t remote_assets < <(jq -r '.assets[].name' <<<"$release_json" | LC_ALL=C sort)
        diff <(printf '%s\n' "${assets[@]}") <(printf '%s\n' "${remote_assets[@]}")
        for asset in "${assets[@]}"; do
          local_digest="sha256:$(sha256sum "$asset" | awk '{print $1}')"
          remote_digest="$(jq -r --arg name "$asset" '.assets[] | select(.name == $name) | .digest' <<<"$release_json")"
          if [[ "$remote_digest" != "$local_digest" ]]; then
            echo "::error::Uploaded digest mismatch for $asset: $remote_digest != $local_digest" >&2
            exit 1
          fi
        done
        gh api --method PATCH "repos/$GITHUB_REPOSITORY/releases/$release_id" -F draft=false >/dev/null
    timeout-minutes: 30

  update_homebrew_tap:
    runs-on: ubuntu-22.04
    needs:
      - prepare_release
      - publish_release
    environment: production-distribution
    steps:
    - name: release::download_macos_dmgs
      id: hashes
      env:
        GH_TOKEN: ${{ github.token }}
        VERSION: ${{ needs.prepare_release.outputs.tag }}
      shell: bash
      run: |
        mkdir -p target/homebrew
        gh release download "$VERSION" --repo 66HEX/frame --pattern Frame-aarch64.dmg --dir target/homebrew
        gh release download "$VERSION" --repo 66HEX/frame --pattern Frame-x86_64.dmg --dir target/homebrew
        gh release download "$VERSION" --repo 66HEX/frame --pattern SHA256SUMS --dir target/homebrew
        (cd target/homebrew && sha256sum --check --strict --ignore-missing SHA256SUMS)
        echo "HASH_ARM=$(sha256sum target/homebrew/Frame-aarch64.dmg | awk '{print $1}')" >> "$GITHUB_OUTPUT"
        echo "HASH_INTEL=$(sha256sum target/homebrew/Frame-x86_64.dmg | awk '{print $1}')" >> "$GITHUB_OUTPUT"
    - name: release::checkout_tap
      uses: actions/checkout@v4
      with:
        repository: 66HEX/homebrew-frame
        token: ${{ secrets.TAP_GITHUB_TOKEN }}
        path: tap
    - name: release::update_cask
      working-directory: tap
      env:
        VERSION: ${{ needs.prepare_release.outputs.tag }}
        HASH_ARM: ${{ steps.hashes.outputs.HASH_ARM }}
        HASH_INTEL: ${{ steps.hashes.outputs.HASH_INTEL }}
        GH_TOKEN: ${{ secrets.TAP_GITHUB_TOKEN }}
      shell: bash
      run: |
        mkdir -p Casks
        cat <<EOF > Casks/frame.rb
        cask "frame" do
          arch arm: "aarch64", intel: "x86_64"

          version "$VERSION"
          sha256 arm:   "$HASH_ARM",
                 intel: "$HASH_INTEL"

          url "https://github.com/66HEX/frame/releases/download/#{version}/Frame-#{arch}.dmg"
          name "Frame"
          desc "High-performance media conversion utility"
          homepage "https://github.com/66HEX/frame"

          auto_updates true

          app "Frame.app"

          zap trash: [
            "~/Library/Application Support/com.66hex.frame",
            "~/Library/Caches/com.66hex.frame",
            "~/Library/Preferences/com.66hex.frame.plist",
            "~/Library/Saved Application State/com.66hex.frame.savedState",
          ]

        end
        EOF

        git config user.name "github-actions[bot]"
        git config user.email "github-actions[bot]@users.noreply.github.com"
        git add Casks/frame.rb
        if git diff --staged --quiet; then
          echo "No Homebrew cask changes to publish."
        else
          git commit -m "Update Frame to $VERSION"
          gh auth setup-git
          git push
        fi
    timeout-minutes: 20

  update_flathub:
    runs-on: ubuntu-22.04
    needs:
      - prepare_release
      - publish_release
    environment: production-distribution
    env:
      FLATHUB_REPOSITORY: flathub/io.github._66HEX.Frame
    steps:
    - name: release::check_flathub_token
      id: flathub_token
      env:
        FLATHUB_GITHUB_TOKEN: ${{ secrets.FLATHUB_GITHUB_TOKEN }}
      shell: bash
      run: |
        if [[ -z "$FLATHUB_GITHUB_TOKEN" ]]; then
          echo "::notice::FLATHUB_GITHUB_TOKEN is not configured; skipping Flathub manifest update."
          echo "configured=false" >> "$GITHUB_OUTPUT"
        else
          echo "configured=true" >> "$GITHUB_OUTPUT"
        fi
    - name: steps::checkout_repo
      if: steps.flathub_token.outputs.configured == 'true'
      uses: actions/checkout@v4
      with:
        ref: ${{ needs.prepare_release.outputs.commit_sha }}
    - name: steps::setup_rust
      if: steps.flathub_token.outputs.configured == 'true'
      uses: dtolnay/rust-toolchain@stable
    - name: release::download_flathub_sources
      if: steps.flathub_token.outputs.configured == 'true'
      id: flathub_sources
      env:
        GH_TOKEN: ${{ github.token }}
      shell: bash
      run: |
        mkdir -p target/flathub-download
        tag="${{ needs.prepare_release.outputs.tag }}"
        gh release download "$tag" --repo 66HEX/frame --pattern "frame-$tag-source.tar.gz" --dir target/flathub-download
        gh release download "$tag" --repo 66HEX/frame --pattern "frame-$tag-cargo-vendor.tar.gz" --dir target/flathub-download
        gh release download "$tag" --repo 66HEX/frame --pattern SHA256SUMS --dir target/flathub-download
        (cd target/flathub-download && sha256sum --check --strict --ignore-missing SHA256SUMS)
        source_archive="target/flathub-download/frame-$tag-source.tar.gz"
        vendor_archive="target/flathub-download/frame-$tag-cargo-vendor.tar.gz"
        source_sha256="$(sha256sum "$source_archive" | awk '{print $1}')"
        vendor_sha256="$(sha256sum "$vendor_archive" | awk '{print $1}')"
        echo "SOURCE_SHA256=$source_sha256" >> "$GITHUB_OUTPUT"
        echo "VENDOR_SHA256=$vendor_sha256" >> "$GITHUB_OUTPUT"
    - name: release::render_flathub_manifest
      if: steps.flathub_token.outputs.configured == 'true'
      run: |
        tag="${{ needs.prepare_release.outputs.tag }}"
        cargo xtask flathub-manifest \
          --version "$tag" \
          --source-url "https://github.com/66HEX/frame/releases/download/$tag/frame-$tag-source.tar.gz" \
          --source-sha256 "${{ steps.flathub_sources.outputs.SOURCE_SHA256 }}" \
          --vendor-url "https://github.com/66HEX/frame/releases/download/$tag/frame-$tag-cargo-vendor.tar.gz" \
          --vendor-sha256 "${{ steps.flathub_sources.outputs.VENDOR_SHA256 }}" \
          --out target/flathub/repo
    - name: release::checkout_flathub
      if: steps.flathub_token.outputs.configured == 'true'
      uses: actions/checkout@v4
      with:
        repository: ${{ env.FLATHUB_REPOSITORY }}
        token: ${{ secrets.FLATHUB_GITHUB_TOKEN }}
        path: flathub
    - name: release::publish_flathub_pr
      if: steps.flathub_token.outputs.configured == 'true'
      working-directory: flathub
      env:
        GH_TOKEN: ${{ secrets.FLATHUB_GITHUB_TOKEN }}
        VERSION: ${{ needs.prepare_release.outputs.tag }}
      shell: bash
      run: |
        branch="frame-$VERSION"
        git checkout -B "$branch"
        cp ../target/flathub/repo/io.github._66HEX.Frame.yml .
        git config user.name "github-actions[bot]"
        git config user.email "github-actions[bot]@users.noreply.github.com"
        git add io.github._66HEX.Frame.yml
        if git diff --staged --quiet; then
          echo "No Flathub manifest changes to publish."
          exit 0
        fi
        git commit -m "Update Frame to $VERSION"
        gh auth setup-git
        git push --force-with-lease origin "$branch"
        if gh pr view "$branch" --repo "$FLATHUB_REPOSITORY" >/dev/null 2>&1; then
          gh pr edit "$branch" \
            --repo "$FLATHUB_REPOSITORY" \
            --title "Update Frame to $VERSION" \
            --body "Updates Frame to $VERSION using the GitHub release source and cargo vendor archives."
        else
          gh pr create \
            --repo "$FLATHUB_REPOSITORY" \
            --base master \
            --head "$branch" \
            --title "Update Frame to $VERSION" \
            --body "Updates Frame to $VERSION using the GitHub release source and cargo vendor archives."
        fi
    timeout-minutes: 20

  __NIXPKGS_JOBS__


  publish_winget:
    runs-on: ubuntu-22.04
    needs:
      - prepare_release
      - publish_release
    environment: production-distribution
    env:
      KOMAC_FORK_OWNER: 66HEX
      VERSION: ${{ needs.prepare_release.outputs.tag }}
    steps:
    - name: release::install_verified_komac
      shell: bash
      run: |
        set -euo pipefail
        archive="komac-__KOMAC_VERSION__-x86_64-unknown-linux-gnu.tar.gz"
        curl --fail --location --proto '=https' --tlsv1.2 \
          --output "/tmp/$archive" \
          "https://github.com/russellbanks/Komac/releases/download/v__KOMAC_VERSION__/$archive"
        echo '__KOMAC_LINUX_X86_64_SHA256__  /tmp/'"$archive" | sha256sum --check --strict
        tar --extract --gzip --file "/tmp/$archive" --directory /tmp komac
        install -m 0755 /tmp/komac /usr/local/bin/komac
    - name: release::publish_winget
      env:
        GITHUB_TOKEN: ${{ secrets.WINGET_ACC_TOKEN }}
      shell: bash
      run: |
        set -euo pipefail
        test -n "$GITHUB_TOKEN"
        komac update 66HEX.Frame \
          --version "$VERSION" \
          --urls "https://github.com/66HEX/frame/releases/download/$VERSION/Frame-x86_64.exe" \
          --submit
    timeout-minutes: 30
"#
    .replace(
        "  __NIXPKGS_JOBS__\n",
        &nixpkgs_release_jobs(
            Some("[prepare_release, publish_release]"),
            "${{ needs.prepare_release.outputs.tag }}",
        ),
    );
    render_workflow(&workflow)
}

#[expect(
    clippy::too_many_lines,
    reason = "The generated GitHub Actions jobs are kept as one raw template for easier diffing against YAML output."
)]
fn nixpkgs_release_jobs(prepare_needs: Option<&str>, version_expression: &str) -> String {
    let prepare_needs = prepare_needs
        .map(|needs| format!("    needs: {needs}\n"))
        .unwrap_or_default();
    let mut jobs = r#"  prepare_nixpkgs:
    runs-on: ubuntu-22.04
    needs: publish_release
    environment: production-distribution
    outputs:
      branch: ${{ steps.nixpkgs.outputs.branch }}
      commit_subject: ${{ steps.nixpkgs.outputs.commit_subject }}
      should_publish: ${{ steps.nixpkgs.outputs.should_publish }}
    env:
      NIXPKGS_FORK: 66HEX/nixpkgs
      NIXPKGS_UPSTREAM: NixOS/nixpkgs
      NIXPKGS_PACKAGE: frame-media-converter
      VERSION: __NIXPKGS_VERSION__
    steps:
    - name: release::check_nixpkgs_token
      id: token
      env:
        NIXPKGS_GITHUB_TOKEN: ${{ secrets.NIXPKGS_GITHUB_TOKEN }}
      shell: bash
      run: |
        if [[ -z "$NIXPKGS_GITHUB_TOKEN" ]]; then
          echo "::notice::NIXPKGS_GITHUB_TOKEN is not configured; skipping nixpkgs PR."
          echo "configured=false" >> "$GITHUB_OUTPUT"
        else
          echo "configured=true" >> "$GITHUB_OUTPUT"
        fi
    - name: steps::install_nix
      if: steps.token.outputs.configured == 'true'
      uses: cachix/install-nix-action@v31
      with:
        install_url: https://releases.nixos.org/nix/nix-__NIX_VERSION__/install
        extra_nix_config: |
          sandbox = true
    - name: release::checkout_nixpkgs
      if: steps.token.outputs.configured == 'true'
      uses: actions/checkout@v4
      with:
        repository: ${{ env.NIXPKGS_FORK }}
        token: ${{ secrets.NIXPKGS_GITHUB_TOKEN }}
        path: nixpkgs
        fetch-depth: 0
    - name: release::prepare_nixpkgs_branch
      id: nixpkgs
      if: steps.token.outputs.configured == 'true'
      working-directory: nixpkgs
      env:
        GH_TOKEN: ${{ secrets.NIXPKGS_GITHUB_TOKEN }}
      shell: bash
      run: |
        set -euo pipefail

        package_path="pkgs/by-name/fr/frame-media-converter/package.nix"
        branch="frame-media-converter-$VERSION"
        commit_subject="frame-media-converter: init at $VERSION"
        echo "branch=$branch" >> "$GITHUB_OUTPUT"

        git remote add upstream "https://github.com/$NIXPKGS_UPSTREAM.git"
        git fetch upstream master
        git checkout -B "$branch" upstream/master

        if ! grep -q '_66HEX = {' maintainers/maintainer-list.nix; then
          python3 - <<'PY'
        from pathlib import Path
        import re
        import sys

        path = Path("maintainers/maintainer-list.nix")
        text = path.read_text()
        entry = '''  _66HEX = {
            name = "Marek Jóźwiak";
            github = "66HEX";
            githubId = 168720167;
          };
        '''

        for match in re.finditer(r"^  ([A-Za-z0-9_]+) = \{\n", text, re.MULTILINE):
            if match.group(1) > "_66HEX":
                path.write_text(text[:match.start()] + entry + text[match.start():])
                break
        else:
            print("Could not find alphabetical insertion point for _66HEX.", file=sys.stderr)
            sys.exit(1)
        PY
        fi
        if ! grep -q '_66HEX = {' maintainers/maintainer-list.nix; then
          echo "::error::Could not insert _66HEX into maintainer-list.nix." >&2
          exit 1
        fi

        if [[ -f "$package_path" ]]; then
          commit_subject="frame-media-converter: update to $VERSION"
          perl -0pi -e 's/version = "[^"]+";/version = "$ENV{VERSION}";/' "$package_path"
          perl -0pi -e 's/hash = "[^"]+";/hash = lib.fakeHash;/' "$package_path"
          perl -0pi -e 's/cargoHash = "[^"]+";/cargoHash = lib.fakeHash;/' "$package_path"
        else
          mkdir -p "$(dirname "$package_path")"
          cat > "$package_path" <<'EOF'
        {
          lib,
          fetchFromGitHub,
          rustPlatform,
          pkg-config,
          makeWrapper,
          alsa-lib,
          ffmpeg,
          fontconfig,
          freetype,
          libdrm,
          libGL,
          libx11,
          libxcb,
          libxkbcommon,
          wayland,
        }:

        rustPlatform.buildRustPackage (finalAttrs: {
          pname = "frame-media-converter";
          version = "@VERSION@";

          __structuredAttrs = true;

          src = fetchFromGitHub {
            owner = "66HEX";
            repo = "frame";
            tag = finalAttrs.version;
            hash = lib.fakeHash;
          };

          cargoHash = lib.fakeHash;
          cargoBuildFlags = [ "--package" "frame-app" ];
          cargoTestFlags = [ "--package" "frame-app" ];

          nativeBuildInputs = [
            makeWrapper
            pkg-config
          ];

          buildInputs = [
            alsa-lib
            fontconfig
            freetype
            libdrm
            libGL
            libx11
            libxcb
            libxkbcommon
            wayland
          ];

          postInstall = ''
            install -Dm444 frame-app/resources/frame.desktop.in \
              $out/share/applications/frame.desktop
            substituteInPlace $out/share/applications/frame.desktop \
              --replace-fail '$APP_NAME' Frame \
              --replace-fail '$APP_CLI' frame \
              --replace-fail '$APP_ICON' frame

            install -Dm444 frame-app/resources/app-icons/32x32.png \
              $out/share/icons/hicolor/32x32/apps/frame.png
            install -Dm444 frame-app/resources/app-icons/64x64.png \
              $out/share/icons/hicolor/64x64/apps/frame.png
            install -Dm444 frame-app/resources/app-icons/128x128.png \
              $out/share/icons/hicolor/128x128/apps/frame.png
            install -Dm444 frame-app/resources/app-icons/128x128@2x.png \
              $out/share/icons/hicolor/256x256/apps/frame.png
            install -Dm444 frame-app/resources/app-icons/icon.png \
              $out/share/icons/hicolor/512x512/apps/frame.png
          '';

          postFixup = ''
            wrapProgram $out/bin/frame \
              --prefix PATH : ${lib.makeBinPath [ ffmpeg ]} \
              --set FRAME_USE_SYSTEM_MEDIA_TOOLS 1 \
              --set FRAME_UPDATE_EXPLANATION \
                'This Nixpkgs build is managed by Nix. Install updates through your Nix profile, flake, or NixOS configuration.'
          '';

          meta = {
            description = "Native desktop interface for FFmpeg media conversion";
            homepage = "https://github.com/66HEX/frame";
            changelog = "https://github.com/66HEX/frame/blob/${finalAttrs.version}/CHANGELOG.md";
            license = lib.licenses.gpl3Plus;
            mainProgram = "frame";
            maintainers = [ lib.maintainers._66HEX ];
            platforms = [
              "x86_64-linux"
              "aarch64-linux"
            ];
          };
        })
        EOF
          perl -0pi -e 's/\@VERSION\@/$ENV{VERSION}/g' "$package_path"
        fi

        build_and_capture_hash() {
          local log_file="$1"
          set +e
          nix-build -A "$NIXPKGS_PACKAGE" 2>&1 | tee "$log_file"
          local status="${PIPESTATUS[0]}"
          set -e
          return "$status"
        }

        extract_got_hash() {
          awk '/got:[[:space:]]+sha256-/ { print $2 }' "$1" | tail -n1
        }

        if build_and_capture_hash source-hash.log; then
          echo "::error::Source hash was unexpectedly already valid." >&2
          exit 1
        fi
        src_hash="$(extract_got_hash source-hash.log)"
        if [[ -z "$src_hash" ]]; then
          echo "::error::Could not determine nixpkgs source hash." >&2
          exit 1
        fi
        export SRC_HASH="$src_hash"
        perl -0pi -e 's/hash = lib\.fakeHash;/hash = "$ENV{SRC_HASH}";/' "$package_path"

        if build_and_capture_hash cargo-hash.log; then
          echo "::error::Cargo hash was unexpectedly already valid." >&2
          exit 1
        fi
        cargo_hash="$(extract_got_hash cargo-hash.log)"
        if [[ -z "$cargo_hash" ]]; then
          echo "::error::Could not determine nixpkgs cargo hash." >&2
          exit 1
        fi
        export CARGO_HASH="$cargo_hash"
        perl -0pi -e 's/cargoHash = lib\.fakeHash;/cargoHash = "$ENV{CARGO_HASH}";/' "$package_path"

        nix-build -A "$NIXPKGS_PACKAGE"
        nix-shell -I nixpkgs="$PWD" -p nixfmt-rfc-style --run "nixfmt $package_path maintainers/maintainer-list.nix"
        nix-build -A "$NIXPKGS_PACKAGE"
        nix-build lib/tests/maintainers.nix

        git config user.name "github-actions[bot]"
        git config user.email "github-actions[bot]@users.noreply.github.com"
        git add "$package_path" maintainers/maintainer-list.nix
        if git diff --staged --quiet; then
          echo "No nixpkgs changes to publish."
          echo "should_publish=false" >> "$GITHUB_OUTPUT"
          exit 0
        fi
        git commit -m "$commit_subject"
        gh auth setup-git
        git push --force-with-lease origin "$branch"
        echo "commit_subject=$commit_subject" >> "$GITHUB_OUTPUT"
        echo "should_publish=true" >> "$GITHUB_OUTPUT"
    timeout-minutes: 90

  validate_nixpkgs_aarch64:
    runs-on: ubuntu-22.04-arm
    needs: prepare_nixpkgs
    if: needs.prepare_nixpkgs.outputs.should_publish == 'true'
    env:
      NIXPKGS_FORK: 66HEX/nixpkgs
      NIXPKGS_PACKAGE: frame-media-converter
    steps:
    - name: steps::install_nix
      uses: cachix/install-nix-action@v31
      with:
        install_url: https://releases.nixos.org/nix/nix-__NIX_VERSION__/install
        extra_nix_config: |
          sandbox = true
    - name: release::checkout_nixpkgs
      uses: actions/checkout@v4
      with:
        repository: ${{ env.NIXPKGS_FORK }}
        ref: ${{ needs.prepare_nixpkgs.outputs.branch }}
        path: nixpkgs
    - name: release::build_nixpkgs_aarch64
      working-directory: nixpkgs
      run: nix-build -A "$NIXPKGS_PACKAGE"
    timeout-minutes: 90

  publish_nixpkgs:
    runs-on: ubuntu-22.04
    environment: production-distribution
    needs:
      - prepare_nixpkgs
      - validate_nixpkgs_aarch64
    if: needs.prepare_nixpkgs.outputs.should_publish == 'true'
    env:
      NIXPKGS_UPSTREAM: NixOS/nixpkgs
    steps:
    - name: release::publish_nixpkgs_pr
      env:
        BRANCH: ${{ needs.prepare_nixpkgs.outputs.branch }}
        COMMIT_SUBJECT: ${{ needs.prepare_nixpkgs.outputs.commit_subject }}
        GH_TOKEN: ${{ secrets.NIXPKGS_GITHUB_TOKEN }}
      shell: bash
      run: |
        set -euo pipefail

        existing_pr="$(gh pr list \
          --repo "$NIXPKGS_UPSTREAM" \
          --head "66HEX:$BRANCH" \
          --json number \
          --jq '.[0].number')"
        if [[ -n "$existing_pr" ]]; then
          gh pr edit "$existing_pr" \
            --repo "$NIXPKGS_UPSTREAM" \
            --title "$COMMIT_SUBJECT" \
            --body "Updates Frame to ${BRANCH#frame-media-converter-} from the GitHub release tag."
        else
          gh pr create \
            --repo "$NIXPKGS_UPSTREAM" \
            --base master \
            --head "66HEX:$BRANCH" \
            --title "$COMMIT_SUBJECT" \
            --body "Updates Frame to ${BRANCH#frame-media-converter-} from the GitHub release tag."
        fi
    timeout-minutes: 10
"#
        .to_string();
    jobs = jobs.replace("    needs: publish_release\n", &prepare_needs);
    jobs.replace("__NIXPKGS_VERSION__", version_expression)
}

fn publish_nixpkgs_workflow() -> String {
    let mut workflow = r#"# Generated from xtask::workflows::publish_nixpkgs
# Rebuild with `cargo xtask workflows`.
name: publish nixpkgs
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: '1'
on:
  workflow_dispatch:
    inputs:
      tag:
        description: Release tag to publish to nixpkgs, without a v prefix.
        required: true
permissions:
  contents: read
jobs:
  validate_release:
    runs-on: ubuntu-22.04
    outputs:
      tag: ${{ steps.release.outputs.tag }}
    steps:
    - name: release::validate_signed_published_tag
      id: release
      env:
        GH_TOKEN: ${{ github.token }}
        TAG: ${{ inputs.tag }}
      shell: bash
      run: |
        set -euo pipefail
        if [[ ! "$TAG" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
          echo "::error::Release tag must be stable semver without a v prefix." >&2
          exit 1
        fi
        ref_json="$(gh api "repos/$GITHUB_REPOSITORY/git/ref/tags/$TAG")"
        test "$(jq -r '.object.type' <<<"$ref_json")" = tag
        tag_json="$(gh api "repos/$GITHUB_REPOSITORY/git/tags/$(jq -r '.object.sha' <<<"$ref_json")")"
        test "$(jq -r '.verification.verified' <<<"$tag_json")" = true
        test "$(jq -r '.object.type' <<<"$tag_json")" = commit
        commit_sha="$(jq -r '.object.sha' <<<"$tag_json")"
        comparison="$(gh api "repos/$GITHUB_REPOSITORY/compare/$commit_sha...master")"
        comparison_status="$(jq -r '.status' <<<"$comparison")"
        if [[ "$comparison_status" != ahead && "$comparison_status" != identical ]]; then
          echo "::error::Release tag is not an ancestor of master." >&2
          exit 1
        fi
        release_json="$(gh api "repos/$GITHUB_REPOSITORY/releases/tags/$TAG")"
        test "$(jq -r '.draft or .prerelease' <<<"$release_json")" = false
        jq -e '[.assets[].name] | index("SHA256SUMS") != null' <<<"$release_json" >/dev/null
        echo "tag=$TAG" >> "$GITHUB_OUTPUT"
    timeout-minutes: 10
"#
    .to_string();
    workflow.push_str(&nixpkgs_release_jobs(
        Some("validate_release"),
        "${{ needs.validate_release.outputs.tag }}",
    ));
    render_workflow(&workflow)
}

fn repo_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or(XtaskError::RepoRoot)
}

#[derive(Debug)]
enum XtaskError {
    ArchiveEntryMissing {
        expected_names: String,
    },
    CommandFailed {
        program: String,
        status: std::process::ExitStatus,
    },
    CommandSpawn {
        program: String,
        source: io::Error,
    },
    Download {
        url: String,
        source: Box<ureq::Error>,
    },
    HashMismatch {
        subject: String,
        expected: String,
        actual: String,
    },
    GeneratedWorkflowOutOfDate {
        path: String,
    },
    Help,
    Io(io::Error),
    RepoRoot,
    Usage(String),
    Update(frame_updater::UpdateError),
    Json(serde_json::Error),
    Zip(zip::result::ZipError),
}

impl fmt::Display for XtaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveEntryMissing { expected_names } => {
                write!(
                    formatter,
                    "archive did not contain expected file: {expected_names}"
                )
            }
            Self::CommandFailed { program, status } => {
                write!(formatter, "`{program}` failed with status {status}")
            }
            Self::CommandSpawn { program, source } => {
                write!(formatter, "failed to run `{program}`: {source}")
            }
            Self::Download { url, source } => {
                write!(formatter, "failed to download `{url}`: {source}")
            }
            Self::HashMismatch {
                subject,
                expected,
                actual,
            } => write!(
                formatter,
                "SHA-256 mismatch for `{subject}`: expected {expected}, got {actual}"
            ),
            Self::GeneratedWorkflowOutOfDate { path } => write!(
                formatter,
                "generated workflow `{path}` is out of date; run `cargo xtask workflows`"
            ),
            Self::Help => Ok(()),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::RepoRoot => write!(formatter, "failed to resolve repository root"),
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::Update(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "failed to process JSON: {error}"),
            Self::Zip(error) => write!(formatter, "failed to read zip archive: {error}"),
        }
    }
}

impl From<io::Error> for XtaskError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<zip::result::ZipError> for XtaskError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::Zip(error)
    }
}

impl From<serde_json::Error> for XtaskError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<frame_updater::UpdateError> for XtaskError {
    fn from(error: frame_updater::UpdateError) -> Self {
        Self::Update(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_ffmpeg_options_parse_platform_arch_and_force() {
        let args = vec![
            "--force".to_string(),
            "--platform".to_string(),
            "darwin".to_string(),
            "--arch".to_string(),
            "aarch64".to_string(),
        ];

        let options = SetupFfmpegOptions::parse(&args).unwrap();

        assert_eq!(
            options,
            SetupFfmpegOptions {
                force: true,
                platform: Some("darwin".to_string()),
                arch: Some("aarch64".to_string()),
            }
        );
    }

    #[test]
    fn ffmpeg_target_for_maps_macos_arm64_to_darwin_runtime_names() {
        let target = ffmpeg_target_for("darwin", "arm64").unwrap();

        let FfmpegTarget::Individual { binaries, .. } = target else {
            panic!("expected individual macOS binaries");
        };

        let names = binaries
            .iter()
            .map(|entry| entry.destination_name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "ffmpeg-aarch64-apple-darwin",
                "ffprobe-aarch64-apple-darwin"
            ]
        );
    }

    #[test]
    fn ffmpeg_target_for_maps_windows_x64_to_shared_archive() {
        let target = ffmpeg_target_for("win32", "x64").unwrap();

        let FfmpegTarget::SharedArchive { entries, .. } = target else {
            panic!("expected shared Windows archive");
        };

        let names = entries
            .iter()
            .map(|entry| entry.destination_name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "ffmpeg-x86_64-pc-windows-msvc.exe",
                "ffprobe-x86_64-pc-windows-msvc.exe"
            ]
        );
    }

    #[test]
    fn archive_entry_name_matches_nested_zip_paths() {
        assert!(archive_entry_name_matches(
            "ffmpeg-n8.1.2-21-gce3c09c101-win64-gpl-8.1/bin/ffprobe.exe",
            &["ffprobe.exe"],
        ));
    }

    #[test]
    fn windows_ffmpeg_archive_uses_retained_monthly_snapshot() {
        assert_eq!(
            WINDOWS_X86_64_ARCHIVE.url,
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-06-30-13-34/ffmpeg-n8.1.2-21-gce3c09c101-win64-gpl-8.1.zip"
        );
    }

    #[test]
    fn verify_bytes_sha256_accepts_matching_digest() {
        let result = verify_bytes_sha256(
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "test bytes",
        );

        assert!(result.is_ok(), "unexpected error: {result:?}");
    }

    #[test]
    fn verify_bytes_sha256_rejects_mismatched_digest() {
        let error = verify_bytes_sha256(b"abc", &"0".repeat(64), "test bytes").unwrap_err();

        assert!(matches!(error, XtaskError::HashMismatch { .. }));
    }

    #[test]
    fn ffmpeg_binary_needs_download_accepts_verified_cache() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("ffmpeg");
        fs::write(&destination, b"abc").unwrap();
        let entry =
            test_ffmpeg_entry("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");

        let needs_download = ffmpeg_binary_needs_download(&entry, &destination, false).unwrap();

        assert!(!needs_download);
    }

    #[test]
    fn ffmpeg_binary_needs_download_rejects_corrupt_cache() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("ffmpeg");
        fs::write(&destination, b"corrupt").unwrap();
        let entry =
            test_ffmpeg_entry("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");

        let needs_download = ffmpeg_binary_needs_download(&entry, &destination, false).unwrap();

        assert!(needs_download);
    }

    #[test]
    fn write_archive_file_preserves_destination_on_hash_mismatch() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("ffmpeg");
        fs::write(&destination, b"verified binary").unwrap();
        let mut downloaded = Cursor::new(b"corrupt binary");

        let result = write_archive_file(&mut downloaded, &destination, &"0".repeat(64), false);

        assert!(
            result.is_err() && fs::read(destination).unwrap() == b"verified binary",
            "a rejected download must not replace the existing binary"
        );
    }

    #[test]
    fn all_ffmpeg_sources_are_pinned_to_version_8_1_2() {
        let individual_archives = [
            MACOS_X86_64_BINARIES,
            MACOS_AARCH64_BINARIES,
            LINUX_X86_64_BINARIES,
            LINUX_AARCH64_BINARIES,
        ]
        .into_iter()
        .flat_map(|entries| entries.iter().filter_map(|entry| entry.archive));
        let archives = individual_archives.chain([WINDOWS_X86_64_ARCHIVE]);

        assert!(archives.into_iter().all(|archive| {
            archive.url.contains(FFMPEG_VERSION)
                && !archive.url.contains("latest")
                && is_sha256(archive.sha256)
        }));
    }

    #[test]
    fn all_ffmpeg_binaries_have_pinned_sha256() {
        let binaries = [
            MACOS_X86_64_BINARIES,
            MACOS_AARCH64_BINARIES,
            LINUX_X86_64_BINARIES,
            LINUX_AARCH64_BINARIES,
            WINDOWS_X86_64_BINARIES,
        ]
        .into_iter()
        .flatten();

        assert!(binaries.into_iter().all(|entry| is_sha256(entry.sha256)));
    }

    fn is_sha256(value: &str) -> bool {
        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    fn test_ffmpeg_entry(sha256: &'static str) -> FfmpegBinaryEntry {
        FfmpegBinaryEntry {
            id: "ffmpeg",
            archive: None,
            expected_names: &["ffmpeg"],
            destination_name: "ffmpeg",
            sha256,
            make_executable: false,
        }
    }

    #[test]
    fn release_workflow_extracts_release_notes_from_changelog() {
        let workflow = release_workflow();

        assert!(workflow.contains("release::extract_release_notes"));
        assert!(workflow.contains("CHANGELOG.md > target/release-files/release-notes.md"));
    }

    #[test]
    fn release_workflow_uses_changelog_notes_for_update_manifest() {
        let workflow = release_workflow();

        assert!(
            workflow.contains(
                "--release-notes-markdown \"$(< target/release-files/release-notes.md)\""
            )
        );
    }

    #[test]
    fn release_workflow_uses_changelog_notes_for_github_release() {
        let workflow = release_workflow();

        assert!(workflow.contains("--notes-file release-notes.md"));
        assert!(!workflow.contains("--generate-notes"));
    }

    #[test]
    fn release_workflow_publishes_appimage_update_assets() {
        let workflow = release_workflow();

        assert!(workflow.contains("Frame-x86_64.AppImage"));
        assert!(workflow.contains("Frame-x86_64.AppImage.zsync"));
        assert!(workflow.contains("Frame-aarch64.AppImage"));
        assert!(workflow.contains("Frame-aarch64.AppImage.zsync"));
    }

    #[test]
    fn release_workflow_publishes_flathub_source_archives() {
        let workflow = release_workflow();

        assert!(workflow.contains("release::prepare_flathub_sources"));
        assert!(workflow.contains(
            "target/flathub/frame-${{ needs.prepare_release.outputs.version }}-source.tar.gz"
        ));
        assert!(workflow.contains(
            "target/flathub/frame-${{ needs.prepare_release.outputs.version }}-cargo-vendor.tar.gz"
        ));
    }

    #[test]
    fn release_workflow_updates_flathub_manifest_repository() {
        let workflow = release_workflow();

        assert!(workflow.contains("update_flathub:"));
        assert!(workflow.contains("FLATHUB_REPOSITORY: flathub/io.github._66HEX.Frame"));
        assert!(workflow.contains("cargo xtask flathub-manifest"));
        assert!(workflow.contains("FLATHUB_GITHUB_TOKEN"));
    }

    #[test]
    fn release_workflow_publishes_nixpkgs_pr_from_release_tag() {
        let workflow = release_workflow();

        assert!(workflow.contains("prepare_nixpkgs:"));
        assert!(workflow.contains("validate_nixpkgs_aarch64:"));
        assert!(workflow.contains("publish_nixpkgs:"));
        assert!(workflow.contains("needs: [prepare_release, publish_release]"));
        assert!(workflow.contains("runs-on: ubuntu-22.04-arm"));
        assert!(workflow.contains("release::build_nixpkgs_aarch64"));
        assert!(workflow.contains("- validate_nixpkgs_aarch64"));
        assert!(workflow.contains("NIXPKGS_GITHUB_TOKEN"));
        assert!(workflow.contains("NIXPKGS_UPSTREAM: NixOS/nixpkgs"));
        assert!(workflow.contains("NIXPKGS_PACKAGE: frame-media-converter"));
        assert!(workflow.contains("Could not find alphabetical insertion point for _66HEX."));
        assert!(workflow.contains("__structuredAttrs = true;"));
        assert!(!workflow.contains("_0x4A6F"));
        assert!(!workflow.contains("nix-build -A \"$NIXPKGS_PACKAGE\" -L"));
        assert!(workflow.contains("nix-shell -I nixpkgs=\"$PWD\""));
        assert!(workflow.contains("VERSION: ${{ needs.prepare_release.outputs.tag }}"));
        assert!(workflow.contains(r#""x86_64-linux""#));
        assert!(workflow.contains(r#""aarch64-linux""#));
        assert!(workflow.contains("gh pr create"));
        assert!(!workflow.contains("0.31.0"));
    }

    #[test]
    fn publish_nixpkgs_workflow_only_publishes_nixpkgs_from_manual_tag() {
        let workflow = publish_nixpkgs_workflow();

        assert!(workflow.contains("name: publish nixpkgs"));
        assert!(workflow.contains("workflow_dispatch:"));
        assert!(workflow.contains("description: Release tag to publish to nixpkgs"));
        assert!(workflow.contains("prepare_nixpkgs:"));
        assert!(workflow.contains("validate_nixpkgs_aarch64:"));
        assert!(workflow.contains("publish_nixpkgs:"));
        assert!(!workflow.contains("nix-build -A \"$NIXPKGS_PACKAGE\" -L"));
        assert!(workflow.contains("nix-shell -I nixpkgs=\"$PWD\""));
        assert!(!workflow.contains("needs: publish_release"));
        assert!(!workflow.contains("publish_release:"));
        assert!(!workflow.contains("publish_winget:"));
        assert!(!workflow.contains("update_homebrew_tap:"));
        assert!(!workflow.contains("update_flathub:"));
        assert!(!workflow.contains("build_linux_x86_64:"));
        assert!(!workflow.contains("build_macos_aarch64:"));
    }

    #[test]
    fn generated_workflows_pin_every_external_action_to_a_full_commit_sha() {
        for (path, workflow) in generated_workflows() {
            for line in workflow.lines() {
                let Some(reference) = line.trim().strip_prefix("uses: ") else {
                    continue;
                };
                if reference.starts_with("./") {
                    continue;
                }
                let Some((_, revision)) = reference.split_once('@') else {
                    panic!("external action in {path} has no revision: {reference}");
                };
                let revision = revision.split_whitespace().next().unwrap();
                assert!(
                    revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
                    "external action in {path} is not pinned to a full SHA: {reference}"
                );
            }
        }
    }

    #[test]
    fn generated_workflows_disable_checkout_credentials() {
        for (path, workflow) in generated_workflows() {
            let checkout_count = workflow.matches("uses: actions/checkout@").count();
            let disabled_count = workflow.matches("persist-credentials: false").count();
            assert_eq!(
                checkout_count, disabled_count,
                "each checkout in {path} must disable persisted credentials"
            );
        }
    }

    #[test]
    fn generated_workflows_use_explicit_least_privilege_permissions() {
        for (path, workflow) in generated_workflows() {
            assert!(
                workflow.contains("permissions:\n  contents: read"),
                "{path} must default GITHUB_TOKEN to contents: read"
            );
            assert!(!workflow.contains("permissions: write-all"));
        }

        let release = release_workflow();
        assert_eq!(release.matches("      contents: write").count(), 1);
        assert_eq!(release.matches("      id-token: write").count(), 1);
        assert_eq!(release.matches("      attestations: write").count(), 1);
        assert!(!release.contains("      actions: write"));

        let codeql = codeql_workflow();
        assert_eq!(codeql.matches("  security-events: write").count(), 1);
    }

    #[test]
    fn generated_workflows_pin_installed_tool_versions() {
        for (path, workflow) in generated_workflows() {
            for line in workflow.lines() {
                if line.contains("cargo install ") {
                    assert!(
                        line.contains(" --version "),
                        "cargo tool in {path} has no exact version: {line}"
                    );
                }
                if line.contains("rustup toolchain install ") {
                    assert!(
                        line.contains(RUST_VERSION),
                        "Rust toolchain in {path} is not pinned: {line}"
                    );
                }
                if line.contains("choco install ") {
                    assert!(
                        line.contains(" --version "),
                        "Chocolatey tool in {path} has no exact version: {line}"
                    );
                }
            }

            let rust_install_count = workflow.matches("rustup toolchain install ").count();
            assert_eq!(
                rust_install_count,
                workflow
                    .matches(&format!("rustup default {RUST_VERSION}"))
                    .count(),
                "every Rust installation in {path} must activate the pinned toolchain"
            );
            assert_eq!(
                rust_install_count,
                workflow
                    .matches(&format!(
                        "test \"$(rustc --version | awk '{{print $2}}')\" = \"{RUST_VERSION}\""
                    ))
                    .count(),
                "every Rust installation in {path} must verify the active version"
            );

            let nix_action_count = workflow.matches("uses: cachix/install-nix-action@").count();
            let pinned_nix_count = workflow
                .matches(&format!(
                    "install_url: https://releases.nixos.org/nix/nix-{NIX_VERSION}/install"
                ))
                .count();
            assert_eq!(
                nix_action_count, pinned_nix_count,
                "every Nix installation in {path} must use the pinned version"
            );
        }

        for workflow in [run_bundling_workflow(), release_workflow()] {
            assert!(workflow.contains(&format!(
                "/appimagetool/releases/download/{APPIMAGETOOL_VERSION}/"
            )));
            assert!(workflow.contains(APPIMAGETOOL_X86_64_SHA256));
            assert!(workflow.contains(APPIMAGETOOL_AARCH64_SHA256));
            assert!(!workflow.contains("/continuous/"));
        }
    }

    #[test]
    fn release_workflow_enforces_immutable_verified_release_pipeline() {
        let workflow = release_workflow();

        for required in [
            "release::resolve_and_verify_signed_tag",
            ".verification.verified",
            "git merge-base --is-ancestor",
            "actions/workflows/ci.yml/runs",
            "A manual release must be dispatched from the exact requested tag ref",
            "TAG_OBJECT_SHA",
            "Verified release commit is no longer an ancestor of master",
            "generate_release_metadata:",
            "sign_macos_x86_64:",
            "sign_macos_aarch64:",
            "sign_windows_x86_64:",
            "sign_release_metadata:",
            "assemble_release:",
            "publish_release:",
            "Frame.cdx.json",
            "SHA256SUMS",
            "security::attest_release_assets",
            "--draft",
            ".assets[] | select(.name == $name) | .digest",
        ] {
            assert!(
                workflow.contains(required),
                "missing release guard: {required}"
            );
        }
        for forbidden in ["--clobber", "releases/download/continuous", "@stable"] {
            assert!(
                !workflow.contains(forbidden),
                "release workflow contains mutable or overwrite path: {forbidden}"
            );
        }
    }

    #[test]
    fn release_workflow_isolates_platform_builds_from_signing_secrets() {
        let workflow = release_workflow();

        for (build_job, signing_job) in [
            ("build_macos_x86_64", "sign_macos_x86_64"),
            ("build_macos_aarch64", "sign_macos_aarch64"),
            ("build_windows_x86_64", "sign_windows_x86_64"),
        ] {
            let build = workflow_job(&workflow, build_job);
            assert!(build.contains("unsigned"));
            assert!(!build.contains("secrets."));
            assert!(!build.contains("environment: production-release"));

            let signing = workflow_job(&workflow, signing_job);
            assert!(signing.contains("environment: production-release"));
            assert!(signing.contains("secrets."));
            assert!(signing.contains("outputs.unsigned_sha256"));
            assert!(!signing.contains("cargo build"));
            assert!(!signing.contains("cargo bundle"));
        }
    }

    fn workflow_job<'a>(workflow: &'a str, name: &str) -> &'a str {
        let marker = format!("\n  {name}:\n");
        let (_, rest) = workflow
            .split_once(&marker)
            .unwrap_or_else(|| panic!("missing workflow job {name}"));
        rest.split("\n\n  ")
            .next()
            .expect("workflow job should have content")
    }

    #[test]
    fn run_bundling_requires_a_new_labeled_event_and_protected_environment() {
        let workflow = run_bundling_workflow();

        assert!(workflow.contains("types:\n      - labeled"));
        assert!(!workflow.contains("synchronize"));
        assert_eq!(workflow.matches("environment: run-bundling").count(), 5);
        assert!(!workflow.contains("secrets."));
    }

    #[test]
    fn flathub_template_uses_runtime_media_tools_without_bundled_binaries() {
        let template = include_str!("../../../packaging/flathub/io.github._66HEX.Frame.yml.in");

        assert!(template.contains("install -Dm755 target/release/frame /app/bin/frame"));
        assert!(template.contains("--env=FRAME_USE_SYSTEM_MEDIA_TOOLS=1"));
        assert!(template.contains("packaging/flathub/io.github._66HEX.Frame.desktop"));
        assert!(template.contains("packaging/flathub/io.github._66HEX.Frame.metainfo.xml"));
        assert!(!template.contains("desktop-file-edit"));
        assert!(!template.contains("sed"));
        assert!(!template.contains("/app/lib/frame/frame"));
        assert!(!template.contains("export FRAME_USE_SYSTEM_MEDIA_TOOLS=1"));
        assert!(!template.contains("install -Dm644 LICENSE"));
        assert!(!template.contains("strip-components: 1"));
        assert!(!template.contains("resources/binaries"));
        assert!(!template.contains("frame-update-helper"));
        assert!(!template.contains("ffmpeg-full"));
        assert!(!template.contains("add-extensions"));
        assert!(!template.contains("--filesystem=home"));
        assert!(!template.contains("--talk-name=org.freedesktop.Notifications"));
        assert!(!template.contains("--talk-name=org.freedesktop.portal.Desktop"));
        assert!(!template.contains("--socket=session-bus"));
    }

    #[test]
    fn flathub_desktop_file_is_ready_to_install() {
        let desktop = include_str!("../../../packaging/flathub/io.github._66HEX.Frame.desktop");

        assert!(
            desktop.contains("Exec=frame")
                && desktop.contains("TryExec=frame")
                && desktop.contains("Icon=io.github._66HEX.Frame")
                && !desktop.contains("$APP_")
        );
    }

    #[test]
    fn flathub_metainfo_contains_release_metadata() {
        let metainfo =
            include_str!("../../../packaging/flathub/io.github._66HEX.Frame.metainfo.xml");

        assert!(metainfo.contains(r#"<release version="0.31.1" date="2026-07-14" />"#));
    }

    #[test]
    fn flathub_metainfo_validation_rejects_a_different_release() {
        let metainfo = r#"<release version="0.31.0" date="2026-07-11" />"#;

        let error = validate_flathub_metainfo_version(metainfo, "0.31.1").unwrap_err();

        assert!(matches!(error, XtaskError::Usage(_)));
    }

    #[test]
    fn devel_flatpak_template_uses_notification_portal_without_direct_bus_access() {
        let template =
            include_str!("../../../packaging/flatpak/io.github._66HEX.Frame.Devel.yml.in");

        assert!(!template.contains("--talk-name=org.freedesktop.Notifications"));
        assert!(!template.contains("--talk-name=org.freedesktop.portal.Desktop"));
        assert!(!template.contains("--socket=session-bus"));
    }

    #[test]
    fn run_bundling_workflow_requires_appimage_zsync_assets() {
        let workflow = run_bundling_workflow();

        assert!(workflow.contains("zsync"));
        assert!(workflow.contains("run_bundling::verify_appimage_update_information"));
        assert!(workflow.contains("--appimage-updateinformation"));
        assert!(
            workflow.contains("gh-releases-zsync|66HEX|frame|latest|Frame-x86_64.AppImage.zsync")
        );
        assert!(
            workflow.contains("gh-releases-zsync|66HEX|frame|latest|Frame-aarch64.AppImage.zsync")
        );
        assert!(workflow.contains("target/release/Frame-x86_64.AppImage.zsync"));
        assert!(workflow.contains("target/release/Frame-aarch64.AppImage.zsync"));
    }

    #[test]
    fn release_workflow_verifies_appimage_update_information() {
        let workflow = release_workflow();

        assert!(workflow.contains("release::verify_linux_x86_64_appimage_update_information"));
        assert!(workflow.contains("release::verify_linux_aarch64_appimage_update_information"));
        assert!(workflow.contains("--appimage-updateinformation"));
        assert!(
            workflow.contains("gh-releases-zsync|66HEX|frame|latest|Frame-x86_64.AppImage.zsync")
        );
        assert!(
            workflow.contains("gh-releases-zsync|66HEX|frame|latest|Frame-aarch64.AppImage.zsync")
        );
    }

    #[test]
    fn release_workflow_keeps_appimage_and_flatpak_out_of_frame_update_manifest() {
        let workflow = release_workflow();

        assert!(!workflow.contains(":linux-x86_64:linux_appimage"));
        assert!(!workflow.contains(":linux-aarch64:linux_appimage"));
        assert!(!workflow.contains(":linux-x86_64:flatpak"));
        assert!(!workflow.contains(":linux-aarch64:flatpak"));
    }
}
