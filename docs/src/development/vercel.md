# Vercel & Cloud Deployment

Aphrody documents official [`github.com/vercel/vercel-plugin`](https://github.com/vercel/vercel-plugin) and [`github.com/vercel-labs/agent-skills`](https://github.com/vercel-labs/agent-skills) workflows for web/Edge/NSH companions and documentation portals.

## Official Skills & MCP

- Core Vercel deployment skills (`deploy-to-vercel`, `vercel-cli`, `vercel-cli-with-tokens`, `deployments-cicd`, `env-vars`, `vercel-optimize`, `web-design-guidelines`) are owned by `aphrody-labs/ay-vercel` (`skills/`, upstream `skill-repos/vercel-plugin`) and are not vendored here; the `vercel-brain` digests live in `~/.gemini/plugins/vercel/` and `~/.gemini/brain/vercel/`.
- Local offline Vercel auditor MCP (`vercel-brain` in `.agents/plugins/aphrody/mcp_config.json`) exposes `vercel_brain_search`, `vercel_graph_lookup`, `vercel_skills_list`, `vercel_project_audit`, and `vercel_cli_info`.
- Official OAuth remote Vercel MCP (`https://mcp.vercel.com`) connects via `bun x vercel mcp`, and live markdown docs are searchable via `bun x vercel docs search "<query>"` and `https://vercel.com/llms.txt`.

## Production Guardrails

- Configure `Root Directory` only in Vercel Project Settings (never top-level `rootDirectory` in `vercel.json` / `vercel.ts`).
- Pin `bunVersion: "1.4.x"` alongside `bun.lock` v2, prefer Fluid Compute on Node.js 24.x / Bun over legacy Edge runtime, and use `NODE_OPTIONS=--max-old-space-size=6144` or prebuilt deploys (`vercel pull` + `vercel build --prod` + `vercel deploy --prebuilt --prod`) on large workspaces.
