// Browser recon of one site: breadth-first over its pages at a human pace, robots.txt honoured, one screenshot, one
// DOM summary and one Markdown rendering (markitdown) per page. Pages of the same route shape are capped so a site
// with millions of item pages yields its screens, not its catalogue.
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Browser } from "./browser";

export type Dom = {
  title: string;
  headings: string[];
  buttons: string[];
  inputs: string[];
  forms: { method: string; action: string; fields: number }[];
  links: string[];
  nav: string[];
  roles: Record<string, number>;
  components: string[];
  landmarks: string[];
  text: number;
};

export type Screen = {
  id: string;
  url: string;
  final: string;
  depth: number;
  from: string | null;
  viewport: string;
  screenshot: string | null;
  mobile: string | null;
  markdown: string | null;
  dom: Dom | null;
  error?: string;
};

// Runs in the page; returns a JSON string (agent-browser hands results back as strings).
const SUMMARY = `(() => {
  const t = (s) => (s || "").replace(/\\s+/g, " ").trim().slice(0, 80);
  const q = (s) => [...document.querySelectorAll(s)];
  const abs = (a) => { try { const u = new URL(a.getAttribute("href"), location.href); u.hash = ""; return u.href; } catch { return null; } };
  const roles = {};
  for (const e of q("[role]")) roles[e.getAttribute("role")] = (roles[e.getAttribute("role")] || 0) + 1;
  const names = new Set();
  for (const e of q("[data-target],[data-component],[data-testid]")) names.add(e.dataset.target || e.dataset.component || e.dataset.testid);
  for (const e of q("*")) if (e.tagName.includes("-")) names.add(e.tagName.toLowerCase());
  return JSON.stringify({
    title: document.title.slice(0, 120),
    headings: q("h1,h2,h3").map((e) => e.tagName.toLowerCase() + " " + t(e.textContent)).filter((x) => x.length > 3).slice(0, 40),
    buttons: [...new Set(q("button,[role=button]").map((b) => t(b.textContent || b.getAttribute("aria-label"))).filter(Boolean))].slice(0, 60),
    inputs: q("input,select,textarea").filter((i) => i.type !== "hidden").map((i) => (i.type || i.tagName.toLowerCase()) + ":" + t(i.name || i.getAttribute("aria-label") || i.placeholder || "")).slice(0, 40),
    forms: q("form").map((f) => ({ method: (f.getAttribute("method") || "get").toLowerCase(), action: f.getAttribute("action") || "", fields: f.querySelectorAll("input,select,textarea").length })).slice(0, 20),
    links: [...new Set(q("a[href]").map(abs).filter(Boolean))],
    nav: [...new Set(q("header a[href], nav a[href]").map(abs).filter(Boolean))],
    roles,
    components: [...names].filter(Boolean).slice(0, 80),
    landmarks: ["header", "nav", "main", "aside", "footer", "dialog"].filter((s) => document.querySelector(s)),
    text: (document.body?.innerText || "").length,
  });
})()`;

