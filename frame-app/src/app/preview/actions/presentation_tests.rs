use crate::app::preview::state::PreviewRuntimeKey;
use crate::app::*;
use crate::preview_engine::{
    PreviewEngineError, PreviewRenderPresentation, PreviewRenderedFrame, PreviewSession,
    rendered_frame_from_bgra_payload, rendered_frame_from_bgra_payload_with_image_id,
};
use frame_core::aspect::DisplayRatio;
use gpui::{AppContext, ImageId, TestAppContext};
use std::sync::Arc;

fn owned_frame(id: Option<ImageId>, version: u64, token: u64, ratio: u128) -> PreviewRenderedFrame {
    rendered_frame_from_bgra_payload_with_image_id(
        2,
        2,
        8,
        version,
        id.map(|id| (id, version)),
        vec![u8::try_from(version).unwrap(); 16],
    )
    .unwrap()
    .with_presentation(
        PreviewRenderPresentation {
            pixel_aspect_ratio: DisplayRatio::new(ratio, 1).unwrap(),
            ..Default::default()
        },
        token,
    )
}

fn pending(root: &mut FrameRoot, key: &PreviewRuntimeKey, token: u64) {
    root.preview_ui.runtime_request_generation = token;
    root.preview_ui.pending_runtime_key = Some(key.clone());
    root.preview_ui.desired_runtime_key = Some(key.clone());
    // These tests drive completion explicitly; no timer/decoder is needed.
    root.preview_ui.frame_tick_active = true;
}

#[test]
fn selected_subtitle_clock_changes_keep_the_frame_owned_request_through_completion() {
    let mut app = TestAppContext::single();
    let entity = app.new(|_| FrameRoot::new());
    entity.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let file_id = root.file_queue.selected_file_id().unwrap().to_owned();
        let probe = frame_core::probe::parse_ffprobe_stdout("/tmp/timed.m2ts", serde_json::json!({
            "format":{"format_name":"mpegts","start_time":"1.300000","duration":"5"},
            "streams":[
                {"index":0,"codec_type":"video","codec_name":"h264","width":320,"height":180,"time_base":"1/90000","start_pts":145_920},
                {"index":1,"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","time_base":"1/90000","start_pts":117_000},
                {"index":2,"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","time_base":"1/90000","start_pts":189_000}
            ]
        }).to_string()).unwrap();
        root.source_metadata.mark_ready(file_id, crate::source_metadata::source_metadata_from_probe(probe));
        {
            let config = &mut root.file_queue.selected_file_mut().unwrap().config;
            config.container = "mkv".into();
            config.subtitle_burn_path = Some("/tmp/clock.srt".into());
            config.selected_audio_tracks.clear();
            config.selected_subtitle_tracks = vec![1];
        }
        let entry = root.selected_source_metadata_entry();
        let a = root.selected_preview_runtime_request(&entry).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(a.config.clone()));
        let frame_a = owned_frame(None, 1, 0, 1);
        let image_a = frame_a.render_image();
        let _ = session.frame_store().publish(frame_a);
        root.preview_ui.runtime_key = Some(a.key.clone());
        root.preview_ui.session = Some(Arc::clone(&session));
        assert!(root.refresh_preview_render_image(cx));

        root.file_queue.selected_file_mut().unwrap().config.selected_subtitle_tracks = vec![2];
        let b = root.selected_preview_runtime_request(&entry).unwrap();
        assert_ne!(a.key.visual_hash, b.key.visual_hash);
        assert_eq!(a.config.source_clock, b.config.source_clock);
        assert_ne!(a.config.preview_burn_clock, b.config.preview_burn_clock);
        pending(root, &b.key, 1);
        let frame_b = owned_frame(Some(image_a.id), 2, 1, 1);
        let image_b = frame_b.render_image();
        let _ = session.frame_store().publish(frame_b);
        assert!(!root.refresh_preview_render_image(cx));
        assert!(Arc::ptr_eq(root.preview_ui.render_image.as_ref().unwrap(), &image_a));

        root.file_queue.selected_file_mut().unwrap().config.selected_subtitle_tracks = vec![1];
        let c = root.selected_preview_runtime_request(&entry).unwrap();
        assert_eq!(a.key, c.key);
        root.preview_ui.desired_runtime_key = Some(c.key.clone());
        assert!(root.finish_preview_runtime_request(1, &b.key, &Ok(Arc::clone(&session)), cx));
        assert_eq!(root.preview_ui.rendered_runtime_key.as_ref(), Some(&b.key));
        assert!(Arc::ptr_eq(root.preview_ui.render_image.as_ref().unwrap(), &image_b));
        pending(root, &c.key, 2);
        assert!(root.finish_preview_runtime_request(2, &c.key, &Ok(Arc::clone(&session)), cx));
        // Completion without a C frame retains the B pair. A high-generation
        // initial-A frame cannot impersonate C despite equal keys and geometry.
        assert_eq!(root.preview_ui.rendered_runtime_key.as_ref(), Some(&b.key));
        let _ = session.frame_store().publish(owned_frame(Some(image_a.id), 3, 0, 1));
        assert!(!root.refresh_preview_render_image(cx));
        assert!(Arc::ptr_eq(root.preview_ui.render_image.as_ref().unwrap(), &image_b));
        let frame_c = owned_frame(Some(image_a.id), 4, 2, 1);
        let image_c = frame_c.render_image();
        let _ = session.frame_store().publish(frame_c);
        assert!(root.refresh_preview_render_image(cx));
        assert_eq!(root.preview_ui.rendered_runtime_key.as_ref(), Some(&c.key));
        assert!(Arc::ptr_eq(root.preview_ui.render_image.as_ref().unwrap(), &image_c));
    });
}

