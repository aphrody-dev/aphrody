#!/usr/bin/env bun
import { Database } from "bun:sqlite";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

export interface GeminiRoute {
  path: string;
  visibility: "public" | "private" | "external";
  category: string;
  label: string;
  purpose: string;
  http_status?: number;
  source: string;
  updated_at: string;
}

export interface GeminiUiControl {
  surface: string;
  selector: string;
  label: string;
  items_json: string;
  updated_at: string;
}

export interface GeminiLocale {
  hreflang: string;
  href: string;
  updated_at: string;
}

const DEFAULT_DB_PATH = resolve(process.cwd(), "target/gemini-web/index.db");
const DEFAULT_MAP_PATH = resolve(process.cwd(), "target/gemini-web/map.json");
const DEFAULT_DOC_PATH = resolve(
  process.cwd(),
  "docs/aphrody/architecture/gemini-web-sitemap.md",
);

const PRIVATE_WORKSPACE_ROUTES: Omit<GeminiRoute, "updated_at">[] = [
  { path: "/", visibility: "public", category: "core", label: "Nouvelle discussion", purpose: "Root landing / redirects to new chat", source: "browser+sitemap" },
  { path: "/app", visibility: "private", category: "core", label: "Google Gemini", purpose: "Main chat interface with composer, model selector, tools menu, sidebar", source: "browser" },
  { path: "/app/<chat-id>", visibility: "private", category: "core", label: "Chat thread", purpose: "Individual conversation thread", source: "browser" },
  { path: "/app/download", visibility: "public", category: "core", label: "Download", purpose: "App download route (allowed in robots.txt)", source: "robots.txt" },
  { path: "/search", visibility: "private", category: "workspace", label: "Rechercher dans les discussions", purpose: "Search across conversation history", source: "browser" },
  { path: "/library", visibility: "private", category: "workspace", label: "Bibliothèque", purpose: "User library", source: "browser" },
  { path: "/images", visibility: "private", category: "media", label: "Images", purpose: "Generated and saved images gallery", source: "browser" },
  { path: "/videos", visibility: "private", category: "media", label: "Vidéos", purpose: "Generated and saved videos gallery", source: "browser" },
  { path: "/scheduled", visibility: "private", category: "automation", label: "Planifié", purpose: "Scheduled actions and recurring tasks", source: "browser" },
  { path: "/customize", visibility: "private", category: "extensibility", label: "Personnaliser Gemini", purpose: "Apps & Skills hub (Découvrir, Connecteurs, Compétences, Rechercher)", source: "browser" },
  { path: "/notebooks/create", visibility: "private", category: "notebooks", label: "Nouveau notebook", purpose: "Create a new Notebook workspace", source: "browser" },
  { path: "/notebook/<uuid>", visibility: "private", category: "notebooks", label: "Notebook", purpose: "Individual Notebook workspace", source: "browser" },
  { path: "/saved-info", visibility: "private", category: "personalization", label: "Contexte personnel", purpose: "Saved personal context and memory", source: "browser" },
  { path: "/gems/view", visibility: "private", category: "extensibility", label: "Gems", purpose: "Legacy Gems management (migrating to /customize Skills from 17 Nov 2026)", source: "browser" },
  { path: "/usage", visibility: "private", category: "account", label: "Limites d'utilisation", purpose: "Plan tier (ULTRA), current & weekly usage percentage bars and reset times", source: "browser" },
  { path: "/import", visibility: "private", category: "privacy", label: "Importer la mémoire dans Gemini", purpose: "Import external memory into Gemini", source: "browser" },
  { path: "/sharing", visibility: "private", category: "privacy", label: "Vos liens publics", purpose: "Manage public shared conversation links", source: "browser" },
];

const UI_CONTROLS: Omit<GeminiUiControl, "updated_at">[] = [
  {
    surface: "composer_tools",
    selector: 'button[aria-label="Importation et outils"]',
    label: "Importation et outils",
    items_json: JSON.stringify([
      "Importer des fichiers",
      "Ajouter depuis Drive",
      "Plus d'importations",
      "Créer une image",
      "Créer une vidéo",
      "Créer de la musique",
      "Canvas",
      "Deep Research",
      "Apprentissage guidé",
    ]),
  },
  {
    surface: "model_selector",
    selector: 'button[aria-label*="sélecteur de mode"]',
    label: "Sélecteur de mode",
    items_json: JSON.stringify(["Carbon Élevé"]),
  },
  {
    surface: "composer_actions",
    selector: "rich-textarea / composer",
    label: "Composer actions",
    items_json: JSON.stringify([
      "Dicter (^⇧D)",
      "Écouter",
      "Activer les discussions temporaires",
      "Télécharger l'appli",
    ]),
  },
  {
    surface: "settings_menu",
    selector: 'button[aria-label="Paramètres"]',
    label: "Paramètres",
    items_json: JSON.stringify([
      "Limites d'utilisation (/usage)",
      "Gérer l'abonnement",
      "Contexte personnel (/saved-info)",
      "Gems (/gems/view)",
      "Thème",
      "Filigrane multimédia",
      "Importer la mémoire dans Gemini (/import)",
      "Vos liens publics (/sharing)",
      "Envoyer des commentaires",
      "Aide",
    ]),
  },
];

