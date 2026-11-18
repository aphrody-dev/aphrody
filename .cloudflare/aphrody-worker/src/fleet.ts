import type { R2BucketLike, R2ObjectLike, WorkerEnv } from "./types.ts";

export const FLEET_TELEMETRY_KEY = "fleet/telemetry.json";
export const DEFAULT_STALE_TTL_SEC = 300;

export interface FleetNodeTelemetry {
  alias: string;
  region?: string;
  tier?: string;
  reachable: boolean;
  healthy: boolean;
  os?: string;
  arch?: string;
  cpus?: number;
  load1?: number;
  load5?: number;
  load15?: number;
  loadRatio?: number;
  memTotalGb?: number;
  memAvailGb?: number;
  workAvailGb?: number;
  activeJobs?: number;
  rttMs?: number;
  capabilities?: string[];
  labels?: string[];
  score: number;
  reportedAt?: string;
}

export interface FleetTelemetrySnapshot {
  schemaVersion: number;
  updatedAt: string;
  sourceHost?: string;
  ttlSec: number;
  nodes: FleetNodeTelemetry[];
}

export interface FleetAuthResult {
  ok: boolean;
  authenticated: boolean;
  mode: "unauthenticated" | "bearer" | "hmac-sha256" | "rejected";
}

const JOB_CAPABILITY_MAP: Record<string, { required: string[]; preferred: string[] }> = {
  native_rust: { required: ["native_rust"], preferred: ["xfs_reflink"] },
  wasm: { required: ["wasm"], preferred: ["xfs_reflink", "native_rust"] },
  edge_proxy: { required: ["edge_proxy"], preferred: ["btrfs_reflink"] },
  verify: { required: ["native_rust"], preferred: ["wasm", "xfs_reflink"] },
  release: { required: ["native_rust"], preferred: ["xfs_reflink", "wasm"] },
  any: { required: [], preferred: [] },
};

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function timingSafeEqualBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) {
    diff |= a[i] ^ b[i];
  }
  return diff === 0;
}

export async function signFleetPayload(secret: string, body: string | ArrayBuffer | Uint8Array): Promise<string> {
  const bytes =
    typeof body === "string" ? encoder.encode(body) : body instanceof Uint8Array ? body : new Uint8Array(body);
  const key = await crypto.subtle.importKey("raw", encoder.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, [
    "sign",
  ]);
  const mac = new Uint8Array(await crypto.subtle.sign("HMAC", key, bytes));
  return `sha256=${[...mac].map((b) => b.toString(16).padStart(2, "0")).join("")}`;
}

