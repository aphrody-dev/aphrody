# gpui-os Unified Plugin Documentation Index

Central reference hub for the `gpui-os` plugin (`.agents/plugins/gpui-os/`).

## Skills Directory (`skills/`)

| Domain | Skill | Purpose |
| --- | --- | --- |
| **Zed Studio** | [`zed`](../skills/zed/SKILL.md), [`zed-cherry-pick`](../skills/zed-cherry-pick/SKILL.md) | 100% Zed compat, WIT extensions, CLI, upstream cherry-picks |
| **GPUI Core** | [`gpui`](../skills/gpui/SKILL.md), [`gpui-test`](../skills/gpui-test/SKILL.md), [`gpui-bench`](../skills/gpui-bench/SKILL.md), [`lint-creator`](../skills/lint-creator/SKILL.md) | Reactive entities, custom elements, Taffy layout, deterministic tests & Criterion benches, custom dylints |
| **Native Shell (NSH)** | [`nsh`](../skills/nsh/SKILL.md), [`nsh-design-guides`](../skills/nsh-design-guides/SKILL.md), [`bun-gpui`](../skills/bun-gpui/SKILL.md) | Hybrid Rust/TSX UI runtime, shaders, animations, Bun submodule docs |
| **GPUI Kit Components** | [`gpui-kit`](../skills/gpui-kit/SKILL.md), [`gpui-kit-design-guides`](../skills/gpui-kit-design-guides/SKILL.md) | `longbridge/gpui-component` widget patterns, design system, 23 GPUI deep references |
| **GPU & Shaders** | [`wgpu`](../skills/wgpu/SKILL.md), [`vulkan`](../skills/vulkan/SKILL.md), [`directx`](../skills/directx/SKILL.md), [`webgpu`](../skills/webgpu/SKILL.md) | Vendored WGPU/Naga v30, `gpui-wgpu` CLI, Vulkan/SPIR-V, DirectX 12/HLSL/DXC, WebGPU/WGSL |
| **Hardware IHV & SIMD** | [`nvidia`](../skills/nvidia/SKILL.md), [`amd`](../skills/amd/SKILL.md), [`intel`](../skills/intel/SKILL.md) | NVIDIA Ada `sm_89`/CUDA/NVTX3, AMD RDNA3-4/AGS/RGA, Intel Xe/ISPC/Level Zero/OpenCL |
| **Hardware & IHV** | [`nvidia`](../skills/nvidia/SKILL.md), [`amd`](../skills/amd/SKILL.md), [`intel`](../skills/intel/SKILL.md) | Ada Lovelace sm_89/CUDA 13/NVTX3, RDNA 3/Wavefronts/AGS SDK/RGA, Intel Arc Xe/AVX-512/Level Zero/ISPC |
| **Brand & GitHub Ops** | [`aphrody-brand`](../skills/aphrody-brand/SKILL.md), [`github-ops`](../skills/github-ops/SKILL.md) | `product.toml` -> `crates/brand` + `xtask brand`, GitHub Wiki, upstream sync, auto-merge |

## Bundled Reference Trees (`docs/`)

- **Zed Development & Extensions**: [`docs/zed/SUMMARY.md`](zed/SUMMARY.md), [`docs/zed/development/ui-checklist.md`](zed/development/ui-checklist.md), [`docs/zed/extensions.md`](zed/extensions.md), [`docs/conventions/CONVENTIONS.md`](conventions/CONVENTIONS.md)
- **WGPU & Naga v30 (`llms.txt` / `llms-full.txt`)**: [`docs/wgpu/llms.txt`](wgpu/llms.txt), [`docs/wgpu/llms-full.txt`](wgpu/llms-full.txt)
- **Native Shell (NSH)**: [`docs/nsh/README.md`](nsh/README.md), [`docs/nsh/CONTEXT.md`](nsh/CONTEXT.md), [`docs/nsh/dev/`](nsh/dev)
- **GPUI Kit (`longbridge/gpui-component`)**: [`docs/gpui-kit/README.md`](gpui-kit/README.md), [`docs/gpui-kit/ARCHITECTURE.md`](gpui-kit/ARCHITECTURE.md), [`docs/gpui-kit/STYLING-AND-MOTION.md`](gpui-kit/STYLING-AND-MOTION.md), [`docs/gpui-kit/gpui-shell.md`](gpui-kit/gpui-shell.md)
- **GPU Backend Specs & Examples**: [`../../docs/gpu/`](../../../../docs/gpu) (`vulkan`, `directx`, `webgpu`, `nvidia`, `amd`, `intel`) + [`../../examples/gpu/`](../../../../examples/gpu)
- **Bun Runtime & Upstream Docs**: [`docs/bun/landing-prs.md`](bun/landing-prs.md), [`../../../../vendor/bun/docs/`](../../../../vendor/bun/docs)
