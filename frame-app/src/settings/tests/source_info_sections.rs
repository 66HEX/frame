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
            title: "File information",
            rows: vec![
                SourceInfoRow {
                    label: "Image codec",
                    value: "png".to_string(),
                },
                SourceInfoRow {
                    label: "Dimensions",
                    value: "640×480".to_string(),
                },
                SourceInfoRow {
                    label: "Pixel format",
                    value: "rgba".to_string(),
                },
            ],
        }]
    );
}

#[test]
fn source_info_sections_for_video_include_file_and_video_rows() {
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

    assert_eq!(
        sections,
        vec![
            SourceInfoSection::Rows {
                title: "File information",
                rows: vec![
                    SourceInfoRow {
                        label: "Duration",
                        value: "00:00:10".to_string(),
                    },
                    SourceInfoRow {
                        label: "Container bitrate",
                        value: "2.5 Mb/s".to_string(),
                    },
                ],
            },
            SourceInfoSection::Rows {
                title: "Video stream",
                rows: vec![
                    SourceInfoRow {
                        label: "Video codec",
                        value: "h264".to_string(),
                    },
                    SourceInfoRow {
                        label: "Dimensions",
                        value: "1920×1080".to_string(),
                    },
                    SourceInfoRow {
                        label: "Frame rate",
                        value: "59.94 fps".to_string(),
                    },
                    SourceInfoRow {
                        label: "Video bitrate",
                        value: "2.2 Mb/s".to_string(),
                    },
                ],
            },
        ]
    );
}

#[test]
fn source_info_sections_show_transport_program_fields() {
    let metadata = SourceMetadata {
        media_kind: Some(SourceKind::Video),
        video_codec: Some("h264".to_string()),
        transport_stream: Some(frame_core::types::TransportStreamMetadata {
            packet_size: Some(192),
            program_id: Some(7),
            service_name: Some("Camera".to_string()),
            service_provider: Some("Frame".to_string()),
        }),
        ..SourceMetadata::default()
    };

    let sections = source_info_sections(&metadata);
    let rows = sections
        .iter()
        .find_map(|section| match section {
            SourceInfoSection::Rows { title, rows } if *title == "Transport stream" => Some(rows),
            _ => None,
        })
        .unwrap();

    assert_eq!(row_value(rows, "Packet size"), Some("192 bytes"));
    assert_eq!(row_value(rows, "Program ID"), Some("7"));
    assert_eq!(row_value(rows, "Service name"), Some("Camera"));
    assert_eq!(row_value(rows, "Service provider"), Some("Frame"));
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

    assert_eq!(row_value(&tracks[0].rows, "Sample rate"), Some("48 kHz"));
}
