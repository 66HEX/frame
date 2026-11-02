#[wgsl_rs::wgsl]
pub mod path_rasterization {
    use super::super::common::*;
    use wgsl_rs::std::*;

    #[derive(Clone, Copy, Wgsl)]
    pub struct PathRasterizationVertex {
        pub xy_position: Vec2f,
        pub curve_position: Vec2f,
        pub color: Background,
        pub bounds: Bounds,
    }
    storage!(group(1), binding(0), PATH_VERTICES: RuntimeArray<PathRasterizationVertex>);

    pub fn quadratic_bezier_coverage(curve_position: Vec2f) -> f32 {
        let horizontal_derivative = dpdx(curve_position);
        let vertical_derivative = dpdy(curve_position);
        let horizontal_gradient = vec2f(horizontal_derivative.x, vertical_derivative.x);
        if length(horizontal_gradient) < MIN_PATH_GRADIENT {
            return 1.0;
        }

        let implicit_gradient = 2.0 * curve_position.xx() * horizontal_gradient
            - vec2f(horizontal_derivative.y, vertical_derivative.y);
        let implicit_value = curve_position.x * curve_position.x - curve_position.y;
        antialiased_coverage(implicit_value / length(implicit_gradient))
    }

    #[derive(Wgsl)]
    pub struct PathRasterizationVarying {
        #[builtin(position)]
        pub position: Vec4f,
        #[location(0)]
        pub curve_position: Vec2f,
        #[location(1)]
        #[interpolate(flat)]
        pub vertex_id: u32,
        #[location(3)]
        pub clip_distances: Vec4f,
    }

    #[vertex]
    pub fn vertex_path_rasterization(
        #[builtin(vertex_index)] vertex_id: u32,
    ) -> PathRasterizationVarying {
        let vertex = get!(PATH_VERTICES)[vertex_id as usize];
        PathRasterizationVarying {
            position: viewport_to_clip_position(vertex.xy_position),
            curve_position: vertex.curve_position,
            vertex_id,
            clip_distances: clip_distances(vertex.xy_position, vertex.bounds),
        }
    }

    #[fragment]
    pub fn fragment_path_rasterization(input: PathRasterizationVarying) -> Vec4f {
        let coverage = quadratic_bezier_coverage(input.curve_position);
        if is_clipped(input.clip_distances) {
            return transparent();
        }
        let vertex = get!(PATH_VERTICES)[input.vertex_id as usize];
        let paint = Paint::new(vertex.color, vertex.bounds);
        let color = paint_color(paint, input.position.xy(), prepare_paint(paint));
        premultiply(color, coverage)
    }
}

#[wgsl_rs::wgsl]
pub mod path {
    use super::super::common::*;
    use wgsl_rs::std::*;

    #[derive(Clone, Copy, Wgsl)]
    pub struct PathSprite {
        pub bounds: Bounds,
    }
    storage!(group(1), binding(0), PATH_SPRITES: RuntimeArray<PathSprite>);
    texture!(group(1), binding(1), PATH_TEXTURE: Texture2D<f32>);
    sampler!(group(1), binding(2), PATH_SAMPLER: Sampler);

    #[derive(Wgsl)]
    pub struct PathVarying {
        #[builtin(position)]
        pub position: Vec4f,
        #[location(0)]
        pub texture_coords: Vec2f,
    }

    #[vertex]
    pub fn vertex_path(
        #[builtin(vertex_index)] vertex_id: u32,
        #[builtin(instance_index)] instance_id: u32,
    ) -> PathVarying {
        let sprite = get!(PATH_SPRITES)[instance_id as usize];
        let vertex = rectangle_vertex(vertex_id, sprite.bounds);
        PathVarying {
            position: vertex.clip_position,
            texture_coords: vertex.viewport_position / get!(GLOBALS).viewport_size,
        }
    }

    #[fragment]
    pub fn fragment_path(input: PathVarying) -> Vec4f {
        texture_sample(PATH_TEXTURE, PATH_SAMPLER, input.texture_coords)
    }
}
