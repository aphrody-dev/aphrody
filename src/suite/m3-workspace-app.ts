// SPDX-License-Identifier: Apache-2.0
/**
 * Sovereign M3 Workspace & Locally Uncensored Studio
 * Unifies Mail, Notes, Calendar (.ics), Contacts (.vcf), Translate, Argon2id Account, and Local Open-Weights Studio.
 */

import { ensureSuiteStyles } from "./suite-styles";

export type M3WorkspaceTab =
  | "mail"
  | "notes"
  | "calendar"
  | "contacts"
  | "translate"
  | "account"
  | "studio";

export class M3WorkspaceApp {
  public readonly element: HTMLDivElement;
  private activeTab: M3WorkspaceTab;

  constructor(initialTab: M3WorkspaceTab = "mail") {
    ensureSuiteStyles();
    this.activeTab = initialTab;
    this.element = document.createElement("div");
    this.element.className = "sovereign-suite-shell";
    this.render();
  }

  private render(): void {
    const tabs: Array<{ id: M3WorkspaceTab; label: string }> = [
      { id: "mail", label: "Mail" },
      { id: "notes", label: "Notes" },
      { id: "calendar", label: "Calendar (.ics)" },
      { id: "contacts", label: "Contacts (.vcf)" },
      { id: "translate", label: "Translate" },
      { id: "studio", label: "Local Studio" },
      { id: "account", label: "Account" },
    ];

    this.element.innerHTML = `
      <div class="suite-toolbar" role="tablist" aria-label="Sovereign M3 Workspace">
        ${tabs
          .map(
            (t) =>
              `<button type="button" role="tab" class="suite-tab-btn ${t.id === this.activeTab ? "active" : ""}" data-tab="${t.id}">${t.label}</button>`,
          )
          .join("")}
      </div>
      <div class="suite-stage" id="m3-ws-stage"></div>
    `;

    for (const btn of this.element.querySelectorAll<HTMLButtonElement>(
      ".suite-tab-btn",
    )) {
      btn.addEventListener("click", () => {
        this.activeTab = (btn.dataset.tab as M3WorkspaceTab) || "mail";
        this.render();
      });
    }

    const stage = this.element.querySelector("#m3-ws-stage") as HTMLElement;
    if (this.activeTab === "mail") void this.mountMail(stage);
    else if (this.activeTab === "notes") void this.mountNotes(stage);
    else if (this.activeTab === "calendar") void this.mountCalendar(stage);
    else if (this.activeTab === "contacts") void this.mountContacts(stage);
    else if (this.activeTab === "translate") this.mountTranslate(stage);
    else if (this.activeTab === "studio") void this.mountStudio(stage);
    else this.mountAccount(stage);
  }

