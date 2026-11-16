# DirectX 12 & HLSL Architecture Guide

Welcome to the DirectX 12 GPU Architecture documentation for `gpui-os`. This guide documents the native Windows graphics and compute infrastructure, compiler toolchains, Agility SDK runtime, and WGPU D3D12 backend.

---

## 1. Directory Structure

```
gpui-os/
├── .agents/plugins/gpui-os/skills/directx/
│   ├── SKILL.md                          # DirectX 12 skill definition
│   └── references/
│       ├── d3d12-architecture.md         # Enhanced Barriers, Root Signatures, Descriptors
│       ├── hlsl-dxc-and-naga.md          # DXC flags, SM 6.0–6.9, Wave Intrinsics, Naga HLSL
│       └── gpui-and-wgpu-dx12.md         # gpui_windows vs wgpu-hal dx12, DComp, DRED
├── docs/gpu/directx/
│   └── README.md                         # This architecture document
├── crates/gpui_windows/                  # Native Win32 / DirectWrite / DirectX renderer
│   ├── src/directx_renderer.rs           # Direct3D swapchain & render passes
│   ├── src/directx_devices.rs            # DXGI factory & D3D device initialization
│   ├── src/directx_atlas.rs              # Glyph & texture atlas allocation
│   └── src/shaders.hlsl                  # Direct2D/DirectWrite subpixel correction shaders
├── crates/wgpu_cli/                      # Hardware diagnostics, validation & transpile CLI
│   └── examples/directx_backend_probe.rs # Headless DX12 compute & Naga SM 6.6 verification
└── examples/gpu/directx/                 # Pure HLSL & WGSL reference shaders + verify script
    ├── prefix_sum.hlsl                   # Wave Intrinsics compute shader (SM 6.6)
    ├── gpui_quad.hlsl                    # Direct2D quad rendering vertex & pixel shaders
    ├── wave_field.wgsl                   # WGSL wave simulation compute shader
    └── verify.ps1                        # Shader compilation & DX12 adapter test script
```

---

## 2. Verified Toolchains & Paths

All modern DirectX 12 tooling is installed locally in the user profile:

- **DXC (DirectX Shader Compiler)**: `v1.9.2609.5`
  - Binary: `~/.local/bin\dxc.exe`
  - Libraries: `dxcompiler.dll`, `dxil.dll`
  - Targets: `cs_6_6`, `vs_6_6`, `ps_6_6`, `lib_6_6`, `ms_6_6`, `as_6_6`
- **Legacy FXC**: `10.1`
  - Binary: `C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\fxc.exe`
  - Targets: `cs_5_0`, `vs_5_0`, `ps_5_0`
- **Agility SDK (D3D12)**: `1.722.1-preview` & `1.620.2`
  - Runtimes: `~/.local/sdk\directx\agility\bin\x64\D3D12Core.dll`
  - Config CLI: `~/.local/bin\d3dconfig.exe`
- **DirectX-Headers**: `v1.620.2`
  - Headers: `~/.local/sdk\directx\headers\include\directx\`
- **WinPixEventRuntime**: `1.0.240308001`
  - Library: `~/.local/sdk\directx\pix\bin\x64\WinPixEventRuntime.dll`

---

## 3. Shading Language & Compiler Matrix

```
       +-----------------------+
       |   WGSL Shader Source  |
       +-----------------------+
                   |
                   | (gpui-wgpu transpile --target hlsl)
                   v
       +-----------------------+
       |   Naga HLSL Backend   | (naga::back::hlsl::ShaderModel::V6_6)
       +-----------------------+
                   |
                   | (dxc.exe -T cs_6_6 -E main)
                   v
       +-----------------------+
       |  Signed DXIL Bytecode | (Validated by dxil.dll)
       +-----------------------+
                   |
                   | (D3D12CreateComputePipelineState)
                   v
       +-----------------------------------------------+
       | Direct3D 12 Hardware Pipeline (RTX 4070 Ada)  |
       +-----------------------------------------------+
```

---

## 4. Verification & Testing

To compile all example shaders and verify DX12 hardware enumeration:

```powershell
powershell -ExecutionPolicy Bypass -File examples/gpu/directx/verify.ps1
```

To probe the D3D12 adapter and run an offscreen compute pass via Rust:

```powershell
# Run by the coordinator via cargo:
cargo run -p wgpu_cli --example directx_backend_probe
```
