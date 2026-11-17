//! Craft Anything Google Edition Core: clean-room X64 / A64 instruction decoding
//! (`craft-anything-disasm`), `oxc` JavaScript/TypeScript static AST & bundle analyzer
//! (`craft-anything-js`), ELF/PE/Mach-O inspection (`craft-elf`, `craft-pe`, `craft-macho`),
//! and SHA-256 deterministic evidence ledger (`craft-anything-evidence`).

use craft_anything_disasm::{Mode as DisasmMode, decode as decode_insn};
use craft_anything_evidence::{Severity, Unknown, sha256_hex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisassembledInsnRow {
    pub address_hex: String,
    pub arch: String,
    pub bytes_hex: String,
    pub mnemonic_text: String,
    pub ends_basic_block: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsAstReport {
    pub path: String,
    pub mode: String,
    pub export_count: usize,
    pub import_count: usize,
    pub string_literal_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceLedgerEntry {
    pub sha256: String,
    pub subject_name: String,
    pub container_kind: String,
    pub byte_len: usize,
    pub open_unknowns: Vec<String>,
}

/// Shared Craft Anything workbench state consumed identically by both GPUI and `egui 0.37` frontends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftAnythingWorkbench {
    pub active_subject: String,
    pub disasm_rows: Vec<DisassembledInsnRow>,
    pub js_report: Option<JsAstReport>,
    pub evidence: Vec<EvidenceLedgerEntry>,
    pub elf_program_headers: usize,
}

impl Default for CraftAnythingWorkbench {
    fn default() -> Self {
        Self::default_google_workbench()
    }
}

impl CraftAnythingWorkbench {
    /// Initializes a live clean-room workbench with decoded X64 & A64 prologues, `oxc` TSX AST facts, and SHA-256 evidence.
    #[must_use]
    pub fn default_google_workbench() -> Self {
        // Canonical X64 function prologue + arithmetic + ret (`push rbp; mov rbp, rsp; lea rax, [rdi+rsi*4]; pop rbp; ret`)
        let x64_sample: &[u8] = &[
            0x55, 0x48, 0x89, 0xE5, 0x48, 0x8D, 0x04, 0xB7, 0x5D, 0xC3,
        ];
        // Canonical A64 prologue + add + ret (`add x0, x0, x1; ret`)
        let a64_sample: &[u8] = &[0x00, 0x00, 0x01, 0x8B, 0xC0, 0x03, 0x5F, 0xD6];

        let mut rows = Self::disassemble_slice(DisasmMode::X64, x64_sample, 0x1400_1000, 16);
        rows.extend(Self::disassemble_slice(
            DisasmMode::A64,
            a64_sample,
            0x0040_2000,
            16,
        ));

        let tsx_sample = r#"
import { mount, signal } from "nsh";
export const STUDIO_VERSION = "1.0.0";
export function bootStudio() {
  const ready = signal(true);
  return ready();
}
"#;
        let js_report = Some(Self::analyze_js("agy-session-studio.tsx", tsx_sample));

        let unknown = Unknown::new(
            "Verify Wasm SIMD128 dispatch table bounds",
            "wasm_hal",
            Severity::Medium,
        );
        let digest = sha256_hex(x64_sample);

        Self {
            active_subject: "aphrody_studio_kernel.bin".to_owned(),
            disasm_rows: rows,
            js_report,
            evidence: vec![EvidenceLedgerEntry {
                sha256: digest,
                subject_name: "x64_prologue_specimen".to_owned(),
                container_kind: "RawCodeX64+A64".to_owned(),
                byte_len: x64_sample.len() + a64_sample.len(),
                open_unknowns: vec![format!("[{:?}] {}", unknown.severity, unknown.question)],
            }],
            elf_program_headers: 0,
        }
    }

    /// Decodes up to `max_insns` instructions from `bytes` using `craft-anything-disasm`.
    #[must_use]
    pub fn disassemble_slice(
        mode: DisasmMode,
        bytes: &[u8],
        base_addr: u64,
        max_insns: usize,
    ) -> Vec<DisassembledInsnRow> {
        let mut out = Vec::new();
        let mut offset = 0usize;
        while offset < bytes.len() && out.len() < max_insns {
            let addr = base_addr.saturating_add(offset as u64);
            let Some(slice) = bytes.get(offset..) else {
                break;
            };
            let Some(insn) = decode_insn(mode, slice, addr, true) else {
                break;
            };
            let step = (insn.end().saturating_sub(addr) as usize).max(1);
            let raw_bytes = bytes
                .get(offset..offset.saturating_add(step).min(bytes.len()))
                .unwrap_or(&[]);
            let hex_bytes = raw_bytes
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            let ends_block = insn.flow.ends_block();
            out.push(DisassembledInsnRow {
                address_hex: format!("0x{addr:08X}"),
                arch: mode.name().to_owned(),
                bytes_hex: hex_bytes,
                mnemonic_text: insn.text(),
                ends_basic_block: ends_block,
            });
            offset = offset.saturating_add(step);
        }
        out
    }

    /// Runs `oxc` 0.153 static JS/TS/TSX analysis via `craft-anything-js`.
    #[must_use]
    pub fn analyze_js(path_label: &str, src: &str) -> JsAstReport {
        let analysis =
            craft_anything_js::analyze(path_label, src, &craft_anything_js::Limits::default());
        JsAstReport {
            path: path_label.to_owned(),
            mode: format!("{:?}", analysis.facts.mode),
            export_count: analysis.facts.exports.esm.len() + analysis.facts.exports.cjs.len(),
            import_count: analysis.facts.imports.len(),
            string_literal_count: analysis.facts.strings.len(),
        }
    }

    /// Inspects untrusted binary bytes via `craft-elf` and records a SHA-256 evidence entry.
    pub fn ingest_binary_bytes(&mut self, name: &str, bytes: &[u8]) {
        let digest = sha256_hex(bytes);
        let mut kind = "BinaryBlob".to_owned();
        if let Ok(phdrs) = craft_elf::program_headers(bytes) {
            self.elf_program_headers = phdrs.len();
            kind = format!("ELF ({} program headers)", phdrs.len());
        } else if bytes.starts_with(b"MZ") {
            kind = "PE/COFF".to_owned();
        } else if bytes.starts_with(b"\0asm") {
            kind = "WebAssembly".to_owned();
        }
        self.active_subject = name.to_owned();
        self.evidence.push(EvidenceLedgerEntry {
            sha256: digest,
            subject_name: name.to_owned(),
            container_kind: kind,
            byte_len: bytes.len(),
            open_unknowns: Vec::new(),
        });
    }
}
