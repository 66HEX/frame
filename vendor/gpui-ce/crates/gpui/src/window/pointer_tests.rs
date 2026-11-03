//! Tests for pointer-based cursors and hover regions.

use super::*;
use crate::{
    Context, Empty, InputEvent, InteractiveElement, MouseDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, canvas, div, px,
};

struct PositionCursorView {
    renders: Rc<Cell<usize>>,
    resolutions: Rc<Cell<usize>>,
}

impl Render for PositionCursorView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        let resolutions = self.resolutions.clone();
        div()
            .relative()
            .size(px(100.))
            .cursor(crate::CursorStyle::Crosshair)
            .cursor_with(move |position, bounds| {
                resolutions.set(resolutions.get() + 1);
                crate::ResizeRegion::new(px(5.))
                    .corner_size(px(20.))
                    .hit_test(position, bounds)
                    .map(crate::CursorStyle::from)
            })
            .child(
                div()
                    .absolute()
                    .left(px(40.))
                    .top(px(40.))
                    .size(px(20.))
                    .occlude(),
            )
    }
}

struct CachedCursorHost(crate::Entity<PositionCursorView>);

impl Render for CachedCursorHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.0
            .clone()
            .cached(crate::StyleRefinement::default().size(px(100.)))
    }
}

#[gpui::test]
fn position_cursors_update_within_one_hitbox_without_rendering(cx: &mut TestAppContext) {
    let renders = Rc::new(Cell::new(0));
    let resolutions = Rc::new(Cell::new(0));
    let window = cx.add_window({
        let renders = renders.clone();
        let resolutions = resolutions.clone();
        move |_, _| PositionCursorView {
            renders,
            resolutions,
        }
    });
    window
        .update(cx, |_, window, cx| {
            window.active.set(true);
            window.hovered.set(true);
            window.simulate_mouse_move(point(px(50.), px(1.)), cx);
        })
        .unwrap();
    assert_eq!(cx.cursor_style(), crate::CursorStyle::ResizeUp);
    let render_count = renders.get();
    let resolution_count = resolutions.get();
    let previous_hit_test = window
        .update(cx, |_, window, _| {
            window
                .mouse_hit_test
                .iter_hovered()
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap();
    window
        .update(cx, |_, window, cx| {
            window.simulate_mouse_move(point(px(1.), px(1.)), cx);
            assert_eq!(
                window
                    .mouse_hit_test
                    .iter_hovered()
                    .copied()
                    .collect::<Vec<_>>(),
                previous_hit_test,
            );
        })
        .unwrap();
    assert_eq!(cx.cursor_style(), crate::CursorStyle::ResizeUpLeft);
    assert!(resolutions.get() > resolution_count);
    assert_eq!(renders.get(), render_count);

    window
        .update(cx, |_, window, cx| {
            window.simulate_mouse_move(point(px(25.), px(25.)), cx);
        })
        .unwrap();
    assert_eq!(cx.cursor_style(), crate::CursorStyle::Crosshair);
    assert_eq!(renders.get(), render_count);

    let resolution_count = resolutions.get();
    window
        .update(cx, |_, window, cx| {
            window.simulate_mouse_move(point(px(50.), px(50.)), cx);
        })
        .unwrap();
    assert_eq!(cx.cursor_style(), crate::CursorStyle::Arrow);
    assert_eq!(
        resolutions.get(),
        resolution_count,
        "occluded resolvers must not run"
    );

    window
        .update(cx, |_, window, cx| {
            window.simulate_mouse_move(point(px(1.), px(1.)), cx);
            let count = resolutions.get();
            window.dispatch_event(crate::MouseExitEvent::default().to_platform_input(), cx);
            assert_eq!(
                resolutions.get(),
                count,
                "exited hitbox resolvers must not run"
            );
        })
        .unwrap();
    assert_eq!(cx.cursor_style(), crate::CursorStyle::Arrow);
}

#[gpui::test]
fn position_cursors_survive_cached_paint_reuse(cx: &mut TestAppContext) {
    let renders = Rc::new(Cell::new(0));
    let resolutions = Rc::new(Cell::new(0));
    let child = cx.new({
        let renders = renders.clone();
        let resolutions = resolutions.clone();
        move |_| PositionCursorView {
            renders,
            resolutions,
        }
    });
    let window = cx.add_window(move |_, _| CachedCursorHost(child));
    let any_window = AnyWindowHandle::from(window);
    cx.update_window(any_window, |_, window, cx| {
        window.active.set(true);
        window.hovered.set(true);
        window.simulate_mouse_move(point(px(50.), px(1.)), cx);
        window.draw(cx).clear(cx);
    })
    .unwrap();
    let render_count = renders.get();
    for (x, y, expected) in [
        (99., 99., CursorStyle::ResizeDownRight),
        (1., 99., CursorStyle::ResizeDownLeft),
        (99., 1., CursorStyle::ResizeUpRight),
        (25., 25., CursorStyle::Crosshair),
    ] {
        window.update(cx, |_, _, cx| cx.notify()).unwrap();
        cx.update_window(any_window, |_, window, cx| {
            window.draw(cx).clear(cx);
            let count = resolutions.get();
            window.simulate_mouse_move(point(px(x), px(y)), cx);
            assert_eq!(
                resolutions.get(),
                count + 1,
                "paint reuse must retain exactly one resolver"
            );
        })
        .unwrap();
        assert_eq!(cx.cursor_style(), expected);
        assert_eq!(
            renders.get(),
            render_count,
            "cached child should not render again"
        );
    }
}

struct WindowCursorView {
    override_before_hitbox: bool,
    fixed_override: Option<CursorStyle>,
    calls: Rc<RefCell<Vec<&'static str>>>,
}

struct NestedCursorView(Rc<RefCell<Vec<&'static str>>>);

impl Render for NestedCursorView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let parent_calls = self.0.clone();
        let child_calls = self.0.clone();
        div()
            .relative()
            .size(px(100.))
            .cursor(CursorStyle::OpenHand)
            .cursor_with(move |position, _| {
                parent_calls.borrow_mut().push("parent");
                (position.x >= px(80.)).then_some(CursorStyle::ResizeRight)
            })
            .child(
                div()
                    .absolute()
                    .left(px(20.))
                    .top(px(20.))
                    .size(px(40.))
                    .cursor_with(move |position, bounds| {
                        child_calls.borrow_mut().push("child");
                        (position.x < bounds.center().x).then_some(CursorStyle::IBeam)
                    }),
            )
    }
}

#[gpui::test]
fn hitbox_cursor_priority_is_lazy_and_declining_children_fall_back_to_ancestors(
    cx: &mut TestAppContext,
) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let calls = calls.clone();
        move |_, _| NestedCursorView(calls)
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.active.set(true);
        window.hovered.set(true);
        window.draw(cx).clear(cx);
    })
    .unwrap();
    for (x, expected, expected_calls) in [
        (25., CursorStyle::IBeam, vec!["child"]),
        (55., CursorStyle::OpenHand, vec!["child", "parent"]),
        (85., CursorStyle::ResizeRight, vec!["parent"]),
        (110., CursorStyle::Arrow, vec![]),
    ] {
        calls.borrow_mut().clear();
        cx.update_window(window.into(), |_, window, cx| {
            window.simulate_mouse_move(point(px(x), px(25.)), cx)
        })
        .unwrap();
        assert_eq!(cx.cursor_style(), expected, "x={x}");
        assert_eq!(*calls.borrow(), expected_calls, "x={x}");
    }
}

