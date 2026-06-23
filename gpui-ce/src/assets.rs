//! Bundled assets for the GPUI rewrite.

use std::borrow::Cow;

use gpui::{App, AssetSource, Result, SharedString};

pub const FRAME_FONT_FAMILY: &str = "Ioskeley Mono Heavy";
pub const FRAME_FONT_ALIAS: &str = "LoskeleyMono";
pub const FRAME_FONT_PATH: &str = "fonts/IoskeleyMono-Black.woff2";
pub const ICON_FRAME: &str = "icons/frame.svg";
pub const ICON_LAYOUT_LIST: &str = "icons/layout-list.svg";
pub const ICON_TERMINAL: &str = "icons/terminal.svg";
pub const ICON_FILE_UP: &str = "icons/file-up.svg";
pub const ICON_FILE_DOWN: &str = "icons/file-down.svg";
pub const ICON_HARD_DRIVE: &str = "icons/hard-drive.svg";
pub const ICON_FILE_VIDEO: &str = "icons/file-video.svg";
pub const ICON_FILE_IMAGE: &str = "icons/file-image.svg";
pub const ICON_MUSIC: &str = "icons/music.svg";
pub const ICON_CAPTIONS: &str = "icons/captions.svg";
pub const ICON_TAGS: &str = "icons/tags.svg";
pub const ICON_BOOKMARK: &str = "icons/bookmark.svg";
pub const ICON_SETTINGS: &str = "icons/settings.svg";
pub const ICON_PLUS: &str = "icons/plus.svg";
pub const ICON_PLAY: &str = "icons/play.svg";
pub const ICON_PAUSE: &str = "icons/pause.svg";
pub const ICON_ROTATE_CW: &str = "icons/rotate-cw.svg";
pub const ICON_FLIP_HORIZONTAL: &str = "icons/flip-horizontal.svg";
pub const ICON_FLIP_VERTICAL: &str = "icons/flip-vertical.svg";
pub const ICON_CROP: &str = "icons/crop.svg";
pub const ICON_ZOOM_IN: &str = "icons/zoom-in.svg";
pub const ICON_ZOOM_OUT: &str = "icons/zoom-out.svg";
pub const ICON_TRAFFIC_CLOSE_DOT: &str = "icons/traffic-close-dot.svg";
pub const ICON_TRAFFIC_CLOSE_SYMBOL: &str = "icons/traffic-close-symbol.svg";
pub const ICON_TRAFFIC_MINIMIZE_DOT: &str = "icons/traffic-minimize-dot.svg";
pub const ICON_TRAFFIC_MINIMIZE_SYMBOL: &str = "icons/traffic-minimize-symbol.svg";
pub const ICON_TRAFFIC_ZOOM_DOT: &str = "icons/traffic-zoom-dot.svg";
pub const ICON_TRAFFIC_ZOOM_SYMBOL: &str = "icons/traffic-zoom-symbol.svg";

const FRAME_ICON_SVG: &str = include_str!("../assets/icons/frame.svg");
const FRAME_FONT_BYTES: &[u8] =
    include_bytes!("../assets/fonts/LoskeleyMono/IoskeleyMono-Black.woff2");

