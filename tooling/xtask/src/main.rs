use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fmt, fs, io,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use frame_updater::{
    PlatformAssetKey, UpdateAsset, UpdateAssetKind, UpdateChannel, UpdateManifest, file_sha256_hex,
    sign_manifest_bytes,
};
use serde::{Deserialize, Serialize};

const RUN_BUNDLING_WORKFLOW_PATH: &str = ".github/workflows/run_bundling.yml";
const RELEASE_WORKFLOW_PATH: &str = ".github/workflows/release.yml";
const MARTIN_FFMPEG_BASE_URL: &str = "https://ffmpeg.martin-riedl.de/redirect/latest";
const WINDOWS_FFMPEG_ZIP_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";
const DEFAULT_GSTREAMER_VERSION: &str = "1.28.2";
const DEFAULT_GSTREAMER_DOWNLOAD_BASE_URL: &str =
    "https://github.com/66HEX/frame-gstreamer-mirror/releases/download";
const MACOS_GSTREAMER_BUNDLED_RPATH: &str =
    "@executable_path/../Frameworks/GStreamer.framework/Versions/Current/lib";
const OPTIONAL_MACOS_GSTREAMER_PLUGINS: &[&[&str]] = &[&[
    "Versions",
    "Current",
    "lib",
    "gstreamer-1.0",
    "libgstpython.dylib",
]];
const MACOS_PRUNED_RUNTIME_DIRS: &[&[&str]] = &[
    &["Headers"],
    &["Commands"],
    &["Versions", "Current", "bin"],
    &["Versions", "Current", "Commands"],
    &["Versions", "Current", "include"],
    &["Versions", "Current", "Headers"],
    &["Versions", "Current", "lib", "cmake"],
    &["Versions", "Current", "lib", "pkgconfig"],
    &["Versions", "Current", "lib", "gstreamer-1.0", "pkgconfig"],
    &["Versions", "Current", "share", "aclocal"],
    &["Versions", "Current", "share", "cmake"],
    &["Versions", "Current", "share", "gir-1.0"],
    &["Versions", "Current", "share", "gobject-introspection-1.0"],
    &["Versions", "Current", "share", "gstreamer-1.0", "validate"],
];
const MACOS_PRUNED_RUNTIME_DIR_NAMES: &[&str] = &["Headers", "include", "pkgconfig", "cmake"];
const MACOS_PRUNED_RUNTIME_EXTENSIONS: &[&str] = &["a", "h", "pc", "gir", "typelib"];
const MACOS_PRUNED_RUNTIME_FILE_NAMES: &[&str] = &[".gitignore"];
const OPTIONAL_LINUX_GSTREAMER_PLUGINS: &[&str] =
    &["libgstpython.so", "libgstxvimagesink.so", "libgstva.so"];

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
        "bundle" => bundle(args.next().as_deref()),
        "ci" => ci(),
        "run" => run_frame_app(args.collect()),
        "setup-ffmpeg" => {
            let args = args.collect::<Vec<_>>();
            setup_ffmpeg(&args)
        }
        "setup-gstreamer" => {
            let args = args.collect::<Vec<_>>();
            setup_gstreamer(&args)
        }
        "stage-gstreamer" => {
            let args = args.collect::<Vec<_>>();
            stage_gstreamer(&args)
        }
        "update-manifest" => {
            let args = args.collect::<Vec<_>>();
            update_manifest(&args)
        }
        "sign-update-manifest" => {
            let args = args.collect::<Vec<_>>();
            sign_update_manifest(&args)
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
  run               Run the native Frame app with the controlled GStreamer env
  build             Build frame-app with the controlled GStreamer env
  bundle macos      Build the macOS .app and .dmg package
  bundle linux      Build the Linux tarball package
  bundle windows    Build the Windows Inno Setup installer
  setup-ffmpeg      Download FFmpeg and FFprobe runtime binaries
  setup-gstreamer   Download/configure the controlled GStreamer runtime
  stage-gstreamer   Copy the controlled GStreamer runtime into a native bundle
  update-manifest   Generate a signed-update manifest from release artifacts
  sign-update-manifest Sign update-manifest.json with FRAME_UPDATE_SIGNING_KEY
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
    run_frame_app_cargo_command("dev", &cargo_args)
}

fn build_frame_app(args: Vec<String>) -> Result<()> {
    let mut cargo_args = vec![
        "build".to_string(),
        "--manifest-path".to_string(),
        "frame-app/Cargo.toml".to_string(),
    ];
    cargo_args.extend(args);
    run_frame_app_cargo_command("dev", &cargo_args)
}

fn run_frame_app_cargo_command(mode: &str, cargo_args: &[String]) -> Result<()> {
    let manifest = prepare_host_gstreamer_manifest(mode)?;
    let env = gstreamer_command_env(&manifest)?;
    run_command_path_with_env("cargo", cargo_args, &env)
}