impl Render for WindowCursorView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let calls = self.calls.clone();
        let fixed_override = self.fixed_override;
        let override_element = canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                if let Some(style) = fixed_override {
                    window.set_window_cursor_style(style);
                }
                window.set_window_cursor_style_with(move |position| {
                    calls.borrow_mut().push("window");
                    (position.x < px(20.)).then_some(CursorStyle::ClosedHand)
                });
            },
        )
        .size_0();
        let calls = self.calls.clone();
        let target = div()
            .size(px(100.))
            .cursor(CursorStyle::Crosshair)
            .cursor_with(move |position, _| {
                calls.borrow_mut().push("hitbox");
                (position.x < px(80.)).then_some(CursorStyle::PointingHand)
            });
        if self.override_before_hitbox {
            div().child(override_element).child(target)
        } else {
            div().child(target).child(override_element)
        }
    }
}

#[gpui::test]
fn window_cursor_priority_and_fallback_are_independent_of_paint_order(cx: &mut TestAppContext) {
    for override_before_hitbox in [true, false] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let calls = calls.clone();
            move |_, _| WindowCursorView {
                override_before_hitbox,
                fixed_override: Some(CursorStyle::OpenHand),
                calls,
            }
        });
        cx.update_window(window.into(), |_, window, cx| {
            window.active.set(true);
            window.hovered.set(true);
            window.draw(cx).clear(cx);
        })
        .unwrap();
        for (x, expected) in [(10., CursorStyle::ClosedHand), (50., CursorStyle::OpenHand)] {
            calls.borrow_mut().clear();
            window
                .update(cx, |_, window, cx| {
                    window.simulate_mouse_move(point(px(x), px(10.)), cx);
                })
                .unwrap();
            assert_eq!(
                cx.cursor_style(),
                expected,
                "paint order: {override_before_hitbox}"
            );
            assert_eq!(
                *calls.borrow(),
                ["window"],
                "overridden hitbox resolver must not run"
            );
        }

        window
            .update(cx, |view, window, cx| {
                view.fixed_override = None;
                cx.notify();
                window.refresh();
            })
            .unwrap();
        cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        for (x, expected, expected_calls) in [
            (10., CursorStyle::ClosedHand, vec!["window"]),
            (50., CursorStyle::PointingHand, vec!["window", "hitbox"]),
            (90., CursorStyle::Crosshair, vec!["window", "hitbox"]),
            (150., CursorStyle::Arrow, vec!["window"]),
        ] {
            calls.borrow_mut().clear();
            cx.update_window(window.into(), |_, window, cx| {
                window.simulate_mouse_move(point(px(x), px(10.)), cx);
            })
            .unwrap();
            assert_eq!(
                cx.cursor_style(),
                expected,
                "x={x}, paint order: {override_before_hitbox}"
            );
            assert_eq!(*calls.borrow(), expected_calls);
        }
    }
}

