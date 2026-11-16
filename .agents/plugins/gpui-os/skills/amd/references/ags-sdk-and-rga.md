# AMD GPU Services (AGS) & Radeon GPU Analyzer (RGA) Reference

## 1. AMD GPU Services (AGS) SDK

The AGS SDK provides developers with user-space direct extensions to DirectX 11, DirectX 12, and Vulkan drivers on AMD hardware. In `gpui-os`, AGS is packaged as `amd_ags_x64.dll` alongside the native Windows binaries.

### 1.1 Core API Workflow
```c
#include <amd_ags.h>

AGSContext* agsContext = NULL;
AGSGPUInfo gpuInfo;
AGSReturnCode result = agsInitialize(
    AGS_CURRENT_VERSION,
    NULL, // optional config
    &agsContext,
    &gpuInfo
);

if (result == AGS_SUCCESS) {
    // Inspect GPU architecture and capabilities
    printf("AMD GPU: %s (Architecture: %d, CUs: %d, Wavefront: %d)\n",
           gpuInfo.devices[0].adapterString,
           gpuInfo.devices[0].asicFamily,
           gpuInfo.devices[0].numCUs,
           gpuInfo.devices[0].coreClock);
}

// When creating a D3D12 device with AMD extension intrinsics:
AGSDX12DeviceCreationParams creationParams = {};
AGSDX12ExtensionParams extensionParams = {};
AGSDX12ReturnedParams returnedParams = {};

agsDriverExtensionsDX12_CreateDevice(
    agsContext,
    &creationParams,
    &extensionParams,
    &returnedParams
);

// Cleanup on shutdown:
agsDeInitialize(agsContext);
```

### 1.2 Extension Capabilities Enabled by AGS
- **Shader Intrinsics in HLSL**: `AmdExtD3DShaderIntrinsics` allows compiling HLSL shaders with raw AMD intrinsic opcodes without requiring developer-mode driver signatures.
- **Cross-Lane Communication**: Native `WaveReadLaneAt`, `WaveGetLaneIndex`, `WaveActiveAllTrue`.
- **GPU Timestamp Synchronization**: Sub-microsecond accurate hardware GPU clocks aligned with CPU `QueryPerformanceCounter`.

---

## 2. Radeon GPU Analyzer (`rga`) Offline Compilation

`rga` is AMD's official command-line tool for analyzing shader code across different AMD GPU architectures without needing an AMD GPU installed on the machine.

### 2.1 Target Architectures (`-c <asic>`)
- `gfx1100`: Navi 31 (Radeon RX 7900 XTX / 7900 XT - RDNA 3)
- `gfx1101`: Navi 32 (Radeon RX 7800 XT / 7700 XT)
- `gfx1102`: Navi 33 (Radeon RX 7600)
- `gfx1030`: Navi 21 (Radeon RX 6900 XT / 6800 XT - RDNA 2)
- `gfx1010`: Navi 10 (Radeon RX 5700 XT - RDNA 1)
- `gfx900`: Vega 10 / GCN 5

### 2.2 Analysis Commands
```bash
# 1. List supported ASICs and architectures
rga.exe -s vk-spv-offline -l

# 2. Compile SPIR-V compute shader to RDNA 3 ISA assembly and extract stats
rga.exe -s vk-spv-offline -c gfx1100 --comp compute.spv --isa output_isa.txt --analysis stats.csv

# 3. Live register analysis (VGPR and SGPR pressure per instruction)
rga.exe -s vk-spv-offline -c gfx1100 --comp compute.spv --livereg output_livereg.txt

# 4. Generate Control Flow Graph (CFG) in Graphviz DOT format
rga.exe -s vk-spv-offline -c gfx1100 --comp compute.spv --cfg output_cfg.dot
```

### 2.3 Interpreting Analysis Stats (`stats.csv`)
- **VGPR Count**: Target <= 32 to guarantee maximum concurrent wavefronts per SIMD (Wave32).
- **SGPR Count**: Should not exceed hardware budget (typically 100–108 per wave).
- **LDS Size**: Target <= 32 KB per workgroup to allow 2 or more workgroups to run concurrently per WGP.
- **Scratch Memory Spill**: If VGPR spill size > 0, registers are spilling to VRAM/cache, severely degrading memory bandwidth.
