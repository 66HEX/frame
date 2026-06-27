use super::*;

const SUBTITLE_DROPDOWN_TOP_OFFSET: f32 = SETTINGS_CONTROL_HEIGHT + 4.0;
const SUBTITLE_SELECT_MAX_HEIGHT: f32 = 192.0;
const SUBTITLE_COLOR_PANEL_WIDTH: f32 = 220.0;
const SUBTITLE_COLOR_SV_HEIGHT: f32 = 96.0;
const SUBTITLE_COLOR_HUE_HEIGHT: f32 = 10.0;
const SUBTITLE_COLOR_HANDLE_SIZE: f32 = 12.0;
const SUBTITLE_HUE_HANDLE_WIDTH: f32 = 6.0;

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
        let mut style = Style::default();
        style.position = Position::Absolute;
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();

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

pub(in crate::app) struct SettingsSubtitlesTabState<'a> {
    pub(in crate::app) config: &'a ConversionConfig,
    pub(in crate::app) metadata: Option<&'a SourceMetadata>,
    pub(in crate::app) settings_disabled: bool,
    pub(in crate::app) subtitle_fonts: &'a [String],
    pub(in crate::app) color_focuses: SettingsSubtitleColorInputFocuses<'a>,
    pub(in crate::app) active_popover: Option<SettingsSubtitlePopover>,
    pub(in crate::app) font_color_draft: &'a str,
    pub(in crate::app) outline_color_draft: &'a str,
}

struct SettingsSubtitleStyleState<'a> {
    config: &'a ConversionConfig,
    disabled: bool,
    subtitle_fonts: &'a [String],
    color_focuses: SettingsSubtitleColorInputFocuses<'a>,
    active_popover: Option<SettingsSubtitlePopover>,
    font_color_draft: &'a str,
    outline_color_draft: &'a str,
}

struct SettingsSubtitleColorFieldSpec<'a> {
    label: &'static str,
    id: &'static str,
    value: String,
    disabled: bool,
    target: SettingsSubtitleColorTarget,
    focus: Option<&'a FocusHandle>,
    is_open: bool,
    draft: &'a str,
}

pub(in crate::app) fn settings_subtitles_tab(
    state: SettingsSubtitlesTabState<'_>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let config = state.config;
    let copy_mode = config.processing_mode == ProcessingMode::Copy;
    let burn_in_disabled = state.settings_disabled || copy_mode;
    let content = div().flex().flex_col().gap_4().child(
        settings_section("BURN-IN SUBTITLES")
            .child(settings_subtitle_burn_button(config, burn_in_disabled, cx))
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
            settings_section("STYLE").child(settings_subtitle_style_controls(
                SettingsSubtitleStyleState {
                    config,
                    disabled: burn_in_disabled,
                    subtitle_fonts: state.subtitle_fonts,
                    color_focuses: state.color_focuses,
                    active_popover: state.active_popover,
                    font_color_draft: state.font_color_draft,
                    outline_color_draft: state.outline_color_draft,
                },
                window,
                cx,
            )),
        )
    };

    let track_options = subtitle_track_options(config, state.metadata, state.settings_disabled);
    if track_options.is_empty() {
        return content
            .child(settings_section("SOURCE TRACKS").child(settings_hint_text("No subtitles")));
    }

    let mut list = div().grid().grid_cols(1).gap_2();
    for option in track_options {
        list = list.child(settings_subtitle_track_button(option, cx));
    }

    content.child(settings_section("SOURCE TRACKS").child(list))
}

fn settings_subtitle_burn_button(
    config: &ConversionConfig,
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Secondary, false, !disabled);
    let label = subtitle_burn_file_label(config);
    let has_path = config.subtitle_burn_path.is_some();

    let button = div()
        .id("settings-subtitle-burn-file")
        .relative()
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .when(has_path, |this| this.pr(px(32.0)))
        .bg(color(colors.background))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(theme::FOREGROUND))
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(!disabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(disabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(!disabled, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            root.prompt_subtitle_burn_file(cx);
        }))
        .child(div().truncate().child(label));

    if has_path {
        button.child(settings_subtitle_clear_button(disabled, cx))
    } else {
        button
    }
}

