//! Div is the central, reusable element that most GPUI trees will be built from.
//! It functions as a container for other elements, and provides a number of
//! useful features for laying out and styling its children as well as binding
//! mouse events and action handlers. It is meant to be similar to the HTML `<div>`
//! element, but for GPUI.
//!
//! # Build your own div
//!
//! GPUI does not directly provide APIs for stateful, multi step events like `click`
//! and `drag`. We want GPUI users to be able to build their own abstractions for
//! their own needs. However, as a UI framework, we're also obliged to provide some
//! building blocks to make the process of building your own elements easier.
//! For this we have the [`Interactivity`] and the [`StyleRefinement`] structs, as well
//! as several associated traits. Together, these provide the full suite of Dom-like events
//! and Tailwind-like styling that you can use to build your own custom elements. Div is
//! constructed by combining these two systems into an all-in-one element.

#[cfg(debug_assertions)]
use crate::{PaintQuad, TextAlign};

#[cfg(test)]
use crate::{
    AnyWindowHandle, Context, HighlightStyle, InputEvent, Keystroke, StyledText, TestAppContext,
    canvas, hsla, util::FluentBuilder,
};

#[cfg(test)]
use std::{cell::Cell, rc::Weak};

use crate::{
    Action, AnyDrag, AnyElement, AnyTooltip, AnyView, App, AppContext, Bounds, ClickEvent,
    CursorStyle, DispatchPhase, Display, Element, ElementId, Entity, EntityId, ExternalDragPayload,
    FocusHandle, Global, GlobalElementId, Hitbox, HitboxBehavior, HitboxId, InspectorElementId,
    IntoElement, IsZero, KeyContext, KeyDownEvent, KeyUpEvent, KeyboardButton, KeyboardClickEvent,
    LayoutId, ModifiersChangedEvent, MouseButton, MouseClickEvent, MouseDownEvent, MouseExitEvent,
    MouseMoveEvent, MousePressureEvent, MouseUpEvent, OngoingScroll, Overflow, ParentElement,
    PinchEvent, Pixels, Point, PointerChangeCause, Render, ScrollWheelEvent, SharedString, Size,
    Style, StyleRefinement, StyleTransitionContext, StyleTransitionState, StyleTransitions, Styled,
    Task, TooltipId, Visibility, Window, WindowControlArea, point, px, size,
};
use collections::HashMap;
use gpui_util::ResultExt;
use itertools::Either;
use refineable::Refineable;
use smallvec::SmallVec;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    cmp::Ordering,
    fmt::Debug,
    marker::PhantomData,
    mem,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use super::ImageCacheProvider;

mod inline;
pub(crate) use inline::InlineContent;
use inline::InlineDivFrameState;
#[cfg(feature = "stacker")]
type StackSafe<T> = stacksafe::StackSafe<T>;
#[cfg(not(feature = "stacker"))]
type StackSafe<T> = T;

const DRAG_THRESHOLD: f64 = 2.;
const DEFAULT_TOOLTIP_SHOW_DELAY: Duration = Duration::from_millis(500);
const HOVERABLE_TOOLTIP_HIDE_DELAY: Duration = Duration::from_millis(500);

/// The styling information for a given group.
pub struct GroupStyle {
    /// The identifier for this group.
    pub group: SharedString,

    /// The specific style refinement that this group would apply
    /// to its children.
    pub style: Box<StyleRefinement>,
}

/// An event for when a drag is moving over this element, with the given state type.
pub struct DragMoveEvent<T> {
    /// The mouse move event that triggered this drag move event.
    pub event: MouseMoveEvent,

    /// The bounds of this element.
    pub bounds: Bounds<Pixels>,
    drag: PhantomData<T>,
    dragged_item: Arc<dyn Any>,
}

impl<T: 'static> DragMoveEvent<T> {
    /// Returns the drag state for this event.
    pub fn drag<'b>(&self, cx: &'b App) -> &'b T {
        cx.active_drag
            .as_ref()
            .and_then(|drag| drag.value.downcast_ref::<T>())
            .expect("DragMoveEvent is only valid when the stored active drag is of the same type.")
    }

    /// An item that is about to be dropped.
    pub fn dragged_item(&self) -> &dyn Any {
        self.dragged_item.as_ref()
    }
}

impl Interactivity {
    /// Create an `Interactivity`, capturing the caller location in debug mode.
    #[cfg(any(feature = "inspector", debug_assertions))]
    #[track_caller]
    pub fn new() -> Interactivity {
        Interactivity {
            source_location: Some(core::panic::Location::caller()),
            ..Default::default()
        }
    }

    /// Create an `Interactivity`, capturing the caller location in debug mode.
    #[cfg(not(any(feature = "inspector", debug_assertions)))]
    pub fn new() -> Interactivity {
        Interactivity::default()
    }

    /// Gets the source location of construction. Returns `None` when not in debug mode.
    pub fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        #[cfg(any(feature = "inspector", debug_assertions))]
        {
            self.source_location
        }

