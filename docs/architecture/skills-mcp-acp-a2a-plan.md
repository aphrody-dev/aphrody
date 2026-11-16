# Skills, MCP, ACP, A2A, OpenAI HTTP, Plugin & AI-Ready-First Ecosystem Plan

> Unified protocol consolidation and AI-native automation architecture grounded in primary workspace code (`product.toml`, `crates/*`, `.agents/*`) and verified 2026-11-16 `crates.io` / `npm` registries.

---

## 1. Current State & Protocol Duplication Audit

- **Slim Default Binary ([`crates/zed/Cargo.toml`](../../crates/zed/Cargo.toml))**: `default = ["nsh"]`; heavy features (`ai`, `extensions`, `debugger`, `repl`, `collab`, `updater`, `crash-reporting`, `grammars`) are strictly opt-in behind `full`.
- **ACP ([`crates/acp_thread`](../../crates/acp_thread/Cargo.toml), [`crates/agent_servers`](../../crates/agent_servers/Cargo.toml))**: Pins `agent-client-protocol = "=5.3.0"` (`unstable_protocol_v2` with v1 fallback), but has not yet enabled `unstable_mcp_over_acp`, `unstable_agent_skills`, or `agent-client-protocol-rmcp`.
- **MCP Duplication**: [`crates/context_server`](../../crates/context_server/Cargo.toml) implements a hand-rolled MCP client (`2024-11-05`..`2025-11-25`, stdio + Streamable HTTP + OAuth refresh), while [`crates/os_craft/src/mcp.rs`](../../crates/os_craft/src/mcp.rs) runs a separate raw `TcpListener` + JSON-RPC 2.0 MCP server (`2025-03-26`, exposing 10 `os_*`/`craft_*` tools and `/.well-known/webmcp.json`).
- **Skills ([`crates/agent_skills`](../../crates/agent_skills/Cargo.toml), [`skill_tool.rs`](../../crates/agent/src/tools/skill_tool.rs))**: Strictly validates `SKILL.md` (`name <= 64 [a-z0-9-]`, `description <= 1024`, 100 KB file cap, 50 KB catalog cap, XML-escaped envelopes) over `.agents/skills/` and `create-skill`, lacking `.well-known/agent-skills/index.json` discovery, `{baseDir}` substitution (OpenClaw), and `skills-lock.json` integrity checks (Vercel `skills`).
- **OpenAI HTTP ([`crates/open_ai/src/responses.rs`](../../crates/open_ai/src/responses.rs))**: Implements `/responses`, `/responses/compact`, `/responses/input_tokens`, and `ToolDefinition::{Function, Custom}`, missing built-in `type: "mcp"` and `text.format.json_schema` (`strict: true`).
- **Plugins & Runtimes**: [`crates/extension_api`](../../crates/extension_api/Cargo.toml) (`wit-bindgen 0.41`, WIT v0.8.0) + `extension_host` (`wasmtime 48.0.7`); [`vendor/tauri/plugins-workspace`](../../vendor/tauri/plugins-workspace/Cargo.toml) (`tauri 3.0.0-alpha.4`, `specta`, `schemars`); [`crates/nsh-js`](../../crates/nsh-js/Cargo.toml) (`quickjs-jit 0.14.2`, custom `HeapAllocator`, CPU `Budget`, `.nsh/types`); and [`.agents/plugins/aphrody/`](../../.agents/plugins/aphrody/plugin.json).

---

## 2. Verified Target SDK Stack (Live Registry Baseline 2026-11-16)

| Ecosystem | Rust (`crates.io`) | TypeScript / Bun (`npm`) |
| :--- | :--- | :--- |
| **MCP Core (`2026-07-28`)** | `rmcp 3.10.0` (stdio, Streamable HTTP, Tasks SEP-2663, MRTR SEP-2322, `skills-fs` SEP-2640) | `@modelcontextprotocol/server 2.4.0` + `client 2.4.0` (v2) / `@modelcontextprotocol/sdk 1.32.2` (v1) |
| **ACP + MCP Bridge** | `agent-client-protocol 5.3.0` + `agent-client-protocol-rmcp 5.3.0` | `@agentclientprotocol/sdk 2.7.0` |
| **A2A v1.0 & Bridge** | `a2a-server-lf 0.5.3`, `a2a-client-lf 0.2.9`, `a2a-protocol-types 0.15.0`, `a2a-mcp 0.11.3` | `@a2a-js/sdk 1.3.0` |
| **CLI -> MCP / Skills** | `brontes 0.7.3` (`clap::Command` -> `rmcp`), `cargo-doc-md 0.11.0`, `rustdoc-md 0.2.0` | `skills 1.9.0` (`vercel-labs/skills`), `openclaw 2026.11.4` / `clawhub 0.26.0` |
| **WebMCP (W3C)** | `/.well-known/webmcp.json` + `nsh/modelContext` | `webmcp-types 0.1.11`, `@w3c/webmcp-polyfill 0.1.0`, `@mcp-b/global 6.2.0`, `use-webmcp-tool 0.3.0` |

