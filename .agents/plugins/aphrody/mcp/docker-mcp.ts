#!/usr/bin/env bun
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import {
  auditComposeContent,
  auditDockerfileContent,
  listInstalledDockerSkills,
  resolveDockerBrainDir,
  searchDockerBrain,
} from "../lib/docker-brain.ts";

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
    name: "docker_brain_search",
    description:
      "Search official github.com/orgs/docker knowledge digests (INDEX, DOCS_DIGEST, SKILLS_DIGEST, MCP_GATEWAY, BEST_PRACTICES).",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {
        query: { type: "string", description: "Topic, BuildKit flag, Compose field, or Docker CLI keyword." },
        file: { type: "string", description: "Optional brain file filter (e.g. DOCS_DIGEST, MCP_GATEWAY)." },
        limit: { type: "integer", minimum: 1, maximum: 30, description: "Max hits (default 8)." },
      },
      required: ["query"],
    },
  },
  {
    name: "docker_skills_list",
    description:
      "List installed official docker/skills packages and trigger guidance.",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {},
    },
  },
  {
    name: "docker_dockerfile_audit",
    description:
      "Audit a Dockerfile on disk or inline text against BuildKit caching, secret hygiene, and non-root hardening best practices.",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Path to a Dockerfile." },
        content: { type: "string", description: "Optional inline Dockerfile text when no path is given." },
      },
    },
  },
  {
    name: "docker_compose_audit",
    description:
      "Audit a compose.yaml/compose.yml file on disk or inline YAML against production healthcheck, security_opt, network binding, and resource limit practices.",
    annotations: { readOnlyHint: true },
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Path to compose.yaml / compose.yml." },
        content: { type: "string", description: "Optional inline compose YAML." },
      },
    },
  },
  {
    name: "docker_mcp_gateway_info",
    description:
      "Inspect docker/mcp-gateway commands, client JSON snippet, and whether local docker / docker mcp CLI is available.",
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
    case "docker_brain_search": {
      const query = typeof args.query === "string" ? args.query : "";
      if (!query.trim()) return textResult({ error: "Missing required argument: query" }, true);
      const hits = searchDockerBrain(query, {
        file: typeof args.file === "string" ? args.file : undefined,
        limit: typeof args.limit === "number" ? args.limit : undefined,
      });
      return textResult({ query, brainDir: resolveDockerBrainDir(), count: hits.length, hits });
    }
    case "docker_skills_list": {
      const skills = listInstalledDockerSkills();
      return textResult({ count: skills.length, skills });
    }
    case "docker_dockerfile_audit": {
      const p = typeof args.path === "string" && args.path.trim() ? resolve(args.path) : undefined;
      const content = p
        ? existsSync(p)
          ? readFileSync(p, "utf8")
          : null
        : typeof args.content === "string"
          ? args.content
          : null;
      if (content === null) {
        return textResult({ error: `Dockerfile not found or empty: ${p ?? "<inline>"}` }, true);
      }
      const hasIgnore = p
        ? existsSync(join(dirname(p), ".dockerignore")) ||
          existsSync(`${p}.dockerignore`)
        : true;
      const findings = auditDockerfileContent(content, hasIgnore);
      return textResult({ target: p ?? "<inline>", findingCount: findings.length, findings });
    }
    case "docker_compose_audit": {
      const p = typeof args.path === "string" && args.path.trim() ? resolve(args.path) : undefined;
      const content = p
        ? existsSync(p)
          ? readFileSync(p, "utf8")
          : null
        : typeof args.content === "string"
          ? args.content
          : null;
      if (content === null) {
        return textResult({ error: `Compose file not found or empty: ${p ?? "<inline>"}` }, true);
      }
      const findings = auditComposeContent(content);
      return textResult({ target: p ?? "<inline>", findingCount: findings.length, findings });
    }
    case "docker_mcp_gateway_info": {
      const dockerBin = Bun.which("docker");
      const mcpSupported =
        dockerBin !== null &&
        Bun.spawnSync([dockerBin, "mcp", "--help"], { stdout: "ignore", stderr: "ignore" })
          .exitCode === 0;
      return textResult({
        dockerInstalled: dockerBin !== null,
        dockerPath: dockerBin,
        dockerMcpPluginAvailable: mcpSupported,
        gatewayCommand: ["docker", "mcp", "gateway", "run"],
        recommendedClientSnippet: {
          mcpServers: {
            MCP_DOCKER: { command: "docker", args: ["mcp", "gateway", "run"] },
          },
        },
        upstreamRepo: "https://github.com/docker/mcp-gateway",
      });
    }
    default:
      return null;
  }
}

export function handleRpcRequest(req: JsonRpcRequest): JsonRpcResponse | null {
  const id = req.id ?? null;
  const method = req.method ?? "";
  if (id === null && method.startsWith("notifications/")) return null;

  switch (method) {
    case "initialize": {
      const protocolVersion =
        typeof req.params?.protocolVersion === "string"
          ? req.params.protocolVersion
          : "2024-11-05";
      return {
        jsonrpc: "2.0",
        id,
        result: {
          protocolVersion,
          capabilities: { tools: {} },
          serverInfo: { name: "docker-brain", version: "1.0.0" },
        },
      };
    }
    case "ping":
      return { jsonrpc: "2.0", id, result: {} };
    case "tools/list":
      return { jsonrpc: "2.0", id, result: { tools: MCP_TOOLS } };
    case "tools/call": {
      const name = typeof req.params?.name === "string" ? req.params.name : "";
      const args =
        req.params?.arguments && typeof req.params.arguments === "object"
          ? (req.params.arguments as Record<string, unknown>)
          : {};
      const out = callTool(name, args);
      if (!out) {
        return { jsonrpc: "2.0", id, error: { code: -32601, message: `Unknown tool: ${name}` } };
      }
      return { jsonrpc: "2.0", id, result: out };
    }
    default:
      if (id === null) return null;
      return { jsonrpc: "2.0", id, error: { code: -32601, message: `Method not found: ${method}` } };
  }
}

if (import.meta.main) {
  const decoder = new TextDecoder();
  let buf = "";
  const reader = Bun.stdin.stream().getReader();
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buf += decoder.decode(value, { stream: true });
    let nl = buf.indexOf("\n");
    while (nl !== -1) {
      const line = buf.slice(0, nl).trim();
      buf = buf.slice(nl + 1);
      nl = buf.indexOf("\n");
      if (!line) continue;
      try {
        const resp = handleRpcRequest(JSON.parse(line) as JsonRpcRequest);
        if (resp) process.stdout.write(`${JSON.stringify(resp)}\n`);
      } catch (err) {
        process.stdout.write(
          `${JSON.stringify({ jsonrpc: "2.0", id: null, error: { code: -32700, message: String(err) } })}\n`,
        );
      }
    }
  }
}
