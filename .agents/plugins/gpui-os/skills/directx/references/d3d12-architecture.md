# DirectX 12 Architecture & Pipeline Engineering

This document specifies the DirectX 12 (D3D12) graphics architecture, hardware capabilities, resource management, and synchronization protocols implemented across `gpui-os`.

---

## 1. Hardware Baseline: NVIDIA GeForce RTX 4070 (Feature Level 12_2 Ultimate)

The primary reference GPU for `gpui-os` on Windows is the **NVIDIA GeForce RTX 4070** (Ada Lovelace, sm_89):

- **Direct3D Feature Level**: `D3D_FEATURE_LEVEL_12_2` (DirectX 12 Ultimate).
- **Shader Model**: Shader Model 6.6 native, extending up to SM 6.9 with Agility SDK preview runtimes.
- **Wave / Subgroup Size**: Native `32` threads per warp.
- **Hardware Features**:
  - DirectX Raytracing (DXR) Tier 1.1 (inline ray tracing + dynamic state objects).
  - Mesh & Amplification Shaders (`AS` / `MS`) replacing traditional input assembler, hull, and domain stages.
  - Variable Rate Shading (VRS) Tier 2 (per-draw, per-primitive, screenspace image-based shading rates).
  - Sampler Feedback (min/mip streaming and texture space shading).
  - Work Graphs (GPU-driven autonomous compute work generation).
- **Secondary / Fallback Adapter**: Microsoft Basic Render Driver (`WARP`), Device ID `0x008c`, Vendor `0x1414`, CPU-based JIT rasterizer with SM 6.6 support.

---

## 2. Agility SDK Integration & Export Symbols

The Agility SDK decouples DirectX 12 runtime updates from Windows OS upgrades. In `gpui-os`, executable targets configure the Agility SDK by exporting two well-known global symbols:

```c
// C / C++ exports
#include <d3d12.h>

extern "C" {
    __declspec(dllexport) extern const UINT D3D12SDKVersion = 722; // 1.722.1-preview or 620 for 1.620.2
    __declspec(dllexport) extern const char* D3D12SDKPath = ".\\D3D12\\";
}
```

In Rust (`gpui_windows` or standalone binaries), this is declared via `#[no_mangle]`:

```rust
#[no_mangle]
pub static D3D12SDKVersion: u32 = 722;

#[no_mangle]
pub static D3D12SDKPath: &[u8; 10] = b".\\D3D12\\\0";
```

### Configuring the Agility SDK via `d3dconfig.exe`
Installed at `~/.local/bin\d3dconfig.exe`, `d3dconfig` manages driver hooks, debug layers, and validation without registry tampering:

```powershell
# Enable D3D12 Debug Layer for an application
d3dconfig apps --add gpui-wgpu.exe
d3dconfig debug-layer debug-layer-mode=force-on
d3dconfig debug-layer gbv-mode=force-on # GPU-Based Validation

# Configure DRED (Device Removed Extended Data) breadcrumbs
d3dconfig dred --force-on-all
```

---

## 3. Root Signatures & The 64-DWORD Budget

The D3D12 Root Signature defines the binding contract between shader stages and memory descriptors. The hardware root signature register space has a strict maximum size of **64 DWORDs** (256 bytes):

| Root Parameter Type | Size in DWORDs | Performance Cost | Recommended Use Case |
| :--- | :---: | :---: | :--- |
| **Root Constants (`32BitConstants`)** | 1 DWORD per 32-bit value | Lowest latency (direct in GPU registers) | Per-draw push constants, frame indices, viewport dims |
| **Root Descriptors (`CBV`, `SRV`, `UAV`)** | 2 DWORDs (64-bit GPU virtual address) | Low latency (single indirection) | Primary per-frame uniform buffer, unbounded storage buffer |
| **Descriptor Table** | 1 DWORD per table pointer | Moderate latency (double indirection) | Batched textures, bindless resource arrays, samplers |

### Best Practices:
1. Put the most frequently changed parameters at the lowest root parameter indices (index 0, index 1) to minimize driver root state revalidation overhead.
2. In `wgpu-hal/dx12`, immediate constants are mapped directly to `D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS`.

---

## 4. Descriptor Heaps & The 2048-Sampler Limit

