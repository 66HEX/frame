use super::components::frame_text_button;
use super::input::{FrameTextInputSpec, frame_text_input};
use super::primitives::*;
use super::settings_panel::{settings_hint_text, settings_section};
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FrameTitlebarPlatform {
    Macos,
    Windows,
    Linux,
}

impl FrameTitlebarPlatform {
    pub(super) const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

pub(super) fn titlebar(state: FrameAppState, cx: &mut Context<FrameRoot>) -> gpui::Div {
    titlebar_for_platform(FrameTitlebarPlatform::current(), state, cx)
}

pub(super) fn titlebar_for_platform(
    platform: FrameTitlebarPlatform,
    state: FrameAppState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    match platform {
        FrameTitlebarPlatform::Macos => macos_titlebar(state, cx),
        FrameTitlebarPlatform::Windows => windows_titlebar(state, cx),
        FrameTitlebarPlatform::Linux => linux_titlebar(state, cx),
    }
}

pub(super) fn macos_titlebar(state: FrameAppState, cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .pt(px(TITLEBAR_TOP_PADDING))
        .window_control_area(WindowControlArea::Drag)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(
            div()
                .flex()
                .items_center()
                .mt_2()
                .gap_6()
                .child(macos_window_controls(cx))
                .child(frame_logo())
                .child(titlebar_divider())
                .child(titlebar_navigation(state.active_view, cx))
                .child(titlebar_divider())
                .child(titlebar_stats(state)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .mt_2()
                .gap_2()
                .child(
                    action_button(assets::ICON_SETTINGS, None, ButtonVariant::Secondary, true)
                        .id("titlebar-settings")
                        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                            if root.settings_ui.is_open {
                                root.close_app_settings();
                            } else {
                                root.open_app_settings();
                            }
                            cx.notify();
                        })),
                )
                .child(
                    action_button(
                        assets::ICON_PLUS,
                        Some("ADD SOURCE"),
                        ButtonVariant::Secondary,
                        true,
                    )
                    .id("titlebar-add-source")
                    .on_click(cx.listener(
                        |root, _: &ClickEvent, _window, cx| {
                            cx.stop_propagation();
                            root.prompt_add_source(cx);
                        },
                    )),
                )
                .child(
                    action_button(
                        assets::ICON_PLAY,
                        Some(if state.is_processing {
                            "PROCESSING"
                        } else {
                            "START"
                        }),
                        ButtonVariant::Default,
                        state.can_start_conversion(),
                    )
                    .id("titlebar-start")
                    .on_click(cx.listener(
                        move |root, _: &ClickEvent, _window, cx| {
                            cx.stop_propagation();
                            if state.can_start_conversion() {
                                root.start_selected_conversions(cx);
                            }
                        },
                    )),
                ),
        )
}

pub(super) fn windows_titlebar(state: FrameAppState, cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .window_control_area(WindowControlArea::Drag)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(platform_titlebar_content(state, cx))
        .child(windows_window_controls(cx))
}

pub(super) fn linux_titlebar(state: FrameAppState, cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .window_control_area(WindowControlArea::Drag)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(platform_titlebar_content(state, cx))
        .child(linux_window_controls(cx))
}

pub(super) fn platform_titlebar_content(
    state: FrameAppState,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
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
                .gap(px(WORKSPACE_GAP))
                .w_full()
                .child(
                    div()
                        .col_span(LEFT_COLUMN_SPAN)
                        .mt_2()
                        .flex()
                        .items_center()
                        .gap_6()
                        .child(platform_frame_logo())
                        .child(platform_titlebar_divider())
                        .child(titlebar_navigation(state.active_view, cx))
                        .child(platform_titlebar_divider())
                        .child(titlebar_stats(state)),
                )
                .child(
                    div()
                        .col_span(RIGHT_COLUMN_SPAN)
                        .mt_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(titlebar_settings_button(cx))
                        .child(titlebar_add_source_button(cx))
                        .child(titlebar_start_button(state, cx)),
                ),
        )
}

