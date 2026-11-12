import { serve } from "bun";
import { existsSync, readFileSync } from "fs";
import { dirname, join } from "path";
import { getSuggestions } from "./src/data/suggestions";
import { UniversalStoreManager } from "./src/store/universal-manager";
import { runBenchmarkSuite } from "./src/benchmark/stack-picker";
import { AssetStore, respondWithAsset } from "./src/runtime/assets";
import { getHashEngine } from "./src/runtime/hash";
import {
  buildCsp,
  readGoogleConfig,
  type GoogleConfig,
} from "./src/google/config";
import { handleCollect } from "./src/google/relay";
import { handleGeminiSearch } from "./src/gemini/search";
import { indexNowKey, indexNowPayload } from "./src/seo/documents";
import { render404, renderSearchResults } from "./src/seo/html";
import type { Locale } from "./src/seo/site";
import {
  buildDocxArchive,
  buildIcsCalendar,
  buildVcfContacts,
  evaluateSpreadsheet,
  translateSovereignText,
} from "./src/backend/docx-ics-vcf";
import {
  getSharedSearchCluster,
  probeLocalDaemons,
  runLocalUncensoredCompletion,
} from "./src/backend/local-studio";
import {
  applyWasmRgbaFilter,
  assembleMiniWatToWasm,
  optimizeVectorSvg,
  synthesizePcmWav,
  type PhotoFilterMode,
} from "./src/backend/wasm-creative";
import { SovereignWorkspaceDb } from "./src/backend/workspace-db";

const PORT = Number(process.env.PORT) || 3000;

function resolveDist(): string {
  if (process.env.APHRODY_DIST) return process.env.APHRODY_DIST;
  const candidates = [
    join(import.meta.dir, "dist"),
    join(dirname(process.execPath), "dist"),
    join(process.cwd(), "dist"),
  ];
  return (
    candidates.find((c) => existsSync(join(c, "index.html"))) || candidates[0]
  );
}

const DIST_DIR = resolveDist();
const storeManager = UniversalStoreManager.getInstance("var/store.sqlite");
const workspaceDb = SovereignWorkspaceDb.getInstance("var/workspace.sqlite");
const assets = new AssetStore(DIST_DIR);

/** Google ids: runtime env first, else the config baked into dist/ by the build. */
function loadGoogleConfig(): GoogleConfig {
  const fromEnv = readGoogleConfig(process.env);
  if (Object.keys(fromEnv).length) return fromEnv;
  try {
    return readGoogleConfig(
      JSON.parse(readFileSync(join(DIST_DIR, "g-cfg.json"), "utf-8")) as never,
    );
  } catch {
    return {};
  }
}
const GOOGLE_CFG = loadGoogleConfig();
const INDEXNOW_KEY = indexNowKey(process.env);

const SECURITY_HEADERS: Record<string, string> = {
  "Content-Security-Policy": buildCsp(GOOGLE_CFG),
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "SAMEORIGIN",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Permissions-Policy": "microphone=(self)",
  "Strict-Transport-Security": "max-age=63072000; includeSubDomains; preload",
  "X-XSS-Protection": "1; mode=block",
};

/** CDNs (for example Cloudflare) can turn these Link headers into 103 Early Hints. */
const LINK_HOME =
  "</style.css>; rel=preload; as=style, </app.js>; rel=modulepreload, </boot.js>; rel=modulepreload";
const LINK_PAGE =
  "</style.css>; rel=preload; as=style, </boot.js>; rel=modulepreload";

function jsonResponse(data: unknown, status = 200, req?: Request): Response {
  const payload = JSON.stringify(data);
  const headers: Record<string, string> = {
    "Content-Type": "application/json; charset=utf-8",
    "Cache-Control": "no-cache",
    ...SECURITY_HEADERS,
  };
  if (req && (req.headers.get("accept-encoding") || "").includes("gzip")) {
    headers["Content-Encoding"] = "gzip";
    return new Response(Bun.gzipSync(Buffer.from(payload)), {
      status,
      headers,
    });
  }
  return new Response(payload, { status, headers });
}

function withSecurity(res: Response): Response {
  for (const [k, v] of Object.entries(SECURITY_HEADERS))
    if (!res.headers.has(k)) res.headers.set(k, v);
  return res;
}

