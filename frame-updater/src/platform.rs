use crate::{UpdateAssetKind, UpdateError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformAssetKey {
    MacosAarch64,
    MacosX8664,
    WindowsX8664,
    LinuxX8664,
    LinuxAarch64,
}

impl PlatformAssetKey {
    pub fn current() -> Result<Self, UpdateError> {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Ok(Self::MacosAarch64)
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            Ok(Self::MacosX8664)
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Ok(Self::WindowsX8664)
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Ok(Self::LinuxX8664)
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            Ok(Self::LinuxAarch64)
        } else {
            Err(UpdateError::UnsupportedPlatform(format!(
                "{}/{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )))
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MacosAarch64 => "macos-aarch64",
            Self::MacosX8664 => "macos-x86_64",
            Self::WindowsX8664 => "windows-x86_64",
            Self::LinuxX8664 => "linux-x86_64",
            Self::LinuxAarch64 => "linux-aarch64",
        }
    }

    #[must_use]
    pub const fn target_triple(self) -> &'static str {
        match self {
            Self::MacosAarch64 => "aarch64-apple-darwin",
            Self::MacosX8664 => "x86_64-apple-darwin",
            Self::WindowsX8664 => "x86_64-pc-windows-msvc",
            Self::LinuxX8664 => "x86_64-unknown-linux-gnu",
            Self::LinuxAarch64 => "aarch64-unknown-linux-gnu",
        }
    }

    #[must_use]
    pub const fn asset_kind(self) -> UpdateAssetKind {
        match self {
            Self::MacosAarch64 | Self::MacosX8664 => UpdateAssetKind::MacosAppZip,
            Self::WindowsX8664 => UpdateAssetKind::WindowsInno,
            Self::LinuxX8664 | Self::LinuxAarch64 => UpdateAssetKind::LinuxManagedTar,
        }
    }
}
