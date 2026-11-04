use crate::app::accessibility::FrameFocusKey;
use crate::app::input::FrameTextInputKind;
use crate::app::preferences::view::{
    AppSettingsSelectFocuses, AppSettingsSheetProps, app_settings_sheet,
};
use crate::app::queue::drop_overlay::drag_drop_overlay;
use crate::app::root::FrameRoot;
use crate::app::updates::dialog::{UpdateDialogView, update_dialog};
use crate::app::updates::state::UpdateStatus;
use crate::appearance::{ColorTheme, ScalePreset};
use crate::theme;
use gpui::prelude::*;
use gpui::{Context, Window};

impl FrameRoot {
    #[expect(
        clippy::too_many_lines,
        reason = "Keep the existing focus registration and element append order explicit during structural extraction."
    )]
    pub(super) fn render_overlays(
        &mut self,
        mut root: gpui::Stateful<gpui::Div>,
        palette: &'static theme::ThemePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        if self.settings_ui.is_present {
            let update_install_ready = self.can_install_downloaded_update();
            let theme_popover_active = self.settings_ui.theme_popover.is_open();
            let ui_scale_popover_active = self.settings_ui.ui_scale_popover.is_open();
            let theme_trigger_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-theme".to_string()),
                true,
                cx,
            );
            let theme_panel_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-theme-options".to_string()),
                false,
                cx,
            );
            let theme_option_focuses = ColorTheme::ALL
                .into_iter()
                .map(|color_theme| {
                    self.ensure_focus(
                        FrameFocusKey::Control(format!(
                            "app-settings-theme-option-{}",
                            color_theme.persisted()
                        )),
                        theme_popover_active,
                        cx,
                    )
                })
                .collect::<Vec<_>>();
            let ui_scale_trigger_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-ui-scale".to_string()),
                true,
                cx,
            );
            let ui_scale_panel_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-ui-scale-options".to_string()),
                false,
                cx,
            );
            let ui_scale_option_focuses = ScalePreset::ALL
                .into_iter()
                .map(|scale| {
                    self.ensure_focus(
                        FrameFocusKey::Control(format!(
                            "app-settings-ui-scale-option-{}",
                            scale.percent()
                        )),
                        ui_scale_popover_active,
                        cx,
                    )
                })
                .collect::<Vec<_>>();
            let value_focus = self.ensure_text_input_focus(FrameTextInputKind::MaxConcurrency, cx);
            let output_directory_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-output-directory".to_string()),
                true,
                cx,
            );
            let auto_update_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-auto-update-check".to_string()),
                true,
                cx,
            );
            let check_now_enabled = !self.update_ui.status.is_busy();
            let check_now_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-check-now".to_string()),
                check_now_enabled,
                cx,
            );
            let download_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-download".to_string()),
                matches!(&self.update_ui.status, UpdateStatus::Available(_)),
                cx,
            );
            let skip_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-skip".to_string()),
                matches!(&self.update_ui.status, UpdateStatus::Available(_)),
                cx,
            );
            let install_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-update-install".to_string()),
                update_install_ready,
                cx,
            );
            let last_focus = match &self.update_ui.status {
                UpdateStatus::Available(_) => &skip_focus,
                UpdateStatus::ReadyToInstall(_) if update_install_ready => &install_focus,
                UpdateStatus::ReadyToInstall(_)
                | UpdateStatus::UpToDate
                | UpdateStatus::Disabled(_)
                | UpdateStatus::Error(_)
                | UpdateStatus::Idle => &check_now_focus,
                UpdateStatus::Checking
                | UpdateStatus::Downloading { .. }
                | UpdateStatus::Installing => &auto_update_focus,
            };
            let panel_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-panel".to_string()),
                false,
                cx,
            );
            let close_focus = self.ensure_focus(
                FrameFocusKey::Control("app-settings-close".to_string()),
                true,
                cx,
            );
            if self.settings_ui.is_open
                && !self.settings_ui.theme_popover.is_open()
                && !self.settings_ui.ui_scale_popover.is_open()
                && !panel_focus.contains_focused(window, cx)
            {
                close_focus.focus(window, cx);
            }
            root = root.child(app_settings_sheet(
                AppSettingsSheetProps {
                    is_open: self.settings_ui.is_open,
                    current_max_concurrency: self.max_concurrency,
                    draft_max_concurrency: &self.settings_ui.max_concurrency_draft,
                    error: self.settings_ui.max_concurrency_error.as_deref(),
                    default_output_directory: self
                        .default_output_directory
                        .as_deref()
                        .and_then(std::path::Path::to_str),
                    output_directory_error: self.settings_ui.output_directory_error.as_deref(),
                    appearance: self.appearance,
                    palette,
                    appearance_error: self.settings_ui.appearance_error.as_deref(),
                    theme_popover: self.settings_ui.theme_popover,
                    ui_scale_popover: self.settings_ui.ui_scale_popover,
                    scroll_handle: &self.settings_ui.app_settings_scroll_handle,
                    theme_scroll_handle: &self.settings_ui.theme_scroll_handle,
                    ui_scale_scroll_handle: &self.settings_ui.ui_scale_scroll_handle,
                    theme_focuses: AppSettingsSelectFocuses {
                        trigger: &theme_trigger_focus,
                        panel: &theme_panel_focus,
                        options: &theme_option_focuses,
                    },
                    ui_scale_focuses: AppSettingsSelectFocuses {
                        trigger: &ui_scale_trigger_focus,
                        panel: &ui_scale_panel_focus,
                        options: &ui_scale_option_focuses,
                    },
                    auto_update_check: self.auto_update_check,
                    update_status: &self.update_ui.status,
                    update_install_ready,
                    value_focus: &value_focus,
                    output_directory_focus: &output_directory_focus,
                    auto_update_focus: &auto_update_focus,
                    check_now_focus: &check_now_focus,
                    download_focus: &download_focus,
                    skip_focus: &skip_focus,
                    install_focus: &install_focus,
                    panel_focus: &panel_focus,
                    close_focus: &close_focus,
                    last_focus,
                },
                window,
                cx,
            ));
        }

        if self.drag_drop_ui.is_present {
            root = root.child(drag_drop_overlay(
                self.drag_drop_ui.is_open,
                palette,
                window,
                cx,
            ));
        }

        if self.update_ui.dialog_present {
            let update_install_ready = self.can_install_downloaded_update();
            let panel_focus = self.ensure_focus(
                FrameFocusKey::Control("update-dialog-panel".to_string()),
                false,
                cx,
            );
            let close_focus = self.ensure_focus(
                FrameFocusKey::Control("update-dialog-close".to_string()),
                !self.update_ui.status.is_busy(),
                cx,
            );
            if self.update_ui.dialog_open && !panel_focus.contains_focused(window, cx) {
                close_focus.focus(window, cx);
            }
            root = root.child(update_dialog(
                self.update_ui.dialog_open,
                UpdateDialogView {
                    status: &self.update_ui.status,
                    info: self.update_ui.dialog_info.as_deref(),
                    install_ready: update_install_ready,
                    release_notes_scroll_handle: &self.update_ui.release_notes_scroll_handle,
                    panel_focus: &panel_focus,
                    close_focus: &close_focus,
                    palette,
                },
                window,
                cx,
            ));
        }

        root
    }
}
