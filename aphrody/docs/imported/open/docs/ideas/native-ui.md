# Native UI frameworks to bring together

Request: merge into one monorepo (Rust, Bun, C, Zig) the UI frameworks below, with their crates, packages, examples and ecosystem.

| Repository | Description (GitHub) | Language | License |
|---|---|---|---|
| anomalyco/rune | Native UI framework | Rust | MIT |
| longbridge/nsh | Native Shell: native UI for iOS, Android, macOS, Windows, Linux and the web | Rust | not declared |
| microsoft/webui | SSR without JavaScript on the server, Web Components | Rust | MIT |
| vercel-labs/ui | Behaviours for agents to build interfaces | TypeScript | Apache-2.0 |
| vercel-labs/native | Native desktop app toolkit | Zig | Apache-2.0 |
| vercel-labs/wterm | Terminal emulator for the web | TypeScript | Apache-2.0 |
| zed-industries/zed: `crates/gpui` | Zed's GPU UI framework | Rust | not declared |
| anomalyco/opentui | TUI library | TypeScript | MIT |
| iced-rs/iced | Elm-inspired Rust GUI | Rust | MIT |
| emilk/egui | Immediate-mode GUI, web and native | Rust | Apache-2.0 |
| shadcn-ui/ui | Components to copy and adapt | TypeScript | MIT |
| mui/base-ui | Unstyled, accessible components | TypeScript | MIT |

## What a trial showed

- The 12 sources flatten to about 550 MB with a provenance lock.
- `gpui` alone pulls 15 internal Zed crates through path dependencies, plus platform-specific ones (`gpui_linux`, `gpui_macos`, ...) declared in `[target.'cfg(...)'.dependencies]`: those sections must be followed and Zed's workspace (`[workspace.dependencies]`) re-rooted for `cargo` to resolve.
- Several repositories are workspaces themselves (rune, nsh, webui): they stay standalone in the root workspace.
- Two repositories share a name (`aphrody-labs/ui`, the shadcn-ui fork, and `vercel-labs/ui`): prefix with the organisation.

## Open points

- Purpose of the merge: reference catalog, common code base, or choice of a target framework.
- Link with Material 3: which of these frameworks receives the M3 components (see [design-system.md](design-system.md)); D4 asks for Rust at the same level as Flutter.
