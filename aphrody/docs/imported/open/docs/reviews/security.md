# Security findings: hub, ai-router, replica

Scan by class (injection, authentication and access, secrets, dependencies, input handling), each finding refuted or confirmed by a second agent and Critical or High ones by a third. Scope: `crates/hub`, `crates/ai-router`, `tools/replica`, the workspace manifests. 26 agents; 13 findings kept, 7 refuted (35 %). No credential found in the code.

A first run saw empty directories (its finders did not follow the directory junctions that exposed the sources) and covered only input handling; the scan was run again on copies. Its three findings are folded into the table.

## Kept

| Severity | CWE | Finding | Location | Outcome |
|---|---|---|---|---|
| High | 22 | path segments accepted Windows drive prefixes (`C:`), so `Path::join` could leave the repository (write, delete, read) | `crates/hub` `safe_seg` | fixed: segments with `:` and Windows device names rejected; test |
| Medium | 77 | captured site content reaches a coding agent running with all permissions | `tools/replica/agent.ts` | fixed: Claude runs with `acceptEdits` and a tool allowlist (`REPLICA_AGENT_TOOLS`, `REPLICA_AGENT_PERMISSION` to change), and the stage prompt declares captured content data, never instructions |
| Medium | 306 | the router's `/v1/*` had no authentication while the container listens on every interface, spending the operator's provider credentials | `crates/ai-router` server, `deploy/Containerfile` | fixed: bearer tokens (`AI_ROUTER_TOKENS`, constant-time comparison); serving beyond loopback without tokens is refused unless `AI_ROUTER_OPEN=1`; the image check passes a token; test |
| Medium | 1188 | the hub without `HUB_TOKENS` treats every caller as a writer | `crates/hub` `user_of` | fixed: beyond loopback the hub refuses to start without `HUB_TOKENS` (or `HUB_OPEN=1`); loopback keeps the single-user mode |
| Low | 208 | static tokens compared with ordinary equality | `crates/hub` `user_of` | fixed: constant-time comparison |
| Low | 319 | plain HTTP | `crates/hub` | accepted: TLS belongs to the proxy in front; beyond loopback `HUB_PUBLIC_URL` (the https address) is required |
| Low | 319 | links built from the client's `Host` header | `crates/hub` `base_url` | fixed: beyond loopback `HUB_PUBLIC_URL` is required, so links never follow `Host` |
| Low | 1104 | Python dependencies unpinned, no lock file | `tools/replica/py/pyproject.toml` | accepted: the project takes the newest release of every dependency by policy; the environment is rebuilt by uv from PyPI |
| Low | 20 | revision names could collide with a revision's sidecar files | `crates/hub` commit | fixed: revision names restricted to letters, digits, `.`, `_`, `-`, `/`, no leading dot, never a sidecar suffix; test |
| Low | 770 | buffered request bodies up to 1 GiB per thread; mirror bodies unbounded | `crates/hub` | fixed: 64 MiB for buffered bodies (`HUB_MAX_BODY`), LFS objects and mirror bodies capped by `HUB_MAX_OBJECT` and streamed, at most `HUB_MAX_CONNECTIONS` requests at once (503 beyond) |

Three findings were duplicates of rows above (the container binding, the second token comparison, the open mode) and are counted there.

## Refuted

- Argument injection into the browser CLI: arguments go as an argv array without a shell, and every URL starts with its scheme, never with `-`.
- Binaries chosen by environment variables: setting them needs control of the environment of the user who runs the tool.
- Signing secret not from a CSPRNG: the standard library's `RandomState` keys come from the operating system's random source; the timestamp is extra input only.
- Rust audit not completed: a tooling gap, no advisory named.
- `lfs-download` stats the path before checking the oid: the answer is 404 either way, nothing leaks.
- Multipart boundary not validated: the client controls both body and boundary; a bad one only breaks its own request.

## Verification after the fixes

`cargo test -p ai-router` 12 passed, `cargo test -p hub` 8 passed (test host); the hub driven by the latest `huggingface_hub` (14 calls) and as a mirror of huggingface.co; `bun check` and `bun test` of `tools/replica` clean.
