use super::accessibility::{
    apply_accessible_button, apply_accessible_button_with_focus, focus_visible_ring,
    handle_modal_tab_navigation,
};
use super::components::{
    frame_checkbox_row_with_focus, frame_text_button, frame_text_button_with_focus,
    frame_vertical_scrollbar,
};
use super::input::{FrameTextInputSpec, frame_text_input};
use super::primitives::{
    ButtonColors, ButtonVariant, action_button, animated_button_colors, button_colors,
    button_highlight_shadows, button_mouse_down, card_surface_shadows, color, icon_svg,
    input_highlight_shadows, panel_bottom_separator, vertical_separator,
};
use super::settings_panel::{settings_hint_text, settings_section};
use super::{
    ActiveView, ClickEvent, Context, ExternalPaths, FILE_LIST_ACTION_ICON_SIZE, FluentBuilder,
    FocusHandle, FrameAppState, FrameRoot, FrameTextInputKind, InteractiveElement, IntoElement,
    LEFT_COLUMN_SPAN, MouseButton, PANEL_HEADER_HEIGHT, ParentElement, RIGHT_COLUMN_SPAN,
    SETTINGS_CONTROL_HEIGHT, SETTINGS_SHEET_MOTION_DURATION, ScrollHandle,
    StatefulInteractiveElement, Styled, TITLEBAR_ACTION_ICON_SIZE, TITLEBAR_DIVIDER_HEIGHT,
    TITLEBAR_HEIGHT, TITLEBAR_ICON_SIZE, TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
    TITLEBAR_LINUX_WINDOW_CONTROLS_GAP, TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X,
    TITLEBAR_LOGO_SIZE, TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_PLACEHOLDER_WIDTH,
    TITLEBAR_NAV_BUTTON_HEIGHT, TITLEBAR_PLATFORM_DIVIDER_HEIGHT, TITLEBAR_SEGMENT_HEIGHT,
    TITLEBAR_TOP_PADDING, TITLEBAR_TRAFFIC_LIGHT_SIZE, TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
    TITLEBAR_WINDOWS_WINDOW_ICON_SIZE, TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE, UpdateInfo,
    UpdateStatus, WORKSPACE_COLUMNS, WORKSPACE_GAP, Window, WindowControlArea, assets, div,
    ease_out_quint, format_total_size, hover_motion, mix_color, motion_is_hidden, motion_target,
    px, relative, retarget_hover_motion, set_motion_target, settings_sheet_right_inset, svg, theme,
};
use gpui::{HighlightStyle, StyledText};

const MAX_RELEASE_NOTES_CHARS: usize = 8_000;
const UPDATE_RELEASE_NOTES_MAX_HEIGHT: f32 = 360.0;
const UPDATE_RELEASE_NOTES_MIN_HEIGHT: f32 = 180.0;
const UPDATE_RELEASE_NOTES_PADDING_Y: f32 = 24.0;
const UPDATE_RELEASE_NOTES_LINE_HEIGHT: f32 = 16.0;
const UPDATE_RELEASE_NOTES_LINE_PADDING_BOTTOM: f32 = 4.0;
const UPDATE_RELEASE_NOTES_BLANK_LINE_HEIGHT: f32 = 8.0;

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
    let show_workspace_controls = titlebar_shows_workspace_controls(state);

    titlebar_drag_surface()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .pt(px(TITLEBAR_TOP_PADDING))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(
            div()
                .flex()
                .items_center()
                .mt_2()
                .gap_6()
                .child(macos_native_window_controls_placeholder())
                .when(show_workspace_controls, |this| {
                    this.child(frame_logo())
                        .child(titlebar_divider())
                        .child(titlebar_navigation(state.active_view, window, cx))
                        .child(titlebar_divider())
                        .child(titlebar_stats(state))
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .mt_2()
                .gap_2()
                .when(show_workspace_controls, |this| {
                    this.child(titlebar_settings_button(window, cx))
                        .child(titlebar_add_source_button(window, cx))
                        .child(titlebar_start_button(state, window, cx))
                }),
        )
}

pub(super) fn windows_titlebar(
    state: FrameAppState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_drag_surface()
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(platform_titlebar_content(state, window, cx))
        .child(windows_window_controls(window, cx))
}

pub(super) fn linux_titlebar(
    state: FrameAppState,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_drag_surface()
        .relative()
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .child(platform_titlebar_content(state, window, cx))
        .child(linux_window_controls(window, cx))
}

fn titlebar_drag_surface() -> gpui::Div {
    // The root view has a full-window focus hitbox for keyboard navigation.
    // Keep it out of titlebar mouse dispatch so it cannot prevent native window moves.
    div().window_control_area(WindowControlArea::Drag).occlude()
}

pub(super) fn platform_titlebar_content(
    state: FrameAppState,
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
                .gap(px(WORKSPACE_GAP))
                .w_full()
                .child(
                    div()
                        .col_span(LEFT_COLUMN_SPAN)
                        .mt_2()
                        .flex()
                        .items_center()
                        .gap_6()
                        .when(show_workspace_controls, |this| {
                            this.child(platform_frame_logo())
                                .child(platform_titlebar_divider())
                                .child(titlebar_navigation(state.active_view, window, cx))
                                .child(platform_titlebar_divider())
                                .child(titlebar_stats(state))
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
                            this.child(titlebar_settings_button(window, cx))
                                .child(titlebar_add_source_button(window, cx))
                                .child(titlebar_start_button(state, window, cx))
                        }),
                ),
        )
}

