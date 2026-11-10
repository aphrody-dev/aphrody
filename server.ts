import { serve } from 'bun';
import { existsSync, readFileSync, statSync } from 'fs';
import { join } from 'path';
import { getSuggestions } from './src/data/suggestions';

const PORT = Number(process.env.PORT) || 3000;
const DIST_DIR = join(import.meta.dir, 'dist');

const MIME_TYPES: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
  '.woff': 'font/woff',
  '.ttf': 'font/ttf'
};

const SECURITY_HEADERS: Record<string, string> = {
  'Content-Security-Policy': "default-src 'self'; script-src 'self' 'unsafe-inline' https://fonts.googleapis.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com data:; img-src 'self' data: https:; connect-src 'self';",
  'X-Content-Type-Options': 'nosniff',
  'X-Frame-Options': 'SAMEORIGIN',
  'Referrer-Policy': 'strict-origin-when-cross-origin',
  'Permissions-Policy': 'microphone=(self)',
  'X-XSS-Protection': '1; mode=block'
};

export const server = serve({
  port: PORT,
  fetch(req) {
    const url = new URL(req.url);
    const pathname = url.pathname;

    // 1. API: Autocomplete Suggestions
    if (pathname === '/api/complete') {
      const q = url.searchParams.get('q') || '';
      const suggestions = getSuggestions(q);
      const payload = JSON.stringify({ query: q, suggestions });

      const headers: Record<string, string> = {
        'Content-Type': 'application/json; charset=utf-8',
        'Cache-Control': 'public, max-age=60',
        ...SECURITY_HEADERS
      };

      const acceptEncoding = req.headers.get('accept-encoding') || '';
      if (acceptEncoding.includes('gzip')) {
        headers['Content-Encoding'] = 'gzip';
        return new Response(Bun.gzipSync(Buffer.from(payload)), { headers });
      }

      return new Response(payload, { headers });
    }

    // 2. Health check
    if (pathname === '/healthz' || pathname === '/api/health') {
      return new Response(JSON.stringify({ status: 'healthy', service: 'aphrody' }), {
        headers: { 'Content-Type': 'application/json; charset=utf-8', ...SECURITY_HEADERS }
      });
    }

    // 3. Static Assets from dist/
    let targetFile = pathname === '/' ? '/index.html' : pathname;
    let filePath = join(DIST_DIR, targetFile);

    // If file doesn't exist, SPA fallback to index.html
    if (!existsSync(filePath) || !statSync(filePath).isFile()) {
      filePath = join(DIST_DIR, 'index.html');
      targetFile = '/index.html';
    }

    if (existsSync(filePath) && statSync(filePath).isFile()) {
      const ext = targetFile.slice(targetFile.lastIndexOf('.'));
      const contentType = MIME_TYPES[ext] || 'application/octet-stream';
      const headers: Record<string, string> = {
        'Content-Type': contentType,
        ...SECURITY_HEADERS
      };

      // Set long cache for immutable static assets
      if (ext === '.js' || ext === '.css' || ext === '.svg') {
        headers['Cache-Control'] = 'public, max-age=31536000, immutable';
      } else if (ext === '.html') {
        headers['Cache-Control'] = 'no-cache, must-revalidate';
      }

      const acceptEncoding = req.headers.get('accept-encoding') || '';
      const gzPath = filePath + '.gz';

      // Serve pre-compressed gzip if available and client accepts gzip
      if (acceptEncoding.includes('gzip') && existsSync(gzPath) && statSync(gzPath).isFile()) {
        headers['Content-Encoding'] = 'gzip';
        return new Response(readFileSync(gzPath), { headers });
      }

      // Dynamic gzip for text assets if pre-compressed does not exist
      if (acceptEncoding.includes('gzip') && (contentType.startsWith('text/') || contentType.startsWith('application/javascript') || contentType.startsWith('application/json'))) {
        const rawContent = readFileSync(filePath);
        headers['Content-Encoding'] = 'gzip';
        return new Response(Bun.gzipSync(rawContent), { headers });
      }

      return new Response(readFileSync(filePath), { headers });
    }

    return new Response('404 Not Found - Build the app first with bun run build', {
      status: 404,
      headers: { 'Content-Type': 'text/plain; charset=utf-8', ...SECURITY_HEADERS }
    });
  }
});

console.log(`Aphrody server running at http://localhost:${server.port}`);
