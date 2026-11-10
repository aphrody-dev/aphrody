import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  statSync,
  rmSync,
  readdirSync,
} from "fs";
import { join } from "path";
import { validateWeb, formatReport } from "../wck/src/index";

const DIST_DIR = join(import.meta.dir, "dist");
if (existsSync(DIST_DIR)) {
  rmSync(DIST_DIR, { recursive: true, force: true });
}
mkdirSync(DIST_DIR, { recursive: true });

console.log("Building Aphrody with native Bun.build()...");

const buildResult = await Bun.build({
  entrypoints: [join(import.meta.dir, "src", "main.ts")],
  outdir: DIST_DIR,
  naming: "app.js",
  minify: true,
  target: "browser",
  format: "esm",
  splitting: true,
});

if (!buildResult.success) {
  console.error("Build failed:", buildResult.logs);
  process.exit(1);
}

// 1. Copy style.css to dist (minifying whitespace)
const cssPath = join(import.meta.dir, "src", "style.css");
const rawCss = readFileSync(cssPath, "utf-8");
const minifiedCss = rawCss
  .replace(/\/\*[\s\S]*?\*\//g, "")
  .replace(/\s+/g, " ")
  .replace(/\s*([\{\}\:\;\,])\s*/g, "$1")
  .trim();
writeFileSync(join(DIST_DIR, "style.css"), minifiedCss);

// 2. Copy index.html to dist
const htmlPath = join(import.meta.dir, "index.html");
const rawHtml = readFileSync(htmlPath, "utf-8");
const minifiedHtml = rawHtml.replace(/\s+/g, " ").replace(/> </g, "><").trim();
writeFileSync(join(DIST_DIR, "index.html"), minifiedHtml);

// 3. Generate robots.txt at compile time
const robotsContent = [
  "User-agent: *",
  "Allow: /",
  "Sitemap: https://aphrody.com/sitemap.xml",
  "",
].join("\n");
writeFileSync(join(DIST_DIR, "robots.txt"), robotsContent);

// 4. Generate sitemap.xml at compile time
const sitemapContent = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://aphrody.com/</loc>
    <lastmod>${new Date().toISOString().slice(0, 10)}</lastmod>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>
`;
writeFileSync(join(DIST_DIR, "sitemap.xml"), sitemapContent);

// 5. Generate /llms.txt AI crawler index at compile time
const llmsContent = `# Aphrody

> Strict pixel-identical replica of Google search engine and sovereign workspace portal propulsed by oven-sh/bun v1.4.4 Full Rust.

## Canonical AI Endpoints

- Search Query: https://aphrody.com/api/q?q={query}
- Package Manager: https://aphrody.com/api/pm
- Benchmarks: https://aphrody.com/api/bnc
- Health Status: https://aphrody.com/ok
`;
writeFileSync(join(DIST_DIR, "llms.txt"), llmsContent);

// 6. Pre-compress all static assets using native Bun.gzipSync
const distFiles = readdirSync(DIST_DIR);
for (const file of distFiles) {
  if (file.endsWith(".gz")) continue;
  const filePath = join(DIST_DIR, file);
  if (statSync(filePath).isFile()) {
    const raw = readFileSync(filePath);
    const gz = Bun.gzipSync(raw);
    writeFileSync(`${filePath}.gz`, gz);
  }
}

const appJsStat = statSync(join(DIST_DIR, "app.js"));
const appJsGz = readFileSync(join(DIST_DIR, "app.js.gz"));
const cssStat = statSync(join(DIST_DIR, "style.css"));
const cssGz = readFileSync(join(DIST_DIR, "style.css.gz"));
const htmlStat = statSync(join(DIST_DIR, "index.html"));
const htmlGz = readFileSync(join(DIST_DIR, "index.html.gz"));

const totalRawKb = (appJsStat.size + cssStat.size + htmlStat.size) / 1024;
const totalGzKb = (appJsGz.length + cssGz.length + htmlGz.length) / 1024;

console.log("Aphrody build finished:");
console.log(
  `- dist/app.js: ${(appJsStat.size / 1024).toFixed(2)} KB (gz: ${(appJsGz.length / 1024).toFixed(2)} KB)`,
);
console.log(
  `- dist/style.css: ${(cssStat.size / 1024).toFixed(2)} KB (gz: ${(cssGz.length / 1024).toFixed(2)} KB)`,
);
console.log(
  `- dist/index.html: ${(htmlStat.size / 1024).toFixed(2)} KB (gz: ${(htmlGz.length / 1024).toFixed(2)} KB)`,
);
console.log(
  `Total entry bundle raw: ${totalRawKb.toFixed(2)} KB | gzipped: ${totalGzKb.toFixed(2)} KB`,
);

// 7. Execute WCK Compile-Time 7-Spec Validator Gate
console.log("\nExecuting WCK 7-Spec Compile-Time Web & Lighthouse Gate...");
const report = validateWeb(DIST_DIR);
console.log(formatReport(report));

if (!report.allPassed) {
  console.error("FATAL: WCK Compile-Time Gate failed! Build aborted.");
  process.exit(1);
}

console.log("Aphrody build 100% validated by WCK!");