const fn titlebar_shows_workspace_controls(state: FrameAppState) -> bool {
    state.file_count > 0
}

pub(super) fn titlebar_settings_button(
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    action_button(
        "titlebar-settings",
        assets::ICON_SETTINGS,
        None,
        "Settings",
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
        Some("Add source"),
        "Add source",
        ButtonVariant::Secondary,
        true,
        window,
        cx,
    )
    .on_click(cx.listener(|_root, _: &ClickEvent, window, cx| {
        cx.stop_propagation();
        FrameRoot::prompt_add_source(window, cx);
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
            "Processing"
        } else {
            "Start"
        }),
        if state.is_processing {
            "Processing"
        } else {
            "Start conversion"
        },
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

#[derive(Clone, Copy)]
pub(super) struct AppSettingsSheetProps<'a> {
    pub(super) is_open: bool,
    pub(super) current_max_concurrency: usize,
    pub(super) draft_max_concurrency: &'a str,
    pub(super) error: Option<&'a str>,
    pub(super) default_output_directory: Option<&'a str>,
    pub(super) output_directory_error: Option<&'a str>,
    pub(super) auto_update_check: bool,
    pub(super) update_status: &'a UpdateStatus,
    pub(super) value_focus: &'a FocusHandle,
    pub(super) output_directory_focus: &'a FocusHandle,
    pub(super) auto_update_focus: &'a FocusHandle,
    pub(super) check_now_focus: &'a FocusHandle,
    pub(super) download_focus: &'a FocusHandle,
    pub(super) skip_focus: &'a FocusHandle,
    pub(super) install_focus: &'a FocusHandle,
    pub(super) panel_focus: &'a FocusHandle,
    pub(super) close_focus: &'a FocusHandle,
    pub(super) last_focus: &'a FocusHandle,
}

#[expect(
    clippy::too_many_lines,
    reason = "The settings sheet is a declarative GPUI layout kept together to preserve visual structure."
)]
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
    set_motion_target(&transition, target, cx);
    let progress = *transition.evaluate(window, cx);
    let right_inset = settings_sheet_right_inset(progress);
    let first_focus = props.close_focus.clone();
    let last_focus = props.last_focus.clone();

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
        .on_key_down(cx.listener(
            move |root, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => {
                        root.close_app_settings();
                        root.restore_focus_after_settings_close(window, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "tab" => {
                        handle_modal_tab_navigation(event, &first_focus, &last_focus, window, cx);
                    }
                    _ => {}
                }
            },
        ))
        .child(
            div()
                .id("app-settings-backdrop")
                .absolute()
                .inset_0()
                .bg(color(theme::BACKGROUND.with_alpha(0.60 * progress)))
                .backdrop_blur(px(4.0 * progress))
                .occlude()
                .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    root.close_app_settings();
                    root.restore_focus_after_settings_close(window, cx);
                    cx.notify();
                })),
        )
        .child(
            div()
                .id("app-settings-panel")
                .role(gpui::Role::Dialog)
                .aria_label("Settings")
                .track_focus(props.panel_focus)
                .tab_stop(false)
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
                        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                        .text_color(color(theme::FOREGROUND))
                        .child(theme::ui_text("Settings"))
                        .child(
                            app_settings_close_button(
                                "app-settings-close",
                                "Close settings",
                                props.close_focus,
                                window,
                                cx,
                            )
                            .on_click(
                                cx.listener(|root, _: &ClickEvent, window, cx| {
                                    cx.stop_propagation();
                                    root.close_app_settings();
                                    root.restore_focus_after_settings_close(window, cx);
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
                        .child(app_settings_output_directory_section(
                            props.default_output_directory,
                            props.output_directory_error,
                            props.output_directory_focus,
                            window,
                            cx,
                        ))
                        .child(
                            settings_section("Max concurrency")
                                .child(app_settings_concurrency_control(
                                    props.draft_max_concurrency,
                                    draft_is_dirty,
                                    props.error,
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
                                    .role(gpui::Role::Alert)
                                    .aria_label(error.clone())
                                    .text_color(color(theme::FRAME_RED))
                                    .child(error),
                            )
                        })
                        .child(app_settings_updates_section(
                            props.auto_update_check,
                            props.update_status,
                            AppSettingsUpdateFocuses {
                                auto_update: props.auto_update_focus,
                                check_now: props.check_now_focus,
                                download: props.download_focus,
                                skip: props.skip_focus,
                                install: props.install_focus,
                            },
                            window,
                            cx,
                        )),
                ),
        )
}

fn app_settings_output_directory_section(
    default_output_directory: Option<&str>,
    error: Option<&str>,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let selected_path = default_output_directory
        .unwrap_or("No folder selected")
        .to_string();
    let button_label = if default_output_directory.is_some() {
        "Change default output folder"
    } else {
        "Choose default output folder"
    };

    let mut section = settings_section("Output folder")
        .child(
            frame_text_button_with_focus(
                "app-settings-output-directory",
                button_label,
                ButtonVariant::Secondary,
                false,
                true,
                focus,
                window,
                cx,
            )
            .w_full()
            .on_click(cx.listener(|_root, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                FrameRoot::prompt_default_output_folder(window, cx);
            })),
        )
        .child(
            div()
                .id("app-settings-output-directory-path")
                .overflow_hidden()
                .text_color(color(theme::FRAME_GRAY_600))
                .child(selected_path),
        );

    if let Some(error) = error {
        section = section.child(
            div()
                .id("app-settings-output-directory-error")
                .role(gpui::Role::Alert)
                .aria_label(error.to_string())
                .text_color(color(theme::FRAME_RED))
                .child(error.to_string()),
        );
    }

    section
}

#[derive(Clone, Copy)]
struct AppSettingsUpdateFocuses<'a> {
    auto_update: &'a FocusHandle,
    check_now: &'a FocusHandle,
    download: &'a FocusHandle,
    skip: &'a FocusHandle,
    install: &'a FocusHandle,
}

