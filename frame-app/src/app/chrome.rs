use super::accessibility::{
    apply_accessible_button, apply_accessible_button_with_focus, focus_visible_ring,
    handle_modal_tab_navigation,
};
use super::components::{
    FRAME_SELECT_MAX_HEIGHT, apply_frame_select_popover_focus_trap, frame_checkbox_row_with_focus,
    frame_select_content_height, frame_select_option, frame_select_option_with_focus,
    frame_select_options_list, frame_select_popover, frame_select_target_index,
    frame_select_trigger_with_focus, frame_text_button, frame_text_button_with_focus,
    frame_vertical_scrollbar,
};
use super::input::{FrameTextInputSpec, frame_text_input};
use super::primitives::{
    ButtonColors, ButtonVariant, action_button, animated_button_colors, apply_button_motion,
    button_colors, button_highlight_shadows, button_motion, card_surface_shadows, color, icon_svg,
    input_highlight_shadows, panel_bottom_separator, vertical_separator,
};
use super::settings_panel::{settings_field_label, settings_hint_text, settings_section};
use super::{
    ActiveView, AppearancePopover, AppearanceSettings, ClickEvent, ColorTheme, Context,
    ExternalPaths, FILE_LIST_ACTION_ICON_SIZE, FRAME_APP_VERSION, FluentBuilder, FocusHandle,
    FrameAppState, FrameRoot, FrameTextInputKind, INTERACTION_MOTION_DURATION, InteractiveElement,
    IntoElement, LEFT_COLUMN_SPAN, MouseButton, PANEL_HEADER_HEIGHT, ParentElement, PopoverState,
    RIGHT_COLUMN_SPAN, SETTINGS_CONTROL_HEIGHT, SURFACE_MOTION_DURATION, ScalePreset, ScrollHandle,
    StatefulInteractiveElement, Styled, TITLEBAR_ACTION_ICON_SIZE, TITLEBAR_DIVIDER_HEIGHT,
    TITLEBAR_HEIGHT, TITLEBAR_ICON_SIZE, TITLEBAR_LINUX_WINDOW_BUTTON_SIZE,
    TITLEBAR_LINUX_WINDOW_CONTROLS_GAP, TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X,
    TITLEBAR_LOGO_SIZE, TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_PLACEHOLDER_WIDTH,
    TITLEBAR_NAV_BUTTON_HEIGHT, TITLEBAR_PLATFORM_DIVIDER_HEIGHT, TITLEBAR_SEGMENT_HEIGHT,
    TITLEBAR_TOP_PADDING, TITLEBAR_TRAFFIC_LIGHT_SIZE, TITLEBAR_WINDOWS_WINDOW_BUTTON_WIDTH,
    TITLEBAR_WINDOWS_WINDOW_ICON_SIZE, TITLEBAR_WINDOWS_WINDOW_MAX_ICON_SIZE,
    UPDATE_INSTALL_WAIT_MESSAGE, UpdateInfo, UpdateStatus, WORKSPACE_COLUMNS, WORKSPACE_GAP,
    Window, WindowControlArea, assets, div, ease_in_out, format_total_size, mix_color,
    motion_is_hidden, motion_target, px, relative, set_motion_target, settings_sheet_right_inset,
    subtitle_popover_slide_offset, svg, theme,
};
use gpui::{HighlightStyle, StyledText, deferred};

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
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_for_platform(FrameTitlebarPlatform::current(), state, palette, window, cx)
}

