/**
 * Benchmark & Stack Picker Application UI
 * Live Nanosecond Benchmarks, LTO/AOT Analyzer & Stack Recommendation
 * Zero external npm dependencies - 100% pure Web Standards
 */

import {
  runBenchmarkSuite,
  pickOptimalStack,
  CANDIDATE_STACKS,
  type WorkloadType,
} from "../benchmark/stack-picker";
import { renderMaterialSymbol } from "../design/material-tokens";

export class BenchmarkApp {
  public element: HTMLDivElement;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "benchmark-app-wrapper";
    this.render();
  }

  private render(): void {
    this.element.innerHTML = `
      <div class="bench-hardware-banner">
        <div class="hw-chip">${renderMaterialSymbol("speed", 16)} <b>CPU:</b> Intel Core i7-13700F (16C/24T Raptor Lake AVX2)</div>
        <div class="hw-chip">${renderMaterialSymbol("auto_awesome", 16)} <b>GPU:</b> NVIDIA RTX 4070 12GB (Ada Lovelace sm_89, CUDA 13.4)</div>
        <div class="hw-chip">${renderMaterialSymbol("layers", 16)} <b>Runtime:</b> oven-sh/bun v1.4.4 (Full Rust)</div>
      </div>

      <div class="bench-actions-bar">
        <button class="btn-run-bench fluent-btn-brand">
          ${renderMaterialSymbol("play_arrow", 18)} <span>Lancer le Benchmark Haute Performance</span>
        </button>
        <span class="bench-hint">Mesure en nanosecondes réelles (Bun.nanoseconds + WebAssembly SIMD + Zero-Copy)</span>
      </div>

      <div class="bench-results-grid">
        <div class="bench-card" id="card-zerocopy">
          <h4>1. Zero-Copy vs Full Copy (5MB)</h4>
          <div class="bench-stat-large" id="stat-zerocopy-speedup">-- x</div>
          <div class="bench-stat-detail" id="stat-zerocopy-detail">Cliquez sur Lancer pour mesurer</div>
        </div>

        <div class="bench-card" id="card-wasm">
          <h4>2. WASM SIMD vs Pure JS JIT</h4>
          <div class="bench-stat-large" id="stat-wasm-speedup">-- ns</div>
          <div class="bench-stat-detail" id="stat-wasm-detail">100 000 opérations vectorielles</div>
        </div>

        <div class="bench-card" id="card-sqlite">
          <h4>3. bun:sqlite Prepared Statements</h4>
          <div class="bench-stat-large" id="stat-sqlite-ops">-- ops/s</div>
          <div class="bench-stat-detail" id="stat-sqlite-detail">2 000 insertions et requêtes indexées</div>
        </div>
      </div>

      <div class="stack-picker-section">
        <h3>Sélecteur Automatique de Stack Optimisée (LTO, AOT & Zero-Copy)</h3>
        <p>Entrez les paramètres de votre charge de travail pour générer la configuration de compilation optimale.</p>

        <div class="picker-inputs">
          <input type="text" id="picker-pkg-name" class="picker-input" placeholder="Nom de l'outil ou du paquet (ex: storytold-render)..." value="storytold-3d-pipeline" />
          <select id="picker-workload" class="picker-select">
            <option value="bare-metal-gpu">Bare-Metal GPU Compute (CUDA 13.4 sm_89 / C++23)</option>
            <option value="wayland-ui">Wayland / UI Compositor (Rust LTO / C++23)</option>
            <option value="edge-http">Edge HTTP / High-Throughput API (Bun Full Rust FFI)</option>
            <option value="cli-scanner">CLI & Filesystem Scanner (Rust fat LTO / .NET AOT)</option>
            <option value="portable-web">Portable Web / Zero-Install Browser (WASM SIMD + WebGPU)</option>
          </select>
          <button class="btn-pick-stack fluent-btn-brand">Analyser la Stack</button>
        </div>

        <div class="picker-output-card" id="picker-output">
          <!-- Dynamic stack recommendation renders here -->
        </div>
      </div>
    `;

    this.attachEvents();
    this.updateStackPicker();
  }

  private attachEvents(): void {
    const runBtn = this.element.querySelector(".btn-run-bench");
    runBtn?.addEventListener("click", async () => {
      runBtn.setAttribute("disabled", "true");
      runBtn.innerHTML = `<span>Mesure en cours...</span>`;

      try {
        const report = await runBenchmarkSuite();
        this.displayBenchmarkResults(report);
      } catch (err: any) {
        alert("Erreur benchmark: " + err.message);
      } finally {
        runBtn.removeAttribute("disabled");
        runBtn.innerHTML = `${renderMaterialSymbol("play_arrow", 18)} <span>Relancer le Benchmark</span>`;
      }
    });

    const pickBtn = this.element.querySelector(".btn-pick-stack");
    pickBtn?.addEventListener("click", () => {
      this.updateStackPicker();
    });
  }

  private displayBenchmarkResults(report: any): void {
    const zcLarge = this.element.querySelector("#stat-zerocopy-speedup");
    const zcDetail = this.element.querySelector("#stat-zerocopy-detail");
    if (zcLarge && zcDetail) {
      zcLarge.textContent = `${report.zeroCopy.speedupRatio}x`;
      zcDetail.innerHTML = `Zero-copy: <b>${(report.zeroCopy.zeroCopyDurationNs / 1000).toFixed(1)} µs</b> vs Copie: ${(report.zeroCopy.copyDurationNs / 1e6).toFixed(2)} ms`;
    }

    const wasmLarge = this.element.querySelector("#stat-wasm-speedup");
    const wasmDetail = this.element.querySelector("#stat-wasm-detail");
    if (wasmLarge && wasmDetail) {
      wasmLarge.textContent = `${report.wasmSimd.wasmNsPerOp} ns/op`;
      wasmDetail.innerHTML = `WASM: <b>${report.wasmSimd.wasmNsPerOp} ns</b> | JS: ${report.wasmSimd.jsNsPerOp} ns`;
    }

    const sqlLarge = this.element.querySelector("#stat-sqlite-ops");
    const sqlDetail = this.element.querySelector("#stat-sqlite-detail");
    if (sqlLarge && sqlDetail) {
      sqlLarge.textContent = `${(report.sqlite.opsPerSec / 1000).toFixed(0)}k ops/s`;
      sqlDetail.innerHTML = `Durée transaction: <b>${(report.sqlite.insertDurationNs / 1e6).toFixed(2)} ms</b> pour 2000 entrées`;
    }
  }

  private updateStackPicker(): void {
    const nameInput = this.element.querySelector(
      "#picker-pkg-name",
    ) as HTMLInputElement;
    const workloadSelect = this.element.querySelector(
      "#picker-workload",
    ) as HTMLSelectElement;
    const outputContainer = this.element.querySelector("#picker-output");
    if (!outputContainer) return;

    const name = nameInput?.value || "app";
    const workload = (workloadSelect?.value ||
      "bare-metal-gpu") as WorkloadType;

    const rec = pickOptimalStack({ name, workload });

    outputContainer.innerHTML = `
      <div class="rec-stack-header">
        <span class="rec-badge">RECOMMANDATION OFFICIELLE</span>
        <h4>${rec.recommendedStack.name}</h4>
      </div>
      <p class="rec-justification">${rec.justification}</p>
      <div class="rec-grid">
        <div class="rec-cell">
          <label>Compilateur & LTO :</label>
          <b>${rec.recommendedStack.compiler} (${rec.recommendedStack.ltoSetting})</b>
        </div>
        <div class="rec-cell">
          <label>Configuration AOT :</label>
          <b>${rec.recommendedStack.aotSetting}</b>
        </div>
        <div class="rec-cell">
          <label>Stratégie Zero-Copy :</label>
          <b>${rec.zeroCopyStrategy}</b>
        </div>
        <div class="rec-cell">
          <label>Gain de latence prévu :</label>
          <b class="text-gain">${rec.expectedLatencyReduction}</b>
        </div>
      </div>
      <div class="rec-flags-section">
        <label>Drapeaux de compilation optimaux :</label>
        <pre class="flags-code"><code>${rec.optimalFlags.join(" ")}</code></pre>
      </div>
    `;
  }
}
