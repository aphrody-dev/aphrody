#!/usr/bin/env bun
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

interface ProductConfig {
  product: { name: string; slug: string };
  features: { wgpu_cli_bin: string; wgpu_cli_package: string };
}

const repoRoot = resolve(import.meta.dir, "../..");
const product = Bun.TOML.parse(readFileSync(join(repoRoot, "product.toml"), "utf8")) as ProductConfig;
const exeExt = process.platform === "win32" ? ".exe" : "";

function parseExtraPaths(): string[] {
  const paths: string[] = [];
  const envPrefix = product.product.slug.toUpperCase();
  for (const key of [`${envPrefix}_BIN_DIR`, `${envPrefix}_SDK_ROOT`, "VULKAN_SDK", "CUDA_PATH"]) {
    const val = process.env[key];
    if (!val) continue;
    paths.push(val, join(val, "bin"), join(val, "Bin"));
  }
  const home = homedir();
  paths.push(
    join(home, ".local", "bin"),
    join(home, ".local", "sdk", "directx", "dxc", "bin", "x64"),
    join(home, ".local", "sdk", "vulkan", "Bin"),
    join(repoRoot, "target", "debug"),
    join(repoRoot, "target", "release"),
  );
  if (process.platform === "win32") {
    const kitsRoot = join(process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)", "Windows Kits", "10", "bin");
    if (existsSync(kitsRoot)) {
      const versions = readdirSync(kitsRoot)
        .filter((d) => /^\d+\.\d+/.test(d))
        .sort()
        .reverse();
      for (const ver of versions) {
        paths.push(join(kitsRoot, ver, process.arch === "arm64" ? "arm64" : "x64"));
      }
    }
  }
  return paths;
}

const searchDirs = parseExtraPaths();

