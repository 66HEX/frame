use super::*;

const SUBTITLE_FIELD_LABEL_STACK_HEIGHT: f32 = 20.0;
const SUBTITLE_POPOVER_TRIGGER_GAP: f32 = 8.0;
const SUBTITLE_POPOVER_TOP_OFFSET: f32 =
    SUBTITLE_FIELD_LABEL_STACK_HEIGHT + SETTINGS_CONTROL_HEIGHT + SUBTITLE_POPOVER_TRIGGER_GAP;

struct SettingsSubtitleColorDragPreview;

impl Render for SettingsSubtitleColorDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct SettingsSubtitleColorBoundsProbe {
    owner: Entity<FrameRoot>,
    target: SettingsSubtitleColorTarget,
    kind: SettingsSubtitleColorDragKind,
}

impl IntoElement for SettingsSubtitleColorBoundsProbe {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SettingsSubtitleColorBoundsProbe {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            position: Position::Absolute,
            size: size(relative(1.0).into(), relative(1.0).into()),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            ..Style::default()
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.owner.update(cx, |root, _cx| {
            root.set_subtitle_color_picker_bounds(self.target, self.kind, bounds);
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
}

#[derive(Clone, Copy)]
pub(in crate::app) struct SettingsSubtitlesTabState<'a> {
    pub(in crate::app) config: &'a ConversionConfig,
    pub(in crate::app) metadata: Option<&'a SourceMetadata>,
    pub(in crate::app) settings_disabled: bool,
    pub(in crate::app) subtitle_fonts: &'a [String],
    pub(in crate::app) focuses: SettingsSubtitleFocuses<'a>,
    pub(in crate::app) color_focuses: SettingsSubtitleColorInputFocuses<'a>,
    pub(in crate::app) active_popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) rendered_popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) font_select_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) font_size_select_scroll_handle: &'a ScrollHandle,
    pub(in crate::app) font_color_draft: &'a str,
    pub(in crate::app) outline_color_draft: &'a str,
    pub(in crate::app) font_color_hsv_draft: SettingsSubtitleHsv,
    pub(in crate::app) outline_color_hsv_draft: SettingsSubtitleHsv,
}

#[derive(Clone, Copy)]
struct SettingsSubtitleStyleState<'a> {
    config: &'a ConversionConfig,
    disabled: bool,
    subtitle_fonts: &'a [String],
    focuses: SettingsSubtitleFocuses<'a>,
    color_focuses: SettingsSubtitleColorInputFocuses<'a>,
    active_popover: Option<SettingsSubtitlePopover>,
    rendered_popover: Option<SettingsSubtitlePopover>,
    font_select_scroll_handle: &'a ScrollHandle,
    font_size_select_scroll_handle: &'a ScrollHandle,
    font_color_draft: &'a str,
    outline_color_draft: &'a str,
    font_color_hsv_draft: SettingsSubtitleHsv,
    outline_color_hsv_draft: SettingsSubtitleHsv,
}

struct SettingsSubtitleColorFieldSpec<'a> {
    label: &'static str,
    id: &'static str,
    value: String,
    disabled: bool,
    target: SettingsSubtitleColorTarget,
    popover_focuses: SettingsSubtitleColorPopoverFocuses<'a>,
    focus: Option<&'a FocusHandle>,
    active_popover: Option<SettingsSubtitlePopover>,
    rendered_popover: Option<SettingsSubtitlePopover>,
    draft: &'a str,
    hsv: SettingsSubtitleHsv,
}

#[derive(Clone, Copy)]
struct SettingsSubtitleFontSelectState<'a> {
    config: &'a ConversionConfig,
    disabled: bool,
    subtitle_fonts: &'a [String],
    active_popover: Option<SettingsSubtitlePopover>,
    rendered_popover: Option<SettingsSubtitlePopover>,
    popover: SettingsSubtitlePopover,
    scroll_handle: &'a ScrollHandle,
    focuses: SettingsSubtitleSelectFocuses<'a>,
}

#[derive(Clone, Copy)]
struct SettingsSubtitleFontSizeSelectState<'a> {
    config: &'a ConversionConfig,
    disabled: bool,
    active_popover: Option<SettingsSubtitlePopover>,
    rendered_popover: Option<SettingsSubtitlePopover>,
    popover: SettingsSubtitlePopover,
    scroll_handle: &'a ScrollHandle,
    focuses: SettingsSubtitleSelectFocuses<'a>,
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "Tab render state is a short-lived bundle of references and copyable focus handles consumed during render."
)]
pub(in crate::app) fn settings_subtitles_tab(
    state: SettingsSubtitlesTabState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let config = state.config;
    let copy_mode = config.processing_mode == ProcessingMode::Copy;
    let burn_in_disabled = state.settings_disabled || copy_mode;
    let content = div().flex().flex_col().gap_4().child(
        settings_section("Burn-in subtitles")
            .child(settings_subtitle_burn_button(
                config,
                burn_in_disabled,
                state.focuses.burn_file,
                window,
                cx,
            ))
            .child(settings_hint_text(if copy_mode {
                "Burn-in subtitles are disabled in stream copy mode."
            } else {
                "Burning in subtitles will force video re-encoding."
            })),
    );

    let content = if copy_mode {
        content
    } else {
        content.child(
            settings_section("Style").child(settings_subtitle_style_controls(
                SettingsSubtitleStyleState {
                    config,
                    disabled: burn_in_disabled,
                    subtitle_fonts: state.subtitle_fonts,
                    focuses: state.focuses,
                    color_focuses: state.color_focuses,
                    active_popover: state.active_popover,
                    rendered_popover: state.rendered_popover,
                    font_select_scroll_handle: state.font_select_scroll_handle,
                    font_size_select_scroll_handle: state.font_size_select_scroll_handle,
                    font_color_draft: state.font_color_draft,
                    outline_color_draft: state.outline_color_draft,
                    font_color_hsv_draft: state.font_color_hsv_draft,
                    outline_color_hsv_draft: state.outline_color_hsv_draft,
                },
                window,
                cx,
            )),
        )
    };

    let track_options = subtitle_track_options(config, state.metadata, state.settings_disabled);
    if track_options.is_empty() {
        return content
            .child(settings_section("Source tracks").child(settings_hint_text("No subtitles")));
    }

    let mut list = div().grid().grid_cols(1).gap_2();
    for option in track_options {
        list = list.child(settings_subtitle_track_button(option, window, cx));
    }

    content.child(settings_section("Source tracks").child(list))
}

fn settings_subtitle_burn_button(
    config: &ConversionConfig,
    disabled: bool,
    burn_file_focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let has_path = config.subtitle_burn_path.is_some();

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(settings_subtitle_load_button(
                    config,
                    disabled,
                    burn_file_focus,
                    window,
                    cx,
                )),
        )
        .child(settings_subtitle_clear_button(
            disabled || !has_path,
            window,
            cx,
        ))
}

