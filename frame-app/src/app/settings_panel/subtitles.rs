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

pub(in crate::app) struct SettingsSubtitlesTabState<'a> {
    pub(in crate::app) config: &'a ConversionConfig,
    pub(in crate::app) metadata: Option<&'a SourceMetadata>,
    pub(in crate::app) settings_disabled: bool,
    pub(in crate::app) subtitle_fonts: &'a [String],
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

struct SettingsSubtitleStyleState<'a> {
    config: &'a ConversionConfig,
    disabled: bool,
    subtitle_fonts: &'a [String],
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
    focus: Option<&'a FocusHandle>,
    active_popover: Option<SettingsSubtitlePopover>,
    rendered_popover: Option<SettingsSubtitlePopover>,
    draft: &'a str,
    hsv: SettingsSubtitleHsv,
}

pub(in crate::app) fn settings_subtitles_tab(
    state: SettingsSubtitlesTabState<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let config = state.config;
    let copy_mode = config.processing_mode == ProcessingMode::Copy;
    let burn_in_disabled = state.settings_disabled || copy_mode;
    let content = div().flex().flex_col().gap_4().child(
        settings_section("BURN-IN SUBTITLES")
            .child(settings_subtitle_burn_button(
                config,
                burn_in_disabled,
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
            settings_section("STYLE").child(settings_subtitle_style_controls(
                SettingsSubtitleStyleState {
                    config,
                    disabled: burn_in_disabled,
                    subtitle_fonts: state.subtitle_fonts,
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
            .child(settings_section("SOURCE TRACKS").child(settings_hint_text("No subtitles")));
    }

    let mut list = div().grid().grid_cols(1).gap_2();
    for option in track_options {
        list = list.child(settings_subtitle_track_button(option, window, cx));
    }

    content.child(settings_section("SOURCE TRACKS").child(list))
}

fn settings_subtitle_burn_button(
    config: &ConversionConfig,
    disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let colors = button_colors(ButtonVariant::Secondary, false, !disabled);
    let animated = animated_button_colors("settings-subtitle-burn-file", colors, window, cx);
    let background = animated.background;
    let foreground = animated.foreground;
    let hover_transition = animated.hover_transition;
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
        .bg(background)
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .opacity(colors.opacity)
        .shadow(button_highlight_shadows())
        .when(!disabled, |this| {
            this.hover(|style| style.cursor_pointer())
                .active(move |style| style.bg(color(colors.active_background)))
        })
        .when(disabled, |this| this.cursor_not_allowed())
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && !disabled, cx);
        })
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
        button.child(settings_subtitle_clear_button(disabled, window, cx))
    } else {
        button
    }
}

fn settings_subtitle_clear_button(
    disabled: bool,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let hover_transition = hover_motion("settings-subtitle-clear-file-hover", window, cx);
    let hover_progress = *hover_transition.evaluate(window, cx);
    let background = mix_color(theme::FRAME_GRAY_100, theme::FRAME_GRAY_200, hover_progress);

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
        .bg(background)
        .text_color(color(theme::FRAME_RED))
        .opacity(if disabled { 0.5 } else { 1.0 })
        .shadow(button_highlight_shadows())
        .when(!disabled, |this| {
            this.hover(|style| style.cursor_pointer())
                .active(|style| style.bg(color(theme::FRAME_GRAY_200)))
        })
        .when(disabled, |this| this.cursor_not_allowed())
        .on_hover(move |hover, _window, cx| {
            retarget_hover_motion(&hover_transition, *hover && !disabled, cx);
        })
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
                    state.config,
                    state.disabled,
                    state.subtitle_fonts,
                    state.active_popover,
                    state.rendered_popover,
                    SettingsSubtitlePopover::FontName,
                    state.font_select_scroll_handle,
                    window,
                    cx,
                ))
                .child(settings_subtitle_font_size_select(
                    state.config,
                    state.disabled,
                    state.active_popover,
                    state.rendered_popover,
                    SettingsSubtitlePopover::FontSize,
                    state.font_size_select_scroll_handle,
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
                        label: "TEXT COLOR",
                        id: "settings-subtitle-font-color",
                        value: subtitle_color_value(
                            state.config.subtitle_font_color.as_ref(),
                            DEFAULT_SUBTITLE_FONT_COLOR,
                        ),
                        disabled: state.disabled,
                        target: SettingsSubtitleColorTarget::Font,
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
                        label: "OUTLINE COLOR",
                        id: "settings-subtitle-outline-color",
                        value: subtitle_color_value(
                            state.config.subtitle_outline_color.as_ref(),
                            DEFAULT_SUBTITLE_OUTLINE_COLOR,
                        ),
                        disabled: state.disabled,
                        target: SettingsSubtitleColorTarget::Outline,
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
                .child(settings_field_label("POSITION"))
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

fn settings_subtitle_font_select(
    config: &ConversionConfig,
    disabled: bool,
    subtitle_fonts: &[String],
    active_popover: Option<SettingsSubtitlePopover>,
    rendered_popover: Option<SettingsSubtitlePopover>,
    popover: SettingsSubtitlePopover,
    scroll_handle: &ScrollHandle,
    window: &mut Window,
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
        .child(
            frame_select_trigger(
                "settings-subtitle-font-select",
                display,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                root.toggle_subtitle_popover(popover);
                cx.notify();
            })),
        );

    if rendered_popover == Some(popover) && has_options {
        let progress =
            subtitle_popover_progress(popover, active_popover == Some(popover), window, cx);
        let content_height = frame_select_content_height(options.len());
        let mut list =
            frame_select_options_list("settings-subtitle-font-options-list", scroll_handle);

        for option in options {
            let name = option.name.clone();
            let is_enabled = !option.is_disabled;
            list = list.child(settings_subtitle_font_option(option, is_enabled, name, cx));
        }

        let mut popover = frame_select_popover(
            "settings-subtitle-font-options",
            SUBTITLE_POPOVER_TOP_OFFSET + subtitle_popover_slide_offset(progress),
            progress,
            list,
        );

        if content_height > FRAME_SELECT_MAX_HEIGHT {
            popover = popover.child(frame_vertical_scrollbar(
                "settings-subtitle-font-options-scrollbar",
                scroll_handle.clone(),
                content_height,
            ));
        }

        field = field.child(deferred(popover).with_priority(10));
    }

    field
}