#[test]
fn pending_b_frame_is_accepted_as_b_even_when_desired_c_has_another_aspect() {
    let mut app = TestAppContext::single();
    let entity = app.new(|_| FrameRoot::new());
    entity.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let entry = root.selected_source_metadata_entry();
        let a = root.selected_preview_runtime_request(&entry).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(a.config));
        let frame_a = owned_frame(None, 1, 0, 2);
        let image_a = frame_a.render_image();
        let _ = session.frame_store().publish(frame_a);
        root.preview_ui.runtime_key = Some(a.key.clone());
        root.preview_ui.session = Some(Arc::clone(&session));
        assert!(root.refresh_preview_render_image(cx));
        let generation_a = root.preview_ui.render_generation;
        let presented_a = session.snapshot().runtime_metrics.rendered_frames_presented;
        let mut b = a.key;
        b.visual_hash = b.visual_hash.wrapping_add(1);
        pending(root, &b, 1);
        let mut c = b.clone();
        c.visual_hash = c.visual_hash.wrapping_add(1);
        root.preview_ui.desired_runtime_key = Some(c.clone());
        // Real backend texture identity stays the same while content changes.
        let image_b = owned_frame(Some(image_a.id), 2, 1, 3);
        let expected_b = image_b.render_image();
        let _ = session.frame_store().publish(image_b);
        assert!(!root.refresh_preview_render_image(cx));
        assert!(Arc::ptr_eq(
            root.preview_ui.render_image.as_ref().unwrap(),
            &image_a
        ));
        assert_eq!(root.preview_ui.render_generation, generation_a);
        assert_eq!(
            session.snapshot().runtime_metrics.rendered_frames_presented,
            presented_a
        );
        assert_eq!(
            root.preview_ui
                .rendered_presentation
                .pixel_aspect_ratio
                .exact_ratio(),
            Some((2, 1))
        );
        assert!(root.finish_preview_runtime_request(1, &b, &Ok(Arc::clone(&session)), cx));
        assert!(Arc::ptr_eq(
            root.preview_ui.render_image.as_ref().unwrap(),
            &expected_b
        ));
        assert_ne!(image_a.content_version(), expected_b.content_version());
        assert_eq!(
            root.preview_ui
                .rendered_presentation
                .pixel_aspect_ratio
                .exact_ratio(),
            Some((3, 1))
        );
        assert_eq!(root.preview_ui.rendered_runtime_key.as_ref(), Some(&b));
        assert_eq!(root.preview_ui.desired_runtime_key.as_ref(), Some(&c));
        assert!(!root.preview_edit_frame_ready());
    });
}