/** robots.txt rules for one user agent: `allowed(path)` follows the longest match, Allow winning ties. */
export function robots(text: string, agent = "*"): { allowed: (path: string) => boolean; delay: number } {
  type Rule = { allow: boolean; re: RegExp; len: number };
  const groups: { agents: string[]; rules: Rule[]; delay: number }[] = [];
  let cur: (typeof groups)[number] | null = null;
  let lastWasAgent = false;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.replace(/#.*/, "").trim();
    const m = line.match(/^([A-Za-z-]+)\s*:\s*(.*)$/);
    if (!m) continue;
    const key = m[1]!.toLowerCase();
    const val = m[2]!.trim();
    if (key === "user-agent") {
      if (!lastWasAgent || !cur) groups.push((cur = { agents: [], rules: [], delay: 0 }));
      cur.agents.push(val.toLowerCase());
      lastWasAgent = true;
      continue;
    }
    lastWasAgent = false;
    if (!cur) continue;
    if ((key === "allow" || key === "disallow") && val) {
      const re = new RegExp("^" + val.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*").replace(/\\\$$/, "$"));
      cur.rules.push({ allow: key === "allow", re, len: val.length });
    }
    if (key === "crawl-delay") cur.delay = Number(val) || 0;
  }
  const a = agent.toLowerCase();
  const mine = groups.filter((g) => g.agents.some((x) => x !== "*" && a.includes(x)));
  const chosen = mine.length ? mine : groups.filter((g) => g.agents.includes("*"));
  const rules = chosen.flatMap((g) => g.rules);
  return {
    delay: Math.max(0, ...chosen.map((g) => g.delay)),
    allowed: (path) => {
      let best: Rule | null = null;
      for (const r of rules) if (r.re.test(path) && (!best || r.len > best.len || (r.len === best.len && r.allow))) best = r;
      return !best || best.allow;
    },
  };
}

/** Same scheme, host and port: a prefix test would accept https://site.example.evil for https://site.example. */
export const sameOrigin = (url: string, origin: string): boolean => {
  try {
    return new URL(url).origin === origin;
  } catch {
    return false;
  }
};

/** Route shape of a path: navigation roots and route words kept, every other segment a wildcard. A route word
 *  (tree, discussions) follows variable segments: it is seen after two different segments that are not navigation
 *  roots. A namespace seen under several roots (/datasets/acme, /spaces/acme) is not one. */
export class Shapes {
  private roots = new Set<string>();
  private words = new Map<string, Set<string>>();
  addNav(paths: string[]) {
    for (const p of paths) {
      const s = p.split("/").filter(Boolean)[0];
      if (s) this.roots.add(s);
    }
  }
  learn(path: string) {
    const segs = path.split("/").filter(Boolean);
    segs.forEach((s, i) => {
      if (i === 0) return;
      const k = `${i}:${s}`;
      if (!this.words.has(k)) this.words.set(k, new Set());
      this.words.get(k)!.add(segs[i - 1]!);
    });
  }
  of(path: string): string {
    const segs = path.split("/").filter(Boolean);
    const keep = (s: string, i: number) => {
      if (i === 0) return this.roots.has(s);
      const before = [...(this.words.get(`${i}:${s}`) ?? [])];
      return before.length >= 2 && !before.some((b) => this.roots.has(b)) && /^[a-z][a-z-]*$/.test(s);
    };
    return "/" + segs.map((s, i) => (keep(s, i) ? s : "*")).join("/");
  }
}

export type CrawlOptions = {
  seeds: { url: string; depth: number; from: string | null }[];
  out: string;
  depth: number;
  limit: number;
  perShape: number;
  delayMs: number;
  viewport: [number, number];
  mobile: [number, number] | null;
  exclude: RegExp | null;
  markdown: boolean;
  log: (s: string) => void;
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Screens already captured in `out` (screens.jsonl), so a later round continues numbering and skips visited pages. */
export function loadScreens(out: string): Screen[] {
  const f = join(out, "screens.jsonl");
  return existsSync(f) ? readFileSync(f, "utf8").split("\n").filter(Boolean).map((l) => JSON.parse(l) as Screen) : [];
}

async function toMarkdown(html: string, file: string): Promise<boolean> {
  const tmp = join(tmpdir(), `replica-${process.pid}-${Date.now()}.html`);
  writeFileSync(tmp, html);
  const bin = process.env.REPLICA_MARKITDOWN || "markitdown";
  try {
    const p = Bun.spawn([bin, tmp, "-o", file], { stdin: "ignore", stdout: "ignore", stderr: "pipe" });
    return (await p.exited) === 0;
  } catch {
    return false;
  } finally {
    await Bun.file(tmp).delete().catch(() => {});
  }
}

export async function crawl(o: CrawlOptions): Promise<Screen[]> {
  for (const d of ["screens", "pages"]) mkdirSync(join(o.out, d), { recursive: true });
  const done = loadScreens(o.out);
  const seen = new Set(done.flatMap((s) => [s.url, s.final]));
  const origin = new URL(o.seeds[0]!.url).origin;
  const rules = robots(await fetch(`${origin}/robots.txt`, { signal: AbortSignal.timeout(15000) }).then((r) => (r.ok ? r.text() : "")).catch(() => ""));
  const delay = Math.max(o.delayMs, rules.delay * 1000);
  const shapes = new Shapes();
  const perShape = new Map<string, number>();
  for (const s of done) {
    shapes.learn(new URL(s.final).pathname);
    if (s.dom) shapes.addNav(s.dom.nav.filter((u) => sameOrigin(u, origin)).map((u) => new URL(u).pathname));
  }
  for (const s of done) perShape.set(shapes.of(new URL(s.url).pathname), (perShape.get(shapes.of(new URL(s.url).pathname)) ?? 0) + 1);

  const browser = new Browser(`replica-${process.pid}`);
  const queue = [...o.seeds];
  const captured: Screen[] = [];
  let next = Math.max(0, ...done.map((s) => Number(s.id.slice(1)) || 0)) + 1;
  try {
    while (queue.length && captured.length < o.limit) {
      const item = queue.shift()!;
      const u = new URL(item.url);
      if (seen.has(u.href) || u.origin !== origin || !rules.allowed(u.pathname + u.search) || o.exclude?.test(u.href)) continue;
      const shape = shapes.of(u.pathname);
      if ((perShape.get(shape) ?? 0) >= o.perShape) continue;
      seen.add(u.href);
      perShape.set(shape, (perShape.get(shape) ?? 0) + 1);
      const id = `S${String(next++).padStart(2, "0")}`;
      const screen: Screen = { id, url: u.href, final: u.href, depth: item.depth, from: item.from, viewport: o.viewport.join("x"), screenshot: null, mobile: null, markdown: null, dom: null };
      try {
        await browser.open(u.href);
        // engines without a page before the first navigation (Obscura) take the viewport after it
        if (!captured.length) await browser.viewport(...o.viewport).catch(() => ({}));
        await browser.wait(1500);
        screen.final = await browser.url();
        seen.add(screen.final);
        await browser.screenshot(join(o.out, "screens", `${id}.png`));
        screen.screenshot = `screens/${id}.png`;
        screen.dom = JSON.parse(await browser.eval(SUMMARY)) as Dom;
        if (o.markdown) {
          const html = await browser.eval("document.documentElement.outerHTML");
          if (await toMarkdown(html, join(o.out, "pages", `${id}.md`))) screen.markdown = `pages/${id}.md`;
        }
        if (o.mobile) {
          try {
            await browser.viewport(...o.mobile);
            await browser.wait(800);
            await browser.screenshot(join(o.out, "screens", `${id}-mobile.png`));
            screen.mobile = `screens/${id}-mobile.png`;
          } finally {
            // the desktop viewport comes back even when the mobile capture failed
            await browser.viewport(...o.viewport).catch(() => ({}));
          }
        }
        shapes.learn(new URL(screen.final).pathname);
        if (item.depth === 0) shapes.addNav(screen.dom.nav.filter((x) => sameOrigin(x, origin)).map((x) => new URL(x).pathname));
        if (item.depth < o.depth) {
          // navigation links first: they name the site's areas
          const nav = new Set(screen.dom.nav);
          const links = [...screen.dom.nav, ...screen.dom.links.filter((l) => !nav.has(l))];
          for (const l of links) if (!seen.has(l)) queue.push({ url: l, depth: item.depth + 1, from: id });
        }
        o.log(`${id} ${u.pathname} ${screen.dom.title.slice(0, 60)}`);
      } catch (e) {
        screen.error = String(e).slice(0, 300);
        o.log(`${id} ${u.pathname} failed: ${screen.error}`);
      }
      captured.push(screen);
      appendFileSync(join(o.out, "screens.jsonl"), JSON.stringify(screen) + "\n");
      await sleep(delay);
    }
  } finally {
    await browser.close();
  }
  return captured;
}
