// The recon map (recon.md) and the feature matrix (features.csv) of the replica skills, written from the captured
// screens and sources. The matrix is merged on every round: rows already there keep their `clone` and `notes`.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { sameOrigin, Shapes, type Screen } from "./crawl";
export type Source = { url: string; file: string; label: string; group: string; markdown: string | null; openapi: boolean };

export type Row = { feature: string; area: string; priority: string; original: string; clone: string; notes: string };
const COLUMNS: (keyof Row)[] = ["feature", "area", "priority", "original", "clone", "notes"];

export function parseCsv(text: string): Record<string, string>[] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i]!;
    if (quoted) {
      if (c === '"' && text[i + 1] === '"') (cell += '"'), i++;
      else if (c === '"') quoted = false;
      else cell += c;
    } else if (c === '"') quoted = true;
    else if (c === ",") row.push(cell), (cell = "");
    else if (c === "\n" || c === "\r") {
      if (c === "\r" && text[i + 1] === "\n") i++;
      row.push(cell), rows.push(row), (row = []), (cell = "");
    } else cell += c;
  }
  if (cell || row.length) row.push(cell), rows.push(row);
  const [head, ...body] = rows.filter((r) => r.some((x) => x.trim()));
  const keys = (head ?? []).map((h) => h.replace(/^﻿/, "").trim().toLowerCase());
  return body.map((r) => Object.fromEntries(keys.map((k, i) => [k, (r[i] ?? "").trim()])));
}