fn app_settings_updates_section(
    auto_update_check: bool,
    update_status: &UpdateStatus,
    focuses: AppSettingsUpdateFocuses<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let busy = update_status.is_busy();
    let mut section = settings_section("Updates")
        .child(frame_checkbox_row_with_focus(
            "app-settings-auto-update-check",
            "Check automatically",
            "Frame checks for signed releases in the background.",
            auto_update_check,
            false,
            focuses.auto_update,
            cx,
            |root, _event, _window, cx| {
                if root.toggle_auto_update_check(cx) {
                    cx.notify();
                }
            },
        ))
        .child(update_check_now_button(busy, focuses.check_now, window, cx))
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

    if let Some(row) = update_action_row(update_status, focuses, window, cx) {
        section = section.child(row);
    }

    section
}

fn update_status_label(status: &UpdateStatus) -> gpui::Stateful<gpui::Div> {
    let tone = match status {
        UpdateStatus::Error(_) => theme::FRAME_RED,
        UpdateStatus::Disabled(_) => theme::FRAME_AMBER,
        _ => theme::FRAME_GRAY_600,
    };
    let text = update_status_text(status);

    div()
        .id("app-settings-update-status")
        .role(gpui::Role::Status)
        .aria_label(text.clone())
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(tone))
        .child(theme::ui_text_owned(text))
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

