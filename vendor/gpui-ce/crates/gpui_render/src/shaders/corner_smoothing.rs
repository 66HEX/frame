#[allow(clippy::manual_swap)]
#[wgsl_rs::wgsl]
mod source {
    use super::super::common::*;
    use wgsl_rs::std::*;

    pub const SUPERELLIPSE_DIAGONAL_INSET: f32 = 0.2928932188134524;
    pub const FIGMA_SEGMENT_STRAIGHT: u32 = 0u32;
    pub const FIGMA_SEGMENT_FIRST_CUBIC: u32 = 1u32;
    pub const FIGMA_SEGMENT_ARC: u32 = 2u32;
    pub const FIGMA_SEGMENT_SECOND_CUBIC: u32 = 3u32;
    pub const FIGMA_NO_CORNER: u32 = 4u32;
    pub const FIGMA_EPSILON: f32 = 0.000001;

    pub fn corner_values(corner_radii: Corners) -> Vec4f {
        vec4f(
            corner_radii.top_left,
            corner_radii.top_right,
            corner_radii.bottom_right,
            corner_radii.bottom_left,
        )
    }

    pub fn normalized_superellipse_power(corner_smoothing: f32) -> f32 {
        let smoothing = clamp(corner_smoothing, 0.0, 1.0);
        let normalized_diagonal = 1.0 - SUPERELLIPSE_DIAGONAL_INSET / (1.0 + smoothing);
        -log(2.0) / log(normalized_diagonal)
    }

    pub fn normalized_superellipse_reaches(corner_radii: Corners, corner_smoothing: f32) -> Vec4f {
        max(corner_values(corner_radii), vec4f(0.0, 0.0, 0.0, 0.0))
            * (1.0 + clamp(corner_smoothing, 0.0, 1.0))
    }

    pub fn can_use_normalized_superellipse(
        size: Vec2f,
        corner_radii: Corners,
        corner_smoothing: f32,
    ) -> bool {
        let radii = max(corner_values(corner_radii), vec4f(0.0, 0.0, 0.0, 0.0));
        let reaches = normalized_superellipse_reaches(corner_radii, corner_smoothing);
        corner_smoothing > 0.0
            && size.x > 0.0
            && size.y > 0.0
            && (radii.x > 0.0 || radii.y > 0.0 || radii.z > 0.0 || radii.w > 0.0)
            && reaches.x + reaches.y <= size.x
            && reaches.y + reaches.z <= size.y
            && reaches.z + reaches.w <= size.x
            && reaches.w + reaches.x <= size.y
    }

    pub fn normalized_superellipse_signed_distance_from_corner(
        corner_to_point: Vec2f,
        corner_radius: f32,
        corner_smoothing: f32,
        power: f32,
    ) -> f32 {
        let extent = max(corner_radius, 0.0) * (1.0 + clamp(corner_smoothing, 0.0, 1.0));
        let corner_center_to_point = corner_to_point + extent;

        if extent <= 0.0 || corner_center_to_point.x <= 0.0 || corner_center_to_point.y <= 0.0 {
            return max(corner_to_point.x, corner_to_point.y);
        }

        let normalized = corner_center_to_point / extent;
        let powered = pow(normalized, vec2f(power, power));
        let gradient = power * length(pow(normalized, vec2f(power - 1.0, power - 1.0)));

        extent * (powered.x + powered.y - 1.0) / max(gradient, FIGMA_EPSILON)
    }

    pub fn can_use_compact_corner_selection(size: Vec2f, reaches: Vec4f) -> bool {
        let half_short_side = 0.5 * min(size.x, size.y);

        reaches.x <= half_short_side
            && reaches.y <= half_short_side
            && reaches.z <= half_short_side
            && reaches.w <= half_short_side
    }

    pub fn compact_normalized_superellipse_signed_distance(
        point: Vec2f,
        bounds: Bounds,
        corner_radii: Corners,
        corner_smoothing: f32,
        power: f32,
    ) -> f32 {
        let half_size = Bounds::half_size(bounds);
        let center_to_point = point - Bounds::center(bounds);
        let corner_radius = pick_corner_radius(center_to_point, corner_radii);
        let corner_to_point = abs(center_to_point) - half_size;

        normalized_superellipse_signed_distance_from_corner(
            corner_to_point,
            corner_radius,
            corner_smoothing,
            power,
        )
    }

    pub fn reach_aware_normalized_superellipse_signed_distance(
        point: Vec2f,
        bounds: Bounds,
        corner_radii: Corners,
        corner_smoothing: f32,
        reaches: Vec4f,
        power: f32,
    ) -> f32 {
        let local_point = point - bounds.origin;
        let radii = corner_values(corner_radii);
        let mut distance =
            figma_nearest_straight_sample(local_point, bounds.size, reaches, reaches).distance;
        let mut corner = 0u32;

        while corner < 4u32 {
            if figma_is_corner_candidate(
                local_point,
                bounds.size,
                reaches[corner],
                reaches[corner],
                corner,
            ) {
                let candidate = normalized_superellipse_signed_distance_from_corner(
                    figma_corner_to_point(local_point, bounds.size, corner),
                    radii[corner],
                    corner_smoothing,
                    power,
                );

                if abs(candidate) <= abs(distance) {
                    distance = candidate;
                }
            }

            corner += 1u32;
        }

        distance
    }