pub(super) fn titlebar_for_platform(
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

pub(super) fn macos_titlebar(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let show_workspace_controls = titlebar_shows_workspace_controls(state);

    titlebar_drag_surface(cx)
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
                        .child(titlebar_divider(palette))
                        .child(titlebar_stats(state, palette))
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

pub(super) fn windows_titlebar(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_drag_surface(cx)
        .relative()
        .h(theme::ui_rem(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .child(platform_titlebar_content(state, palette, window, cx))
        .child(windows_window_controls(palette, window, cx))
}

pub(super) fn linux_titlebar(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    titlebar_drag_surface(cx)
        .relative()
        .h(theme::ui_rem(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .child(platform_titlebar_content(state, palette, window, cx))
        .child(linux_window_controls(palette, window, cx))
}

fn titlebar_drag_surface(cx: &Context<FrameRoot>) -> gpui::Div {
    div()
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down_out(cx.listener(|root, _event, _window, _cx| {
            root.titlebar_drag.should_move = false;
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|root, _event, _window, _cx| {
                root.titlebar_drag.should_move = false;
            }),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|root, _event, _window, _cx| {
                root.titlebar_drag.should_move = true;
            }),
        )
        .on_mouse_move(cx.listener(|root, _event, window, _cx| {
            if root.titlebar_drag.should_move {
                root.titlebar_drag.should_move = false;
                window.start_window_move();
            }
        }))
}

pub(super) fn platform_titlebar_content(
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
                                .child(platform_titlebar_divider(palette))
                                .child(titlebar_stats(state, palette))
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

const fn titlebar_shows_workspace_controls(state: FrameAppState) -> bool {
    state.file_count > 0
}

pub(super) fn titlebar_settings_button(
    palette: &'static theme::ThemePalette,
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
        palette,
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

pub(super) fn titlebar_start_button(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
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
        palette,
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
    pub(super) appearance: AppearanceSettings,
    pub(super) palette: &'static theme::ThemePalette,
    pub(super) appearance_error: Option<&'a str>,
    pub(super) theme_popover: PopoverState,
    pub(super) ui_scale_popover: PopoverState,
    pub(super) scroll_handle: &'a ScrollHandle,
    pub(super) theme_scroll_handle: &'a ScrollHandle,
    pub(super) ui_scale_scroll_handle: &'a ScrollHandle,
    pub(super) theme_focuses: AppSettingsSelectFocuses<'a>,
    pub(super) ui_scale_focuses: AppSettingsSelectFocuses<'a>,
    pub(super) auto_update_check: bool,
    pub(super) update_status: &'a UpdateStatus,
    pub(super) update_install_ready: bool,
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

#[derive(Clone, Copy)]
pub(super) struct AppSettingsSelectFocuses<'a> {
    pub(super) trigger: &'a FocusHandle,
    pub(super) panel: &'a FocusHandle,
    pub(super) options: &'a [FocusHandle],
}

#[expect(
    clippy::too_many_lines,
    clippy::large_types_passed_by_value,
    reason = "The declarative settings render consumes a copyable bundle so all render inputs remain explicit."
)]
pub(super) fn app_settings_sheet(
    props: AppSettingsSheetProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let palette = props.palette;
    let draft_is_dirty =
        props.draft_max_concurrency.trim() != props.current_max_concurrency.to_string();
    let transition = window
        .use_keyed_transition(
            "app-settings-sheet-motion",
            cx,
            SURFACE_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_in_out);
    let target = motion_target(props.is_open);
    set_motion_target(&transition, target, cx);
    let progress = *transition.evaluate(window, cx);
    let right_inset = settings_sheet_right_inset(progress);
    let first_focus = props.close_focus.clone();
    let last_focus = props.last_focus.clone();
    let theme_trigger_focus = props.theme_focuses.trigger.clone();
    let ui_scale_trigger_focus = props.ui_scale_focuses.trigger.clone();

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
                        if root.settings_ui.theme_popover.is_open() {
                            root.close_app_settings_appearance_popover(AppearancePopover::Theme);
                            theme_trigger_focus.focus(window, cx);
                        } else if root.settings_ui.ui_scale_popover.is_open() {
                            root.close_app_settings_appearance_popover(AppearancePopover::UiScale);
                            ui_scale_trigger_focus.focus(window, cx);
                        } else {
                            root.close_app_settings();
                            root.restore_focus_after_settings_close(window, cx);
                        }
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
                .bg(color(palette.canvas.with_alpha(0.60 * progress)))
                .backdrop_blur(theme::ui_rem(4.0 * progress).to_pixels(window.rem_size()))
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
                .right(theme::ui_rem(right_inset))
                .bottom_2()
                .w(theme::ui_rem(360.0))
                .max_w(relative(0.95))
                .flex()
                .flex_col()
                .rounded(theme::ui_rem(theme::RADIUS_LG))
                .bg(color(palette.surface))
                .opacity(progress)
                .shadow(card_surface_shadows(palette))
                .occlude()
                .on_click(cx.listener(|root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if root.settings_ui.ui_scale_popover.is_open() {
                        root.close_app_settings_appearance_popover(AppearancePopover::UiScale);
                        cx.notify();
                    }
                    if root.settings_ui.theme_popover.is_open() {
                        root.close_app_settings_appearance_popover(AppearancePopover::Theme);
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .h(theme::ui_rem(PANEL_HEADER_HEIGHT))
                        .w_full()
                        .relative()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_4()
                        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                        .text_color(color(palette.text_primary))
                        .child(theme::ui_text("Settings"))
                        .child(
                            app_settings_close_button(
                                "app-settings-close",
                                "Close settings",
                                true,
                                props.close_focus,
                                palette,
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
                        .child(panel_bottom_separator(palette)),
                )
                .child(
                    div()
                        .id("app-settings-scroll-body")
                        .flex_1()
                        .flex()
                        .flex_col()
                        .overflow_y_scroll()
                        .track_scroll(props.scroll_handle)
                        .justify_between()
                        .p_4()
                        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_4()
                                .child(app_settings_output_directory_section(
                                    props.default_output_directory,
                                    props.output_directory_error,
                                    props.output_directory_focus,
                                    palette,
                                    window,
                                    cx,
                                ))
                                .child(app_settings_appearance_section(
                                    props.appearance,
                                    props.appearance_error,
                                    props.theme_popover,
                                    props.ui_scale_popover,
                                    props.theme_scroll_handle,
                                    props.ui_scale_scroll_handle,
                                    props.theme_focuses,
                                    props.ui_scale_focuses,
                                    palette,
                                    window,
                                    cx,
                                ))
                                .child(
                                    settings_section("Max concurrency", palette)
                                        .child(app_settings_concurrency_control(
                                            props.draft_max_concurrency,
                                            draft_is_dirty,
                                            props.error,
                                            props.value_focus,
                                            palette,
                                            window,
                                            cx,
                                        ))
                                        .child(settings_hint_text(
                                            "Controls how many queued conversions can run at the same time.",
                                            palette,
                                        )),
                                )
                                .when_some(props.error.map(str::to_string), |this, error| {
                                    this.child(
                                        div()
                                            .id("app-settings-max-concurrency-error")
                                            .role(gpui::Role::Alert)
                                            .aria_label(error.clone())
                                            .text_color(color(palette.danger))
                                            .child(error),
                                    )
                                })
                                .child(app_settings_updates_section(
                                    props.auto_update_check,
                                    props.update_status,
                                    props.update_install_ready,
                                    AppSettingsUpdateFocuses {
                                        auto_update: props.auto_update_focus,
                                        check_now: props.check_now_focus,
                                        download: props.download_focus,
                                        skip: props.skip_focus,
                                        install: props.install_focus,
                                    },
                                    palette,
                                    window,
                                    cx,
                                )),
                        )
                        .child(app_settings_version_label(palette)),
                ),
        )
}

fn app_settings_version_label(palette: &'static theme::ThemePalette) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .justify_end()
        .pt_4()
        .text_size(theme::ui_rem(11.0))
        .text_color(color(palette.border_subtle))
        .child(theme::ui_text_owned(format!("Frame v{FRAME_APP_VERSION}")))
}

fn app_settings_output_directory_section(
    default_output_directory: Option<&str>,
    error: Option<&str>,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
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

    let mut section = settings_section("Output folder", palette)
        .child(
            frame_text_button_with_focus(
                "app-settings-output-directory",
                button_label,
                ButtonVariant::Secondary,
                false,
                true,
                focus,
                palette,
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
                .text_color(color(palette.text_muted))
                .child(selected_path),
        );

    if let Some(error) = error {
        section = section.child(
            div()
                .id("app-settings-output-directory-error")
                .role(gpui::Role::Alert)
                .aria_label(error.to_string())
                .text_color(color(palette.danger))
                .child(error.to_string()),
        );
    }

    section
}

#[expect(
    clippy::too_many_arguments,
    reason = "The appearance section explicitly receives both independent select states, focus groups, and theme context."
)]
fn app_settings_appearance_section(
    appearance: AppearanceSettings,
    error: Option<&str>,
    theme_popover: PopoverState,
    ui_scale_popover: PopoverState,
    theme_scroll_handle: &ScrollHandle,
    ui_scale_scroll_handle: &ScrollHandle,
    theme_focuses: AppSettingsSelectFocuses<'_>,
    ui_scale_focuses: AppSettingsSelectFocuses<'_>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut section = settings_section("Appearance", palette)
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(app_settings_appearance_select(
                    AppSettingsAppearanceSelectProps {
                        selected: appearance.color_theme,
                        popover_state: theme_popover,
                        scroll_handle: theme_scroll_handle,
                        focuses: theme_focuses,
                        palette,
                    },
                    window,
                    cx,
                ))
                .child(app_settings_appearance_select(
                    AppSettingsAppearanceSelectProps {
                        selected: appearance.ui_scale,
                        popover_state: ui_scale_popover,
                        scroll_handle: ui_scale_scroll_handle,
                        focuses: ui_scale_focuses,
                        palette,
                    },
                    window,
                    cx,
                )),
        )
        .child(settings_hint_text(
            "Changes the color theme and size of the complete interface.",
            palette,
        ));

    if let Some(error) = error {
        section = section.child(
            div()
                .id("app-settings-appearance-error")
                .role(gpui::Role::Alert)
                .aria_label(error.to_string())
                .text_color(color(palette.danger))
                .child(error.to_string()),
        );
    }

    section
}

trait AppSettingsAppearanceValue: Copy + Eq + 'static {
    const LABEL: &'static str;
    const TRIGGER_ID: &'static str;
    const LIST_ID: &'static str;
    const PANEL_ID: &'static str;
    const SCROLLBAR_ID: &'static str;
    const MOTION_ID: &'static str;
    const POPOVER: AppearancePopover;

    fn options() -> &'static [Self];
    fn display(self) -> &'static str;
    fn option_id(self) -> String;
    fn apply(self, root: &mut FrameRoot, window: &mut Window);
}

impl AppSettingsAppearanceValue for ColorTheme {
    const LABEL: &'static str = "Theme";
    const TRIGGER_ID: &'static str = "app-settings-theme";
    const LIST_ID: &'static str = "app-settings-theme-options-list";
    const PANEL_ID: &'static str = "app-settings-theme-options";
    const SCROLLBAR_ID: &'static str = "app-settings-theme-scrollbar";
    const MOTION_ID: &'static str = "app-settings-theme-motion";
    const POPOVER: AppearancePopover = AppearancePopover::Theme;

    fn options() -> &'static [Self] {
        &Self::ALL
    }

    fn display(self) -> &'static str {
        self.display()
    }

    fn option_id(self) -> String {
        format!("app-settings-theme-option-{}", self.persisted())
    }

    fn apply(self, root: &mut FrameRoot, _window: &mut Window) {
        root.set_color_theme(self);
    }
}

impl AppSettingsAppearanceValue for ScalePreset {
    const LABEL: &'static str = "UI scale";
    const TRIGGER_ID: &'static str = "app-settings-ui-scale";
    const LIST_ID: &'static str = "app-settings-ui-scale-options-list";
    const PANEL_ID: &'static str = "app-settings-ui-scale-options";
    const SCROLLBAR_ID: &'static str = "app-settings-ui-scale-scrollbar";
    const MOTION_ID: &'static str = "app-settings-ui-scale-motion";
    const POPOVER: AppearancePopover = AppearancePopover::UiScale;

    fn options() -> &'static [Self] {
        &Self::ALL
    }

    fn display(self) -> &'static str {
        self.display()
    }

    fn option_id(self) -> String {
        format!("app-settings-ui-scale-option-{}", self.percent())
    }

    fn apply(self, root: &mut FrameRoot, window: &mut Window) {
        apply_app_settings_ui_scale(root, self, window);
    }
}

#[derive(Clone, Copy)]
struct AppSettingsAppearanceSelectProps<'a, T> {
    selected: T,
    popover_state: PopoverState,
    scroll_handle: &'a ScrollHandle,
    focuses: AppSettingsSelectFocuses<'a>,
    palette: &'static theme::ThemePalette,
}

