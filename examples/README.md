# Examples & Starter Templates Organization (Zed Upstream Layout)

Aphrody follows `zed-industries/zed`'s canonical separation between **crate-owned Rust examples**, **standalone project templates**, and **multi-language / GPU shader corpora**:

## 1. Rust & GPUI Crate Examples (`crates/<crate>/examples/*.rs`)

Matching `crates/gpui/examples/*.rs` in Zed upstream, runnable Rust examples live inside the crate they exercise:

- **GPUI Core & Web**: [`crates/gpui/examples/`](../crates/gpui/examples/) (`cargo xtask web-examples`)
- **GPUI App Shell**: [`crates/gpui_app_shell/examples/studio_starter.rs`](../crates/gpui_app_shell/examples/studio_starter.rs)
- **Aphrody OS Web**: [`crates/os_desktop/examples/web_os.rs`](../crates/os_desktop/examples/web_os.rs)
- **WGPU & Naga Pipeline**: [`crates/wgpu_cli/examples/webgpu_naga_pipeline.rs`](../crates/wgpu_cli/examples/webgpu_naga_pipeline.rs)
- **`egui 0.37` + Google Design Bridge**: [`crates/egui_gpui/examples/google_design_egui.rs`](../crates/egui_gpui/examples/google_design_egui.rs)
- **Unreal Bevy 5.8 + Solari GI + MaterialX**: [`crates/bevy_gpui/examples/unreal_solari_showcase.rs`](../crates/bevy_gpui/examples/unreal_solari_showcase.rs)
- **Storytold 82-Repo + Dual-GUI Parity Studio**: [`crates/craft_studio/examples/studio_parity_report.rs`](../crates/craft_studio/examples/studio_parity_report.rs)

## 2. Standalone Project Scaffolding (`cargo xtask new-app` & `templates/`)

- Scaffold a new GPUI / Code / Studio / NSH / Craft Studio app:
  `cargo xtask new-app my-studio --template craft-studio`
- Standalone Wasm32 / WebGPU / WASI Preview 2 templates: [`templates/wasm-wgpu/`](../templates/wasm-wgpu/)

## 3. Native Shell TSX & Vendor GPU Shader Suites (`examples/`)

- [`examples/nsh/`](./nsh/): Cross-platform Native Shell TSX & mixed Rust+TSX applications, including [`examples/nsh/agy-session-studio/`](./nsh/agy-session-studio/).
- [`examples/gpu/`](./gpu/): Vendor-specific WGSL, HLSL (DX12 SM 6.x), SPIR-V (Vulkan 1.3), ISPC, and RDNA/Ada compute shaders verified by [`examples/gpu/verify.ts`](./gpu/verify.ts).
