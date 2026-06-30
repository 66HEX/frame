pub const MIN_CROP: f64 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DragHandle {
    Move,
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewRotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

impl From<&str> for PreviewRotation {
    fn from(value: &str) -> Self {
        match value.trim() {
            "90" => Self::Deg90,
            "180" => Self::Deg180,
            "270" => Self::Deg270,
            _ => Self::Deg0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AspectOption {
    pub id: &'static str,
    pub display: &'static str,
    pub ratio: Option<f64>,
}

pub const ASPECT_OPTIONS: [AspectOption; 5] = [
    AspectOption {
        id: "free",
        display: "Free",
        ratio: None,
    },
    AspectOption {
        id: "1:1",
        display: "1:1",
        ratio: Some(1.0),
    },
    AspectOption {
        id: "4:5",
        display: "4:5",
        ratio: Some(4.0 / 5.0),
    },
    AspectOption {
        id: "16:9",
        display: "16:9",
        ratio: Some(16.0 / 9.0),
    },
    AspectOption {
        id: "9:16",
        display: "9:16",
        ratio: Some(9.0 / 16.0),
    },
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragDelta {
    pub dx: f64,
    pub dy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisualCropDrag<'a> {
    pub start_rect: CropRect,
    pub handle: DragHandle,
    pub start_point: Point,
    pub current_point: Point,
    pub aspect_id: &'a str,
    pub source_width: f64,
    pub source_height: f64,
    pub is_side_rotation: bool,
}

#[must_use]
pub fn aspect_value(id: &str) -> Option<f64> {
    ASPECT_OPTIONS
        .iter()
        .find(|option| option.id == id)
        .and_then(|option| option.ratio)
}

#[must_use]
pub const fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

#[must_use]
pub fn clamp_rect(rect: CropRect) -> CropRect {
    let mut x = rect.x;
    let mut y = rect.y;
    let mut width = rect.width;
    let mut height = rect.height;

    if width < MIN_CROP {
        width = MIN_CROP;
    }
    if height < MIN_CROP {
        height = MIN_CROP;
    }
    if x < 0.0 {
        x = 0.0;
    }
    if y < 0.0 {
        y = 0.0;
    }
    if x + width > 1.0 {
        x = 1.0 - width;
    }
    if y + height > 1.0 {
        y = 1.0 - height;
    }

    CropRect {
        x,
        y,
        width,
        height,
    }
}

#[must_use]
pub fn effective_aspect_ratio(
    target_ratio: f64,
    source_width: f64,
    source_height: f64,
    is_side_rotation: bool,
) -> f64 {
    if source_width.abs() <= f64::EPSILON || source_height.abs() <= f64::EPSILON {
        return target_ratio;
    }

    let physical_aspect = source_width / source_height;
    if is_side_rotation {
        1.0 / target_ratio / physical_aspect
    } else {
        target_ratio / physical_aspect
    }
}

#[must_use]
pub fn transform_crop_rect(
    rect: CropRect,
    rotation: PreviewRotation,
    flip_horizontal: bool,
    flip_vertical: bool,
    inverse: bool,
) -> CropRect {
    let center_x = rect.x + rect.width / 2.0 - 0.5;
    let center_y = rect.y + rect.height / 2.0 - 0.5;

    let (center_x, center_y, width, height) = if inverse {
        let (center_x, center_y) = flip_center(center_x, center_y, flip_horizontal, flip_vertical);
        inverse_rotate_center(center_x, center_y, rect.width, rect.height, rotation)
    } else {
        let (center_x, center_y, width, height) =
            rotate_center(center_x, center_y, rect.width, rect.height, rotation);
        let (center_x, center_y) = flip_center(center_x, center_y, flip_horizontal, flip_vertical);
        (center_x, center_y, width, height)
    };

    CropRect {
        x: center_x - width / 2.0 + 0.5,
        y: center_y - height / 2.0 + 0.5,
        width,
        height,
    }
}

#[must_use]
pub fn remap_drag_deltas(
    dx: f64,
    dy: f64,
    rotation: PreviewRotation,
    flip_horizontal: bool,
    flip_vertical: bool,
) -> DragDelta {
    let (mut dx, mut dy) = match rotation {
        PreviewRotation::Deg0 => (dx, dy),
        PreviewRotation::Deg90 => (dy, -dx),
        PreviewRotation::Deg180 => (-dx, -dy),
        PreviewRotation::Deg270 => (-dy, dx),
    };

    if flip_horizontal {
        dx = -dx;
    }
    if flip_vertical {
        dy = -dy;
    }

    DragDelta { dx, dy }
}

#[must_use]
pub fn adjust_rect_to_ratio(
    rect: CropRect,
    ratio: f64,
    source_width: f64,
    source_height: f64,
    is_side_rotation: bool,
) -> CropRect {
    let effective_ratio =
        effective_aspect_ratio(ratio, source_width, source_height, is_side_rotation);
    let mut width = rect.width;
    let mut height = rect.height;

    if width / height > effective_ratio {
        width = height * effective_ratio;
    } else {
        height = width / effective_ratio;
    }

    let center_x = rect.x + rect.width / 2.0;
    let center_y = rect.y + rect.height / 2.0;
    let mut x = center_x - width / 2.0;
    let mut y = center_y - height / 2.0;

    if x < 0.0 {
        x = 0.0;
    }
    if y < 0.0 {
        y = 0.0;
    }
    if x + width > 1.0 {
        x = 1.0 - width;
    }
    if y + height > 1.0 {
        y = 1.0 - height;
    }

    CropRect {
        x,
        y,
        width,
        height,
    }
}

#[must_use]
pub fn enforce_aspect(
    rect: CropRect,
    handle: DragHandle,
    start_rect: CropRect,
    ratio: f64,
    source_width: f64,
    source_height: f64,
    is_side_rotation: bool,
) -> CropRect {
    let effective_ratio =
        effective_aspect_ratio(ratio, source_width, source_height, is_side_rotation);
    let mut width = rect.width;
    let mut height = rect.height;

    if width / height > effective_ratio {
        width = height * effective_ratio;
    } else {
        height = width / effective_ratio;
    }

    let mut next = rect;
    match handle {
        DragHandle::East => {
            next.x = start_rect.x;
            next.width = width;
            let center_y = start_rect.y + start_rect.height / 2.0;
            next.y = center_y - height / 2.0;
            next.height = height;
        }
        DragHandle::West => {
            next.width = width;
            next.x = start_rect.x + start_rect.width - width;
            let center_y = start_rect.y + start_rect.height / 2.0;
            next.y = center_y - height / 2.0;
            next.height = height;
        }
        DragHandle::North => {
            next.height = height;
            next.y = start_rect.y + start_rect.height - height;
            let center_x = start_rect.x + start_rect.width / 2.0;
            next.x = center_x - width / 2.0;
            next.width = width;
        }
        DragHandle::South => {
            next.height = height;
            next.y = start_rect.y;
            let center_x = start_rect.x + start_rect.width / 2.0;
            next.x = center_x - width / 2.0;
            next.width = width;
        }
        DragHandle::NorthEast => {
            next.x = start_rect.x;
            next.y = start_rect.y + start_rect.height - height;
            next.width = width;
            next.height = height;
        }
        DragHandle::NorthWest => {
            next.width = width;
            next.height = height;
            next.x = start_rect.x + start_rect.width - width;
            next.y = start_rect.y + start_rect.height - height;
        }
        DragHandle::SouthEast => {
            next.x = start_rect.x;
            next.y = start_rect.y;
            next.width = width;
            next.height = height;
        }
        DragHandle::SouthWest => {
            next.width = width;
            next.height = height;
            next.x = start_rect.x + start_rect.width - width;
            next.y = start_rect.y;
        }
        DragHandle::Move => {}
    }

    next
}

#[must_use]
pub fn apply_visual_crop_drag(drag: VisualCropDrag<'_>) -> CropRect {
    let dx = drag.current_point.x - drag.start_point.x;
    let dy = drag.current_point.y - drag.start_point.y;

    if drag.handle == DragHandle::Move {
        let x = clamp(drag.start_rect.x + dx, 0.0, 1.0 - drag.start_rect.width);
        let y = clamp(drag.start_rect.y + dy, 0.0, 1.0 - drag.start_rect.height);
        return CropRect {
            x,
            y,
            width: drag.start_rect.width,
            height: drag.start_rect.height,
        };
    }

    let mut left = drag.start_rect.x;
    let mut right = drag.start_rect.x + drag.start_rect.width;
    let mut top = drag.start_rect.y;
    let mut bottom = drag.start_rect.y + drag.start_rect.height;

    if drag.handle.includes_west() {
        left = clamp(drag.start_rect.x + dx, 0.0, right - MIN_CROP);
    }
    if drag.handle.includes_east() {
        right = clamp(
            drag.start_rect.x + drag.start_rect.width + dx,
            left + MIN_CROP,
            1.0,
        );
    }
    if drag.handle.includes_north() {
        top = clamp(drag.start_rect.y + dy, 0.0, bottom - MIN_CROP);
    }
    if drag.handle.includes_south() {
        bottom = clamp(
            drag.start_rect.y + drag.start_rect.height + dy,
            top + MIN_CROP,
            1.0,
        );
    }

    let mut next_rect = CropRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    };

    if let Some(ratio) = aspect_value(drag.aspect_id) {
        next_rect = enforce_aspect(
            next_rect,
            drag.handle,
            drag.start_rect,
            ratio,
            drag.source_width,
            drag.source_height,
            drag.is_side_rotation,
        );
    }

    clamp_rect(next_rect)
}

#[must_use]
pub const fn handle_cursor(handle: DragHandle, is_side_rotation: bool) -> &'static str {
    match handle {
        DragHandle::North | DragHandle::South => {
            if is_side_rotation {
                "ew-resize"
            } else {
                "ns-resize"
            }
        }
        DragHandle::East | DragHandle::West => {
            if is_side_rotation {
                "ns-resize"
            } else {
                "ew-resize"
            }
        }
        DragHandle::NorthWest | DragHandle::SouthEast => {
            if is_side_rotation {
                "nesw-resize"
            } else {
                "nwse-resize"
            }
        }
        DragHandle::NorthEast | DragHandle::SouthWest => {
            if is_side_rotation {
                "nwse-resize"
            } else {
                "nesw-resize"
            }
        }
        DragHandle::Move => "default",
    }
}

impl DragHandle {
    const fn includes_west(self) -> bool {
        matches!(self, Self::West | Self::NorthWest | Self::SouthWest)
    }

    const fn includes_east(self) -> bool {
        matches!(self, Self::East | Self::NorthEast | Self::SouthEast)
    }

    const fn includes_north(self) -> bool {
        matches!(self, Self::North | Self::NorthEast | Self::NorthWest)
    }

    const fn includes_south(self) -> bool {
        matches!(self, Self::South | Self::SouthEast | Self::SouthWest)
    }
}

fn rotate_center(
    center_x: f64,
    center_y: f64,
    width: f64,
    height: f64,
    rotation: PreviewRotation,
) -> (f64, f64, f64, f64) {
    match rotation {
        PreviewRotation::Deg0 => (center_x, center_y, width, height),
        PreviewRotation::Deg90 => (-center_y, center_x, height, width),
        PreviewRotation::Deg180 => (-center_x, -center_y, width, height),
        PreviewRotation::Deg270 => (center_y, -center_x, height, width),
    }
}

fn inverse_rotate_center(
    center_x: f64,
    center_y: f64,
    width: f64,
    height: f64,
    rotation: PreviewRotation,
) -> (f64, f64, f64, f64) {
    match rotation {
        PreviewRotation::Deg0 => (center_x, center_y, width, height),
        PreviewRotation::Deg90 => (center_y, -center_x, height, width),
        PreviewRotation::Deg180 => (-center_x, -center_y, width, height),
        PreviewRotation::Deg270 => (-center_y, center_x, height, width),
    }
}

fn flip_center(
    mut center_x: f64,
    mut center_y: f64,
    flip_horizontal: bool,
    flip_vertical: bool,
) -> (f64, f64) {
    if flip_horizontal {
        center_x = -center_x;
    }
    if flip_vertical {
        center_y = -center_y;
    }

    (center_x, center_y)
}
