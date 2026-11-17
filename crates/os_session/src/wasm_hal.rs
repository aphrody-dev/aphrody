//! Unified native Assembly (`x86_64` / `aarch64`) + WebAssembly Core 2.0 HAL primitives.
//!
//! Designed as a single `#![no_std]`-clean logic layer inside `os_session` that compiles
//! identically across bare-metal/native desktop targets (`x86_64`, `aarch64`) and full-Wasm
//! targets (`wasm32-unknown-unknown`, `wasm32-unknown-unknown` with `+simd128`, and
//! `wasm32-wasip2` Component Model).

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
use core::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Standard 4 KiB hardware MMU page size (`x86_64` and Linux/Windows `aarch64`).
pub const PAGE_SIZE_4K: u32 = 4_096;

/// Standard 16 KiB hardware MMU page size (Apple Silicon `aarch64` macOS/iOS).
pub const PAGE_SIZE_16K: u32 = 16_384;

/// Fixed 64 KiB WebAssembly linear memory page size (`2^16` bytes per W3C Core 2.0 §2.5.5).
pub const WASM_LINEAR_PAGE_BYTES: u32 = 65_536;

/// WebAssembly binary preamble magic bytes (`\0asm`, W3C Core 2.0 §5.5.16).
pub const WASM_MAGIC: [u8; 4] = *b"\0asm";

/// WebAssembly binary format version (`1`, little-endian `[0x01, 0x00, 0x00, 0x00]`).
pub const WASM_BINARY_VERSION: u32 = 1;

/// Textual WebAssembly (`.wat`) companion for [`encode_wasm_core2_noop_module`].
pub const WASM_CORE2_NOOP_WAT: &str = "\
(module
  (func (export \"noop\")
    nop))
";

/// Textual WebAssembly (`.wat`) companion for [`WASM_CORE2_SIMD_PROBE_BYTES`].
pub const WASM_CORE2_SIMD_PROBE_WAT: &str = "\
(module
  (func (result v128)
    i32.const 0
    i8x16.splat
    i8x16.popcnt))
";

/// Minimal 31-byte WebAssembly Core 2.0 `v128` SIMD feature-detection module
/// (matching the `simd-baseline` cohort probe in `tooling/bun/web-build.ts`).
pub const WASM_CORE2_SIMD_PROBE_BYTES: [u8; 31] = [
    0x00, 0x61, 0x73, 0x6D, // magic: \0asm
    0x01, 0x00, 0x00, 0x00, // version: 1
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7B, // Type: 1 functype () -> (v128)
    0x03, 0x02, 0x01, 0x00, // Function: func 0 -> type 0
    0x0A, 0x0A, 0x01, 0x08, 0x00, // Code: 1 body, 8 bytes, 0 locals
    0x41, 0x00, // i32.const 0
    0xFD, 0x0F, // i8x16.splat (0xFD 15)
    0xFD, 0x62, // i8x16.popcnt (0xFD 98)
    0x0B, // end
];

/// Instruction set architecture / execution cohort identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsaKind {
    X86_64,
    Aarch64,
    Wasm32,
    Wasm32Simd128,
    Wasm32WasiP2,
    Portable,
}

impl IsaKind {
    /// Canonical lowercase slug matching build scripts and telemetry manifests.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
            Self::Wasm32 => "wasm32",
            Self::Wasm32Simd128 => "wasm32-simd128",
            Self::Wasm32WasiP2 => "wasm32-wasip2",
            Self::Portable => "portable",
        }
    }

    /// Returns `true` for any WebAssembly execution profile.
    #[must_use]
    pub const fn is_wasm(self) -> bool {
        matches!(
            self,
            Self::Wasm32 | Self::Wasm32Simd128 | Self::Wasm32WasiP2
        )
    }
}

/// Platform ABI calling convention governing parameter passing, stack alignment, and red zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallingConvention {
    /// System V AMD64 ABI (Linux, BSD, macOS x86_64): `rdi, rsi, rdx, rcx, r8, r9`, 128B red zone.
    SystemV,
    /// Microsoft x64 ABI (Windows `x86_64-pc-windows-msvc`): `rcx, rdx, r8, r9`, 32B shadow space.
    Win64,
    /// ARM64 AAPCS64 (`aarch64`): `x0..x7`, `x29` FP, `x30` LR, 16B SP alignment.
    Aapcs64,
    /// WebAssembly structured operand stack + WASI Component Model Canonical ABI (`wasm32*`).
    WasmStack,
}

impl CallingConvention {
    /// Canonical lowercase identifier.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::SystemV => "system_v",
            Self::Win64 => "win64",
            Self::Aapcs64 => "aapcs64",
            Self::WasmStack => "wasm_stack",
        }
    }

    /// Integer argument registers (or operand stack) in parameter order.
    #[must_use]
    pub const fn arg_registers(self) -> &'static [&'static str] {
        match self {
            Self::SystemV => &["rdi", "rsi", "rdx", "rcx", "r8", "r9"],
            Self::Win64 => &["rcx", "rdx", "r8", "r9"],
            Self::Aapcs64 => &["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"],
            Self::WasmStack => &["operand_stack"],
        }
    }

    /// Required call-boundary stack alignment in bytes.
    #[must_use]
    pub const fn stack_alignment_bytes(self) -> u32 {
        16
    }
}

