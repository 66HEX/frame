use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, DispatchPhase, Div, ElementId, Entity, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, MouseButton, MouseMoveEvent, ParentElement,
    Pixels, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement, StyleRefinement,
    Styled, Toggled, Window, canvas, div, prelude::FluentBuilder, px,
};

pub type OnChange = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

type StateRenderer = Box<dyn FnOnce(Stateful<Div>, bool, bool) -> Stateful<Div>>;

/// An unstyled, controlled switch.
///
/// `BaseSwitch` owns pointer, drag, keyboard, focus, and accessibility
/// behavior. Callers own its layout and appearance through GPUI styles and
/// [`with_state`](Self::with_state).
#[derive(
    IntoElement, Styled, ParentElement, StatefulInteractiveElement, derive_setters::Setters,
)]
#[setters(generate = false)]
pub struct BaseSwitch {
    element_id: ElementId,
    base: Stateful<Div>,
    #[style]
    style: StyleRefinement,
    /// Sets the application-controlled checked value.
    #[setters(generate)]
    checked: bool,
    /// Sets whether the switch ignores user interaction.
    #[setters(generate)]
    disabled: bool,
    /// Sets the horizontal distance required to select a value while dragging.
    #[setters(generate)]
    drag_threshold: Pixels,
    #[children]
    children: Vec<AnyElement>,
    on_change: Option<OnChange>,
    /// Sets the name exposed to accessibility clients.
    #[setters(generate, strip_option, into)]
    aria_label: Option<SharedString>,
    state_renderer: Option<StateRenderer>,
}

impl BaseSwitch {
    pub const DRAG_THRESHOLD: Pixels = px(10.);

    pub fn new(element_id: impl Into<ElementId>) -> Self {
        let element_id = element_id.into();

        Self {
            element_id: element_id.clone(),
            base: div().id(element_id),
            style: StyleRefinement::default(),
            checked: false,
            disabled: false,
            drag_threshold: Self::DRAG_THRESHOLD,
            children: Vec::new(),
            on_change: None,
            aria_label: None,
            state_renderer: None,
        }
    }

    /// Handles activation with the next checked value.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));

        self
    }

    /// Builds styles and children using the effective checked and disabled values.
    ///
    /// The checked value previews the pending value during a drag. The
    /// application-controlled value changes only through [`on_change`](Self::on_change).
    pub fn with_state(
        mut self,
        renderer: impl FnOnce(Stateful<Div>, bool, bool) -> Stateful<Div> + 'static,
    ) -> Self {
        self.state_renderer = Some(Box::new(renderer));

        self
    }
}

impl InteractiveElement for BaseSwitch {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for BaseSwitch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.disabled;
        let checked = self.checked;

        let focus_handle = use_focus_handle(self.element_id.clone(), window, cx, None);

        let on_change = if disabled { None } else { self.on_change };

        let drag =
            BaseSwitchDrag::new(&self.element_id, on_change, self.drag_threshold, window, cx);

        let effective_checked = drag.effective_checked(checked, cx);

        let base = self
            .base
            .role(Role::Switch)
            .refine_style(&self.style)
            .aria_toggled(Toggled::from(checked))
            .disabled(disabled)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when(!disabled, |this| this.track_focus(&focus_handle))
            .when(disabled, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation()
                        }),
                )
            })
            .children(self.children);

        let base = match self.state_renderer {
            Some(renderer) => renderer(base, effective_checked, disabled),
            None => base,
        };

        drag.install(base, checked)
    }
}

/// The unstyled thumb rendered by a [`BaseSwitch`].
///
/// Callers provide all visual styles, using [`BaseSwitch::with_state`] to
/// apply the parent switch's effective checked and disabled values.
#[derive(IntoElement, Styled, ParentElement, StatefulInteractiveElement)]
pub struct BaseSwitchThumb {
    base: Stateful<Div>,
    #[style]
    style: StyleRefinement,
    #[children]
    children: Vec<AnyElement>,
}

