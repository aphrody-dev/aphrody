# HLSL, DirectX Shader Compiler (DXC) & Naga Backend

This reference guide details High-Level Shading Language (HLSL) development, DXC compiler toolchains, bytecode validation, Wave Intrinsics, and Naga's cross-compilation mechanics in `gpui-os`.

---

## 1. DXC Toolchain vs Legacy FXC

`gpui-os` uses the latest official Microsoft DirectX Shader Compiler (**DXC v1.9.2609.5**, released September 2026), installed at:
- `~/.local/bin\dxc.exe`
- `~/.local/bin\dxcompiler.dll`
- `~/.local/bin\dxil.dll`

### Comparison Table: FXC vs DXC

| Feature | Legacy FXC (`fxc.exe`) | Modern DXC (`dxc.exe`) |
| :--- | :--- | :--- |
| **Compiler Backend** | Proprietary legacy AST optimizer | LLVM Clang-based optimizer |
| **Output Container** | DXBC (DirectX Bytecode) | DXIL (DirectX Intermediate Language, LLVM 3.7 bitcode) |
| **Validation** | Hardcoded legacy validator | Signed DXIL container validated by `dxil.dll` |
| **Max Shader Model** | Shader Model 5.1 (`cs_5_1`, `ps_5_1`) | Shader Model 6.9 (`cs_6_9`, `lib_6_9`) |
| **Wave Intrinsics** | Unsupported | Full support (`WaveActiveSum`, `WavePrefixSum`, etc.) |
| **16-bit Float / Int** | Emulated via 32-bit arithmetic | Native hardware FP16 / INT16 arithmetic (`-enable-16bit-types`) |
| **Dynamic Indexing** | Restricted table binding | Unbounded descriptor arrays & bindless resources |
| **Raytracing / Mesh** | Unsupported | DXR 1.0/1.1 (`lib_6_x`), Mesh Shaders (`ms_6_5`), Work Graphs |

---

## 2. Shader Model Evolution (SM 6.0 – SM 6.9)

| Shader Model | Key Capabilities | Minimum Hardware / Agility Version |
| :--- | :--- | :--- |
| **SM 6.0** | Wave Intrinsics (`WaveActive*`, `WavePrefix*`, `WaveReadLaneFirst`), 64-bit integers. | Windows 10 1607+ |
| **SM 6.2** | Native 16-bit float (`half`) and integer types (`int16_t`, `uint16_t`). | Windows 10 1809+ |
| **SM 6.4** | Integer Dot Product (`dot4add_u8packed`), low-precision INT8 ML inference primitives. | Windows 10 1903+ |
| **SM 6.5** | DirectX Raytracing (DXR) Tier 1.1, Mesh & Amplification Shaders (`as_6_5`, `ms_6_5`). | Windows 10 2004+ |
| **SM 6.6** | Dynamic Resource Indexing (`ResourceDescriptorHeap`), 64-bit integer atomics (`InterlockedAdd64`), payload access, bit manipulation. | Windows 11 / Agility SDK 1.600+ |
| **SM 6.7** | Advanced Texture Operations (quad any/all, raw/structured buffer integer packing). | Windows 11 22H2 / Agility SDK 1.606+ |
| **SM 6.8** | Work Graphs (`NodeLaunch`), Wave Size Ranges (`[wavesize(32, 64)]`), Expanded Comparison Samplers. | Agility SDK 1.711+ |
| **SM 6.9** | Advanced memory fences, cooperative tensor matrix primitives, ultra-wide atomic operations. | Agility SDK 1.722+ |

---

## 3. Recommended DXC Compiler Invocations

### Compiling Compute Shaders (SM 6.6):
```powershell
dxc.exe -T cs_6_6 -E main -O3 -Qstrip_reflect -Fo compute.dxil compute.hlsl
```

### Compiling Vertex and Pixel Shaders (SM 6.6):
```powershell
# Vertex Shader
dxc.exe -T vs_6_6 -E vs_main -O3 -Fo quad_vs.dxil gpui_quad.hlsl

# Pixel Shader
dxc.exe -T ps_6_6 -E ps_main -O3 -Fo quad_ps.dxil gpui_quad.hlsl
```