pub(super) fn titlebar_settings_button(cx: &mut Context<FrameRoot>) -> impl IntoElement {
    action_button(assets::ICON_SETTINGS, None, ButtonVariant::Secondary, true)
        .id("titlebar-settings")
        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
            if root.settings_ui.is_open {
                root.close_app_settings();
            } else {
                root.open_app_settings();
            }
            cx.notify();
        }))
}

pub(super) fn titlebar_add_source_button(cx: &mut Context<FrameRoot>) -> impl IntoElement {
    action_button(
        assets::ICON_PLUS,
        Some("ADD SOURCE"),
        ButtonVariant::Secondary,
        true,
    )
    .id("titlebar-add-source")
    .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        root.prompt_add_source(cx);
    }))
}

pub(super) fn titlebar_start_button(
    state: FrameAppState,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    action_button(
        assets::ICON_PLAY,
        Some(if state.is_processing {
            "PROCESSING"
        } else {
            "START"
        }),
        ButtonVariant::Default,
        state.can_start_conversion(),
    )
    .id("titlebar-start")
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if state.can_start_conversion() {
            root.start_selected_conversions(cx);
        }
    }))
}

pub(super) fn app_settings_sheet(
    current_max_concurrency: usize,
    draft_max_concurrency: &str,
    error: Option<&str>,
    value_focus: &FocusHandle,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let draft_is_dirty = draft_max_concurrency.trim() != current_max_concurrency.to_string();

    div()
        .id("app-settings-sheet")
        .absolute()
        .inset_0()
        .child(
            div()
                .id("app-settings-backdrop")
                .absolute()
                .inset_0()
                .bg(color(theme::BACKGROUND.with_alpha(0.60)))
                .backdrop_blur(px(4.0))
                .occlude()
                .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    root.close_app_settings();
                    cx.notify();
                })),
        )
        .child(
            div()
                .id("app-settings-panel")
                .absolute()
                .top_2()
                .right_2()
                .bottom_2()
                .w(px(320.0))
                .flex()
                .flex_col()
                .rounded(px(theme::RADIUS_LG))
                .bg(color(theme::SIDEBAR))
                .shadow(card_surface_shadows())
                .occlude()
                .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                }))
                .child(
                    div()
                        .h(px(PANEL_HEADER_HEIGHT))
                        .w_full()
                        .relative()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_4()
                        .text_size(px(theme::TEXT_LABEL_SIZE))
                        .text_color(color(theme::FOREGROUND))
                        .child("SETTINGS")
                        .child(
                            app_settings_close_button().on_click(
                                cx.listener(|root, _: &ClickEvent, _window, cx| {
                                    cx.stop_propagation();
                                    root.close_app_settings();
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(panel_bottom_separator()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .p_4()
                        .text_size(px(theme::TEXT_LABEL_SIZE))
                        .child(
                            settings_section("MAX CONCURRENCY")
                                .child(app_settings_concurrency_control(
                                    draft_max_concurrency,
                                    draft_is_dirty,
                                    value_focus,
                                    window,
                                    cx,
                                ))
                                .child(settings_hint_text(
                                    "Controls how many queued conversions can run at the same time.",
                                )),
                        )
                        .when_some(error.map(str::to_string), |this, error| {
                            this.child(
                                div()
                                    .id("app-settings-max-concurrency-error")
                                    .text_color(color(theme::FRAME_RED))
                                    .child(error),
                            )
                        }),
                ),
        )
}

pub(super) fn app_settings_concurrency_control(
    draft_max_concurrency: &str,
    can_apply: bool,
    value_focus: &FocusHandle,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().flex_1().min_w_0().child(frame_text_input(
            FrameTextInputSpec {
                id: "app-settings-max-concurrency-value",
                value: draft_max_concurrency,
                placeholder: "2",
                disabled: false,
                focus: Some(value_focus),
                kind: FrameTextInputKind::MaxConcurrency,
            },
            window,
            cx,
        )))
        .child(app_settings_apply_button(can_apply).on_click(cx.listener(
            move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if can_apply && root.apply_max_concurrency_draft() {
                    cx.notify();
                }
            },
        )))
}

pub(super) fn app_settings_close_button() -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Ghost, false, true);
    let close_id = "app-settings-close";

    div()
        .id(close_id)
        .group(close_id)
        .w(px(SETTINGS_CONTROL_HEIGHT))
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(colors.background))
        .text_color(color(colors.foreground))
        .hover(move |style| {
            style
                .bg(color(colors.hover_background))
                .text_color(color(colors.hover_foreground))
                .cursor_pointer()
        })
        .active(move |style| style.bg(color(colors.active_background)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .child(icon_svg_with_hover(
            assets::ICON_CLOSE,
            FILE_LIST_ACTION_ICON_SIZE,
            color(colors.foreground),
            close_id,
            color(colors.hover_foreground),
        ))
}

pub(super) fn app_settings_apply_button(enabled: bool) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        "app-settings-max-concurrency-apply",
        "APPLY",
        ButtonVariant::Secondary,
        false,
        enabled,
    )
}

