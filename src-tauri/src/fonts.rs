use font_kit::source::SystemSource;

/// Returns a sorted list of font family names available on the system.
#[tauri::command]
pub fn list_system_fonts() -> Vec<String> {
    let source = SystemSource::new();
    let mut families = source.all_families().unwrap_or_default();
    families.sort_unstable_by_key(|family| family.to_lowercase());
    families
}
