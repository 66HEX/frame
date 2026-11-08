use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::preview::crop_options::select::crop_select;
use crate::app::preview::panel::{
    PreviewCropOptionsProps, PreviewEditToolbarFocus, PreviewShellState,
};
use crate::app::preview::state::CropSelect;
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::theme;
use gpui::{Context, InteractiveElement, ParentElement, Styled, Window, div};

pub(in crate::app) fn preview_crop_options(
    state: &PreviewShellState,
    focuses: PreviewEditToolbarFocus<'_>,
    props: PreviewCropOptionsProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    crop_select(CropSelect::Aspect, state, focuses, props, window, cx)
}

pub(in crate::app) fn preview_crop_custom_fields(
    state: &PreviewShellState,
    focuses: PreviewEditToolbarFocus<'_>,
    props: PreviewCropOptionsProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut fields = div()
        .flex()
        .flex_wrap()
        .min_w_0()
        .max_w_full()
        .items_center()
        .gap_2();
    for (label, id, kind, value, focus) in [
        (
            "W",
            "crop-custom-width",
            FrameTextInputKind::CropWidth,
            props.width,
            props.width_focus,
        ),
        (
            "H",
            "crop-custom-height",
            FrameTextInputKind::CropHeight,
            props.height,
            props.height_focus,
        ),
    ] {
        fields = fields.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .text_color(color(state.palette.text_muted))
                        .child(label),
                )
                .child(
                    div().w(theme::ui_rem(68.0)).child(
                        frame_text_input(
                            FrameTextInputSpec {
                                id,
                                value,
                                placeholder: "",
                                disabled: false,
                                focus: Some(focus),
                                kind,
                            },
                            state.palette,
                            window,
                            cx,
                        )
                        .debug_selector(|| id.to_string())
                        .on_key_down(cx.listener(
                            |root, event: &gpui::KeyDownEvent, window, cx| {
                                if event.keystroke.key == "enter" {
                                    if root.apply_selected_crop() {
                                        root.focus_registered_control(
                                            "preview-tool-crop",
                                            window,
                                            cx,
                                        );
                                    }
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                            },
                        )),
                    ),
                ),
        );
    }
    fields.child(crop_select(
        CropSelect::Units,
        state,
        focuses,
        props,
        window,
        cx,
    ))
}