/// Compile-time and inspectable hardware/VM profile for Aphrody's dual native + Wasm HAL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsaProfile {
    pub isa: IsaKind,
    pub pointer_width_bits: u8,
    pub simd_width_bits: u16,
    pub calling_convention: CallingConvention,
    pub page_size_bytes: u32,
    pub bulk_memory: bool,
    pub reference_types: bool,
    pub multi_value: bool,
    pub component_model: bool,
}

impl IsaProfile {
    /// Returns the active [`IsaProfile`] detected at compile time for the current target.
    #[must_use]
    pub const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            let simd_width_bits = if cfg!(target_feature = "avx512f") {
                512
            } else if cfg!(target_feature = "avx2") {
                256
            } else {
                128
            };
            let calling_convention = if cfg!(target_os = "windows") {
                CallingConvention::Win64
            } else {
                CallingConvention::SystemV
            };
            Self {
                isa: IsaKind::X86_64,
                pointer_width_bits: 64,
                simd_width_bits,
                calling_convention,
                page_size_bytes: PAGE_SIZE_4K,
                bulk_memory: false,
                reference_types: false,
                multi_value: false,
                component_model: false,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            let page_size_bytes = if cfg!(target_vendor = "apple") {
                PAGE_SIZE_16K
            } else {
                PAGE_SIZE_4K
            };
            Self {
                isa: IsaKind::Aarch64,
                pointer_width_bits: 64,
                simd_width_bits: 128,
                calling_convention: CallingConvention::Aapcs64,
                page_size_bytes,
                bulk_memory: false,
                reference_types: false,
                multi_value: false,
                component_model: false,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let has_simd = cfg!(target_feature = "simd128");
            let is_wasip2 = cfg!(all(target_os = "wasi", target_env = "p2"));
            let isa = if is_wasip2 {
                IsaKind::Wasm32WasiP2
            } else if has_simd {
                IsaKind::Wasm32Simd128
            } else {
                IsaKind::Wasm32
            };
            Self {
                isa,
                pointer_width_bits: 32,
                simd_width_bits: if has_simd { 128 } else { 0 },
                calling_convention: CallingConvention::WasmStack,
                page_size_bytes: WASM_LINEAR_PAGE_BYTES,
                bulk_memory: cfg!(target_feature = "bulk-memory"),
                reference_types: cfg!(target_feature = "reference-types"),
                multi_value: cfg!(target_feature = "multivalue"),
                component_model: is_wasip2,
            }
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "wasm32"
        )))]
        {
            Self {
                isa: IsaKind::Portable,
                pointer_width_bits: (core::mem::size_of::<usize>() * 8) as u8,
                simd_width_bits: 0,
                calling_convention: CallingConvention::SystemV,
                page_size_bytes: PAGE_SIZE_4K,
                bulk_memory: false,
                reference_types: false,
                multi_value: false,
                component_model: false,
            }
        }
    }

    /// Constructs a canonical [`IsaProfile`] preset for cross-target planning and testing.
    #[must_use]
    pub const fn for_preset(isa: IsaKind, windows_abi: bool, apple_16k_pages: bool) -> Self {
        match isa {
            IsaKind::X86_64 => Self {
                isa,
                pointer_width_bits: 64,
                simd_width_bits: 128,
                calling_convention: if windows_abi {
                    CallingConvention::Win64
                } else {
                    CallingConvention::SystemV
                },
                page_size_bytes: PAGE_SIZE_4K,
                bulk_memory: false,
                reference_types: false,
                multi_value: false,
                component_model: false,
            },
            IsaKind::Aarch64 => Self {
                isa,
                pointer_width_bits: 64,
                simd_width_bits: 128,
                calling_convention: CallingConvention::Aapcs64,
                page_size_bytes: if apple_16k_pages {
                    PAGE_SIZE_16K
                } else {
                    PAGE_SIZE_4K
                },
                bulk_memory: false,
                reference_types: false,
                multi_value: false,
                component_model: false,
            },
            IsaKind::Wasm32 => Self {
                isa,
                pointer_width_bits: 32,
                simd_width_bits: 0,
                calling_convention: CallingConvention::WasmStack,
                page_size_bytes: WASM_LINEAR_PAGE_BYTES,
                bulk_memory: true,
                reference_types: true,
                multi_value: true,
                component_model: false,
            },
            IsaKind::Wasm32Simd128 => Self {
                isa,
                pointer_width_bits: 32,
                simd_width_bits: 128,
                calling_convention: CallingConvention::WasmStack,
                page_size_bytes: WASM_LINEAR_PAGE_BYTES,
                bulk_memory: true,
                reference_types: true,
                multi_value: true,
                component_model: false,
            },
            IsaKind::Wasm32WasiP2 => Self {
                isa,
                pointer_width_bits: 32,
                simd_width_bits: 0,
                calling_convention: CallingConvention::WasmStack,
                page_size_bytes: WASM_LINEAR_PAGE_BYTES,
                bulk_memory: true,
                reference_types: true,
                multi_value: true,
                component_model: true,
            },
            IsaKind::Portable => Self {
                isa,
                pointer_width_bits: 64,
                simd_width_bits: 0,
                calling_convention: CallingConvention::SystemV,
                page_size_bytes: PAGE_SIZE_4K,
                bulk_memory: false,
                reference_types: false,
                multi_value: false,
                component_model: false,
            },
        }
    }
}

