//! Tailwind v4 dimensions used by the pinned Mira components (16px root rem).

pub const SPACING: f32 = 4.0;
pub const CONTROL_HEIGHT: f32 = 7.0 * SPACING;
pub const CONTROL_PADDING_X: f32 = 2.0 * SPACING;
pub const CONTROL_GAP: f32 = SPACING;
pub const CONTROL_ICON: f32 = 3.5 * SPACING;
pub const TEXT_LINE_HEIGHT: f32 = 1.625;
pub const FIELD_GAP: f32 = 2.0 * SPACING;
// Compact list groups use Mira ItemGroup size=xs (gap-2).
pub const LIST_GAP: f32 = 2.0 * SPACING;
pub const GROUP_GAP: f32 = 4.0 * SPACING;
pub const CARD_PADDING: f32 = 4.0 * SPACING;
pub const POPOVER_PADDING: f32 = 2.5 * SPACING;
pub const MENU_PADDING: f32 = SPACING;
pub const MENU_ITEM_HEIGHT: f32 = CONTROL_HEIGHT;
pub const TABS_HEIGHT: f32 = 8.0 * SPACING;
pub const TABS_PADDING: f32 = 3.0;
pub const TAB_HEIGHT: f32 = TABS_HEIGHT - 2.0 * TABS_PADDING - 1.0;
pub const CHECKBOX_SIZE: f32 = 4.0 * SPACING;
pub const CHECKBOX_RADIUS: f32 = 4.0;
pub const SLIDER_THUMB: f32 = 3.0 * SPACING;
pub const BADGE_HEIGHT: f32 = 5.0 * SPACING;
pub const BADGE_TEXT: f32 = 10.0;
pub const FOCUS_WIDTH: f32 = 2.0;
pub const FOCUS_ALPHA: f32 = 0.3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlSize {
    Xs,
    Sm,
    Default,
    Lg,
}

impl ControlSize {
    #[must_use]
    pub const fn height(self) -> f32 {
        match self {
            Self::Xs => 20.0,
            Self::Sm => 24.0,
            Self::Default => 28.0,
            Self::Lg => 32.0,
        }
    }
    #[must_use]
    pub const fn icon(self) -> f32 {
        match self {
            Self::Xs => 10.0,
            Self::Sm => 12.0,
            Self::Default => 14.0,
            Self::Lg => 16.0,
        }
    }
    #[must_use]
    pub const fn padding(self) -> f32 {
        match self {
            Self::Lg => 10.0,
            _ => 8.0,
        }
    }
    #[must_use]
    pub const fn text(self) -> f32 {
        match self {
            Self::Xs => 10.0,
            _ => 12.0,
        }
    }
    #[must_use]
    pub const fn radius(self) -> f32 {
        match self {
            Self::Xs => super::RADIUS_SM,
            _ => super::RADIUS_MD,
        }
    }
}
