use gpui::WindowAppearance;
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
pub(crate) use objc2_app_kit::{
    NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSAppearanceNameVibrantDark,
    NSAppearanceNameVibrantLight,
};
use objc2_foundation::NSString;

/// Converts an AppKit `NSAppearance` object supplied by a native window or
/// application into GPUI's value type.
///
/// The raw object is only accepted at this FFI boundary; the selector result
/// is immediately owned and typed as `NSString` by objc2.
pub(crate) unsafe fn window_appearance_from_native(appearance: *mut AnyObject) -> WindowAppearance {
    let name: Retained<NSString> = unsafe { msg_send![&*appearance, name] };
    if name.isEqualToString(unsafe { NSAppearanceNameVibrantLight }) {
        WindowAppearance::VibrantLight
    } else if name.isEqualToString(unsafe { NSAppearanceNameVibrantDark }) {
        WindowAppearance::VibrantDark
    } else if name.isEqualToString(unsafe { NSAppearanceNameAqua }) {
        WindowAppearance::Light
    } else if name.isEqualToString(unsafe { NSAppearanceNameDarkAqua }) {
        WindowAppearance::Dark
    } else {
        log::warn!("unknown macOS appearance: {}", name);
        WindowAppearance::Light
    }
}
