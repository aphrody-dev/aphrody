# GPUI & NSH WGPU Integration (`crates/gpui_wgpu` & `crates/nsh-*`)

## 1. `crates/gpui_wgpu` Architecture

`crates/gpui_wgpu` is GPUI's cross-platform GPU renderer powering Linux (`gpui_linux`), Windows (`gpui_windows`), Web (`gpui_web` on WebGPU + WebGL2), and headless visual tests/benchmarks (`WgpuHeadlessRenderer`).

### Source File Responsibilities
- **`src/gpui_wgpu.rs`**: Crate root (`[lib] path = "src/gpui_wgpu.rs"`). Re-exports `WgpuContext`, `WgpuRenderer`, `WgpuSurfaceConfig`, `GpuContext`, `WgpuAtlas`, `CosmicTextSystem`, `WgpuHeadlessRenderer` (under `test` / `bench-support` / `test-support`), and `wgpu`.
- **`src/wgpu_context.rs`**:
  - `WgpuContext` holds `Instance`, `Adapter`, `Arc<Device>`, `Arc<Queue>`, `WgpuBackend`, `dual_source_blending: bool`, `color_texture_format: TextureFormat`, and `Arc<DeviceErrorState>`.
  - **Hybrid GPU Selection (`select_adapter_and_device`)**: Enumerates all adapters, sorts by `(ZED_DEVICE_ID override, CompositorGpuHint match, DeviceType priority, Backend priority)`, and tests each candidate by opening a real `Device` and calling `surface.configure` inside `device.push_error_scope(wgpu::ErrorFilter::Validation)`.
  - **`DeviceErrorState`**: Installs `device.set_device_lost_callback` and `device.on_uncaptured_error` once per `Device`, tracking an atomic `lost` flag and a generation-counted `last_error` so multiple window renderers sharing one `Device` observe errors independently without consuming each other's state.
- **`src/wgpu_atlas.rs`**:
  - `WgpuAtlas` manages dynamic 2D texture pages (`1024x1024` up to `max_texture_dimension_2d`) with `etagere::BucketedAtlasAllocator`:
    - `AtlasTextureKind::Monochrome` -> `R8Unorm` (1 Bpp)
    - `AtlasTextureKind::Subpixel` -> `Bgra8Unorm` / `Rgba8Unorm` (4 Bpp, RGBA->BGRA swizzled on CPU if needed)
    - `AtlasTextureKind::Polychrome` -> `Bgra8Unorm` / `Rgba8Unorm` (4 Bpp)
  - Queues `PendingUpload`s and flushes them in `before_frame()` via `queue.write_texture`. Assigns monotonic `generation: u64` IDs so `WgpuRenderer` refreshes cached `BindGroup`s whenever a freed `AtlasTextureId` slot is reused.
- **`src/wgpu_renderer.rs`**:
  - **Instance Data Transport (`InstanceData`)**:
    - Native & Browser WebGPU: Single growable `STORAGE | COPY_DST` buffer (`initial_capacity = 2 MiB`, up to `256 MiB`), sliced per primitive type at `min_storage_buffer_offset_alignment` boundaries (minimum 16 bytes per binding so empty batches never create zero-sized bindings).
    - WASM WebGL2 (`uses_webgl_instance_data = true`): Packs primitive structs into a 2D `Rgba32Uint` texture (`16` bytes/texel) aligned to `lcm(16, sizeof(T))` so vertex and fragment stages read arbitrary primitive records via `textureLoad` without SSBOs!
  - **Single-Pass Reverse-Z Batching (`record_frame`)**:
    1. Binds `globals_bind_group` at `@group(0)` (`GlobalParams` at binding 0, `GammaParams` at `min_uniform_buffer_offset_alignment`-aligned offset in binding 1).
    2. Draws `opaque_quads` across `0..scene.quads.len()` **front-to-back** with `Depth32Float` (`CompareFunction::Greater`, `depth_write_enabled: Some(true)`, cleared to `0.0`, `StoreOp::Discard`, `TRANSIENT_ATTACHMENT` where supported) to populate early-Z occlusion so covered background pixels skip fragment shading!
    3. Iterates `scene.batches()` (`Quads`, `Shadows`, `Paths`, `Underlines`, `MonochromeSprites`, `SubpixelSprites`, `PolychromeSprites`), locking non-quad batch depth to `quad_depth(quad_cursor, depth_scale)` via `pass.set_viewport(0.0, 0.0, w, h, depth, depth)`.
  - **Subpixel Text Rendering (`SUBPIXEL_SHADERS`)**:
    - Enabled when `adapter.features().contains(wgpu::Features::DUAL_SOURCE_BLENDING)`.
    - Prepends `enable dual_source_blending;` before `shaders.wgsl`, outputs `@location(0) @blend_src(0) foreground` and `@location(0) @blend_src(1) alpha`, and blends with `Src1` / `OneMinusSrc1`. Gracefully falls back to `mono_sprites` when unsupported.
  - **Lifecycle & Device Loss Recovery**:
    - `unconfigure_surface()` drops only `wgpu::Surface<'static>` (`RendererState::Unconfigured { core }`), keeping `Device`, pipelines, and `WgpuAtlas` alive across Android lifecycle pauses.
    - `replace_surface()` attaches a new window surface (must use the **same `wgpu::Instance`** from `WgpuContext`!).
    - `recover()` handles driver resets/TDRs across multi-window apps: the first window to observe `device_lost()` sleeps 350ms, recreates `WgpuContext` rejecting software adapters, resets `WgpuAtlas`, and subsequent windows adopt the recovered `GpuContext`.

