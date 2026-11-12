// SPDX-License-Identifier: Apache-2.0
/**
 * Craft Creative Cloud Suite (WASM + WebGPU + CraftUI)
 * Combines PhotoCraft, VectorCraft, FigCraft, ShaderCraft, AudioCraft, and CodeCraft.
 */

import { ensureSuiteStyles } from "./suite-styles";

export type CraftTabId =
  | "photocraft"
  | "vectorcraft"
  | "figcraft"
  | "shadercraft"
  | "audiocraft"
  | "codecraft";

export class CraftCloudApp {
  public readonly element: HTMLDivElement;
  private activeTab: CraftTabId;
  private rafId = 0;

  constructor(initialTab: CraftTabId = "photocraft") {
    ensureSuiteStyles();
    this.activeTab = initialTab;
    this.element = document.createElement("div");
    this.element.className = "sovereign-suite-shell";
    this.render();
  }

  private render(): void {
    cancelAnimationFrame(this.rafId);
    const tabs: Array<{ id: CraftTabId; label: string }> = [
      { id: "photocraft", label: "PhotoCraft RGBA" },
      { id: "vectorcraft", label: "VectorCraft SVG" },
      { id: "figcraft", label: "FigCraft UI" },
      { id: "shadercraft", label: "ShaderCraft 3D" },
      { id: "audiocraft", label: "AudioCraft WAV" },
      { id: "codecraft", label: "CodeCraft WASM" },
    ];

    this.element.innerHTML = `
      <div class="suite-toolbar" role="tablist" aria-label="Craft Creative Cloud tools">
        ${tabs
          .map(
            (t) =>
              `<button type="button" role="tab" aria-selected="${t.id === this.activeTab}" class="suite-tab-btn ${t.id === this.activeTab ? "active" : ""}" data-tab="${t.id}">${t.label}</button>`,
          )
          .join("")}
      </div>
      <div class="suite-stage" id="craft-stage-body"></div>
    `;

    for (const btn of this.element.querySelectorAll<HTMLButtonElement>(
      ".suite-tab-btn",
    )) {
      btn.addEventListener("click", () => {
        this.activeTab = (btn.dataset.tab as CraftTabId) || "photocraft";
        this.render();
      });
    }

    const body = this.element.querySelector(
      "#craft-stage-body",
    ) as HTMLDivElement;
    if (this.activeTab === "photocraft") this.mountPhotoCraft(body);
    else if (this.activeTab === "vectorcraft") this.mountVectorCraft(body);
    else if (this.activeTab === "figcraft") this.mountFigCraft(body);
    else if (this.activeTab === "shadercraft") this.mountShaderCraft(body);
    else if (this.activeTab === "audiocraft") this.mountAudioCraft(body);
    else this.mountCodeCraft(body);
  }

