use crate::{
    MacDisplay, TISCopyCurrentKeyboardInputSource, TISGetInputSourceProperty, WindowFrameSource,
    events::platform_input_from_native, kTISPropertyInputSourceIsASCIICapable,
    kTISPropertyInputSourceType, kTISTypeKeyboardInputMode, renderer,
};
#[cfg(any(test, feature = "test-support"))]
use anyhow::Result;
use block2::RcBlock;
use dispatch2::DispatchQueue;
use gpui::{
    AnyWindowHandle, BackgroundExecutor, Bounds, Capslock, CursorStyle, ExternalDragPayload,
    ExternalPaths, FileDropEvent, ForegroundExecutor, KeyDownEvent, Keystroke, Modifiers,
    ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    PlatformAtlas, PlatformDisplay, PlatformInput, PlatformInputHandler, PlatformWindow, Point,
    PromptButton, PromptLevel, RequestFrameOptions, SharedString, Size, SystemWindowTab,
    WindowAppearance, WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowKind,
    WindowParams, point, px, size,
};
#[cfg(any(test, feature = "test-support"))]
use image::RgbaImage;

use core_foundation::base::{CFRelease, CFTypeRef};
use core_foundation_sys::base::CFEqual;
use core_foundation_sys::number::{CFBooleanGetValue, CFBooleanRef};
use core_graphics::display::CGDirectDisplayID;
use ctor::ctor;
use foreign_types::ForeignTypeRef;
use futures::channel::oneshot;
use gpui_util::ResultExt;
use objc2::{
    MainThreadMarker, class, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject as Objc2Object, AnyProtocol, Bool, ClassBuilder, Sel},
    sel,
};
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSBackingStoreType, NSBeep, NSButton as Objc2NSButton,
    NSCursorFrameResizeDirections, NSCursorFrameResizePosition, NSEventModifierFlags, NSEventType,
    NSRequestUserAttentionType, NSScreen, NSView as Objc2NSView, NSViewLayerContentsRedrawPolicy,
    NSVisualEffectMaterial, NSVisualEffectState, NSWindow as Objc2NSWindow,
    NSWindowButton as Objc2NSWindowButton, NSWindowCollectionBehavior, NSWindowOcclusionState,
    NSWindowOrderingMode, NSWindowStyleMask, NSWindowTitleVisibility,
};
use objc2_foundation::{
    NSInteger, NSNotFound, NSOperatingSystemVersion, NSPoint as Objc2NSPoint, NSRange,
    NSRangePointer, NSRect as Objc2NSRect, NSSize, NSString, NSUInteger,
};
use parking_lot::Mutex;
use raw_window_handle as rwh;
use smallvec::SmallVec;
use std::{
    cell::Cell,
    ffi::{CStr, CString, c_void},
    mem,
    ops::Range,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    ptr::{self, NonNull},
    rc::Rc,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const WINDOW_STATE_IVAR: &str = "windowState";

unsafe fn set_window_state_ivar(object: ObjcId, state: *mut c_void) {
    let object = unsafe { &mut *object };
    let name = CString::new(WINDOW_STATE_IVAR).unwrap();
    let ivar = object.class().instance_variable(name.as_c_str()).unwrap();
    *unsafe { ivar.load_mut::<*mut c_void>(object) } = state;
}

unsafe fn window_state_ivar(object: &Objc2Object) -> &objc2::runtime::Ivar {
    let name = CString::new(WINDOW_STATE_IVAR).unwrap();
    object.class().instance_variable(name.as_c_str()).unwrap()
}

type ObjcId = *mut Objc2Object;
const NIL: ObjcId = ptr::null_mut();

fn ns_string(value: &str) -> Retained<NSString> {
    NSString::from_str(value)
}

fn filenames_pboard_type() -> Retained<NSString> {
    ns_string("NSFilenamesPboardType")
}

fn invalid_ns_range() -> NSRange {
    NSRange::new(NSNotFound as usize, 0)
}

trait NSRangeMethods {
    fn to_range_option(self) -> Option<Range<usize>>;
}

impl NSRangeMethods for NSRange {
    fn to_range_option(self) -> Option<Range<usize>> {
        (self.location != NSNotFound as usize).then(|| self.location..self.location + self.length)
    }
}

unsafe fn objc_string(value: ObjcId) -> String {
    if value.is_null() {
        return String::new();
    }
    let ptr: *const std::ffi::c_char = msg_send![value, UTF8String];
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

#[allow(non_snake_case)] // Mirrors Objective-C selectors at this raw runtime boundary.
trait Objc2WindowMessages {
    unsafe fn setStyleMask_(self, value: NSWindowStyleMask);
    unsafe fn setFrame_display_(self, value: Objc2NSRect, display: Bool);
    unsafe fn makeKeyAndOrderFront_(self, sender: ObjcId);
    unsafe fn makeFirstResponder_(self, responder: ObjcId) -> Bool;
    unsafe fn occlusionState(self) -> NSWindowOcclusionState;
    unsafe fn screen(self) -> ObjcId;
    unsafe fn visibleFrame(self) -> Objc2NSRect;
    unsafe fn styleMask(self) -> NSWindowStyleMask;
    unsafe fn contentView(self) -> ObjcId;
    unsafe fn initWithContentRect_styleMask_backing_defer_screen_(
        self,
        frame: Objc2NSRect,
        style: NSWindowStyleMask,
        backing: NSBackingStoreType,
        defer: Bool,
        screen: ObjcId,
    ) -> ObjcId;
    unsafe fn setAutoresizingMask_(self, mask: NSUInteger);
    unsafe fn setWantsLayer(self, value: Bool);
    unsafe fn autorelease(self) -> ObjcId;
    unsafe fn addSubview_(self, view: ObjcId);
    unsafe fn setLevel_(self, level: NSInteger);
    unsafe fn setAcceptsMouseMovedEvents_(self, value: Bool);
    unsafe fn setCollectionBehavior_(self, value: NSWindowCollectionBehavior);
    unsafe fn drain(self);
    unsafe fn setDelegate_(self, delegate: ObjcId);
    unsafe fn close(self);
    unsafe fn setContentSize_(self, value: NSSize);
    unsafe fn setContentMinSize_(self, value: NSSize);
    unsafe fn setMovable_(self, value: Bool);
    unsafe fn orderFront_(self, sender: ObjcId);
    unsafe fn mouseLocationOutsideOfEventStream(self) -> Objc2NSPoint;
    unsafe fn requestUserAttention_(self, value: NSRequestUserAttentionType) -> NSInteger;
    unsafe fn isKeyWindow(self) -> Bool;
    unsafe fn setOpaque_(self, value: Bool);
    unsafe fn setBackgroundColor_(self, value: ObjcId);
    unsafe fn miniaturize_(self, sender: ObjcId);
    unsafe fn zoom_(self, sender: ObjcId);
    unsafe fn toggleFullScreen_(self, sender: ObjcId);
    unsafe fn eventType(self) -> NSEventType;
    unsafe fn setTitlebarAppearsTransparent_(self, value: Bool);
    unsafe fn isEqualToString(self, value: &str) -> Bool;
    unsafe fn objectAtIndex(self, index: NSUInteger) -> ObjcId;
}

impl Objc2WindowMessages for ObjcId {
    unsafe fn setStyleMask_(self, value: NSWindowStyleMask) {
        let _: () = msg_send![self, setStyleMask: value];
    }
    unsafe fn setFrame_display_(self, value: Objc2NSRect, display: Bool) {
        let _: () = msg_send![self, setFrame: value, display: display];
    }
    unsafe fn makeKeyAndOrderFront_(self, sender: ObjcId) {
        let _: () = msg_send![self, makeKeyAndOrderFront: sender];
    }
    unsafe fn makeFirstResponder_(self, responder: ObjcId) -> Bool {
        msg_send![self, makeFirstResponder: responder]
    }
    unsafe fn occlusionState(self) -> NSWindowOcclusionState {
        msg_send![self, occlusionState]
    }
    unsafe fn screen(self) -> ObjcId {
        msg_send![self, screen]
    }
    unsafe fn visibleFrame(self) -> Objc2NSRect {
        msg_send![self, visibleFrame]
    }
    unsafe fn styleMask(self) -> NSWindowStyleMask {
        msg_send![self, styleMask]
    }
    unsafe fn contentView(self) -> ObjcId {
        msg_send![self, contentView]
    }
    unsafe fn initWithContentRect_styleMask_backing_defer_screen_(
        self,
        frame: Objc2NSRect,
        style: NSWindowStyleMask,
        backing: NSBackingStoreType,
        defer: Bool,
        screen: ObjcId,
    ) -> ObjcId {
        msg_send![self, initWithContentRect: frame, styleMask: style, backing: backing, defer: defer, screen: screen]
    }
    unsafe fn setAutoresizingMask_(self, mask: NSUInteger) {
        let _: () = msg_send![self, setAutoresizingMask: mask];
    }
    unsafe fn setWantsLayer(self, value: Bool) {
        let _: () = msg_send![self, setWantsLayer: value];
    }
    unsafe fn autorelease(self) -> ObjcId {
        msg_send![self, autorelease]
    }
    unsafe fn addSubview_(self, view: ObjcId) {
        let _: () = msg_send![self, addSubview: view];
    }
    unsafe fn setLevel_(self, level: NSInteger) {
        let _: () = msg_send![self, setLevel: level];
    }
    unsafe fn setAcceptsMouseMovedEvents_(self, value: Bool) {
        let _: () = msg_send![self, setAcceptsMouseMovedEvents: value];
    }
    unsafe fn setCollectionBehavior_(self, value: NSWindowCollectionBehavior) {
        let _: () = msg_send![self, setCollectionBehavior: value];
    }
    unsafe fn drain(self) {
        let _: () = msg_send![self, drain];
    }
    unsafe fn setDelegate_(self, delegate: ObjcId) {
        let _: () = msg_send![self, setDelegate: delegate];
    }
    unsafe fn close(self) {
        let _: () = msg_send![self, close];
    }
    unsafe fn setContentSize_(self, value: NSSize) {
        let _: () = msg_send![self, setContentSize: value];
    }
    unsafe fn setContentMinSize_(self, value: NSSize) {
        let _: () = msg_send![self, setContentMinSize: value];
    }
    unsafe fn setMovable_(self, value: Bool) {
        let _: () = msg_send![self, setMovable: value];
    }
    unsafe fn orderFront_(self, sender: ObjcId) {
        let _: () = msg_send![self, orderFront: sender];
    }
    unsafe fn mouseLocationOutsideOfEventStream(self) -> Objc2NSPoint {
        msg_send![self, mouseLocationOutsideOfEventStream]
    }
    unsafe fn requestUserAttention_(self, value: NSRequestUserAttentionType) -> NSInteger {
        msg_send![self, requestUserAttention: value]
    }
    unsafe fn isKeyWindow(self) -> Bool {
        msg_send![self, isKeyWindow]
    }
    unsafe fn setOpaque_(self, value: Bool) {
        let _: () = msg_send![self, setOpaque: value];
    }
    unsafe fn setBackgroundColor_(self, value: ObjcId) {
        let _: () = msg_send![self, setBackgroundColor: value];
    }
    unsafe fn miniaturize_(self, sender: ObjcId) {
        let _: () = msg_send![self, miniaturize: sender];
    }
    unsafe fn zoom_(self, sender: ObjcId) {
        let _: () = msg_send![self, zoom: sender];
    }
    unsafe fn toggleFullScreen_(self, sender: ObjcId) {
        let _: () = msg_send![self, toggleFullScreen: sender];
    }
    unsafe fn eventType(self) -> NSEventType {
        msg_send![self, type]
    }
    unsafe fn setTitlebarAppearsTransparent_(self, value: Bool) {
        let _: () = msg_send![self, setTitlebarAppearsTransparent: value];
    }
    unsafe fn isEqualToString(self, value: &str) -> Bool {
        let value = ns_string(value);
        msg_send![self, isEqualToString: &*value]
    }
    unsafe fn objectAtIndex(self, index: NSUInteger) -> ObjcId {
        msg_send![self, objectAtIndex: index]
    }
}

static mut WINDOW_CLASS: *const AnyClass = ptr::null();
static mut PANEL_CLASS: *const AnyClass = ptr::null();
static mut VIEW_CLASS: *const AnyClass = ptr::null();
static mut BLURRED_VIEW_CLASS: *const AnyClass = ptr::null();

#[allow(non_upper_case_globals)]
const VIEW_WIDTH_SIZABLE: NSUInteger = 1 << 1;
const VIEW_HEIGHT_SIZABLE: NSUInteger = 1 << 4;
// WindowLevel const value ref: https://docs.rs/core-graphics2/0.4.1/src/core_graphics2/window_level.rs.html
#[allow(non_upper_case_globals)]
const NSNormalWindowLevel: NSInteger = 0;
#[allow(non_upper_case_globals)]
const NSFloatingWindowLevel: NSInteger = 3;
#[allow(non_upper_case_globals)]
const NSPopUpWindowLevel: NSInteger = 101;
#[allow(non_upper_case_globals)]
const NSTrackingMouseEnteredAndExited: NSUInteger = 0x01;
#[allow(non_upper_case_globals)]
const NSTrackingMouseMoved: NSUInteger = 0x02;
#[allow(non_upper_case_globals)]
const NSTrackingActiveAlways: NSUInteger = 0x80;
#[allow(non_upper_case_globals)]
const NSTrackingInVisibleRect: NSUInteger = 0x200;
#[allow(non_upper_case_globals)]
const NSWindowAnimationBehaviorUtilityWindow: NSInteger = 4;
#[allow(non_upper_case_globals)]
// https://developer.apple.com/documentation/appkit/nsdragoperation
type NSDragOperation = NSUInteger;
#[allow(non_upper_case_globals)]
const NSDragOperationNone: NSDragOperation = 0;
#[allow(non_upper_case_globals)]
const NSDragOperationCopy: NSDragOperation = 1;
#[allow(non_upper_case_globals)]
const NSDragOperationMove: NSDragOperation = 16;
const NSDRAGGING_CONTEXT_OUTSIDE_APPLICATION: NSInteger = 0;
const NSDRAGGING_CONTEXT_WITHIN_APPLICATION: NSInteger = 1;
#[derive(PartialEq)]
pub enum UserTabbingPreference {
    Never,
    Always,
    InFullScreen,
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    // AppKit constant naming the icon component of an NSDraggingImageComponent.
    #[allow(non_upper_case_globals)]
    static NSDraggingImageComponentIconKey: ObjcId;
}
#[ctor(unsafe)]
unsafe fn build_classes() {
    unsafe {
        WINDOW_CLASS = build_window_class("GPUIWindow", class!(NSWindow), false);
        PANEL_CLASS = build_window_class("GPUIPanel", class!(NSPanel), true);
        VIEW_CLASS = {
            let mut decl =
                ClassBuilder::new(CString::new("GPUIView").unwrap().as_c_str(), class!(NSView))
                    .unwrap();
            decl.add_ivar::<*mut c_void>(CString::new(WINDOW_STATE_IVAR).unwrap().as_c_str());
            decl.add_method(sel!(dealloc), dealloc_view as unsafe extern "C" fn(_, _));

            decl.add_method(
                sel!(performKeyEquivalent:),
                handle_key_equivalent as unsafe extern "C" fn(_, _, _) -> _,
            );
            decl.add_method(
                sel!(keyDown:),
                handle_key_down as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(sel!(keyUp:), handle_key_up as unsafe extern "C" fn(_, _, _));
            decl.add_method(
                sel!(mouseDown:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(mouseUp:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(rightMouseDown:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(rightMouseUp:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(otherMouseDown:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(otherMouseUp:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(mouseMoved:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(resetCursorRects),
                reset_cursor_rects as unsafe extern "C" fn(_, _),
            );
            decl.add_method(
                sel!(pressureChangeWithEvent:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(mouseExited:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(magnifyWithEvent:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(mouseDragged:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(rightMouseDragged:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(otherMouseDragged:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(scrollWheel:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(swipeWithEvent:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(flagsChanged:),
                handle_view_event as unsafe extern "C" fn(_, _, _),
            );

            decl.add_method(
                sel!(makeBackingLayer),
                make_backing_layer as unsafe extern "C" fn(_, _) -> ObjcId,
            );

            decl.add_protocol(
                AnyProtocol::get(CString::new("CALayerDelegate").unwrap().as_c_str()).unwrap(),
            );
            decl.add_method(
                sel!(viewDidChangeBackingProperties),
                view_did_change_backing_properties as unsafe extern "C" fn(_, _),
            );
            decl.add_method(
                sel!(setFrameSize:),
                set_frame_size as unsafe extern "C" fn(_, _, _),
            );
            decl.add_method(
                sel!(displayLayer:),
                display_layer as unsafe extern "C" fn(_, _, _),
            );

            decl.add_protocol(
                AnyProtocol::get(CString::new("NSTextInputClient").unwrap().as_c_str()).unwrap(),
            );
            decl.add_method(
                sel!(validAttributesForMarkedText),
                valid_attributes_for_marked_text as unsafe extern "C" fn(_, _) -> ObjcId,
            );
            decl.add_method(
                sel!(hasMarkedText),
                has_marked_text as unsafe extern "C" fn(_, _) -> _,
            );
            decl.add_method(
                sel!(markedRange),
                marked_range as unsafe extern "C" fn(_, _) -> NSRange,
            );
            decl.add_method(
                sel!(selectedRange),
                selected_range as unsafe extern "C" fn(_, _) -> NSRange,
            );
            decl.add_method(
                sel!(firstRectForCharacterRange:actualRange:),
                first_rect_for_character_range as unsafe extern "C" fn(_, _, _, _) -> _,
            );
            decl.add_method(
                sel!(insertText:replacementRange:),
                insert_text as unsafe extern "C" fn(_, _, _, _),
            );
            decl.add_method(
                sel!(setMarkedText:selectedRange:replacementRange:),
                set_marked_text as unsafe extern "C" fn(_, _, _, _, _),
            );
            decl.add_method(sel!(unmarkText), unmark_text as unsafe extern "C" fn(_, _));
            decl.add_method(
                sel!(attributedSubstringForProposedRange:actualRange:),
                attributed_substring_for_proposed_range as unsafe extern "C" fn(_, _, _, _) -> _,
            );
            decl.add_method(
                sel!(viewDidChangeEffectiveAppearance),
                view_did_change_effective_appearance as unsafe extern "C" fn(_, _),
            );

            // Suppress beep on keystrokes with modifier keys.
            decl.add_method(
                sel!(doCommandBySelector:),
                do_command_by_selector as unsafe extern "C" fn(_, _, _),
            );

            decl.add_method(
                sel!(acceptsFirstMouse:),
                accepts_first_mouse as unsafe extern "C" fn(_, _, _) -> _,
            );

            decl.add_method(
                sel!(_opaqueRectForWindowMoveWhenInTitlebar),
                opaque_rect_for_window_move_when_in_titlebar
                    as unsafe extern "C" fn(_, _) -> Objc2NSRect,
            );

            decl.add_method(
                sel!(characterIndexForPoint:),
                character_index_for_point as unsafe extern "C" fn(_, _, _) -> _,
            );
            decl.register() as *const AnyClass
        };
        BLURRED_VIEW_CLASS = {
            let mut decl = ClassBuilder::new(
                CString::new("BlurredView").unwrap().as_c_str(),
                class!(NSVisualEffectView),
            )
            .unwrap();
            decl.add_method(
                sel!(initWithFrame:),
                blurred_view_init_with_frame as unsafe extern "C" fn(_, _, _) -> _,
            );
            decl.add_method(
                sel!(updateLayer),
                blurred_view_update_layer as unsafe extern "C" fn(_, _),
            );
            decl.register() as *const AnyClass
        };
    }
}

pub(crate) fn convert_mouse_position(
    position: Objc2NSPoint,
    window_height: Pixels,
) -> Point<Pixels> {
    point(
        px(position.x as f32),
        // macOS screen coordinates are relative to bottom left
        window_height - px(position.y as f32),
    )
}

/// Stores the cursor style on the active GPUI window and invalidates its cursor rects.
///
/// # Safety
///
/// This function is not thread safe. Callers must ensure this is called on the AppKit main
/// thread because it reads the active AppKit window and updates GPUI window state associated
/// with Objective-C objects.
pub(crate) unsafe fn set_active_window_cursor_style(style: CursorStyle) {
    // SAFETY: The caller guarantees AppKit main-thread access. `is_gpui_window` ensures the
    // window has our WINDOW_STATE_IVAR before reading it.
    unsafe {
        let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
        let key_window: ObjcId = msg_send![app, keyWindow];
        let main_window: ObjcId = msg_send![app, mainWindow];
        let active_window = if !key_window.is_null() && is_gpui_window(key_window) {
            Some(key_window)
        } else if !main_window.is_null() && is_gpui_window(main_window) {
            Some(main_window)
        } else {
            None
        };

        let Some(active_window) = active_window else {
            return;
        };

        let window_state = get_window_state(&*active_window);
        let mut window_state = window_state.lock();
        if window_state.cursor_style != style {
            window_state.cursor_style = style;
            let _: () = msg_send![
                window_state.native_window,
                invalidateCursorRectsForView: window_state.native_view.as_ptr()
            ];
        }
    }
}

unsafe fn build_window_class(
    name: &'static str,
    superclass: &AnyClass,
    is_panel: bool,
) -> *const AnyClass {
    unsafe {
        let mut decl =
            ClassBuilder::new(CString::new(name).unwrap().as_c_str(), superclass).unwrap();
        decl.add_ivar::<*mut c_void>(CString::new(WINDOW_STATE_IVAR).unwrap().as_c_str());
        if is_panel {
            decl.add_method(sel!(dealloc), dealloc_panel as unsafe extern "C" fn(_, _));
        } else {
            decl.add_method(sel!(dealloc), dealloc_window as unsafe extern "C" fn(_, _));
        }

        decl.add_method(
            sel!(canBecomeMainWindow),
            yes as unsafe extern "C" fn(_, _) -> _,
        );
        decl.add_method(
            sel!(canBecomeKeyWindow),
            yes as unsafe extern "C" fn(_, _) -> _,
        );
        decl.add_method(
            sel!(windowDidResize:),
            window_did_resize as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowDidChangeOcclusionState:),
            window_did_change_occlusion_state as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowWillEnterFullScreen:),
            window_will_enter_fullscreen as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowWillExitFullScreen:),
            window_will_exit_fullscreen as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowDidExitFullScreen:),
            window_did_exit_fullscreen as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowDidMove:),
            window_did_move as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowDidChangeScreen:),
            window_did_change_screen as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowDidBecomeKey:),
            window_did_change_key_status as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowDidResignKey:),
            window_did_change_key_status as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(windowShouldClose:),
            window_should_close as unsafe extern "C" fn(_, _, _) -> _,
        );

        decl.add_method(sel!(close), close_window as unsafe extern "C" fn(_, _));

        decl.add_method(
            sel!(draggingEntered:),
            dragging_entered as unsafe extern "C" fn(_, _, _) -> NSDragOperation,
        );
        decl.add_method(
            sel!(draggingUpdated:),
            dragging_updated as unsafe extern "C" fn(_, _, _) -> NSDragOperation,
        );
        decl.add_method(
            sel!(draggingExited:),
            dragging_exited as unsafe extern "C" fn(_, _, _),
        );
        decl.add_method(
            sel!(performDragOperation:),
            perform_drag_operation as unsafe extern "C" fn(_, _, _) -> _,
        );
        decl.add_method(
            sel!(concludeDragOperation:),
            conclude_drag_operation as unsafe extern "C" fn(_, _, _),
        );

        decl.add_protocol(
            AnyProtocol::get(CString::new("NSDraggingSource").unwrap().as_c_str()).unwrap(),
        );
        decl.add_method(
            sel!(draggingSession:sourceOperationMaskForDraggingContext:),
            dragging_session_source_operation_mask as unsafe extern "C" fn(_, _, _, _) -> _,
        );
        decl.add_method(
            sel!(draggingSession:endedAtPoint:operation:),
            dragging_session_ended as unsafe extern "C" fn(_, _, _, _, _),
        );

        decl.add_method(
            sel!(addTitlebarAccessoryViewController:),
            add_titlebar_accessory_view_controller as unsafe extern "C" fn(_, _, _),
        );

        decl.add_method(
            sel!(moveTabToNewWindow:),
            move_tab_to_new_window as unsafe extern "C" fn(_, _, _),
        );

        decl.add_method(
            sel!(mergeAllWindows:),
            merge_all_windows as unsafe extern "C" fn(_, _, _),
        );

        decl.add_method(
            sel!(selectNextTab:),
            select_next_tab as unsafe extern "C" fn(_, _, _),
        );

        decl.add_method(
            sel!(selectPreviousTab:),
            select_previous_tab as unsafe extern "C" fn(_, _, _),
        );

        decl.add_method(
            sel!(toggleTabBar:),
            toggle_tab_bar as unsafe extern "C" fn(_, _, _),
        );

        decl.register() as *const AnyClass
    }
}

struct TrafficLightFrames {
    titlebar: Objc2NSRect,
    close: Objc2NSRect,
    minimize: Objc2NSRect,
    zoom: Objc2NSRect,
}

struct TrafficLightButtons {
    close: Retained<Objc2NSButton>,
    minimize: Retained<Objc2NSButton>,
    zoom: Retained<Objc2NSButton>,
}

// `NSApplicationPresentationOptions` bits (see `NSApplication.PresentationOptions`).
const NS_APPLICATION_PRESENTATION_AUTO_HIDE_DOCK: NSUInteger = 1 << 0;
const NS_APPLICATION_PRESENTATION_AUTO_HIDE_MENU_BAR: NSUInteger = 1 << 2;

// State captured when entering simple (borderless) fullscreen, used to restore
// the window on exit.
struct SimpleFullscreenState {
    frame: Objc2NSRect,
    bounds: Bounds<Pixels>,
    style_mask: NSWindowStyleMask,
}

enum SimpleFullscreenPlan {
    Enter { screen_frame: Objc2NSRect },
    Exit(SimpleFullscreenState),
}

struct SimpleFullscreenAppState {
    window_count: usize,
    saved_presentation_options: NSUInteger,
}

static SIMPLE_FULLSCREEN_APP_STATE: Mutex<Option<SimpleFullscreenAppState>> = Mutex::new(None);

unsafe fn push_simple_fullscreen_presentation_options() {
    let mut app_state = SIMPLE_FULLSCREEN_APP_STATE.lock();
    match app_state.as_mut() {
        Some(app_state) => app_state.window_count += 1,
        None => unsafe {
            let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
            let saved_presentation_options: NSUInteger = msg_send![app, presentationOptions];
            let _: () = msg_send![
                app,
                setPresentationOptions: NS_APPLICATION_PRESENTATION_AUTO_HIDE_DOCK
                    | NS_APPLICATION_PRESENTATION_AUTO_HIDE_MENU_BAR
            ];
            *app_state = Some(SimpleFullscreenAppState {
                window_count: 1,
                saved_presentation_options,
            });
        },
    }
}

unsafe fn pop_simple_fullscreen_presentation_options() {
    let mut app_state = SIMPLE_FULLSCREEN_APP_STATE.lock();
    if let Some(state) = app_state.as_mut() {
        state.window_count = state.window_count.saturating_sub(1);
        if state.window_count == 0 {
            unsafe {
                let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
                let _: () = msg_send![
                    app,
                    setPresentationOptions: state.saved_presentation_options
                ];
            }
            *app_state = None;
        }
    }
}

unsafe fn apply_simple_fullscreen_plan(
    native_window: ObjcId,
    native_view: ObjcId,
    plan: SimpleFullscreenPlan,
) {
    unsafe {
        match plan {
            SimpleFullscreenPlan::Exit(saved) => {
                pop_simple_fullscreen_presentation_options();
                native_window.setStyleMask_(saved.style_mask);
                native_window.setFrame_display_(saved.frame, Bool::new(true));
            }
            SimpleFullscreenPlan::Enter { screen_frame } => {
                push_simple_fullscreen_presentation_options();
                native_window.setStyleMask_(NSWindowStyleMask::Borderless);
                native_window.setFrame_display_(screen_frame, Bool::new(true));
            }
        }

        // Changing the style mask makes AppKit resign the window's key status and
        // first responder, so keyboard input stops reaching the editor. Re-make the
        // window key and restore the GPUI view as first responder.
        native_window.makeKeyAndOrderFront_(NIL);
        native_window.makeFirstResponder_(native_view);
    }
}

struct MacWindowState {
    handle: AnyWindowHandle,
    foreground_executor: ForegroundExecutor,
    background_executor: BackgroundExecutor,
    native_window: ObjcId,
    native_view: NonNull<Objc2Object>,
    blurred_view: Option<ObjcId>,
    background_appearance: WindowBackgroundAppearance,
    cursor_style: CursorStyle,
    cursor_visible: Arc<AtomicBool>,
    frame_source: Option<WindowFrameSource>,
    renderer: renderer::Renderer,
    /// Forces an uncached scene after GPU recovery or a transient presentation failure.
    force_render_pending: bool,
    request_frame_callback: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    event_callback: Option<Box<dyn FnMut(PlatformInput) -> gpui::DispatchEventResult>>,
    activate_callback: Option<Box<dyn FnMut(bool)>>,
    resize_callback: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    moved_callback: Option<Box<dyn FnMut()>>,
    should_close_callback: Option<Box<dyn FnMut() -> bool>>,
    close_callback: Option<Box<dyn FnOnce()>>,
    appearance_changed_callback: Option<Box<dyn FnMut()>>,
    input_handler: Option<PlatformInputHandler>,
    last_key_equivalent: Option<KeyDownEvent>,
    last_left_mouse_down_event: Option<Retained<Objc2Object>>,
    synthetic_drag_counter: usize,
    traffic_light_position: Option<Point<Pixels>>,
    traffic_light_frames: Option<TrafficLightFrames>,
    transparent_titlebar: bool,
    previous_modifiers_changed_event: Option<PlatformInput>,
    keystroke_for_do_command: Option<Keystroke>,
    do_command_handled: Option<bool>,
    external_files_dragged: bool,
    // Whether the next left-mouse click is also the focusing click.
    first_mouse: bool,
    // When true, the whole content view is reported as app-owned titlebar content via
    // `_opaqueRectForWindowMoveWhenInTitlebar`, so AppKit does not drag the window from
    // the titlebar or delay titlebar clicks (a delay first observed on macOS 27). Such
    // windows draw their own titlebar and move the window via `start_window_move`.
    app_owns_titlebar_drag: bool,
    fullscreen_restore_bounds: Bounds<Pixels>,
    simple_fullscreen_state: Option<SimpleFullscreenState>,
    move_tab_to_new_window_callback: Option<Box<dyn FnMut()>>,
    merge_all_windows_callback: Option<Box<dyn FnMut()>>,
    select_next_tab_callback: Option<Box<dyn FnMut()>>,
    select_previous_tab_callback: Option<Box<dyn FnMut()>>,
    toggle_tab_bar_callback: Option<Box<dyn FnMut()>>,
    activated_least_once: bool,
    closed: Arc<AtomicBool>,
    accesskit_adapter: Option<accesskit_macos::SubclassingAdapter>,
    // The parent window if this window is a sheet (Dialog kind)
    sheet_parent: Option<ObjcId>,
}

impl MacWindowState {
    fn next_frame_request(&mut self) -> RequestFrameOptions {
        RequestFrameOptions {
            force_render: mem::take(&mut self.force_render_pending),
            ..Default::default()
        }
    }

    fn move_traffic_light(&mut self) {
        if let Some(traffic_light_position) = self.traffic_light_position {
            if self.is_fullscreen() {
                self.restore_traffic_light();
                return;
            }

            if self.traffic_light_frames.is_none() {
                self.traffic_light_frames = self.capture_traffic_light_frames();
            }

            let window_height = Pixels::from(self.native_window().frame().size.height);
            if self.traffic_light_frames.is_some() {
                // AppKit can recreate standard buttons, so fetch the live views for each layout pass.
                let Some(buttons) = self.traffic_light_buttons() else {
                    return;
                };
                let Some(titlebar_container) = Self::titlebar_container(&buttons.close) else {
                    return;
                };

                let close_frame = buttons.close.frame();
                let minimize_frame = buttons.minimize.frame();
                let button_width = Pixels::from(close_frame.size.width);
                let button_height = Pixels::from(close_frame.size.height);
                let button_padding = Pixels::from(
                    minimize_frame.origin.x - close_frame.origin.x - close_frame.size.width,
                );
                let container_height =
                    button_height + traffic_light_position.y + traffic_light_position.y;

                let mut titlebar_frame = titlebar_container.frame();
                titlebar_frame.size.height = container_height.to_f64();
                titlebar_frame.origin.y = (window_height - container_height).to_f64();

                let minimize_x = traffic_light_position.x + button_width + button_padding;
                let zoom_x = minimize_x + button_width + button_padding;

                titlebar_container.setFrame(titlebar_frame);
                buttons.close.setFrameOrigin(Objc2NSPoint::new(
                    traffic_light_position.x.to_f64(),
                    traffic_light_position.y.to_f64(),
                ));
                buttons.minimize.setFrameOrigin(Objc2NSPoint::new(
                    minimize_x.to_f64(),
                    traffic_light_position.y.to_f64(),
                ));
                buttons.zoom.setFrameOrigin(Objc2NSPoint::new(
                    zoom_x.to_f64(),
                    traffic_light_position.y.to_f64(),
                ));

                titlebar_container.updateTrackingAreas();
                buttons.close.updateTrackingAreas();
                buttons.minimize.updateTrackingAreas();
                buttons.zoom.updateTrackingAreas();
            }
        }
    }

    fn capture_traffic_light_frames(&self) -> Option<TrafficLightFrames> {
        let buttons = self.traffic_light_buttons()?;
        let titlebar_container = Self::titlebar_container(&buttons.close)?;

        Some(TrafficLightFrames {
            titlebar: titlebar_container.frame(),
            close: buttons.close.frame(),
            minimize: buttons.minimize.frame(),
            zoom: buttons.zoom.frame(),
        })
    }

    fn native_window(&self) -> &Objc2NSWindow {
        // SAFETY: `MacWindow::new` initializes `self.native_window` with the AppKit
        // window for this state. It is either `NSWindow` or `NSPanel`, so borrowing it
        // as `Objc2NSWindow` is valid here.
        unsafe { &*self.native_window.cast::<Objc2NSWindow>() }
    }

    fn traffic_light_buttons(&self) -> Option<TrafficLightButtons> {
        let window = self.native_window();
        Some(TrafficLightButtons {
            close: window.standardWindowButton(Objc2NSWindowButton::CloseButton)?,
            minimize: window.standardWindowButton(Objc2NSWindowButton::MiniaturizeButton)?,
            zoom: window.standardWindowButton(Objc2NSWindowButton::ZoomButton)?,
        })
    }

    fn titlebar_container(close_button: &Objc2NSButton) -> Option<Retained<Objc2NSView>> {
        // SAFETY: `close_button` comes from AppKit's `standardWindowButton(_:)`.
        // Although `superview` is unsafe, objc2 returns each result as `Retained<NSView>`.
        unsafe {
            let button_container = close_button.superview()?;
            button_container.superview()
        }
    }

    fn restore_traffic_light(&mut self) {
        if let Some(frames) = self.traffic_light_frames.take() {
            let Some(buttons) = self.traffic_light_buttons() else {
                return;
            };
            let Some(titlebar_container) = Self::titlebar_container(&buttons.close) else {
                return;
            };

            buttons.close.setFrame(frames.close);
            buttons.minimize.setFrame(frames.minimize);
            buttons.zoom.setFrame(frames.zoom);
            titlebar_container.setFrame(frames.titlebar);

            titlebar_container.updateTrackingAreas();
            buttons.close.updateTrackingAreas();
            buttons.minimize.updateTrackingAreas();
            buttons.zoom.updateTrackingAreas();
        }
    }

    fn start_display_link(&mut self) {
        self.stop_display_link();
        unsafe {
            if !self
                .native_window
                .occlusionState()
                .contains(NSWindowOcclusionState::Visible)
            {
                return;
            }
        }
        let Some(display_id) = display_id_for_screen(unsafe { self.native_window.screen() }) else {
            // AppKit can temporarily report no screen while displays are being reconfigured.
            return;
        };
        let data = self.native_view.as_ptr() as *mut c_void;
        self.frame_source
            .get_or_insert_with(|| WindowFrameSource::new(data, step))
            .start(display_id)
            .log_err();
    }

    fn stop_display_link(&mut self) {
        if let Some(frame_source) = self.frame_source.as_mut() {
            frame_source.stop();
        }
    }

    fn is_maximized(&self) -> bool {
        fn rect_to_size(rect: Objc2NSRect) -> Size<Pixels> {
            let NSSize { width, height } = rect.size;
            size(width.into(), height.into())
        }

        unsafe {
            let bounds = self.bounds();
            let screen_size = rect_to_size(self.native_window.screen().visibleFrame());
            bounds.size == screen_size
        }
    }

    fn is_fullscreen(&self) -> bool {
        unsafe {
            let style_mask = self.native_window.styleMask();
            style_mask.contains(NSWindowStyleMask::FullScreen)
        }
    }

    fn toggle_simple_fullscreen(&mut self) -> Option<SimpleFullscreenPlan> {
        // If the window is in native fullscreen, simple fullscreen would conflict
        // with AppKit's own fullscreen handling, so ignore the request.
        if self.is_fullscreen() {
            return None;
        }

        if let Some(saved) = self.simple_fullscreen_state.take() {
            Some(SimpleFullscreenPlan::Exit(saved))
        } else {
            let screen = unsafe { self.native_window.screen() };
            if screen == NIL {
                return None;
            }
            // `frame` returns an NSRect. Keep that ABI at this boundary explicit: letting
            // the surrounding enum infer it can silently select the wrong Objective-C
            // stret encoding when this code is refactored.
            let screen_frame: Objc2NSRect = unsafe { msg_send![screen, frame] };
            let bounds = self.bounds();

            let frame: Objc2NSRect = unsafe { msg_send![self.native_window, frame] };
            self.simple_fullscreen_state = Some(SimpleFullscreenState {
                frame,
                bounds,
                style_mask: unsafe { self.native_window.styleMask() },
            });

            Some(SimpleFullscreenPlan::Enter { screen_frame })
        }
    }

    fn bounds(&self) -> Bounds<Pixels> {
        let mut window_frame: Objc2NSRect = unsafe { msg_send![self.native_window, frame] };
        let screen: ObjcId = unsafe { msg_send![self.native_window, screen] };
        if screen == NIL {
            return Bounds::new(point(px(0.), px(0.)), gpui::DEFAULT_WINDOW_SIZE);
        }
        let screen_frame: Objc2NSRect = unsafe { msg_send![screen, frame] };

        // Flip the y coordinate to be top-left origin
        window_frame.origin.y =
            screen_frame.size.height - window_frame.origin.y - window_frame.size.height;

        Bounds::new(
            point(
                px((window_frame.origin.x - screen_frame.origin.x) as f32),
                px((window_frame.origin.y + screen_frame.origin.y) as f32),
            ),
            size(
                px(window_frame.size.width as f32),
                px(window_frame.size.height as f32),
            ),
        )
    }

    fn content_size(&self) -> Size<Pixels> {
        let content_view: ObjcId = unsafe { msg_send![self.native_window, contentView] };
        let frame: Objc2NSRect = unsafe { msg_send![content_view, frame] };
        size(px(frame.size.width as f32), px(frame.size.height as f32))
    }

    fn scale_factor(&self) -> f32 {
        get_scale_factor(self.native_window)
    }

    fn window_bounds(&self) -> WindowBounds {
        if self.is_fullscreen() {
            WindowBounds::Fullscreen(self.fullscreen_restore_bounds)
        } else if let Some(state) = &self.simple_fullscreen_state {
            WindowBounds::Windowed(state.bounds)
        } else {
            WindowBounds::Windowed(self.bounds())
        }
    }
}

unsafe impl Send for MacWindowState {}

pub(crate) struct MacWindow(Arc<Mutex<MacWindowState>>, MainThreadMarker);

impl MacWindow {
    pub fn open(
        handle: AnyWindowHandle,
        WindowParams {
            bounds,
            titlebar,
            kind,
            is_movable,
            app_owns_titlebar_drag,
            is_resizable,
            is_minimizable,
            focus,
            show,
            display_id,
            window_min_size,
            tabbing_identifier,
            ..
        }: WindowParams,
        cursor_visible: Arc<AtomicBool>,
        foreground_executor: ForegroundExecutor,
        background_executor: BackgroundExecutor,
        renderer: renderer::Renderer,
        marker: MainThreadMarker,
    ) -> Self {
        unsafe {
            let pool: ObjcId = msg_send![class!(NSAutoreleasePool), new];

            let allows_automatic_window_tabbing = tabbing_identifier.is_some();
            if allows_automatic_window_tabbing {
                let () =
                    msg_send![class!(NSWindow), setAllowsAutomaticWindowTabbing: Bool::new(true)];
            } else {
                let () =
                    msg_send![class!(NSWindow), setAllowsAutomaticWindowTabbing: Bool::new(false)];
            }

            let mut style_mask;
            if let Some(titlebar) = titlebar.as_ref() {
                style_mask = NSWindowStyleMask::Closable | NSWindowStyleMask::Titled;

                if is_resizable {
                    style_mask |= NSWindowStyleMask::Resizable;
                }

                if is_minimizable {
                    style_mask |= NSWindowStyleMask::Miniaturizable;
                }

                if titlebar.appears_transparent {
                    style_mask |= NSWindowStyleMask::FullSizeContentView;
                }
            } else {
                style_mask = NSWindowStyleMask::Titled | NSWindowStyleMask::FullSizeContentView;
            }

            let native_window: ObjcId = match kind {
                WindowKind::Normal => {
                    msg_send![&*WINDOW_CLASS, alloc]
                }
                // `AnchoredPopup` is rejected in `MacPlatform::open_window`, grouped here only
                // for exhaustiveness.
                WindowKind::PopUp | WindowKind::AnchoredPopup(_) => {
                    style_mask |= NSWindowStyleMask::NonactivatingPanel;
                    msg_send![&*PANEL_CLASS, alloc]
                }
                WindowKind::Floating | WindowKind::Dialog => {
                    msg_send![&*PANEL_CLASS, alloc]
                }
            };

            let display = display_id
                .and_then(MacDisplay::find_by_id)
                .unwrap_or_else(MacDisplay::primary);

            let mut target_screen = NIL;
            let mut screen_frame = None;

            let screens: ObjcId = msg_send![class!(NSScreen), screens];
            let count: NSUInteger = msg_send![screens, count];
            for i in 0..count {
                let screen: ObjcId = msg_send![screens, objectAtIndex: i];
                let Some(display_id) = display_id_for_screen(screen) else {
                    continue;
                };
                let frame: Objc2NSRect = msg_send![screen, frame];
                if display_id == display.0 {
                    screen_frame = Some(frame);
                    target_screen = screen;
                }
            }

            let screen_frame = screen_frame.unwrap_or_else(|| {
                let screen: ObjcId = msg_send![class!(NSScreen), mainScreen];
                target_screen = screen;
                let frame: Objc2NSRect = msg_send![screen, frame];
                frame
            });

            let window_rect = Objc2NSRect::new(
                Objc2NSPoint::new(
                    screen_frame.origin.x + bounds.origin.x.as_f32() as f64,
                    screen_frame.origin.y
                        + (display.bounds().size.height - bounds.origin.y).as_f32() as f64,
                ),
                NSSize::new(
                    bounds.size.width.as_f32() as f64,
                    bounds.size.height.as_f32() as f64,
                ),
            );

            let native_window = native_window.initWithContentRect_styleMask_backing_defer_screen_(
                window_rect,
                style_mask,
                NSBackingStoreType::Buffered,
                Bool::new(false),
                target_screen,
            );
            assert!(!native_window.is_null());
            let filenames_type = filenames_pboard_type();
            let dragged_types: ObjcId =
                msg_send![class!(NSArray), arrayWithObject: &*filenames_type];
            let () = msg_send![native_window, registerForDraggedTypes: dragged_types];
            let () = msg_send![
                native_window,
                setReleasedWhenClosed: Bool::new(false)
            ];

            let content_view = native_window.contentView();
            let native_view: ObjcId = msg_send![&*VIEW_CLASS, alloc];
            let content_bounds: Objc2NSRect = msg_send![content_view, bounds];
            let native_view: ObjcId = msg_send![native_view, initWithFrame: content_bounds];
            assert!(!native_view.is_null());

            let state = Arc::new(Mutex::new(MacWindowState {
                handle,
                foreground_executor,
                background_executor,
                native_window,
                native_view: NonNull::new_unchecked(native_view),
                blurred_view: None,
                background_appearance: WindowBackgroundAppearance::Opaque,
                cursor_style: CursorStyle::Arrow,
                cursor_visible,
                frame_source: None,
                renderer,
                force_render_pending: false,
                request_frame_callback: None,
                event_callback: None,
                activate_callback: None,
                resize_callback: None,
                moved_callback: None,
                should_close_callback: None,
                close_callback: None,
                appearance_changed_callback: None,
                input_handler: None,
                last_key_equivalent: None,
                last_left_mouse_down_event: None,
                synthetic_drag_counter: 0,
                traffic_light_position: titlebar
                    .as_ref()
                    .and_then(|titlebar| titlebar.traffic_light_position),
                traffic_light_frames: None,
                transparent_titlebar: titlebar
                    .as_ref()
                    .is_none_or(|titlebar| titlebar.appears_transparent),
                previous_modifiers_changed_event: None,
                keystroke_for_do_command: None,
                do_command_handled: None,
                external_files_dragged: false,
                first_mouse: false,
                app_owns_titlebar_drag,
                fullscreen_restore_bounds: Bounds::default(),
                simple_fullscreen_state: None,
                move_tab_to_new_window_callback: None,
                merge_all_windows_callback: None,
                select_next_tab_callback: None,
                select_previous_tab_callback: None,
                toggle_tab_bar_callback: None,
                activated_least_once: false,
                closed: Arc::new(AtomicBool::new(false)),
                accesskit_adapter: None,
                sheet_parent: None,
            }));
            let mut window = Self(state, marker);

            set_window_state_ivar(
                native_window,
                Arc::into_raw(window.0.clone()) as *mut c_void,
            );
            native_window.setDelegate_(native_window);
            set_window_state_ivar(native_view, Arc::into_raw(window.0.clone()) as *mut c_void);

            if let Some(title) = titlebar
                .as_ref()
                .and_then(|t| t.title.as_ref().map(AsRef::as_ref))
            {
                window.set_title(title);
            }

            native_window.setMovable_(Bool::new(is_movable));

            if let Some(window_min_size) = window_min_size {
                native_window.setContentMinSize_(NSSize {
                    width: window_min_size.width.to_f64(),
                    height: window_min_size.height.to_f64(),
                });
            }

            if titlebar.is_none_or(|titlebar| titlebar.appears_transparent) {
                native_window.setTitlebarAppearsTransparent_(Bool::new(true));
                let _: () =
                    msg_send![native_window, setTitleVisibility: NSWindowTitleVisibility::Hidden];
            }

            native_view.setAutoresizingMask_(VIEW_WIDTH_SIZABLE | VIEW_HEIGHT_SIZABLE);
            // Create the renderer's CAMetalLayer before adding the view to its window.
            native_view.setWantsLayer(Bool::new(true));
            let _: () = msg_send![
            native_view,
            setLayerContentsRedrawPolicy: NSViewLayerContentsRedrawPolicy::DuringViewResize
            ];

            content_view.addSubview_(native_view.autorelease());
            native_window.makeFirstResponder_(native_view);

            let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
            let main_window: ObjcId = msg_send![app, mainWindow];
            let mut sheet_parent = None;

            match kind {
                WindowKind::Normal | WindowKind::Floating => {
                    if kind == WindowKind::Floating {
                        // Let the window float keep above normal windows.
                        native_window.setLevel_(NSFloatingWindowLevel);
                    } else {
                        native_window.setLevel_(NSNormalWindowLevel);
                    }
                    native_window.setAcceptsMouseMovedEvents_(Bool::new(true));

                    if let Some(tabbing_identifier) = tabbing_identifier {
                        let tabbing_id = ns_string(tabbing_identifier.as_str());
                        let _: () = msg_send![native_window, setTabbingIdentifier: &*tabbing_id];
                    } else {
                        let _: () = msg_send![native_window, setTabbingIdentifier:NIL];
                    }
                }
                // `AnchoredPopup` is rejected in `MacPlatform::open_window`, grouped here only
                // for exhaustiveness.
                WindowKind::PopUp | WindowKind::AnchoredPopup(_) => {
                    // Use a tracking area to allow receiving MouseMoved events even when
                    // the window or application aren't active, which is often the case
                    // e.g. for notification windows.
                    let tracking_area: ObjcId = msg_send![class!(NSTrackingArea), alloc];
                    let _: () = msg_send![
                        tracking_area,
                        initWithRect: Objc2NSRect::new(Objc2NSPoint::new(0., 0.), NSSize::new(0., 0.)),
                        options: NSTrackingMouseEnteredAndExited | NSTrackingMouseMoved | NSTrackingActiveAlways | NSTrackingInVisibleRect,
                        owner: native_view,
                        userInfo: NIL
                    ];
                    let _: () =
                        msg_send![native_view, addTrackingArea: tracking_area.autorelease()];

                    native_window.setLevel_(NSPopUpWindowLevel);
                    let _: () = msg_send![
                        native_window,
                        setAnimationBehavior: NSWindowAnimationBehaviorUtilityWindow
                    ];
                    native_window.setCollectionBehavior_(
                        NSWindowCollectionBehavior::CanJoinAllSpaces
                            | NSWindowCollectionBehavior::FullScreenAuxiliary,
                    );
                }
                WindowKind::Dialog => {
                    if !main_window.is_null() {
                        let parent = {
                            let active_sheet: ObjcId = msg_send![main_window, attachedSheet];
                            if active_sheet.is_null() {
                                main_window
                            } else {
                                active_sheet
                            }
                        };
                        let _: () =
                            msg_send![parent, beginSheet: native_window, completionHandler: NIL];
                        sheet_parent = Some(parent);
                    }
                }
            }

            if allows_automatic_window_tabbing
                && !main_window.is_null()
                && main_window != native_window
            {
                let main_window_is_fullscreen = main_window
                    .styleMask()
                    .contains(NSWindowStyleMask::FullScreen);
                let user_tabbing_preference = Self::get_user_tabbing_preference()
                    .unwrap_or(UserTabbingPreference::InFullScreen);
                let should_add_as_tab = user_tabbing_preference == UserTabbingPreference::Always
                    || user_tabbing_preference == UserTabbingPreference::InFullScreen
                        && main_window_is_fullscreen;

                if should_add_as_tab {
                    let main_window_can_tab: Bool =
                        msg_send![main_window, respondsToSelector: sel!(addTabbedWindow:ordered:)];
                    let main_window_visible: Bool = msg_send![main_window, isVisible];

                    if main_window_can_tab == Bool::new(true)
                        && main_window_visible == Bool::new(true)
                    {
                        let _: () = msg_send![main_window, addTabbedWindow: native_window, ordered: NSWindowOrderingMode::Above];

                        // Ensure the window is visible immediately after adding the tab, since the tab bar is updated with a new entry at this point.
                        // Note: Calling orderFront here can break fullscreen mode (makes fullscreen windows exit fullscreen), so only do this if the main window is not fullscreen.
                        if !main_window_is_fullscreen {
                            let _: () = msg_send![native_window, orderFront: NIL];
                        }
                    }
                }
            }

            if focus && show {
                native_window.makeKeyAndOrderFront_(NIL);
            } else if show {
                native_window.orderFront_(NIL);
            }

            // Set the initial position of the window to the specified origin.
            // Although we already specified the position using `initWithContentRect_styleMask_backing_defer_screen_`,
            // the window position might be incorrect if the main screen (the screen that contains the window that has focus)
            //  is different from the primary screen.
            let _: () = msg_send![native_window, setFrameTopLeftPoint: window_rect.origin];
            {
                let mut window_state = window.0.lock();
                window_state.move_traffic_light();
                window_state.sheet_parent = sheet_parent;
            }

            pool.drain();

            window
        }
    }

    pub fn active_window() -> Option<AnyWindowHandle> {
        unsafe {
            let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
            let main_window: ObjcId = msg_send![app, mainWindow];
            if main_window.is_null() {
                return None;
            }

            let is_gpui_window: Bool = msg_send![main_window, isKindOfClass: &*WINDOW_CLASS];
            if is_gpui_window.as_bool() {
                let handle = get_window_state(&*main_window).lock().handle;
                Some(handle)
            } else {
                None
            }
        }
    }

    pub fn ordered_windows() -> Vec<AnyWindowHandle> {
        unsafe {
            let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
            let windows: ObjcId = msg_send![app, orderedWindows];
            let count: NSUInteger = msg_send![windows, count];

            let mut window_handles = Vec::new();
            for i in 0..count {
                let window: ObjcId = msg_send![windows, objectAtIndex:i];
                let is_gpui_window: Bool = msg_send![window, isKindOfClass: &*WINDOW_CLASS];
                if is_gpui_window.as_bool() {
                    let handle = get_window_state(&*window).lock().handle;
                    window_handles.push(handle);
                }
            }

            window_handles
        }
    }

    pub fn get_user_tabbing_preference() -> Option<UserTabbingPreference> {
        unsafe {
            let defaults: ObjcId = msg_send![class!(NSUserDefaults), standardUserDefaults];
            let domain = ns_string("NSGlobalDomain");
            let key = ns_string("AppleWindowTabbingMode");

            let dict: ObjcId = msg_send![defaults, persistentDomainForName: &*domain];
            let value: ObjcId = if !dict.is_null() {
                msg_send![dict, objectForKey: &*key]
            } else {
                NIL
            };

            let value_str = objc_string(value);

            match value_str.as_ref() {
                "manual" => Some(UserTabbingPreference::Never),
                "always" => Some(UserTabbingPreference::Always),
                _ => Some(UserTabbingPreference::InFullScreen),
            }
        }
    }
}

impl Drop for MacWindow {
    fn drop(&mut self) {
        let mut this = self.0.lock();
        this.renderer.destroy();
        let window = this.native_window;
        let sheet_parent = this.sheet_parent.take();
        this.frame_source.take();
        unsafe {
            this.native_window.setDelegate_(NIL);
        }
        this.input_handler.take();
        this.foreground_executor
            .spawn(async move {
                unsafe {
                    if let Some(parent) = sheet_parent {
                        let _: () = msg_send![parent, endSheet: window];
                    }
                    window.close();
                    window.autorelease();
                }
            })
            .detach();
    }
}

/// Calls `f` if the window is not closed.
///
/// This should be used when spawning foreground tasks interacting with the
/// window, as some messages will end hard faulting if dispatched to no longer
/// valid window handles.
fn if_window_not_closed(closed: Arc<AtomicBool>, f: impl FnOnce()) {
    if !closed.load(Ordering::Acquire) {
        f();
    }
}

impl PlatformWindow for MacWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.0.as_ref().lock().bounds()
    }

    fn window_bounds(&self) -> WindowBounds {
        self.0.as_ref().lock().window_bounds()
    }

    fn is_maximized(&self) -> bool {
        self.0.as_ref().lock().is_maximized()
    }

    fn content_size(&self) -> Size<Pixels> {
        self.0.as_ref().lock().content_size()
    }

    fn resize(&mut self, size: Size<Pixels>) {
        let this = self.0.lock();
        let window = this.native_window;
        let closed = this.closed.clone();
        this.foreground_executor
            .spawn(async move {
                if_window_not_closed(closed, || unsafe {
                    window.setContentSize_(NSSize {
                        width: size.width.as_f32() as f64,
                        height: size.height.as_f32() as f64,
                    });
                })
            })
            .detach();
    }

    fn merge_all_windows(&self) {
        let native_window = self.0.lock().native_window;
        extern "C" fn merge_windows_async(context: *mut std::ffi::c_void) {
            unsafe {
                let native_window = context as ObjcId;
                let _: () = msg_send![native_window, mergeAllWindows:NIL];
            }
        }

        unsafe {
            DispatchQueue::main()
                .exec_async_f(native_window as *mut std::ffi::c_void, merge_windows_async);
        }
    }

    fn move_tab_to_new_window(&self) {
        let native_window = self.0.lock().native_window;
        extern "C" fn move_tab_async(context: *mut std::ffi::c_void) {
            unsafe {
                let native_window = context as ObjcId;
                let _: () = msg_send![native_window, moveTabToNewWindow:NIL];
                let _: () = msg_send![native_window, makeKeyAndOrderFront: NIL];
            }
        }

        unsafe {
            DispatchQueue::main()
                .exec_async_f(native_window as *mut std::ffi::c_void, move_tab_async);
        }
    }

    fn toggle_window_tab_overview(&self) {
        let native_window = self.0.lock().native_window;
        unsafe {
            let _: () = msg_send![native_window, toggleTabOverview:NIL];
        }
    }

    fn set_tabbing_identifier(&self, tabbing_identifier: Option<String>) {
        let native_window = self.0.lock().native_window;
        unsafe {
            let allows_automatic_window_tabbing = tabbing_identifier.is_some();
            if allows_automatic_window_tabbing {
                let () =
                    msg_send![class!(NSWindow), setAllowsAutomaticWindowTabbing: Bool::new(true)];
            } else {
                let () =
                    msg_send![class!(NSWindow), setAllowsAutomaticWindowTabbing: Bool::new(false)];
            }

            if let Some(tabbing_identifier) = tabbing_identifier {
                let tabbing_id = ns_string(tabbing_identifier.as_str());
                let _: () = msg_send![native_window, setTabbingIdentifier: &*tabbing_id];
            } else {
                let _: () = msg_send![native_window, setTabbingIdentifier:NIL];
            }
        }
    }

    fn set_traffic_light_position(&self, position: Point<Pixels>) {
        let mut state = self.0.lock();
        state.traffic_light_position = Some(position);
        state.move_traffic_light();
    }

    fn scale_factor(&self) -> f32 {
        self.0.as_ref().lock().scale_factor()
    }

    fn appearance(&self) -> WindowAppearance {
        unsafe {
            let appearance: ObjcId = msg_send![self.0.lock().native_window, effectiveAppearance];
            crate::window_appearance::window_appearance_from_native(appearance.cast())
        }
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        unsafe {
            let screen = self.0.lock().native_window.screen();
            if screen.is_null() {
                return None;
            }
            Some(Rc::new(MacDisplay(display_id_for_screen(screen)?)))
        }
    }

    fn mouse_position(&self) -> Point<Pixels> {
        let position = unsafe {
            self.0
                .lock()
                .native_window
                .mouseLocationOutsideOfEventStream()
        };
        convert_mouse_position(position, self.content_size().height)
    }

    fn modifiers(&self) -> Modifiers {
        unsafe {
            let modifiers: NSEventModifierFlags = msg_send![class!(NSEvent), modifierFlags];

            let control = modifiers.contains(NSEventModifierFlags::Control);
            let alt = modifiers.contains(NSEventModifierFlags::Option);
            let shift = modifiers.contains(NSEventModifierFlags::Shift);
            let command = modifiers.contains(NSEventModifierFlags::Command);
            let function = modifiers.contains(NSEventModifierFlags::Function);

            Modifiers {
                control,
                alt,
                shift,
                platform: command,
                function,
            }
        }
    }

    fn capslock(&self) -> Capslock {
        unsafe {
            let modifiers: NSEventModifierFlags = msg_send![class!(NSEvent), modifierFlags];

            Capslock {
                on: modifiers.contains(NSEventModifierFlags::CapsLock),
            }
        }
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.0.as_ref().lock().input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.0.as_ref().lock().input_handler.take()
    }

    fn prompt(
        &self,
        level: PromptLevel,
        msg: &str,
        detail: Option<&str>,
        answers: &[PromptButton],
    ) -> Option<oneshot::Receiver<usize>> {
        use objc2_foundation::{NSInteger, NSString};

        // NSAlert's first button keeps Return and Cancel keeps Escape, but the keyboard
        // focus (and therefore Space) defaults to Cancel, leaving the middle button of
        // prompts like "Save / Don't Save / Cancel" unreachable from the keyboard. Move
        // the initial focus onto the last non-cancel, non-default button instead.
        let initial_focus_ix = answers
            .iter()
            .enumerate()
            .rev()
            .find(|(_, label)| !label.is_cancel())
            .map(|(ix, _)| ix)
            .filter(|&ix| ix > 0);

        let alert = NSAlert::new(self.1);
        alert.setAlertStyle(match level {
            PromptLevel::Critical => NSAlertStyle::Critical,
            PromptLevel::Warning => NSAlertStyle::Warning,
            PromptLevel::Info => NSAlertStyle::Informational,
        });
        let message = NSString::from_str(msg);
        alert.setMessageText(message.as_ref());

        if let Some(detail) = detail {
            let detail_text = NSString::from_str(detail);
            alert.setInformativeText(detail_text.as_ref());
        }

        let mut initial_focus_button: Option<Retained<Objc2NSButton>> = None;
        for (ix, answer) in answers.iter().enumerate() {
            let title = NSString::from_str(answer.label());
            let button = alert.addButtonWithTitle(&title);
            button.setTag(ix as NSInteger);

            if answer.is_cancel() {
                if let Some(key) = core::char::from_u32(crate::events::ESCAPE_KEY) {
                    let key = NSString::from_str(&key.to_string());
                    button.setKeyEquivalent(&key);
                }
            } else if Some(ix) == initial_focus_ix {
                initial_focus_button = Some(button);
            }
        }

        if let Some(button) = initial_focus_button {
            alert.window().setInitialFirstResponder(Some(&button));
        }

        let (done_tx, done_rx) = oneshot::channel();
        let done_tx = Cell::new(Some(done_tx));

        let block = RcBlock::new(move |answer: NSInteger| {
            if let Some(done_tx) = done_tx.take() {
                let _ = done_tx.send(answer.try_into().unwrap());
            }
        });

        let lock = self.0.lock();
        let native_window = lock.native_window;
        let closed = lock.closed.clone();
        let executor = lock.foreground_executor.clone();
        executor
            .spawn(async move {
                if !closed.load(Ordering::Acquire) {
                    // SAFETY: `native_window` is an Objective-C `NSWindow` pointer
                    // owned by the platform window; bridge it into objc2.
                    let sheet_window: &Objc2NSWindow =
                        unsafe { &*(native_window as *const Objc2NSWindow) };

                    alert.beginSheetModalForWindow_completionHandler(sheet_window, Some(&block));
                }
            })
            .detach();

        Some(done_rx)
    }

    fn activate(&self, token: Option<&str>) -> bool {
        if token.is_some() {
            return false;
        }
        let lock = self.0.lock();
        let window = lock.native_window;
        let closed = lock.closed.clone();
        let executor = lock.foreground_executor.clone();
        executor
            .spawn(async move {
                if !closed.load(Ordering::Acquire) {
                    unsafe {
                        let _: () = msg_send![window, makeKeyAndOrderFront: NIL];
                    }
                }
            })
            .detach();
        true
    }

    fn request_attention(&self) {
        if self.is_active() {
            return;
        }

        let executor = self.0.lock().foreground_executor.clone();
        executor
            .spawn(async move {
                unsafe {
                    let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
                    app.requestUserAttention_(NSRequestUserAttentionType::InformationalRequest);
                }
            })
            .detach();
    }

    fn is_active(&self) -> bool {
        unsafe { self.0.lock().native_window.isKeyWindow() == Bool::new(true) }
    }

    // is_hovered is unused on macOS. See Window::is_window_hovered.
    fn is_hovered(&self) -> bool {
        false
    }

    fn set_title(&mut self, title: &str) {
        unsafe {
            let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
            let window = self.0.lock().native_window;
            let title = ns_string(title);
            let _: () = msg_send![app, changeWindowsItem: window, title: &*title, filename: false];
            let _: () = msg_send![window, setTitle: &*title];
            self.0.lock().move_traffic_light();
        }
    }

    fn get_title(&self) -> String {
        unsafe {
            let title: ObjcId = msg_send![self.0.lock().native_window, title];
            if title.is_null() {
                "".to_string()
            } else {
                objc_string(title)
            }
        }
    }

    fn set_app_id(&mut self, _app_id: &str) {}

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut this = self.0.as_ref().lock();
        this.background_appearance = background_appearance;

        let opaque = background_appearance.is_opaque();
        this.renderer.update_transparency(!opaque);

        unsafe {
            this.native_window.setOpaque_(Bool::new(opaque));
            let background_color = if opaque {
                msg_send![AnyClass::get(c"NSColor").unwrap(), colorWithSRGBRed: 0f64, green: 0f64, blue: 0f64, alpha: 1f64]
            } else {
                // Not using `+[NSColor clearColor]` to avoid broken shadow.
                msg_send![AnyClass::get(c"NSColor").unwrap(), colorWithSRGBRed: 0f64, green: 0f64, blue: 0f64, alpha: 0.0001]
            };
            this.native_window.setBackgroundColor_(background_color);

            if background_appearance != WindowBackgroundAppearance::Blurred {
                if let Some(blur_view) = this.blurred_view {
                    let _: () = msg_send![blur_view, removeFromSuperview];
                    this.blurred_view = None;
                }
            } else if this.blurred_view.is_none() {
                let content_view = this.native_window.contentView();
                let frame: Objc2NSRect = msg_send![content_view, bounds];
                let mut blur_view: ObjcId = msg_send![&*BLURRED_VIEW_CLASS, alloc];
                blur_view = msg_send![blur_view, initWithFrame: frame];
                blur_view.setAutoresizingMask_(VIEW_WIDTH_SIZABLE | VIEW_HEIGHT_SIZABLE);

                let _: () = msg_send![
                    content_view,
                    addSubview: blur_view,
                    positioned: NSWindowOrderingMode::Below,
                    relativeTo: NIL
                ];
                this.blurred_view = Some(blur_view.autorelease());
            }
        }
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        self.0.as_ref().lock().background_appearance
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        // CoreGraphics rasterization and the retired Metal renderer are grayscale-only on macOS.
        false
    }

    fn set_edited(&mut self, edited: bool) {
        unsafe {
            let window = self.0.lock().native_window;
            msg_send![window, setDocumentEdited: Bool::new(edited)]
        }

        // Changing the document edited state resets the traffic light position,
        // so we have to move it again.
        self.0.lock().move_traffic_light();
    }

    fn set_document_path(&self, path: Option<&std::path::Path>) {
        unsafe {
            let window = self.0.lock().native_window;
            let filename = path.map_or(ns_string(""), |p| ns_string(&p.to_string_lossy()));
            let _: () = msg_send![window, setRepresentedFilename: &*filename];
        }

        // Changing the document path state resets the traffic light position,
        // so we have to move it again.
        self.0.lock().move_traffic_light();
    }

    fn show_character_palette(&self) {
        let this = self.0.lock();
        let window = this.native_window;
        this.foreground_executor
            .spawn(async move {
                unsafe {
                    let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
                    let _: () = msg_send![app, orderFrontCharacterPalette: window];
                }
            })
            .detach();
    }

    fn set_visible(&self, visible: bool) {
        let lock = self.0.lock();
        let window = lock.native_window;
        let closed = lock.closed.clone();
        let executor = lock.foreground_executor.clone();
        executor
            .spawn(async move {
                if !closed.load(Ordering::Acquire) {
                    unsafe {
                        if visible {
                            let _: () = msg_send![window, orderFront: NIL];
                        } else {
                            let _: () = msg_send![window, orderOut: NIL];
                        }
                    }
                }
            })
            .detach();
    }

    fn minimize(&self) {
        let window = self.0.lock().native_window;
        unsafe {
            window.miniaturize_(NIL);
        }
    }

    fn zoom(&self) {
        let this = self.0.lock();
        let window = this.native_window;
        let closed = this.closed.clone();
        this.foreground_executor
            .spawn(async move {
                if_window_not_closed(closed, || unsafe {
                    window.zoom_(NIL);
                })
            })
            .detach();
    }

    fn toggle_fullscreen(&self) {
        let this = self.0.lock();
        let window = this.native_window;
        let closed = this.closed.clone();
        this.foreground_executor
            .spawn(async move {
                if_window_not_closed(closed, || unsafe {
                    window.toggleFullScreen_(NIL);
                })
            })
            .detach();
    }

    fn toggle_simple_fullscreen(&self) {
        let state = self.0.clone();
        let (foreground_executor, closed) = {
            let this = self.0.lock();
            (this.foreground_executor.clone(), this.closed.clone())
        };
        foreground_executor
            .spawn(async move {
                if_window_not_closed(closed, move || {
                    let (native_window, native_view, plan) = {
                        let mut lock = state.lock();
                        (
                            lock.native_window,
                            lock.native_view.as_ptr() as ObjcId,
                            lock.toggle_simple_fullscreen(),
                        )
                    };
                    if let Some(plan) = plan {
                        unsafe { apply_simple_fullscreen_plan(native_window, native_view, plan) };
                    }
                })
            })
            .detach();
    }

    fn is_simple_fullscreen(&self) -> bool {
        self.0.lock().simple_fullscreen_state.is_some()
    }

    fn is_fullscreen(&self) -> bool {
        let this = self.0.lock();
        let window = this.native_window;

        unsafe { window.styleMask().contains(NSWindowStyleMask::FullScreen) }
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.0.as_ref().lock().request_frame_callback = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> gpui::DispatchEventResult>) {
        self.0.as_ref().lock().event_callback = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.as_ref().lock().activate_callback = Some(callback);
    }

    fn on_hover_status_change(&self, _: Box<dyn FnMut(bool)>) {}

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.0.as_ref().lock().resize_callback = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().moved_callback = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.as_ref().lock().should_close_callback = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.as_ref().lock().close_callback = Some(callback);
    }

    fn on_hit_test_window_control(&self, _callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().appearance_changed_callback = Some(callback);
    }

    fn tabbed_windows(&self) -> Option<Vec<SystemWindowTab>> {
        unsafe {
            let windows: ObjcId = msg_send![self.0.lock().native_window, tabbedWindows];
            if windows.is_null() {
                return None;
            }

            let count: NSUInteger = msg_send![windows, count];
            let mut result = Vec::new();
            for i in 0..count {
                let window: ObjcId = msg_send![windows, objectAtIndex:i];
                let is_gpui_window: Bool = msg_send![window, isKindOfClass: &*WINDOW_CLASS];
                if is_gpui_window.as_bool() {
                    let handle = get_window_state(&*window).lock().handle;
                    let title: ObjcId = msg_send![window, title];
                    let title = SharedString::from(objc_string(title));

                    result.push(SystemWindowTab::new(title, handle));
                }
            }

            Some(result)
        }
    }

    fn tab_bar_visible(&self) -> bool {
        unsafe {
            let tab_group: ObjcId = msg_send![self.0.lock().native_window, tabGroup];
            if tab_group.is_null() {
                false
            } else {
                let tab_bar_visible: Bool = msg_send![tab_group, isTabBarVisible];
                tab_bar_visible == Bool::new(true)
            }
        }
    }

    fn on_move_tab_to_new_window(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().move_tab_to_new_window_callback = Some(callback);
    }

    fn on_merge_all_windows(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().merge_all_windows_callback = Some(callback);
    }

    fn on_select_next_tab(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().select_next_tab_callback = Some(callback);
    }

    fn on_select_previous_tab(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().select_previous_tab_callback = Some(callback);
    }

    fn on_toggle_tab_bar(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().toggle_tab_bar_callback = Some(callback);
    }

    fn draw(&self, scene: &gpui::Scene) {
        let mut this = self.0.lock();
        #[cfg(feature = "wgpu")]
        if this.renderer.draw(scene) {
            this.force_render_pending = true;
        }
        #[cfg(not(feature = "wgpu"))]
        this.renderer.draw(scene);
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.0.lock().renderer.sprite_atlas().clone()
    }

    fn gpu_specs(&self) -> Option<gpui::GpuSpecs> {
        #[cfg(feature = "wgpu")]
        return Some(self.0.lock().renderer.gpu_specs());
        #[cfg(not(feature = "wgpu"))]
        None
    }

    #[cfg(feature = "wgpu")]
    fn gpu_context(&self) -> Option<Box<dyn std::any::Any>> {
        let (device, queue) = self.0.lock().renderer.gpu_context();
        Some(Box::new((device, queue)))
    }

    #[cfg(feature = "wgpu")]
    fn gpu_device_lost(&self) -> Option<bool> {
        // Only loads an atomic flag, so it is safe mid-recovery.
        Some(self.0.lock().renderer.device_lost())
    }

    #[cfg(feature = "wgpu")]
    fn gpu_context_info(&self) -> Option<Box<dyn std::any::Any>> {
        self.0
            .lock()
            .renderer
            .gpu_context_info()
            .map(|context| Box::new(context) as Box<dyn std::any::Any>)
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {
        let executor = self.0.lock().foreground_executor.clone();
        executor
            .spawn(async move {
                unsafe {
                    let input_context: ObjcId =
                        msg_send![class!(NSTextInputContext), currentInputContext];
                    if input_context.is_null() {
                        return;
                    }
                    let _: () = msg_send![input_context, invalidateCharacterCoordinates];
                }
            })
            .detach()
    }

    fn titlebar_double_click(&self, is_resizable: bool, is_minimizable: bool) {
        let this = self.0.lock();
        if this.simple_fullscreen_state.is_some() {
            return;
        }
        let window = this.native_window;
        let closed = this.closed.clone();
        this.foreground_executor
            .spawn(async move {
                if_window_not_closed(closed, || {
                    unsafe {
                        let defaults: ObjcId =
                            msg_send![class!(NSUserDefaults), standardUserDefaults];
                        let domain = ns_string("NSGlobalDomain");
                        let key = ns_string("AppleActionOnDoubleClick");

                        let dict: ObjcId = msg_send![defaults, persistentDomainForName: &*domain];
                        let action: ObjcId = if !dict.is_null() {
                            msg_send![dict, objectForKey: &*key]
                        } else {
                            NIL
                        };

                        let action_str = if !action.is_null() {
                            objc_string(action)
                        } else {
                            "".into()
                        };

                        match action_str.as_ref() {
                            "None" => {
                                // "Do Nothing" selected, so do no action
                            }
                            "Minimize" => {
                                if is_minimizable {
                                    window.miniaturize_(NIL);
                                }
                            }
                            "Maximize" => {
                                if is_resizable {
                                    window.zoom_(NIL);
                                }
                            }
                            "Fill" => {
                                // There is no documented API for "Fill" action, so we'll just zoom the window
                                if is_resizable {
                                    window.zoom_(NIL);
                                }
                            }
                            _ => {
                                if is_resizable {
                                    window.zoom_(NIL);
                                }
                            }
                        }
                    }
                })
            })
            .detach();
    }

    fn start_window_move(&self) {
        let this = self.0.lock();
        if this.simple_fullscreen_state.is_some() {
            return;
        }
        let window = this.native_window;

        unsafe {
            let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
            let event: ObjcId = msg_send![app, currentEvent];
            let _: () = msg_send![window, performWindowDragWithEvent: event];
        }
    }

    fn can_start_external_drag(&self) -> bool {
        true
    }

    fn start_external_drag(&self, payload: &ExternalDragPayload) -> bool {
        let ExternalDragPayload::Files(paths) = payload;
        if paths.entries().is_empty() {
            log::warn!("start_external_drag declined: no paths");
            return false;
        }

        let (native_view, native_window, last_left_mouse_down_event) = {
            let state = self.0.lock();
            (
                state.native_view.as_ptr(),
                state.native_window,
                state.last_left_mouse_down_event.clone(),
            )
        };

        let Some(last_left_mouse_down_event) = last_left_mouse_down_event else {
            log::warn!("start_external_drag declined: no retained left mouse down event");
            return false;
        };

        // SAFETY: This method runs on the AppKit/foreground path during drag initiation. The
        // native view/window are retained by MacWindowState, copied out under a short lock above,
        // and Objective-C results that may be NIL are checked before use.
        unsafe {
            let event: ObjcId = Retained::as_ptr(&last_left_mouse_down_event)
                .cast_mut()
                .cast();
            let dragging_items: ObjcId = msg_send![class!(NSMutableArray), array];
            // AppKit keeps this frame's distance from the event's location as the drag image's
            // offset from the cursor, so it has to stay anchored on `event`.
            let location: Objc2NSPoint = msg_send![event, locationInWindow];
            let frame = Objc2NSRect::new(
                Objc2NSPoint::new(location.x - 16., location.y - 16.),
                NSSize::new(32., 32.),
            );

            for (path, is_directory) in paths.entries() {
                // Preserve non-UTF-8 paths
                let Ok(path_bytes) = CString::new(path.as_os_str().as_bytes()) else {
                    log::warn!("start_external_drag skipped path containing an interior nul byte");
                    continue;
                };

                let url: ObjcId = msg_send![
                    class!(NSURL),
                    fileURLWithFileSystemRepresentation: path_bytes.as_ptr(),
                    isDirectory: Bool::new(*is_directory),
                    relativeToURL: NIL
                ];

                if url.is_null() {
                    log::warn!("start_external_drag skipped path with NIL NSURL");
                    continue;
                }

                let item: ObjcId = msg_send![class!(NSDraggingItem), alloc];
                let item: ObjcId = msg_send![item, initWithPasteboardWriter: url];
                if item.is_null() {
                    log::warn!("start_external_drag declined: NSDraggingItem allocation failed");
                    continue;
                }

                // Resolve drag images lazily via `imageComponentsProvider` (Apple's
                // recommendation for large item counts), and by file *type* rather than
                // `iconForFile:`, which can synchronously hit LaunchServices, network
                // mounts, or iCloud for every selected path and beachball drag startup.
                // `iconForFileType:` is deprecated in favor of `iconForContentType:`,
                // but the replacement requires macOS 11 and we target 10.15.
                let file_type = if *is_directory {
                    "public.folder".to_string()
                } else {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .map(|extension| extension.to_string())
                        .unwrap_or_else(|| "public.data".to_string())
                };
                let provider = RcBlock::new(move || -> ObjcId {
                    let component: ObjcId = msg_send![
                        class!(NSDraggingImageComponent),
                        draggingImageComponentWithKey: NSDraggingImageComponentIconKey
                    ];
                    let workspace: ObjcId = msg_send![class!(NSWorkspace), sharedWorkspace];
                    let file_type = ns_string(&file_type);
                    let icon: ObjcId = msg_send![workspace, iconForFileType: &*file_type];
                    let _: () = msg_send![component, setContents: icon];
                    // Component frames are relative to the item's dragging frame.
                    let _: () = msg_send![
                        component,
                        setFrame: Objc2NSRect::new(Objc2NSPoint::new(0., 0.), NSSize::new(32., 32.))
                    ];
                    msg_send![class!(NSArray), arrayWithObject: component]
                });
                let _: () = msg_send![item, setDraggingFrame: frame];
                let _: () = msg_send![item, setImageComponentsProvider: &*provider];
                let _: () = msg_send![dragging_items, addObject: item];
                let _: () = msg_send![item, release];
            }

            let count: NSUInteger = msg_send![dragging_items, count];
            if count == 0 {
                log::warn!("start_external_drag declined: no dragging items");
                return false;
            }

            let session: ObjcId = msg_send![
                native_view,
                beginDraggingSessionWithItems: dragging_items,
                event: event,
                source: native_window
            ];

            let started = !session.is_null();
            if started {
                self.0.lock().synthetic_drag_counter += 1;
            }
            log::debug!(
                "start_external_drag completed: started={}, item_count={}",
                started,
                count
            );
            started
        }
    }

    fn play_system_bell(&self) {
        NSBeep()
    }

    #[cfg(any(test, feature = "test-support"))]
    fn render_to_image(&self, scene: &gpui::Scene) -> Result<RgbaImage> {
        let mut this = self.0.lock();
        this.renderer.render_to_image(scene)
    }

    fn a11y_init(&self, callbacks: gpui::A11yCallbacks) {
        let mut lock = self.0.lock();

        let activation_handler = A11yActivationHandler {
            callback: callbacks.activation,
        };
        let action_handler = A11yActionHandler(callbacks.action);

        let adapter = unsafe {
            accesskit_macos::SubclassingAdapter::for_window(
                lock.native_window as *mut c_void,
                activation_handler,
                action_handler,
            )
        };

        lock.accesskit_adapter = Some(adapter);
    }

    fn a11y_tree_update(&self, tree_update: accesskit::TreeUpdate) {
        let events = {
            let mut lock = self.0.lock();
            lock.accesskit_adapter
                .as_mut()
                .and_then(|adapter| adapter.update_if_active(|| tree_update))
        };
        if let Some(events) = events {
            events.raise();
        }
    }

    fn a11y_update_window_bounds(&self) {
        // macOS handles window bounds tracking automatically via NSAccessibility.
    }
}

struct A11yActivationHandler {
    callback: Box<dyn Fn() -> Option<accesskit::TreeUpdate> + Send + 'static>,
}

impl accesskit::ActivationHandler for A11yActivationHandler {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        (self.callback)()
    }
}

struct A11yActionHandler(Box<dyn Fn(accesskit::ActionRequest) + Send + 'static>);

impl accesskit::ActionHandler for A11yActionHandler {
    fn do_action(&mut self, request: accesskit::ActionRequest) {
        (self.0)(request);
    }
}

impl rwh::HasWindowHandle for MacWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        // SAFETY: The AppKitWindowHandle is a wrapper around a pointer to an NSView
        unsafe {
            Ok(rwh::WindowHandle::borrow_raw(rwh::RawWindowHandle::AppKit(
                rwh::AppKitWindowHandle::new(self.0.lock().native_view.cast()),
            )))
        }
    }
}

impl rwh::HasDisplayHandle for MacWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        Ok(rwh::DisplayHandle::appkit())
    }
}

fn get_scale_factor(native_window: ObjcId) -> f32 {
    let factor = unsafe {
        let screen: ObjcId = msg_send![native_window, screen];
        if screen.is_null() {
            return 2.0;
        }
        let factor: f64 = msg_send![screen, backingScaleFactor];
        factor as f32
    };

    // We are not certain what triggers this, but it seems that sometimes
    // this method would return 0 (https://github.com/zed-industries/zed/issues/6412)
    // It seems most likely that this would happen if the window has no screen
    // (if it is off-screen), though we'd expect to see viewDidChangeBackingProperties before
    // it was rendered for real.
    // Regardless, attempt to avoid the issue here.
    if factor == 0.0 { 2. } else { factor }
}

/// Returns whether `window` is one of GPUI's managed windows.
unsafe fn is_gpui_window(window: ObjcId) -> bool {
    unsafe {
        let is_window: Bool = msg_send![window, isKindOfClass: &*WINDOW_CLASS];
        let is_panel: Bool = msg_send![window, isKindOfClass: &*PANEL_CLASS];
        is_window.as_bool() || is_panel.as_bool()
    }
}

unsafe fn get_window_state(object: &Objc2Object) -> Arc<Mutex<MacWindowState>> {
    unsafe {
        let raw: *mut c_void = *window_state_ivar(object).load(object);
        debug_assert!(
            !raw.is_null(),
            "GPUI callback invoked before its state was installed"
        );
        let rc1 = Arc::from_raw(raw as *mut Mutex<MacWindowState>);
        let rc2 = rc1.clone();
        mem::forget(rc1);
        rc2
    }
}

/// Returns `None` while AppKit is running superclass initialization, before the window state can
/// exist. Overrides invoked in that phase must forward to their superclass without touching GPUI
/// state.
unsafe fn initialized_window_state(object: &Objc2Object) -> Option<Arc<Mutex<MacWindowState>>> {
    unsafe {
        let raw: *mut c_void = *window_state_ivar(object).load(object);
        if raw.is_null() {
            return None;
        }
        let rc1 = Arc::from_raw(raw as *mut Mutex<MacWindowState>);
        let rc2 = rc1.clone();
        mem::forget(rc1);
        Some(rc2)
    }
}

unsafe fn drop_window_state(object: &Objc2Object) {
    unsafe {
        let raw: *mut c_void = *window_state_ivar(object).load(object);
        if !raw.is_null() {
            drop(Arc::from_raw(raw as *mut Mutex<MacWindowState>));
        }
    }
}

unsafe extern "C" fn yes(_: &Objc2Object, _: Sel) -> Bool {
    Bool::new(true)
}

unsafe extern "C" fn dealloc_window(this: &Objc2Object, _: Sel) {
    unsafe {
        drop_window_state(this);
        let _: () = msg_send![super(this, class!(NSWindow)), dealloc];
    }
}

unsafe extern "C" fn dealloc_panel(this: &Objc2Object, _: Sel) {
    unsafe {
        drop_window_state(this);
        let _: () = msg_send![super(this, class!(NSPanel)), dealloc];
    }
}

unsafe extern "C" fn dealloc_view(this: &Objc2Object, _: Sel) {
    unsafe {
        drop_window_state(this);
        let _: () = msg_send![super(this, class!(NSView)), dealloc];
    }
}

unsafe extern "C" fn reset_cursor_rects(this: &Objc2Object, _: Sel) {
    // SAFETY: AppKit invokes cursor-rect updates on the main thread for GPUIView instances,
    // whose WINDOW_STATE_IVAR is initialized when the view is created. The cursor registered
    // below is a valid NSCursor.
    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), resetCursorRects];

        let window_state = get_window_state(this);
        let cursor_style = window_state.lock().cursor_style;

        let cursor: ObjcId = match cursor_style {
            CursorStyle::Arrow => msg_send![class!(NSCursor), arrowCursor],
            CursorStyle::IBeam => msg_send![class!(NSCursor), IBeamCursor],
            CursorStyle::Crosshair => msg_send![class!(NSCursor), crosshairCursor],
            CursorStyle::ClosedHand => msg_send![class!(NSCursor), closedHandCursor],
            CursorStyle::OpenHand => msg_send![class!(NSCursor), openHandCursor],
            CursorStyle::PointingHand => msg_send![class!(NSCursor), pointingHandCursor],
            CursorStyle::ResizeLeftRight => msg_send![class!(NSCursor), resizeLeftRightCursor],
            CursorStyle::ResizeUpDown => msg_send![class!(NSCursor), resizeUpDownCursor],
            CursorStyle::ResizeLeft => msg_send![class!(NSCursor), resizeLeftCursor],
            CursorStyle::ResizeRight => msg_send![class!(NSCursor), resizeRightCursor],
            CursorStyle::ResizeColumn => msg_send![class!(NSCursor), resizeLeftRightCursor],
            CursorStyle::ResizeRow => msg_send![class!(NSCursor), resizeUpDownCursor],
            CursorStyle::ResizeUp => msg_send![class!(NSCursor), resizeUpCursor],
            CursorStyle::ResizeDown => msg_send![class!(NSCursor), resizeDownCursor],

            CursorStyle::ResizeUpLeft
            | CursorStyle::ResizeUpRight
            | CursorStyle::ResizeDownLeft
            | CursorStyle::ResizeDownRight
            | CursorStyle::ResizeUpLeftDownRight
            | CursorStyle::ResizeUpRightDownLeft => {
                let (position, directions) = directional_resize_spec(cursor_style).unwrap();
                directional_resize_cursor(position, directions)
            }

            CursorStyle::IBeamCursorForVerticalLayout => {
                msg_send![class!(NSCursor), IBeamCursorForVerticalLayout]
            }
            CursorStyle::OperationNotAllowed => {
                msg_send![class!(NSCursor), operationNotAllowedCursor]
            }
            CursorStyle::DragLink => msg_send![class!(NSCursor), dragLinkCursor],
            CursorStyle::DragCopy => msg_send![class!(NSCursor), dragCopyCursor],
            CursorStyle::ContextualMenu => msg_send![class!(NSCursor), contextualMenuCursor],
        };

        let bounds: Objc2NSRect = msg_send![this as *const Objc2Object as ObjcId, bounds];
        let _: () = msg_send![this, addCursorRect: bounds, cursor: cursor];
    }
}

