#!/usr/bin/env bun
// Applies standard.toml to every non-archived repository of the orgs and users it names, through the GitHub REST and GraphQL APIs.
// Idempotent: every step reads the live state first and writes only the difference. Without --apply nothing is written (dry run).
//
// Usage:
//   bun tools/apply.ts                         dry run, all steps, prints what would change
//   bun tools/apply.ts --apply                 apply all steps
//   bun tools/apply.ts --apply --only=labels,rulesets
//   bun tools/apply.ts --repo=aphrody-labs/r2b[,owner/name]   restrict per-repo steps
//   bun tools/apply.ts --out=ledger.jsonl      where the change ledger goes (default: $TMPDIR/apply-<timestamp>.jsonl)
// Steps (run in this order): org, archive, settings, labels, security, rulesets, features, readme, forks, needs, templates (report only)
// A dry run directly after an apply must print zero "would" lines: that is the verification.
// Secrets are never printed: the token comes from GH_TOKEN, GITHUB_TOKEN or `gh auth token`.
import { tmpdir } from "node:os";
import { join } from "node:path";

type Json = any;
type Repo = Json;
type Status = "would" | "done" | "noop" | "skip" | "error" | "info";
interface Entry { repo: string; step: string; status: Status; detail: string; http?: number }

const ROOT = join(import.meta.dir, "..");
const std = Bun.TOML.parse(await Bun.file(join(ROOT, "standard.toml")).text()) as Json;
const args = new Map<string, string>();
for (const a of process.argv.slice(2)) {
  const m = /^--([^=]+)(?:=(.*))?$/.exec(a);
  if (m) args.set(m[1]!, m[2] ?? "true");
}
const APPLY = args.has("apply");
const ALL_STEPS = ["org", "archive", "settings", "labels", "security", "rulesets", "features", "readme", "forks", "needs", "templates"];
const STEPS = new Set((args.get("only") ?? ALL_STEPS.join(",")).split(","));
const ONLY_REPOS = args.get("repo")?.split(",");
const OUT = args.get("out") ?? join(tmpdir(), `apply-${new Date().toISOString().replace(/[:.]/g, "-")}.jsonl`);

// --- GitHub client --------------------------------------------------------------------------------
const token = process.env.GH_TOKEN ?? process.env.GITHUB_TOKEN ?? (await Bun.$`gh auth token`.text()).trim();
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
let lastWrite = 0;
let remaining = Infinity;

