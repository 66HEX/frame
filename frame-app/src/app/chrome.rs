use super::components::{frame_checkbox_row, frame_text_button};
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

pub(super) fn titlebar(
    state: FrameAppState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_for_platform(FrameTitlebarPlatform::current(), state, window, cx)
}

pub(super) fn titlebar_for_platform(
    platform: FrameTitlebarPlatform,
    state: FrameAppState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    match platform {
        FrameTitlebarPlatform::Macos => macos_titlebar(state, window, cx),
        FrameTitlebarPlatform::Windows => windows_titlebar(state, window, cx),
        FrameTitlebarPlatform::Linux => linux_titlebar(state, window, cx),
    }
}

pub(super) fn macos_titlebar(
    state: FrameAppState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
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
                .child(titlebar_navigation(state.active_view, window, cx))
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
                    action_button(
                        "titlebar-settings",
                        assets::ICON_SETTINGS,
                        None,
                        ButtonVariant::Secondary,
                        true,
                        window,
                        cx,
                    )
                    .on_click(cx.listener(
                        |root, _: &ClickEvent, _window, cx| {
                            if root.settings_ui.is_open {
                                root.close_app_settings();
                            } else {
                                root.open_app_settings();
                            }
                            cx.notify();
                        },
                    )),
                )
                .child(
                    action_button(
                        "titlebar-add-source",
                        assets::ICON_PLUS,
                        Some("ADD SOURCE"),
                        ButtonVariant::Secondary,
                        true,
                        window,
                        cx,
                    )
                    .on_click(cx.listener(
                        |root, _: &ClickEvent, _window, cx| {
                            cx.stop_propagation();
                            root.prompt_add_source(cx);
                        },
                    )),
                )
                .child(
                    action_button(
                        "titlebar-start",
                        assets::ICON_PLAY,
                        Some(if state.is_processing {
                            "PROCESSING"
                        } else {
                            "START"
                        }),
                        ButtonVariant::Default,
                        state.can_start_conversion(),
                        window,
                        cx,
                    )
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

pub(super) fn windows_titlebar(
    state: FrameAppState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .window_control_area(WindowControlArea::Drag)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(platform_titlebar_content(state, window, cx))
        .child(windows_window_controls(window, cx))
}

pub(super) fn linux_titlebar(
    state: FrameAppState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .window_control_area(WindowControlArea::Drag)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(platform_titlebar_content(state, window, cx))
        .child(linux_window_controls(window, cx))
}

pub(super) fn platform_titlebar_content(
    state: FrameAppState,
    window: &mut Window,
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
                        .child(titlebar_navigation(state.active_view, window, cx))
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
                        .child(titlebar_settings_button(window, cx))
                        .child(titlebar_add_source_button(window, cx))
                        .child(titlebar_start_button(state, window, cx)),
                ),
        )
}

pub(super) fn titlebar_settings_button(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    action_button(
        "titlebar-settings",
        assets::ICON_SETTINGS,
        None,
        ButtonVariant::Secondary,
        true,
        window,
        cx,
    )
    .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
        if root.settings_ui.is_open {
            root.close_app_settings();
        } else {
            root.open_app_settings();
        }
        cx.notify();
    }))
}

pub(super) fn titlebar_add_source_button(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    action_button(
        "titlebar-add-source",
        assets::ICON_PLUS,
        Some("ADD SOURCE"),
        ButtonVariant::Secondary,
        true,
        window,
        cx,
    )
    .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        root.prompt_add_source(cx);
    }))
}

pub(super) fn titlebar_start_button(
    state: FrameAppState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    action_button(
        "titlebar-start",
        assets::ICON_PLAY,
        Some(if state.is_processing {
            "PROCESSING"
        } else {
            "START"
        }),
        ButtonVariant::Default,
        state.can_start_conversion(),
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if state.can_start_conversion() {
            root.start_selected_conversions(cx);
        }
    }))
}

