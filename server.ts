import { serve } from 'bun';
import { existsSync, readFileSync, statSync } from 'fs';
import { join } from 'path';

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
  '.ico': 'image/x-icon'
};

const server = serve({
  port: PORT,
  fetch(req) {
    const url = new URL(req.url);
    let pathname = url.pathname;

    if (pathname === '/') {
      pathname = '/index.html';
    }

    const filePath = join(DIST_DIR, pathname);

    if (existsSync(filePath) && statSync(filePath).isFile()) {
      const ext = pathname.slice(pathname.lastIndexOf('.'));
      const contentType = MIME_TYPES[ext] || 'application/octet-stream';
      return new Response(readFileSync(filePath), {
        headers: { 'Content-Type': contentType }
      });
    }

    // SPA fallback
    const indexPath = join(DIST_DIR, 'index.html');
    if (existsSync(indexPath)) {
      return new Response(readFileSync(indexPath), {
        headers: { 'Content-Type': 'text/html; charset=utf-8' }
      });
    }

    return new Response('404 Not Found - Build the app first with bun run build', { status: 404 });
  }
});

console.log(`Aphrody server running at http://localhost:${server.port}`);
