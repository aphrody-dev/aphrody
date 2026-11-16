# SPIR-V & GLSL Compilation Pipeline (`aphrody`)

SPIR-V (Standard Portable Intermediate Representation) is the standard bytecode consumed by Vulkan drivers. In `aphrody`, shaders originate from either **GLSL** (standalone tools, existing graphics assets) or **WGSL** (WebGPU standard shaders in `crates/gpui_wgpu` and `nsh`), and are translated, validated, and optimized into SPIR-V.

---

## 1. CLI Toolchain Discovery

All tools are resolved cross-platform from `PATH`, `$VULKAN_SDK/bin`, or `bun examples/gpu/verify.ts vulkan`:

| Tool | Primary Command / Role |
| :--- | :--- |
| `glslangValidator` | Strict Khronos GLSL/HLSL reference front-end to SPIR-V |
| `glslc` | Google compiler driver combining front-end, preprocessor, and optimizer |
| `spirv-val` | SPIR-V 1.0–1.6 bytecode structural & semantics validator |
| `spirv-opt` | High-level SPIR-V optimization and dead-code stripping passes |
| `spirv-dis` | Disassembles binary `.spv` into human-readable SPIR-V assembly |
| `spirv-as` | Assembles text SPIR-V assembly into binary `.spv` |
| `gpui-wgpu` | Naga-based WGSL reflection, validation, and SPIR-V transpilation |

---

## 2. Compiling GLSL to SPIR-V

### Using `glslangValidator`
The canonical Khronos compiler directly parses GLSL and emits SPIR-V:
```powershell
# Compute Shader
glslangValidator -V -o wave_compute.spv wave_compute.comp

# Vertex and Fragment Shaders
glslangValidator -V -o triangle.vert.spv triangle.vert
glslangValidator -V -o triangle.frag.spv triangle.frag

# Target specific SPIR-V version (e.g. 1.6 for Vulkan 1.3/1.4 core)
glslangValidator --target-env vulkan1.3 -V -o shader.spv shader.frag
```

### Using `glslc`
`glslc` provides GCC/Clang-style CLI ergonomics, auto-detecting stages by extension (`.vert`, `.frag`, `.comp`, `.geom`, `.tesc`, `.tese`):
```powershell
# Compile compute shader with debug info and optimization
glslc -O -g -o wave_compute.spv wave_compute.comp

# Explicit stage override (useful when file names lack standard extensions)
glslc -fshader-stage=compute -o output.spv custom_kernel.glsl

# Define preprocessor macros
glslc -DENABLE_BLUR=1 -DTILE_SIZE=16 -o blur.spv blur.comp
```

---

## 3. Validating and Optimizing SPIR-V

Before dispatching SPIR-V on device or packaging it into binary bundles, run structural verification:

### Validation (`spirv-val`)
```powershell
# Validate targeting Vulkan 1.3 / SPIR-V 1.6
spirv-val --target-env vulkan1.3 wave_compute.spv

# Validate targeting general SPIR-V 1.6
spirv-val --target-env spv1.6 wave_compute.spv
```
If validation succeeds, `spirv-val` returns exit code `0` with no output. If errors exist (e.g. invalid type conversions, missing layout decorations, mismatched block sizes), descriptive line errors and opcode traces are emitted.

### Optimization (`spirv-opt`)
`spirv-opt` applies dead code elimination, constant folding, loop unrolling, and scalar replacement of aggregates:
```powershell
# Size optimization pass
spirv-opt -Os --target-env vulkan1.3 wave_compute.spv -o wave_compute.opt.spv

# Performance optimization pass
spirv-opt -O --target-env vulkan1.3 wave_compute.spv -o wave_compute.opt.spv

# Strip reflection and debug info for production builds
spirv-opt --strip-debug wave_compute.opt.spv -o wave_compute.final.spv
```

### Disassembly (`spirv-dis`)
```powershell
# Inspect human-readable assembly opcodes
spirv-dis wave_compute.spv -o wave_compute.spvasm
```

