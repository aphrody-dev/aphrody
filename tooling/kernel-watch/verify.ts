// Live endpoint verification: one unconditional request per manifest URL, parsed with the real adapter.
import { ADAPTERS, type FetchedBody } from "./adapters.ts";
import { authHeader } from "./poll.ts";
import type { Manifest } from "./manifest.ts";

export interface Verified {
  id: string;
  url: string;
  status: number | null;
  contentType: string;
  bytes: number;
  shape: string;
}
export interface VerifiedFile {
  verifiedAt: string;
  sources: Verified[];
  sites: Verified[];
}

async function request(manifest: Manifest, url: string, accept: string): Promise<{ res: Response; text: string }> {
  const host = new URL(url).host;
  const token = authHeader(manifest, host, Bun.env);
  const res = await fetch(url, {
    headers: { "user-agent": manifest.watch.userAgent, accept, ...(token ? { authorization: token } : {}) },
    signal: AbortSignal.timeout(manifest.watch.timeout * 1000 * 2),
  });
  return { res, text: await res.text() };
}

export async function verifyAll(manifest: Manifest): Promise<VerifiedFile> {
  const now = Math.floor(Date.now() / 1000);
  const sources = await Promise.all(
    manifest.sources.filter((s) => s.enabled).map(async (source): Promise<Verified> => {
      const adapter = ADAPTERS[source.adapter]!;
      try {
        const { res, text } = await request(manifest, source.url, adapter.accept);
        const base = { id: source.id, url: source.url, status: res.status, contentType: res.headers.get("content-type")?.split(";")[0] ?? "", bytes: new TextEncoder().encode(text).length };
        if (!res.ok) return { ...base, shape: "not parsed" };
        const get = async (url: string): Promise<FetchedBody> => {
          const r = await request(manifest, url, adapter.accept);
          return { status: r.res.status, text: r.text, headers: r.res.headers };
        };
        const parsed = await adapter.parse({ source, text, headers: res.headers, now, get });
        const first = parsed.items[0];
        const parts = [`${parsed.items.length} items`];
        if (parsed.versions.length) parts.push(`versions ${parsed.versions.slice(0, 4).map((v) => `${v.channel}=${v.version}`).join(", ")}`);
        if (parsed.metrics.length) parts.push(`metrics ${parsed.metrics.map((m) => `${m.name}=${m.value}`).join(", ")}`);
        if (first) parts.push(`first ${first.kind}: ${first.title.slice(0, 70)}`);
        return { ...base, shape: parts.join("; ") };
      } catch (err) {
        return { id: source.id, url: source.url, status: null, contentType: "", bytes: 0, shape: `error: ${err instanceof Error ? err.message : err}` };
      }
    }),
  );
  const pages = manifest.kernels.flatMap((k) => [...k.sites.map((s) => ({ id: `${k.id}/${s.id}`, url: s.url })), ...k.orgs.map((o) => ({ id: `${k.id}/${o.id}`, url: o.url }))]);
  const sites = await Promise.all(
    pages.map(async (page): Promise<Verified> => {
      try {
        const { res, text } = await request(manifest, page.url, "text/html");
        return { ...page, status: res.status, contentType: res.headers.get("content-type")?.split(";")[0] ?? "", bytes: new TextEncoder().encode(text).length, shape: /<title>([^<]*)/i.exec(text)?.[1]?.trim().slice(0, 80) ?? "" };
      } catch (err) {
        return { ...page, status: null, contentType: "", bytes: 0, shape: `error: ${err instanceof Error ? err.message : err}` };
      }
    }),
  );
  const byId = (a: Verified, b: Verified): number => (a.id < b.id ? -1 : 1);
  return { verifiedAt: new Date(now * 1000).toISOString().replace(/\.\d+Z$/, "Z"), sources: sources.sort(byId), sites: sites.sort(byId) };
}

export async function verify(manifest: Manifest, path: string, write: boolean): Promise<number> {
  const result = await verifyAll(manifest);
  for (const v of [...result.sources, ...result.sites]) console.log(`${String(v.status ?? "ERR").padEnd(4)} ${v.id.padEnd(34)} ${v.contentType.padEnd(24)} ${String(v.bytes).padStart(9)}B  ${v.shape}`);
  const failed = [...result.sources, ...result.sites].filter((v) => v.status !== 200);
  if (write) await Bun.write(path, `${JSON.stringify(result, null, 2)}\n`);
  console.log(`${result.sources.length} sources, ${result.sites.length} site/org pages; ${failed.length} not 200${write ? `; wrote ${path}` : ""}`);
  return failed.length === 0 ? 0 : 1;
}
