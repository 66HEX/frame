use super::*;
use crate::app::updates::state::UpdateStatus;
use crate::file_queue::FileStatus;
use crate::settings::{AudioTrack, ExternalSubtitleTrack, SubtitleTrack};

mod ui;

fn metadata(kind: SourceKind, video: &str, audio: &str) -> SourceMetadata {
    SourceMetadata {
        media_kind: Some(kind),
        video_codec: (kind != SourceKind::Audio).then(|| video.to_string()),
        resolution: Some("1920x1080".into()),
        audio_tracks: [1, 3, 7]
            .map(|index| AudioTrack {
                index,
                codec: audio.into(),
                ..AudioTrack::default()
            })
            .to_vec(),
        subtitle_tracks: [2, 4]
            .map(|index| SubtitleTrack {
                index,
                codec: "subrip".into(),
                ..SubtitleTrack::default()
            })
            .to_vec(),
        ..SourceMetadata::default()
    }
}

fn add_target(root: &mut FrameRoot, id: &str, status: FileStatus, kind: SourceKind) {
    let mut file = FileItem::from_path(id, format!("/tmp/{id}.mov"), 100);
    file.status = status;
    file.config.selected_audio_tracks = vec![3, 7];
    file.config.selected_subtitle_tracks = vec![4];
    file.config.external_subtitle_tracks = vec![ExternalSubtitleTrack {
        path: format!("/tmp/{id}.srt"),
        language: Some("eng".into()),
        title: Some(id.into()),
        is_default: true,
        is_forced: false,
    }];
    file.config.subtitle_burn_path = Some(format!("/tmp/{id}-burn.srt"));
    root.file_queue.add_file(file);
    root.source_metadata
        .mark_ready(id, metadata(kind, "h264", "aac"));
}