pub(super) fn drag_drop_overlay(cx: &mut Context<FrameRoot>) -> impl IntoElement {
    div()
        .id("drag-drop-overlay")
        .invisible()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(color(theme::BACKGROUND.with_alpha(0.60)))
        .backdrop_blur(px(4.0))
        .group_drag_over::<ExternalPaths>(ROOT_DROP_GROUP, |style| style.visible())
        .on_drop(cx.listener(|root, paths: &ExternalPaths, _window, cx| {
            cx.stop_propagation();
            root.import_source_paths(paths.paths().to_vec(), cx);
        }))
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(theme::RADIUS_LG))
                .border_1()
                .border_dashed()
                .border_color(color(theme::FRAME_GRAY_100))
                .bg(color(theme::FRAME_GRAY_100))
                .shadow(card_surface_shadows())
                .child(
                    div()
                        .text_size(px(theme::TEXT_LABEL_SIZE))
                        .text_color(color(theme::FOREGROUND))
                        .child("IMPORT SOURCE FILES"),
                ),
        )
}

pub(super) fn macos_window_controls(cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .mr_2()
        .group(TRAFFIC_LIGHT_GROUP)
        .child(
            traffic_light(
                TRAFFIC_CLOSE_FILL,
                TRAFFIC_CLOSE_BORDER,
                TRAFFIC_CLOSE_SYMBOL,
                assets::ICON_TRAFFIC_CLOSE_SYMBOL,
            )
            .id("titlebar-close")
            .window_control_area(WindowControlArea::Close)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.remove_window();
            })),
        )
        .child(
            traffic_light(
                TRAFFIC_MINIMIZE_FILL,
                TRAFFIC_MINIMIZE_BORDER,
                TRAFFIC_MINIMIZE_SYMBOL,
                assets::ICON_TRAFFIC_MINIMIZE_SYMBOL,
            )
            .id("titlebar-minimize")
            .window_control_area(WindowControlArea::Min)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.minimize_window();
            })),
        )
        .child(
            traffic_light(
                TRAFFIC_ZOOM_FILL,
                TRAFFIC_ZOOM_BORDER,
                TRAFFIC_ZOOM_SYMBOL,
                assets::ICON_TRAFFIC_ZOOM_SYMBOL,
            )
            .id("titlebar-zoom")
            .window_control_area(WindowControlArea::Max)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.zoom_window();
            })),
        )
}

pub(super) fn windows_window_controls(cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .absolute()
        .top_0()
        .right_0()
        .h_full()
        .flex()
        .items_center()
        .child(
            titlebar_window_button(
                "titlebar-windows-minimize",
                assets::ICON_MINUS,
                TITLEBAR_WINDOWS_WINDOW_ICON_SIZE,
                TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                TITLEBAR_HEIGHT,
                0.0,
                false,
            )
            .window_control_area(WindowControlArea::Min)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.minimize_window();
            })),
        )
        .child(
            titlebar_window_button(
                "titlebar-windows-maximize",
                assets::ICON_SQUARE,
                TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE,
                TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                TITLEBAR_HEIGHT,
                0.0,
                false,
            )
            .window_control_area(WindowControlArea::Max)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.zoom_window();
            })),
        )
        .child(
            titlebar_window_button(
                "titlebar-windows-close",
                assets::ICON_CLOSE,
                TITLEBAR_WINDOWS_WINDOW_ICON_SIZE,
                TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                TITLEBAR_HEIGHT,
                0.0,
                true,
            )
            .window_control_area(WindowControlArea::Close)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.remove_window();
            })),
        )
}

