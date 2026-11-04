#![expect(
    clippy::float_cmp,
    reason = "App state tests compare exact deterministic layout and timeline values."
)]

use super::input::{
    should_capture_text_input_drag, should_handle_text_input, text_input_scroll_x_for_cursor,
};
use super::inspector::{hex_to_subtitle_hsv, subtitle_hsv_to_hex};
use super::preview_actions::{
    lerp_preview_canvas_value, lerp_preview_canvas_value_for_elapsed, preview_canvas_initial_zoom,
    preview_canvas_keyboard_pan_delta, preview_canvas_layout_metrics, preview_canvas_pan_limits,
    preview_canvas_transform_settled, preview_canvas_transform_visual_delta,
    preview_canvas_wheel_zoom_multiplier, preview_crop_keyboard_delta,
    preview_overlay_keyboard_delta, preview_runtime_dimensions,
};
use super::primitives::frame_highlight_px;
use super::*;
use crate::app::preview::*;
use crate::app_persistence::{AppPersistence, AppSettings};
use crate::notifications::{AppNotifier, ConversionNotificationSummary};
use crate::preview_engine::{
    PreviewCrop as EnginePreviewCrop, PreviewFrame, render_image_from_frame,
};
use crate::{TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_X, TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_Y};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static TEST_SETTINGS_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

mod imports;

mod updates;

mod workflows;

mod preview;

mod window;

mod visual_fixtures;

mod surface_highlights;

mod preview_shell;

mod visual_contract;

fn test_settings_path() -> PathBuf {
    let sequence = TEST_SETTINGS_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();

    std::env::temp_dir()
        .join("frame-root-persistence-tests")
        .join(format!("{}-{millis}-{sequence}", std::process::id()))
        .join("settings.json")
}

pub(in crate::app) mod support;