fn bundle(platform: Option<&str>) -> Result<()> {
    match platform {
        Some("macos" | "mac" | "darwin") => run_script("./script/bundle-mac", &[]),
        Some("linux") => run_script("./script/bundle-linux", &[]),
        Some("windows" | "win") => run_script("./script/bundle-windows.ps1", &[]),
        Some(other) => Err(XtaskError::Usage(format!(
            "unknown bundle platform `{other}`"
        ))),
        None => Err(XtaskError::Usage(
            "missing bundle platform: expected macos, linux, or windows".to_string(),
        )),
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
    println!("Detected {}. Preparing FFmpeg binaries...", target.label());

    match target {
        FfmpegTarget::Individual { binaries, .. } => {
            for entry in binaries {
                process_ffmpeg_entry(&entry, &binary_dir, options.force)?;
            }
        }
        FfmpegTarget::SharedArchive { url, entries, .. } => {
            process_ffmpeg_shared_archive(&url, &entries, &binary_dir, options.force)?;
        }
    }

    println!("All binaries are ready in frame-app/resources/binaries.");
    Ok(())
}

fn setup_gstreamer(args: &[String]) -> Result<()> {
    let options = SetupGstreamerOptions::parse(args)?;
    let platform = GstreamerPlatform::parse(
        options
            .platform
            .as_deref()
            .unwrap_or_else(|| host_platform()),
    )?;
    let arch = normalize_arch(options.arch.as_deref().unwrap_or(host_arch()))?;
    let version = options
        .version
        .as_deref()
        .unwrap_or(DEFAULT_GSTREAMER_VERSION);
    let download_dir = options
        .download_dir
        .clone()
        .unwrap_or_else(default_gstreamer_download_dir);

    if options.install {
        install_gstreamer_packages(platform, &arch, version, &download_dir, &options)?;
    }

    let manifest = create_gstreamer_manifest(platform, &arch, version, &download_dir, &options)?;
    write_gstreamer_manifest(&manifest)?;

    if options.print_env {
        print_gstreamer_env(&manifest)?;
    } else {
        println!(
            "Wrote GStreamer manifest: {}",
            gstreamer_manifest_path()?.display()
        );
        println!(
            "GStreamer {} ({}/{})",
            manifest.version, manifest.platform, manifest.arch
        );
    }

    Ok(())
}

fn prepare_host_gstreamer_manifest(mode: &str) -> Result<GstreamerManifest> {
    if let Ok(path) = env::var("FRAME_GSTREAMER_MANIFEST") {
        let manifest_path = PathBuf::from(path);
        if manifest_path.exists() {
            let manifest = read_gstreamer_manifest(&manifest_path)?;
            write_gstreamer_manifest(&manifest)?;
            return Ok(manifest);
        }
    }

    let platform = GstreamerPlatform::parse(host_platform())?;
    let arch = normalize_arch(host_arch())?;
    let version = DEFAULT_GSTREAMER_VERSION;
    let download_dir = default_gstreamer_download_dir();
    let options = SetupGstreamerOptions {
        install: false,
        print_env: false,
        force: false,
        platform: Some(host_platform().to_string()),
        arch: Some(host_arch().to_string()),
        mode: mode.to_string(),
        source: None,
        version: Some(version.to_string()),
        download_dir: Some(download_dir.clone()),
        download_base_url: env::var("FRAME_GSTREAMER_DOWNLOAD_BASE_URL").ok(),
    };

    let manifest =
        match create_gstreamer_manifest(platform, &arch, version, &download_dir, &options) {
            Ok(manifest) => manifest,
            Err(error) => {
                eprintln!("GStreamer runtime is not prepared for {mode}: {error}");
                eprintln!("Preparing controlled GStreamer runtime from the Frame mirror...");
                install_gstreamer_packages(platform, &arch, version, &download_dir, &options)?;
                create_gstreamer_manifest(platform, &arch, version, &download_dir, &options)?
            }
        };

    write_gstreamer_manifest(&manifest)?;
    Ok(manifest)
}

fn stage_gstreamer(args: &[String]) -> Result<()> {
    let options = StageGstreamerOptions::parse(args)?;
    let manifest_path = options
        .manifest
        .clone()
        .or_else(|| env::var("FRAME_GSTREAMER_MANIFEST").ok().map(PathBuf::from))
        .unwrap_or(gstreamer_manifest_path()?);
    let manifest = read_gstreamer_manifest(&manifest_path)?;

    match manifest.platform.as_str() {
        "macos" => {
            let Some(app_path) = options.app.as_deref() else {
                return Err(XtaskError::Usage(
                    "missing --app <path> for macOS GStreamer staging".to_string(),
                ));
            };
            stage_macos_gstreamer_framework(app_path, &manifest)?;
        }
        "windows" => {
            let Some(binary_dir) = options.dir.as_deref() else {
                return Err(XtaskError::Usage(
                    "missing --dir <path> for Windows GStreamer staging".to_string(),
                ));
            };
            stage_windows_gstreamer_runtime(binary_dir, &manifest)?;
        }
        "linux" => {
            let Some(binary_dir) = options.dir.as_deref() else {
                return Err(XtaskError::Usage(
                    "missing --dir <path> for Linux GStreamer staging".to_string(),
                ));
            };
            stage_linux_gstreamer_runtime(binary_dir, &manifest, options.resources.as_deref())?;
        }
        other => {
            return Err(XtaskError::Usage(format!(
                "GStreamer staging is not implemented for `{other}`"
            )));
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct SetupFfmpegOptions {
    force: bool,
    platform: Option<String>,
    arch: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct SetupGstreamerOptions {
    install: bool,
    print_env: bool,
    force: bool,
    platform: Option<String>,
    arch: Option<String>,
    mode: String,
    source: Option<PathBuf>,
    version: Option<String>,
    download_dir: Option<PathBuf>,
    download_base_url: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct StageGstreamerOptions {
    app: Option<PathBuf>,
    dir: Option<PathBuf>,
    resources: Option<PathBuf>,
    manifest: Option<PathBuf>,
}

#[derive(Clone, Debug, Default)]
struct CommandEnv {
    values: Vec<(String, OsString)>,
}

impl CommandEnv {
    fn set(&mut self, key: impl Into<String>, value: impl Into<OsString>) {
        self.values.push((key.into(), value.into()));
    }

    fn apply(&self, command: &mut Command) {
        for (key, value) in &self.values {
            command.env(key, value);
        }
    }
}

impl StageGstreamerOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut options = Self {
            app: None,
            dir: None,
            resources: None,
            manifest: None,
        };
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--app" => {
                    options.app = Some(PathBuf::from(required_option_value(
                        args, &mut index, "--app",
                    )?));
                }
                "--dir" => {
                    options.dir = Some(PathBuf::from(required_option_value(
                        args, &mut index, "--dir",
                    )?));
                }
                "--resources" => {
                    options.resources = Some(PathBuf::from(required_option_value(
                        args,
                        &mut index,
                        "--resources",
                    )?));
                }
                "--manifest" => {
                    options.manifest = Some(PathBuf::from(required_option_value(
                        args,
                        &mut index,
                        "--manifest",
                    )?));
                }
                "-h" | "--help" => {
                    println!(
                        "\
Usage: cargo xtask stage-gstreamer [options]

Options:
  --app <path>       macOS .app bundle path
  --dir <path>       Directory containing the app executable
  --resources <path> Optional resource directory for Linux relocation manifests
  --manifest <path>  Override GStreamer manifest path
"
                    );
                    return Err(XtaskError::Help);
                }
                other => {
                    return Err(XtaskError::Usage(format!(
                        "unknown stage-gstreamer option `{other}`"
                    )));
                }
            }
        }

        Ok(options)
    }
}

impl SetupGstreamerOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut options = Self {
            install: false,
            print_env: false,
            force: false,
            platform: None,
            arch: None,
            mode: "dev".to_string(),
            source: None,
            version: None,
            download_dir: None,
            download_base_url: env::var("FRAME_GSTREAMER_DOWNLOAD_BASE_URL").ok(),
        };
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--install" => {
                    options.install = true;
                    index += 1;
                }
                "--print-env" => {
                    options.print_env = true;
                    index += 1;
                }
                "--force" => {
                    options.force = true;
                    index += 1;
                }
                "--platform" => {
                    options.platform = Some(required_option_value(args, &mut index, "--platform")?);
                }
                "--arch" => {
                    options.arch = Some(required_option_value(args, &mut index, "--arch")?);
                }
                "--mode" => {
                    options.mode = required_option_value(args, &mut index, "--mode")?;
                }
                "--source" => {
                    options.source = Some(PathBuf::from(required_option_value(
                        args, &mut index, "--source",
                    )?));
                }
                "--version" => {
                    options.version = Some(required_option_value(args, &mut index, "--version")?);
                }
                "--download-dir" => {
                    options.download_dir = Some(PathBuf::from(required_option_value(
                        args,
                        &mut index,
                        "--download-dir",
                    )?));
                }
                "--download-base-url" => {
                    options.download_base_url = Some(required_option_value(
                        args,
                        &mut index,
                        "--download-base-url",
                    )?);
                }
                "-h" | "--help" => {
                    println!(
                        "\
Usage: cargo xtask setup-gstreamer [options]

Options:
  --install                  Download/install the controlled GStreamer runtime when needed
  --print-env                Print shell commands for the build environment
  --force                    Re-download/reinstall even when cached files exist
  --platform <name>          Override platform: darwin, linux, or win32
  --arch <name>              Override architecture: x64, x86_64, arm64, or aarch64
  --mode <name>              Manifest mode: dev, ci, or bundle
  --source <path>            Use an existing GStreamer root/framework
  --version <version>        Override GStreamer version
  --download-dir <path>      Override download/cache directory
  --download-base-url <url>  Override mirror base URL
"
                    );
                    return Err(XtaskError::Help);
                }
                other => {
                    return Err(XtaskError::Usage(format!(
                        "unknown setup-gstreamer option `{other}`"
                    )));
                }
            }
        }

        Ok(options)
    }
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
    url: Option<String>,
    expected_names: &'static [&'static str],
    destination_name: String,
    make_executable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FfmpegTarget {
    Individual {
        label: &'static str,
        binaries: Vec<FfmpegBinaryEntry>,
    },
    SharedArchive {
        label: &'static str,
        url: String,
        entries: Vec<FfmpegBinaryEntry>,
    },
}

impl FfmpegTarget {
    const fn label(&self) -> &'static str {
        match self {
            Self::Individual { label, .. } | Self::SharedArchive { label, .. } => label,
        }
    }
}

fn ffmpeg_target_for(platform: &str, arch: &str) -> Result<FfmpegTarget> {
    match (platform, arch) {
        ("darwin", "x64" | "x86_64") => Ok(martin_ffmpeg_target(
            "macOS (Intel)",
            "macos",
            "amd64",
            "x86_64",
            "apple-darwin",
        )),
        ("darwin", "arm64" | "aarch64") => Ok(martin_ffmpeg_target(
            "macOS (Apple Silicon)",
            "macos",
            "arm64",
            "aarch64",
            "apple-darwin",
        )),
        ("linux", "x64" | "x86_64" | "amd64") => Ok(martin_ffmpeg_target(
            "Linux x86_64",
            "linux",
            "amd64",
            "x86_64",
            "unknown-linux-gnu",
        )),
        ("linux", "arm64" | "aarch64") => Ok(martin_ffmpeg_target(
            "Linux ARM64",
            "linux",
            "arm64",
            "aarch64",
            "unknown-linux-gnu",
        )),
        ("win32" | "windows", "x64" | "x86_64") => Ok(windows_ffmpeg_target()),
        _ => Err(XtaskError::Usage(format!(
            "unsupported platform or architecture: {platform}/{arch}"
        ))),
    }
}