function localeOf(req: Request, url: URL): Locale {
  const hl = url.searchParams.get("hl");
  if (hl === "en" || hl === "fr") return hl;
  if (url.pathname.startsWith("/en/") || url.pathname === "/en") return "en";
  return (req.headers.get("accept-language") || "")
    .toLowerCase()
    .startsWith("en")
    ? "en"
    : "fr";
}

function clientId(
  req: Request,
  srv?: { requestIP?: (r: Request) => { address: string } | null },
): string {
  const fwd =
    req.headers.get("cf-connecting-ip") ||
    (req.headers.get("x-forwarded-for") || "").split(",")[0].trim();
  return fwd || srv?.requestIP?.(req)?.address || "local";
}

function htmlHeaders(pathname: string): Record<string, string> {
  return {
    Link: pathname === "/" || pathname === "/en" ? LINK_HOME : LINK_PAGE,
    ...SECURITY_HEADERS,
  };
}

export const server = serve({
  port: PORT,
  async fetch(
    req: Request,
    srv?: { requestIP?: (r: Request) => { address: string } | null },
  ) {
    const url = new URL(req.url);
    const pathname = url.pathname;

    // 1. API: Autocomplete Suggestions (short: /api/q, legacy: /api/complete)
    if (pathname === "/api/q" || pathname === "/api/complete") {
      const q = url.searchParams.get("q") || "";
      const suggestions = getSuggestions(q);
      return jsonResponse({ query: q, suggestions }, 200, req);
    }

    // 1b. API: 3-VPS Datacenter Search Cluster (RRF k=60 across fr1 + fr2 + usa)
    if (pathname === "/api/search") {
      const q = url.searchParams.get("q") || "";
      const cat = url.searchParams.get("category") || "all";
      const cluster = getSharedSearchCluster();
      const result = await cluster.search(q, {
        category: cat,
        useUsaGrounding: true,
      });
      return jsonResponse(result, 200, req);
    }

    if (pathname === "/api/cluster/status") {
      const probe = url.searchParams.get("probe") === "1";
      const cluster = getSharedSearchCluster();
      const status = await cluster.probeClusterHealth(probe);
      return jsonResponse(status, 200, req);
    }

    if (pathname === "/api/crawl" && req.method === "POST") {
      try {
        const body = (await req.json()) as { url?: string; chromium?: boolean };
        if (!body.url) return jsonResponse({ error: "missing url" }, 400, req);
        const cluster = getSharedSearchCluster();
        const crawled = await cluster.crawlAndIndexOnFr2(body.url, {
          renderWithChromium: Boolean(body.chromium),
        });
        return jsonResponse(crawled, 200, req);
      } catch (err: any) {
        return jsonResponse({ error: String(err?.message || err) }, 400, req);
      }
    }

    // 1c. API: Craft Creative Cloud (WASM RGBA, SVG Optimizer, PCM16 WAV, WAT->WASM)
    if (pathname === "/api/craft/rgba" && req.method === "POST") {
      try {
        const body = (await req.json()) as {
          rgbaBase64?: string;
          mode?: PhotoFilterMode;
        };
        const raw = Uint8Array.fromBase64(body.rgbaBase64 || "");
        const filtered = applyWasmRgbaFilter(raw, body.mode || "invert");
        return jsonResponse(
          {
            mode: body.mode || "invert",
            kernel: "wasm-linear-memory-v1",
            rgbaBase64: Buffer.from(filtered).toString("base64"),
          },
          200,
          req,
        );
      } catch (err: any) {
        return jsonResponse({ error: String(err?.message || err) }, 400, req);
      }
    }

    if (pathname === "/api/craft/svg-optimize" && req.method === "POST") {
      const body = (await req.json()) as { svg?: string };
      return jsonResponse(optimizeVectorSvg(body.svg || ""), 200, req);
    }

    if (pathname === "/api/craft/wav" && req.method === "GET") {
      const freq = Number(url.searchParams.get("freq")) || 440;
      const ms = Number(url.searchParams.get("ms")) || 320;
      const wave = (url.searchParams.get("wave") || "harmonic") as
        "sine" | "triangle" | "harmonic";
      const wav = synthesizePcmWav(freq, ms, wave);
      return new Response(new Uint8Array(wav), {
        headers: {
          "Content-Type": "audio/wav",
          "Cache-Control": "no-store",
          ...SECURITY_HEADERS,
        },
      });
    }

    if (pathname === "/api/craft/wat" && req.method === "POST") {
      try {
        const body = (await req.json()) as {
          wat?: string;
          args?: [number, number];
        };
        const compiled = assembleMiniWatToWasm(
          body.wat || "",
          body.args || [6, 7],
        );
        return jsonResponse(
          {
            exportName: compiled.exportName,
            result: compiled.result,
            byteLength: compiled.wasmBytes.byteLength,
            hexPreview: compiled.hexPreview,
          },
          200,
          req,
        );
      } catch (err: any) {
        return jsonResponse({ error: String(err?.message || err) }, 400, req);
      }
    }

    // 1d. API: Fluent 2 Office Suite (WordCraft .docx, GridCraft Zero-Eval Formulas, SQLite Docs)
    if (pathname === "/api/office/docs") {
      if (req.method === "POST") {
        const body = (await req.json()) as {
          id?: string;
          kind?: "word" | "grid" | "deck";
          title?: string;
          payload?: unknown;
        };
        workspaceDb.saveOfficeDoc(
          body.id || `doc-${Date.now()}`,
          body.kind || "word",
          body.title || "Untitled",
          body.payload ?? {},
        );
        return jsonResponse({ saved: true }, 200, req);
      }
      const kind = url.searchParams.get("kind") || undefined;
      return jsonResponse({ docs: workspaceDb.listOfficeDocs(kind) }, 200, req);
    }

    if (pathname === "/api/office/docx" && req.method === "POST") {
      const body = (await req.json()) as { title?: string; body?: string };
      const zip = buildDocxArchive(
        body.title || "Aphrody Document",
        body.body || "",
      );
      return new Response(new Uint8Array(zip), {
        headers: {
          "Content-Type":
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
          "Content-Disposition": 'attachment; filename="aphrody-document.docx"',
          ...SECURITY_HEADERS,
        },
      });
    }

    if (pathname === "/api/office/formula" && req.method === "POST") {
      const body = (await req.json()) as { cells?: Record<string, string> };
      const evaluated = evaluateSpreadsheet(body.cells || {});
      return jsonResponse({ evaluated }, 200, req);
    }

    // 1e. API: Spacedrive VDFS (sd:// Blake3 CAS)
    if (pathname === "/api/spacedrive/files") {
      if (req.method === "POST") {
        const body = (await req.json()) as {
          location?: string;
          virtualPath?: string;
          name?: string;
          mimeType?: string;
          tags?: string[];
          content?: string;
        };
        const entry = workspaceDb.upsertSpacedriveFile({
          location: body.location || "sd://eu-fr1-nvme",
          virtualPath:
            body.virtualPath || `/Workspace/${body.name || "untitled.txt"}`,
          name: body.name || "untitled.txt",
          mimeType: body.mimeType || "text/plain",
          tags: body.tags || ["workspace"],
          content: body.content || "",
        });
        return jsonResponse({ entry }, 200, req);
      }
      const q = url.searchParams.get("q") || "";
      const loc = url.searchParams.get("location") || "all";
      return jsonResponse(
        { files: workspaceDb.listSpacedriveFiles(q, loc) },
        200,
        req,
      );
    }

    // 1f. API: Sovereign M3 Workspace (Mail, Notes, Calendar .ics, Contacts .vcf, Translate, Argon2id Account)
    if (pathname === "/api/workspace/mail") {
      if (req.method === "POST") {
        const body = (await req.json()) as {
          recipient?: string;
          subject?: string;
          body?: string;
        };
        const sent = workspaceDb.sendMail(
          body.recipient || "contact@aphrody.com",
          body.subject || "Message",
          body.body || "",
        );
        return jsonResponse({ sent }, 200, req);
      }
      return jsonResponse({ mail: workspaceDb.listMail() }, 200, req);
    }

    if (pathname === "/api/workspace/notes") {
      if (req.method === "POST") {
        const body = (await req.json()) as {
          title?: string;
          content?: string;
          colorHex?: string;
        };
        const note = workspaceDb.addNote(
          body.title || "Note",
          body.content || "",
          body.colorHex,
        );
        return jsonResponse({ note }, 200, req);
      }
      return jsonResponse({ notes: workspaceDb.listNotes() }, 200, req);
    }

    if (pathname === "/api/workspace/events") {
      return jsonResponse({ events: workspaceDb.listEvents() }, 200, req);
    }

    if (pathname === "/api/workspace/events.ics") {
      const ics = buildIcsCalendar(workspaceDb.listEvents());
      return new Response(ics, {
        headers: {
          "Content-Type": "text/calendar; charset=utf-8",
          "Content-Disposition": 'attachment; filename="aphrody-calendar.ics"',
          ...SECURITY_HEADERS,
        },
      });
    }

    if (pathname === "/api/workspace/contacts") {
      return jsonResponse({ contacts: workspaceDb.listContacts() }, 200, req);
    }

    if (pathname === "/api/workspace/contacts.vcf") {
      const vcf = buildVcfContacts(workspaceDb.listContacts());
      return new Response(vcf, {
        headers: {
          "Content-Type": "text/vcard; charset=utf-8",
          "Content-Disposition": 'attachment; filename="aphrody-contacts.vcf"',
          ...SECURITY_HEADERS,
        },
      });
    }

    if (pathname === "/api/workspace/translate" && req.method === "POST") {
      const body = (await req.json()) as {
        text?: string;
        from?: "fr" | "en";
        to?: "fr" | "en";
      };
      const tr = translateSovereignText(
        body.text || "",
        body.from || "fr",
        body.to || "en",
      );
      return jsonResponse(tr, 200, req);
    }

    if (pathname === "/api/workspace/account" && req.method === "POST") {
      const body = (await req.json()) as {
        email?: string;
        password?: string;
        displayName?: string;
      };
      if (!body.email || !body.password) {
        return jsonResponse({ error: "email and password required" }, 400, req);
      }
      const auth = await workspaceDb.registerOrVerifyAccount(
        body.email,
        body.password,
        body.displayName,
      );
      return jsonResponse(auth, 200, req);
    }

    // 1g. API: Locally Uncensored Studio Router
    if (pathname === "/api/studio/daemons") {
      const daemons = await probeLocalDaemons();
      return jsonResponse({ daemons }, 200, req);
    }

    if (pathname === "/api/studio/complete" && req.method === "POST") {
      const body = (await req.json()) as { prompt?: string };
      const res = await runLocalUncensoredCompletion(body.prompt || "");
      return jsonResponse(res, 200, req);
    }

    // 2. Health check (short: /ok, legacy: /healthz, /api/health)
    if (
      pathname === "/ok" ||
      pathname === "/healthz" ||
      pathname === "/api/health"
    ) {
      return jsonResponse(
        { status: "healthy", service: "aphrody", hash: getHashEngine().name },
        200,
        req,
      );
    }

    // 3. API: Store Search (short: /api/pm, legacy: /api/store/search)
    if (pathname === "/api/pm" || pathname === "/api/store/search") {
      const q = url.searchParams.get("q") || "";
      const eco = (url.searchParams.get("ecosystem") || "all") as any;
      const packages = await storeManager.search(q, eco);
      return jsonResponse(
        { query: q, ecosystem: eco, count: packages.length, packages },
        200,
        req,
      );
    }

    // 4. API: Store Install
    if (pathname === "/api/store/install" && req.method === "POST") {
      try {
        const body = (await req.json()) as { packageId: string };
        const newGen = storeManager.installPackage(body.packageId);
        return jsonResponse({ success: true, generation: newGen }, 200, req);
      } catch (err: any) {
        return jsonResponse({ success: false, error: err.message }, 400, req);
      }
    }

    // 5. API: Store Generations (GET list, POST rollback)
    if (pathname === "/api/store/generations") {
      if (req.method === "POST") {
        try {
          const body = (await req.json()) as { rollbackId: number };
          const rolledBackGen = storeManager.rollback(body.rollbackId);
          return jsonResponse(
            { success: true, generation: rolledBackGen },
            200,
            req,
          );
        } catch (err: any) {
          return jsonResponse({ success: false, error: err.message }, 400, req);
        }
      }
      const generations = storeManager.listGenerations();
      const active = storeManager.getActiveGeneration();
      return jsonResponse({ active, generations }, 200, req);
    }

    // 6. API: Awesome-List to Store Converter
    if (pathname === "/api/store/awesome-convert" && req.method === "POST") {
      try {
        const body = (await req.json()) as {
          markdownUrl?: string;
          markdown?: string;
        };
        const target = body.markdownUrl || body.markdown || "";
        const imported = await storeManager.convertAwesomeList(target);
        return jsonResponse(
          { success: true, count: imported.length, packages: imported },
          200,
          req,
        );
      } catch (err: any) {
        return jsonResponse({ success: false, error: err.message }, 400, req);
      }
    }

    // 7. API: Performance, LTO, AOT & Zero-Copy Stack Benchmark (short: /api/bnc, legacy: /api/bench)
    if (pathname === "/api/bnc" || pathname === "/api/bench") {
      try {
        const report = await runBenchmarkSuite();
        return jsonResponse(report, 200, req);
      } catch (err: any) {
        return jsonResponse({ error: err.message }, 500, req);
      }
    }

    // 8. Gemini grounded search (JSON, or SSE with ?stream=1 / Accept: text/event-stream)
    if (pathname === "/api/gemini/search") {
      if (req.method !== "GET")
        return new Response("Method Not Allowed", {
          status: 405,
          headers: { Allow: "GET" },
        });
      return withSecurity(
        await handleGeminiSearch(req, {
          env: process.env,
          client: clientId(req, srv),
        }),
      );
    }

    // 9. First-party GA4 relay
    if (pathname === "/g/collect") {
      return withSecurity(await handleCollect(req, { ga4: GOOGLE_CFG.ga4 }));
    }

    // 10. IndexNow: GET = dry-run payload, POST = submit (requires INDEXNOW_ADMIN_TOKEN)
    if (pathname === "/api/seo/indexnow") {
      const payload = indexNowPayload(INDEXNOW_KEY);
      if (req.method === "GET")
        return jsonResponse(
          { submit: "POST with Authorization: Bearer <token>", ...payload },
          200,
          req,
        );
      if (req.method === "POST") {
        const token = process.env.INDEXNOW_ADMIN_TOKEN || "";
        if (!token || req.headers.get("authorization") !== `Bearer ${token}`) {
          return jsonResponse({ error: "forbidden" }, 403, req);
        }
        try {
          const r = await fetch("https://api.indexnow.org/indexnow", {
            method: "POST",
            headers: { "Content-Type": "application/json; charset=utf-8" },
            body: JSON.stringify(payload),
            signal: AbortSignal.timeout(8000),
          });
          return jsonResponse(
            {
              submitted: (payload.urlList as string[]).length,
              status: r.status,
            },
            200,
            req,
          );
        } catch (err: any) {
          return jsonResponse(
            { error: "upstream_failed", message: String(err?.message || err) },
            502,
            req,
          );
        }
      }
      return new Response("Method Not Allowed", {
        status: 405,
        headers: { Allow: "GET, POST" },
      });
    }

    // 11. IndexNow key file
    if (pathname === `/${INDEXNOW_KEY}.txt`) {
      return new Response(INDEXNOW_KEY, {
        headers: {
          "Content-Type": "text/plain; charset=utf-8",
          "Cache-Control": "public, max-age=86400",
          ...SECURITY_HEADERS,
        },
      });
    }

    // 12. No-JavaScript search results page (noindex)
    if (pathname === "/search") {
      const q = (url.searchParams.get("q") || "")
        .replace(/\s+/g, " ")
        .trim()
        .slice(0, 200);
      const locale = localeOf(req, url);
      const pkgs = q ? (await storeManager.search(q, "all")).slice(0, 6) : [];
      const html = renderSearchResults(
        q,
        locale,
        q
          ? getSuggestions(q)
              .slice(0, 8)
              .map((s) => s.text)
          : [],
        pkgs.map((p) => ({
          name: p.name,
          description: p.description,
          installCmd: p.installCmd,
        })),
        GOOGLE_CFG,
      );
      return new Response(html, {
        headers: {
          "Content-Type": "text/html; charset=utf-8",
          "Cache-Control": "private, no-cache",
          ...htmlHeaders(pathname),
        },
      });
    }

    // 13. Static assets from memory (pre-compressed, strong ETag, 304)
    if (req.method !== "GET" && req.method !== "HEAD") {
      return new Response("Method Not Allowed", {
        status: 405,
        headers: { Allow: "GET, HEAD", ...SECURITY_HEADERS },
      });
    }
    if (pathname.length > 1 && pathname.endsWith("/")) {
      return Response.redirect(
        url.origin + pathname.slice(0, -1) + url.search,
        301,
      );
    }

    const asset = assets.get(pathname);
    if (asset) {
      return respondWithAsset(
        req,
        asset,
        asset.ext === ".html" ? htmlHeaders(pathname) : SECURITY_HEADERS,
      );
    }

    return new Response(render404(localeOf(req, url), GOOGLE_CFG), {
      status: 404,
      headers: {
        "Content-Type": "text/html; charset=utf-8",
        "Cache-Control": "no-cache",
        ...htmlHeaders(pathname),
      },
    });
  },
});

console.log(`Aphrody server running at http://localhost:${server.port}`);
