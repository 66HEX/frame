use super::*;

fn root_with_timed_tracks() -> FrameRoot {
    let mut root = FrameRoot::new();
    let probe = frame_core::probe::parse_ffprobe_stdout("/tmp/cross-clock.m2ts", serde_json::json!({
        "format": {"format_name":"mpegts", "start_time":"1.300000", "duration":"5"},
        "streams": [
            {"index":0,"codec_type":"video","codec_name":"h264","width":320,"height":180,"time_base":"1/90000","start_pts":145_920},
            {"index":1,"codec_type":"audio","codec_name":"ac3","channels":2,"time_base":"1/90000","start_pts":126_000},
            {"index":2,"codec_type":"audio","codec_name":"ac3","channels":2,"time_base":"1/90000","start_pts":162_000},
            {"index":3,"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","time_base":"1/90000","start_pts":117_000},
            {"index":4,"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","time_base":"1/90000","start_pts":189_000}
        ]
    }).to_string()).unwrap();
    let mut file = FileItem::from_path("clock", "/tmp/cross-clock.m2ts", 1);
    file.config = ConversionConfig {
        container: "mkv".into(),
        subtitle_burn_path: Some("/tmp/clock.srt".into()),
        selected_audio_tracks: vec![1],
        ..Default::default()
    };
    root.file_queue.add_file(file);
    root.source_metadata.mark_ready(
        "clock",
        crate::source_metadata::source_metadata_from_probe(probe),
    );
    root
}

fn request(root: &mut FrameRoot) -> crate::app::preview::state::PreviewRuntimeRequest {
    let entry = root.selected_source_metadata_entry();
    root.selected_preview_runtime_request(&entry).unwrap()
}

#[test]
fn actual_requests_refresh_only_the_effective_burn_clock_for_selected_streams() {
    let mut root = root_with_timed_tracks();
    let a = request(&mut root);
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .selected_audio_tracks = vec![2];
    let b = request(&mut root);
    assert_eq!(a.config.source_clock, b.config.source_clock);
    assert_ne!(a.key.visual_hash, b.key.visual_hash);
    assert_eq!(
        a.config
            .preview_burn_clock
            .unwrap()
            .shift_expression(0)
            .unwrap()
            .as_deref(),
        Some("round(83/(375*TB))")
    );
    assert_eq!(
        b.config
            .preview_burn_clock
            .unwrap()
            .shift_expression(0)
            .unwrap(),
        None
    );
    assert_eq!(a.config.source_width, b.config.source_width);
    assert_eq!(a.config.source_height, b.config.source_height);
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .selected_subtitle_tracks = vec![3];
    let pgs = request(&mut root);
    assert_ne!(pgs.key.visual_hash, b.key.visual_hash);
    assert_eq!(
        pgs.config
            .preview_burn_clock
            .unwrap()
            .shift_expression(0)
            .unwrap()
            .as_deref(),
        Some("round(241/(750*TB))")
    );
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .selected_subtitle_tracks = vec![4];
    assert_eq!(request(&mut root).key.visual_hash, b.key.visual_hash);
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .selected_audio_tracks = vec![2, 1];
    let ordered = request(&mut root);
    root.file_queue
        .selected_file_mut()
        .unwrap()
        .config
        .selected_audio_tracks = vec![1, 2];
    let reversed = request(&mut root);
    assert_eq!(ordered.key.visual_hash, reversed.key.visual_hash);
    assert_ne!(ordered.key.audio_hash, reversed.key.audio_hash);
    assert_eq!(
        ordered.config.conversion_config.selected_audio_tracks,
        vec![2, 1]
    );
    assert_eq!(
        reversed.config.conversion_config.selected_audio_tracks,
        vec![1, 2]
    );
}

#[test]
fn incomplete_export_drafts_keep_preview_and_dormant_clocks_do_not_refresh_visuals() {
    let mut root = root_with_timed_tracks();
    {
        let config = &mut root.file_queue.selected_file_mut().unwrap().config;
        config.selected_audio_tracks = vec![99];
        config.video_codec = "incomplete encoder".into();
        config.video_bitrate = "incomplete bitrate".into();
    }
    let draft = root.selected_config().unwrap().clone();
    let missing = request(&mut root);
    assert_eq!(
        missing.config.preview_burn_clock.unwrap().export_clock(),
        frame_core::source_timing::ResolvedSourceClock::Unknown
    );
    assert_eq!(root.selected_config().unwrap(), &draft);
    for dormant in ["no-burn", "copy", "audio"] {
        let mut root = root_with_timed_tracks();
        let config = &mut root.file_queue.selected_file_mut().unwrap().config;
        match dormant {
            "no-burn" => config.subtitle_burn_path = None,
            "copy" => config.processing_mode = crate::settings::ProcessingMode::Copy,
            _ => config.container = "wav".into(),
        }
        let a = request(&mut root);
        root.file_queue
            .selected_file_mut()
            .unwrap()
            .config
            .selected_audio_tracks = vec![2];
        let b = request(&mut root);
        assert!(a.config.preview_burn_clock.is_none());
        assert!(b.config.preview_burn_clock.is_none());
        assert_eq!(a.key.visual_hash, b.key.visual_hash, "{dormant}");
    }
}