fn update_release_notes_block(
    notes: &str,
    scroll_handle: &ScrollHandle,
) -> gpui::Stateful<gpui::Div> {
    let lines = normalized_release_note_lines(notes);
    let content_height = update_release_notes_content_height(&lines);
    let mut content = div()
        .id("update-dialog-release-notes-content")
        .min_h(px(UPDATE_RELEASE_NOTES_MIN_HEIGHT))
        .max_h(px(UPDATE_RELEASE_NOTES_MAX_HEIGHT))
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .p_3()
        .pr_5();

    for line in lines {
        content = content.child(update_release_note_line(&line));
    }

    div()
        .id("update-dialog-release-notes")
        .relative()
        .min_h(px(UPDATE_RELEASE_NOTES_MIN_HEIGHT))
        .max_h(px(UPDATE_RELEASE_NOTES_MAX_HEIGHT))
        .overflow_hidden()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::FRAME_GRAY_100))
        .child(content)
        .child(frame_vertical_scrollbar(
            "update-dialog-release-notes-scrollbar",
            scroll_handle.clone(),
            content_height,
        ))
}

fn update_release_notes_content_height(lines: &[String]) -> f32 {
    UPDATE_RELEASE_NOTES_PADDING_Y
        + lines.iter().fold(0.0, |height, line| {
            if line.trim().is_empty() {
                height + UPDATE_RELEASE_NOTES_BLANK_LINE_HEIGHT
            } else {
                height + UPDATE_RELEASE_NOTES_LINE_HEIGHT + UPDATE_RELEASE_NOTES_LINE_PADDING_BOTTOM
            }
        })
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
                theme::TEXT_WEIGHT_MEDIUM,
            )
        } else if let Some(bullet) = bullet {
            (
                format!("• {bullet}"),
                8.0,
                theme::FRAME_GRAY_600,
                theme::TEXT_WEIGHT_REGULAR,
            )
        } else {
            (
                trimmed.to_string(),
                0.0,
                theme::FRAME_GRAY_600,
                theme::TEXT_WEIGHT_REGULAR,
            )
        };
    let (text, highlights) = parse_update_release_note_emphasis(&text);

    let mut line = div()
        .pl(px(left_padding))
        .pb(px(4.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .line_height(px(16.0))
        .text_color(color(text_color))
        .font_weight(font_weight);

    if highlights.is_empty() {
        line = line.child(text);
    } else {
        line = line.child(StyledText::new(text).with_highlights(highlights));
    }

    line
}

fn parse_update_release_note_emphasis(
    input: &str,
) -> (String, Vec<(std::ops::Range<usize>, HighlightStyle)>) {
    let mut text = String::with_capacity(input.len());
    let mut highlights = Vec::new();
    let mut rest = input;
    let highlight_style = HighlightStyle {
        color: Some(color(theme::FOREGROUND).into()),
        font_weight: Some(theme::TEXT_WEIGHT_MEDIUM),
        ..HighlightStyle::default()
    };

    loop {
        let Some(start) = rest.find("**") else {
            text.push_str(rest);
            break;
        };
        text.push_str(&rest[..start]);

        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("**") else {
            text.push_str(&rest[start..]);
            break;
        };

        let highlight_start = text.len();
        text.push_str(&after_start[..end]);
        let highlight_end = text.len();
        if highlight_start < highlight_end {
            highlights.push((highlight_start..highlight_end, highlight_style));
        }
        rest = &after_start[end + 2..];
    }

    (text, highlights)
}

fn update_progress_bar(progress_percent: Option<u8>) -> gpui::Stateful<gpui::Div> {
    let fraction = progress_percent.map_or(0.0, |percent| f32::from(percent) / 100.0);
    let numeric_percent = progress_percent.map_or(0.0, f64::from);
    let value_text = progress_percent.map_or_else(
        || "Download progress unknown".to_string(),
        |percent| format!("{percent}%"),
    );

    div()
        .id("app-settings-update-progress")
        .role(gpui::Role::ProgressIndicator)
        .aria_label("Update download progress")
        .aria_numeric_value(numeric_percent)
        .aria_min_numeric_value(0.0)
        .aria_max_numeric_value(100.0)
        .aria_value(value_text)
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
    focuses: AppSettingsUpdateFocuses<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> Option<gpui::Div> {
    match status {
        UpdateStatus::Available(_) => Some(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    frame_text_button_with_focus(
                        "app-settings-update-download",
                        "Download",
                        ButtonVariant::Default,
                        false,
                        true,
                        focuses.download,
                        window,
                        cx,
                    )
                    .on_click(cx.listener(
                        |root, _: &ClickEvent, _window, cx| {
                            cx.stop_propagation();
                            root.download_available_update(cx);
                            cx.notify();
                        },
                    )),
                )
                .child(
                    frame_text_button_with_focus(
                        "app-settings-update-skip",
                        "Skip",
                        ButtonVariant::Secondary,
                        false,
                        true,
                        focuses.skip,
                        window,
                        cx,
                    )
                    .on_click(cx.listener(
                        |root, _: &ClickEvent, _window, cx| {
                            cx.stop_propagation();
                            if root.skip_available_update(cx) {
                                cx.notify();
                            }
                        },
                    )),
                ),
        ),
        UpdateStatus::ReadyToInstall(_) => Some(
            div().flex().items_center().gap_2().child(
                frame_text_button_with_focus(
                    "app-settings-update-install",
                    "Install and restart",
                    ButtonVariant::Default,
                    false,
                    true,
                    focuses.install,
                    window,
                    cx,
                )
                .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    root.install_downloaded_update(cx);
                    cx.notify();
                })),
            ),
        ),
        UpdateStatus::UpToDate | UpdateStatus::Disabled(_) | UpdateStatus::Error(_) => None,
        UpdateStatus::Idle
        | UpdateStatus::Checking
        | UpdateStatus::Downloading { .. }
        | UpdateStatus::Installing => None,
    }
}

