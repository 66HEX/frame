use super::{
    ButtonVariant, ClickEvent, Context, ConversionConfig, FRAME_ICON_BUTTON_SM_SIZE,
    FRAME_ICON_SM_SIZE, FluentBuilder, FocusHandle, FrameIconButtonSize, FrameIconButtonVariant,
    FrameRoot, FrameTextInputKind, FrameTextInputSpec, InteractiveElement, ParentElement,
    PresetDefinition, PresetNotice, PresetNoticeTone, PresetOption, SourceMetadata,
    StatefulInteractiveElement, Styled, Window, assets, color, div, frame_icon_button,
    frame_list_item, frame_text_button, frame_text_input, preset_options, settings_section_label,
    theme,
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
    let mut list = div().grid().grid_cols(1);
    for option in preset_options(state.config, state.presets, state.metadata) {
        list = list.child(settings_preset_row(
            option,
            state.settings_disabled,
            palette,
            window,
            cx,
        ));
    }

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(settings_presets_header(state.notice, palette))
        .child(settings_presets_save_row(
            state.preset_name,
            state.settings_disabled,
            state.preset_name_focus,
            palette,
            window,
            cx,
        ))
        .child(list)
}

fn settings_presets_header(
    notice: Option<&PresetNotice>,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let mut header = div()
        .relative()
        .w_full()
        .child(settings_section_label("Preset library", palette));
    if let Some(notice) = notice {
        header = header.child(
            div()
                .id("settings-presets-notice")
                .absolute()
                .top_0()
                .right_0()
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
                .child(theme::ui_text_owned(notice.text.clone())),
        );
    }

    header
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
    .pr(theme::ui_rem(4.0))
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
            .child(
                div()
                    .pr(theme::ui_rem(8.0))
                    .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                    .font_weight(theme::TEXT_WEIGHT_REGULAR)
                    .text_color(color(palette.text_muted))
                    .child(theme::ui_text(status.unwrap_or_default())),
            )
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