/// Emits an architecture-appropriate spin-wait hint inside busy-wait loops.
///
/// - `x86_64`: `pause` (`F3 90`) via `core::arch::asm!`
/// - `aarch64`: `yield` via `core::arch::asm!`
/// - `wasm32` / portable: [`core::hint::spin_loop()`]
#[inline]
pub fn spin_wait_hint() {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: `pause` is an unprivileged architectural spin-loop hint (`rep nop`)
        // that alters neither registers, memory, stack, nor `RFLAGS`.
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack, preserves_flags));
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: `yield` is an unprivileged A64 hint instruction (`HINT #1`) that
        // modifies no architectural registers, condition flags, stack, or memory.
        unsafe {
            core::arch::asm!("yield", options(nomem, nostack, preserves_flags));
        }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        core::hint::spin_loop();
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
static MONOTONIC_FALLBACK_TICKS: AtomicU64 = AtomicU64::new(1);

/// Reads a monotonic high-resolution hardware or runtime tick counter.
///
/// - `x86_64`: `rdtsc` via `core::arch::asm!` (64-bit CPU cycle counter)
/// - `aarch64`: `mrs ..., cntvct_el0` via `core::arch::asm!` (EL0 virtual timer counter)
/// - `wasm32-wasip2`: monotonic nanoseconds anchored at first call combined with an atomic floor
/// - `wasm32-unknown-unknown` / portable: lock-free monotonic `AtomicU64` tick counter (panic-free)
#[inline]
#[must_use]
pub fn read_timestamp_ticks() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        let lo: u32;
        let hi: u32;
        // SAFETY: `rdtsc` is unprivileged when `CR4.TSD = 0` (standard user/kernel environments)
        // and writes the 64-bit time-stamp counter strictly into `EAX` and `EDX`.
        unsafe {
            core::arch::asm!(
                "rdtsc",
                out("eax") lo,
                out("edx") hi,
                options(nomem, nostack, preserves_flags),
            );
        }
        ((hi as u64) << 32) | (lo as u64)
    }
    #[cfg(target_arch = "aarch64")]
    {
        let ticks: u64;
        // SAFETY: `cntvct_el0` is the EL0-accessible virtual timer counter register;
        // reading it modifies no memory, stack, or NZCV flags.
        unsafe {
            core::arch::asm!(
                "mrs {ticks}, cntvct_el0",
                ticks = out(reg) ticks,
                options(nomem, nostack, preserves_flags),
            );
        }
        ticks
    }
    #[cfg(all(target_arch = "wasm32", target_os = "wasi"))]
    {
        use std::sync::OnceLock;
        use std::time::Instant;

        static EPOCH: OnceLock<Instant> = OnceLock::new();
        let origin = EPOCH.get_or_init(Instant::now);
        let ns = origin.elapsed().as_nanos() as u64;
        let stepped = MONOTONIC_FALLBACK_TICKS.fetch_add(1, Ordering::Relaxed);
        ns.max(stepped)
    }
    #[cfg(all(
        not(target_arch = "x86_64"),
        not(target_arch = "aarch64"),
        not(all(target_arch = "wasm32", target_os = "wasi"))
    ))]
    {
        MONOTONIC_FALLBACK_TICKS.fetch_add(1, Ordering::Relaxed)
    }
}