pub(super) struct AppSettingsSheetProps<'a> {
    pub(super) is_open: bool,
    pub(super) current_max_concurrency: usize,
    pub(super) draft_max_concurrency: &'a str,
    pub(super) error: Option<&'a str>,
    pub(super) auto_update_check: bool,
    pub(super) update_status: &'a UpdateStatus,
    pub(super) value_focus: &'a FocusHandle,
}

pub(super) fn app_settings_sheet(
    props: AppSettingsSheetProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let draft_is_dirty =
        props.draft_max_concurrency.trim() != props.current_max_concurrency.to_string();
    let transition = window
        .use_keyed_transition(
            "app-settings-sheet-motion",
            cx,
            SETTINGS_SHEET_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_out_quint());
    let target = motion_target(props.is_open);
    if *transition.read_goal(cx) != target {
        transition.update(cx, |progress, cx| {
            *progress = target;
            cx.notify();
        });
    }
    let progress = *transition.evaluate(window, cx);
    let right_inset = settings_sheet_right_inset(progress);

    if !props.is_open && motion_is_hidden(progress) {
        cx.defer_in(window, |root, _window, cx| {
            if root.finish_app_settings_close() {
                cx.notify();
            }
        });
    }

    div()
        .id("app-settings-sheet")
        .absolute()
        .inset_0()
        .child(
            div()
                .id("app-settings-backdrop")
                .absolute()
                .inset_0()
                .bg(color(theme::BACKGROUND.with_alpha(0.60 * progress)))
                .backdrop_blur(px(4.0 * progress))
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
                .right(px(right_inset))
                .bottom_2()
                .w(px(360.0))
                .flex()
                .flex_col()
                .rounded(px(theme::RADIUS_LG))
                .bg(color(theme::SIDEBAR))
                .opacity(progress)
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
                            app_settings_close_button(window, cx).on_click(
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
                                    props.draft_max_concurrency,
                                    draft_is_dirty,
                                    props.value_focus,
                                    window,
                                    cx,
                                ))
                                .child(settings_hint_text(
                                    "Controls how many queued conversions can run at the same time.",
                                )),
                        )
                        .when_some(props.error.map(str::to_string), |this, error| {
                            this.child(
                                div()
                                    .id("app-settings-max-concurrency-error")
                                    .text_color(color(theme::FRAME_RED))
                                    .child(error),
                            )
                        })
                        .child(app_settings_updates_section(
                            props.auto_update_check,
                            props.update_status,
                            window,
                            cx,
                        )),
                ),
        )
}

fn app_settings_updates_section(
    auto_update_check: bool,
    update_status: &UpdateStatus,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let busy = update_status.is_busy();
    let mut section = settings_section("UPDATES")
        .child(
            frame_checkbox_row(
                "app-settings-auto-update-check",
                "Check automatically",
                "Frame checks for signed releases in the background.",
                auto_update_check,
                false,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if root.toggle_auto_update_check() {
                    cx.notify();
                }
            })),
        )
        .child(update_status_label(update_status));

    if let UpdateStatus::Downloading {
        progress_percent,
        received_bytes,
        total_bytes,
        ..
    } = update_status
    {
        section = section.child(update_progress_bar(*progress_percent));
        section = section.child(update_download_detail(
            *received_bytes,
            *total_bytes,
            *progress_percent,
        ));
    }

    section.child(update_action_row(update_status, busy, window, cx))
}

fn update_status_label(status: &UpdateStatus) -> gpui::Div {
    let tone = match status {
        UpdateStatus::Error(_) => theme::FRAME_RED,
        UpdateStatus::Disabled(_) => theme::FRAME_AMBER,
        _ => theme::FRAME_GRAY_600,
    };

    div()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(tone))
        .child(update_status_text(status))
}