fn batch_root() -> FrameRoot {
    let mut root = FrameRoot::new();
    add_target(&mut root, "donor", FileStatus::Idle, SourceKind::Video);
    let donor = root.file_queue.selected_file_mut().unwrap();
    donor.config.container = "mkv".into();
    donor.config.quality = 80;
    donor.config.flip_horizontal = true;
    donor.config.opus_vbr = false;
    donor.config.start_time = Some("00:00:01.000".into());
    donor.config.end_time = Some("00:00:03.000".into());
    donor.config.metadata.comment = Some("Shared output comment".into());
    donor.config.selected_audio_tracks = vec![1];
    donor.config.selected_subtitle_tracks = vec![2];
    root
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The mixed batch fixture and complete field-preservation assertions form one transaction regression."
)]
fn batch_preserves_source_bound_state_and_all_non_config_fields_without_persistence() {
    let mut root = batch_root();
    for (id, status, kind) in [
        ("checked", FileStatus::Idle, SourceKind::Video),
        ("unchecked", FileStatus::Error, SourceKind::Video),
        ("silent", FileStatus::Idle, SourceKind::Video),
        ("audio", FileStatus::Idle, SourceKind::Audio),
        ("image", FileStatus::Idle, SourceKind::Image),
        ("queued", FileStatus::Queued, SourceKind::Video),
        ("converting", FileStatus::Converting, SourceKind::Video),
        ("paused", FileStatus::Paused, SourceKind::Video),
        ("cancelling", FileStatus::Cancelling, SourceKind::Video),
        ("completed", FileStatus::Completed, SourceKind::Video),
        ("loading", FileStatus::Idle, SourceKind::Video),
        ("error", FileStatus::Idle, SourceKind::Video),
    ] {
        add_target(&mut root, id, status, kind);
    }
    root.file_queue
        .file_by_id_mut("unchecked")
        .unwrap()
        .is_selected_for_conversion = false;
    root.file_queue
        .file_by_id_mut("silent")
        .unwrap()
        .config
        .selected_audio_tracks
        .clear();
    root.source_metadata.mark_loading("loading");
    root.source_metadata.mark_error("error", "unavailable");
    let donor = &mut root.file_queue.file_by_id_mut("donor").unwrap().config;
    donor.crop = Some(crate::settings::CropSettings {
        enabled: true,
        x: 20,
        y: 10,
        width: 640,
        height: 360,
        source_width: Some(1920),
        source_height: Some(1080),
        aspect_ratio: Some("16:9".into()),
    });
    donor.overlay = Some(crate::settings::OverlaySettings {
        enabled: true,
        path: "/tmp/logo.png".into(),
        width: 0.2,
        opacity: 0.8,
        ..crate::settings::OverlaySettings::default()
    });
    let before = root.file_queue.files().to_vec();
    let presets = root.presets.clone();
    let sequence = root.settings_ui.next_custom_preset_sequence;
    let generations = (
        root.preview_ui.runtime_request_generation,
        root.preview_ui.media_command_generation,
    );
    let snapshot = root.snapshot_apply_all().unwrap();
    assert_eq!(
        snapshot
            .targets
            .iter()
            .map(|target| target.id.as_str())
            .collect::<Vec<_>>(),
        ["checked", "unchecked", "silent"]
    );
    assert!(root.apply_all_snapshot(&snapshot));
    for (old, new) in before.iter().zip(root.file_queue.files()) {
        let mut non_config = new.clone();
        non_config.config = old.config.clone();
        assert_eq!(
            &non_config, old,
            "{} identity/status/output/checkmark",
            old.id
        );
        if ["checked", "unchecked", "silent"].contains(&old.id.as_str()) {
            assert_eq!(
                new.config,
                candidate_for(
                    &snapshot.preset,
                    &old.config,
                    root.source_metadata.ready_metadata_for(&old.id).unwrap()
                )
                .unwrap()
            );
            assert_eq!(
                new.config.selected_audio_tracks,
                old.config.selected_audio_tracks
            );
            assert_eq!(new.config.selected_subtitle_tracks, [4]);
            assert_eq!(
                new.config.external_subtitle_tracks,
                old.config.external_subtitle_tracks
            );
            assert_eq!(new.config.subtitle_burn_path, old.config.subtitle_burn_path);
            assert_eq!(new.config.quality, 80);
            assert_eq!(new.config.crop, before[0].config.crop);
            assert_eq!(new.config.overlay, before[0].config.overlay);
            assert_eq!(new.config.start_time, before[0].config.start_time);
            assert_eq!(new.config.end_time, before[0].config.end_time);
            assert_eq!(new.config.metadata, before[0].config.metadata);
        } else {
            assert_eq!(new, old);
        }
    }
    assert!(!root.apply_all_snapshot(&snapshot));
    assert_eq!(root.presets, presets);
    assert_eq!(root.settings_ui.next_custom_preset_sequence, sequence);
    assert_eq!(
        (
            root.preview_ui.runtime_request_generation,
            root.preview_ui.media_command_generation
        ),
        generations
    );
}

#[test]
fn snapshot_freezes_donor_settings_and_targets_but_rechecks_current_source_state() {
    let mut root = batch_root();
    for id in [
        "valid",
        "locked",
        "removed",
        "replaced",
        "late",
        "changed-metadata",
    ] {
        add_target(&mut root, id, FileStatus::Idle, SourceKind::Video);
    }
    root.source_metadata.mark_loading("late");
    let snapshot = root.snapshot_apply_all().unwrap();
    let donor_config = snapshot.preset.config.clone();
    root.file_queue
        .file_by_id_mut("donor")
        .unwrap()
        .config
        .quality = 21;
    root.file_queue.select_existing_file("valid");
    root.file_queue.file_by_id_mut("locked").unwrap().status = FileStatus::Queued;
    root.file_queue.remove_file("removed");
    root.file_queue.file_by_id_mut("replaced").unwrap().path = "/tmp/replacement.mov".into();
    root.source_metadata
        .mark_ready("late", metadata(SourceKind::Video, "h264", "aac"));
    root.source_metadata.mark_ready(
        "changed-metadata",
        SourceMetadata {
            audio_tracks: vec![],
            ..metadata(SourceKind::Video, "h264", "aac")
        },
    );
    add_target(&mut root, "new", FileStatus::Idle, SourceKind::Video);
    let before = root.file_queue.files().to_vec();
    assert!(root.apply_all_snapshot(&snapshot));
    for (old, new) in before.iter().zip(root.file_queue.files()) {
        if old.id == "valid" {
            assert_eq!(new.config.quality, donor_config.quality);
        } else {
            assert_eq!(new, old);
        }
    }
    assert_eq!(root.file_queue.selected_file_id(), Some("valid"));
    assert_eq!(
        root.file_queue.file_by_id("donor").unwrap().config.quality,
        21
    );
}

