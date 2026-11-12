// SPDX-License-Identifier: Apache-2.0
/**
 * Spacedrive VDFS Client (sd:// Virtual Distributed File System)
 * Content-addressed Blake2b/Blake3 storage across eu-fr1-nvme, eu-fr2-replica, and creative-vault.
 */

import { ensureSuiteStyles } from "./suite-styles";

export class SpacedriveApp {
  public readonly element: HTMLDivElement;
  private locationFilter = "all";
  private searchQuery = "";

  constructor() {
    ensureSuiteStyles();
    this.element = document.createElement("div");
    this.element.className = "sovereign-suite-shell";
    this.renderShell();
    void this.refreshFiles();
  }

  private renderShell(): void {
    this.element.innerHTML = `
      <div class="suite-toolbar">
        <button type="button" class="suite-tab-btn active" data-loc="all">All Locations</button>
        <button type="button" class="suite-tab-btn" data-loc="sd://eu-fr1-nvme">sd://eu-fr1-nvme</button>
        <button type="button" class="suite-tab-btn" data-loc="sd://eu-fr2-replica">sd://eu-fr2-replica</button>
        <button type="button" class="suite-tab-btn" data-loc="sd://creative-vault">sd://creative-vault</button>
      </div>
      <div class="suite-stage">
        <div class="suite-actions" style="margin-bottom:12px">
          <input id="sd-search" type="search" class="suite-input" placeholder="Search VDFS files or tags..." aria-label="Search Spacedrive" style="flex:1" />
          <input id="sd-new-name" type="text" class="suite-input" placeholder="new-note.md" aria-label="New file name" style="width:160px" />
          <button type="button" id="sd-create-btn" class="suite-btn primary">Index File</button>
        </div>
        <div id="sd-list" class="suite-list"></div>
      </div>
    `;

    for (const btn of this.element.querySelectorAll<HTMLButtonElement>(
      "[data-loc]",
    )) {
      btn.addEventListener("click", () => {
        for (const b of this.element.querySelectorAll("[data-loc]"))
          b.classList.remove("active");
        btn.classList.add("active");
        this.locationFilter = btn.dataset.loc || "all";
        void this.refreshFiles();
      });
    }

    const searchIn = this.element.querySelector(
      "#sd-search",
    ) as HTMLInputElement;
    searchIn.addEventListener("input", () => {
      this.searchQuery = searchIn.value;
      void this.refreshFiles();
    });

    const newNameIn = this.element.querySelector(
      "#sd-new-name",
    ) as HTMLInputElement;
    this.element
      .querySelector("#sd-create-btn")
      ?.addEventListener("click", async () => {
        const name = newNameIn.value.trim() || `note-${Date.now()}.md`;
        await fetch("/api/spacedrive/files", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            location:
              this.locationFilter === "all"
                ? "sd://eu-fr1-nvme"
                : this.locationFilter,
            virtualPath: `/Workspace/${name}`,
            name,
            mimeType: "text/markdown",
            tags: ["workspace", "cas"],
            content: `# ${name}\nIndexed via Spacedrive VDFS CAS on Pure Bun v1.4.4.`,
          }),
        });
        newNameIn.value = "";
        void this.refreshFiles();
      });
  }

  private async refreshFiles(): Promise<void> {
    const listEl = this.element.querySelector("#sd-list");
    if (!listEl) return;
    const url = `/api/spacedrive/files?q=${encodeURIComponent(this.searchQuery)}&location=${encodeURIComponent(this.locationFilter)}`;
    const res = await fetch(url);
    if (!res.ok) return;
    const data = (await res.json()) as {
      files: Array<{
        location: string;
        virtualPath: string;
        name: string;
        sizeBytes: number;
        blake3Hash: string;
        tags: string[];
        previewText: string;
      }>;
    };

    listEl.innerHTML = data.files
      .map(
        (f) => `<div class="suite-card-row">
          <div>
            <strong>${this.esc(f.name)}</strong>
            <span class="suite-badge">${this.esc(f.location)}</span>
            <div class="suite-muted">${this.esc(f.virtualPath)} · ${f.sizeBytes} B · CAS <code>${this.esc(f.blake3Hash.slice(0, 14))}</code></div>
            <div class="suite-muted">${this.esc(f.previewText)}</div>
          </div>
        </div>`,
      )
      .join("");
  }

  private esc(s: string): string {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }
}
