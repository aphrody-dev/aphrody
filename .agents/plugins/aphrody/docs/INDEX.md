# aphrody Unified Plugin Documentation Index

Central reference hub for the `aphrody` plugin (`.agents/plugins/aphrody/`).

## Skills Directory (`skills/`)

| Domain | Skill | Purpose |
| --- | --- | --- |
| **Zed Studio** | `fw: plugins/aphrody/skills/zed`, `fw: plugins/aphrody/skills/zed-cherry-pick` | 100% Zed compat, WIT extensions, CLI, upstream cherry-picks |
| **GPUI Core** | `fw: plugins/aphrody/skills/gpui`, [`gpui-test`](../skills/gpui-test/SKILL.md), [`gpui-bench`](../skills/gpui-bench/SKILL.md), `fw: plugins/aphrody/skills/lint-creator` | Reactive entities, custom elements, Taffy layout, deterministic tests & Criterion benches, custom dylints |
| **Native Shell (NSH)** | `fw: plugins/aphrody/skills/nsh`, [`nsh-design-guides`](../skills/nsh-design-guides/SKILL.md), [`bun-gpui`](../skills/bun-gpui/SKILL.md) | Hybrid Rust/TSX UI runtime, shaders, animations, Bun docs |
| **GPUI Kit Components** | [`gpui-kit`](../skills/gpui-kit/SKILL.md), [`gpui-kit-design-guides`](../skills/gpui-kit-design-guides/SKILL.md) | `longbridge/gpui-component` widget patterns, design system, 23 GPUI deep references |
| **GPU & Shaders** | `fw: plugins/aphrody/skills/wgpu`, `fw: plugins/aphrody/skills/vulkan`, `fw: plugins/aphrody/skills/directx`, `fw: plugins/aphrody/skills/webgpu`, `fw: plugins/aphrody/skills/webassembly` | Vendored WGPU/Naga v30, `gpui-wgpu` CLI, Vulkan/SPIR-V, DirectX 12/HLSL/DXC, WebGPU/WGSL, Wasm Core 2.0 / WASI P2 / Binaryen |
| **Design & Standards** | [`google-design`](../skills/google-design/SKILL.md), `fw: plugins/aphrody/skills/stitch-design-md`, [`aphrody-web-guidance`](../skills/aphrody-web-guidance/SKILL.md) | `https://design.google/` M3 Expressive, Google Sans Flex, `DESIGN.md`, Stitch Studio, Baseline/CWV/Lighthouse/shadcn Starter Pack |
| **Hardware IHV & SIMD** | `fw: plugins/aphrody/skills/nvidia`, `fw: plugins/aphrody/skills/amd`, `fw: plugins/aphrody/skills/intel` | NVIDIA Ada `sm_89`/CUDA 13/NVTX3, AMD RDNA 3-4/AGS SDK/RGA, Intel Arc Xe/AVX-512/Level Zero/OpenCL/ISPC |
| **Brand & GitHub Ops** | `fw: plugins/aphrody/skills/aphrody-brand`, `fw: plugins/aphrody/skills/github-ops` | `product.toml` -> `crates/brand` + `xtask brand`, GitHub Wiki, upstream sync, auto-merge |

## Reference Trees

- **Zed Development & Extensions**: [`docs/src/SUMMARY.md`](../../../../docs/src/SUMMARY.md), [`docs/zed/development/ui-checklist.md`](zed/development/ui-checklist.md) (Aphrody checklist), [`docs/src/extensions.md`](../../../../docs/src/extensions.md), [`docs/.conventions/CONVENTIONS.md`](../../../../docs/.conventions/CONVENTIONS.md)
- **WGPU & Naga v30 (`llms.txt` / `llms-full.txt`)**: [`docs/aphrody/wgpu/llms.txt`](../../../../docs/aphrody/wgpu/llms.txt), [`docs/aphrody/wgpu/llms-full.txt`](../../../../docs/aphrody/wgpu/llms-full.txt)
- **Native Shell (NSH)**: [`docs/aphrody/nsh/README.md`](../../../../docs/aphrody/nsh/README.md), [`docs/aphrody/nsh/CONTEXT.md`](../../../../docs/aphrody/nsh/CONTEXT.md), [`docs/aphrody/nsh/dev/`](../../../../docs/aphrody/nsh/dev)
- **GPUI Kit (`longbridge/gpui-component`)**: [`docs/gpui-kit/README.md`](gpui-kit/README.md), [`docs/gpui-kit/ARCHITECTURE.md`](gpui-kit/ARCHITECTURE.md), [`docs/gpui-kit/STYLING-AND-MOTION.md`](gpui-kit/STYLING-AND-MOTION.md), [`docs/gpui-kit/gpui-shell.md`](gpui-kit/gpui-shell.md)
- **GPU Backend Specs & Examples**: [`docs/aphrody/gpu/`](../../../../docs/aphrody/gpu) (`vulkan`, `directx`, `webgpu`, `nvidia`, `amd`, `intel`) + [`../../examples/gpu/`](../../../../examples/gpu)
- **Aphrody GPUI OS Crates & Manifests**: [`aphrody_session`](../../../../crates/aphrody_session/src/aphrody_session.rs), [`aphrody_ui`](../../../../crates/aphrody_ui/src/aphrody_ui.rs), [`aphrody_craft`](../../../../crates/aphrody_craft/src/aphrody_craft.rs), [`aphrody_desktop`](../../../../crates/aphrody_desktop/src/aphrody_desktop.rs), [`assets/os/`](../../../../assets/os)
- **Architecture & Production Blueprints**: [`docs/aphrody/architecture/README.md`](../../../../docs/aphrody/architecture/README.md) (Monorepo Prod Guide, Unified Wasm/WebGPU, Skills/MCP/ACP/A2A, Code-to-Docs Website)
- **Bun Runtime & De-vendored Reference Ecosystem**: [`docs/bun/landing-prs.md`](bun/landing-prs.md), `fw: plugins/aphrody/skills/vendor-ecosystem`, [`bun-gpui` skill](../skills/bun-gpui/SKILL.md)