#[test]
fn removed_replaced_loading_locked_donor_or_updater_cancels_whole_snapshot() {
    for scenario in [
        "removed", "replaced", "loading", "error", "queued", "updater",
    ] {
        let mut root = batch_root();
        add_target(&mut root, "target", FileStatus::Idle, SourceKind::Video);
        let snapshot = root.snapshot_apply_all().unwrap();
        match scenario {
            "removed" => {
                root.file_queue.remove_file("donor");
            }
            "replaced" => {
                root.file_queue.file_by_id_mut("donor").unwrap().path = "/tmp/reimport.mov".into();
            }
            "loading" => root.source_metadata.mark_loading("donor"),
            "error" => root.source_metadata.mark_error("donor", "unavailable"),
            "queued" => {
                root.file_queue.file_by_id_mut("donor").unwrap().status = FileStatus::Queued;
            }
            "updater" => root.update_ui.status = UpdateStatus::Installing,
            _ => unreachable!(),
        }
        let before = root.file_queue.files().to_vec();
        assert!(!root.apply_all_snapshot(&snapshot), "{scenario}");
        assert_eq!(root.file_queue.files(), before, "{scenario}");
    }
}

#[test]
fn copy_compatibility_uses_each_targets_actual_selected_indices_and_rechecks_metadata() {
    let mut root = batch_root();
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .processing_mode = ProcessingMode::Copy;
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .container = "mp4".into();
    for (id, video, audio) in [
        ("valid", "h264", "aac"),
        ("prores", "prores", "aac"),
        ("vorbis", "h264", "vorbis"),
        ("stale", "h264", "aac"),
    ] {
        add_target(&mut root, id, FileStatus::Idle, SourceKind::Video);
        root.source_metadata
            .mark_ready(id, metadata(SourceKind::Video, video, audio));
    }
    root.file_queue
        .file_by_id_mut("stale")
        .unwrap()
        .config
        .selected_audio_tracks = vec![99];
    let snapshot = root.snapshot_apply_all().unwrap();
    assert_eq!(snapshot.targets.len(), 1);
    assert_eq!(snapshot.targets[0].id, "valid");
    let before = root.file_queue.files().to_vec();
    root.source_metadata
        .mark_ready("valid", metadata(SourceKind::Video, "prores", "aac"));
    assert!(!root.apply_all_snapshot(&snapshot));
    assert_eq!(root.file_queue.files(), before);
    root.source_metadata
        .mark_ready("valid", metadata(SourceKind::Video, "h264", "aac"));
    assert!(root.apply_all_snapshot(&snapshot));
    assert_eq!(
        root.file_queue
            .file_by_id("valid")
            .unwrap()
            .config
            .selected_audio_tracks,
        [3, 7]
    );
}

