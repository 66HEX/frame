use super::{
    ASPECT_OPTIONS, ClickEvent, Context, CropRect, CropSelect, CropSettings, FrameRoot,
    FrameTextInputKind, FrameTextInputSpec, InteractiveElement, MouseButton, ParentElement,
    PreviewCropOptionsProps, PreviewEditToolbarFocus, PreviewRotation, PreviewShellState,
    StatefulInteractiveElement, Styled, Window, color, crop_base_dimensions,
    crop_settings_from_rect, div, frame_text_input, full_crop_rect, theme, transform_crop_rect,
};
use crate::app::components::{
    FRAME_SELECT_MAX_HEIGHT, frame_select_content_height, frame_select_option_with_focus,
    frame_select_options_list, frame_select_popover, frame_select_target_index,
    frame_select_trigger_with_focus, frame_vertical_scrollbar,
};
use crate::app::motion::{motion_is_hidden, popover_slide_offset, surface_motion};

impl FrameRoot {
    pub(in crate::app) fn set_crop_custom_mode(&mut self, ratio: bool) {
        self.preview_ui.crop_options.is_ratio = ratio;
        self.preview_ui.crop_options.custom_dirty = false;
        self.preview_ui.crop_options.custom_edit_base = None;
        if ratio {
            let (width, height) =
                if let Some((width, height)) = self.preview_ui.crop_aspect.split_once(':') {
                    (width.to_string(), height.to_string())
                } else if let Some(crop) = self.current_draft_crop_settings() {
                    let mut divisor = crop.width;
                    let mut remainder = crop.height;
                    while remainder != 0 {
                        (divisor, remainder) = (remainder, divisor % remainder);
                    }
                    let divisor = divisor.max(1);
                    (
                        (crop.width / divisor).to_string(),
                        (crop.height / divisor).to_string(),
                    )
                } else {
                    ("1".to_string(), "1".to_string())
                };
            self.preview_ui.crop_options.width = width;
            self.preview_ui.crop_options.height = height;
        } else if let Some(crop) = self.current_draft_crop_settings() {
            self.preview_ui.crop_options.width = crop.width.to_string();
            self.preview_ui.crop_options.height = crop.height.to_string();
        }
    }

    fn current_draft_crop_settings(&self) -> Option<CropSettings> {
        let config = self.selected_config()?;
        crop_settings_from_rect(
            self.preview_ui.draft_crop?,
            &self.preview_ui.crop_aspect,
            &config.rotation,
            config.flip_horizontal,
            config.flip_vertical,
            self.selected_source_metadata().as_ref(),
        )
    }

    pub(in crate::app) fn custom_crop_values(&self) -> Option<(u32, u32)> {
        if !self.preview_ui.crop_mode
            || self.file_queue.selected_file_locked()
            || self.update_installation_in_progress()
        {
            return None;
        }
        let width = self.preview_ui.crop_options.width.parse::<u32>().ok()?;
        let height = self.preview_ui.crop_options.height.parse::<u32>().ok()?;
        if width == 0 || height == 0 || width > 65_535 || height > 65_535 {
            return None;
        }
        if !self.preview_ui.crop_options.is_ratio {
            let metadata = self.selected_source_metadata()?;
            let dimensions =
                crop_base_dimensions(Some(&metadata), &self.selected_config()?.rotation)?;
            if width > dimensions.width || height > dimensions.height {
                return None;
            }
        }
        Some((width, height))
    }

