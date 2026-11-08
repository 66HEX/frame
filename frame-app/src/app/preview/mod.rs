use crate::appearance::ColorTheme;
use crate::theme;
pub(super) use crop::*;
pub(super) use crop_options::*;
#[cfg(test)]
pub(super) use crop_overlay::*;
pub(super) use overlay::*;
pub(super) use panel::*;
pub(super) use timeline::*;
#[cfg(test)]
pub(super) use toolbar::*;
pub(super) use viewport::*;

pub(in crate::app) mod auto_crop;
mod crop;
mod crop_options;
mod crop_overlay;
mod overlay;
mod panel;
mod timeline;
mod toolbar;
mod viewport;

fn preview_handle_color(palette: &theme::ThemePalette) -> theme::RgbaToken {
    if palette.is_light() {
        theme::palette(ColorTheme::Dark).text_primary
    } else {
        palette.text_primary
    }
}

const fn preview_selection_line_color() -> gpui::Hsla {
    gpui::white()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_preview_handles_use_the_dark_theme_color() {
        assert_eq!(
            preview_handle_color(theme::palette(ColorTheme::Light)),
            theme::palette(ColorTheme::Dark).text_primary
        );
    }

    #[test]
    fn dark_preview_handles_keep_their_existing_color() {
        let palette = theme::palette(ColorTheme::Dark);

        assert_eq!(preview_handle_color(palette), palette.text_primary);
    }

    #[test]
    fn preview_selection_lines_are_white_in_every_theme() {
        assert_eq!(preview_selection_line_color(), gpui::white());
    }
}

pub(in crate::app) mod actions;
pub(in crate::app) mod state;
