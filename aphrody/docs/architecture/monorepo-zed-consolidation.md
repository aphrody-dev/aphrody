# Monorepo Consolidation & Alignment with Zed Upstream (`zed-industries/zed`)

> Canonical structural reference ensuring that every Aphrody subsystem — **Google Design (`https://design.google/`)**, **Storytold 82-Repo Suite**, **`egui 0.37` (`egui_gpui`)**, **Unreal Bevy 5.8 (`bevy_gpui`)**, **NSH (`aphrody_nsh`)**, and **Wasm/WebGPU (`gpui_wgpu` / `aphrody_cli`)** — follows `zed-industries/zed`'s exact repository layout, Cargo feature composition, example placement, and `cargo xtask` tooling contract.

---

## 1. Repository Layout vs `zed-industries/zed`

| Directory | Zed Upstream Role | Aphrody Consolidated Role |
| --- | --- | --- |
| [`crates/`](../../../crates/) | First-party Rust crates (`src/<crate>.rs`, zero `mod.rs`, `[lints] workspace = true`) | Identical rule across all Zed crates + `aphrody_ui`, `aphrody_craft`, `aphrody_design`, `egui_gpui`, `bevy_gpui`, `aphrody_craft`, `aphrody_nsh`, `aphrody_cli` |
| `crates/<crate>/examples/` | Crate-owned runnable Rust examples (e.g. [`crates/gpui/examples/`](../../../crates/gpui/examples/)) | Every studio crate owns its Rust examples (`egui_gpui`, `bevy_gpui`, `aphrody_craft`, `gpui_app_shell`, `aphrody_desktop`, `aphrody_cli`) |
| [`tooling/xtask/`](../../../tooling/xtask/src/main.rs) | Single authoritative Rust workspace CLI (`cargo xtask ...`) | Extended cleanly with `brand`, `new-app` (`--template craft-studio`), and [`studio`](../../../tooling/xtask/src/tasks/studio.rs) (`status`, `check`, `parity`, `session`) |
| [`tooling/bun/`](../../../tooling/bun/README.md) | N/A upstream (Node/Python scripts) | Single typed Bun router [`tooling/bun/cli.ts`](../../../tooling/bun/cli.ts) (`bun run xtask -- <domain> <action>`) over 7 domains |
| [`script/`](../../../script/) | Thin shell/PowerShell wrappers (`./script/clippy`, `./script/bundle-*`) | Preserved 1:1 + [`script/studio`](../../../script/studio) & [`script/studio.ps1`](../../../script/studio.ps1) |
| [`assets/`](../../../assets/) | Declarative themes, icons, keymaps, fonts | Extended under `assets/os/*.toml` (`google_design_tokens.toml`, `storytold_ecosystem.toml`, `craft_catalog.toml`) |
| [`docs/`](../../docs/) | `docs/src/` (mdBook) | Preserved + [`aphrody/docs/architecture/`](./README.md) blueprints + [`aphrody/docs/design/`](../design/manifest.json) Google Design asset/kit/MDX pack |
| [`examples/`](../../../examples/README.md) & [`templates/`](../../../templates/) | External language / starter corpora | `examples/nsh/*` (TSX apps), `examples/gpu/*` (vendor shaders), `templates/wasm-wgpu/*` (standalone starters) |

---

## 2. Consolidated Feature Hierarchy (`crates/zed/Cargo.toml`)

Rather than unconditionally pulling studio crates into minimal builds, Aphrody mirrors Zed upstream's additive feature tree:

```text
zed (default = ["nsh"])
├── extensions
├── debugger
├── repl
├── collab
├── ai
├── updater
├── crash-reporting
├── grammars
├── nsh ──────────────► aphrody_nsh (GpuiCodeEditor + Live TSX hot-reload)
├── studio ───────────► aphrody_nsh/studio
│                       ├── aphrody_design (design.google mirror + DESIGN.md + Lighthouse)
│                       └── aphrody_craft (FigCraft + Craft Anything + 18 Storytold Apps)
│                           ├── egui_gpui (egui 0.37 + Craft Controls + AccessKit 0.25)
│                           ├── bevy_gpui (Unreal 5.8 ECS + Nanite Meshlets + MaterialX->WGSL + Solari GI)
│                           └── aphrody_craft (storytold-suite: 50+ pure-Rust format & disasm crates)
└── full ─────────────► [extensions, debugger, repl, collab, ai, updater, crash-reporting, nsh, studio, grammars]
```

---

## 3. Single Reproducible Verification Commands

```sh
# 1. Full Rust + Bun Studio & Zed Conformity Gate (single entrypoint)
cargo xtask studio check

# 2. Inspect studio crate, manifest, and example status as JSON
cargo xtask studio status --json

# 3. Run Dual-GUI (GPUI vs egui 0.37) + Unreal Bevy 5.8 + Lighthouse 100/100 parity report
cargo xtask studio parity

# 4. Launch or visually verify the NSH AGY/Gemini Session Studio
cargo xtask studio session --verify
```
