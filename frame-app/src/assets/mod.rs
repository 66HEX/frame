//! Bundled assets for the GPUI app.

use std::{
    borrow::Cow,
    sync::{Arc, OnceLock},
};

use gpui::{App, AssetSource, FontFeatures, Result, SharedString};

pub const FRAME_FONT_FAMILY: &str = "Overused Grotesk";
pub const FRAME_FONT_ALIAS: &str = "OverusedGrotesk";
pub const FRAME_FONT_REGULAR_PATH: &str = "fonts/OverusedGrotesk-Roman.ttf";
pub const FRAME_FONT_MEDIUM_PATH: &str = "fonts/OverusedGrotesk-Medium.ttf";
pub const FRAME_FONT_FEATURE_TAGS: [(&str, u32); 1] = [("kern", 1)];
pub const FRAME_TABULAR_NUMBER_FONT_FEATURE_TAG: (&str, u32) = ("tnum", 1);
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
const FRAME_FONT_REGULAR_BYTES: &[u8] =
    include_bytes!("../../assets/fonts/OverusedGrotesk-Roman.ttf");
const FRAME_FONT_MEDIUM_BYTES: &[u8] =
    include_bytes!("../../assets/fonts/OverusedGrotesk-Medium.ttf");

const ARROW_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M12 18.502V5.00195" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M18 13.002C18 13.002 13.5811 19.0019 12 19.002C10.4188 19.002 6 13.002 6 13.002" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const LAYOUT_LIST_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M2 11.4C2 10.2417 2.24173 10 3.4 10H20.6C21.7583 10 22 10.2417 22 11.4V12.6C22 13.7583 21.7583 14 20.6 14H3.4C2.24173 14 2 13.7583 2 12.6V11.4Z" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M2 3.4C2 2.24173 2.24173 2 3.4 2H20.6C21.7583 2 22 2.24173 22 3.4V4.6C22 5.75827 21.7583 6 20.6 6H3.4C2.24173 6 2 5.75827 2 4.6V3.4Z" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M2 19.4C2 18.2417 2.24173 18 3.4 18H20.6C21.7583 18 22 18.2417 22 19.4V20.6C22 21.7583 21.7583 22 20.6 22H3.4C2.24173 22 2 21.7583 2 20.6V19.4Z" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/></svg>"#;
const LIST_CHECKS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M11 6L21 6" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M11 12L21 12" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M11 18L21 18" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M3 7.39286C3 7.39286 4 8.04466 4.5 9C4.5 9 6 5.25 8 4" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M3 18.3929C3 18.3929 4 19.0447 4.5 20C4.5 20 6 16.25 8 15" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const TERMINAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M4.00004 17C4.00004 17 9.99999 12.5811 10 11C10 9.41884 4 5 4 5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M12 19H20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const CHECK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M5 13.2592L7.58583 15.9568C8.2525 16.6523 8.58583 17.0001 9.00004 17.0001C9.41425 17.0001 9.74759 16.6523 10.4143 15.9568L19 7.00006" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const CHEVRONS_UP_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M7 19C7 19 10.6824 14 12 14C13.3176 14 17 19 17 19" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M7 5.00004C7 5.00004 10.6824 9.99999 12 10C13.3176 10 17 5 17 5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const CLOSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M18 6L6.00081 17.9992M17.9992 18L6 6.00085" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const FILE_UP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M4 12L4 14.5442C4 17.7892 4 19.4117 4.88607 20.5107C5.06508 20.7327 5.26731 20.9349 5.48933 21.1139C6.58831 22 8.21082 22 11.4558 22C12.1614 22 12.5141 22 12.8372 21.886C12.9044 21.8623 12.9702 21.835 13.0345 21.8043C13.3436 21.6564 13.593 21.407 14.0919 20.9081L18.8284 16.1716C19.4065 15.5935 19.6955 15.3045 19.8478 14.9369C20 14.5694 20 14.1606 20 13.3431V10C20 6.22876 20 4.34315 18.8284 3.17157C17.6569 2 15.7712 2 12 2M13 21.5V21C13 18.1716 13 16.7574 13.8787 15.8787C14.7574 15 16.1716 15 19 15H19.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M10 5C9.41016 4.39316 7.84027 2 7 2C6.15973 2 4.58984 4.39316 4 5M7 3L7 10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const FILE_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M4 7C4.58984 7.60684 6.15973 10 7 10C7.84027 10 9.41016 7.60684 10 7M7 9L7 2" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M4 13L4 14.5442C4 17.7892 4 19.4117 4.88607 20.5107C5.06508 20.7327 5.26731 20.9349 5.48933 21.1139C6.58831 22 8.21082 22 11.4558 22C12.1614 22 12.5141 22 12.8372 21.886C12.9044 21.8623 12.9702 21.835 13.0345 21.8043C13.3436 21.6564 13.593 21.407 14.0919 20.9081L18.8284 16.1716C19.4065 15.5935 19.6955 15.3045 19.8478 14.9369C20 14.5694 20 14.1606 20 13.3431V10C20 6.22876 20 4.34315 18.8284 3.17157C17.6569 2 15.7712 2 12 2M13 21.5V21C13 18.1716 13 16.7574 13.8787 15.8787C14.7574 15 16.1716 15 19 15H19.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const HARD_DRIVE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M20.7104 8.70122L21.9186 12.7288C21.9578 12.8592 21.9773 12.9244 21.9879 12.9914L21.9908 13.0118C22 13.079 22 13.147 22 13.2831C22 16.7797 22 18.528 20.9812 19.6669C20.8824 19.7774 20.7774 19.8824 20.6669 19.9812C19.528 21 17.7797 21 14.2831 21H9.71685C6.22026 21 4.47197 21 3.33311 19.9812C3.22259 19.8824 3.11765 19.7774 3.01877 19.6669C2 18.528 2 16.7797 2 13.2831C2 13.147 2 13.079 2.00915 13.0118L2.01215 12.9914C2.02269 12.9244 2.04225 12.8592 2.08136 12.7288L3.28963 8.70122C4.11355 5.95484 4.5255 4.58166 5.5884 3.79083C6.6513 3 8.08495 3 10.9522 3H13.0478C15.9151 3 17.3487 3 18.4116 3.79083C19.4745 4.58166 19.8865 5.95484 20.7104 8.70122Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M2 13H22" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M18.125 17H18M14.125 17H14M18.25 17C18.25 17.1381 18.1381 17.25 18 17.25C17.8619 17.25 17.75 17.1381 17.75 17C17.75 16.8619 17.8619 16.75 18 16.75C18.1381 16.75 18.25 16.8619 18.25 17ZM14.25 17C14.25 17.1381 14.1381 17.25 14 17.25C13.8619 17.25 13.75 17.1381 13.75 17C13.75 16.8619 13.8619 16.75 14 16.75C14.1381 16.75 14.25 16.8619 14.25 17Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const FILE_VIDEO_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M19 14.0052V10.6606C19 9.84276 19 9.43383 18.8478 9.06613C18.6955 8.69843 18.4065 8.40927 17.8284 7.83096L13.0919 3.09236C12.593 2.59325 12.3436 2.3437 12.0345 2.19583C11.9702 2.16508 11.9044 2.13778 11.8372 2.11406C11.5141 2 11.1614 2 10.4558 2C7.21082 2 5.58831 2 4.48933 2.88646C4.26731 3.06554 4.06508 3.26787 3.88607 3.48998C3 4.58943 3 6.21265 3 9.45908V14.0052C3 17.7781 3 19.6645 4.17157 20.8366C5.11466 21.7801 6.52043 21.9641 9 22M12 2.50022V3.00043C12 5.83009 12 7.24492 12.8787 8.12398C13.7574 9.00304 15.1716 9.00304 18 9.00304H18.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M18 19.5L19.4453 20.4635C20.1297 20.9198 20.4719 21.1479 20.7359 21.0066C21 20.8653 21 20.454 21 19.6315V18.3685C21 17.546 21 17.1347 20.7359 16.9934C20.4719 16.8521 20.1297 17.0802 19.4453 17.5365L18 18.5M18 19.5V18.5M18 19.5C18 20.4346 18 20.9019 17.799 21.25C17.6674 21.478 17.478 21.6674 17.25 21.799C16.9019 22 16.4346 22 15.5 22H15C13.5858 22 12.8787 22 12.4393 21.5607C12 21.1213 12 20.4142 12 19C12 17.5858 12 16.8787 12.4393 16.4393C12.8787 16 13.5858 16 15 16H15.5C16.4346 16 16.9019 16 17.25 16.201C17.478 16.3326 17.6674 16.522 17.799 16.75C18 17.0981 18 17.5654 18 18.5" stroke="currentColor" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const FILE_IMAGE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><circle cx="9.5" cy="12.5" r="1.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M7.5 21.5L14.2929 14.7071C14.7456 14.2544 15.3597 14 16 14C16.6403 14 17.2544 14.2544 17.7071 14.7071L19.8138 16.8138" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M13 2.5V3C13 5.82843 13 7.24264 13.8787 8.12132C14.7574 9 16.1716 9 19 9H19.5M20 10.6569V14C20 17.7712 20 19.6569 18.8284 20.8284C17.6569 22 15.7712 22 12 22C8.22876 22 6.34315 22 5.17157 20.8284C4 19.6569 4 17.7712 4 14V9.45584C4 6.21082 4 4.58831 4.88607 3.48933C5.06508 3.26731 5.26731 3.06508 5.48933 2.88607C6.58831 2 8.21082 2 11.4558 2C12.1614 2 12.5141 2 12.8372 2.11401C12.9044 2.13772 12.9702 2.165 13.0345 2.19575C13.3436 2.34355 13.593 2.593 14.0919 3.09188L18.8284 7.82843C19.4065 8.40649 19.6955 8.69552 19.8478 9.06306C20 9.4306 20 9.83935 20 10.6569Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const MUSIC_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M7 9.5C7 10.8807 5.88071 12 4.5 12C3.11929 12 2 10.8807 2 9.5C2 8.11929 3.11929 7 4.5 7C5.88071 7 7 8.11929 7 9.5ZM7 9.5V2C7.33333 2.5 7.6 4.6 10 5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><circle cx="10.5" cy="19.5" r="2.5" stroke="currentColor" stroke-width="1.5"/><circle cx="20" cy="18" r="2" stroke="currentColor" stroke-width="1.5"/><path d="M13 19.5L13 11C13 10.09 13 9.63502 13.2466 9.35248C13.4932 9.06993 13.9938 9.00163 14.9949 8.86504C18.0085 8.45385 20.2013 7.19797 21.3696 6.42937C21.6498 6.24509 21.7898 6.15295 21.8949 6.20961C22 6.26627 22 6.43179 22 6.76283V17.9259" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M13 13C17.8 13 21 10.6667 22 10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const CAPTIONS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M15 5H9C6.19108 5 4.78661 5 3.77772 5.67412C3.34096 5.96596 2.96596 6.34096 2.67412 6.77772C2 7.78661 2 9.19108 2 12C2 14.8089 2 16.2134 2.67412 17.2223C2.96596 17.659 3.34096 18.034 3.77772 18.3259C4.78661 19 6.19108 19 9 19H15C17.8089 19 19.2134 19 20.2223 18.3259C20.659 18.034 21.034 17.659 21.3259 17.2223C22 16.2134 22 14.8089 22 12C22 9.19108 22 7.78661 21.3259 6.77772C21.034 6.34096 20.659 5.96596 20.2223 5.67412C19.2134 5 17.8089 5 15 5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M6 11H12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M16 11H18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M18 15H12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M8 15H6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const TAGS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M18.058 8.53645L17.058 7.92286C16.0553 7.30762 15.554 7 15 7C14.446 7 13.9447 7.30762 12.942 7.92286L11.942 8.53645C10.9935 9.11848 10.5192 9.40949 10.2596 9.87838C10 10.3473 10 10.9129 10 12.0442V17.9094C10 19.8377 10 20.8019 10.5858 21.4009C11.1716 22 12.1144 22 14 22H16C17.8856 22 18.8284 22 19.4142 21.4009C20 20.8019 20 19.8377 20 17.9094V12.0442C20 10.9129 20 10.3473 19.7404 9.87838C19.4808 9.40949 19.0065 9.11848 18.058 8.53645Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M14 7.10809C13.3612 6.4951 12.9791 6.17285 12.4974 6.05178C11.9374 5.91102 11.3491 6.06888 10.1725 6.3846L8.99908 6.69947C7.88602 6.99814 7.32949 7.14748 6.94287 7.5163C6.55624 7.88513 6.40642 8.40961 6.10679 9.45857L4.55327 14.8971C4.0425 16.6852 3.78712 17.5792 4.22063 18.2836C4.59336 18.8892 6.0835 19.6339 7.5 20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M14.4947 10C15.336 9.44058 16.0828 8.54291 16.5468 7.42653C17.5048 5.12162 16.8944 2.75724 15.1836 2.14554C13.4727 1.53383 11.3091 2.90644 10.3512 5.21135C10.191 5.59667 10.0747 5.98366 10 6.36383" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/></svg>"#;
const BOOKMARK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M4 17.9808V9.70753C4 6.07416 4 4.25748 5.17157 3.12874C6.34315 2 8.22876 2 12 2C15.7712 2 17.6569 2 18.8284 3.12874C20 4.25748 20 6.07416 20 9.70753V17.9808C20 20.2867 20 21.4396 19.2272 21.8523C17.7305 22.6514 14.9232 19.9852 13.59 19.1824C12.8168 18.7168 12.4302 18.484 12 18.484C11.5698 18.484 11.1832 18.7168 10.41 19.1824C9.0768 19.9852 6.26947 22.6514 4.77285 21.8523C4 21.4396 4 20.2867 4 17.9808Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const SETTINGS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M4 11L4 21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M19 13L19 21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M19 3L19 7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M11.5 3L11.5 13" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M4 3L4 5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M11.5 19L11.5 21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M2 9.5C2 9.03406 2 8.80109 2.07612 8.61732C2.17761 8.37229 2.37229 8.17761 2.61732 8.07612C2.80109 8 3.03406 8 3.5 8H4.5C4.96594 8 5.19891 8 5.38268 8.07612C5.62771 8.17761 5.82239 8.37229 5.92388 8.61732C6 8.80109 6 9.03406 6 9.5C6 9.96594 6 10.1989 5.92388 10.3827C5.82239 10.6277 5.62771 10.8224 5.38268 10.9239C5.19891 11 4.96594 11 4.5 11H3.5C3.03406 11 2.80109 11 2.61732 10.9239C2.37229 10.8224 2.17761 10.6277 2.07612 10.3827C2 10.1989 2 9.96594 2 9.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M17 11.5C17 11.0341 17 10.8011 17.0761 10.6173C17.1776 10.3723 17.3723 10.1776 17.6173 10.0761C17.8011 10 18.0341 10 18.5 10H19.5C19.9659 10 20.1989 10 20.3827 10.0761C20.6277 10.1776 20.8224 10.3723 20.9239 10.6173C21 10.8011 21 11.0341 21 11.5C21 11.9659 21 12.1989 20.9239 12.3827C20.8224 12.6277 20.6277 12.8224 20.3827 12.9239C20.1989 13 19.9659 13 19.5 13H18.5C18.0341 13 17.8011 13 17.6173 12.9239C17.3723 12.8224 17.1776 12.6277 17.0761 12.3827C17 12.1989 17 11.9659 17 11.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M9.5 14.5C9.5 14.0341 9.5 13.8011 9.57612 13.6173C9.67761 13.3723 9.87229 13.1776 10.1173 13.0761C10.3011 13 10.5341 13 11 13H12C12.4659 13 12.6989 13 12.8827 13.0761C13.1277 13.1776 13.3224 13.3723 13.4239 13.6173C13.5 13.8011 13.5 14.0341 13.5 14.5C13.5 14.9659 13.5 15.1989 13.4239 15.3827C13.3224 15.6277 13.1277 15.8224 12.8827 15.9239C12.6989 16 12.4659 16 12 16H11C10.5341 16 10.3011 16 10.1173 15.9239C9.87229 15.8224 9.67761 15.6277 9.57612 15.3827C9.5 15.1989 9.5 14.9659 9.5 14.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const PLUS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M12 4V20M20 12H4" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const MINUS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M20 12L4 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const PLAY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M18.8906 12.846C18.5371 14.189 16.8667 15.138 13.5257 17.0361C10.296 18.8709 8.6812 19.7884 7.37983 19.4196C6.8418 19.2671 6.35159 18.9776 5.95624 18.5787C5 17.6139 5 15.7426 5 12C5 8.2574 5 6.3861 5.95624 5.42132C6.35159 5.02245 6.8418 4.73288 7.37983 4.58042C8.6812 4.21165 10.296 5.12907 13.5257 6.96393C16.8667 8.86197 18.5371 9.811 18.8906 11.154C19.0365 11.7084 19.0365 12.2916 18.8906 12.846Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const PAUSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M4 7C4 5.58579 4 4.87868 4.43934 4.43934C4.87868 4 5.58579 4 7 4C8.41421 4 9.12132 4 9.56066 4.43934C10 4.87868 10 5.58579 10 7V17C10 18.4142 10 19.1213 9.56066 19.5607C9.12132 20 8.41421 20 7 20C5.58579 20 4.87868 20 4.43934 19.5607C4 19.1213 4 18.4142 4 17V7Z" stroke="currentColor" stroke-width="1.5"/><path d="M14 7C14 5.58579 14 4.87868 14.4393 4.43934C14.8787 4 15.5858 4 17 4C18.4142 4 19.1213 4 19.5607 4.43934C20 4.87868 20 5.58579 20 7V17C20 18.4142 20 19.1213 19.5607 19.5607C19.1213 20 18.4142 20 17 20C15.5858 20 14.8787 20 14.4393 19.5607C14 19.1213 14 18.4142 14 17V7Z" stroke="currentColor" stroke-width="1.5"/></svg>"#;
const PAUSE_2_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M4 7C4 5.58579 4 4.87868 4.43934 4.43934C4.87868 4 5.58579 4 7 4C8.41421 4 9.12132 4 9.56066 4.43934C10 4.87868 10 5.58579 10 7V17C10 18.4142 10 19.1213 9.56066 19.5607C9.12132 20 8.41421 20 7 20C5.58579 20 4.87868 20 4.43934 19.5607C4 19.1213 4 18.4142 4 17V7Z" stroke="currentColor" stroke-width="1.5"/><path d="M14 7C14 5.58579 14 4.87868 14.4393 4.43934C14.8787 4 15.5858 4 17 4C18.4142 4 19.1213 4 19.5607 4.43934C20 4.87868 20 5.58579 20 7V17C20 18.4142 20 19.1213 19.5607 19.5607C19.1213 20 18.4142 20 17 20C15.5858 20 14.8787 20 14.4393 19.5607C14 19.1213 14 18.4142 14 17V7Z" stroke="currentColor" stroke-width="1.5"/></svg>"#;
const ROTATE_CW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M14 7H15.6C18.617 7 20.1255 7 21.0627 7.91122C22 8.82245 22 10.289 22 13.2222V14.7778C22 17.711 22 19.1776 21.0627 20.0888C20.1255 21 18.617 21 15.6 21H12.4C9.38301 21 7.87452 21 6.93726 20.0888C6 19.1776 6 17.711 6 14.7778V11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M21.5 17.2857L17.4327 13.2712C17.2576 13.0984 17.0104 13 16.7513 13C16.5061 13 16.271 13.0881 16.0977 13.2449L12.4211 16.5714L10.7152 15.0281C10.5437 14.8729 10.3111 14.7857 10.0686 14.7857C9.80735 14.7857 9.5586 14.8868 9.38506 15.0634L6.5 18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M11.0004 7C10.0882 5.78555 8.63582 5 7 5C4.23858 5 2 7.23858 2 10C2 11.1258 2.37209 12.1647 3 13.0005M11.0004 7L10 3M11.0004 7L7.5 8" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M11.625 11.5H11.5M11.75 11.5C11.75 11.6381 11.6381 11.75 11.5 11.75C11.3619 11.75 11.25 11.6381 11.25 11.5C11.25 11.3619 11.3619 11.25 11.5 11.25C11.6381 11.25 11.75 11.3619 11.75 11.5Z" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/></svg>"#;
const FLIP_HORIZONTAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M5.40887 10.5448L3.33488 14.4677C2.27012 16.4816 1.73775 17.4886 2.13166 18.2453C2.52557 19.002 3.58215 19.002 5.6953 19.002H7.76929C9.05748 19.002 9.70157 19.002 10.1018 18.5604C10.502 18.1189 10.502 17.4082 10.502 15.9869V12.064C10.502 7.57709 10.502 5.33363 9.49221 5.03243C8.48246 4.73124 7.45793 6.66909 5.40887 10.5448Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.5"/><path d="M18.5931 10.5448L20.6671 14.4677C21.7318 16.4816 22.2642 17.4886 21.8703 18.2453C21.4764 19.002 20.4198 19.002 18.3067 19.002L16.2327 19.002C14.9445 19.002 14.3004 19.002 13.9002 18.5604C13.5 18.1189 13.5 17.4082 13.5 15.9869L13.5 12.064C13.5 7.57709 13.5 5.33363 14.5097 5.03243C15.5195 4.73124 16.544 6.66909 18.5931 10.5448Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const FLIP_VERTICAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M13.4572 5.40692L9.5343 3.33293C7.52036 2.26817 6.51339 1.73579 5.75669 2.12971C5 2.52362 5 3.58019 5 5.69334V7.76734C5 9.05553 5 9.69962 5.44155 10.0998C5.8831 10.5 6.59376 10.5 8.01508 10.5H11.9379C16.4249 10.5 18.6683 10.5 18.9695 9.49025C19.2707 8.48051 17.3329 7.45598 13.4572 5.40692Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.5"/><path d="M13.4572 18.5931L9.5343 20.6671C7.52036 21.7318 6.51339 22.2642 5.75669 21.8703C5 21.4764 5 20.4198 5 18.3067V16.2327C5 14.9445 5 14.3004 5.44155 13.9002C5.8831 13.5 6.59376 13.5 8.01508 13.5H11.9379C16.4249 13.5 18.6683 13.5 18.9695 14.5097C19.2707 15.5195 17.3329 16.544 13.4572 18.5931Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const CROP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M4 2V4M22 20H20M16.5 20H10C7.17157 20 5.75736 20 4.87868 19.1213C4 18.2426 4 16.8284 4 14V7.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/><path d="M20 22L20 12C20 8.22877 20 6.34315 18.8284 5.17158C17.6569 4 15.7712 4 12 4L2 4" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const SPINNER_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M12 3V6" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M12 18V21" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M21 12L18 12" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M6 12L3 12" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M18.3635 5.63672L16.2422 7.75804" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M7.75804 16.2422L5.63672 18.3635" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M18.3635 18.3635L16.2422 16.2422" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M7.75804 7.75804L5.63672 5.63672" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/></svg>"#;
const SQUARE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" stroke="currentColor" stroke-width="1.5"/></svg>"#;
const TRASH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M19.5 5.5L18.8803 15.5251C18.7219 18.0864 18.6428 19.3671 18.0008 20.2879C17.6833 20.7431 17.2747 21.1273 16.8007 21.416C15.8421 22 14.559 22 11.9927 22C9.42312 22 8.1383 22 7.17905 21.4149C6.7048 21.1257 6.296 20.7408 5.97868 20.2848C5.33688 19.3626 5.25945 18.0801 5.10461 15.5152L4.5 5.5" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M3 5.5H21M16.0557 5.5L15.3731 4.09173C14.9196 3.15626 14.6928 2.68852 14.3017 2.39681C14.215 2.3321 14.1231 2.27454 14.027 2.2247C13.5939 2 13.0741 2 12.0345 2C10.9688 2 10.436 2 9.99568 2.23412C9.8981 2.28601 9.80498 2.3459 9.71729 2.41317C9.32164 2.7167 9.10063 3.20155 8.65861 4.17126L8.05292 5.5" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M9.5 16.5L9.5 10.5" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/><path d="M14.5 16.5L14.5 10.5" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"/></svg>"#;
const TRAFFIC_CLOSE_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><path d="M-1.8 -1.8 L1.8 1.8 M1.8 -1.8 L-1.8 1.8" stroke="#4a0002" stroke-width="1.5" stroke-linecap="round"/></svg>"##;
const TRAFFIC_MINIMIZE_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><line x1="-2.4" y1="0" x2="2.4" y2="0" stroke="#5a3900" stroke-width="1.5" stroke-linecap="round"/></svg>"##;
const TRAFFIC_ZOOM_SYMBOL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><g fill="#004200"><path d="M-2.1 2.1 L-2.1 -1.5 L1.5 2.1 Z"/><path d="M2.1 -2.1 L2.1 1.5 L-1.5 -2.1 Z"/></g></svg>"##;

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameAssets;

impl AssetSource for FrameAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let asset = match path {
            FRAME_FONT_REGULAR_PATH => Cow::Borrowed(FRAME_FONT_REGULAR_BYTES),
            FRAME_FONT_MEDIUM_PATH => Cow::Borrowed(FRAME_FONT_MEDIUM_BYTES),
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
            "fonts" => vec![
                SharedString::from("OverusedGrotesk-Roman.ttf"),
                SharedString::from("OverusedGrotesk-Medium.ttf"),
            ],
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
    cx.text_system().add_fonts(frame_font_bytes())
}

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
