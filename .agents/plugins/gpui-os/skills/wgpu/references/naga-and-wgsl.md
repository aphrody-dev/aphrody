# Naga v30.0.0 & WGSL Reference (`vendor/wgpu/naga`)

## 1. Naga Cargo Features (`vendor/wgpu/naga/Cargo.toml`)

Only enable features that actually exist in `naga/Cargo.toml`:
- **Frontends**: `wgsl-in` (`dep:unicode-ident`), `glsl-in` (`dep:pp-rs`), `spv-in` (`dep:petgraph`, `dep:spirv`).
- **Backends**: `wgsl-out`, `glsl-out`, `hlsl-out`, `hlsl-out-if-target-windows`, `msl-out`, `msl-out-if-target-apple`, `spv-out` (`dep:spirv`), `dot-out`.
- **Diagnostics & Serde**: `stderr`, `termcolor`, `serialize`, `deserialize`.

All 5 shader output backends (`wgsl-out`, `glsl-out`, `hlsl-out`, `msl-out`, `spv-out`) plus `wgsl-in` compile cleanly with zero missing dependencies in `gpui-os` (`spirv 0.4` and `unicode-ident 1.0` are already in `Cargo.lock`).

## 2. Parsing, Validation, & Rich Diagnostics

```rust
use naga::front::wgsl;
use naga::valid::{Capabilities, ShaderStages, SubgroupOperationSet, ValidationFlags, Validator};

pub fn parse_and_validate_wgsl(
    source: &str,
    path_label: &str,
    capabilities: Capabilities,
) -> anyhow::Result<(naga::Module, naga::valid::ModuleInfo)> {
    let options = wgsl::Options {
        parse_doc_comments: false,
        capabilities,
    };
    let mut frontend = wgsl::Frontend::new_with_options(options);
    let module = frontend.parse(source).map_err(|parse_err| {
        anyhow::anyhow!("{}", parse_err.emit_to_string_with_path(source, path_label))
    })?;

    let mut validator = Validator::new(ValidationFlags::all(), capabilities);
    validator
        .subgroup_stages(ShaderStages::all())
        .subgroup_operations(SubgroupOperationSet::all());

    let info = validator.validate(&module).map_err(|val_err| {
        anyhow::anyhow!("{}", val_err.emit_to_string_with_path(source, path_label))
    })?;

    Ok((module, info))
}
```

### Selecting `Capabilities`
- Use `Capabilities::empty()` when enforcing strict baseline WebGPU portability (including WebGL2-compatible shaders without extensions, as `nsh-core` and `gpui_wgpu` `WEBGL_SHADERS`/`STORAGE_BUFFER_SHADERS` tests do).
- Use `Capabilities::DUAL_SOURCE_BLENDING` when `enable dual_source_blending;` is present (`gpui_wgpu` `SUBPIXEL_SHADERS`).
- Use `Capabilities::all()` in general-purpose inspection/CLI tools (`gpui-wgpu validate` / `transpile`) so shaders using `f16`, `subgroups`, `immediates`, `ray_query`, or `binding_array` validate cleanly.

## 3. Reflecting Entry Points, Bindings, Overrides & Struct Layouts

`naga::proc::Layouter` computes exact WGSL byte sizes and alignments (`[WGSL §4.3.7 Memory Layout]`):

```rust
let mut layouter = naga::proc::Layouter::default();
layouter.update(module.to_ctx())?;

// 1. Entry points
for ep in &module.entry_points {
    // ep.name: String
    // ep.stage: ShaderStage::{Vertex, Fragment, Compute, Task, Mesh, ...}
    // ep.workgroup_size: [u32; 3]
}

// 2. Global bindings & Immediates
for (_handle, var) in module.global_variables.iter() {
    let layout = layouter[var.ty];
    let size = layout.size;
    let alignment = layout.alignment * 1u32;
    // var.binding: Option<ResourceBinding { group, binding }>
    // var.space: AddressSpace::{Uniform, Storage { access }, Handle, Immediate, WorkGroup, Private, ...}
    if let naga::TypeInner::Struct { ref members, span } = module.types[var.ty].inner {
        for member in members {
            let member_layout = layouter[member.ty];
            // member.name, member.offset, member_layout.size, member_layout.alignment * 1u32
        }
    }
}

// 3. Pipeline-overridable constants (`override`)
for (_handle, ov) in module.overrides.iter() {
    // ov.name: Option<String>, ov.id: Option<u16>, ov.ty: Handle<Type>, ov.init: Option<Handle<Expression>>
}
```