fn directional_resize_spec(
    style: CursorStyle,
) -> Option<(NSCursorFrameResizePosition, NSCursorFrameResizeDirections)> {
    Some(match style {
        CursorStyle::ResizeUpLeft => (
            NSCursorFrameResizePosition::TopLeft,
            NSCursorFrameResizeDirections::Outward,
        ),
        CursorStyle::ResizeUpRight => (
            NSCursorFrameResizePosition::TopRight,
            NSCursorFrameResizeDirections::Outward,
        ),
        CursorStyle::ResizeDownLeft => (
            NSCursorFrameResizePosition::BottomLeft,
            NSCursorFrameResizeDirections::Outward,
        ),
        CursorStyle::ResizeDownRight => (
            NSCursorFrameResizePosition::BottomRight,
            NSCursorFrameResizeDirections::Outward,
        ),
        CursorStyle::ResizeUpLeftDownRight => (
            NSCursorFrameResizePosition::TopLeft,
            NSCursorFrameResizeDirections::All,
        ),
        CursorStyle::ResizeUpRightDownLeft => (
            NSCursorFrameResizePosition::TopRight,
            NSCursorFrameResizeDirections::All,
        ),
        _ => return None,
    })
}

/// Uses AppKit's directional frame cursors when available, with a legacy fallback.
unsafe fn directional_resize_cursor(
    position: NSCursorFrameResizePosition,
    directions: NSCursorFrameResizeDirections,
) -> ObjcId {
    let cursor_class = class!(NSCursor);
    let modern_selector = sel!(frameResizeCursorFromPosition:inDirections:);
    let supports_modern_api: Bool = msg_send![cursor_class, respondsToSelector: modern_selector];
    if supports_modern_api.as_bool() {
        msg_send![
            cursor_class,
            frameResizeCursorFromPosition: position,
            inDirections: directions
        ]
    } else {
        // Legacy cursors pair opposite corners.
        let nw_se = matches!(
            position,
            NSCursorFrameResizePosition::TopLeft | NSCursorFrameResizePosition::BottomRight
        );
        unsafe { legacy_diagonal_resize_cursor(nw_se) }
    }
}

