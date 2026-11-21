#!/usr/bin/env bun
/**
 * Measures the code-only site (site/public/*.html) in a real Chromium driven by agent-browser:
 *  - runtime requests: the server logs every request (only the document is allowed) and the page's
 *    resource timing entries must be empty;
 *  - canvas backing size against CSS size x devicePixelRatio at DPR 1 and 2 (browser launched with
 *    --force-device-scale-factor; CDP viewport emulation does not change devicePixelContentBoxSize), within 1 device pixel;
 *  - main-thread cost of each requestAnimationFrame callback (including the microtask draw the callback
 *    queues), recorded by wrapping requestAnimationFrame before page scripts run, during a synthetic
 *    pointer sweep and during a maximize click, against the 8.33 ms budget of a 120 Hz frame.
 * Automation browsers throttle rAF: this reports cost per callback in ms, never a frame rate.
 *
 * Run: bun tooling/bun/site-perf.ts [--page index.html] [--budget 8.33]
 * Needs: agent-browser (Chromium with WebGPU) on PATH. Exit 1 when a check fails.
 */
import { mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const flag = (name: string) => (Bun.argv.includes(name) ? Bun.argv[Bun.argv.indexOf(name) + 1] : undefined);
const siteDir = resolve(import.meta.dir, "../../site/public");
const budget = Number(flag("--budget") ?? 8.33);
const pages = flag("--page") ? [flag("--page") as string] : readdirSync(siteDir).filter((name) => name.endsWith(".html")).sort();
const session = `site-perf-${process.pid}`;

const requests: string[] = [];
const server = Bun.serve({
  hostname: "127.0.0.1",
  port: 0,
  fetch(request) {
    const path = new URL(request.url).pathname;
    requests.push(path);
    const file = Bun.file(join(siteDir, path === "/" ? "index.html" : path));
    return file.size ? new Response(file) : new Response("Not Found", { status: 404 });
  },
});

const work = mkdtempSync(join(tmpdir(), "site-perf-"));
const initScript = join(work, "raf.js");
writeFileSync(
  initScript,
  `const raf = window.requestAnimationFrame.bind(window);
window.__perf = { on: false, frames: [] };
window.requestAnimationFrame = (callback) => raf((time) => {
  const start = performance.now();
  try { callback(time); } finally { queueMicrotask(() => { if (window.__perf.on) window.__perf.frames.push(performance.now() - start); }); }
});`,
);

const measure = `(async () => {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const stats = (frames) => {
    const sorted = [...frames].sort((a, b) => a - b);
    const at = (q) => sorted.length ? +sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))].toFixed(3) : null;
    return { frames: sorted.length, p50: at(0.5), p95: at(0.95), max: at(1) };
  };
  await sleep(800);
  const canvas = document.querySelector("canvas");
  const rect = canvas.getBoundingClientRect();
  const out = {
    resources: performance.getEntriesByType("resource").length,
    gpu: !!navigator.gpu,
    dpr: devicePixelRatio,
    canvas: { width: canvas.width, height: canvas.height, expectedWidth: Math.round(rect.width * devicePixelRatio), expectedHeight: Math.round(rect.height * devicePixelRatio) },
  };
  const perf = window.__perf;
  perf.frames.length = 0; perf.on = true;
  for (let i = 0; i < 240; i++) {
    const t = i / 239;
    window.dispatchEvent(new PointerEvent("pointermove", { clientX: innerWidth * (0.1 + 0.8 * t), clientY: innerHeight * (0.5 + 0.4 * Math.sin(t * Math.PI * 4)), pointerType: "mouse", bubbles: true }));
    await sleep(4);
  }
  await sleep(600);
  out.pointerSweep = stats(perf.frames.splice(0));
  const maximize = document.querySelector('[data-action="maximize"]');
  maximize.click();
  await sleep(1500);
  out.maximize = stats(perf.frames.splice(0));
  maximize.click();
  await sleep(1000);
  perf.on = false;
  return JSON.stringify(out);
})()`;

const agent = async (...args: string[]) => {
  const proc = Bun.spawn(["agent-browser", "--session", session, ...args], { stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text()]);
  if ((await proc.exited) !== 0) throw new Error(`agent-browser ${args.join(" ")}: ${stderr || stdout}`);
  return stdout.trim();
};

type Stats = { frames: number; p50: number | null; p95: number | null; max: number | null };
const failures: string[] = [];
const report: unknown[] = [];
try {
  for (const page of pages) {
    for (const scale of [1, 2]) {
      requests.length = 0;
      await agent("close").catch(() => "");
      await agent("open", `http://127.0.0.1:${server.port}/${page}`, "--init-script", initScript, "--args", `--force-device-scale-factor=${scale}`);
      const raw = await agent("eval", measure);
      const result = JSON.parse(JSON.parse(raw.split("\n").at(-1) as string)) as {
        resources: number;
        gpu: boolean;
        dpr: number;
        canvas: { width: number; height: number; expectedWidth: number; expectedHeight: number };
        pointerSweep: Stats;
        maximize: Stats;
      };
      const stray = requests.filter((path) => path !== `/${page}` && path !== "/" && path !== "/favicon.ico");
      const canvasDelta = Math.max(Math.abs(result.canvas.width - result.canvas.expectedWidth), Math.abs(result.canvas.height - result.canvas.expectedHeight));
      const checks = {
        runtime_requests: result.resources + stray.length,
        canvas_delta_px: canvasDelta,
        budget_ms: budget,
      };
      if (checks.runtime_requests !== 0) failures.push(`${page}@${scale}x: ${checks.runtime_requests} runtime requests ${stray.join(",")}`);
      if (!result.gpu) failures.push(`${page}@${scale}x: no WebGPU in this browser`);
      if (canvasDelta > 1) failures.push(`${page}@${scale}x: canvas ${result.canvas.width}x${result.canvas.height} vs ${result.canvas.expectedWidth}x${result.canvas.expectedHeight}`);
      for (const [name, stats] of [["pointerSweep", result.pointerSweep], ["maximize", result.maximize]] as const) {
        if (stats.max !== null && stats.max > budget) failures.push(`${page}@${scale}x ${name}: max ${stats.max} ms over ${budget} ms`);
        if (stats.frames === 0) failures.push(`${page}@${scale}x ${name}: no animation frame observed`);
      }
      report.push({ page, dpr: result.dpr, ...checks, canvas: result.canvas, pointerSweep: result.pointerSweep, maximize: result.maximize });
    }
  }
} finally {
  await agent("close").catch(() => "");
  server.stop(true);
  rmSync(work, { recursive: true, force: true });
}
console.log(JSON.stringify(report, null, 1));
if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}