#[test]
fn equal_texture_and_pixel_aspect_do_not_relabel_the_accepted_output_raster() {
    let mut app = TestAppContext::single();
    let entity = app.new(|_| FrameRoot::new());
    entity.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let entry = root.selected_source_metadata_entry();
        let a = root.selected_preview_runtime_request(&entry).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(a.config));
        let mut frame_a = owned_frame(None, 1, 0, 2);
        frame_a.presentation.output_raster_aspect_ratio = DisplayRatio::new(100, 1);
        let image_a = frame_a.render_image();
        let _ = session.frame_store().publish(frame_a);
        root.preview_ui.runtime_key = Some(a.key.clone());
        root.preview_ui.session = Some(Arc::clone(&session));
        assert!(root.refresh_preview_render_image(cx));
        let mut b = a.key;
        b.visual_hash = b.visual_hash.wrapping_add(1);
        pending(root, &b, 1);
        let mut frame_b = owned_frame(Some(image_a.id), 2, 1, 2);
        frame_b.presentation.output_raster_aspect_ratio = DisplayRatio::new(120, 1);
        let _ = session.frame_store().publish(frame_b);
        assert!(!root.refresh_preview_render_image(cx));
        assert_eq!(
            root.preview_ui
                .rendered_presentation
                .output_raster_aspect_ratio,
            DisplayRatio::new(100, 1)
        );
        assert!(Arc::ptr_eq(
            root.preview_ui.render_image.as_ref().unwrap(),
            &image_a
        ));
        assert!(root.finish_preview_runtime_request(1, &b, &Ok(Arc::clone(&session)), cx));
        assert_eq!(
            root.preview_ui
                .rendered_presentation
                .output_raster_aspect_ratio,
            DisplayRatio::new(120, 1)
        );
        assert_eq!(
            root.preview_ui.rendered_presentation.pixel_aspect_ratio,
            DisplayRatio::new(2, 1).unwrap()
        );
        assert_eq!(
            root.preview_ui.render_image.as_ref().unwrap().size(0),
            image_a.size(0)
        );
    });
}

#[test]
fn successful_rebuild_without_its_own_frame_retains_a_and_rejects_late_a_even_at_equal_aspect() {
    let mut app = TestAppContext::single();
    let entity = app.new(|_| FrameRoot::new());
    entity.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let entry = root.selected_source_metadata_entry();
        let a = root.selected_preview_runtime_request(&entry).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(a.config));
        let first = owned_frame(None, 1, 0, 2);
        let image_a = first.render_image();
        let _ = session.frame_store().publish(first);
        root.preview_ui.runtime_key = Some(a.key.clone());
        root.preview_ui.session = Some(Arc::clone(&session));
        assert!(root.refresh_preview_render_image(cx));
        let mut b = a.key.clone();
        b.preview_dimensions.max_width += 64;
        pending(root, &b, 1);
        assert!(root.finish_preview_runtime_request(1, &b, &Ok(Arc::clone(&session)), cx));
        for version in [2, 3] {
            let _ = session
                .frame_store()
                .publish(owned_frame(Some(image_a.id), version, 0, 2));
            assert!(!root.refresh_preview_render_image(cx));
            assert!(Arc::ptr_eq(
                root.preview_ui.render_image.as_ref().unwrap(),
                &image_a
            ));
            assert_eq!(root.preview_ui.rendered_runtime_key.as_ref(), Some(&a.key));
        }
        let _ = session
            .frame_store()
            .publish(owned_frame(Some(image_a.id), 4, 1, 2));
        assert!(root.refresh_preview_render_image(cx));
        assert_eq!(root.preview_ui.rendered_runtime_key.as_ref(), Some(&b));
        // A -> B -> A has identical dimensions/key/aspect to the first request,
        // but only a newly produced A frame may satisfy its new token.
        pending(root, &a.key, 2);
        assert!(!root.finish_preview_runtime_request(1, &b, &Ok(Arc::clone(&session)), cx));
        assert!(root.finish_preview_runtime_request(2, &a.key, &Ok(Arc::clone(&session)), cx));
        assert_eq!(root.preview_ui.rendered_runtime_key.as_ref(), Some(&b));
        let _ = session
            .frame_store()
            .publish(owned_frame(Some(image_a.id), 5, 2, 2));
        assert!(root.refresh_preview_render_image(cx));
        assert_eq!(root.preview_ui.rendered_runtime_key.as_ref(), Some(&a.key));
        assert_eq!(
            root.preview_ui
                .render_image
                .as_ref()
                .unwrap()
                .content_version(),
            5
        );
    });
}