        #[cfg(not(any(feature = "inspector", debug_assertions)))]
        {
            None
        }
    }

    /// Bind the given callback to the mouse down event for the given mouse button, during the bubble phase.
    /// The imperative API equivalent of [`InteractiveElement::on_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to the view state from this callback.
    pub fn on_mouse_down(
        &mut self,
        button: MouseButton,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.button == button
                    && hitbox.is_hovered(window)
                {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse down event for any button, during the capture phase.
    /// The imperative API equivalent of [`InteractiveElement::capture_any_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_any_mouse_down(
        &mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse down event for any button, during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_any_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_any_mouse_down(
        &mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse pressure event, during the bubble phase
    /// the imperative API equivalent to [`InteractiveElement::on_mouse_pressure`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_pressure(
        &mut self,
        listener: impl Fn(&MousePressureEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_pressure_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse pressure event, during the capture phase
    /// the imperative API equivalent to [`InteractiveElement::on_mouse_pressure`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_mouse_pressure(
        &mut self,
        listener: impl Fn(&MousePressureEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_pressure_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse up event for the given button, during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_up(
        &mut self,
        button: MouseButton,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.button == button
                    && hitbox.is_hovered(window)
                {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse up event for any button, during the capture phase.
    /// The imperative API equivalent to [`InteractiveElement::capture_any_mouse_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_any_mouse_up(
        &mut self,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse up event for any button, during the bubble phase.
    /// The imperative API equivalent to [`Interactivity::on_any_mouse_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_any_mouse_up(
        &mut self,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse down event, on any button, during the capture phase,
    /// when the mouse is outside of the bounds of this element.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_down_out`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_down_out(
        &mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture
                    && !window.has_active_prompt()
                    && !hitbox.contains(&window.mouse_position())
                {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to the mouse down event - for all mouse up events,
    /// regardless of dispatch phase, mouse button, or whether the mouse is
    /// inside or outside the bounds of the element.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_down_all`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_down_all(
        &mut self,
        listener: impl Fn(&MouseDownEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_down_listeners.push(Box::new(listener));
    }

    /// Bind the given callback to the mouse up event, for the given button, during the capture phase,
    /// when the mouse is outside of the bounds of this element.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_up_out`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_up_out(
        &mut self,
        button: MouseButton,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture
                    && event.button == button
                    && !hitbox.is_hovered(window)
                {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to the mouse up event - for all mouse up events,
    /// regardless of dispatch phase, mouse button, or whether the mouse is
    /// inside or outside the bounds of the element.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_up_all`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_up_all(
        &mut self,
        listener: impl Fn(&MouseUpEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_up_listeners.push(Box::new(listener));
    }

    /// Bind the given callback to the mouse move event, during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_move`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_move(
        &mut self,
        listener: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_move_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to the mouse move event, triggering even if the mouse is outside the element's hitbox.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_move_all`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_move_all(
        &mut self,
        listener: impl Fn(&MouseMoveEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_move_listeners.push(Box::new(listener));
    }

    /// Bind the given callback to the mouse exit event, during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_mouse_exit`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_mouse_exit(
        &mut self,
        listener: impl Fn(&MouseExitEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_exit_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to the mouse drag event of the given type. Note that this
    /// will be called for all move events, inside or outside of this element, as long as the
    /// drag was started with this element under the mouse. Useful for implementing draggable
    /// UIs that don't conform to a drag and drop style interaction, like resizing.
    /// The imperative API equivalent to [`InteractiveElement::on_drag_move`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_drag_move<T>(
        &mut self,
        listener: impl Fn(&DragMoveEvent<T>, &mut Window, &mut App) + 'static,
    ) where
        T: 'static,
    {
        self.mouse_move_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Capture
                    && let Some(drag) = &cx.active_drag
                    && drag.value.as_ref().type_id() == TypeId::of::<T>()
                {
                    (listener)(
                        &DragMoveEvent {
                            event: event.clone(),
                            bounds: hitbox.bounds,
                            drag: PhantomData,
                            dragged_item: Arc::clone(&drag.value),
                        },
                        window,
                        cx,
                    );
                }
            }));
    }

    /// Bind the given callback to scroll wheel events during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_scroll_wheel`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_scroll_wheel(
        &mut self,
        listener: impl Fn(&ScrollWheelEvent, &mut Window, &mut App) + 'static,
    ) {
        self.scroll_wheel_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.should_handle_scroll(window) {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to pinch gesture events during the bubble phase.
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_pinch(&mut self, listener: impl Fn(&PinchEvent, &mut Window, &mut App) + 'static) {
        self.pinch_listeners
            .push(Box::new(move |event, phase, hitbox, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    (listener)(event, window, cx);
                }
            }));
    }

    /// Bind the given callback to pinch gesture events during the capture phase.
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_pinch(
        &mut self,
        listener: impl Fn(&PinchEvent, &mut Window, &mut App) + 'static,
    ) {
        self.pinch_listeners
            .push(Box::new(move |event, phase, _hitbox, window, cx| {
                if phase == DispatchPhase::Capture {
                    (listener)(event, window, cx);
                } else {
                    cx.propagate();
                }
            }));
    }

    /// Bind the given callback to an action dispatch during the capture phase.
    /// The imperative API equivalent to [`InteractiveElement::capture_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_action<A: Action>(
        &mut self,
        listener: impl Fn(&A, &mut Window, &mut App) + 'static,
    ) {
        self.action_listeners.push((
            TypeId::of::<A>(),
            Box::new(move |action, phase, window, cx| {
                let action = action.downcast_ref().unwrap();
                if phase == DispatchPhase::Capture {
                    (listener)(action, window, cx)
                } else {
                    cx.propagate();
                }
            }),
        ));
    }

    /// Bind the given callback to an action dispatch during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    #[track_caller]
    pub fn on_action<A: Action>(&mut self, listener: impl Fn(&A, &mut Window, &mut App) + 'static) {
        self.action_listeners.push((
            TypeId::of::<A>(),
            Box::new(move |action, phase, window, cx| {
                let action = action.downcast_ref().unwrap();
                if phase == DispatchPhase::Bubble {
                    (listener)(action, window, cx)
                }
            }),
        ));
    }

    /// Bind the given callback to an action dispatch, based on a dynamic action parameter
    /// instead of a type parameter. Useful for component libraries that want to expose
    /// action bindings to their users.
    /// The imperative API equivalent to [`InteractiveElement::on_boxed_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_boxed_action(
        &mut self,
        action: &dyn Action,
        listener: impl Fn(&dyn Action, &mut Window, &mut App) + 'static,
    ) {
        let action = action.boxed_clone();
        self.action_listeners.push((
            (*action).type_id(),
            Box::new(move |_, phase, window, cx| {
                if phase == DispatchPhase::Bubble {
                    (listener)(&*action, window, cx)
                }
            }),
        ));
    }

    /// Bind the given callback to key down events during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_key_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_key_down(
        &mut self,
        listener: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.key_down_listeners
            .push(Box::new(move |event, phase, window, cx| {
                if phase == DispatchPhase::Bubble {
                    (listener)(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to key down events during the capture phase.
    /// The imperative API equivalent to [`InteractiveElement::capture_key_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_key_down(
        &mut self,
        listener: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.key_down_listeners
            .push(Box::new(move |event, phase, window, cx| {
                if phase == DispatchPhase::Capture {
                    listener(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to key up events during the bubble phase.
    /// The imperative API equivalent to [`InteractiveElement::on_key_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_key_up(&mut self, listener: impl Fn(&KeyUpEvent, &mut Window, &mut App) + 'static) {
        self.key_up_listeners
            .push(Box::new(move |event, phase, window, cx| {
                if phase == DispatchPhase::Bubble {
                    listener(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to key up events during the capture phase.
    /// The imperative API equivalent to [`InteractiveElement::on_key_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn capture_key_up(
        &mut self,
        listener: impl Fn(&KeyUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.key_up_listeners
            .push(Box::new(move |event, phase, window, cx| {
                if phase == DispatchPhase::Capture {
                    listener(event, window, cx)
                }
            }));
    }

    /// Bind the given callback to modifiers changing events.
    /// The imperative API equivalent to [`InteractiveElement::on_modifiers_changed`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_modifiers_changed(
        &mut self,
        listener: impl Fn(&ModifiersChangedEvent, &mut Window, &mut App) + 'static,
    ) {
        self.modifiers_changed_listeners
            .push(Box::new(move |event, window, cx| {
                listener(event, window, cx)
            }));
    }

    /// Bind the given callback to drop events of the given type, whether or not the drag started on this element.
    /// The imperative API equivalent to [`InteractiveElement::on_drop`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_drop<T: 'static>(&mut self, listener: impl Fn(&T, &mut Window, &mut App) + 'static) {
        let type_id = TypeId::of::<T>();
        let on_drop = Box::new(
            move |dragged_any: &dyn Any, _evt: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                listener(dragged_any.downcast_ref().unwrap(), window, cx);
            },
        );
        // Find a pre-existing DropListener for the provided type T, or create a new one if it does not yet exist.
        match self.get_mut_drop_listener(type_id) {
            Some((_, drop_listener)) => {
                drop_listener.on_drop = Some(on_drop);
            }
            None => {
                self.drop_listeners.push((
                    TypeId::of::<T>(),
                    DropListener::<dyn Any> {
                        can_drop: None,
                        on_drag_over: None,
                        on_drop: Some(on_drop),
                    },
                ));
            }
        }
    }

    /// Attaches a [`DropListener`] to this element, which will be notified when a drag is occurring with the provided type.
    /// The imperative API equivalent to [`InteractiveElement::on_drop_alt`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_drop_alt<T: 'static>(&mut self, listener: DropListener<T>) {
        self.drop_listeners
            .push((TypeId::of::<T>(), listener.into_any()));
    }

    fn get_mut_drop_listener(
        &mut self,
        type_id: TypeId,
    ) -> Option<&mut (TypeId, DropListener<dyn Any>)> {
        let mut iter = self.drop_listeners.iter_mut();
        iter.find(|(existing_type_id, _listener)| *existing_type_id == type_id)
    }

    /// Uses the given predicate to determine whether or not a drop event should be dispatched to this element.
    /// Only needed if you have additional logic to do agnostic of drop type and multiple drop listeners.
    /// Otherwise, callers should use [`DropListener::can_drop`].
    ///
    /// The imperative API equivalent to [`InteractiveElement::can_drop`].
    pub fn can_drop(
        &mut self,
        predicate: impl Fn(&dyn Any, &mut Window, &mut App) -> bool + 'static,
    ) {
        self.can_drop_predicate = Some(Box::new(move |value, hitbox, window, cx| {
            hitbox.is_hovered(window) && predicate(value, window, cx)
        }));
    }

    /// Apply the given style when something is dragged over this element.
    /// Will not trigger if the element doesnt have a drop listener for the value being dragged.
    /// Use this when the style of the element is not affected by the type of the dragged item.
    /// Otherwise use [`DropListener::on_drag_over`] or [`InteractiveElement::drag_over`].
    ///
    /// The imperative API equivalent to [`InteractiveElement::on_drag_over_any`].
    pub fn on_drag_over_any(
        &mut self,
        predicate: impl Fn(StyleRefinement, &dyn Any, &mut Window, &mut App) -> StyleRefinement
        + 'static,
    ) {
        self.drag_over_style = Some(Box::new(move |value, window, app| {
            predicate(StyleRefinement::default(), value, window, app)
        }));
    }

    /// Bind the given callback to click events of this element.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_click`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_click(&mut self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static)
    where
        Self: Sized,
    {
        self.click_listeners.push(Rc::new(move |event, window, cx| {
            listener(event, window, cx)
        }));
    }

    /// Bind the given callback to non-primary click events of this element.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_aux_click`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_aux_click(&mut self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static)
    where
        Self: Sized,
    {
        self.aux_click_listeners
            .push(Rc::new(move |event, window, cx| {
                listener(event, window, cx)
            }));
    }

    /// On drag initiation, this callback will be used to create a new view to render the dragged value for a
    /// drag and drop operation. This API should also be used as the equivalent of 'on drag start' with
    /// the [`Self::on_drag_move`] API.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_drag`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_drag<T, W>(
        &mut self,
        value: T,
        constructor: impl Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<W> + 'static,
    ) where
        Self: Sized,
        T: 'static,
        W: 'static + Render,
    {
        let value = Arc::new(value);
        self.on_drag_alt(move |cursor_offset, cursor_style, window, cx| {
            let view = constructor(value.as_ref(), cursor_offset, window, cx);
            AnyDrag {
                view: view.into(),
                value: value.clone(),
                cursor_offset,
                cursor_style,
                external_payload_source: None,
            }
        });
    }

    /// Executed when dragging is initiated. The callback must return a valid [`AnyDrag`]
    /// which contains the app value, a view to render the represented value,
    /// and a way to resolve platform external payload information if desired.
    ///
    /// This API should also be used as the equivalent of 'on drag start' with the [`Self::on_drag_move`] API.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_drag_alt`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_drag_alt<F>(&mut self, listener: F)
    where
        Self: Sized,
        F: 'static + Fn(Point<Pixels>, Option<CursorStyle>, &mut Window, &mut App) -> AnyDrag,
    {
        debug_assert!(
            self.drag_listener.is_none(),
            "calling on_drag more than once on the same element is not supported"
        );
        self.drag_listener = Some(Box::new(listener));
    }

    /// Registers a callback resolving a payload to offer the platform if a drag started by this
    /// element leaves the window. It is invoked at most once per drag gesture, when the pointer
    /// exits the viewport. Must be called after [`Self::on_drag`], with the same dragged value
    /// type `T`.
    pub fn external_drag_payload<T>(
        &mut self,
        resolver: impl Fn(&T, &mut Window, &mut App) -> Option<ExternalDragPayload> + 'static,
    ) where
        Self: Sized,
        T: 'static,
    {
        let Some(drag_listener) = self.drag_listener.take() else {
            debug_assert!(false, "external_drag_payload must be called after on_drag");
            return;
        };

        let resolver = Rc::new(resolver);
        self.on_drag_alt(move |cursor_offset, cursor_style, window, cx| {
            let mut any_drag = drag_listener(cursor_offset, cursor_style, window, cx);
            debug_assert!(
                any_drag.external_payload_source.is_none(),
                "calling external_drag_payload more than once on the same element is not supported"
            );
            any_drag.external_payload_source = Some(Box::new({
                let value = any_drag.value.clone();
                let resolver = resolver.clone();
                move |window, cx| {
                    let value_ref = value
                        .downcast_ref::<T>()
                        .expect("drag value type does not match external payload type");
                    resolver(value_ref, window, cx)
                }
            }));
            any_drag
        });
    }

    /// Bind the given callback on the hover start and end events of this element. Note that the boolean
    /// passed to the callback is true when the hover starts and false when it ends.
    /// Transitions caused by layout changes under a stationary mouse also invoke the callback.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_hover`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_hover(&mut self, listener: impl Fn(&bool, &mut Window, &mut App) + 'static)
    where
        Self: Sized,
    {
        self.on_hover_region(
            |_, _| Some(()),
            move |value, window, cx| listener(&value.is_some(), window, cx),
        );
    }

    /// Resolves this element's cursor from pointer position and bounds.
    /// Return `None` to defer to another cursor request.
    pub fn cursor_with(
        &mut self,
        resolver: impl Fn(Point<Pixels>, Bounds<Pixels>) -> Option<CursorStyle> + 'static,
    ) where
        Self: Sized,
    {
        self.cursor_style_resolver = Some(Rc::new(resolver));
    }

    /// Calls `listener` when the projected pointer value changes.
    /// The projection receives window coordinates and element bounds; return `None` outside active regions.
    /// Layout changes and occlusion are observed independently of event propagation.
    /// Keep listener order stable across renders.
    pub fn on_hover_region<R: PartialEq + 'static>(
        &mut self,
        project: impl Fn(Point<Pixels>, Bounds<Pixels>) -> Option<R> + 'static,
        listener: impl Fn(&Option<R>, &mut Window, &mut App) + 'static,
    ) where
        Self: Sized,
    {
        self.hover_region_listeners
            .push(Box::new(TypedHoverRegionListener {
                project: Rc::new(project),
                callback: Rc::new(listener),
            }));
    }

    /// Use the given callback to construct a new tooltip view when the mouse hovers over this element.
    /// The imperative API equivalent to [`StatefulInteractiveElement::tooltip`].
    pub fn tooltip(&mut self, build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static)
    where
        Self: Sized,
    {
        debug_assert!(
            self.tooltip_builder.is_none(),
            "calling tooltip more than once on the same element is not supported"
        );
        self.tooltip_builder = Some(TooltipBuilder {
            build: Rc::new(build_tooltip),
            hoverable: false,
        });
    }

    /// Use the given callback to construct a new tooltip view when the mouse hovers over this element.
    /// The tooltip itself is also hoverable and won't disappear when the user moves the mouse into
    /// the tooltip. The imperative API equivalent to [`StatefulInteractiveElement::hoverable_tooltip`].
    pub fn hoverable_tooltip(
        &mut self,
        build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) where
        Self: Sized,
    {
        debug_assert!(
            self.tooltip_builder.is_none(),
            "calling tooltip more than once on the same element is not supported"
        );
        self.tooltip_builder = Some(TooltipBuilder {
            build: Rc::new(build_tooltip),
            hoverable: true,
        });
    }

    /// Set the delay before this element's tooltip is shown.
    /// The imperative API equivalent to [`StatefulInteractiveElement::tooltip_show_delay`].
    pub fn tooltip_show_delay(&mut self, delay: Duration) {
        self.tooltip_show_delay = Some(delay);
    }

    /// Block the mouse from all interactions with elements behind this element's hitbox. Typically
    /// `block_mouse_except_scroll` should be preferred.
    ///
    /// The imperative API equivalent to [`InteractiveElement::occlude`]
    pub fn occlude_mouse(&mut self) {
        self.hitbox_behavior = HitboxBehavior::BlockMouse;
    }

    /// Set the bounds of this element as a window control area for the platform window.
    /// The imperative API equivalent to [`InteractiveElement::window_control_area`]
    pub fn window_control_area(&mut self, area: WindowControlArea) {
        self.window_control = Some(area);
    }

    /// Block non-scroll mouse interactions with elements behind this element's hitbox.
    /// The imperative API equivalent to [`InteractiveElement::block_mouse_except_scroll`].
    ///
    /// See [`Hitbox::is_hovered`] for details.
    pub fn block_mouse_except_scroll(&mut self) {
        self.hitbox_behavior = HitboxBehavior::BlockMouseExceptScroll;
    }

    fn has_pinch_listeners(&self) -> bool {
        !self.pinch_listeners.is_empty()
    }

    /// Bind the given callback to be called during prepaint of the element.
    /// The imperative API equivalent to [`StatefulInteractiveElement::on_prepaint`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    pub fn on_prepaint(
        &mut self,
        listener: impl Fn(&InteractivityPrepaint, &mut Window, &mut App) + 'static,
    ) where
        Self: Sized,
    {
        self.prepaint_listeners
            .push(Box::new(move |payload, window, cx| {
                listener(payload, window, cx)
            }));
    }
}

/// A trait for elements that want to use the standard GPUI event handlers that don't
/// require any state.
pub trait InteractiveElement: Sized {
    /// Retrieve the interactivity state associated with this element
    fn interactivity(&mut self) -> &mut Interactivity;

    /// Disable this element's event callbacks and report its disabled state to assistive technology.
    fn disabled(mut self, disabled: bool) -> Self {
        let interactivity = self.interactivity();
        interactivity.disabled = disabled;
        interactivity.aria.disabled = Some(disabled);

        self
    }

    /// Apply the given style when this element is disabled.
    /// Disabled styles take precedence over other interaction styles.
    fn disabled_style(mut self, style: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.interactivity().disabled_style = Some(Box::new(style(StyleRefinement::default())));

        self
    }

    /// Builds transition configuration and animates changes to its style properties.
    ///
    /// An explicit [`.id()`](Self::id) should be used for repeated elements built
    /// at the same call site so each element receives independent motion state.
    #[track_caller]
    fn transitions(mut self, build: impl FnOnce(StyleTransitions) -> StyleTransitions) -> Self {
        let transitions = build(StyleTransitions::new());
        let interactivity = self.interactivity();
        interactivity
            .element_id
            .get_or_insert_with(|| ElementId::CodeLocation(*core::panic::Location::caller()));
        interactivity.style_transitions = Some(Box::new(transitions));
        self
    }

    /// Assign this element to a group of elements that can be styled together
    fn group(mut self, group: impl Into<SharedString>) -> Self {
        self.interactivity().group = Some(group.into());
        self
    }

    /// Assign this element an ID, so that it can be used with interactivity
    fn id(mut self, id: impl Into<ElementId>) -> Stateful<Self> {
        self.interactivity().element_id = Some(id.into());

        Stateful { element: self }
    }

    /// Track the focus state of the given focus handle on this element.
    /// If the focus handle is focused by the application, this element will
    /// apply its focused styles.
    fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.interactivity().focusable = true;
        self.interactivity().tracked_focus_handle = Some(focus_handle.clone());
        self
    }

    /// Set whether this element is a tab stop.
    ///
    /// When false, the element remains in tab-index order but cannot be reached via keyboard navigation.
    /// Useful for container elements: focus the container, then call `window.focus_next(cx)` to focus
    /// the first tab stop inside it while having the container element itself be unreachable via the keyboard.
    /// Should only be used with `tab_index`.
    fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.interactivity().tab_stop = tab_stop;
        self
    }

    /// Set index of the tab stop order, and set this node as a tab stop.
    /// This will default the element to being a tab stop. See [`Self::tab_stop`] for more information.
    /// This should only be used in conjunction with `tab_group`
    /// in order to not interfere with the tab index of other elements.
    fn tab_index(mut self, index: isize) -> Self {
        self.interactivity().focusable = true;
        self.interactivity().tab_index = Some(index);
        self.interactivity().tab_stop = true;
        self
    }

    /// Designate this div as a "tab group". Tab groups have their own location in the tab-index order,
    /// but for children of the tab group, the tab index is reset to 0. This can be useful for swapping
    /// the order of tab stops within the group, without having to renumber all the tab stops in the whole
    /// application.
    fn tab_group(mut self) -> Self {
        self.interactivity().tab_group = true;
        if self.interactivity().tab_index.is_none() {
            self.interactivity().tab_index = Some(0);
        }
        self
    }

    /// Set the keymap context for this element. This will be used to determine
    /// which action to dispatch from the keymap.
    fn key_context<C, E>(mut self, key_context: C) -> Self
    where
        C: TryInto<KeyContext, Error = E>,
        E: std::fmt::Display,
    {
        if let Some(key_context) = key_context.try_into().log_err() {
            self.interactivity().key_context = Some(key_context);
        }
        self
    }

    /// Apply the given style to this element when the mouse hovers over it
    fn hover_style(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        debug_assert!(
            self.interactivity().hover_style.is_none(),
            "hover style already set"
        );
        self.interactivity().hover_style = Some(Box::new(f(StyleRefinement::default())));
        self
    }

    /// Apply the given style to this element when the mouse hovers over it
    #[deprecated(note = "Use InteractiveElement::hover_style instead.")]
    fn hover(self, style: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.hover_style(style)
    }

    /// Apply the given style to this element when the mouse hovers over a group member
    fn group_hover_style(
        mut self,
        group_name: impl Into<SharedString>,
        f: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.interactivity().group_hover_style = Some(GroupStyle {
            group: group_name.into(),
            style: Box::new(f(StyleRefinement::default())),
        });
        self
    }

    /// Apply the given style to this element when the mouse hovers over a group member
    #[deprecated(note = "Use InteractiveElement::group_hover_style instead.")]
    fn group_hover(
        self,
        group_name: impl Into<SharedString>,
        style: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.group_hover_style(group_name, style)
    }

    /// Bind the given callback to the mouse down event for the given mouse button.
    /// The fluent API equivalent to [`Interactivity::on_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to the view state from this callback.
    fn on_mouse_down(
        mut self,
        button: MouseButton,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_down(button, listener);
        self
    }

    #[cfg(any(test, feature = "test-support"))]
    /// Set a key that can be used to look up this element's bounds
    /// in the [`crate::VisualTestContext::debug_bounds`] map
    /// This is a noop in release builds
    fn debug_selector(mut self, f: impl FnOnce() -> String) -> Self {
        self.interactivity().debug_selector = Some(f());
        self
    }

    #[cfg(not(any(test, feature = "test-support")))]
    /// Set a key that can be used to look up this element's bounds
    /// in the [`crate::VisualTestContext::debug_bounds`] map
    /// This is a noop in release builds
    #[inline]
    fn debug_selector(self, _: impl FnOnce() -> String) -> Self {
        self
    }

    /// Bind the given callback to the mouse down event for any button, during the capture phase.
    /// The fluent API equivalent to [`Interactivity::capture_any_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn capture_any_mouse_down(
        mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().capture_any_mouse_down(listener);
        self
    }

    /// Bind the given callback to the mouse down event for any button, during the capture phase.
    /// The fluent API equivalent to [`Interactivity::on_any_mouse_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_any_mouse_down(
        mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_any_mouse_down(listener);
        self
    }

    /// Bind the given callback to the mouse up event for the given button, during the bubble phase.
    /// The fluent API equivalent to [`Interactivity::on_mouse_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_up(
        mut self,
        button: MouseButton,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_up(button, listener);
        self
    }

    /// Bind the given callback to the mouse up event for any button, during the capture phase.
    /// The fluent API equivalent to [`Interactivity::capture_any_mouse_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn capture_any_mouse_up(
        mut self,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().capture_any_mouse_up(listener);
        self
    }

    /// Bind the given callback to the mouse pressure event, during the bubble phase
    /// the fluent API equivalent to [`Interactivity::on_mouse_pressure`]
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_pressure(
        mut self,
        listener: impl Fn(&MousePressureEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_pressure(listener);
        self
    }

    /// Bind the given callback to the mouse pressure event, during the capture phase
    /// the fluent API equivalent to [`Interactivity::on_mouse_pressure`]
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn capture_mouse_pressure(
        mut self,
        listener: impl Fn(&MousePressureEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().capture_mouse_pressure(listener);
        self
    }

    /// Bind the given callback to the mouse down event, on any button, during the capture phase,
    /// when the mouse is outside of the bounds of this element.
    /// The fluent API equivalent to [`Interactivity::on_mouse_down_out`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_down_out(
        mut self,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_down_out(listener);
        self
    }

    /// Bind the given callback to the mouse down event - for all mouse up events,
    /// regardless of dispatch phase, mouse button, or whether the mouse is
    /// inside or outside the bounds of the element.
    /// The fluent API equivalent to [`Interactivity::on_mouse_up_out`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_down_all(
        mut self,
        listener: impl Fn(&MouseDownEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_down_all(listener);
        self
    }

    /// Bind the given callback to the mouse up event, for the given button, during the capture phase,
    /// when the mouse is outside of the bounds of this element.
    /// The fluent API equivalent to [`Interactivity::on_mouse_up_out`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_up_out(
        mut self,
        button: MouseButton,
        listener: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_up_out(button, listener);
        self
    }

    /// Bind the given callback to the mouse up event - for all mouse up events,
    /// regardless of dispatch phase, mouse button, or whether the mouse is
    /// inside or outside the bounds of the element.
    /// The fluent API equivalent to [`Interactivity::on_mouse_up_out`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_up_all(
        mut self,
        listener: impl Fn(&MouseUpEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_up_all(listener);
        self
    }

    /// Bind the given callback to the mouse move event, during the bubble phase.
    /// The fluent API equivalent to [`Interactivity::on_mouse_move`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_move(
        mut self,
        listener: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_move(listener);
        self
    }

    /// Bind the given callback to the mouse move event, triggering even if the mouse is outside the element's hitbox.
    /// The fluent API equivalent to [`Interactivity::on_mouse_move_all`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_move_all(
        mut self,
        listener: impl Fn(&MouseMoveEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_move_all(listener);
        self
    }

    /// Bind the given callback to the mouse exit event, during the bubble phase.
    /// The fluent API equivalent to [`Interactivity::on_mouse_exit`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_mouse_exit(
        mut self,
        listener: impl Fn(&MouseExitEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_mouse_exit(listener);
        self
    }

    /// Bind the given callback to the mouse drag event of the given type. Note that this
    /// will be called for all move events, inside or outside of this element, as long as the
    /// drag was started with this element under the mouse. Useful for implementing draggable
    /// UIs that don't conform to a drag and drop style interaction, like resizing.
    /// The fluent API equivalent to [`Interactivity::on_drag_move`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_drag_move<T: 'static>(
        mut self,
        listener: impl Fn(&DragMoveEvent<T>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_drag_move(listener);
        self
    }

    /// Bind the given callback to scroll wheel events during the bubble phase.
    /// The fluent API equivalent to [`Interactivity::on_scroll_wheel`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_scroll_wheel(
        mut self,
        listener: impl Fn(&ScrollWheelEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_scroll_wheel(listener);
        self
    }

    /// Bind the given callback to pinch gesture events during the bubble phase.
    /// The fluent API equivalent to [`Interactivity::on_pinch`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_pinch(mut self, listener: impl Fn(&PinchEvent, &mut Window, &mut App) + 'static) -> Self {
        self.interactivity().on_pinch(listener);
        self
    }

    /// Bind the given callback to pinch gesture events during the capture phase.
    /// The fluent API equivalent to [`Interactivity::capture_pinch`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn capture_pinch(
        mut self,
        listener: impl Fn(&PinchEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().capture_pinch(listener);
        self
    }
    /// Capture the given action, before normal action dispatch can fire.
    /// The fluent API equivalent to [`Interactivity::capture_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn capture_action<A: Action>(
        mut self,
        listener: impl Fn(&A, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().capture_action(listener);
        self
    }

    /// Bind the given callback to an action dispatch during the bubble phase.
    /// The fluent API equivalent to [`Interactivity::on_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    #[track_caller]
    fn on_action<A: Action>(
        mut self,
        listener: impl Fn(&A, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_action(listener);
        self
    }

    /// Bind the given callback to an action dispatch, based on a dynamic action parameter
    /// instead of a type parameter. Useful for component libraries that want to expose
    /// action bindings to their users.
    /// The fluent API equivalent to [`Interactivity::on_boxed_action`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_boxed_action(
        mut self,
        action: &dyn Action,
        listener: impl Fn(&dyn Action, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_boxed_action(action, listener);
        self
    }

    /// Bind the given callback to key down events during the bubble phase.
    /// The fluent API equivalent to [`Interactivity::on_key_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_key_down(
        mut self,
        listener: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_key_down(listener);
        self
    }

    /// Bind the given callback to key down events during the capture phase.
    /// The fluent API equivalent to [`Interactivity::capture_key_down`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn capture_key_down(
        mut self,
        listener: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().capture_key_down(listener);
        self
    }

    /// Bind the given callback to key up events during the bubble phase.
    /// The fluent API equivalent to [`Interactivity::on_key_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_key_up(
        mut self,
        listener: impl Fn(&KeyUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_key_up(listener);
        self
    }

    /// Bind the given callback to key up events during the capture phase.
    /// The fluent API equivalent to [`Interactivity::capture_key_up`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn capture_key_up(
        mut self,
        listener: impl Fn(&KeyUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().capture_key_up(listener);
        self
    }

    /// Bind the given callback to modifiers changing events.
    /// The fluent API equivalent to [`Interactivity::on_modifiers_changed`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_modifiers_changed(
        mut self,
        listener: impl Fn(&ModifiersChangedEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_modifiers_changed(listener);
        self
    }

    /// Apply the given style when the given data type is dragged over this element
    #[deprecated(note = "use [`on_drag_over`]")]
    fn drag_over<S: 'static>(
        mut self,
        f: impl 'static + Fn(StyleRefinement, &S, &mut Window, &mut App) -> StyleRefinement,
    ) -> Self {
        self.on_drag_over(f)
    }

    /// Apply the given style when the given data type is dragged over this element
    fn on_drag_over<S: 'static>(
        mut self,
        f: impl 'static + Fn(StyleRefinement, &S, &mut Window, &mut App) -> StyleRefinement,
    ) -> Self {
        match self
            .interactivity()
            .get_mut_drop_listener(TypeId::of::<S>())
        {
            Some((_, drop_listener)) => {
                drop_listener.on_drag_over = Some(Box::new(move |value_any, window, cx| {
                    let value = value_any.downcast_ref::<S>().unwrap();
                    f(StyleRefinement::default(), value, window, cx)
                }))
            }
            None => {
                self.interactivity().on_drop_alt(DropListener::<S> {
                    can_drop: None,
                    on_drag_over: Some(Box::new(move |value, window, cx| {
                        f(StyleRefinement::default(), value, window, cx)
                    })),
                    on_drop: None,
                });
            }
        }
        self
    }

    /// Apply the given style when something is dragged over this element
    /// The fluent API equivalent to [`Interactivity::on_drag_over_any`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_drag_over_any(
        mut self,
        f: impl 'static + Fn(StyleRefinement, &dyn Any, &mut Window, &mut App) -> StyleRefinement,
    ) -> Self {
        self.interactivity().on_drag_over_any(f);
        self
    }

    /// Apply the given style when the given data type is dragged over this element's group
    fn group_drag_over_style<S: 'static>(
        mut self,
        group_name: impl Into<SharedString>,
        f: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.interactivity().group_drag_over_styles.push((
            TypeId::of::<S>(),
            GroupStyle {
                group: group_name.into(),
                style: Box::new(f(StyleRefinement::default())),
            },
        ));
        self
    }

    /// Apply the given style when the given data type is dragged over this element's group
    #[deprecated(note = "Use InteractiveElement::group_drag_over_style instead.")]
    fn group_drag_over<Drag: 'static>(
        self,
        group_name: impl Into<SharedString>,
        style: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.group_drag_over_style::<Drag>(group_name, style)
    }

    /// Bind the given callback to drop events of the given type, whether or not the drag started on this element.
    /// The fluent API equivalent to [`Interactivity::on_drop`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_drop<T: 'static>(
        mut self,
        listener: impl Fn(&T, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.interactivity().on_drop(listener);
        self
    }

    /// Attaches a [`DropListener`] to this element, which will be notified when a drag is occurring with the provided type.
    /// The fluent API equivalent to [`Interactivity::on_drop_alt`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_drop_alt<T: 'static>(mut self, listener: DropListener<T>) -> Self {
        self.interactivity().on_drop_alt(listener);
        self
    }

    /// Use the given predicate to determine whether or not a drop event should be dispatched to this element.
    /// Only needed if you have additional logic to do agnostic of drop type and multiple drop listeners.
    /// Otherwise, callers should use [`DropListener::can_drop`].
    ///
    /// The fluent API equivalent to [`Interactivity::can_drop`].
    fn can_drop(
        mut self,
        predicate: impl Fn(&dyn Any, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.interactivity().can_drop(predicate);
        self
    }

    /// Block the mouse from all interactions with elements behind this element's hitbox. Typically
    /// `block_mouse_except_scroll` should be preferred.
    /// The fluent API equivalent to [`Interactivity::occlude_mouse`].
    fn occlude(mut self) -> Self {
        self.interactivity().occlude_mouse();
        self
    }

    /// Set the bounds of this element as a window control area for the platform window.
    /// The fluent API equivalent to [`Interactivity::window_control_area`].
    fn window_control_area(mut self, area: WindowControlArea) -> Self {
        self.interactivity().window_control_area(area);
        self
    }

    /// Block non-scroll mouse interactions with elements behind this element's hitbox.
    /// The fluent API equivalent to [`Interactivity::block_mouse_except_scroll`].
    ///
    /// See [`Hitbox::is_hovered`] for details.
    fn block_mouse_except_scroll(mut self) -> Self {
        self.interactivity().block_mouse_except_scroll();
        self
    }

    /// Set the given styles to be applied when this element, specifically, is focused.
    /// Requires that the element is focusable. Elements can be made focusable using [`InteractiveElement::track_focus`].
    fn focus_style(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self
    where
        Self: Sized,
    {
        self.interactivity().focus_style = Some(Box::new(f(StyleRefinement::default())));
        self
    }

    /// Set the given styles to be applied when this element, specifically, is focused.
    /// Requires that the element is focusable. Elements can be made focusable using [`InteractiveElement::track_focus`].
    #[deprecated(note = "Use InteractiveElement::focus_style instead.")]
    fn focus(self, style: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.focus_style(style)
    }

    /// Set the given styles to be applied when this element is inside another element that is focused.
    /// Requires that the element is focusable. Elements can be made focusable using [`InteractiveElement::track_focus`].
    fn in_focus_style(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self
    where
        Self: Sized,
    {
        self.interactivity().in_focus_style = Some(Box::new(f(StyleRefinement::default())));
        self
    }

    /// Set the given styles to be applied when this element is inside another element that is focused.
    /// Requires that the element is focusable. Elements can be made focusable using [`InteractiveElement::track_focus`].
    #[deprecated(note = "Use InteractiveElement::in_focus_style instead.")]
    fn in_focus(self, style: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.in_focus_style(style)
    }

    /// Applies styles when a descendant of this element has keyboard focus.
    fn contains_focus(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self
    where
        Self: Sized,
    {
        self.interactivity().contains_focus_style = Some(Box::new(f(StyleRefinement::default())));
        self
    }

    /// Set the given styles to be applied when this element is focused via keyboard navigation.
    /// This is similar to CSS's `:focus-visible` pseudo-class - it only applies when the element
    /// is focused AND the user is navigating via keyboard (not mouse clicks).
    /// Requires that the element is focusable. Elements can be made focusable using [`InteractiveElement::track_focus`].
    fn focus_visible_style(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self
    where
        Self: Sized,
    {
        self.interactivity().focus_visible_style = Some(Box::new(f(StyleRefinement::default())));
        self
    }

    /// Set the given styles to be applied when this element is focused via keyboard navigation.
    /// This is similar to CSS's `:focus-visible` pseudo-class - it only applies when the element
    /// is focused AND the user is navigating via keyboard (not mouse clicks).
    /// Requires that the element is focusable. Elements can be made focusable using [`InteractiveElement::track_focus`].
    #[deprecated(note = "Use InteractiveElement::focus_visible_style instead.")]
    fn focus_visible(self, style: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.focus_visible_style(style)
    }

    /// Resolves this element's cursor from pointer position and bounds.
    /// Return `None` to defer to another cursor request.
    fn cursor_with(
        mut self,
        resolver: impl Fn(Point<Pixels>, Bounds<Pixels>) -> Option<CursorStyle> + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().cursor_with(resolver);
        self
    }
}

/// A trait for elements that want to use the standard GPUI interactivity features
/// that require state.
pub trait StatefulInteractiveElement: InteractiveElement {
    /// Set the accessible role for this element.
    ///
    /// See the [accessibility guide](crate::_accessibility) for an overview.
    fn role(mut self, role: accesskit::Role) -> Self {
        debug_assert!(
            role != accesskit::Role::GenericContainer,
            "GenericContainer is filtered out of the a11y tree and has no effect"
        );
        self.interactivity().override_role = Some(role);
        self
    }

    /// Set the author-provided identifier exposed to accessibility clients.
    ///
    /// Unlike the GPUI element ID, this value is visible outside the process.
    /// Keep it stable and unique within its accessibility tree.
    /// AccessKit maps it to platform identifiers where supported, including
    /// UIA `AutomationId` on Windows, `AXIdentifier` on macOS, and AT-SPI
    /// `AccessibleId` on Linux stacks whose deployed adapter exposes it.
    fn accessibility_id(mut self, id: impl Into<SharedString>) -> Self {
        self.interactivity().aria.author_id = Some(id.into());
        self
    }

    /// Set the accessible label for this element.
    fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.interactivity().aria.label = Some(label.into());
        self
    }

    /// Mark the current input value as invalid for assistive technology.
    fn aria_invalid(mut self, invalid: bool) -> Self {
        self.interactivity().aria.invalid = invalid.then_some(accesskit::Invalid::True);
        self
    }

    /// Set the accessible description for this element.
    fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.interactivity().aria.description = Some(description.into());
        self
    }

    /// Set the keyboard shortcut(s) that activate this element, announced by
    /// assistive technology (maps to AccessKit's `keyboard_shortcut`).
    ///
    /// Note that this does not create a keymap. It simply instructs assistive
    /// technology what the keymap is.
    fn aria_keyshortcuts(mut self, keyshortcuts: impl Into<SharedString>) -> Self {
        self.interactivity().aria.keyshortcuts = Some(keyshortcuts.into());
        self
    }

    /// Hide this element and its descendants from assistive technology.
    ///
    /// This does not change rendering, focus, or input. Do not use it on an
    /// element that can receive focus or contains focusable descendants.
    fn aria_hidden(mut self) -> Self {
        self.interactivity().aria.hidden = true;
        self
    }

    /// Report this element as the focused node in the accessibility tree,
    /// overriding the element that holds real keyboard focus — but only while
    /// one of its ancestors actually holds focus.
    ///
    /// This implements the `aria-activedescendant` pattern for composite
    /// widgets that keep keyboard focus on a container (e.g. a menu or
    /// listbox) while a child is "selected": set this on the selected child so
    /// assistive technology announces and highlights it as focused.
    ///
    /// The element must also have a [`role`][Self::role] (and an id) so it
    /// produces an accessibility node. Unlike the web's container-side
    /// `aria-activedescendant`, this is set on the descendant; GPUI honors it
    /// only when a focused ancestor is present in the tree, so it is safe to
    /// set unconditionally on the selected child — if the container isn't
    /// focused, the claim is ignored.
    fn aria_active_descendant(mut self) -> Self {
        self.interactivity().report_active_descendant_focus = true;
        self
    }

    /// Contribute synthetic accessibility nodes — nodes that don't correspond
    /// to any element — as children of this element's a11y node. For example,
    /// text runs describing an editor's text content.
    ///
    /// GPUI calls the closure after prepaint. The element must have an id and
    /// either a [`role`][StatefulInteractiveElement::role] or
    /// [`aria_hidden()`][StatefulInteractiveElement::aria_hidden].
    ///
    /// See [`Element::a11y_synthetic_children`] for details.
    fn a11y_synthetic_children(
        mut self,
        f: impl FnOnce(&mut crate::A11ySubtreeBuilder) + 'static,
    ) -> Self {
        self.interactivity().a11y_synthetic_children = Some(Box::new(f));
        self
    }

    /// Set the selected state for this element.
    fn aria_selected(mut self, selected: bool) -> Self {
        self.interactivity().aria.selected = Some(selected);
        self
    }

    /// Set the expanded state for this element.
    fn aria_expanded(mut self, expanded: bool) -> Self {
        self.interactivity().aria.expanded = Some(expanded);
        self
    }

    /// Set the toggled state for this element.
    fn aria_toggled(mut self, toggled: accesskit::Toggled) -> Self {
        self.interactivity().aria.toggled = Some(toggled);
        self
    }

    /// Set the disabled state exposed to assistive technology, using AccessKit's `Disabled` flag.
    /// Use [`InteractiveElement::disabled`] to also suppress event callbacks.
    fn aria_disabled(mut self, disabled: bool) -> Self {
        self.interactivity().aria.disabled = Some(disabled);
        self
    }

    /// Set the numeric value for this element.
    fn aria_numeric_value(mut self, value: f64) -> Self {
        self.interactivity().aria.numeric_value = Some(value);
        self
    }

    /// Set the step by which assistive technology should expect the numeric
    /// value of this element to change (e.g. when incrementing a spin button).
    fn aria_numeric_value_step(mut self, step: f64) -> Self {
        self.interactivity().aria.numeric_value_step = Some(step);
        self
    }

    /// Set the string value of this element, e.g. the text content of a simple
    /// text input.
    fn aria_value(mut self, value: impl Into<SharedString>) -> Self {
        self.interactivity().aria.value = Some(value.into());
        self
    }

    /// Set the placeholder text reported to assistive technology for this
    /// element, shown when a text input is empty.
    fn aria_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.interactivity().aria.placeholder = Some(placeholder.into());
        self
    }

    /// Set the minimum numeric value for this element.
    fn aria_min_numeric_value(mut self, value: f64) -> Self {
        self.interactivity().aria.min_numeric_value = Some(value);
        self
    }

    /// Set the maximum numeric value for this element.
    fn aria_max_numeric_value(mut self, value: f64) -> Self {
        self.interactivity().aria.max_numeric_value = Some(value);
        self
    }

    /// Set the orientation of this element.
    fn aria_orientation(mut self, orientation: accesskit::Orientation) -> Self {
        self.interactivity().aria.orientation = Some(orientation);
        self
    }

    /// Set the heading level of this element.
    fn aria_level(mut self, level: usize) -> Self {
        self.interactivity().aria.level = Some(level);
        self
    }

    /// Set the position in set of this element.
    fn aria_position_in_set(mut self, position: usize) -> Self {
        self.interactivity().aria.position_in_set = Some(position);
        self
    }

    /// Set the size of set for this element.
    fn aria_size_of_set(mut self, size: usize) -> Self {
        self.interactivity().aria.size_of_set = Some(size);
        self
    }

    /// Set the row index for this element.
    fn aria_row_index(mut self, index: usize) -> Self {
        self.interactivity().aria.row_index = Some(index);
        self
    }

    /// Set the column index for this element.
    fn aria_column_index(mut self, index: usize) -> Self {
        self.interactivity().aria.column_index = Some(index);
        self
    }

    /// Set the row count for this element.
    fn aria_row_count(mut self, count: usize) -> Self {
        self.interactivity().aria.row_count = Some(count);
        self
    }

    /// Set the column count for this element.
    fn aria_column_count(mut self, count: usize) -> Self {
        self.interactivity().aria.column_count = Some(count);
        self
    }

    /// Mark this element (typically a dialog) as explicitly modal, so
    /// assistive technology treats the rest of the window as inert while it
    /// is open (maps to AccessKit's `Modal` flag).
    fn aria_modal(mut self, modal: bool) -> Self {
        self.interactivity().aria.modal = Some(modal);
        self
    }

    /// Set how urgently assistive technology should announce updates to this
    /// element, as an ARIA live region. See [`accesskit::Live`].
    fn aria_live(mut self, live: accesskit::Live) -> Self {
        self.interactivity().aria.live = Some(live);
        self
    }

    /// Set whether updates to this live region should be announced as a
    /// whole rather than describing only the changed part (maps to
    /// AccessKit's `LiveAtomic` flag). Only meaningful alongside
    /// [`aria_live`][Self::aria_live].
    fn aria_live_atomic(mut self, atomic: bool) -> Self {
        self.interactivity().aria.live_atomic = Some(atomic);
        self
    }

    /// Register a handler for an accessibility action on this element.
    /// The handler is called when a screen reader requests the given action.
    ///
    /// See the [accessibility guide](crate::_accessibility) for an overview.
    fn on_a11y_action(
        mut self,
        action: accesskit::Action,
        listener: impl FnMut(Option<&accesskit::ActionData>, &mut crate::Window, &mut crate::App)
        + 'static,
    ) -> Self {
        self.interactivity()
            .a11y_action_listeners
            .push((action, Box::new(listener)));
        self
    }

    /// Set this element to focusable.
    fn focusable(mut self) -> Self {
        self.interactivity().focusable = true;
        self
    }

    /// Set the overflow x and y to scroll.
    fn overflow_scroll(mut self) -> Self {
        self.interactivity().base_style.overflow.x = Some(Overflow::Scroll);
        self.interactivity().base_style.overflow.y = Some(Overflow::Scroll);
        self
    }

    /// Set the overflow x to scroll.
    fn overflow_x_scroll(mut self) -> Self {
        self.interactivity().base_style.overflow.x = Some(Overflow::Scroll);
        self
    }

    /// Set the overflow y to scroll.
    fn overflow_y_scroll(mut self) -> Self {
        self.interactivity().base_style.overflow.y = Some(Overflow::Scroll);
        self
    }

    /// Restrict scrolling of this element to the axis of the input gesture.
    ///
    /// See [`Style::restrict_scroll_to_axis`](crate::Style::restrict_scroll_to_axis) for details.
    fn restrict_scroll_to_axis(mut self) -> Self {
        self.interactivity().base_style.restrict_scroll_to_axis = Some(true);
        self
    }

    /// Track the scroll state of this element with the given handle.
    fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.interactivity().tracked_scroll_handle = Some(scroll_handle.clone());
        self
    }

    /// Track the scroll state of this element with the given handle.
    fn anchor_scroll(mut self, scroll_anchor: Option<ScrollAnchor>) -> Self {
        self.interactivity().scroll_anchor = scroll_anchor;
        self
    }

    /// Set the given styles to be applied when this element is active.
    fn active_style(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self
    where
        Self: Sized,
    {
        self.interactivity().active_style = Some(Box::new(f(StyleRefinement::default())));
        self
    }

    /// Set the given styles to be applied when this element is active.
    #[deprecated(note = "Use StatefulInteractiveElement::active_style instead.")]
    fn active(self, style: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.active_style(style)
    }

    /// Set the given styles to be applied when this element's group is active.
    fn group_active_style(
        mut self,
        group_name: impl Into<SharedString>,
        f: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().group_active_style = Some(GroupStyle {
            group: group_name.into(),
            style: Box::new(f(StyleRefinement::default())),
        });
        self
    }

    /// Set the given styles to be applied when this element's group is active.
    #[deprecated(note = "Use StatefulInteractiveElement::group_active_style instead.")]
    fn group_active(
        self,
        group_name: impl Into<SharedString>,
        style: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.group_active_style(group_name, style)
    }

    /// Bind the given callback to click events of this element.
    /// The fluent API equivalent to [`Interactivity::on_click`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_click(mut self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self
    where
        Self: Sized,
    {
        self.interactivity().on_click(listener);
        self
    }

    /// Bind the given callback to non-primary click events of this element.
    /// The fluent API equivalent to [`Interactivity::on_aux_click`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_aux_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().on_aux_click(listener);
        self
    }

    /// On drag initiation, this callback will be used to create a new view to render the dragged value for a
    /// drag and drop operation. This API should also be used as the equivalent of 'on drag start' with
    /// the [`InteractiveElement::on_drag_move`] API.
    /// The callback also has access to the offset of triggering click from the origin of parent element.
    /// The fluent API equivalent to [`Interactivity::on_drag`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_drag<T, W>(
        mut self,
        value: T,
        constructor: impl Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<W> + 'static,
    ) -> Self
    where
        Self: Sized,
        T: 'static,
        W: 'static + Render,
    {
        self.interactivity().on_drag(value, constructor);
        self
    }

    /// Executed when dragging is initiated. The callback must return a valid [`AnyDrag`]
    /// which contains the app value, a view to render the represented value,
    /// and a way to resolve platform external payload information if desired.
    ///
    /// This API should also be used as the equivalent of 'on drag start' with the [`InteractiveElement::on_drag_move`] API.
    /// The callback also has access to the offset of triggering click from the origin of parent element.
    /// The fluent API equivalent to [`Interactivity::on_drag`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_drag_alt<F>(mut self, listener: F) -> Self
    where
        Self: Sized,
        F: 'static + Fn(Point<Pixels>, Option<CursorStyle>, &mut Window, &mut App) -> AnyDrag,
    {
        self.interactivity().on_drag_alt(listener);
        self
    }

    /// Registers a callback resolving a payload to offer the platform if a drag started by this
    /// element leaves the window. It is invoked at most once per drag gesture, when the pointer
    /// exits the viewport. Must be called after [`Self::on_drag`], with the same dragged value
    /// type `T`.
    /// The fluent API equivalent to [`Interactivity::external_drag_payload`].
    fn external_drag_payload<T>(
        mut self,
        resolver: impl Fn(&T, &mut Window, &mut App) -> Option<ExternalDragPayload> + 'static,
    ) -> Self
    where
        Self: Sized,
        T: 'static,
    {
        self.interactivity().external_drag_payload(resolver);
        self
    }

    /// Bind the given callback on the hover start and end events of this element. Note that the boolean
    /// passed to the callback is true when the hover starts and false when it ends.
    /// Transitions caused by layout changes under a stationary mouse also invoke the callback.
    /// The fluent API equivalent to [`Interactivity::on_hover`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_hover(mut self, listener: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self
    where
        Self: Sized,
    {
        self.interactivity().on_hover(listener);
        self
    }

    /// Calls the listener when a typed pointer projection changes.
    fn on_hover_region<R: PartialEq + 'static>(
        mut self,
        project: impl Fn(Point<Pixels>, Bounds<Pixels>) -> Option<R> + 'static,
        listener: impl Fn(&Option<R>, &mut Window, &mut App) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().on_hover_region(project, listener);
        self
    }

    /// Use the given callback to construct a new tooltip view when the mouse hovers over this element.
    /// The fluent API equivalent to [`Interactivity::tooltip`].
    fn tooltip(mut self, build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self
    where
        Self: Sized,
    {
        self.interactivity().tooltip(build_tooltip);
        self
    }

    /// Use the given callback to construct a new tooltip view when the mouse hovers over this element.
    /// The tooltip itself is also hoverable and won't disappear when the user moves the mouse into
    /// the tooltip. The fluent API equivalent to [`Interactivity::hoverable_tooltip`].
    fn hoverable_tooltip(
        mut self,
        build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().hoverable_tooltip(build_tooltip);
        self
    }

    /// Set the delay before this element's tooltip is shown.
    /// The fluent API equivalent to [`Interactivity::tooltip_show_delay`].
    fn tooltip_show_delay(mut self, delay: Duration) -> Self
    where
        Self: Sized,
    {
        self.interactivity().tooltip_show_delay(delay);
        self
    }

    /// Bind the given callback to execute before the element's prepaint.
    /// The fluent API equivalent to [`Interactivity::on_prepaint`].
    ///
    /// See [`Context::listener`](crate::Context::listener) to get access to a view's state from this callback.
    fn on_prepaint(
        mut self,
        listener: impl Fn(&InteractivityPrepaint, &mut Window, &mut App) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().on_prepaint(listener);
        self
    }
}