    fn custom_crop_selection(&self) -> Option<(CropRect, String)> {
        let (width, height) = self.custom_crop_values()?;
        let config = self.selected_config()?;
        let metadata = self.selected_source_metadata();
        let dimensions = crop_base_dimensions(metadata.as_ref(), &config.rotation)?;
        let rotation = PreviewRotation::from(config.rotation.as_str());
        let current = transform_crop_rect(
            self.preview_ui
                .crop_options
                .custom_edit_base
                .or(self.preview_ui.draft_crop)
                .unwrap_or_else(full_crop_rect),
            rotation,
            config.flip_horizontal,
            config.flip_vertical,
            false,
        );
        let (rect, aspect) = if self.preview_ui.crop_options.is_ratio {
            (
                crate::preview::adjust_rect_to_ratio(
                    current,
                    f64::from(width) / f64::from(height),
                    f64::from(dimensions.width),
                    f64::from(dimensions.height),
                    false,
                ),
                format!("{width}:{height}"),
            )
        } else {
            let w = f64::from(width) / f64::from(dimensions.width);
            let h = f64::from(height) / f64::from(dimensions.height);
            (
                CropRect {
                    x: (current.x + current.width / 2.0 - w / 2.0).clamp(0.0, 1.0 - w),
                    y: (current.y + current.height / 2.0 - h / 2.0).clamp(0.0, 1.0 - h),
                    width: w,
                    height: h,
                },
                "free".to_string(),
            )
        };
        Some((
            transform_crop_rect(
                rect,
                rotation,
                config.flip_horizontal,
                config.flip_vertical,
                true,
            ),
            aspect,
        ))
    }

    pub(in crate::app) fn apply_custom_crop_values(&mut self) -> bool {
        let Some((rect, aspect)) = self.custom_crop_selection() else {
            return false;
        };
        self.preview_ui.draft_crop = Some(rect);
        self.preview_ui.crop_aspect = aspect;
        self.preview_ui.crop_options.popover = None;
        self.preview_ui.crop_options.custom_dirty = false;
        self.preview_ui.crop_options.custom_edit_base = None;
        true
    }

    pub(in crate::app) fn preview_custom_crop_values(&mut self) {
        if !self.preview_ui.crop_options.custom {
            return;
        }
        // Both fields share the pre-edit rectangle so intermediate values cannot
        // progressively shrink the selection while typing a new ratio.
        if let Some((rect, aspect)) = self.custom_crop_selection() {
            self.preview_ui.draft_crop = Some(rect);
            self.preview_ui.crop_aspect = aspect;
        }
    }

    pub(in crate::app) fn finish_crop_custom_edit(&mut self) {
        if self.preview_ui.crop_options.custom && self.preview_ui.crop_options.custom_dirty {
            // Moving or resizing now starts from the visible selection.
            self.set_crop_custom_mode(self.preview_ui.crop_options.is_ratio);
        }
    }

    pub(in crate::app) fn crop_settings_preview(
        &self,
    ) -> Option<crate::settings::ConversionConfig> {
        if !self.preview_ui.crop_mode {
            return None;
        }
        let mut config = self.selected_config()?.clone();
        let metadata = self.selected_source_metadata()?;
        let rect = self.preview_ui.draft_crop?;
        let aspect = &self.preview_ui.crop_aspect;
        config.crop = crop_settings_from_rect(
            rect,
            aspect,
            &config.rotation,
            config.flip_horizontal,
            config.flip_vertical,
            Some(&metadata),
        );
        if super::crop_rect_is_full(rect) {
            config.crop = None;
        }
        config.resolution = "original".to_string();
        config.custom_width = None;
        config.custom_height = None;
        let (width, height) = frame_core::dimensions::output_dimensions(
            &crate::conversion_runner::core_config_from_gpui(&config),
            metadata.width?,
            metadata.height?,
            metadata.pixel_format.as_deref(),
        )?;
        config.resolution = "custom".to_string();
        config.custom_width = Some(width.to_string());
        config.custom_height = Some(height.to_string());
        Some(config)
    }
}

pub(in crate::app) fn preview_crop_options(
    state: &PreviewShellState,
    focuses: PreviewEditToolbarFocus<'_>,
    props: PreviewCropOptionsProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    crop_select(CropSelect::Aspect, state, focuses, props, window, cx)
}

