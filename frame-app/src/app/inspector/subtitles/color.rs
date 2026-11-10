use crate::app::components::frame_hsv_to_hex;
use crate::app::inspector::subtitles::state::{SettingsSubtitleColorTarget, SettingsSubtitleHsv};
use crate::settings::{DEFAULT_SUBTITLE_FONT_COLOR, normalized_hex_color};
use gpui::{Bounds, Pixels, Point};

pub(super) const fn settings_subtitle_sv_label(
    target: SettingsSubtitleColorTarget,
) -> &'static str {
    match target {
        SettingsSubtitleColorTarget::Font => "Subtitle font color saturation and brightness",
        SettingsSubtitleColorTarget::Outline => "Subtitle outline color saturation and brightness",
    }
}

pub(super) const fn settings_subtitle_hue_label(
    target: SettingsSubtitleColorTarget,
) -> &'static str {
    match target {
        SettingsSubtitleColorTarget::Font => "Subtitle font color hue",
        SettingsSubtitleColorTarget::Outline => "Subtitle outline color hue",
    }
}

pub(super) fn settings_subtitle_sv_value(hsv: SettingsSubtitleHsv) -> String {
    format!(
        "Saturation {:.0}%, brightness {:.0}%",
        hsv.s * 100.0,
        hsv.v * 100.0
    )
}

pub(super) fn subtitle_sv_hsv_for_key(
    hsv: SettingsSubtitleHsv,
    key: &str,
) -> Option<SettingsSubtitleHsv> {
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

pub(super) fn subtitle_hue_hsv_for_key(
    hsv: SettingsSubtitleHsv,
    key: &str,
) -> Option<SettingsSubtitleHsv> {
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

pub(super) fn subtitle_hsv_from_sv_bounds(
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

pub(super) fn subtitle_hsv_from_hue_bounds(
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
