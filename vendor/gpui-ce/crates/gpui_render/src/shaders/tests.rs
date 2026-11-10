use super::*;

#[test]
fn validates_subpixel_shader() {
    let generated = subpixel_sprite::WGSL_SOURCE.wgsl_source().unwrap();
    wgsl_rs::validate_wgsl_source(&format!("enable dual_source_blending;\n{generated}")).unwrap();
}

#[test]
fn build_generated_shaders_match_rust_sources() {
    assert_eq!(
        base::WGSL_SOURCE.wgsl_source().unwrap(),
        crate::artifacts::BASE_WGSL
    );
    let subpixel = subpixel_sprite::WGSL_SOURCE.wgsl_source().unwrap();
    assert_eq!(
        format!("enable dual_source_blending;\n{subpixel}"),
        crate::artifacts::SUBPIXEL_DUAL_SOURCE_WGSL
    );
}

#[test]
fn shader_interface_matches_generated_sources() {
    let standard_source = base::WGSL_SOURCE.wgsl_source().unwrap();
    let subpixel_source = subpixel_sprite::WGSL_SOURCE.wgsl_source().unwrap();
    for pipeline in interface::ALL {
        let source = if pipeline.label == interface::SUBPIXEL_SPRITES.label {
            &subpixel_source
        } else {
            &standard_source
        };
        assert!(
            source.contains(&format!("fn {}(", pipeline.vertex_entry)),
            "missing vertex entry point {}",
            pipeline.vertex_entry,
        );
        assert!(
            source.contains(&format!("fn {}(", pipeline.fragment_entry)),
            "missing fragment entry point {}",
            pipeline.fragment_entry,
        );
    }

    assert_eq!(common::GLOBALS.group, interface::GLOBAL_BIND_GROUP);
    assert_eq!(common::GLOBALS.binding, interface::GLOBAL_UNIFORMS_BINDING);
    assert_eq!(
        common::FONT_RASTERIZATION.group,
        interface::GLOBAL_BIND_GROUP
    );
    assert_eq!(
        common::FONT_RASTERIZATION.binding,
        interface::FONT_RASTERIZATION_BINDING
    );
    assert_eq!(quad::QUADS.group(), interface::DATA_BIND_GROUP);
    assert_eq!(quad::QUADS.binding(), interface::DATA_BUFFER_BINDING);
}

fn vertex_output_shape(module: &naga::Module, entry_name: &str) -> (usize, u32) {
    let entry = module
        .entry_points
        .iter()
        .find(|entry| entry.name == entry_name)
        .unwrap_or_else(|| panic!("missing shader entry point {entry_name}"));
    let result = entry
        .function
        .result
        .as_ref()
        .unwrap_or_else(|| panic!("shader entry point {entry_name} has no output"));
    let naga::TypeInner::Struct { members, .. } = &module.types[result.ty].inner else {
        panic!("shader entry point {entry_name} must return a struct");
    };

    members
        .iter()
        .filter(|member| matches!(member.binding, Some(naga::Binding::Location { .. })))
        .fold((0, 0), |(locations, components), member| {
            let member_components = match module.types[member.ty].inner {
                naga::TypeInner::Scalar(_) => 1,
                naga::TypeInner::Vector { size, .. } => u32::from(size),
                ref ty => panic!("unsupported stage output type {ty:?} in {entry_name}"),
            };
            (locations + 1, components + member_components)
        })
}

#[test]
fn ordinary_vertex_interfaces_stay_within_compact_budgets() {
    let module = naga::front::wgsl::parse_str(&base::WGSL_SOURCE.wgsl_source().unwrap()).unwrap();
    for (entry, maximum_shape) in [
        ("vertex_quad", (8, 29)),
        ("vertex_shadow", (5, 17)),
        ("vertex_polychrome_sprite", (4, 9)),
        ("vertex_blur_composite", (2, 6)),
    ] {
        let actual_shape = vertex_output_shape(&module, entry);
        assert!(
            actual_shape.0 <= maximum_shape.0 && actual_shape.1 <= maximum_shape.1,
            "{entry} uses {actual_shape:?}, exceeding {maximum_shape:?} (locations, components)"
        );
    }
}

