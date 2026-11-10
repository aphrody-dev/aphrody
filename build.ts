import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "fs";
import { dirname, extname, join } from "path";
import { brotliCompressSync, constants as zc } from "node:zlib";
import { validateWeb, formatReport } from "../../packages/wck/src/index";
import { getMaterialCssSubset } from "./src/design/material-tokens";
import { readGoogleConfig } from "./src/google/config";
import {
  adsTxt,
  agentJson,
  buildSitemaps,
  humansTxt,
  indexNowKey,
  llmsFullTxt,
  llmsTxt,
  robotsTxt,
  securityTxt,
  sellersJson,
  webmcpJson,
} from "./src/seo/documents";
import { minifyCss } from "./src/seo/page-css";
import { renderHome, renderNews, renderPackageHub, renderVertical } from "./src/seo/html";
import { LOCALES, NEWS, PACKAGE_ECOSYSTEMS, VERTICALS, localizedPath, newsPath, packageHubPath, verticalPath } from "./src/seo/site";
import { UniversalStoreManager } from "./src/store/universal-manager";

const ROOT = import.meta.dir;
const DIST_DIR = join(ROOT, "dist");
const BUILD_DATE = new Date().toISOString().slice(0, 10);
const cfg = readGoogleConfig(process.env);

if (existsSync(DIST_DIR)) rmSync(DIST_DIR, { recursive: true, force: true });
mkdirSync(DIST_DIR, { recursive: true });

function emit(rel: string, content: string | Uint8Array): void {
  const file = join(DIST_DIR, rel);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, content);
}

console.log("Building Aphrody with native Bun.build()...");

// 1. Application bundle (code-split) and the small boot entry.
const appBuild = await Bun.build({
  entrypoints: [join(ROOT, "src", "main.ts")],
  outdir: DIST_DIR,
  naming: { entry: "app.js", chunk: "chunk-[hash].js" },
  minify: true,
  target: "browser",
  format: "esm",
  splitting: true,
});
const bootBuild = await Bun.build({
  entrypoints: [join(ROOT, "src", "client", "boot.ts")],
  outdir: DIST_DIR,
  naming: "boot.js",
  define: { __G_CFG__: JSON.stringify(cfg) },
  minify: true,
  target: "browser",
  format: "esm",
});
for (const r of [appBuild, bootBuild]) {
  if (!r.success) {
    console.error("Build failed:", r.logs);
    process.exit(1);
  }
}

// 2. CSS: legacy stylesheet + generated M3 token subset + M3 layer, minified.
const rawCss = [
  readFileSync(join(ROOT, "src", "style.css"), "utf-8"),
  getMaterialCssSubset(),
  readFileSync(join(ROOT, "src", "m3.css"), "utf-8"),
].join("\n");
emit("style.css", minifyCss(rawCss));

// 3. Static HTML (SSG): home, topic hubs, package hubs, news - fr and en.
const store = new UniversalStoreManager(":memory:");
for (const l of LOCALES) {
  emit(`${localizedPath("/", l).slice(1)}index.html`, renderHome(l, cfg));
  for (const v of VERTICALS) emit(`${localizedPath(verticalPath(v.slug), l).slice(1)}/index.html`, renderVertical(v, l, cfg));
  for (const eco of PACKAGE_ECOSYSTEMS) {
    const pkgs = (await store.search("", eco as never)).map((p) => ({
      name: p.name,
      description: p.description,
      installCmd: p.installCmd,
    }));
    emit(`${localizedPath(packageHubPath(eco), l).slice(1)}/index.html`, renderPackageHub(eco, pkgs, l, cfg));
  }
  for (const n of NEWS) emit(`${localizedPath(newsPath(n.slug), l).slice(1)}/index.html`, renderNews(n, l, cfg));
}

// 4. Crawler, agent and compliance documents.
emit("robots.txt", robotsTxt());
for (const [name, xml] of Object.entries(buildSitemaps(BUILD_DATE))) emit(name, xml);
emit("llms.txt", llmsTxt());
emit("llms-full.txt", llmsFullTxt());
const contact = (process.env.PUBLIC_CONTACT_EMAIL || "").trim() || undefined;
emit("ads.txt", adsTxt(cfg));
emit("sellers.json", JSON.stringify(sellersJson(contact), null, 2));
emit(".well-known/security.txt", securityTxt(BUILD_DATE, contact));
emit(".well-known/agent.json", JSON.stringify(agentJson(), null, 2));
emit(".well-known/webmcp.json", JSON.stringify(webmcpJson(), null, 2));
emit("humans.txt", humansTxt(BUILD_DATE));
const nowKey = indexNowKey(process.env);
emit(`${nowKey}.txt`, nowKey);
emit("g-cfg.json", JSON.stringify(cfg));
emit(
  "favicon.svg",
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#6750a4"/><circle cx="28" cy="28" r="12" fill="none" stroke="#fff" stroke-width="5"/><path d="M37 37l13 13" stroke="#fff" stroke-width="5" stroke-linecap="round"/></svg>',
);
const publicDir = join(ROOT, "public");
if (existsSync(publicDir)) {
  for (const f of readdirSync(publicDir)) copyFileSync(join(publicDir, f), join(DIST_DIR, f));
}

// 5. Pre-compress text assets (gzip + brotli) for in-memory serving.
const COMPRESS = new Set([".html", ".js", ".css", ".svg", ".json", ".txt", ".xml"]);
function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p) : [p];
  });
}
let htmlCount = 0;
for (const file of walk(DIST_DIR)) {
  if (/\.(gz|br)$/.test(file)) continue;
  if (file.endsWith(".html")) htmlCount++;
  if (!COMPRESS.has(extname(file).toLowerCase())) continue;
  const raw = readFileSync(file);
  writeFileSync(`${file}.gz`, Bun.gzipSync(raw));
  writeFileSync(`${file}.br`, brotliCompressSync(raw, { params: { [zc.BROTLI_PARAM_QUALITY]: 11 } }));
}

const sz = (rel: string) => statSync(join(DIST_DIR, rel)).size;
const gz = (rel: string) => statSync(join(DIST_DIR, rel + ".gz")).size;
console.log("Aphrody build finished:");
for (const f of ["app.js", "boot.js", "style.css", "index.html"]) {
  console.log(`- dist/${f}: ${(sz(f) / 1024).toFixed(2)} KB (gz: ${(gz(f) / 1024).toFixed(2)} KB)`);
}
console.log(`- ${htmlCount} HTML pages, ${Object.keys(buildSitemaps(BUILD_DATE)).length} sitemap files`);
console.log(`- google ids configured: ${Object.keys(cfg).join(", ") || "none (no Google tag is loaded)"}`);

// 6. WCK compile-time 7-spec gate (Lighthouse 100, CLS = 0, LCP budget).
console.log("\nExecuting WCK 7-Spec Compile-Time Web & Lighthouse Gate...");
const report = validateWeb(DIST_DIR);
console.log(formatReport(report));
if (!report.allPassed) {
  console.error("FATAL: WCK Compile-Time Gate failed! Build aborted.");
  process.exit(1);
}
console.log("Aphrody build 100% validated by WCK!");
