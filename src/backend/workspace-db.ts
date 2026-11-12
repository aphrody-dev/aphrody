// SPDX-License-Identifier: Apache-2.0
/**
 * Sovereign Workspace Database (bun:sqlite WAL)
 * Powers Spacedrive VDFS (sd:// content-addressed storage), Fluent 2 Office documents,
 * M3 Mail/Notes/Calendar/Contacts/Photos, and Argon2id Sovereign Accounts.
 */
import { Database } from "bun:sqlite";
import { mkdirSync } from "node:fs";
import { dirname } from "node:path";
import type { CalendarEventSpec, ContactCardSpec } from "./docx-ics-vcf.ts";

export interface SpacedriveEntry {
  id: string;
  location: string;
  virtualPath: string;
  name: string;
  mimeType: string;
  sizeBytes: number;
  blake3Hash: string;
  tags: string[];
  previewText: string;
  updatedAt: number;
}

export interface WorkspaceMailItem {
  id: string;
  folder: "inbox" | "sent" | "archive";
  sender: string;
  recipient: string;
  subject: string;
  body: string;
  sentAt: string;
}

export interface WorkspaceNoteItem {
  id: string;
  title: string;
  content: string;
  colorHex: string;
  pinned: boolean;
  updatedAt: number;
}

export class SovereignWorkspaceDb {
  private static shared: SovereignWorkspaceDb | null = null;
  readonly db: Database;

  static getInstance(dbPath = "var/workspace.sqlite"): SovereignWorkspaceDb {
    if (!SovereignWorkspaceDb.shared) {
      SovereignWorkspaceDb.shared = new SovereignWorkspaceDb(dbPath);
    }
    return SovereignWorkspaceDb.shared;
  }

  constructor(dbPath = ":memory:") {
    if (dbPath !== ":memory:") {
      mkdirSync(dirname(dbPath), { recursive: true });
    }
    this.db = new Database(dbPath, { create: true });
    this.db.exec("PRAGMA journal_mode = WAL;");
    this.db.exec("PRAGMA synchronous = NORMAL;");
    this.initSchema();
    this.seedIfEmpty();
  }

