use super::*;

impl From<bool> for common::ShaderBool {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

impl From<gpui::Bounds<gpui::ScaledPixels>> for common::Bounds {
    fn from(bounds: gpui::Bounds<gpui::ScaledPixels>) -> Self {
        Self {
            origin: wgsl_rs::std::vec2f(bounds.origin.x.0, bounds.origin.y.0),
            size: wgsl_rs::std::vec2f(bounds.size.width.0, bounds.size.height.0),
        }
    }
}

impl From<gpui::Edges<gpui::ScaledPixels>> for common::Edges {
    fn from(edges: gpui::Edges<gpui::ScaledPixels>) -> Self {
        Self {
            top: edges.top.0,
            right: edges.right.0,
            bottom: edges.bottom.0,
            left: edges.left.0,
        }
    }
}

impl From<gpui::ContentMask<gpui::ScaledPixels>> for common::ContentMask {
    fn from(mask: gpui::ContentMask<gpui::ScaledPixels>) -> Self {
        Self {
            bounds: mask.bounds.into(),
            fade_out: mask.fade_out.into(),
        }
    }
}

impl From<gpui::Corners<gpui::ScaledPixels>> for common::Corners {
    fn from(corners: gpui::Corners<gpui::ScaledPixels>) -> Self {
        Self {
            top_left: corners.top_left.0,
            top_right: corners.top_right.0,
            bottom_right: corners.bottom_right.0,
            bottom_left: corners.bottom_left.0,
        }
    }
}