### Critical Flags Reference:
- `-T <target>`: Specifies target profile (`cs_6_6`, `vs_6_6`, `ps_6_6`, `lib_6_6`).
- `-E <name>`: Entry point function name.
- `-O3`: Maximum LLVM optimization pass (inlines functions, unrolls loops, vectorizes loads).
- `-Zi`: Generates debugging symbols into PDB.
- `-Qstrip_reflect`: Strips reflection data from binary DXIL blob for minimal memory footprint.
- `-all-resources-bound`: Guarantees driver no null descriptors are bound, enabling aggressive loop invariant code motion.
- `-enable-16bit-types`: Enables true native 16-bit scalar and vector execution.

---

## 4. Constant Buffer (CBV) Alignment Rules

In DirectX 12, Constant Buffers must adhere to two strict alignment rules:

1. **Host-to-Device GPU Virtual Address Alignment**:
   - `D3D12_CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT` is **256 bytes**.
   - Any CBV created with `CreateConstantBufferView` or dynamic root descriptor binding must point to a GPU address that is a multiple of `256`.
   - Size must also be a multiple of `256` bytes (`size = (raw_size + 255) & ~255`).

2. **Internal HLSL Struct Packing (16-Byte Vector Boundary)**:
   - Within a `cbuffer`, HLSL packs data into 4-component 32-bit vectors (16 bytes per chunk).
   - An individual field cannot straddle a 16-byte boundary unless it starts at the beginning of a 16-byte vector.
   - For instance, a `float3` followed by a `float2` will force the `float2` into the next 16-byte vector, leaving 4 bytes of padding.

```hlsl
cbuffer QuadUniforms : register(b0) {
    float4 u_color;       // offset 0  (16 bytes)
    float2 u_origin;      // offset 16 (8 bytes)
    float2 u_size;        // offset 24 (8 bytes)
    float  u_border_width;// offset 32 (4 bytes)
    float3 _pad;          // offset 36 (12 bytes padding to complete 16-byte boundary)
};
```

---

## 5. Wave Intrinsics (Prefix Sum & Warp Reductions)

Wave Intrinsics operate across the GPU's execution unit (e.g. 32 threads on NVIDIA RTX 4070, 32 or 64 on AMD RDNA3) without accessing shared memory or issuing barrier synchronization:

```hlsl
// prefix_sum.hlsl (Shader Model 6.6)
RWStructuredBuffer<uint> g_InputOutput : register(u0);

[numthreads(32, 1, 1)]
void main(uint3 dispatchThreadId : SV_DispatchThreadID) {
    uint index = dispatchThreadId.x;
    uint value = g_InputOutput[index];

    // Compute exclusive scan across all 32 lanes in the wave
    uint wavePrefix = WavePrefixSum(value);

    // Write back prefix sum
    g_InputOutput[index] = wavePrefix;
}
```

### Essential Wave Primitives:
- `WaveIsFirstLane()`: True only on the active lane with the smallest `SV_GroupIndex`. Ideal for allocating wave-wide resources.
- `WaveGetLaneIndex()`: Returns index of current thread within wave `[0..WaveGetLaneCount() - 1]`.
- `WaveActiveSum(val)`: Returns sum of `val` across all active lanes in the wave.
- `WavePrefixSum(val)`: Returns exclusive prefix sum of `val` for preceding active lanes.
- `WaveReadLaneFirst(val)`: Broadcasts the value from the first active lane to all active lanes.

---

## 6. Naga HLSL Translation Architecture (`naga::back::hlsl`)

In `gpui-os`, WGSL shaders are transpiled to HLSL via `naga::back::hlsl`. Key lowering mechanisms:

1. **Storage Buffers as `ByteAddressBuffer` / `RWByteAddressBuffer`**:
   - WebGPU storage buffers allow arbitrary structured read/write.
   - Naga lowers them to HLSL `ByteAddressBuffer` (read-only) or `RWByteAddressBuffer` (read-write), emitting `.Load(...)` and `.Store(...)` at computed byte offsets.

2. **Matrix Layout Transposition**:
   - WGSL matrices in uniforms are column-major (`m[col][row]`).
   - HLSL default matrix indexing accesses rows (`m[row][col]`).
   - Naga handles this by declaring uniform matrices as `row_major` and transposing the dimensions, so `m[col]` matches directly without manual scalar reassembly.

3. **`matCx2` Padding Helpers**:
   - HLSL packs matrix rows to 16-byte boundaries. A 2-row matrix in HLSL (`row_major floatKx2`) would have padding that WGSL does not have.
   - Naga emits synthesized struct types (`float2 m_0; float2 m_1;`) with helper functions `GetMatmOnBaz(...)` to preserve exact WebGPU memory layout.