export async function verifyFleetBeaconAuth(
  expectedToken: string | undefined,
  headers: Headers,
  bodyBytes: ArrayBuffer,
): Promise<FleetAuthResult> {
  const token = expectedToken?.trim();
  if (!token) {
    return { ok: true, authenticated: false, mode: "unauthenticated" };
  }

  const authHeader = headers.get("authorization") ?? "";
  const bearerMatch = /^Bearer\s+(.+)$/i.exec(authHeader.trim());
  if (bearerMatch) {
    const provided = encoder.encode(bearerMatch[1].trim());
    const expected = encoder.encode(token);
    if (timingSafeEqualBytes(provided, expected)) {
      return { ok: true, authenticated: true, mode: "bearer" };
    }
  }

  const sigHeader = (headers.get("x-fleet-signature-256") ?? headers.get("x-hub-signature-256") ?? "").trim();
  if (sigHeader.startsWith("sha256=")) {
    const hex = sigHeader.slice(7).toLowerCase();
    if (/^[0-9a-f]{64}$/.test(hex)) {
      const key = await crypto.subtle.importKey(
        "raw",
        encoder.encode(token),
        { name: "HMAC", hash: "SHA-256" },
        false,
        ["sign"],
      );
      const mac = new Uint8Array(await crypto.subtle.sign("HMAC", key, bodyBytes));
      const got = new Uint8Array(mac.length);
      for (let i = 0; i < got.length; i++) {
        got[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
      }
      if (timingSafeEqualBytes(mac, got)) {
        return { ok: true, authenticated: true, mode: "hmac-sha256" };
      }
    }
  }

  return { ok: false, authenticated: false, mode: "rejected" };
}

async function readR2ObjectText(object: R2ObjectLike): Promise<string> {
  if (typeof object.text === "function") {
    return await object.text();
  }
  if (typeof object.body === "string") {
    return object.body;
  }
  if (object.body instanceof Uint8Array) {
    return decoder.decode(object.body);
  }
  if (object.body && typeof (object.body as ReadableStream).getReader === "function") {
    return await new Response(object.body).text();
  }
  return "";
}

function normalizeNode(raw: unknown, nowIso: string): FleetNodeTelemetry | null {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
  const obj = raw as Record<string, unknown>;
  const aliasRaw = typeof obj.alias === "string" ? obj.alias : typeof obj.host === "string" ? obj.host : "";
  const alias = aliasRaw.trim();
  if (!alias) return null;

  const cpus = typeof obj.cpus === "number" && Number.isFinite(obj.cpus) && obj.cpus > 0 ? obj.cpus : undefined;
  const load1 = typeof obj.load1 === "number" && Number.isFinite(obj.load1) && obj.load1 >= 0 ? obj.load1 : undefined;
  const loadRatio =
    typeof obj.loadRatio === "number" && Number.isFinite(obj.loadRatio)
      ? obj.loadRatio
      : load1 !== undefined && cpus !== undefined
        ? Number((load1 / cpus).toFixed(3))
        : undefined;
  const memTotalGb =
    typeof obj.memTotalGb === "number" && Number.isFinite(obj.memTotalGb) ? obj.memTotalGb : undefined;
  const memAvailGb =
    typeof obj.memAvailGb === "number" && Number.isFinite(obj.memAvailGb) ? obj.memAvailGb : undefined;
  const workAvailGb =
    typeof obj.workAvailGb === "number" && Number.isFinite(obj.workAvailGb) ? obj.workAvailGb : undefined;
  const activeJobs =
    typeof obj.activeJobs === "number" && Number.isFinite(obj.activeJobs) && obj.activeJobs >= 0
      ? obj.activeJobs
      : undefined;
  const rttMs = typeof obj.rttMs === "number" && Number.isFinite(obj.rttMs) && obj.rttMs >= 0 ? obj.rttMs : undefined;

  const reachable = typeof obj.reachable === "boolean" ? obj.reachable : true;
  const healthy = typeof obj.healthy === "boolean" ? obj.healthy : reachable;

  let score: number;
  if (typeof obj.score === "number" && Number.isFinite(obj.score)) {
    score = Number(clamp(obj.score, 0, 100).toFixed(2));
  } else if (!reachable || !healthy) {
    score = 0;
  } else {
    const loadPenalty = clamp(loadRatio ?? 0, 0, 1.5) * 42;
    const memFreeRatio =
      memTotalGb && memTotalGb > 0 && memAvailGb !== undefined ? clamp(memAvailGb / memTotalGb, 0, 1) : 1;
    const memPenalty = (1 - memFreeRatio) * 25;
    const diskRatio = workAvailGb !== undefined ? clamp(workAvailGb / 25, 0, 1) : 1;
    const diskPenalty = (1 - diskRatio) * 18;
    const jobPenalty = (activeJobs ?? 0) * 8;
    const rttPenalty = clamp((rttMs ?? 0) / 500, 0, 1) * 7;
    score = Number(clamp(100 - loadPenalty - memPenalty - diskPenalty - jobPenalty - rttPenalty, 0, 100).toFixed(2));
  }

  const capabilities = Array.isArray(obj.capabilities)
    ? obj.capabilities.filter((c): c is string => typeof c === "string" && c.trim().length > 0)
    : undefined;
  const labels = Array.isArray(obj.labels)
    ? obj.labels.filter((c): c is string => typeof c === "string" && c.trim().length > 0)
    : undefined;

  return {
    alias,
    ...(typeof obj.region === "string" && obj.region ? { region: obj.region } : {}),
    ...(typeof obj.tier === "string" && obj.tier ? { tier: obj.tier } : {}),
    reachable,
    healthy,
    ...(typeof obj.os === "string" && obj.os ? { os: obj.os } : {}),
    ...(typeof obj.arch === "string" && obj.arch ? { arch: obj.arch } : {}),
    ...(cpus !== undefined ? { cpus } : {}),
    ...(load1 !== undefined ? { load1 } : {}),
    ...(typeof obj.load5 === "number" && Number.isFinite(obj.load5) ? { load5: obj.load5 } : {}),
    ...(typeof obj.load15 === "number" && Number.isFinite(obj.load15) ? { load15: obj.load15 } : {}),
    ...(loadRatio !== undefined ? { loadRatio } : {}),
    ...(memTotalGb !== undefined ? { memTotalGb } : {}),
    ...(memAvailGb !== undefined ? { memAvailGb } : {}),
    ...(workAvailGb !== undefined ? { workAvailGb } : {}),
    ...(activeJobs !== undefined ? { activeJobs } : {}),
    ...(rttMs !== undefined ? { rttMs } : {}),
    ...(capabilities ? { capabilities } : {}),
    ...(labels ? { labels } : {}),
    score,
    reportedAt: typeof obj.reportedAt === "string" && obj.reportedAt ? obj.reportedAt : nowIso,
  };
}

async function readTelemetrySnapshot(
  env: WorkerEnv,
): Promise<{ source: "r2" | "default"; snapshot: FleetTelemetrySnapshot }> {
  const candidates: Array<R2BucketLike | undefined> = [env.ASSETS_BUCKET, env.RELEASES_BUCKET];
  for (const bucket of candidates) {
    if (!bucket) continue;
    const obj = await bucket.get(FLEET_TELEMETRY_KEY);
    if (!obj) continue;
    try {
      const text = await readR2ObjectText(obj);
      if (!text.trim()) continue;
      const parsed = JSON.parse(text) as Record<string, unknown>;
      const nowIso = new Date().toISOString();
      const updatedAt = typeof parsed.updatedAt === "string" && parsed.updatedAt ? parsed.updatedAt : nowIso;
      const ttlSec =
        typeof parsed.ttlSec === "number" && Number.isFinite(parsed.ttlSec) && parsed.ttlSec > 0
          ? parsed.ttlSec
          : DEFAULT_STALE_TTL_SEC;
      const rawNodes = Array.isArray(parsed.nodes) ? parsed.nodes : [];
      const nodes = rawNodes
        .map((n) => normalizeNode(n, updatedAt))
        .filter((n): n is FleetNodeTelemetry => n !== null);
      return {
        source: "r2",
        snapshot: {
          schemaVersion: typeof parsed.schemaVersion === "number" ? parsed.schemaVersion : 1,
          updatedAt,
          ...(typeof parsed.sourceHost === "string" ? { sourceHost: parsed.sourceHost } : {}),
          ttlSec,
          nodes,
        },
      };
    } catch {
      continue;
    }
  }
  return {
    source: "default",
    snapshot: {
      schemaVersion: 1,
      updatedAt: new Date(0).toISOString(),
      ttlSec: DEFAULT_STALE_TTL_SEC,
      nodes: [],
    },
  };
}

function isNodeStale(node: FleetNodeTelemetry, fallbackUpdatedAt: string, ttlSec: number, nowMs: number): boolean {
  const ts = Date.parse(node.reportedAt ?? fallbackUpdatedAt);
  if (!Number.isFinite(ts) || ts <= 0) return false;
  return nowMs - ts > ttlSec * 1000;
}

function resolveWritableBucket(env: WorkerEnv): R2BucketLike | undefined {
  if (env.ASSETS_BUCKET && typeof env.ASSETS_BUCKET.put === "function") return env.ASSETS_BUCKET;
  if (env.RELEASES_BUCKET && typeof env.RELEASES_BUCKET.put === "function") return env.RELEASES_BUCKET;
  return undefined;
}

function jsonResponse(payload: unknown, status = 200): Response {
  return Response.json(payload, {
    status,
    headers: {
      "Cache-Control": "no-store",
      "Access-Control-Allow-Origin": "*",
      "X-Content-Type-Options": "nosniff",
    },
  });
}

async function handleStatus(url: URL, env: WorkerEnv): Promise<Response> {
  const { source, snapshot } = await readTelemetrySnapshot(env);
  const ttlParam = Number(url.searchParams.get("ttl"));
  const ttlSec = Number.isFinite(ttlParam) && ttlParam > 0 ? ttlParam : snapshot.ttlSec;
  const nowMs = Date.now();
  const nodes = snapshot.nodes.map((node) => {
    const stale = isNodeStale(node, snapshot.updatedAt, ttlSec, nowMs);
    return {
      ...node,
      stale,
    };
  });
  const healthyNodes = nodes.filter((n) => n.reachable && n.healthy && !n.stale).length;

  return jsonResponse({
    ok: true,
    service: "aphrody-worker",
    endpoint: "fleet/status",
    source,
    updatedAt: snapshot.updatedAt,
    staleTtlSec: ttlSec,
    totalNodes: nodes.length,
    healthyNodes,
    nodes,
  });
}

async function handleRoute(request: Request, url: URL, env: WorkerEnv): Promise<Response> {
  let bodyParams: Record<string, unknown> = {};
  if (request.method === "POST") {
    try {
      const parsed = (await request.json()) as unknown;
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        bodyParams = parsed as Record<string, unknown>;
      }
    } catch {
      return jsonResponse({ ok: false, error: "invalid_json" }, 400);
    }
  }

  const job = String(url.searchParams.get("job") ?? url.searchParams.get("kind") ?? bodyParams.job ?? "any")
    .trim()
    .toLowerCase();
  const jobCaps = JOB_CAPABILITY_MAP[job] ?? { required: [], preferred: [] };

  const explicitCapsRaw =
    url.searchParams.get("capabilities") ??
    url.searchParams.get("capability") ??
    (Array.isArray(bodyParams.capabilities)
      ? bodyParams.capabilities.join(",")
      : typeof bodyParams.capability === "string"
        ? bodyParams.capability
        : "");
  const explicitCaps = explicitCapsRaw
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const requiredCapabilities = [...new Set([...jobCaps.required, ...explicitCaps])];
  const preferredCapabilities = jobCaps.preferred;

  const regionParam = (url.searchParams.get("region") ?? (typeof bodyParams.region === "string" ? bodyParams.region : "")).trim();
  const excludeRaw =
    url.searchParams.get("exclude") ??
    (Array.isArray(bodyParams.exclude) ? bodyParams.exclude.join(",") : typeof bodyParams.exclude === "string" ? bodyParams.exclude : "");
  const excludeSet = new Set(
    excludeRaw
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean),
  );
  const includeStale =
    url.searchParams.get("include_stale") === "1" ||
    url.searchParams.get("include_stale") === "true" ||
    bodyParams.includeStale === true;
  const shardsRaw = Number(url.searchParams.get("shards") ?? bodyParams.shards ?? 1);
  const shardsCount = Number.isFinite(shardsRaw) && shardsRaw >= 1 ? Math.min(32, Math.floor(shardsRaw)) : 1;

  const { source, snapshot } = await readTelemetrySnapshot(env);
  const nowMs = Date.now();

  const ranked = snapshot.nodes
    .filter((node) => {
      if (!node.reachable || !node.healthy) return false;
      if (excludeSet.has(node.alias)) return false;
      if (!includeStale && isNodeStale(node, snapshot.updatedAt, snapshot.ttlSec, nowMs)) return false;
      if (requiredCapabilities.length > 0) {
        const caps = new Set(node.capabilities ?? []);
        if (!requiredCapabilities.every((req) => caps.has(req))) return false;
      }
      return true;
    })
    .map((node) => {
      const caps = new Set(node.capabilities ?? []);
      const preferredMatches = preferredCapabilities.filter((p) => caps.has(p)).length;
      const regionMatched = Boolean(regionParam && node.region === regionParam);
      const effectiveScore = Number(
        clamp(node.score + preferredMatches * 5 + (regionMatched ? 4 : 0), 0, 100).toFixed(2),
      );
      return {
        ...node,
        effectiveScore,
        regionMatched,
      };
    })
    .sort((a, b) => {
      if (b.effectiveScore !== a.effectiveScore) return b.effectiveScore - a.effectiveScore;
      const loadA = a.loadRatio ?? 0;
      const loadB = b.loadRatio ?? 0;
      if (loadA !== loadB) return loadA - loadB;
      const rttA = a.rttMs ?? 0;
      const rttB = b.rttMs ?? 0;
      if (rttA !== rttB) return rttA - rttB;
      return a.alias.localeCompare(b.alias);
    });

  const shards: Array<{
    shard: number;
    host: string;
    effectiveScore: number;
    region?: string;
    capabilities?: string[];
  }> = [];

  if (ranked.length > 0) {
    const virtualAssigned = new Map<string, number>();
    for (let i = 0; i < shardsCount; i++) {
      let best = ranked[0];
      let bestMarginal = -Infinity;
      for (const candidate of ranked) {
        const assigned = virtualAssigned.get(candidate.alias) ?? 0;
        const shardWeight = 24 * (1 + 2 / Math.max(2, candidate.cpus ?? 8));
        const marginal = candidate.effectiveScore - assigned * shardWeight;
        if (marginal > bestMarginal) {
          bestMarginal = marginal;
          best = candidate;
        }
      }
      virtualAssigned.set(best.alias, (virtualAssigned.get(best.alias) ?? 0) + 1);
      shards.push({
        shard: i,
        host: best.alias,
        effectiveScore: Number(clamp(bestMarginal, 0, 100).toFixed(2)),
        ...(best.region ? { region: best.region } : {}),
        ...(best.capabilities ? { capabilities: best.capabilities } : {}),
      });
    }
  }

  const selected = ranked[0] ?? null;
  const fallbackChain = ranked.slice(1).map((n) => n.alias);

  return jsonResponse({
    ok: true,
    service: "aphrody-worker",
    endpoint: "fleet/route",
    job,
    requiredCapabilities,
    source,
    updatedAt: snapshot.updatedAt,
    selected,
    fallbackChain,
    candidates: ranked,
    shards,
    reason: selected ? "selected_lightest_healthy_node" : "no_matching_telemetry_nodes",
  });
}