fn settings_subtitle_load_button(
    config: &ConversionConfig,
    disabled: bool,
    focus: Option<&FocusHandle>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Secondary, false, !disabled);
    let animated = animated_button_colors("settings-subtitle-burn-file", colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let motion = animated.motion;
    let has_path = config.subtitle_burn_path.is_some();
    let label = subtitle_burn_file_label(config);
    let display_label = if has_path {
        label.clone()
    } else {
        theme::ui_text_owned(label.clone())
    };

    let button = div()
        .id("settings-subtitle-burn-file")
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .bg(background)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(!disabled, |this| {
            this.hover(gpui::Styled::cursor_pointer)
                .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(disabled, gpui::Styled::cursor_not_allowed)
        .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            root.prompt_subtitle_burn_file(window, cx);
        }))
        .child(div().truncate().child(display_label));

    let button = apply_button_motion(button, motion, !disabled);

    if let Some(focus) = focus {
        apply_accessible_button_with_focus(button, label, !disabled, focus)
    } else {
        apply_accessible_button(button, label, !disabled)
    }
}

fn settings_subtitle_clear_button(
    disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_icon_button(
        "settings-subtitle-clear-file",
        assets::ICON_TRASH,
        "Clear subtitle file",
        FrameIconButtonVariant::DestructiveGhost,
        !disabled,
        FrameIconButtonSize {
            button: SETTINGS_CONTROL_HEIGHT,
            icon: FRAME_ICON_SM_SIZE,
        },
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if disabled {
            return;
        }
        if root.update_selected_config(|config| apply_subtitle_burn_path(config, None)) {
            cx.notify();
        }
    }))
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "Style render state is a short-lived bundle of references and copyable focus handles consumed during render."
)]
fn settings_subtitle_style_controls(
    state: SettingsSubtitleStyleState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(settings_subtitle_font_select(
                    SettingsSubtitleFontSelectState {
                        config: state.config,
                        disabled: state.disabled,
                        subtitle_fonts: state.subtitle_fonts,
                        active_popover: state.active_popover,
                        rendered_popover: state.rendered_popover,
                        popover: SettingsSubtitlePopover::FontName,
                        scroll_handle: state.font_select_scroll_handle,
                        focuses: state.focuses.font_select,
                    },
                    window,
                    cx,
                ))
                .child(settings_subtitle_font_size_select(
                    SettingsSubtitleFontSizeSelectState {
                        config: state.config,
                        disabled: state.disabled,
                        active_popover: state.active_popover,
                        rendered_popover: state.rendered_popover,
                        popover: SettingsSubtitlePopover::FontSize,
                        scroll_handle: state.font_size_select_scroll_handle,
                        focuses: state.focuses.font_size_select,
                    },
                    window,
                    cx,
                )),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(settings_subtitle_color_field(
                    SettingsSubtitleColorFieldSpec {
                        label: "Text color",
                        id: "settings-subtitle-font-color",
                        value: subtitle_color_value(
                            state.config.subtitle_font_color.as_ref(),
                            DEFAULT_SUBTITLE_FONT_COLOR,
                        ),
                        disabled: state.disabled,
                        target: SettingsSubtitleColorTarget::Font,
                        popover_focuses: state.focuses.font_color,
                        focus: state.color_focuses.font,
                        active_popover: state.active_popover,
                        rendered_popover: state.rendered_popover,
                        draft: state.font_color_draft,
                        hsv: state.font_color_hsv_draft,
                    },
                    window,
                    cx,
                ))
                .child(settings_subtitle_color_field(
                    SettingsSubtitleColorFieldSpec {
                        label: "Outline color",
                        id: "settings-subtitle-outline-color",
                        value: subtitle_color_value(
                            state.config.subtitle_outline_color.as_ref(),
                            DEFAULT_SUBTITLE_OUTLINE_COLOR,
                        ),
                        disabled: state.disabled,
                        target: SettingsSubtitleColorTarget::Outline,
                        popover_focuses: state.focuses.outline_color,
                        focus: state.color_focuses.outline,
                        active_popover: state.active_popover,
                        rendered_popover: state.rendered_popover,
                        draft: state.outline_color_draft,
                        hsv: state.outline_color_hsv_draft,
                    },
                    window,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_field_label("Position"))
                .child(settings_subtitle_position_grid(
                    state.config,
                    state.disabled,
                    window,
                    cx,
                )),
        )
        .child(settings_hint_text(
            "Style applies to burned-in subtitles only.",
        ))
}

fn focus_optional(focus: Option<&FocusHandle>, window: &mut Window, cx: &mut Context<FrameRoot>) {
    if let Some(focus) = focus {
        focus.focus(window, cx);
    }
}

fn defer_focus_optional(focus: Option<FocusHandle>, window: &Window, cx: &mut Context<FrameRoot>) {
    if let Some(focus) = focus {
        cx.defer_in(window, move |_root, window, cx| {
            focus.focus(window, cx);
        });
    }
}

const fn subtitle_select_option_focus(
    index: usize,
    option_count: usize,
    focuses: SettingsSubtitleSelectFocuses<'_>,
) -> Option<&FocusHandle> {
    if index == 0 {
        focuses.first_option
    } else if index + 1 == option_count {
        focuses.last_option
    } else {
        None
    }
}

const fn subtitle_select_last_focus(
    option_count: usize,
    focuses: SettingsSubtitleSelectFocuses<'_>,
) -> Option<&FocusHandle> {
    if option_count <= 1 {
        focuses.first_option
    } else {
        focuses.last_option
    }
}

fn focus_subtitle_select_initial_target(
    key: &str,
    option_count: usize,
    first_focus: Option<&FocusHandle>,
    last_focus: Option<&FocusHandle>,
    scroll_handle: &ScrollHandle,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) {
    if option_count == 0 {
        return;
    }
    let target_index = if matches!(key, "up" | "end") {
        option_count.saturating_sub(1)
    } else {
        0
    };
    scroll_handle.scroll_to_item(target_index);
    if target_index == 0 {
        focus_optional(first_focus, window, cx);
    } else {
        focus_optional(last_focus.or(first_focus), window, cx);
    }
}

fn apply_subtitle_select_popover_focus_trap(
    popover: gpui::Stateful<gpui::Div>,
    focuses: SettingsSubtitleSelectFocuses<'_>,
    option_count: usize,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let Some(panel_focus) = focuses.panel else {
        return popover;
    };
    let Some(first_focus) = focuses.first_option else {
        return popover;
    };
    let Some(last_focus) = subtitle_select_last_focus(option_count, focuses) else {
        return popover;
    };

    let first_focus = first_focus.clone();
    let last_focus = last_focus.clone();
    popover
        .track_focus(panel_focus)
        .tab_stop(false)
        .on_key_down(
            cx.listener(move |_root, event: &gpui::KeyDownEvent, window, cx| {
                handle_modal_tab_navigation(event, &first_focus, &last_focus, window, cx);
            }),
        )
}

#[derive(Clone, Copy)]
struct SubtitleSelectFocusTarget<'a> {
    current_index: usize,
    target_index: usize,
    first_focus: Option<&'a FocusHandle>,
    last_focus: Option<&'a FocusHandle>,
    scroll_handle: &'a ScrollHandle,
}

