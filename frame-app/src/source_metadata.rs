//! Source metadata state and ffprobe integration for the GPUI app.

use std::{collections::HashMap, process::Command};

use frame_core::{
    error::ConversionError,
    probe::{ffprobe_json_args, parse_ffprobe_stdout},
    types::{FfprobeTags, ProbeMetadata},
};

use crate::{
    file_queue::FileQueue,
    runtime_binaries::ffprobe_executable,
    settings::{AudioTrack, SourceKind, SourceMetadata, SourceTags, SubtitleTrack},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MetadataStatus {
    #[default]
    Idle,
    Loading,
    Ready,
    Error,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceMetadataEntry {
    pub status: MetadataStatus,
    pub metadata: Option<SourceMetadata>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceMetadataStore {
    entries: HashMap<String, SourceMetadataEntry>,
}

impl SourceMetadataStore {
    #[must_use]
    pub fn entry_for(&self, id: &str) -> SourceMetadataEntry {
        self.entries.get(id).cloned().unwrap_or_default()
    }

    #[must_use]
    pub fn selected_entry(&self, queue: &FileQueue) -> SourceMetadataEntry {
        queue
            .selected_file_id()
            .map_or_else(SourceMetadataEntry::default, |id| self.entry_for(id))
    }

    #[must_use]
    pub fn metadata_for(&self, id: &str) -> Option<&SourceMetadata> {
        self.entries
            .get(id)
            .and_then(|entry| entry.metadata.as_ref())
    }

    pub fn mark_loading(&mut self, id: impl Into<String>) {
        self.entries.insert(
            id.into(),
            SourceMetadataEntry {
                status: MetadataStatus::Loading,
                metadata: None,
                error: None,
            },
        );
    }

    pub fn mark_ready(&mut self, id: impl Into<String>, metadata: SourceMetadata) {
        self.entries.insert(
            id.into(),
            SourceMetadataEntry {
                status: MetadataStatus::Ready,
                metadata: Some(metadata),
                error: None,
            },
        );
    }

    pub fn mark_error(&mut self, id: impl Into<String>, error: impl Into<String>) {
        self.entries.insert(
            id.into(),
            SourceMetadataEntry {
                status: MetadataStatus::Error,
                metadata: None,
                error: Some(error.into()),
            },
        );
    }

    pub fn remove(&mut self, id: &str) {
        self.entries.remove(id);
    }
}

#[must_use]
pub fn source_metadata_from_probe(probe: ProbeMetadata) -> SourceMetadata {
    SourceMetadata {
        media_kind: source_kind_from_probe(&probe.media_kind),
        duration: probe.duration,
        bitrate: probe.bitrate,
        video_codec: probe.video_codec,
        audio_codec: probe.audio_codec,
        resolution: probe.resolution,
        frame_rate: probe.frame_rate,
        width: probe.width,
        height: probe.height,
        video_bitrate_kbps: probe.video_bitrate_kbps,
        audio_tracks: probe
            .audio_tracks
            .into_iter()
            .map(|track| AudioTrack {
                index: track.index,
                codec: track.codec,
                channels: Some(track.channels),
                language: track.language,
                label: track.label,
                bitrate_kbps: track.bitrate_kbps,
                sample_rate: track.sample_rate,
            })
            .collect(),
        subtitle_tracks: probe
            .subtitle_tracks
            .into_iter()
            .map(|track| SubtitleTrack {
                index: track.index,
                codec: track.codec,
                language: track.language,
                label: track.label,
            })
            .collect(),
        tags: probe.tags.map(source_tags_from_probe),
        pixel_format: probe.pixel_format,
        color_space: probe.color_space,
        color_range: probe.color_range,
        color_primaries: probe.color_primaries,
        profile: probe.profile,
    }
}

/// Probes source metadata with the bundled ffprobe executable.
///
/// # Errors
///
/// Returns an error when ffprobe cannot be executed, exits unsuccessfully, or
/// emits metadata that cannot be parsed.
pub fn probe_source_metadata(file_path: &str) -> Result<SourceMetadata, ConversionError> {
    let executable = ffprobe_executable();
    probe_source_metadata_with_executable(file_path, &executable)
}

/// Probes source metadata with a specific ffprobe executable.
///
/// # Errors
///
/// Returns an error when the executable cannot be launched, exits with a
/// non-zero status, or emits invalid probe JSON.
pub fn probe_source_metadata_with_executable(
    file_path: &str,
    executable: &str,
) -> Result<SourceMetadata, ConversionError> {
    let output = Command::new(executable)
        .args(ffprobe_json_args(file_path))
        .output()
        .map_err(ConversionError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = if stderr.trim().is_empty() {
            format!("ffprobe exited with status {}", output.status)
        } else {
            stderr.trim().to_string()
        };
        return Err(ConversionError::Probe(message));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ffprobe_stdout(file_path, stdout).map(source_metadata_from_probe)
}

fn source_kind_from_probe(kind: &str) -> Option<SourceKind> {
    match kind {
        "video" => Some(SourceKind::Video),
        "audio" => Some(SourceKind::Audio),
        "image" => Some(SourceKind::Image),
        _ => None,
    }
}

fn source_tags_from_probe(tags: FfprobeTags) -> SourceTags {
    SourceTags {
        title: tags.title,
        artist: tags.artist,
        album: tags.album,
        genre: tags.genre,
        date: tags.date,
        comment: tags.comment.or(tags.description_upper),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_core::types::{
        AudioTrack as ProbeAudioTrack, ProbeMetadata, SubtitleTrack as ProbeSubtitleTrack,
    };

    mod source_metadata_from_probe {
        use super::*;

        #[test]
        fn maps_video_metadata_fields() {
            let metadata = source_metadata_from_probe(ProbeMetadata {
                media_kind: "video".to_string(),
                duration: Some("10.000000".to_string()),
                video_codec: Some("h264".to_string()),
                resolution: Some("1920x1080".to_string()),
                frame_rate: Some(29.97),
                width: Some(1920),
                height: Some(1080),
                pixel_format: Some("yuv420p".to_string()),
                ..ProbeMetadata::default()
            });

            assert_eq!(metadata.media_kind, Some(SourceKind::Video));
            assert_eq!(metadata.video_codec.as_deref(), Some("h264"));
            assert_eq!(metadata.resolution.as_deref(), Some("1920x1080"));
        }

        #[test]
        fn maps_audio_tracks() {
            let metadata = source_metadata_from_probe(ProbeMetadata {
                media_kind: "audio".to_string(),
                audio_tracks: vec![ProbeAudioTrack {
                    index: 1,
                    codec: "aac".to_string(),
                    channels: "2".to_string(),
                    language: Some("eng".to_string()),
                    label: Some("Main".to_string()),
                    bitrate_kbps: Some(192.0),
                    sample_rate: Some("48000".to_string()),
                }],
                ..ProbeMetadata::default()
            });

            assert_eq!(metadata.audio_tracks[0].label.as_deref(), Some("Main"));
            assert_eq!(metadata.audio_tracks[0].channels.as_deref(), Some("2"));
        }

        #[test]
        fn maps_subtitle_tracks() {
            let metadata = source_metadata_from_probe(ProbeMetadata {
                subtitle_tracks: vec![ProbeSubtitleTrack {
                    index: 2,
                    codec: "subrip".to_string(),
                    language: Some("eng".to_string()),
                    label: Some("Captions".to_string()),
                }],
                ..ProbeMetadata::default()
            });

            assert_eq!(metadata.subtitle_tracks[0].codec, "subrip");
            assert_eq!(
                metadata.subtitle_tracks[0].label.as_deref(),
                Some("Captions")
            );
        }

        #[test]
        fn maps_format_tags_for_metadata_placeholders() {
            let metadata = source_metadata_from_probe(ProbeMetadata {
                tags: Some(FfprobeTags {
                    title: Some("Original Title".to_string()),
                    artist: Some("Frame".to_string()),
                    comment: Some("Original Comment".to_string()),
                    ..FfprobeTags::default()
                }),
                ..ProbeMetadata::default()
            });

            let tags = metadata.tags.as_ref().expect("tags should be mapped");
            assert_eq!(tags.title.as_deref(), Some("Original Title"));
            assert_eq!(tags.artist.as_deref(), Some("Frame"));
            assert_eq!(tags.comment.as_deref(), Some("Original Comment"));
        }
    }

    mod source_metadata_store {
        use super::*;
        use crate::file_queue::FileItem;

        #[test]
        fn selected_entry_returns_idle_when_queue_is_empty() {
            let store = SourceMetadataStore::default();

            assert_eq!(
                store.selected_entry(&FileQueue::new()).status,
                MetadataStatus::Idle
            );
        }

        #[test]
        fn mark_loading_clears_previous_error() {
            let mut store = SourceMetadataStore::default();
            store.mark_error("file-1", "failed");

            store.mark_loading("file-1");

            let entry = store.entry_for("file-1");
            assert_eq!(entry.status, MetadataStatus::Loading);
            assert_eq!(entry.error, None);
        }

        #[test]
        fn mark_ready_stores_metadata() {
            let mut store = SourceMetadataStore::default();
            let metadata = SourceMetadata {
                video_codec: Some("h264".to_string()),
                ..SourceMetadata::default()
            };

            store.mark_ready("file-1", metadata);

            assert_eq!(
                store
                    .metadata_for("file-1")
                    .and_then(|metadata| metadata.video_codec.as_deref()),
                Some("h264")
            );
        }

        #[test]
        fn selected_entry_reads_selected_file_metadata() {
            let mut queue = FileQueue::new();
            queue.add_file(FileItem::from_path("file-1", "/tmp/one.mp4", 1));
            let mut store = SourceMetadataStore::default();
            store.mark_error("file-1", "probe failed");

            let entry = store.selected_entry(&queue);

            assert_eq!(entry.status, MetadataStatus::Error);
            assert_eq!(entry.error.as_deref(), Some("probe failed"));
        }

        #[test]
        fn remove_deletes_entry() {
            let mut store = SourceMetadataStore::default();
            store.mark_error("file-1", "probe failed");

            store.remove("file-1");

            assert_eq!(store.entry_for("file-1").status, MetadataStatus::Idle);
        }
    }
}