pub(super) fn linux_window_controls(cx: &mut Context<FrameRoot>) -> gpui::Div {
    div()
        .absolute()
        .top_0()
        .right_0()
        .h_full()
        .flex()
        .items_center()
        .gap(px(TITLEBAR_LINUX_WINDOW_CONTROLS_GAP))
        .px(px(TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X))
        .child(
            titlebar_window_button(
                "titlebar-linux-minimize",
                assets::ICON_MINUS,
                TITLEBAR_ACTION_ICON_SIZE,
                TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                theme::RADIUS_SM,
                false,
            )
            .window_control_area(WindowControlArea::Min)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.minimize_window();
            })),
        )
        .child(
            titlebar_window_button(
                "titlebar-linux-maximize",
                assets::ICON_SQUARE,
                TITLEBAR_ACTION_ICON_SIZE,
                TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                theme::RADIUS_SM,
                false,
            )
            .window_control_area(WindowControlArea::Max)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.zoom_window();
            })),
        )
        .child(
            titlebar_window_button(
                "titlebar-linux-close",
                assets::ICON_CLOSE,
                TITLEBAR_ACTION_ICON_SIZE,
                TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                theme::RADIUS_SM,
                true,
            )
            .window_control_area(WindowControlArea::Close)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.remove_window();
            })),
        )
}

pub(super) fn titlebar_window_button(
    id: &'static str,
    icon: &'static str,
    icon_size: f32,
    width: f32,
    height: f32,
    radius: f32,
    destructive: bool,
) -> gpui::Stateful<gpui::Div> {
    let hover_background = if destructive {
        theme::FRAME_RED
    } else {
        theme::FRAME_GRAY_100
    };
    let active_background = if destructive {
        theme::FRAME_RED
    } else {
        theme::FRAME_GRAY_200
    };
    let hover_foreground = theme::FOREGROUND;
    let foreground = theme::FRAME_GRAY_600;

    div()
        .id(id)
        .group(id)
        .w(px(width))
        .h(px(height))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(radius))
        .bg(color(theme::TRANSPARENT))
        .text_color(color(foreground))
        .hover(move |style| {
            style
                .bg(color(hover_background))
                .text_color(color(hover_foreground))
                .cursor_pointer()
        })
        .active(move |style| style.bg(color(active_background)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .child(icon_svg_with_hover(
            icon,
            icon_size,
            color(foreground),
            id,
            color(hover_foreground),
        ))
}

pub(super) fn traffic_light(
    fill: &'static str,
    border: &'static str,
    symbol_color: &'static str,
    symbol_icon: &'static str,
) -> gpui::Div {
    div()
        .w(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .h(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .child(
            div()
                .w(px(TITLEBAR_TRAFFIC_LIGHT_DOT_SIZE))
                .h(px(TITLEBAR_TRAFFIC_LIGHT_DOT_SIZE))
                .rounded_full()
                .bg(parse_hex(fill))
                .border(px(TITLEBAR_TRAFFIC_LIGHT_STROKE_WIDTH))
                .border_color(parse_hex(border)),
        )
        .child(
            svg()
                .path(symbol_icon)
                .absolute()
                .inset_0()
                .w(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
                .h(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
                .opacity(0.0)
                .group_hover(TRAFFIC_LIGHT_GROUP, |style| style.opacity(1.0))
                .text_color(parse_hex(symbol_color)),
        )
}

pub(super) fn frame_logo() -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .text_color(color(theme::FRAME_GRAY_600))
        .child(
            svg()
                .path(assets::ICON_FRAME)
                .w(px(TITLEBAR_LOGO_SIZE))
                .h(px(TITLEBAR_LOGO_SIZE))
                .text_color(color(theme::FRAME_GRAY_600)),
        )
}

pub(super) fn platform_frame_logo() -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .text_color(color(theme::FRAME_GRAY_600))
        .child(
            svg()
                .path(assets::ICON_FRAME)
                .w(px(TITLEBAR_LOGO_SIZE))
                .h(px(TITLEBAR_LOGO_SIZE))
                .text_color(color(theme::FRAME_GRAY_600)),
        )
}

pub(super) fn titlebar_divider() -> gpui::Div {
    vertical_separator(TITLEBAR_DIVIDER_HEIGHT)
}

pub(super) fn platform_titlebar_divider() -> gpui::Div {
    vertical_separator(TITLEBAR_PLATFORM_DIVIDER_HEIGHT)
}

pub(super) fn titlebar_navigation(
    active_view: ActiveView,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .h(px(TITLEBAR_SEGMENT_HEIGHT))
        .flex()
        .items_center()
        .gap_1()
        .rounded(px(theme::RADIUS_MD))
        .bg(color(theme::FRAME_GRAY_100))
        .px(px(3.0))
        .py(px(2.0))
        .shadow(input_highlight_shadows())
        .child(titlebar_segment(
            assets::ICON_LAYOUT_LIST,
            "WORKSPACE",
            ActiveView::Workspace,
            active_view == ActiveView::Workspace,
            cx,
        ))
        .child(titlebar_segment(
            assets::ICON_TERMINAL,
            "LOGS",
            ActiveView::Logs,
            active_view == ActiveView::Logs,
            cx,
        ))
}

pub(super) fn titlebar_stats(state: FrameAppState) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_4()
        .text_color(color(theme::FRAME_GRAY_600))
        .child(titlebar_stat(
            assets::ICON_HARD_DRIVE,
            format!("STORAGE {}", format_total_size(state.total_size_bytes)),
        ))
        .child(titlebar_stat(
            assets::ICON_FILE_VIDEO,
            format!("ITEMS {}", state.file_count),
        ))
}

pub(super) fn titlebar_stat(icon: &'static str, label: String) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(icon_svg(
            icon,
            TITLEBAR_ICON_SIZE,
            color(theme::FRAME_GRAY_600),
        ))
        .child(label)
}

