use super::*;
use frame_core::dimensions::output_dimensions;

#[test]
fn output_size_follows_rotation_crop_scaling_and_padding() {
    let config = ConversionConfig {
        rotation: "90".into(),
        ..ConversionConfig::default()
    };
    let mut core = crate::conversion_runner::core_config_from_gpui(&config);
    assert_eq!(
        output_dimensions(&core, 1920, 1080, Some("yuv420p")),
        Some((1080, 1920))
    );
    core.resolution = "720p".into();
    assert_eq!(
        output_dimensions(&core, 1920, 1080, Some("yuv420p")),
        Some((406, 720))
    );
    core.resolution = "custom".into();
    core.custom_width = Some("641".into());
    core.custom_height = Some("481".into());
    assert_eq!(
        output_dimensions(&core, 1920, 1080, Some("yuv420p")),
        Some((642, 482))
    );
    core.container = "png".into();
    assert_eq!(
        output_dimensions(&core, 1920, 1080, Some("yuv420p")),
        Some((641, 481))
    );
    core.custom_width = Some(String::new());
    assert_eq!(output_dimensions(&core, 1920, 1080, Some("yuv420p")), None);
    core.processing_mode = "copy".into();
    assert_eq!(
        output_dimensions(&core, 1920, 1080, Some("yuv420p")),
        Some((1920, 1080))
    );
    core.container = "mp3".into();
    assert_eq!(output_dimensions(&core, 1920, 1080, Some("yuv420p")), None);
}