#[test]
fn scene_instance_storage_sizes_are_bounded() {
    for (name, actual, maximum) in [
        ("Quad", std::mem::size_of::<gpui::Quad>(), 248),
        ("Shadow", std::mem::size_of::<gpui::Shadow>(), 184),
        (
            "PolychromeSprite",
            std::mem::size_of::<gpui::PolychromeSprite>(),
            136,
        ),
    ] {
        assert!(
            actual <= maximum,
            "{name} is {actual} bytes, exceeding its {maximum}-byte storage budget"
        );
    }
}

#[test]
fn shader_buffer_layouts_match_host_layouts() {
    let modules = [
        crate::artifacts::BASE_WGSL,
        crate::artifacts::SUBPIXEL_DUAL_SOURCE_WGSL,
    ]
    .map(|source| naga::front::wgsl::parse_str(source).unwrap());

    for layouts in [gpui::SCENE_BUFFER_LAYOUTS, interface::RENDER_BUFFER_LAYOUTS] {
        for layout in layouts {
            let ty = modules
                .iter()
                .find_map(|module| {
                    module
                        .types
                        .iter()
                        .find_map(|(_, ty)| (ty.name.as_deref() == Some(layout.name)).then_some(ty))
                })
                .unwrap_or_else(|| panic!("missing shader ABI type {}", layout.name));
            let naga::TypeInner::Struct { members, span } = &ty.inner else {
                panic!("shader ABI type {} must be a struct", layout.name);
            };

            assert_eq!(
                *span as usize, layout.size,
                "shader ABI size: {}",
                layout.name
            );
            assert_eq!(
                members.len(),
                layout.fields.len(),
                "shader ABI fields: {}",
                layout.name
            );
            for (member, (name, offset)) in members.iter().zip(layout.fields) {
                assert_eq!(
                    member.name.as_deref(),
                    Some(*name),
                    "shader ABI field: {}",
                    layout.name
                );
                assert_eq!(
                    member.offset as usize, *offset,
                    "shader ABI offset: {}.{name}",
                    layout.name
                );
            }
        }
    }
}

#[test]
fn shader_discriminants_match_scene_types() {
    assert_eq!(common::ShaderBool::Disabled as u32, 0);
    assert_eq!(common::ShaderBool::Enabled as u32, 1);
    assert_eq!(
        common::BackgroundTag::Solid as u32,
        gpui::BackgroundTag::Solid as u32
    );
    assert_eq!(
        common::BackgroundTag::LinearGradient as u32,
        gpui::BackgroundTag::LinearGradient as u32
    );
    assert_eq!(
        common::BackgroundTag::PatternSlash as u32,
        gpui::BackgroundTag::PatternSlash as u32
    );
    assert_eq!(
        common::BackgroundTag::Checkerboard as u32,
        gpui::BackgroundTag::Checkerboard as u32
    );
    assert_eq!(
        common::ColorSpace::Srgb as u32,
        gpui::ColorSpace::Srgb as u32
    );
    assert_eq!(
        common::ColorSpace::Oklab as u32,
        gpui::ColorSpace::Oklab as u32
    );
    assert_eq!(
        common::BorderStyle::Solid as u32,
        gpui::BorderStyle::Solid as u32
    );
    assert_eq!(
        common::BorderStyle::Dashed as u32,
        gpui::BorderStyle::Dashed as u32
    );
}