struct RegionObserverView {
    renders: Rc<Cell<usize>>,
    changes: Rc<RefCell<Vec<Option<u8>>>>,
}

impl Render for RegionObserverView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        let changes = self.changes.clone();
        div().id("regions").size(px(100.)).on_hover_region(
            |position, bounds| Some(u8::from(position.x >= bounds.center().x)),
            move |value, _, _| {
                changes.borrow_mut().push(*value);
            },
        )
    }
}

struct RegionObserverHost {
    child: crate::Entity<RegionObserverView>,
    occluded: bool,
}

impl Render for RegionObserverHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut root = div().relative().size(px(100.)).child(
            self.child
                .clone()
                .cached(crate::StyleRefinement::default().size(px(100.))),
        );
        if self.occluded {
            root = root.child(div().absolute().top_0().left_0().size_full().occlude());
        }
        root
    }
}

#[gpui::test]
fn cached_region_observers_deduplicate_and_track_stationary_occlusion(cx: &mut TestAppContext) {
    let renders = Rc::new(Cell::new(0));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let child = cx.new({
        let renders = renders.clone();
        let changes = changes.clone();
        move |_| RegionObserverView { renders, changes }
    });
    let window = cx.add_window(move |_, _| RegionObserverHost {
        child,
        occluded: false,
    });
    let any_window = AnyWindowHandle::from(window);
    cx.update_window(any_window, |_, window, cx| {
        window.simulate_mouse_move(point(px(25.), px(25.)), cx);
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(*changes.borrow(), [Some(0)]);
    let render_count = renders.get();
    cx.update_window(any_window, |_, window, cx| {
        window.simulate_mouse_move(point(px(75.), px(25.)), cx);
    })
    .unwrap();
    assert_eq!(*changes.borrow(), [Some(0), Some(1)]);
    assert_eq!(renders.get(), render_count);

    // Reused paint ranges retain one observer and its previous value.
    for _ in 0..3 {
        window.update(cx, |_, _, cx| cx.notify()).unwrap();
        cx.update_window(any_window, |_, window, cx| {
            window.draw(cx).clear(cx);
            window.simulate_mouse_move(point(px(80.), px(25.)), cx);
        })
        .unwrap();
        assert_eq!(*changes.borrow(), [Some(0), Some(1)]);
        assert_eq!(renders.get(), render_count);
    }

    window
        .update(cx, |view, _, cx| {
            view.occluded = true;
            cx.notify();
        })
        .unwrap();
    cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(*changes.borrow(), [Some(0), Some(1), None]);
    assert_eq!(renders.get(), render_count);
    window
        .update(cx, |view, _, cx| {
            view.occluded = false;
            cx.notify();
        })
        .unwrap();
    cx.update_window(any_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(*changes.borrow(), [Some(0), Some(1), None, Some(1)]);
    assert_eq!(renders.get(), render_count);
}

struct CursorLayoutView {
    left: Pixels,
    enabled: bool,
    calls: Rc<RefCell<Vec<(Point<Pixels>, Bounds<Pixels>)>>>,
    regions: Rc<RefCell<Vec<Option<u8>>>>,
}

impl Render for CursorLayoutView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let calls = self.calls.clone();
        let regions = self.regions.clone();
        let mut target = div()
            .id("layout-regions")
            .absolute()
            .left(self.left)
            .top(px(20.))
            .size(px(60.));
        if self.enabled {
            target = target
                .on_hover_region(
                    |position, bounds| Some(u8::from(position.x >= bounds.center().x)),
                    move |value, _, _| regions.borrow_mut().push(*value),
                )
                .cursor_with(move |position, bounds| {
                    calls.borrow_mut().push((position, bounds));
                    Some(if position.x < bounds.center().x {
                        CursorStyle::ResizeLeft
                    } else {
                        CursorStyle::ResizeRight
                    })
                });
        }
        div().relative().size_full().child(target)
    }
}

#[gpui::test]
fn cursor_resolvers_receive_current_window_bounds_and_are_removed_with_registration(
    cx: &mut TestAppContext,
) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let regions = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let calls = calls.clone();
        let regions = regions.clone();
        move |_, _| CursorLayoutView {
            left: px(10.),
            enabled: true,
            calls,
            regions,
        }
    });
    let position = point(px(45.), px(25.));
    cx.update_window(window.into(), |_, window, cx| {
        window.active.set(true);
        window.hovered.set(true);
        window.simulate_mouse_move(position, cx);
    })
    .unwrap();
    assert_eq!(cx.cursor_style(), CursorStyle::ResizeRight);
    assert_eq!(*regions.borrow(), [Some(1)]);
    assert_eq!(
        calls.borrow().last(),
        Some(&(
            position,
            Bounds::new(point(px(10.), px(20.)), size(px(60.), px(60.)))
        ))
    );
    window
        .update(cx, |view, _, cx| {
            view.left = px(20.);
            cx.notify();
        })
        .unwrap();
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(
        cx.cursor_style(),
        CursorStyle::ResizeLeft,
        "layout updates a stationary pointer's cursor"
    );
    assert_eq!(*regions.borrow(), [Some(1), Some(0)]);
    assert_eq!(
        calls.borrow().last(),
        Some(&(
            position,
            Bounds::new(point(px(20.), px(20.)), size(px(60.), px(60.)))
        ))
    );
    for (left, expected_cursor, expected_region) in [
        (100., CursorStyle::Arrow, None),
        (20., CursorStyle::ResizeLeft, Some(0)),
    ] {
        window
            .update(cx, |view, _, cx| {
                view.left = px(left);
                cx.notify();
            })
            .unwrap();
        cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        assert_eq!(cx.cursor_style(), expected_cursor);
        assert_eq!(regions.borrow().last(), Some(&expected_region));
    }
    assert_eq!(*regions.borrow(), [Some(1), Some(0), None, Some(0)]);
    window
        .update(cx, |view, _, cx| {
            view.enabled = false;
            cx.notify();
        })
        .unwrap();
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(cx.cursor_style(), CursorStyle::Arrow);
    let count = calls.borrow().len();
    cx.update_window(window.into(), |_, window, cx| {
        window.simulate_mouse_move(position, cx)
    })
    .unwrap();
    assert_eq!(
        calls.borrow().len(),
        count,
        "removed resolvers must not run on future input"
    );
}

