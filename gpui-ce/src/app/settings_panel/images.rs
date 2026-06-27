use super::*;

pub(in crate::app) fn settings_images_tab(
    config: &ConversionConfig,
    settings_disabled: bool,
    video_width_focus: Option<&FocusHandle>,
    video_height_focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(settings_video_resolution_section(
            config,
            settings_disabled,
            video_width_focus,
            video_height_focus,
            window,
            cx,
        ))
        .child(settings_video_scaling_section(
            config,
            settings_disabled,
            cx,
        ))
        .child(settings_images_pixel_format_section(
            config,
            settings_disabled,
            cx,
        ))
}

fn settings_images_pixel_format_section(
    config: &ConversionConfig,
    settings_disabled: bool,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let mut list = div().grid().grid_cols(1);
    for option in video_pixel_format_options(config) {
        let pixel_format = option.id;
        let enabled = !settings_disabled && !option.is_disabled;
        list = list.child(
            frame_list_item_with_caption(
                format!("images-pixel-format-{pixel_format}"),
                option.label,
                option.caption,
                option.is_selected,
                enabled,
            )
            .on_click(cx.listener(move |root, _: &ClickEvent, _window, cx| {
                cx.stop_propagation();
                if !enabled {
                    return;
                }
                if root.update_selected_config(|config| apply_pixel_format(config, pixel_format)) {
                    cx.notify();
                }
            })),
        );
    }

    settings_section("PIXEL FORMAT").child(list)
}
