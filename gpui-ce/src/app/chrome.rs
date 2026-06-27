use super::input::{FrameTextInputSpec, frame_text_input};
use super::primitives::*;
use super::settings_panel::{settings_hint_text, settings_section};
use super::*;

pub(super) fn titlebar(state: FrameAppState, cx: &mut Context<FrameRoot>) -> impl IntoElement {
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
                            if root.is_settings_open {
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
                .top_0()
                .right_0()
                .bottom_0()
                .w(px(320.0))
                .flex()
                .flex_col()
                .rounded(px(theme::RADIUS_LG))
                .bg(color(theme::SIDEBAR))
                .shadow(card_surface_shadows())
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

    div()
        .id("app-settings-close")
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
        .child(icon_svg(
            assets::ICON_CLOSE,
            FILE_LIST_ACTION_ICON_SIZE,
            color(colors.foreground),
        ))
}

pub(super) fn app_settings_apply_button(enabled: bool) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Secondary, false, enabled);

    div()
        .id("app-settings-max-concurrency-apply")
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .bg(color(colors.background))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!enabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(enabled, window, cx);
        })
        .child("APPLY")
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

pub(super) fn titlebar_divider() -> gpui::Div {
    vertical_separator(TITLEBAR_DIVIDER_HEIGHT)
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
        .child(
            svg()
                .path(icon)
                .w(px(TITLEBAR_ICON_SIZE))
                .h(px(TITLEBAR_ICON_SIZE))
                .text_color(icon_color)
                .group_hover(segment_id, |style| {
                    style.text_color(color(theme::FOREGROUND))
                }),
        )
        .child(label)
}
