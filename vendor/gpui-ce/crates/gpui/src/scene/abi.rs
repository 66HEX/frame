//! CPU layout metadata consumed by the shader compiler. Keep this beside the scene types
//! so private fields are checked too, without exposing them to renderer implementations.

use super::*;
use crate::{AtlasTextureId, DevicePixels, LinearColorStop, Size};

#[doc(hidden)]
pub struct SceneBufferLayout {
    pub name: &'static str,
    pub size: usize,
    pub fields: &'static [(&'static str, usize)],
}

macro_rules! layout {
    ($ty:ty, $name:literal, $($field:ident),+ $(,)?) => {
        SceneBufferLayout {
            name: $name,
            size: std::mem::size_of::<$ty>(),
            fields: &[$((stringify!($field), std::mem::offset_of!($ty, $field))),+],
        }
    };
}

#[doc(hidden)]
pub const SCENE_BUFFER_LAYOUTS: &[SceneBufferLayout] = &[
    layout!(Bounds<ScaledPixels>, "Bounds", origin, size),
    layout!(Bounds<DevicePixels>, "AtlasBounds", origin, size),
    layout!(
        Corners<ScaledPixels>,
        "Corners",
        top_left,
        top_right,
        bottom_right,
        bottom_left
    ),
    layout!(Edges<ScaledPixels>, "Edges", top, right, bottom, left),
    layout!(SceneHsla, "Hsla", h, s, l, a),
    layout!(LinearColorStop, "LinearColorStop", color, percentage),
    layout!(
        Background,
        "Background",
        tag,
        color_space,
        solid,
        gradient_angle_or_pattern_height,
        colors,
        padding
    ),
    layout!(AtlasTextureId, "AtlasTextureId", index, kind),
    layout!(AtlasTile, "AtlasTile", texture_id, tile_id, padding, bounds),
    layout!(
        TransformationMatrix,
        "TransformationMatrix",
        rotation_scale,
        translation
    ),
    layout!(
        Quad,
        "Quad",
        order,
        border_style,
        border_dashed_length,
        border_dashed_gap,
        bounds,
        content_mask,
        background,
        border_color,
        corner_radii,
        border_widths,
        corner_smoothing,
        padding
    ),
    layout!(
        Shadow,
        "Shadow",
        order,
        blur_radius,
        bounds,
        corner_radii,
        content_mask,
        color,
        element_bounds,
        element_corner_radii,
        inset,
        corner_smoothing
    ),
    layout!(
        Underline,
        "Underline",
        order,
        padding,
        bounds,
        content_mask,
        color,
        thickness,
        wavy
    ),
    layout!(
        MonochromeSprite,
        "MonochromeSprite",
        order,
        padding,
        bounds,
        content_mask,
        color,
        tile,
        transformation
    ),
    layout!(
        SubpixelSprite,
        "SubpixelSprite",
        order,
        padding,
        bounds,
        content_mask,
        color,
        tile,
        transformation
    ),
    layout!(
        PolychromeSprite,
        "PolychromeSprite",
        order,
        grayscale,
        opacity,
        corner_smoothing,
        bounds,
        content_mask,
        corner_radii,
        tile,
        transformation
    ),
];

// These wrappers travel as shader scalars/vectors, without separate WGSL structs.
const _: () = {
    assert!(std::mem::size_of::<ShaderBool>() == 4);
    assert!(std::mem::size_of::<BorderStyle>() == 4);
    assert!(std::mem::size_of::<crate::AtlasTextureKind>() == 4);
    assert!(std::mem::size_of::<ContentMask<ScaledPixels>>() == 32);
    assert!(std::mem::offset_of!(ContentMask<ScaledPixels>, bounds) == 0);
    assert!(std::mem::offset_of!(ContentMask<ScaledPixels>, fade_out) == 16);
    assert!(std::mem::size_of::<Point<ScaledPixels>>() == 8);
    assert!(std::mem::offset_of!(Point<ScaledPixels>, x) == 0);
    assert!(std::mem::offset_of!(Point<ScaledPixels>, y) == 4);
    assert!(std::mem::size_of::<Size<ScaledPixels>>() == 8);
    assert!(std::mem::offset_of!(Size<ScaledPixels>, width) == 0);
    assert!(std::mem::offset_of!(Size<ScaledPixels>, height) == 4);
};
