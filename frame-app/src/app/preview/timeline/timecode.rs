use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::layout::PREVIEW_TIMELINE_CONTROL_HEIGHT;
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{Context, FocusHandle, Window, div};

pub(in crate::app) struct PreviewTimecodeFieldSpec<'a> {
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) enabled: bool,
    pub(super) width: f32,
    pub(super) kind: Option<FrameTextInputKind>,
    pub(super) focus: Option<&'a FocusHandle>,
}

pub(in crate::app) fn preview_timecode_field(
    spec: PreviewTimecodeFieldSpec<'_>,
    palette: &'static theme::ThemePalette,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    let PreviewTimecodeFieldSpec {
        label,
        value,
        enabled,
        width,
        kind,
        focus,
    } = spec;
    let field = if let (Some(kind), Some(focus)) = (kind, focus) {
        frame_text_input(
            FrameTextInputSpec {
                id: match kind {
                    FrameTextInputKind::PreviewStartTime => "preview-start-time",
                    FrameTextInputKind::PreviewEndTime => "preview-end-time",
                    _ => "preview-timecode",
                },
                value: &value,
                placeholder: "--:--:--.---",
                disabled: !enabled,
                focus: Some(focus),
                kind,
            },
            palette,
            window,
            cx,
        )
        .font_features(assets::frame_tabular_number_font_features())
        .into_any_element()
    } else {
        div()
            .w_full()
            .h(theme::ui_rem(PREVIEW_TIMELINE_CONTROL_HEIGHT))
            .flex()
            .items_center()
            .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
            .text_color(color(palette.text_primary))
            .font_features(assets::frame_tabular_number_font_features())
            .child(value)
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap(theme::ui_rem(6.0))
        .child(preview_timeline_label(label, palette))
        .child(
            div()
                .w(theme::ui_rem(width))
                .h(theme::ui_rem(PREVIEW_TIMELINE_CONTROL_HEIGHT))
                .child(field),
        )
}

pub(in crate::app) fn preview_timeline_label(
    label: &'static str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .h(theme::ui_rem(12.0))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(palette.text_muted))
        .child(theme::ui_text(label))
}
