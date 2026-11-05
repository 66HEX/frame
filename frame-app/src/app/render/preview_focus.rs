use crate::app::accessibility::FrameFocusKey;
use crate::app::input::FrameTextInputKind;
use crate::app::preview::state::CropSelect;
use crate::app::preview::{PreviewCropRenderState, PreviewOverlayRenderState};
use crate::app::root::FrameRoot;
use crate::preview::{ASPECT_OPTIONS, PreviewControlAvailability, PreviewMediaKind};
use gpui::{Context, FocusHandle, RenderImage, Window};
use std::sync::Arc;

pub(super) struct PreviewFocusHandles {
    pub(super) preview_viewport_focus: FocusHandle,
    pub(super) preview_crop_tool_focus: FocusHandle,
    pub(super) preview_overlay_tool_focus: FocusHandle,
    pub(super) crop_width_focus: FocusHandle,
    pub(super) crop_height_focus: FocusHandle,
    pub(super) crop_option_focuses: Vec<FocusHandle>,
    pub(super) crop_unit_focus: FocusHandle,
    pub(super) crop_unit_option_focuses: Vec<FocusHandle>,
    pub(super) crop_custom_values: Option<(u32, u32)>,
    pub(super) crop_toolbar_panel_focus: FocusHandle,
    pub(super) crop_toolbar_first_focus: FocusHandle,
    pub(super) crop_toolbar_last_focus: FocusHandle,
    pub(super) overlay_toolbar_panel_focus: FocusHandle,
    pub(super) overlay_toolbar_first_focus: FocusHandle,
    pub(super) overlay_toolbar_last_focus: FocusHandle,
}

#[derive(Clone, Copy)]
pub(super) struct PreviewFocusContext<'a> {
    pub(super) preview_availability: &'a PreviewControlAvailability,
    pub(super) preview_crop: &'a PreviewCropRenderState,
    pub(super) preview_overlay: &'a PreviewOverlayRenderState,
    pub(super) preview_render_image: &'a Option<Arc<RenderImage>>,
    pub(super) app_root_focus: &'a FocusHandle,
}

