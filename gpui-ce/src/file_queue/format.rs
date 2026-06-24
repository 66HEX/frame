use std::path::Path;

#[must_use]
pub fn file_size_bytes(path: &Path) -> u64 {
    path.metadata()
        .ok()
        .filter(std::fs::Metadata::is_file)
        .map_or(0, |metadata| metadata.len())
}

#[must_use]
pub fn file_name_from_path(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("unknown")
}

#[must_use]
pub fn original_format_from_name(name: &str) -> &str {
    name.rsplit('.')
        .next()
        .filter(|extension| !extension.is_empty())
        .unwrap_or("unknown")
}

#[must_use]
pub fn derive_output_name(file_name: &str) -> String {
    let base = file_name.rfind('.').map_or(file_name, |dot_index| {
        let extension = &file_name[dot_index + 1..];
        if extension.is_empty() || extension.contains(['/', '\\', '.']) {
            file_name
        } else {
            &file_name[..dot_index]
        }
    });

    if base.is_empty() {
        "output_converted".to_string()
    } else {
        format!("{base}_converted")
    }
}

#[must_use]
pub fn format_file_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }

    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    format!("{} {}", trim_two_decimal_places(value), UNITS[unit_index])
}

fn trim_two_decimal_places(value: f64) -> String {
    let formatted = format!("{value:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
