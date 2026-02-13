use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

use crate::conversion::error::ConversionError;
use crate::conversion::types::{AudioTrack, FfprobeOutput, ProbeMetadata, SubtitleTrack};
use crate::conversion::utils::{parse_frame_rate_string, parse_probe_bitrate};

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

    let mut metadata = ProbeMetadata {
        duration: probe_data.format.duration,
        bitrate: probe_data.format.bit_rate,
        ..ProbeMetadata::default()
    };

    if let Some(tags) = probe_data.format.tags {
        metadata.tags = Some(tags);
    }

    if let Some(video_stream) = probe_data.streams.iter().find(|s| s.codec_type == "video") {
        metadata.video_codec = video_stream.codec_name.clone();
        metadata.pixel_format = video_stream.pix_fmt.clone();
        metadata.color_space = video_stream.color_space.clone();
        metadata.color_range = video_stream.color_range.clone();
        metadata.color_primaries = video_stream.color_primaries.clone();
        metadata.profile = video_stream.profile.clone();

        if let (Some(w), Some(h)) = (video_stream.width, video_stream.height)
            && w > 0
            && h > 0
        {
            metadata.width = Some(w as u32);
            metadata.height = Some(h as u32);
            metadata.resolution = Some(format!("{}x{}", w, h));
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
            codec: stream.codec_name.clone().unwrap_or("unknown".to_string()),
            channels: stream
                .channels
                .map(|c| c.to_string())
                .unwrap_or("?".to_string()),
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
            codec: stream.codec_name.clone().unwrap_or("unknown".to_string()),
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

    Ok(metadata)
}
