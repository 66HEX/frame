use crate::{
    MacDispatcher, MacDisplay, MacKeyboardLayout, MacKeyboardMapper, MacWindow,
    events::key_to_native, haptic_feedback::MacHaptics, pasteboard::Pasteboard, renderer,
    set_active_window_cursor_style,
};
use anyhow::{Context as _, anyhow};
use block2::RcBlock;
use core_foundation::{
    base::{CFType, CFTypeRef, OSStatus, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::{CFDictionary, CFDictionaryRef, CFMutableDictionary},
    runloop::CFRunLoopRun,
    string::{CFString, CFStringRef},
};
use dispatch2::DispatchQueue;
use futures::channel::oneshot;
use gpui::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, ForegroundExecutor,
    KeyContext, Keymap, MacActivationPolicy, Menu, MenuItem, OsMenu, OwnedMenu, PathPromptOptions,
    Platform, PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem,
    PlatformWindow, Result, SystemMenuType, Task, ThermalState, WindowAppearance, WindowKind,
    WindowParams, popup::PopupNotSupportedError,
};
use gpui_util::{ResultExt, new_std_command};
use itertools::Itertools;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAppearance, NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSControlStateValueOn, NSCursor, NSDocumentController, NSEventModifierFlags, NSMenu,
    NSMenuDelegate, NSMenuItem, NSModalResponseOK, NSOpenPanel, NSResponder, NSSavePanel,
    NSScroller, NSWorkspace,
};
#[cfg(feature = "screen-capture")]
use objc2_foundation::NSOperatingSystemVersion;
use objc2_foundation::{
    NSArray, NSAutoreleasePool, NSBundle, NSInteger, NSNotification, NSNotificationCenter,
    NSNumber, NSObjectProtocol, NSProcessInfo, NSString, NSUInteger, NSURL, NSUserDefaults,
};
use parking_lot::Mutex;
use ptr::null_mut;
use semver::Version;
use std::{
    cell::Cell,
    ffi::{CStr, OsStr, c_void},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

fn ns_string(value: &str) -> Retained<NSString> {
    NSString::from_str(value)
}

#[derive(Default)]
struct PlatformIvars {
    platform: Cell<*mut c_void>,
}

define_class!(
    // SAFETY: NSApplication has no subclassing requirements, and this class
    // does not implement Drop.
    #[unsafe(super(NSApplication))]
    #[thread_kind = MainThreadOnly]
    #[name = "GPUIApplication"]
    #[ivars = PlatformIvars]
    struct GPUIApplication;

    impl GPUIApplication {
        #[unsafe(method_id(init))]
        unsafe fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(PlatformIvars::default());
            msg_send![super(this), init]
        }
    }

    unsafe impl NSObjectProtocol for GPUIApplication {}
);

define_class!(
    // SAFETY: NSResponder has no subclassing requirements, and this class
    // does not implement Drop.
    #[unsafe(super(NSResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "GPUIApplicationDelegate"]
    #[ivars = PlatformIvars]
    struct GPUIApplicationDelegate;

    impl GPUIApplicationDelegate {
        #[unsafe(method_id(init))]
        unsafe fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(PlatformIvars::default());
            msg_send![super(this), init]
        }

        #[unsafe(method(applicationWillFinishLaunching:))]
        fn will_finish_launching(&self, _: &NSNotification) {
            will_finish_launching(self);
        }
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _: &NSNotification) {
            did_finish_launching(self);
        }
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _: &NSApplication, has_open_windows: bool) -> bool {
            should_handle_reopen(self, has_open_windows)
        }
        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _: &NSNotification) {
            will_terminate(self);
        }
        #[unsafe(method(handleGPUIMenuItem:))]
        fn handle_menu_item(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }
        #[unsafe(method(cut:))]
        fn cut(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }
        #[unsafe(method(copy:))]
        fn copy(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }
        #[unsafe(method(paste:))]
        fn paste(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }
        #[unsafe(method(selectAll:))]
        fn select_all(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }
        #[unsafe(method(undo:))]
        fn undo(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }
        #[unsafe(method(redo:))]
        fn redo(&self, item: &NSMenuItem) {
            handle_menu_item(self, item);
        }
        #[unsafe(method(validateMenuItem:))]
        fn validate_menu_item(&self, item: &NSMenuItem) -> bool {
            validate_menu_item(self, item)
        }
        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _: &NSMenu) {
            menu_will_open(self);
        }
        #[unsafe(method_id(applicationDockMenu:))]
        fn dock_menu(&self, _: &NSApplication) -> Option<Retained<NSMenu>> {
            handle_dock_menu(self)
        }
        #[unsafe(method(application:openURLs:))]
        fn open_urls(&self, _: &NSApplication, urls: &NSArray<NSURL>) {
            open_urls(self, urls);
        }
        #[unsafe(method(onKeyboardLayoutChange:))]
        fn keyboard_layout_change(&self, _: &NSNotification) {
            on_keyboard_layout_change(self);
        }
        #[unsafe(method(onThermalStateChange:))]
        fn thermal_state_change(&self, _: &NSNotification) {
            on_thermal_state_change(self);
        }
        #[unsafe(method(onSystemWake:))]
        fn system_wake(&self, _: &NSNotification) {
            on_system_wake(self);
        }
    }

    unsafe impl NSObjectProtocol for GPUIApplicationDelegate {}
    unsafe impl NSApplicationDelegate for GPUIApplicationDelegate {}
    unsafe impl NSMenuDelegate for GPUIApplicationDelegate {}
);

impl GPUIApplicationDelegate {
    fn new() -> Retained<Self> {
        let this =
            Self::alloc(MainThreadMarker::new().unwrap()).set_ivars(PlatformIvars::default());
        unsafe { msg_send![super(this), init] }
    }
}

pub struct MacPlatform(Mutex<MacPlatformState>, MainThreadMarker);

