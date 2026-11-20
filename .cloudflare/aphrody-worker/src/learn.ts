import { verifyFleetBeaconAuth } from "./fleet.ts";
import { MONOGRAM_SVG, type ExecutionContextLike, type R2BucketLike, type R2ObjectLike, type WorkerEnv } from "./types.ts";

export const LEARN_BUNDLE_KEY = "learn/bundle/latest.json";
export const LEARN_MANIFEST_KEY = "learn/bundle/manifest.json";
export const LEARN_INBOX_PREFIX = "learn/inbox/";

export interface LearnJournalEntry {
  at: string;
  harness: string;
  agent?: string;
  pcId?: string;
  session?: string;
  cwd?: string;
  kind: "error" | "recovery" | "correction";
  tool?: string;
  summary: string;
}

export interface LearnMemoryTopic {
  slug: string;
  title: string;
  summary: string;
  bullets: string[];
  updatedAt: string;
  sources: string[];
  confidence: number;
}

export interface LearnBrainDoc {
  path: string;
  sha256: string;
  content: string;
  updatedAt: string;
}

export interface LearnSkillDelta {
  skill: string;
  notes: string[];
  updatedAt: string;
}

export interface LearnMcpProfile {
  keepBaseline: string[];
  disableRecommended: string[];
  updatedAt: string;
}

export interface LearnBundle {
  schemaVersion: number;
  revision: number;
  sha256: string;
  updatedAt: string;
  sources: {
    harnesses: string[];
    pcs: string[];
  };
  rules: {
    globalDirectives: Record<string, string>;
  };
  memory: {
    topics: Record<string, LearnMemoryTopic>;
  };
  brain: {
    docs: Record<string, LearnBrainDoc>;
  };
  skills: {
    deltas: Record<string, LearnSkillDelta>;
  };
  mcp: {
    profile: LearnMcpProfile;
  };
  stats: {
    totalEventsProcessed: number;
    lastDistillAt: string;
  };
}

export interface LearnManifest {
  schemaVersion: number;
  revision: number;
  sha256: string;
  updatedAt: string;
  counts: {
    rules: number;
    memoryTopics: number;
    brainDocs: number;
    skillDeltas: number;
    mcpBaseline: number;
    mcpDisabled: number;
    harnesses: number;
    pcs: number;
  };
  endpoints: {
    bundle: string;
    events: string;
    distill: string;
    publish: string;
  };
}

export interface LearnEventShard {
  shardId: string;
  receivedAt: string;
  harness: string;
  agent: string;
  pcId: string;
  entries: LearnJournalEntry[];
  mcpKeep?: string[];
  mcpDisable?: string[];
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

const SECRET_SCRUB_RE =
  /(sk-[A-Za-z0-9_-]{16,}|ghp_\w{20,}|gho_\w{20,}|github_pat_\w{20,}|AIza[\w-]{30,}|xox[bp]-[\w-]+|eyJ[\w-]{10,}\.[\w-]{10,}\.[\w-]{10,}|Bearer\s+\S{16,}|-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:api[_-]?key|token|secret|passw(?:or)?d)["']?\s*[=:]\s*["']?[^\s"',;]{6,})/gi;

function scrubText(input: string, maxLen = 400): string {
  return input.replace(SECRET_SCRUB_RE, "[REDACTED]").replace(/\s+/g, " ").trim().slice(0, maxLen);
}

async function readR2Text(object: R2ObjectLike): Promise<string> {
  if (typeof object.text === "function") return await object.text();
  if (typeof object.body === "string") return object.body;
  if (object.body instanceof Uint8Array) return decoder.decode(object.body);
  if (object.body && typeof (object.body as ReadableStream).getReader === "function") {
    return await new Response(object.body).text();
  }
  return "";
}

