# aphrody Unified Plugin Documentation Index

Central reference hub for the `aphrody` plugin (`.agents/plugins/aphrody/`).

## Skills Directory (`skills/`)

| Domain | Skill | Purpose |
| --- | --- | --- |
| **Zed Studio** | [`zed`](../skills/zed/SKILL.md), [`zed-cherry-pick`](../skills/zed-cherry-pick/SKILL.md) | 100% Zed compat, WIT extensions, CLI, upstream cherry-picks |
| **GPUI Core** | [`gpui`](../skills/gpui/SKILL.md), [`gpui-test`](../skills/gpui-test/SKILL.md), [`gpui-bench`](../skills/gpui-bench/SKILL.md), [`lint-creator`](../skills/lint-creator/SKILL.md) | Reactive entities, custom elements, Taffy layout, deterministic tests & Criterion benches, custom dylints |
| **Native Shell (NSH)** | [`nsh`](../skills/nsh/SKILL.md), [`nsh-design-guides`](../skills/nsh-design-guides/SKILL.md), [`bun-gpui`](../skills/bun-gpui/SKILL.md) | Hybrid Rust/TSX UI runtime, shaders, animations, Bun submodule docs |
| **GPUI Kit Components** | [`gpui-kit`](../skills/gpui-kit/SKILL.md), [`gpui-kit-design-guides`](../skills/gpui-kit-design-guides/SKILL.md) | `longbridge/gpui-component` widget patterns, design system, 23 GPUI deep references |
| **GPU & Shaders** | [`wgpu`](../skills/wgpu/SKILL.md), [`vulkan`](../skills/vulkan/SKILL.md), [`directx`](../skills/directx/SKILL.md), [`webgpu`](../skills/webgpu/SKILL.md), [`webassembly`](../skills/webassembly/SKILL.md) | Vendored WGPU/Naga v30, `gpui-wgpu` CLI, Vulkan/SPIR-V, DirectX 12/HLSL/DXC, WebGPU/WGSL, Wasm Core 2.0 / WASI P2 / Binaryen |
| **Design & Standards** | [`google-design`](../skills/google-design/SKILL.md), [`stitch-design-md`](../skills/stitch-design-md/SKILL.md), [`modern-web-guidance`](../skills/modern-web-guidance/SKILL.md) | `https://design.google/` M3 Expressive, Google Sans Flex, `DESIGN.md`, Stitch Studio, Baseline/CWV/Lighthouse/shadcn Starter Pack |
| **Hardware IHV & SIMD** | [`nvidia`](../skills/nvidia/SKILL.md), [`amd`](../skills/amd/SKILL.md), [`intel`](../skills/intel/SKILL.md) | NVIDIA Ada `sm_89`/CUDA 13/NVTX3, AMD RDNA 3-4/AGS SDK/RGA, Intel Arc Xe/AVX-512/Level Zero/OpenCL/ISPC |
| **Brand & GitHub Ops** | [`aphrody-brand`](../skills/aphrody-brand/SKILL.md), [`github-ops`](../skills/github-ops/SKILL.md) | `product.toml` -> `crates/brand` + `xtask brand`, GitHub Wiki, upstream sync, auto-merge |

## Bundled Reference Trees (`docs/`)

- **Zed Development & Extensions**: [`docs/zed/SUMMARY.md`](zed/SUMMARY.md), [`docs/zed/development/ui-checklist.md`](zed/development/ui-checklist.md), [`docs/zed/extensions.md`](zed/extensions.md), [`docs/conventions/CONVENTIONS.md`](conventions/CONVENTIONS.md)
- **WGPU & Naga v30 (`llms.txt` / `llms-full.txt`)**: [`docs/wgpu/llms.txt`](wgpu/llms.txt), [`docs/wgpu/llms-full.txt`](wgpu/llms-full.txt)
- **Native Shell (NSH)**: [`docs/nsh/README.md`](nsh/README.md), [`docs/nsh/CONTEXT.md`](nsh/CONTEXT.md), [`docs/nsh/dev/`](nsh/dev)
- **GPUI Kit (`longbridge/gpui-component`)**: [`docs/gpui-kit/README.md`](gpui-kit/README.md), [`docs/gpui-kit/ARCHITECTURE.md`](gpui-kit/ARCHITECTURE.md), [`docs/gpui-kit/STYLING-AND-MOTION.md`](gpui-kit/STYLING-AND-MOTION.md), [`docs/gpui-kit/gpui-shell.md`](gpui-kit/gpui-shell.md)
- **GPU Backend Specs & Examples**: [`../../docs/gpu/`](../../../../docs/gpu) (`vulkan`, `directx`, `webgpu`, `nvidia`, `amd`, `intel`) + [`../../examples/gpu/`](../../../../examples/gpu)
- **Aphrody GPUI OS Crates & Manifests**: [`os_session`](../../../../crates/os_session/src/os_session.rs), [`os_ui`](../../../../crates/os_ui/src/os_ui.rs), [`os_craft`](../../../../crates/os_craft/src/os_craft.rs), [`os_desktop`](../../../../crates/os_desktop/src/os_desktop.rs), [`assets/os/`](../../../../assets/os)
- **Architecture & Production Blueprints**: [`docs/architecture/README.md`](../../../../docs/architecture/README.md) (Monorepo Prod Guide, Unified Wasm/WebGPU, Skills/MCP/ACP/A2A, Code-to-Docs Website)
- **Bun Runtime & Upstream Docs**: [`docs/bun/landing-prs.md`](bun/landing-prs.md), [`../../../../vendor/bun/docs/`](../../../../vendor/bun/docs)