pub(in crate::app) fn preview_crop_custom_fields(
    state: &PreviewShellState,
    focuses: PreviewEditToolbarFocus<'_>,
    props: PreviewCropOptionsProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut fields = div().flex().flex_wrap().items_center().gap_2();
    for (label, id, kind, value, focus) in [
        (
            "W",
            "crop-custom-width",
            FrameTextInputKind::CropWidth,
            props.width,
            props.width_focus,
        ),
        (
            "H",
            "crop-custom-height",
            FrameTextInputKind::CropHeight,
            props.height,
            props.height_focus,
        ),
    ] {
        fields = fields.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .text_color(color(state.palette.text_muted))
                        .child(label),
                )
                .child(
                    div().w(theme::ui_rem(68.0)).child(
                        frame_text_input(
                            FrameTextInputSpec {
                                id,
                                value,
                                placeholder: "",
                                disabled: false,
                                focus: Some(focus),
                                kind,
                            },
                            state.palette,
                            window,
                            cx,
                        )
                        .debug_selector(|| id.to_string())
                        .on_key_down(cx.listener(
                            |root, event: &gpui::KeyDownEvent, window, cx| {
                                if event.keystroke.key == "enter" {
                                    if root.apply_selected_crop() {
                                        root.focus_registered_control(
                                            "preview-tool-crop",
                                            window,
                                            cx,
                                        );
                                    }
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                            },
                        )),
                    ),
                ),
        );
    }
    fields.child(crop_select(
        CropSelect::Units,
        state,
        focuses,
        props,
        window,
        cx,
    ))
}

