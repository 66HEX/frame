//! Bundled assets for the GPUI app.

use std::{
    borrow::Cow,
    sync::{Arc, OnceLock},
};

use gpui::{App, AssetSource, FontFeatures, FontWeight, Result, SharedString};

pub const FRAME_FONT_FAMILY: &str = "Instrument Sans";
pub const FRAME_FONT_WEIGHT: FontWeight = FontWeight::NORMAL;
pub const FRAME_FONT_ALIAS: &str = "InstrumentSans";
pub const FRAME_FONT_PATH: &str = "fonts/InstrumentSans-Variable.ttf";
pub const FRAME_FONT_FEATURE_TAGS: [(&str, u32); 4] =
    [("liga", 1), ("ss02", 1), ("ss05", 1), ("kern", 1)];
pub const ICON_FRAME: &str = "icons/frame.svg";
pub const ICON_ARROW_DOWN: &str = "icons/arrow-down.svg";
pub const ICON_LAYOUT_LIST: &str = "icons/layout-list.svg";
pub const ICON_LIST_CHECKS: &str = "icons/list-checks.svg";
pub const ICON_TERMINAL: &str = "icons/terminal.svg";
pub const ICON_CHECK: &str = "icons/check.svg";
pub const ICON_CHEVRONS_UP_DOWN: &str = "icons/chevrons-up-down.svg";
pub const ICON_CLOSE: &str = "icons/close.svg";
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
pub const ICON_MINUS: &str = "icons/minus.svg";
pub const ICON_PLAY: &str = "icons/play.svg";
pub const ICON_PAUSE: &str = "icons/pause.svg";
pub const ICON_PAUSE_2: &str = "icons/pause2.svg";
pub const ICON_ROTATE_CW: &str = "icons/rotate-cw.svg";
pub const ICON_FLIP_HORIZONTAL: &str = "icons/flip-horizontal.svg";
pub const ICON_FLIP_VERTICAL: &str = "icons/flip-vertical.svg";
pub const ICON_CROP: &str = "icons/crop.svg";
pub const ICON_SPINNER: &str = "icons/spinner.svg";
pub const ICON_SQUARE: &str = "icons/square.svg";
pub const ICON_TRASH: &str = "icons/trash.svg";
pub const ICON_TRAFFIC_CLOSE_SYMBOL: &str = "icons/traffic-close-symbol.svg";
pub const ICON_TRAFFIC_MINIMIZE_SYMBOL: &str = "icons/traffic-minimize-symbol.svg";
pub const ICON_TRAFFIC_ZOOM_SYMBOL: &str = "icons/traffic-zoom-symbol.svg";

const FRAME_ICON_SVG: &str = include_str!("../../assets/icons/frame.svg");
const FRAME_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/InstrumentSans-Variable.ttf");

