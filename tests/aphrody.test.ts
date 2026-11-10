import { describe, expect, it } from "bun:test";
import { existsSync, readFileSync, statSync } from "fs";
import { join } from "path";
import {
  GOOGLE_APPS,
  APHRODY_SYSTEM_APPS,
  ALL_APPS,
  getAppIconSvg,
} from "../src/data/apps";
import { getSuggestions, POPULAR_SUGGESTIONS } from "../src/data/suggestions";
import { server } from "../server";
import {
  FLUENT_LIGHT_TOKENS,
  FLUENT_DARK_TOKENS,
  FLUENT_GLOBAL_TOKENS,
  getFluentCssVariables,
} from "../src/design/fluent-tokens";
import {
  MATERIAL_LIGHT_TOKENS,
  MATERIAL_DARK_TOKENS,
  getMaterialBreakpoint,
  renderMaterialSymbol,
  MATERIAL_SYMBOLS,
} from "../src/design/material-tokens";
import {
  buildStorytoldWasmBinary,
  getStorytoldWasm,
} from "../src/suite/wasm-kernel";
import { STORYTOLD_APPS } from "../src/suite/storytold-suite";
import { StoreDatabase } from "../src/store/database";
import { UniversalStoreManager } from "../src/store/universal-manager";
import { AwesomeListConverter } from "../src/store/drivers/awesome-converter";
import { generateNixDeclarativeFlake } from "../src/store/drivers/nix";
import {
  benchmarkZeroCopyVsCopy,
  benchmarkWasmVsJs,
  benchmarkSqliteThroughput,
  pickOptimalStack,
  runBenchmarkSuite,
  CANDIDATE_STACKS,
} from "../src/benchmark/stack-picker";

const ROOT_DIR = join(import.meta.dir, "..");

describe("Aphrody Zero-External-Dependency Architecture", () => {
  it("should have ZERO external dependencies in package.json", () => {
    const pkgPath = join(ROOT_DIR, "package.json");
    const pkg = JSON.parse(readFileSync(pkgPath, "utf-8"));

    const deps = Object.keys(pkg.dependencies || {});
    const devDeps = Object.keys(pkg.devDependencies || {});

    expect(deps.length).toBe(0);
    expect(devDeps.length).toBe(0);
    expect(pkg.dependencies).toEqual({});
    expect(pkg.devDependencies).toEqual({});
  });

  it("should NOT have node_modules directory", () => {
    const nodeModulesPath = join(ROOT_DIR, "node_modules");
    expect(existsSync(nodeModulesPath)).toBe(false);
  });

  it("should NOT have bun.lock file", () => {
    const lockPath = join(ROOT_DIR, "bun.lock");
    expect(existsSync(lockPath)).toBe(false);
  });
});

describe("Aphrody Bundle Build Output", () => {
  it("should generate dist/ production assets using native Bun.build()", () => {
    const distDir = join(ROOT_DIR, "dist");
    const appJsPath = join(distDir, "app.js");
    const cssPath = join(distDir, "style.css");
    const htmlPath = join(distDir, "index.html");
    const appJsGzPath = join(distDir, "app.js.gz");

    expect(existsSync(appJsPath)).toBe(true);
    expect(existsSync(cssPath)).toBe(true);
    expect(existsSync(htmlPath)).toBe(true);
    expect(existsSync(appJsGzPath)).toBe(true);

    const appJsGzStat = statSync(appJsGzPath);
    expect(appJsGzStat.size).toBeGreaterThan(0);
    expect(appJsGzStat.size).toBeLessThan(60 * 1024);
  });

  it("should contain valid HTML referencing pure JS and CSS in index.html", () => {
    const html = readFileSync(join(ROOT_DIR, "dist", "index.html"), "utf-8");
    expect(html).toContain('<div id="root"></div>');
    expect(html).toContain('src="/app.js"');
    expect(html).toContain('href="/style.css"');
    expect(html).toContain(
      "<title>Google - Recherche Web, Applications & Portail Aphrody</title>",
    );
  });
});