export function openIndexDb(dbPath = DEFAULT_DB_PATH): Database {
  mkdirSync(dirname(dbPath), { recursive: true });
  const db = new Database(dbPath);
  db.exec(`
    PRAGMA journal_mode = WAL;
    CREATE TABLE IF NOT EXISTS routes (
      path TEXT PRIMARY KEY,
      visibility TEXT NOT NULL,
      category TEXT NOT NULL,
      label TEXT NOT NULL,
      purpose TEXT NOT NULL,
      http_status INTEGER,
      source TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS ui_controls (
      surface TEXT PRIMARY KEY,
      selector TEXT NOT NULL,
      label TEXT NOT NULL,
      items_json TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS locales (
      hreflang TEXT PRIMARY KEY,
      href TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS crawl_runs (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      started_at TEXT NOT NULL,
      finished_at TEXT NOT NULL,
      routes_count INTEGER NOT NULL,
      locales_count INTEGER NOT NULL,
      status TEXT NOT NULL,
      error TEXT
    );
  `);
  return db;
}

export async function fetchTextWithFallback(url: string): Promise<{ status: number; text: string }> {
  try {
    const response = await fetch(url, {
      headers: { "User-Agent": "Aphrody-Gemini-Indexer/1.0" },
      redirect: "follow",
    });
    return { status: response.status, text: await response.text() };
  } catch {
    const proc = Bun.spawn(["curl", "-sS", "-L", "-w", "\n%{http_code}", url], {
      stdout: "pipe",
      stderr: "pipe",
    });
    const out = await new Response(proc.stdout).text();
    await proc.exited;
    const lines = out.trimEnd().split("\n");
    const status = Number.parseInt(lines.pop() ?? "0", 10) || 0;
    return { status, text: lines.join("\n") };
  }
}