fn martin_ffmpeg_target(
    label: &'static str,
    os_segment: &str,
    download_segment: &str,
    arch_label: &str,
    suffix: &str,
) -> FfmpegTarget {
    FfmpegTarget::Individual {
        label,
        binaries: vec![
            FfmpegBinaryEntry {
                id: "ffmpeg",
                url: Some(format!(
                    "{MARTIN_FFMPEG_BASE_URL}/{os_segment}/{download_segment}/release/ffmpeg.zip"
                )),
                expected_names: &["ffmpeg"],
                destination_name: format!("ffmpeg-{arch_label}-{suffix}"),
                make_executable: true,
            },
            FfmpegBinaryEntry {
                id: "ffprobe",
                url: Some(format!(
                    "{MARTIN_FFMPEG_BASE_URL}/{os_segment}/{download_segment}/release/ffprobe.zip"
                )),
                expected_names: &["ffprobe"],
                destination_name: format!("ffprobe-{arch_label}-{suffix}"),
                make_executable: true,
            },
        ],
    }
}

fn windows_ffmpeg_target() -> FfmpegTarget {
    FfmpegTarget::SharedArchive {
        label: "Windows x86_64",
        url: WINDOWS_FFMPEG_ZIP_URL.to_string(),
        entries: vec![
            FfmpegBinaryEntry {
                id: "ffmpeg",
                url: None,
                expected_names: &["ffmpeg.exe"],
                destination_name: "ffmpeg-x86_64-pc-windows-msvc.exe".to_string(),
                make_executable: false,
            },
            FfmpegBinaryEntry {
                id: "ffprobe",
                url: None,
                expected_names: &["ffprobe.exe"],
                destination_name: "ffprobe-x86_64-pc-windows-msvc.exe".to_string(),
                make_executable: false,
            },
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GstreamerPlatform {
    Macos,
    Windows,
    Linux,
}

impl GstreamerPlatform {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "darwin" | "macos" => Ok(Self::Macos),
            "win32" | "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            other => Err(XtaskError::Usage(format!(
                "unsupported GStreamer platform `{other}`"
            ))),
        }
    }

    const fn manifest_name(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GstreamerManifest {
    version: String,
    platform: String,
    arch: String,
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    framework_path: Option<PathBuf>,
    root: PathBuf,
    bin_dir: PathBuf,
    lib_dir: PathBuf,
    pkg_config_dir: PathBuf,
    plugin_dir: PathBuf,
    scanner_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    lib_triplet: Option<String>,
}

fn install_gstreamer_packages(
    platform: GstreamerPlatform,
    arch: &str,
    version: &str,
    download_dir: &Path,
    options: &SetupGstreamerOptions,
) -> Result<()> {
    match platform {
        GstreamerPlatform::Macos => {
            install_macos_gstreamer_packages(version, download_dir, options)
        }
        GstreamerPlatform::Windows => {
            install_windows_gstreamer_package(version, arch, download_dir, options)
        }
        GstreamerPlatform::Linux => {
            install_linux_gstreamer_packages(version, arch, download_dir, options)
        }
    }
}

fn install_macos_gstreamer_packages(
    version: &str,
    download_dir: &Path,
    options: &SetupGstreamerOptions,
) -> Result<()> {
    fs::create_dir_all(download_dir)?;
    for (name, label) in [
        (
            format!("gstreamer-1.0-{version}-universal.pkg"),
            "GStreamer runtime",
        ),
        (
            format!("gstreamer-1.0-devel-{version}-universal.pkg"),
            "GStreamer development",
        ),
    ] {
        let package_path = download_dir.join(&name);
        download_gstreamer_package(&name, version, &package_path, options)?;
        eprintln!("Installing {label}: {}", package_path.display());
        run_command_path(
            "sudo",
            &[
                "installer".to_string(),
                "-pkg".to_string(),
                package_path.display().to_string(),
                "-target".to_string(),
                "/".to_string(),
            ],
        )?;
    }
    Ok(())
}

fn install_windows_gstreamer_package(
    version: &str,
    arch: &str,
    download_dir: &Path,
    options: &SetupGstreamerOptions,
) -> Result<()> {
    if arch != "x86_64" {
        return Err(XtaskError::Usage(format!(
            "Windows GStreamer setup currently supports x86_64, received {arch}"
        )));
    }

    let root = default_windows_gstreamer_root(download_dir, arch, version);
    if !options.force && windows_gstreamer_manifest(&root, arch, &options.mode).is_ok() {
        eprintln!("Using cached GStreamer runtime: {}", root.display());
        return Ok(());
    }

    fs::remove_dir_all(&root).ok();
    fs::create_dir_all(&root)?;
    let package_dir = download_dir.join(format!("windows-{arch}-{version}"));
    fs::create_dir_all(&package_dir)?;
    let name = format!("gstreamer-1.0-msvc-{arch}-{version}.exe");
    let package_path = package_dir.join(&name);
    download_gstreamer_package(&name, version, &package_path, options)?;
    eprintln!(
        "Installing GStreamer runtime+development: {}",
        package_path.display()
    );
    run_command_path(
        package_path.as_os_str(),
        &[
            "/VERYSILENT".to_string(),
            "/SUPPRESSMSGBOXES".to_string(),
            "/NORESTART".to_string(),
            "/SP-".to_string(),
            format!("/DIR={}", root.display()),
        ],
    )
}

fn install_linux_gstreamer_packages(
    version: &str,
    arch: &str,
    download_dir: &Path,
    options: &SetupGstreamerOptions,
) -> Result<()> {
    let package_arch = linux_gstreamer_package_arch(arch)?;
    let root = default_linux_gstreamer_root(download_dir, arch, version);
    if !options.force && linux_gstreamer_manifest(&root, arch, &options.mode).is_ok() {
        eprintln!("Using cached GStreamer runtime: {}", root.display());
        return Ok(());
    }

    fs::remove_dir_all(&root).ok();
    fs::create_dir_all(&root)?;
    let package_dir = download_dir.join(format!("linux-{arch}-{version}"));
    fs::create_dir_all(&package_dir)?;

    for (name, label) in [
        (
            format!("gstreamer-1.0-linux-{package_arch}-{version}.tar.xz"),
            "GStreamer runtime",
        ),
        (
            format!("gstreamer-1.0-linux-{package_arch}-{version}-devel.tar.xz"),
            "GStreamer development",
        ),
    ] {
        let package_path = package_dir.join(&name);
        download_gstreamer_package(&name, version, &package_path, options)?;
        eprintln!("Extracting {label}: {}", package_path.display());
        run_command_path(
            "tar",
            &[
                "-xJf".to_string(),
                package_path.display().to_string(),
                "-C".to_string(),
                root.display().to_string(),
            ],
        )?;
    }

    Ok(())
}

fn create_gstreamer_manifest(
    platform: GstreamerPlatform,
    arch: &str,
    version: &str,
    download_dir: &Path,
    options: &SetupGstreamerOptions,
) -> Result<GstreamerManifest> {
    match platform {
        GstreamerPlatform::Macos => {
            let root = options.source.clone().unwrap_or_else(|| {
                PathBuf::from("/Library/Frameworks/GStreamer.framework/Versions/1.0")
            });
            macos_gstreamer_manifest(&root, arch, &options.mode)
        }
        GstreamerPlatform::Windows => {
            let root = options
                .source
                .clone()
                .unwrap_or_else(|| default_windows_gstreamer_root(download_dir, arch, version));
            windows_gstreamer_manifest(&root, arch, &options.mode)
        }
        GstreamerPlatform::Linux => {
            let root = options
                .source
                .clone()
                .unwrap_or_else(|| default_linux_gstreamer_root(download_dir, arch, version));
            linux_gstreamer_manifest(&root, arch, &options.mode)
        }
    }
}

fn macos_gstreamer_manifest(root: &Path, arch: &str, mode: &str) -> Result<GstreamerManifest> {
    let framework_path = root
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| {
            XtaskError::Usage(format!(
                "invalid macOS GStreamer framework root `{}`",
                root.display()
            ))
        })?
        .to_path_buf();
    let bin_dir = root.join("bin");
    let lib_dir = root.join("lib");
    let pkg_config_dir = lib_dir.join("pkgconfig");
    let plugin_dir = lib_dir.join("gstreamer-1.0");
    let scanner_path = root
        .join("libexec")
        .join("gstreamer-1.0")
        .join("gst-plugin-scanner");

    require_gstreamer_manifest_paths(&[
        (&framework_path, "GStreamer framework bundle", true),
        (root, "GStreamer framework root", true),
        (&bin_dir, "GStreamer bin directory", true),
        (&lib_dir, "GStreamer lib directory", true),
        (&pkg_config_dir, "GStreamer pkg-config directory", true),
        (&plugin_dir, "GStreamer plugin directory", true),
        (&scanner_path, "GStreamer plugin scanner", false),
    ])?;
    require_gstreamer_pc_files(&pkg_config_dir)?;

    Ok(GstreamerManifest {
        version: detect_gstreamer_version(&pkg_config_dir)?,
        platform: GstreamerPlatform::Macos.manifest_name().to_string(),
        arch: arch.to_string(),
        mode: mode.to_string(),
        framework_path: Some(framework_path),
        root: root.to_path_buf(),
        bin_dir,
        lib_dir,
        pkg_config_dir,
        plugin_dir,
        scanner_path,
        lib_triplet: None,
    })
}

