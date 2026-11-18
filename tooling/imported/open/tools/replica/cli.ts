// replica: one recursive workflow from the recon of a site to a built clone.
//
//   bun tools/replica/cli.ts run <url> --out <dir> --project <dir> [--brief <file>] [--rounds 3] [--target 90]
//                                 [--clone-url <url>] [--agent "<cmd with {prompt}>"] [recon options]
//   bun tools/replica/cli.ts recon <url> --out <dir> [--engine crawl4ai|agent-browser|obscura|lightpanda]
//                                 [--depth 2] [--limit 60] [--per-shape 2] [--delay 2] [--viewport 1440x900]
//                                 [--mobile] [--exclude <regex>] [--binaries <file,file>]
//   bun tools/replica/cli.ts index|parity|env <dir>      ask <dir> "<question>" [-k 6]      rea <dir> <binary...>
//
// Recon: pages crawled (crawl4ai by default; agent-browser with Chrome, Obscura or Lightpanda as engines), each
// with a screenshot, Markdown and a DOM summary; public sources (OpenAPI, documents) typed by Magika and
// converted by docling or markitdown; binaries reported by LIEF and pyghidra; the recon map and feature matrix
// written; everything indexed for retrieval (LlamaIndex, embeddings on PyTorch, CUDA when present).
// Run: the agent stages follow the replica skills, parity is scored (features, and layout through TensorFlow and
// obscura's diagnostics when the clone runs), and while parity is under the target the recon goes deeper where
// parity is lowest and the build stages run again.
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { pickAgent, runStage, skillsDir } from "./agent";
import { crawl as browserCrawl, loadScreens, sameOrigin, type Screen } from "./crawl";
import { ensureEnv, py } from "./ml";
import { parity, parityMarkdown } from "./parity";
import { areaOf, features, frontier, mergeFeatures, parseCsv, reconMap, shapesOf, type Row, type Source } from "./recon";

const { values: o, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    out: { type: "string" },
    project: { type: "string" },
    brief: { type: "string" },
    rounds: { type: "string", default: "3" },
    target: { type: "string", default: "90" },
    "clone-url": { type: "string" },
    agent: { type: "string" },
    skills: { type: "string" },
    engine: { type: "string", default: "crawl4ai" },
    depth: { type: "string", default: "2" },
    limit: { type: "string", default: "60" },
    "per-shape": { type: "string", default: "2" },
    delay: { type: "string", default: "2" },
    viewport: { type: "string", default: "1440x900" },
    mobile: { type: "boolean", default: false },
    exclude: { type: "string" },
    binaries: { type: "string" },
    k: { type: "string", short: "k", default: "6" },
  },
});

const [cmd, ...args] = positionals;
const outDir = resolve(o.out ?? (["index", "parity", "ask", "rea", "env"].includes(cmd ?? "") ? (args[0] ?? "replica") : "replica"));
const log = (s: string) => {
  console.log(s);
  if (existsSync(outDir)) appendFileSync(join(outDir, "workflow.log"), `${new Date().toISOString()} ${s}\n`);
};
const num = (s: string | undefined) => Number(s);

async function recon(seeds: { url: string; depth: number; from: string | null }[], depth: number, limit: number): Promise<Screen[]> {
  mkdirSync(outDir, { recursive: true });
  const origin = new URL(seeds[0]!.url).origin;
  const [w, h] = o.viewport!.split("x").map(Number) as [number, number];
  if (o.engine === "crawl4ai") {
    await py(["crawl", outDir, ...seeds.map((s) => s.url), "--depth", String(depth), "--limit", String(limit), "--per-shape", o["per-shape"]!, "--delay", o.delay!, "--viewport", o.viewport!, ...(o.exclude ? ["--exclude", o.exclude] : [])], log);
  } else {
    if (o.engine !== "agent-browser") process.env.AGENT_BROWSER_ENGINE = o.engine;
    await browserCrawl({ seeds, out: outDir, depth, limit, perShape: num(o["per-shape"]), delayMs: num(o.delay) * 1000, viewport: [w, h], mobile: o.mobile ? [390, 844] : null, exclude: o.exclude ? new RegExp(o.exclude) : null, markdown: true, log });
  }
  await py(["sources", outDir, origin], log);
  if (o.binaries) await py(["rea", outDir, ...o.binaries.split(",")], log);
  const screens = loadScreens(outDir);
  const sources = (existsSync(join(outDir, "sources.json")) ? JSON.parse(readFileSync(join(outDir, "sources.json"), "utf8")) : []) as Source[];
  writeFileSync(join(outDir, "recon.md"), reconMap(screens, sources, origin));
  const rows = mergeFeatures(outDir, features(screens, sources, origin));
  writeFileSync(join(outDir, ".gitignore"), "screens/\nclone-screens/\npages/\nsources/\nindex/\nscreens.jsonl\nsources.json\nstages/*.log\nstages/*.prompt.md\nworkflow.log\n");
  log(`recon: ${screens.length} screens, ${sources.length} sources, ${rows.length} features`);
  const idx = await py<{ model: string; device: string; documents: number }>(["index", outDir], log);
  log(`index: ${idx.documents} documents, ${idx.model} on ${idx.device}`);
  return screens;
}