fn update_status_text(status: &UpdateStatus) -> String {
    match status {
        UpdateStatus::Idle => "No update check is running.".to_string(),
        UpdateStatus::Checking => "Checking for updates...".to_string(),
        UpdateStatus::UpToDate => "Frame is up to date.".to_string(),
        UpdateStatus::Available(info) => {
            format!("Frame {} is available.", info.version)
        }
        UpdateStatus::Downloading {
            version,
            progress_percent,
            ..
        } => progress_percent.map_or_else(
            || format!("Downloading Frame {version}..."),
            |percent| format!("Downloading Frame {version}: {percent}%"),
        ),
        UpdateStatus::ReadyToInstall(package) => {
            format!("Frame {} is ready to install.", package.version)
        }
        UpdateStatus::Installing => "Installing update and restarting...".to_string(),
        UpdateStatus::Disabled(explanation) => explanation.clone(),
        UpdateStatus::Error(error) => error.clone(),
    }
}

fn update_release_notes_text(info: Option<&UpdateInfo>) -> Option<String> {
    let notes = info?.release_notes_markdown.as_deref()?;
    let notes = notes.trim();
    if notes.is_empty() {
        return None;
    }

    const MAX_RELEASE_NOTES_CHARS: usize = 8_000;
    let mut text = notes
        .chars()
        .take(MAX_RELEASE_NOTES_CHARS + 1)
        .collect::<String>();
    if text.chars().count() > MAX_RELEASE_NOTES_CHARS {
        text = text.chars().take(MAX_RELEASE_NOTES_CHARS).collect();
        text.push_str("...");
    }
    Some(text)
}

fn update_release_notes_block(notes: &str) -> gpui::Stateful<gpui::Div> {
    let mut block = div()
        .id("update-dialog-release-notes")
        .min_h(px(180.0))
        .max_h(px(360.0))
        .overflow_y_scroll()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::FRAME_GRAY_100))
        .p_3();

    for line in normalized_release_note_lines(notes) {
        block = block.child(update_release_note_line(&line));
    }

    block
}

fn normalized_release_note_lines(notes: &str) -> Vec<String> {
    let mut lines = notes
        .lines()
        .map(str::trim_end)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        vec!["No release notes were published for this version.".to_string()]
    } else {
        lines
    }
}

fn update_release_note_line(line: &str) -> gpui::Div {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return div().h(px(8.0));
    }

    let heading = trimmed.trim_start_matches('#').trim();
    let bullet = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("• "));
    let (text, left_padding, text_color, font_weight) =
        if !heading.is_empty() && trimmed.starts_with('#') {
            (
                heading.to_string(),
                0.0,
                theme::FOREGROUND,
                gpui::FontWeight::MEDIUM,
            )
        } else if let Some(bullet) = bullet {
            (
                format!("• {bullet}"),
                8.0,
                theme::FRAME_GRAY_600,
                gpui::FontWeight::NORMAL,
            )
        } else {
            (
                trimmed.to_string(),
                0.0,
                theme::FRAME_GRAY_600,
                gpui::FontWeight::NORMAL,
            )
        };

    div()
        .pl(px(left_padding))
        .pb(px(4.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .line_height(px(16.0))
        .text_color(color(text_color))
        .font_weight(font_weight)
        .child(text)
}

fn update_progress_bar(progress_percent: Option<u8>) -> gpui::Div {
    let fraction = progress_percent.map_or(0.0, |percent| f32::from(percent) / 100.0);

    div()
        .h(px(6.0))
        .w_full()
        .overflow_hidden()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::FRAME_GRAY_100))
        .child(
            div()
                .h_full()
                .w(relative(fraction.clamp(0.0, 1.0)))
                .rounded(px(theme::RADIUS_SM))
                .bg(color(theme::FRAME_BLUE)),
        )
}

fn update_download_detail(
    received_bytes: u64,
    total_bytes: Option<u64>,
    progress_percent: Option<u8>,
) -> gpui::Div {
    let detail = match (total_bytes, progress_percent) {
        (Some(total_bytes), Some(percent)) => format!(
            "{} of {} ({percent}%)",
            format_total_size(received_bytes),
            format_total_size(total_bytes)
        ),
        (Some(total_bytes), None) => format!(
            "{} of {}",
            format_total_size(received_bytes),
            format_total_size(total_bytes)
        ),
        (None, _) => format!("{} downloaded", format_total_size(received_bytes)),
    };

    div()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FRAME_GRAY_600))
        .font_features(assets::frame_tabular_number_font_features())
        .child(detail)
}

