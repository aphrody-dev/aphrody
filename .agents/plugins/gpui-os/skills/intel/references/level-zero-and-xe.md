# Intel oneAPI Level Zero & Xe GPU Architecture Reference

## 1. Intel Xe GPU Architecture

Intel Xe architecture powers integrated processors and discrete graphics (Arc Alchemist, Battlemage):

- **Xe Core**:
  - The fundamental compute block containing:
    - 16 Vector Engines (XVE): 256-bit or 512-bit SIMD ALUs executing FP32, FP16, INT32, INT8.
    - 16 Matrix Engines (XMX): Dedicated systolic array units executing FP16, BF16, and INT8 matrix multiplies.
    - Large 192 KB shared L1 cache / Shared Local Memory (SLM).
- **Subgroup Sizing**:
  - Hardware executes waves dynamically at SIMD8, SIMD16, or SIMD32 widths.
  - Compute compilers default to SIMD16 or SIMD32 to saturate the 16 XVE vector execution pipelines.

---

## 2. oneAPI Level Zero Core Architecture

Level Zero is Intel's low-overhead bare-metal runtime API providing direct control over GPU hardware, bypassing the overhead of OpenCL or high-level abstraction layers.

### 2.1 Object Hierarchy
```
Driver (ze_driver_handle_t)
 └── Device (ze_device_handle_t)
      ├── Context (ze_context_handle_t)
      ├── Command Queue (ze_command_queue_handle_t)
      ├── Command List (ze_command_list_handle_t)
      ├── Module (ze_module_handle_t) [Compiled from SPIR-V]
      │    └── Kernel (ze_kernel_handle_t)
      └── Memory Allocations (Unified Shared Memory - USM)
```

### 2.2 Core Workflow
```c
#include <level_zero/ze_api.h>

// 1. Initialize driver
zeInit(ZE_INIT_FLAG_GPU_ONLY);

// 2. Discover driver & device
uint32_t driverCount = 1;
ze_driver_handle_t driver;
zeDriverGet(&driverCount, &driver);

uint32_t deviceCount = 1;
ze_device_handle_t device;
zeDeviceGet(driver, &deviceCount, &device);

// 3. Create context & command queue
ze_context_desc_t contextDesc = { ZE_STRUCTURE_TYPE_CONTEXT_DESC, NULL, 0 };
ze_context_handle_t context;
zeContextCreate(driver, &contextDesc, &context);

ze_command_queue_desc_t queueDesc = {
    ZE_STRUCTURE_TYPE_COMMAND_QUEUE_DESC,
    NULL,
    0, // ordinal
    0, // index
    0, // flags
    ZE_COMMAND_QUEUE_MODE_ASYNCHRONOUS,
    ZE_COMMAND_QUEUE_PRIORITY_NORMAL
};
ze_command_queue_handle_t queue;
zeCommandQueueCreate(context, device, &queueDesc, &queue);
```

### 2.3 Unified Shared Memory (USM)
Level Zero supports three tiers of memory allocation:
1. **Device Memory (`zeMemAllocDevice`)**: High-bandwidth allocation residing strictly in VRAM. Inaccessible directly by CPU.
2. **Host Memory (`zeMemAllocHost`)**: System RAM accessible directly by GPU across PCIe / CXL bus.
3. **Shared Memory (`zeMemAllocShared`)**: Unified virtual address space with automatic page migration between CPU host and GPU device.
