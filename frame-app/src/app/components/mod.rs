pub(in crate::app) use patterns::*;
mod animated_icon;
pub(in crate::app) use animated_icon::frame_animated_icon;
mod patterns;
pub(in crate::app) use button::*;
pub(in crate::app) use checkbox::*;
pub(in crate::app) use color_picker::*;
pub(in crate::app) use hit_target::*;
pub(in crate::app) use list_item::*;
pub(in crate::app) use scrollbar::*;
pub(in crate::app) use select::*;
pub(in crate::app) use slider::*;
pub(in crate::app) use tooltip::*;

mod button;
mod checkbox;
mod color_picker;
#[cfg(test)]
mod control_motion_tests;
mod hit_target;
#[cfg(test)]
mod icon_button_tooltip_tests;
mod list_item;
mod scrollbar;
mod select;
#[cfg(test)]
mod select_ui_tests;
mod slider;
mod tooltip;
mod tooltip_grace;
mod tooltip_position;

pub(in crate::app) mod popover_state;
pub(in crate::app) mod tooltip_state;

mod select_control;
pub(in crate::app) use select_control::*;

mod surface_motion;
pub(in crate::app) use surface_motion::{SurfaceOrigin, frame_surface_motion};
mod floating_popover;
pub(in crate::app) use floating_popover::{PopoverAnchor, frame_floating_popover};

mod select_field;
pub(in crate::app) use select_field::{
    FrameOption, frame_boolean_field, frame_option_field, frame_select_field,
};
mod collapsible;
pub(in crate::app) use collapsible::frame_collapsible;
mod option_tabs;
mod tab_indicator;
pub(in crate::app) use tab_indicator::FrameTabIndicator;

pub(in crate::app) use option_tabs::{frame_option_tabs, frame_tab_trigger};