fn settings_subtitle_clear_button(
    disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id("settings-subtitle-clear-file")
        .absolute()
        .right(px(12.0))
        .w(px(20.0))
        .h(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::FRAME_GRAY_100))
        .text_color(color(theme::FRAME_RED))
        .opacity(if disabled { 0.5 } else { 1.0 })
        .shadow(button_highlight_shadows())
        .when(!disabled, |this| {
            this.hover(|style| {
                style
                    .bg(color(theme::FRAME_GRAY_200))
                    .text_color(color(theme::FRAME_RED))
                    .cursor_pointer()
            })
            .active(|style| style.bg(color(theme::FRAME_GRAY_200)))
        })
        .when(disabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(!disabled, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            if root.update_selected_config(|config| apply_subtitle_burn_path(config, None)) {
                cx.notify();
            }
        }))
        .child(icon_svg(assets::ICON_CLOSE, 12.0, color(theme::FRAME_RED)))
}

fn settings_subtitle_style_controls(
    state: SettingsSubtitleStyleState<'_>,
    window: &Window,
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
                    state.config,
                    state.disabled,
                    state.subtitle_fonts,
                    state.active_popover == Some(SettingsSubtitlePopover::FontName),
                    cx,
                ))
                .child(settings_subtitle_font_size_select(
                    state.config,
                    state.disabled,
                    state.active_popover == Some(SettingsSubtitlePopover::FontSize),
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
                        label: "TEXT COLOR",
                        id: "settings-subtitle-font-color",
                        value: subtitle_color_value(
                            state.config.subtitle_font_color.as_ref(),
                            DEFAULT_SUBTITLE_FONT_COLOR,
                        ),
                        disabled: state.disabled,
                        target: SettingsSubtitleColorTarget::Font,
                        focus: state.color_focuses.font,
                        is_open: state.active_popover == Some(SettingsSubtitlePopover::FontColor),
                        draft: state.font_color_draft,
                    },
                    window,
                    cx,
                ))
                .child(settings_subtitle_color_field(
                    SettingsSubtitleColorFieldSpec {
                        label: "OUTLINE COLOR",
                        id: "settings-subtitle-outline-color",
                        value: subtitle_color_value(
                            state.config.subtitle_outline_color.as_ref(),
                            DEFAULT_SUBTITLE_OUTLINE_COLOR,
                        ),
                        disabled: state.disabled,
                        target: SettingsSubtitleColorTarget::Outline,
                        focus: state.color_focuses.outline,
                        is_open: state.active_popover
                            == Some(SettingsSubtitlePopover::OutlineColor),
                        draft: state.outline_color_draft,
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
                .child(settings_field_label("POSITION"))
                .child(settings_subtitle_position_grid(
                    state.config,
                    state.disabled,
                    cx,
                )),
        )
        .child(settings_hint_text(
            "Style applies to burned-in subtitles only.",
        ))
}

fn settings_subtitle_font_select(
    config: &ConversionConfig,
    disabled: bool,
    subtitle_fonts: &[String],
    is_open: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let display = config
        .subtitle_font_name
        .as_deref()
        .filter(|font| !font.is_empty())
        .unwrap_or("Default (e.g. Arial)");
    let options = subtitle_font_options(config, subtitle_fonts, disabled);
    let has_options = !options.is_empty();
    let enabled = !disabled && has_options;

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label("FONT"))
        .child(settings_subtitle_select_trigger(
            "settings-subtitle-font-select",
            display,
            enabled,
            SettingsSubtitlePopover::FontName,
            cx,
        ));

    if is_open && has_options {
        let mut list = div()
            .absolute()
            .id("settings-subtitle-font-options")
            .top(px(SUBTITLE_DROPDOWN_TOP_OFFSET + 20.0))
            .left_0()
            .right_0()
            .max_h(px(SUBTITLE_SELECT_MAX_HEIGHT))
            .overflow_y_scroll()
            .rounded(px(theme::RADIUS_SM))
            .bg(color(theme::DROPDOWN))
            .shadow(button_highlight_shadows())
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                cx.stop_propagation();
            });
        for option in options {
            let name = option.name.clone();
            let is_enabled = !option.is_disabled;
            list = list.child(settings_subtitle_font_option(option, is_enabled, name, cx));
        }
        field = field.child(deferred(list).with_priority(10));
    }

    field
}

