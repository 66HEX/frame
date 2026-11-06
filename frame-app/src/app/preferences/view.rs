use crate::app::accessibility::handle_modal_tab_navigation;
use crate::app::components::FrameStyle;
use crate::app::components::popover_state::PopoverState;
use crate::app::components::{frame_text_button, frame_text_button_with_focus};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::inspector::{settings_hint_text, settings_section};
use crate::app::layout::PANEL_HEADER_HEIGHT;
use crate::app::motion::{
    SETTINGS_DRAWER_EDGE_INSET, drawer_motion, motion_is_hidden, settings_drawer_right_inset,
    surface_motion,
};
use crate::app::preferences::appearance::app_settings_appearance_section;
use crate::app::preferences::state::AppearancePopover;
use crate::app::primitives::{ButtonVariant, color, icon_svg, panel_bottom_separator};
use crate::app::root::FrameRoot;
use crate::app::updates::preferences::{AppSettingsUpdateFocuses, app_settings_updates_section};
use crate::app::updates::state::UpdateStatus;
use crate::app_info::FRAME_APP_VERSION;
use crate::appearance::AppearanceSettings;
use crate::{assets, theme};
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement, ScrollHandle,
    StatefulInteractiveElement, Window, div, relative,
};

#[derive(Clone, Copy)]
pub(in crate::app) struct AppSettingsDrawerProps<'a> {
    pub(in crate::app) is_open: bool,
    pub(in crate::app) current_max_concurrency: usize,
    pub(in crate::app) draft_max_concurrency: &'a str,
    pub(in crate::app) error: Option<&'a str>,
    pub(in crate::app) default_output_directory: Option<&'a str>,
    pub(in crate::app) output_directory_error: Option<&'a str>,
    pub(in crate::app) appearance: AppearanceSettings,
    pub(in crate::app) palette: &'static theme::ThemePalette,
    pub(in crate::app) appearance_error: Option<&'a str>,
    pub(in crate::app) theme_popover: PopoverState,
    pub(in crate::app) ui_scale_popover: PopoverState,
    pub(in crate::app) scroll_handle: &'a ScrollHandle,
    pub(in crate::app) theme_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) ui_scale_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) theme_focuses: AppSettingsSelectFocuses<'a>,
    pub(in crate::app) ui_scale_focuses: AppSettingsSelectFocuses<'a>,
    pub(in crate::app) auto_update_check: bool,
    pub(in crate::app) update_status: &'a UpdateStatus,
    pub(in crate::app) update_install_ready: bool,
    pub(in crate::app) value_focus: &'a FocusHandle,
    pub(in crate::app) output_directory_focus: &'a FocusHandle,
    pub(in crate::app) auto_update_focus: &'a FocusHandle,
    pub(in crate::app) check_now_focus: &'a FocusHandle,
    pub(in crate::app) download_focus: &'a FocusHandle,
    pub(in crate::app) skip_focus: &'a FocusHandle,
    pub(in crate::app) install_focus: &'a FocusHandle,
    pub(in crate::app) panel_focus: &'a FocusHandle,
    pub(in crate::app) close_focus: &'a FocusHandle,
    pub(in crate::app) last_focus: &'a FocusHandle,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct AppSettingsSelectFocuses<'a> {
    pub(in crate::app) trigger: &'a FocusHandle,
    pub(in crate::app) panel: &'a FocusHandle,
    pub(in crate::app) options: &'a [FocusHandle],
}

#[expect(
    clippy::too_many_lines,
    clippy::large_types_passed_by_value,
    reason = "The declarative settings render consumes a copyable bundle so all render inputs remain explicit."
)]
pub(in crate::app) fn app_settings_drawer(
    props: AppSettingsDrawerProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let palette = props.palette;
    let draft_is_dirty =
        props.draft_max_concurrency.trim() != props.current_max_concurrency.to_string();
    let progress = drawer_motion(props.is_open, window, cx);
    let backdrop_progress =
        surface_motion("app-settings-backdrop-motion", props.is_open, window, cx);
    let right_inset = settings_drawer_right_inset(progress);
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
        .id("app-settings-drawer")
        .debug_selector(|| "app-settings-drawer".to_owned())
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
                .debug_selector(|| "app-settings-backdrop".to_owned())
                .absolute()
                .inset_0()
                .bg(color(palette.overlay_backdrop))
                .opacity(backdrop_progress)
                .backdrop_blur(theme::ui_rem(4.0).to_pixels(window.rem_size()))
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
                .debug_selector(|| "app-settings-panel".to_owned())
                .role(gpui::Role::Dialog)
                .aria_modal(true)
                .aria_label("Settings")
                .track_focus(props.panel_focus)
                .tab_stop(false)
                .absolute()
                .top(theme::ui_rem(SETTINGS_DRAWER_EDGE_INSET))
                .right(theme::ui_rem(right_inset))
                .bottom(theme::ui_rem(SETTINGS_DRAWER_EDGE_INSET))
                .w(theme::ui_rem(384.0))
                .max_w(relative(0.75))
                .flex()
                .flex_col()
                .rounded(theme::ui_rem(theme::RADIUS_XL))
                .bg(theme::shadows::surface_fill(palette.surface, palette))
                .text_color(color(palette.token("card-foreground")))
                .shadow(theme::shadows::ShadowLevel::Lg.shadows())
                .opacity(progress)
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
                        .debug_selector(|| "app-settings-scroll-body".to_owned())
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

pub(in crate::app) fn app_settings_version_label(
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .justify_end()
        .pt_4()
        .text_size(theme::ui_rem(11.0))
        .text_color(color(palette.text_muted))
        .child(theme::ui_text_owned(format!("Frame v{FRAME_APP_VERSION}")))
}

pub(in crate::app) fn app_settings_output_directory_section(
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

pub(in crate::app) fn app_settings_concurrency_control(
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
        this.aria_invalid(true)
            .aria_description(error)
            .invalid_surface(palette)
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

pub(in crate::app) fn app_settings_close_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let button = crate::app::components::frame_button_base(
        id,
        label,
        ButtonVariant::Ghost,
        false,
        enabled,
        theme::metrics::ControlSize::Sm,
        palette,
        window,
        cx,
    )
    .w(theme::ui_rem(24.0))
    .p_0()
    .child(icon_svg(
        assets::ICON_CLOSE,
        12.0,
        color(palette.text_primary),
    ));
    button.track_focus(focus)
}

pub(in crate::app) fn app_settings_apply_button(
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