fn focus_subtitle_select_target(
    key: &str,
    target: SubtitleSelectFocusTarget<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) {
    target.scroll_handle.scroll_to_item(target.target_index);
    match key {
        "home" => focus_optional(target.first_focus, window, cx),
        "end" => focus_optional(target.last_focus, window, cx),
        "down" if target.target_index <= target.current_index => {
            focus_optional(target.first_focus, window, cx);
        }
        "up" if target.target_index >= target.current_index => {
            focus_optional(target.last_focus, window, cx);
        }
        "down" => window.focus_next(cx),
        "up" => window.focus_prev(cx),
        _ => {}
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Subtitle font select keeps trigger, keyboard handling, popover, and scrollbar together for one GPUI control."
)]
fn settings_subtitle_font_select(
    state: SettingsSubtitleFontSelectState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let display = state
        .config
        .subtitle_font_name
        .as_deref()
        .filter(|font| !font.is_empty())
        .unwrap_or("Default (e.g. Arial)");
    let options = subtitle_font_options(state.config, state.subtitle_fonts, state.disabled);
    let has_options = !options.is_empty();
    let enabled = !state.disabled && has_options;
    let popover = state.popover;
    let trigger = if let Some(focus) = state.focuses.trigger {
        frame_select_trigger_with_focus(
            "settings-subtitle-font-select",
            "Subtitle font",
            display,
            enabled,
            state.rendered_popover == Some(popover),
            focus,
            window,
            cx,
        )
    } else {
        frame_select_trigger(
            "settings-subtitle-font-select",
            "Subtitle font",
            display,
            enabled,
            state.rendered_popover == Some(popover),
            window,
            cx,
        )
    };
    let key_first_option_focus = state.focuses.first_option.cloned();
    let key_last_option_focus = subtitle_select_last_focus(options.len(), state.focuses).cloned();
    let key_scroll_handle = state.scroll_handle.clone();
    let option_count = options.len();

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label("Font"))
        .child(
            trigger
                .on_click(cx.listener(move |root, event: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if event.is_keyboard() {
                        return;
                    }
                    root.toggle_subtitle_popover(popover);
                    cx.notify();
                }))
                .on_key_down(
                    cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                        if !enabled {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "down" | "up" if root.subtitle_ui.popover == Some(popover) => {
                                cx.stop_propagation();
                                focus_subtitle_select_initial_target(
                                    event.keystroke.key.as_str(),
                                    option_count,
                                    key_first_option_focus.as_ref(),
                                    key_last_option_focus.as_ref(),
                                    &key_scroll_handle,
                                    window,
                                    cx,
                                );
                            }
                            "down" | "up" | "home" | "end" => {
                                cx.stop_propagation();
                                root.subtitle_ui.popover = Some(popover);
                                root.subtitle_ui.rendered_popover = Some(popover);
                                focus_subtitle_select_initial_target(
                                    event.keystroke.key.as_str(),
                                    option_count,
                                    key_first_option_focus.as_ref(),
                                    key_last_option_focus.as_ref(),
                                    &key_scroll_handle,
                                    window,
                                    cx,
                                );
                                cx.notify();
                            }
                            "enter" | "space" if root.subtitle_ui.popover == Some(popover) => {
                                cx.stop_propagation();
                                root.close_subtitle_popover();
                                cx.notify();
                            }
                            "enter" | "space" => {
                                cx.stop_propagation();
                                root.subtitle_ui.popover = Some(popover);
                                root.subtitle_ui.rendered_popover = Some(popover);
                                key_scroll_handle.scroll_to_item(0);
                                defer_focus_optional(key_first_option_focus.clone(), window, cx);
                                cx.notify();
                            }
                            "escape" => {
                                cx.stop_propagation();
                                root.close_subtitle_popover();
                                cx.notify();
                            }
                            _ => {}
                        }
                    }),
                ),
        );

    if state.rendered_popover == Some(popover) && has_options {
        let progress =
            subtitle_popover_progress(popover, state.active_popover == Some(popover), window, cx);
        let content_height = frame_select_content_height(options.len());
        let mut list =
            frame_select_options_list("settings-subtitle-font-options-list", state.scroll_handle);

        let option_count = options.len();
        let option_keyboard_options = options.clone();
        for (index, option) in options.into_iter().enumerate() {
            let name = option.name.clone();
            let is_enabled = !option.is_disabled;
            let option_focus = subtitle_select_option_focus(index, option_count, state.focuses);
            list = list.child(settings_subtitle_font_option(
                option,
                is_enabled,
                name,
                option_keyboard_options.clone(),
                index,
                option_count,
                option_focus,
                state.focuses,
                state.scroll_handle,
                cx,
            ));
        }

        let mut popover = frame_select_popover(
            "settings-subtitle-font-options",
            SUBTITLE_POPOVER_TOP_OFFSET + subtitle_popover_slide_offset(progress),
            progress,
            list,
        );
        popover =
            apply_subtitle_select_popover_focus_trap(popover, state.focuses, option_count, cx);

        if content_height > FRAME_SELECT_MAX_HEIGHT {
            popover = popover.child(frame_vertical_scrollbar(
                "settings-subtitle-font-options-scrollbar",
                state.scroll_handle.clone(),
                content_height,
            ));
        }

        field = field.child(deferred(popover).with_priority(10));
    }

    field
}

