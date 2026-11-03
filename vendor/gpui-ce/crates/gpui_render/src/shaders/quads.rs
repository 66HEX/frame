#[wgsl_rs::wgsl]
pub mod quad {
    use super::super::common::*;
    use super::super::corner_smoothing::*;
    use wgsl_rs::std::*;

    #[derive(Clone, Copy, Wgsl)]
    pub struct Quad {
        pub order: u32,
        pub border_style: BorderStyle,
        pub border_dashed_length: f32,
        pub border_dashed_gap: f32,
        pub bounds: Bounds,
        pub content_mask: ContentMask,
        pub background: Background,
        pub border_color: Background,
        pub corner_radii: Corners,
        pub border_widths: Edges,
        pub corner_smoothing: f32,
        pub padding: u32,
    }
    storage!(group(1), binding(0), QUADS: RuntimeArray<Quad>);

    pub const DEFINITELY_OUTSIDE_INNER_BORDER: f32 = -1.0;

    #[derive(Clone, Copy, Wgsl)]
    pub struct QuadGeometry {
        pub point: Vec2f,
        pub center_to_point: Vec2f,
        pub corner_radius: f32,
        pub corner_center_to_point: Vec2f,
        pub reduced_border: Vec2f,
        pub straight_border_inner_corner_to_point: Vec2f,
        pub near_rounded_corner: bool,
        pub unrounded: bool,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct BorderDistances {
        pub outer: f32,
        pub inner: f32,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct DashPosition {
        pub position: f32,
        pub perimeter: f32,
        pub velocity: f32,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct RoundedDashLayout {
        pub side_velocities: Edges,
        pub corner_velocities: Corners,
        pub right_start: f32,
        pub bottom_right_start: f32,
        pub bottom_left_start: f32,
        pub left_start: f32,
        pub top_left_start: f32,
        pub perimeter: f32,
    }

    pub fn quad_geometry(quad: Quad, position: Vec2f) -> QuadGeometry {
        let half_size = Bounds::half_size(quad.bounds);
        let point = position - quad.bounds.origin;
        let center_to_point = point - half_size;
        let corner_radius = pick_corner_radius(center_to_point, quad.corner_radii);
        let corner_to_point = abs(center_to_point) - half_size;
        let corner_center_to_point = corner_to_point + corner_radius;
        let border = vec2f(
            select(
                quad.border_widths.right,
                quad.border_widths.left,
                center_to_point.x < 0.0,
            ),
            select(
                quad.border_widths.bottom,
                quad.border_widths.top,
                center_to_point.y < 0.0,
            ),
        );
        let reduced_border = vec2f(
            select(border.x, -PIXEL_ANTIALIAS_RADIUS, border.x == 0.0),
            select(border.y, -PIXEL_ANTIALIAS_RADIUS, border.y == 0.0),
        );
        QuadGeometry {
            point,
            center_to_point,
            corner_radius,
            corner_center_to_point,
            reduced_border,
            straight_border_inner_corner_to_point: corner_to_point + reduced_border,
            near_rounded_corner: corner_center_to_point.x >= 0.0 && corner_center_to_point.y >= 0.0,
            unrounded: Corners::is_zero(quad.corner_radii),
        }
    }

    pub fn is_unaffected_background(geometry: QuadGeometry) -> bool {
        geometry.straight_border_inner_corner_to_point.x < -PIXEL_ANTIALIAS_RADIUS
            && geometry.straight_border_inner_corner_to_point.y < -PIXEL_ANTIALIAS_RADIUS
            && !geometry.near_rounded_corner
    }

    pub fn inner_border_signed_distance(geometry: QuadGeometry, outer_signed_distance: f32) -> f32 {
        if geometry.corner_center_to_point.x <= 0.0 || geometry.corner_center_to_point.y <= 0.0 {
            return -max(
                geometry.straight_border_inner_corner_to_point.x,
                geometry.straight_border_inner_corner_to_point.y,
            );
        }
        if geometry.straight_border_inner_corner_to_point.x > 0.0
            || geometry.straight_border_inner_corner_to_point.y > 0.0
        {
            return DEFINITELY_OUTSIDE_INNER_BORDER;
        }
        if geometry.reduced_border.x == geometry.reduced_border.y {
            return -(outer_signed_distance + geometry.reduced_border.x);
        }

        let ellipse_radii = max(
            vec2f(0.0, 0.0),
            geometry.corner_radius - geometry.reduced_border,
        );
        quarter_ellipse_signed_distance(geometry.corner_center_to_point, ellipse_radii)
    }

    pub fn border_distances(geometry: QuadGeometry) -> BorderDistances {
        let outer = rounded_rectangle_signed_distance_from_corner(
            geometry.corner_center_to_point,
            geometry.corner_radius,
        );
        BorderDistances {
            outer,
            inner: inner_border_signed_distance(geometry, outer),
        }
    }

    pub fn dash_period_per_border_width(quad: Quad) -> f32 {
        max(quad.border_dashed_length, 0.0) + max(quad.border_dashed_gap, 0.0)
    }

    pub fn dash_velocity(border_width: f32, period_per_border_width: f32) -> f32 {
        if border_width <= 0.0 || period_per_border_width <= 0.0 {
            0.0
        } else {
            1.0 / period_per_border_width / border_width
        }
    }

    pub fn straight_dash_position(quad: Quad, geometry: QuadGeometry) -> DashPosition {
        let horizontal = geometry.corner_center_to_point.x < geometry.corner_center_to_point.y;
        let border_width = select(
            max(quad.border_widths.right, quad.border_widths.left),
            max(quad.border_widths.bottom, quad.border_widths.top),
            horizontal,
        );
        let velocity = dash_velocity(border_width, dash_period_per_border_width(quad));

        DashPosition {
            position: select(geometry.point.y, geometry.point.x, horizontal) * velocity,
            perimeter: select(quad.bounds.size.y, quad.bounds.size.x, horizontal) * velocity,
            velocity,
        }
    }

    pub fn side_dash_velocities(border_widths: Edges, period_per_border_width: f32) -> Edges {
        Edges {
            top: dash_velocity(border_widths.top, period_per_border_width),
            right: dash_velocity(border_widths.right, period_per_border_width),
            bottom: dash_velocity(border_widths.bottom, period_per_border_width),
            left: dash_velocity(border_widths.left, period_per_border_width),
        }
    }

    pub fn straight_side_dash_lengths(bounds: Bounds, radii: Corners, velocities: Edges) -> Edges {
        Edges {
            top: (bounds.size.x - radii.top_left - radii.top_right) * velocities.top,
            right: (bounds.size.y - radii.top_right - radii.bottom_right) * velocities.right,
            bottom: (bounds.size.x - radii.bottom_right - radii.bottom_left) * velocities.bottom,
            left: (bounds.size.y - radii.bottom_left - radii.top_left) * velocities.left,
        }
    }

    pub fn corner_dash_velocities(side_velocities: Edges) -> Corners {
        Corners {
            top_left: corner_dash_velocity(side_velocities.top, side_velocities.left),
            top_right: corner_dash_velocity(side_velocities.top, side_velocities.right),
            bottom_right: corner_dash_velocity(side_velocities.bottom, side_velocities.right),
            bottom_left: corner_dash_velocity(side_velocities.bottom, side_velocities.left),
        }
    }

    pub fn corner_dash_lengths(radii: Corners, velocities: Corners) -> Corners {
        let quarter_turn = PI / 2.0;
        Corners {
            top_left: radii.top_left * quarter_turn * velocities.top_left,
            top_right: radii.top_right * quarter_turn * velocities.top_right,
            bottom_right: radii.bottom_right * quarter_turn * velocities.bottom_right,
            bottom_left: radii.bottom_left * quarter_turn * velocities.bottom_left,
        }
    }

    pub fn rounded_dash_layout(quad: Quad) -> RoundedDashLayout {
        let side_velocities =
            side_dash_velocities(quad.border_widths, dash_period_per_border_width(quad));
        let side_lengths =
            straight_side_dash_lengths(quad.bounds, quad.corner_radii, side_velocities);
        let corner_velocities = corner_dash_velocities(side_velocities);
        let corner_lengths = corner_dash_lengths(quad.corner_radii, corner_velocities);

        perimeter_dash_layout(
            side_velocities,
            side_lengths,
            corner_velocities,
            corner_values(corner_lengths),
        )
    }

    pub fn perimeter_dash_layout(
        side_velocities: Edges,
        side_lengths: Edges,
        corner_velocities: Corners,
        corner_lengths: Vec4f,
    ) -> RoundedDashLayout {
        let right_start = side_lengths.top + corner_lengths.y;
        let bottom_right_start = right_start + side_lengths.right;
        let bottom_left_start = bottom_right_start + corner_lengths.z + side_lengths.bottom;
        let left_start = bottom_left_start + corner_lengths.w;
        let top_left_start = left_start + side_lengths.left;

        RoundedDashLayout {
            side_velocities,
            corner_velocities,
            right_start,
            bottom_right_start,
            bottom_left_start,
            left_start,
            top_left_start,
            perimeter: top_left_start + corner_lengths.x,
        }
    }

    pub fn smoothed_corner_lengths(quad: Quad, prepared: PreparedCorners) -> Vec4f {
        if quad.border_style != BorderStyle::Dashed || quad.corner_smoothing <= 0.0 {
            return corner_values(quad.corner_radii) * (PI / 2.0);
        }

        let radii = corner_values(quad.corner_radii);
        let top_left = figma_corner_length(figma_corner_params(
            radii.x,
            prepared.horizontal_reaches.x,
            prepared.vertical_reaches.x,
            prepared.smoothing_factors,
        ));
        let top_right = figma_corner_length(figma_corner_params(
            radii.y,
            prepared.horizontal_reaches.y,
            prepared.vertical_reaches.y,
            prepared.smoothing_factors,
        ));
        let bottom_right = figma_corner_length(figma_corner_params(
            radii.z,
            prepared.horizontal_reaches.z,
            prepared.vertical_reaches.z,
            prepared.smoothing_factors,
        ));
        let bottom_left = figma_corner_length(figma_corner_params(
            radii.w,
            prepared.horizontal_reaches.w,
            prepared.vertical_reaches.w,
            prepared.smoothing_factors,
        ));

        vec4f(top_left, top_right, bottom_right, bottom_left)
    }

    pub fn smoothed_dash_layout(
        quad: Quad,
        prepared: PreparedCorners,
        corner_lengths: Vec4f,
    ) -> RoundedDashLayout {
        let side_velocities =
            side_dash_velocities(quad.border_widths, dash_period_per_border_width(quad));
        let side_lengths = Edges {
            top: (quad.bounds.size.x
                - prepared.horizontal_reaches.x
                - prepared.horizontal_reaches.y)
                * side_velocities.top,
            right: (quad.bounds.size.y - prepared.vertical_reaches.y - prepared.vertical_reaches.z)
                * side_velocities.right,
            bottom: (quad.bounds.size.x
                - prepared.horizontal_reaches.z
                - prepared.horizontal_reaches.w)
                * side_velocities.bottom,
            left: (quad.bounds.size.y - prepared.vertical_reaches.w - prepared.vertical_reaches.x)
                * side_velocities.left,
        };
        let corner_velocities = corner_dash_velocities(side_velocities);

        perimeter_dash_layout(
            side_velocities,
            side_lengths,
            corner_velocities,
            corner_lengths * corner_values(corner_velocities),
        )
    }

    pub fn smoothed_dashed_border_alpha(
        quad: Quad,
        geometry: QuadGeometry,
        rectangle_sample: FigmaRectangleSample,
        prepared: PreparedCorners,
        corner_lengths: Vec4f,
        border: Vec2f,
        straight_border_inner_corner_to_point: Vec2f,
    ) -> f32 {
        let dash_period_per_width = dash_period_per_border_width(quad);

        if dash_period_per_width <= 0.0 {
            return 1.0;
        }

        let dash_layout = smoothed_dash_layout(quad, prepared, corner_lengths);
        let mut dash_position = 0.0;
        let mut dash_velocity = 0.0;

        if rectangle_sample.corner != FIGMA_NO_CORNER
            && rectangle_sample.signed_distance.segment != FIGMA_SEGMENT_STRAIGHT
        {
            let corner_idx = rectangle_sample.corner;
            let radii = corner_values(quad.corner_radii);
            let corner_length = corner_lengths[corner_idx];
            let params = figma_corner_params(
                radii[corner_idx],
                prepared.horizontal_reaches[corner_idx],
                prepared.vertical_reaches[corner_idx],
                prepared.smoothing_factors,
            );
            let progress =
                figma_corner_progress(params, rectangle_sample.signed_distance, corner_length);

            match corner_idx {
                0u32 => {
                    dash_velocity = dash_layout.corner_velocities.top_left;
                    dash_position =
                        dash_layout.top_left_start + (corner_length - progress) * dash_velocity;
                }
                1u32 => {
                    dash_velocity = dash_layout.corner_velocities.top_right;
                    dash_position = dash_layout.right_start + progress * dash_velocity;
                }
                2u32 => {
                    dash_velocity = dash_layout.corner_velocities.bottom_right;
                    dash_position =
                        dash_layout.bottom_right_start + (corner_length - progress) * dash_velocity;
                }
                _ => {
                    dash_velocity = dash_layout.corner_velocities.bottom_left;
                    dash_position = dash_layout.bottom_left_start + progress * dash_velocity;
                }
            }
        } else {
            let mut horizontal =
                straight_border_inner_corner_to_point.x < straight_border_inner_corner_to_point.y;

            let mut straight_width = select(border.x, border.y, horizontal);
            if straight_width <= 0.0 {
                horizontal = !horizontal;
                straight_width = select(border.x, border.y, horizontal);
            }

            if horizontal {
                if geometry.center_to_point.y < 0.0 {
                    dash_velocity = dash_layout.side_velocities.top;
                    dash_position =
                        (geometry.point.x - prepared.horizontal_reaches.x) * dash_velocity;
                } else {
                    dash_velocity = dash_layout.side_velocities.bottom;
                    dash_position = dash_layout.bottom_left_start
                        - (geometry.point.x - prepared.horizontal_reaches.w) * dash_velocity;
                }
            } else if geometry.center_to_point.x < 0.0 {
                dash_velocity = dash_layout.side_velocities.left;
                dash_position = dash_layout.top_left_start
                    - (geometry.point.y - prepared.vertical_reaches.x) * dash_velocity;
            } else {
                dash_velocity = dash_layout.side_velocities.right;
                dash_position = dash_layout.right_start
                    + (geometry.point.y - prepared.vertical_reaches.y) * dash_velocity;
            }
        }

        let dash_length = max(quad.border_dashed_length, 0.0) / dash_period_per_width;

        if dash_layout.perimeter >= 1.0 {
            let period = dash_layout.perimeter / floor(dash_layout.perimeter);
            dash_coverage(dash_position, period, dash_length, dash_velocity)
        } else {
            1.0
        }
    }

    pub fn rounded_dash_position(quad: Quad, geometry: QuadGeometry) -> DashPosition {
        let radii = quad.corner_radii;
        let dash_layout = rounded_dash_layout(quad);
        let horizontal = geometry.corner_center_to_point.x < geometry.corner_center_to_point.y;
        let on_right = geometry.center_to_point.x >= 0.0;
        let on_bottom = geometry.center_to_point.y >= 0.0;

        let top_position = (geometry.point.x - radii.top_left) * dash_layout.side_velocities.top;
        let right_position = dash_layout.right_start
            + (geometry.point.y - radii.top_right) * dash_layout.side_velocities.right;
        let bottom_position = dash_layout.bottom_left_start
            - (geometry.point.x - radii.bottom_left) * dash_layout.side_velocities.bottom;
        let left_position = dash_layout.top_left_start
            - (geometry.point.y - radii.top_left) * dash_layout.side_velocities.left;
        let horizontal_position = select(top_position, bottom_position, on_bottom);
        let vertical_position = select(left_position, right_position, on_right);
        let side_position = select(vertical_position, horizontal_position, horizontal);
        let horizontal_velocity = select(
            dash_layout.side_velocities.top,
            dash_layout.side_velocities.bottom,
            on_bottom,
        );
        let vertical_velocity = select(
            dash_layout.side_velocities.left,
            dash_layout.side_velocities.right,
            on_right,
        );
        let side_velocity = select(vertical_velocity, horizontal_velocity, horizontal);

        if geometry.near_rounded_corner {
            let corner_position = atan2(
                geometry.corner_center_to_point.y,
                geometry.corner_center_to_point.x,
            ) * geometry.corner_radius;
            let top_right_position =
                dash_layout.right_start - corner_position * dash_layout.corner_velocities.top_right;
            let bottom_right_position = dash_layout.bottom_right_start
                + corner_position * dash_layout.corner_velocities.bottom_right;
            let bottom_left_position = dash_layout.left_start
                - corner_position * dash_layout.corner_velocities.bottom_left;
            let top_left_position = dash_layout.top_left_start
                + corner_position * dash_layout.corner_velocities.top_left;
            let right_position = select(top_right_position, bottom_right_position, on_bottom);
            let left_position = select(top_left_position, bottom_left_position, on_bottom);
            let position = select(left_position, right_position, on_right);
            let right_velocity = select(
                dash_layout.corner_velocities.top_right,
                dash_layout.corner_velocities.bottom_right,
                on_bottom,
            );
            let left_velocity = select(
                dash_layout.corner_velocities.top_left,
                dash_layout.corner_velocities.bottom_left,
                on_bottom,
            );
            return DashPosition {
                position,
                perimeter: dash_layout.perimeter,
                velocity: select(left_velocity, right_velocity, on_right),
            };
        }

        DashPosition {
            position: side_position,
            perimeter: dash_layout.perimeter,
            velocity: side_velocity,
        }
    }

    pub fn dashed_border_alpha(quad: Quad, geometry: QuadGeometry) -> f32 {
        let dash_period_per_width = dash_period_per_border_width(quad);

        if dash_period_per_width <= 0.0 {
            return 1.0;
        }

        let mut dash = DashPosition {
            position: 0.0,
            perimeter: 0.0,
            velocity: 0.0,
        };

        if geometry.unrounded {
            dash = straight_dash_position(quad, geometry);
        } else {
            dash = rounded_dash_position(quad, geometry);
        }

        let dash_length = max(quad.border_dashed_length, 0.0) / dash_period_per_width;
        let perimeter = dash.perimeter - select(0.0, dash_length, geometry.unrounded);

        if perimeter >= 1.0 {
            let period = perimeter / floor(perimeter);
            dash_coverage(dash.position, period, dash_length, dash.velocity)
        } else if geometry.unrounded && perimeter > dash_length {
            dash_coverage(dash.position, perimeter, dash_length, dash.velocity)
        } else {
            1.0
        }
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct QuadVertexData {
        pub position: Vec4f,
        pub border: PreparedPaint,
        pub quad_id: u32,
        pub clip_distances: Vec4f,
        pub fill: PreparedPaint,
    }

    pub fn prepare_quad_vertex(vertex_id: u32, instance_id: u32, quad: Quad) -> QuadVertexData {
        let vertex = rectangle_vertex(vertex_id, quad.bounds);
        QuadVertexData {
            position: vertex.clip_position,
            border: prepare_paint(Paint::new(quad.border_color, quad.bounds)),
            quad_id: instance_id,
            clip_distances: clip_distances(vertex.viewport_position, quad.content_mask.bounds),
            fill: prepare_paint(Paint::new(quad.background, quad.bounds)),
        }
    }

    #[derive(Wgsl)]
    pub struct QuadVarying {
        #[builtin(position)]
        pub position: Vec4f,
        #[location(0)]
        #[interpolate(flat)]
        pub border_solid: Vec4f,
        #[location(1)]
        #[interpolate(flat)]
        pub quad_id: u32,
        #[location(2)]
        pub clip_distances: Vec4f,
        #[location(3)]
        #[interpolate(flat)]
        pub fill_solid: Vec4f,
        #[location(4)]
        #[interpolate(flat)]
        pub fill_color0: Vec4f,
        #[location(5)]
        #[interpolate(flat)]
        pub fill_color1: Vec4f,
        #[location(6)]
        #[interpolate(flat)]
        pub border_color0: Vec4f,
        #[location(7)]
        #[interpolate(flat)]
        pub border_color1: Vec4f,
    }

    #[vertex]
    pub fn vertex_quad(
        #[builtin(vertex_index)] vertex_id: u32,
        #[builtin(instance_index)] instance_id: u32,
    ) -> QuadVarying {
        let quad = get!(QUADS)[instance_id as usize];
        let vertex = prepare_quad_vertex(vertex_id, instance_id, quad);
        QuadVarying {
            position: vertex.position,
            border_solid: vertex.border.solid,
            quad_id: vertex.quad_id,
            clip_distances: vertex.clip_distances,
            fill_solid: vertex.fill.solid,
            fill_color0: vertex.fill.color0,
            fill_color1: vertex.fill.color1,
            border_color0: vertex.border.color0,
            border_color1: vertex.border.color1,
        }
    }

    #[fragment]
    pub fn fragment_quad(input: QuadVarying) -> Vec4f {
        if is_clipped(input.clip_distances) {
            return transparent();
        }
        let quad = get!(QUADS)[input.quad_id as usize];
        let fade = ContentMask::alpha(quad.content_mask, input.position.xy());
        let fill_color = paint_color(
            Paint::new(quad.background, quad.bounds),
            input.position.xy(),
            PreparedPaint::new(input.fill_solid, input.fill_color0, input.fill_color1),
        );
        if Edges::is_zero(quad.border_widths) && Corners::is_zero(quad.corner_radii) {
            return blend_color(fill_color, fade);
        }

        let geometry = quad_geometry(quad, input.position.xy());
        if is_unaffected_background(geometry) {
            return blend_color(fill_color, fade);
        }

        let distances = border_distances(geometry);
        let mut color = fill_color;
        if max(distances.inner, distances.outer) < PIXEL_ANTIALIAS_RADIUS {
            let mut border_color = paint_color(
                Paint::new(quad.border_color, quad.bounds),
                input.position.xy(),
                PreparedPaint::new(input.border_solid, input.border_color0, input.border_color1),
            );
            if quad.border_style == BorderStyle::Dashed {
                border_color.w *= dashed_border_alpha(quad, geometry);
            }
            let blended_border = over(fill_color, border_color);
            let factor = antialiased_coverage(distances.inner);
            color = mix(
                fill_color,
                blended_border,
                vec4f(factor, factor, factor, factor),
            );
        }
        blend_color(color, antialiased_coverage(distances.outer) * fade)
    }

    #[derive(Wgsl)]
    pub struct SmoothedQuadVarying {
        #[builtin(position)]
        pub position: Vec4f,
        #[location(0)]
        #[interpolate(flat)]
        pub border_solid: Vec4f,
        #[location(1)]
        #[interpolate(flat)]
        pub quad_id: u32,
        #[location(2)]
        pub clip_distances: Vec4f,
        #[location(3)]
        #[interpolate(flat)]
        pub fill_solid: Vec4f,
        #[location(4)]
        #[interpolate(flat)]
        pub fill_color0: Vec4f,
        #[location(5)]
        #[interpolate(flat)]
        pub fill_color1: Vec4f,
        #[location(6)]
        #[interpolate(flat)]
        pub horizontal_corner_reaches: Vec4f,
        #[location(7)]
        #[interpolate(flat)]
        pub vertical_corner_reaches: Vec4f,
        #[location(8)]
        #[interpolate(flat)]
        pub corner_lengths: Vec4f,
        #[location(9)]
        #[interpolate(flat)]
        pub smoothing_factors: Vec4f,
        #[location(10)]
        #[interpolate(flat)]
        pub superellipse_power: f32,
        #[location(11)]
        #[interpolate(flat)]
        pub border_color0: Vec4f,
        #[location(12)]
        #[interpolate(flat)]
        pub border_color1: Vec4f,
    }

    #[vertex]
    pub fn vertex_smoothed_quad(
        #[builtin(vertex_index)] vertex_id: u32,
        #[builtin(instance_index)] instance_id: u32,
    ) -> SmoothedQuadVarying {
        let quad = get!(QUADS)[instance_id as usize];
        let vertex = prepare_quad_vertex(vertex_id, instance_id, quad);
        let prepared = prepare_corners(
            quad.bounds.size,
            quad.corner_radii,
            quad.corner_smoothing,
            quad.border_style == BorderStyle::Solid && Edges::is_zero(quad.border_widths),
        );

        SmoothedQuadVarying {
            position: vertex.position,
            border_solid: vertex.border.solid,
            quad_id: vertex.quad_id,
            clip_distances: vertex.clip_distances,
            fill_solid: vertex.fill.solid,
            fill_color0: vertex.fill.color0,
            fill_color1: vertex.fill.color1,
            horizontal_corner_reaches: prepared.horizontal_reaches,
            vertical_corner_reaches: prepared.vertical_reaches,
            corner_lengths: smoothed_corner_lengths(quad, prepared),
            smoothing_factors: prepared.smoothing_factors,
            superellipse_power: prepared.superellipse_power,
            border_color0: vertex.border.color0,
            border_color1: vertex.border.color1,
        }
    }

    #[fragment]
    pub fn fragment_smoothed_quad(input: SmoothedQuadVarying) -> Vec4f {
        if is_clipped(input.clip_distances) {
            return transparent();
        }
        let quad = get!(QUADS)[input.quad_id as usize];
        let fade = ContentMask::alpha(quad.content_mask, input.position.xy());
        let fill_color = paint_color(
            Paint::new(quad.background, quad.bounds),
            input.position.xy(),
            PreparedPaint::new(input.fill_solid, input.fill_color0, input.fill_color1),
        );
        let prepared = PreparedCorners {
            horizontal_reaches: input.horizontal_corner_reaches,
            vertical_reaches: input.vertical_corner_reaches,
            smoothing_factors: input.smoothing_factors,
            superellipse_power: input.superellipse_power,
        };

        if Edges::is_zero(quad.border_widths) {
            let distance = prepared_corner_signed_distance(
                input.position.xy(),
                quad.bounds,
                quad.corner_radii,
                quad.corner_smoothing,
                prepared,
            );

            return blend_color(fill_color, antialiased_coverage(distance) * fade);
        }

        let geometry = quad_geometry(quad, input.position.xy());
        let rectangle_sample = figma_smooth_rectangle_sample(
            input.position.xy(),
            quad.bounds,
            quad.corner_radii,
            prepared.horizontal_reaches,
            prepared.vertical_reaches,
            prepared.smoothing_factors,
        );
        let mut border = vec2f(
            select(
                quad.border_widths.right,
                quad.border_widths.left,
                geometry.center_to_point.x < 0.0,
            ),
            select(
                quad.border_widths.bottom,
                quad.border_widths.top,
                geometry.center_to_point.y < 0.0,
            ),
        );

        match rectangle_sample.corner {
            0u32 => {
                border = vec2f(quad.border_widths.left, quad.border_widths.top);
            }
            1u32 => {
                border = vec2f(quad.border_widths.right, quad.border_widths.top);
            }
            2u32 => {
                border = vec2f(quad.border_widths.right, quad.border_widths.bottom);
            }
            3u32 => {
                border = vec2f(quad.border_widths.left, quad.border_widths.bottom);
            }
            _ => {}
        }

        let reduced_border = vec2f(
            select(border.x, -PIXEL_ANTIALIAS_RADIUS, border.x == 0.0),
            select(border.y, -PIXEL_ANTIALIAS_RADIUS, border.y == 0.0),
        );
        let half_size = Bounds::half_size(quad.bounds);
        let corner_to_point = abs(geometry.center_to_point) - half_size;
        let straight_border_inner_corner_to_point = corner_to_point + reduced_border;
        let near_curve = rectangle_sample.signed_distance.segment != FIGMA_SEGMENT_STRAIGHT;

        if straight_border_inner_corner_to_point.x < -PIXEL_ANTIALIAS_RADIUS
            && straight_border_inner_corner_to_point.y < -PIXEL_ANTIALIAS_RADIUS
            && !near_curve
        {
            return blend_color(fill_color, fade);
        }

        let outer = rectangle_sample.signed_distance.distance;
        let mut inner = 0.0;

        if near_curve {
            let normal = abs(rectangle_sample.signed_distance.normal);
            let active_sides = vec2f(
                select(0.0, 1.0, border.x > 0.0),
                select(0.0, 1.0, border.y > 0.0),
            );
            let effective_width = length(border * normal)
                - PIXEL_ANTIALIAS_RADIUS * (1.0 - length(active_sides * normal));
            inner = -(outer + effective_width);
        } else {
            inner = -max(
                straight_border_inner_corner_to_point.x,
                straight_border_inner_corner_to_point.y,
            );
        }

        let mut color = fill_color;

        if max(inner, outer) < PIXEL_ANTIALIAS_RADIUS {
            let mut border_color = paint_color(
                Paint::new(quad.border_color, quad.bounds),
                input.position.xy(),
                PreparedPaint::new(input.border_solid, input.border_color0, input.border_color1),
            );
            if quad.border_style == BorderStyle::Dashed {
                border_color.w *= smoothed_dashed_border_alpha(
                    quad,
                    geometry,
                    rectangle_sample,
                    prepared,
                    input.corner_lengths,
                    border,
                    straight_border_inner_corner_to_point,
                );
            }

            let blended_border = over(fill_color, border_color);
            let factor = antialiased_coverage(inner);

            color = mix(
                fill_color,
                blended_border,
                vec4f(factor, factor, factor, factor),
            );
        }

        blend_color(color, antialiased_coverage(outer) * fade)
    }
}
