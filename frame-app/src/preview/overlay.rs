use super::crop::{Point, clamp};

pub const DEFAULT_OVERLAY_WIDTH: f64 = 0.18;
pub const MIN_OVERLAY_WIDTH: f64 = 0.03;
pub const MAX_OVERLAY_WIDTH: f64 = 0.8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverlayDragHandle {
    Move,
    NorthWest,
    NorthEast,
    SouthEast,
    SouthWest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverlaySizeDirection {
    Increase,
    Decrease,
}

impl OverlaySizeDirection {
    const fn step(self) -> f64 {
        match self {
            Self::Increase => 0.025,
            Self::Decrease => -0.025,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayDragPoint {
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewOverlay {
    pub enabled: bool,
    pub path: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub opacity: f64,
    pub anchor: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OverlayModeChange {
    pub changed: bool,
    pub should_deactivate_crop: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreviewOverlayState {
    overlay_mode: bool,
    overlay: Option<PreviewOverlay>,
    drag_origin: Option<OverlayDragOrigin>,
}

#[derive(Clone, Debug, PartialEq)]
struct OverlayDragOrigin {
    handle: OverlayDragHandle,
    start_overlay: PreviewOverlay,
    start_point: OverlayDragPoint,
}

impl PreviewOverlayState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            overlay_mode: false,
            overlay: None,
            drag_origin: None,
        }
    }

    #[must_use]
    pub const fn overlay_mode(&self) -> bool {
        self.overlay_mode
    }

    #[must_use]
    pub const fn overlay(&self) -> Option<&PreviewOverlay> {
        self.overlay.as_ref()
    }

    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        self.drag_origin.is_some()
    }

    pub fn sync_initial_overlay(&mut self, initial_overlay: Option<&PreviewOverlay>) {
        if self.drag_origin.is_some() {
            return;
        }

        self.overlay = initial_overlay
            .filter(|overlay| overlay.enabled && !overlay.path.is_empty())
            .map(normalize_overlay);

        if self.overlay.is_none() {
            self.overlay_mode = false;
        }
    }

    pub fn set_overlay_from_path(
        &mut self,
        path: impl Into<String>,
        controls_disabled: bool,
    ) -> Option<PreviewOverlay> {
        if controls_disabled {
            return None;
        }

        let overlay = create_default_overlay(path);
        self.overlay_mode = true;
        self.overlay = Some(overlay.clone());
        Some(overlay)
    }

    pub const fn toggle_overlay_mode(&mut self, controls_disabled: bool) -> OverlayModeChange {
        if controls_disabled || self.overlay.is_none() {
            return OverlayModeChange {
                changed: false,
                should_deactivate_crop: false,
            };
        }

        let should_deactivate_crop = !self.overlay_mode;
        self.overlay_mode = !self.overlay_mode;
        OverlayModeChange {
            changed: true,
            should_deactivate_crop,
        }
    }

    pub const fn set_overlay_mode(
        &mut self,
        value: bool,
        controls_disabled: bool,
    ) -> OverlayModeChange {
        if controls_disabled && value {
            return OverlayModeChange {
                changed: false,
                should_deactivate_crop: false,
            };
        }

        let next_mode = value && self.overlay.is_some();
        let changed = self.overlay_mode != next_mode;
        self.overlay_mode = next_mode;
        OverlayModeChange {
            changed,
            should_deactivate_crop: value,
        }
    }

    pub fn begin_overlay_drag(
        &mut self,
        handle: OverlayDragHandle,
        point: OverlayDragPoint,
        controls_disabled: bool,
    ) -> bool {
        let Some(overlay) = &self.overlay else {
            return false;
        };
        if !self.overlay_mode || controls_disabled {
            return false;
        }

        let mut start_overlay = overlay.clone();
        start_overlay.width = point.width.unwrap_or(start_overlay.width);
        self.drag_origin = Some(OverlayDragOrigin {
            handle,
            start_overlay,
            start_point: point,
        });
        true
    }

    pub fn update_overlay_drag(&mut self, point: OverlayDragPoint) -> Option<PreviewOverlay> {
        let Some(drag_origin) = &self.drag_origin else {
            return None;
        };
        self.overlay.as_ref()?;

        let next_overlay = match drag_origin.handle {
            OverlayDragHandle::Move => {
                let height = drag_origin
                    .start_point
                    .height
                    .unwrap_or(drag_origin.start_overlay.width);
                let center = clamp_overlay_center(
                    drag_origin.start_overlay.x + point.x - drag_origin.start_point.x,
                    drag_origin.start_overlay.y + point.y - drag_origin.start_point.y,
                    drag_origin.start_overlay.width,
                    height,
                );
                PreviewOverlay {
                    x: center.x,
                    y: center.y,
                    anchor: "custom".to_string(),
                    ..drag_origin.start_overlay.clone()
                }
            }
            OverlayDragHandle::NorthWest
            | OverlayDragHandle::NorthEast
            | OverlayDragHandle::SouthEast
            | OverlayDragHandle::SouthWest => {
                let (Some(point_width), Some(start_width), Some(start_height)) = (
                    point.width,
                    drag_origin.start_point.width,
                    drag_origin.start_point.height,
                ) else {
                    return None;
                };
                if point_width <= 0.0 || start_width <= 0.0 || start_height <= 0.0 {
                    return None;
                }

                let start_left =
                    drag_origin.start_overlay.x - drag_origin.start_overlay.width / 2.0;
                let start_right =
                    drag_origin.start_overlay.x + drag_origin.start_overlay.width / 2.0;
                let start_top = drag_origin.start_overlay.y - start_height / 2.0;
                let start_bottom = drag_origin.start_overlay.y + start_height / 2.0;
                let anchor_x = if drag_origin.handle.anchors_right_edge() {
                    start_right
                } else {
                    start_left
                };
                let anchor_y = if drag_origin.handle.anchors_bottom_edge() {
                    start_bottom
                } else {
                    start_top
                };
                let aspect = start_height / start_width;
                let raw_width_from_x = (point.x - anchor_x).abs();
                let raw_width_from_y = (point.y - anchor_y).abs() / aspect;
                let width = clamp_overlay_width(raw_width_from_x.max(raw_width_from_y), aspect);
                let height = width * aspect;
                let center = clamp_overlay_center(
                    anchor_x + (drag_origin.handle.direction_x() * width) / 2.0,
                    anchor_y + (drag_origin.handle.direction_y() * height) / 2.0,
                    width,
                    height,
                );

                PreviewOverlay {
                    x: center.x,
                    y: center.y,
                    width,
                    anchor: "custom".to_string(),
                    ..drag_origin.start_overlay.clone()
                }
            }
        };

        self.overlay = Some(next_overlay.clone());
        Some(next_overlay)
    }

    pub fn end_overlay_drag(&mut self) {
        self.drag_origin = None;
    }

    pub fn set_opacity(&mut self, value: f64, controls_disabled: bool) -> Option<PreviewOverlay> {
        if controls_disabled {
            return None;
        }

        let overlay = self.overlay.as_mut()?;
        overlay.opacity = clamp(value, 0.0, 1.0);
        Some(overlay.clone())
    }

    pub fn nudge_size(
        &mut self,
        direction: OverlaySizeDirection,
        height_ratio: Option<f64>,
        controls_disabled: bool,
    ) -> Option<PreviewOverlay> {
        if controls_disabled {
            return None;
        }

        let overlay = self.overlay.as_mut()?;
        let height_ratio_for_width = height_ratio.unwrap_or(1.0);
        let width = clamp_overlay_width(overlay.width + direction.step(), height_ratio_for_width);
        let height = width * height_ratio.unwrap_or(1.0);
        let center = clamp_overlay_center(overlay.x, overlay.y, width, height);
        overlay.x = center.x;
        overlay.y = center.y;
        overlay.width = width;
        overlay.anchor = "custom".to_string();
        Some(overlay.clone())
    }

    pub fn remove_overlay(&mut self, controls_disabled: bool) -> Option<Option<PreviewOverlay>> {
        if controls_disabled {
            return None;
        }

        self.overlay_mode = false;
        self.overlay = None;
        Some(None)
    }

    pub fn destroy(&mut self) {
        self.end_overlay_drag();
    }
}

impl OverlayDragHandle {
    const fn anchors_right_edge(self) -> bool {
        matches!(self, Self::NorthWest | Self::SouthWest)
    }

    const fn anchors_bottom_edge(self) -> bool {
        matches!(self, Self::NorthWest | Self::NorthEast)
    }

    const fn direction_x(self) -> f64 {
        match self {
            Self::NorthWest | Self::SouthWest => -1.0,
            Self::NorthEast | Self::SouthEast | Self::Move => 1.0,
        }
    }

    const fn direction_y(self) -> f64 {
        match self {
            Self::NorthWest | Self::NorthEast => -1.0,
            Self::SouthEast | Self::SouthWest | Self::Move => 1.0,
        }
    }
}

#[must_use]
pub fn clamp_overlay_center(x: f64, y: f64, width: f64, height: f64) -> Point {
    let half_width = (width / 2.0).min(0.5);
    let half_height = (height / 2.0).min(0.5);
    Point {
        x: clamp(x, half_width, 1.0 - half_width),
        y: clamp(y, half_height, 1.0 - half_height),
    }
}

#[must_use]
pub fn max_overlay_width(height_ratio: f64) -> f64 {
    if !height_ratio.is_finite() || height_ratio <= 0.0 {
        return MAX_OVERLAY_WIDTH;
    }

    MAX_OVERLAY_WIDTH.min(1.0 / height_ratio)
}

#[must_use]
pub fn clamp_overlay_width(width: f64, height_ratio: f64) -> f64 {
    let max_width = max_overlay_width(height_ratio);
    let min_width = MIN_OVERLAY_WIDTH.min(max_width);
    clamp(width, min_width, max_width)
}

#[must_use]
pub fn create_default_overlay(path: impl Into<String>) -> PreviewOverlay {
    let width = DEFAULT_OVERLAY_WIDTH;
    let center = clamp_overlay_center(0.5, 0.5, width, width);
    PreviewOverlay {
        enabled: true,
        path: path.into(),
        x: center.x,
        y: center.y,
        width,
        opacity: 1.0,
        anchor: "custom".to_string(),
    }
}

#[must_use]
pub fn normalize_overlay(overlay: &PreviewOverlay) -> PreviewOverlay {
    let width = clamp(overlay.width, MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
    let center = clamp_overlay_center(overlay.x, overlay.y, width, width);
    PreviewOverlay {
        enabled: overlay.enabled,
        path: overlay.path.clone(),
        x: center.x,
        y: center.y,
        width,
        opacity: clamp(overlay.opacity, 0.0, 1.0),
        anchor: "custom".to_string(),
    }
}
