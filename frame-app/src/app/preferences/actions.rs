use crate::app::components::popover_state::PopoverState;
use crate::app::input::FrameTextInputKind;
use crate::app::preferences::state::AppearancePopover;
use crate::app::root::FrameRoot;
use crate::app::{DecreaseUiScale, IncreaseUiScale, ResetUiScale};
use crate::appearance::{AppearanceSettings, ColorTheme, ScalePreset};
use crate::native_dialogs::{output_folder_dialog, pick_output_folder};
use gpui::{Context, Window};

impl FrameRoot {
    pub(in crate::app) fn open_app_settings(&mut self) {
        self.settings_ui.is_open = true;
        self.settings_ui.is_present = true;
        self.settings_ui.max_concurrency_draft = self.max_concurrency.to_string();
        self.settings_ui.max_concurrency_error = None;
        self.settings_ui.output_directory_error = None;
        self.settings_ui.appearance_error = None;
        self.settings_ui.theme_popover = PopoverState::Hidden;
        self.settings_ui.ui_scale_popover = PopoverState::Hidden;
    }

    pub(in crate::app) fn close_app_settings(&mut self) {
        self.settings_ui.is_open = false;
        self.settings_ui.max_concurrency_error = None;
        self.settings_ui.output_directory_error = None;
        self.settings_ui.appearance_error = None;
        self.close_app_settings_appearance_popover(AppearancePopover::Theme);
        self.close_app_settings_appearance_popover(AppearancePopover::UiScale);
        self.text_input_ui
            .focuses
            .clear(FrameTextInputKind::MaxConcurrency);
        if self.text_input_ui.active == Some(FrameTextInputKind::MaxConcurrency) {
            self.stop_text_input_cursor();
        }
    }

    pub(in crate::app) const fn appearance_popover_state(
        &self,
        popover: AppearancePopover,
    ) -> PopoverState {
        match popover {
            AppearancePopover::Theme => self.settings_ui.theme_popover,
            AppearancePopover::UiScale => self.settings_ui.ui_scale_popover,
        }
    }

    pub(in crate::app) const fn toggle_app_settings_appearance_popover(
        &mut self,
        popover: AppearancePopover,
    ) {
        let current = self.appearance_popover_state(popover);
        self.hide_other_appearance_popover(popover);
        *self.appearance_popover_state_mut(popover) = match current {
            PopoverState::Open => PopoverState::Closing,
            PopoverState::Hidden | PopoverState::Closing => PopoverState::Open,
        };
    }

    pub(in crate::app) const fn open_app_settings_appearance_popover(
        &mut self,
        popover: AppearancePopover,
    ) {
        self.hide_other_appearance_popover(popover);
        *self.appearance_popover_state_mut(popover) = PopoverState::Open;
    }

    pub(in crate::app) const fn close_app_settings_appearance_popover(
        &mut self,
        popover: AppearancePopover,
    ) {
        if matches!(self.appearance_popover_state(popover), PopoverState::Open) {
            *self.appearance_popover_state_mut(popover) = PopoverState::Closing;
        }
    }

    pub(in crate::app) const fn finish_app_settings_appearance_popover_close(
        &mut self,
        popover: AppearancePopover,
    ) -> bool {
        if !matches!(
            self.appearance_popover_state(popover),
            PopoverState::Closing
        ) {
            return false;
        }
        *self.appearance_popover_state_mut(popover) = PopoverState::Hidden;
        true
    }

    const fn appearance_popover_state_mut(
        &mut self,
        popover: AppearancePopover,
    ) -> &mut PopoverState {
        match popover {
            AppearancePopover::Theme => &mut self.settings_ui.theme_popover,
            AppearancePopover::UiScale => &mut self.settings_ui.ui_scale_popover,
        }
    }

    const fn hide_other_appearance_popover(&mut self, popover: AppearancePopover) {
        match popover {
            AppearancePopover::Theme => {
                self.settings_ui.ui_scale_popover = PopoverState::Hidden;
            }
            AppearancePopover::UiScale => {
                self.settings_ui.theme_popover = PopoverState::Hidden;
            }
        }
    }

    pub(in crate::app) const fn finish_app_settings_close(&mut self) -> bool {
        if self.settings_ui.is_open || !self.settings_ui.is_present {
            return false;
        }
        self.settings_ui.is_present = false;
        true
    }