async function sha256Hex(text: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", encoder.encode(text));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function defaultLearnBundle(nowIso = new Date().toISOString()): LearnBundle {
  return {
    schemaVersion: 1,
    revision: 0,
    sha256: "",
    updatedAt: nowIso,
    sources: { harnesses: [], pcs: [] },
    rules: { globalDirectives: {} },
    memory: { topics: {} },
    brain: { docs: {} },
    skills: { deltas: {} },
    mcp: {
      profile: {
        keepBaseline: [],
        disableRecommended: [],
        updatedAt: nowIso,
      },
    },
    stats: {
      totalEventsProcessed: 0,
      lastDistillAt: nowIso,
    },
  };
}

export function buildLearnManifest(bundle: LearnBundle): LearnManifest {
  return {
    schemaVersion: bundle.schemaVersion,
    revision: bundle.revision,
    sha256: bundle.sha256,
    updatedAt: bundle.updatedAt,
    counts: {
      rules: Object.keys(bundle.rules.globalDirectives).length,
      memoryTopics: Object.keys(bundle.memory.topics).length,
      brainDocs: Object.keys(bundle.brain.docs).length,
      skillDeltas: Object.keys(bundle.skills.deltas).length,
      mcpBaseline: bundle.mcp.profile.keepBaseline.length,
      mcpDisabled: bundle.mcp.profile.disableRecommended.length,
      harnesses: bundle.sources.harnesses.length,
      pcs: bundle.sources.pcs.length,
    },
    endpoints: {
      bundle: "/api/learn/bundle.json",
      events: "/api/learn/events",
      distill: "/api/learn/distill",
      publish: "/api/learn/publish",
    },
  };
}

function resolveBucket(env: WorkerEnv): R2BucketLike | undefined {
  return env.ASSETS_BUCKET ?? env.RELEASES_BUCKET ?? env.OPEN_SOURCE_WEBSITE_ASSETS_BUCKET;
}

function resolveLearnToken(env: WorkerEnv): string | undefined {
  const explicit = env.LEARN_SYNC_TOKEN?.trim();
  if (explicit) return explicit;
  const fleet = env.FLEET_BEACON_TOKEN?.trim();
  return fleet || undefined;
}

function classifyDomain(entry: LearnJournalEntry): { slug: string; title: string; skill?: string } {
  const hay = `${entry.tool ?? ""} ${entry.summary}`.toLowerCase();
  if (/(session_inbox|session_list|session_register|swarm_wait|relay|fullstack)/.test(hay)) {
    return { slug: "cross-session", title: "Cross-Session & Relay Coordination", skill: "fleet" };
  }
  if (/(\bbun\b|bunx|bun\.test|package\.json|\.test\.ts)/.test(hay)) {
    return { slug: "bun-tooling", title: "Bun Runtime & Test Discovery", skill: "bun-gpui" };
  }
  if (/(ssh|fr1|fr2|prd|xbuild|scp|fleet)/.test(hay)) {
    return { slug: "fleet-ssh", title: "Multi-VPS Fleet & SSH Transport", skill: "fleet" };
  }
  if (/(wrangler|cloudflare|r2|workers|wrangler\.toml)/.test(hay)) {
    return { slug: "cloudflare-workers", title: "Cloudflare Workers & Wrangler Edge", skill: "cloudflare-platform" };
  }
  if (/(cargo|gpui|wgpu|naga|taffy|nsh)/.test(hay)) {
    return { slug: "rust-gpui", title: "Rust, GPUI & WGPU Build Pipeline", skill: "gpui" };
  }
  if (/(mcp|context|schema|token|skill)/.test(hay)) {
    return { slug: "mcp-context", title: "MCP Budget & Context Hygiene" };
  }
  if (/(git |submodule|merge|rebase|commit)/.test(hay)) {
    return { slug: "git-workflow", title: "Git Workflow Hygiene", skill: "github-ops" };
  }
  return { slug: "general", title: "Autonomous Execution & Recovery" };
}

