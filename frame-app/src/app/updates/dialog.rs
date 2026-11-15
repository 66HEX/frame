use crate::app::components::{
    frame_dialog_close_button, frame_dialog_footer, frame_dialog_header, frame_dialog_overlay,
    frame_dialog_panel, frame_text_button,
};
use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::primitives::{ButtonVariant, action_button, color};
use crate::app::root::FrameRoot;
use crate::app::updates::preferences::{update_download_detail, update_progress_bar};
use crate::app::updates::release_notes::{update_release_notes_block, update_release_notes_text};
use crate::app::updates::state::{UPDATE_INSTALL_WAIT_MESSAGE, UpdateStatus};
use crate::{assets, theme};
use frame_updater::UpdateInfo;
use gpui::prelude::{ParentElement, Styled};
use gpui::{
    ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement, ScrollHandle,
    StatefulInteractiveElement, Window, div, relative,
};

#[derive(Clone, Copy)]
pub(in crate::app) struct UpdateDialogView<'a> {
    pub(in crate::app) status: &'a UpdateStatus,
    pub(in crate::app) info: Option<&'a UpdateInfo>,
    pub(in crate::app) install_ready: bool,
    pub(in crate::app) release_notes_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) panel_focus: &'a FocusHandle,
    pub(in crate::app) close_focus: &'a FocusHandle,
    pub(in crate::app) palette: &'static theme::ThemePalette,
}

pub(in crate::app) fn update_dialog(
    is_open: bool,
    view: UpdateDialogView<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let palette = view.palette;
    let scope = view.panel_focus.clone();
    let progress = surface_motion("update-dialog-motion", is_open, window, cx);

    if !is_open && motion_is_hidden(progress) {
        cx.defer_in(window, |root, _window, cx| {
            if root.finish_update_dialog_close() {
                cx.notify();
            }
        });
    }

    frame_dialog_overlay(
        "update-dialog",
        update_dialog_panel(0.0, view, window, cx),
        progress,
        is_open,
        palette,
        window,
    )
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
            match event.keystroke.key.as_str() {
                "escape" if !root.update_ui.status.is_busy() => {
                    root.close_update_dialog();
                    root.restore_focus_after_update_dialog_close(window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
                "tab" => {
                    crate::app::accessibility::handle_focus_scope_tab_navigation(
                        event, &scope, window, cx,
                    );
                }
                _ => {}
            }
        }),
    )
    .on_click(cx.listener(|root, _: &ClickEvent, window, cx| {
        cx.stop_propagation();
        if !root.update_ui.status.is_busy() {
            root.close_update_dialog();
            root.restore_focus_after_update_dialog_close(window, cx);
            cx.notify();
        }
    }))
}

pub(in crate::app) fn update_dialog_panel(
    panel_offset: f32,
    view: UpdateDialogView<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let palette = view.palette;
    let mut panel = frame_dialog_panel(
        "update-dialog-panel",
        &update_dialog_title(view.status),
        None,
        view.panel_focus,
        palette,
        window,
    )
    .role(gpui::Role::AlertDialog)
    .mt(theme::ui_rem(panel_offset))
    .max_h(relative(0.86))
    .overflow_y_scroll()
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
    ));

    if matches!(view.status, UpdateStatus::Downloading { .. }) {
        panel = panel.child(update_dialog_download_state(view.status, palette));
    }

    panel.child(update_dialog_footer(
        view.status,
        view.install_ready,
        palette,
        window,
        cx,
    ))
}

pub(in crate::app) fn update_dialog_header(
    status: &UpdateStatus,
    close_focus: &FocusHandle,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    frame_dialog_header(
        &update_dialog_title(status),
        update_dialog_kicker(status),
        palette,
    )
    .child(
        frame_dialog_close_button(
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
}

pub(in crate::app) fn update_dialog_body(
    status: &UpdateStatus,
    info: Option<&UpdateInfo>,
    install_ready: bool,
    release_notes_scroll_handle: &ScrollHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let notes = update_release_notes_text(info);
    let mut body = div().flex().flex_col().gap_3();

    if let Some(summary) = update_dialog_summary(status, notes.is_some(), install_ready) {
        body = body.child(
            div()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .line_height(relative(1.625))
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
                .rounded(theme::ui_rem(theme::RADIUS_MD))
                .bg(color(palette.danger_surface))
                .p_3()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .line_height(relative(1.625))
                .text_color(color(palette.danger))
                .child(error.clone()),
        );
    }

    body
}

pub(in crate::app) fn update_dialog_download_state(
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

pub(in crate::app) fn update_dialog_footer(
    status: &UpdateStatus,
    install_ready: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    frame_dialog_footer(palette, window)
        .child(
            frame_text_button(
                "update-dialog-later",
                "Later",
                ButtonVariant::Outline,
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

pub(in crate::app) fn update_dialog_primary_action(
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
            ButtonVariant::Primary,
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
            ButtonVariant::Primary,
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

pub(in crate::app) const fn update_dialog_kicker(status: &UpdateStatus) -> Option<&'static str> {
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

pub(in crate::app) fn update_dialog_title(status: &UpdateStatus) -> String {
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

pub(in crate::app) fn update_dialog_summary(
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
