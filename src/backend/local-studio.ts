// SPDX-License-Identifier: Apache-2.0
/**
 * Locally Uncensored Studio Router & 3-VPS Datacenter Search Bridge
 * Routes open-weights requests across ring3-agent (:8787), llama-server (:8129),
 * OpenCode (:4318), and Ollama (:11434), with deterministic 3-VPS index synthesis fallback.
 */
import {
  THREE_VPS_CLUSTER_TOPOLOGY,
  ThreeVpsSearchCluster,
} from "../../../../packages/chromium-bun/src/search/cluster-3vps.ts";

let sharedCluster: ThreeVpsSearchCluster | null = null;

export function getSharedSearchCluster(): ThreeVpsSearchCluster {
  if (!sharedCluster) {
    sharedCluster = new ThreeVpsSearchCluster({
      fr1DbPath: "var/search-fr1.sqlite",
      fr2DbPath: "var/search-fr2.sqlite",
      allowPrivateCrawl: false,
      enableUsaGoogleGrounding: true,
    });
  }
  return sharedCluster;
}

export interface LocalDaemonEndpoint {
  id: string;
  label: string;
  baseUrl: string;
  hardware: string;
}

export const LOCAL_UNCENSORED_ENDPOINTS: readonly LocalDaemonEndpoint[] = [
  {
    id: "ring3-agent",
    label: "ring3-agent (r3d Native Rust)",
    baseUrl: "http://127.0.0.1:8787",
    hardware: "RTX 4070 sm_89 CUDA 13.4 + AVX2",
  },
  {
    id: "llama-local",
    label: "Locally Uncensored Open-Weights",
    baseUrl: "http://127.0.0.1:8129",
    hardware: "Open-Weights GGUF / Heretic",
  },
  {
    id: "opencode-local",
    label: "OpenCode Sovereign Gateway",
    baseUrl: "http://127.0.0.1:4318",
    hardware: "Local Harness Router",
  },
  {
    id: "ollama-local",
    label: "Local Ollama Daemon",
    baseUrl: "http://127.0.0.1:11434",
    hardware: "Local GPU / CPU",
  },
];

export interface StudioCompletionResult {
  prompt: string;
  provider: string;
  liveDaemon: boolean;
  tookMs: number;
  output: string;
  references: Array<{ title: string; url: string }>;
}

export async function probeLocalDaemons(): Promise<
  Array<LocalDaemonEndpoint & { online: boolean; latencyMs: number | null }>
> {
  const results = await Promise.all(
    LOCAL_UNCENSORED_ENDPOINTS.map(async (ep) => {
      const t0 = performance.now();
      try {
        const res = await fetch(`${ep.baseUrl}/v1/models`, {
          signal: AbortSignal.timeout(450),
        }).catch(() =>
          fetch(`${ep.baseUrl}/api/tags`, { signal: AbortSignal.timeout(450) }),
        );
        const online = res.ok;
        return {
          ...ep,
          online,
          latencyMs: online
            ? Math.round((performance.now() - t0) * 100) / 100
            : null,
        };
      } catch {
        return { ...ep, online: false, latencyMs: null };
      }
    }),
  );
  return results;
}

export async function runLocalUncensoredCompletion(
  prompt: string,
): Promise<StudioCompletionResult> {
  const started = performance.now();
  const cleanPrompt = prompt.trim();

  for (const ep of LOCAL_UNCENSORED_ENDPOINTS) {
    try {
      const res = await fetch(`${ep.baseUrl}/v1/chat/completions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: "default",
          messages: [{ role: "user", content: cleanPrompt }],
          max_tokens: 512,
          temperature: 0.2,
        }),
        signal: AbortSignal.timeout(1_500),
      });
      if (res.ok) {
        const data = (await res.json()) as {
          choices?: Array<{ message?: { content?: string } }>;
        };
        const content = data.choices?.[0]?.message?.content?.trim();
        if (content) {
          return {
            prompt: cleanPrompt,
            provider: ep.id,
            liveDaemon: true,
            tookMs: Math.round((performance.now() - started) * 100) / 100,
            output: content,
            references: [],
          };
        }
      }
    } catch {
      // Proceed to next local daemon
    }
  }

  // Deterministic sovereign index synthesis when local GPU daemons are asleep
  const cluster = getSharedSearchCluster();
  const search = await cluster.search(cleanPrompt || "aphrody");
  const topHits = search.hits.slice(0, 4);

  const lines = [
    `[Sovereign Local Synthesis · RRF k=${search.rrfConstantK} · ${THREE_VPS_CLUSTER_TOPOLOGY.fr1.id} + ${THREE_VPS_CLUSTER_TOPOLOGY.fr2.id}]`,
    search.knowledgeCard
      ? `${search.knowledgeCard.title}: ${search.knowledgeCard.summary}`
      : `Indexed results matching "${cleanPrompt}":`,
    ...topHits.map(
      (h, idx) => `${idx + 1}. ${h.title} (${h.domain}) — ${h.snippet}`,
    ),
  ];

  return {
    prompt: cleanPrompt,
    provider: "sovereign-local-fts5-synthesis",
    liveDaemon: false,
    tookMs: Math.round((performance.now() - started) * 100) / 100,
    output: lines.join("\n\n"),
    references: topHits.map((h) => ({ title: h.title, url: h.url })),
  };
}