D3D12 hardware enforces two fundamentally distinct heap types:

1. **CBV / SRV / UAV Descriptor Heaps**:
   - Hardware tier 1: 1,000,000 descriptors visible simultaneously.
   - Used for textures, storage buffers, and uniform buffers.
2. **Sampler Descriptor Heaps**:
   - **Hardware Limit: Exactly 2048 descriptors** visible simultaneously in a shader-visible heap.
   - Any attempt to allocate more than 2048 sampler descriptors in a single shader-visible heap fails with `E_INVALIDARG`.

### The 2048-Sampler Solution in `gpui-os` / `wgpu-hal`
Because an application or UI framework may define thousands of pipelines with independent sampler state, `wgpu-hal` and `naga::back::hlsl` solve this limit via heap virtualization:
- The entire global sampler descriptor heap is bound at pipeline setup.
- Instead of binding individual samplers directly to registers, the shader accesses the sampler heap via an index.
- A dynamic small uniform buffer (the *Sampler Index Buffer*) stores the actual heap offset for each bind group's sampler slots.
- Shaders index into `SamplerState g_Samplers[] : register(s0, space0)` using the fetched offset.

---

## 5. Enhanced Barriers (`ID3D12GraphicsCommandList7`)

Traditional D3D12 `ResourceBarrier` (`D3D12_RESOURCE_BARRIER_TYPE_TRANSITION`) suffered from high driver overhead, subresource iteration costs, and coarse pipeline stalls.

Enhanced Barriers (introduced in Agility SDK and Windows 11) replace transition barriers with fine-grained memory and layout synchronization:

```cpp
D3D12_TEXTURE_BARRIER textureBarrier = {};
textureBarrier.SyncBefore = D3D12_BARRIER_SYNC_RENDER_TARGET;
textureBarrier.SyncAfter  = D3D12_BARRIER_SYNC_PIXEL_SHADING;
textureBarrier.AccessBefore = D3D12_BARRIER_ACCESS_RENDER_TARGET;
textureBarrier.AccessAfter  = D3D12_BARRIER_ACCESS_SHADER_RESOURCE;
textureBarrier.LayoutBefore = D3D12_BARRIER_LAYOUT_RENDER_TARGET;
textureBarrier.LayoutAfter  = D3D12_BARRIER_LAYOUT_SHADER_RESOURCE;
textureBarrier.pResource    = pBackBuffer;
textureBarrier.Subresources = CD3DX12_BARRIER_SUBRESOURCE_RANGE(0, 1, 0, 1);

D3D12_BARRIER_GROUP group = {};
group.Type = D3D12_BARRIER_TYPE_TEXTURE;
group.NumBarriers = 1;
group.pTextureBarriers = &textureBarrier;

pCommandList7->Barrier(1, &group);
```

### Key Differences:
- **`SyncBefore` / `SyncAfter`**: Defines the exact pipeline stage boundary (e.g. `COMPUTE_SHADING`, `COPY`, `RESOLVE`).
- **`AccessBefore` / `AccessAfter`**: Specifies the memory cache coherency requirement (e.g. `SHADER_RESOURCE`, `UNORDERED_ACCESS`).
- **`LayoutBefore` / `LayoutAfter`**: Controls physical hardware compression tiling without forcing cache flushes if the layout does not change.

---

## 6. DXGI Flip-Model Swapchains (`FLIP_DISCARD`)

For low-latency UI rendering and zero-tearing presentation, `gpui-os` strictly requires the DXGI Flip model:

```rust
let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
    Width: width,
    Height: height,
    Format: DXGI_FORMAT_R8G8B8A8_UNORM, // or DXGI_FORMAT_B8G8R8A8_UNORM
    Stereo: false.into(),
    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
    BufferCount: 3, // Triple buffering
    Scaling: DXGI_SCALING_NONE,
    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
    AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED, // For transparent DComp windows
    Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING.0 as u32,
};
```

- **`DXGI_SWAP_EFFECT_FLIP_DISCARD`**: The DWM compositor directly scans out or composes the back buffer without blitting, freeing CPU and GPU memory bandwidth.
- **Tearing Support**: When `DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING` is set and `SyncInterval = 0`, VRR (G-Sync / FreeSync) displays achieve zero-latency input without tearing artifacts.
