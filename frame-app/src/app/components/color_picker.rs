use super::*;

pub(in crate::app) const FRAME_COLOR_PICKER_SV_HEIGHT: f32 = 96.0;
pub(in crate::app) const FRAME_COLOR_PICKER_HUE_HEIGHT: f32 = 10.0;
pub(in crate::app) const FRAME_COLOR_PICKER_HANDLE_SIZE: f32 = 12.0;
pub(in crate::app) const FRAME_COLOR_PICKER_HUE_HANDLE_WIDTH: f32 = 6.0;
pub(in crate::app) const FRAME_COLOR_PICKER_HUE_VISUAL_HEIGHT: f32 = 18.0;

pub(in crate::app) fn frame_color_picker_panel(
    top: f32,
    progress: f32,
    sv_square: impl IntoElement,
    hue_slider: impl IntoElement,
    input: impl IntoElement,
) -> gpui::Div {
    div()
        .absolute()
        .top(px(top))
        .left_0()
        .right_0()
        .flex()
        .flex_col()
        .gap_2()
        .rounded(px(theme::RADIUS_SM))
        .bg(color(theme::DROPDOWN))
        .opacity(progress)
        .p_2()
        .shadow(button_highlight_shadows())
        .occlude()
        .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
            cx.stop_propagation();
        })
        .child(sv_square)
        .child(hue_slider)
        .child(input)
}

pub(in crate::app) fn frame_color_picker_sv_canvas(
    hue: f64,
    saturation: f64,
    value: f64,
) -> gpui::Div {
    div()
        .absolute()
        .left(px(0.0))
        .right(px(0.0))
        .top(px(0.0))
        .bottom(px(0.0))
        .child(
            div()
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .rounded(px(theme::RADIUS_XS))
                .bg(frame_color_picker_hue_color(hue)),
        )
        .child(
            div()
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .top(px(0.0))
                .bottom(px(0.0))
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
                .left(px(0.0))
                .right(px(0.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .rounded(px(theme::RADIUS_XS))
                .bg(linear_gradient(
                    0.0,
                    linear_color_stop(hsla(0.0, 0.0, 0.0, 1.0), 0.0),
                    linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 1.0),
                )),
        )
        .child(frame_color_picker_sv_handle(saturation, value))
}

pub(in crate::app) fn frame_color_picker_hue_track() -> gpui::Div {
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
        .h(px(FRAME_COLOR_PICKER_HUE_HEIGHT))
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

pub(in crate::app) fn frame_color_picker_hue_handle(hue: f64) -> gpui::Div {
    div()
        .absolute()
        .left(relative((hue / 360.0).clamp(0.0, 1.0) as f32))
        .top(px(1.0))
        .ml(px(-(FRAME_COLOR_PICKER_HUE_HANDLE_WIDTH / 2.0)))
        .h(px(16.0))
        .w(px(FRAME_COLOR_PICKER_HUE_HANDLE_WIDTH))
        .rounded(px(1.5))
        .bg(color(theme::BACKGROUND))
        .shadow(button_highlight_shadows())
}

pub(in crate::app) fn frame_color_picker_hue_color(hue: f64) -> Rgba {
    parse_hex(&frame_hsv_to_hex(hue, 1.0, 1.0))
}

pub(in crate::app) fn frame_hsv_to_hex(h: f64, s: f64, v: f64) -> String {
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

    frame_rgb_to_hex(
        (r_prime + m) * 255.0,
        (g_prime + m) * 255.0,
        (b_prime + m) * 255.0,
    )
}

fn frame_color_picker_sv_handle(saturation: f64, value: f64) -> gpui::Div {
    div()
        .absolute()
        .left(relative(saturation.clamp(0.0, 1.0) as f32))
        .top(relative((1.0 - value).clamp(0.0, 1.0) as f32))
        .ml(px(-(FRAME_COLOR_PICKER_HANDLE_SIZE / 2.0)))
        .mt(px(-(FRAME_COLOR_PICKER_HANDLE_SIZE / 2.0)))
        .w(px(FRAME_COLOR_PICKER_HANDLE_SIZE))
        .h(px(FRAME_COLOR_PICKER_HANDLE_SIZE))
        .rounded_full()
        .border_1()
        .border_color(color(theme::FOREGROUND))
        .shadow(vec![BoxShadow {
            color: hsla(0.0, 0.0, 0.0, 0.35),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        }])
}

fn frame_rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    let to_byte = |channel: f64| channel.round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", to_byte(r), to_byte(g), to_byte(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_hsv_to_hex_round_trips_primary_hues() {
        assert_eq!(frame_hsv_to_hex(0.0, 1.0, 1.0), "#ff0000");
        assert_eq!(frame_hsv_to_hex(120.0, 1.0, 1.0), "#00ff00");
        assert_eq!(frame_hsv_to_hex(240.0, 1.0, 1.0), "#0000ff");
    }

    #[test]
    fn frame_color_picker_visual_sizes_match_settings_control() {
        assert_eq!(FRAME_COLOR_PICKER_SV_HEIGHT, 96.0);
        assert_eq!(FRAME_COLOR_PICKER_HUE_HEIGHT, 10.0);
    }
}