const ARROW_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M205.66,149.66l-72,72a8,8,0,0,1-11.32,0l-72-72a8,8,0,0,1,11.32-11.32L120,196.69V40a8,8,0,0,1,16,0V196.69l58.34-58.35a8,8,0,0,1,11.32,11.32Z"/></svg>"#;
const LAYOUT_LIST_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M32,64a8,8,0,0,1,8-8H216a8,8,0,0,1,0,16H40A8,8,0,0,1,32,64Zm104,56H40a8,8,0,0,0,0,16h96a8,8,0,0,0,0-16Zm0,64H40a8,8,0,0,0,0,16h96a8,8,0,0,0,0-16Zm112-24a8,8,0,0,1-3.76,6.78l-64,40A8,8,0,0,1,168,200V120a8,8,0,0,1,12.24-6.78l64,40A8,8,0,0,1,248,160Zm-23.09,0L184,134.43v51.14Z"/></svg>"#;
const LIST_CHECKS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M149.61,85.71l-89.6,88a8,8,0,0,1-11.22,0L10.39,136a8,8,0,1,1,11.22-11.41L54.4,156.79l84-82.5a8,8,0,1,1,11.22,11.42Zm96.1-11.32a8,8,0,0,0-11.32-.1l-84,82.5-18.83-18.5a8,8,0,0,0-11.21,11.42l24.43,24a8,8,0,0,0,11.22,0l89.6-88A8,8,0,0,0,245.71,74.39Z"/></svg>"#;
const TERMINAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M117.31,134l-72,64a8,8,0,1,1-10.63-12L100,128,34.69,70A8,8,0,1,1,45.32,58l72,64a8,8,0,0,1,0,12ZM216,184H120a8,8,0,0,0,0,16h96a8,8,0,0,0,0-16Z"/></svg>"#;
const CHECK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L96,188.69,218.34,66.34a8,8,0,0,1,11.32,11.32Z"/></svg>"#;
const CHEVRONS_UP_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" fill="none"/><polyline points="80 176 128 224 176 176" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/><polyline points="80 80 128 32 176 80" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/></svg>"#;
const CLOSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M205.66,194.34a8,8,0,0,1-11.32,11.32L128,139.31,61.66,205.66a8,8,0,0,1-11.32-11.32L116.69,128,50.34,61.66A8,8,0,0,1,61.66,50.34L128,116.69l66.34-66.35a8,8,0,0,1,11.32,11.32L139.31,128Z"/></svg>"#;
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
const MINUS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M224,128a8,8,0,0,1-8,8H40a8,8,0,0,1,0-16H216A8,8,0,0,1,224,128Z"/></svg>"#;
const PLAY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M240,128a15.74,15.74,0,0,1-7.6,13.51L88.32,229.65a16,16,0,0,1-16.2.3A15.86,15.86,0,0,1,64,216.13V39.87a15.86,15.86,0,0,1,8.12-13.82,16,16,0,0,1,16.2.3L232.4,114.49A15.74,15.74,0,0,1,240,128Z"/></svg>"#;
const PAUSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M96,40V216a8,8,0,0,1-8,8H56a8,8,0,0,1-8-8V40a8,8,0,0,1,8-8H88A8,8,0,0,1,96,40Zm104-8H168a8,8,0,0,0-8,8V216a8,8,0,0,0,8,8h32a8,8,0,0,0,8-8V40A8,8,0,0,0,200,32Z"/></svg>"#;
const PAUSE_2_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M216,48V208a16,16,0,0,1-16,16H160a16,16,0,0,1-16-16V48a16,16,0,0,1,16-16h40A16,16,0,0,1,216,48ZM96,32H56A16,16,0,0,0,40,48V208a16,16,0,0,0,16,16H96a16,16,0,0,0,16-16V48A16,16,0,0,0,96,32Z"/></svg>"#;
const ROTATE_CW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M240,56v48a8,8,0,0,1-8,8H184a8,8,0,0,1,0-16H211.4L184.81,71.64l-.25-.24a80,80,0,1,0-1.67,114.78,8,8,0,0,1,11,11.63A95.44,95.44,0,0,1,128,224h-1.32A96,96,0,1,1,195.75,60L224,85.8V56a8,8,0,1,1,16,0Z"/></svg>"#;
const FLIP_HORIZONTAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M107.18,24.33a15.86,15.86,0,0,0-17.92,9.45l-.06.14-64,159.93A16,16,0,0,0,40,216h64a16,16,0,0,0,16-16V40A15.85,15.85,0,0,0,107.18,24.33ZM104,200H40l.06-.15L104,40Zm126.77-6.15-64-159.93-.06-.14A16,16,0,0,0,136,40V200a16,16,0,0,0,16,16h64a16,16,0,0,0,14.78-22.15ZM152,200V40l63.93,159.84.06.15Z"/></svg>"#;
const FLIP_VERTICAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M56,120H216a16,16,0,0,0,6.23-30.74l-.14-.06-159.93-64A16,16,0,0,0,40,40v64A16,16,0,0,0,56,120Zm0-80,.15.06L216,104H56l0-64Zm160,96H56a16,16,0,0,0-16,16v64a16,16,0,0,0,22.15,14.78l159.93-64,.14-.06A16,16,0,0,0,216,136ZM56.15,215.93,56,216V152H216Z"/></svg>"#;
const CROP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M240,192a8,8,0,0,1-8,8H200v32a8,8,0,0,1-16,0V200H64a8,8,0,0,1-8-8V72H24a8,8,0,0,1,0-16H56V24a8,8,0,0,1,16,0V184H232A8,8,0,0,1,240,192ZM96,72h88v88a8,8,0,0,0,16,0V64a8,8,0,0,0-8-8H96a8,8,0,0,0,0,16Z"/></svg>"#;
const SPINNER_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M136,32V64a8,8,0,0,1-16,0V32a8,8,0,0,1,16,0Zm88,88H192a8,8,0,0,0,0,16h32a8,8,0,0,0,0-16Zm-45.09,47.6a8,8,0,0,0-11.31,11.31l22.62,22.63a8,8,0,0,0,11.32-11.32ZM128,184a8,8,0,0,0-8,8v32a8,8,0,0,0,16,0V192A8,8,0,0,0,128,184ZM77.09,167.6,54.46,190.22a8,8,0,0,0,11.32,11.32L88.4,178.91A8,8,0,0,0,77.09,167.6ZM72,128a8,8,0,0,0-8-8H32a8,8,0,0,0,0,16H64A8,8,0,0,0,72,128ZM65.78,54.46A8,8,0,0,0,54.46,65.78L77.09,88.4A8,8,0,0,0,88.4,77.09Z"/></svg>"#;
const SQUARE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M208,32H48A16,16,0,0,0,32,48V208a16,16,0,0,0,16,16H208a16,16,0,0,0,16-16V48A16,16,0,0,0,208,32Zm0,176H48V48H208V208Z"/></svg>"#;
const TRASH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256"><path d="M216,48H176V40a24,24,0,0,0-24-24H104A24,24,0,0,0,80,40v8H40a8,8,0,0,0,0,16h8V208a16,16,0,0,0,16,16H192a16,16,0,0,0,16-16V64h8a8,8,0,0,0,0-16ZM96,40a8,8,0,0,1,8-8h48a8,8,0,0,1,8,8v8H96Zm96,168H64V64H192ZM112,104v64a8,8,0,0,1-16,0V104a8,8,0,0,1,16,0Zm48,0v64a8,8,0,0,1-16,0V104a8,8,0,0,1,16,0Z"/></svg>"#;
const TRAFFIC_CLOSE_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><path d="M-1.8 -1.8 L1.8 1.8 M1.8 -1.8 L-1.8 1.8" stroke="#4a0002" stroke-width="1.5" stroke-linecap="round"/></svg>"##;
const TRAFFIC_MINIMIZE_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><line x1="-2.4" y1="0" x2="2.4" y2="0" stroke="#5a3900" stroke-width="1.5" stroke-linecap="round"/></svg>"##;
const TRAFFIC_ZOOM_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><g fill="#004200"><path d="M-2.1 2.1 L-2.1 -1.5 L1.5 2.1 Z"/><path d="M2.1 -2.1 L2.1 1.5 L-1.5 -2.1 Z"/></g></svg>"##;

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameAssets;