fn windows_gstreamer_manifest(root: &Path, arch: &str, mode: &str) -> Result<GstreamerManifest> {
    let bin_dir = root.join("bin");
    let lib_dir = root.join("lib");
    let pkg_config_dir = lib_dir.join("pkgconfig");
    let plugin_dir = lib_dir.join("gstreamer-1.0");
    let scanner_path = root
        .join("libexec")
        .join("gstreamer-1.0")
        .join("gst-plugin-scanner.exe");

    require_gstreamer_manifest_paths(&[
        (root, "GStreamer root directory", true),
        (&bin_dir, "GStreamer bin directory", true),
        (&lib_dir, "GStreamer lib directory", true),
        (&pkg_config_dir, "GStreamer pkg-config directory", true),
        (&plugin_dir, "GStreamer plugin directory", true),
        (&scanner_path, "GStreamer plugin scanner", false),
    ])?;
    require_gstreamer_pc_files(&pkg_config_dir)?;

    Ok(GstreamerManifest {
        version: detect_gstreamer_version(&pkg_config_dir)?,
        platform: GstreamerPlatform::Windows.manifest_name().to_string(),
        arch: arch.to_string(),
        mode: mode.to_string(),
        framework_path: None,
        root: root.to_path_buf(),
        bin_dir,
        lib_dir,
        pkg_config_dir,
        plugin_dir,
        scanner_path,
        lib_triplet: None,
    })
}

fn linux_gstreamer_manifest(root: &Path, arch: &str, mode: &str) -> Result<GstreamerManifest> {
    let lib_triplet = linux_gstreamer_lib_triplet(arch)?;
    let bin_dir = root.join("bin");
    let lib_dir = root.join("lib").join(lib_triplet);
    let source_pkg_config_dir = lib_dir.join("pkgconfig");
    let pkg_config_dir = prepare_linux_gstreamer_pkg_config_dir(&source_pkg_config_dir)?;
    let plugin_dir = lib_dir.join("gstreamer-1.0");
    let scanner_path = root
        .join("libexec")
        .join("gstreamer-1.0")
        .join("gst-plugin-scanner");

    require_gstreamer_manifest_paths(&[
        (root, "GStreamer root directory", true),
        (&bin_dir, "GStreamer bin directory", true),
        (&lib_dir, "GStreamer lib directory", true),
        (
            &source_pkg_config_dir,
            "GStreamer source pkg-config directory",
            true,
        ),
        (
            &pkg_config_dir,
            "Frame GStreamer pkg-config directory",
            true,
        ),
        (&plugin_dir, "GStreamer plugin directory", true),
        (&scanner_path, "GStreamer plugin scanner", false),
    ])?;
    require_gstreamer_pc_files(&pkg_config_dir)?;

    Ok(GstreamerManifest {
        version: detect_gstreamer_version(&pkg_config_dir)?,
        platform: GstreamerPlatform::Linux.manifest_name().to_string(),
        arch: arch.to_string(),
        mode: mode.to_string(),
        framework_path: None,
        root: root.to_path_buf(),
        bin_dir,
        lib_dir,
        pkg_config_dir,
        plugin_dir,
        scanner_path,
        lib_triplet: Some(lib_triplet.to_string()),
    })
}

fn download_gstreamer_package(
    name: &str,
    version: &str,
    destination: &Path,
    options: &SetupGstreamerOptions,
) -> Result<()> {
    if !options.force && destination.is_file() {
        eprintln!("Using cached GStreamer package: {}", destination.display());
        return Ok(());
    }

    let base_url = options
        .download_base_url
        .as_deref()
        .unwrap_or(DEFAULT_GSTREAMER_DOWNLOAD_BASE_URL)
        .trim_end_matches('/');
    let url = format!("{base_url}/{version}/{name}");
    eprintln!("Downloading GStreamer package: {url}");
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    download_file_to_path(&url, destination)
}

fn download_file_to_path(url: &str, destination: &Path) -> Result<()> {
    let response = ureq::get(url)
        .call()
        .map_err(|source| XtaskError::Download {
            url: url.to_string(),
            source: Box::new(source),
        })?;
    let temporary_destination = destination.with_extension("download");
    {
        let mut file = fs::File::create(&temporary_destination)?;
        io::copy(&mut response.into_reader(), &mut file)?;
    }
    fs::rename(temporary_destination, destination)?;
    Ok(())
}

fn write_gstreamer_manifest(manifest: &GstreamerManifest) -> Result<()> {
    let manifest_path = gstreamer_manifest_path()?;
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(manifest_path, format!("{json}\n"))?;
    Ok(())
}

fn read_gstreamer_manifest(path: &Path) -> Result<GstreamerManifest> {
    let content = fs::read_to_string(path).map_err(|source| {
        XtaskError::Usage(format!(
            "failed to read GStreamer manifest at {}: {source}",
            path.display()
        ))
    })?;
    serde_json::from_str(&content).map_err(Into::into)
}

fn stage_macos_gstreamer_framework(app_path: &Path, manifest: &GstreamerManifest) -> Result<()> {
    let source_framework = manifest
        .framework_path
        .clone()
        .unwrap_or_else(|| manifest.root.join("..").join(".."));
    let frameworks_dir = app_path.join("Contents").join("Frameworks");
    let destination_framework = frameworks_dir.join("GStreamer.framework");

    require_gstreamer_manifest_paths(&[
        (app_path, "macOS app bundle", true),
        (&source_framework, "GStreamer framework source", true),
    ])?;
    fs::create_dir_all(&frameworks_dir)?;
    remove_path_if_exists(&destination_framework)?;
    run_command_path(
        "ditto",
        &[
            source_framework.display().to_string(),
            destination_framework.display().to_string(),
        ],
    )?;

    let removed_plugins = prune_macos_optional_plugins(&destination_framework)?;
    let pruned_paths = prune_macos_runtime_payload(&destination_framework)?;
    let executable_path = macos_app_executable(app_path)?;
    ensure_macos_bundled_rpath(&executable_path, manifest)?;
    run_command_path(
        "codesign",
        &[
            "--force".to_string(),
            "--deep".to_string(),
            "--sign".to_string(),
            "-".to_string(),
            app_path.display().to_string(),
        ],
    )?;

    println!(
        "Staged GStreamer framework: {}",
        destination_framework.display()
    );
    for plugin_path in removed_plugins {
        println!(
            "Removed optional GStreamer plugin: {}",
            plugin_path.display()
        );
    }
    println!("Pruned macOS GStreamer runtime paths: {pruned_paths}");
    println!("Patched executable rpath: {}", executable_path.display());
    Ok(())
}