/// Returns the current count of 64 KiB linear memory pages on WebAssembly (`memory.size`),
/// or `None` on native MMU platforms.
#[inline]
#[must_use]
pub fn wasm_linear_memory_pages() -> Option<usize> {
    #[cfg(target_arch = "wasm32")]
    {
        Some(core::arch::wasm32::memory_size::<0>())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Portable scalar saturating RGBA8 additive blit (`dst[i] = dst[i].saturating_add(src[i])`).
///
/// Returns the number of processed bytes (`dst.len().min(src.len())`).
#[inline]
pub fn blit_rgba_alpha_add_scalar(dst: &mut [u8], src: &[u8]) -> usize {
    let len = dst.len().min(src.len());
    for (d, s) in dst[..len].iter_mut().zip(&src[..len]) {
        *d = d.saturating_add(*s);
    }
    len
}

/// Unified single-codebase saturating RGBA8 additive blit.
///
/// On `wasm32` compiled with `-C target-feature=+simd128`, dispatches 16-byte chunks through
/// Core 2.0 `v128.load`, `i8x16`/`u8x16` `u8x16_add_sat`, and `v128.store`, handling any
/// `< 16`-byte tail identically to [`blit_rgba_alpha_add_scalar`].
pub fn blit_rgba_alpha_add_simd128(dst: &mut [u8], src: &[u8]) -> usize {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::{u8x16_add_sat, v128, v128_load, v128_store};

        let len = dst.len().min(src.len());
        let (dst_head, dst_tail) = dst[..len].split_at_mut(len - (len % 16));
        let (src_head, src_tail) = src[..len].split_at(len - (len % 16));

        for (dst_chunk, src_chunk) in dst_head.chunks_exact_mut(16).zip(src_head.chunks_exact(16)) {
            // SAFETY: `chunks_exact_mut(16)` and `chunks_exact(16)` yield disjoint, valid,
            // 16-byte slices; `v128_load`/`v128_store` perform 1-aligned unaligned accesses.
            unsafe {
                let d_vec = v128_load(dst_chunk.as_ptr().cast::<v128>());
                let s_vec = v128_load(src_chunk.as_ptr().cast::<v128>());
                v128_store(
                    dst_chunk.as_mut_ptr().cast::<v128>(),
                    u8x16_add_sat(d_vec, s_vec),
                );
            }
        }
        blit_rgba_alpha_add_scalar(dst_tail, src_tail);
        len
    }
    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        blit_rgba_alpha_add_scalar(dst, src)
    }
}

/// Portable scalar 32-bit word XOR checksum (`words.iter().fold(0, |a, &b| a ^ b)`).
#[inline]
#[must_use]
pub fn xor_checksum_words_scalar(words: &[u32]) -> u32 {
    words.iter().copied().fold(0u32, |acc, w| acc ^ w)
}

/// Unified single-codebase 32-bit word XOR checksum.
///
/// On `wasm32` compiled with `-C target-feature=+simd128`, processes 4 `u32` lanes per cycle
/// using `v128.load` and `v128.xor`, then reduces lanes and tail bytes bit-identically to
/// [`xor_checksum_words_scalar`].
#[must_use]
pub fn xor_checksum_words(words: &[u32]) -> u32 {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::{i32x4_extract_lane, u32x4_splat, v128, v128_load, v128_xor};

        let mut acc = u32x4_splat(0);
        let mut chunks = words.chunks_exact(4);
        for chunk in &mut chunks {
            // SAFETY: `chunks_exact(4)` of `u32` guarantees a valid 16-byte readable slice;
            // `v128_load` performs a 1-aligned load requiring no extra pointer alignment.
            let vec = unsafe { v128_load(chunk.as_ptr().cast::<v128>()) };
            acc = v128_xor(acc, vec);
        }
        let folded = (i32x4_extract_lane::<0>(acc) as u32)
            ^ (i32x4_extract_lane::<1>(acc) as u32)
            ^ (i32x4_extract_lane::<2>(acc) as u32)
            ^ (i32x4_extract_lane::<3>(acc) as u32);
        folded ^ xor_checksum_words_scalar(chunks.remainder())
    }
    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        xor_checksum_words_scalar(words)
    }
}

/// WebAssembly Core 2.0 value type encodings (`valtype`, W3C Core 2.0 §5.3.1–§5.3.4 / §A.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WasmValueType {
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
    V128 = 0x7B,
    FuncRef = 0x70,
    ExternRef = 0x6F,
}

impl WasmValueType {
    /// Decodes a single-byte `valtype` tag.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x7F => Some(Self::I32),
            0x7E => Some(Self::I64),
            0x7D => Some(Self::F32),
            0x7C => Some(Self::F64),
            0x7B => Some(Self::V128),
            0x70 => Some(Self::FuncRef),
            0x6F => Some(Self::ExternRef),
            _ => None,
        }
    }

    /// Canonical `.wat` textual name.
    #[must_use]
    pub const fn wat_name(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::V128 => "v128",
            Self::FuncRef => "funcref",
            Self::ExternRef => "externref",
        }
    }
}

/// WebAssembly Core 2.0 binary section IDs (`0..=12`, W3C Core 2.0 §5.5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WasmSectionId {
    Custom = 0,
    Type = 1,
    Import = 2,
    Function = 3,
    Table = 4,
    Memory = 5,
    Global = 6,
    Export = 7,
    Start = 8,
    Element = 9,
    Code = 10,
    Data = 11,
    DataCount = 12,
}

impl WasmSectionId {
    /// All 13 Core 2.0 section identifiers in numeric ID order (`0..=12`).
    pub const ALL: [Self; 13] = [
        Self::Custom,
        Self::Type,
        Self::Import,
        Self::Function,
        Self::Table,
        Self::Memory,
        Self::Global,
        Self::Export,
        Self::Start,
        Self::Element,
        Self::Code,
        Self::Data,
        Self::DataCount,
    ];