#[expect(
    clippy::too_many_lines,
    reason = "The shared appearance select keeps its trigger, animated popover, keyboard navigation, and options together."
)]
fn app_settings_appearance_select<T: AppSettingsAppearanceValue>(
    props: AppSettingsAppearanceSelectProps<'_, T>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let AppSettingsAppearanceSelectProps {
        selected,
        popover_state,
        scroll_handle,
        focuses,
        palette,
    } = props;
    let options = T::options();
    let selected_index = options
        .iter()
        .position(|option| *option == selected)
        .unwrap_or_default();
    let option_focuses = focuses.options.to_vec();
    let key_option_focuses = option_focuses.clone();
    let key_scroll_handle = scroll_handle.clone();
    let trigger = frame_select_trigger_with_focus(
        T::TRIGGER_ID,
        T::LABEL,
        selected.display(),
        true,
        popover_state.is_open(),
        focuses.trigger,
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, event: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if event.is_keyboard() {
            return;
        }
        root.toggle_app_settings_appearance_popover(T::POPOVER);
        cx.notify();
    }))
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
            let key = event.keystroke.key.as_str();
            match key {
                "down" | "up" | "home" | "end" => {
                    cx.stop_propagation();
                    root.open_app_settings_appearance_popover(T::POPOVER);
                    let target = frame_select_target_index(
                        key_option_focuses.len(),
                        Some(selected_index),
                        key,
                        |_| true,
                    )
                    .unwrap_or(selected_index);
                    focus_app_settings_select_option(
                        target,
                        &key_option_focuses,
                        &key_scroll_handle,
                        window,
                        cx,
                    );
                    cx.notify();
                }
                "enter" | "space" if root.appearance_popover_state(T::POPOVER).is_open() => {
                    cx.stop_propagation();
                    root.close_app_settings_appearance_popover(T::POPOVER);
                    cx.notify();
                }
                "enter" | "space" => {
                    cx.stop_propagation();
                    root.open_app_settings_appearance_popover(T::POPOVER);
                    let focus = key_option_focuses.get(selected_index).cloned();
                    key_scroll_handle.scroll_to_item(selected_index);
                    if let Some(focus) = focus {
                        cx.defer_in(window, move |_root, window, cx| {
                            focus.focus(window, cx);
                        });
                    }
                    cx.notify();
                }
                "escape" => {
                    cx.stop_propagation();
                    root.close_app_settings_appearance_popover(T::POPOVER);
                    cx.notify();
                }
                _ => {}
            }
        }),
    );

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label(T::LABEL, palette))
        .child(trigger);

    if popover_state.is_rendered() {
        let progress =
            app_settings_appearance_popover_progress::<T>(popover_state.is_open(), window, cx);
        let content_height = frame_select_content_height(options.len());
        let mut list = frame_select_options_list(T::LIST_ID, scroll_handle);

        for (index, option) in options.iter().copied().enumerate() {
            let focus = option_focuses.get(index);
            list = list.child(app_settings_appearance_option(
                option,
                index,
                selected,
                focus,
                &option_focuses,
                scroll_handle,
                focuses.trigger,
                palette,
                cx,
            ));
        }

        let mut menu = frame_select_popover(
            T::PANEL_ID,
            54.0 + subtitle_popover_slide_offset(progress),
            progress,
            list,
            palette,
        );
        if let (Some(first), Some(last)) = (option_focuses.first(), option_focuses.last()) {
            menu = apply_frame_select_popover_focus_trap(
                menu,
                Some(focuses.panel),
                Some(first),
                Some(last),
                cx,
            );
        }
        if content_height > FRAME_SELECT_MAX_HEIGHT {
            menu = menu.child(frame_vertical_scrollbar(
                T::SCROLLBAR_ID,
                scroll_handle.clone(),
                content_height,
                palette,
            ));
        }
        field = field.child(deferred(menu).with_priority(20));
    }

    field
}

