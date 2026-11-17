// The poller: conditional, rate-limit aware, bounded per run, idempotent and resumable (all state lives in the store).
import { ADAPTERS, type FetchedBody, type Parsed } from "./adapters.ts";
import type { Manifest, Source } from "./manifest.ts";
import type { SourceState, Store } from "./store.ts";

export interface PollOptions {
  /** Poll these source ids only. */
  only?: string[];
  /** Ignore min_interval and backoff. */
  force?: boolean;
  /** Stop after this many sources (the rest stay due for the next run). */
  maxSources?: number;
  /** Wall-clock budget in seconds; sources not started by then stay due. */
  budget?: number;
  /** Environment to read tokens from. */
  env?: Record<string, string | undefined>;
  now?: () => number;
  fetch?: typeof fetch;
}
export interface SourceResult {
  source: string;
  outcome: "updated" | "not-modified" | "failed" | "skipped";
  status: number | null;
  pages: number;
  inserted: number;
  changed: number;
  error?: string;
}
export interface RunSummary {
  results: SourceResult[];
  due: number;
  polled: number;
  deferred: number;
}

const nowSeconds = (): number => Math.floor(Date.now() / 1000);

export function authHeader(manifest: Manifest, host: string, env: Record<string, string | undefined>): string | null {
  const auth = manifest.watch.auth[host];
  if (!auth) return null;
  for (const name of auth.env) if (env[name]) return `${auth.scheme} ${env[name]}`;
  return null;
}

/** Seconds to wait before the next attempt after a failure: server hints first, exponential backoff otherwise. */
export function backoff(res: { status: number; headers: Headers } | null, fails: number, base: number, cap: number, now: number): number {
  let wait = Math.min(cap, base * 2 ** Math.min(fails, 10));
  if (res) {
    const retry = Number(res.headers.get("retry-after"));
    if (Number.isFinite(retry) && retry > 0) wait = Math.max(wait, retry);
    const reset = Number(res.headers.get("x-ratelimit-reset"));
    const remaining = res.headers.get("x-ratelimit-remaining");
    if (remaining === "0" && Number.isFinite(reset) && reset > now) wait = Math.max(wait, reset - now + 1);
  }
  return Math.min(cap, Math.ceil(wait));
}

function nextLink(headers: Headers): string | undefined {
  const m = /<([^>]+)>;\s*rel="next"/.exec(headers.get("link") ?? "");
  return m?.[1];
}