#[test]
fn linear_gradients_preserve_native_dithering() {
    use common::*;
    use wgsl_rs::std::*;

    let color = Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.5,
        a: 0.5,
    };
    let background = Background {
        tag: BackgroundTag::LinearGradient,
        color_space: ColorSpace::Srgb,
        solid: color,
        gradient_angle_or_pattern_height: 90.0,
        colors: [
            LinearColorStop {
                color,
                percentage: 0.0,
            },
            LinearColorStop {
                color,
                percentage: 1.0,
            },
        ],
        padding: 0,
    };
    let bounds = Bounds {
        origin: vec2f(0.0, 0.0),
        size: vec2f(100.0, 100.0),
    };
    let paint = Paint::new(background, bounds);
    let prepared = prepare_paint(paint);
    let first = paint_color(paint, vec2f(10.0, 10.0), prepared);
    let second = paint_color(paint, vec2f(11.0, 10.0), prepared);

    assert_ne!(first.w, second.w);
}

fn shadow_clip_fixture(inset: bool, blur: f32, smoothing: f32) -> shadow::Shadow {
    use common::*;
    use wgsl_rs::std::vec2f;
    let element_bounds = Bounds {
        origin: vec2f(20.0, 20.0),
        size: vec2f(100.0, 60.0),
    };
    let corners = Corners {
        top_left: 8.0,
        top_right: 8.0,
        bottom_right: 8.0,
        bottom_left: 8.0,
    };
    let clear = Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.0,
    };
    shadow::Shadow {
        order: 0,
        blur_radius: blur,
        bounds: if inset {
            Bounds {
                origin: vec2f(21.0, 21.0),
                size: vec2f(98.0, 58.0),
            }
        } else {
            Bounds {
                origin: vec2f(19.0, 19.0),
                size: vec2f(102.0, 62.0),
            }
        },
        corner_radii: corners,
        content_mask: ContentMask {
            bounds: element_bounds,
            fade_out: Edges {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
        },
        color: Background {
            tag: BackgroundTag::Solid,
            color_space: ColorSpace::Srgb,
            solid: clear,
            gradient_angle_or_pattern_height: 0.0,
            colors: [LinearColorStop {
                color: clear,
                percentage: 0.0,
            }; 2],
            padding: 0,
        },
        element_bounds,
        element_corner_radii: corners,
        inset: if inset {
            ShaderBool::Enabled
        } else {
            ShaderBool::Disabled
        },
        corner_smoothing: smoothing,
    }
}

#[test]
fn outer_shadows_leave_transparent_interiors_clear_and_preserve_external_edges() {
    use corner_smoothing::prepare_corners;
    use wgsl_rs::std::vec2f;
    for blur in [0.0, 0.5, 3.0, 42.5] {
        for smoothing in [0.0, 0.6] {
            let shadow = shadow_clip_fixture(false, blur, smoothing);
            let coverage = |position| {
                if smoothing == 0.0 {
                    shadow::shadow_coverage(shadow, position)
                } else {
                    shadow::smoothed_shadow_coverage(
                        shadow,
                        position,
                        prepare_corners(shadow.bounds.size, shadow.corner_radii, smoothing, false),
                        prepare_corners(
                            shadow.element_bounds.size,
                            shadow.element_corner_radii,
                            smoothing,
                            false,
                        ),
                    )
                }
            };
            assert_eq!(
                coverage(vec2f(70.0, 50.0)),
                0.0,
                "blur={blur}, smoothing={smoothing}"
            );
            assert_eq!(coverage(vec2f(21.0, 50.0)), 0.0);
            assert!(coverage(vec2f(19.5, 50.0)) > 0.0);
            // The rounded corner is outside the element, despite being in its AABB.
            if blur > 0.0 {
                assert!(coverage(vec2f(20.5, 20.5)) > 0.0);
            }
        }
    }
}

#[test]
fn inset_shadow_clip_preserves_the_inner_edge() {
    use wgsl_rs::std::vec2f;
    let shadow = shadow_clip_fixture(true, 0.0, 0.0);
    assert_eq!(shadow::shadow_coverage(shadow, vec2f(70.0, 50.0)), 0.0);
    assert_eq!(shadow::shadow_coverage(shadow, vec2f(19.5, 50.0)), 0.0);
    assert!(shadow::shadow_coverage(shadow, vec2f(20.5, 50.0)) > 0.0);
}
