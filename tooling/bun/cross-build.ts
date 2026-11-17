#!/usr/bin/env bun
/**
 * Cross-compilation toolchain auditor and unified runner (`cargo-xwin`, `cargo-zigbuild`, LLVM, NDK, Wasm).
 * Usage:
 *   bun tooling/bun/cross-build.ts doctor
 *   bun tooling/bun/cross-build.ts matrix
 *   bun tooling/bun/cross-build.ts check --target <triple> [--package <pkg>]
 */
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "../..");

export interface CrossTargetSpec {
  target: string;
  driver: "cargo" | "cargo-xwin" | "cargo-zigbuild";
  cargoArgs: string[];
  notes: string;
}

export const CROSS_MATRIX: CrossTargetSpec[] = [
  {
    target: "x86_64-unknown-linux-gnu.2.28",
    driver: "cargo-zigbuild",
    cargoArgs: ["--target", "x86_64-unknown-linux-gnu.2.28"],
    notes: "Zig cc glibc-2.28 portability without legacy Ubuntu 20.04 runners",
  },
  {
    target: "aarch64-unknown-linux-gnu.2.28",
    driver: "cargo-zigbuild",
    cargoArgs: ["--target", "aarch64-unknown-linux-gnu.2.28"],
    notes: "ARM64 Linux desktop/Wayland binary",
  },
  {
    target: "x86_64-pc-windows-msvc",
    driver: "cargo-xwin",
    cargoArgs: ["--target", "x86_64-pc-windows-msvc"],
    notes: "Windows MSVC cross-build via clang-cl + lld-link + XWin SDK cache",
  },
  {
    target: "aarch64-pc-windows-msvc",
    driver: "cargo-xwin",
    cargoArgs: ["--target", "aarch64-pc-windows-msvc"],
    notes: "Windows on ARM64 via XWin SDK",
  },
  {
    target: "aarch64-linux-android",
    driver: "cargo",
    cargoArgs: ["--target", "aarch64-linux-android"],
    notes: "Android NDK / GPUI Mobile SurfaceView",
  },
  {
    target: "wasm32-unknown-unknown",
    driver: "cargo",
    cargoArgs: ["--target", "wasm32-unknown-unknown"],
    notes: "Browser WebGPU/WebGL2 + wasm-bindgen + wasm-opt",
  },
  {
    target: "wasm32-wasip2",
    driver: "cargo",
    cargoArgs: ["--target", "wasm32-wasip2"],
    notes: "Wasmtime 48 WASI Preview 2 component extensions",
  },
];

function probeTool(bin: string, versionArg = "--version"): string | null {
  const res = Bun.spawnSync([bin, versionArg], { stdout: "pipe", stderr: "pipe" });
  if (res.exitCode !== 0) return null;
  const line = new TextDecoder().decode(res.stdout).trim().split(/\r?\n/)[0] ?? "";
  return line || "present";
}

function opt(flag: string): string | undefined {
  const args = Bun.argv.slice(2);
  const idx = args.indexOf(flag);
  if (idx >= 0) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

if (import.meta.main) {
  const cmd = Bun.argv[2] ?? "doctor";
  if (cmd === "matrix") {
    console.log(JSON.stringify(CROSS_MATRIX, null, 2));
  } else if (cmd === "doctor") {
    const tools = {
      cargoXwin: probeTool("cargo-xwin"),
      cargoZigbuild: probeTool("cargo-zigbuild"),
      zig: probeTool("zig", "version"),
      clang: probeTool("clang"),
      lldLink: probeTool("lld-link"),
      llvmObjdump: probeTool("llvm-objdump"),
      gh: probeTool("gh"),
      wrangler: probeTool("wrangler"),
      wasmBindgen: probeTool("wasm-bindgen"),
      wasmOpt: probeTool("wasm-opt"),
      depotToolsLocal: existsSync("/srv/depot_tools") ? "/srv/depot_tools" : null,
    };
    const installedTargets = new TextDecoder()
      .decode(Bun.spawnSync(["rustup", "target", "list", "--installed"], { stdout: "pipe" }).stdout)
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean);
    console.log(JSON.stringify({ platform: process.platform, arch: process.arch, tools, installedTargets }, null, 2));
  } else if (cmd === "check") {
    const target = opt("--target") ?? "x86_64-unknown-linux-gnu.2.28";
    const pkg = opt("--package") ?? "aphrody_session";
    const spec = CROSS_MATRIX.find((s) => s.target === target || s.target.startsWith(`${target}.`));
    const driver = spec?.driver ?? "cargo";
    const sub = driver === "cargo-xwin" ? ["xwin", "check"] : driver === "cargo-zigbuild" ? ["zigbuild"] : ["check"];
    const baseTarget = target.split(".")[0];
    const checkTargets = new TextDecoder().decode(
      Bun.spawnSync(["rustup", "target", "list", "--installed"], { stdout: "pipe" }).stdout,
    );
    if (!checkTargets.split(/\r?\n/).includes(baseTarget)) {
      Bun.spawnSync(["rustup", "target", "add", baseTarget], { cwd: repoRoot, stdout: "inherit", stderr: "inherit" });
    }
    const env: Record<string, string | undefined> = { ...process.env };
    if (baseTarget === "wasm32-unknown-unknown") env.RUSTC_WRAPPER = "";
    const cmdArgs = ["cargo", ...sub, "-p", pkg, "--no-default-features", "--target", target];
    console.log(`$ ${cmdArgs.join(" ")}`);
    const run = Bun.spawnSync(cmdArgs, { cwd: repoRoot, env, stdout: "inherit", stderr: "inherit" });
    process.exit(run.exitCode);
  } else {
    console.error("Usage: bun tooling/bun/cross-build.ts [doctor|matrix|check --target <triple> --package <pkg>]");
    process.exit(1);
  }
}
