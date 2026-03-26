use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

use crate::conversion::error::ConversionError;
use crate::conversion::types::{AudioTrack, FfprobeOutput, ProbeMetadata, SubtitleTrack};
use crate::conversion::utils::{parse_frame_rate_string, parse_probe_bitrate};

fn is_known_image_extension(file_path: &str) -> bool {
    Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" | "avif" | "heif" | "heic"
            )
        })
        .unwrap_or(false)
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

#[expect(
    clippy::too_many_lines,
    reason = "ffprobe parsing keeps track extraction in one pass over streams"
)]
pub async fn probe_media_file(
    app: &AppHandle,
    file_path: &str,
) -> Result<ProbeMetadata, ConversionError> {
    let args = vec![
        "-v".to_string(),
        "quiet".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        file_path.to_string(),
    ];

    let output = app
        .shell()
        .sidecar("ffprobe")
        .map_err(|e| ConversionError::Shell(e.to_string()))?
        .args(args)
        .output()
        .await
        .map_err(|e| ConversionError::Shell(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ConversionError::Probe(stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let probe_data: FfprobeOutput = serde_json::from_str(&stdout)?;

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

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::{format_name_indicates_image, is_known_image_extension};

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
}
