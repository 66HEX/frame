use crate::app::accessibility::{
    apply_accessible_button, apply_accessible_button_with_focus, handle_modal_tab_navigation,
};
use crate::app::components::{FrameIconButtonSize, FrameIconButtonVariant, frame_icon_button};
use crate::app::layout::{PREVIEW_TOOLBAR_BUTTON_SIZE, PREVIEW_TOOLBAR_ICON_SIZE};
use crate::app::preview::overlay::opacity::preview_overlay_opacity_slider;
use crate::app::preview::panel::{PreviewEditToolbarFocus, PreviewShellState};
use crate::app::preview::preview_visual_controls_enabled;
use crate::app::preview::toolbar::preview_toolbar_vertical_separator;
use crate::app::primitives::{
    ButtonVariant, animated_button_colors, apply_button_motion, button_colors, color, icon_svg,
};
use crate::app::root::FrameRoot;
use crate::preview::OverlaySizeDirection;
use crate::{assets, theme};
use gpui::prelude::*;
use gpui::{ClickEvent, Context, FocusHandle, Window, div};

#[expect(
    clippy::too_many_lines,
    reason = "Overlay controls are a compact declarative toolbar whose layout is clearer when kept together."
)]
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
    let media = state.media;
    let first_focus = focuses.first.clone();
    let last_focus = focuses.last.clone();

    let bar = div()
        .id("preview-overlay-toolbar")
        .track_focus(focuses.panel)
        .tab_stop(false)
        .flex()
        .items_center()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.surface_elevated))
        .p(theme::ui_rem(4.0))
        .shadow(theme::shadows::FLOATING.shadows())
        .on_key_down(
            cx.listener(move |_root, event: &gpui::KeyDownEvent, window, cx| {
                handle_modal_tab_navigation(event, &first_focus, &last_focus, window, cx);
            }),
        )
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
        .child(
            preview_overlay_icon_button(
                "decrease",
                assets::ICON_MINUS,
                "Decrease overlay size",
                ButtonVariant::Ghost,
                enabled,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                if root.nudge_selected_overlay_size(OverlaySizeDirection::Decrease, media) {
                    cx.notify();
                }
            })),
        )
        .child(
            preview_overlay_icon_button(
                "increase",
                assets::ICON_PLUS,
                "Increase overlay size",
                ButtonVariant::Ghost,
                enabled,
                palette,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                if root.nudge_selected_overlay_size(OverlaySizeDirection::Increase, media) {
                    cx.notify();
                }
            })),
        )
        .child(preview_overlay_opacity_slider(
            overlay.opacity,
            enabled,
            palette,
            cx,
        ))
        .child(preview_toolbar_vertical_separator(palette))
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

    Some(
        div()
            .absolute()
            .bottom(theme::ui_rem(16.0))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(bar),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Overlay buttons keep semantics, state, palette, and render context explicit."
)]
pub(super) fn preview_overlay_icon_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    variant: ButtonVariant,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    preview_overlay_icon_button_inner(id, icon, label, variant, enabled, None, palette, window, cx)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Overlay toolbar buttons need explicit focus handles for focus trapping."
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
    let colors = button_colors(variant, false, enabled, palette);
    let button_id = format!("preview-overlay-{id}");
    let animated = animated_button_colors(button_id.clone(), colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;
    let highlighted = matches!(variant, ButtonVariant::Primary);

    let button = div()
        .id(button_id)
        .w(theme::ui_rem(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .h(theme::ui_rem(PREVIEW_TOOLBAR_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(background)
        .text_color(foreground)
        .opacity(colors.opacity)
        .when(highlighted, |this| {
            this.shadow(theme::shadows::CONTROL.shadows())
        })
        .when(!enabled, gpui::Styled::cursor_not_allowed)
        .when(enabled, |this| {
            this.hover_style(gpui::Styled::cursor_pointer)
                .active_style(move |style| {
                    style
                        .bg(colors.active_fill())
                        .text_color(color(colors.hover_foreground))
                })
        })
        .child(icon_svg(icon, PREVIEW_TOOLBAR_ICON_SIZE, foreground));

    let button = apply_button_motion(button, motion, enabled);

    if let Some(focus) = focus {
        apply_accessible_button_with_focus(button, label, enabled, focus, palette)
    } else {
        apply_accessible_button(button, label, enabled, palette)
    }
}