fn update_check_now_button(
    busy: bool,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    frame_text_button_with_focus(
        "app-settings-update-check-now",
        "Check now",
        ButtonVariant::Secondary,
        false,
        !busy,
        focus,
        window,
        cx,
    )
    .w_full()
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if !busy {
            root.check_for_updates(true, cx);
            cx.notify();
        }
    }))
}

#[derive(Clone, Copy)]
pub(super) struct UpdateDialogView<'a> {
    pub(super) status: &'a UpdateStatus,
    pub(super) info: Option<&'a UpdateInfo>,
    pub(super) release_notes_scroll_handle: &'a ScrollHandle,
    pub(super) panel_focus: &'a FocusHandle,
    pub(super) close_focus: &'a FocusHandle,
}

pub(super) fn update_dialog(
    is_open: bool,
    view: UpdateDialogView<'_>,
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
        .on_key_down(cx.listener(|root, event: &gpui::KeyDownEvent, window, cx| {
            match event.keystroke.key.as_str() {
                "escape" if !root.update_ui.status.is_busy() => {
                    root.close_update_dialog();
                    root.restore_focus_after_update_dialog_close(window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
                "tab" => {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
                _ => {}
            }
        }))
        .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if !root.update_ui.status.is_busy() {
                root.close_update_dialog();
                root.restore_focus_after_update_dialog_close(window, cx);
                cx.notify();
            }
        }))
        .child(update_dialog_panel(panel_offset, view, window, cx))
}

fn update_dialog_panel(
    panel_offset: f32,
    view: UpdateDialogView<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let mut panel = div()
        .id("update-dialog-panel")
        .role(gpui::Role::AlertDialog)
        .aria_label(update_dialog_title(view.status))
        .track_focus(view.panel_focus)
        .tab_stop(false)
        .mt(px(panel_offset))
        .w_full()
        .max_w(px(640.0))
        .max_h(relative(0.86))
        .overflow_hidden()
        .rounded(px(theme::RADIUS_LG))
        .bg(color(theme::SIDEBAR))
        .shadow(card_surface_shadows())
        .occlude()
        .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
        }))
        .child(update_dialog_header(
            view.status,
            view.close_focus,
            window,
            cx,
        ))
        .child(update_dialog_body(
            view.status,
            view.info,
            view.release_notes_scroll_handle,
        ))
        .child(update_dialog_footer(view.status, window, cx));

    if matches!(view.status, UpdateStatus::Downloading { .. }) {
        panel = panel.child(update_dialog_download_state(view.status));
    }

    panel
}