    /// Parses a single-byte section ID (`0..=12`).
    #[must_use]
    pub const fn from_byte(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Custom),
            1 => Some(Self::Type),
            2 => Some(Self::Import),
            3 => Some(Self::Function),
            4 => Some(Self::Table),
            5 => Some(Self::Memory),
            6 => Some(Self::Global),
            7 => Some(Self::Export),
            8 => Some(Self::Start),
            9 => Some(Self::Element),
            10 => Some(Self::Code),
            11 => Some(Self::Data),
            12 => Some(Self::DataCount),
            _ => None,
        }
    }

    /// Lowercase section slug aligned with W3C Core 2.0 and `tooling/bun/web-build.ts`.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Type => "type",
            Self::Import => "import",
            Self::Function => "function",
            Self::Table => "table",
            Self::Memory => "memory",
            Self::Global => "global",
            Self::Export => "export",
            Self::Start => "start",
            Self::Element => "element",
            Self::Code => "code",
            Self::Data => "data",
            Self::DataCount => "data-count",
        }
    }

    /// Canonical binary emission order per W3C Core 2.0 §5.5.16.
    ///
    /// Notice `DataCount` (`12`) precedes `Code` (`10`) and `Data` (`11`) so single-pass
    /// validators can verify `memory.init` and `data.drop` indices before reading `Data`.
    /// `Custom` (`0`) sections may appear anywhere and return `None`.
    #[must_use]
    pub const fn canonical_order_rank(self) -> Option<u8> {
        match self {
            Self::Custom => None,
            Self::Type => Some(0),
            Self::Import => Some(1),
            Self::Function => Some(2),
            Self::Table => Some(3),
            Self::Memory => Some(4),
            Self::Global => Some(5),
            Self::Export => Some(6),
            Self::Start => Some(7),
            Self::Element => Some(8),
            Self::DataCount => Some(9),
            Self::Code => Some(10),
            Self::Data => Some(11),
        }
    }
}

/// Byte span of a single decoded WebAssembly binary section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmSectionHeader {
    pub id: WasmSectionId,
    pub payload_offset: usize,
    pub payload_len: usize,
}

/// Parsed WebAssembly Core 2.0 binary preamble and section table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmCore2Header {
    pub version: u32,
    pub total_bytes: usize,
    pub sections: Vec<WasmSectionHeader>,
}

impl WasmCore2Header {
    /// Returns the total payload size in bytes across all sections matching `id`.
    #[must_use]
    pub fn section_bytes(&self, id: WasmSectionId) -> usize {
        self.sections
            .iter()
            .filter(|section| section.id == id)
            .map(|section| section.payload_len)
            .sum()
    }
}

/// Errors produced when scanning or validating a WebAssembly Core 2.0 binary header.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WasmBinaryError {
    #[error("binary too short for 8-byte Wasm preamble ({len} bytes)")]
    TooShort { len: usize },
    #[error("invalid Wasm magic header (expected \\0asm)")]
    InvalidMagic,
    #[error("unsupported Wasm binary version {0} (expected 1)")]
    UnsupportedVersion(u32),
    #[error("unknown Core 2.0 section id {id} at byte offset {offset}")]
    UnknownSectionId { offset: usize, id: u8 },
    #[error("malformed unsigned LEB128 u32 at byte offset {offset}")]
    MalformedLeb128 { offset: usize },
    #[error("unexpected end of binary while decoding at byte offset {offset}")]
    UnexpectedEof { offset: usize },
    #[error(
        "section '{id:?}' at offset {offset} declares {declared_len} bytes, but only {remaining} remain"
    )]
    TruncatedSection {
        id: WasmSectionId,
        offset: usize,
        declared_len: usize,
        remaining: usize,
    },
    #[error("duplicate non-custom section '{id:?}' at byte offset {offset}")]
    DuplicateSection { id: WasmSectionId, offset: usize },
    #[error("out-of-order section '{id:?}' at byte offset {offset}")]
    OutOfOrderSection { id: WasmSectionId, offset: usize },
    #[error("invalid UTF-8 custom section name at byte offset {offset}")]
    InvalidCustomNameUtf8 { offset: usize },
}