function score(): ReturnType<typeof parity> {
  const rows = parseCsv(readFileSync(join(outDir, "features.csv"), "utf8")) as unknown as Row[];
  const visual = existsSync(join(outDir, "visual.json")) ? (JSON.parse(readFileSync(join(outDir, "visual.json"), "utf8")) as { screens: Record<string, { layout: number }> }) : null;
  const p = parity(rows, visual ? Object.values(visual.screens).map((s) => s.layout) : []);
  writeFileSync(join(outDir, "parity.md"), parityMarkdown(p));
  return p;
}

async function run(url: string) {
  if (!o.project) throw new Error("run needs --project <dir to build in>");
  const project = resolve(o.project);
  const ctx = { origin: new URL(url).origin, out: outDir, project, skills: skillsDir(o.skills), tools: import.meta.dir, brief: o.brief ? readFileSync(o.brief, "utf8") : "", round: 1 };
  const agent = pickAgent(o.agent);
  if (!existsSync(join(outDir, "recon.md"))) await recon([{ url, depth: 0, from: null }], num(o.depth), num(o.limit));
  for (let round = 1; round <= num(o.rounds); round++) {
    ctx.round = round;
    const stages = round === 1 ? ["architect", "design", "backend", "build", "test", "diff"] : ["build", "test", "diff"];
    for (const s of stages) await runStage(s, ctx, agent, log);
    if (o["clone-url"]) await py(["visual", outDir, o["clone-url"], "--viewport", o.viewport!], log);
    const p = score();
    log(`round ${round}: parity ${p.overall} (features ${p.feature}${p.layout === null ? "" : `, layout ${p.layout}`}), must-haves ${p.mustDone}/${p.mustTotal}`);
    if (p.overall >= num(o.target) || round === num(o.rounds)) break;
    // recursion: deeper recon where parity is lowest, then the build stages again
    const weak = new Set(p.byArea.slice(0, 3).map((a) => a.area));
    const screens = loadScreens(outDir);
    const shapes = shapesOf(screens, ctx.origin);
    const seeds = frontier(screens, ctx.origin, num(o["per-shape"]), Math.ceil(num(o.limit) / 2), (s) => weak.has(areaOf(shapes, s)));
    log(`round ${round}: deeper recon of ${[...weak].join(", ")} from ${seeds.length} links`);
    if (seeds.length) await recon(seeds, 0, seeds.length);
  }
}

const usage = "usage: bun tools/replica/cli.ts run|recon <url> --out <dir> [...] | index|parity|env <dir> | ask <dir> <question> | rea <dir> <binary...>";
switch (cmd) {
  case "env": {
    const e = await ensureEnv(log);
    log(JSON.stringify({ ...e, devices: await py(["devices"], log) }));
    break;
  }
  case "recon": {
    await ensureEnv(log);
    // a directory with screens continues from their unvisited links; an empty one starts at the URL
    const prior = loadScreens(outDir).filter((s) => s.dom);
    const origin = new URL(args[0]!).origin;
    if (prior.length) await recon(frontier(prior, origin, num(o["per-shape"]), num(o.limit)), 0, num(o.limit));
    else await recon([{ url: args[0]!, depth: 0, from: null }], num(o.depth), num(o.limit));
    break;
  }
  case "run":
    await ensureEnv(log);
    await run(args[0]!);
    break;
  case "index":
    console.log(JSON.stringify(await py(["index", outDir], log)));
    break;
  case "ask":
    for (const r of await py<{ score: number; file: string; text: string }[]>(["ask", outDir, args[1]!, "-k", o.k!], log)) console.log(`--- ${r.score} ${r.file}\n${r.text}\n`);
    break;
  case "parity":
    if (o["clone-url"]) await py(["visual", outDir, o["clone-url"], "--viewport", o.viewport!], log);
    console.log(parityMarkdown(score()));
    break;
  case "rea":
    console.log(JSON.stringify(await py(["rea", outDir, ...args.slice(1)], log)));
    break;
  default:
    console.error(usage);
    process.exitCode = 2;
}