pub(crate) struct MacPlatformState {
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
    text_system: Arc<dyn PlatformTextSystem>,
    renderer_context: renderer::Context,
    headless: bool,
    activation_policy: Option<MacActivationPolicy>,
    general_pasteboard: Pasteboard,
    find_pasteboard: Pasteboard,
    reopen: Option<Box<dyn FnMut()>>,
    on_keyboard_layout_change: Option<Box<dyn FnMut()>>,
    on_thermal_state_change: Option<Box<dyn FnMut()>>,
    on_system_wake: Option<Box<dyn FnMut()>>,
    system_wake_observer_registered: bool,
    quit: Option<Box<dyn FnMut() -> bool>>,
    menu_command: Option<Box<dyn FnMut(&dyn Action)>>,
    validate_menu_command: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    will_open_menu: Option<Box<dyn FnMut()>>,
    menu_actions: Vec<Box<dyn Action>>,
    open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    finish_launching: Option<Box<dyn FnOnce()>>,
    dock_menu: Option<Retained<NSMenu>>,
    app_delegate: Option<Retained<GPUIApplicationDelegate>>,
    menus: Option<Vec<OwnedMenu>>,
    keyboard_mapper: Rc<MacKeyboardMapper>,
    /// Mirrors `[NSCursor setHiddenUntilMouseMoves:]` state, which AppKit doesn't expose.
    cursor_visible: Arc<AtomicBool>,
    system_notifications: crate::system_notifications::SystemNotificationState,
    /// Haptic feedback engine (macOS only, lazy-initialized on first use).
    haptics: MacHaptics,
}

impl MacPlatform {
    pub fn new(headless: bool) -> Self {
        let marker = MainThreadMarker::new().expect("Mac platform not created on main thread");
        let dispatcher = Arc::new(MacDispatcher::new());

        let text_system = Arc::new(crate::MacTextSystem::new());

        let keyboard_layout = MacKeyboardLayout::new();
        let keyboard_mapper = Rc::new(MacKeyboardMapper::new(keyboard_layout.id()));

        let state = Mutex::new(MacPlatformState {
            headless,
            text_system,
            background_executor: BackgroundExecutor::new(dispatcher.clone()),
            foreground_executor: ForegroundExecutor::new(dispatcher),
            renderer_context: renderer::Context::default(),
            activation_policy: None,
            general_pasteboard: Pasteboard::general(),
            find_pasteboard: Pasteboard::find(),
            reopen: None,
            quit: None,
            menu_command: None,
            validate_menu_command: None,
            will_open_menu: None,
            menu_actions: Default::default(),
            open_urls: None,
            finish_launching: None,
            dock_menu: None,
            app_delegate: None,
            on_keyboard_layout_change: None,
            on_thermal_state_change: None,
            on_system_wake: None,
            system_wake_observer_registered: false,
            menus: None,
            keyboard_mapper,
            cursor_visible: Arc::new(AtomicBool::new(true)),
            system_notifications: crate::system_notifications::SystemNotificationState::new(),
            haptics: MacHaptics::new(headless),
        });
        Self(state, marker)
    }