fn update_dialog_header(
    status: &UpdateStatus,
    close_focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut title_stack = div().flex().flex_col().gap_1();
    if let Some(kicker) = update_dialog_kicker(status) {
        title_stack = title_stack.child(
            div()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                .text_color(color(theme::FRAME_GRAY_600))
                .child(theme::ui_text(kicker)),
        );
    }
    title_stack = title_stack.child(
        div()
            .text_size(px(theme::TEXT_LABEL_SIZE))
            .font_weight(theme::TEXT_WEIGHT_MEDIUM)
            .text_color(color(theme::FOREGROUND))
            .child(theme::ui_text_owned(update_dialog_title(status))),
    );

    div()
        .relative()
        .h(px(PANEL_HEADER_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .child(title_stack)
        .child(
            app_settings_close_button(
                "update-dialog-close",
                "Close update dialog",
                close_focus,
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                root.close_update_dialog();
                root.restore_focus_after_update_dialog_close(window, cx);
                cx.notify();
            })),
        )
        .child(panel_bottom_separator())
}

fn update_dialog_body(
    status: &UpdateStatus,
    info: Option<&UpdateInfo>,
    release_notes_scroll_handle: &ScrollHandle,
) -> gpui::Div {
    let notes = update_release_notes_text(info);
    let mut body = div().flex().flex_col().gap_3().p_4();

    if let Some(summary) = update_dialog_summary(status, notes.is_some()) {
        body = body.child(
            div()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .line_height(px(16.0))
                .text_color(color(theme::FRAME_GRAY_600))
                .child(theme::ui_text_owned(summary)),
        );
    }

    if let Some(notes) = notes.as_deref() {
        body = body.child(update_release_notes_block(
            notes,
            release_notes_scroll_handle,
        ));
    }

    if let UpdateStatus::Error(error) = status {
        body = body.child(
            div()
                .id("update-dialog-error-alert")
                .role(gpui::Role::Alert)
                .aria_label(error.clone())
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
        .w_full()
        .flex()
        .items_center()
        .justify_end()
        .gap_3()
        .pb_4()
        .px_4()
        .child(
            frame_text_button(
                "update-dialog-later",
                "Later",
                ButtonVariant::Ghost,
                false,
                !status.is_busy(),
                window,
                cx,
            )
            .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                if !root.update_ui.status.is_busy() {
                    root.close_update_dialog();
                    root.restore_focus_after_update_dialog_close(window, cx);
                    cx.notify();
                }
            })),
        )
        .child(update_dialog_primary_action(status, window, cx))
}

fn update_dialog_primary_action(
    status: &UpdateStatus,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    match status {
        UpdateStatus::Available(_) => action_button(
            "update-dialog-download",
            assets::ICON_DOWNLOAD_02,
            Some("Download"),
            "Download",
            ButtonVariant::Default,
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
            "Install and restart",
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
            "Dismiss",
            ButtonVariant::Secondary,
            false,
            true,
            window,
            cx,
        )
        .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            root.dismiss_update_status();
            root.restore_focus_after_update_dialog_close(window, cx);
            cx.notify();
        })),
        UpdateStatus::Downloading { .. } | UpdateStatus::Installing => frame_text_button(
            "update-dialog-busy",
            "Working",
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
            "Close",
            ButtonVariant::Secondary,
            false,
            true,
            window,
            cx,
        )
        .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            root.close_update_dialog();
            root.restore_focus_after_update_dialog_close(window, cx);
            cx.notify();
        })),
    }
}

