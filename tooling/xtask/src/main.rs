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
const RUN_BUNDLING_WORKFLOW_PATH: &str = ".github/workflows/run_bundling.yml";
const RELEASE_WORKFLOW_PATH: &str = ".github/workflows/release.yml";
const PUBLISH_NIXPKGS_WORKFLOW_PATH: &str = ".github/workflows/publish_nixpkgs.yml";
const FLATHUB_MANIFEST_TEMPLATE_PATH: &str = "packaging/flathub/io.github._66HEX.Frame.yml.in";
const FLATHUB_METAINFO_TEMPLATE_PATH: &str =
    "packaging/flathub/io.github._66HEX.Frame.metainfo.xml.in";
const FFMPEG_VERSION: &str = "8.1.2";

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
        "workflows" => write_workflows(),
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

const WINDOWS_X86_64_ARCHIVE: FfmpegArchive = FfmpegArchive {
    url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-06-28-13-24/ffmpeg-n8.1.2-win64-gpl-8.1.zip",
    sha256: "dd247ae801e42777eb8d8b0a0c322f78862c265dad5749f9859cc379665e279e",
};

const WINDOWS_X86_64_BINARIES: &[FfmpegBinaryEntry] = &[
    FfmpegBinaryEntry {
        id: "ffmpeg",
        archive: None,
        expected_names: &["ffmpeg.exe"],
        destination_name: "ffmpeg-x86_64-pc-windows-msvc.exe",
        sha256: "bc4a55f7e5b6ff537890e20f4a178dd6d614073c38612b29c37df665f3170df5",
        make_executable: false,
    },
    FfmpegBinaryEntry {
        id: "ffprobe",
        archive: None,
        expected_names: &["ffprobe.exe"],
        destination_name: "ffprobe-x86_64-pc-windows-msvc.exe",
        sha256: "d08266eac436dc44f00d6e15a15aa3e3eab9e6bd408f9ab49657d32687510670",
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
    let metainfo = render_flathub_metainfo_template(
        &fs::read_to_string(root.join(FLATHUB_METAINFO_TEMPLATE_PATH))?,
        &options,
    );

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
        .replace("@@FRAME_VERSION@@", &options.version)
        .replace("@@FRAME_RELEASE_DATE@@", &options.release_date)
        .replace("@@FRAME_SOURCE_URL@@", &options.source_url)
        .replace("@@FRAME_SOURCE_SHA256@@", &options.source_sha256)
        .replace("@@FRAME_CARGO_VENDOR_URL@@", &options.vendor_url)
        .replace("@@FRAME_CARGO_VENDOR_SHA256@@", &options.vendor_sha256)
}

fn render_flathub_metainfo_template(template: &str, options: &FlathubManifestOptions) -> String {
    template
        .replace("@FRAME_FLATHUB_VERSION@", &options.version)
        .replace("@FRAME_FLATHUB_RELEASE_DATE@", &options.release_date)
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
    release_date: String,
    source_url: String,
    source_sha256: String,
    vendor_url: String,
    vendor_sha256: String,
    out: PathBuf,
}

impl FlathubManifestOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut version = None;
        let mut release_date = None;
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
                "--release-date" => {
                    release_date = Some(required_option_value(args, &mut index, "--release-date")?);
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
  --release-date <yyyy-mm-dd>
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
            release_date: release_date
                .ok_or_else(|| XtaskError::Usage("missing --release-date".to_string()))?,
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
        if options.release_date.len() != 10 {
            return Err(XtaskError::Usage(
                "--release-date must use yyyy-mm-dd format".to_string(),
            ));
        }
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
    run_command(
        "cargo",
        &["fmt", "--manifest-path", "frame-core/Cargo.toml", "--check"],
    )?;
    run_command(
        "cargo",
        &["fmt", "--manifest-path", "frame-app/Cargo.toml", "--check"],
    )?;
    run_command(
        "cargo",
        &[
            "fmt",
            "--manifest-path",
            "tooling/xtask/Cargo.toml",
            "--check",
        ],
    )?;
    run_command(
        "cargo",
        &["test", "--manifest-path", "frame-core/Cargo.toml"],
    )?;
    run_command(
        "cargo",
        &["test", "--manifest-path", "frame-app/Cargo.toml"],
    )?;
    run_command(
        "cargo",
        &["test", "--manifest-path", "tooling/xtask/Cargo.toml"],
    )?;
    run_command(
        "cargo",
        &[
            "clippy",
            "--manifest-path",
            "frame-core/Cargo.toml",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_command(
        "cargo",
        &[
            "clippy",
            "--manifest-path",
            "frame-app/Cargo.toml",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_command(
        "cargo",
        &[
            "clippy",
            "--manifest-path",
            "tooling/xtask/Cargo.toml",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_command("bash", &["-n", "script/bundle-mac"])?;
    run_command("bash", &["-n", "script/bundle-linux"])?;
    run_command("git", &["diff", "--check"])?;
    Ok(())
}

fn write_workflows() -> Result<()> {
    let root = repo_root()?;
    for (relative_path, content) in [
        (RUN_BUNDLING_WORKFLOW_PATH, run_bundling_workflow()),
        (RELEASE_WORKFLOW_PATH, release_workflow()),
        (PUBLISH_NIXPKGS_WORKFLOW_PATH, publish_nixpkgs_workflow()),
    ] {
        let path = root.join(relative_path);
        fs::create_dir_all(path.parent().expect("workflow path should have a parent"))?;
        fs::write(&path, content)?;
        println!("Wrote {}", path.display());
    }
    Ok(())
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
      - synchronize
";

    let jobs = [
        linux_job("x86_64", "ubuntu-22.04"),
        linux_job("aarch64", "ubuntu-22.04-arm"),
        macos_job("x86_64", "x86_64-apple-darwin", "macos-26-intel"),
        macos_job("aarch64", "aarch64-apple-darwin", "macos-26"),
        windows_job("x86_64", "windows-2022"),
    ]
    .join("");

    format!("{header}jobs:\n{jobs}")
}

const fn bundle_if_expression() -> &'static str {
    "      github.event_name == 'workflow_dispatch' ||\n      (github.event.action == 'labeled' && github.event.label.name == 'run-bundling') ||\n      (github.event.action == 'synchronize' && contains(github.event.pull_request.labels.*.name, 'run-bundling'))"
}

const fn checkout_step() -> &'static str {
    r"    - name: steps::checkout_repo
      uses: actions/checkout@v4
"
}

const fn setup_rust_step() -> &'static str {
    r"    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
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
    env:
      CARGO_INCREMENTAL: 0
    steps:
{checkout}{rust}    - name: steps::setup_linux
      run: |
        sudo apt-get update
        sudo apt-get install -y {linux_packages}
    - name: steps::setup_appimagetool
      run: |
        curl -L --fail -o /tmp/appimagetool.AppImage https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-{appimagetool_arch}.AppImage
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
      uses: actions/upload-artifact@v4
      with:
        name: frame-linux-{arch}.tar.gz
        path: target/release/frame-linux-{arch}.tar.gz
        if-no-files-found: error
    - name: run_bundling::upload_appimage_artifact
      uses: actions/upload-artifact@v4
      with:
        name: Frame-{arch}.AppImage
        path: target/release/Frame-{arch}.AppImage
        if-no-files-found: error
    - name: run_bundling::upload_appimage_zsync_artifact
      uses: actions/upload-artifact@v4
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
    env:
      CARGO_INCREMENTAL: 0
    steps:
{checkout}{rust}    - name: steps::install_cargo_bundle
      run: cargo install cargo-bundle --locked
    - name: ./script/bundle-mac
      run: ./script/bundle-mac {target}
    - name: run_bundling::upload_artifact
      uses: actions/upload-artifact@v4
      with:
        name: Frame-{arch}.dmg
        path: target/{target}/release/Frame-{arch}.dmg
        if-no-files-found: error
    - name: run_bundling::upload_update_artifact
      uses: actions/upload-artifact@v4
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
    env:
      CARGO_INCREMENTAL: 0
    steps:
{checkout}{rust}    - name: steps::setup_inno
      shell: pwsh
      run: choco install innosetup --no-progress -y
    - name: ./script/bundle-windows.ps1
      shell: pwsh
      run: ./script/bundle-windows.ps1 -Architecture {arch}
    - name: run_bundling::upload_artifact
      uses: actions/upload-artifact@v4
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
    r#"# Generated from xtask::workflows::release
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
  contents: write
jobs:
  build_linux_x86_64:
    runs-on: ubuntu-22.04
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY || secrets.FRAME_UPDATE_PUBLIC_KEY }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref }}
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
        curl -L --fail -o /tmp/appimagetool.AppImage https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
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
        name: frame-linux-x86_64.tar.gz
        path: target/release/frame-linux-x86_64.tar.gz
        if-no-files-found: error
    - name: release::upload_linux_x86_64_appimage
      uses: actions/upload-artifact@v4
      with:
        name: Frame-x86_64.AppImage
        path: target/release/Frame-x86_64.AppImage
        if-no-files-found: error
    - name: release::upload_linux_x86_64_appimage_zsync
      uses: actions/upload-artifact@v4
      with:
        name: Frame-x86_64.AppImage.zsync
        path: target/release/Frame-x86_64.AppImage.zsync
        if-no-files-found: error
    timeout-minutes: 90

  build_linux_aarch64:
    runs-on: ubuntu-22.04-arm
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY || secrets.FRAME_UPDATE_PUBLIC_KEY }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref }}
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
        curl -L --fail -o /tmp/appimagetool.AppImage https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-aarch64.AppImage
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
        name: frame-linux-aarch64.tar.gz
        path: target/release/frame-linux-aarch64.tar.gz
        if-no-files-found: error
    - name: release::upload_linux_aarch64_appimage
      uses: actions/upload-artifact@v4
      with:
        name: Frame-aarch64.AppImage
        path: target/release/Frame-aarch64.AppImage
        if-no-files-found: error
    - name: release::upload_linux_aarch64_appimage_zsync
      uses: actions/upload-artifact@v4
      with:
        name: Frame-aarch64.AppImage.zsync
        path: target/release/Frame-aarch64.AppImage.zsync
        if-no-files-found: error
    timeout-minutes: 90

  build_macos_x86_64:
    runs-on: macos-26-intel
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY || secrets.FRAME_UPDATE_PUBLIC_KEY }}
      MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
      MACOS_CERTIFICATES_P12: ${{ secrets.MACOS_CERTIFICATES_P12 }}
      MACOS_CERTIFICATES_PASSWORD: ${{ secrets.MACOS_CERTIFICATES_PASSWORD }}
      APPLE_NOTARIZATION_KEY: ${{ secrets.APPLE_NOTARIZATION_KEY }}
      APPLE_NOTARIZATION_KEY_ID: ${{ secrets.APPLE_NOTARIZATION_KEY_ID }}
      APPLE_NOTARIZATION_ISSUER_ID: ${{ secrets.APPLE_NOTARIZATION_ISSUER_ID }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref }}
    - name: release::check_public_key
      run: test -n "$FRAME_UPDATE_PUBLIC_KEY"
    - name: release::import_macos_signing_certificate
      if: env.MACOS_SIGNING_IDENTITY != '' && env.MACOS_CERTIFICATES_P12 != '' && env.MACOS_CERTIFICATES_PASSWORD != ''
      uses: Apple-Actions/import-codesign-certs@v3
      with:
        p12-file-base64: ${{ env.MACOS_CERTIFICATES_P12 }}
        p12-password: ${{ env.MACOS_CERTIFICATES_PASSWORD }}
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: steps::install_cargo_bundle
      run: cargo install cargo-bundle --locked
    - name: ./script/bundle-mac
      run: ./script/bundle-mac x86_64-apple-darwin
    - name: release::upload_macos_x86_64_dmg
      uses: actions/upload-artifact@v4
      with:
        name: Frame-x86_64.dmg
        path: target/x86_64-apple-darwin/release/Frame-x86_64.dmg
        if-no-files-found: error
    - name: release::upload_macos_x86_64_update
      uses: actions/upload-artifact@v4
      with:
        name: Frame-x86_64.app.zip
        path: target/x86_64-apple-darwin/release/Frame-x86_64.app.zip
        if-no-files-found: error
    timeout-minutes: 90

  build_macos_aarch64:
    runs-on: macos-26
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY || secrets.FRAME_UPDATE_PUBLIC_KEY }}
      MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
      MACOS_CERTIFICATES_P12: ${{ secrets.MACOS_CERTIFICATES_P12 }}
      MACOS_CERTIFICATES_PASSWORD: ${{ secrets.MACOS_CERTIFICATES_PASSWORD }}
      APPLE_NOTARIZATION_KEY: ${{ secrets.APPLE_NOTARIZATION_KEY }}
      APPLE_NOTARIZATION_KEY_ID: ${{ secrets.APPLE_NOTARIZATION_KEY_ID }}
      APPLE_NOTARIZATION_ISSUER_ID: ${{ secrets.APPLE_NOTARIZATION_ISSUER_ID }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref }}
    - name: release::check_public_key
      run: test -n "$FRAME_UPDATE_PUBLIC_KEY"
    - name: release::import_macos_signing_certificate
      if: env.MACOS_SIGNING_IDENTITY != '' && env.MACOS_CERTIFICATES_P12 != '' && env.MACOS_CERTIFICATES_PASSWORD != ''
      uses: Apple-Actions/import-codesign-certs@v3
      with:
        p12-file-base64: ${{ env.MACOS_CERTIFICATES_P12 }}
        p12-password: ${{ env.MACOS_CERTIFICATES_PASSWORD }}
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: steps::install_cargo_bundle
      run: cargo install cargo-bundle --locked
    - name: ./script/bundle-mac
      run: ./script/bundle-mac aarch64-apple-darwin
    - name: release::upload_macos_aarch64_dmg
      uses: actions/upload-artifact@v4
      with:
        name: Frame-aarch64.dmg
        path: target/aarch64-apple-darwin/release/Frame-aarch64.dmg
        if-no-files-found: error
    - name: release::upload_macos_aarch64_update
      uses: actions/upload-artifact@v4
      with:
        name: Frame-aarch64.app.zip
        path: target/aarch64-apple-darwin/release/Frame-aarch64.app.zip
        if-no-files-found: error
    timeout-minutes: 90

  build_windows_x86_64:
    runs-on: windows-2022
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY || secrets.FRAME_UPDATE_PUBLIC_KEY }}
      WINDOWS_SIGNTOOL: ${{ secrets.WINDOWS_SIGNTOOL }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref }}
    - name: release::check_public_key
      shell: pwsh
      run: |
        if (-not $env:FRAME_UPDATE_PUBLIC_KEY) { throw "FRAME_UPDATE_PUBLIC_KEY is required" }
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: steps::setup_inno
      shell: pwsh
      run: choco install innosetup --no-progress -y
    - name: ./script/bundle-windows.ps1
      shell: pwsh
      run: ./script/bundle-windows.ps1 -Architecture x86_64
    - name: release::upload_windows_x86_64
      uses: actions/upload-artifact@v4
      with:
        name: Frame-x86_64.exe
        path: target/Frame-x86_64.exe
        if-no-files-found: error
    timeout-minutes: 60

  publish_release:
    runs-on: ubuntu-22.04
    needs:
      - build_linux_x86_64
      - build_linux_aarch64
      - build_macos_x86_64
      - build_macos_aarch64
      - build_windows_x86_64
    env:
      FRAME_UPDATE_SIGNING_KEY: ${{ secrets.FRAME_UPDATE_SIGNING_KEY }}
      GH_TOKEN: ${{ github.token }}
      TAURI_BRIDGE_RELEASE_TAG: '0.29.3'
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v4
      with:
        ref: ${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref }}
    - name: release::check_signing_key
      run: test -n "$FRAME_UPDATE_SIGNING_KEY"
    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
    - name: release::download_artifacts
      uses: actions/download-artifact@v4
      with:
        path: target/release-artifacts
        merge-multiple: true
    - name: release::resolve_tag
      id: release
      shell: bash
      run: |
        tag="${GITHUB_REF_NAME}"
        if [[ "${GITHUB_EVENT_NAME}" == "workflow_dispatch" ]]; then
          tag="${{ inputs.tag }}"
        fi
        if [[ "$tag" == v* ]]; then
          echo "::error::Release tags must not use a v prefix: $tag" >&2
          exit 1
        fi
        if [[ ! "$tag" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
          echo "::error::Release tag must be semver without a v prefix: $tag" >&2
          exit 1
        fi
        version="$tag"
        echo "tag=$tag" >> "$GITHUB_OUTPUT"
        echo "version=$version" >> "$GITHUB_OUTPUT"
    - name: release::extract_release_notes
      shell: bash
      run: |
        mkdir -p target/release
        version="${{ steps.release.outputs.version }}"
        awk -v ver="[$version]" '/^## / { if (p) { exit }; if ($2 == ver) { p=1; next } } p' CHANGELOG.md > target/release/release-notes.md
        if [[ ! -s target/release/release-notes.md ]]; then
          echo "::error::CHANGELOG.md has no release notes section for [$version]" >&2
          exit 1
        fi
    - name: release::prepare_flathub_sources
      run: cargo xtask flathub-sources --version "${{ steps.release.outputs.version }}"
    - name: release::download_tauri_latest_json
      run: |
        mkdir -p target/release
        gh release download "$TAURI_BRIDGE_RELEASE_TAG" \
          --repo 66HEX/frame \
          --pattern latest.json \
          --dir target/release \
          --clobber
        test -s target/release/latest.json
    - name: release::generate_update_manifest
      run: |
        cargo xtask update-manifest \
          --version "${{ steps.release.outputs.version }}" \
          --release-tag "${{ steps.release.outputs.tag }}" \
          --artifact target/release-artifacts/Frame-aarch64.app.zip:macos-aarch64:macos_app_zip \
          --artifact target/release-artifacts/Frame-x86_64.app.zip:macos-x86_64:macos_app_zip \
          --artifact target/release-artifacts/Frame-x86_64.exe:windows-x86_64:windows_inno \
          --artifact target/release-artifacts/frame-linux-x86_64.tar.gz:linux-x86_64:linux_managed_tar \
          --artifact target/release-artifacts/frame-linux-aarch64.tar.gz:linux-aarch64:linux_managed_tar \
          --release-notes-markdown "$(< target/release/release-notes.md)" \
          --out target/release/update-manifest.json
    - name: release::sign_update_manifest
      run: |
        cargo xtask sign-update-manifest \
          --manifest target/release/update-manifest.json \
          --out target/release/update-manifest.json.sig
    - name: release::publish_github_release
      shell: bash
      run: |
        tag="${{ steps.release.outputs.tag }}"
        assets=(
          target/release-artifacts/Frame-aarch64.dmg
          target/release-artifacts/Frame-aarch64.app.zip
          target/release-artifacts/Frame-x86_64.dmg
          target/release-artifacts/Frame-x86_64.app.zip
          target/release-artifacts/Frame-x86_64.exe
          target/release-artifacts/frame-linux-x86_64.tar.gz
          target/release-artifacts/frame-linux-aarch64.tar.gz
          target/release-artifacts/Frame-x86_64.AppImage
          target/release-artifacts/Frame-x86_64.AppImage.zsync
          target/release-artifacts/Frame-aarch64.AppImage
          target/release-artifacts/Frame-aarch64.AppImage.zsync
          target/flathub/frame-${{ steps.release.outputs.version }}-source.tar.gz
          target/flathub/frame-${{ steps.release.outputs.version }}-cargo-vendor.tar.gz
          target/release/latest.json
          target/release/update-manifest.json
          target/release/update-manifest.json.sig
        )
        if gh release view "$tag" >/dev/null 2>&1; then
          gh release edit "$tag" --title "Frame ${{ steps.release.outputs.version }}" --notes-file target/release/release-notes.md
          gh release upload "$tag" "${assets[@]}" --clobber
        else
          gh release create "$tag" "${assets[@]}" --title "Frame ${{ steps.release.outputs.version }}" --notes-file target/release/release-notes.md
        fi
    timeout-minutes: 30

  update_homebrew_tap:
    runs-on: ubuntu-22.04
    needs: publish_release
    steps:
    - name: release::resolve_tag
      id: release
      shell: bash
      run: |
        tag="${GITHUB_REF_NAME}"
        if [[ "${GITHUB_EVENT_NAME}" == "workflow_dispatch" ]]; then
          tag="${{ inputs.tag }}"
        fi
        echo "tag=$tag" >> "$GITHUB_OUTPUT"
    - name: release::download_macos_dmgs
      id: hashes
      env:
        GH_TOKEN: ${{ github.token }}
        VERSION: ${{ steps.release.outputs.tag }}
      shell: bash
      run: |
        mkdir -p target/homebrew
        gh release download "$VERSION" --repo 66HEX/frame --pattern Frame-aarch64.dmg --dir target/homebrew --clobber
        gh release download "$VERSION" --repo 66HEX/frame --pattern Frame-x86_64.dmg --dir target/homebrew --clobber
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
        VERSION: ${{ steps.release.outputs.tag }}
        HASH_ARM: ${{ steps.hashes.outputs.HASH_ARM }}
        HASH_INTEL: ${{ steps.hashes.outputs.HASH_INTEL }}
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

          caveats <<~EOS
            Frame is not notarized. On first launch, you may need to:
            1. Right-click the app and select "Open".
            2. Click "Open" in the security dialog.

            Alternatively, you can run:
              xattr -dr com.apple.quarantine /Applications/Frame.app
          EOS
        end
        EOF

        git config user.name "github-actions[bot]"
        git config user.email "github-actions[bot]@users.noreply.github.com"
        git add Casks/frame.rb
        if git diff --staged --quiet; then
          echo "No Homebrew cask changes to publish."
        else
          git commit -m "Update Frame to $VERSION"
          git push
        fi
    timeout-minutes: 20

  update_flathub:
    runs-on: ubuntu-22.04
    needs: publish_release
    env:
      GH_TOKEN: ${{ github.token }}
      FLATHUB_GITHUB_TOKEN: ${{ secrets.FLATHUB_GITHUB_TOKEN }}
      FLATHUB_REPOSITORY: flathub/io.github._66HEX.Frame
    steps:
    - name: release::resolve_tag
      id: release
      shell: bash
      run: |
        tag="${GITHUB_REF_NAME}"
        if [[ "${GITHUB_EVENT_NAME}" == "workflow_dispatch" ]]; then
          tag="${{ inputs.tag }}"
        fi
        echo "tag=$tag" >> "$GITHUB_OUTPUT"
        echo "release_date=$(date -u +%F)" >> "$GITHUB_OUTPUT"
    - name: release::check_flathub_token
      shell: bash
      run: |
        if [[ -z "$FLATHUB_GITHUB_TOKEN" ]]; then
          echo "::notice::FLATHUB_GITHUB_TOKEN is not configured; skipping Flathub manifest update."
        fi
    - name: steps::checkout_repo
      if: env.FLATHUB_GITHUB_TOKEN != ''
      uses: actions/checkout@v4
      with:
        ref: ${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref }}
    - name: steps::setup_rust
      if: env.FLATHUB_GITHUB_TOKEN != ''
      uses: dtolnay/rust-toolchain@stable
    - name: release::download_flathub_sources
      if: env.FLATHUB_GITHUB_TOKEN != ''
      id: flathub_sources
      shell: bash
      run: |
        mkdir -p target/flathub-download
        tag="${{ steps.release.outputs.tag }}"
        gh release download "$tag" --repo 66HEX/frame --pattern "frame-$tag-source.tar.gz" --dir target/flathub-download --clobber
        gh release download "$tag" --repo 66HEX/frame --pattern "frame-$tag-cargo-vendor.tar.gz" --dir target/flathub-download --clobber
        source_archive="target/flathub-download/frame-$tag-source.tar.gz"
        vendor_archive="target/flathub-download/frame-$tag-cargo-vendor.tar.gz"
        source_sha256="$(sha256sum "$source_archive" | awk '{print $1}')"
        vendor_sha256="$(sha256sum "$vendor_archive" | awk '{print $1}')"
        echo "SOURCE_SHA256=$source_sha256" >> "$GITHUB_OUTPUT"
        echo "VENDOR_SHA256=$vendor_sha256" >> "$GITHUB_OUTPUT"
    - name: release::render_flathub_manifest
      if: env.FLATHUB_GITHUB_TOKEN != ''
      run: |
        tag="${{ steps.release.outputs.tag }}"
        cargo xtask flathub-manifest \
          --version "$tag" \
          --release-date "${{ steps.release.outputs.release_date }}" \
          --source-url "https://github.com/66HEX/frame/releases/download/$tag/frame-$tag-source.tar.gz" \
          --source-sha256 "${{ steps.flathub_sources.outputs.SOURCE_SHA256 }}" \
          --vendor-url "https://github.com/66HEX/frame/releases/download/$tag/frame-$tag-cargo-vendor.tar.gz" \
          --vendor-sha256 "${{ steps.flathub_sources.outputs.VENDOR_SHA256 }}" \
          --out target/flathub/repo
    - name: release::checkout_flathub
      if: env.FLATHUB_GITHUB_TOKEN != ''
      uses: actions/checkout@v4
      with:
        repository: ${{ env.FLATHUB_REPOSITORY }}
        token: ${{ env.FLATHUB_GITHUB_TOKEN }}
        path: flathub
    - name: release::publish_flathub_pr
      if: env.FLATHUB_GITHUB_TOKEN != ''
      working-directory: flathub
      env:
        GH_TOKEN: ${{ env.FLATHUB_GITHUB_TOKEN }}
        VERSION: ${{ steps.release.outputs.tag }}
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
    runs-on: windows-latest
    needs: publish_release
    steps:
    - name: release::resolve_tag
      id: release
      shell: bash
      run: |
        tag="${GITHUB_REF_NAME}"
        if [[ "${GITHUB_EVENT_NAME}" == "workflow_dispatch" ]]; then
          tag="${{ inputs.tag }}"
        fi
        echo "tag=$tag" >> "$GITHUB_OUTPUT"
    - name: release::publish_winget
      uses: vedantmgoyal9/winget-releaser@v2
      with:
        identifier: 66HEX.Frame
        version: ${{ steps.release.outputs.tag }}
        installers-regex: '^Frame-x86_64\.exe$'
        release-repository: frame
        release-tag: ${{ steps.release.outputs.tag }}
        release-notes-url: https://github.com/66HEX/frame/releases/tag/${{ steps.release.outputs.tag }}
        token: ${{ secrets.WINGET_ACC_TOKEN }}
    timeout-minutes: 30
"#
    .replace("  __NIXPKGS_JOBS__\n", &nixpkgs_release_jobs(Some("publish_release")))
}