describe("Microsoft Fluent UI 2 Design System Parity", () => {
  it("should define complete Fluent 2 neutral backgrounds 1..6 and Mica/Acrylic tokens", () => {
    expect(FLUENT_LIGHT_TOKENS.colorNeutralBackground1).toBe("#ffffff");
    expect(FLUENT_LIGHT_TOKENS.colorNeutralBackground2).toBe("#fafafa");
    expect(FLUENT_LIGHT_TOKENS.colorNeutralBackground3).toBe("#f5f5f5");
    expect(FLUENT_LIGHT_TOKENS.colorNeutralBackground4).toBe("#f0f0f0");
    expect(FLUENT_LIGHT_TOKENS.colorNeutralBackground5).toBe("#ebebeb");
    expect(FLUENT_LIGHT_TOKENS.colorNeutralBackground6).toBe("#e6e6e6");

    expect(FLUENT_DARK_TOKENS.colorNeutralBackground1).toBe("#292929");
    expect(FLUENT_DARK_TOKENS.colorNeutralBackground2).toBe("#1f1f1f");
    expect(FLUENT_DARK_TOKENS.colorNeutralBackground3).toBe("#141414");

    expect(FLUENT_LIGHT_TOKENS.colorBackgroundMica).toContain(
      "rgba(255, 255, 255",
    );
    expect(FLUENT_DARK_TOKENS.colorBackgroundMica).toContain("rgba(32, 32, 32");
    expect(FLUENT_LIGHT_TOKENS.colorBrandBackground).toBe("#0f6cbd");
    expect(FLUENT_LIGHT_TOKENS.colorStrokeFocus1).toBe("#ffffff");
    expect(FLUENT_LIGHT_TOKENS.colorStrokeFocus2).toBe("#000000");
  });

  it("should provide Fluent global stroke, radius, and typography ramp tokens", () => {
    expect(FLUENT_GLOBAL_TOKENS.strokeWidthThin).toBe("1px");
    expect(FLUENT_GLOBAL_TOKENS.borderRadiusMedium).toBe("4px");
    expect(FLUENT_GLOBAL_TOKENS.borderRadiusLarge).toBe("8px");
    expect(FLUENT_GLOBAL_TOKENS.fontFamilyBase).toContain("Segoe UI Variable");
    expect(FLUENT_GLOBAL_TOKENS.fontSizeTitle1).toBe("32px");
    expect(FLUENT_GLOBAL_TOKENS.fontSizeDisplay).toBe("68px");
  });

  it("should generate valid CSS variables string for Fluent tokens", () => {
    const cssLight = getFluentCssVariables("light");
    expect(cssLight).toContain("--fluent-colorBrandBackground: #0f6cbd;");
    expect(cssLight).toContain("--fluent-strokeWidthThin: 1px;");

    const cssDark = getFluentCssVariables("dark");
    expect(cssDark).toContain("--fluent-colorNeutralBackground1: #292929;");
  });
});

describe("Google Material Design 3 Parity & Symbols", () => {
  it("should define HCT tonal surface containers lowest..highest in light and dark", () => {
    expect(MATERIAL_LIGHT_TOKENS.surfaceContainerLowest).toBe("#ffffff");
    expect(MATERIAL_LIGHT_TOKENS.surfaceContainerLow).toBe("#f7f2fa");
    expect(MATERIAL_LIGHT_TOKENS.surfaceContainer).toBe("#f3edf7");
    expect(MATERIAL_LIGHT_TOKENS.surfaceContainerHigh).toBe("#ece6f0");
    expect(MATERIAL_LIGHT_TOKENS.surfaceContainerHighest).toBe("#e6e0e9");

    expect(MATERIAL_DARK_TOKENS.surfaceContainerLowest).toBe("#0f0d13");
    expect(MATERIAL_DARK_TOKENS.surfaceContainerHighest).toBe("#36343b");
    expect(MATERIAL_LIGHT_TOKENS.primary).toBe("#6750a4");
  });

  it("should accurately resolve Material 3 adaptive layout breakpoints", () => {
    expect(getMaterialBreakpoint(400)).toBe("compact"); // < 600
    expect(getMaterialBreakpoint(599)).toBe("compact");
    expect(getMaterialBreakpoint(600)).toBe("medium"); // 600..839
    expect(getMaterialBreakpoint(750)).toBe("medium");
    expect(getMaterialBreakpoint(839)).toBe("medium");
    expect(getMaterialBreakpoint(840)).toBe("expanded"); // >= 840
    expect(getMaterialBreakpoint(1440)).toBe("expanded");
  });

  it("should render crisp vector SVG Material Symbols from registry", () => {
    const icons = [
      "search",
      "menu",
      "close",
      "settings",
      "storefront",
      "rocket_launch",
      "tune",
      "speed",
      "code",
    ];
    icons.forEach((icon) => {
      expect(MATERIAL_SYMBOLS[icon]).toBeDefined();
      const svg = renderMaterialSymbol(icon, 24);
      expect(svg).toContain('<svg class="md-symbol"');
      expect(svg).toContain('viewBox="0 0 24 24"');
      expect(svg).toContain('width="24"');
    });
  });
});