function addUniqueCapped(list: string[], value: string, maxItems = 24): string[] {
  const clean = value.trim();
  if (!clean) return list;
  const next = list.includes(clean) ? list : [...list, clean];
  return next.length > maxItems ? next.slice(next.length - maxItems) : next;
}

async function loadBundle(bucket: R2BucketLike | undefined, nowIso: string): Promise<LearnBundle> {
  if (!bucket) return defaultLearnBundle(nowIso);
  const obj = await bucket.get(LEARN_BUNDLE_KEY);
  if (!obj) return defaultLearnBundle(nowIso);
  try {
    const parsed = JSON.parse(await readR2Text(obj)) as Partial<LearnBundle>;
    const base = defaultLearnBundle(nowIso);
    return {
      ...base,
      ...parsed,
      sources: { ...base.sources, ...(parsed.sources ?? {}) },
      rules: { ...base.rules, ...(parsed.rules ?? {}) },
      memory: { ...base.memory, ...(parsed.memory ?? {}) },
      brain: { ...base.brain, ...(parsed.brain ?? {}) },
      skills: { ...base.skills, ...(parsed.skills ?? {}) },
      mcp: {
        profile: {
          ...base.mcp.profile,
          ...(parsed.mcp?.profile ?? {}),
        },
      },
      stats: { ...base.stats, ...(parsed.stats ?? {}) },
    };
  } catch {
    return defaultLearnBundle(nowIso);
  }
}

async function persistBundle(bucket: R2BucketLike, bundle: LearnBundle, nowIso: string): Promise<{ bundle: LearnBundle; manifest: LearnManifest }> {
  bundle.updatedAt = nowIso;
  const bodyForHash = JSON.stringify({
    rules: bundle.rules,
    memory: bundle.memory,
    brain: bundle.brain,
    skills: bundle.skills,
    mcp: bundle.mcp,
  });
  const nextSha = await sha256Hex(bodyForHash);
  if (nextSha !== bundle.sha256 || bundle.revision === 0) {
    bundle.revision += 1;
    bundle.sha256 = nextSha;
  }
  const manifest = buildLearnManifest(bundle);
  if (bucket.put) {
    await Promise.all([
      bucket.put(LEARN_BUNDLE_KEY, JSON.stringify(bundle), {
        httpMetadata: { contentType: "application/json; charset=utf-8" },
      }),
      bucket.put(LEARN_MANIFEST_KEY, JSON.stringify(manifest), {
        httpMetadata: { contentType: "application/json; charset=utf-8" },
      }),
    ]);
  }
  return { bundle, manifest };
}