/// Describes the known state of an Interactivity element before prepaint begins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InteractivityPrepaint {
    /// The bounds of the element
    pub bounds: Bounds<Pixels>,
    /// The size of the contents of the element
    pub content_size: Size<Pixels>,
}
pub(crate) type PrepaintListener =
    Box<dyn Fn(&InteractivityPrepaint, &mut Window, &mut App) + 'static>;

pub(crate) type MouseDownListener =
    Box<dyn Fn(&MouseDownEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static>;
pub(crate) type MouseUpListener =
    Box<dyn Fn(&MouseUpEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static>;
pub(crate) type MousePressureListener =
    Box<dyn Fn(&MousePressureEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static>;
pub(crate) type MouseMoveListener =
    Box<dyn Fn(&MouseMoveEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static>;
pub(crate) type MouseExitListener =
    Box<dyn Fn(&MouseExitEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static>;

pub(crate) type ScrollWheelListener =
    Box<dyn Fn(&ScrollWheelEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static>;

pub(crate) type PinchListener =
    Box<dyn Fn(&PinchEvent, DispatchPhase, &Hitbox, &mut Window, &mut App) + 'static>;

pub(crate) type ClickListener = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type PointerProjector<R> = Rc<dyn Fn(Point<Pixels>, Bounds<Pixels>) -> Option<R>>;
pub(crate) type CursorStyleResolver = PointerProjector<CursorStyle>;
type HoverRegionCallback<R> = Rc<dyn Fn(&Option<R>, &mut Window, &mut App)>;
type HoverRegionObserver = Rc<dyn Fn(Option<Point<Pixels>>, Bounds<Pixels>, &mut Window, &mut App)>;

/// Erases only the stored region type.
#[derive(Default)]
pub(crate) struct HoverRegionState(Option<Rc<dyn Any>>);

pub(crate) trait HoverRegionListener {
    fn bind(&self, state: &mut HoverRegionState) -> HoverRegionObserver;
}

struct TypedHoverRegionListener<R> {
    project: PointerProjector<R>,
    callback: HoverRegionCallback<R>,
}

impl<R: PartialEq + 'static> HoverRegionListener for TypedHoverRegionListener<R> {
    fn bind(&self, state: &mut HoverRegionState) -> HoverRegionObserver {
        // Reset state if this registration changes type.
        let value = state
            .0
            .as_ref()
            .and_then(|state| state.clone().downcast::<RefCell<Rc<Option<R>>>>().ok())
            .unwrap_or_else(|| Rc::new(RefCell::new(Rc::new(None))));
        state.0 = Some(value.clone());
        let project = self.project.clone();
        let callback = self.callback.clone();
        Rc::new(move |position, bounds, window, cx| {
            let next = position.and_then(|position| project(position, bounds));
            let mut previous = value.borrow_mut();
            if **previous == next {
                return;
            }
            let next = Rc::new(next);
            *previous = next.clone();
            // Release the borrow before callbacks can redraw or dispatch input.
            // Keep this snapshot alive across reentrant updates.
            drop(previous);
            callback(&next, window, cx);
        })
    }
}

pub(crate) type DragListener =
    Box<dyn Fn(Point<Pixels>, Option<CursorStyle>, &mut Window, &mut App) -> AnyDrag + 'static>;

/// Function type for [`DropListener::can_drop`], which checks if a typed payload can be dropped on an element's hitbox.
pub type CanDropListenerFn<T> = Box<dyn Fn(&T, &Hitbox, &mut Window, &mut App) -> bool + 'static>;
/// Function type for [`DropListener::on_drag_over`].
pub type OnDragOverFn<T> = Box<dyn Fn(&T, &mut Window, &mut App) -> StyleRefinement + 'static>;
/// Function type for [`DropListener::on_drop`].
pub type OnDropFn<T> = Box<dyn Fn(&T, &MouseUpEvent, &mut Window, &mut App) + 'static>;

/// Behaviors for an interactive element when receiving a dragged element that could be or has been dropped it.
pub struct DropListener<T: ?Sized> {
    /// Called when a dragged value of the matching type could be dropped on the owning element.
    pub can_drop: Option<CanDropListenerFn<T>>,
    /// If a drag value is moving over the owning element,
    /// [`Interactivity::can_drop`] is none or returns true, and [`can_drop`](Self::can_drop) is None or returns true,
    /// then this is called to refine the style of the owning element.
    pub on_drag_over: Option<OnDragOverFn<T>>,
    /// If a drag value is dropped over the owning element,
    /// [`Interactivity::can_drop`] is none or returns true, and [`can_drop`](Self::can_drop) is None or returns true,
    /// then this is called to process the dropped value.
    pub on_drop: Option<OnDropFn<T>>,
}
impl<T> Default for DropListener<T> {
    fn default() -> Self {
        Self {
            can_drop: None,
            on_drag_over: None,
            on_drop: None,
        }
    }
}
impl<T: Sized + 'static> DropListener<T> {
    /// Binds the given predicate to determine whether or not drop events should be dispatched to this handler.
    pub fn can_drop_within(
        mut self,
        predicate: impl Fn(&T, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.can_drop_anywhere(move |value, hitbox, window, cx| {
            hitbox.is_hovered(window) && predicate(value, window, cx)
        })
    }

    /// Binds the given predicate to determine whether or not drop events should be dispatched to this handler.
    pub fn can_drop_anywhere(
        mut self,
        predicate: impl Fn(&T, &Hitbox, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.can_drop = Some(Box::new(predicate));
        self
    }

    /// If a value of type [`T`] is dragged over the owning element,
    /// [`Interactivity::can_drop`] is none or returns true, and [`can_drop`](Self::can_drop) is None or returns true,
    /// then this is called to provide style changes to apply to the owning element.
    pub fn on_drag_over(
        mut self,
        predicate: impl Fn(StyleRefinement, &T, &mut Window, &mut App) -> StyleRefinement + 'static,
    ) -> Self {
        self.on_drag_over = Some(Box::new(move |value, window, cx| {
            predicate(StyleRefinement::default(), value, window, cx)
        }));
        self
    }

    /// If a value of type [`T`] is dropped on the owning element,
    /// [`Interactivity::can_drop`] is none or returns true, and [`can_drop`](Self::can_drop) is None or returns true,
    /// then this is called to process the dropped value.
    pub fn on_drop(
        mut self,
        predicate: impl Fn(&T, &MouseUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drop = Some(Box::new(predicate));
        self
    }

    /// Converts the typed listener into one that can be stored as type-agnostic within an element.
    ///
    /// Effectively wraps all typed callbacks in [`Any::downcast_ref`](https://doc.rust-lang.org/stable/std/any/trait.Any.html#method.downcast_ref) boxed functions.
    pub fn into_any(self) -> DropListener<dyn Any> {
        let DropListener {
            can_drop,
            on_drag_over,
            on_drop,
        } = self;
        let can_drop = can_drop.map(|predicate| {
            Box::new(
                move |any: &dyn Any, hitbox: &Hitbox, window: &mut Window, cx: &mut App| {
                    let Some(value_ref) = any.downcast_ref::<T>() else {
                        return false;
                    };
                    (&predicate)(value_ref, hitbox, window, cx)
                },
            ) as CanDropListenerFn<dyn Any>
        });
        let on_drag_over = on_drag_over.map(|predicate| {
            Box::new(move |any: &dyn Any, window: &mut Window, cx: &mut App| {
                let value_ref = any
                    .downcast_ref::<T>()
                    .expect("drop listener type was incorrect");
                (&predicate)(value_ref, window, cx)
            }) as OnDragOverFn<dyn Any>
        });
        let on_drop = on_drop.map(|predicate| {
            Box::new(
                move |any: &dyn Any, evt: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                    let value_ref = any
                        .downcast_ref::<T>()
                        .expect("drop listener type was incorrect");
                    (&predicate)(value_ref, evt, window, cx)
                },
            ) as OnDropFn<dyn Any>
        });
        DropListener::<dyn Any> {
            can_drop,
            on_drag_over,
            on_drop,
        }
    }
}

type CanDropPredicate = Box<dyn Fn(&dyn Any, &Hitbox, &mut Window, &mut App) -> bool + 'static>;

pub(crate) struct TooltipBuilder {
    build: Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>,
    hoverable: bool,
}

pub(crate) type KeyDownListener =
    Box<dyn Fn(&KeyDownEvent, DispatchPhase, &mut Window, &mut App) + 'static>;

pub(crate) type KeyUpListener =
    Box<dyn Fn(&KeyUpEvent, DispatchPhase, &mut Window, &mut App) + 'static>;

pub(crate) type ModifiersChangedListener =
    Box<dyn Fn(&ModifiersChangedEvent, &mut Window, &mut App) + 'static>;

pub(crate) type ActionListener =
    Box<dyn Fn(&dyn Any, DispatchPhase, &mut Window, &mut App) + 'static>;

/// Construct a new [`Div`] element
#[track_caller]
pub fn div() -> Div {
    Div {
        interactivity: Interactivity::new(),
        children: SmallVec::default(),
        prepaint_listener: None,
        image_cache: None,
        prepaint_order_fn: None,
    }
}

/// A [`Div`] element, the all-in-one element for building complex UIs in GPUI
pub struct Div {
    interactivity: Interactivity,
    children: SmallVec<[StackSafe<AnyElement>; 2]>,
    prepaint_listener: Option<Box<dyn Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) + 'static>>,
    image_cache: Option<Box<dyn ImageCacheProvider>>,
    prepaint_order_fn: Option<Box<dyn Fn(&mut Window, &mut App) -> SmallVec<[usize; 8]>>>,
}

impl Div {
    /// Add a listener to be called when the children of this `Div` are prepainted.
    /// This allows you to store the [`Bounds`] of the children for later use.
    pub fn on_children_prepainted(
        mut self,
        listener: impl Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.prepaint_listener = Some(Box::new(listener));
        self
    }

    /// Add an image cache at the location of this div in the element tree.
    pub fn image_cache(mut self, cache: impl ImageCacheProvider) -> Self {
        self.image_cache = Some(Box::new(cache));
        self
    }

    /// Specify a function that determines the order in which children are prepainted.
    ///
    /// The function is called at prepaint time and should return a vector of child indices
    /// in the desired prepaint order. Each index should appear exactly once.
    ///
    /// This is useful when the prepaint of one child affects state that another child reads.
    /// For example, in split editor views, the editor with an autoscroll request should
    /// be prepainted first so its scroll position update is visible to the other editor.
    pub fn with_dynamic_prepaint_order(
        mut self,
        order_fn: impl Fn(&mut Window, &mut App) -> SmallVec<[usize; 8]> + 'static,
    ) -> Self {
        self.prepaint_order_fn = Some(Box::new(order_fn));
        self
    }
}

/// A frame state for a `Div` element, which contains layout IDs for its children.
///
/// This struct is used internally by the `Div` element to manage the layout state of its children
/// during the UI update cycle. It holds a small vector of `LayoutId` values, each corresponding to
/// a child element of the `Div`. These IDs are used to query the layout engine for the computed
/// bounds of the children after the layout phase is complete.
pub struct DivFrameState {
    child_layout_ids: SmallVec<[LayoutId; 2]>,
    inline: Option<InlineDivFrameState>,
    contents_in_parent_paragraph: bool,
}

impl DivFrameState {
    fn standalone_inline(&self) -> Option<&InlineDivFrameState> {
        if self.contents_in_parent_paragraph {
            None
        } else {
            self.inline.as_ref()
        }
    }

    fn standalone_inline_mut(&mut self) -> Option<&mut InlineDivFrameState> {
        if self.contents_in_parent_paragraph {
            None
        } else {
            self.inline.as_mut()
        }
    }

    /// Returns the display text and layout used to paint each inline paragraph.
    #[cfg(any(test, feature = "test-support"))]
    pub fn measured_inline_paragraphs(&self) -> Vec<(SharedString, Arc<crate::InlineLayout>)> {
        self.inline
            .as_ref()
            .map_or_else(Vec::new, |inline| inline.measured_paragraphs())
    }
}

/// Interactivity state displayed an manipulated in the inspector.
#[derive(Clone)]
pub struct DivInspectorState {
    /// The inspected element's base style. This is used for both inspecting and modifying the
    /// state. In the future it will make sense to separate the read and write, possibly tracking
    /// the modifications.
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub base_style: Box<StyleRefinement>,
    /// Inspects the bounds of the element.
    pub bounds: Bounds<Pixels>,
    /// Size of the children of the element, or `bounds.size` if it has no children.
    pub content_size: Size<Pixels>,
}

impl Styled for Div {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.interactivity.base_style
    }
}

impl InteractiveElement for Div {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

impl ParentElement for Div {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        #[cfg(feature = "stacker")]
        self.children
            .extend(elements.into_iter().map(StackSafe::new));
        #[cfg(not(feature = "stacker"))]
        self.children.extend(elements);
    }
}

impl Element for Div {
    type RequestLayoutState = DivFrameState;
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        self.interactivity.element_id.clone()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.interactivity.source_location()
    }

    fn a11y_role(&self) -> Option<accesskit::Role> {
        // Nodes with `GenericContainer` should never be reported to accesskit.
        // Equivalent to an HTML div with no role.
        self.interactivity
            .override_role
            .filter(|role| *role != accesskit::Role::GenericContainer)
    }

    fn is_a11y_hidden(&self) -> bool {
        self.interactivity.aria.hidden
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.interactivity.write_a11y_info(node);
    }

    fn a11y_synthetic_children(
        &mut self,
        _prepaint: &mut Self::PrepaintState,
        builder: &mut crate::A11ySubtreeBuilder,
    ) {
        if let Some(f) = self.interactivity.a11y_synthetic_children.take() {
            f(builder);
        }
    }

    #[cfg_attr(feature = "stacker", stacksafe::stacksafe)]
    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child_layout_ids = SmallVec::new();
        let mut inline = None;
        let image_cache = self
            .image_cache
            .as_mut()
            .map(|provider| provider.provide(window, cx));

        let layout_id = window.with_image_cache(image_cache, |window| {
            self.interactivity.request_layout(
                global_id,
                inspector_id,
                window,
                cx,
                |style, window, cx| {
                    window.with_text_style(style.text_style().cloned(), |window| {
                        let display = style.display;
                        let collected_by_ancestor =
                            window.collecting_inline && display == Display::Inline;
                        let previous_collecting_inline = std::mem::replace(
                            &mut window.collecting_inline,
                            matches!(display, Display::Block | Display::Inline),
                        );
                        child_layout_ids = self
                            .children
                            .iter_mut()
                            .map(|child| child.request_layout(window, cx))
                            .collect();
                        window.collecting_inline = previous_collecting_inline;

                        let layout_id = if matches!(display, Display::Block | Display::Inline)
                            && !collected_by_ancestor
                        {
                            let (node_id, state) = InlineDivFrameState::request_layout(
                                &style,
                                &child_layout_ids,
                                window,
                                cx,
                            );

                            inline = Some(state);
                            node_id
                        } else if collected_by_ancestor {
                            window.request_layout(style, std::iter::empty(), cx)
                        } else {
                            window.request_layout(style, child_layout_ids.iter().copied(), cx)
                        };

                        if display == Display::Inline {
                            window.publish_inline_content(
                                layout_id,
                                InlineContent::Container {
                                    children: child_layout_ids.clone(),
                                },
                            );
                        }

                        window.set_layout_logical_children(layout_id, &child_layout_ids);

                        layout_id
                    })
                },
            )
        });

        (
            layout_id,
            DivFrameState {
                child_layout_ids,
                inline,
                contents_in_parent_paragraph: false,
            },
        )
    }

    #[cfg_attr(feature = "stacker", stacksafe::stacksafe)]
    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Hitbox> {
        let image_cache = self
            .image_cache
            .as_mut()
            .map(|provider| provider.provide(window, cx));

        let has_prepaint_listener = self.prepaint_listener.is_some();
        let mut children_bounds = Vec::with_capacity(if has_prepaint_listener {
            request_layout.child_layout_ids.len()
        } else {
            0
        });

        let mut child_min = point(Pixels::MAX, Pixels::MAX);
        let mut child_max = Point::default();
        if let Some(handle) = self.interactivity.scroll_anchor.as_ref() {
            *handle.last_origin.borrow_mut() = bounds.origin - window.element_offset();
        }

        request_layout.contents_in_parent_paragraph = window.current_inline_fragments.is_some();

        let content_size = if let Some(inline) = request_layout.standalone_inline() {
            inline.prepare_layout(
                bounds,
                self.interactivity.tracked_scroll_handle.as_ref(),
                &request_layout.child_layout_ids,
                window,
            )
        } else if request_layout.child_layout_ids.is_empty() {
            bounds.size
        } else if let Some(scroll_handle) = self.interactivity.tracked_scroll_handle.as_ref() {
            let mut state = scroll_handle.0.borrow_mut();
            state.child_bounds = Vec::with_capacity(request_layout.child_layout_ids.len());
            for child_layout_id in &request_layout.child_layout_ids {
                let child_bounds = window.layout_bounds(*child_layout_id);
                child_min = child_min.min(&child_bounds.origin);
                child_max = child_max.max(&child_bounds.bottom_right());
                state.child_bounds.push(child_bounds);
            }
            (child_max - child_min).into()
        } else {
            for child_layout_id in &request_layout.child_layout_ids {
                let child_bounds = window.layout_bounds(*child_layout_id);
                child_min = child_min.min(&child_bounds.origin);
                child_max = child_max.max(&child_bounds.bottom_right());

                if has_prepaint_listener {
                    children_bounds.push(child_bounds);
                }
            }
            (child_max - child_min).into()
        };

        if let Some(scroll_handle) = self.interactivity.tracked_scroll_handle.as_ref() {
            scroll_handle.scroll_to_active_item();
        }

        self.interactivity.prepaint(
            global_id,
            inspector_id,
            bounds,
            content_size,
            window,
            cx,
            |style, scroll_offset, hitbox, window, cx| {
                // skip children
                if style.display == Display::None {
                    return hitbox;
                }

                #[inline]
                fn prepaint_children(
                    children: &mut [StackSafe<AnyElement>],
                    order: Option<&[usize]>,
                    window: &mut Window,
                    cx: &mut App,
                ) {
                    let order = match order {
                        Some(order) => Either::Left(order.iter().copied()),
                        None => Either::Right(0..children.len()),
                    };

                    for index in order {
                        if let Some(child) = children.get_mut(index) {
                            child.prepaint(window, cx);
                        }
                    }
                }

                window.with_image_cache(image_cache, |window| {
                    window.with_style_transition_containing_bounds(bounds, |window| {
                        let has_standalone_inline = request_layout.standalone_inline().is_some();
                        let inline_order = if has_standalone_inline {
                            self.prepaint_order_fn
                                .as_ref()
                                .map(|order_fn| order_fn(window, cx))
                        } else {
                            None
                        };

                        window.with_element_offset(scroll_offset, |window| {
                            if let Some(inline) = request_layout.standalone_inline_mut() {
                                inline.record_paragraph_origins(window);
                                prepaint_children(
                                    &mut self.children,
                                    inline_order.as_deref(),
                                    window,
                                    cx,
                                );
                            } else {
                                let order = self
                                    .prepaint_order_fn
                                    .as_ref()
                                    .map(|order_fn| order_fn(window, cx));
                                prepaint_children(&mut self.children, order.as_deref(), window, cx);
                            }
                        });

                        if has_prepaint_listener && has_standalone_inline {
                            children_bounds.extend(
                                request_layout
                                    .child_layout_ids
                                    .iter()
                                    .map(|layout_id| window.layout_bounds(*layout_id)),
                            );
                        }

                        if let Some(listener) = self.prepaint_listener.as_ref() {
                            listener(children_bounds, window, cx);
                        }
                    });
                });

                hitbox
            },
        )
    }

    #[cfg_attr(feature = "stacker", stacksafe::stacksafe)]
    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Option<Hitbox>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let image_cache = self
            .image_cache
            .as_mut()
            .map(|provider| provider.provide(window, cx));

        window.with_image_cache(image_cache, |window| {
            self.interactivity.paint(
                global_id,
                inspector_id,
                bounds,
                hitbox.as_ref(),
                window,
                cx,
                |style, window, cx| {
                    // skip children
                    if style.display == Display::None {
                        return;
                    }

                    for child in &mut self.children {
                        child.paint(window, cx);
                    }

                    if let Some(inline) = request_layout.standalone_inline() {
                        inline.paint_paragraphs(window, cx);
                    }
                },
            )
        });
    }
}

impl IntoElement for Div {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Default)]
pub(crate) struct AriaProperties {
    invalid: Option<accesskit::Invalid>,
    pub(crate) author_id: Option<SharedString>,
    pub(crate) label: Option<SharedString>,
    pub(crate) description: Option<SharedString>,
    pub(crate) keyshortcuts: Option<SharedString>,
    pub(crate) hidden: bool,
    pub(crate) selected: Option<bool>,
    pub(crate) expanded: Option<bool>,
    pub(crate) toggled: Option<accesskit::Toggled>,
    pub(crate) disabled: Option<bool>,
    pub(crate) numeric_value: Option<f64>,
    pub(crate) min_numeric_value: Option<f64>,
    pub(crate) max_numeric_value: Option<f64>,
    pub(crate) numeric_value_step: Option<f64>,
    pub(crate) value: Option<SharedString>,
    pub(crate) placeholder: Option<SharedString>,
    pub(crate) orientation: Option<accesskit::Orientation>,
    pub(crate) level: Option<usize>,
    pub(crate) position_in_set: Option<usize>,
    pub(crate) size_of_set: Option<usize>,
    pub(crate) row_index: Option<usize>,
    pub(crate) column_index: Option<usize>,
    pub(crate) row_count: Option<usize>,
    pub(crate) column_count: Option<usize>,
    pub(crate) modal: Option<bool>,
    pub(crate) live: Option<accesskit::Live>,
    pub(crate) live_atomic: Option<bool>,
}