## 4. WGSL Memory Layout Rules & Rust `#[repr(C)]` Alignment Traps

| WGSL Type | Align | Size | Rust `bytemuck::Pod` Equivalent |
| :--- | :--- | :--- | :--- |
| `f32` / `i32` / `u32` | 4 | 4 | `f32` / `i32` / `u32` (`bool` is forbidden in host-shareable buffers!) |
| `vec2<f32>` | 8 | 8 | `[f32; 2]` (must sit at 8-byte multiple offset in `#[repr(C)]`!) |
| `vec3<f32>` | **16** | **12** | `[f32; 3]` + `f32` scalar or `_pad: f32`! Never follow `[f32; 3]` with `[f32; 2]` without accounting for WGSL's 16B alignment. |
| `vec4<f32>` | 16 | 16 | `[f32; 4]` |
| `mat2x2<f32>` | 8 | 16 | `[[f32; 2]; 2]` (column-major; note HLSL/SPIR-V backends decompose `matCx2` in uniforms automatically) |
| `mat3x3<f32>` | 16 | 48 | `[[f32; 4]; 3]` (each 3-float column is padded to 16 bytes!) |
| `mat4x4<f32>` | 16 | 64 | `[[f32; 4]; 4]` (column-major) |
| `array<vec4<f32>, N>` | 16 | `16 * N` | `[[f32; 4]; N]` (required in `var<uniform>`, where array stride MUST be a multiple of 16!) |
| `array<T>` (runtime) | `Align(T)` | dynamic | Only allowed as the **last member** of a `var<storage>` struct (or direct `var<storage>`). |

**GLES / WebGL2 Uniform Padding Rule**:
Devices without `DownlevelFlags::BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED` reject `var<uniform>` structs whose total `span` is not a multiple of 16 bytes. Always pad `#[repr(C)]` uniform structs to a multiple of 16 bytes (or use `@size(N)` on the last WGSL field, as `nsh-core` does with `@size(12) overflow: f32`).

## 5. Cross-Compilation Recipes (`naga::back::*`)

Always run `naga::back::pipeline_constants::process_overrides(&module, &info, entry_point, &overrides)` before invoking `hlsl`, `msl`, `spv`, or `glsl` writers!

### HLSL (`naga::back::hlsl`)
- `Options { shader_model: ShaderModel::V6_0, fake_missing_bindings: true, ..Default::default() }`: maps `@group(g) @binding(b)` automatically to `register(b/t/u{b}, space{g})`.
- Storage buffers lower to `ByteAddressBuffer` / `RWByteAddressBuffer` with explicit byte-offset `.Load` / `.Store`.
- Uniform matrices emit `row_major` with transposed dimensions so `m[col]` indexing matches WGSL semantics.

### Metal MSL (`naga::back::msl`)
- Metal uses flat per-entry-point `[[buffer(n)]]`, `[[texture(n)]]`, and `[[sampler(n)]]` indices via `Options::per_entry_point_map: BTreeMap<String, EntryPointResources>`.
- Populate `EntryPointResources` by iterating `module.global_variables` and assigning monotonic `buffer`, `texture`, and `sampler: BindSamplerTarget::Resource(slot)` indices per entry point, plus `immediates_buffer` and `sizes_buffer` if runtime arrays are used. (Relying on `fake_missing_bindings: true` alone emits `[[user(fake0)]]` placeholders rather than `[[buffer(0)]]`.)

### GLSL (`naga::back::glsl`)
- Emits **one entry point per `Writer` call** (`PipelineOptions { shader_stage, entry_point, multiview: None }`).
- Versions:
  - Desktop OpenGL: `Version::Desktop(450)` (or `330`..`460`).
  - OpenGL ES: `Version::new_gles(310)` (supports compute + SSBOs).
  - WebGL2: `Version::Embedded { version: 300, is_webgl: true }` (vertex/fragment only, UBOs + textures).
- `writer.write()?` returns `ReflectionInfo` containing `texture_mapping` (combined texture+sampler GLSL uniform names) and `uniforms` (UBO block names).

### SPIR-V (`naga::back::spv`)
- `spv::write_vec(&module, &info, &options, pipeline_options.as_ref())?` returns `Vec<u32>` words; convert to little-endian bytes via `words.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>()`.
- Automatically synthesizes `Std140CompatTypeInfo` wrapper structs when `var<uniform>` contains `matCx2` matrices so Vulkan `std140` extended-layout rules are satisfied.