struct PropagationView {
    consume_input: bool,
    flags: Rc<RefCell<Vec<(bool, bool)>>>,
    hover: Rc<RefCell<Vec<bool>>>,
}

impl Render for PropagationView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let consume_input = self.consume_input;
        let flags = self.flags.clone();
        let hover = self.hover.clone();
        let target = div()
            .id("propagation-regions")
            .absolute()
            .left(px(20.))
            .top(px(20.))
            .size(px(60.))
            .on_mouse_move(move |_, window, cx| {
                if consume_input {
                    cx.stop_propagation();
                    window.prevent_default();
                }
            })
            .on_hover_region(
                |_, _| Some(()),
                |_, window, cx| {
                    cx.stop_propagation();
                    window.prevent_default();
                },
            )
            .on_hover_region(
                |_, _| Some(1u8),
                move |_, window, cx| {
                    flags
                        .borrow_mut()
                        .push((cx.propagate_event, window.default_prevented()));
                },
            )
            .on_hover(move |value, _, _| hover.borrow_mut().push(*value));
        div().relative().size_full().child(target)
    }
}

#[gpui::test]
fn hover_observations_survive_consumed_input_and_isolate_each_callbacks_dispatch_flags(
    cx: &mut TestAppContext,
) {
    for consume_input in [false, true] {
        let flags = Rc::new(RefCell::new(Vec::new()));
        let hover = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_window({
            let flags = flags.clone();
            let hover = hover.clone();
            move |_, _| PropagationView {
                consume_input,
                flags,
                hover,
            }
        });
        cx.update_window(window.into(), |_, window, cx| {
            window.simulate_mouse_move(point(px(25.), px(25.)), cx);
            assert_eq!(
                (cx.propagate_event, window.default_prevented()),
                (!consume_input, consume_input)
            );
        })
        .unwrap();
        assert_eq!(*flags.borrow(), [(!consume_input, consume_input)]);
        assert_eq!(*hover.borrow(), [true]);
        cx.update_window(window.into(), |_, window, cx| {
            window.dispatch_event(crate::MouseExitEvent::default().to_platform_input(), cx);
        })
        .unwrap();
        assert_eq!(*hover.borrow(), [true, false]);
        assert_eq!(
            *flags.borrow(),
            [(!consume_input, consume_input), (true, false)]
        );
    }
}

