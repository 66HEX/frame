use crate::app::components::{
    frame_checkbox_row_with_focus, frame_progress_track, frame_text_button_with_focus,
};
use crate::app::inspector::settings_section;
use crate::app::model::format_total_size;
use crate::app::primitives::{ButtonVariant, color};
use crate::app::root::FrameRoot;
use crate::app::updates::state::{UPDATE_INSTALL_WAIT_MESSAGE, UpdateStatus};
use crate::{assets, theme};
use gpui::prelude::{ParentElement, Styled};
use gpui::{
    ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement, StatefulInteractiveElement,
    Window, div,
};

#[derive(Clone, Copy)]
pub(in crate::app) struct AppSettingsUpdateFocuses<'a> {
    pub(in crate::app) auto_update: &'a FocusHandle,
    pub(in crate::app) check_now: &'a FocusHandle,
    pub(in crate::app) download: &'a FocusHandle,
    pub(in crate::app) skip: &'a FocusHandle,
    pub(in crate::app) install: &'a FocusHandle,
}

pub(in crate::app) fn app_settings_updates_section(
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

pub(in crate::app) fn update_status_label(
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

pub(in crate::app) fn update_status_text(
    status: &UpdateStatus,
    update_install_ready: bool,
) -> String {
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

pub(in crate::app) fn update_progress_bar(
    progress_percent: Option<u8>,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let fraction = progress_percent.map_or(0.0, |percent| f32::from(percent) / 100.0);
    let numeric_percent = progress_percent.map_or(0.0, f64::from);
    let value_text = progress_percent.map_or_else(
        || "Download progress unknown".to_string(),
        |percent| format!("{percent}%"),
    );

    frame_progress_track(fraction, palette)
        .id("app-settings-update-progress")
        .role(gpui::Role::ProgressIndicator)
        .aria_label("Update download progress")
        .aria_numeric_value(numeric_percent)
        .aria_min_numeric_value(0.0)
        .aria_max_numeric_value(100.0)
        .aria_value(value_text)
}

pub(in crate::app) fn update_download_detail(
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

pub(in crate::app) fn update_action_row(
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
                        ButtonVariant::Primary,
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
                    ButtonVariant::Primary,
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

pub(in crate::app) fn update_check_now_button(
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
