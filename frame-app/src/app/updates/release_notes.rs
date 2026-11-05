use crate::app::components::frame_vertical_scrollbar;
use crate::app::primitives::color;
use crate::theme;
use frame_updater::UpdateInfo;
use gpui::prelude::{ParentElement, Styled};
use gpui::{
    HighlightStyle, InteractiveElement, ScrollHandle, StatefulInteractiveElement, StyledText, div,
};

pub(in crate::app) const MAX_RELEASE_NOTES_CHARS: usize = 8_000;

pub(in crate::app) const UPDATE_RELEASE_NOTES_MAX_HEIGHT: f32 = 360.0;

pub(in crate::app) const UPDATE_RELEASE_NOTES_MIN_HEIGHT: f32 = 180.0;

pub(in crate::app) const UPDATE_RELEASE_NOTES_PADDING_Y: f32 = 24.0;

pub(in crate::app) const UPDATE_RELEASE_NOTES_LINE_HEIGHT: f32 = 16.0;

pub(in crate::app) const UPDATE_RELEASE_NOTES_LINE_PADDING_BOTTOM: f32 = 4.0;

pub(in crate::app) const UPDATE_RELEASE_NOTES_BLANK_LINE_HEIGHT: f32 = 8.0;

pub(in crate::app) fn update_release_notes_text(info: Option<&UpdateInfo>) -> Option<String> {
    let notes = info?.release_notes_markdown.as_deref()?;
    let notes = notes.trim();
    if notes.is_empty() {
        return None;
    }

    let mut text = notes
        .chars()
        .take(MAX_RELEASE_NOTES_CHARS + 1)
        .collect::<String>();
    if text.chars().count() > MAX_RELEASE_NOTES_CHARS {
        text = text.chars().take(MAX_RELEASE_NOTES_CHARS).collect();
        text.push_str("...");
    }
    Some(text)
}

pub(in crate::app) fn update_release_notes_block(
    notes: &str,
    scroll_handle: &ScrollHandle,
    palette: &'static theme::ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    let lines = normalized_release_note_lines(notes);
    let content_height = update_release_notes_content_height(&lines);
    let mut content = div()
        .id("update-dialog-release-notes-content")
        .min_h(theme::ui_rem(UPDATE_RELEASE_NOTES_MIN_HEIGHT))
        .max_h(theme::ui_rem(UPDATE_RELEASE_NOTES_MAX_HEIGHT))
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .p_3()
        .pr(theme::ui_rem(crate::app::components::FRAME_SCROLLBAR_WIDTH));

    for line in lines {
        content = content.child(update_release_note_line(&line, palette));
    }

    div()
        .id("update-dialog-release-notes")
        .relative()
        .min_h(theme::ui_rem(UPDATE_RELEASE_NOTES_MIN_HEIGHT))
        .max_h(theme::ui_rem(UPDATE_RELEASE_NOTES_MAX_HEIGHT))
        .overflow_hidden()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.fill_subtle))
        .child(content)
        .child(frame_vertical_scrollbar(
            "update-dialog-release-notes-scrollbar",
            scroll_handle.clone(),
            content_height,
            palette,
        ))
}

pub(in crate::app) fn update_release_notes_content_height(lines: &[String]) -> f32 {
    UPDATE_RELEASE_NOTES_PADDING_Y
        + lines.iter().fold(0.0, |height, line| {
            if line.trim().is_empty() {
                height + UPDATE_RELEASE_NOTES_BLANK_LINE_HEIGHT
            } else {
                height + UPDATE_RELEASE_NOTES_LINE_HEIGHT + UPDATE_RELEASE_NOTES_LINE_PADDING_BOTTOM
            }
        })
}

pub(in crate::app) fn normalized_release_note_lines(notes: &str) -> Vec<String> {
    let mut lines = notes
        .lines()
        .map(str::trim_end)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        vec!["No release notes were published for this version.".to_string()]
    } else {
        lines
    }
}

pub(in crate::app) fn update_release_note_line(
    line: &str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return div().h(theme::ui_rem(8.0));
    }

    let heading = trimmed.trim_start_matches('#').trim();
    let bullet = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("• "));
    let (text, left_padding, text_color, font_weight) =
        if !heading.is_empty() && trimmed.starts_with('#') {
            (
                heading.to_string(),
                0.0,
                palette.text_primary,
                theme::TEXT_WEIGHT_MEDIUM,
            )
        } else if let Some(bullet) = bullet {
            (
                format!("• {bullet}"),
                8.0,
                palette.text_muted,
                theme::TEXT_WEIGHT_REGULAR,
            )
        } else {
            (
                trimmed.to_string(),
                0.0,
                palette.text_muted,
                theme::TEXT_WEIGHT_REGULAR,
            )
        };
    let (text, highlights) = parse_update_release_note_emphasis(&text, palette);

    let mut line = div()
        .pl(theme::ui_rem(left_padding))
        .pb(theme::ui_rem(4.0))
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .line_height(theme::ui_rem(16.0))
        .text_color(color(text_color))
        .font_weight(font_weight);

    if highlights.is_empty() {
        line = line.child(text);
    } else {
        line = line.child(StyledText::new(text).with_highlights(highlights));
    }

    line
}

pub(in crate::app) fn parse_update_release_note_emphasis(
    input: &str,
    palette: &'static theme::ThemePalette,
) -> (String, Vec<(std::ops::Range<usize>, HighlightStyle)>) {
    let mut text = String::with_capacity(input.len());
    let mut highlights = Vec::new();
    let mut rest = input;
    let highlight_style = HighlightStyle {
        color: Some(gpui::rgb_to_hsla(color(palette.text_primary))),
        font_weight: Some(theme::TEXT_WEIGHT_MEDIUM),
        ..HighlightStyle::default()
    };

    loop {
        let Some(start) = rest.find("**") else {
            text.push_str(rest);
            break;
        };
        text.push_str(&rest[..start]);

        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("**") else {
            text.push_str(&rest[start..]);
            break;
        };

        let highlight_start = text.len();
        text.push_str(&after_start[..end]);
        let highlight_end = text.len();
        if highlight_start < highlight_end {
            highlights.push((highlight_start..highlight_end, highlight_style));
        }
        rest = &after_start[end + 2..];
    }

    (text, highlights)
}
