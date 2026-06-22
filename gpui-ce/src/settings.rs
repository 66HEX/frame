//! Settings panel state and visibility rules ported from the Svelte inspector.

use frame_core::media_rules;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsTab {
    Source,
    Output,
    Video,
    Images,
    Audio,
    Subtitles,
    Metadata,
    Presets,
}

impl SettingsTab {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Output => "Output",
            Self::Video => "Video",
            Self::Images => "Images",
            Self::Audio => "Audio",
            Self::Subtitles => "Subtitles",
            Self::Metadata => "Metadata",
            Self::Presets => "Presets",
        }
    }

    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Output => "output",
            Self::Video => "video",
            Self::Images => "images",
            Self::Audio => "audio",
            Self::Subtitles => "subtitles",
            Self::Metadata => "metadata",
            Self::Presets => "presets",
        }
    }
}

pub const ALL_SETTINGS_TABS: [SettingsTab; 8] = [
    SettingsTab::Source,
    SettingsTab::Output,
    SettingsTab::Video,
    SettingsTab::Images,
    SettingsTab::Audio,
    SettingsTab::Subtitles,
    SettingsTab::Metadata,
    SettingsTab::Presets,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ProcessingMode {
    #[default]
    Reencode,
    Copy,
}

impl ProcessingMode {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Reencode => "reencode",
            Self::Copy => "copy",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reencode => "Re-encode",
            Self::Copy => "Cut / Stream Copy",
        }
    }

    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::Reencode => {
                "Decodes and encodes media so all filters and codec settings are available."
            }
            Self::Copy => {
                "Fast trim/remux without re-encoding. Cut precision depends on keyframes."
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionConfig {
    pub processing_mode: ProcessingMode,
    pub container: String,
    pub audio_codec: String,
    pub selected_audio_tracks: Vec<u32>,
    pub selected_subtitle_tracks: Vec<u32>,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            processing_mode: ProcessingMode::Reencode,
            container: "mp4".to_string(),
            audio_codec: media_rules::default_audio_codec_for_container("mp4").to_string(),
            selected_audio_tracks: Vec::new(),
            selected_subtitle_tracks: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputModeOption {
    pub mode: ProcessingMode,
    pub label: &'static str,
    pub hint: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputContainerOption {
    pub container: String,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceInfoRow {
    pub label: &'static str,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceTrackSection {
    pub label: String,
    pub rows: Vec<SourceInfoRow>,
}

#[derive(Clone, Debug, PartialEq)]
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
    pub channels: Option<String>,
    pub language: Option<String>,
    pub label: Option<String>,
    pub bitrate_kbps: Option<f64>,
    pub sample_rate: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubtitleTrack {
    pub index: u32,
    pub codec: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceMetadata {
    pub media_kind: Option<SourceKind>,
    pub duration: Option<String>,
    pub bitrate: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    pub frame_rate: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub video_bitrate_kbps: Option<f64>,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
    pub color_primaries: Option<String>,
    pub profile: Option<String>,
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

#[must_use]
pub fn source_kind_for(metadata: Option<&SourceMetadata>) -> SourceKind {
    metadata.map_or(SourceKind::Video, SourceMetadata::source_kind)
}

#[must_use]
pub fn is_audio_only_container(container: &str) -> bool {
    media_rules::is_audio_only_container(container)
}

#[must_use]
pub fn is_video_only_container(container: &str) -> bool {
    media_rules::is_video_only_container(container)
}

#[must_use]
pub fn is_image_container(container: &str) -> bool {
    media_rules::is_image_container(container)
}

#[must_use]
pub fn is_gif_container(container: &str) -> bool {
    media_rules::is_gif_container(container)
}

#[must_use]
pub fn container_supports_audio(container: &str) -> bool {
    media_rules::container_supports_audio(container)
}

#[must_use]
pub fn container_supports_subtitles(container: &str) -> bool {
    media_rules::container_supports_subtitles(container)
}

#[must_use]
pub fn is_audio_codec_allowed_for_container(container: &str, codec: &str) -> bool {
    media_rules::is_audio_codec_allowed(container, codec)
}

#[must_use]
pub fn is_audio_stream_codec_allowed_for_container(container: &str, codec: &str) -> bool {
    media_rules::is_audio_stream_codec_allowed(container, codec)
}

#[must_use]
pub fn is_video_stream_codec_allowed_for_container(container: &str, codec: &str) -> bool {
    media_rules::is_video_stream_codec_allowed(container, codec)
}

#[must_use]
pub fn is_subtitle_codec_allowed_for_container(container: &str, codec: &str) -> bool {
    media_rules::is_subtitle_codec_allowed(container, codec)
}

#[must_use]
pub fn default_audio_codec_for_container(container: &str) -> &str {
    media_rules::default_audio_codec_for_container(container)
}

#[must_use]
pub fn source_info_sections(metadata: &SourceMetadata) -> Vec<SourceInfoSection> {
    let source_kind = metadata.source_kind();
    let is_image = source_kind == SourceKind::Image;
    let mut sections = Vec::new();

    if is_image {
        sections.push(SourceInfoSection::Rows {
            title: "FILE INFORMATION",
            rows: source_image_rows(metadata),
        });
    } else if has_duration_value(metadata.duration.as_deref())
        || has_bitrate_value(metadata.bitrate.as_deref())
    {
        sections.push(SourceInfoSection::Rows {
            title: "FILE INFORMATION",
            rows: source_file_rows(metadata),
        });
    }

    if metadata.video_codec.is_some() && !is_image {
        sections.push(SourceInfoSection::Rows {
            title: "VIDEO STREAM",
            rows: source_video_rows(metadata),
        });
    }

    if !metadata.audio_tracks.is_empty() {
        sections.push(SourceInfoSection::Tracks {
            title: "AUDIO STREAM",
            tracks: source_audio_track_sections(&metadata.audio_tracks),
        });
    }

    sections
}

#[must_use]
pub fn display_source_value(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "—".to_string();
    };
    let value = value.trim();

    if value.is_empty() {
        "—".to_string()
    } else {
        value.to_string()
    }
}

#[must_use]
pub fn format_source_duration(raw: Option<&str>) -> String {
    let Some(raw) = raw else {
        return "—".to_string();
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return "—".to_string();
    }

    if let Some(seconds) = parse_colon_duration(raw).or_else(|| raw.parse::<f64>().ok()) {
        return format_seconds_as_hms(seconds);
    }

    raw.to_string()
}

#[must_use]
pub fn format_source_resolution(metadata: &SourceMetadata) -> String {
    if let (Some(width), Some(height)) = (metadata.width, metadata.height)
        && width > 0
        && height > 0
    {
        return format!("{width}×{height}");
    }

    display_source_value(metadata.resolution.as_deref())
}

#[must_use]
pub fn format_source_frame_rate(value: Option<f64>) -> String {
    let Some(value) = value else {
        return "—".to_string();
    };
    if value <= 0.0 || !value.is_finite() {
        return "—".to_string();
    }

    let formatted = if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        trim_decimal_zeros(&format!("{value:.3}"))
    };
    format!("{formatted} fps")
}

#[must_use]
pub fn format_source_bitrate_kbps(value: Option<f64>) -> String {
    let Some(value) = value else {
        return "—".to_string();
    };
    if value <= 0.0 || !value.is_finite() {
        return "—".to_string();
    }
    if value >= 1000.0 {
        return format!(
            "{} Mb/s",
            trim_decimal_zeros(&format!("{:.2}", value / 1000.0))
        );
    }

    format!("{:.0} kb/s", value.round())
}

#[must_use]
pub fn format_source_container_bitrate(raw: Option<&str>) -> String {
    let Some(raw) = raw else {
        return "—".to_string();
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return "—".to_string();
    }

    let Ok(bits_per_second) = raw.parse::<f64>() else {
        return raw.to_string();
    };
    if bits_per_second <= 0.0 || !bits_per_second.is_finite() {
        return raw.to_string();
    }
    if bits_per_second >= 1_000_000.0 {
        return format!(
            "{} Mb/s",
            trim_decimal_zeros(&format!("{:.2}", bits_per_second / 1_000_000.0))
        );
    }

    format!("{:.0} kb/s", (bits_per_second / 1000.0).round())
}

#[must_use]
pub fn format_source_hz(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "—".to_string();
    };
    let value = value.trim();
    if value.is_empty() {
        return "—".to_string();
    }

    let Ok(hz) = value.parse::<u32>() else {
        return value.to_string();
    };
    if hz >= 1000 {
        return format!(
            "{} kHz",
            trim_decimal_zeros(&format!("{:.1}", f64::from(hz) / 1000.0))
        );
    }

    format!("{hz} Hz")
}

#[must_use]
pub fn output_processing_mode_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> [OutputModeOption; 2] {
    let is_source_image = source_kind_for(metadata) == SourceKind::Image;
    [
        output_mode_option(ProcessingMode::Reencode, config, disabled),
        output_mode_option(ProcessingMode::Copy, config, disabled || is_source_image),
    ]
}

#[must_use]
pub fn visible_output_containers(metadata: Option<&SourceMetadata>) -> Vec<String> {
    let is_source_image = source_kind_for(metadata) == SourceKind::Image;

    media_rules::all_containers()
        .iter()
        .filter(|container| {
            if is_source_image {
                is_image_container(container) || is_gif_container(container)
            } else {
                !is_image_container(container)
            }
        })
        .cloned()
        .collect()
}

#[must_use]
pub fn output_container_options(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    disabled: bool,
) -> Vec<OutputContainerOption> {
    visible_output_containers(metadata)
        .into_iter()
        .map(|container| {
            let disabled_reason =
                output_container_disabled_reason(config, metadata, &container, disabled);
            OutputContainerOption {
                is_selected: config.container.eq_ignore_ascii_case(&container),
                is_disabled: disabled_reason.is_some(),
                disabled_reason,
                container,
            }
        })
        .collect()
}

#[must_use]
pub fn is_container_compatible_for_stream_copy(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    container: &str,
) -> bool {
    if config.processing_mode != ProcessingMode::Copy {
        return true;
    }
    if source_kind_for(metadata) == SourceKind::Image {
        return false;
    }
    if is_image_container(container) || is_gif_container(container) {
        return false;
    }

    let Some(metadata) = metadata else {
        return true;
    };

    let selected_audio_codecs = selected_audio_codecs(config, metadata);
    if is_audio_only_container(container) {
        return !selected_audio_codecs.is_empty()
            && selected_audio_codecs
                .iter()
                .all(|codec| is_audio_stream_codec_allowed_for_container(container, codec));
    }

    let Some(video_codec) = metadata.video_codec.as_deref() else {
        return false;
    };
    if !is_video_stream_codec_allowed_for_container(container, video_codec) {
        return false;
    }

    let audio_codecs_allowed = selected_audio_codecs
        .iter()
        .all(|codec| is_audio_stream_codec_allowed_for_container(container, codec));
    let subtitle_codecs_allowed = selected_subtitle_codecs(config, metadata)
        .iter()
        .all(|codec| is_subtitle_codec_allowed_for_container(container, codec));

    audio_codecs_allowed && subtitle_codecs_allowed
}

#[must_use]
pub fn sanitize_output_name(value: &str) -> String {
    let candidate = value.rsplit(['/', '\\']).next().unwrap_or_default().trim();

    if candidate == "." || candidate == ".." {
        String::new()
    } else {
        candidate.to_string()
    }
}

pub fn apply_processing_mode(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
    mode: ProcessingMode,
) -> bool {
    if mode == ProcessingMode::Copy && source_kind_for(metadata) == SourceKind::Image {
        return false;
    }

    let changed = config.processing_mode != mode;
    config.processing_mode = mode;
    changed | normalize_output_config(config, metadata)
}

pub fn apply_output_container(config: &mut ConversionConfig, container: &str) -> bool {
    let changed = !config.container.eq_ignore_ascii_case(container);
    config.container = container.to_ascii_lowercase();

    if config.processing_mode != ProcessingMode::Copy
        && container_supports_audio(&config.container)
        && !is_audio_codec_allowed_for_container(&config.container, &config.audio_codec)
    {
        config.audio_codec = default_audio_codec_for_container(&config.container).to_string();
        return true;
    }

    changed
}

pub fn normalize_output_config(
    config: &mut ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> bool {
    let before = config.clone();
    let source_kind = source_kind_for(metadata);

    if source_kind == SourceKind::Audio && !is_audio_only_container(&config.container) {
        config.container = "mp3".to_string();
    }

    if source_kind == SourceKind::Image
        && !is_image_container(&config.container)
        && !is_gif_container(&config.container)
    {
        config.container = "png".to_string();
    }

    if (source_kind == SourceKind::Image || is_gif_container(&config.container))
        && config.processing_mode == ProcessingMode::Copy
    {
        config.processing_mode = ProcessingMode::Reencode;
    }

    if config.processing_mode != ProcessingMode::Copy
        && container_supports_audio(&config.container)
        && !is_audio_codec_allowed_for_container(&config.container, &config.audio_codec)
    {
        config.audio_codec = default_audio_codec_for_container(&config.container).to_string();
    }

    before != *config
}

#[must_use]
pub fn visible_settings_tabs(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> Vec<SettingsTab> {
    let source_kind = source_kind_for(metadata);
    let is_source_audio_only = source_kind == SourceKind::Audio;
    let is_source_image = source_kind == SourceKind::Image;
    let is_copy_mode = config.processing_mode == ProcessingMode::Copy;
    let is_audio_container = is_audio_only_container(&config.container);
    let supports_audio = container_supports_audio(&config.container) && !is_source_image;
    let supports_subtitles = !is_source_audio_only
        && !is_source_image
        && container_supports_subtitles(&config.container);
    let supports_video_tab =
        !is_source_audio_only && !is_source_image && !is_audio_container && !is_copy_mode;
    let supports_images_tab = is_source_image && !is_audio_container && !is_copy_mode;

    ALL_SETTINGS_TABS
        .into_iter()
        .filter(|tab| match tab {
            SettingsTab::Video => supports_video_tab,
            SettingsTab::Images => supports_images_tab,
            SettingsTab::Audio => supports_audio,
            SettingsTab::Subtitles => supports_subtitles,
            SettingsTab::Source
            | SettingsTab::Output
            | SettingsTab::Metadata
            | SettingsTab::Presets => true,
        })
        .collect()
}

#[must_use]
pub fn resolve_active_settings_tab(
    active_tab: SettingsTab,
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
) -> SettingsTab {
    if visible_settings_tabs(config, metadata).contains(&active_tab) {
        active_tab
    } else {
        SettingsTab::Output
    }
}

fn source_image_rows(metadata: &SourceMetadata) -> Vec<SourceInfoRow> {
    let mut rows = Vec::new();
    push_optional_row(&mut rows, "IMAGE CODEC", metadata.video_codec.as_deref());
    rows.push(SourceInfoRow {
        label: "DIMENSIONS",
        value: format_source_resolution(metadata),
    });
    push_optional_row(&mut rows, "PIXEL FORMAT", metadata.pixel_format.as_deref());
    push_optional_row(&mut rows, "PROFILE", metadata.profile.as_deref());
    push_optional_row(&mut rows, "COLOR SPACE", metadata.color_space.as_deref());
    push_optional_row(&mut rows, "COLOR RANGE", metadata.color_range.as_deref());
    push_optional_row(
        &mut rows,
        "COLOR PRIMARIES",
        metadata.color_primaries.as_deref(),
    );
    rows
}

fn source_file_rows(metadata: &SourceMetadata) -> Vec<SourceInfoRow> {
    let mut rows = Vec::new();
    if has_duration_value(metadata.duration.as_deref()) {
        rows.push(SourceInfoRow {
            label: "DURATION",
            value: format_source_duration(metadata.duration.as_deref()),
        });
    }
    if has_bitrate_value(metadata.bitrate.as_deref()) {
        rows.push(SourceInfoRow {
            label: "CONTAINER BITRATE",
            value: format_source_container_bitrate(metadata.bitrate.as_deref()),
        });
    }
    rows
}

fn source_video_rows(metadata: &SourceMetadata) -> Vec<SourceInfoRow> {
    let mut rows = vec![SourceInfoRow {
        label: "VIDEO CODEC",
        value: display_source_value(metadata.video_codec.as_deref()),
    }];
    push_optional_row(&mut rows, "PROFILE", metadata.profile.as_deref());
    rows.push(SourceInfoRow {
        label: "DIMENSIONS",
        value: format_source_resolution(metadata),
    });
    if metadata
        .frame_rate
        .is_some_and(|frame_rate| frame_rate > 0.0)
    {
        rows.push(SourceInfoRow {
            label: "FRAME RATE",
            value: format_source_frame_rate(metadata.frame_rate),
        });
    }
    push_optional_row(&mut rows, "PIXEL FORMAT", metadata.pixel_format.as_deref());
    push_optional_row(&mut rows, "COLOR SPACE", metadata.color_space.as_deref());
    push_optional_row(&mut rows, "COLOR RANGE", metadata.color_range.as_deref());
    push_optional_row(
        &mut rows,
        "COLOR PRIMARIES",
        metadata.color_primaries.as_deref(),
    );
    if metadata
        .video_bitrate_kbps
        .is_some_and(|bitrate| bitrate > 0.0)
    {
        rows.push(SourceInfoRow {
            label: "VIDEO BITRATE",
            value: format_source_bitrate_kbps(metadata.video_bitrate_kbps),
        });
    }
    rows
}

fn source_audio_track_sections(tracks: &[AudioTrack]) -> Vec<SourceTrackSection> {
    tracks
        .iter()
        .enumerate()
        .map(|(index, track)| SourceTrackSection {
            label: format!("Track #{}", index + 1),
            rows: source_audio_track_rows(track),
        })
        .collect()
}

fn source_audio_track_rows(track: &AudioTrack) -> Vec<SourceInfoRow> {
    let mut rows = vec![
        SourceInfoRow {
            label: "CODEC",
            value: display_source_value(Some(&track.codec)),
        },
        SourceInfoRow {
            label: "CHANNELS",
            value: display_source_value(track.channels.as_deref()),
        },
    ];

    if track.sample_rate.is_some() {
        rows.push(SourceInfoRow {
            label: "SAMPLE RATE",
            value: format_source_hz(track.sample_rate.as_deref()),
        });
    }
    if track.bitrate_kbps.is_some() {
        rows.push(SourceInfoRow {
            label: "BITRATE",
            value: format_source_bitrate_kbps(track.bitrate_kbps),
        });
    }
    push_optional_row(&mut rows, "LANGUAGE", track.language.as_deref());
    rows
}

fn push_optional_row(rows: &mut Vec<SourceInfoRow>, label: &'static str, value: Option<&str>) {
    if value.is_some_and(|value| !value.trim().is_empty()) {
        rows.push(SourceInfoRow {
            label,
            value: display_source_value(value),
        });
    }
}

fn has_duration_value(raw: Option<&str>) -> bool {
    raw.is_some_and(|raw| {
        let raw = raw.trim();
        !raw.is_empty() && !raw.eq_ignore_ascii_case("n/a")
    })
}

fn has_bitrate_value(raw: Option<&str>) -> bool {
    raw.is_some_and(|raw| {
        let raw = raw.trim();
        if raw.is_empty() || raw.eq_ignore_ascii_case("n/a") {
            return false;
        }

        raw.parse::<f64>().map_or(true, |value| value > 0.0)
    })
}

fn parse_colon_duration(raw: &str) -> Option<f64> {
    let mut parts = raw.split(':');
    let hours = parts.next()?.parse::<u32>().ok()?;
    let minutes = parts.next()?.parse::<u32>().ok()?;
    let seconds = parts.next()?;
    if parts.next().is_some() {
        return None;
    }

    let seconds = seconds.parse::<f64>().ok()?;
    Some(f64::from(hours) * 3600.0 + f64::from(minutes) * 60.0 + seconds)
}

fn format_seconds_as_hms(seconds: f64) -> String {
    let seconds = seconds.floor();
    let hours = (seconds / 3600.0).floor();
    let minutes = ((seconds % 3600.0) / 60.0).floor();
    let seconds = (seconds % 60.0).floor();

    format!("{hours:02.0}:{minutes:02.0}:{seconds:02.0}")
}

fn trim_decimal_zeros(value: &str) -> String {
    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn output_mode_option(
    mode: ProcessingMode,
    config: &ConversionConfig,
    is_disabled: bool,
) -> OutputModeOption {
    OutputModeOption {
        mode,
        label: mode.label(),
        hint: mode.hint(),
        is_selected: config.processing_mode == mode,
        is_disabled,
    }
}

fn output_container_disabled_reason(
    config: &ConversionConfig,
    metadata: Option<&SourceMetadata>,
    container: &str,
    disabled: bool,
) -> Option<&'static str> {
    let source_kind = source_kind_for(metadata);

    if disabled {
        return Some("Locked");
    }
    if source_kind == SourceKind::Audio && !is_audio_only_container(container) {
        return Some("Video container unavailable for audio sources");
    }
    if source_kind == SourceKind::Image && is_audio_only_container(container) {
        return Some("Audio container unavailable for image sources");
    }
    if !is_container_compatible_for_stream_copy(config, metadata, container) {
        return Some("Incompatible container");
    }

    None
}

fn selected_audio_codecs<'a>(
    config: &ConversionConfig,
    metadata: &'a SourceMetadata,
) -> Vec<&'a str> {
    if metadata.audio_tracks.is_empty() {
        return Vec::new();
    }
    if config.selected_audio_tracks.is_empty() {
        return metadata
            .audio_tracks
            .iter()
            .map(|track| track.codec.as_str())
            .collect();
    }

    metadata
        .audio_tracks
        .iter()
        .filter(|track| config.selected_audio_tracks.contains(&track.index))
        .map(|track| track.codec.as_str())
        .collect()
}

fn selected_subtitle_codecs<'a>(
    config: &ConversionConfig,
    metadata: &'a SourceMetadata,
) -> Vec<&'a str> {
    if metadata.subtitle_tracks.is_empty() {
        return Vec::new();
    }
    if config.selected_subtitle_tracks.is_empty() {
        return metadata
            .subtitle_tracks
            .iter()
            .map(|track| track.codec.as_str())
            .collect();
    }

    metadata
        .subtitle_tracks
        .iter()
        .filter(|track| config.selected_subtitle_tracks.contains(&track.index))
        .map(|track| track.codec.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab_ids(tabs: Vec<SettingsTab>) -> Vec<&'static str> {
        tabs.into_iter().map(SettingsTab::id).collect()
    }

    mod source_metadata {
        use super::*;

        #[test]
        fn source_kind_falls_back_to_audio_when_metadata_has_no_video_codec() {
            let metadata = SourceMetadata::default();

            assert_eq!(metadata.source_kind(), SourceKind::Audio);
        }

        #[test]
        fn source_kind_defaults_to_video_when_metadata_is_missing() {
            assert_eq!(source_kind_for(None), SourceKind::Video);
        }
    }

    mod media_rules {
        use super::*;

        #[test]
        fn mp4_supports_audio_and_subtitles_like_original_rules() {
            assert!(container_supports_audio("mp4"));
            assert!(container_supports_subtitles("mp4"));
        }

        #[test]
        fn image_containers_do_not_support_audio_or_subtitles() {
            assert!(!container_supports_audio("png"));
            assert!(!container_supports_subtitles("png"));
        }

        #[test]
        fn mp4_rejects_flac_reencode_audio_like_original_rules() {
            assert!(!is_audio_codec_allowed_for_container("mp4", "flac"));
        }

        #[test]
        fn mov_accepts_any_audio_codec_like_original_rules() {
            assert!(is_audio_codec_allowed_for_container("mov", "flac"));
        }

        #[test]
        fn webm_default_audio_codec_matches_shared_rules() {
            assert_eq!(default_audio_codec_for_container("webm"), "libopus");
        }
    }

    mod output_name {
        use super::*;

        #[test]
        fn sanitize_output_name_keeps_only_last_path_segment() {
            assert_eq!(sanitize_output_name("/tmp/render/final.mp4"), "final.mp4");
        }

        #[test]
        fn sanitize_output_name_handles_windows_separators() {
            assert_eq!(sanitize_output_name("C:\\media\\final.mov"), "final.mov");
        }

        #[test]
        fn sanitize_output_name_rejects_dot_segments() {
            assert_eq!(sanitize_output_name(".."), "");
        }
    }

    mod output_options {
        use super::*;

        fn audio_metadata(codec: &str) -> SourceMetadata {
            SourceMetadata {
                media_kind: Some(SourceKind::Audio),
                audio_tracks: vec![AudioTrack {
                    index: 0,
                    codec: codec.to_string(),
                    ..AudioTrack::default()
                }],
                ..SourceMetadata::default()
            }
        }

        fn image_metadata() -> SourceMetadata {
            SourceMetadata {
                media_kind: Some(SourceKind::Image),
                video_codec: Some("png".to_string()),
                ..SourceMetadata::default()
            }
        }

        fn video_metadata() -> SourceMetadata {
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                video_codec: Some("h264".to_string()),
                audio_tracks: vec![AudioTrack {
                    index: 1,
                    codec: "aac".to_string(),
                    ..AudioTrack::default()
                }],
                subtitle_tracks: vec![SubtitleTrack {
                    index: 2,
                    codec: "subrip".to_string(),
                }],
                ..SourceMetadata::default()
            }
        }

        #[test]
        fn visible_output_containers_for_video_exclude_image_formats() {
            assert_eq!(
                visible_output_containers(None),
                vec![
                    "mp4", "mkv", "webm", "mov", "gif", "mp3", "m4a", "wav", "flac"
                ]
            );
        }

        #[test]
        fn visible_output_containers_for_images_match_original_image_and_gif_set() {
            assert_eq!(
                visible_output_containers(Some(&image_metadata())),
                vec!["gif", "png", "jpg", "webp", "bmp", "tiff"]
            );
        }

        #[test]
        fn processing_mode_options_disable_copy_for_image_sources() {
            let options = output_processing_mode_options(
                &ConversionConfig::default(),
                Some(&image_metadata()),
                false,
            );

            assert!(options[1].is_disabled);
        }

        #[test]
        fn output_container_options_disable_video_targets_for_audio_sources() {
            let options = output_container_options(
                &ConversionConfig::default(),
                Some(&audio_metadata("aac")),
                false,
            );
            let mp4 = options
                .iter()
                .find(|option| option.container == "mp4")
                .expect("mp4 option should be visible for audio sources");

            assert_eq!(
                mp4.disabled_reason,
                Some("Video container unavailable for audio sources")
            );
        }

        #[test]
        fn stream_copy_audio_target_requires_compatible_audio_codec() {
            let config = ConversionConfig {
                processing_mode: ProcessingMode::Copy,
                container: "mp3".to_string(),
                ..ConversionConfig::default()
            };

            assert!(!is_container_compatible_for_stream_copy(
                &config,
                Some(&audio_metadata("aac")),
                "mp3"
            ));
        }

        #[test]
        fn stream_copy_video_target_rejects_incompatible_subtitles() {
            let config = ConversionConfig {
                processing_mode: ProcessingMode::Copy,
                selected_subtitle_tracks: vec![2],
                ..ConversionConfig::default()
            };

            assert!(!is_container_compatible_for_stream_copy(
                &config,
                Some(&video_metadata()),
                "mp4"
            ));
        }

        #[test]
        fn stream_copy_video_target_accepts_mkv_wildcard_rules() {
            let config = ConversionConfig {
                processing_mode: ProcessingMode::Copy,
                selected_subtitle_tracks: vec![2],
                ..ConversionConfig::default()
            };

            assert!(is_container_compatible_for_stream_copy(
                &config,
                Some(&video_metadata()),
                "mkv"
            ));
        }

        #[test]
        fn stream_copy_without_metadata_keeps_non_image_containers_selectable() {
            let config = ConversionConfig {
                processing_mode: ProcessingMode::Copy,
                ..ConversionConfig::default()
            };

            assert!(is_container_compatible_for_stream_copy(
                &config, None, "mp4"
            ));
        }
    }

    mod output_config {
        use super::*;

        #[test]
        fn normalize_output_config_forces_audio_sources_to_audio_container() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Audio),
                ..SourceMetadata::default()
            };
            let mut config = ConversionConfig::default();

            normalize_output_config(&mut config, Some(&metadata));

            assert_eq!(config.container, "mp3");
        }

        #[test]
        fn normalize_output_config_forces_image_sources_to_image_container() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Image),
                ..SourceMetadata::default()
            };
            let mut config = ConversionConfig::default();

            normalize_output_config(&mut config, Some(&metadata));

            assert_eq!(config.container, "png");
        }

        #[test]
        fn normalize_output_config_reencodes_image_sources() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Image),
                ..SourceMetadata::default()
            };
            let mut config = ConversionConfig {
                processing_mode: ProcessingMode::Copy,
                container: "png".to_string(),
                ..ConversionConfig::default()
            };

            normalize_output_config(&mut config, Some(&metadata));

            assert_eq!(config.processing_mode, ProcessingMode::Reencode);
        }

        #[test]
        fn normalize_output_config_reencodes_gif_outputs() {
            let mut config = ConversionConfig {
                processing_mode: ProcessingMode::Copy,
                container: "gif".to_string(),
                ..ConversionConfig::default()
            };

            normalize_output_config(&mut config, None);

            assert_eq!(config.processing_mode, ProcessingMode::Reencode);
        }

        #[test]
        fn apply_output_container_falls_back_to_default_audio_codec_when_needed() {
            let mut config = ConversionConfig {
                audio_codec: "flac".to_string(),
                ..ConversionConfig::default()
            };

            apply_output_container(&mut config, "webm");

            assert_eq!(config.audio_codec, "libopus");
        }

        #[test]
        fn apply_processing_mode_rejects_copy_for_image_sources() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Image),
                ..SourceMetadata::default()
            };
            let mut config = ConversionConfig::default();

            assert!(!apply_processing_mode(
                &mut config,
                Some(&metadata),
                ProcessingMode::Copy
            ));
        }
    }

    mod source_info_formatting {
        use super::*;

        #[test]
        fn format_source_duration_formats_colon_time_without_fraction() {
            assert_eq!(format_source_duration(Some("01:02:03.450")), "01:02:03");
        }

        #[test]
        fn format_source_duration_formats_numeric_seconds() {
            assert_eq!(format_source_duration(Some("90.4")), "00:01:30");
        }

        #[test]
        fn format_source_duration_keeps_unparseable_values() {
            assert_eq!(format_source_duration(Some("unknown")), "unknown");
        }

        #[test]
        fn format_source_resolution_prefers_dimensions() {
            let metadata = SourceMetadata {
                resolution: Some("1920x1080".to_string()),
                width: Some(3840),
                height: Some(2160),
                ..SourceMetadata::default()
            };

            assert_eq!(format_source_resolution(&metadata), "3840×2160");
        }

        #[test]
        fn format_source_frame_rate_trims_trailing_zeroes() {
            assert_eq!(format_source_frame_rate(Some(29.970)), "29.97 fps");
        }

        #[test]
        fn format_source_bitrate_kbps_uses_megabits_above_threshold() {
            assert_eq!(format_source_bitrate_kbps(Some(2450.0)), "2.45 Mb/s");
        }

        #[test]
        fn format_source_container_bitrate_parses_bits_per_second() {
            assert_eq!(
                format_source_container_bitrate(Some("1250000")),
                "1.25 Mb/s"
            );
        }

        #[test]
        fn format_source_hz_uses_kilohertz_above_threshold() {
            assert_eq!(format_source_hz(Some("48000")), "48 kHz");
        }
    }

    mod source_info_sections {
        use super::*;

        fn row_value<'a>(rows: &'a [SourceInfoRow], label: &str) -> Option<&'a str> {
            rows.iter()
                .find(|row| row.label == label)
                .map(|row| row.value.as_str())
        }

        #[test]
        fn source_info_sections_for_images_use_file_information_only() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Image),
                video_codec: Some("png".to_string()),
                width: Some(640),
                height: Some(480),
                pixel_format: Some("rgba".to_string()),
                ..SourceMetadata::default()
            };

            let sections = source_info_sections(&metadata);

            assert_eq!(
                sections,
                vec![SourceInfoSection::Rows {
                    title: "FILE INFORMATION",
                    rows: vec![
                        SourceInfoRow {
                            label: "IMAGE CODEC",
                            value: "png".to_string(),
                        },
                        SourceInfoRow {
                            label: "DIMENSIONS",
                            value: "640×480".to_string(),
                        },
                        SourceInfoRow {
                            label: "PIXEL FORMAT",
                            value: "rgba".to_string(),
                        },
                    ],
                }]
            );
        }

        #[test]
        fn source_info_sections_for_video_include_file_and_video_sections() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Video),
                duration: Some("00:00:10.50".to_string()),
                bitrate: Some("2500000".to_string()),
                video_codec: Some("h264".to_string()),
                width: Some(1920),
                height: Some(1080),
                frame_rate: Some(59.940),
                video_bitrate_kbps: Some(2200.0),
                ..SourceMetadata::default()
            };

            let sections = source_info_sections(&metadata);

            assert_eq!(sections.len(), 2);
        }

        #[test]
        fn source_info_sections_for_audio_tracks_include_track_rows() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Audio),
                audio_tracks: vec![AudioTrack {
                    index: 3,
                    codec: "aac".to_string(),
                    channels: Some("stereo".to_string()),
                    sample_rate: Some("48000".to_string()),
                    bitrate_kbps: Some(192.0),
                    language: Some("eng".to_string()),
                    ..AudioTrack::default()
                }],
                ..SourceMetadata::default()
            };

            let sections = source_info_sections(&metadata);
            let SourceInfoSection::Tracks { tracks, .. } = &sections[0] else {
                panic!("audio metadata should render audio tracks");
            };

            assert_eq!(row_value(&tracks[0].rows, "SAMPLE RATE"), Some("48 kHz"));
        }
    }

    mod visible_settings_tabs {
        use super::*;

        #[test]
        fn default_video_source_matches_original_default_tab_set() {
            let tabs = tab_ids(super::visible_settings_tabs(
                &ConversionConfig::default(),
                None,
            ));

            assert_eq!(
                tabs,
                vec![
                    "source",
                    "output",
                    "video",
                    "audio",
                    "subtitles",
                    "metadata",
                    "presets"
                ]
            );
        }

        #[test]
        fn audio_source_hides_video_images_and_subtitles() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Audio),
                video_codec: None,
                ..SourceMetadata::default()
            };
            let tabs = tab_ids(super::visible_settings_tabs(
                &ConversionConfig {
                    container: "mp3".to_string(),
                    ..ConversionConfig::default()
                },
                Some(&metadata),
            ));

            assert_eq!(
                tabs,
                vec!["source", "output", "audio", "metadata", "presets"]
            );
        }

        #[test]
        fn image_source_shows_images_and_hides_video_audio_subtitles() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Image),
                video_codec: Some("png".to_string()),
                ..SourceMetadata::default()
            };
            let tabs = tab_ids(super::visible_settings_tabs(
                &ConversionConfig {
                    container: "png".to_string(),
                    ..ConversionConfig::default()
                },
                Some(&metadata),
            ));

            assert_eq!(
                tabs,
                vec!["source", "output", "images", "metadata", "presets"]
            );
        }

        #[test]
        fn copy_mode_hides_video_tab_but_keeps_audio_and_subtitles_when_supported() {
            let config = ConversionConfig {
                processing_mode: ProcessingMode::Copy,
                ..ConversionConfig::default()
            };
            let tabs = tab_ids(super::visible_settings_tabs(&config, None));

            assert_eq!(
                tabs,
                vec![
                    "source",
                    "output",
                    "audio",
                    "subtitles",
                    "metadata",
                    "presets"
                ]
            );
        }

        #[test]
        fn active_hidden_tab_falls_back_to_output() {
            let metadata = SourceMetadata {
                media_kind: Some(SourceKind::Audio),
                video_codec: None,
                ..SourceMetadata::default()
            };
            let active = resolve_active_settings_tab(
                SettingsTab::Video,
                &ConversionConfig::default(),
                Some(&metadata),
            );

            assert_eq!(active, SettingsTab::Output);
        }
    }
}