const cell = (v: string) => (/[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);
export const toCsv = (rows: Row[]) => [COLUMNS.join(","), ...rows.map((r) => COLUMNS.map((k) => cell(r[k] ?? "")).join(","))].join("\n") + "\n";

const path = (u: string) => {
  const x = new URL(u);
  return x.pathname + x.search;
};
const name = (s: Screen) => (s.dom?.headings.find((h) => h.startsWith("h1"))?.slice(3) || s.dom?.title || path(s.url)).slice(0, 70);
const area = (s: Screen) => new URL(s.final).pathname.split("/").filter(Boolean)[0] ?? "home";

function chain(s: Screen, byId: Map<string, Screen>): Screen[] {
  const out = [s];
  for (let p = s.from; p && out.length < 20; p = byId.get(p)?.from ?? null) out.unshift(byId.get(p)!);
  return out.filter(Boolean);
}

/** Shapes learned from the screens: navigation of the first screens, every final path. */
export function shapesOf(screens: Screen[], origin: string): Shapes {
  const shapes = new Shapes();
  for (const s of screens) {
    shapes.learn(new URL(s.final).pathname);
    if (s.depth === 0 && s.dom) shapes.addNav(s.dom.nav.filter((u) => sameOrigin(u, origin)).map((u) => new URL(u).pathname));
  }
  return shapes;
}

/** Area of a screen in the feature matrix: its first route word for item pages, else its first path segment. */
export function areaOf(shapes: Shapes, s: Screen): string {
  const shape = shapes.of(new URL(s.url).pathname);
  return shape.includes("*") ? (shape.split("/").filter(Boolean).find((x) => x !== "*") ?? shape) : area(s);
}

/** Links of the captured screens not visited yet, capped per route shape, as seeds of one page each. */
export function frontier(screens: Screen[], origin: string, perShape: number, limit: number, keep: (s: Screen) => boolean = () => true): { url: string; depth: number; from: string | null }[] {
  const shapes = shapesOf(screens, origin);
  const visited = new Set(screens.flatMap((s) => [s.url, s.final]));
  const counts = new Map<string, number>();
  for (const s of screens) counts.set(shapes.of(new URL(s.url).pathname), (counts.get(shapes.of(new URL(s.url).pathname)) ?? 0) + 1);
  const out: { url: string; depth: number; from: string | null }[] = [];
  for (const s of screens.filter((x) => x.dom && keep(x))) {
    for (const l of s.dom!.links) {
      if (out.length >= limit) return out;
      if (visited.has(l) || !sameOrigin(l, origin)) continue;
      const k = shapes.of(new URL(l).pathname);
      if ((counts.get(k) ?? 0) >= perShape) continue;
      counts.set(k, (counts.get(k) ?? 0) + 1);
      visited.add(l);
      out.push({ url: l, depth: 0, from: s.id });
    }
  }
  return out;
}

export function features(screens: Screen[], sources: Source[], origin: string): Row[] {
  const rows: Row[] = [];
  const add = (feature: string, a: string, priority: string, notes = "") => rows.push({ feature, area: a, priority, original: "yes", clone: "no", notes });
  const navTargets = new Set(screens.flatMap((s) => s.dom?.nav ?? []).filter((u) => sameOrigin(u, origin)));
  // one feature per route shape: item pages of one route (a repository, a paper) are one screen of the site
  const shapes = shapesOf(screens, origin);
  const byShape = new Map<string, Screen[]>();
  for (const s of screens.filter((x) => x.dom)) {
    const k = shapes.of(new URL(s.url).pathname);
    byShape.set(k, [...(byShape.get(k) ?? []), s]);
  }
  for (const [shape, group] of byShape) {
    const s = group[0]!;
    const redirected = path(s.final) !== path(s.url);
    const signIn = redirected && /log-?in|sign-?in|auth/i.test(s.final);
    const item = shape.includes("*");
    const label = item ? `Item page ${shape}` : name(s);
    const where = item ? `${shape} (${group.map((g) => g.id).join(" ")})` : `${path(s.url)} (${s.id})`;
    const a = areaOf(shapes, s);
    add(`${label}: ${where}`, a, group.some((g) => navTargets.has(g.url) || g.depth === 0) ? "must" : "should", signIn ? "needs an account; rebuilt from public docs" : "");
    const forms = new Set(group.flatMap((g) => g.dom!.forms.filter((f) => f.fields > 0).map((f) => `${f.method.toUpperCase()} ${f.action || shape}`)));
    for (const f of forms) add(`Form ${f} on ${item ? shape : path(s.url)}`, a, "should");
  }
  for (const src of sources.filter((x) => x.openapi && x.markdown)) {
    const md = readFileSync(src.markdown!, "utf8");
    const groups = [...md.matchAll(/^## (.+) \((\d+)\)$/gm)].map((m) => ({ g: m[1]!, n: Number(m[2]) }));
    const sorted = groups.map((x) => x.n).sort((a, b) => a - b);
    const median = sorted[Math.floor(sorted.length / 2)] ?? 0;
    for (const { g, n } of groups) add(`API: ${g} (${n} operations)`, "api", n >= median ? "must" : "should", `from ${src.url}`);
  }
  return rows;
}

/** Merges new rows into the matrix in `out`: existing rows keep their clone and notes, new rows are appended. */
export function mergeFeatures(out: string, fresh: Row[]): Row[] {
  const file = join(out, "features.csv");
  const old = existsSync(file) ? (parseCsv(readFileSync(file, "utf8")) as unknown as Row[]) : [];
  const keys = new Set(old.map((r) => r.feature));
  const merged = [...old, ...fresh.filter((r) => !keys.has(r.feature) && (keys.add(r.feature), true))];
  writeFileSync(file, toCsv(merged));
  return merged;
}

export function reconMap(screens: Screen[], sources: Source[], origin: string): string {
  const byId = new Map(screens.map((s) => [s.id, s]));
  const ok = screens.filter((s) => s.dom);
  const L: string[] = [`# Recon map: ${origin}`, ""];
  L.push("Generated by `tools/replica` from the rendered screens and the public sources below; the agent stages refine it.", "");
  L.push("## Sources", "", "| source | type | markdown |", "|---|---|---|");
  for (const s of sources) L.push(`| ${s.url} | ${s.label}${s.openapi ? " (OpenAPI)" : ""} | ${s.markdown ? s.markdown.replace(/\\/g, "/").split("/").slice(-2).join("/") : "-"} |`);
  L.push("", "## Screens", "", "| ID | screen | route | purpose | key components | states seen |", "|---|---|---|---|---|---|");
  for (const s of screens) {
    const d = s.dom;
    const states = [s.viewport, s.mobile ? "mobile" : "", path(s.final) !== path(s.url) ? `redirected to ${path(s.final)}` : "", s.error ? "failed to load" : "", d && d.text < 300 ? "near empty" : ""].filter(Boolean).join(", ");
    const comps = d ? [...d.landmarks, ...Object.entries(d.roles).sort((a, b) => b[1] - a[1]).slice(0, 4).map(([r, n]) => `${r}x${n}`), ...d.components.slice(0, 5)].join(", ") : "";
    L.push(`| ${s.id} | ${name(s).replace(/\|/g, "/")} | ${path(s.url)} | ${(d?.headings[0] ?? "").slice(3, 60).replace(/\|/g, "/")} | ${comps.replace(/\|/g, "/")} | ${states} |`);
  }
  L.push("", "## Flows", "", "Each flow is the click path the crawl took from the first screen; clicks = screens minus one.", "");
  let n = 1;
  for (const s of ok.filter((x) => x.depth > 0 && (x.dom!.forms.some((f) => f.fields > 0) || x.dom!.inputs.length > 2))) {
    const c = chain(s, byId);
    L.push(`F${String(n++).padStart(2, "0")} Use ${name(s)}`, `    ${c.map((x) => `${x.id} ${name(x).slice(0, 30)}`).join(" -> ")}`, `    clicks: ${c.length - 1}; inputs: ${s.dom!.inputs.slice(0, 8).join(", ")}`, "");
  }
  L.push("## Components", "", "| component or control | screens |", "|---|---|");
  const seen = new Map<string, Set<string>>();
  for (const s of ok) for (const c of [...s.dom!.components, ...s.dom!.buttons.map((b) => `button: ${b}`)]) (seen.get(c) ?? seen.set(c, new Set()).get(c)!).add(s.id);
  for (const [c, ids] of [...seen].filter(([, ids]) => ids.size > 1).sort((a, b) => b[1].size - a[1].size).slice(0, 60)) L.push(`| ${c.replace(/\|/g, "/")} | ${[...ids].slice(0, 12).join(" ")}${ids.size > 12 ? " ..." : ""} |`);
  L.push("", "## Inferred data model", "");
  const api = sources.filter((s) => s.openapi && s.markdown);
  if (api.length) {
    for (const s of api) {
      const md = readFileSync(s.markdown!, "utf8");
      for (const m of md.matchAll(/^## (.+) \((\d+)\)\n\n(?:Fields: (.+)\n)?/gm)) L.push(`- **${m[1]}** (${m[2]} operations): ${m[3] ?? "fields not described"}. evidence: ${s.url}. confidence: high`);
    }
  } else {
    for (const s of ok.filter((x) => x.dom!.inputs.length)) L.push(`- **${name(s)}**: ${s.dom!.inputs.join(", ")}. evidence: ${s.id} form fields. confidence: guess`);
  }
  L.push("", "## Out of scope", "", "Content and accounts the site owns (its users, their uploads, its network), anything behind another person's account, partner deals and hardware: listed as `skip` rows in features.csv by the architect stage.", "");
  L.push("## Size", "", `${screens.length} screens, ${n - 1} flows, ${api.length ? "API described by OpenAPI" : "no published API description"}.`, "");
  return L.join("\n");
}
