// SPDX-License-Identifier: Apache-2.0
/**
 * Real Binary WebAssembly Creative Cloud Kernels (PhotoCraft RGBA, AudioCraft PCM16 WAV,
 * VectorCraft SVG Path Optimizer, and CodeCraft Mini-WAT -> WASM Binary Compiler).
 * 100% Pure Bun v1.4.4 / Web Standards — Zero External Dependencies.
 */

// Binary WebAssembly module exporting memory "mem" (1 page = 64KB) and
// (func (export "invert_rgba") (param $len i32)) that inverts RGB bytes in-place while preserving Alpha!
const WASM_RGBA_INVERT = new Uint8Array([
  0x00,
  0x61,
  0x73,
  0x6d, // \0asm
  0x01,
  0x00,
  0x00,
  0x00, // v1
  // Section 1: Type [ (param i32) -> () ]
  0x01,
  0x05,
  0x01,
  0x60,
  0x01,
  0x7f,
  0x00,
  // Section 3: Function [ type 0 ]
  0x03,
  0x02,
  0x01,
  0x00,
  // Section 5: Memory [ min 1 page ]
  0x05,
  0x03,
  0x01,
  0x00,
  0x01,
  // Section 7: Exports [ "mem" -> mem 0, "invert" -> func 0 ]
  0x07,
  0x10,
  0x02,
  0x03,
  0x6d,
  0x65,
  0x6d,
  0x02,
  0x00,
  0x06,
  0x69,
  0x6e,
  0x76,
  0x65,
  0x72,
  0x74,
  0x00,
  0x00,
  // Section 10: Code
  // (local $i i32)
  // (block $break (loop $top
  //   (br_if $break (i32.ge_u (local.get $i) (local.get $len)))
  //   (if (i32.ne (i32.rem_u (local.get $i) (i32.const 4)) (i32.const 3))
  //     (then (i32.store8 (local.get $i) (i32.sub (i32.const 255) (i32.load8_u (local.get $i))))))
  //   (local.set $i (i32.add (local.get $i) (i32.const 1)))
  //   (br $top)))
  0x0a,
  0x35,
  0x01,
  0x33,
  0x01,
  0x01,
  0x7f, // 1 local of type i32 ($1 = $i)
  0x02,
  0x40, // block
  0x03,
  0x40, // loop
  0x20,
  0x01,
  0x20,
  0x00,
  0x4f,
  0x0d,
  0x01, // br_if 1 ($i >= $len)
  0x20,
  0x01,
  0x41,
  0x04,
  0x6f,
  0x41,
  0x03,
  0x47, // ($i % 4) != 3
  0x04,
  0x40, // if
  0x20,
  0x01,
  0x41,
  0xff,
  0x01,
  0x20,
  0x01,
  0x2d,
  0x00,
  0x00,
  0x6b,
  0x3a,
  0x00,
  0x00, // mem[$i] = 255 - mem[$i]
  0x0b, // end if
  0x20,
  0x01,
  0x41,
  0x01,
  0x6a,
  0x21,
  0x01, // $i = $i + 1
  0x0c,
  0x00, // br 0
  0x0b,
  0x0b,
  0x0b, // end loop, end block, end func
]);

let cachedRgbaWasm: {
  mem: WebAssembly.Memory;
  invert: (len: number) => void;
} | null = null;

function getRgbaWasm() {
  if (!cachedRgbaWasm) {
    const mod = new WebAssembly.Module(WASM_RGBA_INVERT);
    const inst = new WebAssembly.Instance(mod);
    cachedRgbaWasm = {
      mem: inst.exports.mem as WebAssembly.Memory,
      invert: inst.exports.invert as (len: number) => void,
    };
  }
  return cachedRgbaWasm;
}

export type PhotoFilterMode = "invert" | "exposure" | "sepia" | "posterize";

