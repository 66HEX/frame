use crate::{
    Action, AnyDrag, AnyElement, AnyImageCache, AnyTooltip, AnyView, App, AppContext, Arena, Asset,
    AsyncWindowContext, AtlasTile, AvailableSpace, BackdropFilter, Background, BorderStyle, Bounds,
    BoxShadow, Capslock, Context, Corners, CursorHideMode, CursorStyle, Decorations, DevicePixels,
    DispatchActionListener, DispatchNodeId, DispatchTree, DisplayId, Edges, Effect, Entity,
    EntityId, EventEmitter, FileDropEvent, Filter, FilterBoundary, FontId, Global, GlobalElementId,
    GlyphId, GlyphRenderMode, GpuSpecs, InputHandler, IntoElement, IsZero, KeyBinding, KeyContext,
    KeyDownEvent, KeyEvent, Keystroke, KeystrokeEvent, LayoutId, Lerp, LineLayoutIndex, Modifiers,
    ModifiersChangedEvent, MonochromeSprite, Motion, MouseButton, MouseEvent, MouseMoveEvent,
    MouseUpEvent, PaintSurface, Path, Pixels, PlatformAtlas, PlatformDisplay, PlatformInput,
    PlatformInputHandler, PlatformWindow, Point, PolychromeSprite, Priority, PromptButton,
    PromptLevel, Quad, RasterizedGlyphFormat, Render, RenderGlyphParams, RenderImage,
    RenderImageParams, RenderSvgParams, Replay, ResizeEdge, ResolvedDirection,
    SMOOTH_SVG_SCALE_FACTOR, SUBPIXEL_VARIANTS_X, SUBPIXEL_VARIANTS_Y, ScaledFilter, ScaledPixels,
    Scene, Shadow, SharedString, Size, StrikethroughStyle, Style, SubpixelSprite, SubscriberSet,
    Subscription, SystemWindowTab, SystemWindowTabController, TabStopMap, TaffyLayoutEngine, Task,
    TextInputConfiguration, TextInputStateChange, TextRenderingMode, TextStyle,
    TextStyleRefinement, ThermalState, TransformationMatrix, Transition, TransitionState,
    Underline, UnderlineStyle, UnicodeBidi, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowControls, WindowDecorations, WindowOptions, WindowParams, WindowTextSystem,
    gestures::{GestureTuning, RecognizedTouchGesture, TouchGestureRecognizer},
    interactive::TouchEvent,
    point, px, rems, size, transparent_black,
    util::{
        atomic_incr_if_not_zero, ceil_to_device_pixel, floor_to_device_pixel,
        round_half_toward_zero, round_half_toward_zero_f64, round_stroke_to_device_pixel,
        round_to_device_pixel,
    },
    white,
};
use anyhow::{Context as _, Result, anyhow};
use collections::{FxHashMap, FxHashSet};
use derive_more::{Deref, DerefMut};
use futures::FutureExt;
use futures::channel::oneshot;
use gpui_util::post_inc;
use gpui_util::{ResultExt, measure};
use itertools::Itertools;
use palette::Hsla;
use parking_lot::RwLock;
use raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle};
use refineable::Refineable;
use scheduler::Instant;
use slotmap::SlotMap;
use smallvec::SmallVec;
use std::{
    any::{Any, TypeId},
    borrow::Cow,
    cell::{Cell, RefCell},
    cmp,
    collections::HashMap,
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::{DerefMut, Range},
    rc::Rc,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
    },
    time::Duration,
};
use uuid::Uuid;

#[cfg(not(target_family = "wasm"))]
use self::a11y::ROOT_NODE_ID;
#[cfg(feature = "profiler")]
use crate::DebugFrameOverlayMode;
#[cfg(any(feature = "inspector", debug_assertions))]
use crate::Inspector;
#[cfg(feature = "profiler")]
use crate::profiler;

pub(crate) mod a11y;
mod prompts;

use a11y::A11y;
pub use a11y::A11ySubtreeBuilder;
pub use prompts::*;

fn quantize_glyph_origin(origin: Point<ScaledPixels>) -> (Point<ScaledPixels>, Point<u8>) {
    fn axis(value: ScaledPixels, variants: u8) -> (ScaledPixels, u8) {
        let quantized = round_half_toward_zero(value.0 * variants as f32) / variants as f32;
        let integer = quantized.floor();
        let variant = ((quantized - integer) * variants as f32).round() as u8;
        (ScaledPixels(integer), variant.min(variants - 1))
    }

    let (integer_x, variant_x) = axis(origin.x, SUBPIXEL_VARIANTS_X);
    let (integer_y, variant_y) = axis(origin.y, SUBPIXEL_VARIANTS_Y);
    (point(integer_x, integer_y), point(variant_x, variant_y))
}

fn quantize_color_glyph_origin(origin: Point<ScaledPixels>) -> (Point<ScaledPixels>, Point<u8>) {
    (
        origin.map(|coordinate| ScaledPixels(round_half_toward_zero(coordinate.0))),
        Point::default(),
    )
}

/// Default window size used when no explicit size is provided.
pub const DEFAULT_WINDOW_SIZE: Size<Pixels> = size(px(1536.), px(1095.));

/// A 6:5 aspect ratio minimum window size to be used for functional,
/// additional-to-main-Zed windows, like the settings and rules library windows.
pub const DEFAULT_ADDITIONAL_WINDOW_SIZE: Size<Pixels> = Size {
    width: Pixels(900.),
    height: Pixels(750.),
};

/// Represents the two different phases when dispatching events.
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
pub enum DispatchPhase {
    /// After the capture phase comes the bubble phase, in which mouse event listeners are
    /// invoked front to back and keyboard event listeners are invoked from the focused element
    /// to the root of the element tree. This is the phase you'll most commonly want to use when
    /// registering event listeners.
    #[default]
    Bubble,
    /// During the initial capture phase, mouse event listeners are invoked back to front, and keyboard
    /// listeners are invoked from the root of the tree downward toward the focused element. This phase
    /// is used for special purposes such as clearing the "pressed" state for click events. If
    /// you stop event propagation during this phase, you need to know what you're doing. Handlers
    /// outside of the immediate region may rely on detecting non-local events during this phase.
    Capture,
}

impl DispatchPhase {
    /// Returns true if this represents the "bubble" phase.
    #[inline]
    pub fn bubble(self) -> bool {
        self == DispatchPhase::Bubble
    }

    /// Returns true if this represents the "capture" phase.
    #[inline]
    pub fn capture(self) -> bool {
        self == DispatchPhase::Capture
    }
}

struct WindowInvalidatorInner {
    #[cfg(feature = "profiler")]
    pub window_id: WindowId,
    pub dirty: bool,
    pub draw_phase: DrawPhase,
    pub dirty_views: FxHashSet<EntityId>,
    pub update_count: usize,
    #[cfg(feature = "profiler")]
    pub frame_dirty: FrameDirtyAccumulator,
    pub platform_waker: Option<Rc<dyn Fn()>>,
}

/// Per-frame invalidation bookkeeping, drained at draw time and emitted to the
/// frame profiler. Tracks when the current frame first became dirty and how
/// many invalidations were coalesced into it, whenever the profiler is
/// compiled in. Retention of the resulting per-frame records is what
/// `profiler::trace_enabled()` controls, not this measurement.
#[cfg(feature = "profiler")]
#[derive(Default)]
struct FrameDirtyAccumulator {
    dirty_at: Option<Instant>,
    invalidations: u64,
}

#[derive(Clone)]
pub(crate) struct WindowInvalidator {
    inner: Rc<RefCell<WindowInvalidatorInner>>,
}

impl WindowInvalidator {
    pub fn new(#[allow(unused_variables)] window_id: WindowId) -> Self {
        WindowInvalidator {
            inner: Rc::new(RefCell::new(WindowInvalidatorInner {
                #[cfg(feature = "profiler")]
                window_id,
                dirty: true,
                draw_phase: DrawPhase::None,
                dirty_views: FxHashSet::default(),
                update_count: 0,
                #[cfg(feature = "profiler")]
                frame_dirty: FrameDirtyAccumulator::default(),
                platform_waker: None,
            })),
        }
    }

    pub fn invalidate_view(&self, entity: EntityId, cx: &mut App) -> bool {
        let mut inner = self.inner.borrow_mut();
        inner.update_count += 1;
        inner.dirty_views.insert(entity);
        if inner.draw_phase == DrawPhase::None {
            #[cfg(feature = "profiler")]
            let dirty_at = Self::record_frame_dirty(&mut inner);
            let became_dirty = !inner.dirty;
            inner.dirty = true;
            let waker = became_dirty.then(|| inner.platform_waker.clone()).flatten();
            #[cfg(feature = "profiler")]
            let window_id = inner.window_id;
            drop(inner);
            #[cfg(feature = "profiler")]
            if became_dirty {
                profiler::journal::record_frame_pending(window_id, dirty_at);
            }
            cx.push_effect(Effect::Notify { emitter: entity });
            if let Some(waker) = waker {
                waker();
            }
            true
        } else {
            false
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.inner.borrow().dirty
    }

    pub fn set_dirty(&self, dirty: bool) {
        let mut inner = self.inner.borrow_mut();
        let became_dirty = dirty && !inner.dirty;
        inner.dirty = dirty;
        if dirty {
            inner.update_count += 1;
        }
        #[cfg(feature = "profiler")]
        let dirty_at = dirty.then(|| Self::record_frame_dirty(&mut inner));
        let waker = became_dirty.then(|| inner.platform_waker.clone()).flatten();
        #[cfg(feature = "profiler")]
        let window_id = inner.window_id;
        drop(inner);
        #[cfg(feature = "profiler")]
        if became_dirty && let Some(dirty_at) = dirty_at {
            profiler::journal::record_frame_pending(window_id, dirty_at);
        }
        if let Some(waker) = waker {
            waker();
        }
    }

    pub fn set_platform_waker(&self, waker: Option<Rc<dyn Fn()>>) {
        let mut inner = self.inner.borrow_mut();
        inner.platform_waker = waker;
        let waker = inner.dirty.then(|| inner.platform_waker.clone()).flatten();
        drop(inner);
        if let Some(waker) = waker {
            waker();
        }
    }

    /// Wakes the platform's frame-request source so a frame request is
    /// delivered even if the platform stops requesting frames for idle
    /// windows. No-op on platforms without a frame waker.
    pub fn wake_platform(&self) {
        let waker = self.inner.borrow().platform_waker.clone();
        if let Some(waker) = waker {
            waker();
        }
    }

    pub fn set_phase(&self, phase: DrawPhase) {
        self.inner.borrow_mut().draw_phase = phase
    }

    pub fn update_count(&self) -> usize {
        self.inner.borrow().update_count
    }

    #[cfg(feature = "profiler")]
    fn record_frame_dirty(inner: &mut WindowInvalidatorInner) -> Instant {
        let dirty_at = *inner.frame_dirty.dirty_at.get_or_insert_with(Instant::now);
        inner.frame_dirty.invalidations += 1;
        dirty_at
    }

    #[cfg(feature = "profiler")]
    fn take_frame_dirty(&self) -> FrameDirtyAccumulator {
        mem::take(&mut self.inner.borrow_mut().frame_dirty)
    }

    pub fn take_views(&self) -> FxHashSet<EntityId> {
        mem::take(&mut self.inner.borrow_mut().dirty_views)
    }

    pub fn replace_views(&self, views: FxHashSet<EntityId>) {
        self.inner.borrow_mut().dirty_views = views;
    }

    pub fn not_drawing(&self) -> bool {
        self.inner.borrow().draw_phase == DrawPhase::None
    }

    #[track_caller]
    pub fn debug_assert_paint(&self) {
        debug_assert!(
            matches!(self.inner.borrow().draw_phase, DrawPhase::Paint),
            "this method can only be called during paint"
        );
    }

    #[track_caller]
    pub fn debug_assert_prepaint(&self) {
        debug_assert!(
            matches!(self.inner.borrow().draw_phase, DrawPhase::Prepaint),
            "this method can only be called during request_layout, or prepaint"
        );
    }

    #[track_caller]
    pub fn debug_assert_paint_or_prepaint(&self) {
        debug_assert!(
            matches!(
                self.inner.borrow().draw_phase,
                DrawPhase::Paint | DrawPhase::Prepaint
            ),
            "this method can only be called during request_layout, prepaint, or paint"
        );
    }
}

type AnyObserver = Box<dyn FnMut(&mut Window, &mut App) -> bool + 'static>;

pub(crate) type AnyWindowFocusListener =
    Box<dyn FnMut(&WindowFocusEvent, &mut Window, &mut App) -> bool + 'static>;

pub(crate) struct WindowFocusEvent {
    pub(crate) previous_focus_path: SmallVec<[FocusId; 8]>,
    pub(crate) current_focus_path: SmallVec<[FocusId; 8]>,
}

impl WindowFocusEvent {
    pub fn is_focus_in(&self, focus_id: FocusId) -> bool {
        !self.previous_focus_path.contains(&focus_id) && self.current_focus_path.contains(&focus_id)
    }

    pub fn is_focus_out(&self, focus_id: FocusId) -> bool {
        self.previous_focus_path.contains(&focus_id) && !self.current_focus_path.contains(&focus_id)
    }
}

/// This is provided when subscribing for `Context::on_focus_out` events.
pub struct FocusOutEvent {
    /// A weak focus handle representing what was blurred.
    pub blurred: WeakFocusHandle,
}

slotmap::new_key_type! {
    /// A globally unique identifier for a focusable element.
    pub struct FocusId;
}

thread_local! {
    /// Fallback arena used when no app-specific arena is active.
    /// In production, each window draw sets CURRENT_ELEMENT_ARENA to the app's arena.
    pub(crate) static ELEMENT_ARENA: RefCell<Arena> = RefCell::new(Arena::new(1024 * 1024));

    /// Points to the current App's element arena during draw operations.
    /// This allows multiple test Apps to have isolated arenas, preventing
    /// cross-session corruption when the scheduler interleaves their tasks.
    static CURRENT_ELEMENT_ARENA: Cell<Option<*const RefCell<Arena>>> = const { Cell::new(None) };
}

/// Whether a window draw is currently in progress on this thread.
///
/// This holds exactly while an `ElementArenaScope` is active: nested scopes
/// restore the previous (still set) arena pointer, so `CURRENT_ELEMENT_ARENA`
/// is `Some` from the outermost draw's start to its end.
///
/// The `on_request_frame` callback uses this to defer draw requests that
/// arrive re-entrantly while a draw is already on the stack (e.g. via nested
/// message pumping in the Windows window procedure), instead of running a
/// nested draw or panicking on the already-borrowed App.
fn draw_in_progress() -> bool {
    CURRENT_ELEMENT_ARENA.with(|current| current.get().is_some())
}

/// Allocates an element in the current arena. Uses the app-specific arena if one
/// is active (during draw), otherwise falls back to the thread-local ELEMENT_ARENA.
pub(crate) fn with_element_arena<R>(f: impl FnOnce(&mut Arena) -> R) -> R {
    CURRENT_ELEMENT_ARENA.with(|current| {
        if let Some(arena_ptr) = current.get() {
            // SAFETY: The pointer is valid for the duration of the draw operation
            // that set it, and we're being called during that same draw.
            let arena_cell = unsafe { &*arena_ptr };
            f(&mut arena_cell.borrow_mut())
        } else {
            ELEMENT_ARENA.with_borrow_mut(f)
        }
    })
}

/// Scope guard that sets CURRENT_ELEMENT_ARENA for the duration of a draw
/// operation and tracks the arena's scope depth, so that a nested draw's
/// `ArenaClearNeeded::clear` is deferred rather than freeing memory the outer
/// draw still references (see `Arena::clear`).
///
/// Call [`ElementArenaScope::exit`] with the same arena that was entered to
/// obtain the [`ArenaClearNeeded`] token the draw now owes; requiring `exit`
/// makes it impossible to request a clear before the scope has ended. The
/// scope's teardown — restoring the thread-local and balancing `begin_scope`
/// with `end_scope` — happens in `Drop`, so the arena's scope depth stays
/// balanced on every path, including when a panic unwinds a draw before `exit`
/// is reached. (If teardown lived only in `exit`, such a panic would leave the
/// scope depth permanently elevated and defer every future clear, leaking
/// memory unboundedly.)
pub(crate) struct ElementArenaScope {
    /// The entered arena: compared against the argument in `exit`, and
    /// dereferenced in `Drop` to end its scope (see the SAFETY note there).
    entered: *const RefCell<Arena>,
    previous: Option<*const RefCell<Arena>>,
    exited: bool,
}

impl ElementArenaScope {
    /// Enter a scope where element allocations use the given arena.
    pub(crate) fn enter(arena: &RefCell<Arena>) -> Self {
        arena.borrow_mut().begin_scope();
        let previous = CURRENT_ELEMENT_ARENA.with(|current| {
            let prev = current.get();
            current.set(Some(arena as *const RefCell<Arena>));
            prev
        });
        Self {
            entered: arena as *const RefCell<Arena>,
            previous,
            exited: false,
        }
    }

    /// End the scope: restores the previously-current arena and ends the
    /// arena's clear-deferral scope. Returns the token for the arena clear the
    /// draw now owes; producing it here makes it impossible to request a clear
    /// before the scope has ended (which would be silently deferred forever).
    ///
    /// Panics if passed a different arena than was entered: ending the scope
    /// of the wrong arena would unbalance two arenas' scope depths, allowing
    /// one of them to clear while a draw still references its memory.
    pub(crate) fn exit(mut self, arena: &RefCell<Arena>) -> ArenaClearNeeded {
        assert!(
            std::ptr::eq(self.entered, arena),
            "ElementArenaScope::exit called with a different arena than was entered"
        );
        self.exited = true;
        // Teardown (restoring the thread-local and ending the arena's
        // clear-deferral scope) runs in `Drop`, which fires both here — `self`
        // is dropped as `exit` returns, before the token reaches the caller —
        // and when a panic unwinds the draw before `exit` is reached.
        ArenaClearNeeded::new(arena)
    }
}

impl Drop for ElementArenaScope {
    fn drop(&mut self) {
        // Teardown lives here (rather than in `exit`) so it runs exactly once on
        // every path: `exit` consumes and drops the guard on the normal path,
        // and unwinding drops it on the panic path. Balancing `begin_scope` here
        // keeps the arena's scope depth correct even when a draw panics; if this
        // only happened in `exit`, a panic between `enter` and `exit` would leave
        // the depth elevated and defer every future clear.
        CURRENT_ELEMENT_ARENA.with(|current| {
            current.set(self.previous);
        });
        // SAFETY: `entered` came from a `&RefCell<Arena>` in `enter`, and the
        // arena (owned by the `App` being drawn) outlives this guard on both the
        // normal and unwinding paths, since the guard is a local of the draw.
        unsafe { &*self.entered }.borrow_mut().end_scope();
        if !self.exited && !std::thread::panicking() {
            debug_assert!(false, "ElementArenaScope dropped without calling exit()");
            log::error!(
                "ElementArenaScope dropped without calling exit(); \
                 the arena clear for this draw was never requested"
            );
        }
    }
}

/// Returned when the element arena has been used and so must be cleared before the next draw.
#[must_use]
pub struct ArenaClearNeeded {
    /// Identity of the arena that was drawn into. Only ever compared against
    /// another pointer in `clear`; never dereferenced.
    arena: *const RefCell<Arena>,
}

impl ArenaClearNeeded {
    /// Create a new ArenaClearNeeded token for the App whose arena was drawn
    /// into. Private: the only way to obtain one is [`ElementArenaScope::exit`].
    fn new(arena: &RefCell<Arena>) -> Self {
        Self {
            arena: arena as *const RefCell<Arena>,
        }
    }

    /// Clear the element arena of the App the draw ran against. If an enclosing
    /// draw is still in progress (this draw was nested inside it), the clear is
    /// deferred to the enclosing draw's own `ArenaClearNeeded` so that its live
    /// allocations aren't freed.
    ///
    /// Panics if passed a different App than the draw ran against, since
    /// clearing another App's arena could free memory its draws still
    /// reference.
    pub fn clear(self, cx: &mut App) {
        assert!(
            std::ptr::eq(self.arena, &cx.element_arena),
            "ArenaClearNeeded::clear called with a different App than the draw ran against"
        );
        cx.element_arena.borrow_mut().clear();
    }
}

pub(crate) type FocusMap = RwLock<SlotMap<FocusId, FocusRef>>;
pub(crate) struct FocusRef {
    pub(crate) ref_count: AtomicUsize,
    pub(crate) tab_index: isize,
    pub(crate) tab_stop: bool,
}

impl FocusId {
    /// Obtains whether the element associated with this handle is currently focused.
    pub fn is_focused(&self, window: &Window) -> bool {
        window.focus == Some(*self)
    }

    /// Obtains whether the element associated with this handle contains the focused
    /// element or is itself focused.
    pub fn contains_focused(&self, window: &Window, cx: &App) -> bool {
        window
            .focused(cx)
            .is_some_and(|focused| self.contains(focused.id, window))
    }

    /// Obtains whether the element associated with this handle is contained within the
    /// focused element or is itself focused.
    pub fn within_focused(&self, window: &Window, cx: &App) -> bool {
        let focused = window.focused(cx);
        focused.is_some_and(|focused| focused.id.contains(*self, window))
    }

    /// Obtains whether this handle contains the given handle in the most recently rendered frame.
    pub(crate) fn contains(&self, other: Self, window: &Window) -> bool {
        window
            .rendered_frame
            .dispatch_tree
            .focus_contains(*self, other)
    }
}

/// A handle which can be used to track and manipulate the focused element in a window.
pub struct FocusHandle {
    pub(crate) id: FocusId,
    handles: Arc<FocusMap>,
    /// The index of this element in the tab order.
    pub tab_index: isize,
    /// Whether this element can be focused by tab navigation.
    pub tab_stop: bool,
}

impl std::fmt::Debug for FocusHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("FocusHandle({:?})", self.id))
    }
}

impl FocusHandle {
    pub(crate) fn new(handles: &Arc<FocusMap>) -> Self {
        let id = handles.write().insert(FocusRef {
            ref_count: AtomicUsize::new(1),
            tab_index: 0,
            tab_stop: false,
        });

        Self {
            id,
            tab_index: 0,
            tab_stop: false,
            handles: handles.clone(),
        }
    }

    pub(crate) fn for_id(id: FocusId, handles: &Arc<FocusMap>) -> Option<Self> {
        let lock = handles.read();
        let focus = lock.get(id)?;
        if atomic_incr_if_not_zero(&focus.ref_count) == 0 {
            return None;
        }
        Some(Self {
            id,
            tab_index: focus.tab_index,
            tab_stop: focus.tab_stop,
            handles: handles.clone(),
        })
    }

    /// Sets the tab index of the element associated with this handle.
    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        if let Some(focus) = self.handles.write().get_mut(self.id) {
            focus.tab_index = index;
        }
        self
    }

    /// Sets whether the element associated with this handle is a tab stop.
    ///
    /// When `false`, the element will not be included in the tab order.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        if let Some(focus) = self.handles.write().get_mut(self.id) {
            focus.tab_stop = tab_stop;
        }
        self
    }

    /// Converts this focus handle into a weak variant, which does not prevent it from being released.
    pub fn downgrade(&self) -> WeakFocusHandle {
        WeakFocusHandle {
            id: self.id,
            handles: Arc::downgrade(&self.handles),
        }
    }

    /// Moves the focus to the element associated with this handle.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(self, cx)
    }

    /// Obtains whether the element associated with this handle is currently focused.
    pub fn is_focused(&self, window: &Window) -> bool {
        self.id.is_focused(window)
    }

    /// Obtains whether the element associated with this handle contains the focused
    /// element or is itself focused.
    pub fn contains_focused(&self, window: &Window, cx: &App) -> bool {
        self.id.contains_focused(window, cx)
    }

    /// Obtains whether the element associated with this handle is contained within the
    /// focused element or is itself focused.
    pub fn within_focused(&self, window: &Window, cx: &mut App) -> bool {
        self.id.within_focused(window, cx)
    }

    /// Obtains whether this handle contains the given handle in the most recently rendered frame.
    pub fn contains(&self, other: &Self, window: &Window) -> bool {
        self.id.contains(other.id, window)
    }

    /// Dispatch an action on the element that rendered this focus handle
    pub fn dispatch_action(&self, action: &dyn Action, window: &mut Window, cx: &mut App) {
        if let Some(node_id) = window
            .rendered_frame
            .dispatch_tree
            .focusable_node_id(self.id)
        {
            window.dispatch_action_on_node(node_id, action, cx)
        }
    }
}

impl Clone for FocusHandle {
    fn clone(&self) -> Self {
        Self::for_id(self.id, &self.handles).unwrap()
    }
}

impl PartialEq for FocusHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for FocusHandle {}

impl Drop for FocusHandle {
    fn drop(&mut self) {
        self.handles
            .read()
            .get(self.id)
            .unwrap()
            .ref_count
            .fetch_sub(1, SeqCst);
    }
}

/// A weak reference to a focus handle.
#[derive(Clone, Debug)]
pub struct WeakFocusHandle {
    pub(crate) id: FocusId,
    pub(crate) handles: Weak<FocusMap>,
}

impl WeakFocusHandle {
    /// Attempts to upgrade the [WeakFocusHandle] to a [FocusHandle].
    pub fn upgrade(&self) -> Option<FocusHandle> {
        let handles = self.handles.upgrade()?;
        FocusHandle::for_id(self.id, &handles)
    }
}

impl PartialEq for WeakFocusHandle {
    fn eq(&self, other: &WeakFocusHandle) -> bool {
        self.id == other.id
    }
}

impl Eq for WeakFocusHandle {}

impl PartialEq<FocusHandle> for WeakFocusHandle {
    fn eq(&self, other: &FocusHandle) -> bool {
        self.id == other.id
    }
}

impl PartialEq<WeakFocusHandle> for FocusHandle {
    fn eq(&self, other: &WeakFocusHandle) -> bool {
        self.id == other.id
    }
}

/// Focusable allows users of your view to easily
/// focus it (using window.focus_view(cx, view))
pub trait Focusable: 'static {
    /// Returns the focus handle associated with this view.
    fn focus_handle(&self, cx: &App) -> FocusHandle;
}

impl<V: Focusable> Focusable for Entity<V> {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.read(cx).focus_handle(cx)
    }
}

/// ManagedView is a view (like a Modal, Popover, Menu, etc.)
/// where the lifecycle of the view is handled by another view.
pub trait ManagedView: Focusable + EventEmitter<DismissEvent> + Render {}

impl<M: Focusable + EventEmitter<DismissEvent> + Render> ManagedView for M {}

/// Emitted by implementers of [`ManagedView`] to indicate the view should be dismissed, such as when a view is presented as a modal.
pub struct DismissEvent;

type FrameCallback = Box<dyn FnOnce(&mut Window, &mut App)>;

pub(crate) type AnyMouseListener =
    Box<dyn FnMut(&dyn Any, DispatchPhase, &mut Window, &mut App) + 'static>;

#[derive(Clone, Copy)]
pub(crate) struct PointerChange {
    pub(crate) position: Option<Point<Pixels>>,
    pub(crate) cause: PointerChangeCause,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerChangeCause {
    Input,
    Layout,
}

type PointerObserver = Rc<dyn Fn(PointerChange, &mut Window, &mut App)>;

#[derive(Clone)]
enum CursorStyleRequest {
    Window(CursorStyleSource),
    Hitbox {
        id: HitboxId,
        style: CursorStyleSource,
    },
}

#[derive(Clone)]
enum CursorStyleSource {
    Fixed(CursorStyle),
    Position(Rc<dyn Fn(Point<Pixels>) -> Option<CursorStyle>>),
}

impl CursorStyleSource {
    fn resolve(&self, position: Point<Pixels>) -> Option<CursorStyle> {
        match self {
            Self::Fixed(style) => Some(*style),
            Self::Position(resolve) => resolve(position),
        }
    }
}

/// Contains information about an occlusion test through hitboxes at a mouse position.
#[derive(Default, Eq, PartialEq)]
pub struct HitTest {
    // HitboxIds which were included in the test's click path (the last of which will be the box that caused occlusion)
    ordered_ids: SmallVec<[HitboxId; 8]>,
    // metadata about all hitboxes, even those outside the test's path
    entries: HashMap<HitboxId, HitTestEntry>,
    // the number of hitbox ids in `ordered_ids` which support mouse. anything after this amount only support scroll.
    hover_hitbox_count: usize,
}

impl HitTest {
    /// Creates a new hit-test by iterating the provided hitboxes to find all those which are not occluded.
    fn new<'a>(hitboxes: impl Iterator<Item = &'a Hitbox>, position: Point<Pixels>) -> Self {
        let mut num_until_mouse_blocked = None::<usize>;
        let mut ids_until_occlusion = SmallVec::default();
        let mut entries = HashMap::default();
        let mut found_hit_occlusion = false;
        // index is 0=closest to viewer, n=farthest from viewer
        for (index, hitbox) in hitboxes.enumerate() {
            entries.insert(
                hitbox.id,
                HitTestEntry {
                    depth: index,
                    tags: hitbox.tags.clone(),
                },
            );

            if found_hit_occlusion {
                continue;
            }

            if !hitbox.contains(&position) {
                continue;
            }

            ids_until_occlusion.push(hitbox.id);

            // Identify the threshold at which mouse is blocked in any way (BlockMouseExceptScroll or BlockMouse)
            if num_until_mouse_blocked.is_none() && hitbox.behavior.blocks_mouse() {
                num_until_mouse_blocked = Some(ids_until_occlusion.len());
            }

            // Found a hitbox that fully occludes boxes behind it. Can now stop iterating.
            if hitbox.behavior.occludes() {
                found_hit_occlusion = true;
            }
        }
        Self {
            hover_hitbox_count: num_until_mouse_blocked.unwrap_or(ids_until_occlusion.len()),
            ordered_ids: ids_until_occlusion,
            entries,
        }
    }

    /// Returns an iterator over all HitboxIds that were not occluded/blocked.
    /// The last element in the list will be the element which blocked the mouse from hitboxes behind it.
    pub fn iter_hovered(&self) -> impl Iterator<Item = &HitboxId> {
        self.ordered_ids.iter().take(self.hover_hitbox_count)
    }

    /// Returns an iterator over all HitboxIds that were not blocking scroll (not `HitboxBehavior::BlockMouse`).
    /// The last element in the list will be the element which blocked the mouse from hitboxes behind it.
    pub fn iter_scrollable(&self) -> impl Iterator<Item = &HitboxId> {
        self.ordered_ids.iter()
    }

    /// Returns metadata about a hitbox included in this hit-test.
    pub fn entry(&self, id: &HitboxId) -> Option<&HitTestEntry> {
        self.entries.get(id)
    }
}

/// An entry in a [`HitTest`] which represents a [`Hitbox`]. Contains some metadata about the
/// hitbox in the frame the HitTest was evaluated on.
#[derive(Debug, PartialEq, Eq)]
pub struct HitTestEntry {
    /// The depth of the hitbox in the ordered list from the Frame.
    /// A smaller number is closer to the viewer, and a larger number is farther from the viewer.
    depth: usize,
    tags: Vec<SharedString>,
}

impl HitTestEntry {
    /// Returns the depth of the [`Hitbox`] in the frame.
    /// A smaller value (closer to 0) represents closer to the viewer.
    /// A larger value represents a hitbox farther from the viewer.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Returns any tags assigned to the [`Hitbox`] in the frame.
    pub fn tags(&self) -> &Vec<SharedString> {
        &self.tags
    }
}

/// A type of window control area that corresponds to the platform window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowControlArea {
    /// An area that allows dragging of the platform window.
    Drag,
    /// An area that allows closing of the platform window.
    Close,
    /// An area that allows maximizing of the platform window.
    Max,
    /// An area that allows minimizing of the platform window.
    Min,
}

fn hit_test_window_control_area(
    hit_test: &HitTest,
    window_control_hitboxes: &[(WindowControlArea, Hitbox)],
) -> Option<WindowControlArea> {
    let hitbox_id = hit_test.ordered_ids.first()?;
    window_control_hitboxes
        .iter()
        .find_map(|(area, hitbox)| (hitbox.id == *hitbox_id).then_some(*area))
}

#[cfg(test)]
mod window_control_hit_test_tests {
    use super::*;

    fn hitbox(id: u64) -> Hitbox {
        Hitbox {
            id: HitboxId(id),
            bounds: Bounds::default(),
            content_mask: ContentMask::default(),
            behavior: HitboxBehavior::Normal,
            fragments: None,
            tags: Vec::new(),
        }
    }

    fn hit_test(ids: &[u64]) -> HitTest {
        let mut hit_test = HitTest::default();
        hit_test
            .ordered_ids
            .extend(ids.iter().copied().map(HitboxId));
        hit_test.hover_hitbox_count = hit_test.ordered_ids.len();
        hit_test
    }

    #[test]
    fn topmost_regular_hitbox_suppresses_window_control_area_behind_it() {
        let controls = [(WindowControlArea::Drag, hitbox(1))];

        assert_eq!(
            hit_test_window_control_area(&hit_test(&[2, 1]), &controls),
            None
        );
    }

    #[test]
    fn topmost_window_control_hitbox_returns_its_area() {
        let controls = [
            (WindowControlArea::Drag, hitbox(1)),
            (WindowControlArea::Close, hitbox(2)),
        ];

        assert_eq!(
            hit_test_window_control_area(&hit_test(&[2, 1]), &controls),
            Some(WindowControlArea::Close)
        );
    }
}

/// An identifier for a [Hitbox] which also includes [HitboxBehavior].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct HitboxId(u64);

#[cfg(feature = "test-support")]
impl HitboxId {
    /// A placeholder HitboxId exclusively for integration testing API's that
    /// need a hitbox but where the value of the hitbox does not matter. The
    /// alternative is to make the Hitbox optional but that complicates the
    /// implementation.
    pub const fn placeholder() -> Self {
        Self(0)
    }
}

impl HitboxId {
    /// Checks if the hitbox with this ID is currently hovered. Returns `false` during keyboard
    /// input modality so that keyboard navigation suppresses hover highlights. Except when handling
    /// `ScrollWheelEvent`, this is typically what you want when determining whether to handle mouse
    /// events or paint hover styles.
    ///
    /// See [`Hitbox::is_hovered`] for details.
    pub fn is_hovered(self, window: &Window) -> bool {
        // If this hitbox has captured the pointer, it's always considered hovered
        if window.captured_hitbox == Some(self) {
            return true;
        }
        if window.hover_suppressed_by_keyboard() {
            return false;
        }
        self.hit_test(window)
    }

    /// Checks if the hitbox with this ID is currently hovered, regardless of the last
    /// input modality used.
    ///
    /// See [`HitboxId::is_hovered`] for more details.
    pub(crate) fn is_hovered_ignoring_last_input(self, window: &Window) -> bool {
        // If this hitbox has captured the pointer, it's always considered hovered
        if window.captured_hitbox == Some(self) {
            return true;
        }
        self.hit_test(window)
    }

    fn hit_test(self, window: &Window) -> bool {
        window.mouse_hit_test().iter_hovered().contains(&self)
    }

    /// Checks if the hitbox with this ID contains the mouse and should handle scroll events.
    /// Typically this should only be used when handling `ScrollWheelEvent`, and otherwise
    /// `is_hovered` should be used. See the documentation of `Hitbox::is_hovered` for details about
    /// this distinction.
    pub fn should_handle_scroll(self, window: &Window) -> bool {
        window.mouse_hit_test().iter_scrollable().contains(&self)
    }

    fn next(mut self) -> HitboxId {
        HitboxId(self.0.wrapping_add(1))
    }
}

/// A rectangular region that potentially blocks hitboxes inserted prior.
/// See [Window::insert_hitbox] for more details.
#[derive(Clone, Debug, Deref)]
pub struct Hitbox {
    /// A unique identifier for the hitbox.
    pub id: HitboxId,
    /// The bounds of the hitbox.
    #[deref]
    pub bounds: Bounds<Pixels>,
    /// The content mask when the hitbox was inserted.
    pub content_mask: ContentMask<Pixels>,
    /// Flags that specify hitbox behavior.
    pub behavior: HitboxBehavior,
    /// Disjoint regions of an inline element. `bounds` is their union.
    pub fragments: Option<Arc<[Bounds<Pixels>]>>,
    /// Additional user-provided tags to extend behavior of the hitbox.
    pub tags: Vec<SharedString>,
}

impl Hitbox {
    /// Tests the actual regions, including the content mask, without occlusion checks.
    pub fn contains(&self, point: &Point<Pixels>) -> bool {
        self.content_mask.bounds.contains(point)
            && self.fragments.as_ref().map_or_else(
                || self.bounds.contains(point),
                |fragments| fragments.iter().any(|bounds| bounds.contains(point)),
            )
    }

    /// Checks if the hitbox is currently hovered. Returns `false` during keyboard input modality
    /// so that keyboard navigation suppresses hover highlights. Except when handling
    /// `ScrollWheelEvent`, this is typically what you want when determining whether to handle mouse
    /// events or paint hover styles.
    ///
    /// This can return `false` even when the hitbox contains the mouse, if a hitbox in front of
    /// this sets `HitboxBehavior::BlockMouse` (`InteractiveElement::occlude`) or
    /// `HitboxBehavior::BlockMouseExceptScroll` (`InteractiveElement::block_mouse_except_scroll`),
    /// or if the current input modality is keyboard (see [`Window::last_input_was_keyboard`]).
    ///
    /// Handling of `ScrollWheelEvent` should typically use `should_handle_scroll` instead.
    /// Concretely, this is due to use-cases like overlays that cause the elements under to be
    /// non-interactive while still allowing scrolling. More abstractly, this is because
    /// `is_hovered` is about element interactions directly under the mouse - mouse moves, clicks,
    /// hover styling, etc. In contrast, scrolling is about finding the current outer scrollable
    /// container.
    pub fn is_hovered(&self, window: &Window) -> bool {
        self.id.is_hovered(window)
    }

    /// Checks if the hitbox contains the mouse and should handle scroll events. Typically this
    /// should only be used when handling `ScrollWheelEvent`, and otherwise `is_hovered` should be
    /// used. See the documentation of `Hitbox::is_hovered` for details about this distinction.
    ///
    /// This can return `false` even when the hitbox contains the mouse, if a hitbox in front of
    /// this sets `HitboxBehavior::BlockMouse` (`InteractiveElement::occlude`).
    pub fn should_handle_scroll(&self, window: &Window) -> bool {
        self.id.should_handle_scroll(window)
    }
}

/// How the hitbox affects mouse behavior.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum HitboxBehavior {
    /// Normal hitbox mouse behavior, doesn't affect mouse handling for other hitboxes.
    #[default]
    Normal,

    /// All hitboxes behind this hitbox will be ignored and so will have `hitbox.is_hovered() ==
    /// false` and `hitbox.should_handle_scroll() == false`. Typically for elements this causes
    /// skipping of all mouse events, hover styles, and tooltips. This flag is set by
    /// [`InteractiveElement::occlude`].
    ///
    /// For mouse handlers that check those hitboxes, this behaves the same as registering a
    /// bubble-phase handler for every mouse event type:
    ///
    /// ```ignore
    /// window.on_mouse_event(move |_: &EveryMouseEventTypeHere, phase, window, cx| {
    ///     if phase == DispatchPhase::Capture && hitbox.is_hovered(window) {
    ///         cx.stop_propagation();
    ///     }
    /// })
    /// ```
    ///
    /// This has effects beyond event handling - any use of hitbox checking, such as hover
    /// styles and tooltips. These other behaviors are the main point of this mechanism. An
    /// alternative might be to not affect mouse event handling - but this would allow
    /// inconsistent UI where clicks and moves interact with elements that are not considered to
    /// be hovered.
    BlockMouse,

    /// All hitboxes behind this hitbox will have `hitbox.is_hovered() == false`, even when
    /// `hitbox.should_handle_scroll() == true`. Typically for elements this causes all mouse
    /// interaction except scroll events to be ignored - see the documentation of
    /// [`Hitbox::is_hovered`] for details. This flag is set by
    /// [`InteractiveElement::block_mouse_except_scroll`].
    ///
    /// For mouse handlers that check those hitboxes, this behaves the same as registering a
    /// bubble-phase handler for every mouse event type **except** `ScrollWheelEvent`:
    ///
    /// ```ignore
    /// window.on_mouse_event(move |_: &EveryMouseEventTypeExceptScroll, phase, window, cx| {
    ///     if phase == DispatchPhase::Bubble && hitbox.should_handle_scroll(window) {
    ///         cx.stop_propagation();
    ///     }
    /// })
    /// ```
    ///
    /// See the documentation of [`Hitbox::is_hovered`] for details of why `ScrollWheelEvent` is
    /// handled differently than other mouse events. If also blocking these scroll events is
    /// desired, then a `cx.stop_propagation()` handler like the one above can be used.
    ///
    /// This has effects beyond event handling - this affects any use of `is_hovered`, such as
    /// hover styles and tooltips. These other behaviors are the main point of this mechanism.
    /// An alternative might be to not affect mouse event handling - but this would allow
    /// inconsistent UI where clicks and moves interact with elements that are not considered to
    /// be hovered.
    BlockMouseExceptScroll,
}

impl HitboxBehavior {
    /// Returns true if this behavior should prevent hit-tests from marking hitboxes behind it as hovered (aka this is the last element that can be hovered).
    pub fn blocks_mouse(&self) -> bool {
        matches!(self, Self::BlockMouse | Self::BlockMouseExceptScroll)
    }

    /// Returns true if this behavior shops all hit-test behavior (both mouse and scroll).
    pub fn occludes(&self) -> bool {
        matches!(self, Self::BlockMouse)
    }
}

/// An identifier for a tooltip.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct TooltipId(usize);

impl TooltipId {
    /// Checks if the tooltip is currently hovered.
    pub fn is_hovered(&self, window: &Window) -> bool {
        window
            .tooltip_bounds
            .as_ref()
            .is_some_and(|tooltip_bounds| {
                tooltip_bounds.id == *self
                    && tooltip_bounds.bounds.contains(&window.mouse_position())
            })
    }
}

pub(crate) struct TooltipBounds {
    id: TooltipId,
    bounds: Bounds<Pixels>,
}

#[derive(Clone)]
pub(crate) struct TooltipRequest {
    id: TooltipId,
    tooltip: AnyTooltip,
}

pub(crate) struct DeferredDraw {
    current_view: EntityId,
    priority: usize,
    parent_node: DispatchNodeId,
    element_id_stack: SmallVec<[ElementId; 32]>,
    text_style_stack: Vec<TextStyleRefinement>,
    content_mask: Option<ContentMask<Pixels>>,
    rem_size: Pixels,
    element: Option<AnyElement>,
    absolute_offset: Point<Pixels>,
    prepaint_range: Range<PrepaintStateIndex>,
    paint_range: Range<PaintIndex>,
}

pub(crate) struct Frame {
    pub(crate) focus: Option<FocusId>,
    pub(crate) window_active: bool,
    pub(crate) element_states: FxHashMap<(GlobalElementId, TypeId), ElementStateBox>,
    accessed_element_states: Vec<(GlobalElementId, TypeId)>,
    pub(crate) mouse_listeners: Vec<Option<AnyMouseListener>>,
    pointer_observers: Rc<Vec<PointerObserver>>,
    pub(crate) dispatch_tree: DispatchTree,
    pub(crate) scene: Scene,
    pub(crate) hitboxes: Vec<Hitbox>,
    pub(crate) window_control_hitboxes: Vec<(WindowControlArea, Hitbox)>,
    pub(crate) deferred_draws: Vec<DeferredDraw>,
    pub(crate) input_handlers: Vec<Option<PlatformInputHandler>>,
    pub(crate) tooltip_requests: Vec<Option<TooltipRequest>>,
    cursor_styles: Vec<CursorStyleRequest>,
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) debug_bounds: FxHashMap<String, Bounds<Pixels>>,
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub(crate) next_inspector_instance_ids: FxHashMap<Rc<crate::InspectorElementPath>, usize>,
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub(crate) inspector_hitboxes: FxHashMap<HitboxId, crate::InspectorElementId>,
    pub(crate) tab_stops: TabStopMap,
}

#[derive(Clone, Default)]
pub(crate) struct PrepaintStateIndex {
    hitboxes_index: usize,
    tooltips_index: usize,
    deferred_draws_index: usize,
    dispatch_tree_index: usize,
    accessed_element_states_index: usize,
    line_layout_index: LineLayoutIndex,
}

#[derive(Clone, Default)]
pub(crate) struct PaintIndex {
    scene_index: usize,
    mouse_listeners_index: usize,
    pointer_observers_index: usize,
    input_handlers_index: usize,
    cursor_styles_index: usize,
    accessed_element_states_index: usize,
    tab_handle_index: usize,
    line_layout_index: LineLayoutIndex,
}

impl Frame {
    pub(crate) fn new(dispatch_tree: DispatchTree) -> Self {
        Frame {
            focus: None,
            window_active: false,
            element_states: FxHashMap::default(),
            accessed_element_states: Vec::new(),
            mouse_listeners: Vec::new(),
            pointer_observers: Rc::default(),
            dispatch_tree,
            scene: Scene::default(),
            hitboxes: Vec::new(),
            window_control_hitboxes: Vec::new(),
            deferred_draws: Vec::new(),
            input_handlers: Vec::new(),
            tooltip_requests: Vec::new(),
            cursor_styles: Vec::new(),

            #[cfg(any(test, feature = "test-support"))]
            debug_bounds: FxHashMap::default(),

            #[cfg(any(feature = "inspector", debug_assertions))]
            next_inspector_instance_ids: FxHashMap::default(),

            #[cfg(any(feature = "inspector", debug_assertions))]
            inspector_hitboxes: FxHashMap::default(),
            tab_stops: TabStopMap::default(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.element_states.clear();
        self.accessed_element_states.clear();
        self.mouse_listeners.clear();
        Rc::make_mut(&mut self.pointer_observers).clear();
        self.dispatch_tree.clear();
        self.scene.clear();
        self.input_handlers.clear();
        self.tooltip_requests.clear();
        self.cursor_styles.clear();
        self.hitboxes.clear();
        self.window_control_hitboxes.clear();
        self.deferred_draws.clear();
        self.tab_stops.clear();
        self.focus = None;

        #[cfg(any(test, feature = "test-support"))]
        {
            self.debug_bounds.clear();
        }

        #[cfg(any(feature = "inspector", debug_assertions))]
        {
            self.next_inspector_instance_ids.clear();
            self.inspector_hitboxes.clear();
        }
    }

    pub(crate) fn cursor_style(&self, window: &Window) -> Option<CursorStyle> {
        // Resolve window-wide requests first, regardless of paint order.
        self.cursor_styles
            .iter()
            .rev()
            .find_map(|request| match request {
                CursorStyleRequest::Window(style) => style.resolve(window.mouse_position()),
                CursorStyleRequest::Hitbox { .. } => None,
            })
            .or_else(|| {
                if !window.pointer_inside_window {
                    return None;
                }
                self.cursor_styles
                    .iter()
                    .rev()
                    .find_map(|request| match request {
                        CursorStyleRequest::Hitbox { id, style }
                            if id.is_hovered_ignoring_last_input(window) =>
                        {
                            style.resolve(window.mouse_position())
                        }
                        _ => None,
                    })
            })
    }

    pub(crate) fn hit_test(&self, position: Point<Pixels>) -> HitTest {
        HitTest::new(self.hitboxes.iter().rev(), position)
    }

    pub(crate) fn focus_path(&self) -> SmallVec<[FocusId; 8]> {
        self.focus
            .map(|focus_id| self.dispatch_tree.focus_path(focus_id))
            .unwrap_or_default()
    }

    pub(crate) fn finish(&mut self, prev_frame: &mut Self) {
        for element_state_key in &self.accessed_element_states {
            if let Some((element_state_key, element_state)) =
                prev_frame.element_states.remove_entry(element_state_key)
            {
                self.element_states.insert(element_state_key, element_state);
            }
        }

        self.scene.finish();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
enum InputModality {
    Mouse,
    Keyboard,
    Touch,
}

/// Holds the state for a specific window.
pub struct Window {
    pub(crate) handle: AnyWindowHandle,
    pub(crate) invalidator: WindowInvalidator,
    pub(crate) removed: bool,
    pub(crate) platform_window: Box<dyn PlatformWindow>,
    display_id: Option<DisplayId>,
    is_resizable: bool,
    is_minimizable: bool,
    sprite_atlas: Arc<dyn PlatformAtlas>,
    text_system: Arc<WindowTextSystem>,
    text_rendering_mode: Rc<Cell<TextRenderingMode>>,
    rem_size: Pixels,
    /// The stack of override values for the window's rem size.
    ///
    /// This is used by `with_rem_size` to allow rendering an element tree with
    /// a given rem size.
    rem_size_override_stack: SmallVec<[Pixels; 8]>,
    pub(crate) viewport_size: Size<Pixels>,
    layout_engine: Option<TaffyLayoutEngine>,
    pub(crate) collecting_inline: bool,
    pub(crate) current_inline_fragments: Option<Arc<[Bounds<Pixels>]>>,
    pub(crate) root: Option<AnyView>,
    pub(crate) element_id_stack: SmallVec<[ElementId; 32]>,
    pub(crate) text_style_stack: Vec<TextStyleRefinement>,
    pub(crate) measurement_direction: ResolvedDirection,
    pub(crate) measurement_unicode_bidi: UnicodeBidi,
    pub(crate) rendered_entity_stack: Vec<EntityId>,
    pub(crate) element_offset_stack: Vec<Point<Pixels>>,
    /// Bounds of the parent `Div` currently prepainting this element as one of its children.
    /// Inset transitions use these bounds to resolve `auto` from the child's rendered position.
    style_transition_containing_bounds: Option<Bounds<Pixels>>,
    pub(crate) content_mask_stack: Vec<ContentMask<Pixels>>,
    pub(crate) requested_autoscroll: Option<Bounds<Pixels>>,
    /// The [`TextInputConfiguration`] most recently forwarded to the platform
    /// window, so that only actual changes are forwarded (reconfiguring a live
    /// input session can restart the IME connection).
    last_text_input_configuration: Option<TextInputConfiguration>,
    focused_text_input_active: bool,
    pub(crate) image_cache_stack: Vec<AnyImageCache>,
    pub(crate) rendered_frame: Frame,
    pub(crate) next_frame: Frame,
    next_hitbox_id: HitboxId,
    pub(crate) next_tooltip_id: TooltipId,
    pub(crate) tooltip_bounds: Option<TooltipBounds>,
    pub(crate) next_frame_callbacks: Rc<RefCell<Vec<FrameCallback>>>,
    pub(crate) dirty_views: FxHashSet<EntityId>,
    focus_listeners: SubscriberSet<(), AnyWindowFocusListener>,
    pub(crate) focus_lost_listeners: SubscriberSet<(), AnyObserver>,
    focus_lost_path: SmallVec<[FocusId; 8]>,
    default_prevented: bool,
    mouse_position: Point<Pixels>,
    pointer_inside_window: bool,
    mouse_hit_test: HitTest,
    modifiers: Modifiers,
    capslock: Capslock,
    scale_factor: f32,
    pub(crate) bounds_observers: SubscriberSet<(), AnyObserver>,
    appearance: WindowAppearance,
    pub(crate) appearance_observers: SubscriberSet<(), AnyObserver>,
    pub(crate) button_layout_observers: SubscriberSet<(), AnyObserver>,
    active: Rc<Cell<bool>>,
    hovered: Rc<Cell<bool>>,
    pub(crate) needs_present: Rc<Cell<bool>>,
    /// Tracks recent input event timestamps to determine if input is arriving at a high rate.
    /// Used to selectively enable VRR optimization only when input rate exceeds 60fps.
    pub(crate) input_rate_tracker: Rc<RefCell<InputRateTracker>>,
    #[cfg(feature = "profiler")]
    window_profiler: profiler::WindowProfiler,
    last_input_modality: InputModality,
    keyboard_focus_visible: bool,
    touch_gestures: TouchGestureRecognizer,
    touch_prediction_enabled: bool,
    long_press_timer: Option<Task<()>>,
    long_press_capture: Option<EntityId>,
    pub(crate) refreshing: bool,
    pub(crate) activation_observers: SubscriberSet<(), AnyObserver>,
    pub(crate) focus: Option<FocusId>,
    focus_enabled: bool,
    /// Incremented every time focus moves. Used to invalidate a
    /// pending keyboard activation state when focus changes.
    pub(crate) focus_generation: u64,
    pending_input: Option<PendingInput>,
    pending_modifier: ModifierState,
    pub(crate) pending_input_observers: SubscriberSet<(), AnyObserver>,
    prompt: Option<RenderablePromptHandle>,
    pub(crate) client_inset: Option<Pixels>,
    /// The hitbox that has captured the pointer, if any.
    /// While captured, mouse events route to this hitbox regardless of hit testing.
    captured_hitbox: Option<HitboxId>,
    #[cfg(any(feature = "inspector", debug_assertions))]
    inspector: Option<Entity<Inspector>>,
    #[cfg(feature = "profiler")]
    debug_frame_overlay: crate::debug_overlay::DebugFrameOverlay,
    pub(crate) a11y: A11y,
}

#[derive(Clone, Debug, Default)]
struct ModifierState {
    modifiers: Modifiers,
    saw_other_input: bool,
}

/// Tracks input event timestamps to determine if input is arriving at a high rate.
/// Used for selective VRR (Variable Refresh Rate) optimization.
#[derive(Clone, Debug)]
pub(crate) struct InputRateTracker {
    timestamps: Vec<Instant>,
    window: Duration,
    inputs_per_second: u32,
    sustain_until: Instant,
    sustain_duration: Duration,
}

impl Default for InputRateTracker {
    fn default() -> Self {
        Self {
            timestamps: Vec::new(),
            window: Duration::from_millis(100),
            inputs_per_second: 60,
            sustain_until: Instant::now(),
            sustain_duration: Duration::from_secs(1),
        }
    }
}

impl InputRateTracker {
    pub fn record_input(&mut self) {
        let now = Instant::now();
        self.timestamps.push(now);
        self.prune_old_timestamps(now);

        let min_events = self.inputs_per_second as u128 * self.window.as_millis() / 1000;
        if self.timestamps.len() as u128 >= min_events {
            self.sustain_until = now + self.sustain_duration;
        }
    }

    pub fn is_high_rate(&self) -> bool {
        Instant::now() < self.sustain_until
    }

    fn prune_old_timestamps(&mut self, now: Instant) {
        self.timestamps
            .retain(|&t| now.duration_since(t) <= self.window);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DrawPhase {
    None,
    Prepaint,
    Paint,
    Focus,
}

#[derive(Default, Debug)]
struct PendingInput {
    keystrokes: SmallVec<[Keystroke; 1]>,
    focus: Option<FocusId>,
    timer: Option<Task<()>>,
    needs_timeout: bool,
}

pub(crate) struct ElementStateBox {
    pub(crate) inner: Box<dyn Any>,
    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

fn default_bounds(display_id: Option<DisplayId>, cx: &mut App) -> WindowBounds {
    // TODO, BUG: if you open a window with the currently active window
    // on the stack, this will erroneously fallback to `None`
    //
    // TODO these should be the initial window bounds not considering maximized/fullscreen
    let active_window_bounds = cx
        .active_window()
        .and_then(|w| w.update(cx, |_, window, _| window.window_bounds()).ok());

    const CASCADE_OFFSET: f32 = 25.0;

    let display = display_id
        .map(|id| cx.find_display(id))
        .unwrap_or_else(|| cx.primary_display());

    let default_placement = || Bounds::new(point(px(0.), px(0.)), DEFAULT_WINDOW_SIZE);

    // Use visible_bounds to exclude taskbar/dock areas
    let display_bounds = display
        .as_ref()
        .map(|d| d.visible_bounds())
        .unwrap_or_else(default_placement);

    let (
        Bounds {
            origin: base_origin,
            size: base_size,
        },
        window_bounds_ctor,
    ): (_, fn(Bounds<Pixels>) -> WindowBounds) = match active_window_bounds {
        Some(bounds) => match bounds {
            WindowBounds::Windowed(bounds) => (bounds, WindowBounds::Windowed),
            WindowBounds::Maximized(bounds) => (bounds, WindowBounds::Maximized),
            WindowBounds::Fullscreen(bounds) => (bounds, WindowBounds::Fullscreen),
        },
        None => (
            display
                .as_ref()
                .map(|d| d.default_bounds())
                .unwrap_or_else(default_placement),
            WindowBounds::Windowed,
        ),
    };

    let cascade_offset = point(px(CASCADE_OFFSET), px(CASCADE_OFFSET));
    let proposed_origin = base_origin + cascade_offset;
    let proposed_bounds = Bounds::new(proposed_origin, base_size);

    let display_right = display_bounds.origin.x + display_bounds.size.width;
    let display_bottom = display_bounds.origin.y + display_bounds.size.height;
    let window_right = proposed_bounds.origin.x + proposed_bounds.size.width;
    let window_bottom = proposed_bounds.origin.y + proposed_bounds.size.height;

    let fits_horizontally = window_right <= display_right;
    let fits_vertically = window_bottom <= display_bottom;

    let final_origin = match (fits_horizontally, fits_vertically) {
        (true, true) => proposed_origin,
        (false, true) => point(display_bounds.origin.x, base_origin.y),
        (true, false) => point(base_origin.x, display_bounds.origin.y),
        (false, false) => display_bounds.origin,
    };
    window_bounds_ctor(Bounds::new(final_origin, base_size))
}

impl Window {
    pub(crate) fn new(
        handle: AnyWindowHandle,
        options: WindowOptions,
        cx: &mut App,
    ) -> Result<Self> {
        let background_appearance = options.background_appearance();
        let WindowOptions {
            window_bounds,
            titlebar,
            focus,
            show,
            kind,
            is_movable,
            app_owns_titlebar_drag,
            inactive_frame_interval,
            is_resizable,
            is_minimizable,
            display_id,
            app_id,
            window_min_size,
            client_side_frame,
            window_decorations,
            #[cfg_attr(
                not(any(target_os = "linux", target_os = "freebsd")),
                allow(unused_variables)
            )]
            icon,
            #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
            tabbing_identifier,
            ..
        } = options;

        let initial_window_title = titlebar
            .as_ref()
            .and_then(|titlebar| titlebar.title.clone());

        let window_bounds = window_bounds.unwrap_or_else(|| default_bounds(display_id, cx));
        let mut platform_window = cx.platform.open_window(
            handle,
            WindowParams {
                bounds: window_bounds.get_bounds(),
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
                client_side_frame,
                app_id: app_id.clone(),
                icon,
                #[cfg(target_os = "macos")]
                tabbing_identifier,
            },
        )?;

        let tab_bar_visible = platform_window.tab_bar_visible();
        SystemWindowTabController::init_visible(cx, tab_bar_visible);
        if let Some(tabs) = platform_window.tabbed_windows() {
            SystemWindowTabController::add_tab(cx, handle.window_id(), tabs);
        }

        let display_id = platform_window.display().map(|display| display.id());
        let sprite_atlas = platform_window.sprite_atlas();
        let mouse_position = platform_window.mouse_position();
        let modifiers = platform_window.modifiers();
        let capslock = platform_window.capslock();
        let content_size = platform_window.content_size();
        let scale_factor = platform_window.scale_factor();
        let appearance = platform_window.appearance();
        let text_system = Arc::new(WindowTextSystem::new(cx.text_system().clone()));
        let invalidator = WindowInvalidator::new(handle.window_id());
        let active = Rc::new(Cell::new(platform_window.is_active()));
        let hovered = Rc::new(Cell::new(platform_window.is_hovered()));
        let needs_present = Rc::new(Cell::new(false));
        let next_frame_callbacks: Rc<RefCell<Vec<FrameCallback>>> = Default::default();
        let input_rate_tracker = Rc::new(RefCell::new(InputRateTracker::default()));
        let last_frame_time = Rc::new(Cell::new(None));

        platform_window
            .request_decorations(window_decorations.unwrap_or(WindowDecorations::Server));
        platform_window.set_background_appearance(background_appearance);

        match window_bounds {
            WindowBounds::Fullscreen(_) => platform_window.toggle_fullscreen(),
            WindowBounds::Maximized(_) => platform_window.zoom(),
            WindowBounds::Windowed(_) => {}
        }

        let accessibility_force_disabled = cx.accessibility_force_disabled;
        let accessibility_forced = cx.accessibility_forced;
        let a11y_active_flag = Arc::new(AtomicBool::new(false));

        #[cfg(not(target_family = "wasm"))]
        if !accessibility_force_disabled {
            let mut initial_root_node = accesskit::Node::new(accesskit::Role::Window);
            if let Some(title) = &initial_window_title {
                initial_root_node.set_label(title.to_string());
            }
            let initial_tree = accesskit::TreeUpdate {
                nodes: vec![(ROOT_NODE_ID, initial_root_node)],
                tree: Some(accesskit::TreeInfo::new(ROOT_NODE_ID)),
                tree_id: accesskit::TreeId::ROOT,
                focus: ROOT_NODE_ID,
            };
            let (activation_sender, activation_receiver) = async_channel::unbounded::<()>();
            let (deactivation_sender, deactivation_receiver) = async_channel::unbounded::<()>();
            let (action_sender, action_receiver) =
                async_channel::unbounded::<accesskit::ActionRequest>();

            platform_window.a11y_init(crate::A11yCallbacks {
                activation: {
                    let active_flag = a11y_active_flag.clone();
                    Box::new(move || {
                        log::info!("Accessibility activated");
                        active_flag.store(true, SeqCst);
                        activation_sender.send_blocking(()).log_err();
                        Some(initial_tree.clone())
                    })
                },
                action: Box::new(move |request| {
                    action_sender.send_blocking(request).log_err();
                }),
                deactivation: {
                    let active_flag = a11y_active_flag.clone();
                    Box::new(move || {
                        log::info!("Accessibility deactivated");
                        active_flag.store(false, SeqCst);
                        deactivation_sender.send_blocking(()).log_err();
                    })
                },
            });

            // A11y can be activated at any time, and so we cannot compute a
            // correct `TreeUpdate` on-demand. When this happens, we return a
            // default empty `TreeUpdate`.
            //
            // So we force a new frame, which will then send a correct `TreeUpdate`.
            let mut async_cx = cx.to_async();
            cx.foreground_executor()
                .spawn(async move {
                    while activation_receiver.recv().await.is_ok() {
                        handle
                            .update(&mut async_cx, |_, window, _| window.refresh())
                            .log_err();
                    }
                })
                .detach();

            let mut async_cx = cx.to_async();
            cx.foreground_executor()
                .spawn(async move {
                    while deactivation_receiver.recv().await.is_ok() {
                        handle
                            .update(&mut async_cx, |_, window, _| window.refresh())
                            .log_err();
                    }
                })
                .detach();

            let mut async_cx = cx.to_async();
            cx.foreground_executor()
                .spawn(async move {
                    while let Ok(request) = action_receiver.recv().await {
                        handle
                            .update(&mut async_cx, |_, window, cx| {
                                window.handle_a11y_action(request, cx);
                            })
                            .log_err();
                    }
                })
                .detach();
        }

        #[cfg(all(
            feature = "hot-patching",
            debug_assertions,
            not(target_family = "wasm")
        ))]
        let (hot_patch_sender, hot_patch_receiver) = async_channel::bounded(1);

        platform_window.on_close(Box::new({
            let window_id = handle.window_id();
            let mut cx = cx.to_async();
            #[cfg(all(
                feature = "hot-patching",
                debug_assertions,
                not(target_family = "wasm")
            ))]
            let hot_patch_sender = hot_patch_sender.clone();
            move || {
                #[cfg(all(
                    feature = "hot-patching",
                    debug_assertions,
                    not(target_family = "wasm")
                ))]
                hot_patch_sender.close();
                let _ = handle.update(&mut cx, |_, window, _| window.remove_window());
                let _ = cx.update(|cx| {
                    SystemWindowTabController::remove_tab(cx, window_id);
                });
            }
        }));
        #[cfg(all(
            feature = "hot-patching",
            debug_assertions,
            not(target_family = "wasm")
        ))]
        {
            // The patch callback runs off the UI thread. Forward it to the
            // foreground executor so refresh can wake demand-driven platforms.
            subsecond::register_handler(Arc::new(move || {
                let _ = hot_patch_sender.try_send(());
            }));
            let mut cx = cx.to_async();
            let foreground_executor = cx.foreground_executor().clone();
            foreground_executor
                .spawn(async move {
                    while hot_patch_receiver.recv().await.is_ok() {
                        if handle
                            .update(&mut cx, |_, window, _| window.refresh())
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
        }
        platform_window.on_request_frame(Box::new({
            let mut cx = cx.to_async();
            let invalidator = invalidator.clone();
            let active = active.clone();
            let needs_present = needs_present.clone();
            let next_frame_callbacks = next_frame_callbacks.clone();
            let input_rate_tracker = input_rate_tracker.clone();
            let mut deferred_force_render = false;
            move |request_frame_options| {
                #[cfg(feature = "profiler")]
                let _foreground_turn = profiler::journal::foreground_turn();
                // This must be checked before anything else: if this request
                // arrived re-entrantly while a draw is on this thread's stack
                // (e.g. via a nested message pump in the Windows window
                // procedure), drawing would nest draws, and even touching the
                // App would panic on its already-mutable borrow. Skip instead;
                // the platform leaves the window invalidated (or re-invalidates
                // it), so a fresh request arrives once the in-progress draw
                // unwinds. Remember force_render so the deferred frame still
                // bypasses the view cache.
                //
                // Returning here skips `complete_frame`, which on Wayland would
                // stall the window's frame callbacks (no `surface.commit()`) —
                // but calling it would hit the App borrow panic above, and this
                // branch is unreachable there in practice: only Windows pumps
                // platform events (and thus requests frames) mid-draw.
                if draw_in_progress() {
                    log::debug!("deferring re-entrant window draw request");
                    deferred_force_render |= request_frame_options.force_render;
                    return;
                }
                // Take the deferred flag first: `||` short-circuits, and leaving
                // the flag set when this request already forces a render would
                // force a second, redundant render on the next frame.
                let force_render =
                    mem::take(&mut deferred_force_render) || request_frame_options.force_render;

                let thermal_state = handle
                    .update(&mut cx, |_, _, cx| cx.thermal_state())
                    .log_err();

                // Throttle frame rate based on conditions:
                // - Thermal pressure (Serious/Critical): cap to ~60fps
                // - Inactive window (not focused): cap to ~30fps to save energy
                let min_frame_interval = if request_frame_options.require_presentation
                    || (!request_frame_options.force_render
                        && next_frame_callbacks.borrow().is_empty())
                {
                    None
                } else if !active.get() && !input_rate_tracker.borrow_mut().is_high_rate() {
                    inactive_frame_interval
                } else if let Some(ThermalState::Critical | ThermalState::Serious) = thermal_state {
                    Some(Duration::from_micros(16667))
                } else {
                    None
                };

                let now = Instant::now();
                if let Some(min_interval) = min_frame_interval {
                    if let Some(last_frame) = last_frame_time.get()
                        && now.duration_since(last_frame) < min_interval
                    {
                        // Don't lose a pending forced render to throttling.
                        deferred_force_render |= force_render;
                        // Deferred by throttling: ask demand-driven platforms to retry.
                        handle
                            .update(&mut cx, |_, window, _| {
                                window.platform_window.schedule_frame();
                            })
                            .log_err();
                        // The demand that entered this branch (a deferred forced
                        // render or pending next-frame callbacks) is still
                        // unserved; platforms that stop requesting frames for
                        // idle windows need a wakeup to deliver the retry.
                        invalidator.wake_platform();
                        return;
                    }
                }
                last_frame_time.set(Some(now));

                let pending_next_frame_callbacks = next_frame_callbacks.take();
                if !pending_next_frame_callbacks.is_empty() {
                    handle
                        .update(&mut cx, |_, window, cx| {
                            for callback in pending_next_frame_callbacks {
                                callback(window, cx);
                            }
                        })
                        .log_err();
                }

                // Keep presenting if input was recently arriving at a high rate (>= 60fps).
                // Once high-rate input is detected, we sustain presentation for 1 second
                // to prevent display underclocking during active input.
                let needs_present = request_frame_options.require_presentation
                    || needs_present.get()
                    || input_rate_tracker.borrow_mut().is_high_rate();

                if invalidator.is_dirty() || force_render {
                    measure("frame duration", || {
                        handle
                            .update(&mut cx, |_, window, cx| {
                                if force_render {
                                    // Bypass cached view reuse so we don't replay stale
                                    // atlas tile references after a GPU device recovery.
                                    window.refresh();
                                }
                                let arena_clear_needed = window.draw(cx);
                                window.present();
                                arena_clear_needed.clear(cx);
                            })
                            .log_err();
                    })
                } else if needs_present {
                    handle
                        .update(&mut cx, |_, window, _| window.present())
                        .log_err();
                }

                handle
                    .update(&mut cx, |_, window, _| {
                        if window.invalidator.is_dirty()
                            || !window.next_frame_callbacks.borrow().is_empty()
                        {
                            window.platform_window.schedule_frame();
                        }
                    })
                    .log_err();

                // Platforms that stop requesting frames for idle windows only
                // deliver another request after a wakeup. If demand remains
                // after this frame (the window was re-invalidated mid-draw, or
                // animations scheduled next-frame callbacks), re-arm the frame
                // source explicitly.
                if invalidator.is_dirty() || !next_frame_callbacks.borrow().is_empty() {
                    invalidator.wake_platform();
                }
            }
        }));
        invalidator.set_platform_waker(platform_window.frame_waker());
        platform_window.on_resize(Box::new({
            let mut cx = cx.to_async();
            move |_, _| {
                handle
                    .update(&mut cx, |_, window, cx| window.bounds_changed(cx))
                    .log_err();
            }
        }));
        platform_window.on_moved(Box::new({
            let mut cx = cx.to_async();
            move || {
                handle
                    .update(&mut cx, |_, window, cx| window.bounds_changed(cx))
                    .log_err();
            }
        }));
        platform_window.on_appearance_changed(Box::new({
            let cx = cx.to_async();
            let foreground_executor = cx.foreground_executor().clone();
            move || {
                let mut cx = cx.clone();
                // Defer the update because changing the AppKit appearance may
                // synchronously invoke this callback while App is already borrowed.
                foreground_executor
                    .spawn(async move {
                        handle
                            .update(&mut cx, |_, window, cx| window.appearance_changed(cx))
                            .log_err();
                    })
                    .detach();
            }
        }));
        platform_window.on_button_layout_changed(Box::new({
            let mut cx = cx.to_async();
            move || {
                handle
                    .update(&mut cx, |_, window, cx| window.button_layout_changed(cx))
                    .log_err();
            }
        }));
        platform_window.on_active_status_change(Box::new({
            let mut cx = cx.to_async();
            move |active| {
                handle
                    .update(&mut cx, |_, window, cx| {
                        window.active.set(active);
                        window.modifiers = window.platform_window.modifiers();
                        window.capslock = window.platform_window.capslock();
                        window
                            .activation_observers
                            .clone()
                            .retain(&(), |callback| callback(window, cx));

                        window.bounds_changed(cx);
                        window.refresh();

                        SystemWindowTabController::update_last_active(cx, window.handle.id);
                    })
                    .log_err();
            }
        }));
        platform_window.on_hover_status_change(Box::new({
            let mut cx = cx.to_async();
            move |active| {
                handle
                    .update(&mut cx, |_, window, _| {
                        window.hovered.set(active);
                        window.pointer_inside_window = active;
                        window.refresh();
                    })
                    .log_err();
            }
        }));
        platform_window.on_input({
            let mut cx = cx.to_async();
            Box::new(move |event| {
                handle
                    .update(&mut cx, |_, window, cx| window.dispatch_event(event, cx))
                    .log_err()
                    .unwrap_or(DispatchEventResult::default())
            })
        });
        platform_window.on_hit_test_window_control({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, window, _cx| {
                        hit_test_window_control_area(
                            window.mouse_hit_test(),
                            &window.rendered_frame.window_control_hitboxes,
                        )
                    })
                    .log_err()
                    .unwrap_or(None)
            })
        });
        platform_window.on_move_tab_to_new_window({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, _window, cx| {
                        SystemWindowTabController::move_tab_to_new_window(cx, handle.window_id());
                    })
                    .log_err();
            })
        });
        platform_window.on_merge_all_windows({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, _window, cx| {
                        SystemWindowTabController::merge_all_windows(cx, handle.window_id());
                    })
                    .log_err();
            })
        });
        platform_window.on_select_next_tab({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, _window, cx| {
                        SystemWindowTabController::select_next_tab(cx, handle.window_id());
                    })
                    .log_err();
            })
        });
        platform_window.on_select_previous_tab({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, _window, cx| {
                        SystemWindowTabController::select_previous_tab(cx, handle.window_id())
                    })
                    .log_err();
            })
        });
        platform_window.on_toggle_tab_bar({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, window, cx| {
                        let tab_bar_visible = window.platform_window.tab_bar_visible();
                        SystemWindowTabController::set_visible(cx, tab_bar_visible);
                    })
                    .log_err();
            })
        });

        if let Some(app_id) = app_id {
            platform_window.set_app_id(&app_id);
        }

        platform_window.map_window().unwrap();

        Ok(Window {
            handle,
            invalidator,
            removed: false,
            platform_window,
            display_id,
            is_resizable,
            is_minimizable,
            sprite_atlas,
            text_system,
            text_rendering_mode: cx.text_rendering_mode.clone(),
            rem_size: px(16.),
            rem_size_override_stack: SmallVec::new(),
            viewport_size: content_size,
            layout_engine: Some(TaffyLayoutEngine::new()),
            collecting_inline: false,
            current_inline_fragments: None,
            root: None,
            element_id_stack: SmallVec::default(),
            text_style_stack: Vec::new(),
            measurement_direction: ResolvedDirection::LeftToRight,
            measurement_unicode_bidi: UnicodeBidi::Normal,
            rendered_entity_stack: Vec::new(),
            element_offset_stack: Vec::new(),
            style_transition_containing_bounds: None,
            content_mask_stack: Vec::new(),
            requested_autoscroll: None,
            last_text_input_configuration: None,
            focused_text_input_active: false,
            rendered_frame: Frame::new(DispatchTree::new(cx.keymap.clone(), cx.actions.clone())),
            next_frame: Frame::new(DispatchTree::new(cx.keymap.clone(), cx.actions.clone())),
            next_frame_callbacks,
            next_hitbox_id: HitboxId(0),
            next_tooltip_id: TooltipId::default(),
            tooltip_bounds: None,
            dirty_views: FxHashSet::default(),
            focus_listeners: SubscriberSet::new(),
            focus_lost_listeners: SubscriberSet::new(),
            focus_lost_path: SmallVec::new(),
            default_prevented: true,
            mouse_position,
            pointer_inside_window: true,
            mouse_hit_test: HitTest::default(),
            modifiers,
            capslock,
            scale_factor,
            bounds_observers: SubscriberSet::new(),
            appearance,
            appearance_observers: SubscriberSet::new(),
            button_layout_observers: SubscriberSet::new(),
            active,
            hovered,
            needs_present,
            input_rate_tracker,
            #[cfg(feature = "profiler")]
            window_profiler: profiler::WindowProfiler::new(handle.window_id())?,
            last_input_modality: InputModality::Mouse,
            keyboard_focus_visible: false,
            touch_gestures: TouchGestureRecognizer::new(
                cx.platform
                    .gestures()
                    .map_or_else(GestureTuning::default, |gestures| gestures.tuning()),
            ),
            touch_prediction_enabled: true,
            long_press_timer: None,
            long_press_capture: None,
            refreshing: false,
            activation_observers: SubscriberSet::new(),
            focus: None,
            focus_enabled: true,
            focus_generation: 0,
            pending_input: None,
            pending_modifier: ModifierState::default(),
            pending_input_observers: SubscriberSet::new(),
            prompt: None,
            client_inset: None,
            image_cache_stack: Vec::new(),
            captured_hitbox: None,
            #[cfg(any(feature = "inspector", debug_assertions))]
            inspector: None,
            #[cfg(feature = "profiler")]
            debug_frame_overlay: crate::debug_overlay::DebugFrameOverlay::new(),
            a11y: A11y::new(
                a11y_active_flag,
                accessibility_force_disabled,
                accessibility_forced,
                initial_window_title,
            ),
        })
    }

    pub(crate) fn new_focus_listener(
        &self,
        value: AnyWindowFocusListener,
    ) -> (Subscription, impl FnOnce() + use<>) {
        self.focus_listeners.insert((), value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(missing_docs)]
pub struct DispatchEventResult {
    pub propagate: bool,
    pub default_prevented: bool,
}

/// Indicates which region of the window is visible. Content falling outside of this mask will not be
/// rendered. Currently, only rectangular content masks are supported, but we give the mask its own type
/// to leave room to support more complex shapes in the future. Edge fades are evaluated in
/// window space against these axis-aligned bounds and are not transform-aware.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct ContentMask<P: Clone + Debug + Default + PartialEq> {
    /// The bounds
    pub bounds: Bounds<P>,
    /// Edge fade distances. A zero distance leaves that edge fully opaque.
    pub fade_out: Edges<P>,
}

impl ContentMask<Pixels> {
    /// Scale the content mask's pixel units by the given scaling factor.
    pub fn scale(&self, factor: f32) -> ContentMask<ScaledPixels> {
        ContentMask {
            bounds: self.bounds.scale(factor),
            fade_out: self.fade_out.scale(factor),
        }
    }

    /// Intersect the content mask with the given content mask.
    ///
    /// For each edge, the mask that actually clips at that edge contributes its
    /// fade; when both masks clip at the same coordinate, the stronger fade wins.
    pub fn intersect(&self, other: &Self) -> Self {
        fn edge_fade(
            a: Pixels,
            b: Pixels,
            a_fade: Pixels,
            b_fade: Pixels,
            a_clips_closer: impl Fn(Pixels, Pixels) -> bool,
        ) -> Pixels {
            if a_clips_closer(a, b) {
                a_fade
            } else if a_clips_closer(b, a) {
                b_fade
            } else {
                a_fade.max(b_fade)
            }
        }

        ContentMask {
            bounds: self.bounds.intersect(&other.bounds),
            fade_out: Edges {
                top: edge_fade(
                    self.bounds.top(),
                    other.bounds.top(),
                    self.fade_out.top,
                    other.fade_out.top,
                    |a, b| a > b,
                ),
                right: edge_fade(
                    self.bounds.right(),
                    other.bounds.right(),
                    self.fade_out.right,
                    other.fade_out.right,
                    |a, b| a < b,
                ),
                bottom: edge_fade(
                    self.bounds.bottom(),
                    other.bounds.bottom(),
                    self.fade_out.bottom,
                    other.fade_out.bottom,
                    |a, b| a < b,
                ),
                left: edge_fade(
                    self.bounds.left(),
                    other.bounds.left(),
                    self.fade_out.left,
                    other.fade_out.left,
                    |a, b| a > b,
                ),
            },
        }
    }
}

#[cfg(test)]
mod content_mask_tests {
    use super::*;

    fn mask(left: f32, width: f32, fade_left: f32) -> ContentMask<Pixels> {
        ContentMask {
            bounds: Bounds::new(point(px(left), px(0.)), size(px(width), px(10.))),
            fade_out: Edges {
                left: px(fade_left),
                ..Default::default()
            },
        }
    }

    #[test]
    fn intersect_keeps_only_the_fade_at_the_visible_edge() {
        let outer = mask(0., 100., 20.);
        let inner = mask(30., 40., 0.);
        assert_eq!(outer.intersect(&inner).fade_out.left, px(0.));

        let child = mask(0., 40., 0.);
        assert_eq!(outer.intersect(&child).fade_out.left, px(20.));
    }
}

impl Window {
    fn mark_view_dirty(&mut self, view_id: EntityId) {
        // Mark ancestor views as dirty. If already in the `dirty_views` set, then all its ancestors
        // should already be dirty.
        for view_id in self
            .rendered_frame
            .dispatch_tree
            .view_path_reversed(view_id)
        {
            if !self.dirty_views.insert(view_id) {
                break;
            }
        }
    }

    /// Registers a callback to be invoked when the window appearance changes.
    pub fn observe_window_appearance(
        &self,
        mut callback: impl FnMut(&mut Window, &mut App) + 'static,
    ) -> Subscription {
        let (subscription, activate) = self.appearance_observers.insert(
            (),
            Box::new(move |window, cx| {
                callback(window, cx);
                true
            }),
        );
        activate();
        subscription
    }

    /// Registers a callback to be invoked when the window button layout changes.
    pub fn observe_button_layout_changed(
        &self,
        mut callback: impl FnMut(&mut Window, &mut App) + 'static,
    ) -> Subscription {
        let (subscription, activate) = self.button_layout_observers.insert(
            (),
            Box::new(move |window, cx| {
                callback(window, cx);
                true
            }),
        );
        activate();
        subscription
    }

    /// Replaces the root entity of the window with a new one.
    pub fn replace_root<E>(
        &mut self,
        cx: &mut App,
        build_view: impl FnOnce(&mut Window, &mut Context<E>) -> E,
    ) -> Entity<E>
    where
        E: 'static + Render,
    {
        let view = cx.new(|cx| build_view(self, cx));
        self.root = Some(view.clone().into());
        self.refresh();
        view
    }

    /// Returns the root entity of the window, if it has one.
    pub fn root<E>(&self) -> Option<Option<Entity<E>>>
    where
        E: 'static + Render,
    {
        self.root
            .as_ref()
            .map(|view| view.clone().downcast::<E>().ok())
    }

    /// Obtain a handle to the window that belongs to this context.
    pub fn window_handle(&self) -> AnyWindowHandle {
        self.handle
    }

    /// Mark the window as dirty, scheduling it to be redrawn on the next frame.
    pub fn refresh(&mut self) {
        if self.invalidator.not_drawing() {
            self.refreshing = true;
            self.invalidator.set_dirty(true);
        }
    }

    /// Close this window.
    pub fn remove_window(&mut self) {
        self.removed = true;
    }

    /// Obtain the currently focused [`FocusHandle`]. If no elements are focused, returns `None`.
    pub fn focused(&self, cx: &App) -> Option<FocusHandle> {
        self.focus
            .and_then(|id| FocusHandle::for_id(id, &cx.focus_handles))
    }

    /// While focus-lost listeners are being dispatched, returns the closest ancestor of the
    /// previously focused element that can still receive focus, making it a suitable target
    /// for focus restoration. Returns `None` at all other times, or when no such ancestor exists.
    pub fn focus_lost_restore_target(&self, cx: &App) -> Option<FocusHandle> {
        let (_leaf, ancestors) = self.focus_lost_path.split_last()?;
        ancestors.iter().rev().find_map(|id| {
            self.rendered_frame.dispatch_tree.focusable_node_id(*id)?;
            FocusHandle::for_id(*id, &cx.focus_handles)
        })
    }

    /// Move focus to the element associated with the given [`FocusHandle`].
    pub fn focus(&mut self, handle: &FocusHandle, cx: &mut App) {
        if !self.focus_enabled || self.focus == Some(handle.id) {
            return;
        }

        self.focus = Some(handle.id);
        self.focus_generation = self.focus_generation.wrapping_add(1);
        self.clear_pending_keystrokes(cx);

        self.refresh();
    }

    /// Remove focus from all elements within this context's window.
    pub fn blur(&mut self, cx: &mut App) {
        self.clear_pending_keystrokes(cx);

        if !self.focus_enabled {
            return;
        }

        if self.focus.is_some() {
            self.focus_generation = self.focus_generation.wrapping_add(1);
        }
        self.focus = None;
        self.refresh();
    }

    /// Blur the window and don't allow anything in it to be focused again.
    pub fn disable_focus(&mut self, cx: &mut App) {
        self.blur(cx);
        self.focus_enabled = false;
    }

    /// Move focus to next tab stop.
    pub fn focus_next(&mut self, cx: &mut App) {
        if !self.focus_enabled {
            return;
        }

        if let Some(handle) = self.rendered_frame.tab_stops.next(self.focus.as_ref()) {
            self.focus(&handle, cx)
        }
    }

    /// Move focus to previous tab stop.
    pub fn focus_prev(&mut self, cx: &mut App) {
        if !self.focus_enabled {
            return;
        }

        if let Some(handle) = self.rendered_frame.tab_stops.prev(self.focus.as_ref()) {
            self.focus(&handle, cx)
        }
    }

    /// Accessor for the text system.
    pub fn text_system(&self) -> &Arc<WindowTextSystem> {
        &self.text_system
    }

    /// The current text style. Which is composed of all the style refinements provided to `with_text_style`.
    pub fn text_style(&self) -> TextStyle {
        let mut style = TextStyle::default();
        for refinement in &self.text_style_stack {
            style.refine(refinement);
        }
        style
    }

    /// Returns the resolved direction of the current measured or prepainted layout node.
    ///
    /// Custom elements can use this when direction changes intrinsic measurement or a detached
    /// sublayout created during prepaint.
    pub fn resolved_direction(&self) -> ResolvedDirection {
        self.measurement_direction
    }

    /// Returns the bidirectional formatting mode of the measured layout node.
    #[doc(hidden)]
    pub fn resolved_unicode_bidi(&self) -> UnicodeBidi {
        self.measurement_unicode_bidi
    }

    /// Check if the platform window is maximized.
    ///
    /// On some platforms (namely Windows) this is different than the bounds being the size of the display
    pub fn is_maximized(&self) -> bool {
        self.platform_window.is_maximized()
    }

    /// request a certain window decoration (Wayland)
    pub fn request_decorations(&self, decorations: WindowDecorations) {
        self.platform_window.request_decorations(decorations);
    }

    /// Set the exclusive zone for a layer-shell surface: how much screen space it
    /// reserves so other surfaces avoid occluding it (e.g. a panel reserving space).
    /// Positive values reserve that distance from the anchored edge, 0 lets the
    /// surface be moved out of others' exclusive zones, and -1 ignores reserved
    /// space and may extend under other surfaces. (Wayland layer-shell windows only)
    pub fn set_exclusive_zone(&self, zone: Pixels) {
        self.platform_window.set_exclusive_zone(zone);
    }

    /// Set which anchored edge a layer-shell surface's exclusive zone applies to.
    /// This is only needed to disambiguate a corner-anchored surface; otherwise the
    /// edge is deduced from the anchor. The edge must be a single edge the surface
    /// is anchored to, or it is ignored. (Wayland layer-shell windows only)
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    pub fn set_exclusive_edge(&self, edge: crate::layer_shell::Anchor) {
        self.platform_window.set_exclusive_edge(edge);
    }

    /// Start an interactive window resize operation if this window is resizable.
    pub fn start_window_resize(&self, edge: ResizeEdge) {
        if self.is_resizable {
            self.platform_window.start_window_resize(edge);
        }
    }

    /// Linux (wayland) only: Set the window's input region, the area that receives pointer
    /// and touch input. Events outside it pass through to whatever is below the window.
    ///
    /// - `Some(rects)` restricts input to the union of `rects`, in window coordinates.
    /// - `Some(&[])` is an empty region, so the window receives no pointer or touch input.
    /// - `None` resets the region to the default, so the whole window receives input again.
    pub fn set_input_region(&self, region: Option<&[Bounds<Pixels>]>) {
        self.platform_window.set_input_region(region);
    }

    /// Return the `WindowBounds` to indicate that how a window should be opened
    /// after it has been closed
    pub fn window_bounds(&self) -> WindowBounds {
        self.platform_window.window_bounds()
    }

    /// Return the `WindowBounds` excluding insets (Wayland and X11)
    pub fn inner_window_bounds(&self) -> WindowBounds {
        self.platform_window.inner_window_bounds()
    }

    /// Dispatch the given action on the currently focused element.
    pub fn dispatch_action(&mut self, action: Box<dyn Action>, cx: &mut App) {
        let focus_id = self.focused(cx).map(|handle| handle.id);

        let window = self.handle;
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    let node_id = window.focus_node_id_in_rendered_frame(focus_id);
                    window.dispatch_action_on_node(node_id, action.as_ref(), cx);
                })
                .log_err();
        })
    }

    pub(crate) fn dispatch_keystroke_observers(
        &mut self,
        event: &dyn Any,
        action: Option<Box<dyn Action>>,
        context_stack: Vec<KeyContext>,
        cx: &mut App,
    ) {
        let Some(key_down_event) = event.downcast_ref::<KeyDownEvent>() else {
            return;
        };

        cx.keystroke_observers.clone().retain(&(), move |callback| {
            (callback)(
                &KeystrokeEvent {
                    keystroke: key_down_event.keystroke.clone(),
                    action: action.as_ref().map(|action| action.boxed_clone()),
                    context_stack: context_stack.clone(),
                },
                self,
                cx,
            )
        });
    }

    pub(crate) fn dispatch_keystroke_interceptors(
        &mut self,
        event: &dyn Any,
        context_stack: Vec<KeyContext>,
        cx: &mut App,
    ) {
        let Some(key_down_event) = event.downcast_ref::<KeyDownEvent>() else {
            return;
        };

        cx.keystroke_interceptors
            .clone()
            .retain(&(), move |callback| {
                (callback)(
                    &KeystrokeEvent {
                        keystroke: key_down_event.keystroke.clone(),
                        action: None,
                        context_stack: context_stack.clone(),
                    },
                    self,
                    cx,
                )
            });
    }

    /// Schedules the given function to be run at the end of the current effect cycle, allowing entities
    /// that are currently on the stack to be returned to the app.
    pub fn defer(&self, cx: &mut App, f: impl FnOnce(&mut Window, &mut App) + 'static) {
        let handle = self.handle;
        cx.defer(move |cx| {
            handle.update(cx, |_, window, cx| f(window, cx)).ok();
        });
    }

    /// Subscribe to events emitted by a entity.
    /// The entity to which you're subscribing must implement the [`EventEmitter`] trait.
    /// The callback will be invoked a handle to the emitting entity, the event, and a window context for the current window.
    pub fn observe<T: 'static>(
        &mut self,
        observed: &Entity<T>,
        cx: &mut App,
        mut on_notify: impl FnMut(Entity<T>, &mut Window, &mut App) + 'static,
    ) -> Subscription {
        let entity_id = observed.entity_id();
        let observed = observed.downgrade();
        let window_handle = self.handle;
        cx.new_observer(
            entity_id,
            Box::new(move |cx| {
                window_handle
                    .update(cx, |_, window, cx| {
                        if let Some(handle) = observed.upgrade() {
                            on_notify(handle, window, cx);
                            true
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false)
            }),
        )
    }

    /// Subscribe to events emitted by a entity.
    /// The entity to which you're subscribing must implement the [`EventEmitter`] trait.
    /// The callback will be invoked a handle to the emitting entity, the event, and a window context for the current window.
    pub fn subscribe<Emitter, Evt>(
        &mut self,
        entity: &Entity<Emitter>,
        cx: &mut App,
        mut on_event: impl FnMut(Entity<Emitter>, &Evt, &mut Window, &mut App) + 'static,
    ) -> Subscription
    where
        Emitter: EventEmitter<Evt>,
        Evt: 'static,
    {
        let entity_id = entity.entity_id();
        let handle = entity.downgrade();
        let window_handle = self.handle;
        cx.new_subscription(
            entity_id,
            (
                TypeId::of::<Evt>(),
                Box::new(move |event, cx| {
                    window_handle
                        .update(cx, |_, window, cx| {
                            if let Some(entity) = handle.upgrade() {
                                let event = event.downcast_ref().expect("invalid event type");
                                on_event(entity, event, window, cx);
                                true
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false)
                }),
            ),
        )
    }

    /// Register a callback to be invoked when the given `Entity` is released.
    pub fn observe_release<T>(
        &self,
        entity: &Entity<T>,
        cx: &mut App,
        mut on_release: impl FnOnce(&mut T, &mut Window, &mut App) + 'static,
    ) -> Subscription
    where
        T: 'static,
    {
        let entity_id = entity.entity_id();
        let window_handle = self.handle;
        let (subscription, activate) = cx.release_listeners.insert(
            entity_id,
            Box::new(move |entity, cx| {
                let entity = entity.downcast_mut().expect("invalid entity type");
                let _ = window_handle.update(cx, |_, window, cx| on_release(entity, window, cx));
            }),
        );
        activate();
        subscription
    }

    /// Creates an [`AsyncWindowContext`], which has a static lifetime and can be held across
    /// await points in async code.
    pub fn to_async(&self, cx: &App) -> AsyncWindowContext {
        AsyncWindowContext::new_context(cx.to_async(), self.handle)
    }

    /// Schedule the given closure to be run directly after the current frame is rendered.
    pub fn on_next_frame(&self, callback: impl FnOnce(&mut Window, &mut App) + 'static) {
        RefCell::borrow_mut(&self.next_frame_callbacks).push(Box::new(callback));
        self.platform_window.schedule_frame();
        // Next-frame callbacks create frame demand without dirtying the
        // window, so the platform's frame source must be woken explicitly.
        self.invalidator.wake_platform();
    }

    /// Schedule a frame to be drawn on the next animation frame.
    ///
    /// This is useful for elements that need to animate continuously, such as a video player or an animated GIF.
    /// It will cause the window to redraw on the next frame, even if no other changes have occurred.
    ///
    /// If called from within a view, it will notify that view on the next frame. Otherwise, it will refresh the entire window.
    ///
    /// Callers driving purely decorative animations (spinners, pulses, and the
    /// like) should prefer [`AnimationExt::with_animation`](crate::AnimationExt::with_animation),
    /// which automatically respects [`App::reduce_motion`]. When using this
    /// method directly for decorative motion, check [`App::reduce_motion`]
    /// and skip the frame request when it is set.
    pub fn request_animation_frame(&self) {
        let entity = self.current_view();
        self.on_next_frame(move |_, cx| cx.notify(entity));
    }

    /// Runs all callbacks scheduled via [`Self::on_next_frame`], returning how many ran.
    ///
    /// Tests have no platform frame loop, so this simulates the delivery of the
    /// next frame.
    #[cfg(any(test, feature = "test-support"))]
    pub fn simulate_next_frame(&mut self, cx: &mut App) -> usize {
        let callbacks = self.next_frame_callbacks.take();
        let count = callbacks.len();
        for callback in callbacks {
            callback(self, cx);
        }
        count
    }

    /// Spawn the future returned by the given closure on the application thread pool.
    /// The closure is provided a handle to the current window and an `AsyncWindowContext` for
    /// use within your future.
    #[track_caller]
    pub fn spawn<AsyncFn, R>(&self, cx: &App, f: AsyncFn) -> Task<R>
    where
        R: 'static,
        AsyncFn: AsyncFnOnce(&mut AsyncWindowContext) -> R + 'static,
    {
        let handle = self.handle;
        cx.spawn(async move |app| {
            let mut async_window_cx = AsyncWindowContext::new_context(app.clone(), handle);
            f(&mut async_window_cx).await
        })
    }

    /// Spawn the future returned by the given closure on the application thread
    /// pool, with the given priority. The closure is provided a handle to the
    /// current window and an `AsyncWindowContext` for use within your future.
    #[track_caller]
    pub fn spawn_with_priority<AsyncFn, R>(
        &self,
        priority: Priority,
        cx: &App,
        f: AsyncFn,
    ) -> Task<R>
    where
        R: 'static,
        AsyncFn: AsyncFnOnce(&mut AsyncWindowContext) -> R + 'static,
    {
        let handle = self.handle;
        cx.spawn_with_priority(priority, async move |app| {
            let mut async_window_cx = AsyncWindowContext::new_context(app.clone(), handle);
            f(&mut async_window_cx).await
        })
    }

    /// Notify the window that its bounds have changed.
    ///
    /// This updates internal state like `viewport_size` and `scale_factor` from
    /// the platform window, then notifies observers. Normally called automatically
    /// by the platform's resize callback, but exposed publicly for test infrastructure.
    pub fn bounds_changed(&mut self, cx: &mut App) {
        self.scale_factor = self.platform_window.scale_factor();
        self.viewport_size = self.platform_window.content_size();
        self.display_id = self.platform_window.display().map(|display| display.id());
        self.mouse_position = self.platform_window.mouse_position();

        self.refresh();

        self.bounds_observers
            .clone()
            .retain(&(), |callback| callback(self, cx));
    }

    /// Returns the bounds of the current window in the global coordinate space, which could span across multiple displays.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.platform_window.bounds()
    }

    /// Renders the current frame's scene to a texture and returns the pixel data as an RGBA image.
    /// This does not present the frame to screen - useful for visual testing where we want
    /// to capture what would be rendered without displaying it or requiring the window to be visible.
    #[cfg(any(test, feature = "test-support"))]
    pub fn render_to_image(&self) -> anyhow::Result<image::RgbaImage> {
        self.platform_window
            .render_to_image(&self.rendered_frame.scene)
    }

    /// Returns the current frame's quad and glyph sprite counts.
    #[cfg(any(test, feature = "test-support"))]
    pub fn rendered_primitive_counts(&self) -> (usize, usize, usize, usize) {
        let scene = &self.rendered_frame.scene;
        (
            scene.quads.len(),
            scene.monochrome_sprites.len(),
            scene.subpixel_sprites.len(),
            scene.polychrome_sprites.len(),
        )
    }

    /// Returns the quads in the most recently rendered frame's scene, so tests can assert on
    /// painted output without rasterizing the frame. Quad bounds are in scaled pixels and are
    /// not clipped; each quad carries the content mask it will be clipped to when drawn. Quads
    /// whose bounds don't intersect their content mask are culled at paint time and won't appear.
    #[cfg(any(test, feature = "test-support"))]
    pub fn painted_quads(&self) -> Vec<Quad> {
        self.rendered_frame.scene.quads.clone()
    }

    /// Set the content size of the window.
    pub fn resize(&mut self, size: Size<Pixels>) {
        self.platform_window.resize(size);
    }

    /// Returns whether or not the window is currently fullscreen
    pub fn is_fullscreen(&self) -> bool {
        self.platform_window.is_fullscreen()
    }

    /// Returns whether the window is currently in simple (borderless) fullscreen,
    /// where it covers the entire screen including the menu bar and notch area.
    /// Always `false` on platforms other than macOS.
    pub fn is_simple_fullscreen(&self) -> bool {
        self.platform_window.is_simple_fullscreen()
    }

    pub(crate) fn appearance_changed(&mut self, cx: &mut App) {
        self.appearance = self.platform_window.appearance();

        self.appearance_observers
            .clone()
            .retain(&(), |callback| callback(self, cx));
    }

    pub(crate) fn button_layout_changed(&mut self, cx: &mut App) {
        self.button_layout_observers
            .clone()
            .retain(&(), |callback| callback(self, cx));
    }

    /// Returns the appearance of the current window.
    pub fn appearance(&self) -> WindowAppearance {
        self.appearance
    }

    /// Returns the size of the drawable area within the window.
    pub fn viewport_size(&self) -> Size<Pixels> {
        self.viewport_size
    }

    /// Returns whether this window is focused by the operating system (receiving key events).
    pub fn is_window_active(&self) -> bool {
        self.active.get()
    }

    /// Returns whether this window is considered to be the window
    /// that currently owns the mouse cursor.
    /// On mac, this is equivalent to `is_window_active`.
    pub fn is_window_hovered(&self) -> bool {
        if cfg!(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd"
        )) {
            self.hovered.get()
        } else {
            self.is_window_active()
        }
    }

    /// Toggle zoom on the window.
    pub fn zoom_window(&self) {
        self.platform_window.zoom();
    }

    /// Opens the native title bar context menu, useful when implementing client side decorations (Wayland and X11)
    pub fn show_window_menu(&self, position: Point<Pixels>) {
        self.platform_window.show_window_menu(position)
    }

    /// Handle window movement for Linux and macOS.
    /// Tells the compositor to take control of window movement (Wayland and X11)
    ///
    /// Events may not be received during a move operation.
    pub fn start_window_move(&self) {
        self.platform_window.start_window_move()
    }

    /// When using client side decorations, set this to the width of the invisible decorations (Wayland and X11)
    pub fn set_client_inset(&mut self, inset: Pixels) {
        self.client_inset = Some(inset);
        self.platform_window.set_client_inset(inset);
    }

    /// Returns the client_inset value by [`Self::set_client_inset`].
    pub fn client_inset(&self) -> Option<Pixels> {
        self.client_inset
    }

    /// Returns whether the title bar window controls need to be rendered by the application (Wayland and X11)
    pub fn window_decorations(&self) -> Decorations {
        self.platform_window.window_decorations()
    }

    /// Returns whether this window is resizable.
    pub fn is_resizable(&self) -> bool {
        self.is_resizable
    }

    /// Returns whether this window is minimizable.
    pub fn is_minimizable(&self) -> bool {
        self.is_minimizable
    }

    /// Returns the controls supported by the platform.
    pub fn window_controls(&self) -> WindowControls {
        self.platform_window.window_controls()
    }

    /// Updates the window's title at the platform level.
    pub fn set_window_title(&mut self, title: &str) {
        self.platform_window.set_title(title);
        self.a11y.set_window_title(title.to_string());
    }

    /// Sets the position of the macOS traffic light buttons.
    #[cfg(target_os = "macos")]
    pub fn set_traffic_light_position(&self, position: Point<Pixels>) {
        self.platform_window.set_traffic_light_position(position);
    }

    /// Sets the application identifier.
    pub fn set_app_id(&mut self, app_id: &str) {
        self.platform_window.set_app_id(app_id);
    }

    /// Sets the window background appearance.
    pub fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        self.platform_window
            .set_background_appearance(background_appearance);
    }

    /// Mark the window as dirty at the platform level.
    pub fn set_window_edited(&mut self, edited: bool) {
        self.platform_window.set_edited(edited);
    }

    /// Set the path of the file this window represents.
    /// On macOS, this sets the window's accessibility document property (AXDocument).
    pub fn set_document_path(&self, path: Option<&std::path::Path>) {
        self.platform_window.set_document_path(path);
    }

    /// Determine the display on which the window is visible.
    pub fn display(&self, cx: &App) -> Option<Rc<dyn PlatformDisplay>> {
        cx.platform
            .displays()
            .into_iter()
            .find(|display| Some(display.id()) == self.display_id)
    }

    /// Show the platform character palette.
    ///
    /// On macOS and Windows, opens the system character or emoji picker. On Linux,
    /// opens an installed desktop picker (Plasma, IBus, GNOME Characters, or
    /// KCharSelect); these pickers may require copying and pasting the selection.
    /// Logs a warning if the platform has no available picker.
    pub fn show_character_palette(&self) {
        self.platform_window.show_character_palette();
    }

    /// The scale factor of the display associated with the window. For example, it could
    /// return 2.0 for a "retina" display, indicating that each logical pixel should actually
    /// be rendered as two pixels on screen.
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Overrides the display scale factor for tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
        self.refresh();
    }

    /// The size of an em for the base font of the application. Adjusting this value allows the
    /// UI to scale, just like zooming a web page.
    pub fn rem_size(&self) -> Pixels {
        self.rem_size_override_stack
            .last()
            .copied()
            .unwrap_or(self.rem_size)
    }

    /// Sets the size of an em for the base font of the application. Adjusting this value allows the
    /// UI to scale, just like zooming a web page.
    pub fn set_rem_size(&mut self, rem_size: impl Into<Pixels>) {
        self.rem_size = rem_size.into();
    }

    /// Acquire a globally unique identifier for the given ElementId.
    /// Only valid for the duration of the provided closure.
    pub fn with_global_id<R>(
        &mut self,
        element_id: ElementId,
        f: impl FnOnce(&GlobalElementId, &mut Self) -> R,
    ) -> R {
        self.with_id(element_id, |this| {
            let global_id = GlobalElementId(Arc::from(&*this.element_id_stack));

            f(&global_id, this)
        })
    }

    /// Calls the provided closure with the element ID pushed on the stack.
    #[inline]
    pub fn with_id<R>(
        &mut self,
        element_id: impl Into<ElementId>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.element_id_stack.push(element_id.into());
        let result = f(self);
        self.element_id_stack.pop();
        result
    }

    /// Executes the provided function with the specified rem size.
    ///
    /// This method must only be called as part of element drawing.
    // This function is called in a highly recursive manner in editor
    // prepainting, make sure its inlined to reduce the stack burden
    #[inline]
    pub fn with_rem_size<F, R>(&mut self, rem_size: Option<impl Into<Pixels>>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.invalidator.debug_assert_paint_or_prepaint();

        if let Some(rem_size) = rem_size {
            self.rem_size_override_stack.push(rem_size.into());
            let result = f(self);
            self.rem_size_override_stack.pop();
            result
        } else {
            f(self)
        }
    }

    /// The line height associated with the current text style.
    pub fn line_height(&self) -> Pixels {
        self.text_style().line_height_in_pixels(self.rem_size())
    }

    /// Rounds a logical value to the nearest device pixel.
    #[inline]
    pub fn pixel_snap(&self, value: Pixels) -> Pixels {
        px(round_to_device_pixel(value.0, self.scale_factor()) / self.scale_factor())
    }

    /// f64 variant of [`Self::pixel_snap`].
    #[inline]
    pub fn pixel_snap_f64(&self, value: f64) -> f64 {
        let scale_factor = f64::from(self.scale_factor());
        round_half_toward_zero_f64(value * scale_factor) / scale_factor
    }

    /// Snaps a bounds' origin and size to the nearest device pixel.
    #[inline]
    pub fn pixel_snap_bounds(&self, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        bounds.map(|c| self.pixel_snap(c))
    }

    /// Snaps a point's coordinates to the nearest device pixel.
    #[inline]
    pub fn pixel_snap_point(&self, position: Point<Pixels>) -> Point<Pixels> {
        position.map(|c| self.pixel_snap(c))
    }

    #[inline]
    fn snap_bounds(&self, bounds: Bounds<Pixels>) -> Bounds<ScaledPixels> {
        let scale_factor = self.scale_factor();
        let left = round_to_device_pixel(bounds.left().0, scale_factor);
        let top = round_to_device_pixel(bounds.top().0, scale_factor);
        let right = round_to_device_pixel(bounds.right().0, scale_factor).max(left);
        let bottom = round_to_device_pixel(bounds.bottom().0, scale_factor).max(top);
        Bounds::from_corners(
            point(ScaledPixels(left), ScaledPixels(top)),
            point(ScaledPixels(right), ScaledPixels(bottom)),
        )
    }

    /// Rounds half-to-zero but clamps any non-zero input up to 1 dp so thin strokes do not disappear.
    #[inline]
    fn snap_stroke(&self, value: Pixels) -> ScaledPixels {
        ScaledPixels(round_stroke_to_device_pixel(value.0, self.scale_factor()))
    }

    #[inline]
    fn snap_border_widths(&self, edges: Edges<Pixels>) -> Edges<ScaledPixels> {
        edges.map(|e| self.snap_stroke(*e))
    }

    /// Floors the near edge and ceils the far edge, producing a strict superset of the raw region.
    #[inline]
    fn cover_bounds(&self, bounds: Bounds<Pixels>) -> Bounds<ScaledPixels> {
        let scale_factor = self.scale_factor();
        let left = floor_to_device_pixel(bounds.left().0, scale_factor);
        let top = floor_to_device_pixel(bounds.top().0, scale_factor);
        let right = ceil_to_device_pixel(bounds.right().0, scale_factor).max(left);
        let bottom = ceil_to_device_pixel(bounds.bottom().0, scale_factor).max(top);
        Bounds::from_corners(
            point(ScaledPixels(left), ScaledPixels(top)),
            point(ScaledPixels(right), ScaledPixels(bottom)),
        )
    }

    #[inline]
    fn snapped_content_mask(&self) -> ContentMask<ScaledPixels> {
        let mask = self.content_mask();
        ContentMask {
            bounds: self.cover_bounds(mask.bounds),
            fade_out: mask.fade_out.scale(self.scale_factor()),
        }
    }

    /// Call to prevent the default action of an event. Currently only used to prevent
    /// parent elements from becoming focused on mouse down.
    pub fn prevent_default(&mut self) {
        self.default_prevented = true;
    }

    /// Obtain whether default has been prevented for the event currently being dispatched.
    pub fn default_prevented(&self) -> bool {
        self.default_prevented
    }

    /// Determine whether the given action is available along the dispatch path to the currently focused element.
    pub fn is_action_available(&self, action: &dyn Action, cx: &App) -> bool {
        let node_id =
            self.focus_node_id_in_rendered_frame(self.focused(cx).map(|handle| handle.id));
        self.rendered_frame
            .dispatch_tree
            .is_action_available(action, node_id)
    }

    /// Determine whether the given action is available along the dispatch path to the given focus_handle.
    pub fn is_action_available_in(&self, action: &dyn Action, focus_handle: &FocusHandle) -> bool {
        let node_id = self.focus_node_id_in_rendered_frame(Some(focus_handle.id));
        self.rendered_frame
            .dispatch_tree
            .is_action_available(action, node_id)
    }

    /// The position of the mouse relative to the window.
    pub fn mouse_position(&self) -> Point<Pixels> {
        self.mouse_position
    }

    /// Captures the pointer for the given hitbox. While captured, all mouse move and mouse up
    /// events will be routed to listeners that check this hitbox's `is_hovered` status,
    /// regardless of actual hit testing. This enables drag operations that continue
    /// even when the pointer moves outside the element's bounds.
    ///
    /// The capture is automatically released on mouse up.
    pub fn capture_pointer(&mut self, hitbox_id: HitboxId) {
        self.captured_hitbox = Some(hitbox_id);
    }

    /// Releases any active pointer capture.
    pub fn release_pointer(&mut self) {
        self.captured_hitbox = None;
    }

    /// Returns the hitbox that has captured the pointer, if any.
    pub fn captured_hitbox(&self) -> Option<HitboxId> {
        self.captured_hitbox
    }

    /// Captures the current long press for the given entity.
    ///
    /// The capture is released when the gesture ends or is cancelled, or when
    /// a replacement touch begins. A listener must also call
    /// [`Self::prevent_default`] on the started event to claim the gesture.
    pub fn capture_long_press<T: 'static>(&mut self, entity: &Entity<T>) {
        self.long_press_capture = Some(entity.entity_id());
    }

    /// Returns whether the given entity has captured the current long press.
    pub fn has_long_press_capture<T: 'static>(&self, entity: &Entity<T>) -> bool {
        self.long_press_capture == Some(entity.entity_id())
    }

    /// The current state of the keyboard's modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Returns true if the last input event was keyboard-based (key press, tab navigation, etc.)
    /// This is used for focus-visible styling to show focus indicators only for keyboard navigation.
    pub fn last_input_was_keyboard(&self) -> bool {
        self.keyboard_focus_visible
    }

    pub(crate) fn hover_suppressed_by_keyboard(&self) -> bool {
        self.last_input_modality == InputModality::Keyboard
    }

    pub(crate) fn last_input_was_touch(&self) -> bool {
        self.last_input_modality == InputModality::Touch
    }

    /// The current state of the keyboard's capslock
    pub fn capslock(&self) -> Capslock {
        self.capslock
    }

    /// Produces a new frame and assigns it to `rendered_frame`. To actually show
    /// the contents of the new [`Scene`], use [`Self::present`].
    #[profiling::function]
    pub fn draw(&mut self, cx: &mut App) -> ArenaClearNeeded {
        // Drain every draw in profiler builds so a previous frame's
        // first-invalidation timestamp can't be attributed to this one.
        #[cfg(feature = "profiler")]
        let frame_dirty = self.invalidator.take_frame_dirty();
        #[cfg(feature = "profiler")]
        self.window_profiler.begin_draw();

        // Set up the per-App arena for element allocation during this draw.
        // This ensures that multiple test Apps have isolated arenas.
        let arena_scope = ElementArenaScope::enter(&cx.element_arena);

        self.invalidate_entities();
        cx.entities.clear_accessed();
        debug_assert!(self.rendered_entity_stack.is_empty());
        self.invalidator.set_dirty(false);
        self.requested_autoscroll = None;

        // Restore the previously-used input handler.
        // Place it back into a None slot (left by a previous .take()) so that
        // cached paint_range indices in reuse_paint find the handler at the
        // expected position.
        if let Some(input_handler) = self.platform_window.take_input_handler() {
            if let Some(slot) = self
                .rendered_frame
                .input_handlers
                .iter_mut()
                .rev()
                .find(|h| h.is_none())
            {
                *slot = Some(input_handler);
            } else {
                self.rendered_frame.input_handlers.push(Some(input_handler));
            }
        }
        if !cx.mode.skip_drawing() {
            self.draw_roots(cx);
            #[cfg(feature = "profiler")]
            {
                let viewport_size = self.viewport_size;
                let scale_factor = self.scale_factor();
                self.debug_frame_overlay.paint(
                    &mut self.next_frame.scene,
                    viewport_size,
                    scale_factor,
                );
            }
        }
        self.dirty_views.clear();
        self.next_frame.window_active = self.active.get();

        // Register requested input handler with the platform window.
        // Use .take() instead of .pop() to preserve Vec length, so that cached
        // paint_range indices remain valid for reuse_paint on the next frame.
        // Search backwards to find the last Some entry, since reuse_paint may
        // have copied None slots from the previous frame. (Fixes #50456)
        let focused_text_input_active = if let Some(mut input_handler) = self
            .next_frame
            .input_handlers
            .iter_mut()
            .rev()
            .find_map(|h| h.take())
        {
            let accepts_text_input = input_handler.accepts_text_input(self, cx);
            self.platform_window.set_input_handler(input_handler);
            accepts_text_input
        } else {
            false
        };
        self.apply_text_input_configuration(cx);
        if focused_text_input_active != self.focused_text_input_active {
            self.focused_text_input_active = focused_text_input_active;
            self.platform_window
                .text_input_state_changed(if focused_text_input_active {
                    TextInputStateChange::FocusGained
                } else {
                    TextInputStateChange::FocusLost
                });
        }

        self.layout_engine.as_mut().unwrap().clear();
        self.text_system().finish_frame();
        self.next_frame.finish(&mut self.rendered_frame);

        self.invalidator.set_phase(DrawPhase::Focus);
        let previous_focus_path = self.rendered_frame.focus_path();
        let previous_window_active = self.rendered_frame.window_active;
        mem::swap(&mut self.rendered_frame, &mut self.next_frame);
        self.next_frame.clear();
        let current_focus_path = self.rendered_frame.focus_path();
        let current_window_active = self.rendered_frame.window_active;
        let mut focus_before_listeners = self.focus;

        if previous_focus_path != current_focus_path
            || previous_window_active != current_window_active
        {
            if !previous_focus_path.is_empty() && current_focus_path.is_empty() {
                self.focus_lost_path = previous_focus_path.clone();
                self.focus_lost_listeners
                    .clone()
                    .retain(&(), |listener| listener(self, cx));
                self.focus_lost_path = SmallVec::new();
                // The focus-lost fallback (e.g. a workspace refocusing itself) may target
                // an element that isn't part of the element tree, in which case scheduling
                // a redraw below would dispatch focus-lost again, looping forever. Only
                // track focus movement caused by the focus listeners.
                focus_before_listeners = self.focus;
            }

            let event = WindowFocusEvent {
                previous_focus_path: if previous_window_active {
                    previous_focus_path
                } else {
                    Default::default()
                },
                current_focus_path: if current_window_active {
                    current_focus_path
                } else {
                    Default::default()
                },
            };
            self.focus_listeners
                .clone()
                .retain(&(), |listener| listener(&event, self, cx));
        }

        debug_assert!(self.rendered_entity_stack.is_empty());
        self.record_entities_accessed(cx);
        self.reset_cursor_style(cx);
        self.refreshing = false;
        self.invalidator.set_phase(DrawPhase::None);
        self.notify_pointer_observers(PointerChangeCause::Layout, cx);
        // Focus listeners may move focus (e.g. a dock forwarding focus to its active
        // panel). `Window::focus` suppresses `refresh` while a draw is in progress, so
        // schedule another frame here to render the new focus state and dispatch the
        // resulting focus events.
        if self.focus != focus_before_listeners {
            self.refresh();
        }
        self.needs_present.set(true);

        #[cfg(feature = "profiler")]
        {
            let draw_duration = self
                .window_profiler
                .end_draw(frame_dirty.dirty_at, frame_dirty.invalidations);
            self.debug_frame_overlay.record_frame(draw_duration);
        }

        // Exit the scope to obtain the arena-clear token this draw owes; the
        // scope's teardown itself happens in `ElementArenaScope::drop`.
        arena_scope.exit(&cx.element_arena)
    }

    fn record_entities_accessed(&mut self, cx: &mut App) {
        let mut entities_ref = cx.entities.accessed_entities.get_mut();
        let mut entities = mem::take(entities_ref.deref_mut());
        let handle = self.handle;
        cx.record_entities_accessed(
            handle,
            // Try moving window invalidator into the Window
            self.invalidator.clone(),
            &entities,
        );
        let mut entities_ref = cx.entities.accessed_entities.get_mut();
        mem::swap(&mut entities, entities_ref.deref_mut());
    }

    fn invalidate_entities(&mut self) {
        let mut views = self.invalidator.take_views();
        for entity in views.drain() {
            self.mark_view_dirty(entity);
        }
        self.invalidator.replace_views(views);
    }

    #[profiling::function]
    fn present(&mut self) {
        #[cfg(feature = "profiler")]
        let _foreground_turn = profiler::journal::foreground_turn();
        #[cfg(feature = "profiler")]
        let present_start = Instant::now();
        self.platform_window.draw(&self.rendered_frame.scene);
        #[cfg(feature = "profiler")]
        self.window_profiler.record_present(
            present_start,
            Instant::now(),
            self.active.get(),
            !self.next_frame_callbacks.borrow().is_empty(),
        );
        self.needs_present.set(false);
        profiling::finish_frame!();
    }

    /// Presents the most recently drawn frame if it hasn't been presented yet.
    ///
    /// Benchmarks drive drawing synchronously rather than through a platform
    /// frame-request loop, so they call this after each measured update to
    /// submit the frame like production presentation would.
    #[cfg(any(feature = "bench-support", all(test, feature = "profiler")))]
    pub fn present_if_needed(&mut self) {
        if self.needs_present.get() {
            self.present();
        }
    }

    /// Returns a snapshot of the current input-latency histograms.
    #[cfg(feature = "profiler")]
    pub fn input_latency_snapshot(&self) -> profiler::InputLatencySnapshot {
        self.window_profiler.input_latency_snapshot()
    }

    /// Returns a snapshot of the current frame-duration histograms.
    #[cfg(feature = "profiler")]
    pub fn frame_duration_snapshot(&self) -> profiler::FrameDurationSnapshot {
        self.window_profiler.frame_duration_snapshot()
    }

    /// Returns the current mode of the debug frame overlay.
    #[cfg(feature = "profiler")]
    pub fn debug_frame_overlay_mode(&self) -> DebugFrameOverlayMode {
        self.debug_frame_overlay.mode()
    }

    /// Sets the mode of the debug frame overlay and schedules a redraw.
    #[cfg(feature = "profiler")]
    pub fn set_debug_frame_overlay_mode(&mut self, mode: DebugFrameOverlayMode) {
        self.debug_frame_overlay.set_mode(mode);
        self.refresh();
    }

    /// Advances the debug frame overlay through its hidden, frame-time-only,
    /// and detailed modes.
    #[cfg(feature = "profiler")]
    pub fn cycle_debug_frame_overlay_mode(&mut self) {
        self.set_debug_frame_overlay_mode(self.debug_frame_overlay.mode().next());
    }

    /// Clears the debug frame overlay's frame-time statistics, except for the
    /// total frame count, and schedules a redraw.
    #[cfg(feature = "profiler")]
    pub fn reset_debug_frame_overlay_stats(&mut self) {
        self.debug_frame_overlay.reset_stats();
        self.refresh();
    }

    fn draw_roots(&mut self, cx: &mut App) {
        self.invalidator.set_phase(DrawPhase::Prepaint);
        self.tooltip_bounds.take();

        self.a11y.sync_active_flag();
        if self.a11y.is_active() {
            self.a11y.begin_frame();
        }

        let _inspector_width: Pixels = rems(30.0).to_pixels(self.rem_size());
        let root_size = {
            #[cfg(any(feature = "inspector", debug_assertions))]
            {
                if self.inspector.is_some() {
                    let mut size = self.viewport_size;
                    size.width = (size.width - _inspector_width).max(px(0.0));
                    size
                } else {
                    self.viewport_size
                }
            }
            #[cfg(not(any(feature = "inspector", debug_assertions)))]
            {
                self.viewport_size
            }
        };

        // Layout all root elements. Like the root element on the web, which
        // stretches to fill the viewport unless explicitly sized, window roots
        // fill the window when their size is `auto`.
        let scale_factor = self.scale_factor();
        let mut root_element = self.root.as_ref().unwrap().clone().into_any_element();
        let root_layout_id = root_element.request_layout(self, cx);
        self.layout_engine
            .as_mut()
            .unwrap()
            .stretch_auto_size_to_fill(root_layout_id, root_size, scale_factor);
        root_element.prepaint_as_root(Point::default(), root_size.into(), self, cx);

        #[cfg(any(feature = "inspector", debug_assertions))]
        let inspector_element = self.prepaint_inspector(_inspector_width, cx);

        self.prepaint_deferred_draws(cx);

        let mut prompt_element = None;
        let mut active_drag_element = None;
        let mut tooltip_element = None;
        if let Some(prompt) = self.prompt.take() {
            let mut element = prompt.view.any_view().into_any_element();
            let prompt_layout_id = element.request_layout(self, cx);
            self.layout_engine
                .as_mut()
                .unwrap()
                .stretch_auto_size_to_fill(prompt_layout_id, root_size, scale_factor);
            element.prepaint_as_root(Point::default(), root_size.into(), self, cx);
            prompt_element = Some(element);
            self.prompt = Some(prompt);
        } else if let Some(active_drag) = cx.active_drag.take() {
            let mut element = active_drag.view.clone().into_any_element();
            let offset = self.mouse_position() - active_drag.cursor_offset;
            element.prepaint_as_root(offset, AvailableSpace::min_size(), self, cx);
            active_drag_element = Some(element);
            cx.active_drag = Some(active_drag);
        } else {
            tooltip_element = self.prepaint_tooltip(cx);
        }

        self.mouse_hit_test = self.next_frame.hit_test(self.mouse_position);

        // Now actually paint the elements.
        self.invalidator.set_phase(DrawPhase::Paint);
        root_element.paint(self, cx);

        #[cfg(any(feature = "inspector", debug_assertions))]
        self.paint_inspector(inspector_element, cx);

        self.paint_deferred_draws(cx);

        if let Some(mut prompt_element) = prompt_element {
            prompt_element.paint(self, cx);
        } else if let Some(mut drag_element) = active_drag_element {
            drag_element.paint(self, cx);
        } else if let Some(mut tooltip_element) = tooltip_element {
            tooltip_element.paint(self, cx);
        }

        #[cfg(any(feature = "inspector", debug_assertions))]
        self.paint_inspector_hitbox(cx);

        // a11y may have been activated/deactivated halfway through the frame
        let a11y_active_start_of_frame = self.a11y.is_active();
        self.a11y.sync_active_flag();
        let a11y_active_end_of_frame = self.a11y.is_active();

        let should_send_a11y_update = a11y_active_start_of_frame && a11y_active_end_of_frame;

        if a11y_active_start_of_frame {
            // Harvest frame metadata for the debug dump while the live window
            // and frame are still in scope.
            let frame_info = crate::window::a11y::debug::FrameDebugInfo {
                viewport_size: self.viewport_size,
                scale_factor: self.scale_factor,
                tab_stop_count: self.next_frame.tab_stops.tab_stop_count(),
            };
            // clear the builder state regardless
            let tree_update = self.a11y.end_frame(frame_info);

            if should_send_a11y_update {
                log::debug!(
                    "Sending a11y tree update: {} nodes",
                    tree_update.nodes.len()
                );
                self.platform_window.a11y_tree_update(tree_update);
            }
        }
    }

    fn prepaint_tooltip(&mut self, cx: &mut App) -> Option<AnyElement> {
        // Use indexing instead of iteration to avoid borrowing self for the duration of the loop.
        for tooltip_request_index in (0..self.next_frame.tooltip_requests.len()).rev() {
            let Some(Some(tooltip_request)) = self
                .next_frame
                .tooltip_requests
                .get(tooltip_request_index)
                .cloned()
            else {
                log::error!("Unexpectedly absent TooltipRequest");
                continue;
            };
            let mut element = tooltip_request.tooltip.view.clone().into_any_element();
            let mouse_position = tooltip_request.tooltip.mouse_position;
            let tooltip_size = element.layout_as_root(AvailableSpace::min_size(), self, cx);

            let mut tooltip_bounds =
                Bounds::new(mouse_position + point(px(1.), px(1.)), tooltip_size);
            let window_bounds = Bounds {
                origin: Point::default(),
                size: self.viewport_size(),
            };

            if tooltip_bounds.right() > window_bounds.right() {
                let new_x = mouse_position.x - tooltip_bounds.size.width - px(1.);
                if new_x >= Pixels::ZERO {
                    tooltip_bounds.origin.x = new_x;
                } else {
                    tooltip_bounds.origin.x = cmp::max(
                        Pixels::ZERO,
                        tooltip_bounds.origin.x - tooltip_bounds.right() - window_bounds.right(),
                    );
                }
            }

            if tooltip_bounds.bottom() > window_bounds.bottom() {
                let new_y = mouse_position.y - tooltip_bounds.size.height - px(1.);
                if new_y >= Pixels::ZERO {
                    tooltip_bounds.origin.y = new_y;
                } else {
                    tooltip_bounds.origin.y = cmp::max(
                        Pixels::ZERO,
                        tooltip_bounds.origin.y - tooltip_bounds.bottom() - window_bounds.bottom(),
                    );
                }
            }

            // It's possible for an element to have an active tooltip while not being painted (e.g.
            // via the `visible_on_hover` method). Since mouse listeners are not active in this
            // case, instead update the tooltip's visibility here.
            let is_visible =
                (tooltip_request.tooltip.check_visible_and_update)(tooltip_bounds, self, cx);
            if !is_visible {
                continue;
            }

            self.with_absolute_element_offset(tooltip_bounds.origin, |window| {
                element.prepaint(window, cx)
            });

            self.tooltip_bounds = Some(TooltipBounds {
                id: tooltip_request.id,
                bounds: tooltip_bounds,
            });
            return Some(element);
        }
        None
    }

    fn prepaint_deferred_draws(&mut self, cx: &mut App) {
        assert_eq!(self.element_id_stack.len(), 0);

        // Process deferred draws in multiple rounds to support nesting.
        // Each round processes all current deferred draws, which may push new ones.
        //
        // The draws are processed in place rather than being moved out of
        // `next_frame.deferred_draws`: `prepaint_index` snapshots that vector's
        // length, so any prepaint range recorded during a round (view caches,
        // nested deferred draws) must index the same vector `reuse_prepaint`
        // slices on the next frame. Moving the draws out and re-appending them
        // shifts the indices of nested draws, causing reused subtrees to graft
        // the wrong deferred draws and panic in the dispatch tree.
        let mut round_start = 0;
        let mut depth = 0;
        loop {
            let round_end = self.next_frame.deferred_draws.len();
            if round_start == round_end {
                break;
            }
            // Limit maximum nesting depth to prevent infinite loops.
            assert!(depth < 10, "Exceeded maximum (10) deferred depth");
            depth += 1;

            // Sort this round by priority.
            let mut traversal_order = (round_start..round_end).collect::<SmallVec<[usize; 8]>>();
            traversal_order.sort_by_key(|ix| self.next_frame.deferred_draws[*ix].priority);

            for deferred_draw_ix in traversal_order {
                let (
                    priority,
                    element,
                    parent_node,
                    current_view,
                    rem_size,
                    absolute_offset,
                    prepaint_range,
                ) = {
                    let deferred_draw = &mut self.next_frame.deferred_draws[deferred_draw_ix];
                    self.element_id_stack
                        .clone_from(&deferred_draw.element_id_stack);
                    self.text_style_stack
                        .clone_from(&deferred_draw.text_style_stack);
                    (
                        deferred_draw.priority,
                        deferred_draw.element.take(),
                        deferred_draw.parent_node,
                        deferred_draw.current_view,
                        deferred_draw.rem_size,
                        deferred_draw.absolute_offset,
                        deferred_draw.prepaint_range.clone(),
                    )
                };
                self.next_frame.dispatch_tree.set_active_node(parent_node);

                let prepaint_start = self.prepaint_index();
                if let Some(mut element) = element {
                    self.with_rendered_view(current_view, |window| {
                        window.with_rem_size(Some(rem_size), |window| {
                            window.with_absolute_element_offset(absolute_offset, |window| {
                                crate::DeferredPriorityStackCache::push(priority, cx);
                                element.prepaint(window, cx);
                                crate::DeferredPriorityStackCache::pop(cx);
                            });
                        });
                    });
                    self.next_frame.deferred_draws[deferred_draw_ix].element = Some(element);
                } else {
                    self.reuse_prepaint(prepaint_range);
                }
                let prepaint_end = self.prepaint_index();
                self.next_frame.deferred_draws[deferred_draw_ix].prepaint_range =
                    prepaint_start..prepaint_end;
            }

            self.element_id_stack.clear();
            self.text_style_stack.clear();
            round_start = round_end;
        }
    }

    fn paint_deferred_draws(&mut self, cx: &mut App) {
        assert_eq!(self.element_id_stack.len(), 0);

        // Paint all deferred draws in priority order.
        // Since prepaint has already processed nested deferreds, we just paint them all.
        if self.next_frame.deferred_draws.len() == 0 {
            return;
        }

        // Deferred draws are overlays (tooltips, popovers, drag images) and must sort above the
        // whole main scene. Raise the order floor so they do — this also keeps a deferred
        // backdrop's order from falling inside a content-filter order range left by the main scene.
        self.next_frame.scene.raise_order_floor();

        let traversal_order = self.deferred_draw_traversal_order();
        let mut deferred_draws = mem::take(&mut self.next_frame.deferred_draws);
        for deferred_draw_ix in traversal_order {
            let mut deferred_draw = &mut deferred_draws[deferred_draw_ix];
            self.element_id_stack
                .clone_from(&deferred_draw.element_id_stack);
            self.next_frame
                .dispatch_tree
                .set_active_node(deferred_draw.parent_node);

            let paint_start = self.paint_index();
            let content_mask = deferred_draw.content_mask;
            if let Some(element) = deferred_draw.element.as_mut() {
                self.with_rendered_view(deferred_draw.current_view, |window| {
                    window.with_content_mask(content_mask, |window| {
                        window.with_rem_size(Some(deferred_draw.rem_size), |window| {
                            element.paint(window, cx);
                        });
                    })
                })
            } else {
                self.reuse_paint(deferred_draw.paint_range.clone());
            }
            let paint_end = self.paint_index();
            deferred_draw.paint_range = paint_start..paint_end;
        }
        self.next_frame.deferred_draws = deferred_draws;
        self.element_id_stack.clear();
    }

    fn deferred_draw_traversal_order(&mut self) -> SmallVec<[usize; 8]> {
        let deferred_count = self.next_frame.deferred_draws.len();
        let mut sorted_indices = (0..deferred_count).collect::<SmallVec<[_; 8]>>();
        sorted_indices.sort_by_key(|ix| self.next_frame.deferred_draws[*ix].priority);
        sorted_indices
    }

    pub(crate) fn prepaint_index(&self) -> PrepaintStateIndex {
        PrepaintStateIndex {
            hitboxes_index: self.next_frame.hitboxes.len(),
            tooltips_index: self.next_frame.tooltip_requests.len(),
            deferred_draws_index: self.next_frame.deferred_draws.len(),
            dispatch_tree_index: self.next_frame.dispatch_tree.len(),
            accessed_element_states_index: self.next_frame.accessed_element_states.len(),
            line_layout_index: self.text_system.layout_index(),
        }
    }

    pub(crate) fn reuse_prepaint(&mut self, range: Range<PrepaintStateIndex>) {
        self.next_frame.hitboxes.extend(
            self.rendered_frame.hitboxes[range.start.hitboxes_index..range.end.hitboxes_index]
                .iter()
                .cloned(),
        );
        self.next_frame.tooltip_requests.extend(
            self.rendered_frame.tooltip_requests
                [range.start.tooltips_index..range.end.tooltips_index]
                .iter_mut()
                .map(|request| request.take()),
        );
        self.next_frame.accessed_element_states.extend(
            self.rendered_frame.accessed_element_states[range.start.accessed_element_states_index
                ..range.end.accessed_element_states_index]
                .iter()
                .map(|(id, type_id)| (id.clone(), *type_id)),
        );
        self.text_system
            .reuse_layouts(range.start.line_layout_index..range.end.line_layout_index);

        let reused_subtree = self.next_frame.dispatch_tree.reuse_subtree(
            range.start.dispatch_tree_index..range.end.dispatch_tree_index,
            &mut self.rendered_frame.dispatch_tree,
            self.focus,
        );

        if reused_subtree.contains_focus() {
            self.next_frame.focus = self.focus;
        }

        self.next_frame.deferred_draws.extend(
            self.rendered_frame.deferred_draws
                [range.start.deferred_draws_index..range.end.deferred_draws_index]
                .iter()
                .map(|deferred_draw| DeferredDraw {
                    current_view: deferred_draw.current_view,
                    parent_node: reused_subtree.refresh_node_id(deferred_draw.parent_node),
                    element_id_stack: deferred_draw.element_id_stack.clone(),
                    text_style_stack: deferred_draw.text_style_stack.clone(),
                    content_mask: deferred_draw.content_mask,
                    rem_size: deferred_draw.rem_size,
                    priority: deferred_draw.priority,
                    element: None,
                    absolute_offset: deferred_draw.absolute_offset,
                    prepaint_range: deferred_draw.prepaint_range.clone(),
                    paint_range: deferred_draw.paint_range.clone(),
                }),
        );
    }

    pub(crate) fn paint_index(&self) -> PaintIndex {
        PaintIndex {
            scene_index: self.next_frame.scene.len(),
            mouse_listeners_index: self.next_frame.mouse_listeners.len(),
            pointer_observers_index: self.next_frame.pointer_observers.len(),
            input_handlers_index: self.next_frame.input_handlers.len(),
            cursor_styles_index: self.next_frame.cursor_styles.len(),
            accessed_element_states_index: self.next_frame.accessed_element_states.len(),
            tab_handle_index: self.next_frame.tab_stops.paint_index(),
            line_layout_index: self.text_system.layout_index(),
        }
    }

    pub(crate) fn reuse_paint(&mut self, range: Range<PaintIndex>) {
        Rc::make_mut(&mut self.next_frame.pointer_observers).extend(
            self.rendered_frame.pointer_observers
                [range.start.pointer_observers_index..range.end.pointer_observers_index]
                .iter()
                .cloned(),
        );
        self.next_frame.cursor_styles.extend(
            self.rendered_frame.cursor_styles
                [range.start.cursor_styles_index..range.end.cursor_styles_index]
                .iter()
                .cloned(),
        );
        self.next_frame.input_handlers.extend(
            self.rendered_frame.input_handlers
                [range.start.input_handlers_index..range.end.input_handlers_index]
                .iter_mut()
                .map(|handler| handler.take()),
        );
        self.next_frame.mouse_listeners.extend(
            self.rendered_frame.mouse_listeners
                [range.start.mouse_listeners_index..range.end.mouse_listeners_index]
                .iter_mut()
                .map(|listener| listener.take()),
        );
        self.next_frame.accessed_element_states.extend(
            self.rendered_frame.accessed_element_states[range.start.accessed_element_states_index
                ..range.end.accessed_element_states_index]
                .iter()
                .map(|(id, type_id)| (id.clone(), *type_id)),
        );
        self.next_frame.tab_stops.replay(
            &self.rendered_frame.tab_stops.insertion_history
                [range.start.tab_handle_index..range.end.tab_handle_index],
        );

        self.text_system
            .reuse_layouts(range.start.line_layout_index..range.end.line_layout_index);
        self.next_frame.scene.replay(
            range.start.scene_index..range.end.scene_index,
            &self.rendered_frame.scene,
        );
    }

    /// Push a text style onto the stack, and call a function with that style active.
    /// Use [`Window::text_style`] to get the current, combined text style. This method
    /// should only be called as part of element drawing.
    pub fn with_text_style<F, R>(&mut self, style: Option<TextStyleRefinement>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.invalidator.debug_assert_paint_or_prepaint();
        if let Some(style) = style {
            self.text_style_stack.push(style);
            let result = f(self);
            self.text_style_stack.pop();
            result
        } else {
            f(self)
        }
    }

    /// Updates the cursor style at the platform level. This method should only be called
    /// during the paint phase of element drawing.
    pub fn set_cursor_style(&mut self, style: CursorStyle, hitbox: &Hitbox) {
        self.invalidator.debug_assert_paint();
        self.next_frame
            .cursor_styles
            .push(CursorStyleRequest::Hitbox {
                id: hitbox.id,
                style: CursorStyleSource::Fixed(style),
            });
    }

    /// Resolves this hitbox's cursor from window coordinates on pointer input.
    /// Return `None` to use the next cursor request. Cached paints retain the resolver.
    pub fn set_cursor_style_with(
        &mut self,
        hitbox: &Hitbox,
        resolve: impl Fn(Point<Pixels>) -> Option<CursorStyle> + 'static,
    ) {
        self.invalidator.debug_assert_paint();
        self.next_frame
            .cursor_styles
            .push(CursorStyleRequest::Hitbox {
                id: hitbox.id,
                style: CursorStyleSource::Position(Rc::new(resolve)),
            });
    }

    /// Updates the cursor style for the entire window at the platform level. A cursor
    /// style using this method will have precedence over any cursor style set using
    /// `set_cursor_style`. This method should only be called during the paint
    /// phase of element drawing.
    pub fn set_window_cursor_style(&mut self, style: CursorStyle) {
        self.invalidator.debug_assert_paint();
        self.next_frame
            .cursor_styles
            .push(CursorStyleRequest::Window(CursorStyleSource::Fixed(style)));
    }

    /// Resolves a window-wide cursor from pointer position.
    /// Return `None` to defer to hitbox cursor requests.
    pub fn set_window_cursor_style_with(
        &mut self,
        resolve: impl Fn(Point<Pixels>) -> Option<CursorStyle> + 'static,
    ) {
        self.invalidator.debug_assert_paint();
        self.next_frame
            .cursor_styles
            .push(CursorStyleRequest::Window(CursorStyleSource::Position(
                Rc::new(resolve),
            )));
    }

    /// Sets a tooltip to be rendered for the upcoming frame. This method should only be called
    /// during the paint phase of element drawing.
    pub fn set_tooltip(&mut self, tooltip: AnyTooltip) -> TooltipId {
        self.invalidator.debug_assert_prepaint();
        let id = TooltipId(post_inc(&mut self.next_tooltip_id.0));
        self.next_frame
            .tooltip_requests
            .push(Some(TooltipRequest { id, tooltip }));
        id
    }

    /// Invoke the given function with the given content mask after intersecting it
    /// with the current mask. This method should only be called during element drawing.
    // This function is called in a highly recursive manner in editor
    // prepainting, make sure its inlined to reduce the stack burden
    #[inline]
    pub fn with_content_mask<R>(
        &mut self,
        mask: Option<ContentMask<Pixels>>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.invalidator.debug_assert_paint_or_prepaint();
        if let Some(mask) = mask {
            let mask = mask.intersect(&self.content_mask());
            self.content_mask_stack.push(mask);
            let result = f(self);
            self.content_mask_stack.pop();
            result
        } else {
            f(self)
        }
    }

    /// Updates the global element offset relative to the current offset. This is used to implement
    /// scrolling. This method should only be called during the prepaint phase of element drawing.
    pub fn with_element_offset<R>(
        &mut self,
        offset: Point<Pixels>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.invalidator.debug_assert_prepaint();

        if offset.is_zero() {
            return f(self);
        };

        let abs_offset = self.element_offset() + offset;
        self.with_absolute_element_offset(abs_offset, f)
    }

    pub(crate) fn with_style_transition_containing_bounds<R>(
        &mut self,
        bounds: Bounds<Pixels>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.invalidator.debug_assert_prepaint();
        let previous_bounds = self.style_transition_containing_bounds.replace(bounds);
        let result = f(self);
        self.style_transition_containing_bounds = previous_bounds;
        result
    }

    pub(crate) fn style_transition_containing_bounds(&self) -> Option<Bounds<Pixels>> {
        self.style_transition_containing_bounds
    }

    /// Updates the global element offset based on the given offset. This is used to implement
    /// drag handles and other manual painting of elements. This method should only be called during
    /// the prepaint phase of element drawing.
    pub fn with_absolute_element_offset<R>(
        &mut self,
        offset: Point<Pixels>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.invalidator.debug_assert_prepaint();
        self.element_offset_stack.push(offset);
        let result = f(self);
        self.element_offset_stack.pop();
        result
    }

    pub(crate) fn with_element_opacity<R>(
        &mut self,
        opacity: Option<f32>,
        paint: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.invalidator.debug_assert_paint();
        let opacity = opacity.unwrap_or(1.0).clamp(0.0, 1.0);

        if opacity == 1.0 {
            return paint(self);
        }

        // The existing targets cover the viewport, preserving shadows and visible overflow.
        // Child paint operations already carry their clipping and edge fades.
        let bounds = self.cover_bounds(Bounds::new(Point::default(), self.viewport_size()));
        let boundary = FilterBoundary {
            order: 0,
            bounds,
            content_mask: ContentMask {
                bounds,
                fade_out: Edges::default(),
            },
            corner_radii: Corners::default(),
            corner_smoothing: 0.0,
            filters: SmallVec::new(),
            opacity,
            is_start: true,
        };

        self.with_filter_boundary(boundary, paint)
    }

    fn with_filter_boundary<R>(
        &mut self,
        boundary: FilterBoundary,
        paint: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.next_frame.scene.insert_primitive(boundary.clone());
        let result = paint(self);
        self.next_frame.scene.insert_primitive(FilterBoundary {
            is_start: false,
            ..boundary
        });

        result
    }

    /// Perform prepaint on child elements in a "retryable" manner, so that any side effects
    /// of prepaints can be discarded before prepainting again. This is used to support autoscroll
    /// where we need to prepaint children to detect the autoscroll bounds, then adjust the
    /// element offset and prepaint again. See [`crate::List`] for an example. This method should only be
    /// called during the prepaint phase of element drawing.
    pub fn transact<T, U>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, U>) -> Result<T, U> {
        self.invalidator.debug_assert_prepaint();
        let index = self.prepaint_index();
        let result = f(self);
        if result.is_err() {
            self.next_frame.hitboxes.truncate(index.hitboxes_index);
            self.next_frame
                .tooltip_requests
                .truncate(index.tooltips_index);
            self.next_frame
                .deferred_draws
                .truncate(index.deferred_draws_index);
            self.next_frame
                .dispatch_tree
                .truncate(index.dispatch_tree_index);
            self.next_frame
                .accessed_element_states
                .truncate(index.accessed_element_states_index);
            self.text_system.truncate_layouts(index.line_layout_index);
        }
        result
    }

    /// When you call this method during [`Element::prepaint`], containing elements will attempt to
    /// scroll to cause the specified bounds to become visible. When they decide to autoscroll, they will call
    /// [`Element::prepaint`] again with a new set of bounds. See [`crate::List`] for an example of an element
    /// that supports this method being called on the elements it contains. This method should only be
    /// called during the prepaint phase of element drawing.
    pub fn request_autoscroll(&mut self, bounds: Bounds<Pixels>) {
        self.invalidator.debug_assert_prepaint();
        self.requested_autoscroll = Some(bounds);
    }

    /// This method can be called from a containing element such as [`crate::List`] to support the autoscroll behavior
    /// described in [`Self::request_autoscroll`].
    pub fn take_autoscroll(&mut self) -> Option<Bounds<Pixels>> {
        self.invalidator.debug_assert_prepaint();
        self.requested_autoscroll.take()
    }

    /// Asynchronously load an asset, if the asset hasn't finished loading this will return None.
    /// Your view will be re-drawn once the asset has finished loading.
    ///
    /// Note that the multiple calls to this method will only result in one `Asset::load` call at a
    /// time.
    pub fn use_asset<A: Asset>(&mut self, source: &A::Source, cx: &mut App) -> Option<A::Output> {
        let (task, is_first) = cx.fetch_asset::<A>(source);
        task.clone().now_or_never().or_else(|| {
            if is_first {
                let entity_id = self.current_view();
                self.spawn(cx, {
                    let task = task.clone();
                    async move |cx| {
                        task.await;

                        cx.on_next_frame(move |_, cx| {
                            cx.notify(entity_id);
                        });
                    }
                })
                .detach();
            }

            None
        })
    }

    /// Asynchronously load an asset, if the asset hasn't finished loading or doesn't exist this will return None.
    /// Your view will not be re-drawn once the asset has finished loading.
    ///
    /// Note that the multiple calls to this method will only result in one `Asset::load` call at a
    /// time.
    pub fn get_asset<A: Asset>(&mut self, source: &A::Source, cx: &mut App) -> Option<A::Output> {
        let (task, _) = cx.fetch_asset::<A>(source);
        task.now_or_never()
    }
    /// Obtain the current element offset. This method should only be called during the
    /// prepaint phase of element drawing.
    pub fn element_offset(&self) -> Point<Pixels> {
        self.invalidator.debug_assert_prepaint();
        self.element_offset_stack
            .last()
            .copied()
            .unwrap_or_default()
    }

    /// Obtain the current content mask. This method should only be called during element drawing.
    pub fn content_mask(&self) -> ContentMask<Pixels> {
        self.invalidator.debug_assert_paint_or_prepaint();
        self.content_mask_stack
            .last()
            .cloned()
            .unwrap_or_else(|| ContentMask {
                bounds: Bounds {
                    origin: Point::default(),
                    size: self.viewport_size,
                },
                ..Default::default()
            })
    }

    /// Provide elements in the called function with a new namespace in which their identifiers must be unique.
    /// This can be used within a custom element to distinguish multiple sets of child elements.
    pub fn with_element_namespace<R>(
        &mut self,
        element_id: impl Into<ElementId>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.element_id_stack.push(element_id.into());
        let result = f(self);
        self.element_id_stack.pop();
        result
    }

    /// Use a piece of state that exists as long this element is being rendered in consecutive frames.
    ///
    /// This method should only be called during `Render::render`, `RenderOnce::render`, or the drawing functions of `Element`.
    pub fn use_keyed_state<S: 'static>(
        &mut self,
        key: impl Into<ElementId>,
        cx: &mut App,
        init: impl FnOnce(&mut Self, &mut Context<S>) -> S,
    ) -> Entity<S> {
        let current_view = self.current_view();
        self.with_global_id(key.into(), |global_id, window| {
            window.with_element_state(global_id, |state: Option<Entity<S>>, window| {
                if let Some(state) = state {
                    (state.clone(), state)
                } else {
                    let new_state = cx.new(|cx| init(window, cx));
                    cx.observe(&new_state, move |_, cx| {
                        cx.notify(current_view);
                    })
                    .detach();
                    (new_state.clone(), new_state)
                }
            })
        })
    }

    /// Use a piece of state that exists as long this element is being rendered in consecutive frames, without needing to specify a key
    ///
    /// NOTE: This method uses the location of the caller to generate an ID for this state.
    ///       If this is not sufficient to identify your state (e.g. you're rendering a list item),
    ///       you can provide a custom ElementID using the `use_keyed_state` method.
    #[track_caller]
    pub fn use_state<S: 'static>(
        &mut self,
        cx: &mut App,
        init: impl FnOnce(&mut Self, &mut Context<S>) -> S,
    ) -> Entity<S> {
        self.use_keyed_state(
            ElementId::CodeLocation(*core::panic::Location::caller()),
            cx,
            init,
        )
    }

    /// Updates or initializes state for an element with the given id that lives across multiple
    /// frames. If an element with this ID existed in the rendered frame, its state will be passed
    /// to the given closure. The state returned by the closure will be stored so it can be referenced
    /// when drawing the next frame.
    ///
    /// This method should only be called during `Render::render`, `RenderOnce::render`, or the drawing functions of `Element`.
    pub fn with_element_state<S, R>(
        &mut self,
        global_id: &GlobalElementId,
        f: impl FnOnce(Option<S>, &mut Self) -> (R, S),
    ) -> R
    where
        S: 'static,
    {
        self.invalidator.debug_assert_paint_or_prepaint();

        let key = (global_id.clone(), TypeId::of::<S>());
        self.next_frame.accessed_element_states.push(key.clone());

        if let Some(any) = self
            .next_frame
            .element_states
            .remove(&key)
            .or_else(|| self.rendered_frame.element_states.remove(&key))
        {
            let ElementStateBox {
                inner,
                #[cfg(debug_assertions)]
                type_name,
            } = any;
            // Using the extra inner option to avoid needing to reallocate a new box.
            let mut state_box = inner
                .downcast::<Option<S>>()
                .map_err(|_| {
                    #[cfg(debug_assertions)]
                    {
                        anyhow::anyhow!(
                            "invalid element state type for id, requested {:?}, actual: {:?}",
                            std::any::type_name::<S>(),
                            type_name
                        )
                    }

                    #[cfg(not(debug_assertions))]
                    {
                        anyhow::anyhow!(
                            "invalid element state type for id, requested {:?}",
                            std::any::type_name::<S>(),
                        )
                    }
                })
                .unwrap();

            let state = state_box.take().expect(
                "reentrant call to with_element_state for the same state type and element id",
            );
            let (result, state) = f(Some(state), self);
            state_box.replace(state);
            self.next_frame.element_states.insert(
                key,
                ElementStateBox {
                    inner: state_box,
                    #[cfg(debug_assertions)]
                    type_name,
                },
            );
            result
        } else {
            let (result, state) = f(None, self);
            self.next_frame.element_states.insert(
                key,
                ElementStateBox {
                    inner: Box::new(Some(state)),
                    #[cfg(debug_assertions)]
                    type_name: std::any::type_name::<S>(),
                },
            );
            result
        }
    }

    /// A variant of `with_element_state` that allows the element's id to be optional. This is a convenience
    /// method for elements where the element id may or may not be assigned. Prefer using `with_element_state`
    /// when the element is guaranteed to have an id.
    ///
    /// The first option means 'no ID provided'
    /// The second option means 'not yet initialized'
    pub fn with_optional_element_state<S, R>(
        &mut self,
        global_id: Option<&GlobalElementId>,
        f: impl FnOnce(Option<Option<S>>, &mut Self) -> (R, Option<S>),
    ) -> R
    where
        S: 'static,
    {
        self.invalidator.debug_assert_paint_or_prepaint();

        if let Some(global_id) = global_id {
            self.with_element_state(global_id, |state, cx| {
                let (result, state) = f(Some(state), cx);
                let state =
                    state.expect("you must return some state when you pass some element id");
                (result, state)
            })
        } else {
            let (result, state) = f(None, self);
            debug_assert!(
                state.is_none(),
                "you must not return an element state when passing None for the global id"
            );
            result
        }
    }

    /// Creates a new transition with automatic state management.
    ///
    /// The state for this transition is managed internally and will be recreated
    /// on each render. For persistent state across renders, use [`use_keyed_transition`](Self::use_keyed_transition).
    pub fn use_transition<T: Lerp + Clone + PartialEq + 'static>(
        &mut self,
        cx: &mut App,
        motion: impl Into<Motion>,
        init: impl Fn(&mut Window, &mut Context<TransitionState<T>>) -> T,
    ) -> Transition<T> {
        let motion = motion.into();
        let state = self.use_state(cx, |window, cx| {
            TransitionState::new(init(window, cx), motion.clone())
        });

        Transition::new(state, motion)
    }

    /// Creates a new keyed transition with persistent state.
    ///
    /// The state for this transition is associated with the provided key and will
    /// persist across renders as long as the key remains the same. This is the
    /// recommended method for most use cases where you want smooth, continuous
    /// animations.
    ///
    /// This method should only be called during `Render::render`, `RenderOnce::render`, or the drawing functions of `Element`.
    pub fn use_keyed_transition<T: Lerp + Clone + PartialEq + 'static>(
        &mut self,
        key: impl Into<ElementId>,
        cx: &mut App,
        motion: impl Into<Motion>,
        init: impl Fn(&mut Window, &mut Context<TransitionState<T>>) -> T,
    ) -> Transition<T> {
        let motion = motion.into();
        let state = self.use_keyed_state(key, cx, |window, cx| {
            TransitionState::new(init(window, cx), motion.clone())
        });

        Transition::new(state, motion)
    }

    /// Executes the given closure within the context of a tab group.
    #[inline]
    pub fn with_tab_group<R>(&mut self, index: Option<isize>, f: impl FnOnce(&mut Self) -> R) -> R {
        if let Some(index) = index {
            self.next_frame.tab_stops.begin_group(index);
            let result = f(self);
            self.next_frame.tab_stops.end_group();
            result
        } else {
            f(self)
        }
    }

    /// Defers the drawing of the given element, scheduling it to be painted on top of the currently-drawn tree
    /// at a later time. The `priority` parameter determines the drawing order relative to other deferred elements,
    /// with higher values being drawn on top.
    ///
    /// When `content_mask` is provided, the deferred element will be clipped to that region during
    /// both prepaint and paint. When `None`, no additional clipping is applied.
    ///
    /// This method should only be called as part of the prepaint phase of element drawing.
    pub fn defer_draw(
        &mut self,
        element: AnyElement,
        absolute_offset: Point<Pixels>,
        priority: usize,
        content_mask: Option<ContentMask<Pixels>>,
    ) {
        self.invalidator.debug_assert_prepaint();
        let parent_node = self.next_frame.dispatch_tree.active_node_id().unwrap();
        self.next_frame.deferred_draws.push(DeferredDraw {
            current_view: self.current_view(),
            parent_node,
            element_id_stack: self.element_id_stack.clone(),
            text_style_stack: self.text_style_stack.clone(),
            content_mask,
            rem_size: self.rem_size(),
            priority,
            element: Some(element),
            absolute_offset,
            prepaint_range: PrepaintStateIndex::default()..PrepaintStateIndex::default(),
            paint_range: PaintIndex::default()..PaintIndex::default(),
        });
    }

    /// Paint a subtree at a uniform scale around a window-space origin, without
    /// changing its layout. Nested scopes and cached paint replay retain the transform.
    /// Call during paint; use `with_prepaint_scale` for the matching pointer geometry.
    pub fn with_paint_scale<R>(
        &mut self,
        origin: Point<Pixels>,
        factor: f32,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.invalidator.debug_assert_paint();
        assert!(factor.is_finite() && factor > 0.0);
        if factor == 1.0 {
            return f(self);
        }
        let scale = crate::scene::PaintScale {
            origin: origin.scale(self.scale_factor()),
            factor,
            outer_mask: self.content_mask().bounds.scale(self.scale_factor()),
        };
        self.next_frame.scene.push_scale(scale);
        let result = f(self);
        self.next_frame.scene.pop_scale();
        result
    }

    /// Match pointer hitboxes to a `with_paint_scale` subtree. Call during prepaint.
    pub fn with_prepaint_scale<R>(
        &mut self,
        origin: Point<Pixels>,
        factor: f32,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.invalidator.debug_assert_prepaint();
        assert!(factor.is_finite() && factor > 0.0);
        if factor == 1.0 {
            return f(self);
        }
        let start = self.next_frame.hitboxes.len();
        let outer_mask = self.content_mask().bounds;
        let result = f(self);
        let transform = |b: Bounds<Pixels>| {
            Bounds::new(
                origin + (b.origin - origin) * factor,
                b.size.map(|v| v * factor),
            )
        };
        for hitbox in &mut self.next_frame.hitboxes[start..] {
            hitbox.bounds = transform(hitbox.bounds);
            if hitbox.content_mask.bounds != outer_mask {
                hitbox.content_mask.bounds =
                    transform(hitbox.content_mask.bounds).intersect(&outer_mask);
            }
            if let Some(fragments) = &mut hitbox.fragments {
                *fragments = fragments.iter().copied().map(transform).collect();
            }
        }
        result
    }

    /// Creates a new painting layer for the specified bounds. A "layer" is a batch
    /// of geometry that are non-overlapping and have the same draw order. This is typically used
    /// for performance reasons.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_layer<R>(&mut self, bounds: Bounds<Pixels>, f: impl FnOnce(&mut Self) -> R) -> R {
        self.invalidator.debug_assert_paint();

        let content_mask = self.content_mask();
        let clipped_bounds = bounds.intersect(&content_mask.bounds);
        if !clipped_bounds.is_empty() {
            self.next_frame
                .scene
                .push_layer(self.cover_bounds(clipped_bounds));
        }

        let result = f(self);

        if !clipped_bounds.is_empty() {
            self.next_frame.scene.pop_layer();
        }

        result
    }

    /// Paint the drop (non-inset) shadows from `shadows` into the scene at the current
    /// z-index. Inset shadows are skipped; paint those with [`Self::paint_inset_shadows`]
    /// after the element's background so they layer on top of the fill.
    /// `corner_smoothing` ranges from circular corners at `0.0` to maximum smoothing at `1.0`.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_drop_shadows(
        &mut self,
        bounds: Bounds<Pixels>,
        corner_radii: Corners<Pixels>,
        corner_smoothing: f32,
        shadows: &[BoxShadow],
    ) {
        self.paint_shadows(bounds, corner_radii, corner_smoothing, shadows, false);
    }

    /// Paint the inset shadows from `shadows` into the scene at the current z-index. Should
    /// be called after the element's background so the shadow layers on top of the fill.
    /// Drop shadows are skipped; paint those with [`Self::paint_drop_shadows`] before the background.
    /// `corner_smoothing` ranges from circular corners at `0.0` to maximum smoothing at `1.0`.
    pub fn paint_inset_shadows(
        &mut self,
        bounds: Bounds<Pixels>,
        corner_radii: Corners<Pixels>,
        corner_smoothing: f32,
        shadows: &[BoxShadow],
    ) {
        self.paint_shadows(bounds, corner_radii, corner_smoothing, shadows, true);
    }

    fn paint_shadows(
        &mut self,
        bounds: Bounds<Pixels>,
        corner_radii: Corners<Pixels>,
        corner_smoothing: f32,
        shadows: &[BoxShadow],
        inset: bool,
    ) {
        self.invalidator.debug_assert_paint();

        let scale_factor = self.scale_factor();
        let content_mask = self.snapped_content_mask();
        let element_bounds = self.cover_bounds(bounds);
        let element_corner_radii = corner_radii.scale(scale_factor);
        let corner_smoothing = corner_smoothing.clamp(0.0, 1.0);

        for shadow in shadows.iter().filter(|shadow| shadow.inset == inset) {
            let spread = if inset {
                -shadow.spread_radius
            } else {
                shadow.spread_radius
            };

            let shadow_bounds = (bounds + shadow.offset).dilate(spread);

            let shadow_radii = if inset {
                // Large inset spreads must not produce negative radii in the shader.
                corner_radii.map(|radius| (*radius - shadow.spread_radius).max(Pixels::ZERO))
            } else {
                corner_radii
            };

            self.next_frame.scene.insert_primitive(Shadow {
                order: 0,
                blur_radius: shadow.blur_radius.scale(scale_factor),
                bounds: self.cover_bounds(shadow_bounds),
                content_mask,
                corner_radii: shadow_radii.scale(scale_factor),
                color: shadow.color,
                element_bounds,
                element_corner_radii,
                inset: inset.into(),
                corner_smoothing,
            });
        }
    }

    /// Paint a backdrop filter into the scene for the next frame at the current z-index. The
    /// renderer blurs the content already painted behind `bounds` and composites the result
    /// into the rounded rectangle described by `bounds` and `corner_radii` — the CSS
    /// `backdrop-filter` effect (frosted glass). Typically the element then paints a translucent
    /// background quad on top so its color tints the blurred backdrop.
    /// `corner_smoothing` ranges from circular corners at `0.0` to maximum smoothing at `1.0`.
    ///
    /// Does nothing when `filters` produce no visible blur.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_backdrop_filter(
        &mut self,
        bounds: Bounds<Pixels>,
        corner_radii: Corners<Pixels>,
        corner_smoothing: f32,
        filters: &[Filter],
    ) {
        self.invalidator.debug_assert_paint();

        let scale_factor = self.scale_factor();
        let filters: SmallVec<[ScaledFilter; 4]> = filters
            .iter()
            .filter(|filter| !filter.is_identity())
            .map(|filter| filter.scale(scale_factor))
            .collect();
        if filters.is_empty() {
            return;
        }

        self.next_frame.scene.insert_primitive(BackdropFilter {
            order: 0,
            bounds: self.snap_bounds(bounds),
            content_mask: self.snapped_content_mask(),
            corner_radii: corner_radii.scale(scale_factor),
            corner_smoothing: corner_smoothing.clamp(0.0, 1.0),
            filters,
            opacity: 1.0,
        });
    }

    /// Isolate the painting performed by `f` into a content-filter group: the renderer renders
    /// everything `f` paints into an offscreen target, blurs it as a single layer, and
    /// composites the result back into the rounded rectangle described by `bounds` and
    /// `corner_radii` — the CSS `filter` effect (e.g. blurring an element and its children).
    /// `corner_smoothing` ranges from circular corners at `0.0` to maximum smoothing at `1.0`.
    ///
    /// When `filters` produce no visible blur this simply runs `f` with no offscreen
    /// indirection.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn with_filter_layer<R>(
        &mut self,
        bounds: Bounds<Pixels>,
        corner_radii: Corners<Pixels>,
        corner_smoothing: f32,
        filters: &[Filter],
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.invalidator.debug_assert_paint();

        let scale_factor = self.scale_factor();
        let filters: SmallVec<[ScaledFilter; 4]> = filters
            .iter()
            .filter(|filter| !filter.is_identity())
            .map(|filter| filter.scale(scale_factor))
            .collect();
        if filters.is_empty() {
            return f(self);
        }

        let boundary = FilterBoundary {
            order: 0,
            bounds: self.snap_bounds(bounds),
            content_mask: self.snapped_content_mask(),
            corner_radii: corner_radii.scale(scale_factor),
            corner_smoothing: corner_smoothing.clamp(0.0, 1.0),
            filters,
            opacity: 1.0,
            is_start: true,
        };

        self.with_filter_boundary(boundary, f)
    }

    fn largest_border_interior(quad: &Quad) -> Bounds<ScaledPixels> {
        let radii = &quad.corner_radii;
        let widths = &quad.border_widths;
        let reach_factor = 1.0 + quad.corner_smoothing;
        let edge_reaches = Edges {
            top: radii.top_left.max(radii.top_right) * reach_factor,
            right: radii.top_right.max(radii.bottom_right) * reach_factor,
            bottom: radii.bottom_left.max(radii.bottom_right) * reach_factor,
            left: radii.top_left.max(radii.bottom_left) * reach_factor,
        };

        let antialias_inset = point(ScaledPixels(1.0), ScaledPixels(1.0));
        let inset_bounds = |top_left_inset, bottom_right_inset| {
            Bounds::from_corners(
                quad.bounds.origin + top_left_inset + antialias_inset,
                quad.bounds.bottom_right() - bottom_right_inset - antialias_inset,
            )
        };

        // Rounded corners need only be excluded on one axis. Either candidate
        // is empty of border pixels, so use the larger interior.
        let horizontal_band = inset_bounds(
            point(widths.left, widths.top.max(edge_reaches.top)),
            point(widths.right, widths.bottom.max(edge_reaches.bottom)),
        );
        let vertical_band = inset_bounds(
            point(widths.left.max(edge_reaches.left), widths.top),
            point(widths.right.max(edge_reaches.right), widths.bottom),
        );

        let area = |bounds: &Bounds<ScaledPixels>| {
            bounds.size.width.0.max(0.) * bounds.size.height.0.max(0.)
        };
        if area(&horizontal_band) >= area(&vertical_band) {
            horizontal_band
        } else {
            vertical_band
        }
    }

    /// Paint one or more quads into the scene for the next frame at the current stacking context.
    /// Quads are colored rectangular regions with an optional background, border, and corner radius.
    /// Use [`PaintQuad::new`] and its builder methods to construct the quad.
    /// `quad.corner_smoothing` ranges from circular corners at `0.0` to maximum smoothing at `1.0`.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    ///
    /// Note that the `quad.corner_radii` are allowed to exceed the bounds, creating sharp corners
    /// where the circular arcs meet. This will not display well when combined with dashed borders.
    /// Use `Corners::clamp_radii_for_quad_size` if the radii should fit within the bounds.
    pub fn paint_quad(&mut self, quad: PaintQuad) {
        self.invalidator.debug_assert_paint();

        let pixel_snap = quad.pixel_snap;
        let snapped_bounds = if pixel_snap {
            self.snap_bounds(quad.bounds)
        } else {
            quad.bounds.scale(self.scale_factor())
        };
        let snapped_border_widths = if pixel_snap {
            self.snap_border_widths(quad.border_widths)
        } else {
            quad.border_widths.scale(self.scale_factor())
        };
        let quad = Quad {
            order: 0,
            bounds: snapped_bounds,
            content_mask: self.snapped_content_mask(),
            background: quad.background,
            border_color: quad.border_color,
            corner_radii: quad.corner_radii.scale(self.scale_factor()),
            border_widths: snapped_border_widths,
            border_style: quad.border_style,
            border_dashed_length: quad.border_dashed_length,
            border_dashed_gap: quad.border_dashed_gap,
            corner_smoothing: quad.corner_smoothing.clamp(0.0, 1.0),
            padding: 0,
        };

        // Fractional strip boundaries would clip the same antialiased pixel in two
        // separate draws, introducing seams. Keep animated outlines in one draw.
        if !pixel_snap || !quad.background.is_transparent() {
            self.next_frame.scene.insert_primitive(quad);
            return;
        }

        // Splitting a border-only quad around its empty interior avoids shading
        // every transparent pixel inside large outlines.
        let outer_bounds = quad.bounds;
        let inner_bounds = Self::largest_border_interior(&quad);

        if inner_bounds.is_empty() {
            self.next_frame.scene.insert_primitive(quad);
            return;
        }

        let strips = [
            // Top
            Bounds::from_corners(
                outer_bounds.origin,
                point(outer_bounds.right(), inner_bounds.top()),
            ),
            // Bottom
            Bounds::from_corners(
                point(outer_bounds.left(), inner_bounds.bottom()),
                outer_bounds.bottom_right(),
            ),
            // Left
            Bounds::from_corners(
                point(outer_bounds.left(), inner_bounds.top()),
                inner_bounds.bottom_left(),
            ),
            // Right
            Bounds::from_corners(
                inner_bounds.top_right(),
                point(outer_bounds.right(), inner_bounds.bottom()),
            ),
        ];

        for strip in strips {
            let content_mask_bounds = quad.content_mask.bounds.intersect(&strip);
            if !content_mask_bounds.is_empty() {
                self.next_frame.scene.insert_primitive(Quad {
                    content_mask: ContentMask {
                        bounds: content_mask_bounds,
                        ..Default::default()
                    },
                    ..quad
                });
            }
        }
    }

    /// Paint the given `Path` into the scene for the next frame at the current z-index.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_path(&mut self, mut path: Path<Pixels>, color: impl Into<Background>) {
        self.invalidator.debug_assert_paint();

        let scale_factor = self.scale_factor();
        let content_mask = self.content_mask();
        path.content_mask = content_mask;
        let color: Background = color.into();
        path.color = color;
        self.next_frame
            .scene
            .insert_primitive(path.scale(scale_factor));
    }

    /// Paint an underline into the scene for the next frame at the current z-index.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_underline(
        &mut self,
        origin: Point<Pixels>,
        width: Pixels,
        style: &UnderlineStyle,
    ) {
        self.invalidator.debug_assert_paint();

        let scale_factor = self.scale_factor();
        let thickness = self.snap_stroke(style.thickness);
        let height = if style.wavy {
            ScaledPixels(thickness.0 * 3.)
        } else {
            thickness
        };
        let bounds = Bounds {
            origin: origin.map(|c| ScaledPixels(round_to_device_pixel(c.0, scale_factor))),
            size: size(self.snap_stroke(width), height),
        };

        self.next_frame.scene.insert_primitive(Underline {
            order: 0,
            padding: 0,
            bounds,
            content_mask: self.snapped_content_mask(),
            color: style.color.unwrap_or_default().into(),
            thickness,
            wavy: style.wavy.into(),
        });
    }

    /// Paint a strikethrough into the scene for the next frame at the current z-index.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_strikethrough(
        &mut self,
        origin: Point<Pixels>,
        width: Pixels,
        style: &StrikethroughStyle,
    ) {
        self.invalidator.debug_assert_paint();

        let scale_factor = self.scale_factor();
        let height = style.thickness;
        let bounds = Bounds {
            origin: origin.map(|c| ScaledPixels(round_to_device_pixel(c.0, scale_factor))),
            size: size(self.snap_stroke(width), self.snap_stroke(height)),
        };

        self.next_frame.scene.insert_primitive(Underline {
            order: 0,
            padding: 0,
            bounds,
            content_mask: self.snapped_content_mask(),
            thickness: self.snap_stroke(style.thickness),
            color: style.color.unwrap_or_default().into(),
            wavy: false.into(),
        });
    }

    /// Paints a monochrome (non-emoji) glyph into the scene for the next frame at the current z-index.
    ///
    /// The y component of the origin is the baseline of the glyph.
    /// You should generally prefer to use the [`ShapedLine::paint`](crate::ShapedLine::paint) or
    /// [`ShapedText::paint`](crate::ShapedText::paint) methods in the [`TextSystem`](crate::TextSystem).
    /// This method is only useful if you need to paint a single glyph that has already been shaped.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_glyph(
        &mut self,
        origin: Point<Pixels>,
        font_id: FontId,
        glyph_id: GlyphId,
        font_size: Pixels,
        color: Hsla,
    ) -> Result<()> {
        self.invalidator.debug_assert_paint();

        let scale_factor = self.scale_factor();
        let glyph_origin = origin.scale(scale_factor);

        let (integer_origin, subpixel_variant) = quantize_glyph_origin(glyph_origin);
        let requested_mode = if self.should_use_subpixel_rendering(font_id, font_size) {
            GlyphRenderMode::Subpixel
        } else {
            GlyphRenderMode::Grayscale
        };
        let raster_style = self.text_system().prepare_raster_style(
            font_id,
            glyph_id,
            color,
            requested_mode,
            self.text_rendering_mode.get(),
        );
        let params = RenderGlyphParams {
            font_id,
            glyph_id,
            font_size,
            subpixel_variant,
            scale_factor,
            raster_style,
        };

        self.paint_glyph_from_atlas(integer_origin, params, color)
    }

    fn paint_glyph_from_atlas(
        &mut self,
        integer_origin: Point<ScaledPixels>,
        params: RenderGlyphParams,
        mask_color: Hsla,
    ) -> Result<()> {
        let text_system = self.text_system().clone();
        let entry = self
            .sprite_atlas
            .get_or_insert_glyph_with(&params, &mut || text_system.rasterize_glyph(&params))?;
        let Some(tile) = entry.tile else {
            return Ok(());
        };

        debug_assert_eq!(entry.bounds.size, tile.bounds.size.map(Into::into));
        let bounds = Bounds {
            origin: integer_origin + entry.bounds.origin.map(Into::into),
            size: tile.bounds.size.map(Into::into),
        };
        let content_mask = self.snapped_content_mask();

        match entry.format {
            RasterizedGlyphFormat::AlphaMask => {
                self.next_frame.scene.insert_primitive(MonochromeSprite {
                    order: 0,
                    padding: 0,
                    bounds,
                    content_mask,
                    color: mask_color.into(),
                    tile,
                    transformation: TransformationMatrix::unit(),
                });
            }
            RasterizedGlyphFormat::BgraSubpixelMask => {
                self.next_frame.scene.insert_primitive(SubpixelSprite {
                    order: 0,
                    padding: 0,
                    bounds,
                    content_mask,
                    color: mask_color.into(),
                    tile,
                    transformation: TransformationMatrix::unit(),
                });
            }
            RasterizedGlyphFormat::BgraColor => {
                self.next_frame.scene.insert_primitive(PolychromeSprite {
                    transformation: crate::TransformationMatrix::unit(),
                    order: 0,
                    grayscale: false.into(),
                    corner_smoothing: 0.0,
                    bounds,
                    corner_radii: Default::default(),
                    content_mask,
                    tile,
                    opacity: 1.0,
                });
            }
        }
        Ok(())
    }

    fn should_use_subpixel_rendering(&self, font_id: FontId, font_size: Pixels) -> bool {
        if self.next_frame.scene.is_in_filter_group() {
            return false;
        }

        if self
            .platform_window
            .background_appearance()
            .is_transparent()
        {
            return false;
        }

        if !self.platform_window.is_subpixel_rendering_supported() {
            return false;
        }

        let mode = match self.text_rendering_mode.get() {
            TextRenderingMode::PlatformDefault => self
                .text_system()
                .recommended_rendering_mode(font_id, font_size),
            mode => mode,
        };

        mode == TextRenderingMode::Subpixel
    }

    /// Paints an emoji glyph into the scene for the next frame at the current z-index.
    ///
    /// The y component of the origin is the baseline of the glyph.
    /// You should generally prefer to use the [`ShapedLine::paint`](crate::ShapedLine::paint) or
    /// [`ShapedText::paint`](crate::ShapedText::paint) methods in the [`TextSystem`](crate::TextSystem).
    /// This method is only useful if you need to paint a single emoji that has already been shaped.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_emoji(
        &mut self,
        origin: Point<Pixels>,
        font_id: FontId,
        glyph_id: GlyphId,
        font_size: Pixels,
    ) -> Result<()> {
        self.paint_emoji_with_color(origin, font_id, glyph_id, font_size, white())
    }

    /// Paints a color glyph with an application foreground for `currentColor` layers.
    pub fn paint_emoji_with_color(
        &mut self,
        origin: Point<Pixels>,
        font_id: FontId,
        glyph_id: GlyphId,
        font_size: Pixels,
        color: Hsla,
    ) -> Result<()> {
        self.invalidator.debug_assert_paint();

        let scale_factor = self.scale_factor();
        let glyph_origin = origin.scale(scale_factor);
        let (integer_origin, subpixel_variant) = quantize_color_glyph_origin(glyph_origin);
        let raster_style = self.text_system().prepare_raster_style(
            font_id,
            glyph_id,
            color,
            GlyphRenderMode::Color,
            self.text_rendering_mode.get(),
        );
        let params = RenderGlyphParams {
            font_id,
            glyph_id,
            font_size,
            subpixel_variant,
            scale_factor,
            raster_style,
        };

        self.paint_glyph_from_atlas(integer_origin, params, color)
    }

    /// Paint a monochrome SVG into the scene for the next frame at the current stacking context.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_svg(
        &mut self,
        bounds: Bounds<Pixels>,
        path: SharedString,
        mut data: Option<&[u8]>,
        transformation: TransformationMatrix,
        color: Hsla,
        cx: &App,
    ) -> Result<()> {
        self.invalidator.debug_assert_paint();

        let bounds = self.snap_bounds(bounds);

        let params = RenderSvgParams {
            path,
            size: bounds.size.map(|pixels| {
                DevicePixels::from((pixels.0 * SMOOTH_SVG_SCALE_FACTOR).ceil() as i32)
            }),
        };

        let Some(tile) =
            self.sprite_atlas
                .get_or_insert_with(&params.clone().into(), &mut || {
                    let Some((size, bytes)) = cx.svg_renderer.render_alpha_mask(&params, data)?
                    else {
                        return Ok(None);
                    };
                    Ok(Some((size, Cow::Owned(bytes))))
                })?
        else {
            return Ok(());
        };
        let content_mask = self.snapped_content_mask();
        let svg_bounds = Bounds {
            origin: bounds.center()
                - Point::new(
                    ScaledPixels(tile.bounds.size.width.0 as f32 / SMOOTH_SVG_SCALE_FACTOR / 2.),
                    ScaledPixels(tile.bounds.size.height.0 as f32 / SMOOTH_SVG_SCALE_FACTOR / 2.),
                ),
            size: tile
                .bounds
                .size
                .map(|value| ScaledPixels(value.0 as f32 / SMOOTH_SVG_SCALE_FACTOR)),
        };
        let final_bounds = svg_bounds
            .map_origin(|value| ScaledPixels(round_half_toward_zero(value.0)))
            .map_size(|size| size.ceil());

        self.next_frame.scene.insert_primitive(MonochromeSprite {
            order: 0,
            padding: 0,
            bounds: final_bounds,
            content_mask,
            color: color.into(),
            tile,
            transformation,
        });

        Ok(())
    }

    /// Paint an image into the scene for the next frame at the current z-index.
    /// This method will panic if the frame_index is not valid
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    /// Paint an image into `bounds`, positioning and scaling it according to `image_bounds`.
    ///
    /// The visible region rendered is `bounds.intersect(&image_bounds)`, with `corner_radii`
    /// applied to `bounds`.
    /// `corner_smoothing` ranges from circular corners at `0.0` to maximum smoothing at `1.0`.
    pub fn paint_image(
        &mut self,
        bounds: Bounds<Pixels>,
        image_bounds: Bounds<Pixels>,
        corner_radii: Corners<Pixels>,
        corner_smoothing: f32,
        data: Arc<RenderImage>,
        frame_index: usize,
        grayscale: bool,
    ) -> Result<()> {
        self.paint_image_transformed(
            bounds,
            image_bounds,
            corner_radii,
            corner_smoothing,
            data,
            frame_index,
            grayscale,
            TransformationMatrix::unit(),
        )
    }

    /// Paint an image with a GPU transformation, retaining atlas storage across frame updates.
    pub fn paint_image_transformed(
        &mut self,
        bounds: Bounds<Pixels>,
        image_bounds: Bounds<Pixels>,
        corner_radii: Corners<Pixels>,
        corner_smoothing: f32,
        data: Arc<RenderImage>,
        frame_index: usize,
        grayscale: bool,
        transformation: TransformationMatrix,
    ) -> Result<()> {
        self.invalidator.debug_assert_paint();

        let visible_bounds = bounds.intersect(&image_bounds);
        if visible_bounds.size.width <= Pixels::ZERO || visible_bounds.size.height <= Pixels::ZERO {
            return Ok(());
        }
        if image_bounds.size.width <= Pixels::ZERO || image_bounds.size.height <= Pixels::ZERO {
            return Ok(());
        }

        let params = RenderImageParams {
            image_id: data.id,
            frame_index,
        };

        let tile = self
            .sprite_atlas
            .get_or_update_with(&params.into(), data.content_version(), &mut || {
                Ok(Some((
                    data.size(frame_index),
                    Cow::Borrowed(
                        data.as_bytes(frame_index)
                            .expect("It's the caller's job to pass a valid frame index"),
                    ),
                )))
            })?
            .expect("Callback above only returns Some");

        let visible_bounds_snapped = self.snap_bounds(visible_bounds);

        let sub_tile = if visible_bounds == image_bounds {
            tile
        } else {
            let x_offset_ratio =
                (visible_bounds.origin.x - image_bounds.origin.x) / image_bounds.size.width;
            let y_offset_ratio =
                (visible_bounds.origin.y - image_bounds.origin.y) / image_bounds.size.height;
            let width_ratio = visible_bounds.size.width / image_bounds.size.width;
            let height_ratio = visible_bounds.size.height / image_bounds.size.height;

            let tile_origin_x = tile.bounds.origin.x.0;
            let tile_origin_y = tile.bounds.origin.y.0;
            let tile_width = tile.bounds.size.width.0;
            let tile_height = tile.bounds.size.height.0;

            let sub_origin_x = tile_origin_x + (x_offset_ratio * tile_width as f32).round() as i32;
            let sub_origin_y = tile_origin_y + (y_offset_ratio * tile_height as f32).round() as i32;
            let sub_width = (width_ratio * tile_width as f32).round() as i32;
            let sub_height = (height_ratio * tile_height as f32).round() as i32;

            let max_x = tile_origin_x + tile_width;
            let max_y = tile_origin_y + tile_height;

            let clamped_origin_x = sub_origin_x.clamp(tile_origin_x, max_x);
            let clamped_origin_y = sub_origin_y.clamp(tile_origin_y, max_y);
            let clamped_width = sub_width.min(max_x - clamped_origin_x).max(0);
            let clamped_height = sub_height.min(max_y - clamped_origin_y).max(0);

            AtlasTile {
                bounds: Bounds {
                    origin: point(
                        DevicePixels(clamped_origin_x),
                        DevicePixels(clamped_origin_y),
                    ),
                    size: size(DevicePixels(clamped_width), DevicePixels(clamped_height)),
                },
                ..tile
            }
        };

        let content_mask = self.snapped_content_mask();
        let corner_radii = corner_radii
            .clamp_radii_for_quad_size(visible_bounds.size)
            .scale(self.scale_factor());

        self.next_frame.scene.insert_primitive(PolychromeSprite {
            order: 0,
            grayscale: grayscale.into(),
            corner_smoothing: corner_smoothing.clamp(0.0, 1.0),
            bounds: visible_bounds_snapped,
            content_mask,
            corner_radii,
            tile: sub_tile,
            transformation,
            opacity: 1.0,
        });
        Ok(())
    }

    /// Paint a surface into the scene for the next frame at the current z-index.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn paint_surface(
        &mut self,
        bounds: Bounds<Pixels>,
        source: impl Into<crate::SurfaceSource>,
    ) {
        self.invalidator.debug_assert_paint();

        let bounds = self.snap_bounds(bounds);
        let content_mask = self.snapped_content_mask();
        self.next_frame.scene.insert_primitive(PaintSurface {
            order: 0,
            bounds,
            content_mask,
            source: source.into(),
        });
    }

    /// Removes an image from the sprite atlas.
    pub fn drop_image(&mut self, data: Arc<RenderImage>) -> Result<()> {
        for frame_index in 0..data.frame_count() {
            let params = RenderImageParams {
                image_id: data.id,
                frame_index,
            };

            self.sprite_atlas.remove(&params.clone().into());
        }

        Ok(())
    }

    /// Returns whether every frame of an image is present in the sprite atlas.
    #[cfg(any(test, feature = "test-support"))]
    pub fn has_image_atlas_entry(&self, data: &RenderImage) -> bool {
        data.frame_count() > 0
            && (0..data.frame_count()).all(|frame_index| {
                self.sprite_atlas.contains(
                    &RenderImageParams {
                        image_id: data.id,
                        frame_index,
                    }
                    .into(),
                )
            })
    }

    /// Add a node to the layout tree for the current frame. Takes the `Style` of the element for which
    /// layout is being requested, along with the layout ids of any children. This method is called during
    /// calls to the [`Element::request_layout`] trait method and enables any element to participate in layout.
    ///
    /// This method should only be called as part of the request_layout or prepaint phase of element drawing.
    #[must_use]
    pub fn request_layout(
        &mut self,
        style: Style,
        children: impl IntoIterator<Item = LayoutId>,
        cx: &mut App,
    ) -> LayoutId {
        self.invalidator.debug_assert_prepaint();

        cx.layout_id_buffer.clear();
        cx.layout_id_buffer.extend(children);
        let rem_size = self.rem_size();
        let scale_factor = self.scale_factor();

        self.layout_engine.as_mut().unwrap().request_layout(
            style,
            rem_size,
            scale_factor,
            &cx.layout_id_buffer,
        )
    }

    /// Add a node to the layout tree for the current frame. Instead of taking a `Style` and children,
    /// this variant takes a function that is invoked during layout so you can use arbitrary logic to
    /// determine the element's size. One place this is used internally is when measuring text.
    ///
    /// The given closure is invoked at layout time with the known dimensions and available space and
    /// returns a `Size`.
    ///
    /// This method should only be called as part of the request_layout or prepaint phase of element drawing.
    pub fn request_measured_layout<F>(&mut self, style: Style, measure: F) -> LayoutId
    where
        F: Fn(Size<Option<Pixels>>, Size<AvailableSpace>, &mut Window, &mut App) -> Size<Pixels>
            + 'static,
    {
        self.invalidator.debug_assert_prepaint();

        let rem_size = self.rem_size();
        let scale_factor = self.scale_factor();
        self.layout_engine
            .as_mut()
            .unwrap()
            .request_measured_layout(style, rem_size, scale_factor, measure)
    }

    /// Compute the layout for the given id within the given available space.
    /// This method is called for its side effect, typically by the framework prior to painting.
    /// After calling it, you can request the bounds of the given layout node id or any descendant.
    ///
    /// This method should only be called as part of the prepaint phase of element drawing.
    pub fn compute_layout(
        &mut self,
        layout_id: LayoutId,
        available_space: Size<AvailableSpace>,
        cx: &mut App,
    ) {
        self.invalidator.debug_assert_prepaint();

        let mut layout_engine = self.layout_engine.take().unwrap();
        layout_engine.compute_layout(
            layout_id,
            available_space,
            self.measurement_direction,
            self,
            cx,
        );
        self.layout_engine = Some(layout_engine);
    }

    /// Obtain the bounds computed for the given LayoutId relative to the window. This method will usually be invoked by
    /// GPUI itself automatically in order to pass your element its `Bounds` automatically.
    ///
    /// This method should only be called as part of element drawing.
    pub fn layout_bounds(&mut self, layout_id: LayoutId) -> Bounds<Pixels> {
        self.invalidator.debug_assert_prepaint();

        let scale_factor = self.scale_factor();
        let mut bounds = self
            .layout_engine
            .as_mut()
            .unwrap()
            .layout_bounds(layout_id, scale_factor)
            .map(Into::into);
        let snapped_offset = self.pixel_snap_point(self.element_offset());
        bounds.origin += snapped_offset;
        bounds
    }

    /// Obtain layout bounds snapped within the coordinate space of their parent.
    ///
    /// This is useful for text-bearing elements whose position within their parent must remain
    /// stable while that parent moves across the device-pixel grid.
    pub fn parent_relative_layout_bounds(&mut self, layout_id: LayoutId) -> Bounds<Pixels> {
        self.invalidator.debug_assert_prepaint();

        let scale_factor = self.scale_factor();
        let mut bounds = self
            .layout_engine
            .as_mut()
            .unwrap()
            .parent_relative_layout_bounds(layout_id, scale_factor);
        bounds.origin += self.pixel_snap_point(self.element_offset());
        bounds
    }

    pub(crate) fn publish_inline_content(
        &mut self,
        node_id: LayoutId,
        content: crate::InlineContent,
    ) -> bool {
        let Some(layout_engine) = self.layout_engine.as_mut() else {
            return false;
        };

        if let crate::InlineContent::Text { text, .. } = &content {
            layout_engine.set_direction_text(node_id, text.clone());
        }

        layout_engine
            .inline_content
            .insert(node_id, Arc::new(content));
        true
    }

    pub(crate) fn set_layout_logical_children(&mut self, node_id: LayoutId, children: &[LayoutId]) {
        self.layout_engine
            .as_mut()
            .unwrap()
            .set_logical_children(node_id, children);
    }

    pub(crate) fn set_layout_auto_direction_hint(
        &mut self,
        node_id: LayoutId,
        direction: Option<ResolvedDirection>,
    ) {
        self.layout_engine
            .as_mut()
            .unwrap()
            .set_auto_direction_hint(node_id, direction);
    }

    pub(crate) fn layout_auto_direction_contribution(
        &self,
        node_id: LayoutId,
    ) -> Option<ResolvedDirection> {
        self.layout_engine
            .as_ref()
            .unwrap()
            .auto_direction_contribution(node_id)
    }

    pub(crate) fn layout_direction_handle(
        &self,
        node_id: LayoutId,
    ) -> crate::taffy::LayoutDirectionHandle {
        self.layout_engine
            .as_ref()
            .unwrap()
            .direction_handle(node_id)
    }

    pub(crate) fn layout_directionality(
        &self,
        node_id: LayoutId,
    ) -> (ResolvedDirection, UnicodeBidi) {
        self.layout_engine.as_ref().unwrap().directionality(node_id)
    }

    pub(crate) fn with_layout_direction_context<R>(
        &mut self,
        node_id: LayoutId,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let (direction, unicode_bidi) = self.layout_directionality(node_id);
        let previous_direction = std::mem::replace(&mut self.measurement_direction, direction);
        let previous_unicode_bidi =
            std::mem::replace(&mut self.measurement_unicode_bidi, unicode_bidi);
        let result = f(self);
        self.measurement_direction = previous_direction;
        self.measurement_unicode_bidi = previous_unicode_bidi;
        result
    }

    /// Supplies source text used by `LayoutDirection::Auto` for a custom layout node.
    ///
    /// Built-in text elements register their content automatically. Call this during
    /// `request_layout` after obtaining the node's [`LayoutId`].
    pub fn set_layout_direction_text(&mut self, node_id: LayoutId, text: impl Into<SharedString>) {
        self.layout_engine
            .as_mut()
            .unwrap()
            .set_direction_text(node_id, text.into());
    }

    pub(crate) fn inline_content(&self, node_id: LayoutId) -> Option<Arc<crate::InlineContent>> {
        self.layout_engine
            .as_ref()?
            .inline_content
            .get(&node_id)
            .cloned()
    }

    pub(crate) fn layout_display_and_position(
        &self,
        node_id: LayoutId,
    ) -> Option<(crate::Display, crate::Position)> {
        self.layout_engine
            .as_ref()
            .map(|layout_engine| layout_engine.display_and_position(node_id))
    }

    pub(crate) fn inline_fragments(&self, node_id: LayoutId) -> Option<Arc<[Bounds<Pixels>]>> {
        self.layout_engine
            .as_ref()?
            .inline_fragments
            .get(&node_id)
            .map(|fragments| {
                let offset = self.pixel_snap_point(self.element_offset());
                if offset == Point::default() {
                    return fragments.clone();
                }

                fragments
                    .iter()
                    .map(|bounds| Bounds::new(bounds.origin + offset, bounds.size))
                    .collect()
            })
    }

    pub(crate) fn place_inline(
        &mut self,
        node_id: LayoutId,
        mut bounds: Bounds<Pixels>,
        fragments: Option<Vec<Bounds<Pixels>>>,
    ) -> bool {
        if self.layout_engine.is_none() {
            return false;
        }

        let offset = self.pixel_snap_point(self.element_offset());
        bounds.origin -= offset;

        let scale = self.scale_factor();
        let Some(engine) = self.layout_engine.as_mut() else {
            return false;
        };

        engine.place_inline(node_id, bounds, scale);

        if let Some(mut fragments) = fragments {
            // A zero offset leaves fragment origins unchanged, so skip those writes.
            if offset != Point::default() {
                for fragment in &mut fragments {
                    fragment.origin -= offset;
                }
            }

            engine.inline_fragments.insert(node_id, fragments.into());
        }

        true
    }

    pub(crate) fn layout_vertical_align(
        &self,
        layout_id: LayoutId,
    ) -> Option<crate::VerticalAlign> {
        self.layout_engine
            .as_ref()
            .map(|layout_engine| layout_engine.vertical_align(layout_id))
    }

    /// This method should be called during `prepaint`. You can use the returned [Hitbox]
    /// during `paint` or in an event handler to determine whether the inserted hitbox was the topmost.
    ///
    /// This method should only be called as part of the prepaint phase of element drawing.
    pub fn insert_hitbox(&mut self, bounds: Bounds<Pixels>, behavior: HitboxBehavior) -> Hitbox {
        self.insert_hitbox_mut(bounds, behavior).clone()
    }

    /// This method should be called during `prepaint`. You can use the returned [Hitbox]
    /// during `paint` or in an event handler to determine whether the inserted hitbox was the topmost.
    ///
    /// Returns a mutable reference to the hitbox inserted for the next frame.
    ///
    /// This method should only be called as part of the prepaint phase of element drawing.
    pub fn insert_hitbox_mut(
        &mut self,
        bounds: Bounds<Pixels>,
        behavior: HitboxBehavior,
    ) -> &mut Hitbox {
        self.invalidator.debug_assert_prepaint();

        let content_mask = self.content_mask();
        let hitbox_id = self.next_hitbox_id;
        self.next_hitbox_id = self.next_hitbox_id.next();
        let hitbox = Hitbox {
            id: hitbox_id,
            bounds,
            content_mask,
            behavior,
            fragments: self.current_inline_fragments.clone(),
            tags: Vec::default(),
        };

        self.next_frame.hitboxes.push_mut(hitbox)
    }

    /// Set a hitbox which will act as a control area of the platform window.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn insert_window_control_hitbox(&mut self, area: WindowControlArea, hitbox: Hitbox) {
        self.invalidator.debug_assert_paint();
        self.next_frame.window_control_hitboxes.push((area, hitbox));
    }

    /// Sets the key context for the current element. This context will be used to translate
    /// keybindings into actions.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn set_key_context(&mut self, context: KeyContext) {
        self.invalidator.debug_assert_paint();
        self.next_frame.dispatch_tree.set_key_context(context);
    }

    /// Sets the focus handle for the current element. This handle will be used to manage focus state
    /// and keyboard event dispatch for the element.
    ///
    /// This method should only be called as part of the prepaint phase of element drawing.
    pub fn set_focus_handle(&mut self, focus_handle: &FocusHandle, _: &App) {
        self.invalidator.debug_assert_prepaint();
        if focus_handle.is_focused(self) {
            self.next_frame.focus = Some(focus_handle.id);
        }
        self.next_frame.dispatch_tree.set_focus_id(focus_handle.id);
    }

    /// Sets the view id for the current element, which will be used to manage view caching.
    ///
    /// This method should only be called as part of element prepaint. We plan on removing this
    /// method eventually when we solve some issues that require us to construct editor elements
    /// directly instead of always using editors via views.
    pub fn set_view_id(&mut self, view_id: EntityId) {
        self.invalidator.debug_assert_prepaint();
        self.next_frame.dispatch_tree.set_view_id(view_id);
    }

    /// Get the entity ID for the currently rendering view
    pub fn current_view(&self) -> EntityId {
        self.invalidator.debug_assert_paint_or_prepaint();
        self.rendered_entity_stack.last().copied().unwrap()
    }

    #[inline]
    pub(crate) fn with_rendered_view<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.rendered_entity_stack.push(id);
        let result = f(self);
        self.rendered_entity_stack.pop();
        result
    }

    /// Executes the provided function with the specified image cache.
    pub fn with_image_cache<F, R>(&mut self, image_cache: Option<AnyImageCache>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        if let Some(image_cache) = image_cache {
            self.image_cache_stack.push(image_cache);
            let result = f(self);
            self.image_cache_stack.pop();
            result
        } else {
            f(self)
        }
    }

    /// Sets an input handler, such as [`ElementInputHandler`][element_input_handler], which interfaces with the
    /// platform to receive textual input with proper integration with concerns such
    /// as IME interactions. This handler will be active for the upcoming frame until the following frame is
    /// rendered.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    ///
    /// [element_input_handler]: crate::ElementInputHandler
    pub fn handle_input(
        &mut self,
        focus_handle: &FocusHandle,
        input_handler: impl InputHandler,
        cx: &App,
    ) {
        self.invalidator.debug_assert_paint();

        if focus_handle.is_focused(self) {
            let cx = self.to_async(cx);
            self.next_frame
                .input_handlers
                .push(Some(PlatformInputHandler::new(cx, Box::new(input_handler))));
        }
    }

    /// Forwards the focused input handler's [`TextInputConfiguration`] to the
    /// platform window when it differs from the last forwarded value. With no
    /// input handler the default configuration applies, so a field's
    /// preferences don't outlive its focus.
    fn apply_text_input_configuration(&mut self, cx: &mut App) {
        let configuration = match self.platform_window.take_input_handler() {
            Some(mut input_handler) => {
                let configuration = input_handler.text_input_configuration(self, cx);
                self.platform_window.set_input_handler(input_handler);
                configuration
            }
            None => TextInputConfiguration::default(),
        };
        if self.last_text_input_configuration.as_ref() != Some(&configuration) {
            self.platform_window
                .set_text_input_configuration(configuration.clone());
            self.last_text_input_configuration = Some(configuration);
        }
    }

    /// Register a mouse event listener on the window for the next frame. The type of event
    /// is determined by the first parameter of the given listener. When the next frame is rendered
    /// the listener will be cleared.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn on_mouse_event<Event: MouseEvent>(
        &mut self,
        mut listener: impl FnMut(&Event, DispatchPhase, &mut Window, &mut App) + 'static,
    ) {
        self.invalidator.debug_assert_paint();

        self.next_frame.mouse_listeners.push(Some(Box::new(
            move |event: &dyn Any, phase: DispatchPhase, window: &mut Window, cx: &mut App| {
                if let Some(event) = event.downcast_ref() {
                    listener(event, phase, window, cx)
                }
            },
        )));
    }

    // Observe completed hit tests independently of event propagation.
    pub(crate) fn on_pointer_change(
        &mut self,
        observer: impl Fn(PointerChange, &mut Window, &mut App) + 'static,
    ) {
        self.invalidator.debug_assert_paint();
        Rc::make_mut(&mut self.next_frame.pointer_observers).push(Rc::new(observer));
    }

    fn notify_pointer_observers(&mut self, cause: PointerChangeCause, cx: &mut App) {
        let change = PointerChange {
            position: self.pointer_inside_window.then_some(self.mouse_position),
            cause,
        };
        // Snapshot observers before callbacks can redraw the frame.
        let observers = self.rendered_frame.pointer_observers.clone();
        let propagate_event = cx.propagate_event;
        let default_prevented = self.default_prevented;
        for observer in observers.iter() {
            observer(change, self, cx);
            // Observers must not change the event's dispatch state.
            cx.propagate_event = propagate_event;
            self.default_prevented = default_prevented;
        }
    }

    /// Register a key event listener on this node for the next frame. The type of event
    /// is determined by the first parameter of the given listener. When the next frame is rendered
    /// the listener will be cleared.
    ///
    /// This is a fairly low-level method, so prefer using event handlers on elements unless you have
    /// a specific need to register a listener yourself.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn on_key_event<Event: KeyEvent>(
        &mut self,
        listener: impl Fn(&Event, DispatchPhase, &mut Window, &mut App) + 'static,
    ) {
        self.invalidator.debug_assert_paint();

        self.next_frame.dispatch_tree.on_key_event(Rc::new(
            move |event: &dyn Any, phase, window: &mut Window, cx: &mut App| {
                if let Some(event) = event.downcast_ref::<Event>() {
                    listener(event, phase, window, cx)
                }
            },
        ));
    }

    /// Register a modifiers changed event listener on the window for the next frame.
    ///
    /// This is a fairly low-level method, so prefer using event handlers on elements unless you have
    /// a specific need to register a global listener.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn on_modifiers_changed(
        &mut self,
        listener: impl Fn(&ModifiersChangedEvent, &mut Window, &mut App) + 'static,
    ) {
        self.invalidator.debug_assert_paint();

        self.next_frame.dispatch_tree.on_modifiers_changed(Rc::new(
            move |event: &ModifiersChangedEvent, window: &mut Window, cx: &mut App| {
                listener(event, window, cx)
            },
        ));
    }

    /// Register a listener to be called when the given focus handle or one of its descendants receives focus.
    /// This does not fire if the given focus handle - or one of its descendants - was previously focused.
    /// Returns a subscription and persists until the subscription is dropped.
    pub fn on_focus_in(
        &mut self,
        handle: &FocusHandle,
        cx: &mut App,
        mut listener: impl FnMut(&mut Window, &mut App) + 'static,
    ) -> Subscription {
        let focus_id = handle.id;
        let (subscription, activate) =
            self.new_focus_listener(Box::new(move |event, window, cx| {
                if event.is_focus_in(focus_id) {
                    listener(window, cx);
                }
                true
            }));
        cx.defer(move |_| activate());
        subscription
    }

    /// Register a listener to be called when the given focus handle or one of its descendants loses focus.
    /// Returns a subscription and persists until the subscription is dropped.
    pub fn on_focus_out(
        &mut self,
        handle: &FocusHandle,
        cx: &mut App,
        mut listener: impl FnMut(FocusOutEvent, &mut Window, &mut App) + 'static,
    ) -> Subscription {
        let focus_id = handle.id;
        let (subscription, activate) =
            self.new_focus_listener(Box::new(move |event, window, cx| {
                if let Some(blurred_id) = event.previous_focus_path.last().copied()
                    && event.is_focus_out(focus_id)
                {
                    let event = FocusOutEvent {
                        blurred: WeakFocusHandle {
                            id: blurred_id,
                            handles: Arc::downgrade(&cx.focus_handles),
                        },
                    };
                    listener(event, window, cx)
                }
                true
            }));
        cx.defer(move |_| activate());
        subscription
    }

    fn reset_cursor_style(&self, cx: &mut App) {
        // Set the cursor only if we're the active window.
        if self.is_window_hovered() {
            let style = self
                .rendered_frame
                .cursor_style(self)
                .unwrap_or(CursorStyle::Arrow);
            cx.platform.set_cursor_style(style);
        }
    }

    /// Dispatch a given keystroke as though the user had typed it.
    /// You can create a keystroke with Keystroke::parse("").
    pub fn dispatch_keystroke(&mut self, keystroke: Keystroke, cx: &mut App) -> bool {
        let keystroke = keystroke.with_simulated_ime();
        let result = self.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        if !result.propagate {
            return true;
        }

        if let Some(input) = keystroke.key_char
            && let Some(mut input_handler) = self.platform_window.take_input_handler()
        {
            input_handler.dispatch_input(&input, self, cx);
            self.platform_window.set_input_handler(input_handler);
            return true;
        }

        false
    }

    /// Return a key binding string for an action, to display in the UI. Uses the highest precedence
    /// binding for the action (last binding added to the keymap).
    pub fn keystroke_text_for(&self, action: &dyn Action) -> String {
        self.highest_precedence_binding_for_action(action)
            .map(|binding| {
                binding
                    .keystrokes()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| action.name().to_string())
    }

    /// Dispatch a mouse, keyboard, or touch event on the window.
    #[profiling::function]
    pub fn dispatch_event(&mut self, event: PlatformInput, cx: &mut App) -> DispatchEventResult {
        #[cfg(feature = "profiler")]
        self.window_profiler.begin_input(event.kind_name());
        let update_count_before = self.invalidator.update_count();
        // Track input modality for focus-visible styling and hover suppression.
        // Hover is suppressed during keyboard modality so that keyboard navigation
        // doesn't show hover highlights on the item under the mouse cursor.
        let old_focus_visible = self.keyboard_focus_visible;
        // Pointer motion restores hover without hiding the keyboard focus ring.
        // Only intentional pointer interaction changes focus-visible styling.
        self.keyboard_focus_visible = match &event {
            PlatformInput::KeyDown(_) => true,
            PlatformInput::MouseDown(_)
            | PlatformInput::MouseUp(_)
            | PlatformInput::ScrollWheel(_)
            | PlatformInput::Pinch(_)
            | PlatformInput::Touch(_) => false,
            _ => self.keyboard_focus_visible,
        };
        let old_modality = self.last_input_modality;
        self.last_input_modality = match &event {
            PlatformInput::KeyDown(_) => InputModality::Keyboard,
            PlatformInput::MouseDown(_)
            | PlatformInput::MouseUp(_)
            | PlatformInput::ScrollWheel(_)
            | PlatformInput::Pinch(_)
            | PlatformInput::MouseMove(_) => InputModality::Mouse,
            PlatformInput::Touch(_) => InputModality::Touch,
            _ => self.last_input_modality,
        };
        if self.last_input_modality != old_modality
            || self.keyboard_focus_visible != old_focus_visible
        {
            self.refresh();
        }

        // Handlers may set this to false by calling `stop_propagation`.
        cx.propagate_event = true;
        // Handlers may set this to true by calling `prevent_default`.
        self.default_prevented = false;

        let event = match event {
            // Track the mouse position with our own state, since accessing the platform
            // API for the mouse position can only occur on the main thread.
            PlatformInput::MouseMove(mouse_move) => {
                self.mouse_position = mouse_move.position;
                self.modifiers = mouse_move.modifiers;
                PlatformInput::MouseMove(mouse_move)
            }
            PlatformInput::MouseDown(mouse_down) => {
                self.mouse_position = mouse_down.position;
                self.modifiers = mouse_down.modifiers;
                PlatformInput::MouseDown(mouse_down)
            }
            PlatformInput::MouseUp(mouse_up) => {
                self.mouse_position = mouse_up.position;
                self.modifiers = mouse_up.modifiers;
                PlatformInput::MouseUp(mouse_up)
            }
            PlatformInput::MousePressure(mouse_pressure) => {
                PlatformInput::MousePressure(mouse_pressure)
            }
            PlatformInput::MouseExited(mouse_exited) => {
                self.modifiers = mouse_exited.modifiers;
                PlatformInput::MouseExited(mouse_exited)
            }
            PlatformInput::ModifiersChanged(modifiers_changed) => {
                self.modifiers = modifiers_changed.modifiers;
                self.capslock = modifiers_changed.capslock;
                PlatformInput::ModifiersChanged(modifiers_changed)
            }
            PlatformInput::ScrollWheel(scroll_wheel) => {
                self.mouse_position = scroll_wheel.position;
                self.modifiers = scroll_wheel.modifiers;
                PlatformInput::ScrollWheel(scroll_wheel)
            }
            PlatformInput::Pinch(pinch) => {
                self.mouse_position = pinch.position;
                self.modifiers = pinch.modifiers;
                PlatformInput::Pinch(pinch)
            }
            // Translate dragging and dropping of external files from the operating system
            // to internal drag and drop events.
            PlatformInput::FileDrop(file_drop) => match file_drop {
                FileDropEvent::Entered { position, paths } => {
                    self.mouse_position = position;
                    let source_window = self.handle.window_id();
                    if !cx.restore_platform_drag(source_window) && cx.active_drag.is_none() {
                        cx.active_drag = Some(AnyDrag {
                            value: Arc::new(paths.clone()),
                            view: cx.new(|_| paths).into(),
                            cursor_offset: position,
                            cursor_style: None,
                            external_payload_source: None,
                        });
                    }
                    PlatformInput::MouseMove(MouseMoveEvent {
                        position,
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Modifiers::default(),
                    })
                }
                FileDropEvent::Pending { position } => {
                    self.mouse_position = position;
                    PlatformInput::MouseMove(MouseMoveEvent {
                        position,
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Modifiers::default(),
                    })
                }
                FileDropEvent::Submit { position } => {
                    cx.activate(true);
                    self.mouse_position = position;
                    PlatformInput::MouseUp(MouseUpEvent {
                        button: MouseButton::Left,
                        position,
                        modifiers: Modifiers::default(),
                        click_count: 1,
                    })
                }
                FileDropEvent::Exited => {
                    if !cx.hand_restored_drag_to_platform(self.handle.window_id()) {
                        cx.active_drag.take();
                    }
                    self.refresh();
                    PlatformInput::FileDrop(FileDropEvent::Exited)
                }
                FileDropEvent::Ended => {
                    cx.end_platform_drag(self.handle.window_id());
                    self.refresh();
                    PlatformInput::FileDrop(FileDropEvent::Ended)
                }
            },
            PlatformInput::Touch(touch) => PlatformInput::Touch(touch),
            PlatformInput::LongPress(long_press) => {
                self.mouse_position = if long_press.phase == crate::TouchPhase::Started {
                    long_press.start_position
                } else {
                    long_press.position
                };
                if long_press.phase == crate::TouchPhase::Started {
                    self.long_press_capture = None;
                }
                PlatformInput::LongPress(long_press)
            }
            PlatformInput::TouchDrag(touch_drag) => {
                self.mouse_position = touch_drag.start_position;
                PlatformInput::TouchDrag(touch_drag)
            }
            PlatformInput::KeyDown(_) | PlatformInput::KeyUp(_) => event,
        };

        if let Some(any_mouse_event) = event.mouse_event() {
            self.dispatch_mouse_event(any_mouse_event, cx);
        } else if let Some(any_key_event) = event.keyboard_event() {
            self.dispatch_key_event(any_key_event, cx);
        } else if let Some(touch_event) = event.touch_event() {
            self.dispatch_touch_event(touch_event, cx);
        }
        if let PlatformInput::LongPress(long_press) = &event {
            match long_press.phase {
                crate::TouchPhase::Started if !self.default_prevented => {
                    self.long_press_capture = None;
                }
                crate::TouchPhase::Ended | crate::TouchPhase::Cancelled => {
                    self.long_press_capture = None;
                }
                crate::TouchPhase::Started | crate::TouchPhase::Moved => {}
            }
        }

        // Must run after the move is dispatched: the platform owns the gesture afterwards, so this
        // is the last chance for drag listeners to see the pointer leave and reset their state.
        self.promote_external_drag_to_platform(&event, cx);

        let caused_invalidation = self.invalidator.update_count() > update_count_before;
        if caused_invalidation {
            self.input_rate_tracker.borrow_mut().record_input();
        }
        #[cfg(feature = "profiler")]
        self.window_profiler.end_input(caused_invalidation);

        DispatchEventResult {
            propagate: cx.propagate_event,
            default_prevented: self.default_prevented,
        }
    }

    fn promote_external_drag_to_platform(&mut self, event: &PlatformInput, cx: &mut App) {
        let PlatformInput::MouseMove(mouse_move) = event else {
            return;
        };
        if mouse_move.pressed_button != Some(MouseButton::Left) {
            return;
        }
        if Bounds::new(Point::default(), self.viewport_size).contains(&mouse_move.position) {
            return;
        }
        if !self.platform_window.can_start_external_drag() {
            return;
        }
        let Some(payload_source) = cx
            .active_drag
            .as_mut()
            .and_then(|drag| drag.external_payload_source.take())
        else {
            return;
        };
        let Some(payload) = payload_source(self, cx) else {
            return;
        };
        if self.platform_window.start_external_drag(&payload)
            && cx.hand_active_drag_to_platform(self.handle.window_id())
        {
            self.refresh();
        }
    }

    /// Whether recognized touch pans may use the platform's predicted touch
    /// positions ([`TouchEvent::predicted_position`]) to compensate for input
    /// latency. Defaults to true.
    pub fn touch_prediction_enabled(&self) -> bool {
        self.touch_prediction_enabled
    }

    /// Sets whether recognized touch pans may use the platform's predicted
    /// touch positions. Disabling drops [`TouchEvent::predicted_position`]
    /// before gesture recognition, so pans track only raw touch positions.
    pub fn set_touch_prediction_enabled(&mut self, enabled: bool) {
        self.touch_prediction_enabled = enabled;
    }

    /// Runs the portable gesture recognizer over a raw touch event and
    /// dispatches whatever it resolves (scroll steps, synthesized taps)
    /// through the ordinary mouse-event path.
    fn dispatch_touch_event(&mut self, event: &TouchEvent, cx: &mut App) {
        let mut event = event.clone();
        if !self.touch_prediction_enabled {
            event.predicted_position = None;
        }
        let recognized_gestures = self.touch_gestures.handle_event(&event);
        if event.phase == crate::TouchPhase::Started
            && let Some(touch_drag) = self.touch_gestures.offer_touch_drag(event.id)
        {
            self.dispatch_recognized_touch_gesture(touch_drag, cx);
        }
        if event.phase == crate::TouchPhase::Started
            && self.touch_gestures.pending_long_press().is_some()
        {
            self.long_press_capture = None;
        }
        let mut tapped = false;
        for gesture in recognized_gestures {
            tapped |= matches!(gesture, RecognizedTouchGesture::Tap { .. });
            self.dispatch_recognized_touch_gesture(gesture, cx);
        }
        if event.phase == crate::TouchPhase::Started {
            self.schedule_long_press_timer(cx);
        } else if self.touch_gestures.pending_long_press().is_none() {
            self.long_press_timer.take();
        }
        // The platform's touch-release handler may inspect the input handler
        // as soon as this dispatch returns (the web platform decides virtual
        // keyboard visibility there, inside the user gesture). Input handlers
        // are registered during draw, so draw now to make them reflect any
        // focus change the tap just caused.
        if tapped && self.invalidator.is_dirty() {
            self.draw(cx).clear(cx);
        }
        if self.touch_gestures.has_momentum() {
            self.schedule_touch_momentum_tick();
        }
    }

    fn dispatch_recognized_touch_gesture(&mut self, gesture: RecognizedTouchGesture, cx: &mut App) {
        match gesture {
            RecognizedTouchGesture::Scroll(scroll_wheel) => {
                self.mouse_position = scroll_wheel.position;
                cx.propagate_event = true;
                self.dispatch_mouse_event(&scroll_wheel, cx);
            }
            RecognizedTouchGesture::Tap { down, up } => {
                self.mouse_position = up.position;
                cx.propagate_event = true;
                self.dispatch_mouse_event(&down, cx);
                cx.propagate_event = true;
                self.dispatch_mouse_event(&up, cx);
            }
            RecognizedTouchGesture::TouchDrag(touch_drag) => {
                self.mouse_position = touch_drag.start_position;
                cx.propagate_event = true;
                self.default_prevented = false;
                let started = touch_drag.phase == crate::TouchPhase::Started;
                self.dispatch_mouse_event(&touch_drag, cx);
                if started {
                    self.touch_gestures
                        .resolve_touch_drag(self.default_prevented);
                }
            }
            RecognizedTouchGesture::LongPress(long_press) => {
                self.mouse_position = if long_press.phase == crate::TouchPhase::Started {
                    long_press.start_position
                } else {
                    long_press.position
                };
                cx.propagate_event = true;
                self.default_prevented = false;
                let started = long_press.phase == crate::TouchPhase::Started;
                let ended = matches!(
                    long_press.phase,
                    crate::TouchPhase::Ended | crate::TouchPhase::Cancelled
                );
                self.dispatch_mouse_event(&long_press, cx);
                if started {
                    let claimed = self.default_prevented;
                    self.touch_gestures.resolve_long_press(claimed);
                    if !claimed {
                        self.long_press_capture = None;
                    }
                }
                if ended {
                    self.long_press_capture = None;
                }
            }
        }
    }

    fn schedule_long_press_timer(&mut self, cx: &mut App) {
        self.long_press_timer.take();
        let Some((touch_id, duration)) = self.touch_gestures.pending_long_press() else {
            return;
        };
        self.long_press_timer = Some(self.spawn(cx, async move |cx| {
            cx.background_executor.timer(duration).await;
            cx.update(move |window, cx| {
                window.long_press_timer.take();
                if let Some(gesture) = window.touch_gestures.offer_long_press(touch_id) {
                    window.dispatch_recognized_touch_gesture(gesture, cx);
                }
            })
            .log_err();
        }));
    }

    fn schedule_touch_momentum_tick(&mut self) {
        self.on_next_frame(|window, cx| {
            if let Some(gesture) = window.touch_gestures.tick_momentum() {
                window.dispatch_recognized_touch_gesture(gesture, cx);
            }
            if window.touch_gestures.has_momentum() {
                window.schedule_touch_momentum_tick();
            }
        });
    }

    /// Returns the hit-test that was most recently cached from a mouse event on the rendered frame.
    pub fn mouse_hit_test(&self) -> &HitTest {
        &self.mouse_hit_test
    }

    fn dispatch_mouse_event(&mut self, event: &dyn Any, cx: &mut App) {
        if event.is::<crate::MouseExitEvent>() {
            self.pointer_inside_window = false;
        } else if event.is::<MouseMoveEvent>()
            || event.is::<crate::MouseDownEvent>()
            || event.is::<MouseUpEvent>()
        {
            self.pointer_inside_window =
                Bounds::new(Point::default(), self.viewport_size).contains(&self.mouse_position);
        }
        let hit_test = self.rendered_frame.hit_test(self.mouse_position());
        if hit_test != self.mouse_hit_test {
            self.mouse_hit_test = hit_test;
        }
        // Resolve on every pointer event, including movement within one hitbox.
        self.reset_cursor_style(cx);

        #[cfg(any(feature = "inspector", debug_assertions))]
        if self.is_inspector_picking(cx) {
            self.handle_inspector_mouse_event(event, cx);
            // When inspector is picking, all other mouse handling is skipped.
            return;
        }

        let mut mouse_listeners = mem::take(&mut self.rendered_frame.mouse_listeners);

        // Capture phase, events bubble from back to front. Handlers for this phase are used for
        // special purposes, such as detecting events outside of a given Bounds.
        for listener in &mut mouse_listeners {
            let listener = listener.as_mut().unwrap();
            listener(event, DispatchPhase::Capture, self, cx);
            if !cx.propagate_event {
                break;
            }
        }

        // Bubble phase, where most normal handlers do their work.
        if cx.propagate_event {
            for listener in mouse_listeners.iter_mut().rev() {
                let listener = listener.as_mut().unwrap();
                listener(event, DispatchPhase::Bubble, self, cx);
                if !cx.propagate_event {
                    break;
                }
            }
        }

        self.rendered_frame.mouse_listeners = mouse_listeners;

        if cx.has_active_drag() {
            if event.is::<MouseMoveEvent>() {
                // If this was a mouse move event, redraw the window so that the
                // active drag can follow the mouse cursor.
                self.refresh();
            } else if event.is::<MouseUpEvent>() {
                // If this was a mouse up event, cancel the active drag and redraw
                // the window.
                cx.active_drag = None;
                self.refresh();
            }
        }

        // Auto-release pointer capture on mouse up
        if event.is::<MouseUpEvent>() && self.captured_hitbox.is_some() {
            self.captured_hitbox = None;
        }
        if event.is::<MouseMoveEvent>()
            || event.is::<crate::MouseExitEvent>()
            || event.is::<MouseUpEvent>()
        {
            self.notify_pointer_observers(PointerChangeCause::Input, cx);
        }
    }

    fn dispatch_key_event(&mut self, event: &dyn Any, cx: &mut App) {
        if self.invalidator.is_dirty() {
            self.draw(cx).clear(cx);
        }

        let node_id = self.focus_node_id_in_rendered_frame(self.focus);
        let dispatch_path = self.rendered_frame.dispatch_tree.dispatch_path(node_id);

        let mut keystroke: Option<Keystroke> = None;

        if let Some(event) = event.downcast_ref::<ModifiersChangedEvent>() {
            if event.modifiers.number_of_modifiers() == 0
                && self.pending_modifier.modifiers.number_of_modifiers() == 1
                && !self.pending_modifier.saw_other_input
            {
                let key = match self.pending_modifier.modifiers {
                    modifiers if modifiers.shift => Some("shift"),
                    modifiers if modifiers.control => Some("control"),
                    modifiers if modifiers.alt => Some("alt"),
                    modifiers if modifiers.platform => Some("platform"),
                    modifiers if modifiers.function => Some("function"),
                    _ => None,
                };
                if let Some(key) = key {
                    keystroke = Some(Keystroke {
                        key: key.to_string(),
                        key_char: None,
                        modifiers: Modifiers::default(),
                    });
                }
            }

            if self.pending_modifier.modifiers.number_of_modifiers() == 0
                && event.modifiers.number_of_modifiers() == 1
            {
                self.pending_modifier.saw_other_input = false
            } else if event.modifiers.number_of_modifiers() > 1 {
                self.pending_modifier.saw_other_input = true
            }
            self.pending_modifier.modifiers = event.modifiers
        } else if let Some(key_down_event) = event.downcast_ref::<KeyDownEvent>() {
            self.pending_modifier.saw_other_input = true;
            keystroke = Some(key_down_event.keystroke.clone());
            if key_down_event.keystroke.key_char.is_some()
                && matches!(
                    cx.cursor_hide_mode,
                    CursorHideMode::OnTyping | CursorHideMode::OnTypingAndAction
                )
            {
                cx.platform.hide_cursor_until_mouse_moves();
            }
        }

        let Some(keystroke) = keystroke else {
            self.finish_dispatch_key_event(event, dispatch_path, self.context_stack(), cx);
            return;
        };

        cx.propagate_event = true;
        self.dispatch_keystroke_interceptors(event, self.context_stack(), cx);
        if !cx.propagate_event {
            self.finish_dispatch_key_event(event, dispatch_path, self.context_stack(), cx);
            return;
        }

        let mut currently_pending = self.pending_input.take().unwrap_or_default();
        if currently_pending.focus.is_some() && currently_pending.focus != self.focus {
            currently_pending = PendingInput::default();
        }

        let match_result = self.rendered_frame.dispatch_tree.dispatch_key(
            currently_pending.keystrokes,
            keystroke,
            &dispatch_path,
        );

        if !match_result.to_replay.is_empty() {
            self.replay_pending_input(match_result.to_replay, cx);
            cx.propagate_event = true;
        }

        if !match_result.pending.is_empty() {
            currently_pending.timer.take();
            currently_pending.keystrokes = match_result.pending;
            currently_pending.focus = self.focus;

            let text_input_requires_timeout = event
                .downcast_ref::<KeyDownEvent>()
                .filter(|key_down| key_down.keystroke.key_char.is_some())
                .and_then(|_| self.platform_window.take_input_handler())
                .map_or(false, |mut input_handler| {
                    let accepts = input_handler.accepts_text_input(self, cx);
                    self.platform_window.set_input_handler(input_handler);
                    accepts
                });

            currently_pending.needs_timeout |=
                match_result.pending_has_binding || text_input_requires_timeout;

            if currently_pending.needs_timeout {
                currently_pending.timer = Some(self.spawn(cx, async move |cx| {
                    cx.background_executor.timer(Duration::from_secs(1)).await;
                    cx.update(move |window, cx| {
                        let Some(currently_pending) = window
                            .pending_input
                            .take()
                            .filter(|pending| pending.focus == window.focus)
                        else {
                            return;
                        };

                        let node_id = window.focus_node_id_in_rendered_frame(window.focus);
                        let dispatch_path =
                            window.rendered_frame.dispatch_tree.dispatch_path(node_id);

                        let to_replay = window
                            .rendered_frame
                            .dispatch_tree
                            .flush_dispatch(currently_pending.keystrokes, &dispatch_path);

                        window.pending_input_changed(cx);
                        window.replay_pending_input(to_replay, cx)
                    })
                    .log_err();
                }));
            } else {
                currently_pending.timer = None;
            }
            self.pending_input = Some(currently_pending);
            self.pending_input_changed(cx);
            cx.propagate_event = false;
            return;
        }

        let skip_bindings = event
            .downcast_ref::<KeyDownEvent>()
            .filter(|key_down_event| key_down_event.prefer_character_input)
            .map(|_| {
                self.platform_window
                    .take_input_handler()
                    .map_or(false, |mut input_handler| {
                        let accepts = input_handler.accepts_text_input(self, cx);
                        self.platform_window.set_input_handler(input_handler);
                        // If modifiers are not excessive (e.g. AltGr), and the input handler is accepting text input,
                        // we prefer the text input over bindings.
                        accepts
                    })
            })
            .unwrap_or(false);

        if !skip_bindings {
            for binding in match_result.bindings {
                self.dispatch_action_on_node(node_id, binding.action.as_ref(), cx);
                if !cx.propagate_event {
                    self.dispatch_keystroke_observers(
                        event,
                        Some(binding.action),
                        match_result.context_stack,
                        cx,
                    );
                    self.pending_input_changed(cx);
                    return;
                }
            }
        }

        self.finish_dispatch_key_event(event, dispatch_path, match_result.context_stack, cx);
        self.pending_input_changed(cx);
    }

    fn finish_dispatch_key_event(
        &mut self,
        event: &dyn Any,
        dispatch_path: SmallVec<[DispatchNodeId; 32]>,
        context_stack: Vec<KeyContext>,
        cx: &mut App,
    ) {
        self.dispatch_key_down_up_event(event, &dispatch_path, cx);
        if !cx.propagate_event {
            return;
        }

        self.dispatch_modifiers_changed_event(event, &dispatch_path, cx);
        if !cx.propagate_event {
            return;
        }

        self.dispatch_keystroke_observers(event, None, context_stack, cx);
    }

    pub(crate) fn pending_input_changed(&mut self, cx: &mut App) {
        self.pending_input_observers
            .clone()
            .retain(&(), |callback| callback(self, cx));
    }

    fn defer_pending_input_changed(&self, cx: &mut App) {
        // Avoid re-entrant entity updates by deferring observer notifications to the end of the
        // current effect cycle, and only for this window.
        let window_handle = self.handle;
        cx.defer(move |cx| {
            window_handle
                .update(cx, |_, window, cx| {
                    window.pending_input_changed(cx);
                })
                .ok();
        });
    }

    fn dispatch_key_down_up_event(
        &mut self,
        event: &dyn Any,
        dispatch_path: &SmallVec<[DispatchNodeId; 32]>,
        cx: &mut App,
    ) {
        // Capture phase
        for node_id in dispatch_path {
            let node = self.rendered_frame.dispatch_tree.node(*node_id);

            for key_listener in node.key_listeners.clone() {
                key_listener(event, DispatchPhase::Capture, self, cx);
                if !cx.propagate_event {
                    return;
                }
            }
        }

        // Bubble phase
        for node_id in dispatch_path.iter().rev() {
            // Handle low level key events
            let node = self.rendered_frame.dispatch_tree.node(*node_id);
            for key_listener in node.key_listeners.clone() {
                key_listener(event, DispatchPhase::Bubble, self, cx);
                if !cx.propagate_event {
                    return;
                }
            }
        }
    }

    fn dispatch_modifiers_changed_event(
        &mut self,
        event: &dyn Any,
        dispatch_path: &SmallVec<[DispatchNodeId; 32]>,
        cx: &mut App,
    ) {
        let Some(event) = event.downcast_ref::<ModifiersChangedEvent>() else {
            return;
        };
        for node_id in dispatch_path.iter().rev() {
            let node = self.rendered_frame.dispatch_tree.node(*node_id);
            for listener in node.modifiers_changed_listeners.clone() {
                listener(event, self, cx);
                if !cx.propagate_event {
                    return;
                }
            }
        }
    }

    /// Pending input that can still complete a binding. Input left over from a previous focus can
    /// never complete one.
    fn active_pending_input(&self) -> Option<&PendingInput> {
        self.pending_input
            .as_ref()
            .filter(|pending_input| pending_input.focus == self.focus)
    }

    /// Determine whether a potential multi-stroke key binding is in progress on this window.
    pub fn has_pending_keystrokes(&self) -> bool {
        self.active_pending_input().is_some()
    }

    #[cfg(test)]
    pub(crate) fn pending_input_is_none(&self) -> bool {
        self.pending_input.is_none()
    }

    pub(crate) fn clear_pending_keystrokes(&mut self, cx: &mut App) {
        if self.pending_input.take().is_some() {
            self.defer_pending_input_changed(cx);
        }
    }

    /// Returns the currently pending input keystrokes that might result in a multi-stroke key binding.
    pub fn pending_input_keystrokes(&self) -> Option<&[Keystroke]> {
        self.active_pending_input()
            .map(|pending_input| pending_input.keystrokes.as_slice())
    }

    fn replay_pending_input(&mut self, replays: SmallVec<[Replay; 1]>, cx: &mut App) {
        let node_id = self.focus_node_id_in_rendered_frame(self.focus);
        let dispatch_path = self.rendered_frame.dispatch_tree.dispatch_path(node_id);

        'replay: for replay in replays {
            let event = KeyDownEvent {
                keystroke: replay.keystroke.clone(),
                is_held: false,
                prefer_character_input: true,
            };

            cx.propagate_event = true;
            for binding in replay.bindings {
                self.dispatch_action_on_node(node_id, binding.action.as_ref(), cx);
                if !cx.propagate_event {
                    self.dispatch_keystroke_observers(
                        &event,
                        Some(binding.action),
                        Vec::default(),
                        cx,
                    );
                    continue 'replay;
                }
            }

            self.dispatch_key_down_up_event(&event, &dispatch_path, cx);
            if !cx.propagate_event {
                continue 'replay;
            }
            if let Some(input) = replay.keystroke.key_char.as_ref().cloned()
                && let Some(mut input_handler) = self.platform_window.take_input_handler()
            {
                input_handler.dispatch_input(&input, self, cx);
                self.platform_window.set_input_handler(input_handler)
            }
        }
    }

    fn focus_node_id_in_rendered_frame(&self, focus_id: Option<FocusId>) -> DispatchNodeId {
        focus_id
            .and_then(|focus_id| {
                self.rendered_frame
                    .dispatch_tree
                    .focusable_node_id(focus_id)
            })
            .unwrap_or_else(|| self.rendered_frame.dispatch_tree.root_node_id())
    }

    fn dispatch_action_on_node(
        &mut self,
        node_id: DispatchNodeId,
        action: &dyn Action,
        cx: &mut App,
    ) {
        self.dispatch_action_on_node_inner(node_id, action, cx);

        if !cx.propagate_event
            && cx.cursor_hide_mode == CursorHideMode::OnTypingAndAction
            && self.last_input_was_keyboard()
        {
            cx.platform.hide_cursor_until_mouse_moves();
        }
    }

    fn dispatch_action_on_node_inner(
        &mut self,
        node_id: DispatchNodeId,
        action: &dyn Action,
        cx: &mut App,
    ) {
        let dispatch_path = self.rendered_frame.dispatch_tree.dispatch_path(node_id);

        // Capture phase for global actions.
        cx.propagate_event = true;
        if let Some(mut global_listeners) = cx
            .global_action_listeners
            .remove(&action.as_any().type_id())
        {
            for listener in &global_listeners {
                #[cfg(feature = "profiler")]
                self.window_profiler.begin_action_handler(action, cx);
                listener(action.as_any(), DispatchPhase::Capture, cx);
                #[cfg(feature = "profiler")]
                self.window_profiler.end_action_handler();
                if !cx.propagate_event {
                    break;
                }
            }

            global_listeners.extend(
                cx.global_action_listeners
                    .remove(&action.as_any().type_id())
                    .unwrap_or_default(),
            );

            cx.global_action_listeners
                .insert(action.as_any().type_id(), global_listeners);
        }

        if !cx.propagate_event {
            return;
        }

        // Capture phase for window actions.
        for node_id in &dispatch_path {
            let node = self.rendered_frame.dispatch_tree.node(*node_id);
            for DispatchActionListener {
                action_type,
                listener,
            } in node.action_listeners.clone()
            {
                let any_action = action.as_any();
                if action_type == any_action.type_id() {
                    #[cfg(feature = "profiler")]
                    self.window_profiler.begin_action_handler(action, cx);
                    listener(any_action, DispatchPhase::Capture, self, cx);
                    #[cfg(feature = "profiler")]
                    self.window_profiler.end_action_handler();

                    if !cx.propagate_event {
                        return;
                    }
                }
            }
        }

        // Bubble phase for window actions.
        for node_id in dispatch_path.iter().rev() {
            let node = self.rendered_frame.dispatch_tree.node(*node_id);
            for DispatchActionListener {
                action_type,
                listener,
            } in node.action_listeners.clone()
            {
                let any_action = action.as_any();
                if action_type == any_action.type_id() {
                    cx.propagate_event = false; // Actions stop propagation by default during the bubble phase
                    #[cfg(feature = "profiler")]
                    self.window_profiler.begin_action_handler(action, cx);
                    listener(any_action, DispatchPhase::Bubble, self, cx);
                    #[cfg(feature = "profiler")]
                    self.window_profiler.end_action_handler();

                    if !cx.propagate_event {
                        return;
                    }
                }
            }
        }

        // Bubble phase for global actions.
        if let Some(mut global_listeners) = cx
            .global_action_listeners
            .remove(&action.as_any().type_id())
        {
            for listener in global_listeners.iter().rev() {
                cx.propagate_event = false; // Actions stop propagation by default during the bubble phase

                #[cfg(feature = "profiler")]
                self.window_profiler.begin_action_handler(action, cx);
                listener(action.as_any(), DispatchPhase::Bubble, cx);
                #[cfg(feature = "profiler")]
                self.window_profiler.end_action_handler();
                if !cx.propagate_event {
                    break;
                }
            }

            global_listeners.extend(
                cx.global_action_listeners
                    .remove(&action.as_any().type_id())
                    .unwrap_or_default(),
            );

            cx.global_action_listeners
                .insert(action.as_any().type_id(), global_listeners);
        }
    }

    /// Register the given handler to be invoked whenever the global of the given type
    /// is updated.
    pub fn observe_global<G: Global>(
        &mut self,
        cx: &mut App,
        f: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Subscription {
        let window_handle = self.handle;
        let (subscription, activate) = cx.global_observers.insert(
            TypeId::of::<G>(),
            Box::new(move |cx| {
                window_handle
                    .update(cx, |_, window, cx| f(window, cx))
                    .is_ok()
            }),
        );
        cx.defer(move |_| activate());
        subscription
    }

    /// Focus the current window and bring it to the foreground at the platform level.
    pub fn activate(&self) {
        self.platform_window.activate(None);
    }

    /// Request focus using a token supplied by the desktop shell, such as a
    /// Wayland tray host. Show the window before calling this method.
    ///
    /// Returns whether the platform submitted the request. The compositor may
    /// still deny focus. Returns false for empty tokens or unsupported platforms;
    /// callers can then fall back to [`Self::activate`]. Tokens must not
    /// be reused for subsequent activations.
    pub fn activate_with_token(&self, token: &str) -> bool {
        !token.is_empty() && self.platform_window.activate(Some(token))
    }

    /// Requests that the operating system draw attention to this window.
    pub fn request_attention(&self) {
        self.platform_window.request_attention();
    }

    /// Minimize the current window at the platform level.
    pub fn minimize_window(&self) {
        self.platform_window.minimize();
    }

    /// Show or hide the current window at the platform level.
    ///
    /// Call [`Window::activate`] separately when the window should also receive focus.
    /// The window manager may still focus a window when it is shown.
    pub fn set_visible(&self, visible: bool) {
        self.platform_window.set_visible(visible);
    }

    /// Toggle full screen status on the current window at the platform level.
    pub fn toggle_fullscreen(&self) {
        self.platform_window.toggle_fullscreen();
    }

    /// Toggle simple (borderless) fullscreen, where the window covers the entire
    /// screen including the menu bar and, on notched displays, the area around the
    /// notch. Unlike [`Window::toggle_fullscreen`], this does not move the window
    /// into its own Mission Control space. Only has an effect on macOS.
    pub fn toggle_simple_fullscreen(&self) {
        self.platform_window.toggle_simple_fullscreen();
    }

    /// Updates the IME panel position suggestions for languages like japanese, chinese.
    pub fn invalidate_character_coordinates(&self) {
        self.on_next_frame(|window, cx| {
            if let Some(mut input_handler) = window.platform_window.take_input_handler() {
                if let Some(bounds) = input_handler.selected_bounds(window, cx) {
                    window.platform_window.update_ime_position(bounds);
                }
                window.platform_window.set_input_handler(input_handler);
            }
        });
    }

    /// Present a platform dialog.
    /// The provided message will be presented, along with buttons for each answer.
    /// When a button is clicked, the returned Receiver will receive the index of the clicked button.
    pub fn prompt<T>(
        &mut self,
        level: PromptLevel,
        message: &str,
        detail: Option<&str>,
        answers: &[T],
        cx: &mut App,
    ) -> oneshot::Receiver<usize>
    where
        T: Clone + Into<PromptButton>,
    {
        let prompt_builder = cx.prompt_builder.take();
        let Some(prompt_builder) = prompt_builder else {
            unreachable!("Re-entrant window prompting is not supported by GPUI");
        };

        let answers = answers
            .iter()
            .map(|answer| answer.clone().into())
            .collect::<Vec<_>>();

        let receiver = match &prompt_builder {
            PromptBuilder::Default => self
                .platform_window
                .prompt(level, message, detail, &answers)
                .unwrap_or_else(|| {
                    self.build_custom_prompt(&prompt_builder, level, message, detail, &answers, cx)
                }),
            PromptBuilder::Custom(_) => {
                self.build_custom_prompt(&prompt_builder, level, message, detail, &answers, cx)
            }
        };

        cx.prompt_builder = Some(prompt_builder);

        receiver
    }

    fn build_custom_prompt(
        &mut self,
        prompt_builder: &PromptBuilder,
        level: PromptLevel,
        message: &str,
        detail: Option<&str>,
        answers: &[PromptButton],
        cx: &mut App,
    ) -> oneshot::Receiver<usize> {
        let (sender, receiver) = oneshot::channel();
        let handle = PromptHandle::new(sender);
        let handle = (prompt_builder)(level, message, detail, answers, handle, self, cx);
        self.prompt = Some(handle);
        receiver
    }

    /// Returns whether a prompt rendered by GPUI is currently active in this window.
    ///
    /// This is only true for prompts rendered in the window (see
    /// [`App::set_prompt_builder`]), not for platform-native prompt dialogs.
    pub fn has_active_prompt(&self) -> bool {
        self.prompt.is_some()
    }

    /// Returns the current context stack.
    pub fn context_stack(&self) -> Vec<KeyContext> {
        let node_id = self.focus_node_id_in_rendered_frame(self.focus);
        let dispatch_tree = &self.rendered_frame.dispatch_tree;
        dispatch_tree
            .dispatch_path(node_id)
            .iter()
            .filter_map(move |&node_id| dispatch_tree.node(node_id).context.clone())
            .collect()
    }

    /// Returns all available actions for the focused element.
    pub fn available_actions(&self, cx: &App) -> Vec<Box<dyn Action>> {
        let node_id = self.focus_node_id_in_rendered_frame(self.focus);
        let mut actions = self.rendered_frame.dispatch_tree.available_actions(node_id);
        for action_type in cx.global_action_listeners.keys() {
            if let Err(ix) = actions.binary_search_by_key(action_type, |a| a.as_any().type_id()) {
                let action = cx.actions.build_action_type(action_type).ok();
                if let Some(action) = action {
                    actions.insert(ix, action);
                }
            }
        }
        actions
    }

    /// Returns key bindings that invoke an action on the currently focused element. Bindings are
    /// returned in the order they were added. For display, the last binding should take precedence.
    pub fn bindings_for_action(&self, action: &dyn Action) -> Vec<KeyBinding> {
        self.rendered_frame
            .dispatch_tree
            .bindings_for_action(action, &self.rendered_frame.dispatch_tree.context_stack)
    }

    /// Returns the highest precedence key binding that invokes an action on the currently focused
    /// element. This is more efficient than getting the last result of `bindings_for_action`.
    pub fn highest_precedence_binding_for_action(&self, action: &dyn Action) -> Option<KeyBinding> {
        self.rendered_frame
            .dispatch_tree
            .highest_precedence_binding_for_action(
                action,
                &self.rendered_frame.dispatch_tree.context_stack,
            )
    }

    /// Returns the key bindings for an action in a context.
    pub fn bindings_for_action_in_context(
        &self,
        action: &dyn Action,
        context: KeyContext,
    ) -> Vec<KeyBinding> {
        let dispatch_tree = &self.rendered_frame.dispatch_tree;
        dispatch_tree.bindings_for_action(action, &[context])
    }

    /// Returns the highest precedence key binding for an action in a context. This is more
    /// efficient than getting the last result of `bindings_for_action_in_context`.
    pub fn highest_precedence_binding_for_action_in_context(
        &self,
        action: &dyn Action,
        context: KeyContext,
    ) -> Option<KeyBinding> {
        let dispatch_tree = &self.rendered_frame.dispatch_tree;
        dispatch_tree.highest_precedence_binding_for_action(action, &[context])
    }

    /// Returns any bindings that would invoke an action on the given focus handle if it were
    /// focused. Bindings are returned in the order they were added. For display, the last binding
    /// should take precedence.
    pub fn bindings_for_action_in(
        &self,
        action: &dyn Action,
        focus_handle: &FocusHandle,
    ) -> Vec<KeyBinding> {
        let dispatch_tree = &self.rendered_frame.dispatch_tree;
        let Some(context_stack) = self.context_stack_for_focus_handle(focus_handle) else {
            return vec![];
        };
        dispatch_tree.bindings_for_action(action, &context_stack)
    }

    /// Returns the highest precedence key binding that would invoke an action on the given focus
    /// handle if it were focused. This is more efficient than getting the last result of
    /// `bindings_for_action_in`.
    pub fn highest_precedence_binding_for_action_in(
        &self,
        action: &dyn Action,
        focus_handle: &FocusHandle,
    ) -> Option<KeyBinding> {
        let dispatch_tree = &self.rendered_frame.dispatch_tree;
        let context_stack = self.context_stack_for_focus_handle(focus_handle)?;
        dispatch_tree.highest_precedence_binding_for_action(action, &context_stack)
    }

    /// Find the bindings that can follow the current input sequence for the current context stack.
    pub fn possible_bindings_for_input(&self, input: &[Keystroke]) -> Vec<KeyBinding> {
        self.rendered_frame
            .dispatch_tree
            .possible_next_bindings_for_input(input, &self.context_stack())
    }

    fn context_stack_for_focus_handle(
        &self,
        focus_handle: &FocusHandle,
    ) -> Option<Vec<KeyContext>> {
        let dispatch_tree = &self.rendered_frame.dispatch_tree;
        let node_id = dispatch_tree.focusable_node_id(focus_handle.id)?;
        let context_stack: Vec<_> = dispatch_tree
            .dispatch_path(node_id)
            .into_iter()
            .filter_map(|node_id| dispatch_tree.node(node_id).context.clone())
            .collect();
        Some(context_stack)
    }

    /// Returns a generic event listener that invokes the given listener with the view and context associated with the given view handle.
    pub fn listener_for<T: 'static, E>(
        &self,
        view: &Entity<T>,
        f: impl Fn(&mut T, &E, &mut Window, &mut Context<T>) + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let view = view.downgrade();
        move |e: &E, window: &mut Window, cx: &mut App| {
            view.update(cx, |view, cx| f(view, e, window, cx)).ok();
        }
    }

    /// Returns a generic handler that invokes the given handler with the view and context associated with the given view handle.
    pub fn handler_for<E: 'static, Callback: Fn(&mut E, &mut Window, &mut Context<E>) + 'static>(
        &self,
        entity: &Entity<E>,
        f: Callback,
    ) -> impl Fn(&mut Window, &mut App) + 'static {
        let entity = entity.downgrade();
        move |window: &mut Window, cx: &mut App| {
            entity.update(cx, |entity, cx| f(entity, window, cx)).ok();
        }
    }

    /// Register a callback that can interrupt the closing of the current window based the returned boolean.
    /// If the callback returns false, the window won't be closed.
    pub fn on_window_should_close(
        &self,
        cx: &App,
        f: impl Fn(&mut Window, &mut App) -> bool + 'static,
    ) {
        let mut cx = self.to_async(cx);
        self.platform_window.on_should_close(Box::new(move || {
            cx.update(|window, cx| f(window, cx)).unwrap_or(true)
        }))
    }

    /// Register an action listener on this node for the next frame. The type of action
    /// is determined by the first parameter of the given listener. When the next frame is rendered
    /// the listener will be cleared.
    ///
    /// This is a fairly low-level method, so prefer using action handlers on elements unless you have
    /// a specific need to register a listener yourself.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn on_action(
        &mut self,
        action_type: TypeId,
        listener: impl Fn(&dyn Any, DispatchPhase, &mut Window, &mut App) + 'static,
    ) {
        self.invalidator.debug_assert_paint();

        self.next_frame
            .dispatch_tree
            .on_action(action_type, Rc::new(listener));
    }

    /// Register a capturing action listener on this node for the next frame if the condition is true.
    /// The type of action is determined by the first parameter of the given listener. When the next
    /// frame is rendered the listener will be cleared.
    ///
    /// This is a fairly low-level method, so prefer using action handlers on elements unless you have
    /// a specific need to register a listener yourself.
    ///
    /// This method should only be called as part of the paint phase of element drawing.
    pub fn on_action_when(
        &mut self,
        condition: bool,
        action_type: TypeId,
        listener: impl Fn(&dyn Any, DispatchPhase, &mut Window, &mut App) + 'static,
    ) {
        self.invalidator.debug_assert_paint();

        if condition {
            self.next_frame
                .dispatch_tree
                .on_action(action_type, Rc::new(listener));
        }
    }

    /// Read information about the GPU backing this window.
    /// Currently returns None on Mac and Windows.
    pub fn gpu_specs(&self) -> Option<GpuSpecs> {
        self.platform_window.gpu_specs()
    }

    /// Returns the GPU context (device + queue) if available.
    /// The returned `Box` contains `(Arc<wgpu::Device>, Arc<wgpu::Queue>)`.
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "windows",
        target_os = "macos"
    ))]
    pub fn gpu_context(&self) -> Option<Box<dyn std::any::Any>> {
        self.platform_window.gpu_context()
    }

    /// Returns backend-specific typed GPU context information for custom
    /// controls. Use the rendering backend's context type to downcast the
    /// returned value.
    #[cfg(any(
        target_family = "wasm",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "macos"
    ))]
    pub fn gpu_context_info(&self) -> Option<Box<dyn std::any::Any>> {
        self.platform_window.gpu_context_info()
    }

    /// Whether the GPU device backing this window has been lost (recovery
    /// happens on a subsequent platform draw). `None` when the backend
    /// cannot know. Embedders that captured the device from
    /// [`Self::gpu_context`] should stop submitting while this is
    /// `Some(true)` and re-acquire the device once it reads `Some(false)`.
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "windows",
        target_os = "macos"
    ))]
    pub fn gpu_device_lost(&self) -> Option<bool> {
        self.platform_window.gpu_device_lost()
    }

    /// Perform titlebar double-click action.
    /// This is macOS specific.
    pub fn titlebar_double_click(&self) {
        self.platform_window
            .titlebar_double_click(self.is_resizable, self.is_minimizable);
    }

    /// Gets the window's title at the platform level.
    /// This is macOS specific.
    pub fn window_title(&self) -> String {
        self.platform_window.get_title()
    }

    /// Returns a list of all tabbed windows and their titles.
    /// This is macOS specific.
    pub fn tabbed_windows(&self) -> Option<Vec<SystemWindowTab>> {
        self.platform_window.tabbed_windows()
    }

    /// Returns the tab bar visibility.
    /// This is macOS specific.
    pub fn tab_bar_visible(&self) -> bool {
        self.platform_window.tab_bar_visible()
    }

    /// Merges all open windows into a single tabbed window.
    /// This is macOS specific.
    pub fn merge_all_windows(&self) {
        self.platform_window.merge_all_windows()
    }

    /// Moves the tab to a new containing window.
    /// This is macOS specific.
    pub fn move_tab_to_new_window(&self) {
        self.platform_window.move_tab_to_new_window()
    }

    /// Shows or hides the window tab overview.
    /// This is macOS specific.
    pub fn toggle_window_tab_overview(&self) {
        self.platform_window.toggle_window_tab_overview()
    }

    /// Sets the tabbing identifier for the window.
    /// This is macOS specific.
    pub fn set_tabbing_identifier(&self, tabbing_identifier: Option<String>) {
        self.platform_window
            .set_tabbing_identifier(tabbing_identifier)
    }

    /// Request the OS to play an alert sound. On some platforms this is associated
    /// with the window, for others it's just a simple global function call.
    pub fn play_system_bell(&self) {
        self.platform_window.play_system_bell()
    }

    /// Returns whether accessibility features are active for this frame,
    /// i.e. whether assistive technology (such as a screen reader) is
    /// connected and an accessibility tree is being built.
    ///
    /// Use this to skip computing data during rendering that is only
    /// observable through the accessibility tree. When accessibility is
    /// activated, a redraw is forced, so gated work is recomputed before the
    /// next tree update is sent to the platform.
    ///
    /// See the [accessibility guide](crate::_accessibility) for an overview.
    pub fn is_a11y_active(&self) -> bool {
        self.a11y.is_active()
    }

    /// Build this window's accessibility tree every frame, even with no
    /// assistive technology connected, so [`Self::debug_a11y_tree_json`] and
    /// the accessibility actions work for automation. Forces a redraw, since
    /// the tree is built during prepaint. [`crate::Application::new_inaccessible`]
    /// still wins.
    pub fn set_a11y_forced(&mut self, forced: bool) {
        self.a11y.set_forced(forced);
        self.refresh();
    }

    /// Debug representation of the last frame's accessibility information.
    pub fn debug_a11y_tree_json(&self) -> Option<String> {
        self.a11y.debug_tree_json()
    }

    /// The accessibility tree built by the last frame, when one was built (see
    /// [`Self::is_a11y_active`] and [`Self::set_a11y_forced`]). Node ids are
    /// stable while the element identity is; bounds come from
    /// [`Self::a11y_node_bounds`].
    pub fn a11y_tree(&self) -> Option<&accesskit::TreeUpdate> {
        self.a11y.last_tree_update()
    }

    /// Window-space bounds, in logical pixels, of a node in the last built tree.
    pub fn a11y_node_bounds(&self, node: accesskit::NodeId) -> Option<Bounds<Pixels>> {
        self.a11y.last_node_bounds(node)
    }

    /// How many accessibility trees this window has built; zero before the
    /// first. Lets a caller tell a fresh tree from the one it already read.
    pub fn a11y_frame_number(&self) -> u64 {
        self.a11y.frame_number()
    }

    /// Register a listener for an accessibility action on a specific node.
    /// The listener will be called when a screen reader requests the given
    /// action on the node identified by `node_id`.
    ///
    /// See the [accessibility guide](crate::_accessibility) for an overview.
    pub fn on_a11y_action(
        &mut self,
        node_id: accesskit::NodeId,
        action: accesskit::Action,
        listener: impl FnMut(Option<&accesskit::ActionData>, &mut Window, &mut App) + 'static,
    ) {
        self.a11y
            .action_listeners
            .entry(node_id)
            .or_default()
            .push((action, Box::new(listener)));
    }

    /// Perform an accessibility action on a node as assistive technology
    /// would: a registered [`Self::on_a11y_action`] listener first, else the
    /// built-in handling (`Click` synthesizes a pointer click at the node's
    /// bounds, `Focus` focuses it, `Blur` blurs the window).
    #[cfg(not(target_family = "wasm"))]
    pub fn perform_a11y_action(
        &mut self,
        target: accesskit::NodeId,
        action: accesskit::Action,
        data: Option<accesskit::ActionData>,
        cx: &mut App,
    ) {
        self.handle_a11y_action(
            accesskit::ActionRequest {
                action,
                target_tree: accesskit::TreeId::ROOT,
                target_node: target,
                data,
            },
            cx,
        );
    }

    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn handle_a11y_action(&mut self, request: accesskit::ActionRequest, cx: &mut App) {
        // Take listeners out temporarily so the closures can borrow Window
        // mutably, then restore them afterward.
        if let Some(mut listeners) = self.a11y.action_listeners.remove(&request.target_node) {
            let extra_data = request.data.as_ref();
            let mut matched = false;
            for (action, listener) in &mut listeners {
                if *action == request.action {
                    listener(extra_data, self, cx);
                    matched = true;
                }
            }
            self.a11y
                .action_listeners
                .insert(request.target_node, listeners);
            if matched {
                return;
            }
        }

        // Fall back to built-in action handling.
        match request.action {
            accesskit::Action::Click => {
                if let Some(bounds) = self.a11y.node_bounds.get(&request.target_node).copied() {
                    let center = bounds.center();
                    let mouse_down = PlatformInput::MouseDown(crate::MouseDownEvent {
                        button: MouseButton::Left,
                        position: center,
                        modifiers: Modifiers::default(),
                        click_count: 1,
                        first_mouse: false,
                    });
                    let mouse_up = PlatformInput::MouseUp(MouseUpEvent {
                        button: MouseButton::Left,
                        position: center,
                        modifiers: Modifiers::default(),
                        click_count: 1,
                    });
                    self.dispatch_event(mouse_down, cx);
                    self.dispatch_event(mouse_up, cx);
                }
            }
            accesskit::Action::Focus => {
                if let Some(focus_id) = self.a11y.focus_ids.get(&request.target_node).copied()
                    && let Some(handle) = FocusHandle::for_id(focus_id, &cx.focus_handles)
                {
                    self.focus(&handle, cx);
                }
            }
            accesskit::Action::Blur => {
                self.blur(cx);
            }
            _ => {
                log::debug!(
                    "Unhandled a11y action: {:?} on {:?}",
                    request.action,
                    request.target_node
                );
            }
        }
    }

    /// Toggles the inspector mode on this window.
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub fn toggle_inspector(&mut self, cx: &mut App) {
        self.inspector = match self.inspector {
            None => Some(cx.new(|_| Inspector::new())),
            Some(_) => None,
        };
        self.refresh();
    }

    /// Returns true if the window is in inspector mode.
    pub fn is_inspector_picking(&self, _cx: &App) -> bool {
        #[cfg(any(feature = "inspector", debug_assertions))]
        {
            if let Some(inspector) = &self.inspector {
                return inspector.read(_cx).is_picking();
            }
        }
        false
    }

    /// Executes the provided function with mutable access to an inspector state.
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub fn with_inspector_state<T: 'static, R>(
        &mut self,
        _inspector_id: Option<&crate::InspectorElementId>,
        cx: &mut App,
        f: impl FnOnce(&mut Option<T>, &mut Self) -> R,
    ) -> R {
        if let Some(inspector_id) = _inspector_id
            && let Some(inspector) = &self.inspector
        {
            let inspector = inspector.clone();
            let active_element_id = inspector.read(cx).active_element_id();
            if Some(inspector_id) == active_element_id {
                return inspector.update(cx, |inspector, _cx| {
                    inspector.with_active_element_state(self, f)
                });
            }
        }
        f(&mut None, self)
    }

    #[cfg(any(feature = "inspector", debug_assertions))]
    pub(crate) fn build_inspector_element_id(
        &mut self,
        path: crate::InspectorElementPath,
    ) -> crate::InspectorElementId {
        self.invalidator.debug_assert_paint_or_prepaint();
        let path = Rc::new(path);
        let next_instance_id = self
            .next_frame
            .next_inspector_instance_ids
            .entry(path.clone())
            .or_insert(0);
        let instance_id = *next_instance_id;
        *next_instance_id += 1;
        crate::InspectorElementId { path, instance_id }
    }

    #[cfg(any(feature = "inspector", debug_assertions))]
    fn prepaint_inspector(&mut self, inspector_width: Pixels, cx: &mut App) -> Option<AnyElement> {
        if let Some(inspector) = self.inspector.take() {
            let mut inspector_element = AnyView::from(inspector.clone()).into_any_element();
            inspector_element.prepaint_as_root(
                point(self.viewport_size.width - inspector_width, px(0.0)),
                size(inspector_width, self.viewport_size.height).into(),
                self,
                cx,
            );
            self.inspector = Some(inspector);
            Some(inspector_element)
        } else {
            None
        }
    }

    #[cfg(any(feature = "inspector", debug_assertions))]
    fn paint_inspector(&mut self, mut inspector_element: Option<AnyElement>, cx: &mut App) {
        if let Some(mut inspector_element) = inspector_element {
            inspector_element.paint(self, cx);
        };
    }

    /// Registers a hitbox that can be used for inspector picking mode, allowing users to select and
    /// inspect UI elements by clicking on them.
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub fn insert_inspector_hitbox(
        &mut self,
        hitbox_id: HitboxId,
        inspector_id: Option<&crate::InspectorElementId>,
        cx: &App,
    ) {
        self.invalidator.debug_assert_paint_or_prepaint();
        if !self.is_inspector_picking(cx) {
            return;
        }
        if let Some(inspector_id) = inspector_id {
            self.next_frame
                .inspector_hitboxes
                .insert(hitbox_id, inspector_id.clone());
        }
    }

    #[cfg(any(feature = "inspector", debug_assertions))]
    fn paint_inspector_hitbox(&mut self, cx: &App) {
        if let Some(inspector) = self.inspector.as_ref() {
            let inspector = inspector.read(cx);
            if let Some((hitbox_id, _)) = self.hovered_inspector_hitbox(inspector, &self.next_frame)
                && let Some(hitbox) = self
                    .next_frame
                    .hitboxes
                    .iter()
                    .find(|hitbox| hitbox.id == hitbox_id)
            {
                self.paint_quad(PaintQuad::new(hitbox.bounds).background(crate::rgba(0x61afef4d)));
            }
        }
    }

    #[cfg(any(feature = "inspector", debug_assertions))]
    fn handle_inspector_mouse_event(&mut self, event: &dyn Any, cx: &mut App) {
        let Some(inspector) = self.inspector.clone() else {
            return;
        };
        if event.downcast_ref::<MouseMoveEvent>().is_some() {
            inspector.update(cx, |inspector, _cx| {
                if let Some((_, inspector_id)) =
                    self.hovered_inspector_hitbox(inspector, &self.rendered_frame)
                {
                    inspector.hover(inspector_id, self);
                }
            });
        } else if event.downcast_ref::<crate::MouseDownEvent>().is_some() {
            inspector.update(cx, |inspector, _cx| {
                if let Some((_, inspector_id)) =
                    self.hovered_inspector_hitbox(inspector, &self.rendered_frame)
                {
                    inspector.select(inspector_id, self);
                }
            });
        } else if let Some(event) = event.downcast_ref::<crate::ScrollWheelEvent>() {
            // This should be kept in sync with SCROLL_LINES in x11 platform.
            const SCROLL_LINES: f32 = 3.0;
            const SCROLL_PIXELS_PER_LAYER: f32 = 36.0;
            let delta_y = event
                .delta
                .pixel_delta(px(SCROLL_PIXELS_PER_LAYER / SCROLL_LINES))
                .y;
            if let Some(inspector) = self.inspector.clone() {
                inspector.update(cx, |inspector, _cx| {
                    if let Some(depth) = inspector.pick_depth.as_mut() {
                        *depth += f32::from(delta_y) / SCROLL_PIXELS_PER_LAYER;
                        let max_depth = self.mouse_hit_test.entries.len() as f32 - 0.5;
                        if *depth < 0.0 {
                            *depth = 0.0;
                        } else if *depth > max_depth {
                            *depth = max_depth;
                        }
                        if let Some((_, inspector_id)) =
                            self.hovered_inspector_hitbox(inspector, &self.rendered_frame)
                        {
                            inspector.set_active_element_id(inspector_id, self);
                        }
                    }
                });
            }
        }
    }

    #[cfg(any(feature = "inspector", debug_assertions))]
    fn hovered_inspector_hitbox(
        &self,
        inspector: &Inspector,
        frame: &Frame,
    ) -> Option<(HitboxId, crate::InspectorElementId)> {
        if let Some(pick_depth) = inspector.pick_depth {
            let depth = (pick_depth as i64).try_into().unwrap_or(0);
            let max_skipped = self.mouse_hit_test.entries.len().saturating_sub(1);
            let skip_count = (depth as usize).min(max_skipped);
            for hitbox_id in self.mouse_hit_test.ordered_ids.iter().skip(skip_count) {
                if let Some(inspector_id) = frame.inspector_hitboxes.get(hitbox_id) {
                    return Some((*hitbox_id, inspector_id.clone()));
                }
            }
        }
        None
    }

    /// For testing: set the current modifier keys state.
    /// This does not generate any events.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_modifiers(&mut self, modifiers: Modifiers) {
        self.modifiers = modifiers;
    }

    /// For testing: simulate a mouse move event to the given position.
    /// This dispatches the event through the normal event handling path,
    /// which will trigger hover states and tooltips.
    #[cfg(any(test, feature = "test-support"))]
    pub fn simulate_mouse_move(&mut self, position: Point<Pixels>, cx: &mut App) {
        let event = PlatformInput::MouseMove(MouseMoveEvent {
            position,
            modifiers: self.modifiers,
            pressed_button: None,
        });
        let _ = self.dispatch_event(event, cx);
    }
}

// #[derive(Clone, Copy, Eq, PartialEq, Hash)]
slotmap::new_key_type! {
    /// A unique identifier for a window.
    pub struct WindowId;
}

impl WindowId {
    /// Converts this window ID to a `u64`.
    pub fn as_u64(&self) -> u64 {
        self.0.as_ffi()
    }
}

impl From<u64> for WindowId {
    fn from(value: u64) -> Self {
        WindowId(slotmap::KeyData::from_ffi(value))
    }
}

/// A handle to a window with a specific root view type.
/// Note that this does not keep the window alive on its own.
#[derive(Deref, DerefMut)]
pub struct WindowHandle<V> {
    #[deref]
    #[deref_mut]
    pub(crate) any_handle: AnyWindowHandle,
    state_type: PhantomData<fn(V) -> V>,
}

impl<V> Debug for WindowHandle<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowHandle")
            .field("any_handle", &self.any_handle.id.as_u64())
            .finish()
    }
}

impl<V: 'static + Render> WindowHandle<V> {
    /// Creates a new handle from a window ID.
    /// This does not check if the root type of the window is `V`.
    pub fn new(id: WindowId) -> Self {
        WindowHandle {
            any_handle: AnyWindowHandle {
                id,
                state_type: TypeId::of::<V>(),
                root_entity_type_name: std::any::type_name::<V>(),
            },
            state_type: PhantomData,
        }
    }

    /// Get the root view out of this window.
    ///
    /// This will fail if the window is closed or if the root view's type does not match `V`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn root<C>(&self, cx: &mut C) -> Result<Entity<V>>
    where
        C: AppContext,
    {
        cx.update_window(self.any_handle, |root_view, _, _| {
            root_view
                .downcast::<V>()
                .map_err(|_| anyhow!("the type of the window's root view has changed"))
        })?
    }

    /// Updates the root view of this window.
    ///
    /// This will fail if the window has been closed or if the root view's type does not match
    pub fn update<C, R>(
        &self,
        cx: &mut C,
        update: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R,
    ) -> Result<R>
    where
        C: AppContext,
    {
        cx.update_window(self.any_handle, |root_view, window, cx| {
            let view = root_view
                .downcast::<V>()
                .map_err(|_| anyhow!("the type of the window's root view has changed"))?;

            Ok(view.update(cx, |view, cx| update(view, window, cx)))
        })?
    }

    /// Read the root view out of this window.
    ///
    /// This will fail if the window is closed or if the root view's type does not match `V`.
    pub fn read<'a>(&self, cx: &'a App) -> Result<&'a V> {
        let x = cx
            .windows
            .get(self.id)
            .and_then(|window| {
                window
                    .as_deref()
                    .and_then(|window| window.root.clone())
                    .map(|root_view| root_view.downcast::<V>())
            })
            .context("window not found")?
            .map_err(|_| anyhow!("the type of the window's root view has changed"))?;

        Ok(x.read(cx))
    }

    /// Read the root view out of this window, with a callback
    ///
    /// This will fail if the window is closed or if the root view's type does not match `V`.
    pub fn read_with<C, R>(&self, cx: &C, read_with: impl FnOnce(&V, &App) -> R) -> Result<R>
    where
        C: AppContext,
    {
        cx.read_window(self, |root_view, cx| read_with(root_view.read(cx), cx))
    }

    /// Read the root view pointer off of this window.
    ///
    /// This will fail if the window is closed or if the root view's type does not match `V`.
    pub fn entity<C>(&self, cx: &C) -> Result<Entity<V>>
    where
        C: AppContext,
    {
        cx.read_window(self, |root_view, _cx| root_view)
    }

    /// Check if this window is 'active'.
    ///
    /// Will return `None` if the window is closed or currently
    /// borrowed.
    pub fn is_active(&self, cx: &mut App) -> Option<bool> {
        cx.update_window(self.any_handle, |_, window, _| window.is_window_active())
            .ok()
    }
}

impl<V> Copy for WindowHandle<V> {}

impl<V> Clone for WindowHandle<V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V> PartialEq for WindowHandle<V> {
    fn eq(&self, other: &Self) -> bool {
        self.any_handle == other.any_handle
    }
}

impl<V> Eq for WindowHandle<V> {}

impl<V> Hash for WindowHandle<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.any_handle.hash(state);
    }
}

impl<V: 'static> From<WindowHandle<V>> for AnyWindowHandle {
    fn from(val: WindowHandle<V>) -> Self {
        val.any_handle
    }
}

/// A handle to a window with any root view type, which can be downcast to a window with a specific root view type.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct AnyWindowHandle {
    pub(crate) id: WindowId,
    state_type: TypeId,
    root_entity_type_name: &'static str,
}

impl AnyWindowHandle {
    /// Get the ID of this window.
    pub fn window_id(&self) -> WindowId {
        self.id
    }

    /// Returns the name of the window's declared root entity type.
    pub fn root_entity_type_name(&self) -> &'static str {
        self.root_entity_type_name
    }

    /// Attempt to convert this handle to a window handle with a specific root view type.
    /// If the types do not match, this will return `None`.
    pub fn downcast<T: 'static>(&self) -> Option<WindowHandle<T>> {
        if TypeId::of::<T>() == self.state_type {
            Some(WindowHandle {
                any_handle: *self,
                state_type: PhantomData,
            })
        } else {
            None
        }
    }

    /// Updates the state of the root view of this window.
    ///
    /// This will fail if the window has been closed.
    pub fn update<C, R>(
        self,
        cx: &mut C,
        update: impl FnOnce(AnyView, &mut Window, &mut App) -> R,
    ) -> Result<R>
    where
        C: AppContext,
    {
        cx.update_window(self, update)
    }

    /// Read the state of the root view of this window.
    ///
    /// This will fail if the window has been closed.
    pub fn read<T, C, R>(self, cx: &C, read: impl FnOnce(Entity<T>, &App) -> R) -> Result<R>
    where
        C: AppContext,
        T: 'static,
    {
        let view = self
            .downcast::<T>()
            .context("the type of the window's root view has changed")?;

        cx.read_window(&view, read)
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, HandleError> {
        self.platform_window.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, HandleError> {
        self.platform_window.display_handle()
    }
}

/// An identifier for an [`Element`].
///
/// Can be constructed with a string, a number, or both, as well
/// as other internal representations.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ElementId {
    /// The ID of a View element
    View(EntityId),
    /// An integer ID.
    Integer(u64),
    /// A string based ID.
    Name(SharedString),
    /// A UUID.
    Uuid(Uuid),
    /// An ID that's equated with a focus handle.
    FocusHandle(FocusId),
    /// A combination of a name and an integer.
    NamedInteger(SharedString, u64),
    /// A path.
    Path(Arc<std::path::Path>),
    /// A code location.
    CodeLocation(core::panic::Location<'static>),
    /// A labeled child of an element.
    NamedChild(Arc<ElementId>, SharedString),
    /// A byte array ID (used for text-anchors)
    OpaqueId([u8; 20]),
}

impl ElementId {
    /// Constructs an `ElementId::NamedInteger` from a name and `usize`.
    pub fn named_usize(name: impl Into<SharedString>, integer: usize) -> ElementId {
        Self::NamedInteger(name.into(), integer as u64)
    }
}

impl Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementId::View(entity_id) => write!(f, "view-{}", entity_id)?,
            ElementId::Integer(ix) => write!(f, "{}", ix)?,
            ElementId::Name(name) => write!(f, "{}", name)?,
            ElementId::FocusHandle(_) => write!(f, "FocusHandle")?,
            ElementId::NamedInteger(s, i) => write!(f, "{}-{}", s, i)?,
            ElementId::Uuid(uuid) => write!(f, "{}", uuid)?,
            ElementId::Path(path) => write!(f, "{}", path.display())?,
            ElementId::CodeLocation(location) => write!(f, "{}", location)?,
            ElementId::NamedChild(id, name) => write!(f, "{}-{}", id, name)?,
            ElementId::OpaqueId(opaque_id) => write!(f, "{:x?}", opaque_id)?,
        }

        Ok(())
    }
}

impl TryInto<SharedString> for ElementId {
    type Error = anyhow::Error;

    fn try_into(self) -> anyhow::Result<SharedString> {
        if let ElementId::Name(name) = self {
            Ok(name)
        } else {
            anyhow::bail!("element id is not string")
        }
    }
}

impl From<usize> for ElementId {
    fn from(id: usize) -> Self {
        ElementId::Integer(id as u64)
    }
}

impl From<i32> for ElementId {
    fn from(id: i32) -> Self {
        Self::Integer(id as u64)
    }
}

impl From<SharedString> for ElementId {
    fn from(name: SharedString) -> Self {
        ElementId::Name(name)
    }
}

impl From<String> for ElementId {
    fn from(name: String) -> Self {
        ElementId::Name(name.into())
    }
}

impl From<Arc<str>> for ElementId {
    fn from(name: Arc<str>) -> Self {
        ElementId::Name(name.into())
    }
}

impl From<Arc<std::path::Path>> for ElementId {
    fn from(path: Arc<std::path::Path>) -> Self {
        ElementId::Path(path)
    }
}

impl From<&'static str> for ElementId {
    fn from(name: &'static str) -> Self {
        ElementId::Name(SharedString::new_static(name))
    }
}

impl<'a> From<&'a FocusHandle> for ElementId {
    fn from(handle: &'a FocusHandle) -> Self {
        ElementId::FocusHandle(handle.id)
    }
}

impl From<(&'static str, EntityId)> for ElementId {
    fn from((name, id): (&'static str, EntityId)) -> Self {
        ElementId::NamedInteger(SharedString::new_static(name), id.as_u64())
    }
}

impl From<(&'static str, usize)> for ElementId {
    fn from((name, id): (&'static str, usize)) -> Self {
        ElementId::NamedInteger(SharedString::new_static(name), id as u64)
    }
}

impl From<(SharedString, usize)> for ElementId {
    fn from((name, id): (SharedString, usize)) -> Self {
        ElementId::NamedInteger(name, id as u64)
    }
}

impl From<(&'static str, u64)> for ElementId {
    fn from((name, id): (&'static str, u64)) -> Self {
        ElementId::NamedInteger(SharedString::new_static(name), id)
    }
}

impl From<Uuid> for ElementId {
    fn from(value: Uuid) -> Self {
        Self::Uuid(value)
    }
}

impl From<(&'static str, u32)> for ElementId {
    fn from((name, id): (&'static str, u32)) -> Self {
        ElementId::NamedInteger(SharedString::new_static(name), u64::from(id))
    }
}

impl<T: Into<SharedString>> From<(ElementId, T)> for ElementId {
    fn from((id, name): (ElementId, T)) -> Self {
        ElementId::NamedChild(Arc::new(id), name.into())
    }
}

impl From<&'static core::panic::Location<'static>> for ElementId {
    fn from(location: &'static core::panic::Location<'static>) -> Self {
        ElementId::CodeLocation(*location)
    }
}

impl From<[u8; 20]> for ElementId {
    fn from(opaque_id: [u8; 20]) -> Self {
        ElementId::OpaqueId(opaque_id)
    }
}

/// A rectangle to be rendered in the window at the given position and size.
/// Passed as an argument [`Window::paint_quad`].
#[derive(Clone, derive_setters::Setters)]
#[setters(into, generate = false)]
pub struct PaintQuad {
    /// The bounds of the quad within the window.
    pub bounds: Bounds<Pixels>,
    /// Whether to snap bounds and border widths to device pixels. Disable for animated rings.
    pub pixel_snap: bool,
    /// The radii of the quad's corners.
    #[setters(generate)]
    pub corner_radii: Corners<Pixels>,
    /// Corner smoothing, from circular corners at `0.0` to maximum smoothing at `1.0`.
    pub corner_smoothing: f32,
    /// The background color of the quad.
    #[setters(generate)]
    pub background: Background,
    /// The widths of the quad's borders.
    #[setters(generate)]
    pub border_widths: Edges<Pixels>,
    /// The background painted into the quad's borders.
    #[setters(generate)]
    pub border_color: Background,
    /// The style of the quad's borders.
    #[setters(generate)]
    pub border_style: BorderStyle,
    /// The length of each border dash, as a multiple of the border width.
    pub border_dashed_length: f32,
    /// The gap between border dashes, as a multiple of the border width.
    pub border_dashed_gap: f32,
}

impl PaintQuad {
    /// Creates a transparent quad with no border or rounded corners.
    pub fn new(bounds: impl Into<Bounds<Pixels>>) -> Self {
        Self {
            bounds: bounds.into(),
            pixel_snap: true,
            corner_radii: (0.).into(),
            corner_smoothing: 0.0,
            background: transparent_black().into(),
            border_widths: (0.).into(),
            border_color: transparent_black().into(),
            border_style: BorderStyle::default(),
            border_dashed_length: crate::scene::DEFAULT_BORDER_DASHED_LENGTH,
            border_dashed_gap: crate::scene::DEFAULT_BORDER_DASHED_GAP,
        }
    }

    /// Controls device-pixel snapping. Fractional geometry is antialiased by the renderer.
    pub fn pixel_snap(mut self, enabled: bool) -> Self {
        self.pixel_snap = enabled;
        self
    }

    /// Sets the corner smoothing, clamped to the range `0.0..=1.0`.
    pub fn corner_smoothing(self, amount: f32) -> Self {
        PaintQuad {
            corner_smoothing: amount.clamp(0.0, 1.0),
            ..self
        }
    }

    /// Sets the length of each border dash as a multiple of the border width.
    pub fn border_dashed_length(self, length_per_border_width: f32) -> Self {
        PaintQuad {
            border_dashed_length: length_per_border_width.max(0.0),
            ..self
        }
    }

    /// Sets the gap between border dashes as a multiple of the border width.
    pub fn border_dashed_gap(self, gap_per_border_width: f32) -> Self {
        PaintQuad {
            border_dashed_gap: gap_per_border_width.max(0.0),
            ..self
        }
    }
}

/// Creates a quad with the given parameters.
#[deprecated(note = "replaced with builder syntax for PaintQuad")]
pub fn quad(
    bounds: Bounds<Pixels>,
    corner_radii: impl Into<Corners<Pixels>>,
    background: impl Into<Background>,
    border_widths: impl Into<Edges<Pixels>>,
    border_color: impl Into<Background>,
    border_style: BorderStyle,
) -> PaintQuad {
    PaintQuad::new(bounds)
        .corner_radii(corner_radii)
        .background(background)
        .border_widths(border_widths)
        .border_color(border_color)
        .border_style(border_style)
}

/// Creates a filled quad with the given bounds and background color.
#[deprecated(note = "replaced with builder syntax for PaintQuad")]
pub fn fill(bounds: impl Into<Bounds<Pixels>>, background: impl Into<Background>) -> PaintQuad {
    PaintQuad::new(bounds).background(background)
}

/// Creates a rectangle outline with the given bounds, border color, and a 1px border width
#[deprecated(note = "replaced with builder syntax for PaintQuad")]
pub fn outline(
    bounds: impl Into<Bounds<Pixels>>,
    border_color: impl Into<Background>,
    border_style: BorderStyle,
) -> PaintQuad {
    PaintQuad::new(bounds)
        .border_widths(1f32)
        .border_color(border_color)
        .border_style(border_style)
}

#[cfg(test)]
mod pointer_tests;

#[cfg(test)]
#[path = "window/svg_tests.rs"]
mod svg_current_color_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::FluentBuilder;
    use crate::{
        ColorExt, DragMoveEvent, Empty, ExternalDragPayload, ExternalPaths, FileDragPaths,
        FocusHandle, Font, FontMetrics, ImageSource, InlineLayout, InlineLayoutRequest, InputEvent,
        InteractiveElement, LineLayout, LongPressEvent, MouseDownEvent, ParentElement,
        PlatformTextSystem, PreparedRasterStyle, RasterColorEffect, RasterStyleRequest,
        RasterizedGlyph, RequestFrameOptions, ShaderBool, StatefulInteractiveElement, Styled,
        TestApp, TestAppContext, TestTextSystem, TextLayoutRequest, TouchDragEvent, TouchId,
        TouchPhase, canvas, div, hsla, img, linear_color_stop, linear_gradient,
    };
    use image::{Frame as ImageFrame, ImageBuffer, Rgba};
    use proptest::prelude::*;
    use smallvec::smallvec;
    use std::{path::PathBuf, sync::Mutex as StdMutex};

    #[test]
    fn hit_test_preserves_inline_regions_occlusion_and_metadata() {
        let bounds = Bounds::new(Point::default(), size(px(100.), px(80.)));
        let background = Hitbox {
            id: HitboxId(1),
            bounds,
            content_mask: ContentMask {
                bounds,
                ..Default::default()
            },
            behavior: HitboxBehavior::Normal,
            tags: vec!["background".into()],
            fragments: None,
        };
        let mut inline = Hitbox {
            id: HitboxId(2),
            bounds,
            content_mask: ContentMask {
                bounds: Bounds::new(Point::default(), size(px(80.), px(80.))),
                ..Default::default()
            },
            behavior: HitboxBehavior::Normal,
            tags: vec!["inline".into()],
            fragments: Some(Arc::from([
                Bounds::new(point(px(60.), px(0.)), size(px(40.), px(40.))),
                Bounds::new(point(px(0.), px(40.)), size(px(40.), px(40.))),
            ])),
        };

        for behavior in [
            HitboxBehavior::Normal,
            HitboxBehavior::BlockMouseExceptScroll,
            HitboxBehavior::BlockMouse,
        ] {
            inline.behavior = behavior;

            for (position, inside_inline, inside_background) in [
                (point(px(70.), px(20.)), true, true),
                (point(px(20.), px(60.)), true, true),
                (point(px(20.), px(20.)), false, true),
                (point(px(90.), px(20.)), false, true),
                (point(px(120.), px(20.)), false, false),
            ] {
                let hit_test = HitTest::new([&inline, &background].into_iter(), position);
                let mut hovered = Vec::new();
                let mut scrollable = Vec::new();

                if inside_inline {
                    hovered.push(inline.id);
                    scrollable.push(inline.id);
                }

                if inside_background && (!inside_inline || behavior == HitboxBehavior::Normal) {
                    hovered.push(background.id);
                }

                if inside_background && (!inside_inline || behavior != HitboxBehavior::BlockMouse) {
                    scrollable.push(background.id);
                }

                assert_eq!(
                    hit_test.iter_hovered().copied().collect::<Vec<_>>(),
                    hovered
                );
                assert_eq!(
                    hit_test.iter_scrollable().copied().collect::<Vec<_>>(),
                    scrollable
                );

                for (depth, hitbox) in [&inline, &background].into_iter().enumerate() {
                    let entry = hit_test.entry(&hitbox.id).unwrap();
                    assert_eq!(entry.depth(), depth);
                    assert_eq!(entry.tags(), &hitbox.tags);
                }
            }
        }
    }

    proptest! {
        #[test]
        fn glyph_origin_quantization_preserves_position_with_bounded_error(
            x in -10_000.0f32..10_000.0,
            y in -10_000.0f32..10_000.0,
        ) {
            let (integer, variant) =
                quantize_glyph_origin(point(ScaledPixels(x), ScaledPixels(y)));
            let reconstructed_x =
                integer.x.0 + f32::from(variant.x) / f32::from(SUBPIXEL_VARIANTS_X);
            let reconstructed_y =
                integer.y.0 + f32::from(variant.y) / f32::from(SUBPIXEL_VARIANTS_Y);
            let maximum_error_x = 0.5 / f32::from(SUBPIXEL_VARIANTS_X) + 0.001;
            let maximum_error_y = 0.5 / f32::from(SUBPIXEL_VARIANTS_Y) + 0.001;

            prop_assert!(variant.x < SUBPIXEL_VARIANTS_X);
            prop_assert!(variant.y < SUBPIXEL_VARIANTS_Y);
            prop_assert_eq!(variant.y, 0, "vertical glyph variants change baseline snapping");
            prop_assert!((reconstructed_x - x).abs() <= maximum_error_x);
            prop_assert!((reconstructed_y - y).abs() <= maximum_error_y);

            let (color_origin, color_variant) =
                quantize_color_glyph_origin(point(ScaledPixels(x), ScaledPixels(y)));
            prop_assert_eq!(color_variant, Point::default());
            prop_assert!((color_origin.x.0 - x).abs() <= 0.501);
            prop_assert!((color_origin.y.0 - y).abs() <= 0.501);
        }
    }

    #[derive(Default)]
    struct RasterFormatTextSystem {
        rasterized: StdMutex<Vec<RenderGlyphParams>>,
        fail_next: AtomicBool,
    }

    impl PlatformTextSystem for RasterFormatTextSystem {
        fn add_fonts(&self, _fonts: Vec<Cow<'static, [u8]>>) -> anyhow::Result<()> {
            Ok(())
        }

        fn all_font_names(&self) -> Vec<String> {
            Vec::new()
        }

        fn font_id(&self, descriptor: &Font) -> anyhow::Result<FontId> {
            PlatformTextSystem::font_id(&TestTextSystem, descriptor)
        }

        fn font_metrics(&self, font_id: FontId) -> FontMetrics {
            PlatformTextSystem::font_metrics(&TestTextSystem, font_id)
        }

        fn typographic_bounds(
            &self,
            font_id: FontId,
            glyph_id: GlyphId,
        ) -> anyhow::Result<Bounds<f32>> {
            PlatformTextSystem::typographic_bounds(&TestTextSystem, font_id, glyph_id)
        }

        fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> anyhow::Result<Size<f32>> {
            PlatformTextSystem::advance(&TestTextSystem, font_id, glyph_id)
        }

        fn glyph_for_char(&self, font_id: FontId, character: char) -> Option<GlyphId> {
            PlatformTextSystem::glyph_for_char(&TestTextSystem, font_id, character)
        }

        fn rasterize_glyph(&self, params: &RenderGlyphParams) -> anyhow::Result<RasterizedGlyph> {
            self.rasterized.lock().unwrap().push(params.clone());
            if self.fail_next.swap(false, SeqCst) {
                anyhow::bail!("injected glyph rasterization failure");
            }

            let (format, pixels) = match params.glyph_id.0 {
                1 => (RasterizedGlyphFormat::AlphaMask, vec![0, 255]),
                2 => (
                    RasterizedGlyphFormat::BgraSubpixelMask,
                    vec![1, 2, 3, 0, 4, 5, 6, 0],
                ),
                3 | 5 => (
                    RasterizedGlyphFormat::BgraColor,
                    vec![10, 20, 30, 128, 40, 50, 60, 255],
                ),
                node_id => anyhow::bail!("unexpected scripted glyph {node_id}"),
            };
            let size = size(DevicePixels(2), DevicePixels(1));
            Ok(RasterizedGlyph {
                bounds: Bounds {
                    origin: point(DevicePixels(-1), DevicePixels(-2)),
                    size,
                },
                size,
                format,
                pixels,
            })
        }

        fn prepare_raster_style(&self, mut request: RasterStyleRequest) -> PreparedRasterStyle {
            if request.requested_mode == GlyphRenderMode::Color {
                request.foreground_dependency = match request.glyph_id {
                    GlyphId(3) => crate::ForegroundDependency::AlphaOnly,
                    _glyph_id => request.foreground_dependency,
                };

                PreparedRasterStyle::preblend(request)
            } else {
                PreparedRasterStyle::independent(request.requested_mode)
            }
        }

        fn layout_text(&self, request: TextLayoutRequest<'_>) -> LineLayout {
            PlatformTextSystem::layout_text(&TestTextSystem, request)
        }

        fn layout_inline(&self, request: InlineLayoutRequest<'_>) -> InlineLayout {
            PlatformTextSystem::layout_inline(&TestTextSystem, request)
        }
    }

    struct RasterFormatView;

    impl Render for RasterFormatView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let color = hsla(0.6, 0.7, 0.4, 0.8);
            let alternate_color = hsla(0.1, 0.6, 0.3, 0.8);
            div().size_full().opacity(0.5).child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        for (index, glyph_id) in [GlyphId(1), GlyphId(2)].into_iter().enumerate() {
                            let offset = px(index as f32 * 5.0);
                            let mut origin = point(px(5.0) + offset, px(10.0) + offset);

                            if index == 0 {
                                origin += point(px(0.13), px(0.245));
                            }

                            window
                                .paint_glyph(origin, FontId(7), glyph_id, px(16.0), color)
                                .unwrap();
                        }

                        let mut index = 0;

                        for glyph_id in [GlyphId(3), GlyphId(5)] {
                            for foreground in [color, alternate_color] {
                                let offset = px(index as f32 * 5.0);
                                let origin = point(px(15.0) + offset, px(20.0) + offset);

                                window
                                    .paint_emoji_with_color(
                                        origin,
                                        FontId(7),
                                        glyph_id,
                                        px(16.0),
                                        foreground,
                                    )
                                    .unwrap();
                                index += 1;
                            }
                        }
                    },
                )
                .size_full(),
            )
        }
    }

    #[test]
    fn returned_raster_format_drives_atlas_and_scene_behavior() {
        let text_system = Arc::new(RasterFormatTextSystem::default());
        let mut app = TestApp::with_text_system(text_system.clone());
        let mut test_window = app.open_window(|_, _| RasterFormatView);
        test_window.draw();

        test_window.update(|_, window, _| {
            assert_eq!(window.rendered_primitive_counts(), (0, 2, 0, 4));
            let scene = &window.rendered_frame.scene;
            let expected_color = hsla(0.6, 0.7, 0.4, 0.8).into();

            assert_eq!(scene.monochrome_sprites[0].color, expected_color);
            assert_eq!(scene.monochrome_sprites[1].color, expected_color);
            assert!(
                scene
                    .polychrome_sprites
                    .iter()
                    .all(|sprite| sprite.opacity == 1.0)
            );
            assert_eq!(
                scene.monochrome_sprites[0].bounds.origin,
                point(ScaledPixels(9.0), ScaledPixels(18.0))
            );
            assert_eq!(
                scene.monochrome_sprites[0].tile.texture_id.kind,
                crate::AtlasTextureKind::Monochrome
            );
            assert_eq!(
                scene.monochrome_sprites[1].tile.texture_id.kind,
                crate::AtlasTextureKind::Subpixel
            );
            assert_eq!(
                scene.polychrome_sprites[0].tile.texture_id.kind,
                crate::AtlasTextureKind::Polychrome
            );
            let first_tile = scene.polychrome_sprites[0].tile;
            let second_tile = scene.polychrome_sprites[1].tile;
            let third_tile = scene.polychrome_sprites[2].tile;
            let fourth_tile = scene.polychrome_sprites[3].tile;
            assert_eq!(first_tile, second_tile);
            assert_ne!(third_tile, fourth_tile);
        });

        let rasterized = text_system.rasterized.lock().unwrap();
        let color_styles = rasterized
            .iter()
            .filter(|params| params.raster_style.mode == GlyphRenderMode::Color)
            .map(|params| params.raster_style.color_effect)
            .collect::<Vec<_>>();
        assert_eq!(
            color_styles,
            [
                RasterColorEffect::Preblend(crate::Rgba8::new(0, 0, 0, 204)),
                RasterColorEffect::Preblend(crate::Rgba8::new(31, 88, 173, 204)),
                RasterColorEffect::Preblend(crate::Rgba8::new(122, 86, 31, 204)),
            ]
        );
    }

    struct FragmentFailureView;

    impl Render for FragmentFailureView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child("x😀")
        }
    }

    #[test]
    fn a_failed_glyph_does_not_hide_the_rest_of_its_fragment() {
        let text_system = Arc::new(RasterFormatTextSystem {
            rasterized: StdMutex::default(),
            fail_next: AtomicBool::new(true),
        });
        let mut app = TestApp::with_text_system(text_system.clone());
        let mut test_window = app.open_window(|_, _| FragmentFailureView);
        test_window.draw();

        test_window.update(|_, window, _| {
            assert!(!window.rendered_frame.scene.subpixel_sprites.is_empty());
        });

        let glyph_ids = text_system
            .rasterized
            .lock()
            .unwrap()
            .iter()
            .map(|params| params.glyph_id)
            .collect::<Vec<_>>();
        assert!(
            glyph_ids.starts_with(&[GlyphId(1), GlyphId(2)]),
            "unexpected rasterization sequence: {glyph_ids:?}"
        );
    }

    struct EmptyView;

    impl Render for EmptyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn shadow_gradient() -> Background {
        linear_gradient(
            90.,
            linear_color_stop(hsla(0., 1., 0.5, 0.8), 0.),
            linear_color_stop(hsla(2. / 3., 1., 0.5, 0.6), 1.),
        )
    }

    struct ShadowBackgroundView;

    impl Render for ShadowBackgroundView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .m(px(40.))
                .w(px(80.))
                .h(px(60.))
                .rounded(px(8.))
                .bg(white())
                .opacity(0.5)
                .shadow(vec![
                    BoxShadow::new(px(4.), px(2.), shadow_gradient())
                        .blur_radius(px(6.))
                        .spread_radius(px(2.)),
                    BoxShadow::new(px(-3.), px(1.), hsla(0.3, 0.7, 0.4, 0.4)),
                    BoxShadow::new(px(0.), px(2.), shadow_gradient())
                        .blur_radius(px(4.))
                        .spread_radius(px(3.))
                        .inset(),
                ])
        }
    }

    struct CornerSmoothingSceneView {
        image: Arc<RenderImage>,
    }

    fn assert_smoothing(name: &str, minimum_count: usize, values: impl IntoIterator<Item = f32>) {
        let values = values.into_iter().collect::<Vec<_>>();
        assert!(values.len() >= minimum_count, "missing {name}");
        assert!(
            values.iter().all(|&value| value == 1.0),
            "{name}: {values:?}"
        );
    }

    impl Render for CornerSmoothingSceneView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut image = img(ImageSource::Render(self.image.clone()))
                .size(px(24.))
                .rounded(px(8.));
            image.style().corner_smoothing = Some(1.5);

            let mut root = div()
                .size(px(80.))
                .rounded(px(24.))
                .bg(crate::blue())
                .border_t(px(2.))
                .border_r(px(4.))
                .border_b(px(6.))
                .border_l(px(8.))
                .border_color(crate::white())
                .shadow(vec![
                    BoxShadow::new(px(2.), px(3.), crate::black().opacity(0.5))
                        .blur_radius(px(5.))
                        .spread_radius(px(1.)),
                    BoxShadow::new(px(1.), px(1.), crate::white().opacity(0.4))
                        .blur_radius(px(3.))
                        .inset(),
                ])
                .backdrop_blur(px(4.))
                .blur(px(2.))
                .child(image);
            root.style().corner_smoothing = Some(1.5);
            root
        }
    }

    #[gpui::test]
    fn shadow_backgrounds_survive_painting_with_geometry_and_opacity(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| ShadowBackgroundView);
        let shadows = window
            .update(cx, |_, window, _| {
                window.rendered_frame.scene.shadows.clone()
            })
            .unwrap();

        assert_eq!(shadows.len(), 3);
        let gradient = shadow_gradient();
        let drop_gradient = shadows
            .iter()
            .find(|shadow| shadow.inset == ShaderBool::Disabled && shadow.color == gradient)
            .expect("gradient drop shadow should be painted");
        let inset_gradient = shadows
            .iter()
            .find(|shadow| shadow.inset == ShaderBool::Enabled && shadow.color == gradient)
            .expect("gradient inset shadow should be painted");
        let solid = shadows
            .iter()
            .find(|shadow| shadow.color.as_solid().is_some())
            .expect("solid shadows should remain supported");

        assert_eq!(
            solid.color.as_solid(),
            Some(hsla(0.3, 0.7, 0.4, 0.4)),
            "shadow alpha stays intrinsic until the group composite"
        );
        assert_eq!(drop_gradient.element_bounds, inset_gradient.element_bounds);
        assert!(drop_gradient.bounds.size.width > drop_gradient.element_bounds.size.width);
        assert!(drop_gradient.bounds.size.height > drop_gradient.element_bounds.size.height);
        assert!(inset_gradient.bounds.size.width < inset_gradient.element_bounds.size.width);
        assert!(inset_gradient.bounds.size.height < inset_gradient.element_bounds.size.height);
        assert!(drop_gradient.order < inset_gradient.order);
    }

    #[gpui::test]
    fn corner_smoothing_reaches_every_styled_scene_primitive(cx: &mut TestAppContext) {
        let frame = ImageFrame::new(ImageBuffer::from_pixel(2, 2, Rgba([255, 255, 255, 255])));
        let image = Arc::new(RenderImage::new(smallvec![frame]));
        let window = cx.add_window(move |_, _| CornerSmoothingSceneView { image });

        window
            .update(cx, |_, window, _| {
                let scene = &window.rendered_frame.scene;
                assert_smoothing(
                    "quads",
                    2,
                    scene.quads.iter().map(|quad| quad.corner_smoothing),
                );
                assert_smoothing(
                    "shadows",
                    2,
                    scene.shadows.iter().map(|shadow| shadow.corner_smoothing),
                );
                assert_smoothing(
                    "backdrop filters",
                    1,
                    scene
                        .backdrop_filters
                        .iter()
                        .map(|filter| filter.corner_smoothing),
                );
                assert_smoothing(
                    "filter boundaries",
                    2,
                    scene
                        .filter_boundaries
                        .iter()
                        .filter(|boundary| !boundary.filters.is_empty())
                        .map(|boundary| boundary.corner_smoothing),
                );
                assert_smoothing(
                    "images",
                    1,
                    scene
                        .polychrome_sprites
                        .iter()
                        .map(|image| image.corner_smoothing),
                );
                for inset in [ShaderBool::Disabled, ShaderBool::Enabled] {
                    assert!(scene.shadows.iter().any(|shadow| shadow.inset == inset));
                }

                let border = scene
                    .quads
                    .iter()
                    .find(|quad| quad.border_widths.top.0 > 0.0)
                    .expect("border quad");
                let scale = window.scale_factor();
                assert_eq!(
                    [
                        border.border_widths.top.0,
                        border.border_widths.right.0,
                        border.border_widths.bottom.0,
                        border.border_widths.left.0,
                    ],
                    [2.0, 4.0, 6.0, 8.0].map(|width| width * scale)
                );
            })
            .unwrap();
    }

    struct OpensWindowOnPaint {
        opened: Rc<Cell<bool>>,
    }

    impl Render for OpensWindowOnPaint {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let opened = self.opened.clone();
            div()
                .size_full()
                .child(canvas(
                    |_, _, _| {},
                    move |_, _, _window, cx| {
                        if !opened.replace(true) {
                            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| EmptyView))
                                .unwrap();
                        }
                    },
                ))
                // Siblings painted after the canvas: their elements were
                // allocated in the arena before the nested draw, so they detect
                // a mid-draw arena clear when painted afterwards.
                .child(div().child("after"))
        }
    }

    /// Opening a window synchronously draws it and requests an element arena
    /// clear. When that happens from within another window's draw (here: from
    /// an element's paint), the clear must be deferred until the outer draw
    /// finishes, or the outer draw's arena-allocated elements would be freed
    /// out from under it.
    #[test]
    fn test_window_opened_during_draw_defers_arena_clear() {
        let mut cx = TestAppContext::single();

        let opened = Rc::new(Cell::new(false));
        // add_window draws once, which runs the nested open_window mid-draw.
        let window = cx.add_window({
            let opened = opened.clone();
            move |_, _| OpensWindowOnPaint { opened }
        });

        assert!(opened.get());
        assert_eq!(cx.windows().len(), 2);

        // The deferred clear must actually run once the outer draw unwinds:
        // subsequent draws of both windows work against a fresh arena.
        cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    }

    /// Platforms that stop requesting frames for idle windows (currently web)
    /// rely on the frame waker firing whenever frame demand arises; a demand
    /// source that skips the waker shows up there as a window that silently
    /// stops repainting until unrelated activity wakes it.
    #[gpui::test]
    fn test_frame_waker_fires_on_frame_demand(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| EmptyView);
        let test_window = cx.test_window(window.into());

        // Windows start dirty, and that can predate waker installation;
        // installing the waker must deliver the pending wake or the first
        // frame would never be requested.
        assert!(
            test_window.frame_wake_count() >= 1,
            "opening a window must wake the frame source for the initial frame"
        );

        // Serve outstanding demand (present the frame drawn by `add_window`).
        test_window.simulate_frame_request(RequestFrameOptions::default());

        // An idle window must not wake on clean frames or plain updates, or
        // the frame source could never stop.
        let baseline = test_window.frame_wake_count();
        test_window.simulate_frame_request(RequestFrameOptions::default());
        window.update(cx, |_, _, _| {}).unwrap();
        assert_eq!(
            test_window.frame_wake_count(),
            baseline,
            "clean frames and non-notifying updates must not wake the frame source"
        );

        // Notifying a view in an idle window is the core demand signal.
        window.update(cx, |_, _, cx| cx.notify()).unwrap();
        assert!(
            test_window.frame_wake_count() > baseline,
            "notifying a view in an idle window must wake the frame source"
        );

        // Serving that demand returns to idle without further wakes.
        test_window.simulate_frame_request(RequestFrameOptions::default());
        let baseline = test_window.frame_wake_count();
        test_window.simulate_frame_request(RequestFrameOptions::default());
        assert_eq!(
            test_window.frame_wake_count(),
            baseline,
            "serving demand must return the window to idle"
        );

        // Next-frame callbacks create demand without dirtying the window.
        window
            .update(cx, |_, window, _| window.on_next_frame(|_, _| {}))
            .unwrap();
        assert!(
            test_window.frame_wake_count() > baseline,
            "scheduling a next-frame callback in an idle window must wake the frame source"
        );
    }

    /// A frame request that arrives while next-frame callbacks are pending
    /// must never strand them: either the frame runs them, or (when the
    /// inactive-window frame-rate throttle defers the frame) the waker fires
    /// so another request is delivered.
    #[gpui::test]
    fn test_pending_next_frame_callbacks_are_not_stranded(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| EmptyView);
        let test_window = cx.test_window(window.into());
        // Establish a recent last-frame time so the inactive-window throttle
        // can engage on the next request.
        test_window.simulate_frame_request(RequestFrameOptions::default());

        let callback_ran = Rc::new(Cell::new(false));
        window
            .update(cx, {
                let callback_ran = callback_ran.clone();
                move |_, window, _| {
                    window.on_next_frame(move |_, _| callback_ran.set(true));
                }
            })
            .unwrap();

        let baseline = test_window.frame_wake_count();
        test_window.simulate_frame_request(RequestFrameOptions::default());
        // The test window is inactive, so this request throttles to ~30fps
        // when it lands within the throttle interval of the previous frame
        // (the common case here, but timing-dependent): the callback is
        // deferred and the waker must re-arm the frame source. On a slow run
        // the request instead lands outside the interval and runs the
        // callback directly.
        assert!(
            test_window.frame_wake_count() > baseline || callback_ran.get(),
            "a frame request with pending next-frame callbacks must either run them or re-arm the frame source"
        );
    }

    #[gpui::test]
    fn frame_pointer_motion_restores_hover_without_hiding_keyboard_focus(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| EmptyView);
        cx.update_window(window.into(), |_, window, cx| {
            window.dispatch_event(
                PlatformInput::KeyDown(crate::KeyDownEvent {
                    keystroke: crate::Keystroke::parse("tab").unwrap(),
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
            assert!(window.last_input_was_keyboard());
            assert!(window.hover_suppressed_by_keyboard());
            window.dispatch_event(
                PlatformInput::MouseMove(crate::MouseMoveEvent {
                    position: point(px(10.0), px(10.0)),
                    pressed_button: None,
                    modifiers: Modifiers::default(),
                }),
                cx,
            );
            assert!(
                window.last_input_was_keyboard(),
                "incidental pointer motion must retain the focus ring"
            );
            assert!(
                !window.hover_suppressed_by_keyboard(),
                "pointer hover must resume without requiring a click"
            );
            window.dispatch_event(
                PlatformInput::MouseDown(crate::MouseDownEvent {
                    button: MouseButton::Left,
                    position: point(px(10.0), px(10.0)),
                    modifiers: Modifiers::default(),
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
            assert!(!window.last_input_was_keyboard());
        })
        .unwrap();
    }

    #[gpui::test]
    fn test_window_reports_no_raw_handle_instead_of_panicking(cx: &mut TestAppContext) {
        use raw_window_handle::{HandleError, HasDisplayHandle as _, HasWindowHandle as _};

        let window = cx.add_window(|_, _| EmptyView);
        window
            .update(cx, |_, window, _| {
                assert!(matches!(
                    window.window_handle(),
                    Err(HandleError::NotSupported)
                ));
                assert!(matches!(
                    window.display_handle(),
                    Err(HandleError::NotSupported)
                ));
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_window_visibility_can_be_changed(cx: &mut TestAppContext) {
        for show in [false, true] {
            let window = cx.update(|cx| {
                cx.open_window(WindowOptions::new().show(show), |_, cx| {
                    cx.new(|_| EmptyView)
                })
                .unwrap()
            });
            let platform_window = cx.test_window(window.into());
            assert_eq!(platform_window.is_visible(), show);

            for visible in [false, false, true, true, false, true] {
                window
                    .update(cx, |_, window, _| window.set_visible(visible))
                    .unwrap();
                assert_eq!(platform_window.is_visible(), visible);
            }
        }
    }

    #[gpui::test]
    fn test_appearance_change_runs_after_app_update(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| EmptyView);
        let observed_appearance = Rc::new(Cell::new(None));
        let _subscription = window
            .update(cx, {
                let observed_appearance = observed_appearance.clone();
                move |_, window, _| {
                    window.observe_window_appearance(move |window, _| {
                        observed_appearance.set(Some(window.appearance()));
                    })
                }
            })
            .unwrap();
        let test_window = cx.test_window(window.into());

        cx.update(|_| {
            test_window.simulate_appearance_change(WindowAppearance::Dark);
            assert_eq!(observed_appearance.get(), None);
        });
        cx.run_until_parked();

        assert_eq!(observed_appearance.get(), Some(WindowAppearance::Dark));
    }

    #[gpui::test]
    fn queued_frame_callback_wakes_a_parked_render_loop(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| Empty);
        let test_window = cx.test_window(window.into());

        assert!(test_window.simulate_scheduled_frame());
        assert!(test_window.simulate_scheduled_frame());
        assert!(!test_window.frame_scheduled());

        cx.update_window(window.into(), |_, window, _| {
            window.active.set(true);
            window.on_next_frame(|_, _| {});
        })
        .unwrap();
        assert!(
            test_window.frame_scheduled(),
            "queuing work on a parked window must wake the render loop"
        );

        assert!(test_window.simulate_scheduled_frame());
        assert!(
            test_window.frame_scheduled(),
            "presenting the frame must await one compositor callback"
        );
        assert!(test_window.simulate_scheduled_frame());
        assert!(!test_window.frame_scheduled());
    }

    #[gpui::test]
    fn pending_presentation_wakes_a_parked_render_loop(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| Empty);
        let test_window = cx.test_window(window.into());

        assert!(test_window.simulate_scheduled_frame());
        assert!(test_window.simulate_scheduled_frame());
        assert!(!test_window.frame_scheduled());

        cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();

        assert!(
            test_window.frame_scheduled(),
            "a rendered scene awaiting presentation must wake the render loop"
        );
    }

    #[gpui::test]
    fn callback_queued_during_a_frame_requests_a_follow_up(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| Empty);
        let test_window = cx.test_window(window.into());

        let callback_ran = Rc::new(Cell::new(false));
        cx.update_window(window.into(), |_, window, _| {
            // Inactive windows are frame-rate throttled, which would defer the
            // ticks this test drives manually.
            window.active.set(true);
            let callback_ran = callback_ran.clone();
            window.on_next_frame(move |window, _| {
                window.on_next_frame(move |_, _| callback_ran.set(true));
            });
        })
        .unwrap();

        assert!(test_window.simulate_scheduled_frame());
        assert!(!callback_ran.get());
        assert!(
            test_window.frame_scheduled(),
            "a callback queued mid-frame must schedule a follow-up before the loop parks"
        );

        assert!(test_window.simulate_scheduled_frame());
        assert!(callback_ran.get());
    }

    struct RootView {
        explicit_size: bool,
        child_bounds: Rc<Cell<Bounds<Pixels>>>,
    }

    impl Render for RootView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let child_bounds = self.child_bounds.clone();
            let root = div().flex().flex_col().child(
                canvas(
                    move |bounds, _, _| child_bounds.set(bounds),
                    |_, _, _, _| {},
                )
                .size_full(),
            );
            if self.explicit_size {
                root.w(px(300.)).h(px(200.))
            } else {
                root
            }
        }
    }

    #[test]
    fn auto_sized_window_root_fills_the_window() {
        let mut cx = TestAppContext::single();
        let child_bounds = Rc::new(Cell::new(Bounds::default()));
        let window = cx.add_window({
            let child_bounds = child_bounds.clone();
            move |_, _| RootView {
                explicit_size: false,
                child_bounds,
            }
        });

        let viewport_size = cx
            .update_window(window.into(), |_, window, cx| {
                window.draw(cx).clear(cx);
                window.viewport_size()
            })
            .unwrap();

        assert_eq!(child_bounds.get().size, viewport_size);
    }

    #[test]
    fn explicitly_sized_window_root_keeps_its_size() {
        let mut cx = TestAppContext::single();
        let child_bounds = Rc::new(Cell::new(Bounds::default()));
        let window = cx.add_window({
            let child_bounds = child_bounds.clone();
            move |_, _| RootView {
                explicit_size: true,
                child_bounds,
            }
        });

        cx.update_window(window.into(), |_, window, cx| {
            window.draw(cx).clear(cx);
        })
        .unwrap();

        assert_eq!(child_bounds.get().size, size(px(300.), px(200.)));
    }

    struct FileDragView {
        path: PathBuf,
        observed_drag_moves: Rc<RefCell<Vec<Point<Pixels>>>>,
        observed_drops: Rc<RefCell<Vec<PathBuf>>>,
    }

    impl Render for FileDragView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("file-drag")
                .size_full()
                .on_drag(self.path.clone(), |_, _, _, cx| cx.new(|_| Empty))
                .external_drag_payload(|path: &PathBuf, _, _| {
                    Some(ExternalDragPayload::Files(FileDragPaths::new([(
                        path.clone(),
                        true,
                    )])))
                })
                .on_drag_move({
                    let observed_drag_moves = self.observed_drag_moves.clone();
                    move |event: &DragMoveEvent<PathBuf>, _, _| {
                        observed_drag_moves.borrow_mut().push(event.event.position);
                    }
                })
                .on_drop({
                    let observed_drops = self.observed_drops.clone();
                    move |path: &PathBuf, _, _| observed_drops.borrow_mut().push(path.clone())
                })
        }
    }

    #[gpui::test]
    fn file_drag_is_promoted_once_and_restored_in_source_window(cx: &mut TestAppContext) {
        struct Drag {
            window: AnyWindowHandle,
            observed_drag_moves: Rc<RefCell<Vec<Point<Pixels>>>>,
            observed_drops: Rc<RefCell<Vec<PathBuf>>>,
        }

        fn start_drag(cx: &mut TestAppContext, path: PathBuf, platform_result: bool) -> Drag {
            let observed_drag_moves = Rc::new(RefCell::new(Vec::new()));
            let observed_drops = Rc::new(RefCell::new(Vec::new()));
            let window: AnyWindowHandle = cx
                .add_window({
                    let observed_drag_moves = observed_drag_moves.clone();
                    let observed_drops = observed_drops.clone();
                    move |_, _| FileDragView {
                        path,
                        observed_drag_moves,
                        observed_drops,
                    }
                })
                .into();
            cx.test_window(window)
                .set_start_external_drag_result(platform_result);

            let update_result = cx.update_window(window, |_, window, cx| {
                window.draw(cx).clear(cx);
                window.dispatch_event(
                    MouseDownEvent {
                        position: point(px(10.), px(10.)),
                        button: MouseButton::Left,
                        modifiers: Default::default(),
                        click_count: 1,
                        first_mouse: false,
                    }
                    .to_platform_input(),
                    cx,
                );
                window.dispatch_event(
                    MouseMoveEvent {
                        position: point(px(20.), px(20.)),
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Default::default(),
                    }
                    .to_platform_input(),
                    cx,
                );
                assert!(cx.active_drag.is_some());
            });
            assert!(
                update_result.is_ok(),
                "failed to start drag: {update_result:?}"
            );

            assert!(cx.test_window(window).external_drag_files().is_empty());
            Drag {
                window,
                observed_drag_moves,
                observed_drops,
            }
        }

        let successful_path = PathBuf::from("/tmp/successful-drag");
        let successful = start_drag(cx, successful_path.clone(), true);
        let outside_position = point(px(-1.), px(20.));
        let update_result = cx.update_window(successful.window, |_, window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position: outside_position,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(cx.active_drag.is_none());
        });
        assert!(
            update_result.is_ok(),
            "failed to promote drag: {update_result:?}"
        );
        assert_eq!(
            cx.test_window(successful.window).external_drag_files(),
            [(successful_path.clone(), true)]
        );
        // Views must still see the move that leaves the window, otherwise they never learn to tear
        // down the drag state they built up while the pointer was inside.
        assert_eq!(
            successful.observed_drag_moves.borrow().last(),
            Some(&outside_position)
        );

        let destination: AnyWindowHandle = cx.add_window(|_, _| EmptyView).into();
        let reentry_position = point(px(30.), px(30.));
        let external_paths = || ExternalPaths([successful_path.clone()].into_iter().collect());
        let update_result = cx.update_window(destination, |_, window, cx| {
            window.dispatch_event(
                FileDropEvent::Entered {
                    position: reentry_position,
                    paths: external_paths(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(
                cx.active_drag
                    .as_ref()
                    .is_some_and(|drag| drag.value.downcast_ref::<ExternalPaths>().is_some())
            );
            window.dispatch_event(FileDropEvent::Exited.to_platform_input(), cx);
            assert!(cx.active_drag.is_none());
        });
        assert!(
            update_result.is_ok(),
            "failed to handle drag in destination window: {update_result:?}"
        );

        let update_result = cx.update_window(successful.window, |_, window, cx| {
            window.dispatch_event(
                FileDropEvent::Entered {
                    position: reentry_position,
                    paths: external_paths(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(
                cx.active_drag
                    .as_ref()
                    .is_some_and(|drag| drag.value.downcast_ref::<PathBuf>().is_some())
            );
            assert_eq!(
                successful.observed_drag_moves.borrow().last(),
                Some(&reentry_position)
            );

            window.dispatch_event(FileDropEvent::Exited.to_platform_input(), cx);
            assert!(cx.active_drag.is_none());

            window.dispatch_event(
                FileDropEvent::Entered {
                    position: reentry_position,
                    paths: external_paths(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(
                cx.active_drag
                    .as_ref()
                    .is_some_and(|drag| drag.value.downcast_ref::<PathBuf>().is_some())
            );

            window.dispatch_event(
                FileDropEvent::Submit {
                    position: reentry_position,
                }
                .to_platform_input(),
                cx,
            );
            assert_eq!(
                successful.observed_drops.borrow().as_slice(),
                std::slice::from_ref(&successful_path)
            );
            assert!(cx.active_drag.is_none());

            window.dispatch_event(FileDropEvent::Exited.to_platform_input(), cx);
            assert!(cx.active_drag.is_none());
            window.dispatch_event(FileDropEvent::Ended.to_platform_input(), cx);
            assert!(cx.active_drag.is_none());

            window.dispatch_event(
                FileDropEvent::Entered {
                    position: reentry_position,
                    paths: external_paths(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(
                cx.active_drag
                    .as_ref()
                    .is_some_and(|drag| drag.value.downcast_ref::<ExternalPaths>().is_some())
            );
            window.dispatch_event(FileDropEvent::Exited.to_platform_input(), cx);
        });
        assert!(
            update_result.is_ok(),
            "failed to restore drag in source window: {update_result:?}"
        );

        let cancelled_path = PathBuf::from("/tmp/cancelled-drag");
        let cancelled = start_drag(cx, cancelled_path.clone(), true);
        let update_result = cx.update_window(cancelled.window, |_, window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position: outside_position,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(cx.active_drag.is_none());

            window.dispatch_event(
                FileDropEvent::Entered {
                    position: reentry_position,
                    paths: ExternalPaths([cancelled_path].into_iter().collect()),
                }
                .to_platform_input(),
                cx,
            );
            assert!(
                cx.active_drag
                    .as_ref()
                    .is_some_and(|drag| drag.value.downcast_ref::<PathBuf>().is_some())
            );
            assert!(cx.stop_active_drag(window));
            assert!(cx.active_drag.is_none());
        });
        assert!(
            update_result.is_ok(),
            "failed to cancel restored drag: {update_result:?}"
        );
        assert!(!cx.update(|cx| cx.end_platform_drag(cancelled.window.window_id())));

        let removed_path = PathBuf::from("/tmp/removed-window-drag");
        let removed = start_drag(cx, removed_path, true);
        let removed_window_id = removed.window.window_id();
        let update_result = cx.update_window(removed.window, |_, window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position: outside_position,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(cx.active_drag.is_none());
            window.remove_window();
        });
        assert!(
            update_result.is_ok(),
            "failed to remove drag source window: {update_result:?}"
        );
        assert!(!cx.update(|cx| cx.end_platform_drag(removed_window_id)));

        let failed_path = PathBuf::from("/tmp/failed-drag");
        let failed = start_drag(cx, failed_path.clone(), false);
        let update_result = cx.update_window(failed.window, |_, window, cx| {
            for x_position in [-1., -2.] {
                window.dispatch_event(
                    MouseMoveEvent {
                        position: point(px(x_position), px(20.)),
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Default::default(),
                    }
                    .to_platform_input(),
                    cx,
                );
            }
            assert!(cx.active_drag.is_some());
        });
        assert!(
            update_result.is_ok(),
            "failed to retain drag after platform failure: {update_result:?}"
        );
        assert_eq!(
            cx.test_window(failed.window).external_drag_files(),
            [(failed_path, true)]
        );
    }

    struct FocusForwarder {
        a: FocusHandle,
        b: FocusHandle,
    }

    impl Render for FocusForwarder {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(div().w(px(50.)).h(px(50.)).track_focus(&self.a))
                .child(div().w(px(50.)).h(px(50.)).track_focus(&self.b))
        }
    }

    /// When a focus listener moves focus again (e.g. a dock forwarding focus to its
    /// active panel), the resulting focus events must be dispatched without waiting
    /// for an unrelated redraw of the window.
    #[gpui::test]
    fn test_focus_moved_by_focus_listener_is_dispatched(cx: &mut TestAppContext) {
        let b_focus_count = Rc::new(Cell::new(0));
        let window = cx.add_window({
            let b_focus_count = b_focus_count.clone();
            move |window, cx| {
                let a = cx.focus_handle();
                let b = cx.focus_handle();
                cx.on_focus(&a, window, |this: &mut FocusForwarder, window, cx| {
                    let b = this.b.clone();
                    window.focus(&b, cx);
                })
                .detach();
                cx.on_focus(&b, window, move |_, _, _| {
                    b_focus_count.set(b_focus_count.get() + 1);
                })
                .detach();
                FocusForwarder { a, b }
            }
        });

        window.update(cx, |_, window, _| window.activate()).unwrap();
        cx.executor().run_until_parked();

        window
            .update(cx, |this, window, cx| {
                let a = this.a.clone();
                window.focus(&a, cx);
            })
            .unwrap();
        cx.executor().run_until_parked();

        window
            .update(cx, |this, window, _| {
                assert!(this.b.is_focused(window));
            })
            .unwrap();
        assert_eq!(b_focus_count.get(), 1);
    }

    #[gpui::test]
    fn claimed_touch_drag_receives_movement_and_release(cx: &mut TestAppContext) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let events = events.clone();
            move |_, _| TouchDragListener { events }
        });
        let touch = TouchId(1);

        dispatch_touch(window, cx, touch, TouchPhase::Started, 10.);
        dispatch_touch(window, cx, touch, TouchPhase::Moved, 30.);
        dispatch_touch(window, cx, touch, TouchPhase::Ended, 40.);

        assert_eq!(
            events.borrow().as_slice(),
            [
                (TouchPhase::Started, px(10.)),
                (TouchPhase::Moved, px(30.)),
                (TouchPhase::Ended, px(40.)),
            ]
        );
    }

    struct TouchDragListener {
        events: Rc<RefCell<Vec<(TouchPhase, Pixels)>>>,
    }

    impl Render for TouchDragListener {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let events = self.events.clone();
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    window.on_mouse_event(move |event: &TouchDragEvent, phase, window, _cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        events.borrow_mut().push((event.phase, event.position.x));
                        if event.phase == TouchPhase::Started {
                            window.prevent_default();
                        }
                    });
                },
            )
        }
    }

    #[gpui::test]
    fn long_press_is_claimed_only_when_started_prevents_default(cx: &mut TestAppContext) {
        for response in [
            LongPressResponse::PreventDefault,
            LongPressResponse::StopPropagation,
            LongPressResponse::None,
        ] {
            let phases = Rc::new(RefCell::new(Vec::new()));
            let window = cx.add_window({
                let phases = phases.clone();
                move |_, _| LongPressListener { phases, response }
            });
            dispatch_touch(window, cx, TouchId(1), TouchPhase::Started, 0.);
            cx.executor().advance_clock(Duration::from_millis(501));
            cx.executor().run_until_parked();
            window
                .update(cx, |_, window, _| {
                    assert_eq!(
                        window.long_press_capture.is_some(),
                        response == LongPressResponse::PreventDefault
                    );
                })
                .unwrap();
            dispatch_touch(window, cx, TouchId(1), TouchPhase::Moved, 2.);
            dispatch_touch(window, cx, TouchId(1), TouchPhase::Ended, 2.);
            window
                .update(cx, |_, window, _| {
                    assert!(window.long_press_capture.is_none());
                })
                .unwrap();

            let phases = phases.borrow();
            if response == LongPressResponse::PreventDefault {
                assert_eq!(
                    phases.as_slice(),
                    [TouchPhase::Started, TouchPhase::Moved, TouchPhase::Ended]
                );
            } else {
                assert_eq!(phases.as_slice(), [TouchPhase::Started]);
            }
        }
    }

    #[gpui::test]
    fn stale_default_prevention_does_not_claim_long_press(cx: &mut TestAppContext) {
        let phases = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let phases = phases.clone();
            move |_, _| LongPressListener {
                phases,
                response: LongPressResponse::None,
            }
        });
        window
            .update(cx, |_, window, _| {
                window.prevent_default();
            })
            .unwrap();

        dispatch_touch(window, cx, TouchId(1), TouchPhase::Started, 0.);
        cx.executor().advance_clock(Duration::from_millis(501));
        cx.executor().run_until_parked();
        dispatch_touch(window, cx, TouchId(1), TouchPhase::Moved, 2.);

        assert_eq!(phases.borrow().as_slice(), [TouchPhase::Started]);
    }

    #[gpui::test]
    fn resolved_touch_cancels_scheduled_long_press(cx: &mut TestAppContext) {
        for (phase, position) in [
            (TouchPhase::Ended, 0.),
            (TouchPhase::Cancelled, 0.),
            (TouchPhase::Moved, 20.),
        ] {
            let phases = Rc::new(RefCell::new(Vec::new()));
            let window = cx.add_window({
                let phases = phases.clone();
                move |_, _| LongPressListener {
                    phases,
                    response: LongPressResponse::PreventDefault,
                }
            });
            dispatch_touch(window, cx, TouchId(1), TouchPhase::Started, 0.);
            dispatch_touch(window, cx, TouchId(1), phase, position);
            cx.executor().advance_clock(Duration::from_millis(501));
            cx.executor().run_until_parked();

            assert!(phases.borrow().is_empty(), "{phase:?} allowed long press");
        }
    }

    #[gpui::test]
    fn stale_long_press_timer_cannot_affect_replacement_touch(cx: &mut TestAppContext) {
        let phases = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let phases = phases.clone();
            move |_, _| LongPressListener {
                phases,
                response: LongPressResponse::PreventDefault,
            }
        });
        let first_touch = TouchId(1);
        dispatch_touch(window, cx, first_touch, TouchPhase::Started, 0.);
        cx.executor().advance_clock(Duration::from_millis(250));
        dispatch_touch(window, cx, first_touch, TouchPhase::Cancelled, 0.);
        dispatch_touch(window, cx, TouchId(2), TouchPhase::Started, 10.);

        cx.executor().advance_clock(Duration::from_millis(251));
        cx.executor().run_until_parked();
        assert!(phases.borrow().is_empty());

        cx.executor().advance_clock(Duration::from_millis(250));
        cx.executor().run_until_parked();
        assert_eq!(phases.borrow().as_slice(), [TouchPhase::Started]);
    }

    #[derive(Clone, Copy, PartialEq)]
    enum LongPressResponse {
        PreventDefault,
        StopPropagation,
        None,
    }

    struct LongPressListener {
        phases: Rc<RefCell<Vec<TouchPhase>>>,
        response: LongPressResponse,
    }

    impl Render for LongPressListener {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let entity = cx.entity();
            let phases = self.phases.clone();
            let response = self.response;
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    window.on_mouse_event(move |event: &LongPressEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        phases.borrow_mut().push(event.phase);
                        match response {
                            LongPressResponse::PreventDefault => {
                                window.capture_long_press(&entity);
                                window.prevent_default();
                            }
                            LongPressResponse::StopPropagation => cx.stop_propagation(),
                            LongPressResponse::None => {}
                        }
                    });
                },
            )
        }
    }

    fn dispatch_touch<T: 'static>(
        window: crate::WindowHandle<T>,
        cx: &mut TestAppContext,
        id: TouchId,
        phase: TouchPhase,
        x: f32,
    ) {
        window
            .update(cx, |_, window, cx| {
                window.dispatch_event(
                    TouchEvent {
                        id,
                        phase,
                        position: point(px(x), px(0.)),
                        predicted_position: None,
                        force: None,
                    }
                    .to_platform_input(),
                    cx,
                );
            })
            .unwrap();
    }

    struct CachedOpacityChild {
        renders: Rc<Cell<usize>>,
        paints: Rc<Cell<usize>>,
    }

    impl Render for CachedOpacityChild {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            self.renders.set(self.renders.get() + 1);
            let paints = self.paints.clone();

            div().size(px(20.0)).bg(crate::red()).child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        paints.set(paints.get() + 1);
                        window.paint_quad(PaintQuad::new(bounds).background(crate::blue()));
                    },
                )
                .size_full()
                .opacity(0.5),
            )
        }
    }

    struct CachedOpacityParent {
        opacity: Option<f32>,
        child: Entity<CachedOpacityChild>,
        cached: bool,
    }

    impl Render for CachedOpacityParent {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut child_style = div().size(px(20.0));
            let child = if self.cached {
                self.child
                    .clone()
                    .cached(child_style.style().clone())
                    .into_any_element()
            } else {
                self.child.clone().into_any_element()
            };

            div()
                .size(px(40.0))
                .when_some(self.opacity, |parent, opacity| parent.opacity(opacity))
                .child(child)
        }
    }

    #[test]
    fn parent_opacity_changes_replay_cached_canvas_groups() {
        let renders = Rc::new(Cell::new(0));
        let paints = Rc::new(Cell::new(0));
        let mut app = TestApp::new();
        let mut test_window = app.open_window(|_, cx| CachedOpacityParent {
            opacity: None,
            cached: true,
            child: cx.new(|_| CachedOpacityChild {
                renders: renders.clone(),
                paints: paints.clone(),
            }),
        });
        test_window.draw();
        let initial_renders = renders.get();
        let initial_paints = paints.get();
        assert!(initial_renders > 0);
        assert!(initial_paints > 0);

        let scene_snapshot = |scene: &Scene| {
            (
                scene.render_commands().to_vec(),
                scene
                    .quads
                    .iter()
                    .map(|quad| (quad.order, quad.bounds, quad.content_mask, quad.background))
                    .collect::<Vec<_>>(),
                scene
                    .filter_boundaries
                    .iter()
                    .map(|boundary| {
                        (
                            boundary.bounds,
                            boundary.content_mask,
                            boundary.opacity,
                            boundary.is_start,
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        };

        for opacity in [Some(0.5), Some(0.25), Some(1.0), Some(0.0), None] {
            let mut fresh_app = TestApp::new();
            let mut fresh_window = fresh_app.open_window(|_, cx| CachedOpacityParent {
                opacity,
                cached: false,
                child: cx.new(|_| CachedOpacityChild {
                    renders: Rc::new(Cell::new(0)),
                    paints: Rc::new(Cell::new(0)),
                }),
            });
            fresh_window.draw();
            let fresh_scene =
                fresh_window.update(|_, window, _| scene_snapshot(&window.rendered_frame.scene));

            test_window.update(|view, _, cx| {
                view.opacity = opacity;
                cx.notify();
            });
            test_window.draw();
            test_window.update(|_, window, _| {
                let scene = &window.rendered_frame.scene;
                assert_eq!(scene_snapshot(scene), fresh_scene, "opacity {opacity:?}");
            });
            assert_eq!(renders.get(), initial_renders);
            assert_eq!(paints.get(), initial_paints);
        }
    }
}