#[expect(
    clippy::too_many_arguments,
    reason = "Each option needs its selection, focus, navigation, scrolling, and root action context."
)]
fn app_settings_appearance_option<T: AppSettingsAppearanceValue>(
    option: T,
    index: usize,
    selected: T,
    focus: Option<&FocusHandle>,
    option_focuses: &[FocusHandle],
    scroll_handle: &ScrollHandle,
    trigger_focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let option_id = option.option_id();
    let option_focuses_for_key = option_focuses.to_vec();
    let scroll_handle_for_key = scroll_handle.clone();
    let trigger_focus_for_click = trigger_focus.clone();
    let trigger_focus_for_key = trigger_focus.clone();
    let element = if let Some(focus) = focus {
        frame_select_option_with_focus(
            option_id,
            option.display(),
            option == selected,
            true,
            focus,
            palette,
        )
    } else {
        frame_select_option(
            option_id,
            option.display(),
            option == selected,
            true,
            palette,
        )
    };

    element
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if event.is_keyboard() {
                return;
            }
            option.apply(root, window);
            root.close_app_settings_appearance_popover(T::POPOVER);
            trigger_focus_for_click.focus(window, cx);
            cx.notify();
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                match key {
                    "enter" | "space" => {
                        cx.stop_propagation();
                        option.apply(root, window);
                        root.close_app_settings_appearance_popover(T::POPOVER);
                        let focus = trigger_focus_for_key.clone();
                        cx.defer_in(window, move |_root, window, cx| {
                            focus.focus(window, cx);
                        });
                        cx.notify();
                    }
                    "up" | "down" | "home" | "end" => {
                        cx.stop_propagation();
                        if let Some(target) = frame_select_target_index(
                            option_focuses_for_key.len(),
                            Some(index),
                            key,
                            |_| true,
                        ) {
                            focus_app_settings_select_option(
                                target,
                                &option_focuses_for_key,
                                &scroll_handle_for_key,
                                window,
                                cx,
                            );
                        }
                    }
                    "escape" => {
                        cx.stop_propagation();
                        root.close_app_settings_appearance_popover(T::POPOVER);
                        trigger_focus_for_key.focus(window, cx);
                        cx.notify();
                    }
                    _ => {}
                }
            }),
        )
}