impl BaseSwitchThumb {
    pub fn new(element_id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(element_id),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl InteractiveElement for BaseSwitchThumb {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for BaseSwitchThumb {
    fn render(self, _window: &mut Window, _app: &mut App) -> impl IntoElement {
        self.base
            .aria_hidden()
            .refine_style(&self.style)
            .children(self.children)
    }
}

struct BaseSwitchDrag {
    state: Entity<DragState>,
    on_change: Option<OnChange>,
    threshold: Pixels,
}

impl BaseSwitchDrag {
    fn new(
        element_id: &ElementId,
        on_change: Option<OnChange>,
        threshold: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let state =
            window.use_keyed_state((element_id.clone(), "state:drag"), cx, |_window, _cx| {
                DragState::default()
            });

        let drag = Self {
            state,
            on_change,
            threshold,
        };

        if drag.on_change.is_none() {
            drag.cancel(cx);
        }

        drag
    }

    fn effective_checked(&self, checked: bool, cx: &App) -> bool {
        self.on_change
            .as_ref()
            .and_then(|_handler| self.state.read(cx).dragged_to)
            .unwrap_or(checked)
    }

    fn cancel(&self, cx: &mut App) {
        if self.state.read(cx).start_x.is_none() {
            return;
        }

        self.state
            .update(cx, |state, _cx| *state = DragState::default());
    }

    fn install<Element>(self, base: Element, checked: bool) -> Element
    where
        Element: ParentElement + StatefulInteractiveElement,
    {
        let Some(on_change) = self.on_change.clone() else {
            return base;
        };

        let on_mouse_up = on_change.clone();
        let drag_state_on_down = self.state.clone();
        let drag_state_on_move = self.state.clone();
        let drag_state_on_up = self.state.clone();
        let threshold = self.threshold;

        base.child(Self::track_mouse_x(drag_state_on_move, threshold))
            .on_mouse_down_all(move |event, phase, hitbox, window, cx| {
                if phase != DispatchPhase::Bubble
                    || event.button != MouseButton::Left
                    || !hitbox.is_hovered(window)
                {
                    return;
                }

                drag_state_on_down.update(cx, |state, _cx| state.start(event.position.x));
            })
            .on_mouse_up_all(move |event, phase, hitbox, window, cx| {
                if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                    return;
                }

                let next = drag_state_on_up.update(cx, |state, cx| {
                    state.start_x?;

                    let next = state.finish(
                        event.position.x,
                        checked,
                        hitbox.bounds.contains(&event.position),
                        threshold,
                    );
                    cx.notify();

                    next
                });

                if let Some(next) = next {
                    on_mouse_up(&next, window, cx);
                }
            })
            .on_click(move |event, window, cx| {
                if matches!(event, ClickEvent::Mouse(_mouse_event)) {
                    return;
                }

                on_change(&!checked, window, cx);
            })
    }

    fn track_mouse_x(state: Entity<DragState>, threshold: Pixels) -> impl IntoElement {
        canvas(
            |_bounds, _window, _app| (),
            move |_bounds, _prepaint, window, _app| {
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
                    if phase != DispatchPhase::Capture || !event.dragging() {
                        return;
                    }

                    state.update(cx, |state, cx| {
                        if !state.update(event.position.x, threshold) {
                            return;
                        }

                        cx.notify();
                    });
                });
            },
        )
        .absolute()
        .inset_0()
    }
}

#[derive(Clone, Copy, Default)]
struct DragState {
    start_x: Option<Pixels>,
    dragged_to: Option<bool>,
}

impl DragState {
    fn start(&mut self, x: Pixels) {
        self.start_x = Some(x);
        self.dragged_to = None;
    }

    fn update(&mut self, x: Pixels, threshold: Pixels) -> bool {
        let Some(start_x) = self.start_x else {
            return false;
        };

        let previous = self.dragged_to;
        let delta = x - start_x;

        if delta > threshold {
            self.dragged_to = Some(true);
        } else if delta < -threshold {
            self.dragged_to = Some(false);
        }

        self.dragged_to != previous
    }

    fn finish(
        &mut self,
        x: Pixels,
        checked: bool,
        released_inside: bool,
        threshold: Pixels,
    ) -> Option<bool> {
        self.update(x, threshold);

        let next = match self.dragged_to {
            Some(next) if next != checked => Some(next),
            Some(_dragged_to) => None,
            None if released_inside => Some(!checked),
            None => None,
        };

        *self = Self::default();

        next
    }
}