const fn update_dialog_kicker(status: &UpdateStatus) -> Option<&'static str> {
    match status {
        UpdateStatus::Available(_) => None,
        UpdateStatus::Downloading { .. } => Some("Downloading update"),
        UpdateStatus::ReadyToInstall(_) => Some("Ready to install"),
        UpdateStatus::Installing => Some("Installing update"),
        UpdateStatus::Error(_) => Some("Update error"),
        UpdateStatus::Checking => Some("Checking for updates"),
        UpdateStatus::UpToDate => Some("No update available"),
        UpdateStatus::Disabled(_) => Some("Updates disabled"),
        UpdateStatus::Idle => Some("Updates"),
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

fn update_dialog_summary(status: &UpdateStatus, has_notes: bool) -> Option<String> {
    match status {
        UpdateStatus::Available(_) if has_notes => None,
        UpdateStatus::Available(_) => Some(
            "A signed update is available, but this release did not include notes.".to_string()
        ),
        UpdateStatus::Downloading { .. } => Some(
            "Keep Frame open while the update package is downloaded and verified.".to_string()
        ),
        UpdateStatus::ReadyToInstall(_) => Some(
            "The update was downloaded and verified. Frame will restart to finish installation."
                .to_string()
        ),
        UpdateStatus::Installing => Some(
            "Frame is handing installation to the bundled update helper.".to_string()
        ),
        UpdateStatus::Error(_) => Some(
            "The updater stopped before installation completed. You can dismiss this and try again."
                .to_string()
        ),
        UpdateStatus::Checking => Some(
            "Frame is checking the latest signed release manifest.".to_string()
        ),
        UpdateStatus::UpToDate => Some("No newer signed release is available.".to_string()),
        UpdateStatus::Disabled(explanation) => Some(explanation.clone()),
        UpdateStatus::Idle => Some("No update check is running.".to_string()),
    }
}

pub(super) fn app_settings_concurrency_control(
    draft_max_concurrency: &str,
    can_apply: bool,
    error: Option<&str>,
    value_focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let input = frame_text_input(
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
    )
    .when_some(error.map(str::to_string), |this, error| {
        this.aria_invalid(true).aria_description(error)
    });

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().flex_1().min_w_0().child(input))
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
    id: &'static str,
    label: &'static str,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Ghost, false, true);
    let animated = animated_button_colors(id, colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;

    let button = div()
        .id(id)
        .group(id)
        .w(px(SETTINGS_CONTROL_HEIGHT))
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(background)
        .text_color(foreground)
        .hover(gpui::Styled::cursor_pointer)
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
        ));

    apply_accessible_button_with_focus(button, label, true, focus)
}

pub(super) fn app_settings_apply_button(
    enabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        "app-settings-max-concurrency-apply",
        "Apply",
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
            FrameRoot::import_source_paths(paths.paths().to_vec(), cx);
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
                        .child(theme::ui_text("Import source files")),
                ),
        )
}

pub(super) fn macos_native_window_controls_placeholder() -> gpui::Div {
    div()
        .w(px(TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_PLACEHOLDER_WIDTH))
        .h(px(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .mr_2()
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
                "Minimize window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_WINDOWS_WINDOW_ICON_SIZE,
                    width: TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                    height: TITLEBAR_HEIGHT,
                    radius: 0.0,
                },
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
                "Maximize window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE,
                    width: TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                    height: TITLEBAR_HEIGHT,
                    radius: 0.0,
                },
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
                "Close window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_WINDOWS_WINDOW_ICON_SIZE,
                    width: TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
                    height: TITLEBAR_HEIGHT,
                    radius: 0.0,
                },
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
                "Minimize window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_ACTION_ICON_SIZE,
                    width: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    height: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    radius: theme::RADIUS_SM,
                },
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
                "Maximize window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_ACTION_ICON_SIZE,
                    width: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    height: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    radius: theme::RADIUS_SM,
                },
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
                "Close window",
                TitlebarWindowButtonMetrics {
                    icon_size: TITLEBAR_ACTION_ICON_SIZE,
                    width: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    height: TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
                    radius: theme::RADIUS_SM,
                },
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TitlebarWindowButtonMetrics {
    pub(super) icon_size: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) radius: f32,
}

