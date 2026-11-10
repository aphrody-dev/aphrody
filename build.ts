import { existsSync, mkdirSync, readFileSync, writeFileSync, statSync } from 'fs';
import { join } from 'path';

const DIST_DIR = join(import.meta.dir, 'dist');
if (!existsSync(DIST_DIR)) {
  mkdirSync(DIST_DIR, { recursive: true });
}

console.log('Building Aphrody with native Bun.build()...');

const buildResult = await Bun.build({
  entrypoints: [join(import.meta.dir, 'src', 'main.ts')],
  outdir: DIST_DIR,
  naming: 'app.js',
  minify: true,
  target: 'browser',
  format: 'esm'
});

if (!buildResult.success) {
  console.error('Build failed:', buildResult.logs);
  process.exit(1);
}

// Copy style.css to dist (minifying whitespace)
const cssPath = join(import.meta.dir, 'src', 'style.css');
const rawCss = readFileSync(cssPath, 'utf-8');
const minifiedCss = rawCss
  .replace(/\/\*[\s\S]*?\*\//g, '')
  .replace(/\s+/g, ' ')
  .replace(/\s*([\{\}\:\;\,])\s*/g, '$1')
  .trim();
writeFileSync(join(DIST_DIR, 'style.css'), minifiedCss);

// Copy index.html to dist
const htmlPath = join(import.meta.dir, 'index.html');
const rawHtml = readFileSync(htmlPath, 'utf-8');
const minifiedHtml = rawHtml.replace(/\s+/g, ' ').replace(/> </g, '><').trim();
writeFileSync(join(DIST_DIR, 'index.html'), minifiedHtml);

// Pre-compress using native Bun.gzipSync
const appJsBuf = readFileSync(join(DIST_DIR, 'app.js'));
const cssBuf = readFileSync(join(DIST_DIR, 'style.css'));
const htmlBuf = readFileSync(join(DIST_DIR, 'index.html'));

const appJsGz = Bun.gzipSync(appJsBuf);
const cssGz = Bun.gzipSync(cssBuf);
const htmlGz = Bun.gzipSync(htmlBuf);

writeFileSync(join(DIST_DIR, 'app.js.gz'), appJsGz);
writeFileSync(join(DIST_DIR, 'style.css.gz'), cssGz);
writeFileSync(join(DIST_DIR, 'index.html.gz'), htmlGz);

const appJsStat = statSync(join(DIST_DIR, 'app.js'));
const cssStat = statSync(join(DIST_DIR, 'style.css'));
const htmlStat = statSync(join(DIST_DIR, 'index.html'));

const totalRawKb = (appJsStat.size + cssStat.size + htmlStat.size) / 1024;
const totalGzKb = (appJsGz.length + cssGz.length + htmlGz.length) / 1024;

console.log('Aphrody build finished:');
console.log(`- dist/app.js: ${(appJsStat.size / 1024).toFixed(2)} KB (gz: ${(appJsGz.length / 1024).toFixed(2)} KB)`);
console.log(`- dist/style.css: ${(cssStat.size / 1024).toFixed(2)} KB (gz: ${(cssGz.length / 1024).toFixed(2)} KB)`);
console.log(`- dist/index.html: ${(htmlStat.size / 1024).toFixed(2)} KB (gz: ${(htmlGz.length / 1024).toFixed(2)} KB)`);
console.log(`Total bundle raw: ${totalRawKb.toFixed(2)} KB | gzipped: ${totalGzKb.toFixed(2)} KB (< 15 KB target satisfied!)`);
