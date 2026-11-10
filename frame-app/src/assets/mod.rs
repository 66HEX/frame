//! Bundled assets for the GPUI app.

use gpui::{App, AssetSource, FontFeatures, Result, SharedString};
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

pub const FRAME_FONT_FAMILY: &str = "Inter";
pub const FRAME_FONT_ALIAS: &str = "Inter";
pub const FRAME_FONT_REGULAR_PATH: &str = "fonts/Inter-Regular.ttf";
pub const FRAME_FONT_MEDIUM_PATH: &str = "fonts/Inter-Medium.ttf";
pub const FRAME_FONT_FEATURE_TAGS: [(&str, u32); 1] = [("kern", 1)];
pub const FRAME_TABULAR_NUMBER_FONT_FEATURE_TAG: (&str, u32) = ("tnum", 1);
pub const TOOLTIP_ARROW: &str = "ui/tooltip-arrow.svg";
pub const ICON_FRAME: &str = "icons/frame.svg";
pub const ICON_ARROW_DOWN: &str = "icons/arrow-down.svg";
pub const ICON_LAYOUT_LIST: &str = "icons/layout-list.svg";
pub const ICON_LIST_CHECKS: &str = "icons/list-checks.svg";
pub const ICON_TERMINAL: &str = "icons/terminal.svg";
pub const ICON_CHECK: &str = "icons/check.svg";
pub const ICON_COPY: &str = "icons/copy.svg";
pub const ICON_UNFOLD_MORE: &str = "icons/unfold-more.svg";
pub const ICON_CLOSE: &str = "icons/close.svg";
pub const ICON_DOWNLOAD_02: &str = "icons/download-02.svg";
pub const ICON_FILE: &str = "icons/file.svg";
pub const ICON_FILE_UP: &str = "icons/file-up.svg";
pub const ICON_FILE_DOWN: &str = "icons/file-down.svg";
pub const ICON_FILE_IMPORT: &str = "icons/file-import.svg";
pub const ICON_FOLDER_IMPORT: &str = "icons/folder-import.svg";
pub const ICON_HARD_DRIVE: &str = "icons/hard-drive.svg";
pub const ICON_FILE_VIDEO: &str = "icons/file-video.svg";
pub const ICON_FILE_IMAGE: &str = "icons/file-image.svg";
pub const ICON_MUSIC: &str = "icons/music.svg";
pub const ICON_VIDEO_FILTERS: &str = "icons/video-filters.svg";
pub const ICON_AUDIO_FILTERS: &str = "icons/audio-filters.svg";
pub const ICON_CAPTIONS: &str = "icons/captions.svg";
pub const ICON_TAGS: &str = "icons/tags.svg";
pub const ICON_BOOKMARK: &str = "icons/bookmark.svg";
pub const ICON_SETTINGS: &str = "icons/settings.svg";
pub const ICON_PLUS: &str = "icons/plus.svg";
pub const ICON_MINUS: &str = "icons/minus.svg";
pub const ICON_PLAY: &str = "icons/play.svg";
pub const ICON_PAUSE: &str = "icons/pause.svg";
pub const ICON_PAUSE_2: &str = "icons/pause2.svg";
pub const ICON_ROTATE_CW: &str = "icons/rotate-cw.svg";
pub const ICON_REFRESH: &str = "icons/refresh.svg";
pub const ICON_FLIP_HORIZONTAL: &str = "icons/flip-horizontal.svg";
pub const ICON_FLIP_VERTICAL: &str = "icons/flip-vertical.svg";
pub const ICON_CROP: &str = "icons/crop.svg";
pub const ICON_AUTO_CROP: &str = "icons/auto-crop.svg";
pub const ICON_WARNING: &str = "icons/warning.svg";
pub const ICON_SPINNER: &str = "icons/spinner.svg";
pub const ICON_SQUARE: &str = "icons/square.svg";
pub const ICON_TRASH: &str = "icons/trash.svg";

const FRAME_ICON_SVG: &str = include_str!("../../assets/icons/frame.svg");
const FRAME_FONT_REGULAR_BYTES: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
const FRAME_FONT_MEDIUM_BYTES: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.ttf");

