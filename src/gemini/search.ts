/**
 * /api/gemini/search engine.
 * - With GEMINI_API_KEY or GOOGLE_API_KEY: calls the Gemini generateContent REST
 *   API with the google_search grounding tool and returns text plus citations.
 * - Without a key, or on any upstream failure: a deterministic local fallback
 *   built from the site hubs and the package catalog, so tests run offline.
 * The key is read from the env object passed in and is never logged or returned.
 */

import { VERTICALS, SITE_URL, type Locale } from "../seo/site";
import { getSuggestions } from "../data/suggestions";
import { runKeylessGoogleSearchGrounding } from "../../../../packages/ai-os/src/extracted/index";

export interface GeminiCitation {
  title: string;
  url: string;
}

export interface GeminiSearchResult {
  query: string;
  answer: string;
  citations: GeminiCitation[];
  grounded: boolean;
  source: "gemini-google-search" | "local-fallback";
  model: string | null;
  searchQueries: string[];
}

export interface GeminiEnv {
  GEMINI_API_KEY?: string;
  GOOGLE_API_KEY?: string;
  GEMINI_MODEL?: string;
  [key: string]: string | undefined;
}

export const MAX_QUERY_LENGTH = 300;
export const DEFAULT_GEMINI_MODEL = "gemini-2.5-flash";
const GEMINI_BASE = "https://generativelanguage.googleapis.com/v1beta/models";

export function normalizeQuery(raw: string | null | undefined): string {
  return (raw || "").replace(/\s+/g, " ").trim().slice(0, MAX_QUERY_LENGTH);
}

export function localFallback(
  query: string,
  locale: Locale = "fr",
): GeminiSearchResult {
  const q = query.toLowerCase();
  const words = q.split(" ").filter((w) => w.length > 2);
  const scored = VERTICALS.map((v) => {
    const hub = v[locale];
    const hay =
      `${v.slug} ${hub.title} ${hub.description} ${hub.intro}`.toLowerCase();
    return { v, score: words.filter((w) => hay.includes(w)).length };
  })
    .filter((s) => s.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, 3);
  const citations: GeminiCitation[] = scored.map((s) => ({
    title: s.v[locale].title,
    url: `${SITE_URL}${locale === "fr" ? "" : "/en"}/search/${s.v.slug}`,
  }));
  const suggestions = getSuggestions(query)
    .slice(0, 5)
    .map((s) => s.text);
  const lines: string[] = [];
  if (scored.length) {
    lines.push(
      locale === "fr"
        ? `Rubriques Aphrody liées à "${query}" :`
        : `Aphrody topics related to "${query}":`,
    );
    for (const s of scored)
      lines.push(`- ${s.v[locale].title}: ${s.v[locale].description}`);
  } else {
    lines.push(
      locale === "fr"
        ? `Aucune rubrique Aphrody ne correspond à "${query}". Réponse locale sans ancrage Google Search.`
        : `No Aphrody topic matches "${query}". Local answer without Google Search grounding.`,
    );
  }
  if (suggestions.length) {
    lines.push(
      (locale === "fr" ? "Suggestions : " : "Suggestions: ") +
        suggestions.join(", "),
    );
  }
  return {
    query,
    answer: lines.join("\n"),
    citations,
    grounded: false,
    source: "local-fallback",
    model: null,
    searchQueries: [],
  };
}

interface GeminiApiResponse {
  candidates?: Array<{
    content?: { parts?: Array<{ text?: string }> };
    groundingMetadata?: {
      webSearchQueries?: string[];
      groundingChunks?: Array<{ web?: { uri?: string; title?: string } }>;
    };
  }>;
}