#[test]
fn failed_rebuild_does_not_adopt_a_premature_frame_or_change_the_accepted_clock() {
    let mut app = TestAppContext::single();
    let entity = app.new(|_| FrameRoot::new());
    entity.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let entry = root.selected_source_metadata_entry();
        let request = root.selected_preview_runtime_request(&entry).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(request.config));
        let frame = owned_frame(None, 1, 0, 2);
        let image = frame.render_image();
        let _ = session.frame_store().publish(frame);
        root.preview_ui.session = Some(Arc::clone(&session));
        root.preview_ui.runtime_key = Some(request.key.clone());
        assert!(root.refresh_preview_render_image(cx));
        let mut b = request.key;
        b.visual_hash = b.visual_hash.wrapping_add(1);
        pending(root, &b, 1);
        let _ = session
            .frame_store()
            .publish(owned_frame(Some(image.id), 2, 1, 3));
        let failure = Err(PreviewEngineError::Ffmpeg("rebuild failed".into()));
        assert!(root.finish_preview_runtime_request(1, &b, &failure, cx));
        assert!(!root.refresh_preview_render_image(cx));
        assert_eq!(root.preview_ui.accepted_request_token, 0);
        assert!(Arc::ptr_eq(
            root.preview_ui.render_image.as_ref().unwrap(),
            &image
        ));
        // Ordinary playback/seek frames of the still-accepted request keep token zero.
        let _ = session
            .frame_store()
            .publish(owned_frame(Some(image.id), 3, 0, 2));
        assert!(root.refresh_preview_render_image(cx));
        assert_eq!(
            root.preview_ui
                .render_image
                .as_ref()
                .unwrap()
                .content_version(),
            3
        );
        assert!(root.preview_ui.runtime_error.is_some());
    });
}

#[test]
fn token_zero_remains_the_default_for_manually_seeded_frames() {
    let frame = rendered_frame_from_bgra_payload(1, 1, 4, 0, vec![0; 4]).unwrap();
    assert_eq!(frame.request_token, 0);
    assert_eq!(frame.presentation, PreviewRenderPresentation::default());
}

#[test]
fn static_fixture_hook_never_survives_selection_clear_or_another_file() {
    let mut app = TestAppContext::single();
    let entity = app.new(|_| FrameRoot::new());
    entity.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewAnamorphicCrop));
        let entry = root.selected_source_metadata_entry();
        let mut request = root.selected_preview_runtime_request(&entry).unwrap();
        root.sync_preview_runtime_for_selection(Some(request.clone()), cx);
        assert!(root.preview_ui.session.is_none());
        assert!(root.preview_ui.pending_runtime_key.is_none());
        assert!(root.preview_edit_frame_ready());
        request.key.preview_dimensions.max_width += 64;
        root.sync_preview_runtime_for_selection(Some(request.clone()), cx);
        assert_eq!(
            root.preview_ui.rendered_runtime_key.as_ref(),
            Some(&request.key)
        );
        request.key.file_id = "another-file".into();
        request.config.file_id.clone_from(&request.key.file_id);
        root.sync_preview_runtime_for_selection(Some(request), cx);
        assert!(root.preview_ui.static_fixture_file_id.is_none());
        assert!(root.preview_ui.render_image.is_none());
        assert!(root.preview_ui.pending_runtime_key.is_some());
        root.clear_preview_runtime(cx).detach();
        assert!(root.preview_ui.static_fixture_file_id.is_none());
    });
}

