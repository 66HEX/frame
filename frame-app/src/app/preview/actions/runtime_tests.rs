use super::*;
use crate::app::*;
use gpui::{AppContext, TestAppContext};

#[test]
fn stale_preview_completion_cannot_replace_a_new_request_for_the_same_key() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let metadata = root.selected_source_metadata_entry();
        let request = root.selected_preview_runtime_request(&metadata).unwrap();
        let old_generation = root.preview_ui.runtime_request_generation;
        root.preview_ui.pending_runtime_key = Some(request.key.clone());
        let old_command_generation = root.preview_ui.media_command_generation;

        root.clear_preview_runtime(cx).detach();
        assert_ne!(
            root.preview_ui.media_command_generation,
            old_command_generation
        );
        root.preview_ui.pending_runtime_key = Some(request.key.clone());
        let new_generation = root.preview_ui.runtime_request_generation;
        let result = Ok(Arc::new(PreviewSession::new_for_test(request.config)));
        assert!(!root.finish_preview_runtime_request(old_generation, &request.key, &result, cx));
        assert!(root.preview_ui.session.is_none());
        assert_eq!(
            root.preview_ui.pending_runtime_key.as_ref(),
            Some(&request.key)
        );

        let error = Err(PreviewEngineError::Ffmpeg("stale failure".to_string()));
        assert!(!root.finish_preview_runtime_request(old_generation, &request.key, &error, cx));
        assert!(root.preview_ui.runtime_error.is_none());

        root.preview_ui.frame_tick_active = true;
        assert!(root.finish_preview_runtime_request(new_generation, &request.key, &result, cx));
        assert_eq!(root.preview_ui.runtime_key, Some(request.key));
        assert!(root.preview_ui.pending_runtime_key.is_none());
        assert!(Arc::ptr_eq(
            root.preview_ui.session.as_ref().unwrap(),
            result.as_ref().unwrap()
        ));
    });
}

#[test]
fn preset_changes_coalesce_while_a_rebuild_is_pending_and_keep_the_old_frame() {
    let mut app = TestAppContext::single();
    let root = app.new(|_| FrameRoot::new());
    root.update(&mut app, |root, cx| {
        root.apply_visual_fixture(Some(VisualFixture::PreviewReady));
        let metadata = root.selected_source_metadata_entry();
        let current = root.selected_preview_runtime_request(&metadata).unwrap();
        let session = Arc::new(PreviewSession::new_for_test(current.config));
        let frame = crate::preview_engine::rendered_frame_from_bgra_payload(1, 1, 4, 0, vec![0; 4])
            .unwrap();
        let _ = session.frame_store().publish(frame);
        root.preview_ui.session = Some(Arc::clone(&session));
        root.refresh_preview_render_image(cx);
        let image = root.preview_ui.render_image.clone().unwrap();
        root.preview_ui.runtime_key = Some(current.key.clone());
        let mut pending_key = current.key.clone();
        pending_key.visual_hash = current.key.visual_hash.wrapping_add(1);
        root.preview_ui.pending_runtime_key = Some(pending_key.clone());

        let mut latest = root.selected_preview_runtime_request(&metadata).unwrap();
        latest.key.visual_hash = current.key.visual_hash.wrapping_add(2);
        root.sync_preview_runtime_for_selection(Some(latest), cx);

        assert_eq!(root.preview_ui.runtime_key, Some(current.key));
        assert_eq!(root.preview_ui.pending_runtime_key, Some(pending_key));
        assert!(Arc::ptr_eq(
            root.preview_ui.session.as_ref().unwrap(),
            &session
        ));
        assert!(Arc::ptr_eq(
            root.preview_ui.render_image.as_ref().unwrap(),
            &image
        ));
    });
}