fn use_focus_handle(
    base_id: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    focus_handle: Option<FocusHandle>,
) -> FocusHandle {
    focus_handle.unwrap_or_else(|| {
        window
            .use_keyed_state((base_id.into(), "state:focus_handle"), cx, |_window, cx| {
                cx.focus_handle().tab_stop(true)
            })
            .read(cx)
            .clone()
    })
}

#[cfg(test)]
mod tests {
    use super::{BaseSwitch, BaseSwitchThumb};

    use std::sync::{Arc, Mutex};

    use gpui::{
        AppContext, Bounds, Context, Div, Element as _, Entity, InteractiveElement,
        InteractivityPrepaint, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
        MouseButton, ParentElement, Pixels, Render, RenderOnce, StatefulInteractiveElement, Styled,
        TestAppContext, VisualTestContext, Window, accesskit, canvas, div, point,
        prelude::FluentBuilder, px,
    };

    struct SwitchHarness {
        checked: bool,
        disabled: bool,
        drag_threshold: Pixels,
        has_handler: bool,
        accept_changes: bool,
        changes: Vec<bool>,
        parent_clicks: usize,
        switch_bounds: Option<Bounds<Pixels>>,
        thumb_bounds: Option<Bounds<Pixels>>,
    }

    impl Render for SwitchHarness {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let on_thumb_prepaint =
                cx.listener(|this, info: &InteractivityPrepaint, _window, _cx| {
                    this.thumb_bounds = Some(info.bounds);
                });