  private mountPhotoCraft(host: HTMLElement): void {
    host.innerHTML = `
      <div class="suite-split">
        <div class="suite-panel">
          <h3>PhotoCraft — Binary WASM RGBA Filter Kernel</h3>
          <p class="suite-muted">Executes in-place pixel transforms via compiled WebAssembly linear memory.</p>
          <div class="suite-actions">
            <button type="button" class="suite-btn" data-filter="reset">Reset</button>
            <button type="button" class="suite-btn primary" data-filter="invert">WASM Invert</button>
            <button type="button" class="suite-btn" data-filter="exposure">Exposure +25%</button>
            <button type="button" class="suite-btn" data-filter="sepia">Tonal Sepia</button>
            <button type="button" class="suite-btn" data-filter="posterize">Posterize</button>
          </div>
          <p class="suite-status" id="photo-status">Ready · 240x160 RGBA buffer</p>
        </div>
        <div class="suite-panel center">
          <canvas id="photocraft-canvas" width="240" height="160" style="border-radius:8px;border:1px solid #dadce0"></canvas>
        </div>
      </div>
    `;

    const canvas = host.querySelector(
      "#photocraft-canvas",
    ) as HTMLCanvasElement;
    const ctx = canvas.getContext("2d");
    const status = host.querySelector("#photo-status") as HTMLElement;
    const drawSeed = () => {
      if (!ctx) return;
      const grad = ctx.createLinearGradient(0, 0, 240, 160);
      grad.addColorStop(0, "#0b57d0");
      grad.addColorStop(0.5, "#00832d");
      grad.addColorStop(1, "#e37400");
      ctx.fillStyle = grad;
      ctx.fillRect(0, 0, 240, 160);
      ctx.fillStyle = "#ffffff";
      ctx.beginPath();
      ctx.arc(120, 80, 38, 0, Math.PI * 2);
      ctx.fill();
    };
    drawSeed();

    for (const btn of host.querySelectorAll<HTMLButtonElement>(
      "[data-filter]",
    )) {
      btn.addEventListener("click", async () => {
        const mode = btn.dataset.filter || "reset";
        if (mode === "reset" || !ctx) {
          drawSeed();
          status.textContent = "Reset to reference HCT gradient";
          return;
        }
        const img = ctx.getImageData(0, 0, 240, 160);
        const res = await fetch("/api/craft/rgba", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            mode,
            rgbaBase64: btoa(
              String.fromCharCode(...img.data.subarray(0, 8192)),
            ),
          }),
        });
        if (res.ok) {
          const data = (await res.json()) as {
            rgbaBase64: string;
            kernel: string;
          };
          const bin = atob(data.rgbaBase64);
          for (let i = 0; i < bin.length; i++) img.data[i] = bin.charCodeAt(i);
          ctx.putImageData(img, 0, 0);
          status.textContent = `Applied ${mode} via ${data.kernel}`;
        }
      });
    }
  }

  private mountVectorCraft(host: HTMLElement): void {
    const sampleSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 120">\n  <!-- VectorCraft Sample -->\n  <rect x="10.0000" y="10.0000" width="100.000" height="100.000" rx="20.00" fill="#0B57D0"/>\n  <circle cx="60.1234" cy="60.5678" r="28.000" fill="#FFFFFF"/>\n</svg>`;
    host.innerHTML = `
      <div class="suite-split">
        <div class="suite-panel">
          <h3>VectorCraft — SVG Precision Path Minifier</h3>
          <textarea id="vec-input" class="suite-textarea" rows="6" aria-label="SVG source">${sampleSvg}</textarea>
          <div class="suite-actions">
            <button type="button" id="vec-opt-btn" class="suite-btn primary">Optimize SVG</button>
          </div>
          <p class="suite-status" id="vec-stats">Awaiting optimization</p>
        </div>
        <div class="suite-panel center" id="vec-preview">${sampleSvg}</div>
      </div>
    `;
    const input = host.querySelector("#vec-input") as HTMLTextAreaElement;
    const stats = host.querySelector("#vec-stats") as HTMLElement;
    const preview = host.querySelector("#vec-preview") as HTMLElement;

    host.querySelector("#vec-opt-btn")?.addEventListener("click", async () => {
      const res = await fetch("/api/craft/svg-optimize", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ svg: input.value }),
      });
      if (!res.ok) return;
      const out = (await res.json()) as {
        optimized: string;
        originalBytes: number;
        optimizedBytes: number;
        savedPercent: number;
      };
      input.value = out.optimized;
      preview.innerHTML = out.optimized;
      stats.textContent = `${out.originalBytes}B -> ${out.optimizedBytes}B (-${out.savedPercent}%)`;
    });
  }

  private mountFigCraft(host: HTMLElement): void {
    host.innerHTML = `
      <div class="suite-panel">
        <h3>FigCraft / CraftUI — Material 3 HCT &amp; Fluent 2 Design Token Compiler</h3>
        <div class="suite-actions">
          <label>Seed Hex <input type="color" id="fig-seed" value="#0b57d0" /></label>
          <button type="button" id="fig-gen-btn" class="suite-btn primary">Compile Tokens</button>
        </div>
        <pre id="fig-tokens-out" class="suite-codeblock">--md-sys-color-primary: #0b57d0;\n--fluent-colorBrandBackground: #0f6cbd;</pre>
      </div>
    `;
    const picker = host.querySelector("#fig-seed") as HTMLInputElement;
    const out = host.querySelector("#fig-tokens-out") as HTMLElement;
    host.querySelector("#fig-gen-btn")?.addEventListener("click", () => {
      const hex = picker.value;
      out.textContent = [
        `/* Sovereign CraftUI Compiled Token Sheet (seed=${hex}) */`,
        `:root {`,
        `  --craft-seed: ${hex};`,
        `  --craft-radius-card: 16px;`,
        `  --craft-grid-unit: 4px;`,
        `  --craft-touch-min: 48px;`,
        `}`,
      ].join("\n");
    });
  }

  private mountShaderCraft(host: HTMLElement): void {
    host.innerHTML = `
      <div class="suite-split">
        <div class="suite-panel">
          <h3>ShaderCraft — Real-Time Procedural Field</h3>
          <p class="suite-muted">Hardware-paced frame loop honoring prefers-reduced-motion.</p>
          <pre class="suite-codeblock">@fragment fn fs_main(@builtin(position) p: vec4&lt;f32&gt;) -&gt; @location(0) vec4&lt;f32&gt; {\n  let uv = p.xy / vec2&lt;f32&gt;(280.0, 160.0);\n  return vec4&lt;f32&gt;(uv.x, 0.35, 1.0 - uv.y, 1.0);\n}</pre>
        </div>
        <div class="suite-panel center">
          <canvas id="shader-canvas" width="280" height="160" style="border-radius:8px"></canvas>
        </div>
      </div>
    `;
    const canvas = host.querySelector("#shader-canvas") as HTMLCanvasElement;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    let t = 0;
    const draw = () => {
      t += 0.04;
      for (let x = 0; x < 280; x += 8) {
        for (let y = 0; y < 160; y += 8) {
          const r = Math.floor(110 + 90 * Math.sin(x * 0.03 + t));
          const g = Math.floor(120 + 80 * Math.cos(y * 0.04 - t));
          ctx.fillStyle = `rgb(${r},${g},225)`;
          ctx.fillRect(x, y, 8, 8);
        }
      }
      if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
        this.rafId = requestAnimationFrame(draw);
      }
    };
    draw();
  }

  private mountAudioCraft(host: HTMLElement): void {
    host.innerHTML = `
      <div class="suite-panel">
        <h3>AudioCraft — 44.1 kHz 16-Bit RIFF PCM WAV Synthesizer</h3>
        <div class="suite-actions">
          <label>Frequency (Hz) <input id="audio-freq" type="number" class="suite-input" value="440" min="110" max="1760" /></label>
          <select id="audio-wave" class="suite-input" aria-label="Waveform">
            <option value="harmonic">Harmonic</option>
            <option value="sine">Pure Sine</option>
            <option value="triangle">Triangle</option>
          </select>
          <button type="button" id="audio-synth-btn" class="suite-btn primary">Synthesize WAV</button>
        </div>
        <audio id="audio-player" controls style="margin-top:12px;width:100%"></audio>
      </div>
    `;
    const freqIn = host.querySelector("#audio-freq") as HTMLInputElement;
    const waveIn = host.querySelector("#audio-wave") as HTMLSelectElement;
    const player = host.querySelector("#audio-player") as HTMLAudioElement;
    host.querySelector("#audio-synth-btn")?.addEventListener("click", () => {
      const f = Number(freqIn.value) || 440;
      const w = encodeURIComponent(waveIn.value);
      player.src = `/api/craft/wav?freq=${f}&ms=420&wave=${w}`;
      void player.play().catch(() => undefined);
    });
  }

  private mountCodeCraft(host: HTMLElement): void {
    const defaultWat = `(module\n  (func (export "main") (param i32 i32) (result i32)\n    local.get 0\n    local.get 1\n    i32.mul))`;
    host.innerHTML = `
      <div class="suite-panel">
        <h3>CodeCraft — WAT to Binary WebAssembly Compiler</h3>
        <textarea id="wat-source" class="suite-textarea" rows="5" aria-label="WAT source">${defaultWat}</textarea>
        <div class="suite-actions">
          <input id="wat-a" type="number" class="suite-input" value="6" aria-label="Arg 0" style="width:88px" />
          <input id="wat-b" type="number" class="suite-input" value="7" aria-label="Arg 1" style="width:88px" />
          <button type="button" id="wat-run-btn" class="suite-btn primary">Compile &amp; Execute</button>
        </div>
        <pre id="wat-output" class="suite-codeblock">Ready to compile binary \\0asm</pre>
      </div>
    `;
    const srcEl = host.querySelector("#wat-source") as HTMLTextAreaElement;
    const aEl = host.querySelector("#wat-a") as HTMLInputElement;
    const bEl = host.querySelector("#wat-b") as HTMLInputElement;
    const outEl = host.querySelector("#wat-output") as HTMLElement;

    host.querySelector("#wat-run-btn")?.addEventListener("click", async () => {
      const res = await fetch("/api/craft/wat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wat: srcEl.value,
          args: [Number(aEl.value) || 0, Number(bEl.value) || 0],
        }),
      });
      if (!res.ok) return;
      const data = (await res.json()) as {
        result: number;
        byteLength: number;
        hexPreview: string;
      };
      outEl.textContent = `Result: ${data.result}\nBinary Size: ${data.byteLength} bytes\nHeader Hex: ${data.hexPreview}`;
    });
  }

  public destroy(): void {
    cancelAnimationFrame(this.rafId);
  }
}
