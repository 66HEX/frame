#[wgsl_rs::wgsl]
pub mod shadow {
    use super::super::common::*;
    use super::super::corner_smoothing::*;
    use wgsl_rs::std::*;

    #[derive(Clone, Copy, Wgsl)]
    pub struct Shadow {
        pub order: u32,
        pub blur_radius: f32,
        pub bounds: Bounds,
        pub corner_radii: Corners,
        pub content_mask: ContentMask,
        pub color: Background,
        pub element_bounds: Bounds,
        pub element_corner_radii: Corners,
        pub inset: ShaderBool,
        pub corner_smoothing: f32,
    }
    storage!(group(1), binding(0), SHADOWS: RuntimeArray<Shadow>);

    pub const SHADOW_INTEGRATION_SAMPLE_COUNT: i32 = 4;

    pub fn shadow_geometry(shadow: Shadow) -> Bounds {
        if is_enabled(shadow.inset) {
            return shadow.element_bounds;
        }

        let margin = GAUSSIAN_CUTOFF_STANDARD_DEVIATIONS * shadow.blur_radius;
        Bounds {
            origin: shadow.bounds.origin - vec2f(margin, margin),
            size: shadow.bounds.size + vec2f(2.0 * margin, 2.0 * margin),
        }
    }

    pub fn shadow_paint(shadow: Shadow) -> Paint {
        let mut bounds = shadow.bounds;
        if is_enabled(shadow.inset) {
            bounds = shadow.element_bounds;
        }

        Paint::new(shadow.color, bounds)
    }

    pub fn blurred_shadow_coverage(shadow: Shadow, position: Vec2f) -> f32 {
        let half_size = Bounds::half_size(shadow.bounds);
        let center_to_point = position - Bounds::center(shadow.bounds);
        let corner_radius = pick_corner_radius(center_to_point, shadow.corner_radii);
        let vertical_minimum = center_to_point.y - half_size.y;
        let vertical_maximum = center_to_point.y + half_size.y;
        let integration_radius = GAUSSIAN_CUTOFF_STANDARD_DEVIATIONS * shadow.blur_radius;
        let integration_start = clamp(-integration_radius, vertical_minimum, vertical_maximum);
        let integration_end = clamp(integration_radius, vertical_minimum, vertical_maximum);
        let sample_step =
            (integration_end - integration_start) / SHADOW_INTEGRATION_SAMPLE_COUNT as f32;

        let mut coverage = 0.0;
        let mut sample_index = 0;
        let mut sample_position = integration_start + sample_step * 0.5;
        while sample_index < SHADOW_INTEGRATION_SAMPLE_COUNT {
            coverage += integrated_rounded_rectangle_coverage(
                center_to_point.x,
                center_to_point.y - sample_position,
                shadow.blur_radius,
                corner_radius,
                half_size,
            ) * gaussian(sample_position, shadow.blur_radius)
                * sample_step;
            sample_position += sample_step;
            sample_index += 1;
        }
        coverage
    }

    pub fn shadow_coverage(shadow: Shadow, position: Vec2f) -> f32 {
        let mut coverage = 0.0;
        if shadow.blur_radius == 0.0 {
            coverage = antialiased_coverage(rounded_rectangle_signed_distance(
                position,
                shadow.bounds,
                shadow.corner_radii,
            ));
        } else {
            coverage = blurred_shadow_coverage(shadow, position);
        }

        if is_enabled(shadow.inset) {
            coverage = (1.0 - coverage)
                * antialiased_coverage(rounded_rectangle_signed_distance(
                    position,
                    shadow.element_bounds,
                    shadow.element_corner_radii,
                ));
        }
        coverage
    }