fn nixpkgs_release_jobs(prepare_needs: Option<&str>) -> String {
    let prepare_needs = prepare_needs
        .map(|needs| format!("    needs: {needs}\n"))
        .unwrap_or_default();
    let mut jobs = r#"  prepare_nixpkgs:
    runs-on: ubuntu-22.04
    needs: publish_release
    outputs:
      branch: ${{ steps.nixpkgs.outputs.branch }}
      commit_subject: ${{ steps.nixpkgs.outputs.commit_subject }}
      should_publish: ${{ steps.nixpkgs.outputs.should_publish }}
    env:
      GH_TOKEN: ${{ secrets.NIXPKGS_GITHUB_TOKEN }}
      NIXPKGS_FORK: 66HEX/nixpkgs
      NIXPKGS_UPSTREAM: NixOS/nixpkgs
      NIXPKGS_PACKAGE: frame-media-converter
    steps:
    - name: release::resolve_tag
      id: release
      shell: bash
      run: |
        tag="${GITHUB_REF_NAME}"
        if [[ "${GITHUB_EVENT_NAME}" == "workflow_dispatch" ]]; then
          tag="${{ inputs.tag }}"
        fi
        echo "tag=$tag" >> "$GITHUB_OUTPUT"
    - name: release::check_nixpkgs_token
      shell: bash
      run: |
        if [[ -z "$GH_TOKEN" ]]; then
          echo "::notice::NIXPKGS_GITHUB_TOKEN is not configured; skipping nixpkgs PR."
        fi
    - name: steps::install_nix
      if: env.GH_TOKEN != ''
      uses: cachix/install-nix-action@v31
      with:
        extra_nix_config: |
          sandbox = true
    - name: release::checkout_nixpkgs
      if: env.GH_TOKEN != ''
      uses: actions/checkout@v4
      with:
        repository: ${{ env.NIXPKGS_FORK }}
        token: ${{ env.GH_TOKEN }}
        path: nixpkgs
        fetch-depth: 0
    - name: release::prepare_nixpkgs_branch
      id: nixpkgs
      if: env.GH_TOKEN != ''
      working-directory: nixpkgs
      env:
        VERSION: ${{ steps.release.outputs.tag }}
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
          nix-build -A "$NIXPKGS_PACKAGE" -L 2>&1 | tee "$log_file"
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

        nix-build -A "$NIXPKGS_PACKAGE" -L
        nix-shell -p nixfmt-rfc-style --run "nixfmt $package_path maintainers/maintainer-list.nix"
        nix-build -A "$NIXPKGS_PACKAGE" -L
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
        git push --force-with-lease origin "$branch"
        echo "commit_subject=$commit_subject" >> "$GITHUB_OUTPUT"
        echo "should_publish=true" >> "$GITHUB_OUTPUT"
    timeout-minutes: 90

  validate_nixpkgs_aarch64:
    runs-on: ubuntu-22.04-arm
    needs: prepare_nixpkgs
    if: needs.prepare_nixpkgs.outputs.should_publish == 'true'
    env:
      GH_TOKEN: ${{ secrets.NIXPKGS_GITHUB_TOKEN }}
      NIXPKGS_FORK: 66HEX/nixpkgs
      NIXPKGS_PACKAGE: frame-media-converter
    steps:
    - name: steps::install_nix
      uses: cachix/install-nix-action@v31
      with:
        extra_nix_config: |
          sandbox = true
    - name: release::checkout_nixpkgs
      uses: actions/checkout@v4
      with:
        repository: ${{ env.NIXPKGS_FORK }}
        ref: ${{ needs.prepare_nixpkgs.outputs.branch }}
        token: ${{ env.GH_TOKEN }}
        path: nixpkgs
    - name: release::build_nixpkgs_aarch64
      working-directory: nixpkgs
      run: nix-build -A "$NIXPKGS_PACKAGE" -L
    timeout-minutes: 90

  publish_nixpkgs:
    runs-on: ubuntu-22.04
    needs:
      - prepare_nixpkgs
      - validate_nixpkgs_aarch64
    if: needs.prepare_nixpkgs.outputs.should_publish == 'true'
    env:
      GH_TOKEN: ${{ secrets.NIXPKGS_GITHUB_TOKEN }}
      NIXPKGS_UPSTREAM: NixOS/nixpkgs
    steps:
    - name: release::publish_nixpkgs_pr
      env:
        BRANCH: ${{ needs.prepare_nixpkgs.outputs.branch }}
        COMMIT_SUBJECT: ${{ needs.prepare_nixpkgs.outputs.commit_subject }}
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
    jobs
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
"#
    .to_string();
    workflow.push_str(&nixpkgs_release_jobs(None));
    workflow
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
            "ffmpeg-n8.1.2-win64-gpl-8.1/bin/ffprobe.exe",
            &["ffprobe.exe"],
        ));
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
        assert!(workflow.contains("CHANGELOG.md > target/release/release-notes.md"));
    }

    #[test]
    fn release_workflow_uses_changelog_notes_for_update_manifest() {
        let workflow = release_workflow();

        assert!(
            workflow.contains("--release-notes-markdown \"$(< target/release/release-notes.md)\"")
        );
    }

    #[test]
    fn release_workflow_uses_changelog_notes_for_github_release() {
        let workflow = release_workflow();

        assert!(workflow.contains("--notes-file target/release/release-notes.md"));
        assert!(!workflow.contains("--generate-notes"));
    }

    #[test]
    fn release_workflow_publishes_appimage_update_assets() {
        let workflow = release_workflow();

        assert!(workflow.contains("target/release-artifacts/Frame-x86_64.AppImage"));
        assert!(workflow.contains("target/release-artifacts/Frame-x86_64.AppImage.zsync"));
        assert!(workflow.contains("target/release-artifacts/Frame-aarch64.AppImage"));
        assert!(workflow.contains("target/release-artifacts/Frame-aarch64.AppImage.zsync"));
    }

    #[test]
    fn release_workflow_publishes_flathub_source_archives() {
        let workflow = release_workflow();

        assert!(workflow.contains("release::prepare_flathub_sources"));
        assert!(
            workflow.contains(
                "target/flathub/frame-${{ steps.release.outputs.version }}-source.tar.gz"
            )
        );
        assert!(workflow.contains(
            "target/flathub/frame-${{ steps.release.outputs.version }}-cargo-vendor.tar.gz"
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
        assert!(workflow.contains("needs: publish_release"));
        assert!(workflow.contains("runs-on: ubuntu-22.04-arm"));
        assert!(workflow.contains("release::build_nixpkgs_aarch64"));
        assert!(workflow.contains("- validate_nixpkgs_aarch64"));
        assert!(workflow.contains("NIXPKGS_GITHUB_TOKEN"));
        assert!(workflow.contains("NIXPKGS_UPSTREAM: NixOS/nixpkgs"));
        assert!(workflow.contains("NIXPKGS_PACKAGE: frame-media-converter"));
        assert!(workflow.contains("Could not find alphabetical insertion point for _66HEX."));
        assert!(!workflow.contains("_0x4A6F"));
        assert!(workflow.contains("VERSION: ${{ steps.release.outputs.tag }}"));
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
        assert!(!workflow.contains("needs: publish_release"));
        assert!(!workflow.contains("publish_release:"));
        assert!(!workflow.contains("publish_winget:"));
        assert!(!workflow.contains("update_homebrew_tap:"));
        assert!(!workflow.contains("update_flathub:"));
        assert!(!workflow.contains("build_linux_x86_64:"));
        assert!(!workflow.contains("build_macos_aarch64:"));
    }

    #[test]
    fn flathub_template_uses_runtime_media_tools_without_bundled_binaries() {
        let template = include_str!("../../../packaging/flathub/io.github._66HEX.Frame.yml.in");

        assert!(template.contains("--env=FRAME_USE_SYSTEM_MEDIA_TOOLS=1"));
        assert!(!template.contains("resources/binaries"));
        assert!(!template.contains("frame-update-helper"));
        assert!(!template.contains("ffmpeg-full"));
        assert!(!template.contains("add-extensions"));
        assert!(!template.contains("--filesystem=home"));
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
