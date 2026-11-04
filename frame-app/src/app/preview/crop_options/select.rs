use crate::app::components::{
    FRAME_SELECT_MAX_HEIGHT, frame_select_content_height, frame_select_option_with_focus,
    frame_select_options_list, frame_select_popover, frame_select_scrollbar,
    frame_select_target_index, frame_select_trigger_with_focus,
};
use crate::app::motion::{motion_is_hidden, popover_slide_offset, surface_motion};
use crate::app::preview::panel::{
    PreviewCropOptionsProps, PreviewEditToolbarFocus, PreviewShellState,
};
use crate::app::preview::state::CropSelect;
use crate::app::root::FrameRoot;
use crate::preview::ASPECT_OPTIONS;
use crate::theme;
use gpui::{
    ClickEvent, Context, InteractiveElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

#[expect(
    clippy::too_many_lines,
    reason = "The select uses the shared trigger, list, options and scrollbar with crop-specific state and keyboard callbacks."
)]
pub(super) fn crop_select(
    kind: CropSelect,
    state: &PreviewShellState,
    focuses: PreviewEditToolbarFocus<'_>,
    props: PreviewCropOptionsProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let (id, panel_id, list_id, label, width, focus, option_focuses, options, selected) = match kind
    {
        CropSelect::Aspect => {
            let options: Vec<_> = ASPECT_OPTIONS
                .iter()
                .map(|o| (o.id, o.display))
                .chain(std::iter::once(("custom", "Custom")))
                .collect();
            let selected = if props.custom {
                "custom"
            } else {
                state.crop.crop_aspect.as_str()
            };
            (
                "preview-crop-ratio",
                "crop-options-panel",
                "crop-ratio-options",
                "Crop aspect ratio",
                112.0,
                focuses.first,
                props.option_focuses,
                options,
                selected,
            )
        }
        CropSelect::Units => (
            "crop-units",
            "crop-units-panel",
            "crop-unit-options",
            "Crop units",
            96.0,
            props.unit_focus,
            props.unit_option_focuses,
            vec![("pixels", "Pixels"), ("ratio", "Ratio")],
            if props.ratio { "ratio" } else { "pixels" },
        ),
    };
    let selected_index = options.iter().position(|(value, _)| *value == selected);
    let display = selected_index.map_or(selected, |index| options[index].1);
    let open = props.popover == Some(kind);
    let progress = surface_motion(format!("{id}-popover-motion"), open, window, cx);
    let key_focuses = option_focuses.to_vec();
    let trigger_focus = focus.clone();
    let pointer_focus = focus.clone();
    let trigger = frame_select_trigger_with_focus(
        id,
        label,
        display,
        true,
        open,
        focus,
        state.palette,
        window,
        cx,
    )
    .debug_selector(|| id.to_string())
    .on_mouse_down(
        MouseButton::Left,
        cx.listener(move |root, _, window, cx| {
            pointer_focus.focus(window, cx);
            root.preview_ui.crop_options.popover = if open { None } else { Some(kind) };
            if !open {
                root.preview_ui
                    .crop_options
                    .scroll
                    .scroll_to_item(selected_index.unwrap_or(0));
            }
            cx.stop_propagation();
            cx.notify();
        }),
    )
    .on_click(cx.listener(move |root, event: &ClickEvent, _, cx| {
        if event.is_keyboard() {
            root.preview_ui.crop_options.popover = if open { None } else { Some(kind) };
            if !open {
                root.preview_ui
                    .crop_options
                    .scroll
                    .scroll_to_item(selected_index.unwrap_or(0));
            }
            cx.stop_propagation();
            cx.notify();
        }
    }))
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
            if let Some(index) = frame_select_target_index(
                key_focuses.len(),
                selected_index,
                &event.keystroke.key,
                |_| true,
            ) {
                root.preview_ui.crop_options.popover = Some(kind);
                root.preview_ui.crop_options.scroll.scroll_to_item(index);
                let focus = key_focuses[index].clone();
                cx.defer_in(window, move |_, window, cx| focus.focus(window, cx));
                cx.stop_propagation();
                cx.notify();
            } else if event.keystroke.key == "escape" && open {
                root.preview_ui.crop_options.popover = None;
                cx.stop_propagation();
                cx.notify();
            }
        }),
    );
    let mut field = div()
        .relative()
        .w(theme::ui_rem(width))
        .flex_shrink_0()
        .child(trigger);
    if !open && motion_is_hidden(progress) {
        return field;
    }
    let count = options.len();
    let mut list = frame_select_options_list(
        list_id,
        props.scroll,
        frame_select_content_height(count) > FRAME_SELECT_MAX_HEIGHT,
    );
    for (index, (value, label)) in options.into_iter().enumerate() {
        let key_focuses = option_focuses.to_vec();
        let key_scroll = props.scroll.clone();
        let focus = trigger_focus.clone();
        let option_id = format!(
            "{}-{value}",
            if kind == CropSelect::Aspect {
                "crop-ratio"
            } else {
                "crop-units"
            }
        );
        list = list.child(
            frame_select_option_with_focus(
                option_id.clone(),
                label,
                Some(index) == selected_index,
                open,
                &option_focuses[index],
                state.palette,
            )
            .debug_selector(|| option_id)
            .on_key_down(
                cx.listener(move |_, event: &gpui::KeyDownEvent, window, cx| {
                    if let Some(target) = frame_select_target_index(
                        key_focuses.len(),
                        Some(index),
                        &event.keystroke.key,
                        |_| true,
                    ) {
                        key_scroll.scroll_to_item(target);
                        key_focuses[target].focus(window, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }
                }),
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
                if !open || root.preview_ui.crop_options.popover != Some(kind) {
                    return;
                }
                match kind {
                    CropSelect::Aspect if value == "custom" => {
                        root.preview_ui.crop_options.custom = true;
                        root.set_crop_custom_mode(true);
                        root.preview_custom_crop_values();
                    }
                    CropSelect::Aspect => {
                        root.preview_ui.crop_options.custom = false;
                        root.select_preview_crop_aspect(value);
                    }
                    CropSelect::Units => {
                        root.set_crop_custom_mode(value == "ratio");
                        root.preview_custom_crop_values();
                    }
                }
                root.preview_ui.crop_options.popover = None;
                focus.focus(window, cx);
                cx.stop_propagation();
                cx.notify();
            })),
        );
    }
    let mut popover = frame_select_popover(
        panel_id,
        crate::SETTINGS_CONTROL_HEIGHT + 6.0 + popover_slide_offset(progress),
        progress,
        list,
        state.palette,
    )
    .debug_selector(|| panel_id.to_string())
    .on_mouse_down_out(cx.listener(move |root, _, _, cx| {
        if open {
            root.preview_ui.crop_options.popover = None;
            cx.notify();
        }
    }))
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape" {
                root.preview_ui.crop_options.popover = None;
                trigger_focus.focus(window, cx);
                cx.stop_propagation();
                cx.notify();
            }
        }),
    );
    let content_height = frame_select_content_height(count);
    if content_height > FRAME_SELECT_MAX_HEIGHT {
        popover = popover.child(frame_select_scrollbar(
            "crop-options-scrollbar",
            props.scroll.clone(),
            content_height,
            state.palette,
        ));
    }
    field = field.child(gpui::deferred(popover).priority(20));
    field
}