fn settings_subtitle_font_size_select(
    config: &ConversionConfig,
    disabled: bool,
    is_open: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let display = config
        .subtitle_font_size
        .as_deref()
        .filter(|size| !size.is_empty())
        .unwrap_or("Default");
    let options = subtitle_font_size_options(config, disabled);
    let enabled = !disabled;

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label("SIZE"))
        .child(settings_subtitle_select_trigger(
            "settings-subtitle-font-size-select",
            display,
            enabled,
            SettingsSubtitlePopover::FontSize,
            cx,
        ));

    if is_open {
        let mut list = div()
            .absolute()
            .id("settings-subtitle-font-size-options")
            .top(px(SUBTITLE_DROPDOWN_TOP_OFFSET + 20.0))
            .left_0()
            .right_0()
            .max_h(px(SUBTITLE_SELECT_MAX_HEIGHT))
            .overflow_y_scroll()
            .rounded(px(theme::RADIUS_SM))
            .bg(color(theme::DROPDOWN))
            .shadow(button_highlight_shadows())
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                cx.stop_propagation();
            });

        for option in options {
            let size = option.size;
            let is_enabled = !option.is_disabled;
            list = list.child(settings_subtitle_size_option(option, is_enabled, size, cx));
        }

        field = field.child(deferred(list).with_priority(10));
    }

    field
}

fn settings_subtitle_select_trigger(
    id: &'static str,
    display: &str,
    enabled: bool,
    popover: SettingsSubtitlePopover,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Secondary, false, enabled);

    div()
        .id(id)
        .h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .min_w_0()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .bg(color(colors.background))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!enabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            button_mouse_down(enabled, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            root.toggle_subtitle_popover(popover);
            cx.notify();
        }))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(color(theme::FOREGROUND))
                .child(display.to_string()),
        )
        .child(icon_svg(
            assets::ICON_CHEVRONS_UP_DOWN,
            12.0,
            color(theme::FOREGROUND),
        ))
}

fn settings_subtitle_font_option(
    option: SubtitleFontOption,
    is_enabled: bool,
    name: String,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    settings_subtitle_select_option(
        format!("subtitle-font-{name}"),
        option.name,
        option.is_selected,
        is_enabled,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if !is_enabled {
            return;
        }
        let changed = root.update_selected_config(|config| apply_subtitle_font_name(config, &name));
        root.close_subtitle_popover();
        if changed {
            cx.notify();
        }
    }))
}

fn settings_subtitle_size_option(
    option: SubtitleFontSizeOption,
    is_enabled: bool,
    size: &'static str,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    settings_subtitle_select_option(
        format!("subtitle-size-{size}"),
        option.size,
        option.is_selected,
        is_enabled,
    )
    .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        if !is_enabled {
            return;
        }
        let changed = root.update_selected_config(|config| apply_subtitle_font_size(config, size));
        root.close_subtitle_popover();
        if changed {
            cx.notify();
        }
    }))
}

fn settings_subtitle_select_option(
    id: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    let text_color = if selected {
        theme::FOREGROUND
    } else {
        theme::FRAME_GRAY_600
    };

    div()
        .id(id.into())
        .h(px(28.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px(px(12.0))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(text_color))
        .opacity(if enabled { 1.0 } else { 0.5 })
        .when(enabled, |this| {
            this.hover(|style| {
                style
                    .bg(color(theme::FRAME_GRAY_100))
                    .text_color(color(theme::FOREGROUND))
                    .cursor_pointer()
            })
        })
        .when(!enabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            button_mouse_down(enabled, window, cx);
        })
        .child(div().min_w_0().truncate().child(label))
        .when(selected, |this| {
            this.child(icon_svg(assets::ICON_CHECK, 12.0, color(theme::FOREGROUND)))
        })
}

