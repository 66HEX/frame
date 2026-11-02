//! Build-time shader pipeline.
//!
//! Compiles the Rust-authored shaders in `src/shaders` into one artifact per dialect
//! (WebGpu WGSL, downlevel WGSL for WebGL2/GLES, HLSL/DXBC, GLSL, and MSL) and validates each
//! against the Naga versions its consumers require. Dialect gaps close via mechanical
//! transforms here, never per-backend hand edits.

#[path = "src/path_types.rs"]
#[allow(dead_code)]
mod path_types;
#[path = "src/shaders/mod.rs"]
mod shaders;

use std::{collections::BTreeSet, env, fmt::Write as _, fs, path::PathBuf};

#[cfg(windows)]
use std::ffi::CString;

/// Width, in texels, of the downlevel scene-data texture; the runtime must match.
const DATA_TEXTURE_WIDTH: u32 = 1024;
/// Group-1 binding carrying the per-draw batch base for the downlevel transport.
const DOWNLEVEL_RANGE_BINDING: u32 = 4;

#[derive(Clone, Copy)]
enum NagaCompatibility {
    Current,
    CurrentAndFloor,
}

struct ShaderArtifact {
    file_name: &'static str,
    source: String,
    compatibility: NagaCompatibility,
}

impl ShaderArtifact {
    fn modern(file_name: &'static str, source: String) -> Self {
        Self {
            file_name,
            source,
            compatibility: NagaCompatibility::CurrentAndFloor,
        }
    }

    fn current_only(file_name: &'static str, source: String) -> Self {
        Self {
            file_name,
            source,
            compatibility: NagaCompatibility::Current,
        }
    }

    fn downlevel(file_name: &'static str, source: String) -> Self {
        Self::modern(file_name, source)
    }

    fn validate_and_write(&self, out_dir: &std::path::Path) {
        validate_artifact(self);
        write_shader(out_dir, self.file_name, &self.source);
    }
}

#[derive(Clone, Copy)]
enum ReflectionDialect {
    Modern,
    Downlevel,
}

struct BindingLayout<'a> {
    constant: &'static str,
    source: &'a str,
    group: u32,
    dialect: ReflectionDialect,
}

struct NativeShaderModule {
    pipeline: &'static shaders::interface::Pipeline,
    /// Path of `pipeline` inside `crate::shaders::interface`, for the generated table.
    pipeline_path: &'static str,
    source: &'static wgsl_rs::Source,
    requires_dual_source_lowering: bool,
}

struct StorageArray {
    name: String,
    element_type: String,
}

fn main() {
    println!("cargo:rerun-if-changed=src/shaders");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));

    // --- WebGpu dialect (modern targets) ------------------------------------------------
    let base = ShaderArtifact::modern(
        "gpui_base.wgsl",
        shader_source("gpui_base.wgsl", &shaders::base::WGSL_SOURCE),
    );
    let subpixel = shader_source("gpui_subpixel.wgsl", &shaders::subpixel_sprite::WGSL_SOURCE);
    let subpixel_dual_source = ShaderArtifact::current_only(
        "gpui_subpixel_dual_source.wgsl",
        format!("enable dual_source_blending;\n{subpixel}"),
    );
    // Dual-source blending only exists in current compilers, so that artifact skips the floor.
    base.validate_and_write(&out_dir);
    subpixel_dual_source.validate_and_write(&out_dir);

    // --- Downlevel dialect (WebGL2 / GLES): storage arrays become data-texture loads ----
    let downlevel = ShaderArtifact::downlevel(
        "gpui_base_downlevel.wgsl",
        downlevel_dialect("gpui_base_downlevel.wgsl", &base.source),
    );
    downlevel.validate_and_write(&out_dir);

    let quad = shader_source("quad interface", &shaders::quad::WGSL_SOURCE);
    let monochrome = shader_source(
        "monochrome sprite interface",
        &shaders::monochrome_sprite::WGSL_SOURCE,
    );
    let polychrome = shader_source(
        "polychrome sprite interface",
        &shaders::polychrome_sprite::WGSL_SOURCE,
    );
    let subpixel_interface = format!(
        "enable dual_source_blending;\n{}",
        shader_source(
            "subpixel sprite interface",
            &shaders::subpixel_sprite::WGSL_SOURCE,
        )
    );
    let surface = shader_source("surface interface", &shaders::surface::WGSL_SOURCE);
    let blur = shader_source("blur interface", &shaders::blur::WGSL_SOURCE);
    let downlevel_quad = downlevel_dialect("quad interface", &quad);
    let downlevel_polychrome = downlevel_dialect("polychrome sprite interface", &polychrome);
    let downlevel_surface = downlevel_dialect("surface interface", &surface);
    let downlevel_blur = downlevel_dialect("blur interface", &blur);

    let layouts = [
        BindingLayout {
            constant: "GLOBAL_BINDINGS",
            source: &base.source,
            group: shaders::interface::GLOBAL_BIND_GROUP,
            dialect: ReflectionDialect::Modern,
        },
        BindingLayout {
            constant: "INSTANCE_BINDINGS",
            source: &quad,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Modern,
        },
        BindingLayout {
            constant: "MONOCHROME_INSTANCE_BINDINGS",
            source: &monochrome,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Modern,
        },
        BindingLayout {
            constant: "SUBPIXEL_INSTANCE_BINDINGS",
            source: &subpixel_interface,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Modern,
        },
        BindingLayout {
            constant: "TEXTURED_INSTANCE_BINDINGS",
            source: &polychrome,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Modern,
        },
        BindingLayout {
            constant: "SURFACE_BINDINGS",
            source: &surface,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Modern,
        },
        BindingLayout {
            constant: "BLUR_BINDINGS",
            source: &blur,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Modern,
        },
    ];
    let downlevel_layouts = [
        BindingLayout {
            constant: "DOWNLEVEL_INSTANCE_BINDINGS",
            source: &downlevel_quad,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Downlevel,
        },
        BindingLayout {
            constant: "DOWNLEVEL_TEXTURED_INSTANCE_BINDINGS",
            source: &downlevel_polychrome,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Downlevel,
        },
        BindingLayout {
            constant: "DOWNLEVEL_SURFACE_BINDINGS",
            source: &downlevel_surface,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Downlevel,
        },
        BindingLayout {
            constant: "DOWNLEVEL_BLUR_BINDINGS",
            source: &downlevel_blur,
            group: shaders::interface::DATA_BIND_GROUP,
            dialect: ReflectionDialect::Downlevel,
        },
    ];
    write_interface(&out_dir, &layouts, &downlevel_layouts);
    write_native_shaders(&out_dir);
}

