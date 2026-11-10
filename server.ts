import { serve } from "bun";
import { existsSync, readFileSync, statSync } from "fs";
import { join } from "path";
import { getSuggestions } from "./src/data/suggestions";
import { UniversalStoreManager } from "./src/store/universal-manager";
import { runBenchmarkSuite } from "./src/benchmark/stack-picker";

const PORT = Number(process.env.PORT) || 3000;
const DIST_DIR = join(import.meta.dir, "dist");
const storeManager = UniversalStoreManager.getInstance("var/store.sqlite");

const MIME_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".ico": "image/x-icon",
  ".woff2": "font/woff2",
  ".woff": "font/woff",
  ".ttf": "font/ttf",
  ".wasm": "application/wasm",
};

const SECURITY_HEADERS: Record<string, string> = {
  "Content-Security-Policy":
    "default-src 'self'; script-src 'self' 'unsafe-inline' https://fonts.googleapis.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com data:; img-src 'self' data: https:; connect-src 'self';",
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "SAMEORIGIN",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Permissions-Policy": "microphone=(self)",
  "X-XSS-Protection": "1; mode=block",
};

function jsonResponse(data: any, status = 200, req?: Request): Response {
  const payload = JSON.stringify(data);
  const headers: Record<string, string> = {
    "Content-Type": "application/json; charset=utf-8",
    "Cache-Control": "no-cache",
    ...SECURITY_HEADERS,
  };

  if (req) {
    const acceptEncoding = req.headers.get("accept-encoding") || "";
    if (acceptEncoding.includes("gzip")) {
      headers["Content-Encoding"] = "gzip";
      return new Response(Bun.gzipSync(Buffer.from(payload)), {
        status,
        headers,
      });
    }
  }

  return new Response(payload, { status, headers });
}

export const server = serve({
  port: PORT,
  async fetch(req: Request) {
    const url = new URL(req.url);
    const pathname = url.pathname;

    // 1. API: Autocomplete Suggestions
    if (pathname === "/api/complete") {
      const q = url.searchParams.get("q") || "";
      const suggestions = getSuggestions(q);
      return jsonResponse({ query: q, suggestions }, 200, req);
    }

    // 2. Health check
    if (pathname === "/healthz" || pathname === "/api/health") {
      return jsonResponse({ status: "healthy", service: "aphrody" }, 200, req);
    }

    // 3. API: Store Search
    if (pathname === "/api/store/search") {
      const q = url.searchParams.get("q") || "";
      const eco = (url.searchParams.get("ecosystem") || "all") as any;
      const packages = await storeManager.search(q, eco);
      return jsonResponse(
        { query: q, ecosystem: eco, count: packages.length, packages },
        200,
        req,
      );
    }

    // 4. API: Store Install
    if (pathname === "/api/store/install" && req.method === "POST") {
      try {
        const body = (await req.json()) as { packageId: string };
        const newGen = storeManager.installPackage(body.packageId);
        return jsonResponse({ success: true, generation: newGen }, 200, req);
      } catch (err: any) {
        return jsonResponse({ success: false, error: err.message }, 400, req);
      }
    }

    // 5. API: Store Generations (GET list, POST rollback)
    if (pathname === "/api/store/generations") {
      if (req.method === "POST") {
        try {
          const body = (await req.json()) as { rollbackId: number };
          const rolledBackGen = storeManager.rollback(body.rollbackId);
          return jsonResponse(
            { success: true, generation: rolledBackGen },
            200,
            req,
          );
        } catch (err: any) {
          return jsonResponse({ success: false, error: err.message }, 400, req);
        }
      }
      const generations = storeManager.listGenerations();
      const active = storeManager.getActiveGeneration();
      return jsonResponse({ active, generations }, 200, req);
    }

    // 6. API: Awesome-List to Store Converter
    if (pathname === "/api/store/awesome-convert" && req.method === "POST") {
      try {
        const body = (await req.json()) as {
          markdownUrl?: string;
          markdown?: string;
        };
        const target = body.markdownUrl || body.markdown || "";
        const imported = await storeManager.convertAwesomeList(target);
        return jsonResponse(
          { success: true, count: imported.length, packages: imported },
          200,
          req,
        );
      } catch (err: any) {
        return jsonResponse({ success: false, error: err.message }, 400, req);
      }
    }

    // 7. API: Performance, LTO, AOT & Zero-Copy Stack Benchmark
    if (pathname === "/api/bench") {
      try {
        const report = await runBenchmarkSuite();
        return jsonResponse(report, 200, req);
      } catch (err: any) {
        return jsonResponse({ error: err.message }, 500, req);
      }
    }

    // 8. Static Assets from dist/
    let targetFile = pathname === "/" ? "/index.html" : pathname;
    let filePath = join(DIST_DIR, targetFile);

    // If file doesn't exist, SPA fallback to index.html
    if (!existsSync(filePath) || !statSync(filePath).isFile()) {
      filePath = join(DIST_DIR, "index.html");
      targetFile = "/index.html";
    }

    if (existsSync(filePath) && statSync(filePath).isFile()) {
      const ext = targetFile.slice(targetFile.lastIndexOf("."));
      const contentType = MIME_TYPES[ext] || "application/octet-stream";
      const headers: Record<string, string> = {
        "Content-Type": contentType,
        ...SECURITY_HEADERS,
      };

      // Set long cache for immutable static assets
      if (
        ext === ".js" ||
        ext === ".css" ||
        ext === ".svg" ||
        ext === ".wasm"
      ) {
        headers["Cache-Control"] = "public, max-age=31536000, immutable";
      } else if (ext === ".html") {
        headers["Cache-Control"] = "no-cache, must-revalidate";
      }

      const acceptEncoding = req.headers.get("accept-encoding") || "";
      const gzPath = filePath + ".gz";

      // Serve pre-compressed gzip if available and client accepts gzip
      if (
        acceptEncoding.includes("gzip") &&
        existsSync(gzPath) &&
        statSync(gzPath).isFile()
      ) {
        headers["Content-Encoding"] = "gzip";
        return new Response(readFileSync(gzPath), { headers });
      }

      // Dynamic gzip for text assets if pre-compressed does not exist
      if (
        acceptEncoding.includes("gzip") &&
        (contentType.startsWith("text/") ||
          contentType.startsWith("application/javascript") ||
          contentType.startsWith("application/json"))
      ) {
        const rawContent = readFileSync(filePath);
        headers["Content-Encoding"] = "gzip";
        return new Response(Bun.gzipSync(rawContent), { headers });
      }

      return new Response(readFileSync(filePath), { headers });
    }

    return new Response(
      "404 Not Found - Build the app first with bun run build",
      {
        status: 404,
        headers: {
          "Content-Type": "text/plain; charset=utf-8",
          ...SECURITY_HEADERS,
        },
      },
    );
  },
});

console.log(`Aphrody server running at http://localhost:${server.port}`);
