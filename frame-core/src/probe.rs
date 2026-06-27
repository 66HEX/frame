//! FFprobe argument construction and metadata parsing.

use std::path::Path;

use crate::error::ConversionError;
use crate::types::{AudioTrack, FfprobeOutput, ProbeMetadata, SubtitleTrack};
use crate::utils::{parse_frame_rate_string, parse_probe_bitrate};

#[must_use]
pub fn ffprobe_json_args(file_path: &str) -> Vec<String> {
    vec![
        "-v".to_string(),
        "quiet".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        file_path.to_string(),
    ]
}

pub fn parse_ffprobe_stdout(
    file_path: &str,
    stdout: impl AsRef<str>,
) -> Result<ProbeMetadata, ConversionError> {
    let probe_data: FfprobeOutput = serde_json::from_str(stdout.as_ref())?;
    Ok(metadata_from_ffprobe(file_path, probe_data))
}

#[expect(
    clippy::too_many_lines,
    reason = "ffprobe parsing keeps track extraction in one pass over streams"
)]
fn metadata_from_ffprobe(file_path: &str, probe_data: FfprobeOutput) -> ProbeMetadata {
    let source_format_name = probe_data.format.format_name.clone();

    let mut metadata = ProbeMetadata {
        duration: probe_data.format.duration,
        bitrate: probe_data.format.bit_rate,
        ..ProbeMetadata::default()
    };

    if let Some(tags) = probe_data.format.tags {
        metadata.tags = Some(tags);
    }

    if let Some(video_stream) = probe_data.streams.iter().find(|s| s.codec_type == "video") {
        metadata.video_codec.clone_from(&video_stream.codec_name);
        metadata.pixel_format.clone_from(&video_stream.pix_fmt);
        metadata.color_space.clone_from(&video_stream.color_space);
        metadata.color_range.clone_from(&video_stream.color_range);
        metadata
            .color_primaries
            .clone_from(&video_stream.color_primaries);
        metadata.profile.clone_from(&video_stream.profile);

        if let (Some(w), Some(h)) = (video_stream.width, video_stream.height)
            && w > 0
            && h > 0
        {
            metadata.width = u32::try_from(w).ok();
            metadata.height = u32::try_from(h).ok();
            metadata.resolution = Some(format!("{w}x{h}"));
        }

        if metadata.frame_rate.is_none() {
            metadata.frame_rate = parse_frame_rate_string(video_stream.avg_frame_rate.as_deref());
        }

        if metadata.video_bitrate_kbps.is_none() {
            metadata.video_bitrate_kbps = parse_probe_bitrate(video_stream.bit_rate.as_deref());
        }
    }

    for stream in probe_data
        .streams
        .iter()
        .filter(|s| s.codec_type == "audio")
    {
        let label = stream.tags.as_ref().and_then(|t| t.title.clone());
        let language = stream.tags.as_ref().and_then(|t| t.language.clone());
        let track_bitrate = parse_probe_bitrate(stream.bit_rate.as_deref());

        metadata.audio_tracks.push(AudioTrack {
            index: stream.index,
            codec: stream
                .codec_name
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            channels: stream
                .channels
                .map_or_else(|| "?".to_string(), |c| c.to_string()),
            label,
            language,
            bitrate_kbps: track_bitrate,
            sample_rate: stream.sample_rate.clone(),
        });
    }

    for stream in probe_data
        .streams
        .iter()
        .filter(|s| s.codec_type == "subtitle")
    {
        let label = stream.tags.as_ref().and_then(|t| t.title.clone());
        let language = stream.tags.as_ref().and_then(|t| t.language.clone());

        metadata.subtitle_tracks.push(SubtitleTrack {
            index: stream.index,
            codec: stream
                .codec_name
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            language,
            label,
        });
    }

    if let Some(first_audio) = metadata.audio_tracks.first() {
        metadata.audio_codec = Some(first_audio.codec.clone());
    }

    if metadata.video_bitrate_kbps.is_none()
        && let Some(container_kbps) = parse_probe_bitrate(metadata.bitrate.as_deref())
    {
        let audio_sum: f64 = metadata
            .audio_tracks
            .iter()
            .filter_map(|track| track.bitrate_kbps)
            .sum();
        if container_kbps > audio_sum {
            metadata.video_bitrate_kbps = Some(container_kbps - audio_sum);
        }
    }

    let has_audio = !metadata.audio_tracks.is_empty();
    let has_video = metadata.video_codec.is_some();
    metadata.media_kind = if has_video {
        if !has_audio
            && (is_known_image_extension(file_path)
                || format_name_indicates_image(source_format_name.as_deref()))
        {
            "image".to_string()
        } else {
            "video".to_string()
        }
    } else {
        "audio".to_string()
    };

    if metadata.media_kind == "image" {
        metadata.duration = None;
        metadata.bitrate = None;
        metadata.frame_rate = None;
        metadata.video_bitrate_kbps = None;
    }

    metadata
}