async function api(method: string, path: string, body?: unknown, tries = 4): Promise<{ status: number; data: Json; link?: string }> {
  const write = method !== "GET";
  for (let i = 0; i < tries; i++) {
    if (write) { const wait = lastWrite + 450 - Date.now(); lastWrite = Date.now() + Math.max(0, wait); if (wait > 0) await sleep(wait); }
    const res = await fetch(path.startsWith("http") ? path : `https://api.github.com${path}`, {
      method,
      headers: { authorization: `Bearer ${token}`, accept: "application/vnd.github+json", "x-github-api-version": "2022-11-28", "content-type": "application/json", "user-agent": "aphrody-apply" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    remaining = Number(res.headers.get("x-ratelimit-remaining") ?? remaining);
    const text = await res.text();
    let data: Json = text;
    try { data = text ? JSON.parse(text) : null; } catch { /* keep text */ }
    if ((res.status === 403 || res.status === 429) && i < tries - 1 && /rate limit|abuse|retry/i.test(typeof data === "string" ? data : (data?.message ?? ""))) {
      const ra = Number(res.headers.get("retry-after"));
      const reset = Number(res.headers.get("x-ratelimit-reset"));
      const ms = ra ? ra * 1000 : remaining === 0 && reset ? Math.max(1000, reset * 1000 - Date.now() + 2000) : 65_000;
      if (ms > 15 * 60_000) throw new Error(`rate limited for ${Math.round(ms / 60000)} min; rerun after reset`);
      console.error(`rate limit, waiting ${Math.round(ms / 1000)}s`);
      await sleep(ms);
      continue;
    }
    if (res.status >= 500 && i < tries - 1) { await sleep(1500 * (i + 1)); continue; }
    return { status: res.status, data, link: res.headers.get("link") ?? undefined };
  }
  throw new Error("unreachable");
}

async function paginate(path: string): Promise<Json[]> {
  const out: Json[] = [];
  let url: string | undefined = `${path}${path.includes("?") ? "&" : "?"}per_page=100`;
  while (url) {
    const r = await api("GET", url);
    if (r.status >= 400) throw new Error(`GET ${url} -> ${r.status} ${JSON.stringify(r.data).slice(0, 200)}`);
    const arr = Array.isArray(r.data) ? r.data : (Object.values(r.data).find(Array.isArray) as Json[] | undefined) ?? [];
    out.push(...arr);
    url = /<([^>]+)>;\s*rel="next"/.exec(r.link ?? "")?.[1];
  }
  return out;
}

async function graphql(query: string): Promise<Json> {
  const r = await api("POST", "/graphql", { query });
  if (r.status >= 400 || !r.data?.data) throw new Error(`graphql ${r.status} ${JSON.stringify(r.data).slice(0, 300)}`);
  return r.data.data;
}

// --- ledger -----------------------------------------------------------------------------------------
const ledger: Entry[] = [];
const outFile = Bun.file(OUT);
let outBuf = "";
function rec(repo: string, step: string, status: Status, detail: string, http?: number): void {
  const e: Entry = { repo, step, status, detail, ...(http ? { http } : {}) };
  ledger.push(e);
  outBuf += JSON.stringify(e) + "\n";
  console.log(`${status.padEnd(5)} ${step.padEnd(9)} ${repo.padEnd(44)} ${detail}`);
}

/** Runs a write in apply mode, records it as "would" in a dry run. Returns true when the change was made or would be made. */
async function change(repo: string, step: string, detail: string, method: string, path: string, body?: unknown, okStatuses: number[] = [200, 201, 202, 204]): Promise<boolean> {
  if (!APPLY) { rec(repo, step, "would", `${detail} [${method} ${path}]`); return true; }
  const r = await api(method, path, body);
  if (okStatuses.includes(r.status)) { rec(repo, step, "done", detail, r.status); return true; }
  rec(repo, step, "error", `${detail} -> ${r.status} ${JSON.stringify(r.data).slice(0, 220)}`, r.status);
  return false;
}

// --- inventory --------------------------------------------------------------------------------------
const orgName: string = std.meta.org;
const userName: string = std.meta.user;
const skip = new Set<string>(std.scope.skip_repos.map((s: string) => s.toLowerCase()));
const frozen: string[] = std.scope.frozen_domains;

async function inventory(): Promise<Repo[]> {
  const [org, user] = await Promise.all([paginate(`/orgs/${orgName}/repos?type=all`), paginate(`/user/repos?affiliation=owner`)]);
  const seen = new Map<string, Repo>();
  for (const r of [...org, ...user.filter((r) => r.owner.login === userName)]) seen.set(r.full_name.toLowerCase(), r);
  return [...seen.values()];
}

const all = await inventory();
const isSkipped = (r: Repo) => skip.has(r.full_name.toLowerCase());
const selected = (r: Repo) => !ONLY_REPOS || ONLY_REPOS.some((x) => x.toLowerCase() === r.full_name.toLowerCase());
const live = all.filter((r) => !r.archived);
const inScope = live.filter((r) => !isSkipped(r));
const originals = inScope.filter((r) => !r.fork && selected(r));
const forks = inScope.filter((r) => r.fork && selected(r));
console.log(`inventory: ${all.length} repos, ${live.length} live, ${inScope.length} in scope (${originals.length} originals, ${forks.length} forks), ${live.length - inScope.length} skipped; mode=${APPLY ? "APPLY" : "dry-run"}`);

const archivedNow = new Set<string>();   // archived by this run: later steps leave them alone
const isDone = (r: Repo) => archivedNow.has(r.full_name);

// Bounded parallelism for reads; writes serialize through the spacing in api().
async function pool<T>(items: T[], n: number, fn: (x: T) => Promise<void>): Promise<void> {
  let i = 0;
  await Promise.all(Array.from({ length: Math.min(n, items.length) }, async () => { while (i < items.length) { const x = items[i++]!; try { await fn(x); } catch (e) { rec("-", "pool", "error", String((e as Error).message).slice(0, 240)); } } }));
}

async function rootEntries(r: Repo): Promise<string[]> {
  const t = await api("GET", `/repos/${r.full_name}/git/trees/${encodeURIComponent(r.default_branch)}`);
  return t.status === 200 ? (t.data.tree as Json[]).map((x) => x.path as string) : [];
}
async function readText(r: Repo, path: string): Promise<string | undefined> {
  const c = await api("GET", `/repos/${r.full_name}/contents/${path}?ref=${encodeURIComponent(r.default_branch)}`);
  return c.status === 200 && c.data.content ? Buffer.from(c.data.content, "base64").toString("utf8") : undefined;
}

// GraphQL facts for every repo: immediate parent and the head commit's check suites (failed Actions run detection).
const gql = new Map<string, { parent?: string; failing: boolean }>();
async function loadGraphql(repos: Repo[]): Promise<void> {
  for (let i = 0; i < repos.length; i += 40) {
    const chunk = repos.slice(i, i + 40);
    const q = chunk.map((r, k) => `r${k}: repository(owner: ${JSON.stringify(r.owner.login)}, name: ${JSON.stringify(r.name)}) { parent { nameWithOwner } defaultBranchRef { target { ... on Commit { checkSuites(first: 15) { nodes { conclusion app { slug } } } } } } }`).join("\n");
    const d = await graphql(`{ ${q} }`);
    chunk.forEach((r, k) => {
      const x = d[`r${k}`];
      const suites: Json[] = x?.defaultBranchRef?.target?.checkSuites?.nodes ?? [];
      gql.set(r.full_name, { parent: x?.parent?.nameWithOwner, failing: suites.some((s) => s.app?.slug === "github-actions" && s.conclusion === "FAILURE") });
    });
  }
}

// Role of an original, derived from [profile.roles]; first match wins, the fallback role last.
function roleOf(r: Repo): string {
  for (const role of std.profile.roles as Json[]) {
    if ((role.repos ?? []).some((x: string) => x.toLowerCase() === r.full_name.toLowerCase())) return role.id;
    if ((role.patterns ?? []).some((p: string) => new RegExp(p).test(r.name))) return role.id;
  }
  return (std.profile.roles as Json[]).find((x) => x.fallback_for_originals)!.id;
}
const langTopic = (l: string | null): string | undefined => l ? l.toLowerCase().replace(/\+\+/g, "pp").replace(/#/g, "sharp").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") : undefined;
const gatedSet = new Set<string>(std.rulesets.gated_repos.map((s: string) => s.toLowerCase()));
const vendoredSet = new Set<string>((std.security.vendored?.repos ?? []).map((s: string) => s.toLowerCase()));
const isVendored = (r: Repo): boolean => vendoredSet.has(r.full_name.toLowerCase()) || (r.topics ?? []).includes(std.security.vendored?.topic);

// ====================================================================================================
// org: Actions workflow permissions, security configurations and their defaults
// ====================================================================================================
const CFG_FIELDS = ["name", "description", "advanced_security", "dependency_graph", "dependabot_alerts", "dependabot_security_updates", "code_scanning_default_setup", "secret_scanning", "secret_scanning_push_protection", "secret_scanning_non_provider_patterns", "secret_scanning_validity_checks", "private_vulnerability_reporting", "enforcement"];
const cfgIds: Record<string, number> = {};

async function ensureConfig(key: "originals" | "forks"): Promise<void> {
  const want = std.security.configurations[key];
  const body: Json = {};
  for (const f of CFG_FIELDS) if (f in want) body[f] = want[f];
  const list = await paginate(`/orgs/${orgName}/code-security/configurations`);
  const cur = list.find((c) => c.name === want.name);
  if (!cur) {
    if (!APPLY) { rec(orgName, "org", "would", `create security configuration ${want.name}`); return; }
    const r = await api("POST", `/orgs/${orgName}/code-security/configurations`, body);
    if (r.status === 201) { cfgIds[key] = r.data.id; rec(orgName, "org", "done", `created security configuration ${want.name} id=${r.data.id}`, r.status); }
    else rec(orgName, "org", "error", `create ${want.name} -> ${r.status} ${JSON.stringify(r.data).slice(0, 220)}`, r.status);
    return;
  }
  cfgIds[key] = cur.id;
  const diff = Object.keys(body).filter((k) => cur[k] !== body[k]);
  if (!diff.length) { rec(orgName, "org", "noop", `security configuration ${want.name} id=${cur.id} matches`); return; }
  await change(orgName, "org", `update security configuration ${want.name} (${diff.join(", ")})`, "PATCH", `/orgs/${orgName}/code-security/configurations/${cur.id}`, body);
}

async function stepOrg(): Promise<void> {
  const a = std.actions.org;
  const wf = (await api("GET", `/orgs/${orgName}/actions/permissions/workflow`)).data;
  const wantWf = { default_workflow_permissions: a.default_workflow_permissions, can_approve_pull_request_reviews: a.can_approve_pull_request_reviews };
  if (wf.default_workflow_permissions === wantWf.default_workflow_permissions && wf.can_approve_pull_request_reviews === wantWf.can_approve_pull_request_reviews) rec(orgName, "org", "noop", "actions workflow permissions already read, no PR approval");
  else await change(orgName, "org", `actions default permissions ${wf.default_workflow_permissions}->${wantWf.default_workflow_permissions}, can_approve ${wf.can_approve_pull_request_reviews}->${wantWf.can_approve_pull_request_reviews}`, "PUT", `/orgs/${orgName}/actions/permissions/workflow`, wantWf);
  // The allow-list (allowed_actions = selected) is not applied here: it needs the union of third-party actions used by every original (see "needs").
  rec(orgName, "org", "info", `allowed_actions stays ${(await api("GET", `/orgs/${orgName}/actions/permissions`)).data.allowed_actions}; selected allow-list needs a full workflow scan first`);
  await ensureConfig("originals");
  await ensureConfig("forks");
  if (cfgIds.forks) {
    const d = await api("GET", `/orgs/${orgName}/code-security/configurations/defaults`);
    const has = Array.isArray(d.data) && d.data.some((x: Json) => x.configuration?.id === cfgIds.forks && x.default_for_new_repos === "public");
    if (has) rec(orgName, "org", "noop", "aphrody-forks already default for new public repositories");
    else await change(orgName, "org", "aphrody-forks becomes default for new public repositories", "PUT", `/orgs/${orgName}/code-security/configurations/${cfgIds.forks}/defaults`, { default_for_new_repos: std.security.configurations.forks.default_for_new_repos });
  }
}

// ====================================================================================================
// archive: proven duplicates (same upstream, nothing unique) and dead originals (standard section 1)
// ====================================================================================================
async function gitmodulesRefs(): Promise<Set<string>> {
  const refs = new Set<string>();
  await pool(live.filter((r) => !r.fork), 6, async (r) => {
    const t = await readText(r, ".gitmodules");
    for (const m of t?.matchAll(/url\s*=\s*(\S+)/g) ?? []) {
      const u = m[1]!.replace(/\.git$/, "");
      refs.add(u.split(/[/:]/).slice(-2).join("/").toLowerCase());
      refs.add(u.split("/").pop()!.toLowerCase());   // relative urls ("../name.git")
    }
  });
  return refs;
}

async function stepArchive(): Promise<void> {
  await loadGraphql(live.filter((r) => r.fork));
  const refs = await gitmodulesRefs();
  const byParent = new Map<string, Repo[]>();
  for (const r of live.filter((x) => x.fork && !isSkipped(x))) {
    const p = gql.get(r.full_name)?.parent;
    if (p) byParent.set(p, [...(byParent.get(p) ?? []), r]);
  }
  const toArchive = new Map<string, string>();
  for (const [parent, group] of byParent) {
    if (group.length < 2) continue;
    const referenced = (r: Repo) => refs.has(r.full_name.toLowerCase()) || refs.has(r.name.toLowerCase());
    const upstreamName = parent.split("/")[1]!.toLowerCase();
    const rank = (r: Repo) => (referenced(r) ? 4 : 0) + (r.name.toLowerCase() === upstreamName ? 2 : 0) + (r.owner.login === userName ? 1 : 0);
    const sorted = [...group].sort((a, b) => rank(b) - rank(a) || Date.parse(b.pushed_at) - Date.parse(a.pushed_at) || a.name.length - b.name.length);
    const keeper = sorted[0]!;
    for (const dup of sorted.slice(1)) {
      if (referenced(dup)) { rec(dup.full_name, "archive", "skip", `same upstream ${parent} as ${keeper.full_name} but referenced as a submodule`); continue; }
      const base = (await api("GET", `/repos/${keeper.full_name}`)).data;
      const cmp = await api("GET", `/repos/${keeper.full_name}/compare/${encodeURIComponent(base.default_branch)}...${dup.owner.login}:${encodeURIComponent(dup.default_branch)}`);
      if (cmp.status !== 200) { rec(dup.full_name, "archive", "skip", `cannot compare with ${keeper.full_name} (${cmp.status})`); continue; }
      if (cmp.data.ahead_by === 0) toArchive.set(dup.full_name, `${cmp.data.status === "identical" ? "identical to" : `strict subset of (${cmp.data.behind_by} commits behind)`} ${keeper.full_name}, same upstream ${parent}, no submodule reference`);
      else {
        // The ranked keeper may be the subset: when it has nothing the other copy lacks, the keeper is the redundant side.
        const rev = await api("GET", `/repos/${dup.full_name}/compare/${encodeURIComponent(dup.default_branch)}...${keeper.owner.login}:${encodeURIComponent(base.default_branch)}`);
        if (rev.status === 200 && rev.data.ahead_by === 0 && !referenced(keeper)) toArchive.set(keeper.full_name, `strict subset of (${rev.data.behind_by} commits behind) ${dup.full_name}, same upstream ${parent}, no submodule reference`);
        else rec(dup.full_name, "archive", "skip", `has ${cmp.data.ahead_by} commits not in ${keeper.full_name}: not a duplicate`);
      }
    }
  }
  // Dead originals: no push for dead_days, no stars, forks, issues or pull requests. Superseded is checked by hand, so this only lists.
  const deadDays: number = std.archive?.dead_days ?? 30;
  for (const r of originals) {
    const idleDays = (Date.now() - Date.parse(r.pushed_at)) / 86_400_000;
    if (idleDays >= deadDays && r.open_issues_count === 0 && r.stargazers_count === 0 && r.forks_count === 0 && !refs.has(r.full_name.toLowerCase()) && !refs.has(r.name.toLowerCase()))
      toArchive.set(r.full_name, `no push for ${Math.floor(idleDays)} days (>= ${deadDays}), no issues, PRs, stars, forks or references`);
  }
  if (!toArchive.size) rec("-", "archive", "info", "no dead original reaches the inactivity threshold");
  for (const r of live) {
    const why = toArchive.get(r.full_name);
    if (!why || !selected(r)) continue;
    if (await change(r.full_name, "archive", `archive: ${why}`, "PATCH", `/repos/${r.full_name}`, { archived: true })) archivedNow.add(r.full_name);
  }
}

// ====================================================================================================
// settings: description, homepage, topics, merge and feature toggles (originals), toggles only (forks)
// ====================================================================================================
const wikiIsEmpty = async (r: Repo): Promise<boolean> => {
  const url = `https://github.com/${r.full_name}.wiki.git`;
  const p = Bun.spawn(["git", "ls-remote", url], { stdout: "pipe", stderr: "pipe", env: { ...process.env, GIT_TERMINAL_PROMPT: "0" } });
  const out = await new Response(p.stdout).text();
  await p.exited;
  return out.trim() === "";
};

async function derive(r: Repo, entries: string[]): Promise<{ description?: string; homepage?: string }> {
  const out: { description?: string; homepage?: string } = {};
  if (entries.includes("package.json")) {
    try { const j = JSON.parse((await readText(r, "package.json")) ?? "{}"); if (typeof j.description === "string") out.description = j.description; if (typeof j.homepage === "string") out.homepage = j.homepage; } catch { /* not JSON */ }
  }
  if (!out.description && entries.includes("Cargo.toml")) {
    try { const t = Bun.TOML.parse((await readText(r, "Cargo.toml")) ?? "") as Json; const p = t.package ?? t.workspace?.package ?? {}; if (typeof p.description === "string") out.description = p.description; if (typeof p.homepage === "string") out.homepage = out.homepage ?? p.homepage; } catch { /* not TOML */ }
  }
  const readme = entries.find((e) => /^readme(\.md)?$/i.test(e));
  if (!out.description && readme) {
    const t = await readText(r, readme);
    const line = t?.split("\n").map((l) => l.trim()).find((l) => l && !/^(#|<|\[|!|\||```|---|=)/.test(l));
    if (line && line.length >= 10 && line.length <= 160 && /^[\x20-\x7e]+$/.test(line)) out.description = line.replace(/\s+/g, " ");
  }
  return out;
}

async function stepSettings(): Promise<void> {
  const want = std.repo_settings.original;
  const topicOk = /^[a-z0-9][a-z0-9-]{0,49}$/;
  await pool(originals.filter((r) => !isDone(r)), 4, async (r) => {
    const cur = (await api("GET", `/repos/${r.full_name}`)).data;
    const entries = await rootEntries(r);
    const patch: Json = {};
    // has_discussions is decided below (per-repository list); auto-merge is unavailable on private repositories of the free plan.
    const flags = ["has_issues", "has_projects", "delete_branch_on_merge", ...(r.private ? [] : ["allow_auto_merge"]), "allow_update_branch", "allow_squash_merge", "allow_rebase_merge", "allow_merge_commit", "squash_merge_commit_title", "squash_merge_commit_message", "web_commit_signoff_required"];
    for (const f of flags) if (f in want && cur[f] !== want[f]) patch[f] = want[f];
    const wantDisc = std.discussions.enable_on.map((s: string) => s.toLowerCase()).includes(r.full_name.toLowerCase());
    if (cur.has_discussions !== wantDisc) patch.has_discussions = wantDisc;
    // Wiki off only when it is empty; a wiki with content is kept and listed.
    if (cur.has_wiki && want.has_wiki === false) {
      if (await wikiIsEmpty(r)) patch.has_wiki = false;
      else rec(r.full_name, "settings", "skip", "wiki has content: kept (move to docs/ first)");
    }
    const d = await derive(r, entries);
    if (!cur.description && d.description) patch.description = d.description;
    else if (!cur.description) rec(r.full_name, "settings", "info", "description missing and no manifest or README line to derive it from: owner input");
    if (!cur.homepage && d.homepage && /^https?:\/\//.test(d.homepage) && d.homepage !== cur.html_url) patch.homepage = d.homepage;
    else if (!cur.homepage && cur.has_pages) {
      const pg = (await api("GET", `/repos/${r.full_name}/pages`)).data;
      if (pg?.html_url) patch.homepage = pg.html_url;
    }
    if (Object.keys(patch).length) await change(r.full_name, "settings", `patch ${Object.keys(patch).map((k) => `${k}=${String(patch[k]).slice(0, 40)}`).join(", ")}`, "PATCH", `/repos/${r.full_name}`, patch);
    else rec(r.full_name, "settings", "noop", "repository settings match");

    const topics = new Set<string>(cur.topics ?? []);
    topics.add(`aphrody-${roleOf(r)}`);
    const lt = langTopic(cur.language);
    if (lt && topicOk.test(lt)) topics.add(lt);
    if (gatedSet.has(r.full_name.toLowerCase())) topics.add(std.rulesets.gated_topic);
    if (vendoredSet.has(r.full_name.toLowerCase())) topics.add(std.security.vendored.topic);
    const next = [...topics].filter((t) => topicOk.test(t)).sort().slice(0, 20);
    const have = [...(cur.topics ?? [])].sort();
    if (next.join() !== have.join()) await change(r.full_name, "settings", `topics ${have.length}->${next.length}: ${next.join(",")}`, "PUT", `/repos/${r.full_name}/topics`, { names: next });
    else rec(r.full_name, "settings", "noop", "topics match");
  });
}

// ====================================================================================================
// labels
// ====================================================================================================
async function stepLabels(): Promise<void> {
  const labels: Json[] = std.labels;
  const rename: Record<string, string> = std.labels_rename;
  await pool(originals.filter((r) => !isDone(r) && r.has_issues !== false && r.default_branch === std.naming.branch_default), 3, async (r) => {
    const cur = await paginate(`/repos/${r.full_name}/labels`);
    const byName = new Map<string, Json>(cur.map((l) => [l.name.toLowerCase(), l]));
    let changes = 0;
    for (const [from, to] of Object.entries(rename)) {
      const src = byName.get(from.toLowerCase());
      if (src && !byName.has(to.toLowerCase())) {
        const want = labels.find((l) => l.name === to)!;
        if (await change(r.full_name, "labels", `rename ${from} -> ${to}`, "PATCH", `/repos/${r.full_name}/labels/${encodeURIComponent(src.name)}`, { new_name: to, color: want.color, description: want.description })) { byName.delete(from.toLowerCase()); byName.set(to.toLowerCase(), { name: to, color: want.color, description: want.description }); changes++; }
      }
    }
    for (const l of labels) {
      const have = byName.get(l.name.toLowerCase());
      if (!have) { if (await change(r.full_name, "labels", `create ${l.name}`, "POST", `/repos/${r.full_name}/labels`, { name: l.name, color: l.color, description: l.description })) changes++; }
      else if (have.name !== l.name || have.color.toLowerCase() !== l.color || (have.description ?? "") !== l.description) { if (await change(r.full_name, "labels", `update ${l.name}`, "PATCH", `/repos/${r.full_name}/labels/${encodeURIComponent(have.name)}`, { new_name: l.name, color: l.color, description: l.description })) changes++; }
    }
    if (!changes) rec(r.full_name, "labels", "noop", `${labels.length} labels match`);
  });
}

// ====================================================================================================
// security: org configurations for public repositories, per-repository calls as the fallback and for private repositories
// ====================================================================================================
async function attach(key: "originals" | "forks", repos: Repo[]): Promise<void> {
  const want = std.security.configurations[key].name;
  if (!cfgIds[key]) cfgIds[key] = (await paginate(`/orgs/${orgName}/code-security/configurations`)).find((c) => c.name === want)?.id;
  const id = cfgIds[key];
  if (!id) { rec(orgName, "security", APPLY ? "error" : "would", `configuration ${want} not available yet, attach ${repos.length} repositories after creating it`); return; }
  const attached = new Set<number>((await paginate(`/orgs/${orgName}/code-security/configurations/${id}/repositories`)).filter((x) => ["attached", "attaching", "enforced", "failed"].includes(x.status)).map((x) => x.repository.id));
  const todo = repos.filter((r) => !attached.has(r.id) && r.owner.login === orgName);
  if (!todo.length) { rec(orgName, "security", "noop", `${repos.length} repositories already attached to ${want}`); return; }
  for (let i = 0; i < todo.length; i += 200) {
    const chunk = todo.slice(i, i + 200);
    const detail = `attach ${want} to ${chunk.length} repositories (${chunk.slice(0, 3).map((r) => r.name).join(", ")}...)`;
    const path = `/orgs/${orgName}/code-security/configurations/${id}/attach`;
    if (!APPLY) { await change(orgName, "security", detail, "POST", path); continue; }
    // GitHub applies one enablement at a time per organization (409 while another is running): wait and retry.
    for (let k = 0; k < 20; k++) {
      const r = await api("POST", path, { scope: "selected", selected_repository_ids: chunk.map((x) => x.id) });
      if (r.status === 202 || r.status === 200) { rec(orgName, "security", "done", detail, r.status); break; }
      if (r.status === 409 && k < 19) { await sleep(30_000); continue; }
      rec(orgName, "security", "error", `${detail} -> ${r.status} ${JSON.stringify(r.data).slice(0, 200)}`, r.status);
      break;
    }
  }
}

async function perRepoSecurity(r: Repo): Promise<void> {
  const pub = r.visibility === "public";
  const want = isVendored(r) ? std.security.forks : pub ? std.security.originals_public : std.security.originals_private;
  const cur = (await api("GET", `/repos/${r.full_name}`)).data;
  const sa = cur.security_and_analysis ?? {};
  const body: Json = {};
  if (pub) {
    if (sa.secret_scanning?.status !== "enabled") body.secret_scanning = { status: "enabled" };
    if (sa.secret_scanning_push_protection?.status !== "enabled") body.secret_scanning_push_protection = { status: "enabled" };
  }
  if (Object.keys(body).length) await change(r.full_name, "security", `enable ${Object.keys(body).join(", ")}`, "PATCH", `/repos/${r.full_name}`, { security_and_analysis: body });
  if (want.dependabot_alerts) {
    const v = await api("GET", `/repos/${r.full_name}/vulnerability-alerts`);
    if (v.status !== 204) await change(r.full_name, "security", "enable Dependabot alerts", "PUT", `/repos/${r.full_name}/vulnerability-alerts`);
  }
  if (want.dependabot_security_updates) {
    const v = await api("GET", `/repos/${r.full_name}/automated-security-fixes`);
    if (!v.data?.enabled) await change(r.full_name, "security", "enable Dependabot security updates", "PUT", `/repos/${r.full_name}/automated-security-fixes`);
  } else if (want.dependabot_security_updates === false) {
    const v = await api("GET", `/repos/${r.full_name}/automated-security-fixes`);
    if (v.data?.enabled) await change(r.full_name, "security", "disable Dependabot security updates (vendored trees)", "DELETE", `/repos/${r.full_name}/automated-security-fixes`);
  }
  if (pub && want.private_vulnerability_reporting) {
    const v = await api("GET", `/repos/${r.full_name}/private-vulnerability-reporting`);
    if (!v.data?.enabled) await change(r.full_name, "security", "enable private vulnerability reporting", "PUT", `/repos/${r.full_name}/private-vulnerability-reporting`);
  }
  if (pub && want.code_scanning_default_setup === "configured") {
    const v = await api("GET", `/repos/${r.full_name}/code-scanning/default-setup`);
    if (v.status === 200 && v.data.state === "configured") return;
    const langs: string[] = v.data?.languages ?? [];
    // CodeQL only runs for languages it supports; a repository without one is recorded, not forced.
    if (v.status === 200 && langs.length === 0 && v.data.state === "not-configured" && v.data.languages !== undefined) { rec(r.full_name, "security", "skip", "CodeQL default setup: no supported language detected"); return; }
    await change(r.full_name, "security", "CodeQL default setup", "PATCH", `/repos/${r.full_name}/code-scanning/default-setup`, { state: "configured", query_suite: std.security.originals_public.code_scanning_query_suite });
  }
}

async function stepSecurity(): Promise<void> {
  const pub = originals.filter((r) => r.visibility === "public" && !isDone(r));
  await attach("originals", pub.filter((r) => !isVendored(r)));
  const vendored = pub.filter(isVendored);
  if (vendored.length) await attach("forks", vendored);
  if (APPLY) await sleep(20_000);   // configuration attach is asynchronous
  await pool(originals.filter((r) => !isDone(r)), 4, perRepoSecurity);
}

// ====================================================================================================
// rulesets: tier from gated_repos; default branch must be the standard one (mirrors with other default branches are listed, not changed)
// ====================================================================================================
const stable = (v: unknown): string => JSON.stringify(v, (_k, x) => (x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.entries(x).sort(([a], [b]) => a.localeCompare(b))) : x));

async function stepRulesets(): Promise<void> {
  const rs = std.rulesets;
  await pool(originals.filter((r) => !isDone(r)), 3, async (r) => {
    if (r.default_branch !== std.naming.branch_default) { rec(r.full_name, "rulesets", "skip", `default branch is ${r.default_branch}: mirror, no ruleset`); return; }
    const gated = gatedSet.has(r.full_name.toLowerCase()) || (r.topics ?? []).includes(rs.gated_topic);
    let names: string[] = [...rs.tiers.solo];
    if (gated) {
      // The required check name must come from a real run before it is enforced.
      const cr = await api("GET", `/repos/${r.full_name}/commits/${r.default_branch}/check-runs?per_page=100`);
      const seen = new Set<string>((cr.data?.check_runs ?? []).map((c: Json) => c.name));
      if (seen.has(std.actions.reusable.required_check)) names = [...rs.tiers.gated];
      else rec(r.full_name, "rulesets", "skip", `main-gated deferred: check "${std.actions.reusable.required_check}" has not run on ${r.default_branch} yet (adopt the ci caller first)`);
    }
    const existing = await api("GET", `/repos/${r.full_name}/rulesets?includes_parents=false`);
    if (existing.status === 403 || existing.status === 404) { rec(r.full_name, "rulesets", "skip", `rulesets unavailable (${existing.status}): ${existing.data?.message ?? ""}`.slice(0, 200), existing.status); return; }
    for (const name of names) {
      const body = JSON.parse(rs[name].body);
      const have = (existing.data as Json[]).find((x) => x.name === name);
      if (!have) { await change(r.full_name, "rulesets", `create ${name}`, "POST", `/repos/${r.full_name}/rulesets`, body); continue; }
      const detail = (await api("GET", `/repos/${r.full_name}/rulesets/${have.id}`)).data;
      const same = stable(detail.rules.map((x: Json) => ({ type: x.type, parameters: x.parameters }))) === stable(body.rules.map((x: Json) => ({ type: x.type, parameters: x.parameters }))) && detail.enforcement === body.enforcement && stable(detail.conditions) === stable(body.conditions) && stable(detail.bypass_actors ?? []) === stable(body.bypass_actors);
      if (same) rec(r.full_name, "rulesets", "noop", `${name} id=${have.id} matches`);
      else await change(r.full_name, "rulesets", `update ${name} id=${have.id}`, "PUT", `/repos/${r.full_name}/rulesets/${have.id}`, body);
    }
  });
}

// ====================================================================================================
// features: Pages only with a docs or site build and a pages.yml caller (workflow source); Discussions and wiki are handled in settings
// ====================================================================================================
async function stepFeatures(): Promise<void> {
  await pool(originals.filter((r) => r.has_pages && !isDone(r)), 3, async (r) => {
    const pg = (await api("GET", `/repos/${r.full_name}/pages`)).data;
    const wf = await api("GET", `/repos/${r.full_name}/contents/.github/workflows`);
    let hasCaller = false;
    for (const f of (Array.isArray(wf.data) ? wf.data : []) as Json[]) {
      const t = await readText(r, f.path);
      if (t && /aphrody-labs\/\.github\/\.github\/workflows\/pages\.yml@/.test(t)) hasCaller = true;
    }
    if (hasCaller && pg.build_type === "workflow") { rec(r.full_name, "features", "noop", "Pages on workflow source with pages.yml caller"); return; }
    await change(r.full_name, "features", `disable Pages (build_type=${pg.build_type}, no pages.yml caller; ${pg.html_url ?? ""})`, "DELETE", `/repos/${r.full_name}/pages`);
  });
}

// ====================================================================================================
// readme: the standard skeleton where no README exists (default branch must be the standard one)
// ====================================================================================================
function readmeSkeleton(r: Repo, entries: string[]): string {
  const tpl = std.meta.org;
  const has = (f: string) => entries.includes(f);
  const install: string[] = [];
  const dev: string[] = [];
  if (has("package.json")) { install.push("bun add " + r.name); dev.push("bun install --frozen-lockfile", "bun run lint && bun run typecheck && bun test"); }
  if (has("Cargo.toml")) { install.push(`cargo add ${r.name}`); dev.push("cargo fmt --check && cargo clippy && cargo test"); }
  if (has("go.mod")) { install.push(`go get github.com/${r.full_name}`); dev.push("go vet ./... && go test ./..."); }
  const sh = (l: string[], fallback: string) => "```sh\n" + (l.length ? l.join("\n") : fallback) + "\n```";
  const license = r.license?.spdx_id && r.license.spdx_id !== "NOASSERTION" ? `${r.license.spdx_id}, see [LICENSE](LICENSE).` : "See the repository for the license terms (a LICENSE file is pending).";
  return `# ${r.name}

${r.description ?? "One sentence: what it is and who it is for."}

[![license](https://img.shields.io/github/license/${r.full_name})](LICENSE)

## Install

${sh(install, `git clone https://github.com/${r.full_name}.git`)}

## Usage

See the documentation under [docs/](docs) when it exists, or the source entry points.

## Development

${sh(dev, "# no build step documented yet")}

Contributions follow the [organization guide](https://github.com/${tpl}/.github/blob/main/CONTRIBUTING.md). Report vulnerabilities as described in the [security policy](https://github.com/${tpl}/.github/blob/main/SECURITY.md).

## License

${license}
`;
}

async function stepReadme(): Promise<void> {
  await pool(originals.filter((r) => !isDone(r)), 4, async (r) => {
    const entries = await rootEntries(r);
    if (entries.some((e) => /^readme(\.|$)/i.test(e))) { rec(r.full_name, "readme", "noop", "README exists"); return; }
    if (r.default_branch !== std.naming.branch_default) { rec(r.full_name, "readme", "skip", `no README but default branch is ${r.default_branch} (mirror): not committed`); return; }
    if (r.private) { rec(r.full_name, "readme", "skip", "private repository without README: not committed"); return; }
    const body = { message: "docs: add README skeleton from the Aphrody Labs standard", content: Buffer.from(readmeSkeleton(r, entries)).toString("base64"), branch: r.default_branch };
    await change(r.full_name, "readme", `commit README.md skeleton to ${r.default_branch}`, "PUT", `/repos/${r.full_name}/contents/README.md`, body);
  });
}

// ====================================================================================================
// forks: org security configuration, issue/wiki/project toggles, Actions disabled where the head commit's workflows fail
// ====================================================================================================
async function stepForks(): Promise<void> {
  const want = std.repo_settings.fork;
  const orgForks = forks.filter((r) => r.owner.login === orgName && r.visibility === "public" && !isDone(r));
  await attach("forks", orgForks);
  const patchFor = (r: Repo): Json => {
    const p: Json = {};
    for (const f of ["has_issues", "has_projects", "has_wiki", "has_discussions"]) if (f in want && r[f] !== want[f]) p[f] = want[f];
    return p;
  };
  for (const r of forks.filter((x) => !isDone(x))) {
    const p = patchFor(r);
    if (!Object.keys(p).length) continue;
    if (p.has_wiki === false && !(await wikiIsEmpty(r))) { delete p.has_wiki; rec(r.full_name, "forks", "skip", "fork wiki has content: kept"); }
    if (Object.keys(p).length) await change(r.full_name, "forks", `patch ${Object.keys(p).join(", ")}`, "PATCH", `/repos/${r.full_name}`, p);
  }
  // User-owned forks have no org configuration: enable the same features per repository.
  for (const r of forks.filter((x) => x.owner.login !== orgName && x.visibility === "public")) {
    const sa = (await api("GET", `/repos/${r.full_name}`)).data.security_and_analysis ?? {};   // the user repository list omits it
    const body: Json = {};
    if (sa.secret_scanning?.status !== "enabled") body.secret_scanning = { status: "enabled" };
    if (sa.secret_scanning_push_protection?.status !== "enabled") body.secret_scanning_push_protection = { status: "enabled" };
    if (Object.keys(body).length) await change(r.full_name, "forks", `enable ${Object.keys(body).join(", ")}`, "PATCH", `/repos/${r.full_name}`, { security_and_analysis: body });
    const v = await api("GET", `/repos/${r.full_name}/vulnerability-alerts`);
    if (v.status !== 204) await change(r.full_name, "forks", "enable Dependabot alerts", "PUT", `/repos/${r.full_name}/vulnerability-alerts`);
  }
  await loadGraphql(forks);
  for (const r of forks) {
    if (!gql.get(r.full_name)?.failing) continue;
    const a = (await api("GET", `/repos/${r.full_name}/actions/permissions`)).data;
    if (a?.enabled === false) { rec(r.full_name, "forks", "noop", "Actions already disabled"); continue; }
    await change(r.full_name, "forks", "disable inherited upstream workflows (head commit workflow run failed)", "PUT", `/repos/${r.full_name}/actions/permissions`, { enabled: false });
  }
}

// ====================================================================================================
// needs: what the excluded repositories would need (read only, never written)
// ====================================================================================================
async function stepNeeds(): Promise<void> {
  for (const r of live.filter(isSkipped)) {
    const needs: string[] = [];
    const sa = r.security_and_analysis ?? {};
    if (!r.description) needs.push("description");
    if (!r.homepage) needs.push("homepage");
    if (!(r.topics ?? []).length) needs.push("topics");
    if (r.has_wiki) needs.push("wiki off if empty");
    if (r.visibility === "public") {
      if (sa.secret_scanning?.status !== "enabled") needs.push("secret scanning");
      if (sa.secret_scanning_push_protection?.status !== "enabled") needs.push("push protection");
    }
    const rs = await api("GET", `/repos/${r.full_name}/rulesets?includes_parents=false`);
    const rnames = Array.isArray(rs.data) ? rs.data.map((x: Json) => x.name) : [];
    for (const n of std.rulesets.tiers.solo) if (!rnames.includes(n)) needs.push(`ruleset ${n}`);
    const entries = await rootEntries(r);
    for (const f of std.files.original_required) if (!f.includes("/") && !entries.includes(f) && !(f === "README.md" && entries.some((e) => /^readme/i.test(e)))) needs.push(`file ${f}`);
    const suggests: string[] = [];
    for (const t of std.files.templates ?? []) {
      if (t.mode !== "copy" || (t.when && !entries.includes(t.when)) || t.dest.includes("/")) continue;
      if (!entries.includes(t.dest)) suggests.push(t.dest);
    }
    rec(r.full_name, "needs", "info", (needs.length ? `would need: ${needs.join(", ")}` : "meets the standard") + (suggests.length ? `; suggested from templates/: ${suggests.join(", ")}` : ""));
  }
  rec("-", "needs", "info", `frozen domains (inventory only, never changed): ${frozen.join(", ")}`);
}

// ====================================================================================================
// templates: report only. Which original would receive which root file copied from templates/ (files.templates, mode copy).
// Never writes, with or without --apply: copying waits until each repository's owner session is free.
// ====================================================================================================
async function stepTemplates(): Promise<void> {
  const tpls = ((std.files.templates ?? []) as Json[]).filter((x) => x.mode === "copy" && !String(x.dest).includes("/"));
  const targets = [...originals, ...live.filter((r) => isSkipped(r) && !r.fork)];
  const seen = new Set<string>();
  await pool(targets.filter((r) => !seen.has(r.full_name) && seen.add(r.full_name)), 4, async (r) => {
    const entries = await rootEntries(r);
    const missing = tpls.filter((x) => (!x.when || entries.includes(x.when)) && !entries.includes(x.dest)).map((x) => `${x.source} -> ${x.dest}`);
    rec(r.full_name, "templates", "info", missing.length ? `would receive: ${missing.join(", ")}${isSkipped(r) ? " (skipped repository, owner session decides)" : ""}` : "has every template copy");
  });
}

// --- run ---------------------------------------------------------------------------------------------
const table: Record<string, () => Promise<void>> = { org: stepOrg, archive: stepArchive, settings: stepSettings, labels: stepLabels, security: stepSecurity, rulesets: stepRulesets, features: stepFeatures, readme: stepReadme, forks: stepForks, needs: stepNeeds, templates: stepTemplates };
for (const s of ALL_STEPS) {
  if (!STEPS.has(s)) continue;
  console.log(`\n== ${s} (rate remaining ${remaining})`);
  try { await table[s]!(); } catch (e) { rec("-", s, "error", String((e as Error).message).slice(0, 300)); }
}
await Bun.write(outFile, outBuf);
const count = (s: Status) => ledger.filter((e) => e.status === s).length;
console.log(`\nledger ${OUT}\nwould=${count("would")} done=${count("done")} noop=${count("noop")} skip=${count("skip")} info=${count("info")} error=${count("error")} rate-remaining=${remaining}`);
process.exit(count("error") ? 1 : 0);