    pub fn smoothed_shadow_coverage(
        shadow: Shadow,
        position: Vec2f,
        prepared: PreparedCorners,
        prepared_element: PreparedCorners,
    ) -> f32 {
        let mut coverage = 0.0;

        if shadow.blur_radius == 0.0 {
            coverage = antialiased_coverage(prepared_corner_signed_distance(
                position,
                shadow.bounds,
                shadow.corner_radii,
                shadow.corner_smoothing,
                prepared,
            ));
        } else if !Corners::is_zero(shadow.corner_radii) {
            let blur_limit = GAUSSIAN_CUTOFF_STANDARD_DEVIATIONS * shadow.blur_radius;
            let half_size = Bounds::half_size(shadow.bounds);
            let center_to_point = position - Bounds::center(shadow.bounds);
            let box_delta = abs(center_to_point) - half_size;
            let outside_delta = max(box_delta, vec2f(0.0, 0.0));

            if dot(outside_delta, outside_delta) <= blur_limit * blur_limit {
                let local_point = position - shadow.bounds.origin;
                let edge_depth = min(
                    min(local_point.x, shadow.bounds.size.x - local_point.x),
                    min(local_point.y, shadow.bounds.size.y - local_point.y),
                );
                let corner_candidate = figma_has_corner_candidate(
                    local_point,
                    shadow.bounds.size,
                    prepared.horizontal_reaches,
                    prepared.vertical_reaches,
                );

                if !corner_candidate && edge_depth >= blur_limit {
                    coverage = 1.0;
                } else {
                    let distance = prepared_corner_signed_distance(
                        position,
                        shadow.bounds,
                        shadow.corner_radii,
                        shadow.corner_smoothing,
                        prepared,
                    );
                    coverage = gaussian_signed_distance_coverage(distance, shadow.blur_radius);
                }
            }
        } else {
            coverage = blurred_shadow_coverage(shadow, position);
        }

        if is_enabled(shadow.inset) {
            coverage = (1.0 - coverage)
                * antialiased_coverage(prepared_corner_signed_distance(
                    position,
                    shadow.element_bounds,
                    shadow.element_corner_radii,
                    shadow.corner_smoothing,
                    prepared_element,
                ));
        }

        coverage
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct ShadowVertexData {
        pub position: Vec4f,
        pub paint: PreparedPaint,
        pub shadow_id: u32,
        pub clip_distances: Vec4f,
    }

    pub fn prepare_shadow_vertex(
        vertex_id: u32,
        instance_id: u32,
        shadow: Shadow,
    ) -> ShadowVertexData {
        let vertex = rectangle_vertex(vertex_id, shadow_geometry(shadow));
        ShadowVertexData {
            position: vertex.clip_position,
            paint: prepare_paint(shadow_paint(shadow)),
            shadow_id: instance_id,
            clip_distances: clip_distances(vertex.viewport_position, shadow.content_mask.bounds),
        }
    }

    #[derive(Wgsl)]
    pub struct ShadowVarying {
        #[builtin(position)]
        pub position: Vec4f,
        #[location(0)]
        #[interpolate(flat)]
        pub paint_solid: Vec4f,
        #[location(1)]
        #[interpolate(flat)]
        pub paint_color0: Vec4f,
        #[location(2)]
        #[interpolate(flat)]
        pub paint_color1: Vec4f,
        #[location(3)]
        #[interpolate(flat)]
        pub shadow_id: u32,
        #[location(4)]
        pub clip_distances: Vec4f,
    }

    #[vertex]
    pub fn vertex_shadow(
        #[builtin(vertex_index)] vertex_id: u32,
        #[builtin(instance_index)] instance_id: u32,
    ) -> ShadowVarying {
        let shadow = get!(SHADOWS)[instance_id as usize];
        let vertex = prepare_shadow_vertex(vertex_id, instance_id, shadow);
        ShadowVarying {
            position: vertex.position,
            paint_solid: vertex.paint.solid,
            paint_color0: vertex.paint.color0,
            paint_color1: vertex.paint.color1,
            shadow_id: vertex.shadow_id,
            clip_distances: vertex.clip_distances,
        }
    }

    #[fragment]
    pub fn fragment_shadow(input: ShadowVarying) -> Vec4f {
        if is_clipped(input.clip_distances) {
            return transparent();
        }
        let shadow = get!(SHADOWS)[input.shadow_id as usize];
        let color = paint_color(
            shadow_paint(shadow),
            input.position.xy(),
            PreparedPaint::new(input.paint_solid, input.paint_color0, input.paint_color1),
        );
        blend_color(
            color,
            shadow_coverage(shadow, input.position.xy())
                * ContentMask::alpha(shadow.content_mask, input.position.xy()),
        )
    }

    #[derive(Wgsl)]
    pub struct SmoothedShadowVarying {
        #[builtin(position)]
        pub position: Vec4f,
        #[location(0)]
        #[interpolate(flat)]
        pub paint_solid: Vec4f,
        #[location(1)]
        #[interpolate(flat)]
        pub paint_color0: Vec4f,
        #[location(2)]
        #[interpolate(flat)]
        pub paint_color1: Vec4f,
        #[location(3)]
        #[interpolate(flat)]
        pub shadow_id: u32,
        #[location(4)]
        pub clip_distances: Vec4f,
        #[location(5)]
        #[interpolate(flat)]
        pub horizontal_corner_reaches: Vec4f,
        #[location(6)]
        #[interpolate(flat)]
        pub vertical_corner_reaches: Vec4f,
        #[location(7)]
        #[interpolate(flat)]
        pub element_horizontal_corner_reaches: Vec4f,
        #[location(8)]
        #[interpolate(flat)]
        pub element_vertical_corner_reaches: Vec4f,
        #[location(9)]
        #[interpolate(flat)]
        pub smoothing_factors: Vec4f,
    }

    #[vertex]
    pub fn vertex_smoothed_shadow(
        #[builtin(vertex_index)] vertex_id: u32,
        #[builtin(instance_index)] instance_id: u32,
    ) -> SmoothedShadowVarying {
        let shadow = get!(SHADOWS)[instance_id as usize];
        let vertex = prepare_shadow_vertex(vertex_id, instance_id, shadow);
        let prepared = prepare_corners(
            shadow.bounds.size,
            shadow.corner_radii,
            shadow.corner_smoothing,
            false,
        );
        let prepared_element = prepare_corners(
            shadow.element_bounds.size,
            shadow.element_corner_radii,
            shadow.corner_smoothing,
            false,
        );

        SmoothedShadowVarying {
            position: vertex.position,
            paint_solid: vertex.paint.solid,
            paint_color0: vertex.paint.color0,
            paint_color1: vertex.paint.color1,
            shadow_id: vertex.shadow_id,
            clip_distances: vertex.clip_distances,
            horizontal_corner_reaches: prepared.horizontal_reaches,
            vertical_corner_reaches: prepared.vertical_reaches,
            element_horizontal_corner_reaches: prepared_element.horizontal_reaches,
            element_vertical_corner_reaches: prepared_element.vertical_reaches,
            smoothing_factors: prepared.smoothing_factors,
        }
    }

    #[fragment]
    pub fn fragment_smoothed_shadow(input: SmoothedShadowVarying) -> Vec4f {
        if is_clipped(input.clip_distances) {
            return transparent();
        }
        let shadow = get!(SHADOWS)[input.shadow_id as usize];
        let coverage = smoothed_shadow_coverage(
            shadow,
            input.position.xy(),
            PreparedCorners {
                horizontal_reaches: input.horizontal_corner_reaches,
                vertical_reaches: input.vertical_corner_reaches,
                smoothing_factors: input.smoothing_factors,
                superellipse_power: 0.0,
            },
            PreparedCorners {
                horizontal_reaches: input.element_horizontal_corner_reaches,
                vertical_reaches: input.element_vertical_corner_reaches,
                smoothing_factors: input.smoothing_factors,
                superellipse_power: 0.0,
            },
        );
        let color = paint_color(
            shadow_paint(shadow),
            input.position.xy(),
            PreparedPaint::new(input.paint_solid, input.paint_color0, input.paint_color1),
        );
        blend_color(
            color,
            coverage * ContentMask::alpha(shadow.content_mask, input.position.xy()),
        )
    }
}