fn settings_subtitle_color_field(
    spec: SettingsSubtitleColorFieldSpec<'_>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let SettingsSubtitleColorFieldSpec {
        label,
        id,
        value,
        disabled,
        target,
        focus,
        is_open,
        draft,
    } = spec;
    let popover = match target {
        SettingsSubtitleColorTarget::Font => SettingsSubtitlePopover::FontColor,
        SettingsSubtitleColorTarget::Outline => SettingsSubtitlePopover::OutlineColor,
    };
    let enabled = !disabled;
    let colors = button_colors(ButtonVariant::Secondary, false, enabled);
    let click_value = value.clone();

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label(label))
        .child(
            div()
                .id(id)
                .h(px(SETTINGS_CONTROL_HEIGHT))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded(px(theme::RADIUS_SM))
                .px(px(10.0))
                .bg(color(colors.background))
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .opacity(colors.opacity)
                .shadow(button_highlight_shadows())
                .when(enabled, |this| {
                    this.hover(move |style| {
                        style
                            .bg(color(colors.hover_background))
                            .text_color(color(colors.hover_foreground))
                            .cursor_pointer()
                    })
                    .active(move |style| style.bg(color(colors.active_background)))
                })
                .when(!enabled, |this| this.cursor_not_allowed())
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    button_mouse_down(enabled, window, cx);
                })
                .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if !enabled {
                        return;
                    }
                    root.open_subtitle_color_popover(popover, target, &click_value);
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(14.0))
                                .h(px(14.0))
                                .flex_shrink_0()
                                .rounded(px(theme::RADIUS_XS))
                                .bg(parse_hex(&value))
                                .shadow(input_highlight_shadows()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .w_full()
                                .truncate()
                                .text_color(color(theme::FOREGROUND))
                                .child(value.to_uppercase()),
                        ),
                )
                .child(div().flex_shrink_0().child(icon_svg(
                    assets::ICON_CHEVRONS_UP_DOWN,
                    12.0,
                    color(theme::FOREGROUND),
                ))),
        );

    if is_open {
        field = field.child(
            deferred(settings_subtitle_color_picker(
                target, &value, draft, focus, window, cx,
            ))
            .with_priority(10),
        );
    }

    field
}

fn settings_subtitle_color_picker(
    target: SettingsSubtitleColorTarget,
    value: &str,
    draft: &str,
    focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let hsv = hex_to_subtitle_hsv(value);
    let align_right = target == SettingsSubtitleColorTarget::Outline;
    let input_kind = match target {
        SettingsSubtitleColorTarget::Font => FrameTextInputKind::SubtitleFontColorHex,
        SettingsSubtitleColorTarget::Outline => FrameTextInputKind::SubtitleOutlineColorHex,
    };

    div()
        .absolute()
        .top(px(SUBTITLE_DROPDOWN_TOP_OFFSET + 20.0))
        .when(align_right, |this| this.right_0())
        .when(!align_right, |this| this.left_0())
        .w(px(SUBTITLE_COLOR_PANEL_WIDTH))
        .flex()
        .flex_col()
        .gap_2()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::DROPDOWN))
        .p_2()
        .shadow(button_highlight_shadows())
        .occlude()
        .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
            cx.stop_propagation();
        })
        .child(settings_subtitle_sv_square(target, hsv, cx))
        .child(settings_subtitle_hue_slider(target, hsv, cx))
        .child(frame_text_input(
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
        ))
}