#[expect(
    clippy::too_many_lines,
    reason = "Subtitle font size select keeps trigger, keyboard handling, popover, and scrollbar together for one GPUI control."
)]
fn settings_subtitle_font_size_select(
    state: SettingsSubtitleFontSizeSelectState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let display = state
        .config
        .subtitle_font_size
        .as_deref()
        .filter(|size| !size.is_empty())
        .unwrap_or("Default");
    let options = subtitle_font_size_options(state.config, state.disabled);
    let enabled = !state.disabled;
    let popover = state.popover;
    let trigger = if let Some(focus) = state.focuses.trigger {
        frame_select_trigger_with_focus(
            "settings-subtitle-font-size-select",
            "Subtitle font size",
            display,
            enabled,
            state.rendered_popover == Some(popover),
            focus,
            window,
            cx,
        )
    } else {
        frame_select_trigger(
            "settings-subtitle-font-size-select",
            "Subtitle font size",
            display,
            enabled,
            state.rendered_popover == Some(popover),
            window,
            cx,
        )
    };
    let key_first_option_focus = state.focuses.first_option.cloned();
    let key_last_option_focus = subtitle_select_last_focus(options.len(), state.focuses).cloned();
    let key_scroll_handle = state.scroll_handle.clone();
    let option_count = options.len();

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label("Size"))
        .child(
            trigger
                .on_click(cx.listener(move |root, event: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if event.is_keyboard() {
                        return;
                    }
                    root.toggle_subtitle_popover(popover);
                    cx.notify();
                }))
                .on_key_down(
                    cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                        if !enabled {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "down" | "up" if root.subtitle_ui.popover == Some(popover) => {
                                cx.stop_propagation();
                                focus_subtitle_select_initial_target(
                                    event.keystroke.key.as_str(),
                                    option_count,
                                    key_first_option_focus.as_ref(),
                                    key_last_option_focus.as_ref(),
                                    &key_scroll_handle,
                                    window,
                                    cx,
                                );
                            }
                            "down" | "up" | "home" | "end" => {
                                cx.stop_propagation();
                                root.subtitle_ui.popover = Some(popover);
                                root.subtitle_ui.rendered_popover = Some(popover);
                                focus_subtitle_select_initial_target(
                                    event.keystroke.key.as_str(),
                                    option_count,
                                    key_first_option_focus.as_ref(),
                                    key_last_option_focus.as_ref(),
                                    &key_scroll_handle,
                                    window,
                                    cx,
                                );
                                cx.notify();
                            }
                            "enter" | "space" if root.subtitle_ui.popover == Some(popover) => {
                                cx.stop_propagation();
                                root.close_subtitle_popover();
                                cx.notify();
                            }
                            "enter" | "space" => {
                                cx.stop_propagation();
                                root.subtitle_ui.popover = Some(popover);
                                root.subtitle_ui.rendered_popover = Some(popover);
                                key_scroll_handle.scroll_to_item(0);
                                defer_focus_optional(key_first_option_focus.clone(), window, cx);
                                cx.notify();
                            }
                            "escape" => {
                                cx.stop_propagation();
                                root.close_subtitle_popover();
                                cx.notify();
                            }
                            _ => {}
                        }
                    }),
                ),
        );

    if state.rendered_popover == Some(popover) {
        let progress =
            subtitle_popover_progress(popover, state.active_popover == Some(popover), window, cx);
        let content_height = frame_select_content_height(options.len());
        let mut list = frame_select_options_list(
            "settings-subtitle-font-size-options-list",
            state.scroll_handle,
        );

        let option_count = options.len();
        let option_keyboard_options = options.to_vec();
        for (index, option) in options.into_iter().enumerate() {
            let size = option.size;
            let is_enabled = !option.is_disabled;
            let option_focus = subtitle_select_option_focus(index, option_count, state.focuses);
            list = list.child(settings_subtitle_size_option(
                option,
                is_enabled,
                size,
                option_keyboard_options.clone(),
                index,
                option_count,
                option_focus,
                state.focuses,
                state.scroll_handle,
                cx,
            ));
        }

        let mut popover = frame_select_popover(
            "settings-subtitle-font-size-options",
            SUBTITLE_POPOVER_TOP_OFFSET + subtitle_popover_slide_offset(progress),
            progress,
            list,
        );
        popover =
            apply_subtitle_select_popover_focus_trap(popover, state.focuses, option_count, cx);

        if content_height > FRAME_SELECT_MAX_HEIGHT {
            popover = popover.child(frame_vertical_scrollbar(
                "settings-subtitle-font-size-options-scrollbar",
                state.scroll_handle.clone(),
                content_height,
            ));
        }

        field = field.child(deferred(popover).with_priority(10));
    }

    field
}

#[expect(
    clippy::too_many_arguments,
    reason = "Font options need option state, list navigation context, optional focus, and render context."
)]
fn settings_subtitle_font_option(
    option: SubtitleFontOption,
    is_enabled: bool,
    name: String,
    keyboard_options: Vec<SubtitleFontOption>,
    index: usize,
    option_count: usize,
    focus: Option<&FocusHandle>,
    focuses: SettingsSubtitleSelectFocuses<'_>,
    scroll_handle: &ScrollHandle,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let click_name = name.clone();
    let key_name = name;
    let trigger_focus_for_click = focuses.trigger.cloned();
    let trigger_focus_for_key = focuses.trigger.cloned();
    let first_focus_for_key = focuses.first_option.cloned();
    let last_focus_for_key = subtitle_select_last_focus(option_count, focuses).cloned();
    let scroll_handle_for_key = scroll_handle.clone();
    let option = if let Some(focus) = focus {
        frame_select_option_with_focus(
            format!("subtitle-font-{click_name}"),
            option.name,
            option.is_selected,
            is_enabled,
            focus,
        )
    } else {
        frame_select_option(
            format!("subtitle-font-{click_name}"),
            option.name,
            option.is_selected,
            is_enabled,
        )
    };
    option
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if event.is_keyboard() {
                return;
            }
            if !is_enabled {
                return;
            }
            let changed =
                root.update_selected_config(|config| apply_subtitle_font_name(config, &click_name));
            root.close_subtitle_popover();
            focus_optional(trigger_focus_for_click.as_ref(), window, cx);
            if changed {
                cx.notify();
            }
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                match key {
                    "enter" | "space" if is_enabled => {
                        cx.stop_propagation();
                        let changed = root.update_selected_config(|config| {
                            apply_subtitle_font_name(config, &key_name)
                        });
                        root.close_subtitle_popover();
                        defer_focus_optional(trigger_focus_for_key.clone(), window, cx);
                        if changed {
                            cx.notify();
                        }
                    }
                    "up" | "down" | "home" | "end" if is_enabled => {
                        let target_index = subtitle_select_target_index(
                            keyboard_options.len(),
                            Some(index),
                            key,
                            |index| !keyboard_options[index].is_disabled,
                        );
                        if let Some(target_index) = target_index {
                            focus_subtitle_select_target(
                                key,
                                SubtitleSelectFocusTarget {
                                    current_index: index,
                                    target_index,
                                    first_focus: first_focus_for_key.as_ref(),
                                    last_focus: last_focus_for_key.as_ref(),
                                    scroll_handle: &scroll_handle_for_key,
                                },
                                window,
                                cx,
                            );
                        }
                        cx.stop_propagation();
                    }
                    "escape" => {
                        cx.stop_propagation();
                        root.close_subtitle_popover();
                        focus_optional(trigger_focus_for_key.as_ref(), window, cx);
                        cx.notify();
                    }
                    _ => {}
                }
            }),
        )
}

#[expect(
    clippy::too_many_arguments,
    clippy::option_if_let_else,
    reason = "Size options mirror the font option builder and preserve the explicit focus wiring."
)]
fn settings_subtitle_size_option(
    option: SubtitleFontSizeOption,
    is_enabled: bool,
    size: &'static str,
    keyboard_options: Vec<SubtitleFontSizeOption>,
    index: usize,
    option_count: usize,
    focus: Option<&FocusHandle>,
    focuses: SettingsSubtitleSelectFocuses<'_>,
    scroll_handle: &ScrollHandle,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let trigger_focus_for_click = focuses.trigger.cloned();
    let trigger_focus_for_key = focuses.trigger.cloned();
    let first_focus_for_key = focuses.first_option.cloned();
    let last_focus_for_key = subtitle_select_last_focus(option_count, focuses).cloned();
    let scroll_handle_for_key = scroll_handle.clone();
    let option = if let Some(focus) = focus {
        frame_select_option_with_focus(
            format!("subtitle-size-{size}"),
            option.size,
            option.is_selected,
            is_enabled,
            focus,
        )
    } else {
        frame_select_option(
            format!("subtitle-size-{size}"),
            option.size,
            option.is_selected,
            is_enabled,
        )
    };
    option
        .on_click(cx.listener(move |root, event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            if event.is_keyboard() {
                return;
            }
            if !is_enabled {
                return;
            }
            let changed =
                root.update_selected_config(|config| apply_subtitle_font_size(config, size));
            root.close_subtitle_popover();
            focus_optional(trigger_focus_for_click.as_ref(), window, cx);
            if changed {
                cx.notify();
            }
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                match key {
                    "enter" | "space" if is_enabled => {
                        cx.stop_propagation();
                        let changed = root.update_selected_config(|config| {
                            apply_subtitle_font_size(config, size)
                        });
                        root.close_subtitle_popover();
                        defer_focus_optional(trigger_focus_for_key.clone(), window, cx);
                        if changed {
                            cx.notify();
                        }
                    }
                    "up" | "down" | "home" | "end" if is_enabled => {
                        let target_index = subtitle_select_target_index(
                            keyboard_options.len(),
                            Some(index),
                            key,
                            |index| !keyboard_options[index].is_disabled,
                        );
                        if let Some(target_index) = target_index {
                            focus_subtitle_select_target(
                                key,
                                SubtitleSelectFocusTarget {
                                    current_index: index,
                                    target_index,
                                    first_focus: first_focus_for_key.as_ref(),
                                    last_focus: last_focus_for_key.as_ref(),
                                    scroll_handle: &scroll_handle_for_key,
                                },
                                window,
                                cx,
                            );
                        }
                        cx.stop_propagation();
                    }
                    "escape" => {
                        cx.stop_propagation();
                        root.close_subtitle_popover();
                        focus_optional(trigger_focus_for_key.as_ref(), window, cx);
                        cx.notify();
                    }
                    _ => {}
                }
            }),
        )
}

