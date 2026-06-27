//! Frame visual tokens ported from the original Svelte/Tailwind stylesheet.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RgbaToken {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl RgbaToken {
    #[must_use]
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red: red as f32 / 255.0,
            green: green as f32 / 255.0,
            blue: blue as f32 / 255.0,
            alpha: 1.0,
        }
    }

    #[must_use]
    pub const fn with_alpha(self, alpha: f32) -> Self {
        Self { alpha, ..self }
    }
}

pub const BACKGROUND: RgbaToken = RgbaToken::from_rgb(20, 22, 26);
pub const FOREGROUND: RgbaToken = RgbaToken::from_rgb(255, 255, 255);
pub const TRANSPARENT: RgbaToken = RgbaToken::from_rgb(0, 0, 0).with_alpha(0.0);
pub const SIDEBAR: RgbaToken = RgbaToken::from_rgb(32, 34, 37);
pub const DROPDOWN: RgbaToken = RgbaToken::from_rgb(43, 45, 48);

pub const FRAME_GRAY_100: RgbaToken = FOREGROUND.with_alpha(0.05);
pub const FRAME_GRAY_200: RgbaToken = FOREGROUND.with_alpha(0.10);
pub const FRAME_GRAY_400: RgbaToken = FOREGROUND.with_alpha(0.20);
pub const FRAME_GRAY_600: RgbaToken = FOREGROUND.with_alpha(0.40);

pub const FRAME_BLUE: RgbaToken = RgbaToken::from_rgb(29, 78, 216);
pub const FRAME_RED: RgbaToken = RgbaToken::from_rgb(185, 28, 28);
pub const FRAME_AMBER: RgbaToken = RgbaToken::from_rgb(245, 158, 11);

pub const RADIUS_BASE: f32 = 3.6;
pub const RADIUS_XS: f32 = RADIUS_BASE;
pub const RADIUS_SM: f32 = RADIUS_BASE * 2.0;
pub const RADIUS_MD: f32 = RADIUS_BASE * 3.0;
pub const RADIUS_LG: f32 = RADIUS_BASE * 4.0;
pub const RADIUS_XL: f32 = RADIUS_BASE * 6.0;

pub const TEXT_SCALE: f32 = 1.0;
pub const TEXT_UI_BASE_SIZE: f32 = 10.0;
pub const TEXT_ROW_BASE_SIZE: f32 = 12.0;
pub const TEXT_EMOJI_BASE_SIZE: f32 = 16.0;
pub const TEXT_MARKDOWN_BASE_SIZE: f32 = 10.0;
pub const TEXT_MARKDOWN_LIST_BASE_SIZE: f32 = 10.0;
pub const TEXT_INPUT_CARET_BASE_HEIGHT: f32 = 14.0;

pub const TEXT_UI_SIZE: f32 = TEXT_UI_BASE_SIZE * TEXT_SCALE;
pub const TEXT_LABEL_SIZE: f32 = TEXT_UI_SIZE;
pub const TEXT_ROW_SIZE: f32 = TEXT_ROW_BASE_SIZE * TEXT_SCALE;
pub const TEXT_EMOJI_SIZE: f32 = TEXT_EMOJI_BASE_SIZE * TEXT_SCALE;
pub const TEXT_MARKDOWN_SIZE: f32 = TEXT_MARKDOWN_BASE_SIZE * TEXT_SCALE;
pub const TEXT_MARKDOWN_LIST_SIZE: f32 = TEXT_MARKDOWN_LIST_BASE_SIZE * TEXT_SCALE;
pub const TEXT_INPUT_CARET_HEIGHT: f32 = TEXT_INPUT_CARET_BASE_HEIGHT * TEXT_SCALE;
pub const MIN_HIT_AREA: f32 = 40.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_channel_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 0.0001);
    }

    mod rgba_token {
        use super::*;

        #[test]
        fn from_rgb_normalizes_8_bit_channels() {
            let color = RgbaToken::from_rgb(48, 49, 54);

            assert_eq!(
                color,
                RgbaToken {
                    red: 48.0 / 255.0,
                    green: 49.0 / 255.0,
                    blue: 54.0 / 255.0,
                    alpha: 1.0,
                }
            );
        }

        #[test]
        fn with_alpha_preserves_rgb_channels() {
            let color = FOREGROUND.with_alpha(0.4);

            assert_eq!(color, FRAME_GRAY_600);
        }
    }

    mod color_tokens {
        use super::*;

        #[test]
        fn solid_tokens_match_original_oklch_srgb_values() {
            assert_eq!(BACKGROUND, RgbaToken::from_rgb(20, 22, 26));
            assert_eq!(SIDEBAR, RgbaToken::from_rgb(32, 34, 37));
            assert_eq!(DROPDOWN, RgbaToken::from_rgb(43, 45, 48));
            assert_eq!(FRAME_BLUE, RgbaToken::from_rgb(29, 78, 216));
            assert_eq!(FRAME_RED, RgbaToken::from_rgb(185, 28, 28));
            assert_eq!(FRAME_AMBER, RgbaToken::from_rgb(245, 158, 11));
        }

        #[test]
        fn translucent_frame_grays_use_white_with_original_alpha_steps() {
            assert_channel_close(FRAME_GRAY_100.alpha, 0.05);
            assert_channel_close(FRAME_GRAY_200.alpha, 0.10);
            assert_channel_close(FRAME_GRAY_400.alpha, 0.20);
            assert_channel_close(FRAME_GRAY_600.alpha, 0.40);
        }
    }

    mod radius_tokens {
        use super::*;

        #[test]
        fn large_radius_matches_tailwind_rounded_lg_from_original_radius_scale() {
            assert_eq!(RADIUS_LG, 14.4);
        }

        #[test]
        fn nested_icon_button_radius_keeps_concentric_relationship_with_padding() {
            assert_channel_close(RADIUS_LG - RADIUS_MD, RADIUS_BASE);
        }
    }

    mod interaction_tokens {
        use super::*;

        #[test]
        fn minimum_hit_area_matches_design_system_floor() {
            assert_eq!(MIN_HIT_AREA, 40.0);
        }
    }

    mod typography_tokens {
        use super::*;

        #[test]
        fn text_scale_defaults_to_original_svelte_size_scale() {
            assert_eq!(TEXT_SCALE, 1.0);
        }

        #[test]
        fn ui_text_matches_original_ten_pixel_controls() {
            assert_eq!(TEXT_UI_SIZE, 10.0);
            assert_eq!(TEXT_LABEL_SIZE, TEXT_UI_SIZE);
        }

        #[test]
        fn row_text_matches_original_file_list_text_xs() {
            assert_eq!(TEXT_ROW_SIZE, 12.0);
        }

        #[test]
        fn auxiliary_text_tokens_match_remaining_svelte_contexts() {
            assert_eq!(TEXT_EMOJI_SIZE, 16.0);
            assert_eq!(TEXT_MARKDOWN_SIZE, 11.0);
            assert_eq!(TEXT_MARKDOWN_LIST_SIZE, 10.0);
            assert_eq!(TEXT_INPUT_CARET_HEIGHT, 14.0);
        }
    }
}