#[expect(
    clippy::too_many_lines,
    reason = "The select uses the shared trigger, list, options and scrollbar with crop-specific state and keyboard callbacks."
)]
fn crop_select(
    kind: CropSelect,
    state: &PreviewShellState,
    focuses: PreviewEditToolbarFocus<'_>,
    props: PreviewCropOptionsProps<'_>,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let (id, panel_id, list_id, label, width, focus, option_focuses, options, selected) = match kind
    {
        CropSelect::Aspect => {
            let options: Vec<_> = ASPECT_OPTIONS
                .iter()
                .map(|o| (o.id, o.display))
                .chain(std::iter::once(("custom", "Custom")))
                .collect();
            let selected = if props.custom {
                "custom"
            } else {
                state.crop.crop_aspect.as_str()
            };
            (
                "preview-crop-ratio",
                "crop-options-panel",
                "crop-ratio-options",
                "Crop aspect ratio",
                112.0,
                focuses.first,
                props.option_focuses,
                options,
                selected,
            )
        }
        CropSelect::Units => (
            "crop-units",
            "crop-units-panel",
            "crop-unit-options",
            "Crop units",
            96.0,
            props.unit_focus,
            props.unit_option_focuses,
            vec![("pixels", "Pixels"), ("ratio", "Ratio")],
            if props.ratio { "ratio" } else { "pixels" },
        ),
    };
    let selected_index = options.iter().position(|(value, _)| *value == selected);
    let display = selected_index.map_or(selected, |index| options[index].1);
    let open = props.popover == Some(kind);
    let progress = surface_motion(format!("{id}-popover-motion"), open, window, cx);
    let key_focuses = option_focuses.to_vec();
    let trigger_focus = focus.clone();
    let pointer_focus = focus.clone();
    let trigger = frame_select_trigger_with_focus(
        id,
        label,
        display,
        true,
        open,
        focus,
        state.palette,
        window,
        cx,
    )
    .debug_selector(|| id.to_string())
    .on_mouse_down(
        MouseButton::Left,
        cx.listener(move |root, _, window, cx| {
            pointer_focus.focus(window, cx);
            root.preview_ui.crop_options.popover = if open { None } else { Some(kind) };
            if !open {
                root.preview_ui
                    .crop_options
                    .scroll
                    .scroll_to_item(selected_index.unwrap_or(0));
            }
            cx.stop_propagation();
            cx.notify();
        }),
    )
    .on_click(cx.listener(move |root, event: &ClickEvent, _, cx| {
        if event.is_keyboard() {
            root.preview_ui.crop_options.popover = if open { None } else { Some(kind) };
            if !open {
                root.preview_ui
                    .crop_options
                    .scroll
                    .scroll_to_item(selected_index.unwrap_or(0));
            }
            cx.stop_propagation();
            cx.notify();
        }
    }))
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
            if let Some(index) = frame_select_target_index(
                key_focuses.len(),
                selected_index,
                &event.keystroke.key,
                |_| true,
            ) {
                root.preview_ui.crop_options.popover = Some(kind);
                root.preview_ui.crop_options.scroll.scroll_to_item(index);
                let focus = key_focuses[index].clone();
                cx.defer_in(window, move |_, window, cx| focus.focus(window, cx));
                cx.stop_propagation();
                cx.notify();
            } else if event.keystroke.key == "escape" && open {
                root.preview_ui.crop_options.popover = None;
                cx.stop_propagation();
                cx.notify();
            }
        }),
    );
    let mut field = div()
        .relative()
        .w(theme::ui_rem(width))
        .flex_shrink_0()
        .child(trigger);
    if !open && motion_is_hidden(progress) {
        return field;
    }
    let count = options.len();
    let mut list = frame_select_options_list(
        list_id,
        props.scroll,
        frame_select_content_height(count) > FRAME_SELECT_MAX_HEIGHT,
    );
    for (index, (value, label)) in options.into_iter().enumerate() {
        let key_focuses = option_focuses.to_vec();
        let key_scroll = props.scroll.clone();
        let focus = trigger_focus.clone();
        let option_id = format!(
            "{}-{value}",
            if kind == CropSelect::Aspect {
                "crop-ratio"
            } else {
                "crop-units"
            }
        );
        list = list.child(
            frame_select_option_with_focus(
                option_id.clone(),
                label,
                Some(index) == selected_index,
                open,
                &option_focuses[index],
                state.palette,
            )
            .debug_selector(|| option_id)
            .on_key_down(
                cx.listener(move |_, event: &gpui::KeyDownEvent, window, cx| {
                    if let Some(target) = frame_select_target_index(
                        key_focuses.len(),
                        Some(index),
                        &event.keystroke.key,
                        |_| true,
                    ) {
                        key_scroll.scroll_to_item(target);
                        key_focuses[target].focus(window, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }
                }),
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
                if !open || root.preview_ui.crop_options.popover != Some(kind) {
                    return;
                }
                match kind {
                    CropSelect::Aspect if value == "custom" => {
                        root.preview_ui.crop_options.custom = true;
                        root.set_crop_custom_mode(true);
                        root.preview_custom_crop_values();
                    }
                    CropSelect::Aspect => {
                        root.preview_ui.crop_options.custom = false;
                        root.select_preview_crop_aspect(value);
                    }
                    CropSelect::Units => {
                        root.set_crop_custom_mode(value == "ratio");
                        root.preview_custom_crop_values();
                    }
                }
                root.preview_ui.crop_options.popover = None;
                focus.focus(window, cx);
                cx.stop_propagation();
                cx.notify();
            })),
        );
    }
    let mut popover = frame_select_popover(
        panel_id,
        crate::SETTINGS_CONTROL_HEIGHT + 6.0 + popover_slide_offset(progress),
        progress,
        list,
        state.palette,
    )
    .debug_selector(|| panel_id.to_string())
    .on_mouse_down_out(cx.listener(move |root, _, _, cx| {
        if open {
            root.preview_ui.crop_options.popover = None;
            cx.notify();
        }
    }))
    .on_key_down(
        cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape" {
                root.preview_ui.crop_options.popover = None;
                trigger_focus.focus(window, cx);
                cx.stop_propagation();
                cx.notify();
            }
        }),
    );
    let content_height = frame_select_content_height(count);
    if content_height > FRAME_SELECT_MAX_HEIGHT {
        popover = popover.child(frame_vertical_scrollbar(
            "crop-options-scrollbar",
            props.scroll.clone(),
            content_height,
            state.palette,
        ));
    }
    field = field.child(gpui::deferred(popover).priority(20));
    field
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        file_queue::FileItem,
        preview::aspect_value,
        settings::{SourceKind, SourceMetadata},
    };
    use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, px, size};

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn click(cx: &mut VisualTestContext, id: &'static str) {
        let center = cx
            .debug_bounds(id)
            .unwrap_or_else(|| panic!("missing {id}"))
            .center();
        cx.simulate_click(center, Modifiers::default());
        draw(cx);
    }

    fn crop_window(app: &mut TestAppContext) -> (Entity<FrameRoot>, &mut VisualTestContext) {
        let (root, cx) = app.add_window_view(|_, cx| {
            crate::app::init_app(cx, "Frame crop test");
            let mut root = FrameRoot::new();
            root.apply_visual_fixture(Some(crate::VisualFixture::PreviewCrop));
            // Exercise the real view without starting FFmpeg for the fixture path.
            let metadata = root.selected_source_metadata_entry();
            root.preview_ui.pending_runtime_key = root
                .selected_preview_runtime_request(&metadata)
                .map(|request| request.key);
            root
        });
        cx.update(|window, _| window.activate());
        cx.simulate_resize(size(px(1440.0), px(900.0)));
        draw(cx);
        (root, cx)
    }

    fn video_root(rotation: &str) -> FrameRoot {
        let mut root = FrameRoot::new();
        root.file_queue
            .add_file(FileItem::from_path("video", "/tmp/crop.mp4", 1));
        root.file_queue.selected_file_mut().unwrap().config.rotation = rotation.into();
        root.source_metadata.mark_ready(
            "video",
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                ..SourceMetadata::default()
            },
        );
        assert!(root.toggle_selected_crop_mode());
        root
    }

    #[test]
    fn loaded_preview_keeps_timeline_and_queue_inside_window() {
        let mut app = TestAppContext::single();
        let (root, cx) = crop_window(&mut app);
        cx.simulate_resize(size(px(1200.0), px(800.0)));
        root.update(cx, |root, cx| {
            root.apply_visual_fixture(Some(crate::VisualFixture::PreviewCropCustom));
            let metadata = root.selected_source_metadata_entry();
            root.preview_ui.pending_runtime_key = root
                .selected_preview_runtime_request(&metadata)
                .map(|request| request.key);
            let frame = crate::preview_engine::PreviewFrame::bgra(
                1280,
                720,
                1280 * 4,
                0,
                vec![255; 1280 * 720 * 4],
            )
            .unwrap();
            root.preview_ui.render_image =
                Some(crate::preview_engine::render_image_from_frame(&frame).unwrap());
            cx.notify();
        });
        draw(cx);
        let viewport = cx.debug_bounds("preview-viewport").unwrap();
        let queue = cx.debug_bounds("file-list-body").unwrap();
        assert!(
            viewport.bottom() < queue.top(),
            "preview={viewport:?}, queue={queue:?}"
        );
        assert!(
            queue.bottom() <= px(800.0),
            "queue must remain visible: {queue:?}"
        );
        assert!(
            viewport.size.height < px(500.0),
            "preview must retain its workspace row share: {viewport:?}"
        );
    }

    #[test]
    fn crop_selector_keyboard_and_pointer_work_in_rendered_window() {
        let mut app = TestAppContext::single();
        let (root, cx) = crop_window(&mut app);
        click(cx, "preview-crop-ratio");
        assert!(root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
        click(cx, "crop-ratio-4:3");
        assert_eq!(
            root.read_with(cx, |root, _| root.preview_ui.crop_aspect.clone()),
            "4:3"
        );
        assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
        cx.simulate_keystrokes("down");
        draw(cx);
        assert!(root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
        cx.simulate_keystrokes("end");
        draw(cx);
        cx.simulate_keystrokes("up");
        draw(cx);
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        draw(cx);
        assert_eq!(
            root.read_with(cx, |root, _| root.preview_ui.crop_aspect.clone()),
            "1:2"
        );
        click(cx, "preview-crop-ratio");
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
    }

    #[test]
    fn crop_custom_fields_accept_typing_and_tab_navigation() {
        let mut app = TestAppContext::single();
        let (root, cx) = crop_window(&mut app);
        click(cx, "preview-crop-ratio");
        cx.simulate_keystrokes("end");
        draw(cx);
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        draw(cx);
        click(cx, "crop-units");
        click(cx, "crop-units-pixels");
        click(cx, "crop-custom-width");
        cx.simulate_keystrokes("cmd-a");
        cx.simulate_input("640");
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_eq!(
            cx.update(|window, cx| root.read(cx).focused_text_input_kind(window)),
            Some(FrameTextInputKind::CropHeight)
        );
        cx.simulate_keystrokes("cmd-a");
        cx.simulate_input("480");
        draw(cx);
        let live_crop = root.read_with(cx, |root, _| root.current_draft_crop_settings().unwrap());
        assert_eq!((live_crop.width, live_crop.height), (640, 480));
        assert!(root.read_with(cx, |root, _| root.selected_config().unwrap().crop.is_none()));
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
        let crop = root.read_with(cx, |root, _| {
            root.selected_config().unwrap().crop.clone().unwrap()
        });
        assert_eq!((crop.width, crop.height), (640, 480));
        assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_mode));
        root.update(cx, |root, cx| {
            assert!(root.toggle_selected_crop_mode());
            cx.notify();
        });
        draw(cx);
        click(cx, "preview-crop-ratio");
        // Clicking the trigger a second time closes it without reopening on mouse-up.
        assert!(
            !cx.debug_bounds("crop-options-panel")
                .unwrap()
                .contains(&cx.debug_bounds("preview-crop-ratio").unwrap().center())
        );
        click(cx, "preview-crop-ratio");
        assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
    }

    #[test]
    fn crop_picker_fits_small_windows_at_large_interface_scales() {
        for scale in [
            crate::appearance::ScalePreset::Percent100,
            crate::appearance::ScalePreset::Percent200,
        ] {
            let mut app = TestAppContext::single();
            let (root, cx) = crop_window(&mut app);
            root.update(cx, |root, cx| {
                root.appearance.ui_scale = scale;
                cx.notify();
            });
            cx.simulate_resize(size(px(1200.0), px(800.0)));
            draw(cx);
            click(cx, "preview-crop-ratio");
            let panel = cx.debug_bounds("crop-options-panel").unwrap();
            assert!(
                panel.left() >= px(0.0) && panel.top() >= px(0.0),
                "{scale:?}: {panel:?}"
            );
            assert!(
                panel.right() <= px(1200.0) && panel.bottom() <= px(800.0),
                "{scale:?}: {panel:?}"
            );
            let trigger = cx.debug_bounds("preview-crop-ratio").unwrap();
            assert!(
                !panel.contains(&trigger.center()),
                "{scale:?}: panel={panel:?}, trigger={trigger:?}"
            );
            cx.simulate_keystrokes("end");
            draw(cx);
            cx.simulate_keystrokes("up");
            draw(cx);
            cx.simulate_keystrokes("enter");
            cx.simulate_event(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse("enter").unwrap(),
            });
            draw(cx);
            assert_eq!(
                root.read_with(cx, |root, _| root.preview_ui.crop_aspect.clone()),
                "1:2"
            );
            click(cx, "preview-crop-ratio");
            cx.simulate_click(gpui::point(px(1100.0), px(700.0)), Modifiers::default());
            draw(cx);
            assert!(!root.read_with(cx, |root, _| root.preview_ui.crop_options.popover.is_some()));
        }
    }

    #[test]
    fn custom_ratio_updates_preview_while_typing_without_cumulative_shrinking() {
        let mut root = video_root("0");
        root.select_preview_crop_aspect("16:9");
        root.preview_ui.crop_options.custom = true;
        root.set_crop_custom_mode(true);
        root.write_text_input_value(FrameTextInputKind::CropWidth, "9");
        root.write_text_input_value(FrameTextInputKind::CropHeight, "1");
        root.write_text_input_value(FrameTextInputKind::CropHeight, "16");
        assert_eq!(root.preview_ui.crop_aspect, "9:16");
        let live_crop = root.current_draft_crop_settings().unwrap();
        assert_eq!((live_crop.width, live_crop.height), (608, 1080));
        assert!(root.selected_config().unwrap().crop.is_none());
        assert!(root.apply_selected_crop());
        assert_eq!(root.selected_config().unwrap().crop, Some(live_crop));
    }

    #[test]
    fn invalid_custom_input_keeps_last_valid_preview_and_resolution() {
        let mut root = video_root("0");
        root.preview_ui.crop_options.custom = true;
        root.set_crop_custom_mode(true);
        root.write_text_input_value(FrameTextInputKind::CropWidth, "7");
        root.write_text_input_value(FrameTextInputKind::CropHeight, "5");
        let valid_rect = root.preview_ui.draft_crop;
        let valid_settings = root.crop_settings_preview().unwrap();
        for invalid in ["", "0", "65536"] {
            root.write_text_input_value(FrameTextInputKind::CropHeight, invalid);
            assert_eq!(root.preview_ui.draft_crop, valid_rect);
            assert_eq!(root.crop_settings_preview(), Some(valid_settings.clone()));
            assert!(!root.apply_selected_crop());
        }
        root.write_text_input_value(FrameTextInputKind::CropHeight, "5");
        assert_eq!(root.preview_ui.draft_crop, valid_rect);
        assert!(root.toggle_selected_crop_mode());
        assert!(root.selected_config().unwrap().crop.is_none());
    }

    #[test]
    fn moving_a_live_custom_crop_is_preserved_on_apply() {
        let mut root = video_root("0");
        root.preview_ui.crop_options.custom = true;
        root.set_crop_custom_mode(false);
        root.write_text_input_value(FrameTextInputKind::CropWidth, "640");
        root.write_text_input_value(FrameTextInputKind::CropHeight, "480");
        let before = root.current_draft_crop_settings().unwrap();
        root.apply_preview_crop_drag(
            crate::preview::DragHandle::Move,
            super::super::PreviewPoint { x: 0.5, y: 0.5 },
        );
        assert!(root.apply_preview_crop_drag(
            crate::preview::DragHandle::Move,
            super::super::PreviewPoint { x: 0.6, y: 0.55 }
        ));
        root.end_preview_crop_drag();
        let moved = root.current_draft_crop_settings().unwrap();
        assert!(moved.x > before.x && moved.y > before.y);
        assert_eq!((moved.width, moved.height), (640, 480));
        assert!(root.apply_selected_crop());
        assert_eq!(root.selected_config().unwrap().crop, Some(moved));
    }

    #[test]
    fn ratio_mode_uses_the_visible_free_crop_proportions() {
        let mut root = video_root("0");
        root.preview_ui.crop_options.custom = true;
        root.set_crop_custom_mode(true);
        assert_eq!(root.custom_crop_values(), Some((16, 9)));
        root.preview_custom_crop_values();
        assert_eq!(root.preview_ui.crop_aspect, "16:9");
        assert_eq!(root.preview_ui.draft_crop, Some(full_crop_rect()));
    }

    #[test]
    fn crop_resolution_preview_commits_on_apply_and_preserves_settings_on_cancel() {
        let mut root = video_root("0");
        root.file_queue
            .selected_file_mut()
            .unwrap()
            .config
            .resolution = "720p".into();
        assert!(root.select_preview_crop_aspect("4:3"));
        let preview = root.crop_settings_preview().unwrap();
        assert_eq!(preview.resolution, "custom");
        assert_eq!(preview.custom_width.as_deref(), Some("1440"));
        assert_eq!(preview.custom_height.as_deref(), Some("1080"));
        assert_eq!(root.selected_config().unwrap().resolution, "720p");
        assert!(root.toggle_selected_crop_mode());
        assert!(root.crop_settings_preview().is_none());
        assert_eq!(root.selected_config().unwrap().resolution, "720p");
        assert!(root.toggle_selected_crop_mode());
        assert!(root.select_preview_crop_aspect("4:3"));
        assert!(root.apply_selected_crop());
        let config = root.selected_config().unwrap();
        assert_eq!(config.resolution, "custom");
        assert_eq!(config.custom_width.as_deref(), Some("1440"));
        assert_eq!(config.custom_height.as_deref(), Some("1080"));
        assert!(root.toggle_selected_crop_mode());
        let metadata = root.selected_source_metadata_entry();
        let request = root.selected_preview_runtime_request(&metadata).unwrap();
        assert_eq!(request.config.conversion_config.resolution, "original");
        assert!(request.config.conversion_config.crop.is_none());
    }

    #[test]
    fn custom_crop_resolution_uses_encoder_dimensions_before_and_after_apply() {
        let mut root = video_root("90");
        root.source_metadata.mark_ready(
            "video",
            SourceMetadata {
                media_kind: Some(SourceKind::Video),
                width: Some(1920),
                height: Some(1080),
                pixel_format: Some("yuv420p".into()),
                ..SourceMetadata::default()
            },
        );
        root.preview_ui.crop_options.custom = true;
        root.set_crop_custom_mode(false);
        root.write_text_input_value(FrameTextInputKind::CropWidth, "481");
        root.write_text_input_value(FrameTextInputKind::CropHeight, "359");
        let preview = root.crop_settings_preview().unwrap();
        assert_eq!(preview.custom_width.as_deref(), Some("480"));
        assert_eq!(preview.custom_height.as_deref(), Some("358"));
        assert!(root.apply_selected_crop());
        let config = root.selected_config().unwrap();
        assert_eq!(config.custom_width, preview.custom_width);
        assert_eq!(config.custom_height, preview.custom_height);
        assert_eq!(config.crop, preview.crop);
    }

    #[test]
    fn reset_discards_pending_custom_dimensions() {
        let mut root = video_root("0");
        root.preview_ui.crop_options.custom = true;
        root.set_crop_custom_mode(false);
        root.write_text_input_value(FrameTextInputKind::CropWidth, "640");
        root.write_text_input_value(FrameTextInputKind::CropHeight, "480");
        assert!(root.reset_preview_crop_selection());
        assert!(!root.preview_ui.crop_options.custom);
        assert!(root.apply_selected_crop());
        assert!(root.selected_config().unwrap().crop.is_none());
    }

    #[test]
    fn crop_four_three_matches_issue_example() {
        let mut root = video_root("0");
        assert!(root.select_preview_crop_aspect("4:3"));
        assert!(root.apply_selected_crop());
        let crop = root.selected_config().unwrap().crop.as_ref().unwrap();
        assert_eq!(
            (crop.x, crop.y, crop.width, crop.height),
            (240, 0, 1440, 1080)
        );
    }

    #[test]
    fn crop_exact_size_survives_all_rotations_and_flips() {
        for rotation in ["0", "90", "180", "270"] {
            for flip in [false, true] {
                let mut root = video_root(rotation);
                root.file_queue
                    .selected_file_mut()
                    .unwrap()
                    .config
                    .flip_horizontal = flip;
                root.preview_ui.crop_options.is_ratio = false;
                root.preview_ui.crop_options.width = "640".into();
                root.preview_ui.crop_options.height = "480".into();
                assert!(root.apply_custom_crop_values());
                assert!(root.apply_selected_crop());
                let crop = root.selected_config().unwrap().crop.as_ref().unwrap();
                assert_eq!(
                    (crop.width, crop.height),
                    (640, 480),
                    "{rotation}, flip={flip}"
                );
            }
        }
    }

    #[test]
    fn crop_rejects_invalid_custom_values_without_changing_selection() {
        let mut root = video_root("90");
        root.preview_ui.crop_options.is_ratio = false;
        let original = root.preview_ui.draft_crop;
        for (width, height) in [
            ("", "480"),
            ("0", "480"),
            ("1081", "480"),
            ("640", "1921"),
            ("999999999999", "480"),
        ] {
            root.preview_ui.crop_options.width = width.into();
            root.preview_ui.crop_options.height = height.into();
            assert!(!root.apply_custom_crop_values());
            assert_eq!(root.preview_ui.draft_crop, original);
        }
    }

    #[test]
    fn crop_custom_ratio_is_preserved_on_apply_and_reopen() {
        let mut root = video_root("0");
        root.preview_ui.crop_options.width = "7".into();
        root.preview_ui.crop_options.height = "5".into();
        assert!(root.apply_custom_crop_values());
        assert!(root.apply_selected_crop());
        assert!(root.toggle_selected_crop_mode());
        assert_eq!(root.preview_ui.crop_aspect, "7:5");
        assert!(aspect_value(&root.preview_ui.crop_aspect).is_some());
    }
}