    fn create_menu_bar(
        &self,
        menus: &Vec<Menu>,
        delegate: &GPUIApplicationDelegate,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> Retained<NSMenu> {
        let delegate = ProtocolObject::from_ref(delegate);
        let application_menu = NSMenu::new(self.1);
        application_menu.setDelegate(Some(&delegate));

        for menu_config in menus {
            let menu = NSMenu::new(self.1);
            let menu_title = ns_string(&menu_config.name);
            menu.setTitle(&menu_title);
            menu.setDelegate(Some(&delegate));

            for item_config in &menu_config.items {
                menu.addItem(&Self::create_menu_item(
                    item_config,
                    &delegate,
                    actions,
                    keymap,
                    self.1,
                ));
            }

            let menu_item = NSMenuItem::new(self.1);
            menu_item.setTitle(&menu_title);
            menu_item.setSubmenu(Some(&menu));
            application_menu.addItem(&menu_item);

            if menu_config.name == "Window" {
                let app: Retained<GPUIApplication> =
                    unsafe { msg_send![GPUIApplication::class(), sharedApplication] };
                app.as_super().setWindowsMenu(Some(&menu));
            }
        }
        application_menu
    }

    fn create_dock_menu(
        &self,
        menu_items: Vec<MenuItem>,
        delegate: &GPUIApplicationDelegate,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> Retained<NSMenu> {
        let delegate = ProtocolObject::from_ref(delegate);
        let dock_menu = NSMenu::new(self.1);
        dock_menu.setDelegate(Some(&delegate));
        for item_config in menu_items {
            dock_menu.addItem(&Self::create_menu_item(
                &item_config,
                &delegate,
                actions,
                keymap,
                self.1,
            ));
        }
        dock_menu
    }

    fn create_menu_item(
        item: &MenuItem,
        delegate: &ProtocolObject<dyn NSMenuDelegate>,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
        marker: MainThreadMarker,
    ) -> Retained<NSMenuItem> {
        static DEFAULT_CONTEXT: OnceLock<Vec<KeyContext>> = OnceLock::new();

        match item {
            MenuItem::Separator => NSMenuItem::separatorItem(marker),
            MenuItem::Action {
                name,
                action,
                os_action,
                checked,
                disabled,
            } => {
                // Note that this is intentionally using earlier bindings, whereas typically
                // later ones take display precedence. See the discussion on
                // https://github.com/zed-industries/zed/issues/23621
                let keystrokes = keymap
                    .bindings_for_action(action.as_ref())
                    .find_or_first(|binding| {
                        binding.predicate().is_none_or(|predicate| {
                            predicate.eval(DEFAULT_CONTEXT.get_or_init(|| {
                                let mut workspace_context = KeyContext::new_with_defaults();
                                workspace_context.add("Workspace");
                                let mut pane_context = KeyContext::new_with_defaults();
                                pane_context.add("Pane");
                                let mut editor_context = KeyContext::new_with_defaults();
                                editor_context.add("Editor");

                                pane_context.extend(&editor_context);
                                workspace_context.extend(&pane_context);
                                vec![workspace_context]
                            }))
                        })
                    })
                    .map(|binding| binding.keystrokes());

                let selector = match os_action {
                    Some(gpui::OsAction::Cut) => Some(objc2::sel!(cut:)),
                    Some(gpui::OsAction::Copy) => Some(objc2::sel!(copy:)),
                    Some(gpui::OsAction::Paste) => Some(objc2::sel!(paste:)),
                    Some(gpui::OsAction::SelectAll) => Some(objc2::sel!(selectAll:)),
                    // "undo:" and "redo:" are always disabled in our case, as
                    // we don't have a NSTextView/NSTextField to enable them on.
                    Some(gpui::OsAction::Undo) => Some(objc2::sel!(handleGPUIMenuItem:)),
                    Some(gpui::OsAction::Redo) => Some(objc2::sel!(handleGPUIMenuItem:)),
                    None => Some(objc2::sel!(handleGPUIMenuItem:)),
                };

                let item;
                if let Some(keystrokes) = keystrokes {
                    if keystrokes.len() == 1 {
                        let keystroke = &keystrokes[0];
                        let mut mask = NSEventModifierFlags::empty();
                        for (modifier, flag) in &[
                            (
                                keystroke.modifiers().platform,
                                NSEventModifierFlags::Command,
                            ),
                            (keystroke.modifiers().control, NSEventModifierFlags::Control),
                            (keystroke.modifiers().alt, NSEventModifierFlags::Option),
                            (keystroke.modifiers().shift, NSEventModifierFlags::Shift),
                        ] {
                            if *modifier {
                                mask |= *flag;
                            }
                        }

                        item = unsafe {
                            NSMenuItem::initWithTitle_action_keyEquivalent(
                                NSMenuItem::alloc(marker),
                                &ns_string(name),
                                selector,
                                &ns_string(key_to_native(keystroke.key()).as_ref()),
                            )
                        };
                        if Self::os_version() >= Version::new(12, 0, 0) {
                            let _: () = unsafe {
                                msg_send![&*item, setAllowsAutomaticKeyEquivalentLocalization: false]
                            };
                        }
                        item.setKeyEquivalentModifierMask(mask);
                    } else {
                        item = unsafe {
                            NSMenuItem::initWithTitle_action_keyEquivalent(
                                NSMenuItem::alloc(marker),
                                &ns_string(name),
                                selector,
                                &ns_string(""),
                            )
                        };
                    }
                } else {
                    item = unsafe {
                        NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(marker),
                            &ns_string(name),
                            selector,
                            &ns_string(""),
                        )
                    };
                }

                if *checked {
                    item.setState(NSControlStateValueOn);
                }
                item.setEnabled(!*disabled);

                let tag = actions.len() as NSInteger;
                item.setTag(tag);
                actions.push(action.boxed_clone());
                item
            }
            MenuItem::Submenu(Menu {
                name,
                items,
                disabled,
            }) => {
                let item = NSMenuItem::new(marker);
                let submenu = NSMenu::new(marker);
                submenu.setDelegate(Some(delegate));
                for item in items {
                    submenu.addItem(&Self::create_menu_item(
                        item, delegate, actions, keymap, marker,
                    ));
                }
                item.setSubmenu(Some(&submenu));
                item.setEnabled(!*disabled);
                item.setTitle(&ns_string(name));
                item
            }
            MenuItem::SystemMenu(OsMenu { name, menu_type }) => {
                let item = NSMenuItem::new(marker);
                let submenu = NSMenu::new(marker);
                submenu.setDelegate(Some(delegate));
                item.setSubmenu(Some(&submenu));
                item.setTitle(&ns_string(name));

                match menu_type {
                    SystemMenuType::Services => {
                        let app: Retained<GPUIApplication> =
                            unsafe { msg_send![GPUIApplication::class(), sharedApplication] };
                        app.as_super().setServicesMenu(Some(&submenu));
                    }
                }

                item
            }
        }
    }

    fn os_version() -> Version {
        let version = NSProcessInfo::processInfo().operatingSystemVersion();
        Version::new(
            version.majorVersion as u64,
            version.minorVersion as u64,
            version.patchVersion as u64,
        )
    }
}

impl Platform for MacPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.0.lock().background_executor.clone()
    }

    fn foreground_executor(&self) -> gpui::ForegroundExecutor {
        self.0.lock().foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.0.lock().text_system.clone()
    }

    fn set_mac_activation_policy(&self, policy: MacActivationPolicy) {
        self.0.lock().activation_policy = Some(policy);
    }

    fn run(&self, on_finish_launching: Box<dyn FnOnce()>) {
        let mut state = self.0.lock();
        if state.headless {
            drop(state);
            on_finish_launching();
            unsafe { CFRunLoopRun() };
        } else {
            state.finish_launching = Some(on_finish_launching);
            drop(state);
        }

        unsafe {
            let app: Retained<GPUIApplication> =
                msg_send![GPUIApplication::class(), sharedApplication];
            let app_delegate = GPUIApplicationDelegate::new();
            let app_delegate_protocol = ProtocolObject::from_ref(&*app_delegate);
            app.as_super().setDelegate(Some(&app_delegate_protocol));

            let self_ptr = self as *const Self as *mut c_void;
            app.ivars().platform.set(self_ptr);
            app_delegate.ivars().platform.set(self_ptr);
            self.0.lock().app_delegate = Some(app_delegate);

            let pool = NSAutoreleasePool::new();
            app.as_super().run();
            drop(pool);

            app.ivars().platform.set(null_mut());
            self.0.lock().app_delegate = None;
        }
    }

    fn quit(&self) {
        // Quitting the app causes us to close windows, which invokes `Window::on_close` callbacks
        // synchronously before this method terminates. If we call `Platform::quit` while holding a
        // borrow of the app state (which most of the time we will do), we will end up
        // double-borrowing the app state in the `on_close` callbacks for our open windows. To solve
        // this, we make quitting the application asynchronous so that we aren't holding borrows to
        // the app state on the stack when we actually terminate the app.

        unsafe {
            DispatchQueue::main().exec_async_f(ptr::null_mut(), quit);
        }

        extern "C" fn quit(_: *mut c_void) {
            NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).terminate(None);
        }
    }

    fn restart(&self, binary_path: Option<PathBuf>, arguments: Vec<std::ffi::OsString>) {
        use std::os::unix::process::CommandExt as _;

        let app_pid = std::process::id().to_string();
        let app_path = binary_path
            .or_else(|| {
                self.app_path()
                    .ok()
                    // When the app is not bundled, `app_path` returns the
                    // directory containing the executable. Disregard this
                    // and get the path to the executable itself.
                    .and_then(|path| (path.extension()?.to_str()? == "app").then_some(path))
            })
            .unwrap_or_else(|| std::env::current_exe().unwrap());

        // Wait until this process has exited and then re-open this path.
        let script = r#"
            while kill -0 $0 2> /dev/null; do
                sleep 0.1
            done
            app_path="$1"
            shift
            if (($# > 0)); then
                open "$app_path" --args "$@"
            else
                open "$app_path"
            fi
        "#;

        #[allow(
            clippy::disallowed_methods,
            reason = "We are restarting ourselves, using std command thus is fine"
        )]
        let restart_process = new_std_command("/bin/bash")
            .arg("-c")
            .arg(script)
            .arg(app_pid)
            .arg(app_path)
            .args(arguments)
            .process_group(0)
            .spawn();

        match restart_process {
            Ok(_) => self.quit(),
            Err(e) => log::error!("failed to spawn restart script: {:?}", e),
        }
    }

    fn activate(&self, ignoring_other_apps: bool) {
        unsafe {
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            if ignoring_other_apps {
                let _: () = msg_send![&*app, activateIgnoringOtherApps: true];
            } else {
                app.activate();
            }
        }
    }

    fn hide(&self) {
        NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).hide(None);
    }

    fn hide_other_apps(&self) {
        NSApplication::sharedApplication(MainThreadMarker::new().unwrap())
            .hideOtherApplications(None);
    }

    fn unhide_other_apps(&self) {
        NSApplication::sharedApplication(MainThreadMarker::new().unwrap())
            .unhideAllApplications(None);
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(Rc::new(MacDisplay::primary()))
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        MacDisplay::all()
            .map(|screen| Rc::new(screen) as Rc<_>)
            .collect()
    }

    #[cfg(feature = "screen-capture")]
    fn is_screen_capture_supported(&self) -> bool {
        NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(NSOperatingSystemVersion {
            majorVersion: 12,
            minorVersion: 3,
            patchVersion: 0,
        })
    }

    #[cfg(feature = "screen-capture")]
    fn screen_capture_sources(
        &self,
    ) -> oneshot::Receiver<Result<Vec<Rc<dyn gpui::ScreenCaptureSource>>>> {
        crate::screen_capture::get_sources()
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        MacWindow::active_window()
    }

    // Returns the windows ordered front-to-back, meaning that the active
    // window is the first one in the returned vec.
    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        Some(MacWindow::ordered_windows())
    }

    #[cfg(feature = "wgpu")]
    fn set_gpu_requirements(&self, requirements: Box<dyn std::any::Any>) {
        if let Ok(reqs) = requirements.downcast::<gpui_wgpu::WgpuDeviceRequirements>() {
            self.0.lock().renderer_context.set_requirements(*reqs);
        } else {
            log::warn!("set_gpu_requirements: unexpected type, expected WgpuDeviceRequirements");
        }
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> Result<Box<dyn PlatformWindow>> {
        // Native popups are not implemented on macOS yet. Rejecting lets callers fall back to
        // gpui's in-window popovers.
        if let WindowKind::AnchoredPopup(_) = options.kind {
            return Err(PopupNotSupportedError.into());
        }

        let (cursor_visible, foreground_executor, background_executor, renderer_context) = {
            let guard = self.0.lock();
            (
                guard.cursor_visible.clone(),
                guard.foreground_executor.clone(),
                guard.background_executor.clone(),
                guard.renderer_context.clone(),
            )
        };

        let renderer = renderer::new_renderer(
            renderer_context,
            options.bounds.size.map(|pixels| pixels.as_f32()),
            false,
        )?;
        Ok(Box::new(MacWindow::open(
            handle,
            options,
            cursor_visible,
            foreground_executor,
            background_executor,
            renderer,
            self.1,
        )))
    }

    fn window_appearance(&self) -> WindowAppearance {
        let app = NSApplication::sharedApplication(self.1);
        let appearance = app.effectiveAppearance();
        unsafe {
            crate::window_appearance::window_appearance_from_native(
                Retained::<NSAppearance>::as_ptr(&appearance) as *mut AnyObject,
            )
        }
    }

    fn set_window_appearance(&self, appearance: Option<WindowAppearance>) {
        let app: Retained<GPUIApplication> =
            unsafe { msg_send![GPUIApplication::class(), sharedApplication] };
        let ns_appearance = appearance.and_then(|appearance| unsafe {
            let name = match appearance {
                WindowAppearance::Light => crate::window_appearance::NSAppearanceNameAqua,
                WindowAppearance::Dark => crate::window_appearance::NSAppearanceNameDarkAqua,
                WindowAppearance::VibrantLight => {
                    crate::window_appearance::NSAppearanceNameVibrantLight
                }
                WindowAppearance::VibrantDark => {
                    crate::window_appearance::NSAppearanceNameVibrantDark
                }
            };
            NSAppearance::appearanceNamed(name)
        });
        app.as_super().setAppearance(ns_appearance.as_deref());
    }

    fn open_url(&self, url: &str) {
        let Some(url) = NSURL::URLWithString(&ns_string(url)) else {
            log::error!("Failed to create NSURL from string: {}", url);
            return;
        };
        NSWorkspace::sharedWorkspace().openURL(&url);
    }

    fn register_url_scheme(&self, scheme: &str) -> Task<anyhow::Result<()>> {
        // API only available post Monterey
        // https://developer.apple.com/documentation/appkit/nsworkspace/3753004-setdefaultapplicationaturl
        let (done_tx, done_rx) = oneshot::channel();
        if Self::os_version() < Version::new(12, 0, 0) {
            return Task::ready(Err(anyhow!(
                "macOS 12.0 or later is required to register URL schemes"
            )));
        }

        let bundle_id = NSBundle::mainBundle().bundleIdentifier();
        let Some(bundle_id) = bundle_id else {
            return Task::ready(Err(anyhow!("Can only register URL scheme in bundled apps")));
        };

        {
            let workspace = NSWorkspace::sharedWorkspace();
            let scheme = ns_string(scheme);
            let Some(app) = workspace.URLForApplicationWithBundleIdentifier(&bundle_id) else {
                return Task::ready(Err(anyhow!(
                    "Cannot register URL scheme until app is installed"
                )));
            };
            let done_tx = Cell::new(Some(done_tx));
            let block = RcBlock::new(move |error: *mut objc2_foundation::NSError| {
                let result = if let Some(error) = unsafe { error.as_ref() } {
                    Err(anyhow!(
                        "Failed to register: {}",
                        error.localizedDescription()
                    ))
                } else {
                    Ok(())
                };

                if let Some(done_tx) = done_tx.take() {
                    let _ = done_tx.send(result);
                }
            });
            workspace.setDefaultApplicationAtURL_toOpenURLsWithScheme_completionHandler(
                &app,
                &scheme,
                Some(&block),
            );
        }

        self.background_executor()
            .spawn(async { done_rx.await.map_err(|e| anyhow!(e))? })
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.0.lock().open_urls = Some(callback);
    }

    fn prompt_for_paths(
        &self,
        options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (done_tx, done_rx) = oneshot::channel();
        self.foreground_executor()
            .spawn(async move {
                {
                    let panel = NSOpenPanel::openPanel(MainThreadMarker::new().unwrap());
                    panel.setCanChooseDirectories(options.directories);
                    panel.setCanChooseFiles(options.files);
                    panel.setAllowsMultipleSelection(options.multiple);

                    panel.setCanCreateDirectories(true);
                    panel.setResolvesAliases(false);
                    let panel_for_callback = panel.clone();
                    let done_tx = Cell::new(Some(done_tx));
                    let block = RcBlock::new(move |response: objc2_app_kit::NSModalResponse| {
                        let result = if response == NSModalResponseOK {
                            let mut result = Vec::new();
                            let urls = panel_for_callback.URLs();
                            for i in 0..urls.count() {
                                let url = urls.objectAtIndex(i as NSUInteger);
                                if url.isFileURL()
                                    && let Ok(path) = ns_url_to_path(&url)
                                {
                                    result.push(path)
                                }
                            }
                            Some(result)
                        } else {
                            None
                        };

                        if let Some(done_tx) = done_tx.take() {
                            let _ = done_tx.send(Ok(result));
                        }
                    });
                    if let Some(prompt) = options.prompt {
                        panel.setPrompt(Some(&ns_string(&prompt)));
                    }

                    panel.beginWithCompletionHandler(&block);
                }
            })
            .detach();
        done_rx
    }

    fn prompt_for_new_path(
        &self,
        directory: &Path,
        suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let directory = directory.to_owned();
        let suggested_name = suggested_name.map(|s| s.to_owned());
        let (done_tx, done_rx) = oneshot::channel();
        self.foreground_executor()
            .spawn(async move {
                {
                    let panel = NSSavePanel::savePanel(MainThreadMarker::new().unwrap());
                    let path = ns_string(directory.to_string_lossy().as_ref());
                    let url = NSURL::fileURLWithPath_isDirectory(&path, true);
                    panel.setDirectoryURL(Some(&url));
                    let panel_for_callback = panel.clone();

                    if let Some(suggested_name) = suggested_name {
                        let name_string = ns_string(&suggested_name);
                        panel.setNameFieldStringValue(&name_string);
                    }

                    let done_tx = Cell::new(Some(done_tx));
                    let block = RcBlock::new(move |response: objc2_app_kit::NSModalResponse| {
                        let mut result = None;
                        if response == NSModalResponseOK {
                            if let Some(url) = panel_for_callback.URL()
                                && url.isFileURL()
                            {
                                result = ns_url_to_path(&url).ok().map(|mut result| {
                                    let Some(filename) = result.file_name() else {
                                        return result;
                                    };
                                    let chunks = filename
                                        .as_bytes()
                                        .split(|&b| b == b'.')
                                        .collect::<Vec<_>>();

                                    // https://github.com/zed-industries/zed/issues/16969
                                    // Workaround a bug in macOS Sequoia that adds an extra file-extension
                                    // sometimes. e.g. `a.sql` becomes `a.sql.s` or `a.txtx` becomes `a.txtx.txt`
                                    //
                                    // This is conditional on OS version because I'd like to get rid of it, so that
                                    // you can manually create a file called `a.sql.s`. That said it seems better
                                    // to break that use-case than breaking `a.sql`.
                                    if chunks.len() == 3
                                        && chunks[1].starts_with(chunks[2])
                                        && Self::os_version() >= Version::new(15, 0, 0)
                                    {
                                        let new_filename = OsStr::from_bytes(
                                            &filename.as_bytes()
                                                [..chunks[0].len() + 1 + chunks[1].len()],
                                        )
                                        .to_owned();
                                        result.set_file_name(&new_filename);
                                    }
                                    result
                                })
                            }
                        }

                        if let Some(done_tx) = done_tx.take() {
                            let _ = done_tx.send(Ok(result));
                        }
                    });
                    panel.beginWithCompletionHandler(&block);
                }
            })
            .detach();

        done_rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        true
    }

    fn reveal_path(&self, path: &Path) {
        let path = path.to_path_buf();
        self.0
            .lock()
            .background_executor
            .spawn(async move {
                let full_path = ns_string(path.to_str().unwrap_or(""));
                let root_full_path = ns_string("");
                NSWorkspace::sharedWorkspace()
                    .selectFile_inFileViewerRootedAtPath(Some(&full_path), &root_full_path);
            })
            .detach();
    }

    fn open_with_system(&self, path: &Path) {
        let path = path.to_owned();
        self.0
            .lock()
            .background_executor
            .spawn(async move {
                #[allow(
                    clippy::disallowed_methods,
                    reason = "running on a background thread, so blocking is fine"
                )]
                new_std_command("open")
                    .arg("--")
                    .arg(path)
                    .status()
                    .context("invoking open command")
                    .log_err();
            })
            .detach();
    }

    fn on_quit(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.lock().quit = Some(callback);
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().reopen = Some(callback);
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().on_keyboard_layout_change = Some(callback);
    }

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        self.0.lock().menu_command = Some(callback);
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().will_open_menu = Some(callback);
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        self.0.lock().validate_menu_command = Some(callback);
    }

    fn on_thermal_state_change(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().on_thermal_state_change = Some(callback);
    }

    fn on_system_wake(&self, callback: Box<dyn FnMut()>) {
        let mut state = self.0.lock();
        state.on_system_wake = Some(callback);
        if state.system_wake_observer_registered {
            return;
        }
        drop(state);

        let delegate = self.0.lock().app_delegate.clone();
        if let Some(delegate) = delegate {
            register_system_wake_observer(&delegate);
            self.0.lock().system_wake_observer_registered = true;
        }
    }

    fn thermal_state(&self) -> ThermalState {
        match NSProcessInfo::processInfo().thermalState().0 {
            0 => ThermalState::Nominal,
            1 => ThermalState::Fair,
            2 => ThermalState::Serious,
            3 => ThermalState::Critical,
            _ => ThermalState::Nominal,
        }
    }

    fn show_system_notification(&self, notification: gpui::SystemNotification) {
        let mut state = self.0.lock();
        let executor = state.foreground_executor.clone();
        state.system_notifications.show(&executor, notification);
    }

    fn dismiss_system_notification(&self, tag: &str) {
        let mut state = self.0.lock();
        let executor = state.foreground_executor.clone();
        state.system_notifications.dismiss(&executor, tag);
    }

    fn on_system_notification_response(
        &self,
        callback: Box<dyn FnMut(gpui::SystemNotificationResponse)>,
    ) {
        let mut state = self.0.lock();
        let executor = state.foreground_executor.clone();
        state.system_notifications.on_response(&executor, callback);
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(MacKeyboardLayout::new())
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        self.0.lock().keyboard_mapper.clone()
    }

    fn app_path(&self) -> Result<PathBuf> {
        let bundle = NSBundle::mainBundle();
        Ok(PathBuf::from(bundle.bundlePath().to_string()))
    }

    fn set_menus(&self, menus: Vec<Menu>, keymap: &Keymap) {
        let app: Retained<GPUIApplication> =
            unsafe { msg_send![GPUIApplication::class(), sharedApplication] };
        let mut state = self.0.lock();
        let delegate = state
            .app_delegate
            .clone()
            .expect("app delegate not initialized");
        let actions = &mut state.menu_actions;
        let menu = self.create_menu_bar(&menus, &delegate, actions, keymap);
        drop(state);
        app.as_super().setMainMenu(Some(&menu));
        self.0.lock().menus = Some(menus.into_iter().map(|menu| menu.owned()).collect());
    }

    fn get_menus(&self) -> Option<Vec<OwnedMenu>> {
        self.0.lock().menus.clone()
    }

    fn set_dock_menu(&self, menu: Vec<MenuItem>, keymap: &Keymap) {
        let mut state = self.0.lock();
        let delegate = state
            .app_delegate
            .clone()
            .expect("app delegate not initialized");
        let actions = &mut state.menu_actions;
        let new = self.create_dock_menu(menu, &delegate, actions, keymap);
        state.dock_menu = Some(new);
    }

    fn add_recent_document(&self, path: &Path) {
        if let Some(path_str) = path.to_str() {
            let document_controller = NSDocumentController::sharedDocumentController(self.1);
            let url = NSURL::fileURLWithPath(&ns_string(path_str));
            document_controller.noteNewRecentDocumentURL(&url);
        }
    }

    fn path_for_auxiliary_executable(&self, name: &str) -> Result<PathBuf> {
        let bundle = NSBundle::mainBundle();
        let name = ns_string(name);
        let url = bundle.URLForAuxiliaryExecutable(&name);
        anyhow::ensure!(url.is_some(), "resource not found");
        ns_url_to_path(url.as_ref().unwrap())
    }

    /// Match cursor style to one of the styles available
    /// in macOS's [NSCursor](https://developer.apple.com/documentation/appkit/nscursor).
    fn set_cursor_style(&self, style: CursorStyle) {
        unsafe { set_active_window_cursor_style(style) };
    }

    fn hide_cursor_until_mouse_moves(&self) {
        let cursor_visible = self.0.lock().cursor_visible.clone();
        if !cursor_visible.swap(false, Ordering::Relaxed) {
            return;
        }
        NSCursor::setHiddenUntilMouseMoves(true);
    }

    fn is_cursor_visible(&self) -> bool {
        self.0.lock().cursor_visible.load(Ordering::Relaxed)
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        NSScroller::preferredScrollerStyle(self.1).0 == 1
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        let state = self.0.lock();
        state.general_pasteboard.read()
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        let state = self.0.lock();
        state.general_pasteboard.write(item);
    }

    fn read_from_find_pasteboard(&self) -> Option<ClipboardItem> {
        let state = self.0.lock();
        state.find_pasteboard.read()
    }

    fn write_to_find_pasteboard(&self, item: ClipboardItem) {
        let state = self.0.lock();
        state.find_pasteboard.write(item);
    }

    fn write_credentials(&self, url: &str, username: &str, password: &[u8]) -> Task<Result<()>> {
        let url = url.to_string();
        let username = username.to_string();
        let password = password.to_vec();
        self.background_executor().spawn(async move {
            unsafe {
                use security::*;

                let url = CFString::from(url.as_str());
                let username = CFString::from(username.as_str());
                let password = CFData::from_buffer(&password);

                // First, check if there are already credentials for the given server. If so, then
                // update the username and password.
                let mut verb = "updating";
                let mut query_attrs = CFMutableDictionary::with_capacity(2);
                query_attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
                query_attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());

                let mut attrs = CFMutableDictionary::with_capacity(4);
                attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
                attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());
                attrs.set(kSecAttrAccount as *const _, username.as_CFTypeRef());
                attrs.set(kSecValueData as *const _, password.as_CFTypeRef());

                let mut status = SecItemUpdate(
                    query_attrs.as_concrete_TypeRef(),
                    attrs.as_concrete_TypeRef(),
                );

                // If there were no existing credentials for the given server, then create them.
                if status == errSecItemNotFound {
                    verb = "creating";
                    status = SecItemAdd(attrs.as_concrete_TypeRef(), ptr::null_mut());
                }
                anyhow::ensure!(status == errSecSuccess, "{verb} password failed: {status}");
            }
            Ok(())
        })
    }

    fn read_credentials(&self, url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        let url = url.to_string();
        self.background_executor().spawn(async move {
            let url = CFString::from(url.as_str());
            let cf_true = CFBoolean::true_value().as_CFTypeRef();

            unsafe {
                use security::*;

                // Find any credentials for the given server URL.
                let mut attrs = CFMutableDictionary::with_capacity(5);
                attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
                attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());
                attrs.set(kSecReturnAttributes as *const _, cf_true);
                attrs.set(kSecReturnData as *const _, cf_true);

                let mut result = CFTypeRef::from(ptr::null());
                let status = SecItemCopyMatching(attrs.as_concrete_TypeRef(), &mut result);
                match status {
                    security::errSecSuccess => {}
                    security::errSecItemNotFound | security::errSecUserCanceled => return Ok(None),
                    _ => anyhow::bail!("reading password failed: {status}"),
                }

                let result = CFType::wrap_under_create_rule(result)
                    .downcast::<CFDictionary>()
                    .context("keychain item was not a dictionary")?;
                let username = result
                    .find(kSecAttrAccount as *const _)
                    .context("account was missing from keychain item")?;
                let username = CFType::wrap_under_get_rule(*username)
                    .downcast::<CFString>()
                    .context("account was not a string")?;
                let password = result
                    .find(kSecValueData as *const _)
                    .context("password was missing from keychain item")?;
                let password = CFType::wrap_under_get_rule(*password)
                    .downcast::<CFData>()
                    .context("password was not a string")?;

                Ok(Some((username.to_string(), password.bytes().to_vec())))
            }
        })
    }

    fn delete_credentials(&self, url: &str) -> Task<Result<()>> {
        let url = url.to_string();

        self.background_executor().spawn(async move {
            unsafe {
                use security::*;

                let url = CFString::from(url.as_str());
                let mut query_attrs = CFMutableDictionary::with_capacity(2);
                query_attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
                query_attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());

                let status = SecItemDelete(query_attrs.as_concrete_TypeRef());
                anyhow::ensure!(status == errSecSuccess, "delete password failed: {status}");
            }
            Ok(())
        })
    }

    fn supports_haptic_feedback(&self) -> bool {
        self.0.lock().haptics.supported()
    }

    fn play_haptic_feedback(&self, style: gpui::HapticFeedbackStyle) {
        self.0.lock().haptics.play(style)
    }
}