// Nucleo UI 1.9.0, Fill Duo (glyph-duo), native 18px grid with layered opacity.
// SVG colors inherit the theme; source mapping and copyright notice live with the assets.
const ARROW_DOWN_SVG: &str = include_str!("../../assets/icons/nucleo/18px_arrow-down.svg");
const LAYOUT_LIST_SVG: &str = include_str!("../../assets/icons/nucleo/18px_grid-layout-rows.svg");
const LIST_CHECKS_SVG: &str = include_str!("../../assets/icons/nucleo/18px_check-list.svg");
const TERMINAL_SVG: &str = include_str!("../../assets/icons/nucleo/18px_terminal.svg");
const CHECK_SVG: &str = include_str!("../../assets/icons/nucleo/18px_check.svg");
const COPY_SVG: &str = include_str!("../../assets/icons/nucleo/18px_copy.svg");
const UNFOLD_MORE_SVG: &str = include_str!("../../assets/icons/nucleo/18px_chevron-expand-y.svg");
const CLOSE_SVG: &str = include_str!("../../assets/icons/nucleo/18px_xmark.svg");
const DOWNLOAD_02_SVG: &str = include_str!("../../assets/icons/nucleo/18px_download.svg");
const FILE_SVG: &str = include_str!("../../assets/icons/nucleo/18px_file.svg");
const FILE_UP_SVG: &str = include_str!("../../assets/icons/nucleo/18px_file-arrow-in.svg");
const FILE_DOWN_SVG: &str = include_str!("../../assets/icons/nucleo/18px_file-download.svg");
const FILE_IMPORT_SVG: &str = include_str!("../../assets/icons/nucleo/18px_file-arrow-in.svg");
const FOLDER_IMPORT_SVG: &str = include_str!("../../assets/icons/nucleo/18px_folder-import.svg");
const HARD_DRIVE_SVG: &str = include_str!("../../assets/icons/nucleo/18px_hard-drive.svg");
const FILE_VIDEO_SVG: &str = include_str!("../../assets/icons/nucleo/18px_film.svg");
const FILE_IMAGE_SVG: &str = include_str!("../../assets/icons/nucleo/18px_file-image.svg");
const MUSIC_SVG: &str = include_str!("../../assets/icons/nucleo/18px_music-note.svg");
const VIDEO_FILTERS_SVG: &str = include_str!("../../assets/icons/nucleo/18px_filters.svg");
const AUDIO_FILTERS_SVG: &str = include_str!("../../assets/icons/nucleo/18px_waveform-lines.svg");
const CAPTIONS_SVG: &str = include_str!("../../assets/icons/nucleo/18px_closed-captions.svg");
const TAGS_SVG: &str = include_str!("../../assets/icons/nucleo/18px_tags.svg");
const BOOKMARK_SVG: &str = include_str!("../../assets/icons/nucleo/18px_bookmark.svg");
const SETTINGS_SVG: &str = include_str!("../../assets/icons/nucleo/18px_sliders-3-vertical.svg");
const PLUS_SVG: &str = include_str!("../../assets/icons/nucleo/18px_plus.svg");
const MINUS_SVG: &str = include_str!("../../assets/icons/nucleo/18px_minus.svg");
const PLAY_SVG: &str = include_str!("../../assets/icons/nucleo/18px_media-play.svg");
const PAUSE_SVG: &str = include_str!("../../assets/icons/nucleo/18px_media-pause.svg");
const PAUSE_2_SVG: &str = include_str!("../../assets/icons/nucleo/18px_media-pause.svg");
const ROTATE_CW_SVG: &str =
    include_str!("../../assets/icons/nucleo/18px_rotate-image-clockwise.svg");
const REFRESH_SVG: &str = include_str!("../../assets/icons/nucleo/18px_refresh.svg");
const FLIP_HORIZONTAL_SVG: &str =
    include_str!("../../assets/icons/nucleo/18px_flip-horizontal.svg");
const FLIP_VERTICAL_SVG: &str = include_str!("../../assets/icons/nucleo/18px_flip-vertical.svg");
const CROP_SVG: &str = include_str!("../../assets/icons/nucleo/18px_crop.svg");
const AUTO_CROP_SVG: &str =
    include_str!("../../assets/icons/nucleo/18px_square-dashed-sparkle.svg");
const WARNING_SVG: &str = include_str!("../../assets/icons/nucleo/18px_triangle-warning.svg");
const SPINNER_SVG: &str = include_str!("../../assets/icons/nucleo/18px_loader.svg");
const SQUARE_SVG: &str = include_str!("../../assets/icons/nucleo/18px_media-stop.svg");
const TRASH_SVG: &str = include_str!("../../assets/icons/nucleo/18px_trash.svg");

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameAssets;