const LAYOUT_LIST_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M32,64a8,8,0,0,1,8-8H216a8,8,0,0,1,0,16H40A8,8,0,0,1,32,64Zm104,56H40a8,8,0,0,0,0,16h96a8,8,0,0,0,0-16Zm0,64H40a8,8,0,0,0,0,16h96a8,8,0,0,0,0-16Zm112-24a8,8,0,0,1-3.76,6.78l-64,40A8,8,0,0,1,168,200V120a8,8,0,0,1,12.24-6.78l64,40A8,8,0,0,1,248,160Zm-23.09,0L184,134.43v51.14Z"/></svg>"#;
const TERMINAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M117.31,134l-72,64a8,8,0,1,1-10.63-12L100,128,34.69,70A8,8,0,1,1,45.32,58l72,64a8,8,0,0,1,0,12ZM216,184H120a8,8,0,0,0,0,16h96a8,8,0,0,0,0-16Z"/></svg>"#;
const FILE_UP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M213.66,82.34l-56-56A8,8,0,0,0,152,24H56A16,16,0,0,0,40,40V216a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V88A8,8,0,0,0,213.66,82.34ZM160,51.31,188.69,80H160ZM200,216H56V40h88V88a8,8,0,0,0,8,8h48V216Zm-42.34-77.66a8,8,0,0,1-11.32,11.32L136,139.31V184a8,8,0,0,1-16,0V139.31l-10.34,10.35a8,8,0,0,1-11.32-11.32l24-24a8,8,0,0,1,11.32,0Z"/></svg>"#;
const FILE_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M213.66,82.34l-56-56A8,8,0,0,0,152,24H56A16,16,0,0,0,40,40V216a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V88A8,8,0,0,0,213.66,82.34ZM160,51.31,188.69,80H160ZM200,216H56V40h88V88a8,8,0,0,0,8,8h48V216Zm-42.34-61.66a8,8,0,0,1,0,11.32l-24,24a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L120,164.69V120a8,8,0,0,1,16,0v44.69l10.34-10.35A8,8,0,0,1,157.66,154.34Z"/></svg>"#;
const HARD_DRIVE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M208,136H48a16,16,0,0,0-16,16v48a16,16,0,0,0,16,16H208a16,16,0,0,0,16-16V152A16,16,0,0,0,208,136Zm0,64H48V152H208v48Zm0-160H48A16,16,0,0,0,32,56v48a16,16,0,0,0,16,16H208a16,16,0,0,0,16-16V56A16,16,0,0,0,208,40Zm0,64H48V56H208v48ZM192,80a12,12,0,1,1-12-12A12,12,0,0,1,192,80Zm0,96a12,12,0,1,1-12-12A12,12,0,0,1,192,176Z"/></svg>"#;
const FILE_VIDEO_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M213.66,82.34l-56-56A8,8,0,0,0,152,24H56A16,16,0,0,0,40,40v72a8,8,0,0,0,16,0V40h88V88a8,8,0,0,0,8,8h48V216h-8a8,8,0,0,0,0,16h8a16,16,0,0,0,16-16V88A8,8,0,0,0,213.66,82.34ZM160,51.31,188.69,80H160ZM155.88,145a8,8,0,0,0-8.12.22l-19.95,12.46A16,16,0,0,0,112,144H48a16,16,0,0,0-16,16v48a16,16,0,0,0,16,16h64a16,16,0,0,0,15.81-13.68l19.95,12.46A8,8,0,0,0,160,216V152A8,8,0,0,0,155.88,145ZM112,208H48V160h64v48Zm32-6.43-16-10V176.43l16-10Z"/></svg>"#;
const FILE_IMAGE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M110.66,147.56a8,8,0,0,0-13.32,0L76.49,178.85l-9.76-15.18a8,8,0,0,0-13.46,0l-36,56A8,8,0,0,0,24,232H152a8,8,0,0,0,6.66-12.44ZM38.65,216,60,182.79l9.63,15a8,8,0,0,0,13.39.11l21-31.47L137.05,216Zm175-133.66-56-56A8,8,0,0,0,152,24H56A16,16,0,0,0,40,40v88a8,8,0,0,0,16,0V40h88V88a8,8,0,0,0,8,8h48V216h-8a8,8,0,0,0,0,16h8a16,16,0,0,0,16-16V88A8,8,0,0,0,213.66,82.34ZM160,51.31,188.69,80H160Z"/></svg>"#;
const MUSIC_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M99.06,128.61a8,8,0,0,0-8.72,1.73L68.69,152H48a8,8,0,0,0-8,8v40a8,8,0,0,0,8,8H68.69l21.65,21.66A8,8,0,0,0,104,224V136A8,8,0,0,0,99.06,128.61ZM88,204.69,77.66,194.34A8,8,0,0,0,72,192H56V168H72a8,8,0,0,0,5.66-2.34L88,155.31ZM152,180a40.55,40.55,0,0,1-20,34.91A8,8,0,0,1,124,201.09a24.49,24.49,0,0,0,0-42.18A8,8,0,0,1,132,145.09,40.55,40.55,0,0,1,152,180Zm61.66-97.66-56-56A8,8,0,0,0,152,24H56A16,16,0,0,0,40,40v80a8,8,0,0,0,16,0V40h88V88a8,8,0,0,0,8,8h48V216H168a8,8,0,0,0,0,16h32a16,16,0,0,0,16-16V88A8,8,0,0,0,213.66,82.34ZM160,51.31,188.69,80H160Z"/></svg>"#;
const CAPTIONS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M172,112a8,8,0,0,1-8,8H96a8,8,0,0,1,0-16h68A8,8,0,0,1,172,112Zm-8,24H96a8,8,0,0,0,0,16h68a8,8,0,0,0,0-16Zm68-12A100.11,100.11,0,0,1,132,224H48a16,16,0,0,1-16-16V124a100,100,0,0,1,200,0Zm-16,0a84,84,0,0,0-168,0v84h84A84.09,84.09,0,0,0,216,124Z"/></svg>"#;
const TAGS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M243.31,136,144,36.69A15.86,15.86,0,0,0,132.69,32H40a8,8,0,0,0-8,8v92.69A15.86,15.86,0,0,0,36.69,144L136,243.31a16,16,0,0,0,22.63,0l84.68-84.68a16,16,0,0,0,0-22.63Zm-96,96L48,132.69V48h84.69L232,147.31ZM96,84A12,12,0,1,1,84,72,12,12,0,0,1,96,84Z"/></svg>"#;
const BOOKMARK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M184,32H72A16,16,0,0,0,56,48V224a8,8,0,0,0,12.24,6.78L128,193.43l59.77,37.35A8,8,0,0,0,200,224V48A16,16,0,0,0,184,32Zm0,16V161.57l-51.77-32.35a8,8,0,0,0-8.48,0L72,161.56V48ZM132.23,177.22a8,8,0,0,0-8.48,0L72,209.57V180.43l56-35,56,35v29.14Z"/></svg>"#;
const SETTINGS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M64,105V40a8,8,0,0,0-16,0v65a32,32,0,0,0,0,62v49a8,8,0,0,0,16,0V167a32,32,0,0,0,0-62Zm-8,47a16,16,0,1,1,16-16A16,16,0,0,1,56,152Zm80-95V40a8,8,0,0,0-16,0V57a32,32,0,0,0,0,62v97a8,8,0,0,0,16,0V119a32,32,0,0,0,0-62Zm-8,47a16,16,0,1,1,16-16A16,16,0,0,1,128,104Zm104,64a32.06,32.06,0,0,0-24-31V40a8,8,0,0,0-16,0v97a32,32,0,0,0,0,62v17a8,8,0,0,0,16,0V199A32.06,32.06,0,0,0,232,168Zm-32,16a16,16,0,1,1,16-16A16,16,0,0,1,200,184Z"/></svg>"#;
const PLUS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M224,128a8,8,0,0,1-8,8H136v80a8,8,0,0,1-16,0V136H40a8,8,0,0,1,0-16h80V40a8,8,0,0,1,16,0v80h80A8,8,0,0,1,224,128Z"/></svg>"#;
const PLAY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M240,128a15.74,15.74,0,0,1-7.6,13.51L88.32,229.65a16,16,0,0,1-16.2.3A15.86,15.86,0,0,1,64,216.13V39.87a15.86,15.86,0,0,1,8.12-13.82,16,16,0,0,1,16.2.3L232.4,114.49A15.74,15.74,0,0,1,240,128Z"/></svg>"#;
const PAUSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M96,40V216a8,8,0,0,1-8,8H56a8,8,0,0,1-8-8V40a8,8,0,0,1,8-8H88A8,8,0,0,1,96,40Zm104-8H168a8,8,0,0,0-8,8V216a8,8,0,0,0,8,8h32a8,8,0,0,0,8-8V40A8,8,0,0,0,200,32Z"/></svg>"#;
const ROTATE_CW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" stroke="currentColor" stroke-width="18" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 256 256"><path d="M208 80a88 88 0 1 0 20 56"/><path d="M208 32v48h-48"/></svg>"#;
const FLIP_HORIZONTAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" stroke="currentColor" stroke-width="18" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 256 256"><path d="M128 24v208"/><path d="m96 72-56 56 56 56V72Z"/><path d="m160 72 56 56-56 56V72Z"/></svg>"#;
const FLIP_VERTICAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" stroke="currentColor" stroke-width="18" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 256 256"><path d="M24 128h208"/><path d="m72 96 56-56 56 56H72Z"/><path d="m72 160 56 56 56-56H72Z"/></svg>"#;
const CROP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" stroke="currentColor" stroke-width="18" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 256 256"><path d="M64 24v144a24 24 0 0 0 24 24h144"/><path d="M24 64h144a24 24 0 0 1 24 24v144"/></svg>"#;
const ZOOM_IN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" stroke="currentColor" stroke-width="18" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 256 256"><circle cx="112" cy="112" r="72"/><path d="M163 163l53 53"/><path d="M112 80v64"/><path d="M80 112h64"/></svg>"#;
const ZOOM_OUT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" stroke="currentColor" stroke-width="18" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 256 256"><circle cx="112" cy="112" r="72"/><path d="M163 163l53 53"/><path d="M80 112h64"/></svg>"#;
const TRAFFIC_CLOSE_DOT_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><circle r="6" fill="#ff5f56" stroke="#e0443e" stroke-width="0.6"/></svg>"##;
const TRAFFIC_CLOSE_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><path d="M-1.8 -1.8 L1.8 1.8 M1.8 -1.8 L-1.8 1.8" stroke="#4a0002" stroke-width="1.5" stroke-linecap="round"/></svg>"##;
const TRAFFIC_MINIMIZE_DOT_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><circle r="6" fill="#ffbd2e" stroke="#dea123" stroke-width="0.6"/></svg>"##;
const TRAFFIC_MINIMIZE_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><line x1="-2.4" y1="0" x2="2.4" y2="0" stroke="#5a3900" stroke-width="1.5" stroke-linecap="round"/></svg>"##;
const TRAFFIC_ZOOM_DOT_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><circle r="6" fill="#27c93f" stroke="#1aab29" stroke-width="0.6"/></svg>"##;
const TRAFFIC_ZOOM_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><g fill="#004200"><path d="M-2.1 2.1 L-2.1 -1.5 L1.5 2.1 Z"/><path d="M2.1 -2.1 L2.1 1.5 L-1.5 -2.1 Z"/></g></svg>"##;

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameAssets;