fn shader_source(name: &str, source: &wgsl_rs::Source) -> String {
    source
        .wgsl_source()
        .unwrap_or_else(|error| panic!("failed to generate {name}: {error}"))
}

fn write_shader(out_dir: &std::path::Path, name: &str, source: &str) {
    fs::write(out_dir.join(name), source).unwrap_or_else(|error| {
        panic!("failed to write generated shader {name}: {error}");
    });
}

#[cfg(windows)]
fn write_bytes(out_dir: &std::path::Path, name: &str, bytes: &[u8]) {
    fs::write(out_dir.join(name), bytes).unwrap_or_else(|error| {
        panic!("failed to write generated bytecode {name}: {error}");
    });
}

fn validate_with_current(name: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|error| panic!("current Naga failed to parse {name}: {error}"));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|error| panic!("current Naga failed to validate {name}: {error:?}"));
}

fn validate_with_floor(name: &str, source: &str) {
    let module = naga_old::front::wgsl::parse_str(source)
        .unwrap_or_else(|error| panic!("floor Naga failed to parse {name}: {error}"));
    naga_old::valid::Validator::new(
        naga_old::valid::ValidationFlags::all(),
        naga_old::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|error| panic!("floor Naga failed to validate {name}: {error:?}"));
}

/// Validates with the current Naga always, plus the floor Naga unless current-only.
fn validate_artifact(artifact: &ShaderArtifact) {
    validate_with_current(artifact.file_name, &artifact.source);
    if matches!(artifact.compatibility, NagaCompatibility::CurrentAndFloor) {
        validate_with_floor(artifact.file_name, &artifact.source);
    }
}

/// Lowers modern WGSL to the downlevel dialect: storage arrays become texel loads.
fn downlevel_dialect(name: &str, source: &str) -> String {
    let mut transformed = String::with_capacity(source.len() + 4096);
    let mut arrays = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(array) = parse_storage_array_decl(trimmed) {
            transformed.push_str(&format!(
                "@group(1) @binding(0) var {}_DATA: texture_2d<u32>;\n",
                array.name
            ));
            arrays.push(array);
        } else {
            transformed.push_str(line);
            transformed.push('\n');
        }
    }

    if arrays.is_empty() {
        assert!(
            !source.contains("var<storage"),
            "downlevel transform matched no storage arrays in {name}"
        );
        transformed
    } else {
        for array in &arrays {
            transformed = rewrite_array_accesses(&transformed, &array.name);
        }
        transformed.push_str(&emit_downlevel_runtime(name, source, &arrays));
        transformed
    }
}

fn parse_storage_array_decl(line: &str) -> Option<StorageArray> {
    let rest = line.strip_prefix("@group(1) @binding(0) var<storage, read> ")?;
    let (name, rest) = rest.split_once(": array<")?;
    let type_name = rest
        .strip_suffix(';')?
        .strip_suffix('>')?
        .trim()
        .to_string();
    assert!(
        !type_name.is_empty(),
        "storage array declaration has no element type"
    );
    Some(StorageArray {
        name: name.trim().to_string(),
        element_type: type_name,
    })
}

/// Rewrites `NAME[u32(expr)]` accesses into `dl_load_NAME(expr)` calls.
fn rewrite_array_accesses(source: &str, array_name: &str) -> String {
    let pattern = format!("{array_name}[u32(");
    let mut output = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(position) = rest.find(&pattern) {
        let arg_start = position + pattern.len();
        // Find the matching `)` for the u32( call, then the closing `]`.
        let arg_end = rest[arg_start..]
            .find(')')
            .unwrap_or_else(|| panic!("unbalanced u32( in {array_name} access"));
        let close = rest.get(arg_start + arg_end + 1..=arg_start + arg_end + 1);
        assert_eq!(close, Some("]"), "expected ] after {array_name} access");
        output.push_str(&rest[..position]);
        output.push_str(&format!(
            "dl_load_{array_name}({})",
            &rest[arg_start..arg_start + arg_end]
        ));
        rest = &rest[arg_start + arg_end + 2..];
    }
    output.push_str(rest);
    output
}

