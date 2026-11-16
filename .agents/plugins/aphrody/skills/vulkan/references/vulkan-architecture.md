# Vulkan Architecture in `aphrody`

Vulkan provides the direct-to-metal, low-driver-overhead GPU architecture for graphics and compute in `aphrody`. Through `wgpu-hal` and `gpui_wgpu`, `aphrody` targets Vulkan 1.3 and 1.4 core standards while maintaining graceful fallback to Vulkan 1.2 and Mesa Dozen Direct3D12 emulation layers.

---

## 1. Upstream Specifications & Runtime Discovery

- **Khronos Vulkan Core**: Targets Vulkan 1.3 and 1.4 core specifications and extension registry, with graceful fallback to Vulkan 1.2 or software/emulation adapters.
- **LunarG SDK**: Discovered via `$VULKAN_SDK` or `<SLUG>_SDK_ROOT` for headers, `glslangValidator`/`glslc`/`spirv-val`, and validation layers.
- **Runtime Hardware Query**: Inspect live adapter properties (`vendorID`, `deviceID`, subgroup sizes, driver version) dynamically via `gpui-wgpu adapters --backends vulkan`.

---

## 2. Synchronization2 (`VK_KHR_synchronization2` / Vulkan 1.3 Core)

Legacy Vulkan synchronization (`vkCmdPipelineBarrier`) suffered from 32-bit bitmasks (`VkPipelineStageFlags`, `VkAccessFlags`) that lacked expressiveness for modern GPU stages (mesh shaders, ray tracing, copy queues) and created ambiguity regarding execution and memory dependencies.

Vulkan 1.3 promoted `VK_KHR_synchronization2` to core, introducing:

1. **64-Bit Stage and Access Masks**:
   - `VkPipelineStageFlags2`: Extended flags such as `VK_PIPELINE_STAGE_2_COPY_BIT`, `VK_PIPELINE_STAGE_2_ALL_TRANSFER_BIT`, and separate stage identifiers for compute, mesh, task, and fragment stages.
   - `VkAccessFlags2`: Fine-grained memory access flags such as `VK_ACCESS_2_SHADER_STORAGE_READ_BIT`, `VK_ACCESS_2_SHADER_STORAGE_WRITE_BIT`, and `VK_ACCESS_2_TRANSFER_WRITE_BIT`.

2. **Unified Dependency Structures**:
   Instead of passing raw array pointers to `vkCmdPipelineBarrier`, `vkCmdPipelineBarrier2` receives a single `VkDependencyInfo`:
   ```c
   typedef struct VkDependencyInfo {
       VkStructureType                    sType;
       const void*                        pNext;
       VkDependencyFlags                  dependencyFlags;
       uint32_t                           memoryBarrierCount;
       const VkMemoryBarrier2*            pMemoryBarriers;
       uint32_t                           bufferMemoryBarrierCount;
       const VkBufferMemoryBarrier2*      pBufferMemoryBarriers;
       uint32_t                           imageMemoryBarrierCount;
       const VkImageMemoryBarrier2*       pImageMemoryBarriers;
   } VkDependencyInfo;
   ```

3. **Queue Family Ownership Transfers**:
   Transfers between transfer/compute queues and graphics queues require explicit release and acquire barriers with matching `srcQueueFamilyIndex` and `dstQueueFamilyIndex`. Synchronization2 simplifies image layout transitions during these transfers without redundant pipeline stalls.

---

## 3. Dynamic Rendering (`VK_KHR_dynamic_rendering` / Vulkan 1.3 Core)

Legacy Vulkan mandated pairing every draw call with an immutable `VkRenderPass` object and an explicit `VkFramebuffer` matching attachment dimensions, formats, and sample counts. For modern UI engines with frequent layer compositions, popups, and blurred backgrounds, this resulted in thousands of transient objects and descriptor overhead.

Dynamic Rendering eliminates `VkRenderPass` and `VkFramebuffer` entirely:

