//! A select with local disclosure state and controlled application values.
//! Every settings field uses the same trigger, menu, typeahead and motion.
use super::{FrameSelectItem, FrameSelectProps, frame_select_control};
use crate::app::{motion, primitives::ButtonVariant, root::FrameRoot};
use crate::theme;
use gpui::{Context, FocusHandle, ScrollHandle, Window};
use std::rc::Rc;

type SelectAction = dyn Fn(&mut FrameRoot, &mut Window, &mut Context<FrameRoot>);

pub(in crate::app) struct FrameOption {
    pub(super) item: FrameSelectItem,
    pub(super) display: String,
    pub(super) action: Rc<SelectAction>,
}

impl FrameOption {
    pub(in crate::app) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        selected: bool,
        enabled: bool,
        action: impl Fn(&mut FrameRoot, &mut Window, &mut Context<FrameRoot>) + 'static,
    ) -> Self {
        let label = label.into();
        Self {
            display: label.clone(),
            item: FrameSelectItem {
                id: id.into(),
                label,
                selected,
                enabled,
                focus: None,
            },
            action: Rc::new(action),
        }
    }

    pub(in crate::app) fn with_caption(
        id: impl Into<String>,
        label: impl Into<String>,
        caption: impl Into<String>,
        selected: bool,
        enabled: bool,
        action: impl Fn(&mut FrameRoot, &mut Window, &mut Context<FrameRoot>) + 'static,
    ) -> Self {
        let label = label.into();
        let caption = caption.into();
        let mut choice = Self::new(id, label, selected, enabled, action);
        if !caption.is_empty() && caption != choice.display {
            choice.item.label = format!("{} · {caption}", choice.display);
        }
        choice
    }
}

struct SelectState {
    open: bool,
    focus: FocusHandle,
    scroll: ScrollHandle,
}

pub(in crate::app) fn frame_option_field(
    id: impl Into<String>,
    label: impl Into<String>,
    choices: Vec<FrameOption>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let id = id.into();
    let label = label.into();
    if !choices.is_empty() && choices.len() <= 3 {
        return super::option_tabs::frame_option_tabs(id, label, choices, palette, window, cx);
    }
    frame_select_field(id, label, choices, palette, window, cx)
}

/// Explicit select for long parameter domains such as codecs and containers,
/// even when capability filtering leaves only a few options.
pub(in crate::app) fn frame_select_field(
    id: impl Into<String>,
    label: impl Into<String>,
    choices: Vec<FrameOption>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let id = id.into();
    let label = label.into();
    let state = window.use_keyed_state(format!("{id}-state"), cx, |_, cx| SelectState {
        open: false,
        focus: cx.focus_handle(),
        scroll: ScrollHandle::new(),
    });
    let enabled = choices.iter().any(|choice| choice.item.enabled);
    let open = enabled && state.read(cx).open;
    let focus = state.read(cx).focus.clone().tab_stop(enabled);
    let scroll = state.read(cx).scroll.clone();
    let progress = motion::surface_motion(format!("{id}-motion"), open, window, cx);
    let display = choices
        .iter()
        .find(|choice| choice.item.selected)
        .map_or_else(|| "Select…".to_owned(), |choice| choice.display.clone());
    let items = choices.iter().map(|choice| choice.item.clone()).collect();
    frame_select_control(
        FrameSelectProps {
            id: &id,
            panel_id: &format!("{id}-options"),
            list_id: &format!("{id}-list"),
            scrollbar_id: &format!("{id}-scrollbar"),
            label: &label,
            display: &display,
            items,
            enabled,
            open,
            rendered: open || !motion::motion_is_hidden(progress),
            progress,
            trigger_variant: if palette.is_light() {
                ButtonVariant::Accent
            } else {
                ButtonVariant::Outline
            },
            trigger_focus: Some(&focus),
            panel_focus: None,
            scroll: &scroll,
            palette,
        },
        move |_, open, _, cx| {
            state.update(cx, |state, cx| {
                state.open = open;
                cx.notify();
            });
        },
        move |root, index, window, cx| {
            if let Some(choice) = choices.get(index).filter(|choice| choice.item.enabled) {
                (choice.action)(root, window, cx);
            }
        },
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Binary fields carry explicit labels, state and the same render context as other controls."
)]
pub(in crate::app) fn frame_boolean_field(
    id: impl Into<String>,
    label: impl Into<String>,
    value: bool,
    enabled: bool,
    labels: [&'static str; 2],
    on_select: impl Fn(&mut FrameRoot, bool, &mut Window, &mut Context<FrameRoot>) + 'static,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let id = id.into();
    let on_select = Rc::new(on_select);
    let choices = [false, true]
        .into_iter()
        .map(|candidate| {
            let on_select = on_select.clone();
            FrameOption::new(
                format!("{id}-{candidate}"),
                labels[usize::from(candidate)],
                candidate == value,
                enabled,
                move |root, window, cx| {
                    if candidate != value {
                        on_select(root, candidate, window, cx);
                    }
                },
            )
        })
        .collect();
    frame_option_field(id, label, choices, palette, window, cx)
}