impl AssetSource for FrameAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let asset: Cow<'static, [u8]> = match path {
            TOOLTIP_ARROW => Cow::Borrowed(include_bytes!("../../assets/ui/tooltip-arrow.svg")),
            FRAME_FONT_REGULAR_PATH => Cow::Borrowed(FRAME_FONT_REGULAR_BYTES),
            FRAME_FONT_MEDIUM_PATH => Cow::Borrowed(FRAME_FONT_MEDIUM_BYTES),
            ICON_FRAME => Cow::Borrowed(FRAME_ICON_SVG.as_bytes()),
            ICON_ARROW_DOWN => Cow::Borrowed(ARROW_DOWN_SVG.as_bytes()),
            ICON_LAYOUT_LIST => Cow::Borrowed(LAYOUT_LIST_SVG.as_bytes()),
            ICON_LIST_CHECKS => Cow::Borrowed(LIST_CHECKS_SVG.as_bytes()),
            ICON_TERMINAL => Cow::Borrowed(TERMINAL_SVG.as_bytes()),
            ICON_CHECK => Cow::Borrowed(CHECK_SVG.as_bytes()),
            ICON_COPY => Cow::Borrowed(COPY_SVG.as_bytes()),
            ICON_UNFOLD_MORE => Cow::Borrowed(UNFOLD_MORE_SVG.as_bytes()),
            ICON_CLOSE => Cow::Borrowed(CLOSE_SVG.as_bytes()),
            ICON_DOWNLOAD_02 => Cow::Borrowed(DOWNLOAD_02_SVG.as_bytes()),
            ICON_FILE => Cow::Borrowed(FILE_SVG.as_bytes()),
            ICON_FILE_UP => Cow::Borrowed(FILE_UP_SVG.as_bytes()),
            ICON_FILE_DOWN => Cow::Borrowed(FILE_DOWN_SVG.as_bytes()),
            ICON_FILE_IMPORT => Cow::Borrowed(FILE_IMPORT_SVG.as_bytes()),
            ICON_FOLDER_IMPORT => Cow::Borrowed(FOLDER_IMPORT_SVG.as_bytes()),
            ICON_HARD_DRIVE => Cow::Borrowed(HARD_DRIVE_SVG.as_bytes()),
            ICON_FILE_VIDEO => Cow::Borrowed(FILE_VIDEO_SVG.as_bytes()),
            ICON_FILE_IMAGE => Cow::Borrowed(FILE_IMAGE_SVG.as_bytes()),
            ICON_MUSIC => Cow::Borrowed(MUSIC_SVG.as_bytes()),
            ICON_VIDEO_FILTERS => Cow::Borrowed(VIDEO_FILTERS_SVG.as_bytes()),
            ICON_AUDIO_FILTERS => Cow::Borrowed(AUDIO_FILTERS_SVG.as_bytes()),
            ICON_CAPTIONS => Cow::Borrowed(CAPTIONS_SVG.as_bytes()),
            ICON_TAGS => Cow::Borrowed(TAGS_SVG.as_bytes()),
            ICON_BOOKMARK => Cow::Borrowed(BOOKMARK_SVG.as_bytes()),
            ICON_SETTINGS => Cow::Borrowed(SETTINGS_SVG.as_bytes()),
            ICON_PLUS => Cow::Borrowed(PLUS_SVG.as_bytes()),
            ICON_MINUS => Cow::Borrowed(MINUS_SVG.as_bytes()),
            ICON_PLAY => Cow::Borrowed(PLAY_SVG.as_bytes()),
            ICON_PAUSE => Cow::Borrowed(PAUSE_SVG.as_bytes()),
            ICON_PAUSE_2 => Cow::Borrowed(PAUSE_2_SVG.as_bytes()),
            ICON_ROTATE_CW => Cow::Borrowed(ROTATE_CW_SVG.as_bytes()),
            ICON_REFRESH => Cow::Borrowed(REFRESH_SVG.as_bytes()),
            ICON_FLIP_HORIZONTAL => Cow::Borrowed(FLIP_HORIZONTAL_SVG.as_bytes()),
            ICON_FLIP_VERTICAL => Cow::Borrowed(FLIP_VERTICAL_SVG.as_bytes()),
            ICON_CROP => Cow::Borrowed(CROP_SVG.as_bytes()),
            ICON_AUTO_CROP => Cow::Borrowed(AUTO_CROP_SVG.as_bytes()),
            ICON_WARNING => Cow::Borrowed(WARNING_SVG.as_bytes()),
            ICON_SPINNER => Cow::Borrowed(SPINNER_SVG.as_bytes()),
            ICON_SQUARE => Cow::Borrowed(SQUARE_SVG.as_bytes()),
            ICON_TRASH => Cow::Borrowed(TRASH_SVG.as_bytes()),
            _ => return Ok(None),
        };

        Ok(Some(asset))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let assets = match path {
            "" => {
                let mut paths = Vec::new();
                for directory in ["fonts", "icons"] {
                    paths.extend(
                        self.list(directory)?
                            .into_iter()
                            .map(|name| SharedString::from(format!("{directory}/{name}"))),
                    );
                }
                paths
            }
            "fonts" => vec![
                SharedString::from("Inter-Regular.ttf"),
                SharedString::from("Inter-Medium.ttf"),
            ],
            "icons" => vec![
                SharedString::from("arrow-down.svg"),
                SharedString::from("auto-crop.svg"),
                SharedString::from("bookmark.svg"),
                SharedString::from("captions.svg"),
                SharedString::from("check.svg"),
                SharedString::from("close.svg"),
                SharedString::from("copy.svg"),
                SharedString::from("crop.svg"),
                SharedString::from("download-02.svg"),
                SharedString::from("file.svg"),
                SharedString::from("file-down.svg"),
                SharedString::from("file-image.svg"),
                SharedString::from("file-import.svg"),
                SharedString::from("file-up.svg"),
                SharedString::from("file-video.svg"),
                SharedString::from("flip-horizontal.svg"),
                SharedString::from("flip-vertical.svg"),
                SharedString::from("folder-import.svg"),
                SharedString::from("frame.svg"),
                SharedString::from("hard-drive.svg"),
                SharedString::from("layout-list.svg"),
                SharedString::from("list-checks.svg"),
                SharedString::from("minus.svg"),
                SharedString::from("music.svg"),
                SharedString::from("audio-filters.svg"),
                SharedString::from("pause.svg"),
                SharedString::from("pause2.svg"),
                SharedString::from("play.svg"),
                SharedString::from("plus.svg"),
                SharedString::from("rotate-cw.svg"),
                SharedString::from("refresh.svg"),
                SharedString::from("settings.svg"),
                SharedString::from("spinner.svg"),
                SharedString::from("square.svg"),
                SharedString::from("tags.svg"),
                SharedString::from("terminal.svg"),
                SharedString::from("trash.svg"),
                SharedString::from("unfold-more.svg"),
                SharedString::from("video-filters.svg"),
                SharedString::from("warning.svg"),
            ],
            _ => Vec::new(),
        };

        Ok(assets)
    }
}