fn macos_app_executable(app_path: &Path) -> Result<PathBuf> {
    let plist_path = app_path.join("Contents").join("Info.plist");
    let executable_name = run_command_capture_path(
        "/usr/libexec/PlistBuddy",
        &[
            "-c".to_string(),
            "Print:CFBundleExecutable".to_string(),
            plist_path.display().to_string(),
        ],
    )?
    .trim()
    .to_string();

    if executable_name.is_empty() {
        return Err(XtaskError::Usage(format!(
            "CFBundleExecutable not found in {}",
            plist_path.display()
        )));
    }

    Ok(app_path
        .join("Contents")
        .join("MacOS")
        .join(executable_name))
}

fn ensure_macos_bundled_rpath(binary_path: &Path, manifest: &GstreamerManifest) -> Result<()> {
    let rpaths = macos_binary_rpaths(binary_path)?;
    if !rpaths
        .iter()
        .any(|rpath| rpath == MACOS_GSTREAMER_BUNDLED_RPATH)
    {
        run_command_path(
            "install_name_tool",
            &[
                "-add_rpath".to_string(),
                MACOS_GSTREAMER_BUNDLED_RPATH.to_string(),
                binary_path.display().to_string(),
            ],
        )?;
    }

    let removable_rpaths = [
        manifest.lib_dir.display().to_string(),
        "/Library/Frameworks/GStreamer.framework/Versions/1.0/lib".to_string(),
    ];
    for rpath in macos_binary_rpaths(binary_path)? {
        if !removable_rpaths.iter().any(|removable| removable == &rpath) {
            continue;
        }
        run_command_path(
            "install_name_tool",
            &[
                "-delete_rpath".to_string(),
                rpath,
                binary_path.display().to_string(),
            ],
        )?;
    }

    Ok(())
}

fn macos_binary_rpaths(binary_path: &Path) -> Result<Vec<String>> {
    let output = run_command_capture_path(
        "otool",
        &["-l".to_string(), binary_path.display().to_string()],
    )?;
    let lines = output.lines().collect::<Vec<_>>();
    let mut rpaths = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if !line.contains("cmd LC_RPATH") {
            continue;
        }

        for inner in lines.iter().skip(index + 1).take(7).map(|line| line.trim()) {
            let Some(path) = inner.strip_prefix("path ") else {
                continue;
            };
            let Some((path, _offset)) = path.split_once(" (offset ") else {
                continue;
            };
            rpaths.push(path.to_string());
            break;
        }
    }

    Ok(rpaths)
}

fn prune_macos_optional_plugins(destination_framework: &Path) -> Result<Vec<PathBuf>> {
    let mut removed_plugins = Vec::new();
    for segments in OPTIONAL_MACOS_GSTREAMER_PLUGINS {
        let plugin_path = join_segments(destination_framework, segments);
        if !plugin_path.exists() {
            continue;
        }
        fs::remove_file(&plugin_path)?;
        removed_plugins.push(plugin_path);
    }
    Ok(removed_plugins)
}

fn prune_macos_runtime_payload(destination_framework: &Path) -> Result<usize> {
    let mut removed_paths = 0;

    for segments in MACOS_PRUNED_RUNTIME_DIRS {
        let pruned_path = join_segments(destination_framework, segments);
        if !pruned_path.exists() {
            continue;
        }
        fs::remove_dir_all(&pruned_path)?;
        removed_paths += 1;
    }

    let current_root = destination_framework.join("Versions").join("Current");
    removed_paths += remove_dirs_by_name(&current_root, MACOS_PRUNED_RUNTIME_DIR_NAMES)?;
    removed_paths += remove_files_by_name(destination_framework, MACOS_PRUNED_RUNTIME_FILE_NAMES)?;
    removed_paths += remove_files_by_extension(&current_root, MACOS_PRUNED_RUNTIME_EXTENSIONS)?;
    removed_paths += remove_broken_symlinks(destination_framework)?;

    Ok(removed_paths)
}

fn stage_windows_gstreamer_runtime(binary_dir: &Path, manifest: &GstreamerManifest) -> Result<()> {
    let destination_runtime_root = binary_dir.join("gstreamer");

    require_gstreamer_manifest_paths(&[
        (binary_dir, "Windows binary directory", true),
        (&manifest.root, "GStreamer runtime root", true),
        (&manifest.bin_dir, "GStreamer runtime bin directory", true),
        (&manifest.plugin_dir, "GStreamer plugin directory", true),
        (&manifest.scanner_path, "GStreamer plugin scanner", false),
    ])?;

    remove_path_if_exists(&destination_runtime_root)?;
    fs::create_dir_all(&destination_runtime_root)?;
    for segments in [
        &["lib", "gstreamer-1.0"][..],
        &["libexec", "gstreamer-1.0"][..],
    ] {
        copy_path_recursive(
            &join_segments(&manifest.root, segments),
            &join_segments(&destination_runtime_root, segments),
        )?;
    }

    for entry in fs::read_dir(&manifest.bin_dir)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = binary_dir.join(entry.file_name());
        copy_path_recursive(&source_path, &destination_path)?;
    }

    println!(
        "Staged Windows GStreamer runtime: {}",
        destination_runtime_root.display()
    );
    println!(
        "Scanner path: {}",
        destination_runtime_root
            .join("libexec")
            .join("gstreamer-1.0")
            .join("gst-plugin-scanner.exe")
            .display()
    );
    Ok(())
}

fn stage_linux_gstreamer_runtime(
    binary_dir: &Path,
    manifest: &GstreamerManifest,
    resources_dir: Option<&Path>,
) -> Result<()> {
    let lib_triplet = manifest_linux_lib_triplet(manifest)?;
    let destination_runtime_root = binary_dir.join("gstreamer");

    require_gstreamer_manifest_paths(&[
        (binary_dir, "Linux binary directory", true),
        (&manifest.root, "GStreamer runtime root", true),
        (&manifest.bin_dir, "GStreamer runtime bin directory", true),
        (&manifest.lib_dir, "GStreamer runtime lib directory", true),
        (&manifest.plugin_dir, "GStreamer plugin directory", true),
        (&manifest.scanner_path, "GStreamer plugin scanner", false),
    ])?;

    stage_linux_runtime_tree(
        &destination_runtime_root,
        manifest,
        &lib_triplet,
        "bundled-app",
    )?;
    let staged_manifest = linux_staged_manifest(
        manifest,
        &destination_runtime_root,
        &lib_triplet,
        "bundled-app",
    );
    let removed_plugins = prune_optional_linux_plugins(&staged_manifest.plugin_dir)?;
    patch_linux_runtime_rpaths(binary_dir, &staged_manifest, &lib_triplet)?;

    println!(
        "Staged Linux GStreamer runtime: {}",
        destination_runtime_root.display()
    );
    println!("Library directory: {}", staged_manifest.lib_dir.display());
    println!("Plugin directory: {}", staged_manifest.plugin_dir.display());
    println!("Scanner path: {}", staged_manifest.scanner_path.display());
    for plugin_path in removed_plugins {
        println!(
            "Removed optional GStreamer plugin: {}",
            plugin_path.display()
        );
    }

    if let Some(resources_dir) = resources_dir {
        stage_linux_runtime_tree(resources_dir, manifest, &lib_triplet, "bundled-resource")?;
        let resource_manifest =
            linux_staged_manifest(manifest, resources_dir, &lib_triplet, "bundled-resource");
        let removed_resource_plugins = prune_optional_linux_plugins(&resource_manifest.plugin_dir)?;
        patch_elf_tree(
            &resource_manifest.lib_dir,
            "$ORIGIN:$ORIGIN/..:$ORIGIN/../..:$ORIGIN/../../..",
        )?;
        patch_elf_tree(&resource_manifest.plugin_dir, "$ORIGIN/..")?;
        patch_elf_tree(
            &resource_manifest.bin_dir,
            &format!("$ORIGIN/../lib/{lib_triplet}"),
        )?;
        if is_elf_file(&resource_manifest.scanner_path) {
            set_linux_rpath(
                &resource_manifest.scanner_path,
                &format!("$ORIGIN/../../lib/{lib_triplet}"),
            )?;
        }
        println!(
            "Staged Linux GStreamer bundle resources: {}",
            resources_dir.display()
        );
        for plugin_path in removed_resource_plugins {
            println!(
                "Removed optional GStreamer resource plugin: {}",
                plugin_path.display()
            );
        }
    }

    Ok(())
}