/// The interactivity struct. Powers all of the general-purpose
/// interactivity in the `Div` element.
#[derive(Default)]
pub struct Interactivity {
    /// The element ID of the element. In id is required to support a stateful subset of the interactivity such as on_click.
    pub element_id: Option<ElementId>,
    /// Whether the element was clicked. This will only be present after layout.
    pub active: Option<bool>,
    /// Whether the element was hovered. This will only be present after paint if an hitbox
    /// was created for the interactive element.
    pub hovered: Option<bool>,
    pub(crate) tooltip_id: Option<TooltipId>,
    pub(crate) content_size: Size<Pixels>,
    pub(crate) key_context: Option<KeyContext>,
    pub(crate) focusable: bool,
    pub(crate) disabled: bool,
    pub(crate) tracked_focus_handle: Option<FocusHandle>,
    pub(crate) tracked_scroll_handle: Option<ScrollHandle>,
    pub(crate) scroll_anchor: Option<ScrollAnchor>,
    pub(crate) scroll_offset: Option<Rc<RefCell<Point<Pixels>>>>,
    pub(crate) ongoing_scroll: Option<Rc<RefCell<OngoingScroll>>>,
    pub(crate) group: Option<SharedString>,
    /// The base style of the element, before any modifications are applied
    /// by focus, active, etc.
    pub base_style: Box<StyleRefinement>,
    pub(crate) style_transitions: Option<Box<StyleTransitions>>,
    pub(crate) disabled_style: Option<Box<StyleRefinement>>,
    pub(crate) focus_style: Option<Box<StyleRefinement>>,
    pub(crate) in_focus_style: Option<Box<StyleRefinement>>,
    pub(crate) contains_focus_style: Option<Box<StyleRefinement>>,
    pub(crate) focus_visible_style: Option<Box<StyleRefinement>>,
    pub(crate) hover_style: Option<Box<StyleRefinement>>,
    pub(crate) group_hover_style: Option<GroupStyle>,
    pub(crate) active_style: Option<Box<StyleRefinement>>,
    pub(crate) group_active_style: Option<GroupStyle>,
    pub(crate) drag_over_style:
        Option<Box<dyn Fn(&dyn Any, &mut Window, &mut App) -> StyleRefinement>>,
    pub(crate) group_drag_over_styles: Vec<(TypeId, GroupStyle)>,
    pub(crate) prepaint_listeners: Vec<PrepaintListener>,
    pub(crate) mouse_down_listeners: Vec<MouseDownListener>,
    pub(crate) mouse_up_listeners: Vec<MouseUpListener>,
    pub(crate) mouse_pressure_listeners: Vec<MousePressureListener>,
    pub(crate) mouse_move_listeners: Vec<MouseMoveListener>,
    pub(crate) mouse_exit_listeners: Vec<MouseExitListener>,
    pub(crate) scroll_wheel_listeners: Vec<ScrollWheelListener>,
    pub(crate) pinch_listeners: Vec<PinchListener>,
    pub(crate) key_down_listeners: Vec<KeyDownListener>,
    pub(crate) key_up_listeners: Vec<KeyUpListener>,
    pub(crate) modifiers_changed_listeners: Vec<ModifiersChangedListener>,
    pub(crate) action_listeners: Vec<(TypeId, ActionListener)>,
    pub(crate) drop_listeners: Vec<(TypeId, DropListener<dyn Any>)>,
    pub(crate) can_drop_predicate: Option<CanDropPredicate>,
    pub(crate) click_listeners: Vec<ClickListener>,
    pub(crate) aux_click_listeners: Vec<ClickListener>,
    pub(crate) drag_listener: Option<DragListener>,
    pub(crate) hover_region_listeners: Vec<Box<dyn HoverRegionListener>>,
    /// Resolves a cursor from the current pointer position and this element's bounds.
    pub(crate) cursor_style_resolver: Option<CursorStyleResolver>,
    pub(crate) tooltip_builder: Option<TooltipBuilder>,
    pub(crate) tooltip_show_delay: Option<Duration>,
    pub(crate) window_control: Option<WindowControlArea>,
    pub(crate) hitbox_behavior: HitboxBehavior,
    pub(crate) tab_index: Option<isize>,
    pub(crate) tab_group: bool,
    pub(crate) tab_stop: bool,

    pub(crate) a11y_action_listeners:
        Vec<(accesskit::Action, crate::window::a11y::A11yActionListener)>,
    pub(crate) a11y_synthetic_children: Option<Box<dyn FnOnce(&mut crate::A11ySubtreeBuilder)>>,
    pub(crate) report_active_descendant_focus: bool,
    pub(crate) override_role: Option<accesskit::Role>,
    pub(crate) aria: AriaProperties,

    #[cfg(any(feature = "inspector", debug_assertions))]
    pub(crate) source_location: Option<&'static core::panic::Location<'static>>,

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) debug_selector: Option<String>,
}

impl Interactivity {
    /// Assigns the current scroll offset of an element. No-op if the element's
    /// style does not have overflow enabled.
    ///
    /// Should only be called during the `request_layout` phase.
    pub fn set_scroll_offset(
        &self,
        global_id: Option<&GlobalElementId>,
        window: &mut Window,
        point: Point<Pixels>,
    ) {
        window.with_optional_element_state::<gpui::InteractiveElementState, _>(
            global_id,
            |element_state, _window| {
                let mut element_state =
                    element_state.map(|element_state| element_state.unwrap_or_default());
                let overflow = &self.base_style.overflow;
                if (overflow.x == Some(Overflow::Scroll) || overflow.y == Some(Overflow::Scroll))
                    && let Some(element_state) = element_state.as_mut()
                {
                    let scroll_offset = element_state.scroll_offset.get_or_insert_with(Rc::default);
                    *scroll_offset.borrow_mut() = point;
                }
                ((), element_state)
            },
        );
    }
}