fn settings_subtitle_font_size_select(
    config: &ConversionConfig,
    disabled: bool,
    active_popover: Option<SettingsSubtitlePopover>,
    rendered_popover: Option<SettingsSubtitlePopover>,
    popover: SettingsSubtitlePopover,
    scroll_handle: &ScrollHandle,
    window: &mut Window,
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
        .child(
            frame_select_trigger(
                "settings-subtitle-font-size-select",
                display,
                enabled,
                window,
                cx,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                root.toggle_subtitle_popover(popover);
                cx.notify();
            })),
        );

    if rendered_popover == Some(popover) {
        let progress =
            subtitle_popover_progress(popover, active_popover == Some(popover), window, cx);
        let content_height = frame_select_content_height(options.len());
        let mut list =
            frame_select_options_list("settings-subtitle-font-size-options-list", scroll_handle);

        for option in options {
            let size = option.size;
            let is_enabled = !option.is_disabled;
            list = list.child(settings_subtitle_size_option(option, is_enabled, size, cx));
        }

        let mut popover = frame_select_popover(
            "settings-subtitle-font-size-options",
            SUBTITLE_POPOVER_TOP_OFFSET + subtitle_popover_slide_offset(progress),
            progress,
            list,
        );

        if content_height > FRAME_SELECT_MAX_HEIGHT {
            popover = popover.child(frame_vertical_scrollbar(
                "settings-subtitle-font-size-options-scrollbar",
                scroll_handle.clone(),
                content_height,
            ));
        }

        field = field.child(deferred(popover).with_priority(10));
    }

    field
}

fn settings_subtitle_font_option(
    option: SubtitleFontOption,
    is_enabled: bool,
    name: String,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    frame_select_option(
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
    frame_select_option(
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
            SUBTITLE_POPOVER_MOTION_DURATION,
            |_window, _cx| 0.0_f32,
        )
        .with_easing(ease_out_quint());
    let target = motion_target(is_open);
    if *transition.read_goal(cx) != target {
        transition.update(cx, |progress, cx| {
            *progress = target;
            cx.notify();
        });
    }
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

fn subtitle_popover_motion_key(popover: SettingsSubtitlePopover) -> &'static str {
    match popover {
        SettingsSubtitlePopover::FontName => "settings-subtitle-font-popover-motion",
        SettingsSubtitlePopover::FontSize => "settings-subtitle-size-popover-motion",
        SettingsSubtitlePopover::FontColor => "settings-subtitle-font-color-popover-motion",
        SettingsSubtitlePopover::OutlineColor => "settings-subtitle-outline-color-popover-motion",
    }
}

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

    let mut field = div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .child(settings_field_label(label))
        .child(
            frame_select_trigger_content(id, frame_color_select_value(&value), enabled, window, cx)
                .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    if !enabled {
                        return;
                    }
                    root.toggle_subtitle_color_popover(popover, target, &click_value);
                    cx.notify();
                })),
        );

    if rendered_popover == Some(popover) {
        let progress =
            subtitle_popover_progress(popover, active_popover == Some(popover), window, cx);
        field = field.child(
            deferred(settings_subtitle_color_picker(
                target, hsv, draft, focus, progress, window, cx,
            ))
            .with_priority(10),
        );
    }

    field
}

fn settings_subtitle_color_picker(
    target: SettingsSubtitleColorTarget,
    hsv: SettingsSubtitleHsv,
    draft: &str,
    focus: Option<&FocusHandle>,
    progress: f32,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let input_kind = match target {
        SettingsSubtitleColorTarget::Font => FrameTextInputKind::SubtitleFontColorHex,
        SettingsSubtitleColorTarget::Outline => FrameTextInputKind::SubtitleOutlineColorHex,
    };

    frame_color_picker_panel(
        SUBTITLE_POPOVER_TOP_OFFSET + subtitle_popover_slide_offset(progress),
        progress,
        settings_subtitle_sv_square(target, hsv, cx),
        settings_subtitle_hue_slider(target, hsv, cx),
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
    div()
        .id(match target {
            SettingsSubtitleColorTarget::Font => "settings-subtitle-font-color-sv",
            SettingsSubtitleColorTarget::Outline => "settings-subtitle-outline-color-sv",
        })
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
        })
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

    pub(in crate::app) fn close_subtitle_popover(&mut self) {
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

    fn subtitle_color_hsv_draft(&self, target: SettingsSubtitleColorTarget) -> SettingsSubtitleHsv {
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

    pub(in crate::app) fn set_subtitle_color_picker_bounds(
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

    fn subtitle_color_picker_bounds(
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
        option.index_label,
        option.codec,
        detail,
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