impl FrameRoot {
    #[expect(
        clippy::too_many_lines,
        reason = "Keep the existing focus registration and element append order explicit during structural extraction."
    )]
    pub(super) fn prepare_preview_focus(
        &mut self,
        input: PreviewFocusContext<'_>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PreviewFocusHandles {
        let PreviewFocusContext {
            preview_availability,
            preview_crop,
            preview_overlay,
            preview_render_image,
            app_root_focus,
        } = input;
        let preview_visual_controls_enabled = preview_availability.media_kind
            != PreviewMediaKind::Unknown
            && !preview_availability.hide_visual_controls
            && !self.file_queue.selected_file_locked();
        let crop_tool_enabled = preview_visual_controls_enabled && preview_crop.has_crop_dimensions;
        let overlay_tool_enabled =
            preview_visual_controls_enabled && preview_availability.overlay_available;
        let preview_viewport_pan_enabled = preview_render_image.is_some()
            && preview_visual_controls_enabled
            && !preview_crop.crop_mode
            && !preview_overlay.overlay_mode;
        let preview_viewport_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-viewport".to_string()),
            preview_viewport_pan_enabled,
            cx,
        );
        let preview_crop_tool_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-tool-crop".to_string()),
            crop_tool_enabled,
            cx,
        );
        let preview_overlay_tool_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-tool-overlay".to_string()),
            overlay_tool_enabled,
            cx,
        );
        let crop_width_focus = self.ensure_text_input_focus(FrameTextInputKind::CropWidth, cx);
        let crop_height_focus = self.ensure_text_input_focus(FrameTextInputKind::CropHeight, cx);
        let crop_option_focuses: Vec<_> = ASPECT_OPTIONS
            .iter()
            .map(|option| option.id)
            .chain(std::iter::once("custom"))
            .map(|id| {
                self.ensure_focus(
                    FrameFocusKey::Control(format!("crop-ratio-{id}")),
                    self.preview_ui.crop_options.popover == Some(CropSelect::Aspect),
                    cx,
                )
            })
            .collect();
        let crop_unit_focus = self.ensure_focus(
            FrameFocusKey::Control("crop-units".into()),
            self.preview_ui.crop_options.custom,
            cx,
        );
        let crop_unit_option_focuses: Vec<_> = ["pixels", "ratio"]
            .iter()
            .map(|id| {
                self.ensure_focus(
                    FrameFocusKey::Control(format!("crop-units-{id}")),
                    self.preview_ui.crop_options.popover == Some(CropSelect::Units),
                    cx,
                )
            })
            .collect();
        if self.preview_ui.crop_options.custom && !self.preview_ui.crop_options.custom_dirty {
            self.set_crop_custom_mode(self.preview_ui.crop_options.is_ratio);
        }
        let crop_custom_values = self.custom_crop_values();
        let crop_toolbar_active = preview_crop.crop_mode && preview_crop.draft_crop.is_some();
        let crop_toolbar_panel_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-crop-toolbar".to_string()),
            false,
            cx,
        );
        let crop_toolbar_first_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-crop-action-free".to_string()),
            crop_toolbar_active,
            cx,
        );
        let crop_toolbar_last_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-crop-action-apply".to_string()),
            crop_toolbar_active,
            cx,
        );
        let overlay_toolbar_active =
            preview_overlay.overlay_mode && preview_overlay.overlay.is_some();
        let overlay_toolbar_panel_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-overlay-toolbar".to_string()),
            false,
            cx,
        );
        let overlay_toolbar_first_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-overlay-replace".to_string()),
            overlay_toolbar_active,
            cx,
        );
        let overlay_toolbar_last_focus = self.ensure_focus(
            FrameFocusKey::Control("preview-overlay-done".to_string()),
            overlay_toolbar_active,
            cx,
        );
        if !crop_toolbar_active
            && !overlay_toolbar_active
            && (crop_toolbar_first_focus.is_focused(window)
                || crop_toolbar_last_focus.is_focused(window))
        {
            if crop_tool_enabled {
                preview_crop_tool_focus.focus(window, cx);
            } else {
                app_root_focus.focus(window, cx);
            }
        }
        if !overlay_toolbar_active
            && !crop_toolbar_active
            && (overlay_toolbar_first_focus.is_focused(window)
                || overlay_toolbar_last_focus.is_focused(window))
        {
            if overlay_tool_enabled {
                preview_overlay_tool_focus.focus(window, cx);
            } else {
                app_root_focus.focus(window, cx);
            }
        }
        if !preview_viewport_pan_enabled && preview_viewport_focus.is_focused(window) {
            app_root_focus.focus(window, cx);
        }
        // Editing toolbars are non-modal. Focus their first control only when
        // opening; subsequent Tab presses must also reach the canvas handles.
        let edit_modes =
            window.use_keyed_state("preview-edit-focus-modes", cx, |_, _| (false, false));
        let (entering_crop, entering_overlay) = edit_modes.update(cx, |previous, _| {
            let entering = (
                crop_toolbar_active && !previous.0,
                overlay_toolbar_active && !previous.1,
            );
            *previous = (crop_toolbar_active, overlay_toolbar_active);
            entering
        });
        if entering_crop {
            crop_toolbar_first_focus.focus(window, cx);
        }
        if entering_overlay {
            overlay_toolbar_first_focus.focus(window, cx);
        }
        PreviewFocusHandles {
            preview_viewport_focus,
            preview_crop_tool_focus,
            preview_overlay_tool_focus,
            crop_width_focus,
            crop_height_focus,
            crop_option_focuses,
            crop_unit_focus,
            crop_unit_option_focuses,
            crop_custom_values,
            crop_toolbar_panel_focus,
            crop_toolbar_first_focus,
            crop_toolbar_last_focus,
            overlay_toolbar_panel_focus,
            overlay_toolbar_first_focus,
            overlay_toolbar_last_focus,
        }
    }
}