    pub fn normalized_superellipse_signed_distance(
        point: Vec2f,
        bounds: Bounds,
        corner_radii: Corners,
        corner_smoothing: f32,
        reaches: Vec4f,
        power: f32,
    ) -> f32 {
        if can_use_compact_corner_selection(bounds.size, reaches) {
            return compact_normalized_superellipse_signed_distance(
                point,
                bounds,
                corner_radii,
                corner_smoothing,
                power,
            );
        }

        reach_aware_normalized_superellipse_signed_distance(
            point,
            bounds,
            corner_radii,
            corner_smoothing,
            reaches,
            power,
        )
    }

    // This corner construction derives from Tien Pham's figma-squircle.
    // See ../../THIRD_PARTY_NOTICES.md.
    #[derive(Clone, Copy, Wgsl)]
    pub struct FigmaCornerLayout {
        pub horizontal_budgets: Vec4f,
        pub vertical_budgets: Vec4f,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct FigmaCornerExtents {
        pub horizontal: Vec4f,
        pub vertical: Vec4f,
    }

    pub fn figma_split_side(length: f32, first_radius: f32, second_radius: f32) -> Vec2f {
        let total_radius = first_radius + second_radius;

        if total_radius == 0.0 {
            return vec2f(0.0, 0.0);
        }

        let first_budget = length * first_radius / total_radius;

        vec2f(first_budget, length - first_budget)
    }