/// Emits the data-texture runtime: range uniform, word extractor, typed decoders.
fn emit_downlevel_runtime(name: &str, base_source: &str, arrays: &[StorageArray]) -> String {
    let mut runtime = String::new();
    writeln!(runtime).unwrap();
    writeln!(
        runtime,
        "// --- generated downlevel transport (build.rs) ---"
    )
    .unwrap();
    writeln!(
        runtime,
        "const DATA_TEXTURE_WIDTH: u32 = {DATA_TEXTURE_WIDTH}u;"
    )
    .unwrap();
    writeln!(
        runtime,
        "struct DataRange {{
    base_texel: u32,
    element_count: u32,
    padding0: u32,
    padding1: u32,
}}
@group(1) @binding({DOWNLEVEL_RANGE_BINDING}) var<uniform> DATA_RANGE: DataRange;"
    )
    .unwrap();
    writeln!(
        runtime,
        "fn dl_scene_word(t: texture_2d<u32>, base: u32, w: u32) -> u32 {{
    let linear = base + w;
    let texel = textureLoad(t, vec2<u32>(linear / 4u % DATA_TEXTURE_WIDTH, linear / 4u / DATA_TEXTURE_WIDTH), 0);
    var components = array(texel.x, texel.y, texel.z, texel.w);
    return components[linear % 4u];
}}"
    )
    .unwrap();

    // Parse with the current Naga to drive layout-aware decoder codegen.
    let module = naga::front::wgsl::parse_str(base_source)
        .unwrap_or_else(|error| panic!("failed to parse {name} for decoder codegen: {error}"));
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .unwrap_or_else(|error| panic!("failed to lay out {name}: {error}"));

    let mut emitted: BTreeSet<String> = BTreeSet::new();
    for array in arrays {
        let handle = find_type(&module, &array.element_type, name);
        emit_type_decoder(&module, &layouter, handle, &mut runtime, &mut emitted);
        let words = words_per_element(&layouter, handle);
        writeln!(
            runtime,
            "fn dl_load_{array_name}(i: u32) -> {type_name} {{
    return dl_load_{type_name}_impl({array_name}_DATA, DATA_RANGE.base_texel * 4u + i * {words}u);
}}",
            array_name = array.name,
            type_name = array.element_type,
        )
        .unwrap();
    }
    runtime
}

fn find_type(module: &naga::Module, name: &str, context: &str) -> naga::Handle<naga::Type> {
    module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some(name))
        .map(|(handle, _)| handle)
        .unwrap_or_else(|| panic!("type {name} not found in {context}"))
}

fn words_per_element(layouter: &naga::proc::Layouter, handle: naga::Handle<naga::Type>) -> u32 {
    layouter[handle].to_stride().div_ceil(4)
}

fn emit_type_decoder(
    module: &naga::Module,
    layouter: &naga::proc::Layouter,
    handle: naga::Handle<naga::Type>,
    out: &mut String,
    emitted: &mut BTreeSet<String>,
) {
    let name = module.types[handle]
        .name
        .clone()
        .unwrap_or_else(|| panic!("anonymous types are not supported by the downlevel transport"));
    if !emitted.insert(name.clone()) {
        return;
    }
    // Ensure nested named types are emitted before their users.
    let ty = &module.types[handle];
    let mut members = Vec::new();
    if let naga::TypeInner::Struct { members: list, .. } = &ty.inner {
        for member in list {
            let member_layout = layouter[member.ty];
            // Use Naga's validated member offset rather than reconstructing it
            // from sizes. In particular, array members have a size that covers
            // all elements while their stride is the distance between elements;
            // rebuilding offsets here is easy to get wrong as layouts evolve.
            let offset = member.offset;
            assert_eq!(offset % 4, 0, "struct member is not word aligned");
            if let naga::TypeInner::Struct { .. } = &module.types[member.ty].inner {
                emit_type_decoder(module, layouter, member.ty, out, emitted);
            }
            members.push((member.ty, offset / 4));
            assert!(
                offset
                    .checked_add(member_layout.size)
                    .is_some_and(|end| end <= layouter[handle].size),
                "struct member exceeds the layout of {}",
                name
            );
        }
    } else {
        panic!("expected struct type {name}");
    }

    let arguments = members
        .iter()
        .map(|(member_ty, word)| decode_expr(module, layouter, *member_ty, *word, out, emitted))
        .collect::<Vec<_>>()
        .join(", ");
    write!(
        out,
        "fn dl_load_{name}_impl(t: texture_2d<u32>, base: u32) -> {name} {{
    return {name}({arguments});
}}
"
    )
    .unwrap();
}

