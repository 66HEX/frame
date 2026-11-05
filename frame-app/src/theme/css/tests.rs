use super::*;

const MIRA: &str = include_str!("../../../docs/design-system/reference/theme.css");

#[test]
fn css_import_matches_the_original_cli_snapshot_without_quantizing_alpha() {
    let imported = import(MIRA).unwrap();
    let original: Value = serde_json::from_str(include_str!(
        "../../../docs/design-system/reference/mira-neutral.json"
    ))
    .unwrap();
    assert!((imported.radius - 10.0).abs() < f32::EPSILON);
    for mode in ["light", "dark"] {
        for (name, value) in original[mode].as_object().unwrap() {
            let expected = value["srgb"].as_array().unwrap();
            let actual = imported.colors[mode][name]["srgb"].as_array().unwrap();
            for (a, b) in actual.iter().zip(expected) {
                assert!(
                    (a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 0.00002,
                    "{mode} --{name}: {actual:?} vs {expected:?}"
                );
            }
        }
    }
}

#[test]
fn pasted_theme_changes_colors_alpha_radius_and_aliases_in_both_modes() {
    let css = format!(
        "{MIRA}\n@layer base {{ :root {{ --brand: #123abc; --primary: var(--brand); --radius: 1rem; --border: rgb(12 34 56 / 25%); --ring: 240 50% 50%; }} .dark {{ --primary: hsl(120 100% 50% / .4); }} }}"
    );
    let theme = import(&css).unwrap();
    assert!((theme.radius - 16.0).abs() < f32::EPSILON);
    let light = theme.colors["light"]["primary"]["srgb"].as_array().unwrap();
    assert!((light[0].as_f64().unwrap() - 18.0 / 255.0).abs() < 0.000_001);
    assert!((theme.colors["dark"]["primary"]["srgb"][3].as_f64().unwrap() - 0.4).abs() < 0.000_001);
    assert!(
        (theme.colors["light"]["border"]["srgb"][3].as_f64().unwrap() - 0.25).abs() < 0.000_001
    );
}

#[test]
fn theme_errors_identify_the_token_instead_of_silently_using_neutral() {
    assert!(
        import(&format!("{MIRA} :root {{ --primary: oklch(broken); }}"))
            .err()
            .unwrap()
            .contains("--primary")
    );
    assert!(
        import(&format!(
            "{MIRA} :root {{ --primary: var(--brand); --brand: var(--primary); }}"
        ))
        .err()
        .unwrap()
        .contains("cyclic")
    );
    assert!(
        import(&format!("{MIRA} :root {{ --primary: var(--missing); }}"))
            .err()
            .unwrap()
            .contains("--missing")
    );
}