/// Uses AppKit's private diagonal selectors as a fallback.
unsafe fn legacy_diagonal_resize_cursor(nw_se: bool) -> ObjcId {
    if nw_se {
        msg_send![class!(NSCursor), _windowResizeNorthWestSouthEastCursor]
    } else {
        msg_send![class!(NSCursor), _windowResizeNorthEastSouthWestCursor]
    }
}

unsafe extern "C" fn handle_key_equivalent(
    this: &Objc2Object,
    _: Sel,
    native_event: ObjcId,
) -> Bool {
    unsafe { handle_key_event(this, native_event, true) }
}

unsafe extern "C" fn handle_key_down(this: &Objc2Object, _: Sel, native_event: ObjcId) {
    unsafe { handle_key_event(this, native_event, false) };
}

unsafe extern "C" fn handle_key_up(this: &Objc2Object, _: Sel, native_event: ObjcId) {
    unsafe { handle_key_event(this, native_event, false) };
}

// Things to test if you're modifying this method:
//  U.S. layout:
//   - The IME consumes characters like 'j' and 'k', which makes paging through `less` in
//     the terminal behave incorrectly by default. This behavior should be patched by our
//     IME integration
//   - `alt-t` should open the tasks menu
//   - In vim mode, this keybinding should work:
//     ```
//        {
//          "context": "Editor && vim_mode == insert",
//          "bindings": {"j j": "vim::NormalBefore"}
//        }
//     ```
//     and typing 'j k' in insert mode with this keybinding should insert the two characters
//  Brazilian layout:
//   - `" space` should create an unmarked quote
//   - `" backspace` should delete the marked quote
//   - `" "`should create an unmarked quote and a second marked quote
//   - `" up` should insert a quote, unmark it, and move up one line
//   - `" cmd-down` should insert a quote, unmark it, and move to the end of the file
//   - `cmd-ctrl-space` and clicking on an emoji should type it
//  Czech (QWERTY) layout:
//   - in vim mode `option-4`  should go to end of line (same as $)
//  Japanese (Romaji) layout:
//   - type `a i left down up enter enter` should create an unmarked text "愛"
//   - In vim mode with `jj` bound to `vim::NormalBefore` in insert mode, typing 'j i' with
//     Japanese IME should produce "じ" (ji), not "jい"