            div()
                .id("switch-parent")
                .tab_group()
                .size_full()
                .on_click(cx.listener(|this, _event, _window, _app| this.parent_clicks += 1))
                .child(
                    BaseSwitch::new("switch-under-test")
                        .checked(self.checked)
                        .disabled(self.disabled)
                        .drag_threshold(self.drag_threshold)
                        .absolute()
                        .left(px(40.))
                        .top(px(40.))
                        .w(px(100.))
                        .h(px(30.))
                        .on_prepaint(cx.listener(
                            |this, info: &InteractivityPrepaint, _window, _cx| {
                                this.switch_bounds = Some(info.bounds);
                            },
                        ))
                        .when(self.has_handler, |this| {
                            this.on_change(cx.listener(|this, checked, _window, cx| {
                                this.changes.push(*checked);

                                if this.accept_changes {
                                    this.checked = *checked;
                                }

                                cx.notify();
                            }))
                        })
                        .with_state(move |base, checked, disabled| {
                            base.when(checked, |base| base.w(px(120.)))
                                .when(disabled, |base| base.h(px(20.)))
                                .child(
                                    BaseSwitchThumb::new("thumb-under-test")
                                        .w(px(10.))
                                        .h(px(10.))
                                        .on_prepaint(on_thumb_prepaint)
                                        .when(checked, |thumb| thumb.w(px(20.)))
                                        .when(disabled, |thumb| thumb.h(px(5.))),
                                )
                        }),
                )
        }
    }

    fn setup(
        cx: &mut TestAppContext,
        checked: bool,
        disabled: bool,
        accept_changes: bool,
    ) -> (Entity<SwitchHarness>, &mut VisualTestContext) {
        cx.add_window_view(move |_window, _cx| SwitchHarness {
            checked,
            disabled,
            drag_threshold: BaseSwitch::DRAG_THRESHOLD,
            has_handler: true,
            accept_changes,
            changes: Vec::new(),
            parent_clicks: 0,
            switch_bounds: None,
            thumb_bounds: None,
        })
    }

    fn bounds(
        view: &Entity<SwitchHarness>,
        cx: &VisualTestContext,
    ) -> (Bounds<Pixels>, Bounds<Pixels>) {
        cx.read_entity(view, |view, _cx| {
            (
                view.switch_bounds.expect("switch should be rendered"),
                view.thumb_bounds.expect("thumb should be rendered"),
            )
        })
    }

    fn state(view: &Entity<SwitchHarness>, cx: &VisualTestContext) -> (bool, Vec<bool>, usize) {
        cx.read_entity(view, |view, _cx| {
            (view.checked, view.changes.clone(), view.parent_clicks)
        })
    }

    fn activate_key(cx: &mut VisualTestContext, key: &str) {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });

        cx.simulate_event(KeyUpEvent { keystroke });
    }

    #[gpui::test]
    fn pointer_gestures_project_state_and_report_changes_once(cx: &mut TestAppContext) {
        let (view, cx) = setup(cx, false, false, true);
        let initial = bounds(&view, cx).0;
        let inside = point(initial.left() + px(10.), initial.top() + px(10.));

        assert_eq!(initial.size.width, px(100.));
        assert_eq!(bounds(&view, cx).1.size.width, px(10.));

        let at_default_threshold = point(inside.x + BaseSwitch::DRAG_THRESHOLD, inside.y);
        let beyond_default_threshold =
            point(inside.x + BaseSwitch::DRAG_THRESHOLD + px(1.), inside.y);
        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(at_default_threshold, MouseButton::Left, Modifiers::none());
        assert_eq!(bounds(&view, cx).0.size.width, px(100.));
        cx.simulate_mouse_move(
            beyond_default_threshold,
            MouseButton::Left,
            Modifiers::none(),
        );

        assert!(!state(&view, cx).0);
        assert!(state(&view, cx).1.is_empty());
        assert_eq!(bounds(&view, cx).0.size.width, px(120.));
        assert_eq!(bounds(&view, cx).1.size.width, px(20.));

        cx.simulate_mouse_up(
            beyond_default_threshold,
            MouseButton::Left,
            Modifiers::none(),
        );
        assert!(state(&view, cx).0);
        assert_eq!(state(&view, cx).1, vec![true]);

        let outside_left = point(initial.left() - px(30.), inside.y);
        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(outside_left, MouseButton::Left, Modifiers::none());
        assert_eq!(bounds(&view, cx).0.size.width, px(100.));
        assert_eq!(bounds(&view, cx).1.size.width, px(10.));
        cx.simulate_mouse_up(outside_left, MouseButton::Left, Modifiers::none());
        assert_eq!(state(&view, cx).1, vec![true, false]);

        cx.simulate_click(inside, Modifiers::none());
        let small_move = point(inside.x + px(5.), inside.y);
        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(small_move, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(small_move, MouseButton::Left, Modifiers::none());

        let outside_below = point(inside.x, initial.bottom() + px(20.));
        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(outside_below, MouseButton::Left, Modifiers::none());

        assert!(!state(&view, cx).0);
        assert_eq!(state(&view, cx).1, vec![true, false, true, false]);

        view.update(cx, |view, cx| {
            view.drag_threshold = px(40.);
            cx.notify();
        });

        let below_custom_threshold = point(inside.x + px(20.), inside.y);
        let beyond_custom_threshold = point(inside.x + px(41.), inside.y);
        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(below_custom_threshold, MouseButton::Left, Modifiers::none());
        assert_eq!(bounds(&view, cx).0.size.width, px(100.));
        assert_eq!(state(&view, cx).1, vec![true, false, true, false]);

        cx.simulate_mouse_move(
            beyond_custom_threshold,
            MouseButton::Left,
            Modifiers::none(),
        );
        assert_eq!(bounds(&view, cx).0.size.width, px(120.));
        assert_eq!(state(&view, cx).1, vec![true, false, true, false]);

        cx.simulate_mouse_up(
            beyond_custom_threshold,
            MouseButton::Left,
            Modifiers::none(),
        );
        assert!(state(&view, cx).0);
        assert_eq!(state(&view, cx).1, vec![true, false, true, false, true]);
    }

    #[gpui::test]
    fn controlled_state_changes_only_when_rendered_back(cx: &mut TestAppContext) {
        let (view, cx) = setup(cx, false, false, false);
        let switch = bounds(&view, cx).0;
        let inside = point(switch.left() + px(10.), switch.top() + px(10.));

        cx.simulate_click(inside, Modifiers::none());
        cx.simulate_click(inside, Modifiers::none());
        activate_key(cx, "enter");
        activate_key(cx, "space");

        assert!(!state(&view, cx).0);
        assert_eq!(state(&view, cx).1, vec![true, true, true, true]);
        assert_eq!(bounds(&view, cx).0.size.width, px(100.));
    }

    #[gpui::test]
    fn unavailable_switch_is_inert_and_cancels_an_active_drag(cx: &mut TestAppContext) {
        let (view, cx) = setup(cx, false, true, true);
        let switch = bounds(&view, cx).0;
        let inside = point(switch.left() + px(10.), switch.top() + px(10.));
        let outside_right = point(switch.right() + px(50.), inside.y);

        assert_eq!(switch.size.height, px(20.));
        assert_eq!(bounds(&view, cx).1.size.height, px(5.));

        cx.simulate_click(inside, Modifiers::none());
        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(outside_right, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(outside_right, MouseButton::Left, Modifiers::none());
        activate_key(cx, "enter");
        assert_eq!(state(&view, cx), (false, Vec::new(), 0));

        view.update(cx, |view, cx| {
            view.disabled = false;
            cx.notify();
        });

        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(outside_right, MouseButton::Left, Modifiers::none());
        assert_eq!(bounds(&view, cx).0.size.width, px(120.));

        view.update(cx, |view, cx| {
            view.disabled = true;
            cx.notify();
        });

        assert_eq!(bounds(&view, cx).0.size.width, px(100.));
        cx.simulate_mouse_up(outside_right, MouseButton::Left, Modifiers::none());

        assert!(!state(&view, cx).0);
        assert!(state(&view, cx).1.is_empty());

        view.update(cx, |view, cx| {
            view.disabled = false;
            cx.notify();
        });

        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(outside_right, MouseButton::Left, Modifiers::none());
        assert_eq!(bounds(&view, cx).0.size.width, px(120.));

        view.update(cx, |view, cx| {
            view.has_handler = false;
            cx.notify();
        });

        assert_eq!(bounds(&view, cx).0.size.width, px(100.));
        cx.simulate_mouse_up(outside_right, MouseButton::Left, Modifiers::none());
        cx.simulate_click(inside, Modifiers::none());
        activate_key(cx, "enter");

        assert!(!state(&view, cx).0);
        assert!(state(&view, cx).1.is_empty());

        view.update(cx, |view, cx| {
            view.has_handler = true;
            cx.notify();
        });

        cx.simulate_click(inside, Modifiers::none());
        assert!(state(&view, cx).0);
        assert_eq!(state(&view, cx).1, vec![true]);
    }

    #[gpui::test]
    fn accessibility_exposes_the_switch_value_and_action(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<(accesskit::Node, accesskit::Node)>>>;

        struct AccessibilityProbe {
            captured: Captured,
        }

        impl Render for AccessibilityProbe {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let captured = self.captured.clone();

                canvas(
                    move |_bounds, window, cx| {
                        let mut info = |switch: BaseSwitch| {
                            let mut node = accesskit::Node::new(accesskit::Role::Switch);
                            switch
                                .render(window, cx)
                                .into_any_element()
                                .downcast_mut::<Div>()
                                .expect("control should render a div")
                                .write_a11y_info(&mut node);

                            node
                        };

                        let enabled = info(
                            BaseSwitch::new("enabled")
                                .checked(true)
                                .aria_label("Notifications")
                                .on_change(|_checked, _window, _app| {}),
                        );
                        let disabled = info(
                            BaseSwitch::new("disabled")
                                .disabled(true)
                                .aria_label("Notifications")
                                .on_change(|_checked, _window, _app| {}),
                        );
                        *captured.lock().unwrap() = Some((enabled, disabled));
                    },
                    |_bounds, _prepaint, _window, _app| {},
                )
            }
        }

        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_view, cx) = cx.add_window_view(move |_window, _app| AccessibilityProbe { captured });

        cx.update(|window, cx| window.draw(cx).clear(cx));

        let (enabled, disabled) = result.lock().unwrap().take().unwrap();

        assert_eq!(enabled.role(), accesskit::Role::Switch);
        assert_eq!(enabled.label(), Some("Notifications"));
        assert_eq!(enabled.toggled(), Some(accesskit::Toggled::True));
        assert!(enabled.supports_action(accesskit::Action::Click));

        assert_eq!(disabled.role(), accesskit::Role::Switch);
        assert_eq!(disabled.toggled(), Some(accesskit::Toggled::False));
        assert!(disabled.is_disabled());
        assert!(!disabled.supports_action(accesskit::Action::Click));
    }
}
