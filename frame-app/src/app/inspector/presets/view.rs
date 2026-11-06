use crate::app::components::{
    FRAME_ICON_BUTTON_SM_SIZE, FRAME_ICON_SM_SIZE, FrameIconButtonSize, FrameIconButtonVariant,
    frame_icon_button, frame_list_item, frame_text_button,
};
use crate::app::input::{FrameTextInputKind, FrameTextInputSpec, frame_text_input};
use crate::app::primitives::{ButtonVariant, color};
use crate::app::root::FrameRoot;
use crate::settings::{
    ConversionConfig, PresetDefinition, PresetNotice, PresetNoticeTone, PresetOption,
    SourceMetadata, preset_options,
};
use crate::{assets, theme};
use gpui::prelude::{FluentBuilder, ParentElement, Styled};
use gpui::{
    ClickEvent, Context, FocusHandle, InteractiveElement, StatefulInteractiveElement, Window, div,
};

#[derive(Clone, Copy)]
pub(in crate::app) struct SettingsPresetsTabState<'a> {
    pub(in crate::app) config: &'a ConversionConfig,
    pub(in crate::app) metadata: Option<&'a SourceMetadata>,
    pub(in crate::app) settings_disabled: bool,
    pub(in crate::app) preset_name: &'a str,
    pub(in crate::app) preset_name_focus: Option<&'a FocusHandle>,
    pub(in crate::app) presets: &'a [PresetDefinition],
    pub(in crate::app) notice: Option<&'a PresetNotice>,
    pub(in crate::app) palette: &'static theme::ThemePalette,
}

pub(in crate::app) fn settings_presets_tab(
    state: SettingsPresetsTabState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let palette = state.palette;
    let mut list = crate::app::components::frame_list_group();
    for option in preset_options(state.config, state.presets, state.metadata) {
        list = list.child(settings_preset_row(
            option,
            state.settings_disabled,
            palette,
            window,
            cx,
        ));
    }

    crate::app::components::frame_field_group()
        .child(settings_presets_save_row(
            state.preset_name,
            state.settings_disabled,
            state.preset_name_focus,
            palette,
            window,
            cx,
        ))
        .when_some(state.notice, |content, notice| {
            content.child(settings_presets_notice(notice, palette))
        })
        .child(list)
}

fn settings_presets_notice(
    notice: &PresetNotice,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id("settings-presets-notice")
        .w_full()
        .role(match notice.tone {
            PresetNoticeTone::Success => gpui::Role::Status,
            PresetNoticeTone::Error => gpui::Role::Alert,
        })
        .aria_label(notice.text.clone())
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .text_color(color(match notice.tone {
            PresetNoticeTone::Success => palette.text_primary,
            PresetNoticeTone::Error => palette.danger,
        }))
        .child(theme::ui_text_owned(notice.text.clone()))
}

fn settings_presets_save_row(
    preset_name: &str,
    settings_disabled: bool,
    preset_name_focus: Option<&FocusHandle>,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let save_enabled = !settings_disabled && !preset_name.trim().is_empty();
    div()
        .flex()
        .gap_2()
        .child(div().flex_1().child(frame_text_input(
            FrameTextInputSpec {
                id: "settings-preset-name-field",
                value: preset_name,
                placeholder: "Preset label",
                disabled: settings_disabled,
                focus: preset_name_focus,
                kind: FrameTextInputKind::PresetName,
            },
            palette,
            window,
            cx,
        )))
        .child(settings_save_preset_button(
            save_enabled,
            palette,
            window,
            cx,
        ))
}

fn settings_save_preset_button(
    enabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_text_button(
        "settings-save-preset",
        "Save",
        ButtonVariant::Secondary,
        false,
        enabled,
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if enabled && root.save_preset_from_draft() {
            cx.notify();
        }
    }))
}

fn settings_preset_row(
    option: PresetOption,
    settings_disabled: bool,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let preset = option.preset;
    let preset_id = preset.id.clone();
    let apply_all_id = preset.id.clone();
    let delete_id = preset.id.clone();
    let is_enabled = !settings_disabled && option.is_compatible;
    let selected = option.is_selected;
    let status = option.status;

    frame_list_item(
        format!("preset-{}", preset.id),
        preset.name.clone(),
        selected,
        is_enabled,
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if is_enabled && root.apply_preset_to_selected(&preset_id) {
            cx.notify();
        }
    }))
    .child(
        div()
            .min_w_0()
            .flex_1()
            .truncate()
            .child(preset.name.clone()),
    )
    .child(
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .when_some(status, |this, status| {
                this.child(
                    div()
                        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                        .font_weight(theme::TEXT_WEIGHT_REGULAR)
                        .text_color(color(palette.text_muted))
                        .child(theme::ui_text(status)),
                )
            })
            .when(option.is_compatible, |this| {
                this.child(settings_preset_icon_button(
                    format!("settings-preset-apply-all-{apply_all_id}"),
                    assets::ICON_LIST_CHECKS,
                    "Apply preset to all files",
                    FrameIconButtonVariant::Ghost,
                    !settings_disabled,
                    palette,
                    move |root, window, cx| {
                        root.confirm_apply_preset_to_all(&apply_all_id, window, cx);
                    },
                    window,
                    cx,
                ))
            })
            .when(!preset.built_in, |this| {
                this.child(settings_preset_icon_button(
                    format!("settings-preset-delete-{delete_id}"),
                    assets::ICON_TRASH,
                    "Delete preset",
                    FrameIconButtonVariant::DestructiveGhost,
                    !settings_disabled,
                    palette,
                    move |root, _window, cx| {
                        if root.delete_preset(&delete_id) {
                            cx.notify();
                        }
                    },
                    window,
                    cx,
                ))
            }),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Preset icon actions need explicit a11y labels plus the existing visual button contract."
)]
fn settings_preset_icon_button(
    id: String,
    icon: &'static str,
    label: &'static str,
    variant: FrameIconButtonVariant,
    enabled: bool,
    palette: &'static theme::ThemePalette,
    action: impl Fn(&mut FrameRoot, &mut Window, &mut Context<FrameRoot>) + 'static,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_icon_button(
        id,
        icon,
        label,
        variant,
        enabled,
        FrameIconButtonSize {
            button: FRAME_ICON_BUTTON_SM_SIZE,
            icon: FRAME_ICON_SM_SIZE,
        },
        palette,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
        cx.stop_propagation();
        if enabled {
            action(root, window, cx);
        }
    }))
}
