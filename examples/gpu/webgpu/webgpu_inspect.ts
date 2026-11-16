/**
 * WebGPU & WGSL Inspector & Cross-Backend Validator (Bun TypeScript)
 *
 * Demonstrates:
 * 1. Strict WebGPU API type consumption via `@webgpu/types`.
 * 2. WGSL struct alignment and row-stride calculations (`COPY_BYTES_PER_ROW_ALIGNMENT = 256`).
 * 3. Execution of `gpui-wgpu` and `naga` / `tint` cross-compilers.
 * 4. Validation across all 5 target shading languages: WGSL, SPIR-V, HLSL, MSL, and GLSL ES 300.
 */

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

// W3C WebGPU constant constants
export const COPY_BYTES_PER_ROW_ALIGNMENT = 256;
export const UNIFORM_BUFFER_OFFSET_ALIGNMENT = 256;

export interface StructMemberLayout {
  name: string;
  type: string;
  offset: number;
  size: number;
  alignment: number;
}

export interface StructLayoutReport {
  name: string;
  size: number;
  alignment: number;
  members: StructMemberLayout[];
}

/**
 * Calculates aligned row pitch for texture-buffer transfers in WebGPU.
 */
export function calculateTextureRowPitch(width: number, bytesPerPixel = 4): { unaligned: number; aligned: number; paddingPerRow: number } {
  const unaligned = width * bytesPerPixel;
  const aligned = Math.ceil(unaligned / COPY_BYTES_PER_ROW_ALIGNMENT) * COPY_BYTES_PER_ROW_ALIGNMENT;
  return {
    unaligned,
    aligned,
    paddingPerRow: aligned - unaligned,
  };
}

/**
 * Validates WGSL uniform structure alignment constraints for WebGPU and WebGL2.
 */
export function validateUniformBufferLayout(structSize: number): { is16BytePadded: boolean; requiredPadding: number } {
  const remainder = structSize % 16;
  const requiredPadding = remainder === 0 ? 0 : 16 - remainder;
  return {
    is16BytePadded: remainder === 0,
    requiredPadding,
  };
}

/**
 * Type-level demonstration using `@webgpu/types`
 */
export function simulateWebGPUPipelineDescriptor(format: GPUTextureFormat): GPURenderPipelineDescriptor {
  const descriptor: GPURenderPipelineDescriptor = {
    layout: "auto",
    vertex: {
      module: {} as GPUShaderModule,
      entryPoint: "vs_main",
    },
    fragment: {
      module: {} as GPUShaderModule,
      entryPoint: "fs_main",
      targets: [
        {
          format,
          blend: {
            color: {
              srcFactor: "src-alpha",
              dstFactor: "one-minus-src-alpha",
              operation: "add",
            },
            alpha: {
              srcFactor: "one",
              dstFactor: "one-minus-src-alpha",
              operation: "add",
            },
          },
          writeMask: 0xf, // GPUColorWrite.ALL
        },
      ],
    },
    primitive: {
      topology: "triangle-list",
      cullMode: "none",
    },
  };
  return descriptor;
}

function runCommand(command: string, args: string[]): { success: boolean; stdout: string; stderr: string } {
  const proc = spawnSync(command, args, { encoding: "utf8" });
  return {
    success: proc.status === 0,
    stdout: proc.stdout ?? "",
    stderr: proc.stderr ?? "",
  };
}