fn stage_linux_runtime_tree(
    destination_root: &Path,
    manifest: &GstreamerManifest,
    lib_triplet: &str,
    mode: &str,
) -> Result<()> {
    remove_path_if_exists(destination_root)?;
    fs::create_dir_all(destination_root)?;

    for segments in [
        &["bin"][..],
        &["lib", lib_triplet][..],
        &["libexec", "gstreamer-1.0"][..],
        &["share"][..],
    ] {
        let source_path = join_segments(&manifest.root, segments);
        if !source_path.exists() {
            continue;
        }
        copy_path_recursive(&source_path, &join_segments(destination_root, segments))?;
    }

    let staged_manifest = linux_staged_manifest(manifest, destination_root, lib_triplet, mode);
    write_gstreamer_manifest_at(&destination_root.join("manifest.json"), &staged_manifest)
}

fn linux_staged_manifest(
    manifest: &GstreamerManifest,
    runtime_root: &Path,
    lib_triplet: &str,
    mode: &str,
) -> GstreamerManifest {
    let lib_dir = runtime_root.join("lib").join(lib_triplet);
    GstreamerManifest {
        version: manifest.version.clone(),
        platform: manifest.platform.clone(),
        arch: manifest.arch.clone(),
        mode: mode.to_string(),
        framework_path: None,
        root: runtime_root.to_path_buf(),
        bin_dir: runtime_root.join("bin"),
        lib_dir: lib_dir.clone(),
        pkg_config_dir: lib_dir.join("frame-pkgconfig"),
        plugin_dir: lib_dir.join("gstreamer-1.0"),
        scanner_path: runtime_root
            .join("libexec")
            .join("gstreamer-1.0")
            .join("gst-plugin-scanner"),
        lib_triplet: Some(lib_triplet.to_string()),
    }
}

fn write_gstreamer_manifest_at(path: &Path, manifest: &GstreamerManifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

fn manifest_linux_lib_triplet(manifest: &GstreamerManifest) -> Result<String> {
    if let Some(lib_triplet) = manifest.lib_triplet.as_deref() {
        return Ok(lib_triplet.to_string());
    }
    manifest
        .lib_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| {
            XtaskError::Usage(format!(
                "failed to infer Linux GStreamer lib triplet from {}",
                manifest.lib_dir.display()
            ))
        })
}

fn prune_optional_linux_plugins(plugin_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut removed_plugins = Vec::new();
    for plugin_name in OPTIONAL_LINUX_GSTREAMER_PLUGINS {
        let plugin_path = plugin_dir.join(plugin_name);
        if !plugin_path.exists() {
            continue;
        }
        fs::remove_file(&plugin_path)?;
        removed_plugins.push(plugin_path);
    }
    Ok(removed_plugins)
}

fn patch_linux_runtime_rpaths(
    binary_dir: &Path,
    manifest: &GstreamerManifest,
    lib_triplet: &str,
) -> Result<()> {
    let executable_rpath = format!("$ORIGIN/gstreamer/lib/{lib_triplet}:$ORIGIN/../lib");
    for entry in fs::read_dir(binary_dir)? {
        let entry = entry?;
        let file_path = entry.path();
        if entry.file_type()?.is_file() && is_elf_file(&file_path) {
            set_linux_rpath(&file_path, &executable_rpath)?;
        }
    }

    patch_elf_tree(
        &manifest.lib_dir,
        "$ORIGIN:$ORIGIN/..:$ORIGIN/../..:$ORIGIN/../../..",
    )?;
    patch_elf_tree(&manifest.plugin_dir, "$ORIGIN/..")?;
    patch_elf_tree(&manifest.bin_dir, &format!("$ORIGIN/../lib/{lib_triplet}"))?;
    if is_elf_file(&manifest.scanner_path) {
        set_linux_rpath(
            &manifest.scanner_path,
            &format!("$ORIGIN/../../lib/{lib_triplet}"),
        )?;
    }
    Ok(())
}

fn patch_elf_tree(root: &Path, rpath: &str) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry.file_type()?.is_dir() {
            patch_elf_tree(&entry_path, rpath)?;
        } else if entry.file_type()?.is_file() && is_elf_file(&entry_path) {
            set_linux_rpath(&entry_path, rpath)?;
        }
    }
    Ok(())
}

fn set_linux_rpath(file_path: &Path, rpath: &str) -> Result<()> {
    run_command_path(
        "patchelf",
        &[
            "--force-rpath".to_string(),
            "--set-rpath".to_string(),
            rpath.to_string(),
            file_path.display().to_string(),
        ],
    )
}

fn is_elf_file(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic).is_ok() && magic == [0x7f, b'E', b'L', b'F']
}

fn copy_path_recursive(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    remove_path_if_exists(destination)?;

    if metadata.file_type().is_symlink() {
        copy_symlink_or_target(source, destination)?;
        return Ok(());
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_path_recursive(&entry.path(), &destination.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

fn copy_symlink_or_target(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let target = fs::read_link(source)?;
        symlink(target, destination)?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let target = fs::canonicalize(source)?;
        copy_path_recursive(&target, destination)
    }
}

fn remove_path_if_exists(path: &Path) -> Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };

    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn remove_dirs_by_name(root: &Path, dir_names: &[&str]) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let mut removed = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let entry_path = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if dir_names.contains(&name) {
            fs::remove_dir_all(&entry_path)?;
            removed += 1;
        } else {
            removed += remove_dirs_by_name(&entry_path, dir_names)?;
        }
    }

    Ok(removed)
}

fn remove_files_by_name(root: &Path, file_names: &[&str]) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let mut removed = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry.file_type()?.is_dir() {
            removed += remove_files_by_name(&entry_path, file_names)?;
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if entry.file_type()?.is_file() && file_names.contains(&name) {
            fs::remove_file(&entry_path)?;
            removed += 1;
        }
    }

    Ok(removed)
}

fn remove_files_by_extension(root: &Path, extensions: &[&str]) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let mut removed = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry.file_type()?.is_dir() {
            removed += remove_files_by_extension(&entry_path, extensions)?;
            continue;
        }
        let extension = entry_path
            .extension()
            .and_then(|extension| extension.to_str());
        if entry.file_type()?.is_file() && extension.is_some_and(|ext| extensions.contains(&ext)) {
            fs::remove_file(&entry_path)?;
            removed += 1;
        }
    }

    Ok(removed)
}

fn remove_broken_symlinks(root: &Path) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let mut removed = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)?;
        if metadata.file_type().is_symlink() {
            if !entry_path.exists() {
                fs::remove_file(&entry_path)?;
                removed += 1;
            }
        } else if metadata.is_dir() {
            removed += remove_broken_symlinks(&entry_path)?;
        }
    }

    Ok(removed)
}

fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    segments
        .iter()
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

fn print_gstreamer_env(manifest: &GstreamerManifest) -> Result<()> {
    let manifest_path = gstreamer_manifest_path()?;
    if cfg!(target_os = "windows") {
        print_powershell_gstreamer_env(manifest, &manifest_path);
    } else {
        print_posix_gstreamer_env(manifest, &manifest_path);
    }
    Ok(())
}

fn gstreamer_command_env(manifest: &GstreamerManifest) -> Result<CommandEnv> {
    let manifest_path = gstreamer_manifest_path()?;
    let mut env = CommandEnv::default();

    env.set("PATH", prepend_env_path("PATH", &manifest.bin_dir)?);
    env.set(
        "PKG_CONFIG_PATH",
        prepend_env_path("PKG_CONFIG_PATH", &manifest.pkg_config_dir)?,
    );
    if manifest.platform != "linux" {
        env.set(
            "PKG_CONFIG_LIBDIR",
            manifest.pkg_config_dir.as_os_str().to_os_string(),
        );
    }
    env.set("FRAME_GSTREAMER_MANIFEST", manifest_path.into_os_string());
    env.set(
        "GST_PLUGIN_PATH_1_0",
        manifest.plugin_dir.as_os_str().to_os_string(),
    );
    env.set("GST_PLUGIN_SYSTEM_PATH_1_0", OsString::new());
    env.set(
        "GST_PLUGIN_SCANNER_1_0",
        manifest.scanner_path.as_os_str().to_os_string(),
    );

    let registry_dir = repo_root()?.join("target");
    fs::create_dir_all(&registry_dir)?;
    env.set(
        "GST_REGISTRY",
        registry_dir
            .join("gstreamer-registry-xtask-ci.bin")
            .into_os_string(),
    );

    if manifest.platform == "linux" {
        env.set(
            "LD_LIBRARY_PATH",
            prepend_env_path("LD_LIBRARY_PATH", &manifest.lib_dir)?,
        );
        env.set(
            "RUSTFLAGS",
            append_env_words(
                "RUSTFLAGS",
                &format!(
                    "-C link-arg=-Wl,-rpath,{} -C link-arg=-Wl,--allow-shlib-undefined",
                    manifest.lib_dir.display()
                ),
            ),
        );
    } else if manifest.platform == "macos" {
        env.set(
            "DYLD_FALLBACK_LIBRARY_PATH",
            prepend_env_path("DYLD_FALLBACK_LIBRARY_PATH", &manifest.lib_dir)?,
        );
        env.set(
            "RUSTFLAGS",
            append_env_words(
                "RUSTFLAGS",
                &format!("-C link-arg=-Wl,-rpath,{}", manifest.lib_dir.display()),
            ),
        );
    }

    Ok(env)
}