export function foldShardsIntoBundle(
  bundle: LearnBundle,
  shards: LearnEventShard[],
  nowIso = new Date().toISOString(),
): { changed: boolean; promotedTopics: number; processedEntries: number } {
  let changed = false;
  let processedEntries = 0;
  let promotedTopics = 0;

  const grouped = new Map<string, { count: number; sample: LearnJournalEntry; domain: ReturnType<typeof classifyDomain> }>();

  for (const shard of shards) {
    if (shard.harness && !bundle.sources.harnesses.includes(shard.harness)) {
      bundle.sources.harnesses.push(shard.harness);
      changed = true;
    }
    if (shard.pcId && !bundle.sources.pcs.includes(shard.pcId)) {
      bundle.sources.pcs.push(shard.pcId);
      changed = true;
    }
    for (const keep of shard.mcpKeep ?? []) {
      const next = addUniqueCapped(bundle.mcp.profile.keepBaseline, keep, 32);
      if (next.length !== bundle.mcp.profile.keepBaseline.length) {
        bundle.mcp.profile.keepBaseline = next;
        bundle.mcp.profile.updatedAt = nowIso;
        changed = true;
      }
    }
    for (const dis of shard.mcpDisable ?? []) {
      if (bundle.mcp.profile.keepBaseline.includes(dis)) continue;
      const next = addUniqueCapped(bundle.mcp.profile.disableRecommended, dis, 64);
      if (next.length !== bundle.mcp.profile.disableRecommended.length) {
        bundle.mcp.profile.disableRecommended = next;
        bundle.mcp.profile.updatedAt = nowIso;
        changed = true;
      }
    }
    for (const entry of shard.entries) {
      processedEntries++;
      const domain = classifyDomain(entry);
      const sig = `${entry.kind}:${entry.tool ?? "prompt"}:${entry.summary.slice(0, 72).toLowerCase()}`;
      const prev = grouped.get(sig);
      grouped.set(sig, { count: (prev?.count ?? 0) + 1, sample: entry, domain });
    }
  }

  for (const { count, sample, domain } of grouped.values()) {
    // Auto-promote with zero human approval: every explicit correction, every recovery, or recurring error (x2+).
    const qualifies = sample.kind === "correction" || sample.kind === "recovery" || count >= 2;
    if (!qualifies) continue;

    const existing = bundle.memory.topics[domain.slug] ?? {
      slug: domain.slug,
      title: domain.title,
      summary: `Autonomous learnings for ${domain.title}.`,
      bullets: [],
      updatedAt: nowIso,
      sources: [],
      confidence: 0.7,
    };

    const bullet = `[${sample.kind}] ${scrubText(sample.summary, 220)}`;
    const nextBullets = addUniqueCapped(existing.bullets, bullet, 20);
    const nextSources = addUniqueCapped(existing.sources, `${sample.harness}:${sample.pcId ?? "local"}`, 16);
    if (nextBullets.length !== existing.bullets.length || nextSources.length !== existing.sources.length) {
      existing.bullets = nextBullets;
      existing.sources = nextSources;
      existing.updatedAt = nowIso;
      existing.confidence = Number(Math.min(0.99, existing.confidence + 0.05 * count).toFixed(2));
      bundle.memory.topics[domain.slug] = existing;
      promotedTopics++;
      changed = true;
    }

    if (sample.kind === "correction" || (sample.kind === "recovery" && count >= 2)) {
      const ruleText = scrubText(sample.summary, 180);
      if (ruleText && !bundle.rules.globalDirectives[domain.slug]?.includes(ruleText)) {
        const cur = bundle.rules.globalDirectives[domain.slug] ?? "";
        const merged = cur ? `${cur}\n- ${ruleText}` : `- ${ruleText}`;
        if (merged.length <= 1600) {
          bundle.rules.globalDirectives[domain.slug] = merged;
          changed = true;
        }
      }
    }

    if (domain.skill) {
      const delta = bundle.skills.deltas[domain.skill] ?? {
        skill: domain.skill,
        notes: [],
        updatedAt: nowIso,
      };
      const note = `[${sample.kind}] ${scrubText(sample.summary, 200)}`;
      const nextNotes = addUniqueCapped(delta.notes, note, 16);
      if (nextNotes.length !== delta.notes.length) {
        delta.notes = nextNotes;
        delta.updatedAt = nowIso;
        bundle.skills.deltas[domain.skill] = delta;
        changed = true;
      }
    }
  }

  bundle.stats.totalEventsProcessed += processedEntries;
  bundle.stats.lastDistillAt = nowIso;
  return { changed, promotedTopics, processedEntries };
}