  private async mountMail(host: HTMLElement): Promise<void> {
    const res = await fetch("/api/workspace/mail");
    const data = res.ok
      ? ((await res.json()) as {
          mail: Array<{
            sender: string;
            subject: string;
            body: string;
            sentAt: string;
          }>;
        })
      : { mail: [] };
    host.innerHTML = `
      <div class="suite-panel">
        <h3>Aphrody Mail (Stalwart Sovereign Mesh)</h3>
        <div class="suite-actions" style="margin-bottom:12px">
          <input id="mail-to" class="suite-input" placeholder="recipient@aphrody.com" aria-label="Recipient" />
          <input id="mail-sub" class="suite-input" placeholder="Subject" aria-label="Subject" style="flex:1" />
          <button type="button" id="mail-send-btn" class="suite-btn primary">Send</button>
        </div>
        <div class="suite-list">
          ${data.mail
            .map(
              (m) =>
                `<div class="suite-card-row"><strong>${this.esc(m.subject)}</strong> <span class="suite-badge">${this.esc(m.sender)}</span><div class="suite-muted">${this.esc(m.body)}</div></div>`,
            )
            .join("")}
        </div>
      </div>
    `;
    host
      .querySelector("#mail-send-btn")
      ?.addEventListener("click", async () => {
        const to =
          (host.querySelector("#mail-to") as HTMLInputElement).value ||
          "team@aphrody.com";
        const sub =
          (host.querySelector("#mail-sub") as HTMLInputElement).value ||
          "Status Update";
        await fetch("/api/workspace/mail", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            recipient: to,
            subject: sub,
            body: "Dispatched from Aphrody M3 Mail.",
          }),
        });
        void this.mountMail(host);
      });
  }

  private async mountNotes(host: HTMLElement): Promise<void> {
    const res = await fetch("/api/workspace/notes");
    const data = res.ok
      ? ((await res.json()) as {
          notes: Array<{ title: string; content: string; colorHex: string }>;
        })
      : { notes: [] };
    host.innerHTML = `
      <div class="suite-panel">
        <h3>M3 Notes</h3>
        <div class="suite-actions" style="margin-bottom:12px">
          <input id="note-title" class="suite-input" placeholder="Note title" aria-label="Note title" />
          <input id="note-body" class="suite-input" placeholder="Quick technical note..." aria-label="Note content" style="flex:1" />
          <button type="button" id="note-add-btn" class="suite-btn primary">Add Note</button>
        </div>
        <div class="suite-list">
          ${data.notes
            .map(
              (n) =>
                `<div class="suite-card-row"><strong>${this.esc(n.title)}</strong><div class="suite-muted">${this.esc(n.content)}</div></div>`,
            )
            .join("")}
        </div>
      </div>
    `;
    host.querySelector("#note-add-btn")?.addEventListener("click", async () => {
      const t =
        (host.querySelector("#note-title") as HTMLInputElement).value ||
        "Untitled";
      const c =
        (host.querySelector("#note-body") as HTMLInputElement).value || "";
      await fetch("/api/workspace/notes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title: t, content: c }),
      });
      void this.mountNotes(host);
    });
  }

  private async mountCalendar(host: HTMLElement): Promise<void> {
    const res = await fetch("/api/workspace/events");
    const data = res.ok
      ? ((await res.json()) as {
          events: Array<{
            title: string;
            startsAtIso: string;
            location: string;
          }>;
        })
      : { events: [] };
    host.innerHTML = `
      <div class="suite-panel">
        <div class="suite-actions" style="justify-content:space-between;margin-bottom:12px">
          <h3 style="margin:0">M3 Calendar</h3>
          <a href="/api/workspace/events.ics" download="aphrody-calendar.ics" class="suite-btn primary">Download .ics</a>
        </div>
        <div class="suite-list">
          ${data.events
            .map(
              (ev) =>
                `<div class="suite-card-row"><strong>${this.esc(ev.title)}</strong><div class="suite-muted">${this.esc(ev.startsAtIso)} · ${this.esc(ev.location)}</div></div>`,
            )
            .join("")}
        </div>
      </div>
    `;
  }

  private async mountContacts(host: HTMLElement): Promise<void> {
    const res = await fetch("/api/workspace/contacts");
    const data = res.ok
      ? ((await res.json()) as {
          contacts: Array<{ fullName: string; email: string; role: string }>;
        })
      : { contacts: [] };
    host.innerHTML = `
      <div class="suite-panel">
        <div class="suite-actions" style="justify-content:space-between;margin-bottom:12px">
          <h3 style="margin:0">M3 Contacts</h3>
          <a href="/api/workspace/contacts.vcf" download="aphrody-contacts.vcf" class="suite-btn primary">Download .vcf</a>
        </div>
        <div class="suite-list">
          ${data.contacts
            .map(
              (c) =>
                `<div class="suite-card-row"><strong>${this.esc(c.fullName)}</strong> · <code>${this.esc(c.email)}</code><div class="suite-muted">${this.esc(c.role)}</div></div>`,
            )
            .join("")}
        </div>
      </div>
    `;
  }

  private mountTranslate(host: HTMLElement): void {
    host.innerHTML = `
      <div class="suite-panel">
        <h3>Sovereign Bilingual Technical Translator (FR / EN)</h3>
        <textarea id="tr-src" class="suite-textarea" rows="3" aria-label="Source text">Bienvenue sur le moteur de recherche souverain avec zéro dépendance externe.</textarea>
        <div class="suite-actions">
          <select id="tr-dir" class="suite-input" aria-label="Translation direction">
            <option value="fr-en">Français -> English</option>
            <option value="en-fr">English -> Français</option>
          </select>
          <button type="button" id="tr-btn" class="suite-btn primary">Translate</button>
        </div>
        <pre id="tr-out" class="suite-codeblock"></pre>
      </div>
    `;
    const src = host.querySelector("#tr-src") as HTMLTextAreaElement;
    const dir = host.querySelector("#tr-dir") as HTMLSelectElement;
    const out = host.querySelector("#tr-out") as HTMLElement;
    host.querySelector("#tr-btn")?.addEventListener("click", async () => {
      const [from, to] = dir.value.split("-");
      const res = await fetch("/api/workspace/translate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text: src.value, from, to }),
      });
      if (!res.ok) return;
      const data = (await res.json()) as {
        translated: string;
        matchedTerms: number;
      };
      out.textContent = `${data.translated}\n(${data.matchedTerms} glossary phrases matched)`;
    });
  }

  private async mountStudio(host: HTMLElement): Promise<void> {
    const daemonsRes = await fetch("/api/studio/daemons");
    const daemonsData = daemonsRes.ok
      ? ((await daemonsRes.json()) as {
          daemons: Array<{ label: string; baseUrl: string; online: boolean }>;
        })
      : { daemons: [] };

    host.innerHTML = `
      <div class="suite-panel">
        <h3>Locally Uncensored Studio — Open-Weights Sovereign Router</h3>
        <div class="suite-actions" style="margin-bottom:12px">
          ${daemonsData.daemons
            .map(
              (d) =>
                `<span class="suite-badge">${this.esc(d.label)}: ${d.online ? "LIVE" : "STANDBY"}</span>`,
            )
            .join("")}
        </div>
        <textarea id="studio-prompt" class="suite-textarea" rows="3" aria-label="Prompt">Summarize the Aphrody 3-VPS WireGuard search cluster architecture.</textarea>
        <div class="suite-actions">
          <button type="button" id="studio-run-btn" class="suite-btn primary">Execute Local Synthesis</button>
        </div>
        <pre id="studio-out" class="suite-codeblock">Ready</pre>
      </div>
    `;
    const promptEl = host.querySelector(
      "#studio-prompt",
    ) as HTMLTextAreaElement;
    const outEl = host.querySelector("#studio-out") as HTMLElement;
    host
      .querySelector("#studio-run-btn")
      ?.addEventListener("click", async () => {
        outEl.textContent = "Routing across local open-weights endpoints...";
        const res = await fetch("/api/studio/complete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ prompt: promptEl.value }),
        });
        if (!res.ok) return;
        const data = (await res.json()) as {
          provider: string;
          tookMs: number;
          output: string;
        };
        outEl.textContent = `[Provider: ${data.provider} · ${data.tookMs}ms]\n\n${data.output}`;
      });
  }

  private mountAccount(host: HTMLElement): void {
    host.innerHTML = `
      <div class="suite-panel">
        <h3>Sovereign Identity — Native Bun.password Argon2id</h3>
        <div class="suite-actions">
          <input id="acc-email" type="email" class="suite-input" value="contact@aphrody.com" aria-label="Email" />
          <input id="acc-pass" type="password" class="suite-input" value="sovereign-mesh-2026" aria-label="Passphrase" />
          <button type="button" id="acc-btn" class="suite-btn primary">Authenticate / Register</button>
        </div>
        <p id="acc-status" class="suite-status">Argon2id memory-hard verification</p>
      </div>
    `;
    const emailEl = host.querySelector("#acc-email") as HTMLInputElement;
    const passEl = host.querySelector("#acc-pass") as HTMLInputElement;
    const statusEl = host.querySelector("#acc-status") as HTMLElement;
    host.querySelector("#acc-btn")?.addEventListener("click", async () => {
      const res = await fetch("/api/workspace/account", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: emailEl.value, password: passEl.value }),
      });
      if (!res.ok) return;
      const data = (await res.json()) as {
        authenticated: boolean;
        mode: string;
        email: string;
      };
      statusEl.textContent = data.authenticated
        ? `Authenticated (${data.mode}) as ${data.email}`
        : "Invalid Argon2id credentials";
    });
  }

  private esc(s: string): string {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }
}