fn app_settings_appearance_popover_progress<T: AppSettingsAppearanceValue>(
    is_open: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> f32 {
    let transition = window
        .use_keyed_transition(
            T::MOTION_ID,
            cx,
            INTERACTION_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_in_out);
    set_motion_target(&transition, motion_target(is_open), cx);
    let progress = *transition.evaluate(window, cx);
    if !is_open && motion_is_hidden(progress) {
        cx.defer_in(window, move |root, _window, cx| {
            if root.finish_app_settings_appearance_popover_close(T::POPOVER) {
                cx.notify();
            }
        });
    }
    progress
}

fn focus_app_settings_select_option(
    index: usize,
    option_focuses: &[FocusHandle],
    scroll_handle: &ScrollHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) {
    scroll_handle.scroll_to_item(index);
    if let Some(focus) = option_focuses.get(index) {
        focus.focus(window, cx);
    }
}

fn apply_app_settings_ui_scale(root: &mut FrameRoot, scale: ScalePreset, window: &mut Window) {
    if root.set_ui_scale(scale) {
        window.set_rem_size(px(crate::appearance::BASE_REM_PX * scale.factor()));
    }
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
    update_install_ready: bool,
    focuses: AppSettingsUpdateFocuses<'_>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let busy = update_status.is_busy();
    let mut section = settings_section("Updates", palette)
        .child(frame_checkbox_row_with_focus(
            "app-settings-auto-update-check",
            "Check automatically",
            "Frame checks for signed releases in the background.",
            auto_update_check,
            false,
            focuses.auto_update,
            palette,
            cx,
            |root, _event, _window, cx| {
                if root.toggle_auto_update_check(cx) {
                    cx.notify();
                }
            },
        ))
        .child(update_check_now_button(
            busy,
            focuses.check_now,
            palette,
            window,
            cx,
        ))
        .child(update_status_label(
            update_status,
            update_install_ready,
            palette,
        ));

    if let UpdateStatus::Downloading {
        progress_percent,
        received_bytes,
        total_bytes,
        ..
    } = update_status
    {
        section = section.child(update_progress_bar(*progress_percent, palette));
        section = section.child(update_download_detail(
            *received_bytes,
            *total_bytes,
            *progress_percent,
            palette,
        ));
    }

    if let Some(row) = update_action_row(
        update_status,
        update_install_ready,
        focuses,
        palette,
        window,
        cx,
    ) {
        section = section.child(row);
    }

    section
}

fn update_status_label(
    status: &UpdateStatus,
    update_install_ready: bool,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let tone = match status {
        UpdateStatus::Error(_) => palette.danger,
        UpdateStatus::Disabled(_) => palette.warning,
        _ => palette.text_muted,
    };
    let text = update_status_text(status, update_install_ready);

    div()
        .id("app-settings-update-status")
        .role(gpui::Role::Status)
        .aria_label(text.clone())
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .text_color(color(tone))
        .child(theme::ui_text_owned(text))
}

fn update_status_text(status: &UpdateStatus, update_install_ready: bool) -> String {
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
        UpdateStatus::ReadyToInstall(_) if !update_install_ready => {
            UPDATE_INSTALL_WAIT_MESSAGE.to_string()
        }
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
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let lines = normalized_release_note_lines(notes);
    let content_height = update_release_notes_content_height(&lines);
    let mut content = div()
        .id("update-dialog-release-notes-content")
        .min_h(theme::ui_rem(UPDATE_RELEASE_NOTES_MIN_HEIGHT))
        .max_h(theme::ui_rem(UPDATE_RELEASE_NOTES_MAX_HEIGHT))
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .p_3()
        .pr_5();

    for line in lines {
        content = content.child(update_release_note_line(&line, palette));
    }

    div()
        .id("update-dialog-release-notes")
        .relative()
        .min_h(theme::ui_rem(UPDATE_RELEASE_NOTES_MIN_HEIGHT))
        .max_h(theme::ui_rem(UPDATE_RELEASE_NOTES_MAX_HEIGHT))
        .overflow_hidden()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(color(palette.fill_subtle))
        .child(content)
        .child(frame_vertical_scrollbar(
            "update-dialog-release-notes-scrollbar",
            scroll_handle.clone(),
            content_height,
            palette,
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

fn update_release_note_line(line: &str, palette: &'static theme::ThemePalette) -> gpui::Div {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return div().h(theme::ui_rem(8.0));
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
                palette.text_primary,
                theme::TEXT_WEIGHT_MEDIUM,
            )
        } else if let Some(bullet) = bullet {
            (
                format!("• {bullet}"),
                8.0,
                palette.text_muted,
                theme::TEXT_WEIGHT_REGULAR,
            )
        } else {
            (
                trimmed.to_string(),
                0.0,
                palette.text_muted,
                theme::TEXT_WEIGHT_REGULAR,
            )
        };
    let (text, highlights) = parse_update_release_note_emphasis(&text, palette);

    let mut line = div()
        .pl(theme::ui_rem(left_padding))
        .pb(theme::ui_rem(4.0))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .line_height(theme::ui_rem(16.0))
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
    palette: &'static theme::ThemePalette,
) -> (String, Vec<(std::ops::Range<usize>, HighlightStyle)>) {
    let mut text = String::with_capacity(input.len());
    let mut highlights = Vec::new();
    let mut rest = input;
    let highlight_style = HighlightStyle {
        color: Some(color(palette.text_primary).into()),
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

fn update_progress_bar(
    progress_percent: Option<u8>,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
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
        .h(theme::ui_rem(6.0))
        .w_full()
        .overflow_hidden()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(color(palette.fill_subtle))
        .child(
            div()
                .h_full()
                .w(relative(fraction.clamp(0.0, 1.0)))
                .rounded(theme::ui_rem(theme::RADIUS_SM))
                .bg(color(palette.accent)),
        )
}

fn update_download_detail(
    received_bytes: u64,
    total_bytes: Option<u64>,
    progress_percent: Option<u8>,
    palette: &'static theme::ThemePalette,
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
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .text_color(color(palette.text_muted))
        .font_features(assets::frame_tabular_number_font_features())
        .child(detail)
}

fn update_action_row(
    status: &UpdateStatus,
    update_install_ready: bool,
    focuses: AppSettingsUpdateFocuses<'_>,
    palette: &'static theme::ThemePalette,
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
                        palette,
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
                        palette,
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
                    update_install_ready,
                    focuses.install,
                    palette,
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
        UpdateStatus::UpToDate
        | UpdateStatus::Disabled(_)
        | UpdateStatus::Error(_)
        | UpdateStatus::Idle
        | UpdateStatus::Checking
        | UpdateStatus::Downloading { .. }
        | UpdateStatus::Installing => None,
    }
}

fn update_check_now_button(
    busy: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
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
        palette,
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
    pub(super) install_ready: bool,
    pub(super) release_notes_scroll_handle: &'a ScrollHandle,
    pub(super) panel_focus: &'a FocusHandle,
    pub(super) close_focus: &'a FocusHandle,
    pub(super) palette: &'static theme::ThemePalette,
}

pub(super) fn update_dialog(
    is_open: bool,
    view: UpdateDialogView<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let palette = view.palette;
    let transition = window
        .use_keyed_transition(
            "update-dialog-motion",
            cx,
            SURFACE_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_in_out);
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
        .bg(color(palette.canvas.with_alpha(0.64 * progress)))
        .backdrop_blur(theme::ui_rem(4.0 * progress).to_pixels(window.rem_size()))
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
    let palette = view.palette;
    let mut panel = div()
        .id("update-dialog-panel")
        .role(gpui::Role::AlertDialog)
        .aria_label(update_dialog_title(view.status))
        .track_focus(view.panel_focus)
        .tab_stop(false)
        .mt(theme::ui_rem(panel_offset))
        .w_full()
        .max_w(theme::ui_rem(640.0))
        .max_h(relative(0.86))
        .overflow_hidden()
        .rounded(theme::ui_rem(theme::RADIUS_LG))
        .bg(color(palette.surface))
        .shadow(card_surface_shadows(palette))
        .occlude()
        .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
        }))
        .child(update_dialog_header(
            view.status,
            view.close_focus,
            palette,
            window,
            cx,
        ))
        .child(update_dialog_body(
            view.status,
            view.info,
            view.install_ready,
            view.release_notes_scroll_handle,
            palette,
        ))
        .child(update_dialog_footer(
            view.status,
            view.install_ready,
            palette,
            window,
            cx,
        ));

    if matches!(view.status, UpdateStatus::Downloading { .. }) {
        panel = panel.child(update_dialog_download_state(view.status, palette));
    }

    panel
}