impl AssetSource for FrameAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let asset = match path {
            FRAME_FONT_PATH => Cow::Borrowed(FRAME_FONT_BYTES),
            ICON_FRAME => Cow::Borrowed(FRAME_ICON_SVG.as_bytes()),
            ICON_LAYOUT_LIST => Cow::Borrowed(LAYOUT_LIST_SVG.as_bytes()),
            ICON_TERMINAL => Cow::Borrowed(TERMINAL_SVG.as_bytes()),
            ICON_FILE_UP => Cow::Borrowed(FILE_UP_SVG.as_bytes()),
            ICON_FILE_DOWN => Cow::Borrowed(FILE_DOWN_SVG.as_bytes()),
            ICON_HARD_DRIVE => Cow::Borrowed(HARD_DRIVE_SVG.as_bytes()),
            ICON_FILE_VIDEO => Cow::Borrowed(FILE_VIDEO_SVG.as_bytes()),
            ICON_FILE_IMAGE => Cow::Borrowed(FILE_IMAGE_SVG.as_bytes()),
            ICON_MUSIC => Cow::Borrowed(MUSIC_SVG.as_bytes()),
            ICON_CAPTIONS => Cow::Borrowed(CAPTIONS_SVG.as_bytes()),
            ICON_TAGS => Cow::Borrowed(TAGS_SVG.as_bytes()),
            ICON_BOOKMARK => Cow::Borrowed(BOOKMARK_SVG.as_bytes()),
            ICON_SETTINGS => Cow::Borrowed(SETTINGS_SVG.as_bytes()),
            ICON_PLUS => Cow::Borrowed(PLUS_SVG.as_bytes()),
            ICON_PLAY => Cow::Borrowed(PLAY_SVG.as_bytes()),
            ICON_PAUSE => Cow::Borrowed(PAUSE_SVG.as_bytes()),
            ICON_ROTATE_CW => Cow::Borrowed(ROTATE_CW_SVG.as_bytes()),
            ICON_FLIP_HORIZONTAL => Cow::Borrowed(FLIP_HORIZONTAL_SVG.as_bytes()),
            ICON_FLIP_VERTICAL => Cow::Borrowed(FLIP_VERTICAL_SVG.as_bytes()),
            ICON_CROP => Cow::Borrowed(CROP_SVG.as_bytes()),
            ICON_ZOOM_IN => Cow::Borrowed(ZOOM_IN_SVG.as_bytes()),
            ICON_ZOOM_OUT => Cow::Borrowed(ZOOM_OUT_SVG.as_bytes()),
            ICON_TRAFFIC_CLOSE_DOT => Cow::Borrowed(TRAFFIC_CLOSE_DOT_SVG.as_bytes()),
            ICON_TRAFFIC_CLOSE_SYMBOL => Cow::Borrowed(TRAFFIC_CLOSE_SYMBOL_SVG.as_bytes()),
            ICON_TRAFFIC_MINIMIZE_DOT => Cow::Borrowed(TRAFFIC_MINIMIZE_DOT_SVG.as_bytes()),
            ICON_TRAFFIC_MINIMIZE_SYMBOL => Cow::Borrowed(TRAFFIC_MINIMIZE_SYMBOL_SVG.as_bytes()),
            ICON_TRAFFIC_ZOOM_DOT => Cow::Borrowed(TRAFFIC_ZOOM_DOT_SVG.as_bytes()),
            ICON_TRAFFIC_ZOOM_SYMBOL => Cow::Borrowed(TRAFFIC_ZOOM_SYMBOL_SVG.as_bytes()),
            _ => return Ok(None),
        };

        Ok(Some(asset))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let assets = match path {
            "fonts" => vec![SharedString::from("IoskeleyMono-Black.woff2")],
            "icons" => vec![
                SharedString::from("bookmark.svg"),
                SharedString::from("captions.svg"),
                SharedString::from("file-down.svg"),
                SharedString::from("file-image.svg"),
                SharedString::from("file-up.svg"),
                SharedString::from("file-video.svg"),
                SharedString::from("flip-horizontal.svg"),
                SharedString::from("flip-vertical.svg"),
                SharedString::from("frame.svg"),
                SharedString::from("hard-drive.svg"),
                SharedString::from("layout-list.svg"),
                SharedString::from("music.svg"),
                SharedString::from("pause.svg"),
                SharedString::from("play.svg"),
                SharedString::from("plus.svg"),
                SharedString::from("crop.svg"),
                SharedString::from("rotate-cw.svg"),
                SharedString::from("settings.svg"),
                SharedString::from("tags.svg"),
                SharedString::from("terminal.svg"),
                SharedString::from("traffic-close-dot.svg"),
                SharedString::from("traffic-close-symbol.svg"),
                SharedString::from("traffic-minimize-dot.svg"),
                SharedString::from("traffic-minimize-symbol.svg"),
                SharedString::from("traffic-zoom-dot.svg"),
                SharedString::from("traffic-zoom-symbol.svg"),
                SharedString::from("zoom-in.svg"),
                SharedString::from("zoom-out.svg"),
            ],
            _ => Vec::new(),
        };

        Ok(assets)
    }
}

