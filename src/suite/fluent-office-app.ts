// SPDX-License-Identifier: Apache-2.0
/**
 * Fluent UI 2 Office 365 Productivity Suite
 * WordCraft (.docx OOXML PKZIP), GridCraft (Zero-Eval Recursive-Descent Formulas), DeckCraft (16:9 Vector Slides).
 */

import { ensureSuiteStyles } from "./suite-styles";

export type OfficeTabId = "word" | "grid" | "deck";

export class FluentOfficeApp {
  public readonly element: HTMLDivElement;
  private activeTab: OfficeTabId;

  constructor(initialTab: OfficeTabId = "word") {
    ensureSuiteStyles();
    this.activeTab = initialTab;
    this.element = document.createElement("div");
    this.element.className = "sovereign-suite-shell";
    this.render();
  }

  private render(): void {
    this.element.innerHTML = `
      <div class="suite-toolbar" role="tablist" aria-label="Fluent Office 365 Suite">
        <button type="button" role="tab" class="suite-tab-btn ${this.activeTab === "word" ? "active" : ""}" data-tab="word">WordCraft (.docx)</button>
        <button type="button" role="tab" class="suite-tab-btn ${this.activeTab === "grid" ? "active" : ""}" data-tab="grid">GridCraft (Formulas)</button>
        <button type="button" role="tab" class="suite-tab-btn ${this.activeTab === "deck" ? "active" : ""}" data-tab="deck">DeckCraft (Slides)</button>
      </div>
      <div class="suite-stage" id="office-stage"></div>
    `;

    for (const btn of this.element.querySelectorAll<HTMLButtonElement>(
      ".suite-tab-btn",
    )) {
      btn.addEventListener("click", () => {
        this.activeTab = (btn.dataset.tab as OfficeTabId) || "word";
        this.render();
      });
    }

    const stage = this.element.querySelector("#office-stage") as HTMLElement;
    if (this.activeTab === "word") this.mountWordCraft(stage);
    else if (this.activeTab === "grid") this.mountGridCraft(stage);
    else this.mountDeckCraft(stage);
  }

