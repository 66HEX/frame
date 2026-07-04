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
const RUN_BUNDLING_WORKFLOW_PATH: &str = ".github/workflows/run_bundling.yml";
const RELEASE_WORKFLOW_PATH: &str = ".github/workflows/release.yml";
const MARTIN_FFMPEG_BASE_URL: &str = "https://ffmpeg.martin-riedl.de/redirect/latest";
const WINDOWS_FFMPEG_ZIP_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";

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
        macos_job("x86_64", "x86_64-apple-darwin", "macos-15-intel"),
        macos_job("aarch64", "aarch64-apple-darwin", "macos-15"),
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
    let linux_packages = "clang curl desktop-file-utils file flatpak flatpak-builder libfontconfig1-dev libfreetype6-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libdrm-dev pkg-config patchelf";
    let flatpak_setup = r"    - name: steps::setup_flatpak
      run: |
        flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
        flatpak install --user -y --noninteractive flathub org.freedesktop.Platform//24.08 org.freedesktop.Sdk//24.08
";
    let bundle_args = "--tarball --appimage --flatpak";
    let flatpak_upload = format!(
        r"    - name: run_bundling::upload_flatpak_artifact
      uses: actions/upload-artifact@v4
      with:
        name: Frame-{arch}.flatpak
        path: target/release/Frame-{arch}.flatpak
        if-no-files-found: error
"
    );
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
{flatpak_setup}    - name: ./script/bundle-linux
      env:
        APPIMAGETOOL: /tmp/appimagetool.AppImage
      run: ./script/bundle-linux {bundle_args}
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
{flatpak_upload}
    timeout-minutes: {timeout_minutes}
",
        if_expression = bundle_if_expression(),
        checkout = checkout_step(),
        rust = setup_rust_step(),
        appimagetool_arch = appimagetool_arch,
        linux_packages = linux_packages,
        flatpak_setup = flatpak_setup,
        bundle_args = bundle_args,
        flatpak_upload = flatpak_upload,
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