fn settings_subtitle_sv_square(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let drag = SettingsSubtitleColorDrag {
        target,
        kind: SettingsSubtitleColorDragKind::SaturationValue,
        base_hsv: hsv,
    };
    let hue = subtitle_hue_color(hsv.h);

    div()
        .id(match target {
            SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-sv",
            SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-sv",
        })
        .relative()
        .h(px(SUBTITLE_COLOR_SV_HEIGHT))
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
        .child(
            div()
                .absolute()
                .left(px(1.0))
                .right(px(1.0))
                .top(px(1.0))
                .bottom(px(1.0))
                .rounded(px(theme::RADIUS_XS))
                .bg(hue),
        )
        .child(
            div()
                .absolute()
                .left(px(1.0))
                .right(px(1.0))
                .top(px(1.0))
                .bottom(px(1.0))
                .rounded(px(theme::RADIUS_XS))
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(hsla(0.0, 0.0, 1.0, 1.0), 0.0),
                    linear_color_stop(hsla(0.0, 0.0, 1.0, 0.0), 1.0),
                )),
        )
        .child(
            div()
                .absolute()
                .left(px(1.0))
                .right(px(1.0))
                .top(px(1.0))
                .bottom(px(1.0))
                .rounded(px(theme::RADIUS_XS))
                .bg(linear_gradient(
                    0.0,
                    linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 0.0),
                    linear_color_stop(hsla(0.0, 0.0, 0.0, 1.0), 1.0),
                )),
        )
        .child(
            div()
                .absolute()
                .left(relative(hsv.s as f32))
                .top(relative((1.0 - hsv.v) as f32))
                .ml(px(-(SUBTITLE_COLOR_HANDLE_SIZE / 2.0)))
                .mt(px(-(SUBTITLE_COLOR_HANDLE_SIZE / 2.0)))
                .w(px(SUBTITLE_COLOR_HANDLE_SIZE))
                .h(px(SUBTITLE_COLOR_HANDLE_SIZE))
                .rounded_full()
                .border_1()
                .border_color(color(theme::FOREGROUND))
                .shadow(vec![BoxShadow {
                    color: hsla(0.0, 0.0, 0.0, 0.35),
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(0.0),
                    spread_radius: px(1.0),
                    inset: false,
                }]),
        )
        .on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsSubtitleColorDragPreview)
        })
}

fn settings_subtitle_hue_slider(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let drag = SettingsSubtitleColorDrag {
        target,
        kind: SettingsSubtitleColorDragKind::Hue,
        base_hsv: hsv,
    };

    div()
        .id(match target {
            SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-hue",
            SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-hue",
        })
        .relative()
        .h(px(18.0))
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
        .child(settings_subtitle_hue_segments())
        .child(
            div()
                .absolute()
                .left(relative((hsv.h / 360.0) as f32))
                .top(px(1.0))
                .ml(px(-(SUBTITLE_HUE_HANDLE_WIDTH / 2.0)))
                .h(px(16.0))
                .w(px(SUBTITLE_HUE_HANDLE_WIDTH))
                .rounded(px(1.5))
                .bg(color(theme::BACKGROUND))
                .shadow(button_highlight_shadows()),
        )
        .on_drag(drag, |_drag, _position, _window, cx| {
            cx.new(|_| SettingsSubtitleColorDragPreview)
        })
}

fn settings_subtitle_hue_segments() -> gpui::Div {
    let stops = [
        ("#ff0000", "#ffff00"),
        ("#ffff00", "#00ff00"),
        ("#00ff00", "#00ffff"),
        ("#00ffff", "#0000ff"),
        ("#0000ff", "#ff00ff"),
        ("#ff00ff", "#ff0000"),
    ];

    let mut row = div()
        .absolute()
        .left_0()
        .right_0()
        .top(px(4.0))
        .h(px(SUBTITLE_COLOR_HUE_HEIGHT))
        .flex()
        .overflow_hidden()
        .rounded(px(theme::RADIUS_XS))
        .shadow(input_highlight_shadows());

    for (from, to) in stops {
        row = row.child(div().flex_1().h_full().bg(linear_gradient(
            90.0,
            linear_color_stop(parse_hex(from), 0.0),
            linear_color_stop(parse_hex(to), 1.0),
        )));
    }

    row
}

impl FrameRoot {
    pub(in crate::app) fn toggle_subtitle_popover(&mut self, popover: SettingsSubtitlePopover) {
        self.settings_subtitle_popover = if self.settings_subtitle_popover == Some(popover) {
            None
        } else {
            Some(popover)
        };
    }