### Shader Invariant Tests in `gpui_wgpu`
`crates/gpui_wgpu/src/wgpu_renderer.rs` includes unit tests (`webgl_shader_is_valid_wgsl_without_storage_buffers`, `storage_buffer_shader_is_valid_wgsl`, `subpixel_shader_is_valid_wgsl`, `webgl_record_sizes_match_shader_word_strides`) that validate every concatenated WGSL variant with `naga::front::wgsl::parse_str` + `naga::valid::Validator` and assert Rust struct sizes equal the exact `u32` word strides hardcoded in `shaders_webgl.wgsl`. Whenever you modify a primitive struct in `gpui` or `shaders*.wgsl`, update both transports and run `cargo test -p gpui_wgpu`!

---

## 2. `crates/nsh-core` & `crates/nsh-engine` Custom WGSL Shaders

NSH (`Native Shell`) provides a safe, reactive custom shader system for views, backdrop/content effects, and page/element transitions.

### How `nsh_core::shader::Shader::compile` Works (`crates/nsh-core/src/shader.rs`)
1. **App Authoring Contract**:
   Apps write `fn shade(uv: vec2f) -> vec4f` returning straight (un-premultiplied) sRGB + alpha:
   ```wgsl
   struct Uniforms {
       accent: vec4f,
       intensity: f32,
   }

   fn shade(uv: vec2f) -> vec4f {
       let base = sample_content(uv);
       let wave = 0.5 + 0.5 * sin(uv.x * 12.0 + nsh.time * 2.0);
       return mix(base, uniforms.accent, wave * uniforms.intensity * nsh.progress);
   }
   ```
2. **Built-in Uniforms & Inputs Injected by `nsh-core`**:
   - `@group(0) @binding(0) var<uniform> nsh: Nsh;` (`32` bytes: `size: vec2f`, `time: f32`, `scale: f32`, `progress: f32`, `@size(12) overflow: f32`).
   - `@group(0) @binding(1) var<uniform> uniforms: Uniforms;` (injected iff `struct Uniforms` is declared; up to `4096` bytes; allowed member types: `f32`, `i32`, `u32`, `vec2f`, `vec3f`, `vec4f`, `array<vec4f, N>`).
   - `@group(0) @binding(2) var nsh_sampler: sampler;` + `@binding(3..=6)` `nsh_content`, `nsh_backdrop`, `nsh_from`, `nsh_to` (`texture_2d<f32>`, accessed via `sample_content(uv)`, `sample_backdrop(uv)`, `sample_from(uv)`, `sample_to(uv)` using `textureSampleLevel(..., 0.0)`).
   - `nsh_vertex` (draws one procedural fullscreen triangle `0..3` expanded by `nsh.overflow`) and `nsh_fragment` (clamps `shade(in.uv)` to `0..1` and premultiplies RGB by alpha).
3. **Compile-Time Reflection via Naga**:
   - Validates with `Validator::new(ValidationFlags::all(), Capabilities::empty())` so every valid NSH shader is guaranteed portable across Vulkan, DX12, Metal, OpenGL, WebGPU, and WebGL2.
   - Inspects Naga IR expressions to set `shader.animated()` (`true` iff `nsh.time` is accessed — static shaders render only once when size/uniforms change!) and `shader.inputs()` (which `ShaderInput` textures `shade` transitively samples).
   - Translates to GLSL ES 300 (`#version 300 es`) via `naga::back::glsl` (`Version::Embedded { version: 300, is_webgl: true }`) for `nsh-platform-web`.

### How `nsh_engine::shader::Renderer` Executes Shaders (`crates/nsh-engine/src/shader.rs`)
- Lazily opens one `wgpu::Device` on first shader mount (`MemoryHints::MemoryUsage`, `max_non_sampler_bindings: 65_536`).
- Chooses backend order per OS (`GL` then `PRIMARY` on Linux to avoid slow uncached write-combined Vulkan `MAP_READ` stalls on Intel ANV; `DX12` first on Windows for ~5ms startup; `PRIMARY | GL` on Apple).
- Renders into an offscreen target capped at `MAX_SIDE` pixels with **double-buffered non-blocking `MAP_READ` readback** (`[Readback; 2]` with atomic states `IDLE -> MAPPING -> MAPPED`), or zero-copy `IOSurface`s on Apple (`show_on_surfaces()`), never stalling the main UI thread.