/// Returns true if the current keyboard input source is a composition-based IME
/// (e.g. Japanese Hiragana, Korean, Chinese Pinyin) that produces non-ASCII output.
///
/// This checks two properties:
/// 1. The source type is `kTISTypeKeyboardInputMode` (an IME input mode, not a plain
///    keyboard layout). This excludes non-ASCII layouts like Armenian and Ukrainian
///    that map keys directly without composition.
/// 2. The source is not ASCII-capable, which excludes modes like Japanese Romaji that
///    produce ASCII characters and should allow multi-stroke keybindings like `jj`.
unsafe fn is_ime_input_source_active() -> bool {
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return false;
        }

        let source_type =
            TISGetInputSourceProperty(source, kTISPropertyInputSourceType as *const c_void);
        let is_input_mode = !source_type.is_null()
            && CFEqual(
                source_type as CFTypeRef,
                kTISTypeKeyboardInputMode as CFTypeRef,
            ) != 0;

        let is_ascii = TISGetInputSourceProperty(
            source,
            kTISPropertyInputSourceIsASCIICapable as *const c_void,
        );
        let is_ascii_capable = !is_ascii.is_null() && CFBooleanGetValue(is_ascii as CFBooleanRef);

        CFRelease(source as CFTypeRef);

        is_input_mode && !is_ascii_capable
    }
}