fn subtitle_popover_progress(
    popover: SettingsSubtitlePopover,
    is_open: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> f32 {
    let transition = window
        .use_keyed_transition(
            subtitle_popover_motion_key(popover),
            cx,
            INTERACTION_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_in_out);
    let target = motion_target(is_open);
    set_motion_target(&transition, target, cx);
    let progress = *transition.evaluate(window, cx);

    if !is_open && motion_is_hidden(progress) {
        cx.defer_in(window, move |root, _window, cx| {
            if root.finish_subtitle_popover_close(popover) {
                cx.notify();
            }
        });
    }

    progress
}

const fn subtitle_popover_motion_key(popover: SettingsSubtitlePopover) -> &'static str {
    match popover {
        SettingsSubtitlePopover::FontName => "settings-subtitle-font-popover-motion",
        SettingsSubtitlePopover::FontSize => "settings-subtitle-size-popover-motion",
        SettingsSubtitlePopover::FontColor => "settings-subtitle-font-color-popover-motion",
        SettingsSubtitlePopover::OutlineColor => "settings-subtitle-outline-color-popover-motion",
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "The color field keeps trigger behavior, draft color setup, and popover mounting together."
)]
fn settings_subtitle_color_field(
    spec: SettingsSubtitleColorFieldSpec<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let SettingsSubtitleColorFieldSpec {
        label,
        id,
        value,
        disabled,
        target,
        popover_focuses,
        focus,
        active_popover,
        rendered_popover,
        draft,
        hsv,
    } = spec;
    let popover = match target {
        SettingsSubtitleColorTarget::Font => SettingsSubtitlePopover::FontColor,
        SettingsSubtitleColorTarget::Outline => SettingsSubtitlePopover::OutlineColor,
    };
    let enabled = !disabled;
    let click_value = value.clone();
    let key_value = value.clone();
    let trigger_content = frame_color_select_value(&value);
    let trigger = if let Some(focus) = popover_focuses.trigger {
        frame_select_trigger_content_with_focus(
            id,
            label,
            trigger_content,
            enabled,
            rendered_popover == Some(popover),
            focus,
            window,
            cx,
        )
    } else {
        frame_select_trigger_content(
            id,
            label,
            trigger_content,
            enabled,
            rendered_popover == Some(popover),
            window,
            cx,
        )
    };
    let click_first_focus = popover_focuses.sv.cloned();
    let key_first_focus = popover_focuses.sv.cloned();

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label(label))
        .child(
            trigger
                .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    if !enabled {
                        return;
                    }
                    root.toggle_subtitle_color_popover(popover, target, &click_value);
                    if root.subtitle_ui.popover == Some(popover)
                        && let Some(focus) = click_first_focus.as_ref()
                    {
                        focus.focus(window, cx);
                    }
                    cx.notify();
                }))
                .on_key_down(
                    cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                        if !enabled {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "down" | "up" => {
                                root.open_subtitle_color_popover(popover, target, &key_value);
                                if let Some(focus) = key_first_focus.as_ref() {
                                    focus.focus(window, cx);
                                }
                                cx.stop_propagation();
                                cx.notify();
                            }
                            "enter" | "space" => {
                                root.toggle_subtitle_color_popover(popover, target, &key_value);
                                if root.subtitle_ui.popover == Some(popover)
                                    && let Some(focus) = key_first_focus.as_ref()
                                {
                                    focus.focus(window, cx);
                                }
                                cx.stop_propagation();
                                cx.notify();
                            }
                            "escape" => {
                                root.close_subtitle_popover();
                                cx.stop_propagation();
                                cx.notify();
                            }
                            _ => {}
                        }
                    }),
                ),
        );

    if rendered_popover == Some(popover) {
        let progress =
            subtitle_popover_progress(popover, active_popover == Some(popover), window, cx);
        field = field.child(
            deferred(settings_subtitle_color_picker(
                target,
                hsv,
                draft,
                focus,
                popover_focuses,
                progress,
                window,
                cx,
            ))
            .with_priority(10),
        );
    }

    field
}

#[expect(
    clippy::too_many_arguments,
    reason = "The color picker needs color state, input focus, popover focus handles, animation progress, and render context."
)]
fn settings_subtitle_color_picker(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    draft: &str,
    focus: Option<&FocusHandle>,
    popover_focuses: SettingsSubtitleColorPopoverFocuses<'_>,
    progress: f32,
    window: &Window,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let input_kind = match target {
        SettingsSubtitleColorTarget::Font => FrameTextInputKind::SubtitleFontColorHex,
        SettingsSubtitleColorTarget::Outline => FrameTextInputKind::SubtitleOutlineColorHex,
    };

    let panel_id = match target {
        SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-picker",
        SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-picker",
    };
    let panel = frame_color_picker_panel(
        SUBTITLE_POPOVER_TOP_OFFSET + subtitle_popover_slide_offset(progress),
        progress,
        settings_subtitle_sv_square(target, hsv, popover_focuses.sv, cx),
        settings_subtitle_hue_slider(target, hsv, popover_focuses.hue, cx),
        frame_text_input(
            FrameTextInputSpec {
                id: match target {
                    SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-hex",
                    SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-hex",
                },
                value: draft,
                placeholder: "#FFFFFF",
                disabled: false,
                focus,
                kind: input_kind,
            },
            window,
            cx,
        ),
    )
    .id(panel_id);

    let Some(panel_focus) = popover_focuses.panel else {
        return panel;
    };
    let Some(first_focus) = popover_focuses.sv else {
        return panel;
    };
    let Some(last_focus) = focus else {
        return panel;
    };

    let first_focus = first_focus.clone();
    let last_focus = last_focus.clone();
    let trigger_focus = popover_focuses.trigger.cloned();
    panel
        .track_focus(panel_focus)
        .tab_stop(false)
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                if handle_modal_tab_navigation(event, &first_focus, &last_focus, window, cx) {
                    return;
                }
                if event.keystroke.key.as_str() == "escape" {
                    root.close_subtitle_popover();
                    if let Some(focus) = trigger_focus.as_ref() {
                        focus.focus(window, cx);
                    }
                    cx.stop_propagation();
                    cx.notify();
                }
            }),
        )
}

