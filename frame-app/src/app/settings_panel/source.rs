use super::{
    InteractiveElement, IntoElement, MetadataStatus, ParentElement, SourceInfoSection,
    SourceMetadata, StatefulInteractiveElement, Styled, color, div, horizontal_separator_shadows,
    px, settings_section, settings_value_row, source_info_sections, theme,
};

pub(in crate::app) fn settings_source_tab(
    metadata: Option<&SourceMetadata>,
    status: MetadataStatus,
    error: Option<&str>,
    palette: &'static theme::ThemePalette,
) -> gpui::AnyElement {
    match status {
        MetadataStatus::Loading => {
            return div()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .text_color(color(palette.text_muted))
                .child(theme::ui_text("Analyzing source..."))
                .into_any_element();
        }
        MetadataStatus::Error => {
            let mut error_view = div()
                .id("settings-source-metadata-error")
                .role(gpui::Role::Alert)
                .aria_label("Failed to read source metadata.")
                .flex()
                .flex_col()
                .gap_1()
                .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
                .text_color(color(palette.danger))
                .child(theme::ui_text("Failed to read source metadata."));
            if let Some(error) = error {
                error_view = error_view.child(
                    div()
                        .text_color(color(palette.text_muted))
                        .child(error.to_string()),
                );
            }
            return error_view.into_any_element();
        }
        MetadataStatus::Idle | MetadataStatus::Ready => {}
    }

    let Some(metadata) = metadata else {
        return div()
            .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
            .text_color(color(palette.text_muted))
            .child(theme::ui_text("Metadata unavailable."))
            .into_any_element();
    };

    let sections = source_info_sections(metadata);
    if sections.is_empty() {
        return div()
            .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
            .text_color(color(palette.text_muted))
            .child(theme::ui_text("Metadata unavailable."))
            .into_any_element();
    }

    let mut content = div().flex().flex_col().gap_6();
    for section in sections {
        content = match section {
            SourceInfoSection::Rows { title, rows } => content
                .child(settings_section(title, palette).child(settings_source_rows(rows, palette))),
            SourceInfoSection::Tracks { title, tracks } => content.child(
                settings_section(title, palette).child(settings_source_tracks(tracks, palette)),
            ),
        };
    }
    content.into_any_element()
}

pub(in crate::app) fn settings_source_rows(
    rows: Vec<crate::settings::SourceInfoRow>,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let mut grid = div().flex().flex_col().gap_2();
    for row in rows {
        grid = grid.child(settings_value_row(row.label, row.value, palette));
    }
    grid
}

pub(in crate::app) fn settings_source_tracks(
    tracks: Vec<crate::settings::SourceTrackSection>,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    let mut list = div().flex().flex_col().gap_4();
    for track in tracks {
        list = list.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_track_header(track.label, palette))
                .child(settings_source_rows(track.rows, palette)),
        );
    }
    list
}

pub(in crate::app) fn settings_track_header(
    label: String,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(palette.text_muted))
        .child(theme::ui_text_owned(label))
        .child(
            div()
                .h(px(1.0))
                .flex_1()
                .bg(color(palette.canvas))
                .shadow(horizontal_separator_shadows(palette)),
        )
}

pub(in crate::app) fn settings_section_label(
    label: &'static str,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(theme::ui_rem(theme::TEXT_UI_BASE_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(palette.text_muted))
        .child(theme::ui_text(label))
        .child(
            div()
                .h(px(1.0))
                .w_full()
                .bg(color(palette.canvas))
                .shadow(horizontal_separator_shadows(palette)),
        )
}
