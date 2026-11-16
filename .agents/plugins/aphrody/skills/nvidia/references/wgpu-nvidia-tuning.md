# WGPU & Vulkan/DirectX 12 NVIDIA Tuning Reference

## 1. Backend Selection & Feature Flags

When targeting NVIDIA hardware (Vendor ID `0x10DE`) with `wgpu`:
- Prefer `wgpu::Backends::VULKAN` or `wgpu::Backends::DX12`.
- Vulkan driver features:
  - Supports `wgpu::Features::SUBGROUP` (Subgroup Size 32).
  - Supports `wgpu::Features::SHADER_F16` (native 16-bit floating point arithmetic).
  - Supports `wgpu::Features::MULTI_DRAW_INDIRECT` and `INDIRECT_FIRST_INSTANCE`.
  - Supports `wgpu::Features::TEXTURE_COMPRESSION_BC` (BC1–BC7 formats).

---

## 2. Workgroup Optimization Guidelines

1. **Workgroup Size Tuning**:
   - Workgroup size X/Y/Z product should always be a multiple of 32:
     - Minimum: 32 threads (`@workgroup_size(32, 1, 1)`) = 1 warp.
     - Recommended for general compute: 64 to 256 threads (`@workgroup_size(128, 1, 1)` or `@workgroup_size(16, 16, 1)`).
     - Maximum per workgroup: 1024 threads.

2. **Shared Memory (`var<workgroup>`)**:
   - NVIDIA Ada GPUs feature 32 banks of 4-byte words.
   - Bank conflicts occur when multiple threads in a warp access different words within the same bank.
   - Stride memory access so `bank_id = (word_index) % 32` does not conflict across active threads.
   - Padding arrays with 1 element (e.g. `array<f32, 33>` instead of `32`) eliminates bank conflicts in 2D stencils and matrix tiles.

3. **Memory Alignment**:
   - Dynamic uniform buffer offsets must be aligned to:
     - Vulkan: 64 bytes (`min_uniform_buffer_offset_alignment = 64`).
     - DirectX 12: 256 bytes (`min_uniform_buffer_offset_alignment = 256`).
     - Always use 256 bytes if targeting cross-backend portability.
   - Storage buffer offsets require 32-byte alignment.
   - Texture readbacks must satisfy `COPY_BYTES_PER_ROW_ALIGNMENT = 256`.

---

## 3. NVML Telemetry Integration

For automated GPU monitoring and headless benchmark telemetry:
- Query GPU utilization, temperature, power draw, and VRAM consumption via NVML C API (`nvml.h`) or standard CLI:
  ```bash
  # Query utilization and memory stats in CSV format
  nvidia-smi --query-gpu=timestamp,name,utilization.gpu,utilization.memory,memory.used,memory.free,temperature.gpu,power.draw --format=csv -l 1
  ```