#[test]
fn fast_prefilter_matches_full_normalized_oracle_across_media_modes_and_streams() {
    let mut cases = 0;
    for container in ["mp4", "mkv", "webm", "m2t", "opus", "mp3", "gif", "png"] {
        for mode in [ProcessingMode::Reencode, ProcessingMode::Copy] {
            let config = ConversionConfig {
                container: container.into(),
                processing_mode: mode,
                ..ConversionConfig::default()
            };
            let preset = create_custom_preset(String::new(), "test", &config);
            for kind in [SourceKind::Video, SourceKind::Audio, SourceKind::Image] {
                for (video, audio) in [
                    ("h264", "aac"),
                    ("prores", "vorbis"),
                    ("vp9", "opus"),
                    ("", "flac"),
                ] {
                    let metadata = metadata(kind, video, audio);
                    let mut policy = CandidatePolicy::new(&config);
                    for audio in [vec![], vec![3], vec![3, 7], vec![99], vec![3, 99]] {
                        for subtitles in [vec![], vec![4], vec![99], vec![4, 99]] {
                            let target = ConversionConfig {
                                selected_audio_tracks: audio.clone(),
                                selected_subtitle_tracks: subtitles,
                                ..ConversionConfig::default()
                            };
                            let exact = candidate_for(&preset, &target, &metadata);
                            let fast = policy
                                .may_apply(&target, &metadata)
                                .then(|| candidate_for(&preset, &target, &metadata))
                                .flatten();
                            assert_eq!(
                                fast, exact,
                                "{container} {mode:?} {kind:?} {video} {audio:?}"
                            );
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 3840);
}

#[test]
fn availability_continues_after_rejected_candidates_and_matches_snapshot() {
    for status in [
        FileStatus::Idle,
        FileStatus::Error,
        FileStatus::Queued,
        FileStatus::Completed,
    ] {
        for readiness in [
            MetadataStatus::Ready,
            MetadataStatus::Loading,
            MetadataStatus::Error,
        ] {
            let mut root = batch_root();
            // Skip an incompatible video Copy target and continue to an image target,
            // whose normalized mode is Reencode.
            root.file_queue
                .selected_file_mut()
                .unwrap()
                .config
                .container = "png".into();
            root.file_queue
                .selected_file_mut()
                .unwrap()
                .config
                .processing_mode = ProcessingMode::Copy;
            add_target(&mut root, "rejected", FileStatus::Idle, SourceKind::Video);
            add_target(&mut root, "later", status, SourceKind::Image);
            match readiness {
                MetadataStatus::Loading => root.source_metadata.mark_loading("later"),
                MetadataStatus::Error => root.source_metadata.mark_error("later", "unavailable"),
                _ => {}
            }
            let preset = create_custom_preset(
                String::new(),
                "oracle",
                &root.file_queue.selected_file().unwrap().config,
            );
            let full_oracle = root.file_queue.files().iter().any(|target| {
                target.id != "donor"
                    && target.status.is_actionable_for_conversion()
                    && root
                        .source_metadata
                        .ready_metadata_for(&target.id)
                        .is_some_and(|metadata| {
                            candidate_for(&preset, &target.config, metadata).is_some()
                        })
            });
            assert_eq!(
                root.apply_all_availability() == ApplyAllAvailability::Enabled,
                full_oracle,
                "{status:?} {readiness:?}"
            );
            assert_eq!(
                root.snapshot_apply_all().is_some(),
                status.is_actionable_for_conversion() && readiness == MetadataStatus::Ready
            );
        }
    }
}

#[test]
#[ignore = "5k-row candidate performance workload; run explicitly without competing builds"]
fn benchmark_apply_all_availability_5000_rows() {
    for scenario in [
        "eligible-first",
        "eligible-last",
        "status",
        "kind",
        "stale-audio",
        "copy-video",
        "copy-audio",
    ] {
        let mut root = batch_root();
        let copy = scenario.starts_with("copy-");
        if copy {
            let config = &mut root.file_queue.selected_file_mut().unwrap().config;
            config.processing_mode = ProcessingMode::Copy;
            config.container = "mp4".into();
        }
        root.file_queue
            .selected_file_mut()
            .unwrap()
            .config
            .metadata
            .comment = Some("x".repeat(16 * 1024));
        for index in 0..5000 {
            let id = format!("target-{index}");
            add_target(
                &mut root,
                &id,
                if scenario == "status" {
                    FileStatus::Queued
                } else {
                    FileStatus::Idle
                },
                if scenario == "kind" {
                    SourceKind::Image
                } else {
                    SourceKind::Video
                },
            );
            let target = root.file_queue.file_by_id_mut(&id).unwrap();
            if scenario == "stale-audio" || (scenario == "eligible-last" && index < 4999) {
                target.config.selected_audio_tracks = vec![99];
            }
            if copy {
                root.source_metadata.mark_ready(
                    id,
                    metadata(
                        SourceKind::Video,
                        if scenario == "copy-video" {
                            "prores"
                        } else {
                            "h264"
                        },
                        if scenario == "copy-audio" {
                            "vorbis"
                        } else {
                            "aac"
                        },
                    ),
                );
            }
        }
        measure_availability(&root, scenario);
    }
    measure_heavy_candidate();
}

fn measure_availability(root: &FrameRoot, scenario: &str) {
    let mut samples_ns = Vec::with_capacity(100);
    CANDIDATES.with(|count| count.set(0));
    let total = std::time::Instant::now();
    for _ in 0..100 {
        let start = std::time::Instant::now();
        std::hint::black_box(root.apply_all_availability());
        samples_ns.push(start.elapsed().as_nanos());
    }
    let elapsed_us = total.elapsed().as_micros();
    let candidates = CANDIDATES.with(std::cell::Cell::get);
    assert_eq!(
        candidates,
        usize::from(scenario.starts_with("eligible-")) * 100,
        "{scenario}"
    );
    let mut sorted = samples_ns.clone();
    sorted.sort_unstable();
    println!(
        "apply_all_5000 {}",
        serde_json::json!({
            "scenario": scenario, "iterations": 100, "candidates": candidates,
            "elapsed_us": elapsed_us, "median_ns": sorted[50], "p95_ns": sorted[94],
            "max_ns": sorted[99], "samples_ns": samples_ns,
        })
    );
}

fn measure_heavy_candidate() {
    let mut root = batch_root();
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .metadata
        .comment = Some("x".repeat(16 * 1024));
    add_target(&mut root, "candidate", FileStatus::Idle, SourceKind::Video);
    let preset = create_custom_preset(
        String::new(),
        "benchmark",
        &root.file_queue.selected_file().unwrap().config,
    );
    let target = &root.file_queue.file_by_id("candidate").unwrap().config;
    let metadata = root
        .source_metadata
        .ready_metadata_for("candidate")
        .unwrap();
    CANDIDATES.with(|count| count.set(0));
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(candidate_for(&preset, target, metadata).unwrap());
    }
    assert_eq!(CANDIDATES.with(std::cell::Cell::get), 1000);
    println!(
        "apply_all_heavy_candidate iterations=1000 comment_bytes=16384 elapsed_us={}",
        start.elapsed().as_micros()
    );
}

#[test]
fn apply_all_does_not_write_persisted_app_settings() {
    use crate::app_persistence::{AppPersistence, AppSettings};
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("frame-apply-all-{stamp}.json"));
    let persistence = AppPersistence::from_settings_path(path.clone());
    persistence.save(&AppSettings::default()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let mut root = FrameRoot::new_with_persistence(persistence);
    let seeded = batch_root();
    root.file_queue = seeded.file_queue;
    root.source_metadata = seeded.source_metadata;
    add_target(&mut root, "target", FileStatus::Idle, SourceKind::Video);
    let snapshot = root.snapshot_apply_all().unwrap();
    assert!(root.apply_all_snapshot(&snapshot));
    assert!(!root.apply_all_snapshot(&snapshot));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        modified
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn unavailable_donors_and_rejected_statuses_do_not_construct_candidates() {
    let mut root = batch_root();
    add_target(&mut root, "target", FileStatus::Idle, SourceKind::Video);
    for status in [
        MetadataStatus::Loading,
        MetadataStatus::Error,
        MetadataStatus::Idle,
    ] {
        match status {
            MetadataStatus::Loading => root.source_metadata.mark_loading("donor"),
            MetadataStatus::Error => root.source_metadata.mark_error("donor", "failed"),
            _ => root.source_metadata.remove("donor"),
        }
        CANDIDATES.with(|count| count.set(0));
        assert_ne!(root.apply_all_availability(), ApplyAllAvailability::Enabled);
        assert!(root.snapshot_apply_all().is_none());
        assert_eq!(CANDIDATES.with(std::cell::Cell::get), 0);
    }
    root.source_metadata
        .mark_ready("donor", metadata(SourceKind::Video, "h264", "aac"));
    root.file_queue.file_by_id_mut("target").unwrap().status = FileStatus::Completed;
    CANDIDATES.with(|count| count.set(0));
    assert_eq!(
        root.apply_all_availability(),
        ApplyAllAvailability::Disabled(NO_TARGETS)
    );
    assert_eq!(CANDIDATES.with(std::cell::Cell::get), 0);
}

#[test]
fn codec_memo_matches_helpers_for_exact_keys_both_verdicts_and_bounded_overflow() {
    let mut codecs = [
        "aac", "h264", "opus", "vorbis", "prores", "AAC", " h264 ", "OpUs", "", "unknown",
    ]
    .map(str::to_string)
    .to_vec();
    codecs.extend((0..40).map(|index| format!("unknown-{index}")));
    for container in ["mp4", "webm", "mkv", "opus", "m2ts", "png"] {
        for helper in [
            is_video_stream_codec_allowed_for_container as fn(&str, &str) -> bool,
            is_audio_stream_codec_allowed_for_container,
        ] {
            let mut memo = CodecVerdicts::default();
            let mut calls = 0;
            for (index, codec) in codecs.iter().enumerate() {
                for _ in 0..2 {
                    assert_eq!(
                        memo.allowed(codec, || {
                            calls += 1;
                            helper(container, codec)
                        }),
                        helper(container, codec),
                        "{container} {codec}"
                    );
                }
                assert_eq!(memo.entries.len(), (index + 1).min(CODEC_MEMO_LIMIT));
            }
            assert_eq!(
                calls,
                CODEC_MEMO_LIMIT + 2 * (codecs.len() - CODEC_MEMO_LIMIT)
            );
            assert!(memo.entries.iter().any(|(codec, _)| *codec == "aac"));
            assert!(memo.entries.iter().any(|(codec, _)| *codec == "AAC"));
        }
    }
}

#[test]
fn memo_does_not_cache_target_qualification_or_cross_audio_video_or_containers() {
    let metadata = metadata(SourceKind::Video, "h264", "h264");
    for container in ["mp4", "mkv", "mp4"] {
        let donor = ConversionConfig {
            container: container.into(),
            processing_mode: ProcessingMode::Copy,
            ..ConversionConfig::default()
        };
        let preset = create_custom_preset(String::new(), "test", &donor);
        let mut policy = CandidatePolicy::new(&donor);
        for indices in [vec![3], vec![99], vec![], vec![7], vec![3, 99]] {
            let target = ConversionConfig {
                selected_audio_tracks: indices,
                ..ConversionConfig::default()
            };
            assert_eq!(
                policy.may_apply(&target, &metadata),
                candidate_for(&preset, &target, &metadata).is_some()
            );
        }
        assert_eq!(policy.video_verdicts.entries, [("h264", true)]);
        assert_eq!(policy.audio_verdicts.entries, [("h264", false)]);
    }
}