/// Encodes an unsigned 32-bit integer in canonical WebAssembly LEB128 (`u32`, Core 2.0 §5.2.2).
pub fn encode_uleb128_u32(mut value: u32, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Decodes an unsigned 32-bit integer in WebAssembly LEB128 (`u32`, Core 2.0 §5.2.2)
/// starting at `bytes[start..]`, returning `(value, bytes_consumed)`.
pub fn decode_uleb128_u32(bytes: &[u8], start: usize) -> Result<(u32, usize), WasmBinaryError> {
    let mut value: u32 = 0;
    let mut shift: u32 = 0;
    for index in 0..5 {
        let pos = start + index;
        let Some(&byte) = bytes.get(pos) else {
            return Err(WasmBinaryError::UnexpectedEof { offset: pos });
        };
        if index == 4 && ((byte & 0x80) != 0 || (byte & 0xF0) != 0) {
            return Err(WasmBinaryError::MalformedLeb128 { offset: start });
        }
        value |= u32::from(byte & 0x7F) << shift;
        if (byte & 0x80) == 0 {
            return Ok((value, index + 1));
        }
        shift += 7;
    }
    Err(WasmBinaryError::MalformedLeb128 { offset: start })
}

fn push_wasm_section(out: &mut Vec<u8>, id: WasmSectionId, payload: &[u8]) {
    out.push(id as u8);
    encode_uleb128_u32(payload.len() as u32, out);
    out.extend_from_slice(payload);
}

/// Encodes a minimal valid WebAssembly Core 2.0 module exporting `noop() -> ()`.
///
/// Equivalent textual module: [`WASM_CORE2_NOOP_WAT`].
#[must_use]
pub fn encode_wasm_core2_noop_module() -> Vec<u8> {
    let mut out = Vec::with_capacity(41);
    out.extend_from_slice(&WASM_MAGIC);
    out.extend_from_slice(&WASM_BINARY_VERSION.to_le_bytes());

    // Section 1 (Type): 1 function type `() -> ()` (0x60 0x00 0x00).
    push_wasm_section(&mut out, WasmSectionId::Type, &[0x01, 0x60, 0x00, 0x00]);
    // Section 3 (Function): 1 function referencing type index 0.
    push_wasm_section(&mut out, WasmSectionId::Function, &[0x01, 0x00]);
    // Section 7 (Export): 1 export named "noop" -> func 0 (kind 0x00, idx 0x00).
    push_wasm_section(
        &mut out,
        WasmSectionId::Export,
        &[0x01, 0x04, b'n', b'o', b'o', b'p', 0x00, 0x00],
    );
    // Section 10 (Code): 1 body (len 3: 0 locals, `nop` 0x01, `end` 0x0B).
    push_wasm_section(
        &mut out,
        WasmSectionId::Code,
        &[0x01, 0x03, 0x00, 0x01, 0x0B],
    );
    out
}

/// Parses a WebAssembly Core 2.0 binary preamble and section directory (`0..=12`),
/// verifying LEB128 section lengths, bounds, uniqueness, and canonical §5.5.16 order.
pub fn parse_wasm_core2_header(bytes: &[u8]) -> Result<WasmCore2Header, WasmBinaryError> {
    if bytes.len() < 8 {
        return Err(WasmBinaryError::TooShort { len: bytes.len() });
    }
    if bytes[0..4] != WASM_MAGIC {
        return Err(WasmBinaryError::InvalidMagic);
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != WASM_BINARY_VERSION {
        return Err(WasmBinaryError::UnsupportedVersion(version));
    }

    let mut sections = Vec::new();
    let mut offset = 8usize;
    let mut last_rank: Option<u8> = None;

    while offset < bytes.len() {
        let section_offset = offset;
        let raw_id = bytes[offset];
        offset += 1;

        let Some(id) = WasmSectionId::from_byte(raw_id) else {
            return Err(WasmBinaryError::UnknownSectionId {
                offset: section_offset,
                id: raw_id,
            });
        };

        if let Some(rank) = id.canonical_order_rank()
            && let Some(prev_rank) = last_rank
        {
            if rank == prev_rank {
                return Err(WasmBinaryError::DuplicateSection {
                    id,
                    offset: section_offset,
                });
            }
            if rank < prev_rank {
                return Err(WasmBinaryError::OutOfOrderSection {
                    id,
                    offset: section_offset,
                });
            }
        }
        if let Some(rank) = id.canonical_order_rank() {
            last_rank = Some(rank);
        }

        let (payload_len_u32, leb_bytes) = decode_uleb128_u32(bytes, offset)?;
        offset += leb_bytes;

        let payload_len = payload_len_u32 as usize;
        let remaining = bytes.len() - offset;
        if payload_len > remaining {
            return Err(WasmBinaryError::TruncatedSection {
                id,
                offset: section_offset,
                declared_len: payload_len,
                remaining,
            });
        }

        sections.push(WasmSectionHeader {
            id,
            payload_offset: offset,
            payload_len,
        });
        offset += payload_len;
    }

    Ok(WasmCore2Header {
        version,
        total_bytes: bytes.len(),
        sections,
    })
}

/// Decodes the UTF-8 name of a [`WasmSectionId::Custom`] section (`vec(byte)`, Core 2.0 §5.5.3).
pub fn parse_custom_section_name(
    bytes: &[u8],
    section: WasmSectionHeader,
) -> Result<&str, WasmBinaryError> {
    let payload_end = section
        .payload_offset
        .checked_add(section.payload_len)
        .ok_or(WasmBinaryError::UnexpectedEof {
            offset: section.payload_offset,
        })?;
    let payload =
        bytes
            .get(section.payload_offset..payload_end)
            .ok_or(WasmBinaryError::UnexpectedEof {
                offset: section.payload_offset,
            })?;
    let (name_len_u32, leb_bytes) = decode_uleb128_u32(payload, 0)?;
    let name_len = name_len_u32 as usize;
    let name_bytes =
        payload
            .get(leb_bytes..leb_bytes + name_len)
            .ok_or(WasmBinaryError::UnexpectedEof {
                offset: section.payload_offset + leb_bytes,
            })?;
    core::str::from_utf8(name_bytes).map_err(|_| WasmBinaryError::InvalidCustomNameUtf8 {
        offset: section.payload_offset + leb_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isa_profile_presets_and_active_target_are_consistent() {
        let active = IsaProfile::current();
        assert!(!active.isa.slug().is_empty());
        assert_eq!(active.calling_convention.stack_alignment_bytes(), 16);
        assert!(active.pointer_width_bits == 32 || active.pointer_width_bits == 64);

        let sysv = IsaProfile::for_preset(IsaKind::X86_64, false, false);
        assert_eq!(sysv.calling_convention, CallingConvention::SystemV);
        assert_eq!(sysv.calling_convention.arg_registers().len(), 6);
        assert_eq!(sysv.page_size_bytes, PAGE_SIZE_4K);

        let win64 = IsaProfile::for_preset(IsaKind::X86_64, true, false);
        assert_eq!(win64.calling_convention, CallingConvention::Win64);
        assert_eq!(win64.calling_convention.arg_registers().len(), 4);

        let apple_arm = IsaProfile::for_preset(IsaKind::Aarch64, false, true);
        assert_eq!(apple_arm.calling_convention, CallingConvention::Aapcs64);
        assert_eq!(apple_arm.page_size_bytes, PAGE_SIZE_16K);
        assert_eq!(apple_arm.simd_width_bits, 128);

        let wasm_simd = IsaProfile::for_preset(IsaKind::Wasm32Simd128, false, false);
        assert_eq!(wasm_simd.page_size_bytes, WASM_LINEAR_PAGE_BYTES);
        assert_eq!(wasm_simd.simd_width_bits, 128);
        assert!(wasm_simd.bulk_memory && wasm_simd.reference_types && wasm_simd.multi_value);

        let wasip2 = IsaProfile::for_preset(IsaKind::Wasm32WasiP2, false, false);
        assert!(wasip2.component_model);
        assert_eq!(wasip2.calling_convention, CallingConvention::WasmStack);
    }

    #[test]
    fn spin_wait_and_monotonic_timestamps_execute_safely() {
        let t0 = read_timestamp_ticks();
        spin_wait_hint();
        spin_wait_hint();
        let t1 = read_timestamp_ticks();
        assert!(t1 >= t0, "timestamp counter moved backwards: {t0} -> {t1}");
        if cfg!(target_arch = "wasm32") {
            assert!(wasm_linear_memory_pages().is_some());
        } else {
            assert_eq!(wasm_linear_memory_pages(), None);
        }
    }

    #[test]
    fn simd128_and_scalar_kernels_are_bit_identical() {
        for len in [0usize, 1, 3, 15, 16, 17, 31, 64, 131] {
            let initial: Vec<u8> = (0..len).map(|i| ((i * 37) & 0xFF) as u8).collect();
            let addend: Vec<u8> = (0..len).map(|i| ((i * 53 + 200) & 0xFF) as u8).collect();

            let mut dst_scalar = initial.clone();
            let mut dst_simd = initial;

            let n_scalar = blit_rgba_alpha_add_scalar(&mut dst_scalar, &addend);
            let n_simd = blit_rgba_alpha_add_simd128(&mut dst_simd, &addend);
            assert_eq!(n_scalar, len);
            assert_eq!(n_simd, len);
            assert_eq!(dst_simd, dst_scalar, "blit mismatch at len={len}");
        }

        for word_len in [0usize, 1, 2, 3, 4, 5, 7, 8, 19, 64] {
            let words: Vec<u32> = (0..word_len)
                .map(|i| 0x9E37_79B9u32.wrapping_mul((i as u32) + 1))
                .collect();
            assert_eq!(
                xor_checksum_words(&words),
                xor_checksum_words_scalar(&words),
                "xor checksum mismatch at word_len={word_len}"
            );
        }
    }

    #[test]
    fn encode_and_parse_wasm_core2_modules_and_sections() {
        let noop = encode_wasm_core2_noop_module();
        assert!(WASM_CORE2_NOOP_WAT.contains("(export \"noop\")"));
        let parsed = parse_wasm_core2_header(&noop).expect("noop module must parse");
        assert_eq!(parsed.version, WASM_BINARY_VERSION);
        assert_eq!(
            parsed.sections.iter().map(|s| s.id).collect::<Vec<_>>(),
            vec![
                WasmSectionId::Type,
                WasmSectionId::Function,
                WasmSectionId::Export,
                WasmSectionId::Code,
            ]
        );
        assert_eq!(parsed.section_bytes(WasmSectionId::Code), 5);

        // Parse the 31-byte SIMD128 probe from web-build.ts.
        let simd_hdr = parse_wasm_core2_header(&WASM_CORE2_SIMD_PROBE_BYTES)
            .expect("SIMD probe binary must parse");
        assert_eq!(simd_hdr.sections.len(), 3);
        assert_eq!(simd_hdr.sections[0].id, WasmSectionId::Type);
        assert_eq!(simd_hdr.sections[1].id, WasmSectionId::Function);
        assert_eq!(simd_hdr.sections[2].id, WasmSectionId::Code);
    }

    #[test]
    fn leb128_and_core2_all_sections_0_through_12_parse_in_canonical_order() {
        let mut leb = Vec::new();
        for val in [0u32, 1, 127, 128, 65_536, u32::MAX] {
            leb.clear();
            encode_uleb128_u32(val, &mut leb);
            let (decoded, consumed) = decode_uleb128_u32(&leb, 0).expect("valid uleb128");
            assert_eq!(decoded, val);
            assert_eq!(consumed, leb.len());
        }

        // Synthesize a binary touching every Core 2.0 section ID 0..=12 in §5.5.16 order:
        // Custom(0), Type(1), Import(2), Function(3), Table(4), Memory(5), Global(6),
        // Export(7), Start(8), Element(9), DataCount(12), Code(10), Data(11), Custom(0).
        let mut module = Vec::new();
        module.extend_from_slice(&WASM_MAGIC);
        module.extend_from_slice(&WASM_BINARY_VERSION.to_le_bytes());

        let custom_payload = [0x04, b'n', b'a', b'm', b'e'];
        push_wasm_section(&mut module, WasmSectionId::Custom, &custom_payload);
        for id in [
            WasmSectionId::Type,
            WasmSectionId::Import,
            WasmSectionId::Function,
            WasmSectionId::Table,
            WasmSectionId::Memory,
            WasmSectionId::Global,
            WasmSectionId::Export,
            WasmSectionId::Start,
            WasmSectionId::Element,
            WasmSectionId::DataCount,
            WasmSectionId::Code,
            WasmSectionId::Data,
        ] {
            push_wasm_section(&mut module, id, &[0x00]);
        }
        push_wasm_section(&mut module, WasmSectionId::Custom, &custom_payload);

        let parsed = parse_wasm_core2_header(&module).expect("all Core 2.0 sections must parse");
        assert_eq!(parsed.sections.len(), 14);
        assert_eq!(
            parse_custom_section_name(&module, parsed.sections[0]).expect("custom name"),
            "name"
        );
        for expected_id in WasmSectionId::ALL {
            assert!(
                parsed.sections.iter().any(|s| s.id == expected_id),
                "missing section {:?}",
                expected_id
            );
        }
    }

    #[test]
    fn rejects_malformed_wasm_headers_leb128_and_out_of_order_sections() {
        assert_eq!(
            parse_wasm_core2_header(&[0x00, 0x61]),
            Err(WasmBinaryError::TooShort { len: 2 })
        );
        assert_eq!(
            parse_wasm_core2_header(b"NOTW\x01\0\0\0"),
            Err(WasmBinaryError::InvalidMagic)
        );
        assert_eq!(
            parse_wasm_core2_header(b"\0asm\x02\0\0\0"),
            Err(WasmBinaryError::UnsupportedVersion(2))
        );
        assert_eq!(
            decode_uleb128_u32(&[0x80, 0x80, 0x80, 0x80, 0x10], 0),
            Err(WasmBinaryError::MalformedLeb128 { offset: 0 })
        );

        // Out-of-order: Data (11) before Code (10) violates Core 2.0 §5.5.16.
        let mut bad_order = Vec::new();
        bad_order.extend_from_slice(&WASM_MAGIC);
        bad_order.extend_from_slice(&WASM_BINARY_VERSION.to_le_bytes());
        push_wasm_section(&mut bad_order, WasmSectionId::Data, &[0x00]);
        push_wasm_section(&mut bad_order, WasmSectionId::Code, &[0x00]);
        assert!(matches!(
            parse_wasm_core2_header(&bad_order),
            Err(WasmBinaryError::OutOfOrderSection {
                id: WasmSectionId::Code,
                ..
            })
        ));
    }

    #[test]
    fn value_types_cover_core2_specification() {
        assert_eq!(WasmValueType::from_byte(0x7F), Some(WasmValueType::I32));
        assert_eq!(WasmValueType::from_byte(0x7E), Some(WasmValueType::I64));
        assert_eq!(WasmValueType::from_byte(0x7D), Some(WasmValueType::F32));
        assert_eq!(WasmValueType::from_byte(0x7C), Some(WasmValueType::F64));
        assert_eq!(WasmValueType::from_byte(0x7B), Some(WasmValueType::V128));
        assert_eq!(WasmValueType::from_byte(0x70), Some(WasmValueType::FuncRef));
        assert_eq!(
            WasmValueType::from_byte(0x6F),
            Some(WasmValueType::ExternRef)
        );
        assert_eq!(WasmValueType::from_byte(0x7B).unwrap().wat_name(), "v128");
    }
}
