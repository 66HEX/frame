use super::*;
use super::{
    accessibility::{
        apply_accessible_button, apply_accessible_button_with_focus, apply_accessible_slider,
        apply_accessible_toggle_button, focus_visible_ring, handle_modal_tab_navigation,
    },
    components::{
        FrameIconButtonSize, FrameIconButtonVariant, frame_icon_button, frame_slider,
        frame_slider_handle,
    },
    input::{FrameTextInputSpec, frame_text_input},
    primitives::{
        ButtonVariant, FrameSurface, animated_button_colors, apply_button_motion, button_colors,
        button_highlight_shadows, card_surface_shadows, color, icon_svg, input_highlight_shadows,
    },
};

mod crop;
mod crop_overlay;
mod overlay;
mod panel;
mod timeline;
mod toolbar;
mod viewport;

pub(super) use crop::*;
pub(super) use crop_overlay::*;
pub(super) use overlay::*;
pub(super) use panel::*;
pub(super) use timeline::*;
pub(super) use toolbar::*;
pub(super) use viewport::*;

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