// Region values need equality, not cloning, across reentrant callbacks.
#[derive(Debug, PartialEq)]
struct RegionToken(u8);

struct ReentrantRegionView {
    changes: Rc<RefCell<Vec<Option<u8>>>>,
    snapshots: Rc<RefCell<Vec<u8>>>,
}

impl Render for ReentrantRegionView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let changes = self.changes.clone();
        let snapshots = self.snapshots.clone();
        div()
            .id("reentrant-regions")
            .ml(px(20.))
            .size(px(100.))
            .on_hover_region(
                |position, bounds| Some(RegionToken(u8::from(position.x >= bounds.center().x))),
                move |value, window, cx| {
                    changes.borrow_mut().push(value.as_ref().map(|v| v.0));
                    if let Some(RegionToken(1)) = value {
                        window.draw(cx).clear(cx);
                        window.simulate_mouse_move(point(px(25.), px(25.)), cx);
                        snapshots.borrow_mut().push(value.as_ref().unwrap().0);
                    }
                },
            )
    }
}

#[gpui::test]
fn hover_regions_commit_state_before_reentrant_draw_and_input(cx: &mut TestAppContext) {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let snapshots = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let changes = changes.clone();
        let snapshots = snapshots.clone();
        move |_, _| ReentrantRegionView { changes, snapshots }
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.simulate_mouse_move(point(px(25.), px(25.)), cx);
        window.simulate_mouse_move(point(px(95.), px(25.)), cx);
        window.simulate_mouse_move(point(px(25.), px(25.)), cx);
    })
    .unwrap();
    assert_eq!(*changes.borrow(), [Some(0), Some(1), Some(0)]);
    assert_eq!(
        *snapshots.borrow(),
        [1],
        "nested input cannot invalidate the outer callback's value"
    );
}