fn get_mac_platform(object: &GPUIApplicationDelegate) -> &MacPlatform {
    let platform_ptr = object.ivars().platform.get();
    assert!(!platform_ptr.is_null());
    unsafe { &*(platform_ptr as *const MacPlatform) }
}

fn will_finish_launching(this: &GPUIApplicationDelegate) {
    {
        let user_defaults = NSUserDefaults::standardUserDefaults();

        // The autofill heuristic controller causes slowdown and high CPU usage.
        // We don't know exactly why. This disables the full heuristic controller.
        //
        // Adapted from: https://github.com/ghostty-org/ghostty/pull/8625
        let name = ns_string("NSAutoFillHeuristicControllerEnabled");
        let existing_value = user_defaults.objectForKey(&name);
        if existing_value.is_none() {
            let false_value = NSNumber::numberWithBool(false);
            unsafe { user_defaults.setObject_forKey(Some(&false_value), &name) };
        }

        let platform = get_mac_platform(this);
        let state = platform.0.lock();
        if let Some(policy) = state.activation_policy {
            let app: Retained<GPUIApplication> =
                unsafe { msg_send![GPUIApplication::class(), sharedApplication] };
            let ns_policy = match policy {
                MacActivationPolicy::Regular => NSApplicationActivationPolicy::Regular,
                MacActivationPolicy::Accessory => NSApplicationActivationPolicy::Accessory,
                MacActivationPolicy::Prohibited => NSApplicationActivationPolicy::Prohibited,
            };
            app.as_super().setActivationPolicy(ns_policy);
        }
    }
}