unsafe extern "C" fn handle_key_event(
    this: &Objc2Object,
    native_event: ObjcId,
    key_equivalent: bool,
) -> Bool {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();

    let window_height = lock.content_size().height;
    let event = unsafe { platform_input_from_native(native_event.cast(), Some(window_height)) };

    let Some(event) = event else {
        return Bool::new(false);
    };

    let run_callback = |event: PlatformInput| -> Bool {
        let mut callback = window_state.as_ref().lock().event_callback.take();
        let handled: Bool = if let Some(callback) = callback.as_mut() {
            Bool::new(!callback(event).propagate)
        } else {
            Bool::new(false)
        };
        window_state.as_ref().lock().event_callback = callback;
        handled
    };

    match event {
        PlatformInput::KeyDown(key_down_event) => {
            // For certain keystrokes, macOS will first dispatch a "key equivalent" event.
            // If that event isn't handled, it will then dispatch a "key down" event. GPUI
            // makes no distinction between these two types of events, so we need to ignore
            // the "key down" event if we've already just processed its "key equivalent" version.
            if key_equivalent {
                lock.last_key_equivalent = Some(key_down_event.clone());
            } else if lock.last_key_equivalent.take().as_ref() == Some(&key_down_event) {
                return Bool::new(false);
            }

            drop(lock);

            let is_composing =
                with_input_handler(this, |input_handler| input_handler.marked_text_range())
                    .flatten()
                    .is_some();

            // If we're composing, send the key to the input handler first;
            // otherwise we only send to the input handler if we don't have a matching binding.
            // The input handler may call `do_command_by_selector` if it doesn't know how to handle
            // a key. If it does so, it will return Bool::new(true) so we won't send the key twice.
            // We also do this for non-printing keys (like arrow keys and escape) as the IME menu
            // may need them even if there is no marked text;
            // however we skip keys with control or the input handler adds control-characters to the buffer.
            // and keys with function, as the input handler swallows them.
            // and keys with platform (Cmd), so that Cmd+key events (e.g. Cmd+`) are not
            // consumed by the IME on non-QWERTY / dead-key layouts.
            // We also send printable keys to the IME first when an IME input source (e.g. Japanese,
            // Korean, Chinese) is active and the input handler accepts text input. This prevents
            // multi-stroke keybindings like `jj` from intercepting keys that the IME should compose
            // (e.g. typing 'ji' should produce 'じ', not 'jい'). If the IME doesn't handle the key,
            // it calls `doCommandBySelector:` which routes it back to keybinding matching.
            let is_ime_printable_key = !is_composing
                && key_down_event
                    .keystroke
                    .key_char
                    .as_ref()
                    .is_some_and(|key_char| key_char.chars().all(|c| !c.is_control()))
                && !key_down_event.keystroke.modifiers.control
                && !key_down_event.keystroke.modifiers.function
                && !key_down_event.keystroke.modifiers.platform
                && unsafe { is_ime_input_source_active() }
                && with_input_handler(this, |input_handler| {
                    input_handler.query_prefers_ime_for_printable_keys()
                })
                .unwrap_or(false);

            if is_composing
                || is_ime_printable_key
                || (key_down_event.keystroke.key_char.is_none()
                    && !key_down_event.keystroke.modifiers.control
                    && !key_down_event.keystroke.modifiers.function
                    && !key_down_event.keystroke.modifiers.platform)
            {
                {
                    let mut lock = window_state.as_ref().lock();
                    lock.keystroke_for_do_command = Some(key_down_event.keystroke.clone());
                    lock.do_command_handled.take();
                    drop(lock);
                }

                let handled: Bool = unsafe {
                    let input_context: ObjcId = msg_send![this, inputContext];
                    msg_send![input_context, handleEvent: native_event]
                };
                window_state.as_ref().lock().keystroke_for_do_command.take();
                if let Some(handled) = window_state.as_ref().lock().do_command_handled.take() {
                    return Bool::new(handled);
                } else if handled == Bool::new(true) {
                    return Bool::new(true);
                }

                let handled = run_callback(PlatformInput::KeyDown(key_down_event));
                return handled;
            }

            let handled = run_callback(PlatformInput::KeyDown(key_down_event.clone()));
            if handled == Bool::new(true) {
                return Bool::new(true);
            }

            if key_down_event.is_held
                && let Some(key_char) = key_down_event.keystroke.key_char.as_ref()
            {
                let handled = with_input_handler(this, |input_handler| {
                    if !input_handler.apple_press_and_hold_enabled() {
                        input_handler.replace_text_in_range(None, key_char);
                        return Bool::new(true);
                    }
                    Bool::new(false)
                });
                if handled == Some(Bool::new(true)) {
                    return Bool::new(true);
                }
            }

            // Don't send key equivalents to the input handler if there are key modifiers other
            // than Function key, or macOS shortcuts like cmd-` will stop working.
            if key_equivalent && key_down_event.keystroke.modifiers != Modifiers::function() {
                return Bool::new(false);
            }

            unsafe {
                let input_context: ObjcId = msg_send![this, inputContext];
                msg_send![input_context, handleEvent: native_event]
            }
        }

        PlatformInput::KeyUp(_) => {
            drop(lock);
            run_callback(event)
        }

        _ => Bool::new(false),
    }
}