impl Interactivity {
    /// Layout this element according to this interactivity state's configured styles
    pub fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(Style, &mut Window, &mut App) -> LayoutId,
    ) -> LayoutId {
        #[cfg(any(feature = "inspector", debug_assertions))]
        window.with_inspector_state(
            _inspector_id,
            cx,
            |inspector_state: &mut Option<DivInspectorState>, _window| {
                if let Some(inspector_state) = inspector_state {
                    self.base_style = inspector_state.base_style.clone();
                } else {
                    *inspector_state = Some(DivInspectorState {
                        base_style: self.base_style.clone(),
                        bounds: Default::default(),
                        content_size: Default::default(),
                    })
                }
            },
        );

        window.with_optional_element_state::<InteractiveElementState, _>(
            global_id,
            |element_state, window| {
                let mut element_state =
                    element_state.map(|element_state| element_state.unwrap_or_default());

                if let Some(element_state) = element_state.as_ref()
                    && cx.has_active_drag()
                {
                    if let Some(pending_mouse_down) = element_state.pending_mouse_down.as_ref() {
                        *pending_mouse_down.borrow_mut() = None;
                    }
                    if let Some(clicked_state) = element_state.clicked_state.as_ref() {
                        *clicked_state.borrow_mut() = ElementClickedState::default();
                    }
                }

                // Ensure we store a focus handle in our element state if we're focusable.
                // If there's an explicit focus handle we're tracking, use that. Otherwise
                // create a new handle and store it in the element state, which lives for as
                // as frames contain an element with this id.
                if self.focusable
                    && self.tracked_focus_handle.is_none()
                    && let Some(element_state) = element_state.as_mut()
                {
                    let mut handle = element_state
                        .focus_handle
                        .get_or_insert_with(|| cx.focus_handle())
                        .clone()
                        .tab_stop(self.tab_stop);

                    if let Some(index) = self.tab_index {
                        handle = handle.tab_index(index);
                    }

                    self.tracked_focus_handle = Some(handle);
                }

                if let Some(scroll_handle) = self.tracked_scroll_handle.as_ref() {
                    let scroll_handle_state = scroll_handle.0.borrow();
                    self.scroll_offset = Some(scroll_handle_state.offset.clone());
                    self.ongoing_scroll = Some(scroll_handle_state.ongoing_scroll.clone());
                } else if (self.base_style.overflow.x == Some(Overflow::Scroll)
                    || self.base_style.overflow.y == Some(Overflow::Scroll))
                    && let Some(element_state) = element_state.as_mut()
                {
                    self.scroll_offset = Some(
                        element_state
                            .scroll_offset
                            .get_or_insert_with(Rc::default)
                            .clone(),
                    );
                    self.ongoing_scroll = Some(
                        element_state
                            .ongoing_scroll
                            .get_or_insert_with(|| Rc::new(RefCell::new(OngoingScroll::default())))
                            .clone(),
                    );
                }

                let style =
                    self.compute_style_internal(None, None, element_state.as_mut(), window, cx);
                let layout_id = f(style, window, cx);
                (layout_id, element_state)
            },
        )
    }

    /// Commit the bounds of this element according to this interactivity state's configured styles.
    pub fn prepaint<R>(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        content_size: Size<Pixels>,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&Style, Point<Pixels>, Option<Hitbox>, &mut Window, &mut App) -> R,
    ) -> R {
        self.content_size = content_size;

        if !self.prepaint_listeners.is_empty() {
            let payload = InteractivityPrepaint {
                bounds,
                content_size,
            };
            for listener in self.prepaint_listeners.drain(..) {
                listener(&payload, window, cx);
            }
        }

        #[cfg(any(feature = "inspector", debug_assertions))]
        window.with_inspector_state(
            _inspector_id,
            cx,
            |inspector_state: &mut Option<DivInspectorState>, _window| {
                if let Some(inspector_state) = inspector_state {
                    inspector_state.bounds = bounds;
                    inspector_state.content_size = content_size;
                }
            },
        );

        if let Some(focus_handle) = self.tracked_focus_handle.as_ref() {
            window.set_focus_handle(focus_handle, cx);

            if window.a11y.is_active() {
                if let Some(global_id) = global_id {
                    let node_id = global_id.accesskit_node_id();
                    window.a11y.set_focusable(node_id, focus_handle.id);
                    if focus_handle.is_focused(window) {
                        window.a11y.set_focus(node_id);
                    }
                } else if focus_handle.is_focused(window) {
                    // Focusable, but with no element id it can't have an
                    // accessibility node, so screen readers fall back to the
                    // whole window.
                    window
                        .a11y
                        .note_focus_without_node(focus_handle.id, "it has no element id");
                }
            }
        }

        if self.report_active_descendant_focus && window.a11y.is_active() {
            if let Some(global_id) = global_id {
                window
                    .a11y
                    .set_active_descendant(global_id.accesskit_node_id());
            }
        }
        window.with_optional_element_state::<InteractiveElementState, _>(
            global_id,
            |element_state, window| {
                let mut element_state =
                    element_state.map(|element_state| element_state.unwrap_or_default());
                let style = self.compute_style_internal(
                    None,
                    Some(bounds),
                    element_state.as_mut(),
                    window,
                    cx,
                );

                if let Some(element_state) = element_state.as_mut() {
                    if self
                        .tracked_focus_handle
                        .as_ref()
                        .is_some_and(|focus| focus.is_focused(window))
                        && window.last_input_was_keyboard()
                        && element_state.revealed_focus_generation != Some(window.focus_generation)
                    {
                        element_state.revealed_focus_generation = Some(window.focus_generation);
                        // Include focus indicators, not just the control's border box.
                        window.request_autoscroll(bounds.dilate(px(4.0)));
                    }
                    if let Some(clicked_state) = element_state.clicked_state.as_ref() {
                        let clicked_state = clicked_state.borrow();
                        self.active = Some(clicked_state.element);
                    }
                    if self.hover_style.is_some() || self.group_hover_style.is_some() {
                        element_state
                            .hover_state
                            .get_or_insert_with(Default::default);
                    }
                    if let Some(active_tooltip) = element_state.active_tooltip.as_ref() {
                        if self.tooltip_builder.is_some() {
                            self.tooltip_id = set_tooltip_on_window(active_tooltip, window);
                        } else {
                            // If there is no longer a tooltip builder, remove the active tooltip.
                            element_state.active_tooltip.take();
                        }
                    }
                }

                window.with_text_style(style.text_style().cloned(), |window| {
                    window.with_content_mask(
                        style.overflow_mask(bounds, window.rem_size()),
                        |window| {
                            let hitbox = if self.should_insert_hitbox(&style, window, cx) {
                                let hitbox = window.insert_hitbox_mut(bounds, self.hitbox_behavior);
                                // attach the group of the div to the hitbox's tags so it can be queried in hit-test related listeners
                                if let Some(group) = &self.group {
                                    hitbox.tags.push(group.clone());
                                }
                                Some(hitbox.clone())
                            } else {
                                None
                            };

                            let scroll_offset =
                                self.clamp_scroll_position(bounds, &style, window, cx);
                            let previous_autoscroll = window.take_autoscroll();
                            let result = f(&style, scroll_offset, hitbox, window, cx);
                            if let Some(mut target) = window.take_autoscroll() {
                                self.reveal_descendant(
                                    &mut target,
                                    bounds,
                                    &style,
                                    scroll_offset,
                                    window,
                                );
                                window.request_autoscroll(target);
                            } else if let Some(target) = previous_autoscroll {
                                window.request_autoscroll(target);
                            }
                            (result, element_state)
                        },
                    )
                })
            },
        )
    }

    /// Bubble a focus reveal through nested scroll containers. Apply offsets
    /// next frame so paint and hit testing use the same layout for this frame.
    fn reveal_descendant(
        &self,
        target: &mut Bounds<Pixels>,
        bounds: Bounds<Pixels>,
        style: &Style,
        scroll_offset: Point<Pixels>,
        window: &mut Window,
    ) {
        let Some(offset) = self.scroll_offset.as_ref() else {
            return;
        };
        let padding = style
            .padding
            .to_pixels(bounds.size.into(), window.rem_size());
        let max = Point::from(
            self.content_size + size(padding.left + padding.right, padding.top + padding.bottom)
                - bounds.size,
        )
        .max(&Point::default());
        let reveal_axis = |value: Pixels,
                           limit: Pixels,
                           start: Pixels,
                           end: Pixels,
                           view_start: Pixels,
                           view_end: Pixels| {
            let delta = if start < view_start {
                view_start - start
            } else if end > view_end {
                view_end - end
            } else {
                px(0.0)
            };
            (value + delta).clamp(-limit, px(0.0))
        };
        let mut next = scroll_offset;
        if style.overflow.x == Overflow::Scroll {
            next.x = reveal_axis(
                next.x,
                max.x,
                target.left(),
                target.right(),
                bounds.left(),
                bounds.right(),
            );
        }
        if style.overflow.y == Overflow::Scroll {
            next.y = reveal_axis(
                next.y,
                max.y,
                target.top(),
                target.bottom(),
                bounds.top(),
                bounds.bottom(),
            );
        }
        if next != scroll_offset {
            target.origin += next - scroll_offset;
            let offset = offset.clone();
            window.on_next_frame(move |window, _| {
                *offset.borrow_mut() = next;
                window.refresh();
            });
        }
    }

    fn should_insert_hitbox(&self, style: &Style, window: &Window, cx: &App) -> bool {
        self.hitbox_behavior != HitboxBehavior::Normal
            || self.window_control.is_some()
            || style.mouse_cursor.is_some()
            || self.cursor_style_resolver.is_some()
            || self.group.is_some()
            || self.scroll_offset.is_some()
            || self.tracked_focus_handle.is_some()
            || self.hover_style.is_some()
            || self.group_hover_style.is_some()
            || !self.hover_region_listeners.is_empty()
            || !self.mouse_up_listeners.is_empty()
            || !self.mouse_pressure_listeners.is_empty()
            || !self.mouse_down_listeners.is_empty()
            || !self.mouse_move_listeners.is_empty()
            || !self.mouse_exit_listeners.is_empty()
            || !self.click_listeners.is_empty()
            || !self.aux_click_listeners.is_empty()
            || !self.scroll_wheel_listeners.is_empty()
            || self.has_pinch_listeners()
            || self.drag_listener.is_some()
            || !self.drop_listeners.is_empty()
            || self.drag_over_style.is_some()
            || self.tooltip_builder.is_some()
            || window.is_inspector_picking(cx)
    }

    fn clamp_scroll_position(
        &self,
        bounds: Bounds<Pixels>,
        style: &Style,
        window: &mut Window,
        _cx: &mut App,
    ) -> Point<Pixels> {
        fn round_to_two_decimals(pixels: Pixels) -> Pixels {
            const ROUNDING_FACTOR: f32 = 100.0;
            (pixels * ROUNDING_FACTOR).round() / ROUNDING_FACTOR
        }

        if let Some(scroll_offset) = self.scroll_offset.as_ref() {
            let mut scroll_to_bottom = false;
            let mut tracked_scroll_handle = self
                .tracked_scroll_handle
                .as_ref()
                .map(|handle| handle.0.borrow_mut());
            if let Some(mut scroll_handle_state) = tracked_scroll_handle.as_deref_mut() {
                scroll_handle_state.overflow = style.overflow;
                scroll_to_bottom = mem::take(&mut scroll_handle_state.scroll_to_bottom);
            }

            let rem_size = window.rem_size();
            // Taffy lays the box out with the padding snapped to the device pixel
            // grid (`to_taffy`); recomputed unsnapped, e.g. py_1 at a fractional
            // rem size, it exceeds `bounds` and leaves the box scrollable by the
            // sub-pixel difference.
            let padding = style
                .padding
                .to_pixels(bounds.size.into(), rem_size)
                .map(|edge| window.pixel_snap(*edge));
            let padding_size = size(padding.left + padding.right, padding.top + padding.bottom);
            // The floating point values produced by Taffy and ours often vary
            // slightly after ~5 decimal places. This can lead to cases where after
            // subtracting these, the container becomes scrollable for less than
            // 0.00000x pixels. As we generally don't benefit from a precision that
            // high for the maximum scroll, we round the scroll max to 2 decimal
            // places here.
            let padded_content_size = self.content_size + padding_size;
            let scroll_max = Point::from(padded_content_size - bounds.size)
                .map(round_to_two_decimals)
                .max(&Default::default());
            // Clamp scroll offset in case scroll max is smaller now (e.g., if children
            // were removed or the bounds became larger).
            let mut scroll_offset = scroll_offset.borrow_mut();

            scroll_offset.x = scroll_offset.x.clamp(-scroll_max.x, px(0.));
            if scroll_to_bottom {
                scroll_offset.y = -scroll_max.y;
            } else {
                scroll_offset.y = scroll_offset.y.clamp(-scroll_max.y, px(0.));
            }

            if let Some(mut scroll_handle_state) = tracked_scroll_handle {
                scroll_handle_state.max_offset = scroll_max;
                scroll_handle_state.bounds = bounds;
            }

            *scroll_offset
        } else {
            Point::default()
        }
    }

    /// Paint this element according to this interactivity state's configured styles
    /// and bind the element's mouse and keyboard events.
    ///
    /// content_size is the size of the content of the element, which may be larger than the
    /// element's bounds if the element is scrollable.
    ///
    /// the final computed style will be passed to the provided function, along
    /// with the current scroll offset
    pub fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        hitbox: Option<&Hitbox>,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&Style, &mut Window, &mut App),
    ) {
        self.hovered = hitbox.map(|hitbox| hitbox.is_hovered(window));
        window.with_optional_element_state::<InteractiveElementState, _>(
            global_id,
            |element_state, window| {
                let mut element_state =
                    element_state.map(|element_state| element_state.unwrap_or_default());

                let style = self.compute_style_internal(
                    hitbox,
                    Some(bounds),
                    element_state.as_mut(),
                    window,
                    cx,
                );

                if let Some(state) = element_state.as_mut() {
                    state
                        .hover_region_states
                        .truncate(self.hover_region_listeners.len());
                }

                #[cfg(any(feature = "test-support", test))]
                if let Some(debug_selector) = &self.debug_selector {
                    window
                        .next_frame
                        .debug_bounds
                        .insert(debug_selector.clone(), bounds);
                }

                self.paint_hover_group_handler(window, cx);

                if style.visibility == Visibility::Hidden {
                    return ((), element_state);
                }

                let tab_group = self.tab_group.then_some(self.tab_index).flatten();

                let paint_content = |window: &mut Window, cx: &mut App| {
                    // Register the container inside its tab group so `focus_next` enters
                    // its own children. Outside the group, it would sort ahead of the
                    // items in every sibling group and enter the first group instead.
                    if let Some(focus_handle) = &self.tracked_focus_handle {
                        window.next_frame.tab_stops.insert(focus_handle);
                    }

                    if let Some(hitbox) = hitbox {
                        #[cfg(debug_assertions)]
                        self.paint_debug_info(global_id, hitbox, &style, window, cx);

                        if let Some(drag) = cx.active_drag.as_ref() {
                            if let Some(mouse_cursor) = drag.cursor_style {
                                window.set_window_cursor_style(mouse_cursor);
                            }
                        } else {
                            if let Some(mouse_cursor) = style.mouse_cursor {
                                window.set_cursor_style(mouse_cursor, hitbox);
                            }

                            if let Some(resolve_cursor) = self.cursor_style_resolver.clone() {
                                window.set_cursor_style_with(hitbox, move |position| {
                                    resolve_cursor(position, bounds)
                                });
                            }
                        }

                        if let Some(group) = self.group.clone() {
                            GroupHitboxes::push(group, hitbox.id, cx);
                        }

                        if let Some(area) = self.window_control {
                            window.insert_window_control_hitbox(area, hitbox.clone());
                        }

                        self.paint_mouse_listeners(hitbox, element_state.as_mut(), window, cx);
                        self.paint_scroll_listener(hitbox, &style, window, cx);
                    }

                    self.paint_keyboard_listeners(window, cx);

                    if window.a11y.is_active()
                        && let Some(global_id) = global_id
                        && !self.a11y_action_listeners.is_empty()
                    {
                        let node_id = global_id.accesskit_node_id();

                        for (action, listener) in self.a11y_action_listeners.drain(..) {
                            if self.disabled {
                                continue;
                            }

                            window.on_a11y_action(node_id, action, listener);
                        }
                    }

                    f(&style, window, cx);

                    if let Some(_hitbox) = hitbox {
                        #[cfg(any(feature = "inspector", debug_assertions))]
                        window.insert_inspector_hitbox(_hitbox.id, _inspector_id, cx);

                        if let Some(group) = self.group.as_ref() {
                            GroupHitboxes::pop(group, cx);
                        }
                    }
                };

                let paint_tab_group = |window: &mut Window, cx: &mut App| {
                    window.with_tab_group(tab_group, |window| paint_content(window, cx));
                };

                let paint_content_mask = |window: &mut Window, cx: &mut App| {
                    window.with_content_mask(
                        style.overflow_mask(bounds, window.rem_size()),
                        |window| paint_tab_group(window, cx),
                    );
                };

                let paint_text_style = |window: &mut Window, cx: &mut App| {
                    window.with_text_style(style.text_style().cloned(), |window| {
                        paint_content_mask(window, cx);
                    });
                };

                style.paint(bounds, window, cx, paint_text_style);

                ((), element_state)
            },
        );
    }

    #[cfg(debug_assertions)]
    fn paint_debug_info(
        &self,
        global_id: Option<&GlobalElementId>,
        hitbox: &Hitbox,
        style: &Style,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(global_id) = global_id
            && (style.debug || style.debug_below || cx.has_global::<crate::DebugBelow>())
            && hitbox.is_hovered(window)
        {
            const FONT_SIZE: crate::Pixels = crate::Pixels(10.);
            let element_id = format!("{global_id:?}");
            let str_len = element_id.len();

            let render_debug_text = |window: &mut Window| {
                if let Some(text) = window
                    .text_system()
                    .shape_text(
                        &element_id,
                        FONT_SIZE,
                        &[window.text_style().to_run(str_len)],
                        None,
                        None,
                    )
                    .ok()
                {
                    text.paint(hitbox.origin, FONT_SIZE, TextAlign::Left, None, window, cx)
                        .ok();

                    let text_bounds = crate::Bounds {
                        origin: hitbox.origin,
                        size: text.size(FONT_SIZE),
                    };
                    if let Some(source_location) = self.source_location
                        && text_bounds.contains(&window.mouse_position())
                        && window.modifiers().secondary()
                    {
                        let secondary_held = window.modifiers().secondary();
                        window.on_key_event({
                            move |e: &crate::ModifiersChangedEvent, _phase, window, _cx| {
                                if e.modifiers.secondary() != secondary_held
                                    && text_bounds.contains(&window.mouse_position())
                                {
                                    window.refresh();
                                }
                            }
                        });

                        let was_hovered = hitbox.is_hovered(window);
                        let current_view = window.current_view();
                        window.on_mouse_event({
                            let hitbox = hitbox.clone();
                            move |_: &MouseMoveEvent, phase, window, cx| {
                                if phase == DispatchPhase::Capture {
                                    let hovered = hitbox.is_hovered(window);
                                    if hovered != was_hovered {
                                        cx.notify(current_view)
                                    }
                                }
                            }
                        });

                        window.on_mouse_event({
                            let hitbox = hitbox.clone();
                            move |e: &crate::MouseDownEvent, phase, window, cx| {
                                if text_bounds.contains(&e.position)
                                    && phase.capture()
                                    && hitbox.is_hovered(window)
                                {
                                    cx.stop_propagation();
                                    let Ok(dir) = std::env::current_dir() else {
                                        return;
                                    };

                                    eprintln!(
                                        "This element was created at:\n{}:{}:{}",
                                        dir.join(source_location.file()).to_string_lossy(),
                                        source_location.line(),
                                        source_location.column()
                                    );
                                }
                            }
                        });
                        window.paint_quad(
                            PaintQuad::new(crate::Bounds {
                                origin: hitbox.origin
                                    + crate::point(crate::px(0.), FONT_SIZE - px(2.)),
                                size: crate::Size {
                                    width: text_bounds.size.width,
                                    height: crate::px(1.),
                                },
                            })
                            .border_widths(px(1.))
                            .border_color(crate::red()),
                        )
                    }
                }
            };

            window.with_text_style(
                Some(crate::TextStyleRefinement {
                    color: Some(crate::red()),
                    line_height: Some(FONT_SIZE.into()),
                    background_color: Some(crate::white()),
                    ..Default::default()
                }),
                render_debug_text,
            )
        }
    }

    fn has_drag_over_styles(&self) -> bool {
        self.drag_over_style.is_some() || {
            // whether there is a drop listener which affects styling
            self.drop_listeners
                .iter()
                .find(|(_, listener)| listener.on_drag_over.is_some())
                .is_some()
        }
    }

    fn paint_mouse_listeners(
        &mut self,
        hitbox: &Hitbox,
        element_state: Option<&mut InteractiveElementState>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let disabled = self.disabled;

        let is_focused = self
            .tracked_focus_handle
            .as_ref()
            .map(|handle| handle.is_focused(window))
            .unwrap_or(false);

        // If this element can be focused, register a mouse down listener
        // that will automatically transfer focus when hitting the element.
        // This behavior can be suppressed by using `cx.prevent_default()`.
        if let Some(focus_handle) = self.tracked_focus_handle.clone() {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |_: &MouseDownEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && hitbox.is_hovered(window)
                    && !window.default_prevented()
                {
                    window.focus(&focus_handle, cx);
                    // If there is a parent that is also focusable, prevent it
                    // from transferring focus because we already did so.
                    window.prevent_default();
                }
            });
        }

        for listener in self.mouse_down_listeners.drain(..) {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if disabled {
                    return;
                }

                listener(event, phase, &hitbox, window, cx);
            })
        }

        for listener in self.mouse_up_listeners.drain(..) {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if disabled {
                    return;
                }

                listener(event, phase, &hitbox, window, cx);
            })
        }

        for listener in self.mouse_pressure_listeners.drain(..) {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |event: &MousePressureEvent, phase, window, cx| {
                if disabled {
                    return;
                }

                listener(event, phase, &hitbox, window, cx);
            })
        }

        for listener in self.mouse_move_listeners.drain(..) {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if disabled {
                    return;
                }

                listener(event, phase, &hitbox, window, cx);
            })
        }

        for listener in self.mouse_exit_listeners.drain(..) {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |event: &MouseExitEvent, phase, window, cx| {
                if disabled {
                    return;
                }

                listener(event, phase, &hitbox, window, cx);
            })
        }

        for listener in self.scroll_wheel_listeners.drain(..) {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                if disabled {
                    return;
                }

                listener(event, phase, &hitbox, window, cx);
            })
        }

        for listener in self.pinch_listeners.drain(..) {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |event: &PinchEvent, phase, window, cx| {
                if disabled {
                    return;
                }

                listener(event, phase, &hitbox, window, cx);
            })
        }

        if self.hover_style.is_some()
            || self.base_style.mouse_cursor.is_some()
            || cx.active_drag.is_some() && self.has_drag_over_styles()
        {
            let hitbox = hitbox.clone();
            let hover_state = self.hover_style.as_ref().and_then(|_| {
                element_state
                    .as_ref()
                    .and_then(|state| state.hover_state.as_ref())
                    .cloned()
            });
            let current_view = window.current_view();

            window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, cx| {
                let hovered = hitbox.is_hovered(window);
                let was_hovered = hover_state
                    .as_ref()
                    .is_some_and(|state| state.borrow().element);
                if phase == DispatchPhase::Capture && hovered != was_hovered {
                    if let Some(hover_state) = &hover_state {
                        hover_state.borrow_mut().element = hovered;
                        cx.notify(current_view);
                    }
                }
            });
        }

        if let Some(group_hover) = self.group_hover_style.as_ref() {
            if let Some(group_hitbox_id) = GroupHitboxes::get(&group_hover.group, cx) {
                let hover_state = element_state
                    .as_ref()
                    .and_then(|element| element.hover_state.as_ref())
                    .cloned();
                let current_view = window.current_view();

                window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, cx| {
                    let group_hovered = group_hitbox_id.is_hovered(window);
                    let was_group_hovered = hover_state
                        .as_ref()
                        .is_some_and(|state| state.borrow().group);
                    if phase == DispatchPhase::Capture && group_hovered != was_group_hovered {
                        if let Some(hover_state) = &hover_state {
                            hover_state.borrow_mut().group = group_hovered;
                            cx.notify(current_view);
                        }
                    }
                });
            }
        }

        let drag_cursor_style = self.base_style.as_ref().mouse_cursor;

        let mut drag_listener = mem::take(&mut self.drag_listener);
        let drop_listeners = mem::take(&mut self.drop_listeners);
        let click_listeners = mem::take(&mut self.click_listeners);
        let aux_click_listeners = mem::take(&mut self.aux_click_listeners);
        let can_drop_predicate = mem::take(&mut self.can_drop_predicate);

        if !drop_listeners.is_empty() {
            let hitbox = hitbox.clone();
            window.on_mouse_event({
                move |evt: &MouseUpEvent, phase, window, cx| {
                    if disabled {
                        return;
                    }

                    if !phase.bubble() {
                        return;
                    }

                    let drag_value = match cx.active_drag() {
                        None => return,
                        Some(drag) => drag.value.clone(),
                    };

                    // Testing whether the element will be able to receive, regardless of if its hitbox is hovered.
                    // Elements that override this can check if the hitbox is hovered, manually (or via interactivity api).
                    if let Some(predicate) = &can_drop_predicate {
                        if !predicate(drag_value.as_ref(), &hitbox, window, cx) {
                            return;
                        }
                    }

                    let drag_state_type = drag_value.as_ref().type_id();
                    for (recv_drop_type, listener) in &drop_listeners {
                        if *recv_drop_type != drag_state_type {
                            continue;
                        }

                        // Test whether the DropListener api can receive the payload
                        // (which may or may not test the hitbox being hovered).
                        if let Some(predicate) = &listener.can_drop {
                            if !predicate(drag_value.as_ref(), &hitbox, window, cx) {
                                continue;
                            }
                        }
                        // Finally, if the element does override can_drop and the listener doesn't,
                        // then we only still permit drop here if the hitbox is hovered.
                        // This allows both previous predicates to override this check if they see fit
                        // (or implement it manually or via interactivity).
                        else if !hitbox.is_hovered(window) {
                            continue;
                        }

                        let drag_value =
                            cx.stop_drag(window).expect("already confirmed drag exists");

                        if let Some(predicate) = &listener.on_drop {
                            predicate(drag_value.as_ref(), evt, window, cx);
                        }
                        window.refresh();
                        cx.stop_propagation();
                        break;
                    }
                }
            });
        }

        if let Some(element_state) = element_state {
            if !click_listeners.is_empty()
                || !aux_click_listeners.is_empty()
                || drag_listener.is_some()
            {
                let pending_mouse_down = element_state
                    .pending_mouse_down
                    .get_or_insert_with(Default::default)
                    .clone();

                let pending_keyboard_down = element_state
                    .pending_keyboard_down
                    .get_or_insert_with(Default::default)
                    .clone();

                let clicked_state = element_state
                    .clicked_state
                    .get_or_insert_with(Default::default)
                    .clone();

                window.on_mouse_event({
                    let pending_mouse_down = pending_mouse_down.clone();
                    let hitbox = hitbox.clone();
                    let has_aux_click_listeners = !aux_click_listeners.is_empty();
                    move |event: &MouseDownEvent, phase, window, _cx| {
                        if disabled {
                            return;
                        }

                        if phase == DispatchPhase::Bubble
                            && (event.button == MouseButton::Left || has_aux_click_listeners)
                            && hitbox.is_hovered(window)
                        {
                            *pending_mouse_down.borrow_mut() = Some(event.clone());
                            window.refresh();
                        }
                    }
                });

                window.on_mouse_event({
                    let pending_mouse_down = pending_mouse_down.clone();
                    let hitbox = hitbox.clone();
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        if disabled {
                            return;
                        }

                        if phase == DispatchPhase::Capture || drag_listener.is_none() {
                            return;
                        }

                        let mut pending_mouse_down = pending_mouse_down.borrow_mut();
                        let Some(mouse_down) = pending_mouse_down.clone() else {
                            return;
                        };
                        if cx.has_active_drag() || mouse_down.button != MouseButton::Left {
                            return;
                        }
                        if (event.position - mouse_down.position).magnitude() <= DRAG_THRESHOLD {
                            return;
                        }
                        if let Some(listener) = drag_listener.take() {
                            *clicked_state.borrow_mut() = ElementClickedState::default();
                            let cursor_offset = event.position - hitbox.origin;
                            let drag = listener(cursor_offset, drag_cursor_style, window, cx);
                            cx.start_drag(drag);
                            pending_mouse_down.take();
                            window.refresh();
                            cx.stop_propagation();
                        }
                    }
                });

                if is_focused {
                    // Record the focus generation at which an enter/space key
                    // down event happened on this element. The next key up
                    // event will be mapped to a click event if both of the
                    // following are true:
                    // - no other key events happen in between
                    // - the focus generation is the same (implying focus did not move)
                    //
                    // This design avoids an ABA problem that happens if you
                    // store the focus handle that registered the keypress.
                    window.on_key_event({
                        let pending_keyboard_down = pending_keyboard_down.clone();
                        move |event: &KeyDownEvent, phase, window, _cx| {
                            if disabled {
                                return;
                            }

                            if phase.bubble() && !window.default_prevented() {
                                let stroke = &event.keystroke;
                                let is_activation_key = (stroke.key.eq("enter")
                                    || stroke.key.eq("space"))
                                    && !stroke.modifiers.modified();
                                *pending_keyboard_down.borrow_mut() =
                                    is_activation_key.then_some(window.focus_generation);
                            }
                        }
                    });

                    // Press enter, space to trigger click, when the element is focused.
                    window.on_key_event({
                        let click_listeners = click_listeners.clone();
                        let hitbox = hitbox.clone();
                        move |event: &KeyUpEvent, phase, window, cx| {
                            if disabled {
                                return;
                            }

                            if phase.bubble() && !window.default_prevented() {
                                let stroke = &event.keystroke;
                                let keyboard_button = if stroke.key.eq("enter") {
                                    Some(KeyboardButton::Enter)
                                } else if stroke.key.eq("space") {
                                    Some(KeyboardButton::Space)
                                } else {
                                    None
                                };

                                if let Some(button) = keyboard_button
                                    && !stroke.modifiers.modified()
                                {
                                    let pending =
                                        std::mem::take(&mut *pending_keyboard_down.borrow_mut());
                                    if pending != Some(window.focus_generation) {
                                        return;
                                    }

                                    let click_event = ClickEvent::Keyboard(KeyboardClickEvent {
                                        button,
                                        bounds: hitbox.bounds,
                                    });

                                    for listener in &click_listeners {
                                        listener(&click_event, window, cx);
                                    }
                                } else {
                                    // Releasing any other key mid-press means
                                    // this isn't a clean activation, so cancel
                                    // the pending keydown.
                                    *pending_keyboard_down.borrow_mut() = None;
                                }
                            }
                        }
                    });
                }

                window.on_mouse_event({
                    let mut captured_mouse_down = None;
                    let hitbox = hitbox.clone();
                    move |event: &MouseUpEvent, phase, window, cx| {
                        if disabled {
                            return;
                        }

                        match phase {
                            // Clear the pending mouse down during the capture phase,
                            // so that it happens even if another event handler stops
                            // propagation.
                            DispatchPhase::Capture => {
                                let mut pending_mouse_down = pending_mouse_down.borrow_mut();
                                if pending_mouse_down.is_some() && hitbox.is_hovered(window) {
                                    captured_mouse_down = pending_mouse_down.take();
                                    window.refresh();
                                } else if pending_mouse_down.is_some() {
                                    // Clear the pending mouse down event (without firing click handlers)
                                    // if the hitbox is not being hovered.
                                    // This avoids dragging elements that changed their position
                                    // immediately after being clicked.
                                    // See https://github.com/zed-industries/zed/issues/24600 for more details
                                    pending_mouse_down.take();
                                    window.refresh();
                                }
                            }
                            // Fire click handlers during the bubble phase.
                            DispatchPhase::Bubble => {
                                if let Some(mouse_down) = captured_mouse_down.take() {
                                    let btn = mouse_down.button;

                                    let mouse_click = ClickEvent::Mouse(MouseClickEvent {
                                        down: mouse_down,
                                        up: event.clone(),
                                    });

                                    match btn {
                                        MouseButton::Left => {
                                            for listener in &click_listeners {
                                                listener(&mouse_click, window, cx);
                                            }
                                        }
                                        _ => {
                                            for listener in &aux_click_listeners {
                                                listener(&mouse_click, window, cx);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            }

            if !self.hover_region_listeners.is_empty() {
                let states = &mut element_state.hover_region_states;
                states.resize_with(self.hover_region_listeners.len(), HoverRegionState::default);
                for (listener, state) in self.hover_region_listeners.iter().zip(states) {
                    let observe = listener.bind(state);
                    let region_hitbox = hitbox.clone();
                    let pending_mouse_down = element_state
                        .pending_mouse_down
                        .get_or_insert_with(Default::default)
                        .clone();
                    window.on_pointer_change(move |change, window, cx| {
                        if disabled {
                            return;
                        }

                        let layout_changed = change.cause == PointerChangeCause::Layout;
                        if layout_changed && pending_mouse_down.borrow().is_some() {
                            return;
                        }
                        let position = if cx.has_active_drag()
                            || !layout_changed && pending_mouse_down.borrow().is_some()
                        {
                            None
                        } else if change.position.is_some() && !region_hitbox.is_hovered(window) {
                            None
                        } else {
                            change.position
                        };
                        observe(position, region_hitbox.bounds, window, cx);
                    });
                }
            }

            if let Some(tooltip_builder) = self.tooltip_builder.take() {
                let active_tooltip = element_state
                    .active_tooltip
                    .get_or_insert_with(Default::default)
                    .clone();
                let pending_mouse_down = element_state
                    .pending_mouse_down
                    .get_or_insert_with(Default::default)
                    .clone();

                let tooltip_is_hoverable = tooltip_builder.hoverable;
                let build_tooltip = Rc::new(move |window: &mut Window, cx: &mut App| {
                    Some(((tooltip_builder.build)(window, cx), tooltip_is_hoverable))
                });

                // Check hitbox geometry directly because hover state is unavailable during prepaint.
                let check_is_hovered_during_prepaint = Rc::new({
                    let pending_mouse_down = pending_mouse_down.clone();
                    let source_hitbox = hitbox.clone();
                    move |window: &Window| {
                        !window.hover_suppressed_by_keyboard()
                            && pending_mouse_down.borrow().is_none()
                            && source_hitbox.contains(&window.mouse_position())
                    }
                });
                let check_is_hovered = Rc::new({
                    let hitbox = hitbox.clone();
                    move |window: &Window| {
                        pending_mouse_down.borrow().is_none() && hitbox.is_hovered(window)
                    }
                });
                register_tooltip_mouse_handlers(
                    &active_tooltip,
                    self.tooltip_id,
                    build_tooltip,
                    check_is_hovered,
                    check_is_hovered_during_prepaint,
                    self.tooltip_show_delay,
                    window,
                );
            }

            // We unconditionally bind both the mouse up and mouse down active state handlers
            // Because we might not get a chance to render a frame before the mouse up event arrives.
            let active_state = element_state
                .clicked_state
                .get_or_insert_with(Default::default)
                .clone();

            {
                let active_state = active_state.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, phase, window, _cx| {
                    if phase == DispatchPhase::Capture && active_state.borrow().is_clicked() {
                        *active_state.borrow_mut() = ElementClickedState::default();
                        window.refresh();
                    }
                });
            }

            {
                let active_group_hitbox = self
                    .group_active_style
                    .as_ref()
                    .and_then(|group_active| GroupHitboxes::get(&group_active.group, cx));
                let hitbox = hitbox.clone();
                window.on_mouse_event(move |_: &MouseDownEvent, phase, window, _cx| {
                    if phase == DispatchPhase::Bubble && !window.default_prevented() {
                        let group_hovered = active_group_hitbox
                            .is_some_and(|group_hitbox_id| group_hitbox_id.is_hovered(window));
                        let element_hovered = hitbox.is_hovered(window);
                        if group_hovered || element_hovered {
                            *active_state.borrow_mut() = ElementClickedState {
                                group: group_hovered,
                                element: element_hovered,
                            };
                            window.refresh();
                        }
                    }
                });
            }
        }
    }

    fn paint_keyboard_listeners(&mut self, window: &mut Window, _cx: &mut App) {
        if self.disabled {
            return;
        }

        let key_down_listeners = mem::take(&mut self.key_down_listeners);
        let key_up_listeners = mem::take(&mut self.key_up_listeners);
        let modifiers_changed_listeners = mem::take(&mut self.modifiers_changed_listeners);
        let action_listeners = mem::take(&mut self.action_listeners);
        if let Some(context) = self.key_context.clone() {
            window.set_key_context(context);
        }

        for listener in key_down_listeners {
            window.on_key_event(move |event: &KeyDownEvent, phase, window, cx| {
                listener(event, phase, window, cx);
            })
        }

        for listener in key_up_listeners {
            window.on_key_event(move |event: &KeyUpEvent, phase, window, cx| {
                listener(event, phase, window, cx);
            })
        }

        for listener in modifiers_changed_listeners {
            window.on_modifiers_changed(move |event: &ModifiersChangedEvent, window, cx| {
                listener(event, window, cx);
            })
        }

        for (action_type, listener) in action_listeners {
            window.on_action(action_type, listener)
        }
    }

    fn paint_hover_group_handler(&self, window: &mut Window, cx: &mut App) {
        let group_hitbox = self
            .group_hover_style
            .as_ref()
            .and_then(|group_hover| GroupHitboxes::get(&group_hover.group, cx));

        if let Some(group_hitbox) = group_hitbox {
            let was_hovered = group_hitbox.is_hovered(window);
            let current_view = window.current_view();
            window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, cx| {
                let hovered = group_hitbox.is_hovered(window);
                if phase == DispatchPhase::Capture && hovered != was_hovered {
                    cx.notify(current_view);
                }
            });
        }
    }

    fn paint_scroll_listener(
        &self,
        hitbox: &Hitbox,
        style: &Style,
        window: &mut Window,
        _cx: &mut App,
    ) {
        if let Some(scroll_offset) = self.scroll_offset.clone() {
            let ongoing_scroll = self.ongoing_scroll.clone();
            let overflow = style.overflow;
            let allow_concurrent_scroll = style.allow_concurrent_scroll;
            let restrict_scroll_to_axis = style.restrict_scroll_to_axis;
            let line_height = window.line_height();
            let hitbox = hitbox.clone();
            let current_view = window.current_view();
            window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble && hitbox.should_handle_scroll(window) {
                    let mut scroll_offset = scroll_offset.borrow_mut();
                    let old_scroll_offset = *scroll_offset;
                    let mut delta = event.delta.pixel_delta(line_height);

                    if restrict_scroll_to_axis
                        && event.delta.precise()
                        && let Some(ongoing_scroll) = &ongoing_scroll
                    {
                        ongoing_scroll
                            .borrow_mut()
                            .filter(&mut delta, event.touch_phase);
                    }

                    let mut delta_x = match overflow.x {
                        Overflow::Scroll if !delta.x.is_zero() => delta.x,
                        Overflow::Scroll
                            if !restrict_scroll_to_axis && overflow.y != Overflow::Scroll =>
                        {
                            delta.y
                        }
                        _ => Pixels::ZERO,
                    };
                    let mut delta_y = match overflow.y {
                        Overflow::Scroll if !delta.y.is_zero() => delta.y,
                        Overflow::Scroll
                            if !restrict_scroll_to_axis && overflow.x != Overflow::Scroll =>
                        {
                            delta.x
                        }
                        _ => Pixels::ZERO,
                    };
                    if !allow_concurrent_scroll && !delta_x.is_zero() && !delta_y.is_zero() {
                        if delta_x.abs() > delta_y.abs() {
                            delta_y = Pixels::ZERO;
                        } else {
                            delta_x = Pixels::ZERO;
                        }
                    }
                    scroll_offset.y += delta_y;
                    scroll_offset.x += delta_x;
                    if *scroll_offset != old_scroll_offset {
                        cx.notify(current_view);
                    }
                }
            });
        }
    }

    /// Compute the visual style for this element, based on the current bounds and the element's state.
    pub fn compute_style(
        &self,
        global_id: Option<&GlobalElementId>,
        hitbox: Option<&Hitbox>,
        window: &mut Window,
        cx: &mut App,
    ) -> Style {
        window.with_optional_element_state(global_id, |element_state, window| {
            let mut element_state =
                element_state.map(|element_state| element_state.unwrap_or_default());
            let style = self.compute_style_internal(
                hitbox,
                hitbox.map(|hitbox| hitbox.bounds),
                element_state.as_mut(),
                window,
                cx,
            );
            (style, element_state)
        })
    }

    /// Called from internal methods that have already called with_element_state.
    fn compute_style_internal(
        &self,
        hitbox: Option<&Hitbox>,
        bounds: Option<Bounds<Pixels>>,
        mut element_state: Option<&mut InteractiveElementState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Style {
        let mut style = Style::default();
        style.refine(&self.base_style);

        if let Some(focus_handle) = self.tracked_focus_handle.as_ref() {
            if let Some(in_focus_style) = self.in_focus_style.as_ref()
                && focus_handle.within_focused(window, cx)
            {
                style.refine(in_focus_style);
            }

            if let Some(contains_focus_style) = self.contains_focus_style.as_ref()
                && focus_handle.contains_focused(window, cx)
            {
                style.refine(contains_focus_style);
            }

            if let Some(focus_style) = self.focus_style.as_ref()
                && focus_handle.is_focused(window)
            {
                style.refine(focus_style);
            }

            if let Some(focus_visible_style) = self.focus_visible_style.as_ref()
                && focus_handle.is_focused(window)
                && window.last_input_was_keyboard()
            {
                style.refine(focus_visible_style);
            }
        }

        if !cx.has_active_drag() {
            if let Some(group_hover) = self.group_hover_style.as_ref() {
                let is_group_hovered =
                    if let Some(group_hitbox_id) = GroupHitboxes::get(&group_hover.group, cx) {
                        !window.last_input_was_touch() && group_hitbox_id.is_hovered(window)
                    } else if let Some(element_state) = element_state.as_ref() {
                        element_state
                            .hover_state
                            .as_ref()
                            .is_some_and(|state| state.borrow().group_is_active(window))
                    } else {
                        false
                    };

                if is_group_hovered {
                    style.refine(&group_hover.style);
                }
            }

            if let Some(hover_style) = self.hover_style.as_ref() {
                let is_hovered = if let Some(hitbox) = hitbox {
                    !window.last_input_was_touch() && hitbox.is_hovered(window)
                } else if let Some(element_state) = element_state.as_ref() {
                    element_state
                        .hover_state
                        .as_ref()
                        .is_some_and(|state| state.borrow().element_is_active(window))
                } else {
                    false
                };

                if is_hovered {
                    style.refine(hover_style);
                }
            }
        }

        if let Some(hitbox) = hitbox {
            if let Some(drag) = cx.active_drag.take() {
                let drag_state_type = drag.value.as_ref().type_id();
                let can_drop_without_listener = self
                    .can_drop_predicate
                    .as_ref()
                    .map(|predicate| {
                        if self.disabled {
                            return false;
                        }

                        predicate(drag.value.as_ref(), hitbox, window, cx)
                    })
                    .unwrap_or(true);
                let relevant_drop_listener = match can_drop_without_listener {
                    false => None,
                    true => {
                        let mut iter = self.drop_listeners.iter();
                        iter.find(|(drop_state_type, listener)| {
                            if self.disabled {
                                return false;
                            }

                            if *drop_state_type != drag_state_type {
                                return false;
                            }

                            let mut can_drop = true;
                            if let Some(predicate) = &listener.can_drop {
                                can_drop = predicate(drag.value.as_ref(), hitbox, window, cx);
                            }
                            can_drop
                        })
                    }
                };

                if let Some((_, drop_listener)) = relevant_drop_listener {
                    for (state_type, group_drag_style) in &self.group_drag_over_styles {
                        if let Some(group_hitbox_id) =
                            GroupHitboxes::get(&group_drag_style.group, cx)
                            && *state_type == drag.value.as_ref().type_id()
                            && group_hitbox_id.is_hovered(window)
                        {
                            style.refine(&group_drag_style.style);
                        }
                    }

                    if hitbox.is_hovered(window) {
                        if let Some(drag_over) = &self.drag_over_style {
                            style.refine(&drag_over(drag.value.as_ref(), window, cx));
                        }
                        if let Some(drag_over) = &drop_listener.on_drag_over {
                            style.refine(&drag_over(drag.value.as_ref(), window, cx));
                        }
                    }
                }

                style.mouse_cursor = drag.cursor_style;
                cx.active_drag = Some(drag);
            }
        }

        if let Some(element_state) = element_state.as_deref_mut() {
            let clicked_state = element_state
                .clicked_state
                .get_or_insert_with(Default::default)
                .borrow();
            if clicked_state.group
                && let Some(group) = self.group_active_style.as_ref()
            {
                style.refine(&group.style)
            }

            if let Some(active_style) = self.active_style.as_ref()
                && clicked_state.element
            {
                style.refine(active_style)
            }
        }

        if self.disabled
            && let Some(disabled_style) = self.disabled_style.as_ref()
        {
            style.refine(disabled_style);
        }

        if let Some(element_state) = element_state {
            if let Some(transitions) = self.style_transitions.as_ref() {
                if transitions.apply(
                    &mut style,
                    element_state
                        .style_transitions
                        .get_or_insert_with(Default::default),
                    StyleTransitionContext::new(bounds, window.rem_size())
                        .with_containing_bounds(window.style_transition_containing_bounds()),
                    cx.background_executor().now(),
                    cx.reduce_motion(),
                ) {
                    window.request_animation_frame();
                }
            } else {
                element_state.style_transitions = None;
            }
        }

        style
    }

    pub(crate) fn write_a11y_info(&self, node: &mut accesskit::Node) {
        if let Some(id) = &self.aria.author_id {
            node.set_author_id(id.to_string());
        }
        if let Some(label) = &self.aria.label {
            node.set_label(label.to_string());
        }
        if let Some(description) = &self.aria.description {
            node.set_description(description.to_string());
        }
        if let Some(keyshortcuts) = &self.aria.keyshortcuts {
            node.set_keyboard_shortcut(keyshortcuts.to_string());
        }
        if let Some(invalid) = self.aria.invalid {
            node.set_invalid(invalid);
        }
        if let Some(selected) = self.aria.selected {
            node.set_selected(selected);
        }
        if let Some(expanded) = self.aria.expanded {
            node.set_expanded(expanded);
        }
        if let Some(toggled) = self.aria.toggled {
            node.set_toggled(toggled);
        }
        if let Some(disabled) = self.aria.disabled {
            if disabled {
                node.set_disabled();
            } else {
                node.clear_disabled();
            }
        }
        if let Some(value) = self.aria.numeric_value {
            node.set_numeric_value(value);
        }
        if let Some(value) = self.aria.min_numeric_value {
            node.set_min_numeric_value(value);
        }
        if let Some(value) = self.aria.max_numeric_value {
            node.set_max_numeric_value(value);
        }
        if let Some(step) = self.aria.numeric_value_step {
            node.set_numeric_value_step(step);
        }
        if let Some(value) = &self.aria.value {
            node.set_value(value.to_string());
        }
        if let Some(placeholder) = &self.aria.placeholder {
            node.set_placeholder(placeholder.to_string());
        }
        if let Some(orientation) = self.aria.orientation {
            node.set_orientation(orientation);
        }
        if let Some(level) = self.aria.level {
            node.set_level(level);
        }
        if let Some(position) = self.aria.position_in_set {
            node.set_position_in_set(position);
        }
        if let Some(size) = self.aria.size_of_set {
            node.set_size_of_set(size);
        }
        if let Some(index) = self.aria.row_index {
            node.set_row_index(index);
        }
        if let Some(index) = self.aria.column_index {
            node.set_column_index(index);
        }
        if let Some(count) = self.aria.row_count {
            node.set_row_count(count);
        }
        if let Some(count) = self.aria.column_count {
            node.set_column_count(count);
        }
        if let Some(modal) = self.aria.modal {
            if modal {
                node.set_modal();
            } else {
                node.clear_modal();
            }
        }
        if let Some(live) = self.aria.live {
            node.set_live(live);
        }
        if let Some(atomic) = self.aria.live_atomic {
            if atomic {
                node.set_live_atomic();
            } else {
                node.clear_live_atomic();
            }
        }
        if self.tracked_focus_handle.is_some() || self.focusable {
            node.add_action(accesskit::Action::Focus);
        }

        if self.disabled {
            return;
        }

        if !self.click_listeners.is_empty() {
            node.add_action(accesskit::Action::Click);
        }

        for (action, _) in &self.a11y_action_listeners {
            node.add_action(*action);
        }
    }
}

/// The per-frame state of an interactive element. Used for tracking stateful interactions like clicks
/// and scroll offsets.
#[derive(Default)]
pub struct InteractiveElementState {
    /// Reveal keyboard focus once per move; manual scrolling must remain possible.
    revealed_focus_generation: Option<u64>,
    pub(crate) focus_handle: Option<FocusHandle>,
    pub(crate) clicked_state: Option<Rc<RefCell<ElementClickedState>>>,
    pub(crate) hover_state: Option<Rc<RefCell<ElementHoverState>>>,
    pub(crate) hover_region_states: Vec<HoverRegionState>,
    pub(crate) pending_mouse_down: Option<Rc<RefCell<Option<MouseDownEvent>>>>,
    /// Set to the window's [`focus_generation`](crate::Window::focus_generation)
    /// when an Enter/Space keydown is received while this element is focused,
    /// recording that we are waiting for the matching keyup to fire a keyboard
    /// click. On keyup the click only fires if the stored generation still
    /// matches the window's current one, i.e. focus never moved during the
    /// press (mirroring the browser clearing a control's pressed state on
    /// blur). `None` means no activation key is pending.
    pub(crate) pending_keyboard_down: Option<Rc<RefCell<Option<u64>>>>,
    pub(crate) scroll_offset: Option<Rc<RefCell<Point<Pixels>>>>,
    ongoing_scroll: Option<Rc<RefCell<OngoingScroll>>>,
    pub(crate) active_tooltip: Option<Rc<RefCell<Option<ActiveTooltip>>>>,
    pub(crate) style_transitions: Option<Box<StyleTransitionState>>,
}

/// Whether or not the element or a group that contains it is clicked by the mouse.
#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct ElementClickedState {
    /// True if this element's group has been clicked, false otherwise
    pub group: bool,

    /// True if this element has been clicked, false otherwise
    pub element: bool,
}

impl ElementClickedState {
    fn is_clicked(&self) -> bool {
        self.group || self.element
    }
}

/// Whether or not the element or a group that contains it is hovered.
#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct ElementHoverState {
    /// True if this element's group is hovered, false otherwise
    pub group: bool,

    /// True if this element is hovered, false otherwise
    pub element: bool,
}

impl ElementHoverState {
    fn group_is_active(&self, window: &Window) -> bool {
        self.group && !window.hover_suppressed_by_keyboard() && !window.last_input_was_touch()
    }

    fn element_is_active(&self, window: &Window) -> bool {
        self.element && !window.hover_suppressed_by_keyboard() && !window.last_input_was_touch()
    }
}

pub(crate) enum ActiveTooltip {
    /// Currently delaying before showing the tooltip.
    WaitingForShow { _task: Task<()> },
    /// Tooltip is visible, element was hovered or for hoverable tooltips, the tooltip was hovered.
    Visible {
        tooltip: AnyTooltip,
        is_hoverable: bool,
    },
    /// Tooltip is visible and hoverable, but the mouse is no longer hovering. Currently delaying
    /// before hiding it.
    WaitingForHide {
        tooltip: AnyTooltip,
        _task: Task<()>,
    },
}

pub(crate) fn clear_active_tooltip(
    active_tooltip: &Rc<RefCell<Option<ActiveTooltip>>>,
    window: &mut Window,
) {
    match active_tooltip.borrow_mut().take() {
        None => {}
        Some(ActiveTooltip::WaitingForShow { .. }) => {}
        Some(ActiveTooltip::Visible { .. }) => window.refresh(),
        Some(ActiveTooltip::WaitingForHide { .. }) => window.refresh(),
    }
}

pub(crate) fn clear_active_tooltip_if_not_hoverable(
    active_tooltip: &Rc<RefCell<Option<ActiveTooltip>>>,
    window: &mut Window,
) {
    let should_clear = match active_tooltip.borrow().as_ref() {
        None => false,
        Some(ActiveTooltip::WaitingForShow { .. }) => false,
        Some(ActiveTooltip::Visible { is_hoverable, .. }) => !is_hoverable,
        Some(ActiveTooltip::WaitingForHide { .. }) => false,
    };
    if should_clear {
        active_tooltip.borrow_mut().take();
        window.refresh();
    }
}

pub(crate) fn set_tooltip_on_window(
    active_tooltip: &Rc<RefCell<Option<ActiveTooltip>>>,
    window: &mut Window,
) -> Option<TooltipId> {
    let tooltip = match active_tooltip.borrow().as_ref() {
        None => return None,
        Some(ActiveTooltip::WaitingForShow { .. }) => return None,
        Some(ActiveTooltip::Visible { tooltip, .. }) => tooltip.clone(),
        Some(ActiveTooltip::WaitingForHide { tooltip, .. }) => tooltip.clone(),
    };
    Some(window.set_tooltip(tooltip))
}

pub(crate) fn register_tooltip_mouse_handlers(
    active_tooltip: &Rc<RefCell<Option<ActiveTooltip>>>,
    tooltip_id: Option<TooltipId>,
    build_tooltip: Rc<dyn Fn(&mut Window, &mut App) -> Option<(AnyView, bool)>>,
    check_is_hovered: Rc<dyn Fn(&Window) -> bool>,
    check_is_hovered_during_prepaint: Rc<dyn Fn(&Window) -> bool>,
    show_delay: Option<Duration>,
    window: &mut Window,
) {
    let current_view = window.current_view();
    let show_delay = show_delay.unwrap_or(DEFAULT_TOOLTIP_SHOW_DELAY);

    window.on_mouse_event({
        let active_tooltip = active_tooltip.clone();
        let build_tooltip = build_tooltip.clone();
        let check_is_hovered = check_is_hovered.clone();
        move |_: &MouseMoveEvent, phase, window, cx| {
            handle_tooltip_mouse_move(
                &active_tooltip,
                &build_tooltip,
                &check_is_hovered,
                &check_is_hovered_during_prepaint,
                tooltip_id,
                current_view,
                phase,
                show_delay,
                window,
                cx,
            )
        }
    });

    window.on_mouse_event({
        let active_tooltip = active_tooltip.clone();
        move |_: &MouseDownEvent, _phase, window: &mut Window, _cx| {
            if !tooltip_id.is_some_and(|tooltip_id| tooltip_id.is_hovered(window)) {
                clear_active_tooltip_if_not_hoverable(&active_tooltip, window);
            }
        }
    });

    window.on_mouse_event({
        let active_tooltip = active_tooltip.clone();
        move |_: &ScrollWheelEvent, _phase, window: &mut Window, _cx| {
            if !tooltip_id.is_some_and(|tooltip_id| tooltip_id.is_hovered(window)) {
                clear_active_tooltip_if_not_hoverable(&active_tooltip, window);
            }
        }
    });
}

/// Handles displaying tooltips when an element is hovered.
///
/// The mouse hovering logic also relies on being called from window prepaint in order to handle the
/// case where the element the tooltip is on is not rendered - in that case its mouse listeners are
/// also not registered. During window prepaint, the hitbox information is not available, so
/// `check_is_hovered_during_prepaint` is used which bases the check off of the absolute bounds of
/// the element.
///
/// TODO: There's a minor bug due to the use of absolute bounds while checking during prepaint - it
/// does not know if the hitbox is occluded. In the case where a tooltip gets displayed and then
/// gets occluded after display, it will stick around until the mouse exits the hover bounds.
fn handle_tooltip_mouse_move(
    active_tooltip: &Rc<RefCell<Option<ActiveTooltip>>>,
    build_tooltip: &Rc<dyn Fn(&mut Window, &mut App) -> Option<(AnyView, bool)>>,
    check_is_hovered: &Rc<dyn Fn(&Window) -> bool>,
    check_is_hovered_during_prepaint: &Rc<dyn Fn(&Window) -> bool>,
    tooltip_id: Option<TooltipId>,
    current_view: EntityId,
    phase: DispatchPhase,
    show_delay: Duration,
    window: &mut Window,
    cx: &mut App,
) {
    // Separates logic for what mutation should occur from applying it, to avoid overlapping
    // RefCell borrows.
    enum Action {
        None,
        CancelShow,
        ScheduleShow,
        CheckVisible,
    }

    let action = match active_tooltip.borrow().as_ref() {
        None => {
            let is_hovered = check_is_hovered(window);
            if is_hovered && phase.bubble() {
                Action::ScheduleShow
            } else {
                Action::None
            }
        }
        Some(ActiveTooltip::WaitingForShow { .. }) => {
            let is_hovered = check_is_hovered(window);
            if is_hovered {
                Action::None
            } else {
                Action::CancelShow
            }
        }
        Some(ActiveTooltip::Visible { is_hoverable, .. }) => {
            if phase.capture()
                && !check_is_hovered(window)
                && (!*is_hoverable
                    || !tooltip_id.is_some_and(|tooltip_id| tooltip_id.is_hovered(window)))
            {
                Action::CheckVisible
            } else {
                Action::None
            }
        }
        Some(ActiveTooltip::WaitingForHide { .. }) => {
            if phase.capture()
                && (check_is_hovered(window)
                    || tooltip_id.is_some_and(|tooltip_id| tooltip_id.is_hovered(window)))
            {
                Action::CheckVisible
            } else {
                Action::None
            }
        }
    };

    match action {
        Action::None => {}
        Action::CancelShow => {
            // Cancel waiting to show tooltip when it is no longer hovered.
            active_tooltip.borrow_mut().take();
        }
        Action::ScheduleShow => {
            let delayed_show_task = window.spawn(cx, {
                let weak_active_tooltip = Rc::downgrade(active_tooltip);
                let build_tooltip = build_tooltip.clone();
                let check_is_hovered_during_prepaint = check_is_hovered_during_prepaint.clone();
                async move |cx| {
                    cx.background_executor().timer(show_delay).await;
                    let Some(active_tooltip) = weak_active_tooltip.upgrade() else {
                        return;
                    };
                    cx.update(|window, cx| {
                        let new_tooltip =
                            build_tooltip(window, cx).map(|(view, tooltip_is_hoverable)| {
                                let weak_active_tooltip = Rc::downgrade(&active_tooltip);
                                ActiveTooltip::Visible {
                                    tooltip: AnyTooltip {
                                        view,
                                        mouse_position: window.mouse_position(),
                                        check_visible_and_update: Rc::new(
                                            move |tooltip_bounds, window, cx| {
                                                let Some(active_tooltip) =
                                                    weak_active_tooltip.upgrade()
                                                else {
                                                    return false;
                                                };
                                                handle_tooltip_check_visible_and_update(
                                                    &active_tooltip,
                                                    tooltip_is_hoverable,
                                                    &check_is_hovered_during_prepaint,
                                                    tooltip_bounds,
                                                    window,
                                                    cx,
                                                )
                                            },
                                        ),
                                    },
                                    is_hoverable: tooltip_is_hoverable,
                                }
                            });
                        *active_tooltip.borrow_mut() = new_tooltip;
                        window.refresh();
                    })
                    .ok();
                }
            });
            active_tooltip
                .borrow_mut()
                .replace(ActiveTooltip::WaitingForShow {
                    _task: delayed_show_task,
                });
        }
        Action::CheckVisible => cx.notify(current_view),
    }
}

/// Returns a callback which will be called by window prepaint to update tooltip visibility. The
/// purpose of doing this logic here instead of the mouse move handler is that the mouse move
/// handler won't get called when the element is not painted (e.g. via use of `visible_on_hover`).
fn handle_tooltip_check_visible_and_update(
    active_tooltip: &Rc<RefCell<Option<ActiveTooltip>>>,
    tooltip_is_hoverable: bool,
    check_is_hovered: &Rc<dyn Fn(&Window) -> bool>,
    tooltip_bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    // Separates logic for what mutation should occur from applying it, to avoid overlapping RefCell
    // borrows.
    enum Action {
        None,
        Hide,
        ScheduleHide(AnyTooltip),
        CancelHide(AnyTooltip),
    }

    let is_hovered = check_is_hovered(window)
        || (tooltip_is_hoverable && tooltip_bounds.contains(&window.mouse_position()));
    let action = match active_tooltip.borrow().as_ref() {
        Some(ActiveTooltip::Visible { tooltip, .. }) => {
            if is_hovered {
                Action::None
            } else {
                if tooltip_is_hoverable {
                    Action::ScheduleHide(tooltip.clone())
                } else {
                    Action::Hide
                }
            }
        }
        Some(ActiveTooltip::WaitingForHide { tooltip, .. }) => {
            if is_hovered {
                Action::CancelHide(tooltip.clone())
            } else {
                Action::None
            }
        }
        None | Some(ActiveTooltip::WaitingForShow { .. }) => Action::None,
    };

    match action {
        Action::None => {}
        Action::Hide => clear_active_tooltip(active_tooltip, window),
        Action::ScheduleHide(tooltip) => {
            let delayed_hide_task = window.spawn(cx, {
                let weak_active_tooltip = Rc::downgrade(active_tooltip);
                async move |cx| {
                    cx.background_executor()
                        .timer(HOVERABLE_TOOLTIP_HIDE_DELAY)
                        .await;
                    let Some(active_tooltip) = weak_active_tooltip.upgrade() else {
                        return;
                    };
                    if active_tooltip.borrow_mut().take().is_some() {
                        cx.update(|window, _cx| window.refresh()).ok();
                    }
                }
            });
            active_tooltip
                .borrow_mut()
                .replace(ActiveTooltip::WaitingForHide {
                    tooltip,
                    _task: delayed_hide_task,
                });
        }
        Action::CancelHide(tooltip) => {
            // Cancel waiting to hide tooltip when it becomes hovered.
            active_tooltip.borrow_mut().replace(ActiveTooltip::Visible {
                tooltip,
                is_hoverable: true,
            });
        }
    }

    active_tooltip.borrow().is_some()
}

/// Temporary cache of hitbox groups, written and read from during paint recursion.
#[derive(Default)]
pub(crate) struct GroupHitboxes(HashMap<SharedString, SmallVec<[HitboxId; 1]>>);

impl Global for GroupHitboxes {}

impl GroupHitboxes {
    pub fn get(name: &SharedString, cx: &mut App) -> Option<HitboxId> {
        cx.default_global::<Self>()
            .0
            .get(name)
            .and_then(|bounds_stack| bounds_stack.last())
            .cloned()
    }

    pub fn push(name: SharedString, hitbox_id: HitboxId, cx: &mut App) {
        cx.default_global::<Self>()
            .0
            .entry(name)
            .or_default()
            .push(hitbox_id);
    }

    pub fn pop(name: &SharedString, cx: &mut App) {
        cx.default_global::<Self>().0.get_mut(name).unwrap().pop();
    }
}

/// A wrapper around an element that can store state, produced after assigning an ElementId.
pub struct Stateful<E> {
    pub(crate) element: E,
}

impl<E> Styled for Stateful<E>
where
    E: Styled,
{
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.style()
    }
}

