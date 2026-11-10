use crate::app::components::{FrameSelectItem, FrameSelectProps, frame_select_control};
use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::preview::panel::{
    PreviewCropOptionsProps, PreviewEditToolbarFocus, PreviewShellState,
};
use crate::app::preview::state::CropSelect;
use crate::app::primitives::ButtonVariant;
use crate::app::root::FrameRoot;
use crate::preview::ASPECT_OPTIONS;
use crate::theme;
use gpui::{Context, ParentElement, Styled, Window, div};

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
    let values: Vec<_> = options.iter().map(|(value, _)| *value).collect();
    let items = options
        .into_iter()
        .enumerate()
        .map(|(index, (value, label))| FrameSelectItem {
            id: format!(
                "{}-{value}",
                if kind == CropSelect::Aspect {
                    "crop-ratio"
                } else {
                    "crop-units"
                }
            ),
            label: label.into(),
            selected: Some(index) == selected_index,
            enabled: true,
            focus: option_focuses.get(index).cloned(),
        })
        .collect();
    div()
        .w(theme::ui_rem(width))
        .flex_shrink_0()
        .child(frame_select_control(
            FrameSelectProps {
                id,
                panel_id,
                list_id,
                scrollbar_id: "crop-options-scrollbar",
                label,
                display,
                items,
                enabled: true,
                open,
                rendered: open || !motion_is_hidden(progress),
                progress,
                trigger_variant: ButtonVariant::Outline,
                trigger_focus: Some(focus),
                panel_focus: None,
                scroll: props.scroll,
                palette: state.palette,
            },
            move |root, open, _, _| {
                root.preview_ui.crop_options.popover = if open { Some(kind) } else { None }
            },
            move |root, index, _, _| {
                let value = values[index];
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
            },
            window,
            cx,
        ))
}