unsafe extern "C" fn handle_view_event(this: &Objc2Object, _: Sel, native_event: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let weak_window_state = Arc::downgrade(&window_state);
    let mut lock = window_state.as_ref().lock();
    let window_height = lock.content_size().height;
    let native_event_type = unsafe { native_event.eventType() };
    match native_event_type {
        NSEventType::LeftMouseDown => {
            // AppKit owns `native_event` for the callback; retain it so the drag session can still
            // be started later, once the pointer leaves the window.
            lock.last_left_mouse_down_event =
                unsafe { Retained::retain(native_event.cast::<Objc2Object>()) };
        }
        NSEventType::LeftMouseUp => {
            lock.last_left_mouse_down_event = None;
        }
        _ => {}
    }
    let event = unsafe { platform_input_from_native(native_event.cast(), Some(window_height)) };

    if let Some(mut event) = event {
        // AppKit unhides the cursor on the next mouse movement; mirror that here.
        if matches!(
            event,
            PlatformInput::MouseMove(_)
                | PlatformInput::MouseDown(_)
                | PlatformInput::MouseUp(_)
                | PlatformInput::MousePressure(_)
                | PlatformInput::MouseExited(_)
                | PlatformInput::ScrollWheel(_)
                | PlatformInput::Pinch(_)
        ) {
            lock.cursor_visible.store(true, Ordering::Relaxed);
        }

        match &mut event {
            PlatformInput::MouseDown(
                event @ MouseDownEvent {
                    button: MouseButton::Left,
                    modifiers: Modifiers { control: true, .. },
                    ..
                },
            ) => {
                // On mac, a ctrl-left click should be handled as a right click.
                *event = MouseDownEvent {
                    button: MouseButton::Right,
                    modifiers: Modifiers {
                        control: false,
                        ..event.modifiers
                    },
                    click_count: 1,
                    ..*event
                };
            }

            // Handles focusing click.
            PlatformInput::MouseDown(
                event @ MouseDownEvent {
                    button: MouseButton::Left,
                    ..
                },
            ) if (lock.first_mouse) => {
                *event = MouseDownEvent {
                    first_mouse: true,
                    ..*event
                };
                lock.first_mouse = false;
            }

            // Because we map a ctrl-left_down to a right_down -> right_up let's ignore
            // the ctrl-left_up to avoid having a mismatch in button down/up events if the
            // user is still holding ctrl when releasing the left mouse button
            PlatformInput::MouseUp(
                event @ MouseUpEvent {
                    button: MouseButton::Left,
                    modifiers: Modifiers { control: true, .. },
                    ..
                },
            ) => {
                *event = MouseUpEvent {
                    button: MouseButton::Right,
                    modifiers: Modifiers {
                        control: false,
                        ..event.modifiers
                    },
                    click_count: 1,
                    ..*event
                };
            }

            _ => {}
        };

        match &event {
            PlatformInput::MouseDown(_) => {
                drop(lock);
                unsafe {
                    let input_context: ObjcId = msg_send![this, inputContext];
                    let _: Bool = msg_send![input_context, handleEvent: native_event];
                }
                lock = window_state.as_ref().lock();
            }
            PlatformInput::MouseMove(
                event @ MouseMoveEvent {
                    pressed_button: Some(_),
                    ..
                },
            ) => {
                // Synthetic drag is used for selecting long buffer contents while buffer is being scrolled.
                // External file drag and drop is able to emit its own synthetic mouse events which will conflict
                // with these ones.
                if !lock.external_files_dragged {
                    lock.synthetic_drag_counter += 1;
                    let executor = lock.foreground_executor.clone();
                    executor
                        .spawn(synthetic_drag(
                            weak_window_state,
                            lock.synthetic_drag_counter,
                            event.clone(),
                            lock.background_executor.clone(),
                        ))
                        .detach();
                }
            }

            PlatformInput::MouseUp(MouseUpEvent { .. }) => {
                lock.synthetic_drag_counter += 1;
            }

            PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                modifiers,
                capslock,
            }) => {
                // Only raise modifiers changed event when they have actually changed
                if let Some(PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                    modifiers: prev_modifiers,
                    capslock: prev_capslock,
                })) = &lock.previous_modifiers_changed_event
                    && prev_modifiers == modifiers
                    && prev_capslock == capslock
                {
                    return;
                }

                lock.previous_modifiers_changed_event = Some(event.clone());
            }

            _ => {}
        }

        if let Some(mut callback) = lock.event_callback.take() {
            drop(lock);
            callback(event);
            window_state.lock().event_callback = Some(callback);
        }
    }
}

