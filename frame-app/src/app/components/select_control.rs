//! Controlled select shared by settings, subtitles and crop tools.
//! Keyboard behavior follows @radix-ui/react-select (including its 1 s typeahead).
use super::{
    FRAME_SELECT_MAX_HEIGHT, frame_select_content_height, frame_select_option_with_focus,
    frame_select_options_list, frame_select_popover, frame_select_scrollbar,
    frame_select_target_index, frame_select_trigger_with_focus,
};
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::prelude::*;
use gpui::{Context, FocusHandle, ScrollHandle, Window, div};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(in crate::app) struct FrameSelectItem {
    pub id: String,
    pub label: String,
    pub selected: bool,
    pub enabled: bool,
    pub focus: Option<FocusHandle>,
}

pub(in crate::app) struct FrameSelectProps<'a> {
    pub id: &'a str,
    pub panel_id: &'a str,
    pub list_id: &'a str,
    pub scrollbar_id: &'a str,
    pub label: &'a str,
    pub display: &'a str,
    pub items: Vec<FrameSelectItem>,
    pub enabled: bool,
    pub open: bool,
    pub rendered: bool,
    pub progress: f32,
    pub trigger_variant: ButtonVariant,
    pub trigger_focus: Option<&'a FocusHandle>,
    pub panel_focus: Option<&'a FocusHandle>,
    pub scroll: &'a ScrollHandle,
    pub palette: &'static theme::ThemePalette,
}

#[derive(Default)]
struct SelectInteraction {
    open: bool,
    search: String,
    last_key: Option<Instant>,
}

impl SelectInteraction {
    fn search(
        &mut self,
        key: &str,
        now: Instant,
        items: &[FrameSelectItem],
        current: usize,
    ) -> Option<usize> {
        if self
            .last_key
            .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1))
        {
            self.search.clear();
        }
        self.last_key = Some(now);
        self.search.push_str(key);
        let repeated = self.search.chars().all(|ch| self.search.starts_with(ch));
        let search = if repeated {
            self.search.chars().take(1).collect::<String>()
        } else {
            self.search.clone()
        }
        .to_lowercase();
        (0..items.len())
            .map(|offset| (current + offset) % items.len())
            .find(|index| {
                items[*index].enabled
                    && (*index != current || search.chars().count() > 1)
                    && items[*index].label.to_lowercase().starts_with(&search)
            })
            .filter(|index| *index != current)
    }
}

fn typed_key<'a>(event: &'a gpui::KeyDownEvent, state: &SelectInteraction) -> Option<&'a str> {
    let modifiers = event.keystroke.modifiers;
    if modifiers.control || modifiers.platform || modifiers.alt {
        return None;
    }
    let key = event.keystroke.key.as_str();
    if key == "space"
        && !state.search.is_empty()
        && state
            .last_key
            .is_some_and(|last| last.elapsed() < Duration::from_secs(1))
    {
        return Some(" ");
    }
    (key.chars().count() == 1).then_some(key)
}