  private initSchema(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS sd_files (
        id TEXT PRIMARY KEY,
        location TEXT NOT NULL,
        virtual_path TEXT NOT NULL,
        name TEXT NOT NULL,
        mime_type TEXT NOT NULL,
        size_bytes INTEGER NOT NULL,
        blake3_hash TEXT NOT NULL,
        tags_json TEXT NOT NULL,
        preview_text TEXT NOT NULL,
        updated_at INTEGER NOT NULL
      );

      CREATE TABLE IF NOT EXISTS office_docs (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,
        title TEXT NOT NULL,
        payload_json TEXT NOT NULL,
        updated_at INTEGER NOT NULL
      );

      CREATE TABLE IF NOT EXISTS m3_mail (
        id TEXT PRIMARY KEY,
        folder TEXT NOT NULL,
        sender TEXT NOT NULL,
        recipient TEXT NOT NULL,
        subject TEXT NOT NULL,
        body TEXT NOT NULL,
        sent_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS m3_notes (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        content TEXT NOT NULL,
        color_hex TEXT NOT NULL,
        pinned INTEGER NOT NULL DEFAULT 0,
        updated_at INTEGER NOT NULL
      );

      CREATE TABLE IF NOT EXISTS m3_events (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        starts_at TEXT NOT NULL,
        ends_at TEXT NOT NULL,
        location TEXT NOT NULL,
        description TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS m3_contacts (
        id TEXT PRIMARY KEY,
        full_name TEXT NOT NULL,
        email TEXT NOT NULL,
        organization TEXT NOT NULL,
        role TEXT NOT NULL,
        phone TEXT NOT NULL DEFAULT ''
      );

      CREATE TABLE IF NOT EXISTS m3_accounts (
        email TEXT PRIMARY KEY,
        display_name TEXT NOT NULL,
        password_hash TEXT NOT NULL,
        created_at INTEGER NOT NULL
      );
    `);
  }

  private hashContent(text: string): string {
    return new Bun.CryptoHasher("blake2b256").update(text).digest("hex");
  }

  private seedIfEmpty(): void {
    const countRow = this.db
      .query<{ c: number }, []>("SELECT COUNT(*) AS c FROM sd_files")
      .get();
    if ((countRow?.c ?? 0) > 0) return;

    const now = Date.now();
    const seedFiles: Array<Omit<SpacedriveEntry, "blake3Hash" | "updatedAt">> =
      [
        {
          id: "sd-01",
          location: "sd://eu-fr1-nvme",
          virtualPath: "/Architecture/deploy-3vps-topology.toml",
          name: "deploy-3vps-topology.toml",
          mimeType: "application/toml",
          sizeBytes: 4280,
          tags: ["infra", "wireguard", "ovh"],
          previewText:
            "3-VPS WireGuard 10.200.77.0/24 mesh: fr1 (51.77.147.152), fr2 (51.255.162.6), usa (72.60.186.43).",
        },
        {
          id: "sd-02",
          location: "sd://creative-vault",
          virtualPath: "/Design/aphrody-pet-codex-v2.json",
          name: "aphrody-pet-codex-v2.json",
          mimeType: "application/json",
          sizeBytes: 15360,
          tags: ["design", "pets", "m3"],
          previewText:
            "Lossless 74-cell RGBA spritesheet manifest with 16-way cursor gaze and 966-tick parity.",
        },
        {
          id: "sd-03",
          location: "sd://eu-fr2-replica",
          virtualPath: "/Office/q4-compute-budget.grid.json",
          name: "q4-compute-budget.grid.json",
          mimeType: "application/json",
          sizeBytes: 2190,
          tags: ["office", "finance", "gridcraft"],
          previewText:
            "OVH fr1 + fr2 + Hostinger usa monthly infrastructure ledger with deterministic SUM/NPV formulas.",
        },
        {
          id: "sd-04",
          location: "sd://creative-vault",
          virtualPath: "/Shaders/raymarch-sdf-sm89.wgsl",
          name: "raymarch-sdf-sm89.wgsl",
          mimeType: "text/wgsl",
          sizeBytes: 3120,
          tags: ["webgpu", "wgsl", "3d"],
          previewText:
            "@fragment fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32>",
        },
      ];

    for (const f of seedFiles) {
      this.upsertSpacedriveFile({
        ...f,
        content: f.previewText,
      });
    }

    // Seed Office docs
    this.saveOfficeDoc(
      "doc-spec-01",
      "word",
      "Aphrody Sovereign Architecture Charter",
      {
        body: "1. Zero External Dependencies on Pure Bun v1.4.4 Full-Rust.\n2. End-to-End 3-VPS Search Cluster (fr1 + fr2 + usa).\n3. Unified Craft Creative Cloud + Fluent 2 Office + Spacedrive VDFS.",
      },
    );
    this.saveOfficeDoc(
      "grid-budget-01",
      "grid",
      "Cluster Datacenter Capacity & Cost Ledger",
      {
        cells: {
          A1: "fr1 OVH Edge",
          B1: "48",
          A2: "fr2 OVH Chromium",
          B2: "64",
          A3: "usa Google Egress",
          B3: "24",
          A4: "Total EUR/mo",
          B4: "=SUM(B1:B3)",
          A5: "Mean Node EUR",
          B5: "=ROUND(AVG(B1:B3), 2)",
        },
      },
    );

    // Seed Mail
    const insertMail = this.db.query(
      "INSERT INTO m3_mail (id, folder, sender, recipient, subject, body, sent_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    );
    insertMail.run(
      "mail-01",
      "inbox",
      "infra@aphrody.com",
      "contact@aphrody.com",
      "3-VPS WireGuard Mesh Health Verified (10.200.77.0/24)",
      "All shards online: fr1 FTS5 Shard-0, fr2 Headless Chromium Crawler Shard-1, and usa strict Google Grounding Egress.",
      "2026-11-11T09:00:00Z",
    );
    insertMail.run(
      "mail-02",
      "inbox",
      "design@aphrody.com",
      "contact@aphrody.com",
      "Material Design 3 HCT & Aphrody Pet Spritesheet Ready",
      "74-cell lossless WebP/PNG spritesheets and optimized Material Symbols virtual bundler plugins passed 100% parity.",
      "2026-11-11T10:30:00Z",
    );

    // Seed Notes
    const insertNote = this.db.query(
      "INSERT INTO m3_notes (id, title, content, color_hex, pinned, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    );
    insertNote.run(
      "note-01",
      "WCK Critical Path Budget",
      "Keep index.html.gz + style.css.gz <= 14 KB and every lazy JS chunk <= 30 KB gzip.",
      "#D3E3FD",
      1,
      now,
    );
    insertNote.run(
      "note-02",
      "Egress Pinning Policy",
      "Node usa (72.60.186.43) runs strictly Google Search & Gemini grounding; all app & VDFS workloads run on fr1 + fr2.",
      "#C4EED0",
      1,
      now,
    );

    // Seed Calendar
    const insertEv = this.db.query(
      "INSERT INTO m3_events (id, title, starts_at, ends_at, location, description) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    );
    insertEv.run(
      "ev-01",
      "Blue/Green Cluster Rollout (fr1 + fr2)",
      "2026-11-12T14:00:00Z",
      "2026-11-12T15:00:00Z",
      "WireGuard Mesh 10.200.77.1",
      "Automated zero-downtime health-checked cutover via src-deploy.",
    );

    // Seed Contacts
    const insertCt = this.db.query(
      "INSERT INTO m3_contacts (id, full_name, email, organization, role, phone) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    );
    insertCt.run(
      "ct-01",
      "Aphrody Organization",
      "contact@aphrody.com",
      "Aphrody Sovereign Engineering",
      "Primary Infrastructure & Design Identity",
      "+33 1 86 95 00 00",
    );
  }

  listSpacedriveFiles(query = "", location = "all"): SpacedriveEntry[] {
    const q = query.trim().toLowerCase();
    const rows = this.db
      .query<
        {
          id: string;
          location: string;
          virtual_path: string;
          name: string;
          mime_type: string;
          size_bytes: number;
          blake3_hash: string;
          tags_json: string;
          preview_text: string;
          updated_at: number;
        },
        []
      >("SELECT * FROM sd_files ORDER BY updated_at DESC")
      .all();

    return rows
      .map((r) => ({
        id: r.id,
        location: r.location,
        virtualPath: r.virtual_path,
        name: r.name,
        mimeType: r.mime_type,
        sizeBytes: r.size_bytes,
        blake3Hash: r.blake3_hash,
        tags: JSON.parse(r.tags_json) as string[],
        previewText: r.preview_text,
        updatedAt: r.updated_at,
      }))
      .filter((item) => {
        if (location !== "all" && item.location !== location) return false;
        if (!q) return true;
        return (
          item.name.toLowerCase().includes(q) ||
          item.virtualPath.toLowerCase().includes(q) ||
          item.tags.some((t) => t.toLowerCase().includes(q)) ||
          item.previewText.toLowerCase().includes(q)
        );
      });
  }

  upsertSpacedriveFile(input: {
    id?: string;
    location: string;
    virtualPath: string;
    name: string;
    mimeType: string;
    sizeBytes?: number;
    tags?: string[];
    content: string;
  }): SpacedriveEntry {
    const id = input.id ?? `sd-${Date.now().toString(36)}`;
    const blake3Hash = this.hashContent(input.content);
    const sizeBytes =
      input.sizeBytes ?? new TextEncoder().encode(input.content).byteLength;
    const tags = input.tags ?? ["workspace"];
    const previewText = input.content.slice(0, 320);
    const updatedAt = Date.now();

    this.db
      .query(
        `INSERT INTO sd_files (id, location, virtual_path, name, mime_type, size_bytes, blake3_hash, tags_json, preview_text, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
           location = excluded.location,
           virtual_path = excluded.virtual_path,
           name = excluded.name,
           mime_type = excluded.mime_type,
           size_bytes = excluded.size_bytes,
           blake3_hash = excluded.blake3_hash,
           tags_json = excluded.tags_json,
           preview_text = excluded.preview_text,
           updated_at = excluded.updated_at`,
      )
      .run(
        id,
        input.location,
        input.virtualPath,
        input.name,
        input.mimeType,
        sizeBytes,
        blake3Hash,
        JSON.stringify(tags),
        previewText,
        updatedAt,
      );

    return {
      id,
      location: input.location,
      virtualPath: input.virtualPath,
      name: input.name,
      mimeType: input.mimeType,
      sizeBytes,
      blake3Hash,
      tags,
      previewText,
      updatedAt,
    };
  }

  saveOfficeDoc(
    id: string,
    kind: "word" | "grid" | "deck",
    title: string,
    payload: unknown,
  ): void {
    this.db
      .query(
        `INSERT INTO office_docs (id, kind, title, payload_json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET title = excluded.title, payload_json = excluded.payload_json, updated_at = excluded.updated_at`,
      )
      .run(id, kind, title, JSON.stringify(payload), Date.now());
  }

  listOfficeDocs(kind?: string): Array<{
    id: string;
    kind: string;
    title: string;
    payload: unknown;
    updatedAt: number;
  }> {
    const rows = kind
      ? this.db
          .query<
            {
              id: string;
              kind: string;
              title: string;
              payload_json: string;
              updated_at: number;
            },
            [string]
          >(
            "SELECT * FROM office_docs WHERE kind = ?1 ORDER BY updated_at DESC",
          )
          .all(kind)
      : this.db
          .query<
            {
              id: string;
              kind: string;
              title: string;
              payload_json: string;
              updated_at: number;
            },
            []
          >("SELECT * FROM office_docs ORDER BY updated_at DESC")
          .all();

    return rows.map((r) => ({
      id: r.id,
      kind: r.kind,
      title: r.title,
      payload: JSON.parse(r.payload_json),
      updatedAt: r.updated_at,
    }));
  }

  listMail(): WorkspaceMailItem[] {
    return this.db
      .query<
        {
          id: string;
          folder: "inbox" | "sent" | "archive";
          sender: string;
          recipient: string;
          subject: string;
          body: string;
          sent_at: string;
        },
        []
      >("SELECT * FROM m3_mail ORDER BY sent_at DESC")
      .all()
      .map((r) => ({
        id: r.id,
        folder: r.folder,
        sender: r.sender,
        recipient: r.recipient,
        subject: r.subject,
        body: r.body,
        sentAt: r.sent_at,
      }));
  }

  sendMail(
    recipient: string,
    subject: string,
    body: string,
  ): WorkspaceMailItem {
    const item: WorkspaceMailItem = {
      id: `mail-${Date.now().toString(36)}`,
      folder: "sent",
      sender: "contact@aphrody.com",
      recipient,
      subject,
      body,
      sentAt: new Date().toISOString(),
    };
    this.db
      .query(
        "INSERT INTO m3_mail (id, folder, sender, recipient, subject, body, sent_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
      )
      .run(
        item.id,
        item.folder,
        item.sender,
        item.recipient,
        item.subject,
        item.body,
        item.sentAt,
      );
    return item;
  }

  listNotes(): WorkspaceNoteItem[] {
    return this.db
      .query<
        {
          id: string;
          title: string;
          content: string;
          color_hex: string;
          pinned: number;
          updated_at: number;
        },
        []
      >("SELECT * FROM m3_notes ORDER BY pinned DESC, updated_at DESC")
      .all()
      .map((r) => ({
        id: r.id,
        title: r.title,
        content: r.content,
        colorHex: r.color_hex,
        pinned: r.pinned === 1,
        updatedAt: r.updated_at,
      }));
  }

  addNote(
    title: string,
    content: string,
    colorHex = "#D3E3FD",
  ): WorkspaceNoteItem {
    const item: WorkspaceNoteItem = {
      id: `note-${Date.now().toString(36)}`,
      title,
      content,
      colorHex,
      pinned: false,
      updatedAt: Date.now(),
    };
    this.db
      .query(
        "INSERT INTO m3_notes (id, title, content, color_hex, pinned, updated_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)",
      )
      .run(item.id, item.title, item.content, item.colorHex, item.updatedAt);
    return item;
  }

  listEvents(): CalendarEventSpec[] {
    return this.db
      .query<
        {
          id: string;
          title: string;
          starts_at: string;
          ends_at: string;
          location: string;
          description: string;
        },
        []
      >("SELECT * FROM m3_events ORDER BY starts_at ASC")
      .all()
      .map((r) => ({
        id: r.id,
        title: r.title,
        startsAtIso: r.starts_at,
        endsAtIso: r.ends_at,
        location: r.location,
        description: r.description,
      }));
  }

  listContacts(): ContactCardSpec[] {
    return this.db
      .query<
        {
          id: string;
          full_name: string;
          email: string;
          organization: string;
          role: string;
          phone: string;
        },
        []
      >("SELECT * FROM m3_contacts ORDER BY full_name ASC")
      .all()
      .map((r) => ({
        id: r.id,
        fullName: r.full_name,
        email: r.email,
        organization: r.organization,
        role: r.role,
        phone: r.phone || undefined,
      }));
  }

  async registerOrVerifyAccount(
    email: string,
    password: string,
    displayName = "Aphrody Member",
  ): Promise<{
    authenticated: boolean;
    email: string;
    displayName: string;
    mode: "registered" | "verified";
  }> {
    const cleanEmail = email.trim().toLowerCase();
    const existing = this.db
      .query<
        { email: string; display_name: string; password_hash: string },
        [string]
      >(
        "SELECT email, display_name, password_hash FROM m3_accounts WHERE email = ?1",
      )
      .get(cleanEmail);

    if (!existing) {
      const hash = await Bun.password.hash(password, {
        algorithm: "argon2id",
        memoryCost: 4096,
        timeCost: 1,
      });
      this.db
        .query(
          "INSERT INTO m3_accounts (email, display_name, password_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
        )
        .run(cleanEmail, displayName, hash, Date.now());
      return {
        authenticated: true,
        email: cleanEmail,
        displayName,
        mode: "registered",
      };
    }

    const valid = await Bun.password.verify(password, existing.password_hash);
    return {
      authenticated: valid,
      email: existing.email,
      displayName: existing.display_name,
      mode: "verified",
    };
  }

  close(): void {
    this.db.close();
  }
}
