//! Runtime environment detection shared by platform integrations.

use std::path::Path;

const FLATPAK_INFO_PATH: &str = "/.flatpak-info";

#[must_use]
pub fn is_flatpak() -> bool {
    is_flatpak_from(
        std::env::var_os("FLATPAK_ID").is_some(),
        Path::new(FLATPAK_INFO_PATH).is_file(),
    )
}

const fn is_flatpak_from(flatpak_id_present: bool, flatpak_info_exists: bool) -> bool {
    flatpak_id_present || flatpak_info_exists
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatpak_detection_accepts_environment_marker() {
        assert!(is_flatpak_from(true, false));
    }

    #[test]
    fn flatpak_detection_accepts_info_file() {
        assert!(is_flatpak_from(false, true));
    }

    #[test]
    fn flatpak_detection_accepts_both_markers() {
        assert!(is_flatpak_from(true, true));
    }

    #[test]
    fn flatpak_detection_rejects_missing_markers() {
        assert!(!is_flatpak_from(false, false));
    }
}