fn decode_expr(
    module: &naga::Module,
    layouter: &naga::proc::Layouter,
    handle: naga::Handle<naga::Type>,
    word: u32,
    out: &mut String,
    emitted: &mut BTreeSet<String>,
) -> String {
    match &module.types[handle].inner {
        naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Uint,
            ..
        }) => format!("dl_scene_word(t, base, {word}u)"),
        naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Float,
            ..
        }) => format!("bitcast<f32>(dl_scene_word(t, base, {word}u))"),
        naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Sint,
            ..
        }) => format!("bitcast<i32>(dl_scene_word(t, base, {word}u))"),
        naga::TypeInner::Vector { size, scalar } => {
            let kind = match scalar.kind {
                naga::ScalarKind::Uint => "u32",
                naga::ScalarKind::Float => "f32",
                naga::ScalarKind::Sint => "i32",
                other => panic!("unsupported vector scalar {other:?}"),
            };
            let cast = |w: u32| match scalar.kind {
                naga::ScalarKind::Uint => format!("dl_scene_word(t, base, {w}u)"),
                naga::ScalarKind::Float => {
                    format!("bitcast<f32>(dl_scene_word(t, base, {w}u))")
                }
                naga::ScalarKind::Sint => {
                    format!("bitcast<i32>(dl_scene_word(t, base, {w}u))")
                }
                other => panic!("unsupported scalar {other:?}"),
            };
            let size_word = u32::from(*size);
            let components = (0..size_word)
                .map(|index| cast(word + index))
                .collect::<Vec<_>>()
                .join(", ");
            format!("vec{size_word}<{kind}>({components})")
        }
        naga::TypeInner::Matrix { columns, rows, .. } => {
            assert_eq!(scalar_kind(module, handle), naga::ScalarKind::Float);
            let rows_word = u32::from(*rows);
            let columns_word = u32::from(*columns);
            let column_exprs = (0..columns_word)
                .map(|column| {
                    let base = word + column * rows_word;
                    let components = (0..rows_word)
                        .map(|component| {
                            format!(
                                "bitcast<f32>(dl_scene_word(t, base, {}u))",
                                base + component
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("vec{rows_word}<f32>({components})")
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("mat{columns_word}x{rows_word}<f32>({column_exprs})")
        }
        naga::TypeInner::Array { base, size, stride } => {
            // `Layouter::to_stride` on an array handle is the stride of the
            // array as a member of another array, i.e. its total size rounded
            // to alignment. The element stride lives on TypeInner::Array and
            // must be used for indexing individual elements.
            assert_eq!(*stride % 4, 0, "array element stride is not word aligned");
            let element_stride = *stride / 4;
            let naga::ArraySize::Constant(length) = size else {
                panic!("runtime-sized arrays are not supported inside downlevel elements");
            };
            let element_exprs = (0..length.get())
                .map(|index| {
                    decode_expr(
                        module,
                        layouter,
                        *base,
                        word + index * element_stride,
                        out,
                        emitted,
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("array({element_exprs})")
        }
        naga::TypeInner::Struct { .. } => {
            emit_type_decoder(module, layouter, handle, out, emitted);
            let name = module.types[handle].name.clone().expect("named struct");
            format!("dl_load_{name}_impl(t, base + {word}u)")
        }
        other => panic!("unsupported downlevel type {other:?}"),
    }
}

fn scalar_kind(module: &naga::Module, handle: naga::Handle<naga::Type>) -> naga::ScalarKind {
    match &module.types[handle].inner {
        naga::TypeInner::Matrix { scalar, .. }
        | naga::TypeInner::Vector { scalar, .. }
        | naga::TypeInner::Scalar(scalar) => scalar.kind,
        other => panic!("no scalar kind for {other:?}"),
    }
}

/// Closes the dual-source gap for natives: the second blend input becomes a second target.
fn native_dialect(source: &str) -> String {
    source
        .replace("@location(0) @blend_src(0)", "@location(0)")
        .replace("@location(0) @blend_src(1)", "@location(1)")
}

// --- Generated interface tables ------------------------------------------------------

fn write_interface(
    out_dir: &std::path::Path,
    layouts: &[BindingLayout<'_>],
    downlevel_layouts: &[BindingLayout<'_>],
) {
    let mut generated = String::from("// @generated by build.rs; do not edit.\n");
    writeln!(
        generated,
        "pub const DATA_TEXTURE_WIDTH: u32 = {DATA_TEXTURE_WIDTH};"
    )
    .unwrap();
    writeln!(
        generated,
        "pub const DOWNLEVEL_RANGE_BINDING: u32 = {DOWNLEVEL_RANGE_BINDING};"
    )
    .unwrap();
    for layout in layouts {
        validate_binding_layout(layout);
        write_bindings(&mut generated, layout);
    }
    for layout in downlevel_layouts {
        validate_binding_layout(layout);
        write_bindings(&mut generated, layout);
    }
    fs::write(out_dir.join("shader_interface.rs"), generated)
        .expect("failed to write generated shader interface");
}

fn validate_binding_layout(layout: &BindingLayout<'_>) {
    validate_with_current(layout.constant, layout.source);
    if matches!(layout.dialect, ReflectionDialect::Downlevel) {
        validate_with_floor(layout.constant, layout.source);
    }
}

fn write_bindings(generated: &mut String, layout: &BindingLayout<'_>) {
    let module = naga::front::wgsl::parse_str(layout.source).unwrap_or_else(|error| {
        panic!(
            "failed to parse {} for reflection: {error}",
            layout.constant
        )
    });
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|error| {
        panic!(
            "failed to validate {} for reflection: {error}",
            layout.constant
        )
    });
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .unwrap_or_else(|error| panic!("failed to lay out {}: {error}", layout.constant));

    let mut bindings = module
        .global_variables
        .iter()
        .filter_map(|(handle, variable)| {
            let binding = variable.binding.as_ref()?;
            (binding.group == layout.group).then_some((handle, variable, binding.binding))
        })
        .collect::<Vec<_>>();
    bindings.sort_by_key(|(_, _, binding)| *binding);
    let mut seen = BTreeSet::new();

    writeln!(
        generated,
        "pub const {}: &[GeneratedBinding] = &[",
        layout.constant,
    )
    .unwrap();
    for (handle, variable, binding) in bindings {
        assert!(
            seen.insert(binding),
            "duplicate binding {}:{binding} while generating {}",
            layout.group,
            layout.constant,
        );
        let visibility = module
            .entry_points
            .iter()
            .enumerate()
            .filter(|(index, _)| !info.get_entry_point(*index)[handle].is_empty())
            .fold(0, |stages, (_, entry)| {
                stages
                    | match entry.stage {
                        naga::ShaderStage::Vertex => 1,
                        naga::ShaderStage::Fragment => 2,
                        naga::ShaderStage::Compute => 4,
                        stage => {
                            panic!("unsupported shader stage {stage:?} in {}", layout.constant)
                        }
                    }
            });
        assert_ne!(
            visibility, 0,
            "binding {}:{binding} is unused in {}",
            layout.group, layout.constant
        );
        let kind = binding_kind(&module, &layouter, variable, binding, layout.dialect);
        writeln!(
            generated,
            "    GeneratedBinding {{ binding: {binding}, visibility: {visibility}, kind: {kind} }},"
        )
        .unwrap();
    }
    generated.push_str("];\n");
}

fn binding_kind(
    module: &naga::Module,
    layouter: &naga::proc::Layouter,
    variable: &naga::GlobalVariable,
    binding: u32,
    dialect: ReflectionDialect,
) -> String {
    match variable.space {
        naga::AddressSpace::Uniform
            if matches!(dialect, ReflectionDialect::Downlevel)
                && binding == DOWNLEVEL_RANGE_BINDING =>
        {
            "GeneratedBindingKind::RangeUniform".into()
        }
        naga::AddressSpace::Uniform => {
            format!(
                "GeneratedBindingKind::Uniform({})",
                layouter[variable.ty].size
            )
        }
        naga::AddressSpace::Storage { access } => {
            assert!(
                !access.contains(naga::StorageAccess::STORE),
                "writable shader storage is not supported by the generated interface"
            );
            let min_size = match module.types[variable.ty].inner {
                // Naga reports the static portion of a runtime array as its layout size;
                // wgpu requires room for one array element in a storage binding.
                naga::TypeInner::Array {
                    size: naga::ArraySize::Dynamic,
                    stride,
                    ..
                } => stride,
                _ => layouter[variable.ty].size,
            };
            format!("GeneratedBindingKind::StorageRead({})", min_size)
        }
        naga::AddressSpace::Handle => match module.types[variable.ty].inner {
            naga::TypeInner::Image {
                dim: naga::ImageDimension::D2,
                arrayed: false,
                class:
                    naga::ImageClass::Sampled {
                        kind: naga::ScalarKind::Float,
                        multi: false,
                    },
            } => "GeneratedBindingKind::Texture2dFloat".into(),
            naga::TypeInner::Image {
                dim: naga::ImageDimension::D2,
                arrayed: false,
                class:
                    naga::ImageClass::Sampled {
                        kind: naga::ScalarKind::Uint,
                        multi: false,
                    },
            } if matches!(dialect, ReflectionDialect::Downlevel) => {
                "GeneratedBindingKind::DataTexture".into()
            }
            naga::TypeInner::Sampler { comparison: false } => {
                "GeneratedBindingKind::FilteringSampler".into()
            }
            ref ty => panic!("unsupported shader resource type {ty:?}"),
        },
        space => panic!("unsupported shader resource address space {space:?}"),
    }
}

// --- Native artifacts: HLSL via floor Naga, GLSL/MSL via current Naga -----------------

fn write_native_shaders(out_dir: &std::path::Path) {
    use shaders::interface::*;

    let modules = [
        NativeShaderModule {
            pipeline: &QUADS,
            pipeline_path: "QUADS",
            source: &shaders::quad::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &SMOOTHED_QUADS,
            pipeline_path: "SMOOTHED_QUADS",
            source: &shaders::quad::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &SHADOWS,
            pipeline_path: "SHADOWS",
            source: &shaders::shadow::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &SMOOTHED_SHADOWS,
            pipeline_path: "SMOOTHED_SHADOWS",
            source: &shaders::shadow::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &PATH_RASTERIZATION,
            pipeline_path: "PATH_RASTERIZATION",
            source: &shaders::path_rasterization::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &PATHS,
            pipeline_path: "PATHS",
            source: &shaders::path::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &UNDERLINES,
            pipeline_path: "UNDERLINES",
            source: &shaders::underline::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &MONOCHROME_SPRITES,
            pipeline_path: "MONOCHROME_SPRITES",
            source: &shaders::monochrome_sprite::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &SUBPIXEL_SPRITES,
            pipeline_path: "SUBPIXEL_SPRITES",
            source: &shaders::subpixel_sprite::WGSL_SOURCE,
            requires_dual_source_lowering: true,
        },
        NativeShaderModule {
            pipeline: &POLYCHROME_SPRITES,
            pipeline_path: "POLYCHROME_SPRITES",
            source: &shaders::polychrome_sprite::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &SMOOTHED_POLYCHROME_SPRITES,
            pipeline_path: "SMOOTHED_POLYCHROME_SPRITES",
            source: &shaders::polychrome_sprite::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &SURFACES,
            pipeline_path: "SURFACES",
            source: &shaders::surface::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &EMOJI_RASTERIZATION,
            pipeline_path: "EMOJI_RASTERIZATION",
            source: &shaders::emoji_rasterization::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &BLUR_DOWNSAMPLE,
            pipeline_path: "BLUR_DOWNSAMPLE",
            source: &shaders::blur::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &BLUR,
            pipeline_path: "BLUR",
            source: &shaders::blur::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &BLUR_COMPOSITE,
            pipeline_path: "BLUR_COMPOSITE",
            source: &shaders::blur::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
        NativeShaderModule {
            pipeline: &SMOOTHED_BLUR_COMPOSITE,
            pipeline_path: "SMOOTHED_BLUR_COMPOSITE",
            source: &shaders::blur::WGSL_SOURCE,
            requires_dual_source_lowering: false,
        },
    ];
    let mut generated = String::from("// @generated by build.rs; do not edit.\n");
    generated.push_str("pub const NATIVE_SHADERS: &[NativeShader] = &[\n");

    for module in modules {
        let pipeline = module.pipeline;
        let mut wgsl = shader_source(pipeline.label, module.source);
        if module.requires_dual_source_lowering {
            wgsl = native_dialect(&wgsl);
        }
        let legacy_module = naga_old::front::wgsl::parse_str(&wgsl)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", pipeline.label));
        let legacy_info = naga_old::valid::Validator::new(
            naga_old::valid::ValidationFlags::all(),
            naga_old::valid::Capabilities::all(),
        )
        .validate(&legacy_module)
        .unwrap_or_else(|error| panic!("failed to validate {}: {error}", pipeline.label));

        let hlsl = write_hlsl(&legacy_module, &legacy_info, &wgsl, pipeline);
        let current_module = naga::front::wgsl::parse_str(&wgsl).unwrap_or_else(|error| {
            panic!(
                "failed to parse {} with current Naga: {error}",
                pipeline.label
            )
        });
        let current_info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&current_module)
        .unwrap_or_else(|error| {
            panic!(
                "failed to validate {} with current Naga: {error}",
                pipeline.label
            )
        });
        let msl = write_msl(&current_module, &current_info, pipeline.label);
        // GLSL 3.30 and GLES 3.00 cannot use storage buffers. Generate them from
        // the same data-texture dialect that backs WGPU's WebGL2 tier. Naga
        // validates that IR, then checks each requested GLSL target floor.
        let downlevel_wgsl = downlevel_dialect(pipeline.label, &wgsl);
        let downlevel_module =
            naga::front::wgsl::parse_str(&downlevel_wgsl).unwrap_or_else(|error| {
                panic!(
                    "failed to parse downlevel {} with current Naga: {error}",
                    pipeline.label
                )
            });
        let downlevel_info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&downlevel_module)
        .unwrap_or_else(|error| {
            panic!(
                "failed to validate downlevel {} with current Naga: {error}",
                pipeline.label
            )
        });
        let glsl_330_vertex = write_glsl(
            &downlevel_module,
            &downlevel_info,
            pipeline.label,
            naga::back::glsl::Version::Desktop(330),
            naga::ShaderStage::Vertex,
            pipeline.vertex_entry,
        );
        let glsl_330_fragment = write_glsl(
            &downlevel_module,
            &downlevel_info,
            pipeline.label,
            naga::back::glsl::Version::Desktop(330),
            naga::ShaderStage::Fragment,
            pipeline.fragment_entry,
        );
        let gles_300_vertex = write_glsl(
            &downlevel_module,
            &downlevel_info,
            pipeline.label,
            naga::back::glsl::Version::new_gles(300),
            naga::ShaderStage::Vertex,
            pipeline.vertex_entry,
        );
        let gles_300_fragment = write_glsl(
            &downlevel_module,
            &downlevel_info,
            pipeline.label,
            naga::back::glsl::Version::new_gles(300),
            naga::ShaderStage::Fragment,
            pipeline.fragment_entry,
        );
        let dx11_name = format!("{}.hlsl", pipeline.label);
        let msl_name = format!("{}.metal", pipeline.label);
        let glsl_330_vertex_name = format!("{}.330.vert.glsl", pipeline.label);
        let glsl_330_fragment_name = format!("{}.330.frag.glsl", pipeline.label);
        let gles_300_vertex_name = format!("{}.300es.vert.glsl", pipeline.label);
        let gles_300_fragment_name = format!("{}.300es.frag.glsl", pipeline.label);
        write_shader(out_dir, &dx11_name, &hlsl);
        write_shader(out_dir, &msl_name, &msl);
        write_shader(out_dir, &glsl_330_vertex_name, &glsl_330_vertex);
        write_shader(out_dir, &glsl_330_fragment_name, &glsl_330_fragment);
        write_shader(out_dir, &gles_300_vertex_name, &gles_300_vertex);
        write_shader(out_dir, &gles_300_fragment_name, &gles_300_fragment);
        let msl_path = format!("/{msl_name}");
        let glsl_330_vertex_path = format!("/{glsl_330_vertex_name}");
        let glsl_330_fragment_path = format!("/{glsl_330_fragment_name}");
        let gles_300_vertex_path = format!("/{gles_300_vertex_name}");
        let gles_300_fragment_path = format!("/{gles_300_fragment_name}");
        let dx11_artifact = write_dx11_bytecode(out_dir, pipeline.label, &hlsl, pipeline)
            .map(|bytecode| {
                let draw_constants = match dx11_draw_constants_register(pipeline) {
                    Some(register) => {
                        format!("Some(Dx11DrawConstantsBinding {{ register: {register} }})")
                    }
                    None => "None".into(),
                };
                format!(
                    "Dx11Shader::Sm50(Dx11Bytecode {{ vertex: include_bytes!(concat!(env!(\"OUT_DIR\"), {:?})), fragment: include_bytes!(concat!(env!(\"OUT_DIR\"), {:?})), draw_constants: {draw_constants} }})",
                    bytecode.vertex_path, bytecode.fragment_path,
                )
            })
            .unwrap_or_else(|| "Dx11Shader::NativeWindowsBuildRequired".into());
        writeln!(
            generated,
            r#"    NativeShader {{
        label: {:?},
        pipeline: &crate::shaders::interface::{},
        vertex_entry: {:?},
        fragment_entry: {:?},
        dx11: {},
        glsl_330: GlslShader::new(
            GlslShaderStage {{ source: include_str!(concat!(env!("OUT_DIR"), {:?})) }},
            GlslShaderStage {{ source: include_str!(concat!(env!("OUT_DIR"), {:?})) }},
        ),
        gles_300: GlslShader::new(
            GlslShaderStage {{ source: include_str!(concat!(env!("OUT_DIR"), {:?})) }},
            GlslShaderStage {{ source: include_str!(concat!(env!("OUT_DIR"), {:?})) }},
        ),
        msl: include_str!(concat!(env!("OUT_DIR"), {:?})),
    }},"#,
            pipeline.label,
            module.pipeline_path,
            pipeline.vertex_entry,
            pipeline.fragment_entry,
            dx11_artifact,
            glsl_330_vertex_path,
            glsl_330_fragment_path,
            gles_300_vertex_path,
            gles_300_fragment_path,
            msl_path,
        )
        .unwrap();
    }
    generated.push_str("];\n");
    fs::write(out_dir.join("native_shaders.rs"), generated)
        .expect("failed to write generated native shader table");
}

struct Dx11BytecodePaths {
    vertex_path: String,
    fragment_path: String,
}

/// Compile D3D11 bytecode while Cargo builds on Windows. A non-Windows build
/// may generate non-DX artifacts, but it cannot target Windows: that build is
/// rejected instead of hiding a first-draw runtime compiler cost.
#[cfg(windows)]
fn write_dx11_bytecode(
    out_dir: &std::path::Path,
    label: &str,
    source: &str,
    pipeline: &shaders::interface::Pipeline,
) -> Option<Dx11BytecodePaths> {
    use windows::{
        Win32::Graphics::Direct3D::{Fxc::D3DCompile, ID3DInclude},
        core::PCSTR,
    };

    fn compile(source: &str, label: &str, entry: &str, profile: &[u8]) -> Vec<u8> {
        let entry = CString::new(entry).expect("shader entry point contains NUL");
        let mut blob = None;
        let mut errors = None;
        unsafe {
            D3DCompile(
                source.as_ptr().cast(),
                source.len(),
                PCSTR::from_raw(b"gpui_shaders.hlsl\0".as_ptr()),
                None,
                None::<&ID3DInclude>,
                PCSTR::from_raw(entry.as_ptr().cast()),
                PCSTR::from_raw(profile.as_ptr()),
                0,
                0,
                &mut blob,
                Some(&mut errors),
            )
        }
        .unwrap_or_else(|error| {
            let details = errors.as_ref().map(|errors| unsafe {
                std::ffi::CStr::from_ptr(errors.GetBufferPointer().cast())
                    .to_string_lossy()
                    .into_owned()
            });
            panic!(
                "failed to compile generated HLSL for {label} ({entry:?}): {error}\n{}",
                details.unwrap_or_default()
            )
        });
        let blob = blob.expect("D3DCompile returned no bytecode");
        unsafe {
            std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize())
                .to_vec()
        }
    }

    let vertex_name = format!("{label}.vs_5_0.dxbc");
    let fragment_name = format!("{label}.ps_5_0.dxbc");
    write_bytes(
        out_dir,
        &vertex_name,
        &compile(source, label, pipeline.vertex_entry, b"vs_5_0\0"),
    );
    write_bytes(
        out_dir,
        &fragment_name,
        &compile(source, label, pipeline.fragment_entry, b"ps_5_0\0"),
    );
    Some(Dx11BytecodePaths {
        vertex_path: format!("/{vertex_name}"),
        fragment_path: format!("/{fragment_name}"),
    })
}

/// Set to build a Windows target from another host without DXBC, for type-checking only.
/// The resulting `gpui_windows` cannot draw: every pipeline reports `NativeWindowsBuildRequired`.
#[cfg(not(windows))]
const ALLOW_MISSING_DXBC: &str = "GPUI_RENDER_ALLOW_MISSING_DXBC";

#[cfg(not(windows))]
fn write_dx11_bytecode(
    _out_dir: &std::path::Path,
    _label: &str,
    _source: &str,
    _pipeline: &shaders::interface::Pipeline,
) -> Option<Dx11BytecodePaths> {
    println!("cargo:rerun-if-env-changed={ALLOW_MISSING_DXBC}");
    if env::var_os("CARGO_CFG_TARGET_OS").as_deref() == Some(std::ffi::OsStr::new("windows"))
        && env::var_os(ALLOW_MISSING_DXBC).is_none()
    {
        panic!(
            "building a Windows target from a non-Windows host cannot produce required DXBC; \
             build on Windows rather than deferring HLSL compilation to runtime, or set \
             {ALLOW_MISSING_DXBC}=1 for a check-only build"
        );
    }
    None
}

fn write_glsl(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    label: &str,
    version: naga::back::glsl::Version,
    shader_stage: naga::ShaderStage,
    entry_point: &str,
) -> String {
    let mut binding_map = naga::back::glsl::BindingMap::default();
    for (_, variable) in module.global_variables.iter() {
        let Some(binding) = variable.binding else {
            continue;
        };
        // A resource binding must be globally unique in legacy GLSL. This
        // mirrors WGPU's group-first allocation without coupling artifacts to
        // a particular GL context's final binding slots.
        let slot = binding
            .group
            .checked_mul(8)
            .and_then(|group| group.checked_add(binding.binding))
            .and_then(|slot| u8::try_from(slot).ok())
            .unwrap_or_else(|| panic!("GLSL binding out of range in {label}"));
        binding_map.insert(binding, slot);
    }
    let options = naga::back::glsl::Options {
        version,
        writer_flags: naga::back::glsl::WriterFlags::ADJUST_COORDINATE_SPACE,
        binding_map,
        zero_initialize_workgroup_memory: true,
    };
    let pipeline_options = naga::back::glsl::PipelineOptions {
        shader_stage,
        entry_point: entry_point.into(),
        multiview: None,
    };
    let mut output = String::new();
    let mut writer = naga::back::glsl::Writer::new(
        &mut output,
        module,
        info,
        &options,
        &pipeline_options,
        naga::proc::BoundsCheckPolicies::default(),
    )
    .unwrap_or_else(|error| {
        panic!("failed to initialize {version} GLSL for {label} ({entry_point}): {error}")
    });
    writer.write().unwrap_or_else(|error| {
        panic!("failed to generate {version} GLSL for {label} ({entry_point}): {error}")
    });
    // Naga's GLSL frontend intentionally does not accept all dialects its
    // backend emits (notably GLSL 330 resource-layout extensions), so a
    // parse-and-revalidate round trip would reject valid backend output. The
    // input IR has already passed Naga validation and `Writer::new` checks the
    // target's feature floor; retain cheap invariants that catch truncated or
    // incorrectly targeted output without pretending the frontend is a GL compiler.
    let expected_version = match version {
        naga::back::glsl::Version::Desktop(version) => format!("#version {version} core"),
        naga::back::glsl::Version::Embedded { version, .. } => format!("#version {version} es"),
    };
    assert!(
        output.starts_with(&expected_version) && output.contains("void main()"),
        "Naga emitted an incomplete GLSL artifact for {label} ({entry_point})"
    );
    output
}

/// Instanced pipelines read their batch base from the draw-constants cbuffer.
///
/// Direct3D 11 never folds `StartInstanceLocation` into `SV_InstanceID`, so a shader that
/// indexes a whole-frame instance buffer needs the base delivered some other way. Naga's
/// "special constants" cbuffer is that way: `instance_index` becomes
/// `first_instance + SV_InstanceID`. Fullscreen and per-draw-uniform pipelines draw one
/// instance from vertex zero and carry no such cbuffer.
fn dx11_draw_constants_register(pipeline: &shaders::interface::Pipeline) -> Option<u32> {
    use shaders::interface::DataLayout;
    match pipeline.data_layout {
        DataLayout::Instances
        | DataLayout::TexturedInstances
        | DataLayout::MonochromeSprites
        | DataLayout::SubpixelSprites => Some(shaders::interface::DX11_DRAW_CONSTANTS_REGISTER),
        DataLayout::NativeOnly | DataLayout::Surface | DataLayout::Blur => None,
    }
}

fn write_hlsl(
    module: &naga_old::Module,
    info: &naga_old::valid::ModuleInfo,
    wgsl: &str,
    pipeline: &shaders::interface::Pipeline,
) -> String {
    let label = pipeline.label;
    let mut binding_map = naga_old::back::hlsl::BindingMap::default();
    for (_, variable) in module.global_variables.iter() {
        if let Some(binding) = &variable.binding {
            binding_map.insert(
                binding.clone(),
                naga_old::back::hlsl::BindTarget {
                    space: 0,
                    register: binding.binding + if binding.group == 0 { 0 } else { 2 },
                    ..Default::default()
                },
            );
        }
    }
    let draw_constants_register = dx11_draw_constants_register(pipeline);
    let options = naga_old::back::hlsl::Options {
        shader_model: naga_old::back::hlsl::ShaderModel::V5_0,
        binding_map,
        fake_missing_bindings: false,
        special_constants_binding: draw_constants_register.map(|register| {
            naga_old::back::hlsl::BindTarget {
                space: 0,
                register,
                ..Default::default()
            }
        }),
        ..Default::default()
    };
    let mut output = String::new();
    naga_old::back::hlsl::Writer::new(&mut output, &options)
        .write(module, info, None)
        .unwrap_or_else(|error| panic!("failed to generate HLSL for {label}: {error}"));
    match draw_constants_register {
        Some(register) => lower_draw_constants_to_sm50(output, wgsl, label, register),
        None => {
            assert!(
                !wgsl.contains("instance_index"),
                "{label} reads instance_index but has no DX11 draw constants"
            );
            output
        }
    }
}

/// Naga declares its special constants with `ConstantBuffer<T>`, a shader-model 5.1 form
/// that `vs_5_0` rejects. Rewrite that one declaration into the classic `cbuffer` block and
/// prove the base actually reaches every instance lookup.
fn lower_draw_constants_to_sm50(hlsl: String, wgsl: &str, label: &str, register: u32) -> String {
    let declaration =
        format!("ConstantBuffer<NagaConstants> _NagaConstants: register(b{register});");
    assert_eq!(
        hlsl.matches(&declaration).count(),
        1,
        "{label}: expected exactly one Naga special-constants declaration"
    );
    assert_eq!(
        hlsl.matches(&format!("register(b{register})")).count(),
        1,
        "{label}: draw-constants register b{register} collides with another cbuffer"
    );
    let lowered = hlsl.replace(
        &declaration,
        &format!(
            "cbuffer DrawConstants : register(b{register}) {{ NagaConstants _NagaConstants; }}"
        ),
    );
    if wgsl.contains("instance_index") {
        assert!(
            lowered.contains("_NagaConstants.first_instance + "),
            "{label}: instance_index must be offset by the draw-constants base"
        );
    }
    lowered
}

fn write_msl(module: &naga::Module, info: &naga::valid::ModuleInfo, label: &str) -> String {
    let mut resources = naga::back::msl::BindingMap::default();
    for (_, variable) in module.global_variables.iter() {
        let Some(binding) = &variable.binding else {
            continue;
        };
        let target = match variable.space {
            naga::AddressSpace::Uniform | naga::AddressSpace::Storage { .. } => {
                naga::back::msl::BindTarget {
                    buffer: Some((binding.binding + if binding.group == 0 { 0 } else { 2 }) as u8),
                    ..Default::default()
                }
            }
            naga::AddressSpace::Handle => match module.types[variable.ty].inner {
                naga::TypeInner::Image { .. } => naga::back::msl::BindTarget {
                    texture: Some(binding.binding.saturating_sub(1) as u8),
                    ..Default::default()
                },
                naga::TypeInner::Sampler { .. } => naga::back::msl::BindTarget {
                    sampler: Some(naga::back::msl::BindSamplerTarget::Resource(0)),
                    ..Default::default()
                },
                ref ty => panic!("unsupported MSL resource type {ty:?} in {label}"),
            },
            space => panic!("unsupported MSL resource space {space:?} in {label}"),
        };
        resources.insert(*binding, target);
    }
    let entry_resources = naga::back::msl::EntryPointResources {
        resources,
        sizes_buffer: Some(3),
        ..Default::default()
    };
    let per_entry_point_map = module
        .entry_points
        .iter()
        .map(|entry| (entry.name.clone(), entry_resources.clone()))
        .collect();
    let options = naga::back::msl::Options {
        lang_version: (2, 0),
        per_entry_point_map,
        fake_missing_bindings: false,
        ..Default::default()
    };
    naga::back::msl::write_string(
        module,
        info,
        &options,
        &naga::back::msl::PipelineOptions::default(),
    )
    .map(|(source, _)| source)
    .unwrap_or_else(|error| panic!("failed to generate MSL for {label}: {error}"))
}