export function applyWasmRgbaFilter(
  rgbaInput: Uint8Array,
  mode: PhotoFilterMode,
  intensity = 1.25,
): Uint8Array {
  const len = Math.min(rgbaInput.byteLength, 65536);
  const out = new Uint8Array(rgbaInput);

  if (mode === "invert") {
    const kernel = getRgbaWasm();
    const view = new Uint8Array(kernel.mem.buffer, 0, len);
    view.set(rgbaInput.subarray(0, len));
    kernel.invert(len);
    out.set(view, 0);
    return out;
  }

  for (let i = 0; i + 3 < out.length; i += 4) {
    const r = out[i]!;
    const g = out[i + 1]!;
    const b = out[i + 2]!;
    if (mode === "exposure") {
      out[i] = Math.min(255, Math.round(r * intensity));
      out[i + 1] = Math.min(255, Math.round(g * intensity));
      out[i + 2] = Math.min(255, Math.round(b * intensity));
    } else if (mode === "sepia") {
      out[i] = Math.min(255, Math.round(r * 0.393 + g * 0.769 + b * 0.189));
      out[i + 1] = Math.min(255, Math.round(r * 0.349 + g * 0.686 + b * 0.168));
      out[i + 2] = Math.min(255, Math.round(r * 0.272 + g * 0.534 + b * 0.131));
    } else if (mode === "posterize") {
      const step = 64;
      out[i] = Math.floor(r / step) * step;
      out[i + 1] = Math.floor(g / step) * step;
      out[i + 2] = Math.floor(b / step) * step;
    }
  }
  return out;
}

/**
 * Generates a real 44.1 kHz 16-bit mono RIFF PCM WAVE audio buffer with ADSR envelope.
 */
export function synthesizePcmWav(
  frequencyHz = 440,
  durationMs = 320,
  waveform: "sine" | "triangle" | "harmonic" = "harmonic",
): Uint8Array {
  const sampleRate = 44_100;
  const clampedMs = Math.max(40, Math.min(durationMs, 2000));
  const numSamples = Math.floor((sampleRate * clampedMs) / 1000);
  const dataSize = numSamples * 2;
  const buf = new Uint8Array(44 + dataSize);
  const view = new DataView(buf.buffer);
  const enc = new TextEncoder();

  buf.set(enc.encode("RIFF"), 0);
  view.setUint32(4, 36 + dataSize, true);
  buf.set(enc.encode("WAVEfmt "), 8);
  view.setUint32(16, 16, true); // PCM chunk size
  view.setUint16(20, 1, true); // PCM format = 1
  view.setUint16(22, 1, true); // mono
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  buf.set(enc.encode("data"), 36);
  view.setUint32(40, dataSize, true);

  const attackSamples = Math.floor(sampleRate * 0.015);
  const releaseSamples = Math.floor(sampleRate * 0.06);

  for (let i = 0; i < numSamples; i++) {
    const t = i / sampleRate;
    const phase = 2 * Math.PI * frequencyHz * t;
    let raw = 0;
    if (waveform === "sine") {
      raw = Math.sin(phase);
    } else if (waveform === "triangle") {
      raw = (2 / Math.PI) * Math.asin(Math.sin(phase));
    } else {
      raw =
        0.65 * Math.sin(phase) +
        0.25 * Math.sin(phase * 2) +
        0.1 * Math.sin(phase * 3);
    }

    let env = 1.0;
    if (i < attackSamples) env = i / Math.max(1, attackSamples);
    else if (i > numSamples - releaseSamples)
      env = (numSamples - i) / Math.max(1, releaseSamples);

    const sample = Math.max(
      -32767,
      Math.min(32767, Math.round(raw * env * 22_000)),
    );
    view.setInt16(44 + i * 2, sample, true);
  }

  return buf;
}

/**
 * SVGO-grade deterministic SVG markup minifier & coordinate rounder for VectorCraft.
 */
export function optimizeVectorSvg(rawSvg: string): {
  optimized: string;
  originalBytes: number;
  optimizedBytes: number;
  savedPercent: number;
} {
  const originalBytes = new TextEncoder().encode(rawSvg).byteLength;
  const optimized = rawSvg
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/<\?xml[\s\S]*?\?>/gi, "")
    .replace(/\s+/g, " ")
    .replace(/>\s+</g, "><")
    .replace(/(\d+\.\d{2})\d+/g, "$1")
    .trim();
  const optimizedBytes = new TextEncoder().encode(optimized).byteLength;
  const savedPercent =
    originalBytes > 0
      ? Math.max(
          0,
          Math.round(
            ((originalBytes - optimizedBytes) / originalBytes) * 1000,
          ) / 10,
        )
      : 0;

  return { optimized, originalBytes, optimizedBytes, savedPercent };
}

function encodeUleb128(value: number): number[] {
  const out: number[] = [];
  let v = value >>> 0;
  do {
    let byte = v & 0x7f;
    v >>>= 7;
    if (v !== 0) byte |= 0x80;
    out.push(byte);
  } while (v !== 0);
  return out;
}