fn prepend_env_path(key: &str, path: &Path) -> Result<OsString> {
    let mut paths = vec![path.to_path_buf()];
    if let Some(existing) = env::var_os(key) {
        paths.extend(env::split_paths(&existing));
    }
    env::join_paths(paths)
        .map_err(|source| XtaskError::Usage(format!("failed to build {key}: {source}")))
}

fn append_env_words(key: &str, suffix: &str) -> OsString {
    let mut value = env::var_os(key).unwrap_or_default();
    if !value.as_os_str().is_empty() {
        value.push(" ");
    }
    value.push(suffix);
    value
}

fn print_posix_gstreamer_env(manifest: &GstreamerManifest, manifest_path: &Path) {
    println!(
        "export PATH=\"{}:${{PATH:-}}\"",
        shell_escape_path(&manifest.bin_dir)
    );
    println!(
        "export PKG_CONFIG_PATH=\"{}:${{PKG_CONFIG_PATH:-}}\"",
        shell_escape_path(&manifest.pkg_config_dir)
    );
    if manifest.platform != "linux" {
        println!(
            "export PKG_CONFIG_LIBDIR=\"{}\"",
            shell_escape_path(&manifest.pkg_config_dir)
        );
    }
    if manifest.platform == "linux" {
        println!(
            "export LD_LIBRARY_PATH=\"{}:${{LD_LIBRARY_PATH:-}}\"",
            shell_escape_path(&manifest.lib_dir)
        );
        println!(
            "export RUSTFLAGS=\"${{RUSTFLAGS:-}} -C link-arg=-Wl,-rpath,{} -C link-arg=-Wl,--allow-shlib-undefined\"",
            shell_escape_path(&manifest.lib_dir)
        );
    }
    if manifest.platform == "macos" {
        println!(
            "export RUSTFLAGS=\"${{RUSTFLAGS:-}} -C link-arg=-Wl,-rpath,{}\"",
            shell_escape_path(&manifest.lib_dir)
        );
        println!(
            "export DYLD_FALLBACK_LIBRARY_PATH=\"{}:${{DYLD_FALLBACK_LIBRARY_PATH:-}}\"",
            shell_escape_path(&manifest.lib_dir)
        );
    }
    println!(
        "export FRAME_GSTREAMER_MANIFEST=\"{}\"",
        shell_escape_path(manifest_path)
    );
}

fn print_powershell_gstreamer_env(manifest: &GstreamerManifest, manifest_path: &Path) {
    println!(
        "$env:PATH = \"{};$env:PATH\"",
        powershell_escape_path(&manifest.bin_dir)
    );
    println!(
        "$env:PKG_CONFIG_PATH = \"{};$env:PKG_CONFIG_PATH\"",
        powershell_escape_path(&manifest.pkg_config_dir)
    );
    println!(
        "$env:PKG_CONFIG_LIBDIR = \"{}\"",
        powershell_escape_path(&manifest.pkg_config_dir)
    );
    println!(
        "$env:FRAME_GSTREAMER_MANIFEST = \"{}\"",
        powershell_escape_path(manifest_path)
    );
}

fn require_gstreamer_manifest_paths(entries: &[(&Path, &str, bool)]) -> Result<()> {
    for (path, label, is_dir) in entries {
        if *is_dir && path.is_dir() {
            continue;
        }
        if !*is_dir && path.is_file() {
            continue;
        }
        return Err(XtaskError::Usage(format!(
            "{label} not found: {}",
            path.display()
        )));
    }
    Ok(())
}

fn require_gstreamer_pc_files(pkg_config_dir: &Path) -> Result<()> {
    for pc_file in [
        "gstreamer-1.0.pc",
        "gstreamer-app-1.0.pc",
        "gstreamer-video-1.0.pc",
        "gstreamer-pbutils-1.0.pc",
    ] {
        let path = pkg_config_dir.join(pc_file);
        if !path.is_file() {
            return Err(XtaskError::Usage(format!(
                "GStreamer pkg-config file not found: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn detect_gstreamer_version(pkg_config_dir: &Path) -> Result<String> {
    let pc_file = pkg_config_dir.join("gstreamer-1.0.pc");
    let content = fs::read_to_string(&pc_file)?;
    content
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("Version:")
                .map(str::trim)
                .map(str::to_string)
        })
        .ok_or_else(|| {
            XtaskError::Usage(format!(
                "GStreamer version not found in {}",
                pc_file.display()
            ))
        })
}

fn prepare_linux_gstreamer_pkg_config_dir(source_pkg_config_dir: &Path) -> Result<PathBuf> {
    let destination = source_pkg_config_dir
        .parent()
        .ok_or_else(|| {
            XtaskError::Usage(format!(
                "invalid Linux pkg-config dir `{}`",
                source_pkg_config_dir.display()
            ))
        })?
        .join("frame-pkgconfig");
    fs::remove_dir_all(&destination).ok();
    fs::create_dir_all(&destination)?;

    for entry in fs::read_dir(source_pkg_config_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if linux_gstreamer_pkg_config_allowed(name) {
            fs::copy(entry.path(), destination.join(name))?;
        }
    }

    Ok(destination)
}

fn linux_gstreamer_pkg_config_allowed(name: &str) -> bool {
    let is_pc = name
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("pc"));
    let has_allowed_prefix = name.starts_with("gst-")
        || name.starts_with("gstreamer-")
        || name.starts_with("libunwind-");
    (is_pc && has_allowed_prefix)
        || name.eq_ignore_ascii_case("orc-0.4.pc")
        || name.eq_ignore_ascii_case("zlib.pc")
        || name.eq_ignore_ascii_case("libunwind.pc")
}

fn default_gstreamer_download_dir() -> PathBuf {
    repo_root()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".cache")
        .join("gstreamer")
}

fn default_windows_gstreamer_root(download_dir: &Path, arch: &str, version: &str) -> PathBuf {
    download_dir
        .join(format!("windows-{arch}-{version}"))
        .join("msvc")
}

fn default_linux_gstreamer_root(download_dir: &Path, arch: &str, version: &str) -> PathBuf {
    download_dir
        .join(format!("linux-{arch}-{version}"))
        .join("runtime")
}

fn gstreamer_manifest_path() -> Result<PathBuf> {
    Ok(repo_root()?
        .join("frame-app")
        .join("vendor")
        .join("gstreamer")
        .join("manifest.json"))
}

fn linux_gstreamer_package_arch(arch: &str) -> Result<&'static str> {
    match arch {
        "x86_64" => Ok("x86_64"),
        "aarch64" => Ok("arm64"),
        other => Err(XtaskError::Usage(format!(
            "Linux GStreamer setup supports x86_64/aarch64, received {other}"
        ))),
    }
}

fn linux_gstreamer_lib_triplet(arch: &str) -> Result<&'static str> {
    match arch {
        "x86_64" => Ok("x86_64-linux-gnu"),
        "aarch64" => Ok("aarch64-linux-gnu"),
        other => Err(XtaskError::Usage(format!(
            "Linux library triplet is not configured for {other}"
        ))),
    }
}