---

## 4. Naga SPIR-V Pipeline in `aphrody`

In Rust code, `aphrody` compiles WGSL into SPIR-V programmatically via `naga`:

```rust
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::back::spv::{Options, WriterFlags, write_vec};

// 1. Parse WGSL source string into Naga IR module
let module = wgsl::parse_str(wgsl_source)?;

// 2. Validate module types, bindings, and expressions
let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
let module_info = validator.validate(&module)?;

// 3. Configure SPIR-V emission options
let mut options = Options::default();
options.lang_version = (1, 6); // SPIR-V 1.6 for Vulkan 1.3+
options.flags = WriterFlags::ADJUST_COORDINATE_SPACE
    | WriterFlags::LABEL_VARYINGS
    | WriterFlags::CLAMP_FRAG_DEPTH;

// 4. Emit SPIR-V binary words (u32 words slice)
let spirv_words: Vec<u32> = write_vec(&module, &module_info, &options, None)?;
```

### CLI Transpilation with `gpui-wgpu`
`gpui-wgpu` exposes this exact pipeline via the CLI:
```powershell
gpui-wgpu transpile --stage compute input.wgsl --target spv --output output.spv
```

---

## 5. Coordinate Systems & Viewport Y-Flipping

One of the most frequent sources of rendering discrepancies across graphics APIs is the clip space coordinate system convention.

### The Fundamental Coordinate Discrepancy
- **Vulkan Normalized Device Coordinates (NDC)**:
  - $X \in [-1.0, 1.0]$ (left to right)
  - $Y \in [-1.0, 1.0]$ with **$Y = +1.0$ at the bottom** and **$Y = -1.0$ at the top** (Y points down).
  - $Z \in [0.0, 1.0]$ (near to far).
- **WebGPU / DirectX / Metal NDC**:
  - $X \in [-1.0, 1.0]$ (left to right)
  - $Y \in [-1.0, 1.0]$ with **$Y = +1.0$ at the top** and **$Y = -1.0$ at the bottom** (Y points up).
  - $Z \in [0.0, 1.0]$ (near to far).

If a shader written with WebGPU or DirectX conventions runs directly on Vulkan without coordinate compensation, rendered geometry appears vertically inverted (upside down), and triangle winding order (`CW` vs `CCW`) flips, triggering unwanted backface culling.

### Solution 1: Negative Viewport Height (`VK_KHR_maintenance1` / Vulkan 1.1 Core)
This is the zero-overhead, hardware-rasterizer-level approach used throughout `wgpu-hal`:
```rust
// Verified from vendor/wgpu/wgpu-hal/src/vulkan/command.rs:898-904
let vk_viewports = [vk::Viewport {
    x: 0.0,
    y: desc.extent.height as f32,       // Start at the bottom of the render area
    width: desc.extent.width as f32,
    height: -(desc.extent.height as f32), // Negative height flips the rasterizer NDC Y mapping
    min_depth: 0.0,
    max_depth: 1.0,
}];
```
- **Winding order preservation**: Because the viewport inversion occurs during primitive assembly, standard front-face definitions (such as `FrontFace::Ccw`) remain unchanged.
- **Hardware requirement**: Supported on all Vulkan 1.1+ implementations and mandated by Vulkan 1.3/1.4 core.

### Solution 2: Shader-Side Coordinate Adjustment (`ADJUST_COORDINATE_SPACE`)
When generating standalone SPIR-V for external consumers or offline tools that do not use negative viewport heights:
- Naga's `WriterFlags::ADJUST_COORDINATE_SPACE` injects an arithmetic negation into the vertex shader before writing `gl_Position`:
  ```glsl
  gl_Position.y = -gl_Position.y;
  ```
- Note: Shader-side negation inverts the apparent triangle winding order, requiring the pipeline rasterizer state to swap `FrontFace::Ccw` and `FrontFace::Cw`.
