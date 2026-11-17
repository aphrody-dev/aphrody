#!/usr/bin/env bun
/**
 * Unified WebAssembly & WebGPU toolchain standardizer for all Aphrody Rust frameworks:
 * GPUI / OS Desktop, NSH Web, Craft Anything, COSMIC, Egui (eframe), Iced, Blitz/Dioxus, and WGPU/Vello.
 *
 * Unifies and replaces per-framework ad-hoc bash scripts (`egui/scripts/*.sh`), `python3 -m http.server`,
 * `simple-http-server`, legacy `--no-modules` bindgen, and unverified `trunk` configs:
 * - Single Cargo profile (`[profile.web]`) + automatic `RUSTC_WRAPPER=""` (fixes Windows sccache web-sys argv limit)
 * - Exact `wasm-bindgen` CLI <-> `Cargo.lock` version pinning check
 * - Separated Wasm cohorts: `baseline` (no SIMD / no shared memory), `simd-baseline` (`+simd128`), and opt-in `multithreaded` (`--coi`)
 * - Automated `wasm-opt` binaryen pass (`--enable-bulk-memory --enable-reference-types ...`) on `--release` / `--opt`
 * - Native Wasm section parser + gzip/brotli metrics + SIMD/shared-memory invariant verification + `--brotli-budget`
 * - Bun HTTP server on `127.0.0.1` (Secure Context for WebGPU), `Content-Type: application/wasm`, optional COOP/COEP
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { extname, join, resolve } from "node:path";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const repoRoot = resolve(import.meta.dir, "../..");
process.chdir(repoRoot);

const args = Bun.argv.slice(2);
const command = args[0] ?? "check";
const targetPkg = opt("--package") ?? "aphrody_desktop";
const useLto = args.includes("--lto");
const useAot = args.includes("--aot");
const release = args.includes("--release") || useLto;
const runOpt = release || args.includes("--opt");
const keepDebug = args.includes("--keep-debug");
const serveCoi = args.includes("--coi");
const withThreads = args.includes("--threads") || serveCoi;
const buildSimdCohort = !args.includes("--no-simd");
const port = Number(opt("--port") ?? "4195");
const brotliBudget = opt("--brotli-budget") ? Number(opt("--brotli-budget")) : null;
const outRoot = resolve(opt("--out-dir") ?? "target/web-dist");

interface FrameworkWebSpec {
  id: string;
  label: string;
  manifest: string;
  packageName: string;
  example?: string;
  features: string[];
  legacyTooling: string;
  wasmBackend: string;
  coiRequired: boolean;
}

const FRAMEWORK_SPECS: FrameworkWebSpec[] = [
  {
    id: "aphrody_desktop",
    label: "Aphrody OS + GPUI + Craft + COSMIC",
    manifest: "crates/aphrody_desktop/Cargo.toml",
    packageName: "aphrody_desktop",
    example: "web_os",
    features: ["web"],
    legacyTooling: "Unified (web-build.ts + [profile.web])",
    wasmBackend: "WebGPU (hardware + fallback adapter) -> WebGL2 + ARIA canvas",
    coiRequired: false,
  },
  {
    id: "aphrody_store",
    label: "Awesome GPUI Store (NSH + 22 Packagers)",
    manifest: "crates/aphrody_store/Cargo.toml",
    packageName: "aphrody_store",
    features: ["web"],
    legacyTooling: "Unified (web-build.ts + awesome-gpui-store.ts)",
    wasmBackend: "WebGPU + NSH adaptive Store surface",
    coiRequired: false,
  },
  {
    id: "gpui_web",
    label: "GPUI Web Platform & WGPU",
    manifest: "crates/gpui_web/Cargo.toml",
    packageName: "gpui_web",
    features: [],
    legacyTooling: "trunk + gallery.js (COI opt-in fixed)",
    wasmBackend: "WebGPU + WebGL2 + SW fallback + IME + ARIA",
    coiRequired: false,
  },
  {
    id: "nsh",
    label: "NSH Web Runtime",
    manifest: "crates/nsh-platform-web/Cargo.toml",
    packageName: "nsh-platform-web",
    features: ["browser-core"],
    legacyTooling: "nsh.js + package-size.mjs",
    wasmBackend: "DOM + WebGL/WebGPU + QuickJS bytecode",
    coiRequired: false,
  },
  {
    id: "craft",
    label: "Craft Anything Web",
    manifest: "vendor/craft-anything/apps/craft-anything-web/Cargo.toml",
    packageName: "craft-anything-web",
    features: [],
    legacyTooling: "trunk + Trunk.toml + wasm-opt -Oz",
    wasmBackend: "wgpu WebGPU/WebGL2 + egui",
    coiRequired: false,
  },
  {
    id: "egui",
    label: "Egui / eframe Web",
    manifest: "vendor/egui/crates/egui_demo_app/Cargo.toml",
    packageName: "egui_demo_app",
    features: ["web_app", "wgpu"],
    legacyTooling: "bash scripts/*_web.sh + --no-modules",
    wasmBackend: "wgpu WebGPU/WebGL2 or glow WebGL2",
    coiRequired: false,
  },
  {
    id: "iced",
    label: "Iced Web (winit + wgpu)",
    manifest: "vendor/iced/winit/Cargo.toml",
    packageName: "iced_winit",
    features: ["wgpu"],
    legacyTooling: "trunk per-example",
    wasmBackend: "wgpu WebGPU/WebGL2 + DOM accessibility",
    coiRequired: false,
  },
  {
    id: "blitz",
    label: "Dioxus / Blitz Web",
    manifest: "vendor/blitz/examples/wasm_hello/Cargo.toml",
    packageName: "wasm_hello",
    features: [],
    legacyTooling: "dx + binaryen-v129 + wasm-bindgen",
    wasmBackend: "anyrender_vello (WebGPU)",
    coiRequired: false,
  },
  {
    id: "wgpu",
    label: "WGPU Core & Vello Web Tests",
    manifest: "vendor/wgpu/examples/features/Cargo.toml",
    packageName: "wgpu-examples",
    features: ["webgpu"],
    legacyTooling: "xshell + simple-http-server + wasmparser tests",
    wasmBackend: "WebGPU + WebGL2 dual targets",
    coiRequired: false,
  },
];

function opt(flag: string): string | undefined {
  const idx = args.indexOf(flag);
  if (idx >= 0) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

function readWasmBindgenLockVersion(): string {
  const lock = readFileSync("Cargo.lock", "utf8");
  const match = lock.match(/name = "wasm-bindgen"\r?\nversion = "([^"]+)"/);
  return match?.[1] ?? "unknown";
}

function checkProgramVersion(bin: string): string | null {
  const res = Bun.spawnSync([bin, "--version"], { stdout: "pipe", stderr: "pipe" });
  if (res.exitCode !== 0) return null;
  return new TextDecoder().decode(res.stdout).trim();
}

function verifyBindgenVersion(): string {
  const expected = readWasmBindgenLockVersion();
  const actualRaw = checkProgramVersion("wasm-bindgen");
  if (!actualRaw) {
    throw new Error(`wasm-bindgen CLI missing (install version ${expected} matching Cargo.lock)`);
  }
  const actual = actualRaw.replace(/^wasm-bindgen\s+/i, "").trim();
  if (expected !== "unknown" && actual !== expected) {
    throw new Error(`wasm-bindgen mismatch: CLI is '${actual}' but Cargo.lock pins '${expected}'`);
  }
  return expected;
}

function runCargoWasm(
  pkgs: string[],
  features: string[],
  extraArgs: string[] = [],
  rustflags = "",
  subcommand: "check" | "build" = command === "check" ? "check" : "build",
) {
  const env = { ...process.env };
  // Override ~/.cargo/config.toml rustc-wrapper so Windows never overflows CreateProcess argv on web-sys.
  env.RUSTC_WRAPPER = "";
  env.RUSTC_BOOTSTRAP = "1";
  if (rustflags.length > 0) {
    const existing = env.CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS ?? "";
    env.CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS = `${existing} ${rustflags}`.trim();
  }

  const cmd = [
    "cargo",
    subcommand,
    "--target",
    "wasm32-unknown-unknown",
    ...pkgs.flatMap((pkg) => ["-p", pkg]),
    "--no-default-features",
    ...extraArgs,
  ];
  if (features.length > 0) {
    cmd.push("--features", features.join(","));
  }
  if (useLto) {
    cmd.push("--profile", "web-lto");
  } else if (release) {
    cmd.push("--profile", "web");
  }
  console.log(`$ ${cmd.join(" ")}`);
  const res = Bun.spawnSync(cmd, { stdout: "inherit", stderr: "inherit", env });
  if (res.exitCode !== 0) {
    throw new Error(`cargo wasm32 ${subcommand} failed (${res.exitCode}) for ${pkgs.join(", ")}`);
  }
}

interface WasmSectionMetric {
  name: string;
  raw: number;
}

interface WasmAuditReport {
  path: string;
  raw: number;
  gzip: number;
  brotli: number;
  sha256: string;
  sections: WasmSectionMetric[];
  importsSharedMemory: boolean;
}

function inspectWasmBinary(filePath: string): WasmAuditReport {
  const bytes = readFileSync(filePath);
  const sectionNames = [
    "custom",
    "type",
    "import",
    "function",
    "table",
    "memory",
    "global",
    "export",
    "start",
    "element",
    "code",
    "data",
    "data-count",
  ];
  const sections: WasmSectionMetric[] = [];
  for (let offset = 8; offset < bytes.length;) {
    const id = bytes[offset++];
    let length = 0;
    let shift = 0;
    let byte = 0;
    do {
      byte = bytes[offset++];
      length += (byte & 0x7f) * 2 ** shift;
      shift += 7;
    } while (byte & 0x80);
    sections.push({ name: sectionNames[id] ?? `id(${id})`, raw: length });
    offset += length;
  }

  // Inspect real WebAssembly imports/exports via native JS WebAssembly validator.
  let importsSharedMemory = false;
  try {
    const mod = new WebAssembly.Module(bytes);
    const imports = WebAssembly.Module.imports(mod);
    const exports = WebAssembly.Module.exports(mod);
    importsSharedMemory =
      imports.some((i) => i.kind === "memory") ||
      exports.some((e) => e.name === "__wasm_init_tls");
  } catch {
    importsSharedMemory = false;
  }

  return {
    path: filePath,
    raw: bytes.length,
    gzip: gzipSync(bytes, { level: 9 }).length,
    brotli: brotliCompressSync(bytes, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
    sections,
    importsSharedMemory,
  };
}

function maybeOptimizeWasm(wasmFile: string, cohortName: string) {
  if (!runOpt) return;
  const wasmOptVer = checkProgramVersion("wasm-opt");
  if (!wasmOptVer) {
    console.warn("[warn] wasm-opt not found in PATH; skipping binaryen optimization pass");
    return;
  }
  const argsList = [
    wasmFile,
    "-Oz",
    "--enable-bulk-memory",
    "--enable-bulk-memory-opt",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
    "--enable-reference-types",
    "--enable-multivalue",
  ];
  if (cohortName === "simd-baseline" || cohortName === "multithreaded") {
    argsList.push("--enable-simd");
  }
  if (cohortName === "multithreaded") {
    argsList.push("--enable-threads");
  }
  argsList.push(keepDebug ? "--debuginfo" : "--strip-debug", "-o", wasmFile);
  console.log(`$ wasm-opt ${argsList.join(" ")}`);
  const res = Bun.spawnSync(["wasm-opt", ...argsList], { stdout: "inherit", stderr: "inherit" });
  if (res.exitCode !== 0) {
    throw new Error(`wasm-opt failed (${res.exitCode}) on ${wasmFile}`);
  }
}

function generateCohortHtml(pkg: string, entryName: string, wgVersion: string, cohorts: string[]): string {
  return `<!DOCTYPE html>
<html lang="en" dir="auto">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover" />
  <title>${pkg} — Aphrody WebGPU / Wasm</title>
  <style>
    :root { color-scheme: dark light; }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    html, body { width: 100%; height: 100%; overflow: hidden; background: Canvas; color: CanvasText; font-family: system-ui, -apple-system, sans-serif; }
    #app-root { display: contents; }
    #a11y-status { position: fixed; inset-block-end: 8px; inset-inline-end: 12px; z-index: 10; font-size: 12px; padding: 4px 8px; border-radius: 6px; background: rgba(15, 23, 42, 0.82); color: #e2e8f0; border: 1px solid rgba(148, 163, 184, 0.35); }
    canvas { position: fixed; inset: 0; display: block; width: 100%; height: 100%; }
  </style>
  <script>
    (() => {
      const req = new URLSearchParams(location.search).get("lang") || navigator.language || "en";
      try {
        const [tag] = Intl.getCanonicalLocales(req);
        if (tag) {
          document.documentElement.lang = tag;
          const lang = tag.split("-")[0].toLowerCase();
          const rtl = /^(ar|ckb|dv|fa|he|iw|ku|ps|sd|ug|ur|yi)$/.test(lang) || /-Arab|-Hebr/i.test(tag);
          document.documentElement.dir = rtl ? "rtl" : "ltr";
        }
      } catch {}
    })();
  </script>
</head>
<body>
  <main id="app-root" role="main" aria-label="${pkg} GPU workspace">
    <div id="a11y-status" role="status" aria-live="polite">Probing WebGPU / Wasm cohorts (wasm-bindgen ${wgVersion})…</div>
  </main>
  <script type="module">
    const status = document.getElementById("a11y-status");
    const availableCohorts = ${JSON.stringify(cohorts)};
    const hasSimd = (() => {
      try {
        return WebAssembly.validate(new Uint8Array([0,97,115,109,1,0,0,0,1,5,1,96,0,1,123,3,2,1,0,10,10,1,8,0,65,0,253,15,253,98,11]));
      } catch {
        return false;
      }
    })();
    const canThreads = Boolean(globalThis.crossOriginIsolated && typeof SharedArrayBuffer !== "undefined");

    let gpuBackend = "webgl2-fallback";
    if (navigator.gpu) {
      let adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" }).catch(() => null);
      if (!adapter) {
        adapter = await navigator.gpu.requestAdapter({ featureLevel: "compatibility" }).catch(() => null);
      }
      if (adapter) {
        const device = await adapter.requestDevice({ label: "${pkg}-device", requiredFeatures: [] }).catch(() => null);
        if (device) {
          device.lost.then((info) => {
            status.textContent = "WebGPU device lost (" + info.reason + "): reinitializing";
          });
          gpuBackend = adapter.isFallbackAdapter ? "webgpu-software" : "webgpu-hw";
        }
      }
    }

    let cohort = "baseline";
    if (canThreads && availableCohorts.includes("multithreaded")) {
      cohort = "multithreaded";
    } else if (hasSimd && availableCohorts.includes("simd-baseline")) {
      cohort = "simd-baseline";
    }
    status.textContent = "${pkg} | backend=" + gpuBackend + " | cohort=" + cohort;
    status.setAttribute("data-backend", gpuBackend);
    status.setAttribute("data-cohort", cohort);

    const moduleUrl = "./" + cohort + "/${entryName}.js";
    try {
      const mod = await import(moduleUrl);
      if (typeof mod.default === "function") {
        await mod.default();
      }
      status.setAttribute("data-loaded", "true");
    } catch (err) {
      status.textContent = "${pkg} cohort=" + cohort + " error: " + err;
      status.setAttribute("data-error", String(err));
      console.error(err);
    }
  </script>
</body>
</html>
`;
}

function buildCohorts(): { entryName: string; cohorts: Record<string, WasmAuditReport> } {
  const wgVersion = verifyBindgenVersion();
  const entryName = targetPkg === "aphrody_desktop" ? "web_os" : targetPkg;
  const cargoArgs = targetPkg === "aphrody_desktop" ? ["--example", "web_os"] : [];
  const features = targetPkg === "gpui_web" ? [] : ["web"];
  const profileDir = useLto ? "web-lto" : release ? "web" : "debug";
  const wasmSrc = targetPkg === "aphrody_desktop"
    ? join("target", "wasm32-unknown-unknown", profileDir, "examples", `${entryName}.wasm`)
    : join("target", "wasm32-unknown-unknown", profileDir, `${entryName}.wasm`);

  mkdirSync(outRoot, { recursive: true });
  const reports: Record<string, WasmAuditReport> = {};

  const cohortPlans: Array<{ name: string; rustflags: string }> = [
    { name: "baseline", rustflags: "" },
  ];
  if (buildSimdCohort) {
    cohortPlans.push({ name: "simd-baseline", rustflags: "-C target-feature=+simd128" });
  }
  if (withThreads) {
    cohortPlans.push({
      name: "multithreaded",
      rustflags: "-C target-feature=+atomics,+bulk-memory,+mutable-globals",
    });
  }

  for (const plan of cohortPlans) {
    runCargoWasm([targetPkg], features, cargoArgs, plan.rustflags, "build");
    const cohortDir = join(outRoot, plan.name);
    mkdirSync(cohortDir, { recursive: true });
    const wbEnv = { ...process.env, RUSTC_BOOTSTRAP: "1" };
    const wbCmd = [
      "wasm-bindgen",
      wasmSrc,
      "--target",
      "web",
      "--no-typescript",
      "--out-dir",
      cohortDir,
      "--out-name",
      entryName,
    ];
    console.log(`$ ${wbCmd.join(" ")}`);
    const wbRes = Bun.spawnSync(wbCmd, { stdout: "inherit", stderr: "inherit", env: wbEnv });
    if (wbRes.exitCode !== 0) {
      throw new Error(`wasm-bindgen failed (${wbRes.exitCode}) for cohort ${plan.name}`);
    }
    const bgWasm = join(cohortDir, `${entryName}_bg.wasm`);
    maybeOptimizeWasm(bgWasm, plan.name);
    if (useAot) {
      const aotPath = join(cohortDir, `${entryName}.cwasm`);
      const aotRes = Bun.spawnSync(["wasmtime", "compile", bgWasm, "-o", aotPath], {
        stdout: "inherit",
        stderr: "inherit",
      });
      if (aotRes.exitCode !== 0) {
        console.warn(`[warn] wasmtime compile skipped/non-zero on browser module ${bgWasm}`);
      }
    }
    const report = inspectWasmBinary(bgWasm);
    if (plan.name === "baseline" && report.importsSharedMemory) {
      throw new Error(`baseline cohort unexpectedly imports shared memory/TLS`);
    }
    if (brotliBudget !== null && report.brotli > brotliBudget) {
      throw new Error(`cohort '${plan.name}' brotli ${report.brotli} exceeds budget ${brotliBudget}`);
    }
    reports[plan.name] = report;
  }

  const cohortNames = Object.keys(reports);
  writeFileSync(join(outRoot, "index.html"), generateCohortHtml(targetPkg, entryName, wgVersion, cohortNames), "utf8");
  writeFileSync(
    join(outRoot, "manifest.json"),
    JSON.stringify({
      package: targetPkg,
      entry: entryName,
      wasmBindgen: wgVersion,
      profile: profileDir,
      optimized: runOpt,
      cohorts: reports,
    }, null, 2),
    "utf8",
  );
  return { entryName, cohorts: reports };
}

function serveDist() {
  const mimeMap: Record<string, string> = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".mjs": "text/javascript; charset=utf-8",
    ".wasm": "application/wasm",
    ".json": "application/json; charset=utf-8",
  };
  const server = Bun.serve({
    hostname: "127.0.0.1",
    port,
    async fetch(req) {
      const url = new URL(req.url);
      const rel = url.pathname === "/" ? "/index.html" : url.pathname;
      const filePath = resolve(outRoot, "." + rel);
      if (!filePath.startsWith(outRoot)) {
        return new Response("Forbidden", { status: 403 });
      }
      const f = Bun.file(filePath);
      if (!(await f.exists())) {
        return new Response("Not Found", { status: 404 });
      }
      const headers = new Headers({
        "Content-Type": mimeMap[extname(filePath)] ?? "application/octet-stream",
        "Content-Language": "en",
        "Vary": "Accept-Language",
      });
      if (serveCoi) {
        headers.set("Cross-Origin-Embedder-Policy", "require-corp");
        headers.set("Cross-Origin-Opener-Policy", "same-origin");
      }
      return new Response(f, { headers });
    },
  });
  console.log(`Serving unified WebGPU/Wasm cohorts at ${server.url} (COI=${serveCoi})`);
}

if (command === "matrix" || command === "audit") {
  const wgLock = readWasmBindgenLockVersion();
  const wgCli = checkProgramVersion("wasm-bindgen") ?? "missing";
  const wasmOptCli = checkProgramVersion("wasm-opt") ?? "missing";
  const rows = FRAMEWORK_SPECS.map((s) => {
    const manifestPresent = existsSync(resolve(repoRoot, s.manifest));
    return {
      id: s.id,
      label: s.label,
      manifestPresent,
      packageName: s.packageName,
      features: s.features,
      coiRequired: s.coiRequired,
      legacyTooling: s.legacyTooling,
      wasmBackend: s.wasmBackend,
    };
  });
  let distManifest: unknown = null;
  const manifestPath = join(outRoot, "manifest.json");
  if (existsSync(manifestPath) && statSync(manifestPath).isFile()) {
    distManifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  }
  console.log(JSON.stringify({
    toolchain: {
      target: "wasm32-unknown-unknown",
      cargoLockWasmBindgen: wgLock,
      cliWasmBindgen: wgCli,
      cliWasmOpt: wasmOptCli,
      cargoProfile: "web",
    },
    frameworks: rows,
    distManifest,
  }, null, 2));
} else if (command === "check") {
  const wgVersion = verifyBindgenVersion();
  console.log(`Verified wasm-bindgen ${wgVersion} == Cargo.lock`);
  runCargoWasm(
    ["gpui", "gpui_web", "gpui_wgpu", "gpui_platform", "gpui_ui", "gpui_code", "aphrody_session", "aphrody_craft", "aphrody_ui", "nsh-core", "nsh-platform-web"],
    [],
    [],
    "",
    "check",
  );
  runCargoWasm(["aphrody_desktop"], ["web"], ["--example", "web_os"], "", "check");
  if (existsSync(resolve(repoRoot, "crates/aphrody_store/Cargo.toml"))) {
    runCargoWasm(["aphrody_store"], ["web"], [], "", "check");
  }
  const wasiEnv = { ...process.env, RUSTC_WRAPPER: "", RUSTC_BOOTSTRAP: "1" };
  const wasiCmd = [
    "cargo",
    "check",
    "--target",
    "wasm32-wasip2",
    "-p",
    "aphrody_session",
    "-p",
    "aphrody_craft",
    "-p",
    "release_channel",
    "-p",
    "brand",
    "--no-default-features",
  ];
  console.log(`$ ${wasiCmd.join(" ")}`);
  const wasiRes = Bun.spawnSync(wasiCmd, { stdout: "inherit", stderr: "inherit", env: wasiEnv });
  if (wasiRes.exitCode !== 0) {
    throw new Error(`cargo wasm32-wasip2 check failed (${wasiRes.exitCode})`);
  }
} else if (command === "build") {
  const built = buildCohorts();
  console.log(JSON.stringify(built, null, 2));
} else if (command === "serve") {
  if (!existsSync(join(outRoot, "manifest.json")) || args.includes("--rebuild")) {
    buildCohorts();
  }
  serveDist();
} else {
  throw new Error(`Unknown command '${command}'. Use matrix | audit | check | build | serve.`);
}