    pub fn figma_corner_layout(size: Vec2f, corner_radii: Corners) -> FigmaCornerLayout {
        let mut radii = [
            max(corner_radii.top_left, 0.0),
            max(corner_radii.top_right, 0.0),
            max(corner_radii.bottom_right, 0.0),
            max(corner_radii.bottom_left, 0.0),
        ];
        let mut budgets = [-1.0, -1.0, -1.0, -1.0];
        let mut order = [0u32, 1u32, 3u32, 2u32];

        let mut sort_pass = 0u32;

        while sort_pass < 3u32 {
            let mut index = 0u32;

            while index < 3u32 - sort_pass {
                if radii[order[index as usize] as usize]
                    < radii[order[(index + 1u32) as usize] as usize]
                {
                    let swap = order[index as usize];
                    order[index as usize] = order[(index + 1u32) as usize];
                    order[(index + 1u32) as usize] = swap;
                }

                index += 1u32;
            }

            sort_pass += 1u32;
        }

        let mut rank = 0u32;
        while rank < 4u32 {
            let corner = order[rank as usize];
            let radius = radii[corner as usize];
            let mut horizontal_neighbor = 0u32;
            let mut vertical_neighbor = 0u32;

            match corner {
                0u32 => {
                    horizontal_neighbor = 1u32;
                    vertical_neighbor = 3u32;
                }
                1u32 => {
                    horizontal_neighbor = 0u32;
                    vertical_neighbor = 2u32;
                }
                2u32 => {
                    horizontal_neighbor = 3u32;
                    vertical_neighbor = 1u32;
                }
                _ => {
                    horizontal_neighbor = 2u32;
                    vertical_neighbor = 0u32;
                }
            }

            let horizontal_radius = radii[horizontal_neighbor as usize];
            let mut horizontal_budget = 0.0;

            if radius != 0.0 || horizontal_radius != 0.0 {
                if budgets[horizontal_neighbor as usize] >= 0.0 {
                    horizontal_budget = size.x - budgets[horizontal_neighbor as usize];
                } else {
                    horizontal_budget = size.x * radius / (radius + horizontal_radius);
                }
            }

            let vertical_radius = radii[vertical_neighbor as usize];
            let mut vertical_budget = 0.0;

            if radius != 0.0 || vertical_radius != 0.0 {
                if budgets[vertical_neighbor as usize] >= 0.0 {
                    vertical_budget = size.y - budgets[vertical_neighbor as usize];
                } else {
                    vertical_budget = size.y * radius / (radius + vertical_radius);
                }
            }

            let budget = max(0.0, min(horizontal_budget, vertical_budget));
            budgets[corner as usize] = budget;
            radii[corner as usize] = min(radius, budget);
            rank += 1u32;
        }

        let top = figma_split_side(size.x, radii[0usize], radii[1usize]);
        let bottom = figma_split_side(size.x, radii[3usize], radii[2usize]);
        let left = figma_split_side(size.y, radii[0usize], radii[3usize]);
        let right = figma_split_side(size.y, radii[1usize], radii[2usize]);

        FigmaCornerLayout {
            horizontal_budgets: vec4f(top.x, top.y, bottom.y, bottom.x),
            vertical_budgets: vec4f(left.x, right.x, right.y, left.y),
        }
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct FigmaAxisParams {
        pub a: f32,
        pub b: f32,
        pub p: f32,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct FigmaCornerParams {
        pub radius: f32,
        pub smoothing: f32,
        pub arc_sweep: f32,
        pub c: f32,
        pub d: f32,
        pub horizontal: FigmaAxisParams,
        pub vertical: FigmaAxisParams,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct CubicClosestPoint {
        pub point: Vec2f,
        pub tangent: Vec2f,
        pub distance: f32,
        pub path_t: f32,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct SignedDistanceSample {
        pub distance: f32,
        pub normal: Vec2f,
        pub path_t: f32,
        pub segment: u32,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct FigmaRectangleSample {
        pub signed_distance: SignedDistanceSample,
        pub corner: u32,
    }

    #[derive(Clone, Copy, Wgsl)]
    pub struct PreparedCorners {
        pub horizontal_reaches: Vec4f,
        pub vertical_reaches: Vec4f,
        pub smoothing_factors: Vec4f,
        pub superellipse_power: f32,
    }

    pub fn figma_smoothing_factors(corner_smoothing: f32) -> Vec4f {
        let smoothing = clamp(corner_smoothing, 0.0, 1.0);
        let arc_sweep = 0.5 * PI * (1.0 - smoothing);
        let beta = (PI / 4.0) * smoothing;
        let join_handle_factor = tan(0.5 * beta);

        vec4f(
            smoothing,
            sin(0.5 * arc_sweep) * sqrt(2.0),
            join_handle_factor * cos(beta),
            join_handle_factor * sin(beta),
        )
    }

    pub fn figma_corner_params(
        corner_radius: f32,
        horizontal_reach: f32,
        vertical_reach: f32,
        smoothing_factors: Vec4f,
    ) -> FigmaCornerParams {
        let safe_horizontal_reach = max(horizontal_reach, 0.0);
        let safe_vertical_reach = max(vertical_reach, 0.0);
        let radius = min(
            max(corner_radius, 0.0),
            min(safe_horizontal_reach, safe_vertical_reach),
        );
        let smoothing = select(0.0, smoothing_factors.x, radius > FIGMA_EPSILON);
        let desired_reach = radius * (1.0 + smoothing);
        let arc_sweep = 0.5 * PI * (1.0 - smoothing);
        let arc_delta = smoothing_factors.y * radius;
        let c = smoothing_factors.z * radius;
        let d = smoothing_factors.w * radius;
        let core_length = arc_delta + c + d;
        let ideal_b = max((desired_reach - core_length) / 3.0, 0.0);
        let horizontal_available = max(safe_horizontal_reach - core_length, 0.0);
        let vertical_available = max(safe_vertical_reach - core_length, 0.0);
        let shared_b = min(
            ideal_b,
            min(
                horizontal_available * (5.0 / 6.0),
                vertical_available * (5.0 / 6.0),
            ),
        );

        FigmaCornerParams {
            radius,
            smoothing,
            arc_sweep,
            c,
            d,
            horizontal: FigmaAxisParams {
                a: horizontal_available - shared_b,
                b: shared_b,
                p: safe_horizontal_reach,
            },
            vertical: FigmaAxisParams {
                a: vertical_available - shared_b,
                b: shared_b,
                p: safe_vertical_reach,
            },
        }
    }

    pub fn figma_corner_extents(
        corner_radii: Corners,
        horizontal_budgets: Vec4f,
        vertical_budgets: Vec4f,
        corner_smoothing: f32,
    ) -> FigmaCornerExtents {
        let zero = vec4f(0.0, 0.0, 0.0, 0.0);
        let horizontal = max(horizontal_budgets, zero);
        let vertical = max(vertical_budgets, zero);
        let radii = min(
            max(corner_values(corner_radii), zero),
            min(horizontal, vertical),
        );
        let reaches = radii * (1.0 + clamp(corner_smoothing, 0.0, 1.0));

        FigmaCornerExtents {
            horizontal: min(reaches, horizontal),
            vertical: min(reaches, vertical),
        }
    }

    pub fn prepare_corners(
        size: Vec2f,
        radii: Corners,
        smoothing: f32,
        allow_superellipse: bool,
    ) -> PreparedCorners {
        let radius_values = corner_values(radii);
        let mut prepared = PreparedCorners {
            horizontal_reaches: radius_values,
            vertical_reaches: radius_values,
            smoothing_factors: vec4f(0.0, 1.0, 0.0, 0.0),
            superellipse_power: 0.0,
        };

        if smoothing <= 0.0 {
            return prepared;
        }

        if allow_superellipse && can_use_normalized_superellipse(size, radii, smoothing) {
            let reaches = normalized_superellipse_reaches(radii, smoothing);
            prepared.horizontal_reaches = reaches;
            prepared.vertical_reaches = reaches;
            prepared.superellipse_power = normalized_superellipse_power(smoothing);
            return prepared;
        }

        let corner_layout = figma_corner_layout(size, radii);
        let extents = figma_corner_extents(
            radii,
            corner_layout.horizontal_budgets,
            corner_layout.vertical_budgets,
            smoothing,
        );

        prepared.horizontal_reaches = extents.horizontal;
        prepared.vertical_reaches = extents.vertical;
        prepared.smoothing_factors = figma_smoothing_factors(smoothing);
        prepared
    }

    pub fn figma_cubic_point(axis: FigmaAxisParams, c: f32, d: f32, t: f32) -> Vec2f {
        let x1 = 3.0 * axis.a;
        let x2 = 3.0 * (axis.b - axis.a);
        let x3 = axis.a - 2.0 * axis.b + c;
        let t2 = t * t;

        vec2f(t * (x1 + t * (x2 + t * x3)), d * t2 * t)
    }

    pub fn figma_cubic_derivative(axis: FigmaAxisParams, c: f32, d: f32, t: f32) -> Vec2f {
        let x1 = 3.0 * axis.a;
        let x2 = 3.0 * (axis.b - axis.a);
        let x3 = axis.a - 2.0 * axis.b + c;
        let t2 = t * t;

        vec2f(x1 + 2.0 * x2 * t + 3.0 * x3 * t2, 3.0 * d * t2)
    }

    pub fn figma_cubic_second_derivative(axis: FigmaAxisParams, c: f32, d: f32, t: f32) -> Vec2f {
        let x2 = 3.0 * (axis.b - axis.a);
        let x3 = axis.a - 2.0 * axis.b + c;

        vec2f(2.0 * x2 + 6.0 * x3 * t, 6.0 * d * t)
    }

    pub fn closest_figma_cubic(
        point: Vec2f,
        axis: FigmaAxisParams,
        c: f32,
        d: f32,
    ) -> CubicClosestPoint {
        let y_seed = pow(clamp(point.y / max(d, 0.00001), 0.0, 1.0), 1.0 / 3.0);
        let chord = figma_cubic_point(axis, c, d, 1.0);
        let chord_seed = clamp(
            dot(point, chord) / max(dot(chord, chord), FIGMA_EPSILON),
            0.0,
            1.0,
        );
        let y_delta = figma_cubic_point(axis, c, d, y_seed) - point;
        let chord_delta = figma_cubic_point(axis, c, d, chord_seed) - point;
        let mut t = select(
            y_seed,
            chord_seed,
            dot(chord_delta, chord_delta) < dot(y_delta, y_delta),
        );

        let mut iteration = 0u32;
        while iteration < 4u32 {
            let curve_point = figma_cubic_point(axis, c, d, t);
            let tangent = figma_cubic_derivative(axis, c, d, t);
            let second_derivative = figma_cubic_second_derivative(axis, c, d, t);
            let delta = curve_point - point;
            let denominator = dot(tangent, tangent) + dot(delta, second_derivative);

            if abs(denominator) > FIGMA_EPSILON {
                t = clamp(t - dot(delta, tangent) / denominator, 0.0, 1.0);
            }

            iteration += 1u32;
        }

        let mut closest_t = t;
        let mut closest_point = figma_cubic_point(axis, c, d, t);
        let mut closest_distance = length(point - closest_point);
        let start = vec2f(0.0, 0.0);
        let start_distance = length(point - start);

        if start_distance < closest_distance {
            closest_t = 0.0;
            closest_point = start;
            closest_distance = start_distance;
        }

        let end_distance = length(point - chord);

        if end_distance < closest_distance {
            closest_t = 1.0;
            closest_point = chord;
            closest_distance = end_distance;
        }

        CubicClosestPoint {
            point: closest_point,
            tangent: figma_cubic_derivative(axis, c, d, closest_t),
            distance: closest_distance,
            path_t: closest_t,
        }
    }

    pub fn figma_signed_distance(delta: Vec2f, normal: Vec2f, distance: f32) -> f32 {
        select(-distance, distance, dot(delta, normal) >= 0.0)
    }

    pub fn figma_cross_2d(a: Vec2f, b: Vec2f) -> f32 {
        a.x * b.y - a.y * b.x
    }

    pub fn figma_unfold_normal(normal: Vec2f, mirrored: bool) -> Vec2f {
        select(
            vec2f(normal.x, -normal.y),
            vec2f(-normal.y, normal.x),
            mirrored,
        )
    }

    pub fn figma_corner_signed_distance(
        corner_to_point: Vec2f,
        params: FigmaCornerParams,
    ) -> SignedDistanceSample {
        let z = corner_to_point + vec2f(params.horizontal.p, params.vertical.p);

        if params.radius <= FIGMA_EPSILON || z.x <= 0.0 || z.y <= 0.0 {
            return SignedDistanceSample {
                distance: max(corner_to_point.x, corner_to_point.y),
                normal: select(
                    vec2f(0.0, 1.0),
                    vec2f(1.0, 0.0),
                    corner_to_point.x > corner_to_point.y,
                ),
                path_t: 0.0,
                segment: FIGMA_SEGMENT_STRAIGHT,
            };
        }

        let horizontal_point = vec2f(z.x, params.vertical.p - z.y);
        let vertical_point = vec2f(z.y, params.horizontal.p - z.x);
        let circle_center = vec2f(params.horizontal.p - params.radius, params.radius);
        let join = vec2f(
            params.horizontal.a + params.horizontal.b + params.c,
            params.d,
        );
        let start_direction = (join - circle_center) / params.radius;
        let to_point = horizontal_point - circle_center;
        let to_point_length = length(to_point);
        let point_direction = select(
            start_direction,
            to_point / max(to_point_length, FIGMA_EPSILON),
            to_point_length > FIGMA_EPSILON,
        );

        let arc_angle = clamp(
            atan2(
                figma_cross_2d(start_direction, point_direction),
                dot(start_direction, point_direction),
            ),
            0.0,
            params.arc_sweep,
        );
        let arc_sine = sin(arc_angle);
        let arc_cosine = cos(arc_angle);
        let arc_normal = vec2f(
            arc_cosine * start_direction.x - arc_sine * start_direction.y,
            arc_sine * start_direction.x + arc_cosine * start_direction.y,
        );
        let arc_point = circle_center + params.radius * arc_normal;
        let arc_delta = horizontal_point - arc_point;
        let arc_t = select(
            0.0,
            arc_angle / max(params.arc_sweep, FIGMA_EPSILON),
            params.arc_sweep > FIGMA_EPSILON,
        );

        let mut sample = SignedDistanceSample {
            distance: figma_signed_distance(arc_delta, arc_normal, length(arc_delta)),
            normal: figma_unfold_normal(arc_normal, false),
            path_t: arc_t,
            segment: FIGMA_SEGMENT_ARC,
        };

        if params.smoothing > FIGMA_EPSILON {
            let horizontal_bounds_delta = max(
                vec2f(0.0, 0.0),
                max(-horizontal_point, horizontal_point - join),
            );

            if dot(horizontal_bounds_delta, horizontal_bounds_delta)
                <= sample.distance * sample.distance * 1.000001 + FIGMA_EPSILON
            {
                let cubic =
                    closest_figma_cubic(horizontal_point, params.horizontal, params.c, params.d);
                let normal = normalize(vec2f(cubic.tangent.y, -cubic.tangent.x));
                let distance =
                    figma_signed_distance(horizontal_point - cubic.point, normal, cubic.distance);

                if abs(distance) <= abs(sample.distance) {
                    sample.distance = distance;
                    sample.normal = figma_unfold_normal(normal, false);
                    sample.path_t = cubic.path_t;
                    sample.segment = FIGMA_SEGMENT_FIRST_CUBIC;
                }
            }

            let vertical_join = vec2f(params.vertical.a + params.vertical.b + params.c, params.d);
            let vertical_bounds_delta = max(
                vec2f(0.0, 0.0),
                max(-vertical_point, vertical_point - vertical_join),
            );

            if dot(vertical_bounds_delta, vertical_bounds_delta)
                <= sample.distance * sample.distance * 1.000001 + FIGMA_EPSILON
            {
                let cubic =
                    closest_figma_cubic(vertical_point, params.vertical, params.c, params.d);
                let normal = normalize(vec2f(cubic.tangent.y, -cubic.tangent.x));
                let distance =
                    figma_signed_distance(vertical_point - cubic.point, normal, cubic.distance);
                if abs(distance) <= abs(sample.distance) {
                    sample.distance = distance;
                    sample.normal = figma_unfold_normal(normal, true);
                    sample.path_t = cubic.path_t;
                    sample.segment = FIGMA_SEGMENT_SECOND_CUBIC;
                }
            }
        }

        sample
    }

    pub fn figma_cubic_length(params: FigmaCornerParams, axis: FigmaAxisParams, end_t: f32) -> f32 {
        let half_t = clamp(end_t, 0.0, 1.0) / 2.0;
        if half_t <= 0.0 || params.smoothing <= 0.0 {
            return 0.0;
        }

        let center = half_t;
        let offset1 = half_t * 0.5384693101;
        let offset2 = half_t * 0.9061798459;
        let speed0 = length(figma_cubic_derivative(axis, params.c, params.d, center));
        let speed1 = length(figma_cubic_derivative(
            axis,
            params.c,
            params.d,
            center - offset1,
        )) + length(figma_cubic_derivative(
            axis,
            params.c,
            params.d,
            center + offset1,
        ));
        let speed2 = length(figma_cubic_derivative(
            axis,
            params.c,
            params.d,
            center - offset2,
        )) + length(figma_cubic_derivative(
            axis,
            params.c,
            params.d,
            center + offset2,
        ));

        half_t * (0.5688888889 * speed0 + 0.4786286705 * speed1 + 0.2369268851 * speed2)
    }

    pub fn figma_corner_length(params: FigmaCornerParams) -> f32 {
        figma_cubic_length(params, params.horizontal, 1.0)
            + params.radius * params.arc_sweep
            + figma_cubic_length(params, params.vertical, 1.0)
    }

    pub fn figma_corner_progress(
        params: FigmaCornerParams,
        sample: SignedDistanceSample,
        total_length: f32,
    ) -> f32 {
        if sample.segment == FIGMA_SEGMENT_FIRST_CUBIC {
            return figma_cubic_length(params, params.horizontal, sample.path_t);
        }

        if sample.segment == FIGMA_SEGMENT_ARC {
            return figma_cubic_length(params, params.horizontal, 1.0)
                + params.radius * params.arc_sweep * sample.path_t;
        }

        if sample.segment == FIGMA_SEGMENT_SECOND_CUBIC {
            return total_length - figma_cubic_length(params, params.vertical, sample.path_t);
        }

        0.0
    }

    #[rustfmt::skip]
    pub fn figma_is_corner_candidate(
        point: Vec2f,
        size: Vec2f,
        horizontal_extent: f32,
        vertical_extent: f32,
        corner: u32,
    ) -> bool {
        if horizontal_extent <= FIGMA_EPSILON || vertical_extent <= FIGMA_EPSILON {
            return false;
        }

        match corner {
            0u32 => { point.x <= horizontal_extent && point.y <= vertical_extent },
            1u32 => { size.x - point.x <= horizontal_extent && point.y <= vertical_extent },
            2u32 => { size.x - point.x <= horizontal_extent && size.y - point.y <= vertical_extent },
            _ => { point.x <= horizontal_extent && size.y - point.y <= vertical_extent },
        }
    }

    pub fn figma_has_corner_candidate(
        point: Vec2f,
        size: Vec2f,
        horizontal_extents: Vec4f,
        vertical_extents: Vec4f,
    ) -> bool {
        figma_is_corner_candidate(point, size, horizontal_extents.x, vertical_extents.x, 0u32)
            || figma_is_corner_candidate(
                point,
                size,
                horizontal_extents.y,
                vertical_extents.y,
                1u32,
            )
            || figma_is_corner_candidate(
                point,
                size,
                horizontal_extents.z,
                vertical_extents.z,
                2u32,
            )
            || figma_is_corner_candidate(
                point,
                size,
                horizontal_extents.w,
                vertical_extents.w,
                3u32,
            )
    }

    #[rustfmt::skip]
    pub fn figma_corner_to_point(point: Vec2f, size: Vec2f, corner: u32) -> Vec2f {
        match corner {
            0u32 => { -point },
            1u32 => { vec2f(point.x - size.x, -point.y) },
            2u32 => { point - size },
            _ => { vec2f(-point.x, point.y - size.y) },
        }
    }

    #[rustfmt::skip]
    pub fn figma_orient_corner_normal(normal: Vec2f, corner: u32) -> Vec2f {
        match corner {
            0u32 => { -normal },
            1u32 => { vec2f(normal.x, -normal.y) },
            2u32 => { normal },
            _ => { vec2f(-normal.x, normal.y) },
        }
    }

    pub fn figma_nearest_straight_sample(
        point: Vec2f,
        size: Vec2f,
        horizontal_extents: Vec4f,
        vertical_extents: Vec4f,
    ) -> SignedDistanceSample {
        let mut nearest_delta = point
            - vec2f(
                clamp(
                    point.x,
                    horizontal_extents.x,
                    max(horizontal_extents.x, size.x - horizontal_extents.y),
                ),
                0.0,
            );
        let mut nearest_normal = vec2f(0.0, -1.0);
        let mut nearest_distance_squared = dot(nearest_delta, nearest_delta);
        let mut candidate_delta = point
            - vec2f(
                size.x,
                clamp(
                    point.y,
                    vertical_extents.y,
                    max(vertical_extents.y, size.y - vertical_extents.z),
                ),
            );

        let mut candidate_distance_squared = dot(candidate_delta, candidate_delta);
        if candidate_distance_squared < nearest_distance_squared {
            nearest_delta = candidate_delta;
            nearest_normal = vec2f(1.0, 0.0);
            nearest_distance_squared = candidate_distance_squared;
        }

        candidate_delta = point
            - vec2f(
                clamp(
                    point.x,
                    horizontal_extents.w,
                    max(horizontal_extents.w, size.x - horizontal_extents.z),
                ),
                size.y,
            );

        candidate_distance_squared = dot(candidate_delta, candidate_delta);
        if candidate_distance_squared < nearest_distance_squared {
            nearest_delta = candidate_delta;
            nearest_normal = vec2f(0.0, 1.0);
            nearest_distance_squared = candidate_distance_squared;
        }

        candidate_delta = point
            - vec2f(
                0.0,
                clamp(
                    point.y,
                    vertical_extents.x,
                    max(vertical_extents.x, size.y - vertical_extents.w),
                ),
            );

        candidate_distance_squared = dot(candidate_delta, candidate_delta);
        if candidate_distance_squared < nearest_distance_squared {
            nearest_delta = candidate_delta;
            nearest_normal = vec2f(-1.0, 0.0);
            nearest_distance_squared = candidate_distance_squared;
        }

        SignedDistanceSample {
            distance: figma_signed_distance(
                nearest_delta,
                nearest_normal,
                sqrt(nearest_distance_squared),
            ),
            normal: nearest_normal,
            path_t: 0.0,
            segment: FIGMA_SEGMENT_STRAIGHT,
        }
    }

    pub fn figma_smooth_rectangle_sample(
        point: Vec2f,
        bounds: Bounds,
        corner_radii: Corners,
        horizontal_reaches: Vec4f,
        vertical_reaches: Vec4f,
        smoothing_factors: Vec4f,
    ) -> FigmaRectangleSample {
        let local_point = point - bounds.origin;
        let radii = corner_values(corner_radii);
        let mut result = FigmaRectangleSample {
            signed_distance: figma_nearest_straight_sample(
                local_point,
                bounds.size,
                horizontal_reaches,
                vertical_reaches,
            ),
            corner: FIGMA_NO_CORNER,
        };

        if !figma_has_corner_candidate(
            local_point,
            bounds.size,
            horizontal_reaches,
            vertical_reaches,
        ) {
            return result;
        }

        let mut corner = 0u32;
        while corner < 4u32 {
            if figma_is_corner_candidate(
                local_point,
                bounds.size,
                horizontal_reaches[corner],
                vertical_reaches[corner],
                corner,
            ) {
                let params = figma_corner_params(
                    radii[corner],
                    horizontal_reaches[corner],
                    vertical_reaches[corner],
                    smoothing_factors,
                );

                if params.radius > FIGMA_EPSILON {
                    let mut candidate = figma_corner_signed_distance(
                        figma_corner_to_point(local_point, bounds.size, corner),
                        params,
                    );
                    candidate.normal = figma_orient_corner_normal(candidate.normal, corner);

                    if abs(candidate.distance) <= abs(result.signed_distance.distance) {
                        result.signed_distance = candidate;
                        result.corner = corner;
                    }
                }
            }

            corner += 1u32;
        }

        result
    }

    pub fn prepared_corner_signed_distance(
        point: Vec2f,
        bounds: Bounds,
        corner_radii: Corners,
        corner_smoothing: f32,
        prepared: PreparedCorners,
    ) -> f32 {
        if prepared.superellipse_power > 0.0 {
            return normalized_superellipse_signed_distance(
                point,
                bounds,
                corner_radii,
                corner_smoothing,
                prepared.horizontal_reaches,
                prepared.superellipse_power,
            );
        }

        if prepared.smoothing_factors.x <= 0.0 {
            return rounded_rectangle_signed_distance(point, bounds, corner_radii);
        }

        figma_smooth_rectangle_sample(
            point,
            bounds,
            corner_radii,
            prepared.horizontal_reaches,
            prepared.vertical_reaches,
            prepared.smoothing_factors,
        )
        .signed_distance
        .distance
    }
}

pub use source::*;

#[cfg(test)]
mod tests {
    use super::super::common::*;
    use super::*;
    use wgsl_rs::std::{Vec2f, vec2f};

    fn corners([top_left, top_right, bottom_right, bottom_left]: [f32; 4]) -> Corners {
        Corners {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    fn distance(
        point: Vec2f,
        bounds: Bounds,
        radii: Corners,
        smoothing: f32,
        allow_superellipse: bool,
    ) -> f32 {
        let prepared = prepare_corners(bounds.size, radii, smoothing, allow_superellipse);

        prepared_corner_signed_distance(point, bounds, radii, smoothing, prepared)
    }

    #[test]
    fn smoothed_corner_geometry_preserves_core_invariants() {
        let bounds = Bounds {
            origin: vec2f(0.0, 0.0),
            size: vec2f(120.0, 80.0),
        };
        let cases = [
            corners([20.0, 20.0, 20.0, 20.0]),
            corners([28.0, 8.0, 22.0, 2.0]),
            corners([70.0, 45.0, 60.0, 35.0]),
            corners([0.0, 0.0, 0.0, 0.0]),
        ];

        for radii in cases {
            for smoothing in [0.0, 0.6, 1.0] {
                for y in -1..=9 {
                    for x in -1..=13 {
                        let point = vec2f(x as f32 * 10.0, y as f32 * 10.0);
                        let actual = distance(point, bounds, radii, smoothing, false);
                        assert!(actual.is_finite(), "non-finite SDF at {point:?}");

                        if smoothing == 0.0 {
                            let circular = rounded_rectangle_signed_distance(point, bounds, radii);
                            assert_eq!(actual, circular);
                        }
                    }
                }

                assert!(distance(vec2f(60.0, 40.0), bounds, radii, smoothing, false) < 0.0);
                assert!(distance(vec2f(-2.0, -2.0), bounds, radii, smoothing, false) > 0.0);
            }
        }

        let square = Bounds {
            origin: vec2f(0.0, 0.0),
            size: vec2f(100.0, 100.0),
        };
        let uniform = corners([20.0, 20.0, 20.0, 20.0]);

        for smoothing in [0.0, 0.6, 1.0] {
            let a = distance(vec2f(8.0, 17.0), square, uniform, smoothing, true);
            let b = distance(vec2f(83.0, 8.0), square, uniform, smoothing, true);
            let c = distance(vec2f(92.0, 83.0), square, uniform, smoothing, true);
            let d = distance(vec2f(17.0, 92.0), square, uniform, smoothing, true);
            assert!(
                [b, c, d]
                    .into_iter()
                    .all(|value| (a - value).abs() < 0.0001)
            );

            let diagonal = 20.0 * SUPERELLIPSE_DIAGONAL_INSET;
            let diagonal_distance =
                distance(vec2f(diagonal, diagonal), square, uniform, smoothing, true);
            assert!(diagonal_distance.abs() < 0.0001);
        }

        let circular = prepare_corners(square.size, uniform, 0.0, false);
        let smoothed = prepare_corners(square.size, uniform, 0.6, false);
        assert_eq!(circular.horizontal_reaches.x, 20.0);
        assert!(smoothed.horizontal_reaches.x > circular.horizontal_reaches.x);
        assert!(smoothed.vertical_reaches.x > circular.vertical_reaches.x);
    }

    #[test]
    fn normalized_shortcut_checks_adjacent_corner_overlap_boundaries() {
        let size = vec2f(100.0, 80.0);

        for (side, base, corner_idx) in [
            ("top", [20.0, 30.0, 0.0, 0.0], 1),
            ("right", [0.0, 20.0, 20.0, 0.0], 2),
            ("bottom", [0.0, 0.0, 30.0, 20.0], 3),
            ("left", [20.0, 0.0, 0.0, 20.0], 3),
        ] {
            for (offset, expected) in [(-0.001, true), (0.0, true), (0.001, false)] {
                let mut values = base;
                values[corner_idx] += offset;

                assert_eq!(
                    can_use_normalized_superellipse(size, corners(values), 1.0),
                    expected,
                    "{side}: offset {offset}",
                );
            }
        }

        let preservation = prepare_corners(size, corners([20.0, 30.0, 0.0, 0.0]), 1.0, false);
        assert_eq!(preservation.superellipse_power, 0.0);
    }

    #[test]
    fn normalized_shortcut_selects_the_correct_corner_region() {
        let bounds = Bounds {
            origin: vec2f(0.0, 0.0),
            size: vec2f(120.0, 80.0),
        };

        for (name, radii, smoothing, compact) in [
            ("compact", corners([20.0, 8.0, 16.0, 4.0]), 0.6, true),
            ("reach-aware", corners([35.0, 0.0, 0.0, 0.0]), 1.0, false),
        ] {
            let prepared = prepare_corners(bounds.size, radii, smoothing, true);
            assert!(prepared.superellipse_power > 0.0, "{name}: shortcut");
            assert_eq!(
                can_use_compact_corner_selection(bounds.size, prepared.horizontal_reaches),
                compact,
                "{name}: corner selection",
            );

            if compact {
                let points = (0..=8)
                    .flat_map(|y| (0..=12).map(move |x| vec2f(x as f32 * 10.0, y as f32 * 10.0)));

                for point in points {
                    let expected = compact_normalized_superellipse_signed_distance(
                        point,
                        bounds,
                        radii,
                        smoothing,
                        prepared.superellipse_power,
                    );
                    let actual =
                        prepared_corner_signed_distance(point, bounds, radii, smoothing, prepared);
                    assert_eq!(actual, expected, "{name}: distance at {point:?}");
                }
            } else {
                let shoulder = vec2f(65.0, 0.0);
                let expected = normalized_superellipse_signed_distance_from_corner(
                    -shoulder,
                    radii.top_left,
                    smoothing,
                    prepared.superellipse_power,
                );
                let actual =
                    prepared_corner_signed_distance(shoulder, bounds, radii, smoothing, prepared);
                assert!(
                    (actual - expected).abs() < 0.0001,
                    "{name}: shoulder distance"
                );
                assert!(actual > 0.0, "{name}: shoulder outside");

                let deep_interior = prepared_corner_signed_distance(
                    vec2f(65.0, 65.0),
                    bounds,
                    radii,
                    smoothing,
                    prepared,
                );
                assert!(
                    (deep_interior + 15.0).abs() < 0.0001,
                    "{name}: interior distance"
                );
            }
        }
    }
}
