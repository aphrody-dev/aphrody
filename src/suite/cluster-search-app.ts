// SPDX-License-Identifier: Apache-2.0
/**
 * End-to-End 3-VPS Datacenter Search & Subdomain Portal View
 * Displays RRF k=60 fused results from fr1 + fr2 + usa, Knowledge Card, and *.aphrody.app launchers.
 */

import { ensureSuiteStyles } from "./suite-styles";

export class ClusterSearchApp {
  public readonly element: HTMLDivElement;
  private onOpenSuite?: (appId: string) => void;

  constructor(initialQuery = "aphrody", onOpenSuite?: (appId: string) => void) {
    ensureSuiteStyles();
    this.onOpenSuite = onOpenSuite;
    this.element = document.createElement("div");
    this.element.className = "sovereign-suite-shell";
    this.renderShell(initialQuery);
    void this.executeQuery(initialQuery);
  }

  private renderShell(initialQuery: string): void {
    this.element.innerHTML = `
      <div class="suite-stage">
        <form id="cluster-ui-form" class="suite-actions" style="margin-bottom:16px">
          <input id="cluster-ui-q" type="search" class="suite-input" value="${this.esc(initialQuery)}" placeholder="Search 3-VPS Sovereign Index..." aria-label="Search query" style="flex:1" />
          <button type="submit" class="suite-btn primary">Search Cluster</button>
        </form>
        <div id="cluster-ui-meta" class="suite-muted" style="margin-bottom:12px"></div>
        <div id="cluster-ui-card"></div>
        <div id="cluster-ui-hits" class="suite-list"></div>
      </div>
    `;

    const form = this.element.querySelector("#cluster-ui-form");
    const input = this.element.querySelector(
      "#cluster-ui-q",
    ) as HTMLInputElement;
    form?.addEventListener("submit", (ev) => {
      ev.preventDefault();
      void this.executeQuery(input.value);
    });
  }

  public async executeQuery(query: string): Promise<void> {
    const qInput = this.element.querySelector(
      "#cluster-ui-q",
    ) as HTMLInputElement | null;
    if (qInput && qInput.value !== query) qInput.value = query;

    const metaEl = this.element.querySelector("#cluster-ui-meta");
    const cardEl = this.element.querySelector("#cluster-ui-card");
    const hitsEl = this.element.querySelector("#cluster-ui-hits");
    if (!metaEl || !cardEl || !hitsEl) return;

    const res = await fetch(`/api/search?q=${encodeURIComponent(query)}`);
    if (!res.ok) return;
    const data = (await res.json()) as {
      tookMs: number;
      rrfConstantK: number;
      routedVia: {
        ingress: { id: string; publicIp: string };
        crawlerReplica: { id: string; publicIp: string };
        googleGrounding: { id: string; publicIp: string };
      };
      knowledgeCard: {
        title: string;
        subtitle: string;
        summary: string;
        subdomain?: string;
      } | null;
      hits: Array<{
        url: string;
        title: string;
        snippet: string;
        domain: string;
        rrfScore: number;
        sources: string[];
        subdomain?: string;
      }>;
    };

    metaEl.textContent = `3-VPS RRF (k=${data.rrfConstantK}) in ${data.tookMs}ms · Edge ${data.routedVia.ingress.id} (${data.routedVia.ingress.publicIp}) + Crawler ${data.routedVia.crawlerReplica.id} (${data.routedVia.crawlerReplica.publicIp}) + Egress ${data.routedVia.googleGrounding.id} (${data.routedVia.googleGrounding.publicIp})`;

    cardEl.innerHTML = data.knowledgeCard
      ? `<div class="suite-card-row" style="border-left:4px solid #0b57d0;margin-bottom:12px">
          <div class="suite-muted">${this.esc(data.knowledgeCard.subtitle)}</div>
          <strong>${this.esc(data.knowledgeCard.title)}</strong>
          <div class="suite-muted">${this.esc(data.knowledgeCard.summary)}</div>
        </div>`
      : "";

    hitsEl.innerHTML = data.hits
      .map(
        (h) => `<article class="suite-card-row">
          <div>
            <a href="${this.esc(h.url)}" data-suite-domain="${this.esc(h.subdomain || "")}" style="color:#0b57d0;font-weight:600;text-decoration:none">${this.esc(h.title)}</a>
            <span class="suite-badge">${this.esc(h.domain)} · RRF ${h.rrfScore}</span>
          </div>
          <div class="suite-muted">${this.esc(h.snippet)}</div>
        </article>`,
      )
      .join("");

    for (const a of hitsEl.querySelectorAll<HTMLAnchorElement>(
      "a[data-suite-domain]",
    )) {
      const sub = a.dataset.suiteDomain || "";
      if (sub && this.onOpenSuite) {
        a.addEventListener("click", (ev) => {
          if (sub.startsWith("craft.")) {
            ev.preventDefault();
            this.onOpenSuite?.("craft");
          } else if (sub.startsWith("office.")) {
            ev.preventDefault();
            this.onOpenSuite?.("docs");
          } else if (sub.startsWith("drive.")) {
            ev.preventDefault();
            this.onOpenSuite?.("drive");
          } else if (sub.startsWith("studio.")) {
            ev.preventDefault();
            this.onOpenSuite?.("studio");
          }
        });
      }
    }
  }

  private esc(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }
}