fn update_action_row(
    status: &UpdateStatus,
    busy: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut row = div().flex().items_center().gap_2();
    row = row.child(
        frame_text_button(
            "app-settings-update-check-now",
            "CHECK NOW",
            ButtonVariant::Secondary,
            false,
            !busy,
            window,
            cx,
        )
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if !busy {
                root.check_for_updates(true, cx);
                cx.notify();
            }
        })),
    );

    match status {
        UpdateStatus::Available(_) => row
            .child(
                frame_text_button(
                    "app-settings-update-download",
                    "DOWNLOAD",
                    ButtonVariant::Default,
                    false,
                    true,
                    window,
                    cx,
                )
                .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    root.download_available_update(cx);
                    cx.notify();
                })),
            )
            .child(
                frame_text_button(
                    "app-settings-update-skip",
                    "SKIP",
                    ButtonVariant::Secondary,
                    false,
                    true,
                    window,
                    cx,
                )
                .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if root.skip_available_update() {
                        cx.notify();
                    }
                })),
            ),
        UpdateStatus::ReadyToInstall(_) => row.child(
            frame_text_button(
                "app-settings-update-install",
                "INSTALL AND RESTART",
                ButtonVariant::Default,
                false,
                true,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                root.install_downloaded_update(cx);
                cx.notify();
            })),
        ),
        UpdateStatus::UpToDate | UpdateStatus::Disabled(_) | UpdateStatus::Error(_) => row.child(
            frame_text_button(
                "app-settings-update-dismiss",
                "DISMISS",
                ButtonVariant::Secondary,
                false,
                true,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                root.dismiss_update_status();
                cx.notify();
            })),
        ),
        UpdateStatus::Idle
        | UpdateStatus::Checking
        | UpdateStatus::Downloading { .. }
        | UpdateStatus::Installing => row,
    }
}

pub(super) fn update_dialog(
    is_open: bool,
    status: &UpdateStatus,
    info: Option<&UpdateInfo>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let transition = window
        .use_keyed_transition(
            "update-dialog-motion",
            cx,
            SETTINGS_SHEET_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_out_quint());
    set_motion_target(&transition, motion_target(is_open), cx);
    let progress = *transition.evaluate(window, cx);
    let panel_offset = (1.0 - progress.clamp(0.0, 1.0)) * 12.0;

    if !is_open && motion_is_hidden(progress) {
        cx.defer_in(window, |root, _window, cx| {
            if root.finish_update_dialog_close() {
                cx.notify();
            }
        });
    }

    div()
        .id("update-dialog")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(color(theme::BACKGROUND.with_alpha(0.64 * progress)))
        .backdrop_blur(px(4.0 * progress))
        .opacity(progress)
        .occlude()
        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            root.close_update_dialog();
            cx.notify();
        }))
        .child(update_dialog_panel(panel_offset, status, info, window, cx))
}

fn update_dialog_panel(
    panel_offset: f32,
    status: &UpdateStatus,
    info: Option<&UpdateInfo>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let mut panel = div()
        .id("update-dialog-panel")
        .mt(px(panel_offset))
        .w_full()
        .max_w(px(720.0))
        .max_h(relative(0.86))
        .overflow_hidden()
        .rounded(px(theme::RADIUS_LG))
        .bg(color(theme::SIDEBAR))
        .shadow(card_surface_shadows())
        .occlude()
        .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
        }))
        .child(update_dialog_header(status, window, cx))
        .child(update_dialog_body(status, info))
        .child(update_dialog_footer(status, window, cx));

    if matches!(status, UpdateStatus::Downloading { .. }) {
        panel = panel.child(update_dialog_download_state(status));
    }

    panel
}

