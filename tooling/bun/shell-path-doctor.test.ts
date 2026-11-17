import { describe, expect, test } from "bun:test";
import { analyzePathEntries, classifyPriorityTier, MODERN_CLI_MATRIX, optimizeUserPathEntries } from "./shell-path-doctor.ts";

describe("shell-path-doctor", () => {
  test("detects dead paths, duplicates, and shadow conflicts", () => {
    const existing = new Set([
      "c:\\users\\dev\\.cargo\\bin",
      "c:\\users\\dev\\appdata\\local\\programs\\niubash\\winuxcmd\\usr\\bin",
      "c:\\users\\dev\\.local\\share\\ay\\tools\\cmake-4.4.5-windows-x86_64\\bin",
      "c:\\users\\dev\\appdata\\local\\rosbe\\pkgs\\cmake-3.31.6\\bin",
    ]);
    const checkExists = (p: string) => existing.has(p.replaceAll("/", "\\").toLowerCase());

    const user = [
      "C:\\Users\\dev\\AppData\\Local\\RosBE\\pkgs\\cmake-3.31.6\\bin",
      "C:\\Users\\dev\\AppData\\Local\\Programs\\Niubash\\winuxcmd\\usr\\bin",
      "C:\\Users\\dev\\.cargo\\bin",
      "C:\\Users\\dev\\.cargo\\bin\\",
      "C:\\Missing\\Python314",
    ];
    const machine = [
      "C:\\Users\\dev\\.local\\share\\ay\\tools\\cmake-4.4.5-windows-x86_64\\bin",
    ];

    const report = analyzePathEntries(user, machine, checkExists, true);
    expect(report.deadEntries).toContain("C:\\Missing\\Python314");
    expect(report.duplicateEntries.length).toBe(1);
    expect(report.conflicts.some((c) => c.kind === "shadow" && c.detail.includes("Niubash"))).toBe(true);
    expect(report.conflicts.some((c) => c.kind === "shadow" && c.detail.includes("CMake"))).toBe(true);
  });

  test("optimizeUserPathEntries puts native Rust/Wasm/Nu toolchains first and Niubash/WindowsApps last", () => {
    const home = "C:\\Users\\dev";
    const localAppData = "C:\\Users\\dev\\AppData\\Local";
    const programFiles = "C:\\Program Files";
    const existing = new Set([
      "c:\\users\\dev\\.bun\\bin",
      "c:\\users\\dev\\.cargo\\bin",
      "c:\\users\\dev\\.local\\bin",
      "c:\\users\\dev\\appdata\\local\\programs\\nu\\bin",
      "c:\\program files\\wasmtime\\bin",
      "c:\\users\\dev\\appdata\\local\\microsoft\\winget\\links",
      "c:\\users\\dev\\appdata\\local\\programs\\niubash\\winuxcmd\\usr\\bin",
      "c:\\users\\dev\\appdata\\local\\microsoft\\windowsapps",
      "c:\\users\\dev\\ffmpeg\\ffmpeg-9.0.2-full_build\\bin",
      "c:\\users\\dev\\ffmpeg\\ffmpeg-9.0.2-full_build-shared\\bin",
    ]);
    const checkExists = (p: string) => existing.has(p.replaceAll("/", "\\").toLowerCase());

    const res = optimizeUserPathEntries(
      [
        "C:\\Users\\dev\\AppData\\Local\\Programs\\Niubash\\winuxcmd\\usr\\bin",
        "C:\\Users\\dev\\AppData\\Local\\Microsoft\\WindowsApps",
        "C:\\Users\\dev\\ffmpeg\\ffmpeg-9.0.2-full_build\\bin",
        "C:\\Users\\dev\\ffmpeg\\ffmpeg-9.0.2-full_build-shared\\bin",
        "C:\\DoesNotExist\\NodeBins",
        "C:\\Users\\dev\\.cargo\\bin",
        "C:\\Users\\dev\\.bun\\bin",
      ],
      [],
      home,
      localAppData,
      programFiles,
      checkExists,
    );

    expect(res.removed.some((r) => r.includes("DoesNotExist"))).toBe(true);
    expect(res.removed.some((r) => r.includes("redundant FFmpeg"))).toBe(true);
    expect(res.added).toContain("C:\\Users\\dev\\AppData\\Local\\Programs\\nu\\bin");
    expect(res.added).toContain("C:\\Program Files\\Wasmtime\\bin");

    const bunIdx = res.optimized.indexOf("C:\\Users\\dev\\.bun\\bin");
    const cargoIdx = res.optimized.indexOf("C:\\Users\\dev\\.cargo\\bin");
    const nuIdx = res.optimized.indexOf("C:\\Users\\dev\\AppData\\Local\\Programs\\nu\\bin");
    const niuIdx = res.optimized.indexOf("C:\\Users\\dev\\AppData\\Local\\Programs\\Niubash\\winuxcmd\\usr\\bin");
    expect(bunIdx).toBe(0);
    expect(cargoIdx).toBe(1);
    expect(nuIdx).toBeGreaterThan(cargoIdx);
    expect(niuIdx).toBeGreaterThan(nuIdx);
    expect(classifyPriorityTier("C:\\Users\\dev\\.cargo\\bin", home)).toBe(0);
    expect(classifyPriorityTier("C:\\Users\\dev\\AppData\\Local\\Programs\\Niubash\\winuxcmd\\usr\\bin", home)).toBe(3);
  });

  test("MODERN_CLI_MATRIX tracks nu, pwsh, wasmtime, wasm-opt, wasm-bindgen, xh, and monolith", () => {
    const ids = new Set(MODERN_CLI_MATRIX.map((t) => t.id));
    for (const req of ["nu", "pwsh", "wasmtime", "wasm-opt", "wasm-bindgen", "wasm-tools", "wit-bindgen", "wac", "xh", "monolith"]) {
      expect(ids.has(req)).toBe(true);
    }
  });
});
