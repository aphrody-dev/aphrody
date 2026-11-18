import type { WorkerEnv } from "./types.ts";

export type GithubEvent = {
  v: 1;
  delivery: string;
  event: string;
  action?: string;
  kind: "ci_red" | "push_main" | "release" | "security" | "installation";
  repo?: string;
  ref?: string;
  sha?: string;
  before?: string;
  url?: string;
  title?: string;
  actor?: string;
  at: string;
};

const enc = new TextEncoder();

export async function verify(secret: string | undefined, body: ArrayBuffer, header: string | null): Promise<boolean> {
  if (!secret || !header?.startsWith("sha256=")) return false;
  const key = await crypto.subtle.importKey("raw", enc.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  const mac = new Uint8Array(await crypto.subtle.sign("HMAC", key, body));
  const hex = header.slice(7);
  if (!/^[0-9a-f]{64}$/.test(hex)) return false;
  const got = new Uint8Array(mac.length);
  for (let i = 0; i < got.length; i++) got[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  if (mac.length !== got.length) return false;
  let diff = 0;
  for (let i = 0; i < got.length; i++) diff |= mac[i] ^ got[i];
  return diff === 0;
}

export const OWNERS = new Set(["aphrody-labs", "yolo-rs", "aphrody-dev"]);
const MAIN_EVENTS = new Set(["push", "schedule", "workflow_dispatch", "merge_group"]);
const FAIL = new Set(["failure", "timed_out", "startup_failure", "action_required"]);
const SECURITY = new Set(["code_scanning_alert", "dependabot_alert", "secret_scanning_alert", "repository_advisory"]);
const SECURITY_ACTIONS = new Set(["created", "reopened", "published", "reported"]);

export function classify(event: string, delivery: string, raw: unknown): GithubEvent | null {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
  const p = raw as Record<string, any>;
  const action = typeof p.action === "string" ? p.action : undefined;
  const base = {
    v: 1 as const,
    delivery,
    event,
    action,
    repo: typeof p.repository?.full_name === "string" ? p.repository.full_name : undefined,
    actor: typeof p.sender?.login === "string" ? p.sender.login : undefined,
    at: new Date().toISOString(),
  };
  const main = typeof p.repository?.default_branch === "string" ? p.repository.default_branch : "main";
  const owner = String(p.repository?.owner?.login ?? p.installation?.account?.login ?? "").toLowerCase();
  if (!OWNERS.has(owner)) return null;

  if (
    event === "workflow_run" &&
    action === "completed" &&
    FAIL.has(p.workflow_run?.conclusion) &&
    p.workflow_run?.head_branch === main &&
    MAIN_EVENTS.has(p.workflow_run?.event) &&
    p.workflow_run?.head_repository?.full_name === p.repository?.full_name &&
    typeof p.workflow_run?.head_sha === "string"
  ) {
    return {
      ...base,
      kind: "ci_red",
      ref: main,
      sha: p.workflow_run.head_sha,
      url: typeof p.workflow_run.html_url === "string" ? p.workflow_run.html_url : undefined,
      title: `${p.workflow_run.name ?? "workflow"} : ${p.workflow_run.conclusion}`,
    };
  }

  if (
    event === "push" &&
    p.ref === `refs/heads/${main}` &&
    !p.deleted &&
    typeof p.after === "string" &&
    p.after.length > 0 &&
    !/^0+$/.test(p.after)
  ) {
    const before = typeof p.before === "string" && !/^0+$/.test(p.before) ? p.before : undefined;
    return {
      ...base,
      kind: "push_main",
      ref: main,
      sha: p.after,
      before,
      url: typeof p.compare === "string" ? p.compare : undefined,
      title: String(p.head_commit?.message ?? "").split("\n")[0].slice(0, 200),
    };
  }

  if (event === "release" && action === "published") {
    return {
      ...base,
      kind: "release",
      ref: typeof p.release?.tag_name === "string" ? p.release.tag_name : undefined,
      url: typeof p.release?.html_url === "string" ? p.release.html_url : undefined,
      title: typeof p.release?.name === "string" ? p.release.name : typeof p.release?.tag_name === "string" ? p.release.tag_name : undefined,
    };
  }

  if (SECURITY.has(event) && action && SECURITY_ACTIONS.has(action)) {
    const a = p.alert ?? p.repository_advisory ?? {};
    const title = a?.rule?.description ?? a?.security_advisory?.summary ?? a?.secret_type_display_name ?? a?.summary ?? event;
    return {
      ...base,
      kind: "security",
      url: typeof a?.html_url === "string" ? a.html_url : undefined,
      title: String(title).slice(0, 200),
    };
  }

  if ((event === "installation" || event === "installation_repositories") && action) {
    return {
      ...base,
      kind: "installation",
      repo: typeof p.installation?.account?.login === "string" ? p.installation.account.login : undefined,
      title: `${event} ${action}`,
    };
  }

  return null;
}

export async function handleGithubRequest(request: Request, env: WorkerEnv): Promise<Response> {
  if (request.method !== "POST") return new Response(null, { status: 405, headers: { allow: "POST" } });

  const body = await request.arrayBuffer();
  if (!(await verify(env.GITHUB_WEBHOOK_SECRET, body, request.headers.get("x-hub-signature-256")))) {
    return new Response("Invalid signature", { status: 401 });
  }

  const event = request.headers.get("x-github-event") ?? "";
  const delivery = request.headers.get("x-github-delivery") ?? crypto.randomUUID();
  if (event === "ping") return Response.json({ ok: true, ping: true });

  let payload: unknown;
  try {
    payload = JSON.parse(new TextDecoder().decode(body));
  } catch {
    return new Response("Invalid JSON", { status: 400 });
  }

  const classified = classify(event, delivery, payload);
  if (classified) {
    if (!env.EVENTS) {
      return Response.json({ ok: false, queued: false, kind: classified.kind, error: "queue_unbound" }, { status: 503 });
    }
    try {
      await env.EVENTS.send(classified, { contentType: "json" });
    } catch (err) {
      console.error(JSON.stringify({ event: "github-events-send-failed", kind: classified.kind, delivery, error: err instanceof Error ? err.name : "Error" }));
      return Response.json({ ok: false, queued: false, kind: classified.kind, error: "queue_unavailable" }, { status: 503, headers: { "cache-control": "no-store" } });
    }
  }
  return Response.json({ queued: !!classified, kind: classified?.kind ?? null }, { status: 202 });
}
