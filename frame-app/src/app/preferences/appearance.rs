use crate::app::components::popover_state::PopoverState;
use crate::app::components::{
    FrameSelectItem, FrameSelectProps, frame_field, frame_select_control,
};
use crate::app::inspector::{settings_field_label, settings_hint_text, settings_section};
use crate::app::motion::{motion_is_hidden, surface_motion};
use crate::app::preferences::state::AppearancePopover;
use crate::app::preferences::view::AppSettingsSelectFocuses;
use crate::app::primitives::{ButtonVariant, color};
use crate::app::root::FrameRoot;
use crate::appearance::{AppearanceSettings, ColorTheme, ScalePreset};
use crate::theme;
use gpui::prelude::{ParentElement, Styled};
use gpui::{
    Context, InteractiveElement, ScrollHandle, StatefulInteractiveElement, Window, div, px,
};

#[expect(
    clippy::too_many_arguments,
    reason = "The appearance section explicitly receives both independent select states, focus groups, and theme context."
)]
pub(in crate::app) fn app_settings_appearance_section(
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

pub(in crate::app) trait AppSettingsAppearanceValue: Copy + Eq + 'static {
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
pub(in crate::app) struct AppSettingsAppearanceSelectProps<'a, T> {
    pub(in crate::app) selected: T,
    pub(in crate::app) popover_state: PopoverState,
    pub(in crate::app) scroll_handle: &'a ScrollHandle,
    pub(in crate::app) focuses: AppSettingsSelectFocuses<'a>,
    pub(in crate::app) palette: &'static theme::ThemePalette,
}

pub(in crate::app) fn app_settings_appearance_select<T: AppSettingsAppearanceValue>(
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
    let items = T::options()
        .iter()
        .enumerate()
        .map(|(index, option)| FrameSelectItem {
            id: option.option_id(),
            label: option.display().into(),
            selected: *option == selected,
            enabled: true,
            focus: focuses.options.get(index).cloned(),
        })
        .collect();
    let progress =
        app_settings_appearance_popover_progress::<T>(popover_state.is_open(), window, cx);
    frame_field()
        .child(settings_field_label(T::LABEL, palette))
        .child(frame_select_control(
            FrameSelectProps {
                id: T::TRIGGER_ID,
                panel_id: T::PANEL_ID,
                list_id: T::LIST_ID,
                scrollbar_id: T::SCROLLBAR_ID,
                label: T::LABEL,
                display: selected.display(),
                items,
                enabled: true,
                open: popover_state.is_open(),
                rendered: popover_state.is_rendered(),
                progress,
                trigger_variant: if palette.is_light() {
                    ButtonVariant::Accent
                } else {
                    ButtonVariant::Outline
                },
                trigger_focus: Some(focuses.trigger),
                panel_focus: Some(focuses.panel),
                scroll: scroll_handle,
                palette,
            },
            |root, open, _, _| {
                if open {
                    root.open_app_settings_appearance_popover(T::POPOVER);
                } else {
                    root.close_app_settings_appearance_popover(T::POPOVER);
                }
            },
            |root, index, window, _| T::options()[index].apply(root, window),
            window,
            cx,
        ))
}

pub(in crate::app) fn app_settings_appearance_popover_progress<T: AppSettingsAppearanceValue>(
    is_open: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> f32 {
    let progress = surface_motion(T::MOTION_ID, is_open, window, cx);
    if !is_open && motion_is_hidden(progress) {
        cx.defer_in(window, move |root, _window, cx| {
            if root.finish_app_settings_appearance_popover_close(T::POPOVER) {
                cx.notify();
            }
        });
    }
    progress
}

pub(in crate::app) fn apply_app_settings_ui_scale(
    root: &mut FrameRoot,
    scale: ScalePreset,
    window: &mut Window,
) {
    if root.set_ui_scale(scale) {
        window.set_rem_size(px(crate::appearance::BASE_REM_PX * scale.factor()));
    }
}