#[test]
#[ignore = "requires FFmpeg; exercises the real asynchronous app request path"]
fn runtime_sync_assigns_the_generation_to_the_decoder_and_accepted_frame() {
    use crate::file_queue::FileItem;
    use crate::settings::{SourceKind, SourceMetadata};
    use frame_core::aspect::SampleAspectRatio;
    let path = std::env::temp_dir().join(format!("frame-r7-sync-{}.mkv", std::process::id()));
    let status = crate::process_launch::status(
        std::process::Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=160x90:r=12:d=1,setsar=2/1",
                "-c:v",
                "ffv1",
            ])
            .arg(&path),
    )
    .unwrap();
    assert!(status.success());
    let mut app = TestAppContext::single();
    let entity = app.new(|_| FrameRoot::new());
    entity.update(&mut app, |root, _| {
        root.file_queue
            .add_file(FileItem::from_path("r7-real", path.to_string_lossy(), 1));
        root.source_metadata.mark_ready(
            "r7-real",
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(160),
                height: Some(90),
                duration: Some("1.0".into()),
                sample_aspect_ratio: SampleAspectRatio::new(2, 1),
                ..Default::default()
            },
        );
    });
    let mut previous_token = 0;
    for rotation in ["0", "90", "0"] {
        let token = entity.update(&mut app, |root, cx| {
            root.file_queue.selected_file_mut().unwrap().config.rotation = rotation.into();
            let entry = root.selected_source_metadata_entry();
            let request = root.selected_preview_runtime_request(&entry).unwrap();
            assert_eq!(request.config.request_token, 0);
            root.sync_preview_runtime_for_selection(Some(request), cx);
            root.preview_ui.runtime_request_generation
        });
        assert!(token > previous_token);
        previous_token = token;
        app.run_until_parked();
        entity.read_with(&app, |root, _| {
            assert!(root.preview_ui.static_fixture_file_id.is_none());
            assert!(
                root.preview_ui.runtime_error.is_none(),
                "{:?}",
                root.preview_ui.runtime_error
            );
            assert!(root.preview_ui.pending_runtime_key.is_none());
            assert_eq!(root.preview_ui.accepted_request_token, token);
            let session = root.preview_ui.session.as_ref().unwrap();
            let latest = session.latest_frame().unwrap();
            assert_eq!(latest.frame.request_token, token);
            assert_eq!(
                root.preview_ui.rendered_presentation,
                latest.frame.presentation
            );
            assert_eq!(
                root.preview_ui.rendered_runtime_key,
                root.preview_ui.runtime_key
            );
            assert_eq!(
                latest.frame.presentation.pixel_aspect_ratio.exact_ratio(),
                Some(if rotation == "90" { (1, 2) } else { (2, 1) })
            );
        });
    }
    entity.update(&mut app, |root, cx| root.clear_preview_runtime(cx).detach());
    app.run_until_parked();
    let _ = std::fs::remove_file(path);
}

