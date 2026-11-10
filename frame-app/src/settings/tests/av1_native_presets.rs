use super::*;

#[test]
fn native_av1_presets_preserve_legacy_selections_and_allow_every_value() {
    let mut config = ConversionConfig {
        video_codec: "libsvtav1".into(),
        preset: "slower".into(),
        ..ConversionConfig::default()
    };
    let options = video_preset_options(&config, false);
    assert_eq!(options.len(), 14);
    assert_eq!(
        options
            .iter()
            .find(|option| option.is_selected)
            .unwrap()
            .preset,
        "4"
    );
    for preset in 0..=13 {
        apply_video_preset(&mut config, &preset.to_string());
        normalize_output_config(&mut config, None);
        assert_eq!(config.preset, preset.to_string());
        assert_eq!(
            video_preset_options(&config, false)
                .iter()
                .filter(|option| option.is_selected)
                .count(),
            1
        );
    }
    assert!(!apply_video_preset(&mut config, "14"));
    assert!(!is_video_preset_allowed("libx264", "4"));
    assert!(!is_video_preset_allowed("hevc_nvenc", "4"));
}

#[test]
fn av1_ten_bit_and_preset_four_reach_the_core_unchanged() {
    let mut config = ConversionConfig {
        video_codec: "libsvtav1".into(),
        ..ConversionConfig::default()
    };
    assert!(apply_video_preset(&mut config, "4"));
    assert!(apply_pixel_format(&mut config, "yuv420p10le"));
    let core = crate::conversion_runner::core_config_from_gpui(&config);
    assert_eq!(core.preset, "4");
    assert_eq!(core.pixel_format, "yuv420p10le");
    assert!(
        video_preset_options(&config, true)
            .iter()
            .all(|option| option.is_disabled)
    );
}