export async function distillLearnState(
  env: WorkerEnv,
  maxShards = 60,
): Promise<{
  ok: boolean;
  shardsProcessed: number;
  entriesProcessed: number;
  promotedTopics: number;
  revision: number;
  sha256: string;
}> {
  const nowIso = new Date().toISOString();
  const bucket = resolveBucket(env);
  const bundle = await loadBundle(bucket, nowIso);
  if (!bucket?.list) {
    return {
      ok: true,
      shardsProcessed: 0,
      entriesProcessed: 0,
      promotedTopics: 0,
      revision: bundle.revision,
      sha256: bundle.sha256,
    };
  }

  const listed = await bucket.list({ prefix: LEARN_INBOX_PREFIX, limit: maxShards });
  const shards: LearnEventShard[] = [];
  const consumedKeys: string[] = [];

  for (const item of listed.objects) {
    const obj = await bucket.get(item.key);
    consumedKeys.push(item.key);
    if (!obj) continue;
    try {
      const parsed = JSON.parse(await readR2Text(obj)) as LearnEventShard;
      if (parsed && Array.isArray(parsed.entries)) shards.push(parsed);
    } catch {
      // Skip corrupted shard and allow cleanup.
    }
  }

  const { changed, promotedTopics, processedEntries } = foldShardsIntoBundle(bundle, shards, nowIso);
  let manifest = buildLearnManifest(bundle);
  if (changed || bundle.revision === 0) {
    const saved = await persistBundle(bucket, bundle, nowIso);
    manifest = saved.manifest;
  }

  if (consumedKeys.length > 0 && bucket.delete) {
    await bucket.delete(consumedKeys);
  }

  if (changed && env.EVENTS) {
    try {
      await env.EVENTS.send(
        {
          v: 1,
          kind: "learn_distill",
          revision: manifest.revision,
          sha256: manifest.sha256,
          shards: consumedKeys.length,
          promotedTopics,
          at: nowIso,
        },
        { contentType: "json" },
      );
    } catch {
      // Non-fatal if queue is unavailable during offline tests.
    }
  }

  return {
    ok: true,
    shardsProcessed: consumedKeys.length,
    entriesProcessed: processedEntries,
    promotedTopics,
    revision: manifest.revision,
    sha256: manifest.sha256,
  };
}

function renderLearnHomeHtml(manifest: LearnManifest): string {
  return `<!DOCTYPE html>
<html lang="en" data-theme="neural">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Aphrody Autonomous Learn Hub</title>
  <link rel="icon" href="/icon.svg" type="image/svg+xml" />
  <style>
    body{margin:0;background:#0B0E14;color:#E4EAF4;font-family:"Google Sans Flex",system-ui,sans-serif;padding:40px 24px}
    main{max-width:780px;margin:0 auto;background:#13161F;border:1px solid #2A3242;border-radius:16px;padding:28px}
    h1{margin:0 0 8px;color:#8FD0FF;font-size:24px}
    p{color:#BAC4D6;line-height:1.5}
    .grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;margin:20px 0}
    .card{background:#181C26;border:1px solid #2A3242;border-radius:12px;padding:12px}
    .val{font-size:22px;font-weight:700;color:#8FD0FF}
    .lbl{font-size:12px;color:#BAC4D6;margin-top:4px}
    code{font-family:"Google Sans Code",monospace;background:#0B0E14;padding:2px 6px;border-radius:6px}
  </style>
</head>
<body>
  <main>
    <h1>Aphrody Autonomous Serverless Learn</h1>
    <p>Unified zero-approval knowledge, brain, memory, skills, and MCP synchronization for <code>agy</code>, <code>claude</code>, <code>codex</code>, and fleet agents.</p>
    <div class="grid">
      <div class="card"><div class="val">${manifest.revision}</div><div class="lbl">Revision</div></div>
      <div class="card"><div class="val">${manifest.counts.memoryTopics}</div><div class="lbl">Memory Topics</div></div>
      <div class="card"><div class="val">${manifest.counts.brainDocs}</div><div class="lbl">Brain Docs</div></div>
      <div class="card"><div class="val">${manifest.counts.skillDeltas}</div><div class="lbl">Skill Deltas</div></div>
      <div class="card"><div class="val">${manifest.counts.harnesses}</div><div class="lbl">Harnesses</div></div>
      <div class="card"><div class="val">${manifest.counts.pcs}</div><div class="lbl">Synced PCs</div></div>
    </div>
    <p>Endpoints: <code>/api/learn/status</code> &middot; <code>/api/learn/manifest.json</code> &middot; <code>/api/learn/bundle.json</code> &middot; <code>/api/learn/events</code> &middot; <code>/api/learn/distill</code></p>
  </main>
</body>
</html>`;
}