    pub(in crate::app) fn close_subtitle_popover(&mut self) {
        self.settings_subtitle_popover = None;
        if matches!(
            self.active_text_input,
            Some(
                FrameTextInputKind::SubtitleFontColorHex
                    | FrameTextInputKind::SubtitleOutlineColorHex
            )
        ) {
            self.stop_text_input_cursor();
        }
    }

    pub(in crate::app) fn open_subtitle_color_popover(
        &mut self,
        popover: SettingsSubtitlePopover,
        target: SettingsSubtitleColorTarget,
        value: &str,
    ) {
        self.settings_subtitle_popover = Some(popover);
        self.set_subtitle_color_draft(target, value.to_uppercase());
    }

    pub(in crate::app) fn set_subtitle_color_draft(
        &mut self,
        target: SettingsSubtitleColorTarget,
        value: String,
    ) {
        match target {
            SettingsSubtitleColorTarget::Font => self.subtitle_font_color_draft = value,
            SettingsSubtitleColorTarget::Outline => self.subtitle_outline_color_draft = value,
        }
    }

    pub(in crate::app) fn commit_subtitle_color(
        &mut self,
        target: SettingsSubtitleColorTarget,
        value: &str,
    ) -> bool {
        let normalized = normalized_hex_color(value).unwrap_or_else(|| value.to_string());
        self.set_subtitle_color_draft(target, normalized.to_uppercase());
        self.update_selected_config(|config| match target {
            SettingsSubtitleColorTarget::Font => apply_subtitle_font_color(config, &normalized),
            SettingsSubtitleColorTarget::Outline => {
                apply_subtitle_outline_color(config, &normalized)
            }
        })
    }

    pub(in crate::app) fn commit_subtitle_hsv_color(
        &mut self,
        target: SettingsSubtitleColorTarget,
        hsv: SettingsSubtitleHsv,
    ) -> bool {
        let hex = subtitle_hsv_to_hex(hsv.h, hsv.s, hsv.v);
        self.commit_subtitle_color(target, &hex)
    }

    fn current_subtitle_color(&self, target: SettingsSubtitleColorTarget) -> String {
        self.selected_config()
            .map(|config| match target {
                SettingsSubtitleColorTarget::Font => subtitle_color_value(
                    config.subtitle_font_color.as_ref(),
                    DEFAULT_SUBTITLE_FONT_COLOR,
                ),
                SettingsSubtitleColorTarget::Outline => subtitle_color_value(
                    config.subtitle_outline_color.as_ref(),
                    DEFAULT_SUBTITLE_OUTLINE_COLOR,
                ),
            })
            .unwrap_or_else(|| match target {
                SettingsSubtitleColorTarget::Font => DEFAULT_SUBTITLE_FONT_COLOR.to_string(),
                SettingsSubtitleColorTarget::Outline => DEFAULT_SUBTITLE_OUTLINE_COLOR.to_string(),
            })
    }