fn settings_subtitle_sv_square(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    focus: Option<&FocusHandle>,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let drag = SettingsSubtitleColorDrag {
        target,
        kind: SettingsSubtitleColorDragKind::SaturationValue,
        base_hsv: hsv,
    };
    let square = div()
        .id(match target {
            SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-sv",
            SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-sv",
        })
        .role(gpui::Role::ColorWell)
        .aria_label(settings_subtitle_sv_label(target))
        .aria_value(settings_subtitle_sv_value(hsv))
        .relative()
        .h(px(FRAME_COLOR_PICKER_SV_HEIGHT))
        .w_full()
        .overflow_hidden()
        .rounded(px(theme::RADIUS_SM))
        .border_1()
        .border_color(color(theme::FRAME_GRAY_200))
        .bg(color(theme::FRAME_GRAY_100))
        .cursor_crosshair()
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |root, event: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                if root.commit_subtitle_color_at_position(
                    target,
                    SettingsSubtitleColorDragKind::SaturationValue,
                    event.position,
                ) {
                    cx.notify();
                }
            }),
        )
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<SettingsSubtitleColorDrag>, _window, cx| {
                let drag = *event.drag(cx);
                if drag.kind != SettingsSubtitleColorDragKind::SaturationValue {
                    return;
                }
                if root.commit_subtitle_color_drag_at_position(drag, event.event.position) {
                    cx.notify();
                }
            },
        ))
        .child(SettingsSubtitleColorBoundsProbe {
            owner: cx.entity(),
            target,
            kind: SettingsSubtitleColorDragKind::SaturationValue,
        })
        .child(frame_color_picker_sv_canvas(hsv.h, hsv.s, hsv.v))
        .on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsSubtitleColorDragPreview)
        })
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                let Some(next_hsv) = subtitle_sv_hsv_for_key(hsv, event.keystroke.key.as_str())
                else {
                    return;
                };
                if root.commit_subtitle_hsv_color(target, next_hsv) {
                    cx.notify();
                }
                cx.stop_propagation();
            }),
        );

    if let Some(focus) = focus {
        square
            .track_focus(focus)
            .tab_stop(true)
            .focus_visible(focus_visible_ring)
    } else {
        square
            .focusable()
            .tab_stop(true)
            .focus_visible(focus_visible_ring)
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "The hue slider combines pointer, accessibility increment/decrement, and keyboard behavior for one control."
)]
fn settings_subtitle_hue_slider(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    focus: Option<&FocusHandle>,
    cx: &Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let drag = SettingsSubtitleColorDrag {
        target,
        kind: SettingsSubtitleColorDragKind::Hue,
        base_hsv: hsv,
    };
    let owner = cx.entity();
    let decrement_owner = owner.clone();

    let slider = div()
        .id(match target {
            SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-hue",
            SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-hue",
        })
        .relative()
        .h(px(FRAME_COLOR_PICKER_HUE_VISUAL_HEIGHT))
        .w_full()
        .cursor_ew_resize()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |root, event: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                if root.commit_subtitle_color_at_position(
                    target,
                    SettingsSubtitleColorDragKind::Hue,
                    event.position,
                ) {
                    cx.notify();
                }
            }),
        )
        .on_drag_move(cx.listener(
            |root, event: &DragMoveEvent<SettingsSubtitleColorDrag>, _window, cx| {
                let drag = *event.drag(cx);
                if drag.kind != SettingsSubtitleColorDragKind::Hue {
                    return;
                }
                if root.commit_subtitle_color_drag_at_position(drag, event.event.position) {
                    cx.notify();
                }
            },
        ))
        .child(SettingsSubtitleColorBoundsProbe {
            owner: cx.entity(),
            target,
            kind: SettingsSubtitleColorDragKind::Hue,
        })
        .child(frame_color_picker_hue_track())
        .child(frame_color_picker_hue_handle(hsv.h))
        .on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsSubtitleColorDragPreview)
        });

    let slider = if let Some(focus) = focus {
        apply_accessible_slider_with_focus(
            slider,
            settings_subtitle_hue_label(target),
            true,
            hsv.h,
            0.0,
            360.0,
            format!("{:.0} degrees", hsv.h.round()),
            focus,
        )
    } else {
        apply_accessible_slider(
            slider,
            settings_subtitle_hue_label(target),
            true,
            hsv.h,
            0.0,
            360.0,
            format!("{:.0} degrees", hsv.h.round()),
        )
    };

    slider
        .on_a11y_action(gpui::AccessibleAction::Increment, move |_, _window, cx| {
            owner.update(cx, move |root, cx| {
                if let Some(next_hsv) = subtitle_hue_hsv_for_key(hsv, "right")
                    && root.commit_subtitle_hsv_color(target, next_hsv)
                {
                    cx.notify();
                }
            });
        })
        .on_a11y_action(gpui::AccessibleAction::Decrement, move |_, _window, cx| {
            decrement_owner.update(cx, move |root, cx| {
                if let Some(next_hsv) = subtitle_hue_hsv_for_key(hsv, "left")
                    && root.commit_subtitle_hsv_color(target, next_hsv)
                {
                    cx.notify();
                }
            });
        })
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, _window, cx| {
                let Some(next_hsv) = subtitle_hue_hsv_for_key(hsv, event.keystroke.key.as_str())
                else {
                    return;
                };
                if root.commit_subtitle_hsv_color(target, next_hsv) {
                    cx.notify();
                }
                cx.stop_propagation();
            }),
        )
}

const fn settings_subtitle_sv_label(target: SettingsSubtitleColorTarget) -> &'static str {
    match target {
        SettingsSubtitleColorTarget::Font => "Subtitle font color saturation and brightness",
        SettingsSubtitleColorTarget::Outline => "Subtitle outline color saturation and brightness",
    }
}

const fn settings_subtitle_hue_label(target: SettingsSubtitleColorTarget) -> &'static str {
    match target {
        SettingsSubtitleColorTarget::Font => "Subtitle font color hue",
        SettingsSubtitleColorTarget::Outline => "Subtitle outline color hue",
    }
}

fn settings_subtitle_sv_value(hsv: SettingsSubtitleHsv) -> String {
    format!(
        "Saturation {:.0}%, brightness {:.0}%",
        hsv.s * 100.0,
        hsv.v * 100.0
    )
}

fn subtitle_sv_hsv_for_key(hsv: SettingsSubtitleHsv, key: &str) -> Option<SettingsSubtitleHsv> {
    let mut next = hsv;
    match key {
        "left" => next.s -= 0.01,
        "right" => next.s += 0.01,
        "down" => next.v -= 0.01,
        "up" => next.v += 0.01,
        "pageup" => next.v += 0.10,
        "pagedown" => next.v -= 0.10,
        "home" => {
            next.s = 0.0;
            next.v = 0.0;
        }
        "end" => {
            next.s = 1.0;
            next.v = 1.0;
        }
        _ => return None,
    }
    next.s = next.s.clamp(0.0, 1.0);
    next.v = next.v.clamp(0.0, 1.0);
    Some(next)
}