export async function handleLearnRequest(
  request: Request,
  env: WorkerEnv,
  ctx?: ExecutionContextLike,
): Promise<Response> {
  const url = new URL(request.url);
  const path = url.pathname;
  const nowIso = new Date().toISOString();
  const bucket = resolveBucket(env);

  if (path === "/icon.svg" || path === "/favicon.svg") {
    return new Response(MONOGRAM_SVG, {
      headers: { "Content-Type": "image/svg+xml; charset=utf-8", "Cache-Control": "public, max-age=86400" },
    });
  }

  if (path === "/" || path === "/learn" || path === "/api/learn" || path === "/api/learn/status") {
    const bundle = await loadBundle(bucket, nowIso);
    const manifest = buildLearnManifest(bundle);
    const accept = request.headers.get("accept") ?? "";
    if ((path === "/" || path === "/learn") && accept.includes("text/html")) {
      return new Response(renderLearnHomeHtml(manifest), {
        headers: { "Content-Type": "text/html; charset=utf-8", "Cache-Control": "no-store" },
      });
    }
    return Response.json(
      { ok: true, service: "aphrody-learn", autonomous: true, humanApprovalRequired: false, ...manifest },
      { headers: { "Cache-Control": "no-store" } },
    );
  }

  if (request.method === "GET" && (path === "/api/learn/manifest.json" || path === "/api/learn/bundle.json")) {
    const token = resolveLearnToken(env);
    if (token && env.LEARN_PUBLIC_READ !== "true") {
      const auth = await verifyFleetBeaconAuth(token, request.headers, new ArrayBuffer(0));
      if (!auth.ok) {
        return Response.json({ ok: false, error: "unauthorized" }, { status: 401, headers: { "Cache-Control": "no-store" } });
      }
    }
    const bundle = await loadBundle(bucket, nowIso);
    const manifest = buildLearnManifest(bundle);
    const etag = bundle.sha256 ? `"${bundle.sha256}"` : `"rev-${bundle.revision}"`;
    const sinceRev = Number(url.searchParams.get("since") ?? "-1");
    if (
      request.headers.get("if-none-match") === etag ||
      (Number.isFinite(sinceRev) && sinceRev >= 0 && sinceRev === bundle.revision)
    ) {
      return new Response(null, { status: 304, headers: { etag, "Cache-Control": "no-store" } });
    }
    return Response.json(path === "/api/learn/manifest.json" ? manifest : bundle, {
      headers: { etag, "Cache-Control": "no-store" },
    });
  }

  if (request.method === "POST" && path === "/api/learn/events") {
    const bodyBytes = await request.arrayBuffer();
    const auth = await verifyFleetBeaconAuth(resolveLearnToken(env), request.headers, bodyBytes);
    if (!auth.ok) {
      return Response.json({ ok: false, error: "unauthorized" }, { status: 401, headers: { "Cache-Control": "no-store" } });
    }
    let payload: Record<string, unknown>;
    try {
      payload = JSON.parse(decoder.decode(bodyBytes)) as Record<string, unknown>;
    } catch {
      return Response.json({ ok: false, error: "invalid_json" }, { status: 400 });
    }
    const rawEntries = Array.isArray(payload.entries) ? payload.entries : [];
    const entries: LearnJournalEntry[] = [];
    for (const item of rawEntries.slice(0, 100)) {
      if (!item || typeof item !== "object" || Array.isArray(item)) continue;
      const obj = item as Record<string, unknown>;
      const kind = obj.kind;
      if (kind !== "error" && kind !== "recovery" && kind !== "correction") continue;
      const summary = scrubText(String(obj.summary ?? ""));
      if (!summary) continue;
      entries.push({
        at: typeof obj.at === "string" ? obj.at : nowIso,
        harness: scrubText(String(obj.harness ?? payload.harness ?? "unknown"), 32) || "unknown",
        agent: scrubText(String(obj.agent ?? payload.agent ?? obj.harness ?? "unknown"), 32) || "unknown",
        pcId: scrubText(String(obj.pcId ?? payload.pcId ?? "pc"), 48) || "pc",
        session: typeof obj.session === "string" ? scrubText(obj.session, 64) : undefined,
        cwd: typeof obj.cwd === "string" ? scrubText(obj.cwd, 120) : undefined,
        kind,
        tool: typeof obj.tool === "string" ? scrubText(obj.tool, 48) : undefined,
        summary,
      });
    }
    const mcpKeep = Array.isArray(payload.mcpKeep) ? payload.mcpKeep.filter((x): x is string => typeof x === "string") : [];
    const mcpDisable = Array.isArray(payload.mcpDisable)
      ? payload.mcpDisable.filter((x): x is string => typeof x === "string")
      : [];

    const shard: LearnEventShard = {
      shardId: `${Date.now()}-${crypto.randomUUID().slice(0, 8)}`,
      receivedAt: nowIso,
      harness: scrubText(String(payload.harness ?? entries[0]?.harness ?? "unknown"), 32) || "unknown",
      agent: scrubText(String(payload.agent ?? payload.harness ?? "unknown"), 32) || "unknown",
      pcId: scrubText(String(payload.pcId ?? entries[0]?.pcId ?? "pc"), 48) || "pc",
      entries,
      ...(mcpKeep.length ? { mcpKeep } : {}),
      ...(mcpDisable.length ? { mcpDisable } : {}),
    };

    if (!bucket?.put) {
      return Response.json(
        { ok: true, persisted: false, accepted: entries.length, authMode: auth.mode },
        { status: 202, headers: { "Cache-Control": "no-store" } },
      );
    }

    const shardKey = `${LEARN_INBOX_PREFIX}${shard.shardId}.json`;
    await bucket.put(shardKey, JSON.stringify(shard), {
      httpMetadata: { contentType: "application/json; charset=utf-8" },
    });

    const immediate = url.searchParams.get("distill") === "1" || !ctx?.waitUntil;
    if (immediate) {
      const distilled = await distillLearnState(env);
      return Response.json(
        { ok: true, persisted: true, accepted: entries.length, authMode: auth.mode, distilled },
        { status: 200, headers: { "Cache-Control": "no-store" } },
      );
    }

    ctx?.waitUntil?.(distillLearnState(env));
    return Response.json(
      { ok: true, persisted: true, accepted: entries.length, authMode: auth.mode, queuedDistill: true },
      { status: 202, headers: { "Cache-Control": "no-store" } },
    );
  }

  if (request.method === "POST" && path === "/api/learn/distill") {
    const bodyBytes = await request.arrayBuffer();
    const auth = await verifyFleetBeaconAuth(resolveLearnToken(env), request.headers, bodyBytes);
    if (!auth.ok) {
      return Response.json({ ok: false, error: "unauthorized" }, { status: 401, headers: { "Cache-Control": "no-store" } });
    }
    const result = await distillLearnState(env);
    return Response.json({ ...result, authMode: auth.mode }, { headers: { "Cache-Control": "no-store" } });
  }

  if (request.method === "POST" && path === "/api/learn/publish") {
    const bodyBytes = await request.arrayBuffer();
    const auth = await verifyFleetBeaconAuth(resolveLearnToken(env), request.headers, bodyBytes);
    if (!auth.ok) {
      return Response.json({ ok: false, error: "unauthorized" }, { status: 401, headers: { "Cache-Control": "no-store" } });
    }
    if (!bucket?.put) {
      return Response.json({ ok: false, error: "assets_bucket_unbound" }, { status: 503 });
    }
    let payload: Record<string, unknown>;
    try {
      payload = JSON.parse(decoder.decode(bodyBytes)) as Record<string, unknown>;
    } catch {
      return Response.json({ ok: false, error: "invalid_json" }, { status: 400 });
    }
    const bundle = await loadBundle(bucket, nowIso);
    if (typeof payload.harness === "string" && !bundle.sources.harnesses.includes(payload.harness)) {
      bundle.sources.harnesses.push(payload.harness);
    }
    if (typeof payload.pcId === "string" && !bundle.sources.pcs.includes(payload.pcId)) {
      bundle.sources.pcs.push(payload.pcId);
    }
    const rules = (payload.rules as Record<string, string> | undefined) ?? {};
    for (const [k, v] of Object.entries(rules)) {
      if (typeof v === "string" && v.trim()) bundle.rules.globalDirectives[k] = scrubText(v, 4000);
    }
    const topics = (payload.memoryTopics as Record<string, LearnMemoryTopic> | undefined) ?? {};
    for (const [slug, topic] of Object.entries(topics)) {
      if (topic && typeof topic === "object") {
        bundle.memory.topics[slug] = {
          slug,
          title: scrubText(String(topic.title ?? slug), 120),
          summary: scrubText(String(topic.summary ?? ""), 400),
          bullets: Array.isArray(topic.bullets)
            ? topic.bullets.slice(0, 24).map((b) => scrubText(String(b), 240))
            : [],
          updatedAt: nowIso,
          sources: Array.isArray(topic.sources) ? topic.sources.slice(0, 16) : [],
          confidence: typeof topic.confidence === "number" ? topic.confidence : 0.9,
        };
      }
    }
    const brainDocs = (payload.brainDocs as Record<string, LearnBrainDoc> | undefined) ?? {};
    for (const [docKey, doc] of Object.entries(brainDocs)) {
      if (doc && typeof doc.content === "string") {
        bundle.brain.docs[docKey] = {
          path: doc.path || docKey,
          sha256: doc.sha256 || (await sha256Hex(doc.content)),
          content: doc.content.slice(0, 24000),
          updatedAt: nowIso,
        };
      }
    }
    const skillDeltas = (payload.skillDeltas as Record<string, LearnSkillDelta> | undefined) ?? {};
    for (const [sk, delta] of Object.entries(skillDeltas)) {
      if (delta && Array.isArray(delta.notes)) {
        const cur = bundle.skills.deltas[sk] ?? { skill: sk, notes: [], updatedAt: nowIso };
        let nextNotes = [...cur.notes];
        for (const n of delta.notes) nextNotes = addUniqueCapped(nextNotes, scrubText(String(n), 220), 20);
        bundle.skills.deltas[sk] = { skill: sk, notes: nextNotes, updatedAt: nowIso };
      }
    }
    const mcp = payload.mcpProfile as { keepBaseline?: string[]; disableRecommended?: string[] } | undefined;
    if (mcp) {
      for (const s of mcp.keepBaseline ?? []) {
        bundle.mcp.profile.keepBaseline = addUniqueCapped(bundle.mcp.profile.keepBaseline, s, 32);
      }
      for (const s of mcp.disableRecommended ?? []) {
        bundle.mcp.profile.disableRecommended = addUniqueCapped(bundle.mcp.profile.disableRecommended, s, 64);
      }
      bundle.mcp.profile.updatedAt = nowIso;
    }

    const saved = await persistBundle(bucket, bundle, nowIso);
    return Response.json(
      { ok: true, authMode: auth.mode, manifest: saved.manifest },
      { headers: { "Cache-Control": "no-store" } },
    );
  }

  return Response.json({ ok: false, error: "not_found", path }, { status: 404, headers: { "Cache-Control": "no-store" } });
}
