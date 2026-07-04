use super::{
    InteractiveElement, IntoElement, MetadataStatus, ParentElement, SourceInfoSection,
    SourceMetadata, StatefulInteractiveElement, Styled, color, div, horizontal_separator_shadows,
    px, settings_section, settings_value_row, source_info_sections, theme,
};

pub(in crate::app) fn settings_source_tab(
    metadata: Option<&SourceMetadata>,
    status: MetadataStatus,
    error: Option<&str>,
) -> gpui::AnyElement {
    match status {
        MetadataStatus::Loading => {
            return div()
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_GRAY_600))
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
                .text_size(px(theme::TEXT_LABEL_SIZE))
                .text_color(color(theme::FRAME_RED))
                .child(theme::ui_text("Failed to read source metadata."));
            if let Some(error) = error {
                error_view = error_view.child(
                    div()
                        .text_color(color(theme::FRAME_GRAY_600))
                        .child(error.to_string()),
                );
            }
            return error_view.into_any_element();
        }
        MetadataStatus::Idle | MetadataStatus::Ready => {}
    }

    let Some(metadata) = metadata else {
        return div()
            .text_size(px(theme::TEXT_LABEL_SIZE))
            .text_color(color(theme::FRAME_GRAY_600))
            .child(theme::ui_text("Metadata unavailable."))
            .into_any_element();
    };

    let sections = source_info_sections(metadata);
    if sections.is_empty() {
        return div()
            .text_size(px(theme::TEXT_LABEL_SIZE))
            .text_color(color(theme::FRAME_GRAY_600))
            .child(theme::ui_text("Metadata unavailable."))
            .into_any_element();
    }

    let mut content = div().flex().flex_col().gap_6();
    for section in sections {
        content = match section {
            SourceInfoSection::Rows { title, rows } => {
                content.child(settings_section(title).child(settings_source_rows(rows)))
            }
            SourceInfoSection::Tracks { title, tracks } => {
                content.child(settings_section(title).child(settings_source_tracks(tracks)))
            }
        };
    }
    content.into_any_element()
}

pub(in crate::app) fn settings_source_rows(rows: Vec<crate::settings::SourceInfoRow>) -> gpui::Div {
    let mut grid = div().flex().flex_col().gap_2();
    for row in rows {
        grid = grid.child(settings_value_row(row.label, row.value));
    }
    grid
}

pub(in crate::app) fn settings_source_tracks(
    tracks: Vec<crate::settings::SourceTrackSection>,
) -> gpui::Div {
    let mut list = div().flex().flex_col().gap_4();
    for track in tracks {
        list = list.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_track_header(track.label))
                .child(settings_source_rows(track.rows)),
        );
    }
    list
}

pub(in crate::app) fn settings_track_header(label: String) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(theme::FRAME_GRAY_600))
        .child(theme::ui_text_owned(label))
        .child(
            div()
                .h(px(1.0))
                .flex_1()
                .bg(color(theme::BACKGROUND))
                .shadow(horizontal_separator_shadows()),
        )
}

pub(in crate::app) fn settings_section_label(label: &'static str) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(px(theme::TEXT_LABEL_SIZE))
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(color(theme::FRAME_GRAY_600))
        .child(theme::ui_text(label))
        .child(
            div()
                .h(px(1.0))
                .w_full()
                .bg(color(theme::BACKGROUND))
                .shadow(horizontal_separator_shadows()),
        )
}