/// Registers bundled Frame fonts with GPUI's text system.
///
/// # Errors
///
/// Returns an error when GPUI rejects one of the bundled font byte streams.
pub fn load_frame_fonts(cx: &mut App) -> Result<()> {
    cx.text_system().add_fonts(frame_font_bytes())
}

#[must_use]
pub fn frame_font_bytes() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(FRAME_FONT_REGULAR_BYTES),
        Cow::Borrowed(FRAME_FONT_MEDIUM_BYTES),
    ]
}

pub fn frame_font_features() -> FontFeatures {
    static FEATURES: OnceLock<FontFeatures> = OnceLock::new();

    FEATURES
        .get_or_init(|| font_features_from_tags(FRAME_FONT_FEATURE_TAGS))
        .clone()
}

pub fn frame_tabular_number_font_features() -> FontFeatures {
    static FEATURES: OnceLock<FontFeatures> = OnceLock::new();

    FEATURES
        .get_or_init(|| {
            font_features_from_tags(
                FRAME_FONT_FEATURE_TAGS
                    .into_iter()
                    .chain(std::iter::once(FRAME_TABULAR_NUMBER_FONT_FEATURE_TAG)),
            )
        })
        .clone()
}

fn font_features_from_tags(tags: impl IntoIterator<Item = (&'static str, u32)>) -> FontFeatures {
    FontFeatures(Arc::new(
        tags.into_iter()
            .map(|(tag, value)| (tag.to_string(), value))
            .collect(),
    ))
}

#[cfg(test)]
mod tests;