fn did_finish_launching(this: &GPUIApplicationDelegate) {
    let platform = get_mac_platform(this);
    let state = platform.0.lock();
    if state.activation_policy.is_none() {
        let app: Retained<GPUIApplication> =
            unsafe { msg_send![GPUIApplication::class(), sharedApplication] };
        app.as_super()
            .setActivationPolicy(NSApplicationActivationPolicy::Regular);
    }
    drop(state);

    let notification_center = NSNotificationCenter::defaultCenter();
    let name = ns_string("NSTextInputContextKeyboardSelectionDidChangeNotification");
    unsafe {
        notification_center.addObserver_selector_name_object(
            as_any_object(this),
            objc2::sel!(onKeyboardLayoutChange:),
            Some(&name),
            None,
        );
    }

    let thermal_name = ns_string("NSProcessInfoThermalStateDidChangeNotification");
    let process_info = NSProcessInfo::processInfo();
    unsafe {
        notification_center.addObserver_selector_name_object(
            as_any_object(this),
            objc2::sel!(onThermalStateChange:),
            Some(&thermal_name),
            Some(&process_info),
        );
    }
    let platform = get_mac_platform(this);
    let callback = {
        let mut state = platform.0.lock();
        if state.on_system_wake.is_some() && !state.system_wake_observer_registered {
            register_system_wake_observer(this);
            state.system_wake_observer_registered = true;
        }
        state.finish_launching.take()
    };
    if let Some(callback) = callback {
        callback();
    }
}

