# Assembly (`Assembleur`) & W3C WebAssembly Core 2.0 — Same-Codebase Modern OS Architecture in Aphrody

> Canonical engineering and didactic reference grounded in live upstream specifications ([W3C WebAssembly Core](https://www.w3.org/TR/wasm-core-2/), [Wasm 2.0 Snapshot](https://www.w3.org/TR/2025/CRD-wasm-core-2-20250616/), [WebAssembly.org](https://webassembly.org/), [WASI.dev](https://wasi.dev/), [Component Model WIT](https://component-model.bytecodealliance.org/design/wit.html), [Wikipédia — Assembleur](https://fr.wikipedia.org/wiki/Assembleur)) and verified directly against [`crates/aphrody_session/src/wasm_hal.rs`](../../../crates/aphrody_session/src/wasm_hal.rs), [`tooling/bun/web-build.ts`](../../../tooling/bun/web-build.ts), and [`tooling/bun/cross-build.ts`](../../../tooling/bun/cross-build.ts).

---

## 1. Fundamentals of Assembly (`Assembleur`)

Per [Wikipédia — Assembleur](https://fr.wikipedia.org/wiki/Assembleur), assembly is the lowest-level human-readable representation of machine code: binary bit-patterns are replaced by symbolic **mnemonics**, decimal/hex literals, and symbolic **labels** which an assembler translates into object files or raw executables.

### 1.1 Historical Trajectory & Modern OS Niche

- **1949–1961**: Early stored-program machines (EDSAC, 1949) required hand-translated one-letter mnemonics; Nathaniel Rochester built the first symbolic assembler for the IBM 701 (1954). By 1961, Burroughs MCP (written in ESPOL) demonstrated that high-level systems languages could build entire operating systems, gradually displacing whole-OS assembly (later revived in microcomputers such as IBM PC DOS ~4,000 lines and 1990s consoles).
- **Modern Systems Role**: Today, assembly is reserved for hardware boundaries high-level languages cannot emit directly:
  1. **Privileged / Ring-0 CPU state**: descriptor-table setup (`LGDT`/`LLDT` on x86, MSR/MMU page-table switches, exception vectors, context-switch trampolines).
  2. **Cycle-exact & hint primitives**: `pause`/`rdtsc` (`x86_64`), `yield`/`mrs ..., cntvct_el0` (`aarch64`).
  3. **Explicit SIMD / hot loops & compiler verification**: disassembling generated machine code (`llvm-objdump -d`) to inspect register allocation, vectorization, and ABI conformance.
- **Disassembly Non-Bijection (`Désassemblage`)**: Translating machine bytes back into source assembly is lossy and often ambiguous due to variable-length instruction streams (x86), interleaved code/data, self-modifying code, and stripped comments, macros, conditional blocks, and symbolic labels.

### 1.2 Syntax Families, Directives & Pseudo-Instructions

Two major x86 assembly syntaxes dominate OS development and compiler output:

| Dimension | AT&T / GAS Syntax (GNU `as`, default LLVM inline asm) | Intel Syntax (`NASM`, `MASM`, `.intel_syntax`) |
| :--- | :--- | :--- |
| **Operand Order** | `mnemonic src, dst` (`movb $0x61, %al`) | `mnemonic dst, src` (`mov al, 0x61`) |
| **Register & Immediate Prefixes** | `%al`, `%rax` / `$0x61` | Bare registers (`al`, `rax`) and bare immediates (`0x61`) |
| **Operand Width** | Instruction suffix (`movb`, `movw`, `movl`, `movq`) | Explicit size qualifier (`byte ptr`, `dword ptr`, `qword ptr`) |
| **Memory Indirection** | `disp(%base, %index, scale)` | `[base + index*scale + disp]` |
| **Directives** | `.text`, `.data`, `.global _start`, `.ascii`, `.comm` | `section .text`, `section .data`, `global _start`, `db`, `equ` |

Example machine encoding cited by [Wikipédia — Assembleur](https://fr.wikipedia.org/wiki/Assembleur):
- Machine bits: `10110000 01100001` (`0xB0 0x61`)
- AT&T mnemonic: `movb $0x61, %al` (write `97` / `'a'` into the low byte `AL` of `RAX`)

**Macro-assemblers & Pseudo-instructions**:
- Assemblers provide macros and **pseudo-instructions** that expand to real CPU encodings:
  - On x86, `NOP` (`0x90`) shares its opcode with `XCHG AX, AX` (`nop` modifies neither registers nor `RFLAGS`).
  - On RISC ISAs (ARM/AArch64), fixed 32-bit instructions cannot encode an arbitrary 64-bit immediate in one opcode; assemblers expand pseudo-loads (`ldr x0, =const` or `movz`/`movk` sequences) and stack pseudo-ops (`push`/`pop` into pre-decrement store / post-increment load).

### 1.3 Registers, Addressing Modes & Container Formats

| Architecture | General-Purpose & Control Registers | SIMD / Vector Registers | Typical Page Size | Addressing Highlights |
| :--- | :--- | :--- | :--- | :--- |
| **`x86_64`** | `rax..r15` (64-bit), `rsp` (SP), `rbp` (FP), `rip` (IP), `rflags` | `xmm0..xmm15` (128-bit SSE2), `ymm` (256-bit AVX2), `zmm` (512-bit AVX-512) | `4 KiB` (`4,096 B`) | Immediate, register, `[base + disp]`, SIB `[base + idx*scale + disp]`, `RIP`-relative PIC |
| **`aarch64`** | `x0..x30` (`x29` FP, `x30` LR), `sp`, `xzr`/`wzr`, `pc`, `cntvct_el0` | `v0..v31` (128-bit NEON / AdvSIMD) | `4 KiB` (Linux/Win/Android) or `16 KiB` (Apple Silicon) | Fixed 32-bit instruction words, pre/post-indexed addressing, `adrp` + `add` page-relative PIC |
| **`wasm32`** | Typed operand stack + module `local`s / `global`s (no exposed CPU registers) | `v128` value type (`i8x16`, `i16x8`, `i32x4`, `i64x2`, `f32x4`, `f64x2`) | `64 KiB` (`65,536 B`) linear memory page | Bounds-checked `memarg { align, offset }` relative to Linear Memory 0 (`u32` effective address) |

**Binary Object & Executable Formats**:
- **ELF** (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`): sections (`.text`, `.rodata`, `.data`, `.bss`), dynamic `.got`/`.plt` relocations, `PT_LOAD` segments mapped by the OS kernel loader.
- **PE / COFF** (`x86_64-pc-windows-msvc`): DOS/PE header, `.text`/`.rdata`/`.data`/`.pdata` unwind tables, import address table (IAT).
- **Mach-O** (`aarch64-apple-darwin`): load commands (`LC_SEGMENT_64`), code signatures, universal `lipo` slices.
- **Flat Binary**: raw machine instructions stripped of headers for firmware/boot sectors.
- **WebAssembly (`.wasm`)**: streamable section binary (`\0asm\x01\0\0\0`) separating Type, Import, Function, Code, and Data sections for O(n) single-pass validation and capability sandboxing.

### 1.4 Calling Conventions (ABI Matrix)

Encoded directly in [`CallingConvention`](../../../crates/aphrody_session/src/wasm_hal.rs):

| Calling Convention | Integer / Ptr Arg Registers | Float / Vector Arg Registers | Return Registers | Stack Alignment & ABI Notes |
| :--- | :--- | :--- | :--- | :--- |
| **System V AMD64** (`Linux`/`macOS` x64) | `rdi, rsi, rdx, rcx, r8, r9` | `xmm0..xmm7` | `rax, rdx` / `xmm0, xmm1` | `16 B` aligned pre-`call`; `128 B` red zone below `rsp` |
| **Microsoft x64 (`Win64`)** | `rcx, rdx, r8, r9` | `xmm0..xmm3` | `rax` / `xmm0` | `16 B` aligned; caller reserves `32 B` home/shadow space |
| **AAPCS64** (`aarch64`) | `x0..x7` | `v0..v7` | `x0..x1` / `v0..v3` | `16 B` aligned `sp`; `x29` frame pointer, `x30` link register |
| **Wasm Stack / Canonical ABI** | Operand stack (`i32/i64/f32/f64/v128/ref`) | Operand stack (`v128` / `f32` / `f64`) | Multi-value result stack | Component Model lifts/lowers composite WIT types across linear memory |

### 1.5 Rust `core::arch::asm!` vs External `.s` / LLVM MC

Rust embeds LLVM's inline assembler via `core::arch::asm!`, compiling `.rs` files directly without an external `.s` build step (`cc`/`nasm`), while external `.s` + `clang -c` / `nasm` remains useful for stand-alone boot trampolines before Rust's stack/runtime is initialized.

Critical correctness rules enforced in [`crates/aphrody_session/src/wasm_hal.rs`](../../../crates/aphrody_session/src/wasm_hal.rs):
1. **Accurate register clobbers & options**: `spin_wait_hint()` passes `options(nomem, nostack, preserves_flags)` for `pause` (`x86_64`) and `yield` (`aarch64`).
2. **Never mark non-deterministic hardware counters `pure` or `readonly`**: `read_timestamp_ticks()` omits `pure` on `rdtsc` (`out("eax") lo, out("edx") hi`) and `mrs {ticks}, cntvct_el0`, preventing LLVM from hoisting or deduplicating consecutive timer reads.
3. **Explicit `// SAFETY:` contracts**: required by workspace lints and `cargo clippy -- -D warnings`.

---

## 2. W3C WebAssembly Core 2.0 Specification (`wasm-core-2`)

> **Specification Versioning Note (verified live 2026-11-16)**: Per [`https://webassembly.org/specs/`](https://webassembly.org/specs/), the unversioned living URL [`https://www.w3.org/TR/wasm-core-2/`](https://www.w3.org/TR/wasm-core-2/) now publishes the **W3C Candidate Recommendation Draft (16 November 2026)** describing **Wasm 3.0** (adding `Tag` section ID 13, typed references/GC aggregates, exception handling, multiple memories, `memory64`, and tail calls). The **WebAssembly Core 2.0** baseline snapshot lives at [`https://www.w3.org/TR/2025/CRD-wasm-core-2-20250616/`](https://www.w3.org/TR/2025/CRD-wasm-core-2-20250616/) (and is summarized in Appendix A.6/A.8 *Release 2.0*). Aphrody targets **Core 2.0** (`sections 0..=12`) universally across browsers and Wasmtime `48.0.7`.

### 2.1 Stack Virtual Machine & Structured Control Flow

Unlike native CPUs that jump to arbitrary memory addresses (`jmp *%rax`), WebAssembly Core 2.0 executes over a **typed operand stack** with **structured control flow**:
- Control instructions (`block`, `loop`, `if`/`else`/`end`) nest lexically and target enclosing blocks by relative `labelidx` (`br`, `br_if`, `br_table`), never by raw byte offsets.
- **Why this matters for an OS**: A validator can type-check an entire module in a single linear pass (`O(n)`), preventing arbitrary ROP gadgets or jumps into the middle of multi-byte instructions.

### 2.2 Value Types & Binary Type Opcodes (Core 2.0 §5.3 & §A.7)

Implemented in [`WasmValueType`](../../../crates/aphrody_session/src/wasm_hal.rs):

| Value Type | Category | Binary Byte (`s7` negative) | Width / Semantics |
| :--- | :--- | :--- | :--- |
| `i32` | Number (`numtype`) | `0x7F` (`-1`) | 32-bit two's complement / bit-width integer |
| `i64` | Number (`numtype`) | `0x7E` (`-2`) | 64-bit integer |
| `f32` | Number (`numtype`) | `0x7D` (`-3`) | IEEE-754 binary32 little-endian |
| `f64` | Number (`numtype`) | `0x7C` (`-4`) | IEEE-754 binary64 little-endian |
| `v128` | Vector (`vectype`, **2.0**) | `0x7B` (`-5`) | 128-bit packed SIMD register (`i8x16`, `i16x8`, `i32x4`, `i64x2`, `f32x4`, `f64x2`) |
| `funcref` | Reference (`reftype`, **2.0**) | `0x70` (`-16`) | Opaque nullable reference to a Wasm function |
| `externref` | Reference (`reftype`, **2.0**) | `0x6F` (`-17`) | Opaque nullable host/JS/DOM/GPU capability handle |

*(Special structural encodings: `0x60` = `functype` `[valtype*] -> [valtype*]`, `0x40` = empty `blocktype` `[ϵ]`.)*

### 2.3 `.wat` Text Format vs `.wasm` Binary Format (LEB128 & Sections `0..=12`)

Every `.wasm` module begins with an 8-byte preamble (`\0asm` = `0x00 0x61 0x73 0x6D` + little-endian version `1` = `0x01 0x00 0x00 0x00`), followed by variable-length **sections** whose payload lengths and indices use **LEB128** (`u32` max `ceil(32/7) = 5` bytes with upper unused bits zeroed on byte 5 per Core 2.0 §5.2.2).

Per Core 2.0 §5.5.2 and §5.5.16, non-custom sections **must appear at most once and in strict canonical order** (note `DataCount(12)` precedes `Code(10)` and `Data(11)` for single-pass validation!):

| Canonical Order | Section ID | Name | Purpose in Core 2.0 |
| :---: | :---: | :--- | :--- |
| *(anywhere)* | `0` | `Custom` | Debug names (`name`), `producers`, LLVM `target_features` (ignored by VM execution) |
| 1 | `1` | `Type` | Vector of `functype` (`0x60 params results`) signatures |
| 2 | `2` | `Import` | Host capabilities (functions, tables, memory, globals) imported before instantiation |
| 3 | `3` | `Function` | Maps each defined function index to a `typeidx` (separated from `Code` for parallel/streaming compilation) |
| 4 | `4` | `Table` | Reference tables (`funcref` / `externref`, multiple tables allowed in Core 2.0) |
| 5 | `5` | `Memory` | Linear memory limits (`min`, optional `max`) in `64 KiB` (`65,536 B`) pages |
| 6 | `6` | `Global` | Module-wide `const` or `var` typed globals |
| 7 | `7` | `Export` | Public entry points (`func`, `table`, `memory`, `global`) exposed to host/JS/other components |
| 8 | `8` | `Start` | Optional automatic initialization `funcidx` |
| 9 | `9` | `Element` | Active, passive, or declarative element segments populating tables |
| 10 | `12` | **`DataCount`** *(Core 2.0)* | Optional `u32` count of data segments so single-pass validators can bounds-check `memory.init` / `data.drop` inside `Code` before `Data` is read |
| 11 | `10` | `Code` | Local variable declarations + bytecode expressions for each function |
| 12 | `11` | `Data` | Active (`0`/`2` bitfield) or passive (`1`) memory initialization byte segments |

### 2.4 All Six WebAssembly Core 2.0 Feature Pillars (Release 2.0 §A.6)

Verified in both W3C §A.6 and `rustc 1.98.1` (`rustc --print cfg --target wasm32-unknown-unknown`):

1. **Sign-Extension Operations (`0xC0..0xC4`)**: `i32.extend8_s`, `i32.extend16_s`, `i64.extend8_s`, `i64.extend16_s`, `i64.extend32_s` — eliminates shift-left/arithmetic-shift-right pairs.
2. **Non-Trapping Float-to-Int Conversions (`0xFC 0..7`)**: `i{32,64}.trunc_sat_f{32,64}_{s,u}` — clamps `NaN` to `0` and out-of-range floats to `MIN`/`MAX` instead of trapping (critical for Rust `as` casts and audio/graphics kernels).
3. **Multi-Value Functions & Blocks**: `functype` supports multiple return values and `block`/`loop`/`if` accept positive `s33` `typeidx` block parameters/results without spilling structs to linear memory.
4. **Reference Types & Multiple Tables**: `funcref` (`0x70`) and `externref` (`0x6F`), `ref.null`/`ref.func`/`ref.is_null`, typed `select`, and direct `table.{get,set,size,grow,fill,copy,init}` / `elem.drop` across multiple module tables.
5. **Bulk Memory Instructions (`0xFC 8..11`)**: `memory.init` (`0xFC 8`), `data.drop` (`0xFC 9`), `memory.copy` (`0xFC 10`, overlapping `memmove` semantics), `memory.fill` (`0xFC 11`, `memset`), passive data segments, and `DataCount(12)`.
6. **Fixed-Width 128-bit SIMD (`v128`, `0xFD` prefix + `u32` opcode)**: portable 16-byte vector loads/stores (`v128.load` `0xFD 0`, `v128.store` `0xFD 11`), bitwise (`v128.xor` `0xFD 81`, `v128.not` `0xFD 77`), splats (`i8x16.splat` `0xFD 15`), saturating pixel math (`i8x16.add_sat_u`), and IEEE float vectors (`f32x4`, `f64x2`).

Verified live on Aphrody's own browser Wasm bundle (`llvm-objdump -s --section=target_features target/web-dist/simd-baseline/web_os_bg.wasm`):

```text
+mutable-globals +nontrapping-fptoint +simd128 +bulk-memory +sign-ext
+reference-types +multivalue +bulk-memory-opt +call-indirect-overlong
```

---

## 3. Same-Codebase Full-Wasm Modern OS Architecture in Aphrody

Aphrody compiles a **single `#![no_std]`-clean Rust logic & HAL core** across bare-metal/native OS hosts, Browser Full-Wasm WebGPU desktops, and capability-sandboxed WASI Component Model services:

```text
                  Single Rust Core (crates/aphrody_session/src/wasm_hal.rs + compositor + session)
                                             │
            ┌────────────────────────────────┼────────────────────────────────┐
            ▼                                ▼                                ▼
   Native Ring-0 / Desktop         Browser Full-Wasm OS            Sandboxed WASI 0.2 Microservices
  (x86_64 / aarch64 ELF/PE)     (wasm32-unknown-unknown)            (wasm32-wasip2 Component Model)
  • core::arch::asm! (pause/     • baseline + simd-baseline        • Wasmtime 48.0.7 + wasmtime_wasi::p2
    rdtsc / yield / cntvct_el0)    (+simd128) + opt-in --coi       • WIT capability-gated imports only
  • SSE2 / AVX2 / NEON           • core::arch::wasm32 (v128,       • Epoch interruption & StoreLimits
  • 4KiB / 16KiB MMU page tables   memory_size::<0>())             • Async WASI 0.3 migration path
  • Vulkan / DX12 / Metal        • WebGPU -> WebGL2 + ARIA DOM
```

### 3.1 Single HAL Core (`crates/aphrody_session/src/wasm_hal.rs`)

Re-exported cleanly from [`crates/aphrody_session/src/aphrody_session.rs`](../../../crates/aphrody_session/src/aphrody_session.rs) with zero OS-syscall dependencies:
- **`IsaProfile` & `CallingConvention`**: compile-time introspection of ISA (`X86_64`, `Aarch64`, `Wasm32`, `Wasm32Simd128`, `Wasm32WasiP2`), pointer bits (`64` vs `32`), active SIMD register width (`0`, `128`, `256`, `512`), ABI (`SystemV`/`Win64`/`Aapcs64`/`WasmStack`), and page size (`4 KiB`, `16 KiB`, `64 KiB`).
- **Unified Low-Level Primitives**:
  - `spin_wait_hint()`: `asm!("pause")` on `x86_64`, `asm!("yield")` on `aarch64`, `core::hint::spin_loop()` on `wasm32`.
  - `read_timestamp_ticks()`: `rdtsc` on `x86_64`, `cntvct_el0` on `aarch64`, monotonic WASI clock floor on `wasm32-wasip2`, and panic-free lock-free `AtomicU64` ticks on `wasm32-unknown-unknown` (avoiding `std::time::Instant::now()` panics in bare browser modules).
  - `wasm_linear_memory_pages()`: exposes `core::arch::wasm32::memory_size::<0>()` (`memory.size`) cleanly under `wasm32`.
- **Single-Codebase SIMD128 + Portable Scalar Kernels**:
  - `blit_rgba_alpha_add_simd128` (`v128_load`, `u8x16_add_sat`, `v128_store` under `cfg(all(target_arch = "wasm32", target_feature = "simd128"))`) + `blit_rgba_alpha_add_scalar` tail/fallback.
  - `xor_checksum_words` (`u32x4_splat`, `v128_load`, `v128_xor`, `i32x4_extract_lane`) + `xor_checksum_words_scalar`, verified bit-identical across all slice alignments in unit tests.
- **Pure-Rust Core 2.0 Binary Codec**: `encode_wasm_core2_noop_module`, `parse_wasm_core2_header`, `decode_uleb128_u32`, `parse_custom_section_name`, and `WASM_CORE2_SIMD_PROBE_BYTES` covering section IDs `0..=12` and enforcing §5.5.16 order.

### 3.2 Memory Isolation: Linear Memory Capability Sandboxes vs Hardware MMU

| Isolation Axis | Native MMU Kernel (`x86_64` / `aarch64`) | WebAssembly Core 2.0 + WASI 0.2 (`wasm32`) |
| :--- | :--- | :--- |
| **Address Space Granularity** | Hardware virtual pages (`4 KiB` / `16 KiB` PTEs) | Contiguous bounds-checked Linear Memory (`64 KiB` pages, explicit `memory.size`/`grow`) |
| **Privilege Enforcement** | Ring-0 / Ring-3 hardware privilege transitions + syscalls | Static bytecode validation + Wasmtime guard pages (`4 GiB` virtual reservation + bounds checks) |
| **Authority Model** | Ambient authority (process inherits user's filesystem/network/env unless wrapped in Landlock/AppContainer) | **Capability-based / deny-by-default** ([WASI Security](https://wasi.dev/security)): a binary starts with zero authority and accesses only explicitly linked imports |
| **Cross-Language ABI** | C ABI (`SystemV`/`Win64`/`AAPCS64`) passing raw pointers | **WIT + Canonical ABI** ([Component Model](https://component-model.bytecodealliance.org/design/wit.html)): typed resources, records, lists, and handles marshaled across isolated memories |

### 3.3 Aphrody Runtime, Compositor & WebGPU Integration

1. **Browser Cohort Architecture ([`tooling/bun/web-build.ts`](../../../tooling/bun/web-build.ts), [`crates/aphrody_desktop/examples/web_os.rs`](../../../crates/aphrody_desktop/examples/web_os.rs), [`crates/gpui_web`](../../../crates/gpui_web/src/gpui_web.rs))**:
   - Compiles `wasm32-unknown-unknown` with `[profile.web]` (`opt-level = "z"`, `lto = "thin"`, `strip = true`, `panic = "abort"` in root [`Cargo.toml`](../../../Cargo.toml)) and `RUSTC_WRAPPER=""` to avoid Windows `web-sys` argv overflow.
   - Emits separated deployment cohorts in `target/web-dist/`:
     - `baseline`: universal Core 2.0 Wasm (`+bulk-memory,+nontrapping-fptoint,+sign-ext,+reference-types,+multivalue`), zero `SharedArrayBuffer` / COOP-COEP requirement.
     - `simd-baseline`: adds `-C target-feature=+simd128`, selected dynamically in JS via `WebAssembly.validate(WASM_CORE2_SIMD_PROBE_BYTES)`.
     - `multithreaded` (opt-in `--coi`): adds `+atomics,+bulk-memory,+mutable-globals` gated on `crossOriginIsolated`.
   - Optimizes via Binaryen `wasm-opt v133 -Oz --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types --enable-multivalue` and pins `wasm-bindgen 0.2.129` against [`Cargo.lock`](../../../Cargo.lock).
2. **GPU & Compositor Pipeline ([`crates/aphrody_session/src/compositor.rs`](../../../crates/aphrody_session/src/compositor.rs), [`crates/gpui_wgpu`](../../../crates/gpui_wgpu/src/wgpu_context.rs), [`crates/aphrody_cli`](../../../crates/aphrody_cli/src/main.rs))**:
   - `aphrody_session` computes Taffy tiling/layer-shell layouts identically on native and Wasm (`compositor.rs` has zero platform syscalls).
   - `gpui_web::WebPlatform` pairs `WebBackendPreference` with `gpui_wgpu::WgpuContext` (preferring browser WebGPU `navigator.gpu` and falling back to WebGL2 cleanly).
   - `aphrody_cli` (`gpui-wgpu`) validates and transpiles `.wgsl` compute/render shaders via `naga` across Vulkan, DX12, Metal, and OpenGL ES.
3. **Server / Extension WASI 0.2 Host ([`crates/extension_host/src/wasm_host.rs`](../../../crates/extension_host/src/wasm_host.rs), [WASI Releases](https://wasi.dev/releases))**:
   - Uses `wasmtime 48.0.7` + `wasmtime-wasi` (`p2`) to execute `wasm32-wasip2` components built from WIT contracts, ready to adopt WASI 0.3 (`async func`, `stream<T>`, `future<T>`) cleanly without breaking WASI 0.2 components.

---

## 4. Executable Examples & Verified Commands

All commands below were executed and verified directly inside `C:\src\aphrody` (hosted tooling: `rustc 1.98.1`, `clang 23.1.5`, `LLD 23.1.5`, `llvm-objdump`, `wasm-bindgen 0.2.129`, `wasm-opt 133`, `wat2wasm 1.0.42`, `NASM 3.02`, `Bun 1.4.5`).

### 4.1 Native + Wasm Cross-Verification (`aphrody_session`)

```powershell
# 1. Host unit tests (covers native asm! ticks, SIMD/scalar bit-identity, and Wasm Core 2.0 codec)
cargo test -j 4 -p aphrody_session

# 2. Zero-warning Clippy gate on native host
cargo clippy -j 4 -p aphrody_session --all-targets -- -D warnings

# 3. Zero-warning Clippy across Core 2.0 baseline, +simd128, and WASI Preview 2 targets
$env:RUSTC_WRAPPER=""
cargo clippy -j 4 -p aphrody_session --target wasm32-unknown-unknown -- -D warnings
$env:CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C target-feature=+simd128"
cargo clippy -j 4 -p aphrody_session --target wasm32-unknown-unknown -- -D warnings
Remove-Item Env:\CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS
cargo clippy -j 4 -p aphrody_session --target wasm32-wasip2 -- -D warnings

# 4. Unified cross-build matrix checks (wasm32-unknown-unknown, wasm32-wasip2, Linux glibc 2.28 via cargo-zigbuild)
bun run cross:check -- --target wasm32-unknown-unknown --package aphrody_session
bun run cross:check -- --target wasm32-wasip2 --package aphrody_session
bun run cross:check -- --target x86_64-unknown-linux-gnu.2.28 --package aphrody_session
```

### 4.2 Runnable Core 2.0 `.wat` & Binary Verification in Bun

The 35-byte module generated by `aphrody_session::wasm_hal::encode_wasm_core2_noop_module()` matches the canonical Core 2.0 `.wat` module `(module (func (export "noop") nop))`, and the 31-byte `WASM_CORE2_SIMD_PROBE_BYTES` matches `tooling/bun/web-build.ts`:

```powershell
@'
const noop = new Uint8Array([
  0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
  0x03, 0x02, 0x01, 0x00,
  0x07, 0x08, 0x01, 0x04, 0x6E, 0x6F, 0x6F, 0x70, 0x00, 0x00,
  0x0A, 0x05, 0x01, 0x03, 0x00, 0x01, 0x0B,
]);
const simdProbe = new Uint8Array([
  0,97,115,109, 1,0,0,0, 1,5,1,96,0,1,123, 3,2,1,0, 10,10,1,8,0,65,0,253,15,253,98,11,
]);
console.log("noop valid:", WebAssembly.validate(noop));
console.log("simd valid:", WebAssembly.validate(simdProbe));
new WebAssembly.Instance(new WebAssembly.Module(noop)).exports.noop();
'@ | bun -
```

### 4.3 Inspecting Native & Wasm Binaries (`llvm-objdump`, `web:matrix`, `repo-hygiene`)

```powershell
# Inspect WebAssembly Core 2.0 section order (including DATACOUNT before CODE/DATA) and LLVM target_features
llvm-objdump -h target\web-dist\simd-baseline\web_os_bg.wasm
llvm-objdump -s --section=target_features target\web-dist\simd-baseline\web_os_bg.wasm

# Inspect cross-compilation tools, cohort manifest metrics, and repository link hygiene
bun run cross:doctor
bun run web:matrix
bun run hygiene
```

---

## 5. Related Architecture Guides

- [Architecture Documentation Index](./README.md)
- [Production Readiness & Monorepo Hardening Guide](./production-guide.md)
- [Cross-Compilation, LLVM & Hermetic Native SDK Guide](./cross-compile-and-depot-tools.md)
