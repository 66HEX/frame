use crate::app::components::popover_state::PopoverState;
use crate::app::components::{
    FRAME_SELECT_MAX_HEIGHT, apply_frame_select_popover_focus_trap, frame_select_content_height,
    frame_select_option, frame_select_option_with_focus, frame_select_options_list,
    frame_select_popover, frame_select_scrollbar, frame_select_target_index,
    frame_select_trigger_with_focus,
};
use crate::app::inspector::{settings_field_label, settings_hint_text, settings_section};
use crate::app::motion::{motion_is_hidden, popover_slide_offset, surface_motion};
use crate::app::preferences::state::AppearancePopover;
use crate::app::preferences::view::AppSettingsSelectFocuses;
use crate::app::primitives::color;
use crate::app::root::FrameRoot;
use crate::appearance::{AppearanceSettings, ColorTheme, ScalePreset};
use crate::theme;
use gpui::prelude::{ParentElement, Styled};
use gpui::{
    ClickEvent, Context, FocusHandle, InteractiveElement, ScrollHandle, StatefulInteractiveElement,
    Window, deferred, div, px,
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

#[expect(
    clippy::too_many_lines,
    reason = "The shared appearance select keeps its trigger, animated popover, keyboard navigation, and options together."
)]
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
        let mut list = frame_select_options_list(
            T::LIST_ID,
            scroll_handle,
            content_height > FRAME_SELECT_MAX_HEIGHT,
        );

        for (index, option) in options.iter().copied().enumerate() {
            let focus = option_focuses.get(index);
            list = list.child(app_settings_appearance_option(
                option,
                index,
                selected,
                popover_state.is_open(),
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
            54.0 + popover_slide_offset(progress),
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
            menu = menu.child(frame_select_scrollbar(
                T::SCROLLBAR_ID,
                scroll_handle.clone(),
                content_height,
                palette,
            ));
        }
        field = field.child(deferred(menu).priority(20));
    }

    field
}

#[expect(
    clippy::too_many_arguments,
    reason = "Each option needs its selection, focus, navigation, scrolling, and root action context."
)]
pub(in crate::app) fn app_settings_appearance_option<T: AppSettingsAppearanceValue>(
    option: T,
    index: usize,
    selected: T,
    is_open: bool,
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
        .tab_stop(is_open)
        .aria_disabled(!is_open)
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if event.is_keyboard() || !root.appearance_popover_state(T::POPOVER).is_open() {
                return;
            }
            option.apply(root, window);
            root.close_app_settings_appearance_popover(T::POPOVER);
            trigger_focus_for_click.focus(window, cx);
            cx.notify();
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                if !root.appearance_popover_state(T::POPOVER).is_open() {
                    return;
                }
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

pub(in crate::app) fn focus_app_settings_select_option(
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

pub(in crate::app) fn apply_app_settings_ui_scale(
    root: &mut FrameRoot,
    scale: ScalePreset,
    window: &mut Window,
) {
    if root.set_ui_scale(scale) {
        window.set_rem_size(px(crate::appearance::BASE_REM_PX * scale.factor()));
    }
}