fn subtitle_hue_hsv_for_key(hsv: SettingsSubtitleHsv, key: &str) -> Option<SettingsSubtitleHsv> {
    let mut next = hsv;
    let hue = match key {
        "left" | "down" => hsv.h - 1.0,
        "right" | "up" => hsv.h + 1.0,
        "pageup" => hsv.h - 15.0,
        "pagedown" => hsv.h + 15.0,
        "home" => 0.0,
        "end" => 360.0,
        _ => return None,
    };
    next.h = hue.clamp(0.0, 360.0);
    Some(next)
}

fn subtitle_select_target_index(
    len: usize,
    selected_index: Option<usize>,
    key: &str,
    is_enabled: impl Fn(usize) -> bool,
) -> Option<usize> {
    if len == 0 {
        return None;
    }
    match key {
        "home" => (0..len).find(|index| is_enabled(*index)),
        "end" => (0..len).rev().find(|index| is_enabled(*index)),
        "down" => {
            let start = selected_index.unwrap_or(len - 1);
            (1..=len)
                .map(|offset| (start + offset) % len)
                .find(|index| is_enabled(*index))
        }
        "up" => {
            let start = selected_index.unwrap_or(0);
            (1..=len)
                .map(|offset| (start + len - offset) % len)
                .find(|index| is_enabled(*index))
        }
        _ => None,
    }
}

impl FrameRoot {
    pub(in crate::app) fn toggle_subtitle_popover(&mut self, popover: SettingsSubtitlePopover) {
        if self.subtitle_ui.popover == Some(popover) {
            self.subtitle_ui.popover = None;
        } else {
            self.subtitle_ui.popover = Some(popover);
            self.subtitle_ui.rendered_popover = Some(popover);
        }
    }

    pub(in crate::app) const fn close_subtitle_popover(&mut self) {
        self.subtitle_ui.popover = None;
        if matches!(
            self.text_input_ui.active,
            Some(
                FrameTextInputKind::SubtitleFontColorHex
                    | FrameTextInputKind::SubtitleOutlineColorHex
            )
        ) {
            self.stop_text_input_cursor();
        }
    }

    pub(in crate::app) fn finish_subtitle_popover_close(
        &mut self,
        popover: SettingsSubtitlePopover,
    ) -> bool {
        if self.subtitle_ui.popover.is_some() || self.subtitle_ui.rendered_popover != Some(popover)
        {
            return false;
        }
        self.subtitle_ui.rendered_popover = None;
        true
    }

    pub(in crate::app) fn open_subtitle_color_popover(
        &mut self,
        popover: SettingsSubtitlePopover,
        target: SettingsSubtitleColorTarget,
        value: &str,
    ) {
        self.subtitle_ui.popover = Some(popover);
        self.subtitle_ui.rendered_popover = Some(popover);
        self.set_subtitle_color_hsv_draft(target, hex_to_subtitle_hsv(value));
        self.set_subtitle_color_draft(target, value.to_uppercase());
    }

    pub(in crate::app) fn toggle_subtitle_color_popover(
        &mut self,
        popover: SettingsSubtitlePopover,
        target: SettingsSubtitleColorTarget,
        value: &str,
    ) {
        if self.subtitle_ui.popover == Some(popover) {
            self.close_subtitle_popover();
        } else {
            self.open_subtitle_color_popover(popover, target, value);
        }
    }

    pub(in crate::app) fn set_subtitle_color_draft(
        &mut self,
        target: SettingsSubtitleColorTarget,
        value: String,
    ) -> bool {
        match target {
            SettingsSubtitleColorTarget::Font => {
                if self.subtitle_ui.font_color_draft == value {
                    return false;
                }
                self.subtitle_ui.font_color_draft = value;
            }
            SettingsSubtitleColorTarget::Outline => {
                if self.subtitle_ui.outline_color_draft == value {
                    return false;
                }
                self.subtitle_ui.outline_color_draft = value;
            }
        }
        true
    }

    pub(in crate::app) fn set_subtitle_color_hsv_draft(
        &mut self,
        target: SettingsSubtitleColorTarget,
        hsv: SettingsSubtitleHsv,
    ) -> bool {
        match target {
            SettingsSubtitleColorTarget::Font => {
                if self.subtitle_ui.font_color_hsv_draft == hsv {
                    return false;
                }
                self.subtitle_ui.font_color_hsv_draft = hsv;
            }
            SettingsSubtitleColorTarget::Outline => {
                if self.subtitle_ui.outline_color_hsv_draft == hsv {
                    return false;
                }
                self.subtitle_ui.outline_color_hsv_draft = hsv;
            }
        }
        true
    }

    const fn subtitle_color_hsv_draft(
        &self,
        target: SettingsSubtitleColorTarget,
    ) -> SettingsSubtitleHsv {
        match target {
            SettingsSubtitleColorTarget::Font => self.subtitle_ui.font_color_hsv_draft,
            SettingsSubtitleColorTarget::Outline => self.subtitle_ui.outline_color_hsv_draft,
        }
    }

    pub(in crate::app) fn commit_subtitle_color(
        &mut self,
        target: SettingsSubtitleColorTarget,
        value: &str,
    ) -> bool {
        let Some(normalized) = normalized_hex_color(value) else {
            return self.set_subtitle_color_draft(target, value.to_uppercase());
        };
        let draft_changed = self.set_subtitle_color_draft(target, normalized.to_uppercase());
        let hsv_changed =
            self.set_subtitle_color_hsv_draft(target, hex_to_subtitle_hsv(&normalized));
        let config_changed = self.update_selected_config(|config| match target {
            SettingsSubtitleColorTarget::Font => apply_subtitle_font_color(config, &normalized),
            SettingsSubtitleColorTarget::Outline => {
                apply_subtitle_outline_color(config, &normalized)
            }
        });
        draft_changed || hsv_changed || config_changed
    }

    pub(in crate::app) fn commit_subtitle_hsv_color(
        &mut self,
        target: SettingsSubtitleColorTarget,
        hsv: SettingsSubtitleHsv,
    ) -> bool {
        let hex = subtitle_hsv_to_hex(hsv.h, hsv.s, hsv.v);
        let draft_changed = self.set_subtitle_color_draft(target, hex.to_uppercase());
        let hsv_changed = self.set_subtitle_color_hsv_draft(target, hsv);
        let config_changed = self.update_selected_config(|config| match target {
            SettingsSubtitleColorTarget::Font => apply_subtitle_font_color(config, &hex),
            SettingsSubtitleColorTarget::Outline => apply_subtitle_outline_color(config, &hex),
        });
        draft_changed || hsv_changed || config_changed
    }