1. **Direct Attachment Binding**:
   Render commands are encapsulated directly between `vkCmdBeginRendering` and `vkCmdEndRendering`:
   ```c
   VkRenderingAttachmentInfo colorAttachment = {
       .sType = VK_STRUCTURE_TYPE_RENDERING_ATTACHMENT_INFO,
       .imageView = targetImageView,
       .imageLayout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
       .loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR,
       .storeOp = VK_ATTACHMENT_STORE_OP_STORE,
       .clearValue = { .color = { { 0.0f, 0.0f, 0.0f, 1.0f } } },
   };

   VkRenderingInfo renderingInfo = {
       .sType = VK_STRUCTURE_TYPE_RENDERING_INFO,
       .renderArea = { { 0, 0 }, { width, height } },
       .layerCount = 1,
       .colorAttachmentCount = 1,
       .pColorAttachments = &colorAttachment,
       .pDepthAttachment = NULL,
       .pStencilAttachment = NULL,
   };

   vkCmdBeginRendering(commandBuffer, &renderingInfo);
   // Draw calls here...
   vkCmdEndRendering(commandBuffer);
   ```

2. **wgpu-hal Dynamic Rendering Strategy**:
   In `vendor/wgpu/wgpu-hal/src/vulkan/command.rs`, `wgpu-hal` checks if dynamic rendering is supported by the device. If available, it skips `VkFramebuffer` cache lookups and records `vkCmdBeginRenderingKHR` / `vkCmdBeginRendering` directly, drastically cutting command submission latency.

---

## 4. Timeline Semaphores (`VK_KHR_timeline_semaphore` / Vulkan 1.2 Core)

Binary semaphores have only two states (signaled / unsignaled) and auto-reset upon wait, making CPU-GPU coordination and complex queue dependencies error-prone.

Timeline semaphores hold a strictly monotonically increasing 64-bit integer (`uint64_t`):
- **Host Wait & Query**: The CPU can poll or block on a specific counter value via `vkWaitSemaphores` without creating fences.
- **Queue Coordination**: A single timeline semaphore can sequence independent command queues (e.g. transfer queue signals value `N`, compute queue waits on `N` and signals `N+1`, graphics queue waits on `N+1`).
- **In-flight Frame Tracking**: `aphrody` tracks frame rendering latency and buffer reuse by advancing a per-window timeline value each frame, eliminating CPU-side frame allocation stalls.

---

## 5. Resource Binding: Push Constants vs. Descriptor Indexing

| Binding Mechanism | Latency / Overhead | Maximum Payload | Primary Use Case in `aphrody` |
| :--- | :--- | :--- | :--- |
| **Push Constants** | Near-zero (written directly to GPU registers / command stream) | 128 B guaranteed baseline (256 B on many desktop discrete GPUs) | Per-draw transformation matrices, tint colors, text quad offsets, viewport dimensions. |
| **Descriptor Sets** | Moderate (CPU descriptor pool allocation, GPU memory fetch) | Driver-dependent pool sizes | Static bind groups, compute input/output storage buffers. |
| **Descriptor Indexing (`bindless`)** | Zero CPU descriptor switching, uniform indexing | Query `maxDescriptorSetSampledImages` per adapter | Texture atlases, glyph caches, multi-material UI icon arrays. |

### Descriptor Indexing Features
On Vulkan 1.2+ discrete GPUs, `VkPhysicalDeviceDescriptorIndexingFeatures` exposes:
- `shaderSampledImageArrayNonUniformIndexing = VK_TRUE`
- `descriptorBindingVariableDescriptorCount = VK_TRUE`
- `runtimeDescriptorArray = VK_TRUE`
This allows shaders to dynamically index texture arrays using runtime variables (e.g. `texture_array[nonuniformEXT(icon_index)]`), bypassing CPU bind group updates between UI components.

---

## 6. Subgroup Operations

Hardware subgroup widths vary by vendor (32 on NVIDIA and AMD Wave32, 8–32 on Intel Xe, 64 on AMD Wave64). Query `subgroup_min_size`/`subgroup_max_size` at adapter selection time and use `VK_KHR_subgroup` primitives (`subgroupBallot()`, `subgroupShuffle()`, `subgroupShuffleDown()`, `subgroupAdd()`, `subgroupMin()`) to communicate across lanes without shared memory barriers.

In UI blur passes and compute particle systems, subgroup reductions execute in 1 clock cycle instead of multiple workgroup barrier synchronizations.