export async function geminiSearch(
  query: string,
  env: GeminiEnv,
  locale: Locale = "fr",
  fetchImpl: typeof fetch = fetch,
): Promise<GeminiSearchResult> {
  const key = (env.GEMINI_API_KEY || env.GOOGLE_API_KEY || "").trim();
  if (!key && env.AI_OS_KEYLESS === "1") {
    const grounded = await runKeylessGoogleSearchGrounding(query, {
      locale,
      model: env.GEMINI_MODEL || DEFAULT_GEMINI_MODEL,
      fetchImpl,
    });
    if (grounded.grounded || grounded.transport === "vertex-ai-keyless-adc") {
      return {
        query,
        answer: grounded.answer,
        citations: grounded.citations,
        grounded: grounded.grounded,
        source: "gemini-google-search",
        model: grounded.model,
        searchQueries: grounded.searchQueries,
      };
    }
    return localFallback(query, locale);
  }
  if (!key) return localFallback(query, locale);
  const model = (env.GEMINI_MODEL || DEFAULT_GEMINI_MODEL).trim();
  try {
    const res = await fetchImpl(
      `${GEMINI_BASE}/${encodeURIComponent(model)}:generateContent`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json", "x-goog-api-key": key },
        body: JSON.stringify({
          contents: [
            {
              role: "user",
              parts: [
                {
                  text:
                    (locale === "fr"
                      ? "Réponds en français, de façon concise et factuelle. "
                      : "Answer concisely and factually. ") + query,
                },
              ],
            },
          ],
          tools: [{ google_search: {} }],
        }),
        signal: AbortSignal.timeout(8000),
      },
    );
    if (!res.ok) return localFallback(query, locale);
    const data = (await res.json()) as GeminiApiResponse;
    const cand = data.candidates?.[0];
    const text = (cand?.content?.parts || [])
      .map((p) => p.text || "")
      .join("")
      .trim();
    if (!text) return localFallback(query, locale);
    const seen = new Set<string>();
    const citations: GeminiCitation[] = [];
    for (const c of cand?.groundingMetadata?.groundingChunks || []) {
      const uri = c.web?.uri;
      if (uri && !seen.has(uri)) {
        seen.add(uri);
        citations.push({ title: c.web?.title || uri, url: uri });
      }
    }
    return {
      query,
      answer: text,
      citations,
      grounded: citations.length > 0,
      source: "gemini-google-search",
      model,
      searchQueries: cand?.groundingMetadata?.webSearchQueries || [],
    };
  } catch {
    return localFallback(query, locale);
  }
}

/* Simple per-client token bucket: protects the API key quota. */
const buckets = new Map<string, { n: number; t: number }>();
export function rateLimited(
  client: string,
  limit = 30,
  windowMs = 60_000,
  now = Date.now(),
): boolean {
  const b = buckets.get(client);
  if (!b || now - b.t > windowMs) {
    buckets.set(client, { n: 1, t: now });
    if (buckets.size > 5000) buckets.clear();
    return false;
  }
  b.n += 1;
  return b.n > limit;
}

export interface GeminiHandlerOptions {
  env: GeminiEnv;
  fetchImpl?: typeof fetch;
  client?: string;
}

const enc = new TextEncoder();

function sse(event: string, data: unknown): Uint8Array {
  return enc.encode(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
}

export async function handleGeminiSearch(
  req: Request,
  opts: GeminiHandlerOptions,
): Promise<Response> {
  const url = new URL(req.url);
  const query = normalizeQuery(url.searchParams.get("q"));
  const locale: Locale = url.searchParams.get("hl") === "en" ? "en" : "fr";
  const base = {
    "Cache-Control": "no-store",
    "X-Content-Type-Options": "nosniff",
  };
  if (!query) {
    return Response.json(
      { error: "missing_query", message: "Provide ?q=" },
      { status: 400, headers: base },
    );
  }
  if (opts.client && rateLimited(opts.client)) {
    return Response.json(
      { error: "rate_limited" },
      { status: 429, headers: { ...base, "Retry-After": "60" } },
    );
  }
  const wantsSse =
    url.searchParams.get("stream") === "1" ||
    (req.headers.get("accept") || "").includes("text/event-stream");
  const result = await geminiSearch(query, opts.env, locale, opts.fetchImpl);
  if (!wantsSse) return Response.json(result, { headers: base });

  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(
        sse("meta", {
          query,
          grounded: result.grounded,
          source: result.source,
          model: result.model,
        }),
      );
      const words = result.answer.split(/(\s+)/);
      let chunk = "";
      for (const w of words) {
        chunk += w;
        if (chunk.length >= 48) {
          controller.enqueue(sse("delta", { text: chunk }));
          chunk = "";
        }
      }
      if (chunk) controller.enqueue(sse("delta", { text: chunk }));
      controller.enqueue(sse("citations", result.citations));
      controller.enqueue(sse("done", { ok: true }));
      controller.close();
    },
  });
  return new Response(stream, {
    headers: {
      ...base,
      "Content-Type": "text/event-stream; charset=utf-8",
      Connection: "keep-alive",
    },
  });
}
