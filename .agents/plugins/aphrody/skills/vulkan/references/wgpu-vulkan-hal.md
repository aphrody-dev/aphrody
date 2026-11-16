# wgpu-hal Vulkan Backend Architecture (`aphrody`)

The Vulkan backend of WGPU is implemented in `vendor/wgpu/wgpu-hal/src/vulkan/`. It maps the high-level `wgpu` and `wgpu-core` rendering primitives directly onto raw Vulkan calls via the `ash` crate (`v0.38`) and manages memory allocations through `gpu-allocator` (`v0.28`).

---

## 1. Module Layout & Responsibilities

The codebase in `vendor/wgpu/wgpu-hal/src/vulkan/` is organized into logical subsystems:

| File | Primary Functions & Types | Responsibility |
| :--- | :--- | :--- |
| `instance.rs` | `Instance`, `Entry::load`, `vkCreateInstance` | Entry point loading, Vulkan instance extension queries (`VK_KHR_surface`, `VK_KHR_win32_surface`), and debug messenger setup (`VK_EXT_debug_utils`). |
| `adapter.rs` | `Adapter`, `vkEnumeratePhysicalDevices` | Physical device inspection, driver properties (`VkPhysicalDeviceProperties`), memory types, limits, and feature extraction. |
| `device.rs` | `Device`, `vkCreateDevice` | Logical device creation, queue family management, pipeline layout compilation, descriptor set allocator setup. |
| `conv.rs` | `map_texture_format`, `map_barrier` | Exhaustive mapping between `wgpu_types` and Vulkan `ash::vk` constants (formats, sample counts, blend modes, pipeline stages). |
| `command.rs` | `CommandEncoder`, `CommandBuffer` | Command buffer recording, dynamic rendering passes, negative viewport setup, descriptor set binding, pipeline barrier emission. |
| `descriptor.rs` | `DescriptorSetAllocator` | Dynamic descriptor pool allocation, recycling, and layout caching. |
| `swapchain/` | `Surface`, `Swapchain` | Cross-platform surface lifecycle, swapchain presentation, image acquisition, and format negotiation. |
| `sampler.rs` | `Sampler` | Vulkan sampler creation with anisotropic filtering and comparison modes. |
| `semaphore_list.rs` | `SemaphoreList` | Tracking and garbage collection of in-flight binary and timeline semaphores. |

---

## 2. Dynamic Dispatch with `ash` (v0.38)

`wgpu-hal` interacts with the Vulkan driver dynamically through `ash`:
1. **Entry**: Loads `vulkan-1.dll` at runtime via `libloading` without requiring static linking against the LunarG SDK loader.
2. **Instance Level**: Dynamically loads global function pointers (`vkCreateInstance`, `vkEnumeratePhysicalDevices`, `vkGetInstanceProcAddr`).
3. **Device Level**: Dynamically loads device-specific entry points directly into vtables (`ash::Device`), bypassing loader trampoline overhead for high-frequency commands like `vkCmdDraw` and `vkCmdDispatch`.

---

## 3. GPU Memory Management with `gpu-allocator` (v0.28)

Direct calls to `vkAllocateMemory` are strictly limited by GPU drivers (commonly 4096 allocations maximum per device). `wgpu-hal` integrates `gpu-allocator::vulkan::Allocator` with the `hashbrown` fast-hashing feature:

- **Chunk Allocation**: Large continuous memory blocks (`VkDeviceMemory`, typically 64 MB to 256 MB) are allocated from dedicated heap types:
  - `VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT`: Fast VRAM on discrete GPU for textures, render targets, and static vertex/index buffers.
  - `VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT`: Staging buffers for CPU-to-GPU uploads without explicit flush operations.
  - `VK_MEMORY_PROPERTY_HOST_CACHED_BIT`: Readback buffers for GPU-to-CPU texture and buffer downloads.
- **Suballocation**: Small buffers (uniform buffers, dynamic vertex quads) are suballocated from blocks using TLSF (Two-Level Segregated Fit) algorithms.
- **Dedicated Allocations**: Textures exceeding allocation thresholds or requiring `VK_IMAGE_TILING_OPTIMAL` receive dedicated allocations via `VK_KHR_dedicated_allocation`.

---

## 4. Cross-Platform Surface & Swapchain Lifecycle

Surface management in `wgpu-hal` adapts across platforms via `raw-window-handle`:

### Windows (`Win32SurfaceKHR`)
- Created via `VK_KHR_win32_surface` using `HWND` and `HINSTANCE`.
- Swapchain presentation mode:
  - `PresentMode::Fifo`: Guaranteed vsync via double/triple buffering (`VK_PRESENT_MODE_FIFO_KHR`).
  - `PresentMode::Mailbox`: Triple buffering with lowest input latency, replacing unread frames without tearing (`VK_PRESENT_MODE_MAILBOX_KHR`).
  - `PresentMode::Immediate`: Vsync disabled for maximum benchmark throughput (`VK_PRESENT_MODE_IMMEDIATE_KHR`).

### Linux & Android
- **Linux**: Supports Wayland (`VK_KHR_wayland_surface`) and X11/XCB (`VK_KHR_xcb_surface`, `VK_KHR_xlib_surface`).
- **Android**: Supports `VK_KHR_android_surface` operating directly on native `ANativeWindow` pointers.

---

## 5. Runtime Adapter Probe Schema (`gpui-wgpu adapters --backends vulkan`)

Adapter metadata, subgroup ranges, downlevel flags, and feature counts vary by host GPU and driver. Inspect live capabilities with:

```bash
gpui-wgpu adapters --backends vulkan --json
```

---

## 6. Khronos Validation Layer Setup & Debugging

The Khronos Validation Layer (`VK_LAYER_KHRONOS_validation`) inspects all API calls at runtime to detect invalid state, memory hazards, invalid descriptor bounds, and missing synchronization barriers.

### Configuration
1. Ensure the layer search path is configured from your Vulkan SDK env var:
   ```bash
   export VK_LAYER_PATH="$VULKAN_SDK/bin"
   ```
2. Enable validation in `wgpu` applications:
   ```bash
   export WGPU_VALIDATION=1
   ```
3. In Rust code, pass `InstanceFlags::VALIDATION` into `wgpu::InstanceDescriptor`:
   ```rust
   let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
   descriptor.flags |= wgpu::InstanceFlags::VALIDATION;
   descriptor.backends = wgpu::Backends::VULKAN;
   let instance = wgpu::Instance::new(descriptor.with_env());
   ```

### Debug Callback Routing
`wgpu-hal` routes debug messages from `VK_EXT_debug_utils` into standard Rust `log` / `tracing` macros:
- Validation errors -> `log::error!`
- Validation warnings -> `log::warn!`
- Performance warnings -> `log::info!`