fn update_dialog_header(
    status: &UpdateStatus,
    close_focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut title_stack = div().flex().flex_col().gap_1();
    if let Some(kicker) = update_dialog_kicker(status) {
        title_stack = title_stack.child(
            div()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .font_weight(theme::TEXT_WEIGHT_MEDIUM)
                .text_color(color(palette.text_muted))
                .child(theme::ui_text(kicker)),
        );
    }
    title_stack = title_stack.child(
        div()
            .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
            .font_weight(theme::TEXT_WEIGHT_MEDIUM)
            .text_color(color(palette.text_primary))
            .child(theme::ui_text_owned(update_dialog_title(status))),
    );

    div()
        .relative()
        .h(theme::ui_rem(PANEL_HEADER_HEIGHT))
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
                !status.is_busy(),
                close_focus,
                palette,
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
        .child(panel_bottom_separator(palette))
}

fn update_dialog_body(
    status: &UpdateStatus,
    info: Option<&UpdateInfo>,
    install_ready: bool,
    release_notes_scroll_handle: &ScrollHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let notes = update_release_notes_text(info);
    let mut body = div().flex().flex_col().gap_3().p_4();

    if let Some(summary) = update_dialog_summary(status, notes.is_some(), install_ready) {
        body = body.child(
            div()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .line_height(theme::ui_rem(16.0))
                .text_color(color(palette.text_muted))
                .child(theme::ui_text_owned(summary)),
        );
    }

    if let Some(notes) = notes.as_deref() {
        body = body.child(update_release_notes_block(
            notes,
            release_notes_scroll_handle,
            palette,
        ));
    }

    if let UpdateStatus::Error(error) = status {
        body = body.child(
            div()
                .id("update-dialog-error-alert")
                .role(gpui::Role::Alert)
                .aria_label(error.clone())
                .rounded(theme::ui_rem(theme::RADIUS_SM))
                .bg(color(palette.danger.with_alpha(0.08)))
                .p_3()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .line_height(theme::ui_rem(16.0))
                .text_color(color(palette.danger))
                .child(error.clone()),
        );
    }

    body
}

fn update_dialog_download_state(
    status: &UpdateStatus,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
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
        .child(update_progress_bar(*progress_percent, palette))
        .child(update_download_detail(
            *received_bytes,
            *total_bytes,
            *progress_percent,
            palette,
        ))
}