impl<E> StatefulInteractiveElement for Stateful<E> where Self: InteractiveElement {}

impl<E> InteractiveElement for Stateful<E>
where
    E: InteractiveElement,
{
    fn interactivity(&mut self) -> &mut Interactivity {
        self.element.interactivity()
    }
}

impl<E> Element for Stateful<E>
where
    E: Element,
{
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.element.id()
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        self.element.source_location()
    }

    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.element.a11y_role()
    }

    fn is_a11y_hidden(&self) -> bool {
        self.element.is_a11y_hidden()
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut crate::A11ySubtreeBuilder,
    ) {
        self.element.a11y_synthetic_children(prepaint, builder);
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> E::PrepaintState {
        self.element
            .prepaint(id, inspector_id, bounds, state, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(
            id,
            inspector_id,
            bounds,
            request_layout,
            prepaint,
            window,
            cx,
        );
    }
}

impl<E: IntoElement> IntoElement for Stateful<E> {
    type Element = E::Element;

    fn into_element(self) -> Self::Element {
        self.element.into_element()
    }
}

impl<E> ParentElement for Stateful<E>
where
    E: ParentElement,
{
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.element.extend(elements)
    }
}

/// Represents an element that can be scrolled *to* in its parent element.
/// Contrary to [ScrollHandle::scroll_to_active_item], an anchored element does not have to be an immediate child of the parent.
#[derive(Clone)]
pub struct ScrollAnchor {
    handle: ScrollHandle,
    last_origin: Rc<RefCell<Point<Pixels>>>,
}

impl ScrollAnchor {
    /// Creates a [ScrollAnchor] associated with a given [ScrollHandle].
    pub fn for_handle(handle: ScrollHandle) -> Self {
        Self {
            handle,
            last_origin: Default::default(),
        }
    }
    /// Request scroll to this item on the next frame.
    pub fn scroll_to(&self, window: &mut Window, _cx: &mut App) {
        let this = self.clone();

        window.on_next_frame(move |_, _| {
            let viewport_bounds = this.handle.bounds();
            let self_bounds = *this.last_origin.borrow();
            this.handle.set_offset(viewport_bounds.origin - self_bounds);
        });
    }
}

#[derive(Default, Debug)]
struct ScrollHandleState {
    offset: Rc<RefCell<Point<Pixels>>>,
    ongoing_scroll: Rc<RefCell<OngoingScroll>>,
    bounds: Bounds<Pixels>,
    max_offset: Point<Pixels>,
    child_bounds: Vec<Bounds<Pixels>>,
    scroll_to_bottom: bool,
    overflow: Point<Overflow>,
    active_item: Option<ScrollActiveItem>,
}

#[derive(Default, Debug, Clone, Copy)]
struct ScrollActiveItem {
    index: usize,
    strategy: ScrollStrategy,
}

#[derive(Default, Debug, Clone, Copy)]
enum ScrollStrategy {
    #[default]
    FirstVisible,
    Top,
}

/// A handle to the scrollable aspects of an element.
/// Used for accessing scroll state, like the current scroll offset,
/// and for mutating the scroll state, like scrolling to a specific child.
#[derive(Clone, Debug)]
pub struct ScrollHandle(Rc<RefCell<ScrollHandleState>>);

impl Default for ScrollHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollHandle {
    /// Construct a new scroll handle.
    pub fn new() -> Self {
        Self(Rc::default())
    }

    /// Get the current scroll offset.
    pub fn offset(&self) -> Point<Pixels> {
        *self.0.borrow().offset.borrow()
    }

    /// Get the maximum scroll offset.
    pub fn max_offset(&self) -> Point<Pixels> {
        self.0.borrow().max_offset
    }