fn as_any_object<T>(object: &T) -> &AnyObject {
    unsafe { &*(object as *const T as *const AnyObject) }
}

fn register_system_wake_observer(observer: &GPUIApplicationDelegate) {
    let workspace = NSWorkspace::sharedWorkspace();
    let workspace_center = workspace.notificationCenter();
    let wake_name = ns_string("NSWorkspaceDidWakeNotification");
    unsafe {
        workspace_center.addObserver_selector_name_object(
            as_any_object(observer),
            objc2::sel!(onSystemWake:),
            Some(&wake_name),
            None,
        );
    }
}

fn should_handle_reopen(this: &GPUIApplicationDelegate, has_open_windows: bool) -> bool {
    if !has_open_windows {
        let platform = get_mac_platform(this);
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.reopen.take() {
            drop(lock);
            callback();
            platform.0.lock().reopen.get_or_insert(callback);
        }
    }
    true
}

fn will_terminate(this: &GPUIApplicationDelegate) {
    let platform = get_mac_platform(this);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.quit.take() {
        drop(lock);
        callback();
        platform.0.lock().quit.get_or_insert(callback);
    }
}

fn on_keyboard_layout_change(this: &GPUIApplicationDelegate) {
    let platform = get_mac_platform(this);
    let mut lock = platform.0.lock();
    let keyboard_layout = MacKeyboardLayout::new();
    lock.keyboard_mapper = Rc::new(MacKeyboardMapper::new(keyboard_layout.id()));
    if let Some(mut callback) = lock.on_keyboard_layout_change.take() {
        drop(lock);
        callback();
        platform
            .0
            .lock()
            .on_keyboard_layout_change
            .get_or_insert(callback);
    }
}