fn is_known_image_extension(file_path: &str) -> bool {
    Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" | "avif" | "heif" | "heic"
            )
        })
}

fn format_name_indicates_image(format_name: Option<&str>) -> bool {
    format_name.is_some_and(|raw| {
        raw.split(',').map(str::trim).any(|name| {
            matches!(
                name,
                "image2"
                    | "image2pipe"
                    | "png_pipe"
                    | "jpeg_pipe"
                    | "webp_pipe"
                    | "bmp_pipe"
                    | "tiff_pipe"
                    | "ico_pipe"
                    | "apng"
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffprobe_json_args_match_probe_contract() {
        assert_eq!(
            ffprobe_json_args("/tmp/input.mp4"),
            vec![
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
                "/tmp/input.mp4"
            ]
        );
    }

    #[test]
    fn detects_known_image_extensions() {
        assert!(is_known_image_extension("/tmp/frame.png"));
        assert!(is_known_image_extension("/tmp/frame.JPG"));
        assert!(is_known_image_extension("C:\\frames\\shot.avif"));
        assert!(!is_known_image_extension("/tmp/clip.mp4"));
        assert!(!is_known_image_extension("/tmp/animation.gif"));
    }

    #[test]
    fn detects_image_format_names() {
        assert!(format_name_indicates_image(Some("image2")));
        assert!(format_name_indicates_image(Some("mov,mp4,image2")));
        assert!(format_name_indicates_image(Some("png_pipe")));
        assert!(!format_name_indicates_image(Some(
            "mov,mp4,m4a,3gp,3g2,mj2"
        )));
        assert!(!format_name_indicates_image(None));
    }

    #[test]
    fn parse_ffprobe_stdout_extracts_video_audio_subtitle_metadata() {
        let metadata = parse_ffprobe_stdout(
            "/tmp/source.mp4",
            r#"{
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "video",
                        "codec_name": "h264",
                        "width": 1920,
                        "height": 1080,
                        "bit_rate": "4800000",
                        "avg_frame_rate": "30000/1001",
                        "pix_fmt": "yuv420p",
                        "color_space": "bt709",
                        "color_range": "tv",
                        "color_primaries": "bt709",
                        "profile": "High"
                    },
                    {
                        "index": 1,
                        "codec_type": "audio",
                        "codec_name": "aac",
                        "channels": 2,
                        "bit_rate": "192000",
                        "sample_rate": "48000",
                        "tags": { "language": "eng", "title": "Main" }
                    },
                    {
                        "index": 2,
                        "codec_type": "subtitle",
                        "codec_name": "subrip",
                        "tags": { "language": "eng", "title": "Captions" }
                    }
                ],
                "format": {
                    "format_name": "mov,mp4,m4a,3gp,3g2,mj2",
                    "duration": "10.000000",
                    "bit_rate": "5000000",
                    "tags": { "title": "Demo" }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(metadata.media_kind, "video");
        assert_eq!(metadata.video_codec.as_deref(), Some("h264"));
        assert_eq!(metadata.resolution.as_deref(), Some("1920x1080"));
        assert_eq!(metadata.audio_codec.as_deref(), Some("aac"));
        assert_eq!(metadata.audio_tracks[0].label.as_deref(), Some("Main"));
        assert_eq!(
            metadata.subtitle_tracks[0].label.as_deref(),
            Some("Captions")
        );
        assert_eq!(
            metadata
                .tags
                .as_ref()
                .and_then(|tags| tags.title.as_deref()),
            Some("Demo")
        );
    }

    #[test]
    fn parse_ffprobe_stdout_clears_time_fields_for_still_images() {
        let metadata = parse_ffprobe_stdout(
            "/tmp/frame.png",
            r#"{
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "video",
                        "codec_name": "png",
                        "width": 800,
                        "height": 600,
                        "avg_frame_rate": "25/1"
                    }
                ],
                "format": {
                    "format_name": "png_pipe",
                    "duration": "0.040000",
                    "bit_rate": "100000"
                }
            }"#,
        )
        .unwrap();

        assert_eq!(metadata.media_kind, "image");
        assert_eq!(metadata.duration, None);
        assert_eq!(metadata.bitrate, None);
        assert_eq!(metadata.frame_rate, None);
        assert_eq!(metadata.video_bitrate_kbps, None);
    }
}