export function parseSitemapLocales(xml: string, now: string): GeminiLocale[] {
  const locales: GeminiLocale[] = [];
  const regex = /hreflang="([^"]+)"\s+href="([^"]+)"/g;
  for (const match of xml.matchAll(regex)) {
    locales.push({ hreflang: match[1], href: match[2], updated_at: now });
  }
  return locales;
}

export async function syncGeminiIndex(options?: {
  dbPath?: string;
  mapPath?: string;
  docPath?: string;
  offline?: boolean;
}): Promise<{ routes: number; locales: number; controls: number; status: string }> {
  const dbPath = options?.dbPath ?? DEFAULT_DB_PATH;
  const mapPath = options?.mapPath ?? DEFAULT_MAP_PATH;
  const docPath = options?.docPath ?? DEFAULT_DOC_PATH;
  const startedAt = new Date().toISOString();
  const db = openIndexDb(dbPath);

  let locales: GeminiLocale[] = [];
  let statusMap = new Map<string, number>();

  if (!options?.offline) {
    const [robots, sitemap, rootPage, downloadPage] = await Promise.all([
      fetchTextWithFallback("https://gemini.google.com/robots.txt"),
      fetchTextWithFallback("https://gemini.google.com/sitemap.xml"),
      fetchTextWithFallback("https://gemini.google.com/"),
      fetchTextWithFallback("https://gemini.google.com/app/download"),
    ]);
    statusMap.set("/robots.txt", robots.status);
    statusMap.set("/sitemap.xml", sitemap.status);
    statusMap.set("/", rootPage.status);
    statusMap.set("/app/download", downloadPage.status);
    if (sitemap.status === 200) {
      locales = parseSitemapLocales(sitemap.text, startedAt);
    }
  }

  const upsertRoute = db.prepare(`
    INSERT INTO routes (path, visibility, category, label, purpose, http_status, source, updated_at)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(path) DO UPDATE SET
      visibility = excluded.visibility,
      category = excluded.category,
      label = excluded.label,
      purpose = excluded.purpose,
      http_status = COALESCE(excluded.http_status, routes.http_status),
      source = excluded.source,
      updated_at = excluded.updated_at
  `);

  const upsertControl = db.prepare(`
    INSERT INTO ui_controls (surface, selector, label, items_json, updated_at)
    VALUES (?, ?, ?, ?, ?)
    ON CONFLICT(surface) DO UPDATE SET
      selector = excluded.selector,
      label = excluded.label,
      items_json = excluded.items_json,
      updated_at = excluded.updated_at
  `);

  const upsertLocale = db.prepare(`
    INSERT INTO locales (hreflang, href, updated_at)
    VALUES (?, ?, ?)
    ON CONFLICT(hreflang) DO UPDATE SET
      href = excluded.href,
      updated_at = excluded.updated_at
  `);

  const insertRun = db.prepare(`
    INSERT INTO crawl_runs (started_at, finished_at, routes_count, locales_count, status, error)
    VALUES (?, ?, ?, ?, ?, NULL)
  `);

  try {
    db.transaction(() => {
      for (const route of PRIVATE_WORKSPACE_ROUTES) {
        upsertRoute.run(
          route.path,
          route.visibility,
          route.category,
          route.label,
          route.purpose,
          statusMap.get(route.path) ?? null,
          route.source,
          startedAt,
        );
      }
      for (const ctrl of UI_CONTROLS) {
        upsertControl.run(ctrl.surface, ctrl.selector, ctrl.label, ctrl.items_json, startedAt);
      }
      for (const loc of locales) {
        upsertLocale.run(loc.hreflang, loc.href, loc.updated_at);
      }
      insertRun.run(startedAt, new Date().toISOString(), PRIVATE_WORKSPACE_ROUTES.length, locales.length, "ok");
    })();
  } finally {
    upsertRoute.finalize();
    upsertControl.finalize();
    upsertLocale.finalize();
    insertRun.finalize();
  }

  mkdirSync(dirname(mapPath), { recursive: true });
  await Bun.write(
    mapPath,
    JSON.stringify(
      {
        generatedAt: startedAt,
        routes: PRIVATE_WORKSPACE_ROUTES,
        uiControls: UI_CONTROLS.map((c) => ({ ...c, items: JSON.parse(c.items_json) })),
        localesCount: locales.length,
        locales: locales.slice(0, 50),
      },
      null,
      2,
    ),
  );

  mkdirSync(dirname(docPath), { recursive: true });
  const docLines = [
    "# Gemini Web Sitemap & UI Reference",
    "",
    `> Auto-generated by \`tooling/bun/gemini-web-index.ts\` on ${startedAt}.`,
    "",
    "## Routes",
    "",
    "| Path | Visibility | Category | Label | Purpose |",
    "|---|---|---|---|---|",
    ...PRIVATE_WORKSPACE_ROUTES.map(
      (r) => `| \`${r.path}\` | ${r.visibility} | ${r.category} | ${r.label} | ${r.purpose} |`,
    ),
    "",
    "## UI Controls",
    "",
    ...UI_CONTROLS.map(
      (c) => `- **${c.label}** (\`${c.selector}\`): ${(JSON.parse(c.items_json) as string[]).join(", ")}`,
    ),
    "",
    `## Locales (${locales.length} observed in sitemap.xml)`,
    "",
  ];
  await Bun.write(docPath, docLines.join("\n"));

  db.close();
  return {
    routes: PRIVATE_WORKSPACE_ROUTES.length,
    locales: locales.length,
    controls: UI_CONTROLS.length,
    status: "ok",
  };
}

export function queryGeminiIndex(term: string, dbPath = DEFAULT_DB_PATH) {
  const db = openIndexDb(dbPath);
  const like = `%${term}%`;
  const stmt = db.prepare(
    `SELECT path, visibility, category, label, purpose, http_status FROM routes
     WHERE path LIKE ? OR label LIKE ? OR purpose LIKE ? OR category LIKE ?
     ORDER BY path`,
  );
  try {
    return stmt.all(like, like, like, like);
  } finally {
    stmt.finalize();
    db.close();
  }
}

async function watchLoop(intervalMs: number) {
  let backoffMs = 2000;
  while (true) {
    try {
      const res = await syncGeminiIndex();
      console.log(JSON.stringify({ event: "sync", ...res, at: new Date().toISOString() }));
      backoffMs = 2000;
      await Bun.sleep(intervalMs);
    } catch (error) {
      console.error(JSON.stringify({ event: "restart_backoff", backoffMs, error: String(error) }));
      await Bun.sleep(backoffMs);
      backoffMs = Math.min(backoffMs * 2, 60000);
    }
  }
}

if (import.meta.main) {
  const [cmd = "sync", arg] = process.argv.slice(2);
  if (cmd === "sync") {
    const res = await syncGeminiIndex();
    console.log(JSON.stringify(res, null, 2));
  } else if (cmd === "query") {
    console.log(JSON.stringify(queryGeminiIndex(arg ?? ""), null, 2));
  } else if (cmd === "watch") {
    const intervalMs = Number.parseInt(arg ?? "300000", 10) || 300000;
    await watchLoop(intervalMs);
  } else {
    console.error("Usage: bun tooling/bun/gemini-web-index.ts [sync|query <term>|watch <ms>]");
    process.exit(1);
  }
}