#[test]
#[ignore = "requires FFmpeg; exercises selected export clocks through the actual preview decoder"]
fn runtime_sync_transports_selected_export_clock_to_paused_burned_frames() {
    let (directory, source, metadata) = burn_clock_media_fixture();
    let subtitles = directory.join("clock.srt");
    let audio = metadata.audio_tracks.first().unwrap().index;
    let mut app = TestAppContext::single();
    let entity = app.new(|_| FrameRoot::new());
    entity.update(&mut app, |root, _| {
        let mut file =
            crate::file_queue::FileItem::from_path("r14-real", source.to_string_lossy(), 1);
        file.config.container = "mkv".into();
        file.config.subtitle_burn_path = Some(subtitles.to_string_lossy().into_owned());
        root.file_queue.add_file(file);
        root.source_metadata.mark_ready("r14-real", metadata);
    });
    let mut frames = Vec::new();
    let mut previous_token = 0;
    for selected in [Vec::new(), vec![audio], Vec::new()] {
        let token = entity.update(&mut app, |root, cx| {
            root.file_queue
                .selected_file_mut()
                .unwrap()
                .config
                .selected_audio_tracks = selected.clone();
            let mut request = root
                .selected_preview_runtime_request(&root.selected_source_metadata_entry())
                .unwrap();
            let clock = request.config.preview_burn_clock.unwrap();
            assert!(matches!(
                clock.export_clock(),
                frame_core::source_timing::ResolvedSourceClock::Known { .. }
            ));
            assert_eq!(
                clock.shift_expression(0).unwrap().is_some(),
                !selected.is_empty()
            );
            assert_eq!(
                request.config.conversion_config.selected_audio_tracks,
                selected
            );
            // Paused preview prewarms audio. This regression isolates visual
            // decoder transport without requiring a physical audio device;
            // the actual request key, export selection and clock stay intact.
            request.config.selected_audio_track = None;
            root.sync_preview_runtime_for_selection(Some(request), cx);
            root.preview_ui.runtime_request_generation
        });
        assert!(token > previous_token);
        previous_token = token;
        app.run_until_parked();
        entity.read_with(&app, |root, _| {
            assert!(
                root.preview_ui.runtime_error.is_none(),
                "{:?}",
                root.preview_ui.runtime_error
            );
            assert!(root.preview_ui.pending_runtime_key.is_none());
            assert_eq!(root.preview_ui.accepted_request_token, token);
            assert_eq!(
                root.preview_ui.rendered_runtime_key,
                root.preview_ui.runtime_key
            );
            let latest = root
                .preview_ui
                .session
                .as_ref()
                .unwrap()
                .latest_frame()
                .unwrap();
            assert_eq!(latest.frame.request_token, token);
            assert_eq!(
                root.preview_ui.rendered_presentation,
                latest.frame.presentation
            );
            frames.push(latest.frame.render_image().as_bytes(0).unwrap().to_vec());
        });
    }
    assert!(!frames[0].is_empty());
    assert!(
        frames[0]
            .chunks_exact(4)
            .any(|pixel| pixel[..3].iter().any(|value| *value > 150))
    );
    assert!(
        frames[1]
            .chunks_exact(4)
            .all(|pixel| pixel[..3] == [0, 0, 0])
    );
    assert_eq!(frames[0].len(), frames[1].len());
    assert!(
        frames[0] == frames[2],
        "returning to the original map must restore every caption pixel"
    );
    entity.update(&mut app, |root, cx| root.clear_preview_runtime(cx).detach());
    app.run_until_parked();
    let _ = std::fs::remove_dir_all(directory);
}

fn burn_clock_media_fixture() -> (
    std::path::PathBuf,
    std::path::PathBuf,
    crate::settings::SourceMetadata,
) {
    let directory = std::env::temp_dir().join(format!("frame-r14-sync-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("source.m2ts");
    let subtitles = directory.join("clock.srt");
    std::fs::write(&subtitles, "1\n00:00:00,000 --> 00:00:00,100\nEARLY\n").unwrap();
    let status = crate::process_launch::status(
        std::process::Command::new(crate::runtime_binaries::ffmpeg_executable())
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=160x90:r=20:d=1.5",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=400:sample_rate=48000:duration=1.5",
                "-map",
                "0:v:0",
                "-map",
                "1:a:0",
                "-vf",
                "setpts=PTS+0.2/TB",
                "-c:v",
                "libx264",
                "-crf",
                "0",
                "-g",
                "1",
                "-preset",
                "ultrafast",
                "-c:a",
                "ac3",
                "-mpegts_m2ts_mode",
                "1",
                "-muxdelay",
                "0",
            ])
            .arg(&source),
    )
    .unwrap();
    assert!(status.success());
    let metadata = crate::source_metadata::probe_source_metadata(source.to_str().unwrap()).unwrap();
    (directory, source, metadata)
}
