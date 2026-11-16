# GPUI Windows & WGPU DirectX 12 Integration

This document describes the relationship between the native Windows platform layer (`crates/gpui_windows`), the WGPU hardware abstraction layer (`vendor/wgpu/wgpu-hal/src/dx12`), and the GPUI WGPU renderer (`crates/gpui_wgpu`).

---

## 1. Dual DirectX Subsystems in `aphrody`

`aphrody` features two parallel, specialized DirectX backends:

```
+-----------------------------------------------------------------------------------------+
|                                    aphrody UI Tree                                      |
+-----------------------------------------------------------------------------------------+
                       |                                                |
                       v                                                v
    +--------------------------------------+      +---------------------------------------+
    |         crates/gpui_windows          |      |           crates/gpui_wgpu            |
    |      (Native Windows Backend)        |      |      (Cross-Platform WGPU Core)       |
    +--------------------------------------+      +---------------------------------------+
    | - D3D11.1 / DXGI 1.6 Device          |      | - WGPU Instance (wgpu::Backends::DX12)|
    | - Direct2D & DirectWrite Integration |      | - vendor/wgpu/wgpu-hal/src/dx12       |
    | - DirectComposition Surface Target   |      | - Naga WGSL -> HLSL SM 6.0–6.6       |
    | - Subpixel Alpha Correction (HLSL)   |      | - Dual-Source Color Blending          |
    | - IDWriteTextLayout Glyph Runs       |      | - Headless Compute & Offscreen Render |
    +--------------------------------------+      +---------------------------------------+
                       \                                                /
                        \                                              /
                         v                                            v
        +-----------------------------------------------------------------------+
        |                  DirectX 12 / DXGI Hardware Driver                    |
        |                 (Hardware Discrete/iGPU + WARP Fallback)              |
        +-----------------------------------------------------------------------+
```

### Subsystem Roles:
1. **`crates/gpui_windows`**:
   - Manages HWND window creation, Win32 message dispatch, IME composition, touch/pen gestures, and DirectComposition visual tree integration.
   - Leverages DirectWrite for native Windows typography, font fallback (Segoe UI Variable, Cascadia Code), and complex text shaping.
   - Renders UI primitives with high-performance subpixel anti-aliasing via HLSL pixel shaders (`alpha_correction.hlsl`, `shaders.hlsl`).

2. **`crates/gpui_wgpu` & `wgpu-hal/dx12`**:
   - Provides portable modern GPU rendering via WGSL shaders and WebGPU semantics.
   - Powers NSH shader views, canvas elements, compute passes, texture atlases (`WgpuAtlas`), and headless testing.
   - Translates WGSL directly into HLSL via `naga::back::hlsl` and executes natively on D3D12 command queues.

---

## 2. DirectComposition & Window Transparency

To achieve fluid, tear-free window resizing with transparent window frames (Mica, Acrylic, or borderless dropshadows), `gpui_windows` binds DXGI swapchains to **DirectComposition**:

```rust
// Create DXGI SwapChain for DirectComposition
let swapchain: IDXGISwapChain1 = unsafe {
    dxgi_factory.CreateSwapChainForComposition(
        &d3d_device,
        &swapchain_desc,
        None,
    )?
};

// Bind SwapChain to DirectComposition Visual Tree
let dcomp_visual: IDCompositionVisual = unsafe { dcomp_device.CreateVisual()? };
unsafe {
    dcomp_visual.SetContent(&swapchain)?;
    dcomp_target.SetRoot(&dcomp_visual)?;
    dcomp_device.Commit()?;
}
```

### Presentation Invariants:
- Format is strictly `DXGI_FORMAT_B8G8R8A8_UNORM` or `DXGI_FORMAT_R8G8B8A8_UNORM`.
- Alpha mode is `DXGI_ALPHA_MODE_PREMULTIPLIED` to ensure proper blending with the desktop background.
- Swap effect is `DXGI_SWAP_EFFECT_FLIP_DISCARD`.

---

## 3. Hybrid GPU Selection (Optimus & Enduro)

On laptops and multi-GPU systems equipped with both an integrated GPU and a discrete GPU, D3D12 allows precise adapter enumeration via `IDXGIFactory6::EnumAdapterByGpuPreference`:

```rust
// High-performance discrete GPU preference
let adapter: IDXGIAdapter1 = unsafe {
    dxgi_factory.EnumAdapterByGpuPreference(
        0,
        DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE,
    )?
};
```

In `gpui_wgpu/src/wgpu_context.rs`, GPU selection is mediated by testing adapter capabilities against the required window surface, prioritizing discrete GPUs with Feature Level 12_2.

---

## 4. Device Removal Recovery & DRED (Device Removed Extended Data)

GPU hangs (TDR — Timeout Detection and Recovery) or driver resets can trigger `DXGI_ERROR_DEVICE_REMOVED` or `DXGI_ERROR_DEVICE_RESET`. Both subsystems implement dedicated recovery loops:

### `gpui_windows` Device Loss Recovery:
In `crates/gpui_windows/src/directx_devices.rs`:
```rust
pub(crate) fn try_to_recover_from_device_lost<T>(mut f: impl FnMut() -> Result<T>) -> Result<T> {
    (0..5)
        .map(|i| {
            if i > 0 {
                // Progressive backoff delay while driver reinitializes
                std::thread::sleep(std::time::Duration::from_millis(100 + i * 10));
            }
            f()
        })
        .find_or_last(Result::is_ok)
        .unwrap()
        .context("DirectXRenderer failed to recover from lost device after multiple attempts")
}
```

### `gpui_wgpu` Device Error State:
In `crates/gpui_wgpu/src/wgpu_context.rs`:
- Tracks `DeviceErrorState` with atomic lost flags.
- Registers `set_device_lost_callback` and `on_uncaptured_error`.
- Automatically disposes lost command queues, clears texture atlases, and re-creates the D3D12 device upon window focus or next frame request.

### DRED Diagnostic Capture:
When debugging crashes, DRED breadcrumbs can be queried via `ID3D12DeviceRemovedExtendedDataSettings`:
```cpp
ID3D12DeviceRemovedExtendedDataSettings1* pDredSettings;
if (SUCCEEDED(D3D12GetDebugInterface(IID_PPV_ARGS(&pDredSettings)))) {
    pDredSettings->SetAutoBreadcrumbsEnablement(D3D12_DRED_ENABLEMENT_FORCED_ON);
    pDredSettings->SetPageFaultEnablement(D3D12_DRED_ENABLEMENT_FORCED_ON);
}
```
If a crash occurs, `device->GetDeviceRemovedReason()` coupled with `ID3D12DeviceRemovedExtendedData` pinpoints the exact command and subresource where the fault happened.