fn normalize_arch(value: &str) -> Result<String> {
    match value {
        "arm64" | "aarch64" => Ok("aarch64".to_string()),
        "amd64" | "x64" | "x86_64" => Ok("x86_64".to_string()),
        "x86" | "i686" => Ok("x86".to_string()),
        other => Err(XtaskError::Usage(format!(
            "unsupported architecture `{other}`"
        ))),
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
    let destination = binary_dir.join(&entry.destination_name);
    if !force && destination.is_file() {
        println!(
            "Skipping {} (already exists). Use --force to re-download.",
            entry.destination_name
        );
        return Ok(());
    }

    let Some(url) = entry.url.as_deref() else {
        return Err(XtaskError::Usage(format!(
            "missing download URL for {}",
            entry.id
        )));
    };

    println!("Downloading {} from {url}...", entry.id);
    let archive = download_file(url)?;
    extract_expected_file(&archive, entry, &destination)
}

fn process_ffmpeg_shared_archive(
    url: &str,
    entries: &[FfmpegBinaryEntry],
    binary_dir: &Path,
    force: bool,
) -> Result<()> {
    let destinations = entries
        .iter()
        .map(|entry| binary_dir.join(&entry.destination_name))
        .collect::<Vec<_>>();
    let needs_download = force
        || destinations
            .iter()
            .any(|destination| !destination.is_file());

    if !needs_download {
        println!("Windows binaries already present. Use --force to refresh.");
        return Ok(());
    }

    println!("Downloading Windows bundle from {url}...");
    let archive = download_file(url)?;

    for (entry, destination) in entries.iter().zip(destinations.iter()) {
        if !force && destination.is_file() {
            println!(
                "Skipping {} (already exists). Use --force to re-download.",
                entry.destination_name
            );
            continue;
        }
        extract_expected_file(&archive, entry, destination)?;
    }

    Ok(())
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
            write_archive_file(&mut file, destination, entry.make_executable)?;
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
    let gstreamer_manifest = prepare_host_gstreamer_manifest("ci")?;
    let gstreamer_env = gstreamer_command_env(&gstreamer_manifest)?;
    run_command_with_env(
        "cargo",
        &["test", "--manifest-path", "frame-app/Cargo.toml"],
        &gstreamer_env,
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
    run_command_with_env(
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
        &gstreamer_env,
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
    run_command_inner(program, args, None)
}

fn run_command_with_env(program: &str, args: &[&str], env: &CommandEnv) -> Result<()> {
    run_command_inner(program, args, Some(env))
}

fn run_command_inner(program: &str, args: &[&str], env: Option<&CommandEnv>) -> Result<()> {
    let root = repo_root()?;
    println!("$ {} {}", program, args.join(" "));
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(env) = env {
        env.apply(&mut command);
    }
    let status = command
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
    run_command_path_inner(program, args, None)
}

fn run_command_path_with_env(
    program: impl AsRef<OsStr>,
    args: &[String],
    env: &CommandEnv,
) -> Result<()> {
    run_command_path_inner(program, args, Some(env))
}

fn run_command_path_inner(
    program: impl AsRef<OsStr>,
    args: &[String],
    env: Option<&CommandEnv>,
) -> Result<()> {
    let program = program.as_ref();
    println!(
        "$ {} {}",
        program.to_string_lossy(),
        args.iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    );
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(repo_root()?)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(env) = env {
        env.apply(&mut command);
    }
    let status = command
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

fn run_command_capture_path(program: impl AsRef<OsStr>, args: &[String]) -> Result<String> {
    let program = program.as_ref();
    println!(
        "$ {} {}",
        program.to_string_lossy(),
        args.iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    );
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| XtaskError::CommandSpawn {
            program: program.to_string_lossy().into_owned(),
            source,
        })?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(XtaskError::Usage(format!(
        "`{}` failed with status {}: {}",
        program.to_string_lossy(),
        output.status,
        stderr.trim()
    )))
}

fn shell_escape_path(path: &Path) -> String {
    path.display().to_string().replace('"', "\\\"")
}

fn powershell_escape_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('`', "``")
        .replace('"', "`\"")
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
  pull_request:
    types:
      - labeled
      - synchronize
";

    let jobs = [
        linux_job("x86_64", "ubuntu-22.04"),
        linux_job("aarch64", "ubuntu-22.04-arm"),
        macos_job("x86_64", "x86_64-apple-darwin", "macos-15-intel"),
        macos_job("aarch64", "aarch64-apple-darwin", "macos-15"),
        windows_job("x86_64", "windows-2022"),
    ]
    .join("");

    format!("{header}jobs:\n{jobs}")
}

const fn bundle_if_expression() -> &'static str {
    "      (github.event.action == 'labeled' && github.event.label.name == 'run-bundling') ||\n      (github.event.action == 'synchronize' && contains(github.event.pull_request.labels.*.name, 'run-bundling'))"
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
        sudo apt-get install -y clang libfontconfig1-dev libfreetype6-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libdrm-dev pkg-config patchelf
    - name: ./script/bundle-linux
      run: ./script/bundle-linux
    - name: run_bundling::upload_artifact
      uses: actions/upload-artifact@v4
      with:
        name: frame-linux-{arch}.tar.gz
        path: target/release/frame-linux-{arch}.tar.gz
        if-no-files-found: error
    timeout-minutes: 60
",
        if_expression = bundle_if_expression(),
        checkout = checkout_step(),
        rust = setup_rust_step(),
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
{checkout}{rust}    - name: ./script/bundle-windows.ps1
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
      - 'v*'
  workflow_dispatch:
    inputs:
      tag:
        description: Release tag to publish.
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
        sudo apt-get install -y clang libfontconfig1-dev libfreetype6-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libdrm-dev pkg-config patchelf
    - name: ./script/bundle-linux
      run: ./script/bundle-linux
    - name: release::upload_linux_x86_64
      uses: actions/upload-artifact@v4
      with:
        name: frame-linux-x86_64.tar.gz
        path: target/release/frame-linux-x86_64.tar.gz
        if-no-files-found: error
    timeout-minutes: 60

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
        sudo apt-get install -y clang libfontconfig1-dev libfreetype6-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libdrm-dev pkg-config patchelf
    - name: ./script/bundle-linux
      run: ./script/bundle-linux
    - name: release::upload_linux_aarch64
      uses: actions/upload-artifact@v4
      with:
        name: frame-linux-aarch64.tar.gz
        path: target/release/frame-linux-aarch64.tar.gz
        if-no-files-found: error
    timeout-minutes: 60

  build_macos_x86_64:
    runs-on: macos-15-intel
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY || secrets.FRAME_UPDATE_PUBLIC_KEY }}
      MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
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
      uses: Apple-Actions/import-codesign-certs@v3
      with:
        p12-file-base64: ${{ secrets.MACOS_CERTIFICATES_P12 }}
        p12-password: ${{ secrets.MACOS_CERTIFICATES_PASSWORD }}
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
    runs-on: macos-15
    env:
      CARGO_INCREMENTAL: 0
      FRAME_UPDATE_PUBLIC_KEY: ${{ vars.FRAME_UPDATE_PUBLIC_KEY || secrets.FRAME_UPDATE_PUBLIC_KEY }}
      MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
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
      uses: Apple-Actions/import-codesign-certs@v3
      with:
        p12-file-base64: ${{ secrets.MACOS_CERTIFICATES_P12 }}
        p12-password: ${{ secrets.MACOS_CERTIFICATES_PASSWORD }}
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
        version="${tag#v}"
        echo "tag=$tag" >> "$GITHUB_OUTPUT"
        echo "version=$version" >> "$GITHUB_OUTPUT"
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
          target/release/update-manifest.json
          target/release/update-manifest.json.sig
        )
        if gh release view "$tag" >/dev/null 2>&1; then
          gh release upload "$tag" "${assets[@]}" --clobber
        else
          gh release create "$tag" "${assets[@]}" --title "Frame ${{ steps.release.outputs.version }}" --generate-notes
        fi
    timeout-minutes: 30
"#
    .to_string()
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
            .map(|entry| entry.destination_name.as_str())
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
            .map(|entry| entry.destination_name.as_str())
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
            "ffmpeg-master-latest-win64-gpl/bin/ffprobe.exe",
            &["ffprobe.exe"],
        ));
    }
}
