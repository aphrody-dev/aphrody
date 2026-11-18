#!/usr/bin/env bun
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  auditVercelProjectConfig,
  listInstalledVercelSkills,
  lookupVercelGraph,
  resolveVercelBrainDir,
  searchVercelBrain,
} from "../lib/vercel-brain.ts";

export interface JsonRpcRequest {
  jsonrpc?: string;
  id?: string | number | null;
  method?: string;
  params?: Record<string, unknown>;
}

export interface JsonRpcResponse {
  jsonrpc: "2.0";
  id: string | number | null;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
}

export const MCP_TOOLS = [
  {
    name: "vercel_brain_search",
    description:
      "Search official Vercel ecosystem knowledge digests (INDEX, KNOWLEDGE_GRAPH, SKILLS_DIGEST, DOCS_DIGEST, COMMANDS, BEST_PRACTICES).",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {
        query: { type: "string", description: "Topic, product, error code (e.g. BUILD_UTILS_SPAWN_137), or CLI keyword." },
        file: { type: "string", description: "Optional digest filter (e.g. KNOWLEDGE_GRAPH, DOCS_DIGEST)." },
        limit: { type: "integer", minimum: 1, maximum: 30, description: "Max hits (default 8)." },
      },
      required: ["query"],
    },
  },
  {
    name: "vercel_graph_lookup",
    description:
      "Look up product dependencies, integrations, and mapped skills from the official vercel/vercel-plugin relational knowledge graph.",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {
        topic: { type: "string", description: "Product or capability (e.g. Fluid Compute, Prebuilt, Workflow, Sandbox)." },
      },
      required: ["topic"],
    },
  },
  {
    name: "vercel_skills_list",
    description: "List all installed official Vercel plugin & agent-skills modules.",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {},
    },
  },
  {
    name: "vercel_project_audit",
    description:
      "Audit a project directory (vercel.json, vercel.ts, next.config.ts) against official Vercel configuration rules and OOM/bundle guardrails.",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {
        dir: { type: "string", description: "Project or app directory path (defaults to current working directory)." },
      },
    },
  },
  {
    name: "vercel_cli_info",
    description:
      "Inspect official Vercel MCP/CLI endpoints (`vercel docs search`, `https://mcp.vercel.com`) and local vercel CLI availability.",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {},
    },
  },
] as const;

function textResult(payload: unknown, isError = false) {
  return {
    content: [{ type: "text", text: JSON.stringify(payload, null, 2) }],
    ...(isError ? { isError: true } : {}),
  };
}

function callTool(name: string, args: Record<string, unknown>) {
  switch (name) {
    case "vercel_brain_search": {
      const query = typeof args.query === "string" ? args.query : "";
      if (!query.trim()) return textResult({ error: "query is required" }, true);
      const hits = searchVercelBrain(query, {
        file: typeof args.file === "string" ? args.file : undefined,
        limit: typeof args.limit === "number" ? args.limit : undefined,
      });
      return textResult({ brainDir: resolveVercelBrainDir(), total: hits.length, hits });
    }
    case "vercel_graph_lookup": {
      const topic = typeof args.topic === "string" ? args.topic : "";
      if (!topic.trim()) return textResult({ error: "topic is required" }, true);
      const matches = lookupVercelGraph(topic);
      return textResult({ topic, total: matches.length, matches });
    }
    case "vercel_skills_list": {
      const skills = listInstalledVercelSkills();
      return textResult({ count: skills.length, skills });
    }
    case "vercel_project_audit": {
      const dir = resolve(typeof args.dir === "string" && args.dir ? args.dir : process.cwd());
      if (!existsSync(dir)) return textResult({ error: `Directory not found: ${dir}` }, true);
      const findings = auditVercelProjectConfig(dir);
      return textResult({ dir, ok: findings.every((f) => f.severity !== "error"), findings });
    }
    case "vercel_cli_info": {
      const hasVercel = Boolean(Bun.which("vercel"));
      return textResult({
        remoteMcpUrl: "https://mcp.vercel.com",
        llmsTxtUrl: "https://vercel.com/llms.txt",
        docsCliSearch: "bun x vercel docs search \"<query>\"",
        docsCliInspect: "bun x vercel docs inspect <page>",
        setupRemoteMcp: "bun x vercel mcp",
        vercelCliInstalled: hasVercel,
        brainDir: resolveVercelBrainDir(),
      });
    }
    default:
      return textResult({ error: `Unknown tool: ${name}` }, true);
  }
}

export function handleRpcRequest(req: JsonRpcRequest): JsonRpcResponse | null {
  const id = req.id ?? null;
  switch (req.method) {
    case "initialize":
      return {
        jsonrpc: "2.0",
        id,
        result: {
          protocolVersion: "2025-03-26",
          capabilities: { tools: {} },
          serverInfo: { name: "vercel-brain", version: "1.0.0" },
        },
      };
    case "notifications/initialized":
    case "notifications/cancelled":
      return null;
    case "ping":
      return { jsonrpc: "2.0", id, result: {} };
    case "tools/list":
      return { jsonrpc: "2.0", id, result: { tools: MCP_TOOLS } };
    case "tools/call": {
      const params = req.params ?? {};
      const toolName = typeof params.name === "string" ? params.name : "";
      const args = (params.arguments && typeof params.arguments === "object"
        ? params.arguments
        : {}) as Record<string, unknown>;
      return { jsonrpc: "2.0", id, result: callTool(toolName, args) };
    }
    default:
      if (req.id === undefined) return null;
      return {
        jsonrpc: "2.0",
        id,
        error: { code: -32601, message: `Method not found: ${req.method}` },
      };
  }
}

if (import.meta.main) {
  let buf = "";
  const stdin = process.stdin;
  stdin.setEncoding("utf8");
  stdin.on("data", (chunk: string) => {
    buf += chunk;
    let idx: number;
    while ((idx = buf.indexOf("\n")) !== -1) {
      const line = buf.slice(0, idx).trim();
      buf = buf.slice(idx + 1);
      if (!line) continue;
      try {
        const req = JSON.parse(line) as JsonRpcRequest;
        const res = handleRpcRequest(req);
        if (res) process.stdout.write(`${JSON.stringify(res)}\n`);
      } catch (err) {
        const res: JsonRpcResponse = {
          jsonrpc: "2.0",
          id: null,
          error: { code: -32700, message: err instanceof Error ? err.message : "Parse error" },
        };
        process.stdout.write(`${JSON.stringify(res)}\n`);
      }
    }
  });
}
