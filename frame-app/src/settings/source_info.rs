use super::model::{
    AudioTrack, SourceInfoRow, SourceInfoSection, SourceKind, SourceMetadata, SourceTrackSection,
};

#[must_use]
pub fn source_info_sections(metadata: &SourceMetadata) -> Vec<SourceInfoSection> {
    let source_kind = metadata.source_kind();
    let is_image = source_kind == SourceKind::Image;
    let mut sections = Vec::new();

    if is_image {
        sections.push(SourceInfoSection::Rows {
            title: "File information",
            rows: source_image_rows(metadata),
        });
    } else if has_duration_value(metadata.duration.as_deref())
        || has_bitrate_value(metadata.bitrate.as_deref())
    {
        sections.push(SourceInfoSection::Rows {
            title: "File information",
            rows: source_file_rows(metadata),
        });
    }

    if metadata.video_codec.is_some() && !is_image {
        sections.push(SourceInfoSection::Rows {
            title: "Video stream",
            rows: source_video_rows(metadata),
        });
    }

    if !metadata.audio_tracks.is_empty() {
        sections.push(SourceInfoSection::Tracks {
            title: "Audio stream",
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

fn source_image_rows(metadata: &SourceMetadata) -> Vec<SourceInfoRow> {
    let mut rows = Vec::new();
    push_optional_row(&mut rows, "Image codec", metadata.video_codec.as_deref());
    rows.push(SourceInfoRow {
        label: "Dimensions",
        value: format_source_resolution(metadata),
    });
    push_optional_row(&mut rows, "Pixel format", metadata.pixel_format.as_deref());
    push_optional_row(&mut rows, "Profile", metadata.profile.as_deref());
    push_optional_row(&mut rows, "Color space", metadata.color_space.as_deref());
    push_optional_row(&mut rows, "Color range", metadata.color_range.as_deref());
    push_optional_row(
        &mut rows,
        "Color primaries",
        metadata.color_primaries.as_deref(),
    );
    rows
}

fn source_file_rows(metadata: &SourceMetadata) -> Vec<SourceInfoRow> {
    let mut rows = Vec::new();
    if has_duration_value(metadata.duration.as_deref()) {
        rows.push(SourceInfoRow {
            label: "Duration",
            value: format_source_duration(metadata.duration.as_deref()),
        });
    }
    if has_bitrate_value(metadata.bitrate.as_deref()) {
        rows.push(SourceInfoRow {
            label: "Container bitrate",
            value: format_source_container_bitrate(metadata.bitrate.as_deref()),
        });
    }
    rows
}

fn source_video_rows(metadata: &SourceMetadata) -> Vec<SourceInfoRow> {
    let mut rows = vec![SourceInfoRow {
        label: "Video codec",
        value: display_source_value(metadata.video_codec.as_deref()),
    }];
    push_optional_row(&mut rows, "Profile", metadata.profile.as_deref());
    rows.push(SourceInfoRow {
        label: "Dimensions",
        value: format_source_resolution(metadata),
    });
    if metadata
        .frame_rate
        .is_some_and(|frame_rate| frame_rate > 0.0)
    {
        rows.push(SourceInfoRow {
            label: "Frame rate",
            value: format_source_frame_rate(metadata.frame_rate),
        });
    }
    push_optional_row(&mut rows, "Pixel format", metadata.pixel_format.as_deref());
    push_optional_row(&mut rows, "Color space", metadata.color_space.as_deref());
    push_optional_row(&mut rows, "Color range", metadata.color_range.as_deref());
    push_optional_row(
        &mut rows,
        "Color primaries",
        metadata.color_primaries.as_deref(),
    );
    if metadata
        .video_bitrate_kbps
        .is_some_and(|bitrate| bitrate > 0.0)
    {
        rows.push(SourceInfoRow {
            label: "Video bitrate",
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
            label: "Codec",
            value: display_source_value(Some(&track.codec)),
        },
        SourceInfoRow {
            label: "Channels",
            value: display_source_value(track.channels.as_deref()),
        },
    ];

    if track.sample_rate.is_some() {
        rows.push(SourceInfoRow {
            label: "Sample rate",
            value: format_source_hz(track.sample_rate.as_deref()),
        });
    }
    if track.bitrate_kbps.is_some() {
        rows.push(SourceInfoRow {
            label: "Bitrate",
            value: format_source_bitrate_kbps(track.bitrate_kbps),
        });
    }
    push_optional_row(&mut rows, "Language", track.language.as_deref());
    rows
}

pub(super) fn audio_track_detail(track: &AudioTrack) -> String {
    let mut parts = Vec::new();
    if let Some(channels) = track.channels.as_deref().filter(|value| !value.is_empty()) {
        parts.push(format!("{channels} channels"));
    }
    if let Some(language) = track.language.as_deref().filter(|value| !value.is_empty()) {
        parts.push(language.to_string());
    }
    if let Some(label) = track.label.as_deref().filter(|value| !value.is_empty()) {
        parts.push(label.to_string());
    }
    if track.bitrate_kbps.is_some_and(|bitrate| bitrate > 0.0) {
        parts.push(format_source_bitrate_kbps(track.bitrate_kbps));
    }

    if parts.is_empty() {
        "Source track".to_string()
    } else {
        parts.join(" • ")
    }
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
