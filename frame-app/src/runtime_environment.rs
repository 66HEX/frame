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
    fn flatpak_detection_follows_the_marker_truth_table() {
        for (flatpak_id_present, flatpak_info_exists, expected) in [
            (true, false, true),
            (false, true, true),
            (true, true, true),
            (false, false, false),
        ] {
            assert_eq!(
                is_flatpak_from(flatpak_id_present, flatpak_info_exists),
                expected,
                "unexpected detection for env={flatpak_id_present}, info-file={flatpak_info_exists}"
            );
        }
    }
}
