use std::{
    env, fs,
    path::{Path, PathBuf},
};

const APP_NAME: &str = "Frame";
const COMPANY_NAME: &str = "66HEX";
const LEGAL_COPYRIGHT: &str = "Copyright 2026 Marek Jozwiak";
const LINUX_APP_ICON: &str = "resources/app-icons/icon.png";
const WINDOWS_APP_ICON: &str = "resources/app-icons/icon.ico";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    println!("cargo:rerun-if-changed={LINUX_APP_ICON}");
    println!("cargo:rerun-if-changed={WINDOWS_APP_ICON}");

    match env::var("CARGO_CFG_TARGET_OS")?.as_str() {
        "windows" => compile_windows_resources()?,
        "linux" | "freebsd" => prepare_app_icon_x11()?,
        _ => {}
    }

    Ok(())
}

fn compile_windows_resources() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let icon_path = manifest_dir.join(WINDOWS_APP_ICON);
    let icon_escaped = icon_path.to_string_lossy().replace('\\', "\\\\");
    let package_version = env::var("CARGO_PKG_VERSION")?;
    let file_version = windows_file_version(&package_version);
    let rc_content = format!(
        r#"1 ICON "{icon_escaped}"

1 VERSIONINFO
FILEVERSION {file_version}
PRODUCTVERSION {file_version}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "FileDescription", "{APP_NAME}\0"
            VALUE "FileVersion", "{package_version}\0"
            VALUE "ProductName", "{APP_NAME}\0"
            VALUE "ProductVersion", "{package_version}\0"
            VALUE "CompanyName", "{COMPANY_NAME}\0"
            VALUE "LegalCopyright", "{LEGAL_COPYRIGHT}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let rc_path = out_dir.join("frame_resources.rc");
    fs::write(&rc_path, rc_content)?;
    embed_resource::compile(&rc_path, embed_resource::NONE).manifest_optional()?;

    Ok(())
}

fn windows_file_version(package_version: &str) -> String {
    let mut parts = package_version
        .split('.')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .chain(std::iter::repeat(0));

    format!(
        "{},{},{},{}",
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0)
    )
}

fn prepare_app_icon_x11() -> Result<(), Box<dyn std::error::Error>> {
    use image::{ImageReader, imageops};

    let icon = ImageReader::open(LINUX_APP_ICON)?
        .decode()?
        .resize(256, 256, imageops::FilterType::Lanczos3)
        .into_rgba8();
    let icon_out_path = Path::new(&env::var("OUT_DIR")?).join("app_icon.png");
    icon.save(icon_out_path)?;

    Ok(())
}
