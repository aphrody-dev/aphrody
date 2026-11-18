import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { auditVercelProjectConfig } from "../lib/vercel-brain.ts";
import { handleRpcRequest, MCP_TOOLS } from "./vercel-mcp.ts";

describe("vercel-mcp.ts", () => {
  test("exposes all 5 read-only Vercel tools and audits project configs accurately", () => {
    const init = handleRpcRequest({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: { protocolVersion: "2025-03-26" },
    });
    expect(init?.result).toMatchObject({
      serverInfo: { name: "vercel-brain", version: "1.0.0" },
    });

    const list = handleRpcRequest({ jsonrpc: "2.0", id: 2, method: "tools/list" });
    const tools = (list?.result as { tools: typeof MCP_TOOLS }).tools;
    expect(tools.map((t) => t.name)).toEqual([
      "vercel_brain_search",
      "vercel_graph_lookup",
      "vercel_skills_list",
      "vercel_project_audit",
      "vercel_cli_info",
    ]);

    const tmp = mkdtempSync(join(tmpdir(), "vercel-audit-test-"));
    try {
      mkdirSync(tmp, { recursive: true });
      writeFileSync(
        join(tmp, "vercel.json"),
        JSON.stringify({ rootDirectory: "apps/web", bunVersion: "1.4.x" }),
      );
      const findings = auditVercelProjectConfig(tmp);
      expect(findings.some((f) => f.rule === "vercel-json-top-level-key")).toBe(true);
    } finally {
      rmSync(tmp, { recursive: true, force: true });
    }
  });
});