#[expect(
    clippy::too_many_lines,
    reason = "A controlled select owns its trigger and popup keyboard/pointer lifecycle in one reusable component."
)]
pub(in crate::app) fn frame_select_control(
    props: FrameSelectProps<'_>,
    on_open: impl Fn(&mut FrameRoot, bool, &mut Window, &mut Context<FrameRoot>) + 'static,
    on_select: impl Fn(&mut FrameRoot, usize, &mut Window, &mut Context<FrameRoot>) + 'static,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let FrameSelectProps {
        id,
        panel_id,
        list_id,
        scrollbar_id,
        label,
        display,
        items,
        enabled,
        open,
        rendered,
        progress,
        trigger_variant,
        trigger_focus,
        panel_focus,
        scroll,
        palette,
    } = props;
    let on_open = Rc::new(on_open);
    let on_select = Rc::new(on_select);
    let trigger_focus = trigger_focus.cloned().unwrap_or_else(|| cx.focus_handle());
    let items = Rc::new(
        items
            .into_iter()
            .map(|mut item| {
                if item.focus.is_none() {
                    item.focus = Some(
                        window
                            .use_keyed_state(format!("{}-focus", item.id), cx, |_, cx| {
                                cx.focus_handle()
                            })
                            .read(cx)
                            .clone(),
                    );
                }
                item
            })
            .collect::<Vec<_>>(),
    );
    let selected = items
        .iter()
        .position(|item| item.selected && item.enabled)
        .or_else(|| items.iter().position(|item| item.enabled))
        .unwrap_or(0);
    let interaction = window.use_keyed_state(format!("{id}-select-interaction"), cx, |_, _| {
        SelectInteraction::default()
    });
    let just_opened = interaction.update(cx, |state, _| {
        let just_opened = open && !state.open;
        if just_opened {
            state.search.clear();
            state.last_key = None;
        }
        state.open = open;
        just_opened
    });
    if just_opened && let Some(focus) = items.get(selected).and_then(|item| item.focus.clone()) {
        scroll.scroll_to_item(selected);
        cx.defer_in(window, move |_, window, cx| focus.focus(window, cx));
    }
    let click_open = on_open.clone();
    let key_open = on_open.clone();
    let key_select = on_select.clone();
    let key_items = items.clone();
    let key_interaction = interaction.clone();
    let anchor = super::PopoverAnchor::default();
    let measured_anchor = anchor.clone();
    let trigger = frame_select_trigger_with_focus(
        id,
        label,
        display,
        trigger_variant,
        enabled,
        open,
        &trigger_focus,
        palette,
        window,
        cx,
    )
    .on_prepaint(move |info, _, _| measured_anchor.measure(info.bounds))
    .on_mouse_down(
        gpui::MouseButton::Left,
        cx.listener(move |root, _, window, cx| {
            cx.stop_propagation();
            if enabled {
                click_open(root, !open, window, cx);
                cx.notify();
            }
        }),
    )
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
            if !enabled {
                return;
            }
            if let Some(key) = typed_key(event, key_interaction.read(cx)) {
                if let Some(index) = key_interaction.update(cx, |state, _| {
                    state.search(key, Instant::now(), &key_items, selected)
                }) {
                    key_select(root, index, window, cx);
                    cx.notify();
                }
                cx.stop_propagation();
            } else if matches!(
                event.keystroke.key.as_str(),
                "enter" | "space" | "up" | "down"
            ) {
                key_open(root, true, window, cx);
                cx.stop_propagation();
                cx.notify();
            }
        }),
    );
    let mut control = div().relative().w_full().child(trigger);
    if !rendered {
        return control;
    }
    let content_height = frame_select_content_height(items.len());
    let mut list = frame_select_options_list(list_id, scroll);
    for (index, item) in items.iter().enumerate() {
        let focus = item.focus.as_ref().unwrap();
        let item_enabled = enabled && item.enabled && open;
        let pointer_focus = focus.clone();
        let click_select = on_select.clone();
        let click_open = on_open.clone();
        let click_trigger = trigger_focus.clone();
        let key_select = on_select.clone();
        let key_open = on_open.clone();
        let key_trigger = trigger_focus.clone();
        let key_items = items.clone();
        let key_interaction = interaction.clone();
        let key_scroll = scroll.clone();
        list = list.child(
            frame_select_option_with_focus(
                item.id.clone(),
                item.label.clone(),
                item.selected,
                item_enabled,
                focus,
                palette,
            )
            .on_mouse_move(cx.listener(move |_, _: &gpui::MouseMoveEvent, window, cx| {
                if item_enabled {
                    pointer_focus.focus(window, cx);
                }
            }))
            .on_click(
                cx.listener(move |root, event: &gpui::ClickEvent, window, cx| {
                    cx.stop_propagation();
                    if item_enabled && !event.is_keyboard() {
                        click_select(root, index, window, cx);
                        click_open(root, false, window, cx);
                        let focus = click_trigger.clone();
                        cx.defer_in(window, move |_, window, cx| focus.focus(window, cx));
                        cx.notify();
                    }
                }),
            )
            .on_key_down(cx.listener(
                move |root, event: &gpui::KeyDownEvent, window, cx| {
                    if !item_enabled {
                        return;
                    }
                    let key = event.keystroke.key.as_str();
                    let target =
                        frame_select_target_index(key_items.len(), Some(index), key, |i| {
                            key_items[i].enabled
                        })
                        .or_else(|| {
                            typed_key(event, key_interaction.read(cx)).and_then(|key| {
                                key_interaction.update(cx, |state, _| {
                                    state.search(key, Instant::now(), &key_items, index)
                                })
                            })
                        });
                    if let Some(target) = target {
                        key_scroll.scroll_to_item(target);
                        key_items[target].focus.as_ref().unwrap().focus(window, cx);
                        cx.stop_propagation();
                    } else if typed_key(event, key_interaction.read(cx)).is_some() {
                        cx.stop_propagation();
                    } else if matches!(key, "enter" | "space") {
                        key_select(root, index, window, cx);
                        key_open(root, false, window, cx);
                        let focus = key_trigger.clone();
                        cx.defer_in(window, move |_, window, cx| focus.focus(window, cx));
                        cx.stop_propagation();
                        cx.notify();
                    }
                },
            )),
        );
    }
    let outside_open = on_open.clone();
    let outside_trigger = trigger_focus.clone();
    let mut menu = frame_select_popover(panel_id, progress, list, palette)
        .on_mouse_down_out(cx.listener(move |root, _, window, cx| {
            if open {
                outside_open(root, false, window, cx);
                outside_trigger.focus(window, cx);
                cx.notify();
            }
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "tab" => cx.stop_propagation(),
                    "escape" if open => {
                        on_open(root, false, window, cx);
                        trigger_focus.focus(window, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }
            }),
        );
    if let Some(focus) = panel_focus {
        menu = menu.track_focus(focus).tab_stop(false);
    }
    if content_height > FRAME_SELECT_MAX_HEIGHT {
        menu = menu.child(frame_select_scrollbar(
            scrollbar_id,
            scroll.clone(),
            content_height,
            palette,
        ));
    }
    control = control.child(
        gpui::deferred(super::frame_floating_popover(
            menu, anchor, progress, 10.0, window,
        ))
        .priority(20),
    );
    control
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typeahead_cycles_repeated_letters_skips_disabled_and_expires() {
        let items: Vec<_> = ["Arial", "Arial Narrow", "Arial Disabled", "Inter", "Roboto"]
            .into_iter()
            .enumerate()
            .map(|(i, label)| FrameSelectItem {
                id: label.into(),
                label: label.into(),
                enabled: i != 2,
                selected: false,
                focus: None,
            })
            .collect();
        let now = Instant::now();
        let mut state = SelectInteraction::default();
        assert_eq!(state.search("a", now, &items, 3), Some(0));
        assert_eq!(state.search("a", now, &items, 0), Some(1));
        assert_eq!(state.search("a", now, &items, 1), Some(0));
        assert_eq!(
            state.search("r", now + Duration::from_secs(2), &items, 0),
            Some(4)
        );
        assert_eq!(
            state.search("o", now + Duration::from_secs(2), &items, 4),
            None
        );
    }
}
