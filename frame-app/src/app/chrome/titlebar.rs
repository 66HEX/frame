use crate::app::chrome::navigation::titlebar_navigation;
use crate::app::chrome::window_controls::{
    linux_window_controls, macos_native_window_controls_placeholder, windows_window_controls,
};
use crate::app::components::FrameTooltip;
use crate::app::layout::{
    LEFT_COLUMN_SPAN, RIGHT_COLUMN_SPAN, TITLEBAR_DIVIDER_HEIGHT, TITLEBAR_HEIGHT,
    TITLEBAR_LOGO_SIZE, TITLEBAR_PLATFORM_DIVIDER_HEIGHT, TITLEBAR_TOP_PADDING, WORKSPACE_COLUMNS,
    WORKSPACE_GAP,
};
use crate::app::model::{FrameAppState, StartAvailability};
use crate::app::primitives::{ButtonVariant, action_button, color, vertical_separator};
use crate::app::root::FrameRoot;
use crate::{assets, theme};
use gpui::prelude::FluentBuilder;
use gpui::{
    ClickEvent, Context, InteractiveElement, IntoElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Styled, Window, WindowControlArea, div, svg,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FrameTitlebarPlatform {
    Macos,
    Windows,
    Linux,
}

impl FrameTitlebarPlatform {
    pub(in crate::app) const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

pub(in crate::app) fn titlebar(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_for_platform(FrameTitlebarPlatform::current(), state, palette, window, cx)
}

pub(in crate::app) fn titlebar_for_platform(
    platform: FrameTitlebarPlatform,
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    match platform {
        FrameTitlebarPlatform::Macos => macos_titlebar(state, palette, window, cx),
        FrameTitlebarPlatform::Windows => windows_titlebar(state, palette, window, cx),
        FrameTitlebarPlatform::Linux => linux_titlebar(state, palette, window, cx),
    }
}

pub(in crate::app) fn macos_titlebar(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let show_workspace_controls = titlebar_shows_workspace_controls(state);

    titlebar_drag_surface(FrameTitlebarPlatform::Macos, cx)
        .h(theme::ui_rem(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .pt(theme::ui_rem(TITLEBAR_TOP_PADDING))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .child(
            div()
                .flex()
                .items_center()
                .mt_2()
                .gap_6()
                .child(macos_native_window_controls_placeholder())
                .when(show_workspace_controls, |this| {
                    this.child(frame_logo(palette))
                        .child(titlebar_divider(palette))
                        .child(titlebar_navigation(state.active_view, palette, window, cx))
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .mt_2()
                .gap_2()
                .when(show_workspace_controls, |this| {
                    this.child(titlebar_settings_button(palette, window, cx))
                        .child(titlebar_add_source_button(palette, window, cx))
                        .child(titlebar_start_button(state, palette, window, cx))
                }),
        )
}

pub(in crate::app) fn windows_titlebar(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_drag_surface(FrameTitlebarPlatform::Windows, cx)
        .relative()
        .h(theme::ui_rem(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .child(platform_titlebar_content(state, palette, window, cx))
        .child(windows_window_controls(palette, window, cx))
}

pub(in crate::app) fn linux_titlebar(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_drag_surface(FrameTitlebarPlatform::Linux, cx)
        .relative()
        .h(theme::ui_rem(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .child(platform_titlebar_content(state, palette, window, cx))
        .child(linux_window_controls(palette, window, cx))
}

pub(super) fn titlebar_drag_surface(
    platform: FrameTitlebarPlatform,
    cx: &Context<FrameRoot>,
) -> gpui::Div {
    let move_window = cx.listener(|root, event: &gpui::MouseMoveEvent, window, _cx| {
        let should_move = std::mem::take(&mut root.titlebar_drag.should_move);
        if should_move && event.pressed_button == Some(MouseButton::Left) {
            window.start_window_move();
        }
    });
    div()
        .window_control_area(WindowControlArea::Drag)
        .capture_any_mouse_down(cx.listener(|root, _event, _window, _cx| {
            root.titlebar_drag.should_move = false;
        }))
        .on_mouse_down_out(cx.listener(|root, _event, _window, _cx| {
            root.titlebar_drag.should_move = false;
        }))
        .capture_any_mouse_up(
            cx.listener(|root, event: &gpui::MouseUpEvent, _window, _cx| {
                if event.button == MouseButton::Left {
                    root.titlebar_drag.should_move = false;
                }
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|root, _event, _window, _cx| {
                root.titlebar_drag.should_move = false;
            }),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |root, event: &gpui::MouseDownEvent, window, _cx| {
                // Buttons consume the press with prevent_default before it bubbles here.
                // Only a press on the remaining titlebar surface may start a window drag.
                if window.default_prevented() {
                    return;
                }
                if platform == FrameTitlebarPlatform::Macos && event.click_count == 2 {
                    // Preserve the system double-click action with native dragging disabled.
                    window.titlebar_double_click();
                } else {
                    root.titlebar_drag.should_move = true;
                }
            }),
        )
        .on_mouse_move_all(move |event, phase, _hitbox, window, cx| {
            // A native event batch can carry the first drag motion past the titlebar.
            // Continue an armed gesture even over another control or the workspace.
            if phase == gpui::DispatchPhase::Capture {
                move_window(event, window, cx);
            }
        })
}

pub(in crate::app) fn platform_titlebar_content(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let show_workspace_controls = titlebar_shows_workspace_controls(state);

    div()
        .absolute()
        .inset_0()
        .mt_2()
        .flex()
        .items_center()
        .px_4()
        .child(
            div()
                .grid()
                .grid_cols(WORKSPACE_COLUMNS)
                .gap(theme::ui_rem(WORKSPACE_GAP))
                .w_full()
                .child(
                    div()
                        .col_span(LEFT_COLUMN_SPAN)
                        .mt_2()
                        .flex()
                        .items_center()
                        .gap_6()
                        .when(show_workspace_controls, |this| {
                            this.child(platform_frame_logo(palette))
                                .child(platform_titlebar_divider(palette))
                                .child(titlebar_navigation(state.active_view, palette, window, cx))
                        }),
                )
                .child(
                    div()
                        .col_span(RIGHT_COLUMN_SPAN)
                        .mt_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .when(show_workspace_controls, |this| {
                            this.child(titlebar_settings_button(palette, window, cx))
                                .child(titlebar_add_source_button(palette, window, cx))
                                .child(titlebar_start_button(state, palette, window, cx))
                        }),
                ),
        )
}

pub(super) const fn titlebar_shows_workspace_controls(state: FrameAppState) -> bool {
    state.file_count > 0
}

pub(in crate::app) fn titlebar_settings_button(
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let focus = crate::app::accessibility::titlebar_settings_focus(window, cx);
    let button = action_button(
        "titlebar-settings",
        assets::ICON_SETTINGS,
        None,
        "Settings",
        ButtonVariant::Secondary,
        true,
        palette,
        window,
        cx,
    )
    .track_focus(&focus);
    FrameTooltip::new("titlebar-settings", "Settings")
        .for_control(button, Some(&focus), true, palette, window, cx)
        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
            if root.settings_ui.is_open {
                root.close_app_settings();
            } else {
                root.open_app_settings();
            }
            cx.notify();
        }))
}

pub(in crate::app) fn titlebar_add_source_button(
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    action_button(
        "titlebar-add-source",
        assets::ICON_PLUS,
        Some("Add source"),
        "Add source",
        ButtonVariant::Secondary,
        true,
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(|_root, _: &ClickEvent, window, cx| {
        cx.stop_propagation();
        FrameRoot::prompt_add_source(window, cx);
    }))
}

pub(in crate::app) fn titlebar_start_button(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let availability = state.start_availability();
    let icon = if matches!(availability, StartAvailability::MissingOutputDirectory) {
        assets::ICON_FOLDER_IMPORT
    } else {
        assets::ICON_PLAY
    };
    action_button(
        "titlebar-start",
        icon,
        Some(availability.button_label()),
        availability.accessibility_label(),
        ButtonVariant::Primary,
        availability.button_enabled(),
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
        cx.stop_propagation();
        match availability {
            StartAvailability::Ready => root.start_selected_conversions(cx),
            StartAvailability::MissingOutputDirectory => {
                FrameRoot::prompt_default_output_folder(window, cx);
            }
            StartAvailability::Processing
            | StartAvailability::NoFiles
            | StartAvailability::NoSelectedFiles
            | StartAvailability::NoActionableFiles => {}
        }
    }))
}

pub(in crate::app) fn frame_logo(palette: &'static theme::ThemePalette) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .text_color(color(palette.text_muted))
        .child(
            svg()
                .path(assets::ICON_FRAME)
                .w(theme::ui_rem(TITLEBAR_LOGO_SIZE))
                .h(theme::ui_rem(TITLEBAR_LOGO_SIZE))
                .text_color(color(palette.text_muted)),
        )
}

pub(in crate::app) fn platform_frame_logo(palette: &'static theme::ThemePalette) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .text_color(color(palette.text_muted))
        .child(
            svg()
                .path(assets::ICON_FRAME)
                .w(theme::ui_rem(TITLEBAR_LOGO_SIZE))
                .h(theme::ui_rem(TITLEBAR_LOGO_SIZE))
                .text_color(color(palette.text_muted)),
        )
}

pub(in crate::app) fn titlebar_divider(palette: &'static theme::ThemePalette) -> gpui::Div {
    vertical_separator(TITLEBAR_DIVIDER_HEIGHT, palette)
}

pub(in crate::app) fn platform_titlebar_divider(
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    vertical_separator(TITLEBAR_PLATFORM_DIVIDER_HEIGHT, palette)
}
