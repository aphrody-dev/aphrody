import { describe, expect, test } from "bun:test";
import { auditComposeContent, auditDockerfileContent } from "../lib/docker-brain.ts";
import { handleRpcRequest, MCP_TOOLS } from "./docker-mcp.ts";

describe("docker-mcp.ts", () => {
  test("exposes all 5 read-only Docker tools and audits Dockerfile / compose.yaml cleanly", () => {
    const init = handleRpcRequest({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: { protocolVersion: "2025-03-26" },
    });
    expect(init?.result).toMatchObject({
      serverInfo: { name: "docker-brain", version: "1.0.0" },
    });

    const list = handleRpcRequest({ jsonrpc: "2.0", id: 2, method: "tools/list" });
    const tools = (list?.result as { tools: typeof MCP_TOOLS }).tools;
    expect(tools.map((t) => t.name)).toEqual([
      "docker_brain_search",
      "docker_skills_list",
      "docker_dockerfile_audit",
      "docker_compose_audit",
      "docker_mcp_gateway_info",
    ]);

    const dfFindings = auditDockerfileContent("FROM node:latest\nENV API_KEY=hardcoded\n", false);
    expect(dfFindings.some((f) => f.rule === "secret-in-layer")).toBe(true);
    expect(dfFindings.some((f) => f.rule === "pin-base-tag")).toBe(true);
    expect(dfFindings.some((f) => f.rule === "non-root-user")).toBe(true);

    const compFindings = auditComposeContent("services:\n  app:\n    image: redis:8-alpine\n    security_opt: [\"no-new-privileges:true\"]\n    healthcheck:\n      test: [\"CMD\", \"true\"]\n");
    expect(compFindings.some((f) => f.rule === "compose-pinned-tags")).toBe(false);
    expect(compFindings.some((f) => f.rule === "no-new-privileges")).toBe(false);
  });
});