    pub(in crate::app) fn set_subtitle_color_picker_bounds(
        &mut self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
        bounds: Bounds<Pixels>,
    ) {
        match (target, kind) {
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::SaturationValue) => {
                self.subtitle_color_picker_bounds.font_sv = Some(bounds);
            }
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_color_picker_bounds.font_hue = Some(bounds);
            }
            (
                SettingsSubtitleColorTarget::Outline,
                SettingsSubtitleColorDragKind::SaturationValue,
            ) => {
                self.subtitle_color_picker_bounds.outline_sv = Some(bounds);
            }
            (SettingsSubtitleColorTarget::Outline, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_color_picker_bounds.outline_hue = Some(bounds);
            }
        }
    }

    fn subtitle_color_picker_bounds(
        &self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
    ) -> Option<Bounds<Pixels>> {
        match (target, kind) {
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::SaturationValue) => {
                self.subtitle_color_picker_bounds.font_sv
            }
            (SettingsSubtitleColorTarget::Font, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_color_picker_bounds.font_hue
            }
            (
                SettingsSubtitleColorTarget::Outline,
                SettingsSubtitleColorDragKind::SaturationValue,
            ) => self.subtitle_color_picker_bounds.outline_sv,
            (SettingsSubtitleColorTarget::Outline, SettingsSubtitleColorDragKind::Hue) => {
                self.subtitle_color_picker_bounds.outline_hue
            }
        }
    }

    pub(in crate::app) fn commit_subtitle_color_at_position(
        &mut self,
        target: SettingsSubtitleColorTarget,
        kind: SettingsSubtitleColorDragKind,
        position: Point<Pixels>,
    ) -> bool {
        let base_hsv = hex_to_subtitle_hsv(&self.current_subtitle_color(target));
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

fn subtitle_hue_color(hue: f64) -> Rgba {
    parse_hex(&subtitle_hsv_to_hex(hue, 1.0, 1.0))
}

pub(in crate::app) fn hex_to_subtitle_hsv(hex: &str) -> SettingsSubtitleHsv {
    let normalized =
        normalized_hex_color(hex).unwrap_or_else(|| DEFAULT_SUBTITLE_FONT_COLOR.to_string());
    let raw = normalized.trim_start_matches('#');
    let r = u8::from_str_radix(&raw[0..2], 16).unwrap_or(255) as f64 / 255.0;
    let g = u8::from_str_radix(&raw[2..4], 16).unwrap_or(255) as f64 / 255.0;
    let b = u8::from_str_radix(&raw[4..6], 16).unwrap_or(255) as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let mut h = 0.0;
    if delta != 0.0 {
        if max == r {
            h = ((g - b) / delta) % 6.0;
        } else if max == g {
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
    let hue = ((h % 360.0) + 360.0) % 360.0;
    let sat = s.clamp(0.0, 1.0);
    let val = v.clamp(0.0, 1.0);
    let chroma = val * sat;
    let x = chroma * (1.0 - (((hue / 60.0) % 2.0) - 1.0).abs());
    let m = val - chroma;

    let (r_prime, g_prime, b_prime) = if hue < 60.0 {
        (chroma, x, 0.0)
    } else if hue < 120.0 {
        (x, chroma, 0.0)
    } else if hue < 180.0 {
        (0.0, chroma, x)
    } else if hue < 240.0 {
        (0.0, x, chroma)
    } else if hue < 300.0 {
        (x, 0.0, chroma)
    } else {
        (chroma, 0.0, x)
    };

    subtitle_rgb_to_hex(
        (r_prime + m) * 255.0,
        (g_prime + m) * 255.0,
        (b_prime + m) * 255.0,
    )
}

fn subtitle_rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    let to_byte = |channel: f64| channel.round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", to_byte(r), to_byte(g), to_byte(b))
}

fn settings_subtitle_position_grid(
    config: &ConversionConfig,
    disabled: bool,
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
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(
        ButtonVariant::Secondary,
        option.is_selected,
        !option.is_disabled,
    );
    let index = option.index;
    let is_enabled = !option.is_disabled;
    let is_selected = option.is_selected;
    let detail = option.detail;

    div()
        .id(format!("subtitle-track-{index}"))
        .min_h(px(SETTINGS_CONTROL_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded(px(theme::RADIUS_SM))
        .px(px(10.0))
        .py(px(6.0))
        .bg(color(colors.background))
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .text_color(color(colors.foreground))
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(is_enabled, |this| {
            this.hover(move |style| {
                style
                    .bg(color(colors.hover_background))
                    .text_color(color(colors.hover_foreground))
                    .cursor_pointer()
            })
            .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(!is_enabled, |this| this.cursor_not_allowed())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            button_mouse_down(is_enabled, window, cx);
        })
        .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
            cx.stop_propagation();
            if !is_enabled {
                return;
            }
            if root.update_selected_config(|config| toggle_subtitle_track_selection(config, index))
            {
                cx.notify();
            }
        }))
        .child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(option.index_label),
                )
                .child(
                    div()
                        .text_color(color(theme::FOREGROUND))
                        .child(option.codec),
                )
                .when(!detail.is_empty(), |this| {
                    this.child(
                        div()
                            .truncate()
                            .text_color(color(theme::FRAME_GRAY_600))
                            .child(format!("• {detail}")),
                    )
                }),
        )
        .child(frame_selection_dot(is_selected))
}