describe("Apple macOS Traffic-Light Windowing System Standards", () => {
  it("should follow pixel-accurate macOS Sequoia traffic-light specs", () => {
    const css = readFileSync(
      join(ROOT_DIR, "src", "style.css"),
      "utf-8",
    ).toLowerCase();
    // Close: #ff5f56 (1px solid #e0443e)
    expect(css).toContain("#ff5f56");
    expect(css).toContain("#e0443e");
    // Minimize: #ffbd2e (1px solid #dea123)
    expect(css).toContain("#ffbd2e");
    expect(css).toContain("#dea123");
    // Maximize: #27c93f (1px solid #1aab29)
    expect(css).toContain("#27c93f");
    expect(css).toContain("#1aab29");
    // Inactive unfocused states
    expect(css).toContain("#dcdcdc");
    expect(css).toContain("#464646");
  });
});

describe("Storytold Pure Binary WASM SIMD Kernel", () => {
  it("should compile a valid binary WebAssembly module from byte opcodes", () => {
    const bytes = buildStorytoldWasmBinary();
    expect(bytes instanceof Uint8Array).toBe(true);
    // Magic: \0asm
    expect(bytes[0]).toBe(0x00);
    expect(bytes[1]).toBe(0x61);
    expect(bytes[2]).toBe(0x73);
    expect(bytes[3]).toBe(0x6d);
    // Version 1
    expect(bytes[4]).toBe(0x01);
  });

  it("should execute exported WebAssembly functions correctly", async () => {
    const wasm = await getStorytoldWasm();
    expect(typeof wasm.add).toBe("function");
    expect(typeof wasm.vec4_dot).toBe("function");
    expect(typeof wasm.fast_lerp).toBe("function");
    expect(typeof wasm.compute_orbit).toBe("function");

    expect(wasm.add(15, 27)).toBe(42);
    // vec4_dot: 1*1 + 2*2 + 3*3 + 4*4 = 1 + 4 + 9 + 16 = 30
    expect(wasm.vec4_dot(1, 2, 3, 4, 1, 2, 3, 4)).toBeCloseTo(30, 2);
    // fast_lerp: 10 + (30 - 10) * 0.5 = 20
    expect(wasm.fast_lerp(10, 30, 0.5)).toBeCloseTo(20, 2);
    // compute_orbit: 3.14 * 10 = 31.4
    expect(wasm.compute_orbit(3.14, 10)).toBeCloseTo(31.4, 1);
  });

  it("should contain all 6 Storytold suite applications with metadata", () => {
    expect(STORYTOLD_APPS.length).toBe(6);
    const ids = STORYTOLD_APPS.map((a) => a.id);
    expect(ids).toContain("artcraft");
    expect(ids).toContain("bevy-3d");
    expect(ids).toContain("hanashi-shader");
    expect(ids).toContain("point-cloud");
    expect(ids).toContain("audio-vis");
    expect(ids).toContain("codecraft");
  });
});