    /// Get the top child that's scrolled into view.
    pub fn top_item(&self) -> usize {
        let state = self.0.borrow();
        let top = state.bounds.top() - state.offset.borrow().y;

        match state.child_bounds.binary_search_by(|bounds| {
            if top < bounds.top() {
                Ordering::Greater
            } else if top > bounds.bottom() {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }) {
            Ok(ix) => ix,
            Err(ix) => ix.min(state.child_bounds.len().saturating_sub(1)),
        }
    }

    /// Get the bottom child that's scrolled into view.
    pub fn bottom_item(&self) -> usize {
        let state = self.0.borrow();
        let bottom = state.bounds.bottom() - state.offset.borrow().y;

        match state.child_bounds.binary_search_by(|bounds| {
            if bottom < bounds.top() {
                Ordering::Greater
            } else if bottom > bounds.bottom() {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }) {
            Ok(ix) => ix,
            Err(ix) => ix.min(state.child_bounds.len().saturating_sub(1)),
        }
    }

    /// Return the bounds into which this child is painted
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.0.borrow().bounds
    }

    /// Get the bounds for a specific child.
    pub fn bounds_for_item(&self, ix: usize) -> Option<Bounds<Pixels>> {
        self.0.borrow().child_bounds.get(ix).cloned()
    }

    /// Update [ScrollHandleState]'s active item for scrolling to in prepaint
    pub fn scroll_to_item(&self, ix: usize) {
        let mut state = self.0.borrow_mut();
        state.active_item = Some(ScrollActiveItem {
            index: ix,
            strategy: ScrollStrategy::default(),
        });
    }

    /// Update [ScrollHandleState]'s active item for scrolling to in prepaint
    /// This scrolls the minimal amount to ensure that the child is the first visible element
    pub fn scroll_to_top_of_item(&self, ix: usize) {
        let mut state = self.0.borrow_mut();
        state.active_item = Some(ScrollActiveItem {
            index: ix,
            strategy: ScrollStrategy::Top,
        });
    }

    /// Scrolls the minimal amount to either ensure that the child is
    /// fully visible or the top element of the view depends on the
    /// scroll strategy
    fn scroll_to_active_item(&self) {
        let mut state = self.0.borrow_mut();

        let Some(active_item) = state.active_item else {
            return;
        };

        let active_item = match state.child_bounds.get(active_item.index) {
            Some(bounds) => {
                let mut scroll_offset = state.offset.borrow_mut();

                match active_item.strategy {
                    ScrollStrategy::FirstVisible => {
                        if state.overflow.y == Overflow::Scroll {
                            let child_height = bounds.size.height;
                            let viewport_height = state.bounds.size.height;
                            if child_height > viewport_height {
                                scroll_offset.y = state.bounds.top() - bounds.top();
                            } else if bounds.top() + scroll_offset.y < state.bounds.top() {
                                scroll_offset.y = state.bounds.top() - bounds.top();
                            } else if bounds.bottom() + scroll_offset.y > state.bounds.bottom() {
                                scroll_offset.y = state.bounds.bottom() - bounds.bottom();
                            }
                        }
                    }
                    ScrollStrategy::Top => {
                        scroll_offset.y = state.bounds.top() - bounds.top();
                    }
                }

                if state.overflow.x == Overflow::Scroll {
                    let child_width = bounds.size.width;
                    let viewport_width = state.bounds.size.width;
                    if child_width > viewport_width {
                        scroll_offset.x = state.bounds.left() - bounds.left();
                    } else if bounds.left() + scroll_offset.x < state.bounds.left() {
                        scroll_offset.x = state.bounds.left() - bounds.left();
                    } else if bounds.right() + scroll_offset.x > state.bounds.right() {
                        scroll_offset.x = state.bounds.right() - bounds.right();
                    }
                }
                None
            }
            None => Some(active_item),
        };
        state.active_item = active_item;
    }

    /// Scrolls to the bottom.
    pub fn scroll_to_bottom(&self) {
        let mut state = self.0.borrow_mut();
        state.scroll_to_bottom = true;
    }

    /// Set the offset explicitly. The offset is the distance from the top left of the
    /// parent container to the top left of the first child.
    /// As you scroll further down the offset becomes more negative.
    pub fn set_offset(&self, mut position: Point<Pixels>) {
        let state = self.0.borrow();
        *state.offset.borrow_mut() = position;
    }

    /// Get the logical scroll top, based on a child index and a pixel offset.
    pub fn logical_scroll_top(&self) -> (usize, Pixels) {
        let ix = self.top_item();
        let state = self.0.borrow();

        if let Some(child_bounds) = state.child_bounds.get(ix) {
            (
                ix,
                child_bounds.top() + state.offset.borrow().y - state.bounds.top(),
            )
        } else {
            (ix, px(0.))
        }
    }

    /// Get the logical scroll bottom, based on a child index and a pixel offset.
    pub fn logical_scroll_bottom(&self) -> (usize, Pixels) {
        let ix = self.bottom_item();
        let state = self.0.borrow();

        if let Some(child_bounds) = state.child_bounds.get(ix) {
            (
                ix,
                child_bounds.bottom() + state.offset.borrow().y - state.bounds.bottom(),
            )
        } else {
            (ix, px(0.))
        }
    }

    /// Get the count of children for scrollable item.
    pub fn children_count(&self) -> usize {
        self.0.borrow().child_bounds.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn inline_div_places_element_children_in_text_flow(cx: &mut TestAppContext) {
        let placed_children = Rc::new(RefCell::new(Vec::new()));
        let captured_bounds = placed_children.clone();
        let window = cx.add_empty_window();
        let highlight_color = hsla(0.4, 0.6, 0.5, 1.0);

        window.draw(
            point(px(10.), px(20.)),
            size(px(160.), px(80.)),
            move |_, _| {
                div()
                    .block()
                    .w(px(160.))
                    .child(StyledText::new("prefix ").with_highlights([(
                        0.."prefix ".len(),
                        HighlightStyle {
                            background_color: Some(highlight_color),
                            ..Default::default()
                        },
                    )]))
                    .child(div().inline_flex().size(px(18.)).align_baseline())
                    .child(div().inline_flex().size(px(18.)).align_middle())
                    .child(" suffix")
                    .on_children_prepainted(move |bounds, _, _| {
                        *captured_bounds.borrow_mut() = bounds;
                    })
                    .into_any_element()
            },
        );

        let placed_children = placed_children.borrow();
        assert_eq!(placed_children.len(), 4);
        let baseline_box = placed_children[1];
        let middle_box = placed_children[2];
        assert_eq!(baseline_box.size, size(px(18.), px(18.)));
        assert_eq!(middle_box.size, baseline_box.size);
        assert!(baseline_box.origin.x > px(10.));
        assert!(middle_box.origin.x >= baseline_box.right());
        assert!(middle_box.right() <= px(170.));
        assert!(baseline_box.origin.y >= px(20.));
        assert!(middle_box.origin.y > baseline_box.origin.y);
        assert!(middle_box.bottom() > baseline_box.bottom());
        assert!(
            window.update(|window, _| {
                window
                    .rendered_frame
                    .scene
                    .quads
                    .iter()
                    .any(|quad| quad.background.solid == highlight_color.into())
            }),
            "the highlighted inline text did not paint its background"
        );
    }

    #[gpui::test]
    fn default_block_preserves_grid_flow_and_hides_none_descendants(cx: &mut TestAppContext) {
        let hidden_prepainted = Rc::new(Cell::new(false));
        let hidden_prepaint = hidden_prepainted.clone();
        let window = cx.add_empty_window();

        window.draw(
            point(px(10.), px(20.)),
            size(px(200.), px(100.)),
            move |_, _| {
                div()
                    .w(px(200.))
                    .child(
                        div()
                            .grid()
                            .w_full()
                            .grid_cols(2)
                            .debug_selector(|| "grid".into())
                            .child(div().h(px(20.)).debug_selector(|| "grid-cell-1".into()))
                            .child(div().h(px(30.)).debug_selector(|| "grid-cell-2".into())),
                    )
                    .child(div().hidden().h(px(300.)).child(canvas(
                        move |_, _, _| hidden_prepaint.set(true),
                        |_, _, _, _| {},
                    )))
                    .child(div().h(px(10.)).debug_selector(|| "following-block".into()))
                    .into_any_element()
            },
        );

        let [grid, first_cell, second_cell, following] =
            ["grid", "grid-cell-1", "grid-cell-2", "following-block"].map(|selector| {
                window
                    .update(|window, _| window.rendered_frame.debug_bounds.get(selector).copied())
                    .unwrap_or_else(|| panic!("{selector} was not rendered"))
            });

        assert_eq!(first_cell.origin.y, grid.origin.y);
        assert_eq!(second_cell.origin.y, grid.origin.y);
        assert!(second_cell.origin.x > first_cell.origin.x);
        assert_eq!(following.origin.y, grid.bottom());
        assert!(!hidden_prepainted.get());
    }

    struct ChildPrepaintTestView {
        custom_order: bool,
        visits: Rc<RefCell<Vec<(usize, Point<Pixels>)>>>,
        callback_offsets: Rc<RefCell<Vec<Point<Pixels>>>>,
        scroll_handle: ScrollHandle,
    }

    impl Render for ChildPrepaintTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let children = (0..3).map(|index| {
                let visits = self.visits.clone();

                canvas(
                    move |_bounds, window, _cx| {
                        visits.borrow_mut().push((index, window.element_offset()));
                    },
                    |_bounds, _state, _window, _cx| {},
                )
                .w_full()
                .h(px(40.))
                .flex_shrink_0()
            });
            let mut element = div().size_full().flex().flex_col().children(children);

            if self.custom_order {
                let visits = self.visits.clone();
                let callback_offsets = self.callback_offsets.clone();
                element = element.with_dynamic_prepaint_order(move |window, _cx| {
                    assert!(visits.borrow().is_empty());
                    callback_offsets.borrow_mut().push(window.element_offset());

                    SmallVec::from_slice(&[2, 9, 0, 1])
                });
            }

            element
                .id("prepaint-order")
                .overflow_y_scroll()
                .track_scroll(&self.scroll_handle)
        }
    }

    #[gpui::test]
    fn child_prepaint_preserves_order_and_scroll_offset(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let origin = point(px(20.), px(30.));
        let scroll_offset = point(px(0.), px(-15.));
        let expected_offset = origin + scroll_offset;

        for custom_order in [false, true] {
            let visits = Rc::new(RefCell::new(Vec::new()));
            let callback_offsets = Rc::new(RefCell::new(Vec::new()));
            let scroll_handle = ScrollHandle::new();
            scroll_handle.set_offset(scroll_offset);

            cx.draw(origin, size(px(50.), px(50.)), |_window, cx| {
                cx.new(|_cx| ChildPrepaintTestView {
                    custom_order,
                    visits: visits.clone(),
                    callback_offsets: callback_offsets.clone(),
                    scroll_handle,
                })
                .into_any_element()
            });

            let expected_order = if custom_order { [2, 0, 1] } else { [0, 1, 2] };
            assert_eq!(
                *visits.borrow(),
                expected_order.map(|index| (index, expected_offset)),
            );

            if custom_order {
                assert_eq!(*callback_offsets.borrow(), [expected_offset]);
            } else {
                assert!(callback_offsets.borrow().is_empty());
            }
        }
    }

    struct GroupHoverTestView {
        render_count: Rc<Cell<usize>>,
        anonymous_paint_count: Rc<Cell<usize>>,
        stateful_width: Rc<Cell<Pixels>>,
    }

