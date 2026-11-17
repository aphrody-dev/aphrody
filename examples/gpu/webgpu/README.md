# WebGPU & WGSL Verification Examples

This directory provides working, production-grade examples demonstrating the WebGPU and WGSL architecture in `aphrody`.

---

## File Catalog

| File | Purpose |
| :--- | :--- |
| **`nsh_ripple_shader.wgsl`** | Complete, runnable WGSL shader module compliant with both Naga and Google Dawn Tint. Implements host-shareable 16-byte aligned uniforms, procedural fullscreen quad, and Oklab color grading. |
| **`nsh_ripple.shade.wgsl`** | The pure fragment authoring snippet (`fn shade(uv: vec2f) -> vec4f`) consumed by the NSH UI engine (`crates/nsh-core` and `crates/nsh-engine`). |
| **`webgpu_inspect.ts`** | Zero-bloat Bun TypeScript inspector consuming official `@webgpu/types`. Tests 256-byte texture row pitch alignment, runs Naga and Tint validators, and cross-compiles to all 5 backend targets. |
| **`package.json`** | Zero-dependency Bun package setup with `@webgpu/types` (`^0.1.74`). |
| **`tsconfig.json`** | Strict TypeScript configuration including `["bun", "@webgpu/types"]`. |
| **`verify.ps1`** | End-to-end PowerShell automated test script executing typechecking, inspection, multi-target transpilation, and headless rendering/compute. |
| **`crates/aphrody_cli/examples/webgpu_naga_pipeline.rs`** | Complete Rust example demonstrating Naga IR validation, multi-target transpilation (WGSL, SPIR-V, HLSL, MSL, GLSL ES 300), headless render to PNG, and storage buffer compute dispatch. |

---

## Running the Examples

### 1. Bun TypeScript Typecheck & Inspector
```powershell
cd examples\gpu\webgpu
bun check
bun run webgpu_inspect.ts
```

### 2. Full Verification Suite
```powershell
powershell -ExecutionPolicy Bypass -File examples\gpu\webgpu\verify.ps1
```

### 3. Shader Validation with Multiple Compilers
```bash
# Cross-platform full verification runner
bun examples/gpu/verify.ts webgpu

# Validate with gpui-wgpu (Naga)
gpui-wgpu validate examples/gpu/webgpu/nsh_ripple_shader.wgsl --json

# Validate with Naga CLI
naga examples/gpu/webgpu/nsh_ripple_shader.wgsl

# Validate with Google Dawn Tint
tint --parse-only true examples/gpu/webgpu/nsh_ripple_shader.wgsl
tint_info --json examples/gpu/webgpu/nsh_ripple_shader.wgsl
```

### 4. Cross-Backend Transpilation
```bash
# SPIR-V (Vulkan)
gpui-wgpu transpile examples/gpu/webgpu/nsh_ripple_shader.wgsl --target spv -o nsh_ripple.spv

# HLSL (DirectX 12)
gpui-wgpu transpile examples/gpu/webgpu/nsh_ripple_shader.wgsl --target hlsl --stage fragment -o nsh_ripple.hlsl

# Metal (macOS/iOS)
gpui-wgpu transpile examples/gpu/webgpu/nsh_ripple_shader.wgsl --target msl --stage fragment -o nsh_ripple.metal

# GLSL ES 300 (WebGL2)
gpui-wgpu transpile examples/gpu/webgpu/nsh_ripple_shader.wgsl --target webgl2 --stage fragment -o nsh_ripple.essl
```

### 5. Headless GPU Render Pass
```bash
gpui-wgpu render examples/gpu/webgpu/nsh_ripple_shader.wgsl --output nsh_ripple.png --width 512 --height 512
```

---

## Key Technical Invariants

1. **Host-Sharing Alignment (`std140` / `std430`)**:
   - `vec3<f32>` has **alignment 16 and size 12**. Unpadded `vec3` structs in Rust will mismatch GPU expectations. Always pad or use `vec4<f32>`.
   - Structures in `var<uniform>` must be padded to a multiple of **16 bytes**.
2. **Texture-Buffer Copy Row Stride**:
   - `bytes_per_row` must align to a multiple of **256 bytes** (`COPY_BYTES_PER_ROW_ALIGNMENT`).
3. **NSH Fragment Contract**:
   - Custom view shaders define only `fn shade(uv: vec2f) -> vec4f` returning straight sRGB color with alpha.
   - Entry points and built-in uniforms (`nsh.size`, `nsh.time`, `nsh.scale`, `nsh.progress`, `nsh.overflow`) are synthesized automatically by `crates/nsh-core/src/shader.rs`.