fn update_dialog_header(
    status: &UpdateStatus,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .relative()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(theme::TEXT_LABEL_SIZE))
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(update_dialog_kicker(status)),
                )
                .child(
                    div()
                        .text_size(px(16.0))
                        .line_height(px(20.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(color(theme::FOREGROUND))
                        .child(update_dialog_title(status)),
                ),
        )
        .child(app_settings_close_button(window, cx).on_click(cx.listener(
            |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                root.close_update_dialog();
                cx.notify();
            },
        )))
        .child(panel_bottom_separator())
}

fn update_dialog_body(status: &UpdateStatus, info: Option<&UpdateInfo>) -> gpui::Div {
    let notes = update_release_notes_text(info);
    let mut body = div().flex().flex_col().gap_3().p_4().child(
        div()
            .text_size(px(theme::TEXT_LABEL_SIZE))
            .line_height(px(16.0))
            .text_color(color(theme::FRAME_GRAY_600))
            .child(update_dialog_summary(status, notes.is_some())),
    );

    if let Some(notes) = notes.as_deref() {
        body = body
            .child(
                div()
                    .text_size(px(theme::TEXT_LABEL_SIZE))
                    .text_color(color(theme::FOREGROUND))
                    .child("RELEASE NOTES"),
            )
            .child(update_release_notes_block(notes));
    }

    if let UpdateStatus::Error(error) = status {
        body = body.child(
            div()
                .rounded(px(theme::RADIUS_SM))
                .bg(color(theme::FRAME_RED.with_alpha(0.08)))
                .p_3()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .line_height(px(16.0))
                .text_color(color(theme::FRAME_RED))
                .child(error.clone()),
        );
    }

    body
}

fn update_dialog_download_state(status: &UpdateStatus) -> gpui::Div {
    let UpdateStatus::Downloading {
        progress_percent,
        received_bytes,
        total_bytes,
        ..
    } = status
    else {
        return div();
    };

    div()
        .px_4()
        .pb_4()
        .flex()
        .flex_col()
        .gap_2()
        .child(update_progress_bar(*progress_percent))
        .child(update_download_detail(
            *received_bytes,
            *total_bytes,
            *progress_percent,
        ))
}

fn update_dialog_footer(
    status: &UpdateStatus,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .relative()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px_4()
        .py_3()
        .child(
            frame_text_button(
                "update-dialog-later",
                "LATER",
                ButtonVariant::Ghost,
                false,
                !status.is_busy(),
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !root.update_ui.status.is_busy() {
                    root.close_update_dialog();
                    cx.notify();
                }
            })),
        )
        .child(update_dialog_primary_action(status, window, cx))
        .child(panel_top_separator())
}

fn update_dialog_primary_action(
    status: &UpdateStatus,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    match status {
        UpdateStatus::Available(_) => frame_text_button(
            "update-dialog-download",
            "DOWNLOAD",
            ButtonVariant::Default,
            false,
            true,
            window,
            cx,
        )
        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            root.download_available_update(cx);
            cx.notify();
        })),
        UpdateStatus::ReadyToInstall(_) => frame_text_button(
            "update-dialog-install",
            "INSTALL AND RESTART",
            ButtonVariant::Default,
            false,
            true,
            window,
            cx,
        )
        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            root.install_downloaded_update(cx);
            cx.notify();
        })),
        UpdateStatus::Error(_) => frame_text_button(
            "update-dialog-dismiss",
            "DISMISS",
            ButtonVariant::Secondary,
            false,
            true,
            window,
            cx,
        )
        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            root.dismiss_update_status();
            cx.notify();
        })),
        UpdateStatus::Downloading { .. } | UpdateStatus::Installing => frame_text_button(
            "update-dialog-busy",
            "WORKING",
            ButtonVariant::Secondary,
            false,
            false,
            window,
            cx,
        ),
        UpdateStatus::Idle
        | UpdateStatus::Checking
        | UpdateStatus::UpToDate
        | UpdateStatus::Disabled(_) => frame_text_button(
            "update-dialog-close",
            "CLOSE",
            ButtonVariant::Secondary,
            false,
            true,
            window,
            cx,
        )
        .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            root.close_update_dialog();
            cx.notify();
        })),
    }
}