#[gpui::test]
fn hover_regions_clear_during_drag_and_restore_on_release(cx: &mut TestAppContext) {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let changes = changes.clone();
        move |_, _| RegionObserverView {
            renders: Rc::new(Cell::new(0)),
            changes,
        }
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.simulate_mouse_move(point(px(25.), px(25.)), cx);
        let drag_view = cx.new(|_| Empty);
        cx.start_drag(crate::AnyDrag::new((), drag_view));
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(*changes.borrow(), [Some(0), None]);
    cx.update_window(window.into(), |_, window, cx| {
        window.simulate_mouse_move(point(px(75.), px(25.)), cx);
        window.draw(cx).clear(cx);
        window.dispatch_event(
            crate::MouseUpEvent {
                position: point(px(75.), px(25.)),
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
        assert!(!cx.has_active_drag());
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(*changes.borrow(), [Some(0), None, Some(1)]);
}

struct PressedRegionView {
    regions: Rc<RefCell<Vec<Option<u8>>>>,
    hover: Rc<RefCell<Vec<bool>>>,
    clicks: Rc<Cell<usize>>,
}

impl Render for PressedRegionView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let regions = self.regions.clone();
        let hover = self.hover.clone();
        let clicks = self.clicks.clone();
        div()
            .id("pressed-regions")
            .size(px(100.))
            .on_hover_region(
                |position, bounds| Some(u8::from(position.x >= bounds.center().x)),
                move |value, _, _| regions.borrow_mut().push(*value),
            )
            .on_hover(move |value, _, _| hover.borrow_mut().push(*value))
            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
    }
}

#[gpui::test]
fn hover_regions_preserve_stationary_press_suppress_pressed_moves_and_resume_on_release(
    cx: &mut TestAppContext,
) {
    let regions = Rc::new(RefCell::new(Vec::new()));
    let hover = Rc::new(RefCell::new(Vec::new()));
    let clicks = Rc::new(Cell::new(0));
    let window = cx.add_window({
        let regions = regions.clone();
        let hover = hover.clone();
        let clicks = clicks.clone();
        move |_, _| PressedRegionView {
            regions,
            hover,
            clicks,
        }
    });
    cx.update_window(window.into(), |_, window, cx| {
        window.simulate_mouse_move(point(px(25.), px(25.)), cx);
        window.dispatch_event(
            MouseDownEvent {
                position: point(px(25.), px(25.)),
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(*regions.borrow(), [Some(0)]);
    assert_eq!(*hover.borrow(), [true]);
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            MouseMoveEvent {
                position: point(px(75.), px(25.)),
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(*regions.borrow(), [Some(0), None]);
    assert_eq!(*hover.borrow(), [true, false]);
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            crate::MouseUpEvent {
                position: point(px(75.), px(25.)),
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(*regions.borrow(), [Some(0), None, Some(1)]);
    assert_eq!(*hover.borrow(), [true, false, true]);
    assert_eq!(
        clicks.get(),
        1,
        "region observers must preserve normal click dispatch"
    );
}