pub(super) fn titlebar_window_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    metrics: TitlebarWindowButtonMetrics,
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

    let button = div()
        .id(id)
        .w(px(metrics.width))
        .h(px(metrics.height))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(metrics.radius))
        .bg(background)
        .text_color(icon_color)
        .hover(gpui::Styled::cursor_pointer)
        .active(move |style| style.bg(color(active_background)))
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover, cx);
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(true, window, cx);
        })
        .child(icon_svg(icon, metrics.icon_size, icon_color));

    apply_accessible_button(button, label, true).tab_stop(false)
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
) -> gpui::Stateful<gpui::Div> {
    div()
        .id("titlebar-main-view-tabs")
        .role(gpui::Role::TabList)
        .aria_label("Main view")
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
            "Workspace",
            ActiveView::Workspace,
            active_view == ActiveView::Workspace,
            window,
            cx,
        ))
        .child(titlebar_segment(
            assets::ICON_TERMINAL,
            "Logs",
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
            format!("Storage {}", format_total_size(state.total_size_bytes)),
        ))
        .child(titlebar_stat(
            assets::ICON_FILE_VIDEO,
            format!("Items {}", state.file_count),
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
        .child(theme::ui_text_owned(label))
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
        .id(segment_id)
        .h(px(TITLEBAR_NAV_BUTTON_HEIGHT))
        .role(gpui::Role::Tab)
        .aria_label(label)
        .aria_selected(selected)
        .focusable()
        .tab_stop(true)
        .focus_visible(focus_visible_ring)
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(theme::RADIUS_SM))
        .group(segment_id)
        .px_2()
        .bg(background)
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .when(selected, |this| this.shadow(button_highlight_shadows()))
        .hover(gpui::Styled::cursor_pointer)
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
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                let Some(next_view) = titlebar_view_for_key(view, event.keystroke.key.as_str())
                else {
                    return;
                };
                if root.active_view != next_view {
                    root.active_view = next_view;
                    cx.notify();
                }
                cx.stop_propagation();
            }),
        )
        .child(icon_svg(icon, TITLEBAR_ICON_SIZE, foreground))
        .child(theme::ui_text(label))
}

fn titlebar_view_for_key(current: ActiveView, key: &str) -> Option<ActiveView> {
    match key {
        "left" | "right" | "up" | "down" => Some(match current {
            ActiveView::Workspace => ActiveView::Logs,
            ActiveView::Logs => ActiveView::Workspace,
        }),
        "home" => Some(ActiveView::Workspace),
        "end" => Some(ActiveView::Logs),
        _ => None,
    }
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

    #[test]
    fn titlebar_view_for_key_switches_between_main_tabs() {
        assert_eq!(
            titlebar_view_for_key(ActiveView::Workspace, "right"),
            Some(ActiveView::Logs)
        );
        assert_eq!(
            titlebar_view_for_key(ActiveView::Logs, "left"),
            Some(ActiveView::Workspace)
        );
        assert_eq!(
            titlebar_view_for_key(ActiveView::Logs, "home"),
            Some(ActiveView::Workspace)
        );
        assert_eq!(titlebar_view_for_key(ActiveView::Logs, "space"), None);
    }

    #[test]
    fn titlebar_workspace_controls_are_hidden_without_files() {
        assert!(!titlebar_shows_workspace_controls(FrameAppState::default()));
    }

    #[test]
    fn titlebar_workspace_controls_are_visible_with_files() {
        let state = FrameAppState {
            file_count: 1,
            ..FrameAppState::default()
        };

        assert!(titlebar_shows_workspace_controls(state));
    }

    #[test]
    fn release_note_emphasis_strips_markers_and_highlights_range() {
        let (text, highlights) =
            parse_update_release_note_emphasis("• **Native GPUI Application:** Rebuilt Frame");

        assert_eq!(text, "• Native GPUI Application: Rebuilt Frame");
        assert_eq!(highlights.len(), 1);
        assert_eq!(&text[highlights[0].0.clone()], "Native GPUI Application:");
        assert_eq!(highlights[0].1.color, Some(color(theme::FOREGROUND).into()));
        assert_eq!(highlights[0].1.font_weight, Some(theme::TEXT_WEIGHT_MEDIUM));
    }

    #[test]
    fn release_note_emphasis_keeps_unclosed_markers_literal() {
        let (text, highlights) =
            parse_update_release_note_emphasis("• **Native GPUI Application: Rebuilt Frame");

        assert_eq!(text, "• **Native GPUI Application: Rebuilt Frame");
        assert!(highlights.is_empty());
    }
}