fn update_dialog_kicker(status: &UpdateStatus) -> &'static str {
    match status {
        UpdateStatus::Available(_) => "UPDATE AVAILABLE",
        UpdateStatus::Downloading { .. } => "DOWNLOADING UPDATE",
        UpdateStatus::ReadyToInstall(_) => "READY TO INSTALL",
        UpdateStatus::Installing => "INSTALLING UPDATE",
        UpdateStatus::Error(_) => "UPDATE ERROR",
        UpdateStatus::Checking => "CHECKING FOR UPDATES",
        UpdateStatus::UpToDate => "NO UPDATE AVAILABLE",
        UpdateStatus::Disabled(_) => "UPDATES DISABLED",
        UpdateStatus::Idle => "UPDATES",
    }
}

fn update_dialog_title(status: &UpdateStatus) -> String {
    match status {
        UpdateStatus::Available(info) => format!("Frame {} is available", info.version),
        UpdateStatus::Downloading { version, .. } => format!("Downloading Frame {version}"),
        UpdateStatus::ReadyToInstall(package) => {
            format!("Frame {} is ready to install", package.version)
        }
        UpdateStatus::Installing => "Installing update and restarting".to_string(),
        UpdateStatus::Error(_) => "Frame could not complete the update".to_string(),
        UpdateStatus::Checking => "Checking for updates".to_string(),
        UpdateStatus::UpToDate => "Frame is up to date".to_string(),
        UpdateStatus::Disabled(explanation) => explanation.clone(),
        UpdateStatus::Idle => "Frame updates".to_string(),
    }
}

fn update_dialog_summary(status: &UpdateStatus, has_notes: bool) -> String {
    match status {
        UpdateStatus::Available(_) if has_notes => {
            "Review what changed before installing this signed update.".to_string()
        }
        UpdateStatus::Available(_) => {
            "A signed update is available, but this release did not include notes.".to_string()
        }
        UpdateStatus::Downloading { .. } => {
            "Keep Frame open while the update package is downloaded and verified.".to_string()
        }
        UpdateStatus::ReadyToInstall(_) => {
            "The update was downloaded and verified. Frame will restart to finish installation."
                .to_string()
        }
        UpdateStatus::Installing => {
            "Frame is handing installation to the bundled update helper.".to_string()
        }
        UpdateStatus::Error(_) => {
            "The updater stopped before installation completed. You can dismiss this and try again."
                .to_string()
        }
        UpdateStatus::Checking => {
            "Frame is checking the latest signed release manifest.".to_string()
        }
        UpdateStatus::UpToDate => "No newer signed release is available.".to_string(),
        UpdateStatus::Disabled(explanation) => explanation.clone(),
        UpdateStatus::Idle => "No update check is running.".to_string(),
    }
}

fn panel_top_separator() -> gpui::Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(1.0))
        .bg(color(theme::FRAME_GRAY_100))
}

pub(super) fn app_settings_concurrency_control(
    draft_max_concurrency: &str,
    can_apply: bool,
    value_focus: &FocusHandle,
    window: &mut Window,
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
        .child(
            app_settings_apply_button(can_apply, window, cx).on_click(cx.listener(
                move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if can_apply && root.apply_max_concurrency_draft() {
                        cx.notify();
                    }
                },
            )),
        )
}

pub(super) fn app_settings_close_button(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Ghost, false, true);
    let close_id = "app-settings-close";
    let animated = animated_button_colors(close_id, colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;

    div()
        .id(close_id)
        .group(close_id)
        .w(px(SETTINGS_CONTROL_HEIGHT))
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(background)
        .text_color(foreground)
        .hover(|style| style.cursor_pointer())
        .active(move |style| style.bg(color(colors.active_background)))
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .child(icon_svg(
            assets::ICON_CLOSE,
            FILE_LIST_ACTION_ICON_SIZE,
            foreground,
        ))
}