fn on_thermal_state_change(this: &GPUIApplicationDelegate) {
    // Defer to the next run loop iteration to avoid re-entrant borrows of the App RefCell,
    // as NSNotificationCenter delivers this notification synchronously and it may fire while
    // the App is already borrowed (same pattern as quit() above).
    let platform = get_mac_platform(this);
    let platform_ptr = platform as *const MacPlatform as *mut c_void;
    unsafe {
        DispatchQueue::main().exec_async_f(platform_ptr, on_thermal_state_change);
    }

    extern "C" fn on_thermal_state_change(context: *mut c_void) {
        let platform = unsafe { &*(context as *const MacPlatform) };
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.on_thermal_state_change.take() {
            drop(lock);
            callback();
            platform
                .0
                .lock()
                .on_thermal_state_change
                .get_or_insert(callback);
        }
    }
}

fn on_system_wake(this: &GPUIApplicationDelegate) {
    // SAFETY: this is the registered app delegate carrying MAC_PLATFORM_IVAR.
    let platform = get_mac_platform(this);
    let platform_ptr = platform as *const MacPlatform as *mut c_void;
    // SAFETY: platform lives for the process lifetime while callbacks are registered.
    unsafe {
        DispatchQueue::main().exec_async_f(platform_ptr, on_system_wake);
    }

    extern "C" fn on_system_wake(context: *mut c_void) {
        // SAFETY: context is the MacPlatform pointer queued above.
        let platform = unsafe { &*(context as *const MacPlatform) };
        let mut lock = platform.0.lock();
        if let Some(mut callback) = lock.on_system_wake.take() {
            drop(lock);
            callback();
            platform.0.lock().on_system_wake.get_or_insert(callback);
        }
    }
}