describe("Universal Declarative Store & Multi-Ecosystem Package Manager", () => {
  it("should support bun:sqlite declarative generations and rollbacks", () => {
    const db = new StoreDatabase(":memory:");
    const initial = db.getActiveGeneration();
    expect(initial.id).toBe(1);
    expect(initial.active).toBe(true);

    const gen2 = db.createGeneration("Install bun-elysia", ["bun-elysia"]);
    expect(gen2.id).toBe(2);
    expect(gen2.active).toBe(true);
    expect(gen2.packages).toEqual(["bun-elysia"]);

    const gen3 = db.createGeneration("Install wgpu", [
      "bun-elysia",
      "wasm-wgpu",
    ]);
    expect(gen3.id).toBe(3);
    expect(gen3.packages.length).toBe(2);

    // Rollback to Generation 2
    const rollbackGen = db.rollbackToGeneration(2);
    expect(rollbackGen.id).toBe(4);
    expect(rollbackGen.rollbackId).toBe(2);
    expect(rollbackGen.packages).toEqual(["bun-elysia"]);

    const active = db.getActiveGeneration();
    expect(active.id).toBe(4);
    expect(active.packages).toEqual(["bun-elysia"]);

    db.close();
  });

  it("should parse arbitrary Awesome List markdown into StorePackages", () => {
    const md = `
# Awesome Graphics & Toolchains

## WebGPU & Shaders
- [wgpu](https://github.com/gfx-rs/wgpu) - Safe, portable GPU library implementing WebGPU in Rust.
- [naga](https://github.com/gfx-rs/wgpu) - Shader translation library for WGSL and SPIR-V.

## Web Runtimes
- [bun](https://github.com/oven-sh/bun) - Fast all-in-one JavaScript runtime in full Rust.
    `;

    const result = AwesomeListConverter.parseMarkdown(md, "Test Awesome");
    expect(result.totalParsed).toBe(3);
    expect(result.categories).toContain("WebGPU & Shaders");
    expect(result.categories).toContain("Web Runtimes");

    const wgpu = result.packages.find((p) => p.name === "wgpu")!;
    expect(wgpu).toBeDefined();
    expect(wgpu.repoUrl).toBe("https://github.com/gfx-rs/wgpu");
    expect(wgpu.pickedStack).toBeDefined();

    const bun = result.packages.find((p) => p.name === "bun")!;
    expect(bun).toBeDefined();
    expect(bun.ecosystem).toBe("bun");
  });

  it("should generate NixOS declarative flakes with package profiles", () => {
    const flake = generateNixDeclarativeFlake(["nixpkgs#bun", "nixpkgs#rustc"]);
    expect(flake).toContain("nixpkgs.url");
    expect(flake).toContain("pkgs.bun");
    expect(flake).toContain("pkgs.rustc");
  });

  it("should search packages across all ecosystems through UniversalStoreManager", async () => {
    const manager = new UniversalStoreManager(":memory:");
    const results = await manager.search("bun", "all");
    expect(results.length).toBeGreaterThan(0);
    const names = results.map((r) => r.name.toLowerCase());
    expect(names.some((n) => n.includes("bun"))).toBe(true);
  });
});

describe("Performance, LTO, AOT & Zero-Copy Stack Benchmark & Picker Engine", () => {
  it("should benchmark Zero-Copy vs Full Copy and measure speedup", () => {
    const res = benchmarkZeroCopyVsCopy(1 * 1024 * 1024);
    expect(res.dataSizeBytes).toBe(1 * 1024 * 1024);
    expect(res.zeroCopyMemoryAllocatedBytes).toBe(0);
    expect(res.copyMemoryAllocatedBytes).toBe(1 * 1024 * 1024);
    expect(res.speedupRatio).toBeGreaterThan(1);
  });

  it("should benchmark WASM SIMD vs JS JIT vector operations", async () => {
    const res = await benchmarkWasmVsJs(20_000);
    expect(res.iterations).toBe(20_000);
    expect(res.wasmNsPerOp).toBeGreaterThan(0);
    expect(res.jsNsPerOp).toBeGreaterThan(0);
  });

  it("should benchmark bun:sqlite prepared statement throughput", async () => {
    const res = await benchmarkSqliteThroughput(1000);
    expect(res.recordsCount).toBe(1000);
    expect(res.opsPerSec).toBeGreaterThan(10_000);
  });

  it("should evaluate and pick optimal compiler stack for different workloads", () => {
    // 1. GPU Compute
    const gpuRec = pickOptimalStack({
      name: "raymarch-shader",
      workload: "bare-metal-gpu",
    });
    expect(gpuRec.recommendedStack.id).toBe("cpp-msvc-cuda");
    expect(
      gpuRec.optimalFlags.some(
        (f) => f.includes("CUDA") || f.includes("sm_89"),
      ),
    ).toBe(true);

    // 2. Portable Web / Sandbox
    const webRec = pickOptimalStack({
      name: "browser-app",
      workload: "portable-web",
    });
    expect(webRec.recommendedStack.id).toBe("wasm-simd-webgpu");

    // 3. Edge HTTP / Concurrency
    const httpRec = pickOptimalStack({
      name: "api-service",
      workload: "edge-http",
    });
    expect(httpRec.recommendedStack.id).toBe("bun-native-ffi");

    // 4. CLI / Filesystem Scanner
    const cliRec = pickOptimalStack({
      name: "tree-scanner",
      workload: "cli-scanner",
    });
    expect(cliRec.recommendedStack.id).toBe("rust-lto-fat");
    expect(cliRec.optimalFlags).toContain("-C lto=fat");
  });

  it("should run full benchmark suite and produce complete system report", async () => {
    const report = await runBenchmarkSuite();
    expect(report.hardware.cpu).toContain("Intel Core i7-13700F");
    expect(report.hardware.gpu).toContain("RTX 4070");
    expect(report.zeroCopy.speedupRatio).toBeGreaterThan(1);
    expect(report.candidateStacks.length).toBe(CANDIDATE_STACKS.length);
  });
});