fn update_dialog_footer(
    status: &UpdateStatus,
    install_ready: bool,
    palette: &'static theme::ThemePalette,
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
                palette,
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
        .child(update_dialog_primary_action(
            status,
            install_ready,
            palette,
            window,
            cx,
        ))
}

fn update_dialog_primary_action(
    status: &UpdateStatus,
    install_ready: bool,
    palette: &'static theme::ThemePalette,
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
            palette,
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
            install_ready,
            palette,
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
            palette,
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
            palette,
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
            palette,
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

fn update_dialog_summary(
    status: &UpdateStatus,
    has_notes: bool,
    install_ready: bool,
) -> Option<String> {
    match status {
        UpdateStatus::Available(_) if has_notes => None,
        UpdateStatus::Available(_) => Some(
            "A signed update is available, but this release did not include notes.".to_string()
        ),
        UpdateStatus::Downloading { .. } => Some(
            "Keep Frame open while the update package is downloaded and verified.".to_string()
        ),
        UpdateStatus::ReadyToInstall(_) if !install_ready => {
            Some(UPDATE_INSTALL_WAIT_MESSAGE.to_string())
        }
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
    palette: &'static theme::ThemePalette,
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
        palette,
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
            app_settings_apply_button(can_apply, palette, window, cx).on_click(cx.listener(
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
    enabled: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Ghost, false, enabled, palette);
    let animated = animated_button_colors(id, colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;

    let button = div()
        .id(id)
        .group(id)
        .w(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
        .h(theme::ui_rem(SETTINGS_CONTROL_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .bg(background)
        .text_color(foreground)
        .when(enabled, |this| this.hover(gpui::Styled::cursor_pointer))
        .when(enabled, |this| {
            this.active(move |style| style.bg(color(colors.active_background)))
        })
        .child(icon_svg(
            assets::ICON_CLOSE,
            FILE_LIST_ACTION_ICON_SIZE,
            foreground,
        ));
    let button = apply_button_motion(button, motion, enabled);

    apply_accessible_button_with_focus(button, label, enabled, focus, palette)
}

pub(super) fn app_settings_apply_button(
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        "app-settings-max-concurrency-apply",
        "Apply",
        ButtonVariant::Secondary,
        false,
        enabled,
        palette,
        window,
        cx,
    )
}

pub(super) fn drag_drop_overlay(
    is_open: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let transition = window
        .use_keyed_transition(
            "drag-drop-overlay-motion",
            cx,
            SURFACE_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_in_out);
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
        .bg(color(palette.canvas.with_alpha(0.60 * progress)))
        .backdrop_blur(theme::ui_rem(4.0 * progress).to_pixels(window.rem_size()))
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
                .rounded(theme::ui_rem(theme::RADIUS_LG))
                .border_1()
                .border_dashed()
                .border_color(color(palette.fill_subtle))
                .bg(color(palette.fill_subtle))
                .shadow(card_surface_shadows(palette))
                .child(
                    div()
                        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                        .text_color(color(palette.text_primary))
                        .child(theme::ui_text("Import source files")),
                ),
        )
}

pub(super) fn macos_native_window_controls_placeholder() -> gpui::Div {
    div()
        .w(theme::ui_rem(
            TITLEBAR_MACOS_NATIVE_TRAFFIC_LIGHT_PLACEHOLDER_WIDTH,
        ))
        .h(theme::ui_rem(TITLEBAR_TRAFFIC_LIGHT_SIZE))
        .mr_2()
}

pub(super) fn windows_window_controls(
    palette: &'static theme::ThemePalette,
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
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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
                palette,
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
                palette,
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
                palette,
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

pub(super) fn linux_window_controls(
    palette: &'static theme::ThemePalette,
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
        .gap(theme::ui_rem(TITLEBAR_LINUX_WINDOW_CONTROLS_GAP))
        .px(theme::ui_rem(TITLEBAR_LINUX_WINDOW_CONTROLS_PADDING_X))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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
                palette,
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
                palette,
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
                palette,
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

#[expect(
    clippy::too_many_arguments,
    reason = "Window chrome keeps platform metrics, semantics, palette, and render context explicit."
)]
pub(super) fn titlebar_window_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    metrics: TitlebarWindowButtonMetrics,
    destructive: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let hover_background = if destructive {
        palette.danger
    } else {
        palette.fill_subtle
    };
    let active_background = if destructive {
        palette.danger
    } else {
        palette.fill_selected
    };
    let hover_foreground = palette.text_primary;
    let foreground = palette.text_muted;
    let colors = ButtonColors {
        background: palette.transparent,
        hover_background,
        active_background,
        foreground,
        hover_foreground,
        opacity: 1.0,
    };
    let animated = animated_button_colors(id, colors, window, cx);
    let background = animated.background;
    let icon_color = animated.foreground;
    let motion = animated.motion;

    let button = div()
        .id(id)
        .w(theme::ui_rem(metrics.width))
        .h(theme::ui_rem(metrics.height))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::ui_rem(metrics.radius))
        .bg(background)
        .text_color(icon_color)
        .hover(gpui::Styled::cursor_pointer)
        .active(move |style| style.bg(color(active_background)))
        .on_mouse_move(|_, _, cx| cx.stop_propagation())
        .child(icon_svg(icon, metrics.icon_size, icon_color));
    let button = apply_button_motion(button, motion, true);

    apply_accessible_button(button, label, true, palette).tab_stop(false)
}

pub(super) fn frame_logo(palette: &'static theme::ThemePalette) -> gpui::Div {
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

pub(super) fn platform_frame_logo(palette: &'static theme::ThemePalette) -> gpui::Div {
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

pub(super) fn titlebar_divider(palette: &'static theme::ThemePalette) -> gpui::Div {
    vertical_separator(TITLEBAR_DIVIDER_HEIGHT, palette)
}

pub(super) fn platform_titlebar_divider(palette: &'static theme::ThemePalette) -> gpui::Div {
    vertical_separator(TITLEBAR_PLATFORM_DIVIDER_HEIGHT, palette)
}

pub(super) fn titlebar_navigation(
    active_view: ActiveView,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id("titlebar-main-view-tabs")
        .role(gpui::Role::TabList)
        .aria_label("Main view")
        .h(theme::ui_rem(TITLEBAR_SEGMENT_HEIGHT))
        .flex()
        .items_center()
        .gap_1()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.surface))
        .px(theme::ui_rem(3.0))
        .py(theme::ui_rem(2.0))
        .shadow(input_highlight_shadows(palette))
        .child(titlebar_segment(
            assets::ICON_LAYOUT_LIST,
            "Workspace",
            ActiveView::Workspace,
            active_view == ActiveView::Workspace,
            palette,
            window,
            cx,
        ))
        .child(titlebar_segment(
            assets::ICON_TERMINAL,
            "Logs",
            ActiveView::Logs,
            active_view == ActiveView::Logs,
            palette,
            window,
            cx,
        ))
}

pub(super) fn titlebar_stats(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_4()
        .text_color(color(palette.text_muted))
        .child(titlebar_stat(
            assets::ICON_HARD_DRIVE,
            format!("Storage {}", format_total_size(state.total_size_bytes)),
            palette,
        ))
        .child(titlebar_stat(
            assets::ICON_FILE_VIDEO,
            format!("Items {}", state.file_count),
            palette,
        ))
}

pub(super) fn titlebar_stat(
    icon: &'static str,
    label: String,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(icon_svg(
            icon,
            TITLEBAR_ICON_SIZE,
            color(palette.text_muted),
        ))
        .child(theme::ui_text_owned(label))
}

pub(super) fn titlebar_segment(
    icon: &'static str,
    label: &'static str,
    view: ActiveView,
    selected: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let colors = button_colors(ButtonVariant::Secondary, selected, true, palette);
    let segment_id = match view {
        ActiveView::Workspace => "titlebar-workspace",
        ActiveView::Logs => "titlebar-logs",
    };
    let motion = button_motion(format!("{segment_id}-hover"), window, cx);
    let hover_progress = *motion.hover_transition.evaluate(window, cx);
    let background = if selected {
        mix_color(colors.background, colors.hover_background, hover_progress)
    } else {
        mix_color(palette.transparent, palette.fill_subtle, hover_progress)
    };
    let foreground = mix_color(
        if selected {
            palette.text_primary
        } else {
            palette.text_muted
        },
        palette.text_primary,
        hover_progress,
    );

    let button = div()
        .id(segment_id)
        .h(theme::ui_rem(TITLEBAR_NAV_BUTTON_HEIGHT))
        .role(gpui::Role::Tab)
        .aria_label(label)
        .aria_selected(selected)
        .focusable()
        .tab_stop(true)
        .focus_visible(move |style| focus_visible_ring(style, palette))
        .flex()
        .items_center()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .group(segment_id)
        .px_2()
        .bg(background)
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .when(selected, |this| {
            this.shadow(button_highlight_shadows(palette))
        })
        .hover(gpui::Styled::cursor_pointer)
        .active(move |style| style.bg(color(colors.active_background)))
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
        .child(theme::ui_text(label));

    apply_button_motion(button, motion, true)
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
    use frame_updater::{PlatformAssetKey, UpdateAssetKind, UpdateChannel, UpdatePackage};
    use semver::Version;
    use std::path::PathBuf;

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
        let palette = theme::palette(crate::appearance::ColorTheme::Dark);
        let (text, highlights) = parse_update_release_note_emphasis(
            "• **Native GPUI Application:** Rebuilt Frame",
            palette,
        );

        assert_eq!(text, "• Native GPUI Application: Rebuilt Frame");
        assert_eq!(highlights.len(), 1);
        assert_eq!(&text[highlights[0].0.clone()], "Native GPUI Application:");
        assert_eq!(
            highlights[0].1.color,
            Some(color(palette.text_primary).into())
        );
        assert_eq!(highlights[0].1.font_weight, Some(theme::TEXT_WEIGHT_MEDIUM));
    }

    #[test]
    fn release_note_emphasis_keeps_unclosed_markers_literal() {
        let (text, highlights) = parse_update_release_note_emphasis(
            "• **Native GPUI Application: Rebuilt Frame",
            theme::palette(crate::appearance::ColorTheme::Dark),
        );

        assert_eq!(text, "• **Native GPUI Application: Rebuilt Frame");
        assert!(highlights.is_empty());
    }

    #[test]
    fn ready_update_explains_why_install_is_blocked() {
        let status = UpdateStatus::ReadyToInstall(Box::new(test_update_package()));

        assert_eq!(
            update_status_text(&status, false),
            UPDATE_INSTALL_WAIT_MESSAGE
        );
        assert_eq!(
            update_dialog_summary(&status, false, false).as_deref(),
            Some(UPDATE_INSTALL_WAIT_MESSAGE)
        );
    }

    fn test_update_package() -> UpdatePackage {
        UpdatePackage {
            version: Version::new(0, 32, 0),
            channel: UpdateChannel::Stable,
            asset_key: PlatformAssetKey::MacosAarch64,
            kind: UpdateAssetKind::MacosAppZip,
            file_name: "Frame.zip".to_string(),
            path: PathBuf::from("/tmp/Frame.zip"),
            size_bytes: 1,
            sha256: "00".repeat(32),
            installer_args: Vec::new(),
        }
    }
}