pub(super) fn app_settings_apply_button(
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        "app-settings-max-concurrency-apply",
        "APPLY",
        ButtonVariant::Secondary,
        false,
        enabled,
        window,
        cx,
    )
}

pub(super) fn drag_drop_overlay(
    is_open: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let transition = window
        .use_keyed_transition(
            "drag-drop-overlay-motion",
            cx,
            SETTINGS_SHEET_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_out_quint());
    set_motion_target(&transition, motion_target(is_open), cx);
    let progress = *transition.evaluate(window, cx);

    if !is_open && motion_is_hidden(progress) {
        cx.defer_in(window, |root, _window, cx| {
            if root.finish_drag_drop_overlay_close() {
                cx.notify();
            }
        });
    }

    div()
        .id("drag-drop-overlay")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(color(theme::BACKGROUND.with_alpha(0.60 * progress)))
        .backdrop_blur(px(4.0 * progress))
        .opacity(progress)
        .occlude()
        .on_drop(cx.listener(|root, paths: &ExternalPaths, _window, cx| {
            cx.stop_propagation();
            root.close_drag_drop_overlay();
            root.import_source_paths(paths.paths().to_vec(), cx);
            cx.notify();
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

pub(super) fn windows_window_controls(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
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
                window,
                cx,
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
                window,
                cx,
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
                window,
                cx,
            )
            .window_control_area(WindowControlArea::Close)
            .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                window.remove_window();
            })),
        )
}

pub(super) fn linux_window_controls(window: &mut Window, cx: &mut Context<FrameRoot>) -> gpui::Div {
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
                window,
                cx,
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
                window,
                cx,
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
                window,
                cx,
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
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
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
    let colors = ButtonColors {
        background: theme::TRANSPARENT,
        hover_background,
        active_background,
        foreground,
        hover_foreground,
        opacity: 1.0,
    };
    let animated = animated_button_colors(id, colors, window, cx);
    let background = animated.background;
    let icon_color = animated.foreground;
    let hover_transition = animated.hover_transition;

    div()
        .id(id)
        .w(px(width))
        .h(px(height))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(radius))
        .bg(background)
        .text_color(icon_color)
        .hover(|style| style.cursor_pointer())
        .active(move |style| style.bg(color(active_background)))
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .child(icon_svg(icon, icon_size, icon_color))
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
    window: &mut Window,
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
            window,
            cx,
        ))
        .child(titlebar_segment(
            assets::ICON_TERMINAL,
            "LOGS",
            ActiveView::Logs,
            active_view == ActiveView::Logs,
            window,
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
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let colors = button_colors(ButtonVariant::Secondary, selected, true);
    let segment_id = match view {
        ActiveView::Workspace => "titlebar-workspace",
        ActiveView::Logs => "titlebar-logs",
    };
    let hover_transition = hover_motion(format!("{segment_id}-hover"), window, cx);
    let hover_progress = *hover_transition.evaluate(window, cx);
    let background = if selected {
        mix_color(colors.background, colors.hover_background, hover_progress)
    } else {
        mix_color(theme::TRANSPARENT, theme::FRAME_GRAY_100, hover_progress)
    };
    let foreground = mix_color(
        if selected {
            theme::FOREGROUND
        } else {
            theme::FRAME_GRAY_600
        },
        theme::FOREGROUND,
        hover_progress,
    );

    div()
        .h(px(TITLEBAR_NAV_BUTTON_HEIGHT))
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_SM))
        .id(segment_id)
        .group(segment_id)
        .px_2()
        .bg(background)
        .text_color(foreground)
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .hover(|style| style.cursor_pointer())
        .active(move |style| style.bg(color(colors.active_background)))
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover, cx);
        })
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
        .child(icon_svg(icon, TITLEBAR_ICON_SIZE, foreground))
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