pub fn load_frame_fonts(cx: &mut App) -> Result<()> {
    cx.text_system()
        .add_fonts(vec![Cow::Borrowed(FRAME_FONT_BYTES)])
}

#[cfg(test)]
mod tests {
    use super::*;

    mod frame_assets {
        use super::*;

        #[test]
        fn load_returns_frame_icon_svg() {
            let loaded = FrameAssets
                .load(ICON_FRAME)
                .expect("asset load should not fail");

            assert!(
                loaded
                    .as_deref()
                    .is_some_and(|bytes| bytes.starts_with(b"<svg"))
            );
        }

        #[test]
        fn load_returns_none_for_unknown_asset() {
            let loaded = FrameAssets
                .load("icons/missing.svg")
                .expect("asset load should not fail");

            assert!(loaded.is_none());
        }

        #[test]
        fn list_returns_titlebar_icon_assets() {
            let listed = FrameAssets
                .list("icons")
                .expect("asset list should not fail");

            assert!(listed.iter().any(|name| name.as_ref() == "layout-list.svg"));
            assert!(listed.iter().any(|name| name.as_ref() == "bookmark.svg"));
            assert!(
                listed
                    .iter()
                    .any(|name| name.as_ref() == "traffic-close-dot.svg")
            );
        }

        #[test]
        fn traffic_light_assets_preserve_original_svg_geometry() {
            let loaded = FrameAssets
                .load(ICON_TRAFFIC_CLOSE_DOT)
                .expect("asset load should not fail")
                .expect("traffic light asset should exist");
            let svg = std::str::from_utf8(loaded.as_ref()).expect("svg should be utf8");

            assert!(svg.contains(r#"viewBox="-10 -10 20 20""#));
            assert!(svg.contains(r#"r="6""#));
            assert!(svg.contains(r#"stroke-width="0.6""#));
        }

        #[test]
        fn frame_font_family_matches_bundled_font_name_table_family() {
            assert_eq!(FRAME_FONT_FAMILY, "Ioskeley Mono Heavy");
        }

        #[test]
        fn frame_font_alias_matches_original_css_family() {
            assert_eq!(FRAME_FONT_ALIAS, "LoskeleyMono");
        }
    }
}
