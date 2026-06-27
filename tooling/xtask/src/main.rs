use std::{
    env, fmt, fs, io,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

const RUN_BUNDLING_WORKFLOW_PATH: &str = ".github/workflows/run_bundling.yml";
const MARTIN_FFMPEG_BASE_URL: &str = "https://ffmpeg.martin-riedl.de/redirect/latest";
const WINDOWS_FFMPEG_ZIP_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

type Result<T> = std::result::Result<T, XtaskError>;

fn main() -> ExitCode {
    match run_xtask() {
        Ok(()) => ExitCode::SUCCESS,
        Err(XtaskError::Help) => ExitCode::SUCCESS,
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
        "bundle" => bundle(args.next().as_deref()),
        "ci" => ci(),
        "setup-ffmpeg" => setup_ffmpeg(args.collect()),
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
  bundle macos      Build the macOS .app and .dmg package
  bundle linux      Build the Linux tarball package
  bundle windows    Build the Windows Inno Setup installer
  setup-ffmpeg      Download FFmpeg and FFprobe runtime binaries
  ci                Run local formatting, tests, lints, and script checks
  workflows         Regenerate GitHub Actions workflows
"
    );
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

fn setup_ffmpeg(args: Vec<String>) -> Result<()> {
    let options = SetupFfmpegOptions::parse(&args)?;
    let target = ffmpeg_target_for(
        options.platform.as_deref().unwrap_or(host_platform()),
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
    fn label(&self) -> &'static str {
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

fn host_arch() -> &'static str {
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
    let path = repo_root()?.join(RUN_BUNDLING_WORKFLOW_PATH);
    let content = run_bundling_workflow();
    fs::create_dir_all(path.parent().expect("workflow path should have a parent"))?;
    fs::write(&path, content)?;
    println!("Wrote {}", path.display());
    Ok(())
}

fn run_script(script: &str, args: &[&str]) -> Result<()> {
    if script.ends_with(".ps1") && !cfg!(target_os = "windows") {
        return Err(XtaskError::Usage(
            "Windows bundles must be built on Windows.".to_string(),
        ));
    }

    if cfg!(target_os = "windows") && script.ends_with(".ps1") {
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

fn bundle_if_expression() -> &'static str {
    "      (github.event.action == 'labeled' && github.event.label.name == 'run-bundling') ||\n      (github.event.action == 'synchronize' && contains(github.event.pull_request.labels.*.name, 'run-bundling'))"
}

fn checkout_step() -> &'static str {
    r#"    - name: steps::checkout_repo
      uses: actions/checkout@v4
"#
}

fn setup_rust_step() -> &'static str {
    r#"    - name: steps::setup_rust
      uses: dtolnay/rust-toolchain@stable
"#
}

fn linux_job(arch: &str, runner: &str) -> String {
    format!(
        r#"  bundle_linux_{arch}:
    if: |-
{if_expression}
    runs-on: {runner}
    env:
      CARGO_INCREMENTAL: 0
    steps:
{checkout}{rust}    - name: steps::setup_linux
      run: |
        sudo apt-get update
        sudo apt-get install -y clang libfontconfig1-dev libfreetype6-dev libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev pkg-config
    - name: ./script/bundle-linux
      run: ./script/bundle-linux
    - name: run_bundling::upload_artifact
      uses: actions/upload-artifact@v4
      with:
        name: frame-linux-{arch}.tar.gz
        path: target/release/frame-linux-{arch}.tar.gz
        if-no-files-found: error
    timeout-minutes: 60
"#,
        if_expression = bundle_if_expression(),
        checkout = checkout_step(),
        rust = setup_rust_step(),
    )
}

fn macos_job(arch: &str, target: &str, runner: &str) -> String {
    format!(
        r#"  bundle_macos_{arch}:
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
    timeout-minutes: 60
"#,
        if_expression = bundle_if_expression(),
        checkout = checkout_step(),
        rust = setup_rust_step(),
    )
}

fn windows_job(arch: &str, runner: &str) -> String {
    format!(
        r#"  bundle_windows_{arch}:
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
"#,
        if_expression = bundle_if_expression(),
        checkout = checkout_step(),
        rust = setup_rust_step(),
    )
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