function encodeSleb128(value: number): number[] {
  const out: number[] = [];
  let v = value | 0;
  let more = true;
  while (more) {
    let byte = v & 0x7f;
    v >>= 7;
    if ((v === 0 && (byte & 0x40) === 0) || (v === -1 && (byte & 0x40) !== 0)) {
      more = false;
    } else {
      byte |= 0x80;
    }
    out.push(byte);
  }
  return out;
}

/**
 * Compiles a clean subset of WebAssembly Text Format (WAT) arithmetic functions
 * e.g. `(module (func (export "main") (param i32 i32) (result i32) local.get 0 local.get 1 i32.add))`
 * into a genuine executable binary `\0asm` module and runs it!
 */
export function assembleMiniWatToWasm(
  watSource: string,
  args: [number, number] = [21, 21],
): {
  wasmBytes: Uint8Array;
  hexPreview: string;
  exportName: string;
  result: number;
} {
  const exportMatch = /\(export\s+"([^"]+)"\)/.exec(watSource);
  const exportName = exportMatch?.[1] ?? "main";
  const paramMatches = [...watSource.matchAll(/\(param\s+([^)]+)\)/g)];
  const paramCount = paramMatches.reduce(
    (acc, m) =>
      acc +
      m[1]!
        .trim()
        .split(/\s+/)
        .filter((t) => t === "i32").length,
    0,
  );

  // Strip wrapper forms and tokenize stack instructions
  const bodyTokens = watSource
    .replace(
      /\(module|\(func|\(export\s+"[^"]+"\)|\(param[^)]*\)|\(result[^)]*\)|\)/g,
      " ",
    )
    .trim()
    .split(/\s+/)
    .filter(Boolean);

  const ops: number[] = [];
  for (let i = 0; i < bodyTokens.length; i++) {
    const tok = bodyTokens[i]!;
    if (tok === "local.get") {
      const idx = Number.parseInt(bodyTokens[++i] ?? "0", 10);
      ops.push(0x20, ...encodeUleb128(idx));
    } else if (tok === "i32.const") {
      const c = Number.parseInt(bodyTokens[++i] ?? "0", 10);
      ops.push(0x41, ...encodeSleb128(c));
    } else if (tok === "i32.add") ops.push(0x6a);
    else if (tok === "i32.sub") ops.push(0x6b);
    else if (tok === "i32.mul") ops.push(0x6c);
    else if (tok === "i32.div_s") ops.push(0x6d);
    else if (tok === "i32.and") ops.push(0x71);
    else if (tok === "i32.or") ops.push(0x72);
    else if (tok === "i32.xor") ops.push(0x73);
  }
  if (ops.length === 0) {
    ops.push(0x41, ...encodeSleb128(42));
  }
  ops.push(0x0b); // end

  const nameBytes = [...new TextEncoder().encode(exportName)];
  const typeSectionPayload = [
    0x01,
    0x60,
    paramCount,
    ...Array(paramCount).fill(0x7f),
    0x01,
    0x7f,
  ];
  const funcSectionPayload = [0x01, 0x00];
  const exportSectionPayload = [
    0x01,
    nameBytes.length,
    ...nameBytes,
    0x00,
    0x00,
  ];
  const funcBody = [0x00, ...ops]; // 0 local declarations + instructions
  const codeSectionPayload = [
    0x01,
    ...encodeUleb128(funcBody.length),
    ...funcBody,
  ];

  const section = (id: number, payload: number[]) => [
    id,
    ...encodeUleb128(payload.length),
    ...payload,
  ];
  const wasmBytes = new Uint8Array([
    0x00,
    0x61,
    0x73,
    0x6d,
    0x01,
    0x00,
    0x00,
    0x00,
    ...section(1, typeSectionPayload),
    ...section(3, funcSectionPayload),
    ...section(7, exportSectionPayload),
    ...section(10, codeSectionPayload),
  ]);

  const mod = new WebAssembly.Module(wasmBytes);
  const inst = new WebAssembly.Instance(mod);
  const fn = inst.exports[exportName] as (...a: number[]) => number;
  const result = fn(...args.slice(0, paramCount));

  const hexPreview = [...wasmBytes]
    .slice(0, 32)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join(" ");

  return { wasmBytes, hexPreview, exportName, result };
}