    pub(in crate::app) const fn set_subtitle_color_picker_bounds(
        &mut self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
        bounds: Bounds<Pixels>,
    ) {
        match (target, kind) {
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::SaturationValue) => {
                self.subtitle_ui.color_picker_bounds.font_sv = Some(bounds);
            }
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_ui.color_picker_bounds.font_hue = Some(bounds);
            }
            (
                SettingsSubtitleColorTarget::Outline,
                SettingsSubtitleColorDragKind::SaturationValue,
            ) => {
                self.subtitle_ui.color_picker_bounds.outline_sv = Some(bounds);
            }
            (SettingsSubtitleColorTarget::Outline, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_ui.color_picker_bounds.outline_hue = Some(bounds);
            }
        }
    }

    const fn subtitle_color_picker_bounds(
        &self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
    ) -> Option<Bounds<Pixels>> {
        match (target, kind) {
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::SaturationValue) => {
                self.subtitle_ui.color_picker_bounds.font_sv
            }
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_ui.color_picker_bounds.font_hue
            }
            (
                SettingsSubtitleColorTarget::Outline,
                SettingsSubtitleColorDragKind::SaturationValue,
            ) => self.subtitle_ui.color_picker_bounds.outline_sv,
            (SettingsSubtitleColorTarget::Outline, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_ui.color_picker_bounds.outline_hue
            }
        }
    }

    pub(in crate::app) fn commit_subtitle_color_at_position(
        &mut self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
        position: Point<Pixels>,
    ) -> bool {
        let base_hsv = self.subtitle_color_hsv_draft(target);
        self.commit_subtitle_color_with_base_hsv(target, kind, position, base_hsv)
    }

    pub(in crate::app) fn commit_subtitle_color_drag_at_position(
        &mut self,
        drag: SettingsSubtitleColorDrag,
        position: Point<Pixels>,
    ) -> bool {
        self.commit_subtitle_color_with_base_hsv(drag.target, drag.kind, position, drag.base_hsv)
    }

    fn commit_subtitle_color_with_base_hsv(
        &mut self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
        position: Point<Pixels>,
        base_hsv: SettingsSubtitleHsv,
    ) -> bool {
        let Some(bounds) = self.subtitle_color_picker_bounds(target, kind) else {
            return false;
        };
        let hsv = match kind {
            SettingsSubtitleColorDragKind::SaturationValue => {
                subtitle_hsv_from_sv_bounds(position, bounds, base_hsv)
            }
            SettingsSubtitleColorDragKind::Hue => {
                subtitle_hsv_from_hue_bounds(position, bounds, base_hsv)
            }
        };
        self.commit_subtitle_hsv_color(target, hsv)
    }
}

fn subtitle_hsv_from_sv_bounds(
    position: Point<Pixels>,
    bounds: Bounds<Pixels>,
    base_hsv: SettingsSubtitleHsv,
) -> SettingsSubtitleHsv {
    let mut hsv = base_hsv;
    let width = bounds.size.width.as_f32();
    let height = bounds.size.height.as_f32();
    if width > 0.0 {
        hsv.s = f64::from(((position.x - bounds.origin.x).as_f32() / width).clamp(0.0, 1.0));
    }
    if height > 0.0 {
        hsv.v = 1.0 - f64::from(((position.y - bounds.origin.y).as_f32() / height).clamp(0.0, 1.0));
    }
    hsv
}

fn subtitle_hsv_from_hue_bounds(
    position: Point<Pixels>,
    bounds: Bounds<Pixels>,
    base_hsv: SettingsSubtitleHsv,
) -> SettingsSubtitleHsv {
    let mut hsv = base_hsv;
    let width = bounds.size.width.as_f32();
    if width > 0.0 {
        hsv.h =
            f64::from(((position.x - bounds.origin.x).as_f32() / width).clamp(0.0, 1.0)) * 360.0;
    }
    hsv
}

pub(in crate::app) fn hex_to_subtitle_hsv(hex: &str) -> SettingsSubtitleHsv {
    let normalized =
        normalized_hex_color(hex).unwrap_or_else(|| DEFAULT_SUBTITLE_FONT_COLOR.to_string());
    let raw = normalized.trim_start_matches('#');
    let r = f64::from(u8::from_str_radix(&raw[0..2], 16).unwrap_or(255)) / 255.0;
    let g = f64::from(u8::from_str_radix(&raw[2..4], 16).unwrap_or(255)) / 255.0;
    let b = f64::from(u8::from_str_radix(&raw[4..6], 16).unwrap_or(255)) / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let mut h = 0.0;
    if delta > f64::EPSILON {
        if r >= g && r >= b {
            h = ((g - b) / delta) % 6.0;
        } else if g >= b {
            h = (b - r) / delta + 2.0;
        } else {
            h = (r - g) / delta + 4.0;
        }
        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
    }

    SettingsSubtitleHsv {
        h,
        s: if max == 0.0 { 0.0 } else { delta / max },
        v: max,
    }
}

pub(in crate::app) fn subtitle_hsv_to_hex(h: f64, s: f64, v: f64) -> String {
    frame_hsv_to_hex(h, s, v)
}

fn settings_subtitle_position_grid(
    config: &ConversionConfig,
    disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut grid = div().grid().grid_cols(3).gap_2();
    for option in subtitle_position_options(config, disabled) {
        let position = option.position;
        let is_enabled = !option.is_disabled;
        grid = grid.child(
            frame_choice_button(
                format!("subtitle-position-{}", position.id()),
                option.label,
                option.is_selected,
                is_enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !is_enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_subtitle_position(config, position)) {
                    cx.notify();
                }
            })),
        );
    }

    grid
}

pub(in crate::app) fn settings_subtitle_track_button(
    option: crate::settings::SubtitleTrackOption,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let index = option.index;
    let is_enabled = !option.is_disabled;
    let detail = option.detail;
    let detail = if detail.is_empty() {
        String::new()
    } else {
        format!("• {detail}")
    };

    frame_track_list_item(
        format!("subtitle-track-{index}"),
        FrameTrackListItemText {
            index_label: option.index_label,
            primary: option.codec,
            detail,
        },
        option.is_selected,
        is_enabled,
        FrameTrackListItemLayout::Inline,
        window,
        cx,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if !is_enabled {
            return;
        }
        if root.update_selected_config(|config| toggle_subtitle_track_selection(config, index)) {
            cx.notify();
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_font_keyboard_selection_skips_disabled_options() {
        let options = [
            SubtitleFontOption {
                name: "Alpha".to_string(),
                is_selected: true,
                is_disabled: false,
            },
            SubtitleFontOption {
                name: "Beta".to_string(),
                is_selected: false,
                is_disabled: true,
            },
            SubtitleFontOption {
                name: "Gamma".to_string(),
                is_selected: false,
                is_disabled: false,
            },
        ];

        let down_index = subtitle_select_target_index(options.len(), Some(0), "down", |index| {
            !options[index].is_disabled
        });
        let up_index = subtitle_select_target_index(options.len(), Some(0), "up", |index| {
            !options[index].is_disabled
        });

        assert_eq!(
            down_index.map(|index| options[index].name.as_str()),
            Some("Gamma")
        );
        assert_eq!(
            up_index.map(|index| options[index].name.as_str()),
            Some("Gamma")
        );
    }

    #[test]
    fn subtitle_font_size_keyboard_selection_supports_home_and_end() {
        let options = [
            SubtitleFontSizeOption {
                size: "12",
                is_selected: false,
                is_disabled: true,
            },
            SubtitleFontSizeOption {
                size: "14",
                is_selected: true,
                is_disabled: false,
            },
            SubtitleFontSizeOption {
                size: "16",
                is_selected: false,
                is_disabled: false,
            },
        ];

        let home_index = subtitle_select_target_index(options.len(), Some(1), "home", |index| {
            !options[index].is_disabled
        });
        let end_index = subtitle_select_target_index(options.len(), Some(1), "end", |index| {
            !options[index].is_disabled
        });

        assert_eq!(home_index.map(|index| options[index].size), Some("14"));
        assert_eq!(end_index.map(|index| options[index].size), Some("16"));
    }
}
