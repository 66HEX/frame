use crate::app::components::{FrameIconButtonSize, FrameIconButtonVariant, frame_icon_button};
use crate::app::components::{FrameStyle, FrameTooltip};
use crate::app::layout::{
    PREVIEW_TOOLBAR_BUTTON_SIZE, PREVIEW_TOOLBAR_ICON_SIZE, PREVIEW_TOOLBAR_OFFSET,
};
use crate::app::preview::overlay::opacity::preview_overlay_opacity_slider;
use crate::app::preview::panel::{PreviewEditToolbarFocus, PreviewShellState};
use crate::app::preview::preview_visual_controls_enabled;
use crate::app::preview::toolbar::{
    preview_toolbar_vertical_separator, preview_toolbar_width, preview_zoom_toolbar_width,
};
use crate::app::primitives::{ButtonVariant, button_colors, color, icon_svg};
use crate::app::root::FrameRoot;
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, Window, div};

pub(in crate::app) fn preview_overlay_controls(
    state: &PreviewShellState,
    focuses: PreviewEditToolbarFocus<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<gpui::Div> {
    let palette = state.palette;
    let overlay = state.overlay.overlay.as_ref()?;
    if !state.overlay.overlay_mode {
        return None;
    }
    let enabled = preview_visual_controls_enabled(state);

    let image_controls = div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .child(
            preview_overlay_icon_button_with_focus(
                "replace",
                assets::ICON_FILE_IMAGE,
                "Replace overlay image",
                ButtonVariant::Ghost,
                enabled,
                focuses.first,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                root.prompt_selected_overlay_image(window, cx);
            })),
        )
        .child(preview_toolbar_vertical_separator(palette))
        .child(preview_overlay_opacity_slider(
            overlay.opacity,
            enabled,
            palette,
            window,
            cx,
        ));
    let actions = div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .child(
            frame_icon_button(
                "preview-overlay-remove",
                assets::ICON_TRASH,
                "Remove overlay",
                FrameIconButtonVariant::DestructiveGhost,
                enabled,
                FrameIconButtonSize {
                    button: PREVIEW_TOOLBAR_BUTTON_SIZE,
                    icon: PREVIEW_TOOLBAR_ICON_SIZE,
                },
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                if root.remove_selected_overlay() {
                    root.focus_registered_control("preview-tool-overlay", window, cx);
                    cx.notify();
                }
            })),
        )
        .child(
            preview_overlay_icon_button_with_focus(
                "done",
                assets::ICON_CHECK,
                "Done editing overlay",
                ButtonVariant::Primary,
                enabled,
                focuses.last,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                if root.set_selected_overlay_mode(false) {
                    root.focus_registered_control("preview-tool-overlay", window, cx);
                    cx.notify();
                }
            })),
        );

    Some(overlay_toolbar_layout(
        image_controls,
        actions,
        focuses.panel,
        palette,
    ))
}

fn overlay_toolbar_layout(
    image_controls: gpui::Div,
    actions: gpui::Div,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let bar = div()
        .id("preview-overlay-toolbar")
        .debug_selector(|| "preview-overlay-toolbar".into())
        .track_focus(focus)
        .tab_stop(false)
        .flex()
        .flex_wrap()
        .items_center()
        .justify_center()
        .gap_x(theme::ui_rem(theme::metrics::GROUP_GAP))
        .gap_y(theme::ui_rem(theme::metrics::FIELD_GAP))
        .min_w_0()
        .min_h_0()
        .max_w_full()
        .max_h_full()
        .overflow_y_scroll()
        .popover_surface(palette)
        .p(theme::ui_rem(4.0))
        .shadow(theme::shadows::FLOATING.shadows())
        .child(image_controls)
        .child(actions);

    div()
        .absolute()
        .top(theme::ui_rem(16.0))
        .bottom(theme::ui_rem(16.0))
        .left_0()
        .right_0()
        // Match the crop bar: edit controls must not cover transform or zoom hit regions.
        .pl(theme::ui_rem(
            PREVIEW_TOOLBAR_OFFSET + preview_toolbar_width() + 8.0,
        ))
        .pr(theme::ui_rem(
            PREVIEW_TOOLBAR_OFFSET + preview_zoom_toolbar_width() + 8.0,
        ))
        .flex()
        .items_end()
        .justify_center()
        .child(bar)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Overlay toolbar buttons need explicit edit focus handles."
)]
pub(super) fn preview_overlay_icon_button_with_focus(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    variant: ButtonVariant,
    enabled: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    preview_overlay_icon_button_inner(
        id,
        icon,
        label,
        variant,
        enabled,
        Some(focus),
        palette,
        window,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Overlay toolbar buttons need optional explicit focus handles."
)]
pub(super) fn preview_overlay_icon_button_inner(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    variant: ButtonVariant,
    enabled: bool,
    focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let button = crate::app::components::frame_button_base(
        format!("preview-overlay-{id}"),
        label,
        variant,
        false,
        enabled,
        theme::metrics::ControlSize::Default,
        palette,
        window,
        cx,
    )
    .w(theme::ui_rem(PREVIEW_TOOLBAR_BUTTON_SIZE))
    .px_0()
    .child(icon_svg(
        icon,
        PREVIEW_TOOLBAR_ICON_SIZE,
        color(button_colors(variant, false, enabled, palette).foreground),
    ));

    FrameTooltip::new(format!("preview-overlay-{id}"), label)
        .for_control(button, focus, enabled, palette, window, cx)
}