unsafe extern "C" fn window_did_change_occlusion_state(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let lock = &mut *window_state.lock();
    unsafe {
        if lock
            .native_window
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible)
        {
            lock.move_traffic_light();
            lock.start_display_link();
        } else {
            lock.stop_display_link();
        }
    }
}

unsafe extern "C" fn window_did_resize(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    window_state.as_ref().lock().move_traffic_light();
}

unsafe extern "C" fn window_will_enter_fullscreen(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    lock.fullscreen_restore_bounds = lock.bounds();
    lock.restore_traffic_light();

    let min_version = NSOperatingSystemVersion {
        majorVersion: 15,
        minorVersion: 3,
        patchVersion: 0,
    };

    if is_macos_version_at_least(min_version) {
        unsafe {
            lock.native_window
                .setTitlebarAppearsTransparent_(Bool::new(false));
        }
    }
}

unsafe extern "C" fn window_will_exit_fullscreen(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let lock = window_state.as_ref().lock();

    let min_version = NSOperatingSystemVersion {
        majorVersion: 15,
        minorVersion: 3,
        patchVersion: 0,
    };

    if is_macos_version_at_least(min_version) && lock.transparent_titlebar {
        unsafe {
            lock.native_window
                .setTitlebarAppearsTransparent_(Bool::new(true));
        }
    }
}

unsafe extern "C" fn window_did_exit_fullscreen(this: &Objc2Object, _: Sel, _: ObjcId) {
    // SAFETY: This method is registered only on GPUI window classes, which initialize
    // WINDOW_STATE_IVAR with an Arc<Mutex<MacWindowState>> during window creation.
    let window_state = unsafe { get_window_state(this) };
    window_state.as_ref().lock().move_traffic_light();
}

pub(crate) fn is_macos_version_at_least(version: NSOperatingSystemVersion) -> bool {
    unsafe {
        let process_info: ObjcId = msg_send![class!(NSProcessInfo), processInfo];
        let result: Bool = msg_send![process_info, isOperatingSystemAtLeastVersion: version];
        result.into()
    }
}

unsafe extern "C" fn window_did_move(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.moved_callback.take() {
        drop(lock);
        callback();
        window_state.lock().moved_callback = Some(callback);
    }
}

// Update the window scale factor and drawable size, and call the resize callback if any.
fn update_window_scale_factor(window_state: &Arc<Mutex<MacWindowState>>) {
    let mut lock = window_state.as_ref().lock();
    let scale_factor = lock.scale_factor();
    let size = lock.content_size();
    let drawable_size = size.to_device_pixels(scale_factor);
    if let Some(layer) = lock.renderer.layer() {
        unsafe {
            let _: () = msg_send![
                layer.as_ptr() as ObjcId,
                setContentsScale: scale_factor as f64
            ];
        }
    }

    lock.renderer.update_drawable_size(drawable_size);

    if let Some(mut callback) = lock.resize_callback.take() {
        let content_size = lock.content_size();
        let scale_factor = lock.scale_factor();
        drop(lock);
        callback(content_size, scale_factor);
        window_state.as_ref().lock().resize_callback = Some(callback);
    };
}

unsafe extern "C" fn window_did_change_screen(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    lock.start_display_link();
    drop(lock);
    update_window_scale_factor(&window_state);
}

unsafe extern "C" fn window_did_change_key_status(this: &Objc2Object, selector: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let lock = window_state.lock();
    let is_active = unsafe { lock.native_window.isKeyWindow() == Bool::new(true) };

    // AppKit also unhides the cursor on activation changes, so mirror that here.
    lock.cursor_visible.store(true, Ordering::Relaxed);

    // When opening a pop-up while the application isn't active, Cocoa sends a spurious
    // `windowDidBecomeKey` message to the previous key window even though that window
    // isn't actually key. This causes a bug if the application is later activated while
    // the pop-up is still open, making it impossible to activate the previous key window
    // even if the pop-up gets closed. The only way to activate it again is to de-activate
    // the app and re-activate it, which is a pretty bad UX.
    // The following code detects the spurious event and invokes `resignKeyWindow`:
    // in theory, we're not supposed to invoke this method manually but it balances out
    // the spurious `becomeKeyWindow` event and helps us work around that bug.
    if selector == sel!(windowDidBecomeKey:) && !is_active {
        let native_window = lock.native_window;
        drop(lock);
        unsafe {
            let _: () = msg_send![native_window, resignKeyWindow];
        }
        return;
    }

    let executor = lock.foreground_executor.clone();
    drop(lock);

    let a11y_events = {
        let mut lock = window_state.lock();
        lock.accesskit_adapter
            .as_mut()
            .and_then(|adapter| adapter.update_view_focus_state(is_active))
    };
    if let Some(events) = a11y_events {
        events.raise();
    }

    // When a window becomes active, trigger an immediate synchronous frame request to prevent
    // tab flicker when switching between windows in native tabs mode.
    //
    // This is only done on subsequent activations (not the first) to ensure the initial focus
    // path is properly established. Without this guard, the focus state would remain unset until
    // the first mouse click, causing keybindings to be non-functional.
    if selector == sel!(windowDidBecomeKey:) && is_active {
        let window_state = unsafe { get_window_state(this) };
        let mut lock = window_state.lock();

        if lock.activated_least_once {
            if let Some(mut callback) = lock.request_frame_callback.take() {
                lock.renderer.set_presents_with_transaction(true);
                lock.stop_display_link();
                let request = lock.next_frame_request();
                drop(lock);
                callback(request);

                let mut lock = window_state.lock();
                lock.request_frame_callback = Some(callback);
                lock.renderer.set_presents_with_transaction(false);
                lock.start_display_link();
            }
        } else {
            lock.activated_least_once = true;
        }
    }

    executor
        .spawn(async move {
            let mut lock = window_state.as_ref().lock();
            if is_active {
                lock.move_traffic_light();
            }

            if let Some(mut callback) = lock.activate_callback.take() {
                drop(lock);
                callback(is_active);
                window_state.lock().activate_callback = Some(callback);
            };
        })
        .detach();
}

unsafe extern "C" fn window_should_close(this: &Objc2Object, _: Sel, _: ObjcId) -> Bool {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.should_close_callback.take() {
        drop(lock);
        let should_close = callback();
        window_state.lock().should_close_callback = Some(callback);
        Bool::new(should_close)
    } else {
        Bool::new(true)
    }
}

unsafe extern "C" fn close_window(this: &Objc2Object, _: Sel) {
    unsafe {
        let (close_callback, simple_fullscreen_state) = {
            let window_state = get_window_state(this);
            let mut lock = window_state.as_ref().lock();
            lock.closed.store(true, Ordering::Release);
            (
                lock.close_callback.take(),
                lock.simple_fullscreen_state.take(),
            )
        };

        if simple_fullscreen_state.is_some() {
            pop_simple_fullscreen_presentation_options();
        }

        if let Some(callback) = close_callback {
            callback();
        }

        let _: () = msg_send![super(this, class!(NSWindow)), close];
    }
}

unsafe extern "C" fn make_backing_layer(this: &Objc2Object, _: Sel) -> ObjcId {
    let window_state = unsafe { get_window_state(this) };
    let window_state = window_state.as_ref().lock();
    window_state.renderer.layer_ptr() as ObjcId
}

unsafe extern "C" fn view_did_change_backing_properties(this: &Objc2Object, _: Sel) {
    let window_state = unsafe { get_window_state(this) };
    update_window_scale_factor(&window_state);
}

unsafe extern "C" fn set_frame_size(this: &Objc2Object, _: Sel, size: NSSize) {
    fn convert(value: NSSize) -> Size<Pixels> {
        Size {
            width: px(value.width as f32),
            height: px(value.height as f32),
        }
    }

    let old_size = unsafe {
        let old_frame: Objc2NSRect = msg_send![this, frame];
        convert(old_frame.size)
    };
    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), setFrameSize: size];
    }

    // `initWithFrame:` may dispatch here before `windowState` is installed. Superclass
    // initialization is complete at this point, but there is no renderer to resize yet.
    let Some(window_state) = (unsafe { initialized_window_state(this) }) else {
        return;
    };
    let mut lock = window_state.as_ref().lock();
    let new_size = convert(size);
    if old_size == new_size {
        return;
    }

    let scale_factor = lock.scale_factor();
    let drawable_size = new_size.to_device_pixels(scale_factor);
    lock.renderer.update_drawable_size(drawable_size);

    if let Some(mut callback) = lock.resize_callback.take() {
        let content_size = lock.content_size();
        let scale_factor = lock.scale_factor();
        drop(lock);
        callback(content_size, scale_factor);
        window_state.lock().resize_callback = Some(callback);
    };
}

unsafe extern "C" fn display_layer(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.lock();
    if let Some(mut callback) = lock.request_frame_callback.take() {
        lock.renderer.set_presents_with_transaction(true);
        lock.stop_display_link();
        let request = lock.next_frame_request();
        drop(lock);
        callback(request);

        let mut lock = window_state.lock();
        lock.request_frame_callback = Some(callback);
        lock.renderer.set_presents_with_transaction(false);
        lock.start_display_link();
    }
}

extern "C" fn step(view: *mut c_void) {
    unsafe {
        let view = view as ObjcId;
        let window_state = get_window_state(&*view);
        let mut lock = window_state.lock();

        if let Some(mut callback) = lock.request_frame_callback.take() {
            let request = lock.next_frame_request();
            drop(lock);
            callback(request);
            window_state.lock().request_frame_callback = Some(callback);
        }
    }
}

unsafe extern "C" fn valid_attributes_for_marked_text(_: &Objc2Object, _: Sel) -> ObjcId {
    unsafe { msg_send![class!(NSArray), array] }
}

unsafe extern "C" fn has_marked_text(this: &Objc2Object, _: Sel) -> Bool {
    let has_marked_text_result =
        with_input_handler(this, |input_handler| input_handler.marked_text_range()).flatten();

    Bool::new(has_marked_text_result.is_some())
}

unsafe extern "C" fn marked_range(this: &Objc2Object, _: Sel) -> NSRange {
    let marked_range_result =
        with_input_handler(this, |input_handler| input_handler.marked_text_range()).flatten();

    marked_range_result.map_or(invalid_ns_range(), |range| range.into())
}

unsafe extern "C" fn selected_range(this: &Objc2Object, _: Sel) -> NSRange {
    let selected_range_result = with_input_handler(this, |input_handler| {
        input_handler.selected_text_range(false)
    })
    .flatten();

    selected_range_result.map_or(invalid_ns_range(), |selection| selection.range.into())
}

unsafe extern "C" fn first_rect_for_character_range(
    this: &Objc2Object,
    _: Sel,
    range: NSRange,
    actual_range: NSRangePointer,
) -> Objc2NSRect {
    if !actual_range.is_null() {
        unsafe { actual_range.write(range) };
    }
    let frame = get_frame(this);
    with_input_handler(this, |input_handler| {
        input_handler.bounds_for_range(range.to_range_option()?)
    })
    .flatten()
    .map_or(
        Objc2NSRect::new(Objc2NSPoint::new(0., 0.), NSSize::new(0., 0.)),
        |bounds| {
            Objc2NSRect::new(
                Objc2NSPoint::new(
                    frame.origin.x + bounds.origin.x.as_f32() as f64,
                    frame.origin.y + frame.size.height
                        - bounds.origin.y.as_f32() as f64
                        - bounds.size.height.as_f32() as f64,
                ),
                NSSize::new(
                    bounds.size.width.as_f32() as f64,
                    bounds.size.height.as_f32() as f64,
                ),
            )
        },
    )
}

fn get_frame(this: &Objc2Object) -> Objc2NSRect {
    unsafe {
        let state = get_window_state(this);
        let lock = state.lock();
        let mut frame: Objc2NSRect = msg_send![lock.native_window, frame];
        let content_layout_rect: Objc2NSRect = msg_send![lock.native_window, contentLayoutRect];
        let style_mask: NSWindowStyleMask = msg_send![lock.native_window, styleMask];
        if !style_mask.contains(NSWindowStyleMask::FullSizeContentView) {
            frame.origin.y -= frame.size.height - content_layout_rect.size.height;
        }
        frame
    }
}

unsafe extern "C" fn insert_text(
    this: &Objc2Object,
    _: Sel,
    text: ObjcId,
    replacement_range: NSRange,
) {
    unsafe {
        let is_attributed_string: Bool = msg_send![text, isKindOfClass: class!(NSAttributedString)];
        let text: ObjcId = if is_attributed_string == Bool::new(true) {
            msg_send![text, string]
        } else {
            text
        };

        let text = objc_string(text);
        let replacement_range = replacement_range.to_range_option();
        with_input_handler(this, |input_handler| {
            input_handler.replace_text_in_range(replacement_range, &text)
        });
    }
}

unsafe extern "C" fn set_marked_text(
    this: &Objc2Object,
    _: Sel,
    text: ObjcId,
    selected_range: NSRange,
    replacement_range: NSRange,
) {
    unsafe {
        let is_attributed_string: Bool = msg_send![text, isKindOfClass: class!(NSAttributedString)];
        let text: ObjcId = if is_attributed_string == Bool::new(true) {
            msg_send![text, string]
        } else {
            text
        };
        let selected_range = selected_range.to_range_option();
        let replacement_range = replacement_range.to_range_option();
        let text = objc_string(text);
        with_input_handler(this, |input_handler| {
            input_handler.replace_and_mark_text_in_range(replacement_range, &text, selected_range)
        });
    }
}
unsafe extern "C" fn unmark_text(this: &Objc2Object, _: Sel) {
    with_input_handler(this, |input_handler| input_handler.unmark_text());
}

unsafe extern "C" fn attributed_substring_for_proposed_range(
    this: &Objc2Object,
    _: Sel,
    range: NSRange,
    actual_range: NSRangePointer,
) -> ObjcId {
    with_input_handler(this, |input_handler| {
        let range = range.to_range_option()?;
        if range.is_empty() {
            return None;
        }
        let mut adjusted: Option<Range<usize>> = None;

        let selected_text = input_handler.text_for_range(range.clone(), &mut adjusted)?;
        if !actual_range.is_null() {
            let actual_range_value = adjusted.as_ref().unwrap_or(&range);
            unsafe { actual_range.write(NSRange::from(actual_range_value.clone())) };
        }
        unsafe {
            let string: ObjcId = msg_send![class!(NSAttributedString), alloc];
            let selected_text = ns_string(&selected_text);
            let string: ObjcId = msg_send![string, initWithString: &*selected_text];
            Some(string)
        }
    })
    .flatten()
    .unwrap_or(NIL)
}

// We ignore which selector it asks us to do because the user may have
// bound the shortcut to something else.
unsafe extern "C" fn do_command_by_selector(this: &Objc2Object, _: Sel, _: Sel) {
    let state = unsafe { get_window_state(this) };
    let mut lock = state.as_ref().lock();
    let keystroke = lock.keystroke_for_do_command.take();
    let mut event_callback = lock.event_callback.take();
    drop(lock);

    if let Some((keystroke, callback)) = keystroke.zip(event_callback.as_mut()) {
        let handled = (callback)(PlatformInput::KeyDown(KeyDownEvent {
            keystroke,
            is_held: false,
            prefer_character_input: false,
        }));
        state.as_ref().lock().do_command_handled = Some(!handled.propagate);
    }

    state.as_ref().lock().event_callback = event_callback;
}