async function handleBeacon(request: Request, env: WorkerEnv): Promise<Response> {
  if (request.method !== "POST") {
    return new Response("Method Not Allowed", { status: 405, headers: { Allow: "POST" } });
  }

  const bodyBytes = await request.arrayBuffer();
  const auth = await verifyFleetBeaconAuth(env.FLEET_BEACON_TOKEN, request.headers, bodyBytes);
  if (!auth.ok) {
    return jsonResponse({ ok: false, error: "unauthorized", authMode: auth.mode }, 401);
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(decoder.decode(bodyBytes));
  } catch {
    return jsonResponse({ ok: false, error: "invalid_json" }, 400);
  }

  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    return jsonResponse({ ok: false, error: "invalid_payload" }, 400);
  }

  const payload = parsed as Record<string, unknown>;
  const nowIso = new Date().toISOString();
  const mergeMode = payload.merge === true;

  let incomingNodes: FleetNodeTelemetry[] = [];
  if (Array.isArray(payload.nodes)) {
    incomingNodes = payload.nodes
      .map((n) => normalizeNode(n, nowIso))
      .filter((n): n is FleetNodeTelemetry => n !== null);
  } else {
    const single = normalizeNode(payload, nowIso);
    if (single) incomingNodes = [single];
  }

  if (incomingNodes.length === 0) {
    return jsonResponse({ ok: false, error: "no_valid_nodes" }, 400);
  }

  const existing = mergeMode ? (await readTelemetrySnapshot(env)).snapshot : null;
  const nodeMap = new Map<string, FleetNodeTelemetry>();
  if (existing) {
    for (const prev of existing.nodes) {
      nodeMap.set(prev.alias, prev);
    }
  }
  for (const next of incomingNodes) {
    nodeMap.set(next.alias, next);
  }

  const mergedNodes = [...nodeMap.values()].sort((a, b) => b.score - a.score || a.alias.localeCompare(b.alias));
  const ttlSec =
    typeof payload.ttlSec === "number" && Number.isFinite(payload.ttlSec) && payload.ttlSec > 0
      ? payload.ttlSec
      : existing?.ttlSec ?? DEFAULT_STALE_TTL_SEC;

  const snapshot: FleetTelemetrySnapshot = {
    schemaVersion: 1,
    updatedAt: nowIso,
    ...(typeof payload.sourceHost === "string" && payload.sourceHost ? { sourceHost: payload.sourceHost } : {}),
    ttlSec,
    nodes: mergedNodes,
  };

  let persisted = false;
  const bucket = resolveWritableBucket(env);
  if (bucket?.put) {
    await bucket.put(FLEET_TELEMETRY_KEY, JSON.stringify(snapshot), {
      httpMetadata: { contentType: "application/json; charset=utf-8" },
    });
    persisted = true;
  }

  const healthyNodes = snapshot.nodes.filter((n) => n.reachable && n.healthy).length;

  return jsonResponse({
    ok: true,
    service: "aphrody-worker",
    endpoint: "fleet/beacon",
    key: FLEET_TELEMETRY_KEY,
    persisted,
    authenticated: auth.authenticated,
    authMode: auth.mode,
    updatedAt: snapshot.updatedAt,
    totalNodes: snapshot.nodes.length,
    healthyNodes,
    nodes: snapshot.nodes,
  });
}

export async function handleFleetRequest(request: Request, env: WorkerEnv): Promise<Response> {
  const url = new URL(request.url);
  const path = url.pathname;

  if (request.method === "OPTIONS") {
    return new Response(null, {
      status: 204,
      headers: {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET, POST, HEAD, OPTIONS",
        "Access-Control-Allow-Headers": "Content-Type, Authorization, X-Fleet-Signature-256",
      },
    });
  }

  if (path === "/api/fleet/status") {
    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("Method Not Allowed", { status: 405, headers: { Allow: "GET, HEAD" } });
    }
    return handleStatus(url, env);
  }

  if (path === "/api/fleet/route") {
    if (request.method !== "GET" && request.method !== "POST") {
      return new Response("Method Not Allowed", { status: 405, headers: { Allow: "GET, POST" } });
    }
    return handleRoute(request, url, env);
  }

  if (path === "/api/fleet/beacon") {
    return handleBeacon(request, env);
  }

  return new Response("Not Found", { status: 404, headers: { "Cache-Control": "no-store" } });
}
