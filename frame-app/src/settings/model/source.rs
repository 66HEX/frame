use crate::settings::model::metadata::MetadataField;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceInfoRow {
    pub label: &'static str,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceTrackSection {
    pub label: String,
    pub rows: Vec<SourceInfoRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceInfoSection {
    Rows {
        title: &'static str,
        rows: Vec<SourceInfoRow>,
    },
    Tracks {
        title: &'static str,
        tracks: Vec<SourceTrackSection>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceKind {
    Video,
    Audio,
    Image,
}

impl SourceKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Image => "Image",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioTrack {
    pub index: u32,
    pub codec: String,
    pub codec_tag: Option<u32>,
    pub channels: Option<String>,
    pub channel_layout: Option<String>,
    pub language: Option<String>,
    pub label: Option<String>,
    pub bitrate_kbps: Option<f64>,
    pub sample_rate: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubtitleTrack {
    pub index: u32,
    pub codec: String,
    pub language: Option<String>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
}

impl SourceTags {
    #[must_use]
    pub fn value(&self, field: MetadataField) -> Option<&str> {
        match field {
            MetadataField::Title => self.title.as_deref(),
            MetadataField::Artist => self.artist.as_deref(),
            MetadataField::Album => self.album.as_deref(),
            MetadataField::Genre => self.genre.as_deref(),
            MetadataField::Date => self.date.as_deref(),
            MetadataField::Comment => self.comment.as_deref(),
            MetadataField::ServiceName | MetadataField::ServiceProvider => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceMetadata {
    pub source_format: Option<String>,
    pub media_kind: Option<SourceKind>,
    pub duration: Option<String>,
    pub bitrate: Option<String>,
    pub video_codec: Option<String>,
    pub video_stream_index: Option<u32>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    pub frame_rate: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub video_bitrate_kbps: Option<f64>,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub tags: Option<SourceTags>,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
    pub color_primaries: Option<String>,
    pub profile: Option<String>,
    pub transport_stream: Option<frame_core::types::TransportStreamMetadata>,
}

impl SourceMetadata {
    #[must_use]
    pub fn source_kind(&self) -> SourceKind {
        self.media_kind.unwrap_or_else(|| {
            if self.video_codec.is_some() {
                SourceKind::Video
            } else {
                SourceKind::Audio
            }
        })
    }
}