> Convergence rule: Standardizing MCP wire types on `rmcp 3.10.0` allows `agent-client-protocol-rmcp` (`McpServer::Acp`) and `a2a-mcp` (`AgentToMcpBridge` / `McpToA2ABridge`) to share one protocol core while gating all server/network crates behind opt-in Cargo features so default `zed` (`nsh`) compiles zero extra dependencies.

---

## 3. Four Composable AI-Ready-First Pipelines

1. **CLI -> MCP**:
   - **Rust (`clap`)**: Wrap `aphrody-os` ([`os_desktop/src/main.rs`](../../crates/os_desktop/src/main.rs)) and `nsh-cli` ([`nsh-cli/src/main.rs`](../../crates/nsh-cli/src/main.rs)) with feature-gated `brontes 0.7.3` (`mcp start` stdio / `mcp stream` HTTP, grouped tool namespaces, `TaskMode::Detached` for builds/tests).
   - **TypeScript / Bun**: Generate MCP stdio/Streamable HTTP facades from typed CLI schemas via `@modelcontextprotocol/server 2.4.0` + `readOnlyHint`/`destructiveHint` annotations.
2. **CLI -> Skills (`SKILL.md`)**:
   - Probe `<bin> --help` & `--version` (stripping ANSI/emails, capping inline summary <= 70 lines, placing deep flags in `references/subcommands.md`), emit OpenClaw `{baseDir}` paths, verify deterministic CI drift (`--check`), and index into `/.well-known/agent-skills/index.json` (`0.2.0`).
3. **App -> Plugin (Unified Capability Descriptor)**:
   - Project every app (`aphrody-os`, `zed`, `nsh`, `tauri`) simultaneously to: **WebMCP** (`navigator.modelContext.registerTool` + `nsh/modelContext`), **MCP Server** (`rmcp 3.10.0` exposing `os_*`, `craft_*`, and Tauri `specta`/`schemars` commands), **Zed WIT `v0.9.0`** ([`extension_api`](../../crates/extension_api/wit/since_v0.8.0/extension.wit) `agent-skills` + `mcp-tools` guarded by [`CapabilityGranter`](../../crates/extension_host/src/capability_granter.rs)), **Claude Code Plugin** ([`.claude-plugin`](../../.agents/plugins/aphrody/plugin.json) layout), and **A2A Agent Card (`v1.0`)** (`/.well-known/agent-card.json`).
4. **Cargo Crates & npm/Bun `package.ts` -> AI-Ready First**:
   - **Rust**: Run [`tooling/bun/crate-to-skill.ts`](../../tooling/bun/crate-to-skill.ts) (`cargo metadata` + `RUSTC_BOOTSTRAP=1 cargo doc-md` / `rustdoc-md`) emitting compliant `SKILL.md`, `references/`, `llms.txt`, and `index.json` SHA-256 bundles.
   - **TypeScript / Bun**: Parse `package.json` + `.d.ts` (reusing [`nsh-js/src/declarations.rs`](../../crates/nsh-js/src/declarations.rs) patterns) to output `SKILL.md`, `references/api.md`, `llms.txt`, and optional `npx skills experimental_sync` metadata.

---

## 4. Phased Implementation Plan & Security Boundaries

1. **Phase 1 (MCP Core + `brontes` + `open_ai`)**: Replace raw TCP loop in `os_craft/src/mcp.rs` with feature-gated `rmcp 3.10.0` + `axum` (`Host` validation, bearer auth, bounded SEP-2663 `task_ttl`, loopback bind); mount `brontes` behind opt-in features; extend `open_ai/src/responses.rs` with `ToolDefinition::Mcp` and strict JSON Schema outputs.
2. **Phase 2 (ACP <-> MCP & Opt-In A2A)**: Enable `agent-client-protocol-rmcp 5.3.0` (`unstable_mcp_over_acp`, `unstable_agent_skills`) in `acp_thread` / `agent_servers`; add feature-gated `agent_a2a` (`a2a-mcp 0.11.3`).
3. **Phase 3 (Unified Skills & AI-Ready Generator)**: Extend `agent_skills` with opt-in OpenClaw `{baseDir}`, `skills-lock.json`, and SHA-256 verified `index.json` (`0.2.0`) discovery; batch-generate Tier-1 workspace crate & CLI skills cleanly into CI-verified targets.
4. **Phase 4 (App -> Plugin Multi-Target)**: Add `webmcp` native module in `nsh-js` (`HeapAllocator` + `Budget` enforced), WIT `v0.9.0` MCP/skill interfaces in `extension_api`, and exporters projecting `CraftCatalog` / Tauri plugins cleanly.