unsafe extern "C" fn view_did_change_effective_appearance(this: &Objc2Object, _: Sel) {
    unsafe {
        let state = get_window_state(this);
        let appearance_changed_callback = {
            let mut lock = state.as_ref().lock();
            lock.appearance_changed_callback.take()
        };

        if let Some(mut callback) = appearance_changed_callback {
            callback();
            state.lock().appearance_changed_callback = Some(callback);
        }

        // AppKit can relayout the standard traffic light buttons as part of
        // applying a new appearance, so reapply GPUI's custom position.
        state.lock().move_traffic_light();
    }
}

unsafe extern "C" fn accepts_first_mouse(this: &Objc2Object, _: Sel, _: ObjcId) -> Bool {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    lock.first_mouse = true;
    Bool::new(true)
}

// Reports which region of the view AppKit should treat as app-owned titlebar content
// (rather than a system-owned window-move region). When `app_owns_titlebar_drag` is
// true, we claim the entire view so AppKit neither drags the window from the titlebar
// nor waits to disambiguate double-clicks before delivering titlebar clicks (the macOS
// 27 delay); such windows implement dragging themselves via [`Window::start_window_move`].
// Otherwise we return an empty rect so AppKit's native titlebar dragging keeps working.
// This is independent of `NSWindow.isMovable`, so the Window-menu tiling items stay
// enabled regardless.
unsafe extern "C" fn opaque_rect_for_window_move_when_in_titlebar(
    this: &Objc2Object,
    _: Sel,
) -> Objc2NSRect {
    let zero_rect = Objc2NSRect::new(Objc2NSPoint::new(0., 0.), NSSize::new(0., 0.));
    let window_state = unsafe { get_window_state(this) };
    let app_owns_titlebar_drag = window_state.as_ref().lock().app_owns_titlebar_drag;
    if app_owns_titlebar_drag {
        unsafe { msg_send![this, bounds] }
    } else {
        zero_rect
    }
}

unsafe extern "C" fn character_index_for_point(
    this: &Objc2Object,
    _: Sel,
    position: Objc2NSPoint,
) -> u64 {
    let position = screen_point_to_gpui_point(this, position);
    with_input_handler(this, |input_handler| {
        input_handler.character_index_for_point(position)
    })
    .flatten()
    .map(|index| index as u64)
    .unwrap_or(NSNotFound as u64)
}

fn screen_point_to_gpui_point(this: &Objc2Object, position: Objc2NSPoint) -> Point<Pixels> {
    let frame = get_frame(this);
    let window_x = position.x - frame.origin.x;
    let window_y = frame.size.height - (position.y - frame.origin.y);

    point(px(window_x as f32), px(window_y as f32))
}

fn is_drag_from_this_window(this: &Objc2Object, dragging_info: ObjcId) -> bool {
    let source: ObjcId = unsafe { msg_send![dragging_info, draggingSource] };
    std::ptr::eq(source as *const Objc2Object, this as *const Objc2Object)
}

unsafe extern "C" fn dragging_entered(
    this: &Objc2Object,
    _: Sel,
    dragging_info: ObjcId,
) -> NSDragOperation {
    let is_source_window = is_drag_from_this_window(this, dragging_info);
    let window_state = unsafe { get_window_state(this) };
    let position = drag_event_position(&window_state, dragging_info);
    let paths = external_paths_from_event(dragging_info);
    if let Some(event) = paths.map(|paths| FileDropEvent::Entered { position, paths })
        && send_file_drop_event(window_state, event)
    {
        if is_source_window {
            return NSDragOperationMove;
        }
        return NSDragOperationCopy;
    }
    NSDragOperationNone
}

unsafe extern "C" fn dragging_updated(
    this: &Objc2Object,
    _: Sel,
    dragging_info: ObjcId,
) -> NSDragOperation {
    let is_source_window = is_drag_from_this_window(this, dragging_info);
    let window_state = unsafe { get_window_state(this) };
    let position = drag_event_position(&window_state, dragging_info);
    if send_file_drop_event(window_state, FileDropEvent::Pending { position }) {
        if is_source_window {
            NSDragOperationMove
        } else {
            NSDragOperationCopy
        }
    } else {
        NSDragOperationNone
    }
}

unsafe extern "C" fn dragging_exited(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    send_file_drop_event(window_state, FileDropEvent::Exited);
}

unsafe extern "C" fn perform_drag_operation(
    this: &Objc2Object,
    _: Sel,
    dragging_info: ObjcId,
) -> Bool {
    let window_state = unsafe { get_window_state(this) };
    let position = drag_event_position(&window_state, dragging_info);
    Bool::new(send_file_drop_event(
        window_state,
        FileDropEvent::Submit { position },
    ))
}

fn external_paths_from_event(dragging_info: *mut Objc2Object) -> Option<ExternalPaths> {
    let mut paths = SmallVec::new();
    let pasteboard: ObjcId = unsafe { msg_send![dragging_info, draggingPasteboard] };
    let filenames_type = filenames_pboard_type();
    let filenames: ObjcId = unsafe { msg_send![pasteboard, propertyListForType: &*filenames_type] };
    if filenames == NIL {
        return None;
    }
    let count: NSUInteger = unsafe { msg_send![filenames, count] };
    for index in 0..count {
        let file: ObjcId = unsafe { msg_send![filenames, objectAtIndex: index] };
        let path = unsafe {
            let f = msg_send![file, UTF8String];
            CStr::from_ptr(f).to_string_lossy().into_owned()
        };
        paths.push(PathBuf::from(path))
    }
    Some(ExternalPaths(paths))
}

unsafe extern "C" fn conclude_drag_operation(this: &Objc2Object, _: Sel, _: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    send_file_drop_event(window_state, FileDropEvent::Exited);
}

unsafe extern "C" fn dragging_session_source_operation_mask(
    _: &Objc2Object,
    _: Sel,
    _: ObjcId,
    context: NSInteger,
) -> NSDragOperation {
    let operation = match context {
        NSDRAGGING_CONTEXT_OUTSIDE_APPLICATION => NSDragOperationCopy,
        NSDRAGGING_CONTEXT_WITHIN_APPLICATION => NSDragOperationCopy | NSDragOperationMove,
        _ => NSDragOperationCopy | NSDragOperationMove,
    };
    log::debug!(
        "dragging_session_source_operation_mask: context={}, operation={}",
        context,
        operation
    );
    operation
}

unsafe extern "C" fn dragging_session_ended(
    this: &Objc2Object,
    _: Sel,
    _: ObjcId,
    _: Objc2NSPoint,
    operation: NSDragOperation,
) {
    log::debug!("dragging_session_ended operation={operation}");
    // SAFETY: AppKit invokes this selector on the GPUIWindow instance registered in build_classes,
    // which always has WINDOW_STATE_IVAR initialized to the owning MacWindowState.
    let window_state = unsafe { get_window_state(this) };
    {
        let mut lock = window_state.lock();
        lock.synthetic_drag_counter += 1;
        lock.last_left_mouse_down_event = None;
    }
    send_file_drop_event(window_state, FileDropEvent::Ended);
}

async fn synthetic_drag(
    window_state: Weak<Mutex<MacWindowState>>,
    drag_id: usize,
    event: MouseMoveEvent,
    executor: BackgroundExecutor,
) {
    loop {
        executor.timer(Duration::from_millis(16)).await;
        if let Some(window_state) = window_state.upgrade() {
            let mut lock = window_state.lock();
            if lock.synthetic_drag_counter == drag_id {
                if let Some(mut callback) = lock.event_callback.take() {
                    drop(lock);
                    callback(PlatformInput::MouseMove(event.clone()));
                    window_state.lock().event_callback = Some(callback);
                }
            } else {
                break;
            }
        }
    }
}

/// Sends the specified FileDropEvent using `PlatformInput::FileDrop` to the window
/// state and updates the window state according to the event passed.
fn send_file_drop_event(
    window_state: Arc<Mutex<MacWindowState>>,
    file_drop_event: FileDropEvent,
) -> bool {
    let external_files_dragged = match file_drop_event {
        FileDropEvent::Entered { .. } => Some(true),
        FileDropEvent::Exited | FileDropEvent::Ended => Some(false),
        _ => None,
    };

    let mut lock = window_state.lock();
    if let Some(mut callback) = lock.event_callback.take() {
        drop(lock);
        callback(PlatformInput::FileDrop(file_drop_event));
        let mut lock = window_state.lock();
        lock.event_callback = Some(callback);
        if let Some(external_files_dragged) = external_files_dragged {
            lock.external_files_dragged = external_files_dragged;
        }
        true
    } else {
        false
    }
}

fn drag_event_position(
    window_state: &Mutex<MacWindowState>,
    dragging_info: ObjcId,
) -> Point<Pixels> {
    let drag_location: Objc2NSPoint = unsafe { msg_send![dragging_info, draggingLocation] };
    convert_mouse_position(drag_location, window_state.lock().content_size().height)
}

fn with_input_handler<F, R>(window: &Objc2Object, f: F) -> Option<R>
where
    F: FnOnce(&mut PlatformInputHandler) -> R,
{
    let window_state = unsafe { get_window_state(window) };
    let mut lock = window_state.as_ref().lock();
    if let Some(mut input_handler) = lock.input_handler.take() {
        drop(lock);
        let result = f(&mut input_handler);
        window_state.lock().input_handler = Some(input_handler);
        Some(result)
    } else {
        None
    }
}

fn display_id_for_screen(screen: ObjcId) -> Option<CGDirectDisplayID> {
    // The raw window bridge gives us an untyped object, but it is documented to return an
    // NSScreen. Rebind it immediately, then use the stable NSScreenNumber bridge rather than
    // the newer -[NSScreen CGDirectDisplayID] selector, which is absent on older macOS.
    unsafe {
        screen
            .cast::<NSScreen>()
            .as_ref()
            .and_then(crate::display::screen_id)
    }
}

unsafe extern "C" fn blurred_view_init_with_frame(
    this: &Objc2Object,
    _: Sel,
    frame: Objc2NSRect,
) -> ObjcId {
    unsafe {
        let view = msg_send![super(this, class!(NSVisualEffectView)), initWithFrame: frame];
        // Use a colorless semantic material. The default value `AppearanceBased`, though not
        // manually set, is deprecated.
        let _: () = msg_send![view, setMaterial: NSVisualEffectMaterial::Selection];
        let _: () = msg_send![view, setState: NSVisualEffectState::Active];
        view
    }
}

unsafe extern "C" fn blurred_view_update_layer(this: &Objc2Object, _: Sel) {
    unsafe {
        let _: () = msg_send![super(this, class!(NSVisualEffectView)), updateLayer];
        let layer: ObjcId = msg_send![this, layer];
        if !layer.is_null() {
            remove_layer_background(layer);
        }
    }
}

unsafe fn remove_layer_background(layer: ObjcId) {
    unsafe {
        let _: () = msg_send![layer, setBackgroundColor:NIL];

        let class_name: ObjcId = msg_send![layer, className];
        if class_name.isEqualToString("CAChameleonLayer").as_bool() {
            // Remove the desktop tinting effect.
            let _: () = msg_send![layer, setHidden: Bool::new(true)];
            return;
        }

        let filters: ObjcId = msg_send![layer, filters];
        if !filters.is_null() {
            // Remove the increased saturation.
            // The effect of a `CAFilter` or `CIFilter` is determined by its name, and the
            // `description` reflects its name and some parameters. Currently `NSVisualEffectView`
            // uses a `CAFilter` named "colorSaturate". If one day they switch to `CIFilter`, the
            // `description` will still contain "Saturat" ("... inputSaturation = ...").
            let test_string = ns_string("Saturat");
            let count = msg_send![filters, count];
            for i in 0..count {
                let description: ObjcId = msg_send![filters.objectAtIndex(i), description];
                let hit: Bool = msg_send![description, containsString: &*test_string];
                if hit == Bool::new(false) {
                    continue;
                }

                let all_indices = NSRange {
                    location: 0,
                    length: count,
                };
                let indices: ObjcId = msg_send![class!(NSMutableIndexSet), indexSet];
                let _: () = msg_send![indices, addIndexesInRange: all_indices];
                let _: () = msg_send![indices, removeIndex:i];
                let filtered: ObjcId = msg_send![filters, objectsAtIndexes: indices];
                let _: () = msg_send![layer, setFilters: filtered];
                break;
            }
        }

        let sublayers: ObjcId = msg_send![layer, sublayers];
        if !sublayers.is_null() {
            let count = msg_send![sublayers, count];
            for i in 0..count {
                let sublayer = sublayers.objectAtIndex(i);
                remove_layer_background(sublayer);
            }
        }
    }
}

unsafe extern "C" fn add_titlebar_accessory_view_controller(
    this: &Objc2Object,
    _: Sel,
    view_controller: ObjcId,
) {
    unsafe {
        let _: () = msg_send![super(this, class!(NSWindow)), addTitlebarAccessoryViewController: view_controller];

        // Hide the native tab bar and set its height to 0, since we render our own.
        let accessory_view: ObjcId = msg_send![view_controller, view];
        let _: () = msg_send![accessory_view, setHidden: Bool::new(true)];
        let mut frame: Objc2NSRect = msg_send![accessory_view, frame];
        frame.size.height = 0.0;
        let _: () = msg_send![accessory_view, setFrame: frame];
    }
}

unsafe extern "C" fn move_tab_to_new_window(this: &Objc2Object, _: Sel, _: ObjcId) {
    unsafe {
        let _: () = msg_send![super(this, class!(NSWindow)), moveTabToNewWindow:NIL];

        let window_state = get_window_state(this);
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.move_tab_to_new_window_callback.take() {
            drop(lock);
            callback();
            window_state.lock().move_tab_to_new_window_callback = Some(callback);
        }
    }
}

unsafe extern "C" fn merge_all_windows(this: &Objc2Object, _: Sel, _: ObjcId) {
    unsafe {
        let _: () = msg_send![super(this, class!(NSWindow)), mergeAllWindows:NIL];

        let window_state = get_window_state(this);
        let mut lock = window_state.as_ref().lock();
        if let Some(mut callback) = lock.merge_all_windows_callback.take() {
            drop(lock);
            callback();
            window_state.lock().merge_all_windows_callback = Some(callback);
        }
    }
}

unsafe extern "C" fn select_next_tab(this: &Objc2Object, _sel: Sel, _id: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.select_next_tab_callback.take() {
        drop(lock);
        callback();
        window_state.lock().select_next_tab_callback = Some(callback);
    }
}

unsafe extern "C" fn select_previous_tab(this: &Objc2Object, _sel: Sel, _id: ObjcId) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.as_ref().lock();
    if let Some(mut callback) = lock.select_previous_tab_callback.take() {
        drop(lock);
        callback();
        window_state.lock().select_previous_tab_callback = Some(callback);
    }
}

unsafe extern "C" fn toggle_tab_bar(this: &Objc2Object, _sel: Sel, _id: ObjcId) {
    unsafe {
        let _: () = msg_send![super(this, class!(NSWindow)), toggleTabBar:NIL];

        let window_state = get_window_state(this);
        let mut lock = window_state.as_ref().lock();
        lock.move_traffic_light();

        if let Some(mut callback) = lock.toggle_tab_bar_callback.take() {
            drop(lock);
            callback();
            window_state.lock().toggle_tab_bar_callback = Some(callback);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_id_for_screen_returns_none_for_null_screen() {
        assert_eq!(display_id_for_screen(NIL), None);
    }

    #[test]
    fn directional_resize_specs_preserve_each_corner() {
        assert_eq!(
            directional_resize_spec(CursorStyle::ResizeUpLeft),
            Some((
                NSCursorFrameResizePosition::TopLeft,
                NSCursorFrameResizeDirections::Outward
            ))
        );
        assert_eq!(
            directional_resize_spec(CursorStyle::ResizeUpRight),
            Some((
                NSCursorFrameResizePosition::TopRight,
                NSCursorFrameResizeDirections::Outward
            ))
        );
        assert_eq!(
            directional_resize_spec(CursorStyle::ResizeDownLeft),
            Some((
                NSCursorFrameResizePosition::BottomLeft,
                NSCursorFrameResizeDirections::Outward
            ))
        );
        assert_eq!(
            directional_resize_spec(CursorStyle::ResizeDownRight),
            Some((
                NSCursorFrameResizePosition::BottomRight,
                NSCursorFrameResizeDirections::Outward
            ))
        );
        assert_eq!(
            directional_resize_spec(CursorStyle::ResizeUpLeftDownRight),
            Some((
                NSCursorFrameResizePosition::TopLeft,
                NSCursorFrameResizeDirections::All
            ))
        );
        assert_eq!(
            directional_resize_spec(CursorStyle::ResizeUpRightDownLeft),
            Some((
                NSCursorFrameResizePosition::TopRight,
                NSCursorFrameResizeDirections::All
            ))
        );
    }
}