  private mountWordCraft(host: HTMLElement): void {
    host.innerHTML = `
      <div class="suite-panel">
        <h3>WordCraft — Fluent 2 Document Studio &amp; Native OOXML .docx Exporter</h3>
        <input id="word-title" type="text" class="suite-input" value="Aphrody Sovereign Infrastructure Report" aria-label="Document title" style="width:100%;margin-bottom:12px" />
        <textarea id="word-body" class="suite-textarea" rows="8" aria-label="Document content">1. Three-VPS Cluster Topology: fr1 (OVH Edge), fr2 (OVH Headless Chromium Crawler), usa (Google Grounding Egress).\n2. Zero external npm dependencies across runtime, bundler, SQLite WAL, and binary OOXML export.\n3. Sub-14KB initial critical path verified by WCK and Lighthouse.</textarea>
        <div class="suite-actions">
          <button type="button" id="word-save-btn" class="suite-btn">Save to SQLite</button>
          <button type="button" id="word-docx-btn" class="suite-btn primary">Export Genuine .docx</button>
        </div>
        <p class="suite-status" id="word-status">Ready</p>
      </div>
    `;

    const titleEl = host.querySelector("#word-title") as HTMLInputElement;
    const bodyEl = host.querySelector("#word-body") as HTMLTextAreaElement;
    const statusEl = host.querySelector("#word-status") as HTMLElement;

    host
      .querySelector("#word-save-btn")
      ?.addEventListener("click", async () => {
        const res = await fetch("/api/office/docs", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            id: "word-active",
            kind: "word",
            title: titleEl.value,
            payload: { body: bodyEl.value },
          }),
        });
        if (res.ok) statusEl.textContent = "Saved to var/workspace.sqlite WAL";
      });

    host
      .querySelector("#word-docx-btn")
      ?.addEventListener("click", async () => {
        const res = await fetch("/api/office/docx", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ title: titleEl.value, body: bodyEl.value }),
        });
        if (!res.ok) return;
        const blob = await res.blob();
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `${titleEl.value.toLowerCase().replace(/[^a-z0-9]+/g, "-") || "document"}.docx`;
        a.click();
        URL.revokeObjectURL(url);
        statusEl.textContent = `Downloaded OOXML .docx (${blob.size} bytes PKZIP)`;
      });
  }

  private mountGridCraft(host: HTMLElement): void {
    const initial: Record<string, string> = {
      A1: "fr1 OVH Edge",
      B1: "48",
      A2: "fr2 OVH Chromium",
      B2: "64",
      A3: "usa Google Egress",
      B3: "24",
      A4: "Cluster Total",
      B4: "=SUM(B1:B3)",
      A5: "Cluster Mean",
      B5: "=ROUND(AVG(B1:B3), 2)",
    };

    const rowsHtml = [1, 2, 3, 4, 5]
      .map(
        (r) => `<tr>
          <th>${r}</th>
          ${["A", "B", "C"]
            .map((c) => {
              const ref = `${c}${r}`;
              return `<td><input class="suite-cell-input" data-cell="${ref}" value="${initial[ref] || ""}" aria-label="Cell ${ref}" /></td>`;
            })
            .join("")}
        </tr>`,
      )
      .join("");

    host.innerHTML = `
      <div class="suite-panel">
        <h3>GridCraft — Deterministic Zero-Eval Spreadsheet Engine</h3>
        <p class="suite-muted">Supports SUM, AVG, MIN, MAX, ROUND, ABS, IF, NPV, and range references (A1:B5).</p>
        <table class="suite-grid-table">
          <thead><tr><th>#</th><th>A</th><th>B</th><th>C</th></tr></thead>
          <tbody>${rowsHtml}</tbody>
        </table>
        <div class="suite-actions">
          <button type="button" id="grid-eval-btn" class="suite-btn primary">Evaluate Formulas</button>
        </div>
        <pre id="grid-eval-out" class="suite-codeblock">Click "Evaluate Formulas" to compute sheet values.</pre>
      </div>
    `;

    const out = host.querySelector("#grid-eval-out") as HTMLElement;
    const runEval = async () => {
      const cells: Record<string, string> = {};
      for (const inp of host.querySelectorAll<HTMLInputElement>(
        "[data-cell]",
      )) {
        const ref = inp.dataset.cell!;
        if (inp.value.trim()) cells[ref] = inp.value.trim();
      }
      const res = await fetch("/api/office/formula", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ cells }),
      });
      if (!res.ok) return;
      const data = (await res.json()) as {
        evaluated: Record<string, string | number>;
      };
      out.textContent = JSON.stringify(data.evaluated, null, 2);
    };
    host
      .querySelector("#grid-eval-btn")
      ?.addEventListener("click", () => void runEval());
    void runEval();
  }

  private mountDeckCraft(host: HTMLElement): void {
    host.innerHTML = `
      <div class="suite-split">
        <div class="suite-panel">
          <h3>DeckCraft — 16:9 Vector Slide Studio</h3>
          <input id="deck-headline" type="text" class="suite-input" value="Aphrody 3-VPS Sovereign Datacenter" aria-label="Slide headline" style="width:100%;margin-bottom:8px" />
          <input id="deck-sub" type="text" class="suite-input" value="Pure Bun v1.4.4 · Chromium CDP · SQLite FTS5 RRF" aria-label="Slide subtitle" style="width:100%" />
        </div>
        <div class="suite-panel center">
          <div id="deck-stage" style="aspect-ratio:16/9;width:100%;max-width:360px;background:#0b57d0;color:#fff;border-radius:12px;padding:24px;display:flex;flex-direction:column;justify-content:center">
            <strong id="deck-h-view" style="font-size:20px;line-height:28px">Aphrody 3-VPS Sovereign Datacenter</strong>
            <span id="deck-s-view" style="font-size:13px;opacity:0.9;margin-top:8px">Pure Bun v1.4.4 · Chromium CDP · SQLite FTS5 RRF</span>
          </div>
        </div>
      </div>
    `;
    const hIn = host.querySelector("#deck-headline") as HTMLInputElement;
    const sIn = host.querySelector("#deck-sub") as HTMLInputElement;
    const hView = host.querySelector("#deck-h-view") as HTMLElement;
    const sView = host.querySelector("#deck-s-view") as HTMLElement;
    hIn.addEventListener("input", () => (hView.textContent = hIn.value));
    sIn.addEventListener("input", () => (sView.textContent = sIn.value));
  }
}