describe("Aphrody HTTP Server & Endpoints", () => {
  it("should serve index.html at GET / with 200 OK and security headers", async () => {
    const req = new Request("http://localhost:3000/");
    const res = await server.fetch(req);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toContain("text/html");
    expect(res.headers.get("x-content-type-options")).toBe("nosniff");
    expect(res.headers.get("x-frame-options")).toBe("SAMEORIGIN");
    expect(res.headers.get("content-security-policy")).toBeDefined();

    const text = await res.text();
    expect(text).toContain("Google");
    expect(text).toContain('id="root"');
  });

  it("should serve autocomplete JSON at GET /api/complete?q=aphrody", async () => {
    const req = new Request("http://localhost:3000/api/complete?q=aphrody");
    const res = await server.fetch(req);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toContain("application/json");

    const data = await res.json();
    expect(data.query).toBe("aphrody");
    expect(Array.isArray(data.suggestions)).toBe(true);
    expect(data.suggestions.length).toBeGreaterThan(0);
  });

  it("should serve Store search results at GET /api/store/search?q=bun", async () => {
    const req = new Request("http://localhost:3000/api/store/search?q=bun");
    const res = await server.fetch(req);
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data.query).toBe("bun");
    expect(Array.isArray(data.packages)).toBe(true);
    expect(data.packages.length).toBeGreaterThan(0);
  });

  it("should serve Store generations at GET /api/store/generations", async () => {
    const req = new Request("http://localhost:3000/api/store/generations");
    const res = await server.fetch(req);
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data.active).toBeDefined();
    expect(Array.isArray(data.generations)).toBe(true);
  });

  it("should run and return benchmark report at GET /api/bench", async () => {
    const req = new Request("http://localhost:3000/api/bench");
    const res = await server.fetch(req);
    expect(res.status).toBe(200);

    const data = await res.json();
    expect(data.hardware).toBeDefined();
    expect(data.zeroCopy).toBeDefined();
    expect(data.sqlite).toBeDefined();
  });

  it("should serve health check at GET /healthz and /ok", async () => {
    const req = new Request("http://localhost:3000/ok");
    const res = await server.fetch(req);
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.status).toBe("healthy");
  });

  it("should serve short 3-7 char routes (/api/q, /api/pm, /api/bnc)", async () => {
    const qRes = await server.fetch(
      new Request("http://localhost:3000/api/q?q=bun"),
    );
    expect(qRes.status).toBe(200);
    const qData = await qRes.json();
    expect(qData.suggestions.length).toBeGreaterThan(0);

    const pmRes = await server.fetch(
      new Request("http://localhost:3000/api/pm?q=bun"),
    );
    expect(pmRes.status).toBe(200);
    const pmData = await pmRes.json();
    expect(pmData.packages.length).toBeGreaterThan(0);

    const bncRes = await server.fetch(
      new Request("http://localhost:3000/api/bnc"),
    );
    expect(bncRes.status).toBe(200);
  });

  it("should serve static crawler files (/robots.txt, /sitemap.xml, /llms.txt)", async () => {
    const rRes = await server.fetch(
      new Request("http://localhost:3000/robots.txt"),
    );
    expect(rRes.status).toBe(200);
    expect(await rRes.text()).toContain("User-agent: *");

    const sRes = await server.fetch(
      new Request("http://localhost:3000/sitemap.xml"),
    );
    expect(sRes.status).toBe(200);
    expect(await sRes.text()).toContain("<urlset");

    const lRes = await server.fetch(
      new Request("http://localhost:3000/llms.txt"),
    );
    expect(lRes.status).toBe(200);
    expect(await lRes.text()).toContain("# Aphrody");
  });
});
