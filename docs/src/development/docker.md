# Docker & Containers

Aphrody provides reproducible, multi-platform Docker workflows aligned with official [`github.com/orgs/docker`](https://github.com/orgs/docker) standards (`docker/docs`, `docker/skills`, `docker/mcp-gateway`, `docker/buildx`, `docker/compose`, `docker/docker-bench-security`).

## Buildx Bake

- **`docker-bake.hcl`**: declarative Buildx Bake builds (`docker buildx bake --print`, `BASE_IMAGE=ubuntu:24.04 docker buildx bake distros`) with OCI labels and BuildKit cache-from/cache-to parameters.
- **`.dockerignore`**: excludes `.git`, `target/`, `node_modules/`, `.env*`, TLS keys, `.agents`, `.claude`, `.gemini`, and screenshot/audit outputs.

## Official Skills & MCP Gateway

- Official Docker coding skills (`docker-project-foundations`, `docker-build-strategies`, `docker-compose-patterns`, `docker-docs`, `docker-destructive-guardrails`, `docker-sandboxes-lifecycle`) live in `.agents/skills/` and `.agents/plugins/aphrody/skills/`.
- Local offline/static Docker auditor MCP (`docker-brain` in `.agents/plugins/aphrody/mcp_config.json`) exposes `docker_brain_search`, `docker_skills_list`, `docker_dockerfile_audit`, `docker_compose_audit`, and `docker_mcp_gateway_info`.
- Containerized MCP runtime uses Docker MCP Gateway (`docker mcp gateway run --secrets=docker-desktop:./.env`).