impl AssetSource for FrameAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let asset = match path {
            FRAME_FONT_PATH => Cow::Borrowed(FRAME_FONT_BYTES),
            ICON_FRAME => Cow::Borrowed(FRAME_ICON_SVG.as_bytes()),
            ICON_ARROW_DOWN => Cow::Borrowed(ARROW_DOWN_SVG.as_bytes()),
            ICON_LAYOUT_LIST => Cow::Borrowed(LAYOUT_LIST_SVG.as_bytes()),
            ICON_LIST_CHECKS => Cow::Borrowed(LIST_CHECKS_SVG.as_bytes()),
            ICON_TERMINAL => Cow::Borrowed(TERMINAL_SVG.as_bytes()),
            ICON_CHECK => Cow::Borrowed(CHECK_SVG.as_bytes()),
            ICON_CHEVRONS_UP_DOWN => Cow::Borrowed(CHEVRONS_UP_DOWN_SVG.as_bytes()),
            ICON_CLOSE => Cow::Borrowed(CLOSE_SVG.as_bytes()),
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
            ICON_MINUS => Cow::Borrowed(MINUS_SVG.as_bytes()),
            ICON_PLAY => Cow::Borrowed(PLAY_SVG.as_bytes()),
            ICON_PAUSE => Cow::Borrowed(PAUSE_SVG.as_bytes()),
            ICON_PAUSE_2 => Cow::Borrowed(PAUSE_2_SVG.as_bytes()),
            ICON_ROTATE_CW => Cow::Borrowed(ROTATE_CW_SVG.as_bytes()),
            ICON_FLIP_HORIZONTAL => Cow::Borrowed(FLIP_HORIZONTAL_SVG.as_bytes()),
            ICON_FLIP_VERTICAL => Cow::Borrowed(FLIP_VERTICAL_SVG.as_bytes()),
            ICON_CROP => Cow::Borrowed(CROP_SVG.as_bytes()),
            ICON_SPINNER => Cow::Borrowed(SPINNER_SVG.as_bytes()),
            ICON_SQUARE => Cow::Borrowed(SQUARE_SVG.as_bytes()),
            ICON_TRASH => Cow::Borrowed(TRASH_SVG.as_bytes()),
            ICON_TRAFFIC_CLOSE_SYMBOL => Cow::Borrowed(TRAFFIC_CLOSE_SYMBOL_SVG.as_bytes()),
            ICON_TRAFFIC_MINIMIZE_SYMBOL => Cow::Borrowed(TRAFFIC_MINIMIZE_SYMBOL_SVG.as_bytes()),
            ICON_TRAFFIC_ZOOM_SYMBOL => Cow::Borrowed(TRAFFIC_ZOOM_SYMBOL_SVG.as_bytes()),
            _ => return Ok(None),
        };

        Ok(Some(asset))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let assets = match path {
            "fonts" => vec![SharedString::from("InstrumentSans-Variable.ttf")],
            "icons" => vec![
                SharedString::from("arrow-down.svg"),
                SharedString::from("bookmark.svg"),
                SharedString::from("captions.svg"),
                SharedString::from("check.svg"),
                SharedString::from("chevrons-up-down.svg"),
                SharedString::from("close.svg"),
                SharedString::from("crop.svg"),
                SharedString::from("file-down.svg"),
                SharedString::from("file-image.svg"),
                SharedString::from("file-up.svg"),
                SharedString::from("file-video.svg"),
                SharedString::from("flip-horizontal.svg"),
                SharedString::from("flip-vertical.svg"),
                SharedString::from("frame.svg"),
                SharedString::from("hard-drive.svg"),
                SharedString::from("layout-list.svg"),
                SharedString::from("list-checks.svg"),
                SharedString::from("minus.svg"),
                SharedString::from("music.svg"),
                SharedString::from("pause.svg"),
                SharedString::from("pause2.svg"),
                SharedString::from("play.svg"),
                SharedString::from("plus.svg"),
                SharedString::from("rotate-cw.svg"),
                SharedString::from("settings.svg"),
                SharedString::from("spinner.svg"),
                SharedString::from("square.svg"),
                SharedString::from("tags.svg"),
                SharedString::from("terminal.svg"),
                SharedString::from("trash.svg"),
                SharedString::from("traffic-close-symbol.svg"),
                SharedString::from("traffic-minimize-symbol.svg"),
                SharedString::from("traffic-zoom-symbol.svg"),
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

pub fn frame_font_features() -> FontFeatures {
    static FEATURES: OnceLock<FontFeatures> = OnceLock::new();

    FEATURES
        .get_or_init(|| {
            FontFeatures(Arc::new(
                FRAME_FONT_FEATURE_TAGS
                    .iter()
                    .map(|(tag, value)| ((*tag).to_string(), *value))
                    .collect(),
            ))
        })
        .clone()
}

#[cfg(test)]
mod tests;