    pub(in crate::app) fn apply_max_concurrency_draft(&mut self) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        let Some(value) = self.parsed_max_concurrency_draft() else {
            self.settings_ui.max_concurrency_error =
                Some("Enter a whole number greater than zero.".to_string());
            return false;
        };

        match self.conversion_processes.update_max_concurrency(value) {
            Ok(()) => {
                self.max_concurrency = value;
                self.settings_ui.max_concurrency_draft = value.to_string();
                self.settings_ui.max_concurrency_error = None;
                if let Err(error) = self.persist_app_settings() {
                    self.settings_ui.max_concurrency_error =
                        Some(format!("Failed to save settings: {error}"));
                }
                true
            }
            Err(error) => {
                self.settings_ui.max_concurrency_error = Some(error.to_string());
                false
            }
        }
    }
    pub(in crate::app) fn parsed_max_concurrency_draft(&self) -> Option<usize> {
        let trimmed = self.settings_ui.max_concurrency_draft.trim();
        let value = trimmed.parse::<usize>().ok()?;
        (value > 0).then_some(value)
    }

    pub(in crate::app) fn set_ui_scale(&mut self, scale: ScalePreset) -> bool {
        let mut appearance = self.appearance;
        appearance.ui_scale = scale;
        self.set_appearance(appearance)
    }

    pub(in crate::app) fn set_color_theme(&mut self, color_theme: ColorTheme) -> bool {
        let mut appearance = self.appearance;
        appearance.color_theme = color_theme;
        self.set_appearance(appearance)
    }

    pub(in crate::app) fn increase_ui_scale(
        &mut self,
        _: &IncreaseUiScale,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_ui_scale_shortcut(self.appearance.ui_scale.next(), window, cx);
    }

    pub(in crate::app) fn decrease_ui_scale(
        &mut self,
        _: &DecreaseUiScale,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_ui_scale_shortcut(self.appearance.ui_scale.previous(), window, cx);
    }

    pub(in crate::app) fn reset_ui_scale(
        &mut self,
        _: &ResetUiScale,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_ui_scale_shortcut(ScalePreset::Percent100, window, cx);
    }

    fn apply_ui_scale_shortcut(
        &mut self,
        scale: ScalePreset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.set_ui_scale(scale) {
            window.set_rem_size(gpui::px(crate::appearance::BASE_REM_PX * scale.factor()));
            cx.notify();
        }
    }

    fn set_appearance(&mut self, appearance: AppearanceSettings) -> bool {
        if self.update_installation_in_progress() || self.appearance == appearance {
            return false;
        }

        let previous = self.appearance;
        self.appearance = appearance;
        if let Err(error) = self.persist_app_settings() {
            self.appearance = previous;
            self.settings_ui.appearance_error = Some(format!("Failed to save settings: {error}"));
            return false;
        }

        self.settings_ui.appearance_error = None;
        true
    }

    pub(in crate::app) fn prompt_default_output_folder(window: &Window, cx: &Context<Self>) {
        let dialog = output_folder_dialog(window);
        cx.spawn(async move |this, cx| {
            let Some(path) = pick_output_folder(dialog).await else {
                return;
            };

            this.update(cx, |root, cx| {
                root.apply_default_output_directory(path);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(in crate::app) fn apply_default_output_directory(
        &mut self,
        path: std::path::PathBuf,
    ) -> bool {
        match self.set_default_output_directory(path) {
            Ok(()) => {
                self.settings_ui.output_directory_error = None;
                true
            }
            Err(error) => {
                self.open_app_settings();
                self.settings_ui.output_directory_error =
                    Some(format!("Failed to save settings: {error}"));
                false
            }
        }
    }

    pub(in crate::app) fn set_default_output_directory(
        &mut self,
        path: std::path::PathBuf,
    ) -> Result<(), crate::app_persistence::AppPersistenceError> {
        if self.update_installation_in_progress() {
            return Err(crate::app_persistence::AppPersistenceError::InstallationInProgress);
        }
        let previous = self.default_output_directory.replace(path);
        if let Err(error) = self.persist_app_settings() {
            self.default_output_directory = previous;
            return Err(error);
        }

        Ok(())
    }
}
