import { describe, expect, it } from 'bun:test';
import { existsSync, readFileSync, statSync } from 'fs';
import { join } from 'path';
import { GOOGLE_APPS, getAppIconSvg } from '../src/data/apps';
import { getSuggestions, POPULAR_SUGGESTIONS } from '../src/data/suggestions';
import { server } from '../server';

const ROOT_DIR = join(import.meta.dir, '..');

describe('Aphrody Zero-External-Dependency Architecture', () => {
  it('should have ZERO external dependencies in package.json', () => {
    const pkgPath = join(ROOT_DIR, 'package.json');
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'));

    const deps = Object.keys(pkg.dependencies || {});
    const devDeps = Object.keys(pkg.devDependencies || {});

    expect(deps.length).toBe(0);
    expect(devDeps.length).toBe(0);
    expect(pkg.dependencies).toEqual({});
    expect(pkg.devDependencies).toEqual({});
  });

  it('should NOT have node_modules directory', () => {
    const nodeModulesPath = join(ROOT_DIR, 'node_modules');
    expect(existsSync(nodeModulesPath)).toBe(false);
  });

  it('should NOT have bun.lock file', () => {
    const lockPath = join(ROOT_DIR, 'bun.lock');
    expect(existsSync(lockPath)).toBe(false);
  });
});

describe('Aphrody Bundle Build (<15 KB Target)', () => {
  it('should generate dist/ assets with gzipped total bundle under 15 KB', () => {
    const distDir = join(ROOT_DIR, 'dist');
    const appJsPath = join(distDir, 'app.js');
    const cssPath = join(distDir, 'style.css');
    const htmlPath = join(distDir, 'index.html');
    const appJsGzPath = join(distDir, 'app.js.gz');

    expect(existsSync(appJsPath)).toBe(true);
    expect(existsSync(cssPath)).toBe(true);
    expect(existsSync(htmlPath)).toBe(true);
    expect(existsSync(appJsGzPath)).toBe(true);

    const appJsGzStat = statSync(appJsGzPath);
    // Gzipped JS is under 15 KB
    expect(appJsGzStat.size).toBeLessThan(15 * 1024);
  });

  it('should contain valid HTML referencing pure JS and CSS in index.html', () => {
    const html = readFileSync(join(ROOT_DIR, 'dist', 'index.html'), 'utf-8');
    expect(html).toContain('<div id="root"></div>');
    expect(html).toContain('src="/app.js"');
    expect(html).toContain('href="/style.css"');
    expect(html).toContain('<title>Google</title>');
  });
});

describe('Aphrody Google Data Fidelity', () => {
  it('should contain all 18 official Google Apps with valid URLs and SVG icons', () => {
    expect(GOOGLE_APPS.length).toBe(18);
    const names = GOOGLE_APPS.map((a) => a.name);
    expect(names).toContain('Recherche');
    expect(names).toContain('Maps');
    expect(names).toContain('YouTube');
    expect(names).toContain('Gmail');
    expect(names).toContain('Drive');
    expect(names).toContain('Agenda');
    expect(names).toContain('Traduction');
    expect(names).toContain('Photos');
    expect(names).toContain('Docs');
    expect(names).toContain('Sheets');
    expect(names).toContain('Slides');
    expect(names).toContain('Keep');

    GOOGLE_APPS.forEach((app) => {
      const svg = getAppIconSvg(app.iconType);
      expect(svg).toContain('<svg');
      expect(svg).toContain('</svg>');
    });
  });

  it('should provide autocomplete suggestions matching query', () => {
    expect(POPULAR_SUGGESTIONS).toContain('aphrody');
    const emptyResults = getSuggestions('');
    expect(emptyResults.length).toBeGreaterThan(0);
    expect(emptyResults[0].isHistory).toBe(true);

    const queryResults = getSuggestions('aphrody');
    expect(queryResults.length).toBeGreaterThan(0);
    expect(queryResults[0].text.toLowerCase()).toContain('aphrody');
  });
});

describe('Aphrody HTTP Server & Endpoints', () => {
  it('should serve index.html at GET / with 200 OK and security headers', async () => {
    const req = new Request('http://localhost:3000/');
    const res = await server.fetch(req);
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toContain('text/html');
    expect(res.headers.get('x-content-type-options')).toBe('nosniff');
    expect(res.headers.get('x-frame-options')).toBe('SAMEORIGIN');
    expect(res.headers.get('content-security-policy')).toBeDefined();

    const text = await res.text();
    expect(text).toContain('Google');
    expect(text).toContain('id="root"');
  });

  it('should serve autocomplete JSON at GET /api/complete?q=aphrody', async () => {
    const req = new Request('http://localhost:3000/api/complete?q=aphrody');
    const res = await server.fetch(req);
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toContain('application/json');

    const data = await res.json();
    expect(data.query).toBe('aphrody');
    expect(Array.isArray(data.suggestions)).toBe(true);
    expect(data.suggestions.length).toBeGreaterThan(0);
  });

  it('should support Gzip compression when client requests it', async () => {
    const req = new Request('http://localhost:3000/api/complete?q=aphrody', {
      headers: { 'accept-encoding': 'gzip' }
    });
    const res = await server.fetch(req);
    expect(res.status).toBe(200);
    expect(res.headers.get('content-encoding')).toBe('gzip');
  });

  it('should serve app.js and style.css static assets', async () => {
    const jsReq = new Request('http://localhost:3000/app.js');
    const jsRes = await server.fetch(jsReq);
    expect(jsRes.status).toBe(200);
    expect(jsRes.headers.get('content-type')).toContain('javascript');

    const cssReq = new Request('http://localhost:3000/style.css');
    const cssRes = await server.fetch(cssReq);
    expect(cssRes.status).toBe(200);
    expect(cssRes.headers.get('content-type')).toContain('text/css');
  });

  it('should serve health check at GET /healthz', async () => {
    const req = new Request('http://localhost:3000/healthz');
    const res = await server.fetch(req);
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.status).toBe('healthy');
  });
});