    impl Render for GroupHoverTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            self.render_count.set(self.render_count.get() + 1);
            let anonymous_paint_count = self.anonymous_paint_count.clone();
            let stateful_width = self.stateful_width.clone();
            div().size_full().child(
                div()
                    .ml(px(20.))
                    .mt(px(20.))
                    .size(px(50.))
                    .relative()
                    .group("hover-group")
                    .child(
                        div()
                            .absolute()
                            .size_full()
                            .invisible()
                            .group_hover_style("hover-group", |style| style.visible())
                            .child(canvas(
                                |_, _, _| {},
                                move |_, _, _, _| {
                                    anonymous_paint_count.set(anonymous_paint_count.get() + 1)
                                },
                            )),
                    )
                    .child(
                        div()
                            .id("stateful-group-hover-target")
                            .absolute()
                            .top_0()
                            .left_0()
                            .size(px(10.))
                            .group_hover_style("hover-group", |style| style.size(px(20.)))
                            .child(
                                canvas(
                                    move |bounds, _, _| stateful_width.set(bounds.size.width),
                                    |_, _, _, _| {},
                                )
                                .size_full(),
                            ),
                    ),
            )
        }
    }

    #[gpui::test]
    fn group_hover_styles_update_only_on_transitions(cx: &mut TestAppContext) {
        let render_count = Rc::new(Cell::new(0));
        let anonymous_paint_count = Rc::new(Cell::new(0));
        let stateful_width = Rc::new(Cell::new(px(0.)));
        let window = cx.add_window({
            let render_count = render_count.clone();
            let anonymous_paint_count = anonymous_paint_count.clone();
            let stateful_width = stateful_width.clone();
            move |_, _| GroupHoverTestView {
                render_count,
                anonymous_paint_count,
                stateful_width,
            }
        });
        let window = AnyWindowHandle::from(window);

        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        assert_eq!(anonymous_paint_count.get(), 0);
        assert_eq!(stateful_width.get(), px(10.));

        let move_mouse = |cx: &mut TestAppContext, position| {
            cx.update_window(window, |_, window, cx| {
                window.simulate_mouse_move(position, cx)
            })
            .unwrap();
        };

        let initial_render_count = render_count.get();
        move_mouse(cx, point(px(25.), px(25.)));
        assert_eq!(render_count.get(), initial_render_count + 1);
        assert_eq!(anonymous_paint_count.get(), 1);
        assert_eq!(stateful_width.get(), px(20.));

        move_mouse(cx, point(px(30.), px(30.)));
        assert_eq!(render_count.get(), initial_render_count + 1);
        assert_eq!(anonymous_paint_count.get(), 1);
        assert_eq!(stateful_width.get(), px(20.));

        move_mouse(cx, point(px(5.), px(5.)));
        assert_eq!(render_count.get(), initial_render_count + 2);
        assert_eq!(anonymous_paint_count.get(), 1);
        assert_eq!(stateful_width.get(), px(10.));

        move_mouse(cx, point(px(10.), px(10.)));
        assert_eq!(render_count.get(), initial_render_count + 2);
        assert_eq!(anonymous_paint_count.get(), 1);
        assert_eq!(stateful_width.get(), px(10.));
    }

    struct HoverListenerLayoutTestView {
        target_left: Pixels,
        hover_transitions: Rc<RefCell<Vec<bool>>>,
    }

    impl Render for HoverListenerLayoutTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let hover_transitions = self.hover_transitions.clone();
            div().relative().size_full().child(
                div()
                    .id("hover-target")
                    .absolute()
                    .left(self.target_left)
                    .top_0()
                    .size(px(20.))
                    .on_click(|_, _, _| {})
                    .on_hover(move |is_hovered, _, _| {
                        hover_transitions.borrow_mut().push(*is_hovered);
                    }),
            )
        }
    }

    #[gpui::test]
    fn hover_listeners_update_when_layout_changes_under_stationary_mouse(cx: &mut TestAppContext) {
        let hover_transitions = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let hover_transitions = hover_transitions.clone();
            move |_, _| HoverListenerLayoutTestView {
                target_left: px(40.),
                hover_transitions,
            }
        });
        let any_window = AnyWindowHandle::from(window);

        cx.update_window(any_window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.simulate_mouse_move(point(px(10.), px(10.)), cx);
        })
        .unwrap();
        assert!(hover_transitions.borrow().is_empty());

        window
            .update(cx, |view, _, cx| {
                view.target_left = px(0.);
                cx.notify();
            })
            .unwrap();
        cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        assert_eq!(*hover_transitions.borrow(), [true]);

        window
            .update(cx, |view, _, cx| {
                view.target_left = px(40.);
                cx.notify();
            })
            .unwrap();
        cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        assert_eq!(*hover_transitions.borrow(), [true, false]);
    }

    #[gpui::test]
    fn hover_listeners_remain_hovered_during_stationary_mouse_press(cx: &mut TestAppContext) {
        let hover_transitions = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let hover_transitions = hover_transitions.clone();
            move |_, _| HoverListenerLayoutTestView {
                target_left: px(0.),
                hover_transitions,
            }
        });
        let any_window = AnyWindowHandle::from(window);
        let mouse_position = point(px(10.), px(10.));

        cx.update_window(any_window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.simulate_mouse_move(mouse_position, cx);
        })
        .unwrap();
        assert_eq!(*hover_transitions.borrow(), [true]);

        cx.update_window(any_window, |_, window, cx| {
            window.dispatch_event(
                MouseDownEvent {
                    position: mouse_position,
                    button: MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
            window.draw(cx).clear(cx);
        })
        .unwrap();
        assert_eq!(*hover_transitions.borrow(), [true]);

        cx.update_window(any_window, |_, window, cx| {
            window.dispatch_event(
                MouseUpEvent {
                    position: mouse_position,
                    button: MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            window.draw(cx).clear(cx);
        })
        .unwrap();
        assert_eq!(*hover_transitions.borrow(), [true]);
    }

    struct HoverRegionView {
        events: Rc<RefCell<Vec<Option<&'static str>>>>,
        projections: Rc<Cell<usize>>,
    }

    impl Render for HoverRegionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let events = self.events.clone();
            let projections = self.projections.clone();
            div().size_full().child(
                div()
                    .id("hover-region-target")
                    .ml(px(20.))
                    .mt(px(20.))
                    .size(px(40.))
                    .on_hover_region(
                        move |position, bounds| {
                            projections.set(projections.get() + 1);
                            let x = position.x - bounds.left();
                            (x < px(15.))
                                .then_some("left")
                                .or_else(|| (x >= px(25.)).then_some("right"))
                        },
                        move |region, _, _| events.borrow_mut().push(*region),
                    ),
            )
        }
    }

    #[gpui::test]
    fn hover_region_transitions_once_and_stays_none_after_exit(cx: &mut TestAppContext) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let projections = Rc::new(Cell::new(0));
        let window = cx.add_window({
            let events = events.clone();
            let projections = projections.clone();
            move |_, _| HoverRegionView {
                events,
                projections,
            }
        });
        let any_window = AnyWindowHandle::from(window);

        let mut expected = Vec::new();
        for (x, transition) in [
            (25., Some(Some("left"))),
            (30., None),
            (40., Some(None)), // The projector can decline while the element stays hovered.
            (42., None),
            (50., Some(Some("right"))),
            (50., None),
        ] {
            cx.update_window(any_window, |_, window, cx| {
                window.simulate_mouse_move(point(px(x), px(25.)), cx);
                window.draw(cx).clear(cx);
            })
            .unwrap();
            if let Some(transition) = transition {
                expected.push(transition);
            }
            assert_eq!(*events.borrow(), expected, "x={x}");
        }
        let count = projections.get();
        cx.update_window(any_window, |_, window, cx| {
            window.dispatch_event(
                // No final move: the window still holds the last inside position.
                MouseExitEvent::default().to_platform_input(),
                cx,
            );
            window.draw(cx).clear(cx);
            window.dispatch_event(MouseExitEvent::default().to_platform_input(), cx);
            window.draw(cx).clear(cx);
        })
        .unwrap();

        expected.push(None);
        assert_eq!(*events.borrow(), expected);
        assert_eq!(
            projections.get(),
            count,
            "exit and subsequent draws must not run the projection"
        );
        cx.update_window(any_window, |_, window, cx| {
            window.simulate_mouse_move(point(px(50.), px(25.)), cx);
            window.simulate_mouse_move(point(px(60.), px(25.)), cx);
            window.simulate_mouse_move(point(px(65.), px(25.)), cx);
        })
        .unwrap();
        expected.extend([Some("right"), None]);
        assert_eq!(
            *events.borrow(),
            expected,
            "reentry and half-open element bounds"
        );
    }

    #[derive(Debug, PartialEq)]
    enum HoverRegionEvent {
        Number(Option<u8>),
        Flag(Option<bool>),
    }

    struct MultipleHoverRegionsView {
        events: Rc<RefCell<Vec<HoverRegionEvent>>>,
    }

    impl Render for MultipleHoverRegionsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let events = self.events.clone();
            let events_second = events.clone();
            div()
                .id("multiple-hover-regions")
                .size(px(40.))
                .on_hover_region(
                    |position, _| (position.x < px(20.)).then_some(1u8),
                    move |value, _, _| events.borrow_mut().push(HoverRegionEvent::Number(*value)),
                )
                .on_hover_region(
                    |position, _| (position.x >= px(20.)).then_some(true),
                    move |value, _, _| {
                        // Keep this listener independently typed from the first one.
                        events_second
                            .borrow_mut()
                            .push(HoverRegionEvent::Flag(*value));
                    },
                )
        }
    }

    #[gpui::test]
    fn hover_regions_keep_independent_state_when_rebuilt(cx: &mut TestAppContext) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let view = cx.add_window({
            let events = events.clone();
            move |_, _| MultipleHoverRegionsView { events }
        });
        let any_window = AnyWindowHandle::from(view);
        cx.update_window(any_window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.simulate_mouse_move(point(px(5.), px(5.)), cx);
            window.simulate_mouse_move(point(px(25.), px(5.)), cx);
        })
        .unwrap();
        use HoverRegionEvent::{Flag, Number};
        assert_eq!(
            *events.borrow(),
            [Number(Some(1)), Number(None), Flag(Some(true))]
        );
        for _ in 0..3 {
            view.update(cx, |_, _, cx| cx.notify()).unwrap();
            cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
                .unwrap();
            assert_eq!(
                *events.borrow(),
                [Number(Some(1)), Number(None), Flag(Some(true))]
            );
        }
        cx.update_window(any_window, |_, window, cx| {
            window.simulate_mouse_move(point(px(5.), px(5.)), cx);
            window.simulate_mouse_move(point(px(50.), px(5.)), cx);
        })
        .unwrap();
        assert_eq!(
            *events.borrow(),
            [
                Number(Some(1)),
                Number(None),
                Flag(Some(true)),
                Number(Some(1)),
                Flag(None),
                Number(None)
            ]
        );
    }

    struct ChangingHoverRegionView {
        enabled: bool,
        alternate_type: bool,
        events: Rc<RefCell<Vec<HoverRegionEvent>>>,
    }

    impl Render for ChangingHoverRegionView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let events = self.events.clone();
            let target = div().id("changing-regions").size(px(40.));
            if !self.enabled {
                target
            } else if self.alternate_type {
                target.on_hover_region(
                    |_, _| Some(true),
                    move |value, _, _| events.borrow_mut().push(HoverRegionEvent::Flag(*value)),
                )
            } else {
                target.on_hover_region(
                    |_, _| Some(1u8),
                    move |value, _, _| events.borrow_mut().push(HoverRegionEvent::Number(*value)),
                )
            }
        }
    }

    #[gpui::test]
    fn hover_region_registration_replaces_types_and_discards_removed_state(
        cx: &mut TestAppContext,
    ) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let events = events.clone();
            move |_, _| ChangingHoverRegionView {
                enabled: true,
                alternate_type: false,
                events,
            }
        });
        let any_window = AnyWindowHandle::from(window);
        cx.update_window(any_window, |_, window, cx| {
            window.simulate_mouse_move(point(px(5.), px(5.)), cx);
        })
        .unwrap();
        assert_eq!(*events.borrow(), [HoverRegionEvent::Number(Some(1))]);
        window
            .update(cx, |view, _, cx| {
                view.alternate_type = true;
                cx.notify();
            })
            .unwrap();
        cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        assert_eq!(
            *events.borrow(),
            [
                HoverRegionEvent::Number(Some(1)),
                HoverRegionEvent::Flag(Some(true))
            ]
        );
        window
            .update(cx, |view, _, cx| {
                view.enabled = false;
                cx.notify();
            })
            .unwrap();
        cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        window
            .update(cx, |view, _, cx| {
                view.enabled = true;
                cx.notify();
            })
            .unwrap();
        cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        assert_eq!(
            *events.borrow(),
            [
                HoverRegionEvent::Number(Some(1)),
                HoverRegionEvent::Flag(Some(true)),
                HoverRegionEvent::Flag(Some(true))
            ]
        );
    }

    struct TestTooltipView;

    impl Render for TestTooltipView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().w(px(20.)).h(px(20.)).child("tooltip")
        }
    }

    type CapturedActiveTooltip = Rc<RefCell<Option<Weak<RefCell<Option<ActiveTooltip>>>>>>;

    struct TooltipCaptureElement {
        child: AnyElement,
        captured_active_tooltip: CapturedActiveTooltip,
    }

    impl IntoElement for TooltipCaptureElement {
        type Element = Self;

        fn into_element(self) -> Self::Element {
            self
        }
    }

    impl Element for TooltipCaptureElement {
        type RequestLayoutState = ();
        type PrepaintState = ();

        fn id(&self) -> Option<ElementId> {
            None
        }

        fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
            None
        }

        fn request_layout(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            window: &mut Window,
            cx: &mut App,
        ) -> (LayoutId, Self::RequestLayoutState) {
            (self.child.request_layout(window, cx), ())
        }

        fn prepaint(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            _bounds: Bounds<Pixels>,
            _request_layout: &mut Self::RequestLayoutState,
            window: &mut Window,
            cx: &mut App,
        ) -> Self::PrepaintState {
            self.child.prepaint(window, cx);
        }

        fn paint(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            _bounds: Bounds<Pixels>,
            _request_layout: &mut Self::RequestLayoutState,
            _prepaint: &mut Self::PrepaintState,
            window: &mut Window,
            cx: &mut App,
        ) {
            self.child.paint(window, cx);
            window.with_global_id("target".into(), |global_id, window| {
                window.with_element_state::<InteractiveElementState, _>(
                    global_id,
                    |state, _window| {
                        let state = state.unwrap();
                        *self.captured_active_tooltip.borrow_mut() =
                            state.active_tooltip.as_ref().map(Rc::downgrade);
                        ((), state)
                    },
                )
            });
        }
    }

    struct TooltipOwner {
        captured_active_tooltip: CapturedActiveTooltip,
        show_delay_override: Option<Duration>,
    }

    impl Render for TooltipOwner {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            TooltipCaptureElement {
                child: div()
                    .size_full()
                    .child(
                        div()
                            .id("target")
                            .w(px(50.))
                            .h(px(50.))
                            .tooltip(|_, cx| cx.new(|_| TestTooltipView).into())
                            .when_some(self.show_delay_override, |this, delay| {
                                this.tooltip_show_delay(delay)
                            }),
                    )
                    .into_any_element(),
                captured_active_tooltip: self.captured_active_tooltip.clone(),
            }
        }
    }

    #[test]
    fn scroll_handle_aligns_wide_children_to_left_edge() {
        let handle = ScrollHandle::new();
        {
            let mut state = handle.0.borrow_mut();
            state.bounds = Bounds::new(point(px(0.), px(0.)), size(px(80.), px(20.)));
            state.child_bounds = vec![Bounds::new(point(px(25.), px(0.)), size(px(200.), px(20.)))];
            state.overflow.x = Overflow::Scroll;
            state.active_item = Some(ScrollActiveItem {
                index: 0,
                strategy: ScrollStrategy::default(),
            });
        }

        handle.scroll_to_active_item();

        assert_eq!(handle.offset().x, px(-25.));
    }

    #[test]
    fn scroll_handle_aligns_tall_children_to_top_edge() {
        let handle = ScrollHandle::new();
        {
            let mut state = handle.0.borrow_mut();
            state.bounds = Bounds::new(point(px(0.), px(0.)), size(px(20.), px(80.)));
            state.child_bounds = vec![Bounds::new(point(px(0.), px(25.)), size(px(20.), px(200.)))];
            state.overflow.y = Overflow::Scroll;
            state.active_item = Some(ScrollActiveItem {
                index: 0,
                strategy: ScrollStrategy::default(),
            });
        }

        handle.scroll_to_active_item();

        assert_eq!(handle.offset().y, px(-25.));
    }

    fn setup_tooltip_owner_test(
        show_delay_override: Option<Duration>,
    ) -> (
        TestAppContext,
        crate::AnyWindowHandle,
        CapturedActiveTooltip,
    ) {
        let mut test_app = TestAppContext::single();
        let captured_active_tooltip: CapturedActiveTooltip = Rc::new(RefCell::new(None));
        let window = test_app.add_window({
            let captured_active_tooltip = captured_active_tooltip.clone();
            move |_, _| TooltipOwner {
                captured_active_tooltip,
                show_delay_override,
            }
        });
        let any_window = window.into();

        test_app
            .update_window(any_window, |_, window, cx| {
                window.draw(cx).clear(cx);
            })
            .unwrap();

        test_app
            .update_window(any_window, |_, window, cx| {
                window.dispatch_event(
                    MouseMoveEvent {
                        position: point(px(10.), px(10.)),
                        modifiers: Default::default(),
                        pressed_button: None,
                    }
                    .to_platform_input(),
                    cx,
                );
            })
            .unwrap();

        test_app
            .update_window(any_window, |_, window, cx| {
                window.draw(cx).clear(cx);
            })
            .unwrap();

        (test_app, any_window, captured_active_tooltip)
    }

    #[test]
    fn tooltip_waiting_for_show_is_released_when_its_owner_disappears() {
        let (mut test_app, any_window, captured_active_tooltip) = setup_tooltip_owner_test(None);

        let weak_active_tooltip = captured_active_tooltip.borrow().clone().unwrap();
        let active_tooltip = weak_active_tooltip.upgrade().unwrap();
        assert!(matches!(
            active_tooltip.borrow().as_ref(),
            Some(ActiveTooltip::WaitingForShow { .. })
        ));

        test_app
            .update_window(any_window, |_, window, _| {
                window.remove_window();
            })
            .unwrap();
        test_app.run_until_parked();
        drop(active_tooltip);

        assert!(weak_active_tooltip.upgrade().is_none());
    }

    #[test]
    fn tooltip_respects_custom_show_delay() {
        let extra_delay = Duration::from_secs(1);
        let show_delay_override = DEFAULT_TOOLTIP_SHOW_DELAY + extra_delay;
        let (mut test_app, _any_window, captured_active_tooltip) =
            setup_tooltip_owner_test(Some(show_delay_override));

        let weak_active_tooltip = captured_active_tooltip.borrow().clone().unwrap();
        let active_tooltip = weak_active_tooltip.upgrade().unwrap();

        test_app
            .dispatcher
            .advance_clock(DEFAULT_TOOLTIP_SHOW_DELAY);
        test_app.run_until_parked();

        assert!(matches!(
            active_tooltip.borrow().as_ref(),
            Some(ActiveTooltip::WaitingForShow { .. })
        ));

        test_app.dispatcher.advance_clock(extra_delay);
        test_app.run_until_parked();

        assert!(matches!(
            active_tooltip.borrow().as_ref(),
            Some(ActiveTooltip::Visible { .. })
        ));
    }

    #[test]
    fn tooltip_is_released_when_its_owner_disappears() {
        let (mut test_app, any_window, captured_active_tooltip) = setup_tooltip_owner_test(None);

        let weak_active_tooltip = captured_active_tooltip.borrow().clone().unwrap();
        let active_tooltip = weak_active_tooltip.upgrade().unwrap();

        test_app
            .dispatcher
            .advance_clock(DEFAULT_TOOLTIP_SHOW_DELAY);
        test_app.run_until_parked();

        assert!(matches!(
            active_tooltip.borrow().as_ref(),
            Some(ActiveTooltip::Visible { .. })
        ));

        test_app
            .update_window(any_window, |_, window, _| {
                window.remove_window();
            })
            .unwrap();
        test_app.run_until_parked();
        drop(active_tooltip);

        assert!(weak_active_tooltip.upgrade().is_none());
    }

    #[test]
    fn tooltip_hides_after_mouse_leaves_origin() {
        let (mut test_app, any_window, captured_active_tooltip) = setup_tooltip_owner_test(None);

        let weak_active_tooltip = captured_active_tooltip.borrow().clone().unwrap();
        let active_tooltip = weak_active_tooltip.upgrade().unwrap();

        test_app
            .dispatcher
            .advance_clock(DEFAULT_TOOLTIP_SHOW_DELAY);
        test_app.run_until_parked();

        assert!(matches!(
            active_tooltip.borrow().as_ref(),
            Some(ActiveTooltip::Visible { .. })
        ));

        test_app
            .update_window(any_window, |_, window, cx| {
                window.dispatch_event(
                    MouseMoveEvent {
                        position: point(px(75.), px(75.)),
                        modifiers: Default::default(),
                        pressed_button: None,
                    }
                    .to_platform_input(),
                    cx,
                );
            })
            .unwrap();

        assert!(active_tooltip.borrow().is_none());
    }

    struct MouseDownOutOwner {
        mouse_down_out_count: Rc<RefCell<usize>>,
    }

    impl Render for MouseDownOutOwner {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mouse_down_out_count = self.mouse_down_out_count.clone();
            div()
                .size_full()
                .child(div().id("target").w(px(50.)).h(px(50.)).on_mouse_down_out(
                    move |_, _, _| {
                        *mouse_down_out_count.borrow_mut() += 1;
                    },
                ))
        }
    }

    #[test]
    fn mouse_down_out_is_suppressed_while_window_prompt_is_active() {
        let mut test_app = TestAppContext::single();
        let mouse_down_out_count = Rc::new(RefCell::new(0));
        let window = test_app.add_window({
            let mouse_down_out_count = mouse_down_out_count.clone();
            move |_, _| MouseDownOutOwner {
                mouse_down_out_count,
            }
        });
        let any_window: AnyWindowHandle = window.into();

        fn dispatch_mouse_down_outside_target(
            test_app: &mut TestAppContext,
            any_window: AnyWindowHandle,
        ) {
            test_app
                .update_window(any_window, |_, window, cx| {
                    window.dispatch_event(
                        MouseDownEvent {
                            position: point(px(75.), px(75.)),
                            button: MouseButton::Left,
                            modifiers: Default::default(),
                            click_count: 1,
                            first_mouse: false,
                        }
                        .to_platform_input(),
                        cx,
                    );
                })
                .unwrap();
        }

        test_app
            .update_window(any_window, |_, window, cx| {
                window.draw(cx).clear(cx);
            })
            .unwrap();

        dispatch_mouse_down_outside_target(&mut test_app, any_window);
        assert_eq!(
            *mouse_down_out_count.borrow(),
            1,
            "mouse down outside the element should fire mouse-down-out listeners"
        );

        test_app
            .update_window(any_window, |_, window, cx| {
                cx.set_prompt_builder(crate::fallback_prompt_renderer);
                let _receiver =
                    window.prompt(crate::PromptLevel::Warning, "message", None, &["Ok"], cx);
                assert!(window.has_active_prompt());
                window.draw(cx).clear(cx);
            })
            .unwrap();

        dispatch_mouse_down_outside_target(&mut test_app, any_window);
        assert_eq!(
            *mouse_down_out_count.borrow(),
            1,
            "mouse down over an active prompt should not fire mouse-down-out listeners"
        );
    }

    #[test]
    fn frame_input_accessibility_metadata_survives_refinement() {
        let mut element = div()
            .id("frame-input")
            .aria_description("Crop width in pixels")
            .aria_value("640")
            .aria_disabled(true)
            .aria_invalid(true);
        let mut node = accesskit::Node::new(accesskit::Role::TextInput);
        element.interactivity().write_a11y_info(&mut node);
        assert_eq!(node.description(), Some("Crop width in pixels"));
        assert_eq!(node.value(), Some("640"));
        assert!(node.is_disabled());
        assert_eq!(node.invalid(), Some(accesskit::Invalid::True));
        element = element.aria_invalid(false).aria_disabled(false);
        let mut node = accesskit::Node::new(accesskit::Role::TextInput);
        element.interactivity().write_a11y_info(&mut node);
        assert_eq!(node.invalid(), None);
        assert!(!node.is_disabled());
    }

    #[test]
    fn test_accessibility_id_builder_writes_author_id() {
        let mut element = div()
            .id("buffer-font-size")
            .accessibility_id("settings.buffer-font-size");
        let mut node = accesskit::Node::new(accesskit::Role::SpinButton);

        element.interactivity().write_a11y_info(&mut node);

        assert_eq!(node.author_id(), Some("settings.buffer-font-size"));
    }

    #[test]
    fn test_write_a11y_info_string_and_numeric_properties() {
        let mut interactivity = Interactivity::default();
        interactivity.aria.author_id = Some("settings.buffer-font-size".into());
        interactivity.aria.label = Some("Buffer Font Size".into());
        interactivity.aria.value = Some("15".into());
        interactivity.aria.placeholder = Some("Search".into());
        interactivity.aria.numeric_value = Some(15.0);
        interactivity.aria.min_numeric_value = Some(6.0);
        interactivity.aria.max_numeric_value = Some(72.0);
        interactivity.aria.numeric_value_step = Some(1.0);

        let mut node = accesskit::Node::new(accesskit::Role::SpinButton);
        interactivity.write_a11y_info(&mut node);

        assert_eq!(node.author_id(), Some("settings.buffer-font-size"));
        assert_eq!(node.label(), Some("Buffer Font Size"));
        assert_eq!(node.value(), Some("15"));
        assert_eq!(node.placeholder(), Some("Search"));
        assert_eq!(node.numeric_value(), Some(15.0));
        assert_eq!(node.min_numeric_value(), Some(6.0));
        assert_eq!(node.max_numeric_value(), Some(72.0));
        assert_eq!(node.numeric_value_step(), Some(1.0));
    }

    #[test]
    fn test_write_a11y_info_disabled_modal_and_live_region() {
        let mut interactivity = Interactivity::default();
        interactivity.aria.disabled = Some(true);
        interactivity.aria.modal = Some(true);
        interactivity.aria.live = Some(accesskit::Live::Assertive);
        interactivity.aria.live_atomic = Some(true);

        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        interactivity.write_a11y_info(&mut node);

        assert!(node.is_disabled());
        assert!(node.is_modal());
        assert_eq!(node.live(), Some(accesskit::Live::Assertive));
        assert!(node.is_live_atomic());
    }

    #[test]
    fn test_write_a11y_info_disabled_and_modal_false_clear_the_flags() {
        // `Some(false)` must explicitly clear a flag-style AccessKit property,
        // not merely leave it at its (already-cleared) default: a caller that
        // re-renders a node from a previous state relies on `Some(false)`
        // overriding a flag that a prior frame set to `true`.
        let mut interactivity = Interactivity::default();
        interactivity.aria.disabled = Some(false);
        interactivity.aria.modal = Some(false);
        interactivity.aria.live_atomic = Some(false);

        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_disabled();
        node.set_modal();
        node.set_live_atomic();

        interactivity.write_a11y_info(&mut node);

        assert!(!node.is_disabled());
        assert!(!node.is_modal());
        assert!(!node.is_live_atomic());
    }

    struct DisabledCallbackProbe {
        disabled: bool,
        aria_disabled: bool,
        callbacks: [usize; 4],
        a11y_callbacks: usize,
        prepaints: usize,
    }

    impl Render for DisabledCallbackProbe {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let probe = cx.weak_entity();

            div()
                .id("disabled-callback-probe")
                .role(accesskit::Role::Button)
                .aria_label("Disabled callback probe")
                .size(px(100.))
                .focusable()
                .disabled(self.disabled)
                .aria_disabled(self.aria_disabled)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|view, _event, _window, _cx| view.callbacks[0] += 1),
                )
                .on_click(cx.listener(|view, _event, _window, _cx| view.callbacks[1] += 1))
                .on_hover(cx.listener(|view, _hovered, _window, _cx| view.callbacks[2] += 1))
                .on_key_down(cx.listener(|view, _event, _window, _cx| view.callbacks[3] += 1))
                .on_a11y_action(accesskit::Action::Increment, move |_data, _window, cx| {
                    probe
                        .update(cx, |probe, _cx| probe.a11y_callbacks += 1)
                        .unwrap();
                })
                .on_prepaint(cx.listener(|view, _info, _window, _cx| view.prepaints += 1))
        }
    }

    #[gpui::test]
    fn disabled_callbacks_are_independent_of_aria_disabled(cx: &mut TestAppContext) {
        for (disabled, aria_disabled) in
            [(false, false), (false, true), (true, false), (true, true)]
        {
            let (view, cx) = cx.add_window_view(move |_window, _cx| DisabledCallbackProbe {
                disabled,
                aria_disabled,
                callbacks: [0; 4],
                a11y_callbacks: 0,
                prepaints: 0,
            });

            cx.update(|window, cx| {
                window.set_a11y_forced(true);
                window.draw(cx).clear(cx);
            });
            cx.simulate_mouse_move(point(px(150.), px(150.)), None, Default::default());
            view.update(cx, |view, _cx| view.callbacks = [0; 4]);
            cx.simulate_mouse_move(point(px(10.), px(10.)), None, Default::default());
            cx.simulate_click(point(px(10.), px(10.)), Default::default());
            cx.simulate_keystrokes("enter");

            cx.update(|window, cx| {
                let probe = view.read(cx);
                assert_eq!(
                    probe.callbacks.map(|count| count == 0),
                    [disabled; 4],
                    "disabled={disabled}, aria_disabled={aria_disabled}",
                );
                assert!(probe.prepaints > 0);

                let (_node_id, node) = window
                    .a11y_tree()
                    .unwrap()
                    .nodes
                    .iter()
                    .find(|(_id, node)| node.label() == Some("Disabled callback probe"))
                    .unwrap();

                assert_eq!(node.is_disabled(), aria_disabled);
                assert_eq!(node.supports_action(accesskit::Action::Click), !disabled);
                assert_eq!(
                    node.supports_action(accesskit::Action::Increment),
                    !disabled
                );

                #[cfg(not(target_family = "wasm"))]
                {
                    window.perform_a11y_action(*_node_id, accesskit::Action::Increment, None, cx);
                    assert_eq!(view.read(cx).a11y_callbacks, usize::from(!disabled));
                }
            });
        }
    }

    /// Two focusable, clickable elements ("a" and "b") used to exercise the
    /// Enter/Space -> synthesized click press/release pairing.
    struct KeyboardActivationTest {
        focus_a: FocusHandle,
        focus_b: FocusHandle,
        clicks: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Render for KeyboardActivationTest {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let clicks_a = self.clicks.clone();
            let clicks_b = self.clicks.clone();
            div()
                .size_full()
                .child(
                    div()
                        .id("a")
                        .w(px(50.))
                        .h(px(50.))
                        .track_focus(&self.focus_a)
                        .on_click(move |_, _, _| clicks_a.borrow_mut().push("a")),
                )
                .child(
                    div()
                        .id("b")
                        .w(px(50.))
                        .h(px(50.))
                        .track_focus(&self.focus_b)
                        .on_click(move |_, _, _| clicks_b.borrow_mut().push("b")),
                )
        }
    }

    fn setup_keyboard_activation_test() -> (
        TestAppContext,
        AnyWindowHandle,
        Rc<RefCell<Vec<&'static str>>>,
        FocusHandle,
        FocusHandle,
    ) {
        let mut cx = TestAppContext::single();
        let (focus_a, focus_b) = cx.update(|cx| (cx.focus_handle(), cx.focus_handle()));
        let clicks: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let focus_a = focus_a.clone();
            let focus_b = focus_b.clone();
            let clicks = clicks.clone();
            move |_, _| KeyboardActivationTest {
                focus_a,
                focus_b,
                clicks,
            }
        });
        (cx, window.into(), clicks, focus_a, focus_b)
    }

    /// Move focus to `handle`, flush effects, then paint so the newly focused
    /// element registers its key handlers for the next dispatched event.
    fn focus_and_draw(cx: &mut TestAppContext, window: AnyWindowHandle, handle: &FocusHandle) {
        cx.update_window(window, |_, window, cx| window.focus(handle, cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.draw(cx).clear(cx);
        })
        .unwrap();
    }

    fn key_down(cx: &mut TestAppContext, window: AnyWindowHandle, key: &str) {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.update_window(window, |_, window, cx| {
            window.dispatch_event(
                KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input: false,
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
    }

    fn key_up(cx: &mut TestAppContext, window: AnyWindowHandle, key: &str) {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.update_window(window, |_, window, cx| {
            window.dispatch_event(KeyUpEvent { keystroke }.to_platform_input(), cx);
        })
        .unwrap();
    }

    /// Pressing and releasing Enter on the same focused element fires a click.
    #[test]
    fn keyboard_activation_fires_click_on_same_element() {
        let (mut cx, window, clicks, focus_a, _focus_b) = setup_keyboard_activation_test();

        focus_and_draw(&mut cx, window, &focus_a);
        key_down(&mut cx, window, "enter");
        key_up(&mut cx, window, "enter");

        assert_eq!(*clicks.borrow(), vec!["a"]);
    }

    /// A key-down whose key-up lands on a *different* element (because focus
    /// moved in between) must not leak a synthesized click onto the newly
    /// focused element. This is the core regression: previously the key-up
    /// handler fired unconditionally on whatever was focused at key-up time.
    #[test]
    fn keyboard_activation_does_not_leak_across_focus_change() {
        let (mut cx, window, clicks, focus_a, focus_b) = setup_keyboard_activation_test();

        // Enter pressed while "a" is focused...
        focus_and_draw(&mut cx, window, &focus_a);
        key_down(&mut cx, window, "enter");

        // ...focus moves to "b" before the release (as a confirm action would)...
        focus_and_draw(&mut cx, window, &focus_b);
        key_up(&mut cx, window, "enter");

        // ...so neither element is clicked: "a" never saw the up, and "b"
        // never saw the down.
        assert!(clicks.borrow().is_empty(), "clicks: {:?}", clicks.borrow());
    }

    /// A keydown whose flag is left pending because focus moved away before
    /// the keyup must not fire a click when focus later *returns* to the same
    /// element (the menu trigger reopening case). The stamped focus generation
    /// no longer matches, so the stale pending state is ignored.
    #[test]
    fn keyboard_activation_does_not_leak_when_focus_returns() {
        let (mut cx, window, clicks, focus_a, focus_b) = setup_keyboard_activation_test();

        // Enter pressed on "a"...
        focus_and_draw(&mut cx, window, &focus_a);
        key_down(&mut cx, window, "enter");

        // ...focus leaves "a" before its keyup (so the pending state is never
        // consumed), then comes back to "a"...
        focus_and_draw(&mut cx, window, &focus_b);
        focus_and_draw(&mut cx, window, &focus_a);
        key_up(&mut cx, window, "enter");

        // ...and the now-stale pending keydown must not fire a click.
        assert!(clicks.borrow().is_empty(), "clicks: {:?}", clicks.borrow());
    }

    /// A non-activation key *released* during the press must cancel the pending
    /// activation. For the sequence escape-down, space-down, escape-up,
    /// space-up the space forms a clean down/up pair, but the intervening
    /// escape-up means this isn't a plain space activation, so no click fires.
    #[test]
    fn keyboard_activation_cleared_by_intervening_key_release() {
        let (mut cx, window, clicks, focus_a, _focus_b) = setup_keyboard_activation_test();

        focus_and_draw(&mut cx, window, &focus_a);
        key_down(&mut cx, window, "escape");
        key_down(&mut cx, window, "space");
        key_up(&mut cx, window, "escape");
        key_up(&mut cx, window, "space");

        assert!(clicks.borrow().is_empty(), "clicks: {:?}", clicks.borrow());
    }

    /// The flag is a single activation marker, not keyed by which activation
    /// key was used, so a Space down paired with an Enter up on the same
    /// element still fires a click.
    #[test]
    fn keyboard_activation_does_not_distinguish_space_and_enter() {
        let (mut cx, window, clicks, focus_a, _focus_b) = setup_keyboard_activation_test();

        focus_and_draw(&mut cx, window, &focus_a);
        key_down(&mut cx, window, "space");
        key_up(&mut cx, window, "enter");

        assert_eq!(*clicks.borrow(), vec!["a"]);
    }

    /// A non-activation key pressed between the activation down and up clears
    /// the pending flag, suppressing the click.
    #[test]
    fn keyboard_activation_cleared_by_intervening_keydown() {
        let (mut cx, window, clicks, focus_a, _focus_b) = setup_keyboard_activation_test();

        focus_and_draw(&mut cx, window, &focus_a);
        key_down(&mut cx, window, "enter");
        key_down(&mut cx, window, "a");
        key_up(&mut cx, window, "enter");

        assert!(clicks.borrow().is_empty(), "clicks: {:?}", clicks.borrow());
    }

    /// A modified Enter (e.g. cmd-enter) is not treated as an activation key,
    /// so it neither sets the pending flag nor fires a click on release.
    #[test]
    fn keyboard_activation_ignores_modified_keys() {
        let (mut cx, window, clicks, focus_a, _focus_b) = setup_keyboard_activation_test();

        focus_and_draw(&mut cx, window, &focus_a);
        key_down(&mut cx, window, "cmd-enter");
        key_up(&mut cx, window, "cmd-enter");

        assert!(clicks.borrow().is_empty(), "clicks: {:?}", clicks.borrow());
    }

    /// Two sibling tab groups, each a focusable container that is *not* itself a
    /// tab stop and holds a single tab stop. Mirrors how the title bar and
    /// status bar expose their controls as ARIA toolbars.
    struct TabGroupFocus {
        group_a: FocusHandle,
        item_a: FocusHandle,
        group_b: FocusHandle,
        item_b: FocusHandle,
    }

    impl Render for TabGroupFocus {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            fn group(container: &FocusHandle, item: &FocusHandle) -> Div {
                div()
                    .track_focus(container)
                    .tab_group()
                    .child(div().track_focus(item))
            }
            div()
                .child(group(&self.group_a, &self.item_a))
                .child(group(&self.group_b, &self.item_b))
        }
    }

    /// Focusing a tab-group container and pressing Tab (`focus_next`) must move
    /// focus to the first tab stop *inside that container*, as documented on
    /// [`InteractiveElement::tab_stop`].
    #[test]
    fn focus_next_from_tab_group_container_enters_that_group() {
        let mut cx = TestAppContext::single();
        let (group_a, item_a, group_b, item_b) = cx.update(|cx| {
            (
                cx.focus_handle(),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle(),
                cx.focus_handle().tab_stop(true),
            )
        });
        let window: AnyWindowHandle = cx
            .add_window({
                let (group_a, item_a, group_b, item_b) =
                    (group_a, item_a, group_b.clone(), item_b.clone());
                move |_, _| TabGroupFocus {
                    group_a,
                    item_a,
                    group_b,
                    item_b,
                }
            })
            .into();
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();

        // Focus the *second* group's container, then advance like Tab would.
        let focused = cx
            .update_window(window, |_, window, cx| {
                window.focus(&group_b, cx);
                window.focus_next(cx);
                window.focused(cx).map(|handle| handle.id)
            })
            .unwrap();

        assert_eq!(focused, Some(item_b.id));
    }

    #[gpui::test]
    fn test_fractional_padding_does_not_make_a_fitting_container_scrollable(
        cx: &mut TestAppContext,
    ) {
        struct PaddedContainer {
            scroll_handle: ScrollHandle,
        }

        impl Render for PaddedContainer {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                // 4.25px of padding snaps to 4px in layout, so a 42px child
                // fits the 50px box exactly.
                div().size_full().child(
                    div()
                        .id("container")
                        .h(px(50.))
                        .w(px(100.))
                        .py(px(4.25))
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll_handle)
                        .child(div().w_full().h(px(42.))),
                )
            }
        }

        let scroll_handle = ScrollHandle::new();
        let window: AnyWindowHandle = cx
            .add_window({
                let scroll_handle = scroll_handle.clone();
                move |_, _| PaddedContainer { scroll_handle }
            })
            .into();
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();

        assert_eq!(scroll_handle.max_offset().y, px(0.));
    }

    struct ContentSizedGrid;

    impl Render for ContentSizedGrid {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let widths = [px(100.), px(200.), px(50.)];
            div().size_full().child(
                div()
                    .w_full()
                    .grid()
                    .grid_cols_max_content(widths.len() as u16)
                    .children(widths.into_iter().enumerate().map(|(index, width)| {
                        div()
                            .debug_selector(move || format!("cell-{index}"))
                            .w(width)
                            .h(px(10.))
                    })),
            )
        }
    }

    #[gpui::test]
    fn grid_cols_max_content_sizes_columns_to_their_content(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| ContentSizedGrid);
        cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();

        let mut bounds = |selector: &'static str| {
            cx.update_window(window.into(), |_, window, _| {
                window.rendered_frame.debug_bounds.get(selector).copied()
            })
            .unwrap()
            .unwrap_or_else(|| panic!("{selector} was not rendered"))
        };

        assert_eq!(bounds("cell-0").origin.x, px(0.));
        assert_eq!(bounds("cell-1").origin.x, px(100.));
        assert_eq!(bounds("cell-2").origin.x, px(300.));
    }
}

#[cfg(test)]
#[path = "div/focus_scroll_tests.rs"]
mod focus_scroll_tests;