export async function pollSource(store: Store, manifest: Manifest, source: Source, opts: PollOptions = {}): Promise<SourceResult> {
  const clock = opts.now ?? nowSeconds;
  const doFetch = opts.fetch ?? fetch;
  const env = opts.env ?? Bun.env;
  const adapter = ADAPTERS[source.adapter]!;
  const state: SourceState = store.state(source.id);
  const token = authHeader(manifest, source.host, env);
  const timeout = manifest.watch.timeout * 1000;
  const headersFor = (conditional: boolean): Record<string, string> => ({
    "user-agent": manifest.watch.userAgent,
    accept: adapter.accept,
    ...(token ? { authorization: token } : {}),
    ...(conditional && state.etag ? { "if-none-match": state.etag } : {}),
    ...(conditional && state.lastModified ? { "if-modified-since": state.lastModified } : {}),
  });
  const get = async (url: string): Promise<FetchedBody> => {
    const res = await doFetch(url, { headers: headersFor(false), signal: AbortSignal.timeout(timeout) });
    return { status: res.status, text: await res.text(), headers: res.headers };
  };

  const started = clock();
  const result: SourceResult = { source: source.id, outcome: "updated", status: null, pages: 0, inserted: 0, changed: 0 };
  let url: string | undefined = source.url;
  let validators: { etag: string | null; lastModified: string | null } | null = null;
  try {
    while (url && result.pages < manifest.watch.maxPages) {
      const first = result.pages === 0;
      const res = await doFetch(url, { headers: headersFor(first), signal: AbortSignal.timeout(timeout), redirect: "follow" });
      result.status = res.status;
      if (first && res.status === 304) {
        store.saveState({ ...state, lastPolled: started, nextAllowed: started + source.minInterval, fails: 0, lastStatus: 304, lastError: null });
        return { ...result, outcome: "not-modified" };
      }
      if (!res.ok) {
        const wait = backoff(res, state.fails, source.minInterval, manifest.watch.maxBackoff, started);
        const error = `HTTP ${res.status} ${res.statusText}`.trim();
        store.saveState({ ...state, lastPolled: started, nextAllowed: started + wait, fails: state.fails + 1, lastStatus: res.status, lastError: error });
        return { ...result, outcome: "failed", error };
      }
      const text = await res.text();
      const parsed: Parsed = await adapter.parse({ source, text, headers: res.headers, now: started, get });
      const applied = store.apply(source.id, source.kernel, parsed.items, parsed.versions, parsed.metrics, started);
      result.pages++;
      result.inserted += applied.inserted;
      result.changed += applied.changed;
      if (first) {
        validators = { etag: res.headers.get("etag"), lastModified: res.headers.get("last-modified") };
      }
      // Paginate only while the whole page was news, so a quiet source costs one request.
      const wholePageNew = parsed.items.length > 0 && applied.inserted + applied.changed === parsed.items.length;
      url = wholePageNew ? (parsed.next ?? nextLink(res.headers)) : undefined;
    }
    const moved = result.inserted + result.changed > 0;
    store.saveState({
      ...state, ...(validators ?? {}), lastPolled: started, nextAllowed: started + source.minInterval, fails: 0, lastStatus: result.status, lastError: null,
      lastChanged: moved ? started : state.lastChanged,
    });
    return result;
  } catch (err) {
    const error = err instanceof Error ? err.message : String(err);
    const wait = backoff(null, state.fails, source.minInterval, manifest.watch.maxBackoff, started);
    store.saveState({ ...state, lastPolled: started, nextAllowed: started + wait, fails: state.fails + 1, lastStatus: result.status, lastError: error });
    return { ...result, outcome: "failed", error };
  }
}

export async function poll(store: Store, manifest: Manifest, opts: PollOptions = {}): Promise<RunSummary> {
  const clock = opts.now ?? nowSeconds;
  const begun = clock();
  const wanted = manifest.sources.filter((s) => s.enabled && (!opts.only || opts.only.includes(s.id)));
  const due = wanted
    .map((source) => ({ source, state: store.state(source.id) }))
    .filter(({ state }) => opts.force || state.nextAllowed <= begun)
    .sort((a, b) => a.state.nextAllowed - b.state.nextAllowed || (a.source.id < b.source.id ? -1 : 1));
  const queue = due.slice(0, opts.maxSources ?? due.length).map((d) => d.source);
  const run = store.startRun(begun);
  const results: SourceResult[] = [];

  // One queue per host with a small worker pool, all hosts in parallel: no origin is hit harder than its limit.
  const byHost = new Map<string, Source[]>();
  for (const source of queue) byHost.set(source.host, [...(byHost.get(source.host) ?? []), source]);
  const deadline = opts.budget === undefined ? Infinity : Date.now() + opts.budget * 1000;
  const skipped: SourceResult[] = [];
  await Promise.all(
    [...byHost.values()].flatMap((jobs) =>
      Array.from({ length: Math.min(manifest.watch.concurrencyPerHost, jobs.length) }, async () => {
        for (let job = jobs.shift(); job; job = jobs.shift()) {
          if (Date.now() >= deadline) skipped.push({ source: job.id, outcome: "skipped", status: null, pages: 0, inserted: 0, changed: 0 });
          else results.push(await pollSource(store, manifest, job, opts));
        }
      }),
    ),
  );
  results.sort((a, b) => (a.source < b.source ? -1 : 1));
  const summary: RunSummary = { results: [...results, ...skipped], due: due.length, polled: results.length, deferred: due.length - results.length };
  const pruned = store.prune(begun - manifest.watch.retentionDays * 86400, ["commit", "mail", "page", "package", "version", "build"]);
  store.finishRun(run, clock(), { ...summary, results: undefined, pruned });
  return summary;
}
