//! Additional main-input streams shared by visual reencoding and preview clocks.

use crate::{
    error::ConversionError,
    media_rules::{SubtitleStreamAction, is_image_container},
    types::ConversionConfig,
    utils::is_audio_only_container,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MainInputStreamMap {
    pub audio: Vec<u32>,
    pub subtitles: Vec<u32>,
}

/// Resolves the extra main-input tracks mapped beside reencoded video.
/// Sidecar inputs never participate; Copy and non-video branches are dormant.
///
/// # Errors
/// Returns an error for a missing selected stream or an unsupported subtitle.
/// Encoder settings unrelated to stream mapping are deliberately not validated.
pub fn resolve_reencode_main_input_map(
    config: &ConversionConfig,
    available_audio: &[u32],
    available_subtitles: &[(u32, &str)],
) -> Result<MainInputStreamMap, ConversionError> {
    if config.processing_mode == "copy"
        || is_audio_only_container(&config.container)
        || is_image_container(&config.container)
        || config.container.eq_ignore_ascii_case("gif")
    {
        return Ok(MainInputStreamMap::default());
    }
    for index in &config.selected_audio_tracks {
        if !available_audio.contains(index) {
            return Err(ConversionError::InvalidInput(format!(
                "Selected audio track #{index} was not found in source"
            )));
        }
    }
    let subtitles = config
        .selected_subtitle_tracks
        .iter()
        .map(|index| {
            available_subtitles
                .iter()
                .find(|(found, _)| found == index)
                .ok_or_else(|| {
                    ConversionError::InvalidInput(format!(
                        "Selected subtitle track #{index} was not found in source"
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (index, codec) in subtitles {
        if let Err(reason) = embedded_subtitle_action(&config.container, codec) {
            return Err(ConversionError::InvalidInput(format!(
                "Subtitle codec '{codec}' from source track #{index} cannot be represented by '{}': {reason}",
                config.container,
            )));
        }
    }
    Ok(MainInputStreamMap {
        audio: config.selected_audio_tracks.clone(),
        subtitles: config.selected_subtitle_tracks.clone(),
    })
}

pub(crate) fn embedded_subtitle_action(
    container: &str,
    codec: &str,
) -> Result<SubtitleStreamAction, &'static str> {
    crate::media_rules::subtitle_stream_action(container, codec).ok_or(
        "subtitle codec cannot be copied or decoded for the selected container; choose another output or leave this track unselected",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapped_selection_keeps_order_duplicates_and_subtitle_error_precedence() {
        let mut config = ConversionConfig {
            container: "mkv".into(),
            selected_audio_tracks: vec![2, 1, 2],
            selected_subtitle_tracks: vec![4, 3],
            ..serde_json::from_value(serde_json::json!({
                "container": "mp4",
                "videoCodec": "libx264",
                "videoBitrateMode": "crf",
                "videoBitrate": "5000",
                "audioCodec": "aac",
                "audioBitrate": "128",
                "audioChannels": "original",
                "selectedAudioTracks": [],
                "selectedSubtitleTracks": [],
                "subtitleBurnPath": null,
                "resolution": "original",
                "customWidth": null,
                "customHeight": null,
                "scalingAlgorithm": "bicubic",
                "fps": "original",
                "crf": 23,
                "preset": "medium",
                "startTime": null,
                "endTime": null
            }))
            .unwrap()
        };
        let subtitles = [(3, "hdmv_pgs_subtitle"), (4, "ass")];
        let mapped = resolve_reencode_main_input_map(&config, &[1, 2], &subtitles).unwrap();
        assert_eq!(mapped.audio, vec![2, 1, 2]);
        assert_eq!(mapped.subtitles, vec![4, 3]);
        config.container = "mp4".into();
        config.selected_subtitle_tracks = vec![3, 99];
        let error = resolve_reencode_main_input_map(&config, &[1, 2], &subtitles).unwrap_err();
        assert!(error.to_string().contains("track #99 was not found"));
        config.selected_subtitle_tracks = vec![3];
        let error = resolve_reencode_main_input_map(&config, &[1, 2], &subtitles).unwrap_err();
        assert!(error.to_string().contains("cannot be represented by 'mp4'"));
        for (mode, container) in [
            ("copy", "mkv"),
            ("reencode", "wav"),
            ("reencode", "png"),
            ("reencode", "gif"),
        ] {
            config.processing_mode = mode.into();
            config.container = container.into();
            assert_eq!(
                resolve_reencode_main_input_map(&config, &[], &[]).unwrap(),
                MainInputStreamMap::default()
            );
        }
    }
}