pub(super) fn titlebar_segment(
    icon: &'static str,
    label: &'static str,
    view: ActiveView,
    selected: bool,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let colors = button_colors(ButtonVariant::Secondary, selected, true);
    let segment_id = match view {
        ActiveView::Workspace => "titlebar-workspace",
        ActiveView::Logs => "titlebar-logs",
    };
    let icon_color = if selected {
        color(theme::FOREGROUND)
    } else {
        color(theme::FRAME_GRAY_600)
    };

    div()
        .h(px(TITLEBAR_NAV_BUTTON_HEIGHT))
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_SM))
        .id(segment_id)
        .group(segment_id)
        .px_2()
        .bg(if selected {
            color(colors.background)
        } else {
            color(theme::TRANSPARENT)
        })
        .text_color(if selected {
            color(theme::FOREGROUND)
        } else {
            color(theme::FRAME_GRAY_600)
        })
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .hover(move |style| {
            let style = style.text_color(color(theme::FOREGROUND)).cursor_pointer();
            if selected {
                style.bg(color(colors.hover_background))
            } else {
                style
            }
        })
        .active(move |style| style.bg(color(colors.active_background)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            if root.active_view != view {
                root.active_view = view;
                cx.notify();
            }
            cx.stop_propagation();
        }))
        .child(icon_svg_with_hover(
            icon,
            TITLEBAR_ICON_SIZE,
            icon_color,
            segment_id,
            color(theme::FOREGROUND),
        ))
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_titlebar_platform_matches_compile_target() {
        let expected = if cfg!(target_os = "macos") {
            FrameTitlebarPlatform::Macos
        } else if cfg!(target_os = "windows") {
            FrameTitlebarPlatform::Windows
        } else {
            FrameTitlebarPlatform::Linux
        };

        assert_eq!(FrameTitlebarPlatform::current(), expected);
    }
}