function resolveTool(name: string): string | null {
  const onPath = Bun.which(name);
  if (onPath) return onPath;
  for (const dir of searchDirs) {
    const candidate = join(dir, `${name}${exeExt}`);
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

function runTool(cmd: string, args: string[], cwd = repoRoot): void {
  const result = Bun.spawnSync([cmd, ...args], { cwd, stdout: "inherit", stderr: "inherit" });
  if (result.exitCode !== 0) {
    throw new Error(`Command failed (${result.exitCode}): ${cmd} ${args.join(" ")}`);
  }
}

function fileSize(path: string): number {
  if (!existsSync(path)) throw new Error(`Missing expected output artifact: ${path}`);
  return statSync(path).size;
}

const wgpuCli = resolveTool(product.features.wgpu_cli_bin);
if (!wgpuCli) {
  throw new Error(
    `Unable to locate ${product.features.wgpu_cli_bin}. Build it with: cargo build -p ${product.features.wgpu_cli_package}`,
  );
}

type Domain = "vulkan" | "directx" | "webgpu" | "nvidia" | "amd" | "intel";

function verifyVulkan(): void {
  const dir = join(repoRoot, "examples", "gpu", "vulkan");
  const out = join(dir, "output");
  mkdirSync(out, { recursive: true });
  const glslang = resolveTool("glslangValidator");
  const glslc = resolveTool("glslc");
  const spirvVal = resolveTool("spirv-val");
  const spirvOpt = resolveTool("spirv-opt");
  const spirvDis = resolveTool("spirv-dis");

  if (glslang) {
    runTool(glslang, ["-V", "wave_compute.comp", "-o", join(out, "wave_compute.spv")], dir);
    runTool(glslang, ["-V", "triangle.vert", "-o", join(out, "triangle.vert.spv")], dir);
    runTool(glslang, ["-V", "triangle.frag", "-o", join(out, "triangle.frag.spv")], dir);
  }
  if (glslc) {
    runTool(glslc, ["wave_compute.comp", "-o", join(out, "wave_compute_glslc.spv")], dir);
  }
  if (spirvVal && glslang) {
    runTool(spirvVal, ["--target-env", "spv1.6", join(out, "wave_compute.spv")], dir);
    runTool(spirvVal, ["--target-env", "spv1.6", join(out, "triangle.vert.spv")], dir);
    runTool(spirvVal, ["--target-env", "spv1.6", join(out, "triangle.frag.spv")], dir);
  }
  if (spirvOpt && spirvDis && existsSync(join(out, "wave_compute.spv"))) {
    const optOut = join(out, "wave_compute.opt.spv");
    runTool(spirvOpt, ["-O", join(out, "wave_compute.spv"), "-o", optOut], dir);
    runTool(spirvDis, [optOut, "-o", join(out, "wave_compute.spvasm")], dir);
  }
  const quadSpv = join(out, "quad_vulkan.spv");
  runTool(wgpuCli!, ["transpile", "--target", "spv", "-o", quadSpv, "quad_vulkan.wgsl"], dir);
  if (spirvVal) {
    runTool(spirvVal, ["--target-env", "spv1.3", quadSpv], dir);
  }
  console.log(`[vulkan] PASS (quad_vulkan.spv ${fileSize(quadSpv)} bytes)`);
}

function verifyDirectX(): void {
  const dir = join(repoRoot, "examples", "gpu", "directx");
  const d3dBackend = process.platform === "win32" ? "dx12" : "vulkan";
  const waveWgsl = join(dir, "wave_field.wgsl");
  const waveHlsl = join(dir, "wave_field.hlsl");
  runTool(wgpuCli!, ["transpile", waveWgsl, "--target", "hlsl", "-o", waveHlsl], dir);
  const dxc = resolveTool("dxc");
  if (dxc && process.platform === "win32") {
    runTool(dxc, ["-T", "cs_6_6", "-E", "main", "-O3", "-Qstrip_reflect", "-Fo", join(dir, "prefix_sum.dxil"), join(dir, "prefix_sum.hlsl")], dir);
    runTool(dxc, ["-T", "vs_6_6", "-E", "vs_main", "-O3", "-Fo", join(dir, "gpui_quad_vs.dxil"), join(dir, "gpui_quad.hlsl")], dir);
    runTool(dxc, ["-T", "ps_6_6", "-E", "ps_main", "-O3", "-Fo", join(dir, "gpui_quad_ps.dxil"), join(dir, "gpui_quad.hlsl")], dir);
    runTool(dxc, ["-T", "cs_6_6", "-E", "main", "-O3", "-Fo", join(dir, "wave_field.dxil"), waveHlsl], dir);
  }
  const fxc = resolveTool("fxc");
  if (fxc && process.platform === "win32") {
    runTool(fxc, ["/nologo", "/T", "vs_5_0", "/E", "vs_main", "/Fo", join(dir, "gpui_quad_vs.dxbc"), join(dir, "gpui_quad.hlsl")], dir);
    runTool(fxc, ["/nologo", "/T", "ps_5_0", "/E", "ps_main", "/Fo", join(dir, "gpui_quad_ps.dxbc"), join(dir, "gpui_quad.hlsl")], dir);
  }
  runTool(wgpuCli!, ["adapters", "--backends", d3dBackend], dir);
  console.log(`[directx] PASS (hlsl ${fileSize(waveHlsl)} bytes, backend=${d3dBackend})`);
}

function verifyWebGpu(): void {
  const dir = join(repoRoot, "examples", "gpu", "webgpu");
  const wgsl = join(dir, "nsh_ripple_shader.wgsl");
  runTool("bun", ["check"], dir);
  runTool("bun", ["run", "webgpu_inspect.ts"], dir);
  for (const target of ["wgsl", "spv", "hlsl", "msl", "webgl2"] as const) {
    const out = join(dir, `nsh_ripple.${target}`);
    const args = target === "wgsl" || target === "spv"
      ? ["transpile", wgsl, "--target", target, "-o", out]
      : ["transpile", wgsl, "--target", target, "--stage", "fragment", "-o", out];
    runTool(wgpuCli!, args, dir);
    fileSize(out);
  }
  const renderPng = join(dir, "nsh_ripple.png");
  runTool(wgpuCli!, ["render", wgsl, "--output", renderPng, "--width", "512", "--height", "512"], dir);
  runTool(wgpuCli!, ["compute", join(dir, "nsh_compute.wgsl"), "--workgroups", "1,1,1", "--output-floats", "64", "--json"], dir);
  console.log(`[webgpu] PASS (nsh_ripple.png ${fileSize(renderPng)} bytes)`);
}

function verifyVendor(domain: "nvidia" | "amd" | "intel", shaderName: string): void {
  const dir = join(repoRoot, "examples", "gpu", domain);
  const shader = join(dir, shaderName);
  runTool(wgpuCli!, ["validate", shader], dir);
  runTool(wgpuCli!, ["transpile", shader, "--target", "hlsl"], dir);
  runTool(wgpuCli!, ["transpile", shader, "--target", "spv"], dir);
  if (domain === "intel") {
    const ispc = resolveTool("ispc");
    if (ispc) {
      runTool(ispc, ["-O3", "--target=avx2-i32x8", join(dir, "vector_fma.ispc"), "-o", join(dir, "vector_fma.obj"), "-h", join(dir, "vector_fma.h")], dir);
    }
  } else if (domain === "amd") {
    const rga = resolveTool("rga");
    if (rga) runTool(rga, ["--version"], dir);
  } else if (domain === "nvidia") {
    const nvcc = resolveTool("nvcc");
    if (nvcc) runTool(nvcc, ["--version"], dir);
  }
  console.log(`[${domain}] PASS`);
}

const arg = (process.argv[2] ?? "all").toLowerCase();
const handlers: Record<Domain, () => void> = {
  vulkan: verifyVulkan,
  directx: verifyDirectX,
  webgpu: verifyWebGpu,
  nvidia: () => verifyVendor("nvidia", "sm89_matmul_warp.wgsl"),
  amd: () => verifyVendor("amd", "wave_reduce_rdna.wgsl"),
  intel: () => verifyVendor("intel", "xe_tiled_conv.wgsl"),
};

console.log(`${product.product.name} GPU verification suite (platform=${process.platform}, arch=${process.arch})`);
if (arg === "all") {
  for (const run of Object.values(handlers)) run();
} else if (arg in handlers) {
  handlers[arg as Domain]();
} else {
  throw new Error(`Unknown GPU domain '${arg}'. Expected one of: all, ${Object.keys(handlers).join(", ")}`);
}
