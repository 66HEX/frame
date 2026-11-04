use super::*;
use frame_core::capabilities::AvailableEncoders;

fn codec_option<'a>(options: &'a [AudioCodecOption], codec: &str) -> &'a AudioCodecOption {
    options
        .iter()
        .find(|option| option.codec == codec)
        .unwrap_or_else(|| panic!("{codec} codec option should exist"))
}

fn encoders() -> AvailableEncoders {
    AvailableEncoders {
        libfdk_aac: true,
        ..AvailableEncoders::default()
    }
}

#[test]
fn marks_default_aac_selected_for_mp4() {
    let options = audio_codec_options(&ConversionConfig::default(), &encoders(), false);

    assert!(codec_option(&options, "aac").is_selected);
}

#[test]
fn marks_flac_incompatible_for_mp4() {
    let options = audio_codec_options(&ConversionConfig::default(), &encoders(), false);

    assert_eq!(
        codec_option(&options, "flac").disabled_reason,
        Some("Incompatible container")
    );
}

#[test]
fn keeps_mov_flac_codec_enabled() {
    let config = ConversionConfig {
        container: "mov".to_string(),
        ..ConversionConfig::default()
    };

    let options = audio_codec_options(&config, &encoders(), false);

    assert!(!codec_option(&options, "flac").is_disabled);
}

#[test]
fn disables_all_codecs_in_stream_copy_mode() {
    let config = ConversionConfig {
        processing_mode: ProcessingMode::Copy,
        ..ConversionConfig::default()
    };

    let options = audio_codec_options(&config, &encoders(), false);

    assert!(options.iter().all(|option| option.is_disabled));
}

#[test]
fn hides_libfdk_aac_when_encoder_is_unavailable() {
    let options = audio_codec_options(
        &ConversionConfig::default(),
        &AvailableEncoders::default(),
        false,
    );

    assert!(options.iter().all(|option| option.codec != "libfdk_aac"));
}

#[test]
fn apply_audio_codec_updates_allowed_codec() {
    let mut config = ConversionConfig {
        container: "mov".to_string(),
        ..ConversionConfig::default()
    };

    assert!(apply_audio_codec(&mut config, "flac"));

    assert_eq!(config.audio_codec, "flac");
}

#[test]
fn apply_audio_codec_rejects_incompatible_codec() {
    let mut config = ConversionConfig::default();

    assert!(!apply_audio_codec(&mut config, "flac"));

    assert_eq!(config.audio_codec, "aac");
}

#[test]
fn apply_audio_codec_rejects_unknown_codec() {
    let mut config = ConversionConfig::default();

    assert!(!apply_audio_codec(&mut config, "totally_unknown"));

    assert_eq!(config.audio_codec, "aac");
}