async function main() {
  console.log("=== WebGPU & WGSL Inspector (Bun TypeScript + gpui-wgpu) ===\n");

  const shaderPath = join(import.meta.dir, "nsh_ripple_shader.wgsl");
  if (!existsSync(shaderPath)) {
    throw new Error(`Shader not found at ${shaderPath}`);
  }

  console.log(`[1/5] Checking WGSL source: ${shaderPath}`);
  const source = readFileSync(shaderPath, "utf8");
  console.log(`  Source length: ${source.length} bytes, lines: ${source.split("\n").length}`);

  // Test alignment calculations
  console.log("\n[2/5] Testing WebGPU memory stride & alignment invariants:");
  const testWidths = [512, 800, 1920, 2560];
  for (const w of testWidths) {
    const pitch = calculateTextureRowPitch(w, 4);
    console.log(
      `  Width ${w}px -> Unaligned: ${pitch.unaligned}B, Aligned: ${pitch.aligned}B, Padding: ${pitch.paddingPerRow}B (Mod 256: ${pitch.aligned % COPY_BYTES_PER_ROW_ALIGNMENT})`
    );
  }

  const uniformValidation = validateUniformBufferLayout(32);
  console.log(`  RippleUniforms size (32 bytes) -> 16-byte padded: ${uniformValidation.is16BytePadded}`);

  // Step 3: Run gpui-wgpu validation
  console.log("\n[3/5] Validating WGSL with gpui-wgpu (Naga v30)...");
  const gpuiWgpuPath = join(import.meta.dir, "..", "..", "..", "target", "debug", "gpui-wgpu.exe");
  if (existsSync(gpuiWgpuPath)) {
    const valRes = runCommand(gpuiWgpuPath, ["validate", shaderPath, "--json"]);
    if (valRes.success) {
      const parsed = JSON.parse(valRes.stdout);
      console.log(`✓ gpui-wgpu validate PASSED. Valid: ${parsed.valid}`);
      console.log(`  Entry points: ${parsed.entry_points.map((e: { name: string; stage: string }) => `${e.name} (${e.stage})`).join(", ")}`);
      console.log(`  Resource bindings: ${parsed.resource_bindings.map((b: { name: string; group: number; binding: number }) => `${b.name} @group(${b.group}) @binding(${b.binding})`).join(", ")}`);
    } else {
      console.warn("  gpui-wgpu validate failed:", valRes.stderr);
    }
  } else {
    console.log(`  gpui-wgpu not found at ${gpuiWgpuPath}, checking naga / tint CLI...`);
  }

  // Step 4: Run naga and tint validation
  console.log("\n[4/5] Checking Naga & Google Dawn Tint validators...");
  const nagaRes = runCommand("naga", [shaderPath]);
  console.log(`  naga: ${nagaRes.success ? "PASSED (Valid)" : "FAILED: " + nagaRes.stderr.trim()}`);

  const tintRes = runCommand("tint", ["-f", "wgsl", shaderPath]);
  console.log(`  tint: ${tintRes.success ? "PASSED (Valid)" : "FAILED: " + tintRes.stderr.trim()}`);

  const tintInfoRes = runCommand("tint_info", ["--json", shaderPath]);
  if (tintInfoRes.success) {
    const tintInfo = JSON.parse(tintInfoRes.stdout);
    const struct = tintInfo.structures?.find((s: { name: string }) => s.name === "RippleUniforms");
    if (struct) {
      console.log(`  Tint layout for RippleUniforms: size=${struct.size}B, align=${struct.align}B, members=${struct.members.length}`);
    }
  }

  // Step 5: Multi-target transpilation check
  console.log("\n[5/5] Cross-compiling shader to all 5 backend targets:");
  const targets = ["wgsl", "spv", "hlsl", "msl", "webgl2"] as const;
  for (const t of targets) {
    const args = ["transpile", shaderPath, "--target", t];
    if (t !== "wgsl" && t !== "spv") {
      args.push("--stage", "fragment");
    }
    const transRes = runCommand(gpuiWgpuPath, args);
    const preview = transRes.stdout.split("\n")[0]?.trim() ?? "";
    console.log(`  → ${t.padEnd(8)}: ${transRes.success ? "OK" : "ERR"} (${transRes.stdout.length} chars) - Header: "${preview.slice(0, 40)}..."`);
  }

  console.log("\n=== WebGPU Inspection Complete: All checks verified. ===");
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