fn open_urls(this: &GPUIApplicationDelegate, native_urls: &NSArray<NSURL>) {
    let urls = (0..native_urls.count())
        .filter_map(|i| {
            native_urls
                .objectAtIndex(i)
                .absoluteString()
                .map(|s| s.to_string())
        })
        .collect::<Vec<_>>();
    let platform = get_mac_platform(this);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.open_urls.take() {
        drop(lock);
        callback(urls);
        platform.0.lock().open_urls.get_or_insert(callback);
    }
}

fn handle_menu_item(this: &GPUIApplicationDelegate, item: &NSMenuItem) {
    let platform = get_mac_platform(this);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.menu_command.take() {
        let index = item.tag() as usize;
        if let Some(action) = lock.menu_actions.get(index) {
            let action = action.boxed_clone();
            drop(lock);
            callback(&*action);
        }
        platform.0.lock().menu_command.get_or_insert(callback);
    }
}

fn validate_menu_item(this: &GPUIApplicationDelegate, item: &NSMenuItem) -> bool {
    let mut result = false;
    let platform = get_mac_platform(this);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.validate_menu_command.take() {
        let index = item.tag() as usize;
        if let Some(action) = lock.menu_actions.get(index) {
            let action = action.boxed_clone();
            drop(lock);
            result = callback(action.as_ref());
        }
        platform
            .0
            .lock()
            .validate_menu_command
            .get_or_insert(callback);
    }
    result
}

fn menu_will_open(this: &GPUIApplicationDelegate) {
    let platform = get_mac_platform(this);
    let mut lock = platform.0.lock();
    if let Some(mut callback) = lock.will_open_menu.take() {
        drop(lock);
        callback();
        platform.0.lock().will_open_menu.get_or_insert(callback);
    }
}

fn handle_dock_menu(this: &GPUIApplicationDelegate) -> Option<Retained<NSMenu>> {
    let platform = get_mac_platform(this);
    let state = platform.0.lock();
    state.dock_menu.clone()
}

fn ns_url_to_path(url: &NSURL) -> Result<PathBuf> {
    let path = url.fileSystemRepresentation();
    let description = url
        .absoluteString()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let _ = description;
    Ok(PathBuf::from(OsStr::from_bytes(unsafe {
        CStr::from_ptr(path.as_ptr()).to_bytes()
    })))
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    pub(super) fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut AnyObject;
    pub(super) fn TISCopyCurrentKeyboardInputSource() -> *mut AnyObject;
    pub(super) fn TISGetInputSourceProperty(
        inputSource: *mut AnyObject,
        propertyKey: *const c_void,
    ) -> *mut AnyObject;

    pub(super) fn UCKeyTranslate(
        keyLayoutPtr: *const ::std::os::raw::c_void,
        virtualKeyCode: u16,
        keyAction: u16,
        modifierKeyState: u32,
        keyboardType: u32,
        keyTranslateOptions: u32,
        deadKeyState: *mut u32,
        maxStringLength: usize,
        actualStringLength: *mut usize,
        unicodeString: *mut u16,
    ) -> u32;
    pub(super) fn LMGetKbdType() -> u8;
    pub(super) static kTISPropertyUnicodeKeyLayoutData: CFStringRef;
    pub(super) static kTISPropertyInputSourceID: CFStringRef;
    pub(super) static kTISPropertyLocalizedName: CFStringRef;
    pub(super) static kTISPropertyInputSourceIsASCIICapable: CFStringRef;
    pub(super) static kTISPropertyInputSourceType: CFStringRef;
    pub(super) static kTISTypeKeyboardInputMode: CFStringRef;
}

mod security {
    #![allow(non_upper_case_globals)]
    use super::*;

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        pub static kSecClass: CFStringRef;
        pub static kSecClassInternetPassword: CFStringRef;
        pub static kSecAttrServer: CFStringRef;
        pub static kSecAttrAccount: CFStringRef;
        pub static kSecValueData: CFStringRef;
        pub static kSecReturnAttributes: CFStringRef;
        pub static kSecReturnData: CFStringRef;

        pub fn SecItemAdd(attributes: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
        pub fn SecItemUpdate(query: CFDictionaryRef, attributes: CFDictionaryRef) -> OSStatus;
        pub fn SecItemDelete(query: CFDictionaryRef) -> OSStatus;
        pub fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    }

    pub const errSecSuccess: OSStatus = 0;
    pub const errSecUserCanceled: OSStatus = -128;
    pub const errSecItemNotFound: OSStatus = -25300;
}
