# GPUI vs Dioxus vs egui / craft-libs vs Iced / libcosmic: Audit, Benchmarks, Feature Matrix & Dependency Conflicts

> Code-verified multi-ecosystem audit, real microbenchmarks, Cargo feature matrix, and `Cargo.lock` conflict analysis across [`crates/gpui`](../../../crates/gpui/Cargo.toml), [`crates/egui_gpui`](../../../crates/egui_gpui/Cargo.toml), [`crates/aphrody_craft`](../../../crates/aphrody_craft/Cargo.toml), and [`crates/iced_gpui`](../../../crates/iced_gpui/Cargo.toml).

---

## 1. Automated CLI & Live Telemetry Snapshot

```bash
bun tooling/bun/gpui-vs-dioxus-audit.ts audit   # Submodules, crates.io, Cargo.lock conflicts, features
bun tooling/bun/gpui-vs-dioxus-audit.ts bench   # Run measured Rust + Bun parity benchmarks
bun tooling/bun/gpui-vs-dioxus-audit.ts all     # Full audit + benchmark + TOML + markdown refresh
bun test tooling/bun/gpui-vs-dioxus-audit.test.ts
```

<!-- BEGIN GPUI_DIOXUS_METRICS -->
| Audit Dimension | Live Verified Telemetry | Evidence Anchor |
| :--- | :--- | :--- |
| **Aphrody Workspace** | `323` members, `10` submodules, `35` `[patch.crates-io]` overrides (`offline` vs `zed` 95 284★) | [`Cargo.toml`](../../../Cargo.toml), [`.gitmodules`](../../../.gitmodules) |
| **Cargo.lock Graph & Splits** | `2419` packages across `37` git repos; `224` multi-version crates | [`Cargo.lock`](../../../Cargo.lock) |
| **Feature Matrix** | `44` Cargo manifests parsed across 6 ecosystems (`395` Cargo features) | `cargo metadata --no-deps` |
| **Measured Studio Bench** | GPUI tree: `609 us`, egui 0.37: `16473 us`, Bevy: `2565 us`, craft_sniff x100: `2971 us`, Iced x1000: `326 us`, RON x100: `317 us` | [`studio_parity_report.rs`](../../../crates/aphrody_craft/examples/studio_parity_report.rs) |
<!-- END GPUI_DIOXUS_METRICS -->

---

## 2. Live Fork & Workspace Architecture Graph

<!-- BEGIN GPUI_DIOXUS_FORK_GRAPH -->
```mermaid
flowchart LR
  subgraph Upstreams["Upstreams (Kept + Reference)"]
    UP_ZED["zed-industries/zed (dd091a1)"]
    UP_WGPU["gfx-rs/wgpu"]
    UP_TAFFY["DioxusLabs/taffy"]
    UP_A11Y["AccessKit/accesskit"]
    UP_COSMIC["pop-os/cosmic-text"]
    UP_EGUI["emilk/egui"]
    UP_CRAFT["storytold/craft-libs + craft-anything"]
    UP_DX["DioxusLabs/dioxus + blitz + anyrender (ref)"]
    UP_ICED["iced-rs/iced + pop-os/libcosmic (ref)"]
  end

  subgraph KeptVendor["Kept Vendor Submodules (.gitmodules)"]
    V_WGPU["vendor/wgpu (2798f45 - diverged (+1/-293))"]
    V_TAFFY["vendor/taffy (2649e09 - ahead (+4))"]
    V_A11Y["vendor/accesskit (432d066 - identical)"]
    V_COSMIC["vendor/cosmic-text (b07cb90 - identical)"]
    V_EGUI["vendor/egui (fd26cf0 - ahead (+26))"]
    V_CRAFT["vendor/craft-libs (62f7a7e - behind (-1))"]
    V_ANY["vendor/craft-anything (d8a2c51 - diverged (+2/-3))"]
  end

  subgraph Workspace["Aphrody Monorepo Bridges"]
    APH_ROOT["Cargo.toml (323 members, offline vs zed)"]
    GPUI_CORE["crates/gpui + gpui_wgpu + gpui-kit + nsh"]
    EGUI_GPUI["crates/egui_gpui + bevy_gpui"]
    CRAFT_APP["crates/aphrody_craft (45+ craft-* decoders)"]
    ICED_GPUI["crates/iced_gpui + aphrody_ui (Elm + RON)"]
  end

  UP_ZED -->|"offline"| APH_ROOT
  UP_WGPU -->|"fork"| V_WGPU
  UP_TAFFY -->|"fork"| V_TAFFY
  UP_A11Y -->|"fork"| V_A11Y
  UP_COSMIC -->|"fork"| V_COSMIC
  UP_EGUI -->|"fork"| V_EGUI
  UP_CRAFT -->|"fork"| V_CRAFT
  UP_CRAFT -->|"fork"| V_ANY
  UP_DX -->|"subsecond =0.7.10 pin"| GPUI_CORE
  UP_ICED -->|"adapted without winit/sctk"| ICED_GPUI

  V_WGPU -->|"patch wgpu 30.0.0"| GPUI_CORE
  V_WGPU -->|"patch wgpu 30.0.0"| EGUI_GPUI
  V_WGPU -->|"patch wgpu 30.0.0"| ICED_GPUI
  V_TAFFY -->|"patch taffy 0.15.1"| GPUI_CORE
  V_A11Y -->|"patch accesskit 0.25.2"| GPUI_CORE
  V_A11Y -->|"patch accesskit 0.25.2"| EGUI_GPUI
  V_COSMIC -->|"patch cosmic-text 0.19.0"| GPUI_CORE
  V_EGUI -->|"0.37.0 path deps"| EGUI_GPUI
  V_CRAFT -->|"17 git patches + path deps"| CRAFT_APP
  V_ANY -->|"analysis/disasm/js"| CRAFT_APP
  GPUI_CORE --> APH_ROOT
  EGUI_GPUI --> CRAFT_APP
  CRAFT_APP --> APH_ROOT
  ICED_GPUI --> APH_ROOT
```
<!-- END GPUI_DIOXUS_FORK_GRAPH -->

---

## 3. Kept Submodules & De-Vendored Reference Ecosystems

<!-- BEGIN GPUI_DIOXUS_SUBMODULES -->
| Kept Submodule / Reference | Local Pin / Bridge | Upstream Target | Drift / Role |
| :--- | :--- | :--- | :--- |
| `vendor/wgpu` | `2798f45c` (`patched`) | `gfx-rs/wgpu@trunk` | `diverged (+1/-293)` — Single-source WGPU v30 + Naga v30 stack shared by gpui_wgpu, egui-wgpu, iced_gpui, bevy_gpui & nsh-engine |
| `vendor/taffy` | `2649e09d` (`patched`) | `DioxusLabs/taffy@main` | `ahead (+4)` — Patched Flexbox/Grid/Block layout engine shared by gpui, nsh-core & aphrody_session |
| `vendor/cosmic-text` | `b07cb90a` (`patched`) | `pop-os/cosmic-text@main` | `identical` — Patched HarfBuzz/Swash text shaper & font fallback engine shared by gpui_wgpu |
| `vendor/accesskit` | `432d0665` (`patched`) | `AccessKit/accesskit@main` | `identical` — Patched OS UIA/AT-SPI2/AX semantic tree unified across gpui, egui 0.37 & aphrody_pilot |
| `vendor/egui` | `fd26cf0d` (`patched`) | `storytold/egui@main` | `ahead (+26)` — Immediate-mode GUI v0.37 embedded via crates/egui_gpui & crates/bevy_gpui on shared wgpu v30 + accesskit |
| `vendor/craft-libs` | `62f7a7e4` (`patched`) | `storytold/craft-libs@main` | `behind (-1)` — 45+ pure-Rust format/decoder/sniff crates powering crates/aphrody_craft & crates/bevy_gpui |
| `vendor/craft-anything` | `d8a2c511` (`workspace`) | `storytold/craft-anything@main` | `diverged (+2/-3)` — Disasm/JS/Evidence/Formats inspector crates embedded into crates/aphrody_craft |
| `vendor/tauri/wry` | `3b6eab2d` (`workspace`) | `tauri-apps/wry@dev` | `identical` — Native WebView2/WebKit/WebKitGTK host consumed by crates/nsh-platform-gpui |
| *(de-vendored)* `dioxus` | `crates/gpui/src/hot_reload.rs (subsecond =0.7.10)` | `DioxusLabs/dioxus` (39 843★) | Root pulls subsecond 0.7.10 from crates.io; dioxus 0.8 alpha pulls conflicting anyrender 0.14 / vello 0.11 |
| *(de-vendored)* `blitz` | `crates/aphrody_ui/src/unified_runtime.rs (FrameworkLayer::Blitz)` | `DioxusLabs/blitz` (4 489★) | HTML5/CSS Stylo reference only; pulls anyrender 0.15 / winit 0.31-beta outside Cargo.lock |
| *(de-vendored)* `anyrender` | `crates/gpui_wgpu/src/wgpu_context.rs (WgpuContext::shared_handles)` | `DioxusLabs/anyrender` (122★) | Zero-copy shared WGPU handle target; not a workspace Cargo dep |
| *(de-vendored)* `iced` | `crates/iced_gpui + aphrody_ui::reactive` | `iced-rs/iced` (32 072★) | Upstream iced pulls older wgpu + winit + softbuffer; superseded locally by iced_gpui on wgpu v30 |
| *(de-vendored)* `libcosmic` | `aphrody_ui::{cosmic_shell,cosmic_theme} + crates/iced_gpui` | `pop-os/libcosmic` (970★) | Pulls iced + sctk versions conflicting with gpui_linux; vendor/cosmic-text keeps text shaping local |
<!-- END GPUI_DIOXUS_SUBMODULES -->

---

## 4. Live `crates.io` Upstream Ecosystem Snapshot

<!-- BEGIN GPUI_DIOXUS_CRATES_IO -->
| Crate | Max Version (`crates.io`) | Max Stable | Total Downloads | Recent 90d |
| :--- | :--- | :--- | ---: | ---: |
| `gpui` | `0.2.2` | `0.2.2` | 519 393 | 301 412 |
| `dioxus` | `0.8.0-alpha.3` | `0.7.10` | 3 808 703 | 1 433 527 |
| `dioxus-cli` | `0.8.0-alpha.3` | `0.7.10` | 392 819 | 66 851 |
| `dioxus-native` | `0.8.0-alpha.3` | `0.7.10` | 49 579 | 21 454 |
| `subsecond` | `0.8.0-alpha.3` | `0.7.10` | 3 171 966 | 1 537 377 |
| `manganis` | `0.8.0-alpha.3` | `0.7.10` | 2 845 718 | 1 206 158 |
| `blitz-dom` | `0.3.0-beta.4` | `0.2.4` | 344 172 | 106 302 |
| `anyrender` | `0.15.0` | `0.15.0` | 306 655 | 207 172 |
| `vello` | `0.12.0` | `0.12.0` | 1 221 768 | 493 478 |
| `egui` | `0.37.0` | `0.37.0` | 32 167 491 | 10 266 546 |
| `eframe` | `0.37.0` | `0.37.0` | 25 439 652 | 8 551 524 |
| `egui-wgpu` | `0.37.0` | `0.37.0` | 18 603 961 | 8 083 245 |
| `epaint` | `0.37.0` | `0.37.0` | 32 137 214 | 10 297 749 |
| `iced` | `0.14.0` | `0.14.0` | 3 407 284 | 848 035 |
| `iced_core` | `0.14.0` | `0.14.0` | 3 520 123 | 859 754 |
| `iced_wgpu` | `0.14.0` | `0.14.0` | 3 336 437 | 814 419 |
| `iced_winit` | `0.14.1` | `0.14.1` | 3 378 930 | 825 848 |
| `smithay-client-toolkit` | `0.21.1` | `0.21.1` | 79 988 815 | 24 646 994 |
| `cosmic-text` | `0.19.0` | `0.19.0` | 12 720 911 | 5 295 674 |
| `softbuffer` | `0.4.8` | `0.4.8` | 35 209 721 | 14 903 470 |
| `wgpu` | `30.0.2` | `30.0.2` | 52 029 771 | 20 070 090 |
| `taffy` | `0.15.1` | `0.15.1` | 20 786 914 | 9 305 354 |
| `accesskit` | `0.25.2` | `0.25.2` | 36 401 267 | 12 743 944 |
| `winit` | `0.31.0-beta.4` | `0.30.13` | 67 474 939 | 17 544 112 |
| `wry` | `0.57.3` | `0.57.3` | 49 525 195 | 20 437 350 |
| `harfrust` | `0.16.0` | `0.16.0` | 21 159 358 | 15 032 129 |
| `skrifa` | `0.50.0` | `0.50.0` | 52 482 741 | 31 423 590 |
| `ropey` | `2.0.0-beta.1` | `1.6.1` | 15 556 050 | 4 985 008 |
<!-- END GPUI_DIOXUS_CRATES_IO -->

---

## 5. Extracted Cargo Feature Matrix (GPUI / NSH / egui / craft-libs / Iced / Foundations)

<!-- BEGIN GPUI_DIOXUS_FEATURES -->
| Manifest Path | Ecosystem | Crate & Version | Features | Default / Sample Flags |
| :--- | :--- | :--- | ---: | :--- |
| `crates/gpui/Cargo.toml` | `aphrody-gpui` | `gpui` `0.2.2` | **18** | `bench, bench-support, default, font-kit, hot-reload, inspector, leak-detection, objc2, +10` |
| `crates/gpui_wgpu/Cargo.toml` | `aphrody-gpui` | `gpui_wgpu` `0.1.0` | **4** | `bench-support, default, font-kit, test-support` |
| `crates/gpui_web/Cargo.toml` | `aphrody-gpui` | `gpui_web` `0.1.0` | **2** | `default, multithreaded` |
| `crates/gpui_ui/Cargo.toml` | `aphrody-gpui` | `gpui_ui` `0.1.0` | **14** | `default, desktop, hot-reload, inspector, kit, nsh, nsh-dev, nsh-full, +6` |
| `crates/gpui_platform/Cargo.toml` | `aphrody-gpui` | `gpui_platform` `0.1.0` | **11** | `bench-support, default, font-kit, gpui_wgpu, inspector, multithreaded, runtime_shaders, screen-capture, +3` |
| `crates/gpui_windows/Cargo.toml` | `aphrody-gpui` | `gpui_windows` `0.1.0` | **4** | `default, scap, screen-capture, test-support` |
| `crates/gpui_macos/Cargo.toml` | `aphrody-gpui` | `gpui_macos` `0.1.0` | **6** | `bench-support, default, font-kit, runtime_shaders, screen-capture, test-support` |
| `crates/gpui_linux/Cargo.toml` | `aphrody-gpui` | `gpui_linux` `0.1.0` | **21** | `as-raw-xcb-connection, ashpd, calloop-wayland-source, default, filedescriptor, gpui_wgpu, open, scap, +13` |
| `crates/gpui_ios/Cargo.toml` | `aphrody-gpui` | `gpui_ios` `0.1.0` | **1** | `runtime_shaders` |
| `crates/gpui-base/Cargo.toml` | `aphrody-gpui` | `gpui-base` `0.10.0` | **5** | `decimal, default, gpui-fast, inspector, test-support` |
| `crates/gpui-component/Cargo.toml` | `aphrody-gpui` | `gpui-component` `0.10.0` | **45** | `decimal, default, gpui-fast, inspector, speech, syntect, test-support, tree-sitter, +37` |
| `crates/gpui-kit/Cargo.toml` | `aphrody-gpui` | `gpui-kit` `0.10.0` | **48** | `assets, component, decimal, default, gpui-fast, inspector, profiler, speech, +40` |
| `crates/aphrody_ui/Cargo.toml` | `aphrody-gpui` | `aphrody_ui` `0.1.0` | **2** | `default, test-support` |
| `crates/aphrody_desktop/Cargo.toml` | `aphrody-gpui` | `aphrody_desktop` `0.1.0` | **7** | `cli, default, desktop, mobile, shell-module, vpn, web` |
| `crates/aphrody_store/Cargo.toml` | `aphrody-gpui` | `aphrody_store` `0.1.0` | **8** | `default, desktop, mobile, nsh-dev, nsh-js, nsh-ui, test-support, web` |
| `crates/nsh-core/Cargo.toml` | `aphrody-nsh` | `nsh-core` `0.6.0` | **7** | `declarations, glsl, loader, shader, svg, updater, websocket` |
| `crates/nsh-base/Cargo.toml` | `aphrody-nsh` | `nsh-base` `0.6.0` | **8** | `default, editor, emoji-picker, highlight, math, plot, rich-editor, term` |
| `crates/nsh-engine/Cargo.toml` | `aphrody-nsh` | `nsh-engine` `0.6.0` | **10** | `browser, crypto, default, js, js-core, net, shader, updater, +2` |
| `crates/nsh-platform-gpui/Cargo.toml` | `aphrody-nsh` | `nsh-platform-gpui` `0.6.0` | **21** | `crypto, default, dev, editor, emoji-picker, highlight, js, math, +13` |
| `crates/nsh-platform-web/Cargo.toml` | `aphrody-nsh` | `nsh-platform-web` `0.6.0` | **10** | `base-full, browser, browser-core, browser-tests, compiler, default, dev, linkable, +2` |
| `vendor/egui/Cargo.toml` | `egui` | `egui` `0.37.0` | **0** | `(none)` |
| `vendor/egui/crates/egui/Cargo.toml` | `egui` | `egui` `0.37.0` | **15** | `_override_unity, bytemuck, callstack, cint, color-hex, default, default_fonts, experimental, +7` |
| `vendor/egui/crates/egui-wgpu/Cargo.toml` | `egui` | `egui-wgpu` `0.37.0` | **7** | `capture, default, fragile-send-sync-non-atomic-wasm, macos-window-resize-jitter-fix, wayland, winit, x11` |
| `vendor/egui/crates/epaint/Cargo.toml` | `egui` | `epaint` `0.37.0` | **12** | `_override_unity, bytemuck, cint, color-hex, color_fonts, default, default_fonts, mint, +4` |
| `vendor/egui/crates/eframe/Cargo.toml` | `egui` | `eframe` `0.37.0` | **18** | `__screenshot, accesskit, android-game-activity, android-native-activity, clipboard, default, default_fonts, experimental, +10` |
| `crates/egui_gpui/Cargo.toml` | `egui` | `egui_gpui` `0.37.0` | **2** | `default, wgpu` |
| `crates/bevy_gpui/Cargo.toml` | `egui` | `bevy_gpui` `0.20.0` | **2** | `default, wgpu` |
| `vendor/craft-libs/Cargo.toml` | `craft-libs` | `craft-libs` `0.1.0` | **0** | `(none)` |
| `crates/aphrody_craft/Cargo.toml` | `craft-libs` | `aphrody_craft` `0.1.0` | **11** | `codecs, craft-anything, default, figcraft, full, native-inspect, storytold-suite, studio, +3` |
| `vendor/craft-anything/crates/analysis/Cargo.toml` | `craft-libs` | `craft-anything-analysis` `0.6.1` | **0** | `(none)` |
| `vendor/craft-libs/crates/sniff/Cargo.toml` | `craft-libs` | `craft-sniff` `0.1.0` | **0** | `(none)` |
| `vendor/craft-libs/crates/elf/Cargo.toml` | `craft-libs` | `craft-elf` `0.1.0` | **1** | `serde` |
| `vendor/craft-libs/crates/pe/Cargo.toml` | `craft-libs` | `craft-pe` `0.1.0` | **1** | `serde` |
| `vendor/craft-libs/crates/macho/Cargo.toml` | `craft-libs` | `craft-macho` `0.1.0` | **1** | `serde` |
| `vendor/craft-libs/crates/wasm/Cargo.toml` | `craft-libs` | `craft-wasm` `0.1.0` | **0** | `(none)` |
| `vendor/craft-libs/crates/fig/Cargo.toml` | `craft-libs` | `craft-fig` `0.1.0` | **0** | `(none)` |
| `vendor/craft-libs/crates/usd/Cargo.toml` | `craft-libs` | `craft-usd` `0.1.0` | **0** | `(none)` |
| `vendor/craft-libs/crates/psd/Cargo.toml` | `craft-libs` | `craft-psd` `0.1.0` | **5** | `corpus, default, serde, test-support, testgen` |
| `crates/iced_gpui/Cargo.toml` | `iced-libcosmic` | `iced_gpui` `0.14.0` | **3** | `cosmic-theme, default, wgpu` |
| `vendor/cosmic-text/Cargo.toml` | `iced-libcosmic` | `cosmic-text` `0.19.0` | **10** | `default, fontconfig, monospace_fallback, no_std, peniko, shape-run-cache, std, vi, +2` |
| `vendor/taffy/Cargo.toml` | `shared-foundation` | `taffy` `0.15.1` | **18** | `alloc, block_layout, calc, content_size, debug, default, detailed_layout_info, flexbox, +10` |
| `vendor/wgpu/wgpu/Cargo.toml` | `shared-foundation` | `wgpu` `0.6.0` | **25** | `angle, counters, custom, default, drm, dx12, exhaust, fragile-send-sync-non-atomic-wasm, +17` |
| `vendor/accesskit/accesskit/Cargo.toml` | `shared-foundation` | `accesskit` `0.25.2` | **4** | `enumn, pyo3, schemars, serde` |
| `vendor/tauri/wry/Cargo.toml` | `shared-foundation` | `wry` `0.57.3` | **8** | `default, devtools, linux-body, mac-proxy, os-webview, serde, tracing, x11` |
<!-- END GPUI_DIOXUS_FEATURES -->

---

## 6. Dependency Conflicts & `[patch]` Unification (`Cargo.lock`)

<!-- BEGIN GPUI_DIOXUS_BRIDGES -->
| Crate / Ecosystem | Status | Active Versions | Consumers & Root Cause | Resolution |
| :--- | :--- | :--- | :--- | :--- |
| `wgpu` | `unified-by-patch` | `30.0.0` | 30.0.0 <- aphrody_cli@0.1.0, egui-wgpu@0.37.0, gpui_wgpu@0.1.0, iced_gpui@0.14.0, nsh-engine@0.6.0 — Shared by gpui_wgpu, egui-wgpu, iced_gpui, nsh-engine, aphrody_cli; older upstream iced/eframe pull wgpu 24–28. | UNIFIED: Root [patch.crates-io] forces vendor/wgpu v30.0.0 across all crates; iced & libcosmic are adapted via iced_gpui. |
| `taffy` | `unified-by-patch` | `0.15.1` | 0.15.1 <- aphrody_session@0.1.0, gpui@0.2.2, nsh-core@0.6.0 — Layout engine for gpui, nsh-core, aphrody_session (vs upstream blitz-dom / dioxus). | UNIFIED: Root [patch.crates-io] forces vendor/taffy 0.15.1 single checkout. |
| `accesskit` | `unified-by-patch` | `0.25.2` | 0.25.2 <- accesskit_atspi_common@0.21.1, accesskit_consumer@0.39.2, accesskit_macos@0.27.2, accesskit_unix@0.24.1, accesskit_windows@0.35.2, aphrody_pilot@0.1.0 (+5) — Accessibility tree across gpui, gpui_{linux,macos,windows}, egui 0.37, aphrody_pilot. | UNIFIED: Root [patch.crates-io] forces vendor/accesskit 0.25.2 single version. |
| `cosmic-text` | `unified-by-patch` | `0.19.0` | 0.19.0 <- gpui_wgpu@0.1.0 — Text shaper across gpui_wgpu vs libcosmic; upstream cosmic-text versions diverge on harfrust/skrifa. | UNIFIED: Root [patch.crates-io] forces vendor/cosmic-text 0.19.0 for gpui_wgpu. |
| `ropey` | `lockfile-multi-version` | `1.6.1 | 2.0.0-beta.1` | 1.6.1 <- nsh-base@0.6.0, oxc_data_structures@0.153.0; 2.0.0-beta.1 <- gpui-base@0.10.0, gpui-component@0.10.0 — gpui-base/gpui-component require ropey =2.0.0-beta.1 (`metric_chars`, `metric_utf16`), whereas nsh-base & oxc_data_structures use ropey 1.6.1. | Kept intentionally split via `ropey_kit` workspace alias until nsh-base/oxc upgrade to ropey 2.0 stable. |
| `harfrust` | `lockfile-multi-version` | `0.12.0 | 0.15.0` | 0.12.0 <- cosmic-text@0.19.0; 0.15.0 <- epaint@0.37.0 — epaint 0.37 (vendor/egui) requires harfrust 0.15.0 + skrifa 0.49, whereas vendor/cosmic-text 0.19 pins harfrust 0.12.0 + skrifa 0.44. | Tracked across vendor/egui vs vendor/cosmic-text; bump cosmic-text upstream or keep egui behind optional feature. |
| `skrifa` | `lockfile-multi-version` | `0.44.0 | 0.49.0` | 0.44.0 <- cosmic-text@0.19.0, swash@0.2.10; 0.49.0 <- epaint@0.37.0, glifo@0.5.0 — Split 0.44.0 (cosmic-text) vs 0.49.0 (epaint/glifo); vendor/craft-libs workspace also references 0.47 (unused). | Aligns automatically when vendor/cosmic-text syncs upstream read-fonts 0.46 / skrifa 0.49. |
| `fontdb` | `lockfile-multi-version` | `0.23.0 | 0.24.0` | 0.23.0 <- nsh-platform-gpui@0.6.0, usvg@0.46.0; 0.24.0 <- cosmic-text@0.19.0 — nsh-platform-gpui & usvg 0.46 depend on fontdb 0.23.0 while cosmic-text 0.19 depends on fontdb 0.24.0. | Upstream resvg/usvg 0.47+ or fontique migration required to drop fontdb 0.23.0. |
| `imagesize` | `lockfile-multi-version` | `0.14.0 | 0.15.0` | 0.14.0 <- usvg@0.46.0; 0.15.0 <- nsh-bundler@0.6.0, nsh-core@0.6.0, nsh-macros@0.6.0 — nsh-{core,bundler,macros} use imagesize 0.15.0 while usvg 0.46 pulls imagesize 0.14.0. | Minor pure-Rust header parser split (~15 KB); drops once usvg bumps imagesize 0.15. |
| `lsp-types` | `lockfile-multi-version` | `0.95.1 | 0.97.0` | 0.95.1 <- lsp@0.1.0; 0.97.0 <- gpui-base@0.10.0, gpui-component@0.10.0, gpui-kit@0.10.0 — gpui-base/gpui-component/gpui-kit require crates.io lsp-types 0.97 (`InlineCompletionItem`), whereas Zed crates/lsp uses zed-industries/lsp-types 0.95.1. | Kept isolated via `lsp_types_kit` alias until upstream zed rebases onto lsp-types 0.97+. |
| `notify` | `lockfile-multi-version` | `8.2.2 | 9.0.0-rc.4` | 8.2.2 <- mullvad-daemon@0.0.0; 9.0.0-rc.4 <- fs@0.1.0, gpui-component@0.10.0, nsh-cli@0.6.0 — Zed fs, gpui-component, nsh-cli use zed-industries/notify 9.0.0-rc.4 while vendor/mullvad uses crates.io notify 8.2.2. | Mullein/Mullvad daemon isolates notify 8.2 behind aphrody_vpn; drops when notify 9.0 stabilizes. |
| `dirs` | `lockfile-multi-version` | `6.0.0 | 7.0.0` | 6.0.0 <- jupyter-zmq-client@3.0.1, shellexpand@3.1.1, zed-font-kit@0.14.1-zed; 7.0.0 <- aphrody_craft@0.1.0, aphrody_session@0.1.0, aphrody_store@0.1.0, aphrody_ui@0.1.0, aphrody_vpn@0.1.0, edit_prediction_cli@0.1.0 (+8) — Workspace updated dirs 6.0 -> 7.0 across Aphrody/Mullvad crates; zed-font-kit/shellexpand still pull dirs 6.0.0; vendor/egui references directories 6.0. | Root workspace converged on dirs 7.0; transitive 6.0 drops when zed-font-kit updates. |
| `thiserror` | `lockfile-multi-version` | `1.0.69 | 2.0.21` | 1.0.69 <- cairo-rs@0.18.5, filedescriptor@0.8.3, gio@0.18.4, glib@0.18.5, gotatun@0.10.3, handlebars@4.5.0 (+12); 2.0.21 <- agent@0.1.0, agent_servers@0.1.0, anthropic@0.1.0, aphrody_craft@0.1.0, aphrody_installer@0.1.0, aphrody_session@0.1.0 (+178) — Modern workspace + craft-libs use thiserror 2.0.21; glib 0.18, livekit 0.7, gotatun pull thiserror 1.0.69. | Harmless proc-macro split; shrinks as gtk/livekit migrate to thiserror 2. |
| `windows` | `lockfile-multi-version` | `0.56.0 | 0.57.0 | 0.58.0 | 0.61.3 | 0.62.2` | 0.56.0 <- trash@5.2.5; 0.57.0 <- sysinfo@0.31.4 — 5 major versions in Cargo.lock (0.56..0.62) due to trash (0.56), sysinfo 0.31 (0.57), ipc-channel (0.58), zed-scap/windows-capture (0.61) vs gpui/accesskit (0.62). | Track upstream zed bumps of zed-scap / ipc-channel to converge onto windows 0.62. |
| `craft-* (storytold/craft-libs)` | `unified-by-patch` | `0.1.0 (17 patched git+ overrides)` | crates/aphrody_craft; crates/bevy_gpui — Unpatched storytold/craft-libs git URLs previously triggered duplicate git checkouts & skrifa 0.47 conflicts. | UNIFIED: All 17 craft-* crates patched via [patch."https://github.com/storytold/craft-libs"] -> 0 git duplicates. |
| `iced + libcosmic + dioxus/blitz/anyrender` | `architectural-isolation` | `bridged in-crate (iced_gpui 0.14, aphrody_ui, gpui hot-reload)` | crates/iced_gpui; crates/aphrody_ui — Upstream iced/libcosmic pull incompatible wgpu 24–28 + winit 0.30 + sctk; Dioxus/Blitz split anyrender 0.14 vs 0.15 & vello 0.11 vs 0.12. | ISOLATED: Excluded from root workspace & de-vendored into reference skill; capabilities live natively in iced_gpui, aphrody_ui, egui_gpui & gpui. |
<!-- END GPUI_DIOXUS_BRIDGES -->

---

## 7. Multi-Framework Comparison Matrix

<!-- BEGIN GPUI_DIOXUS_ECOSYSTEM_MATRIX -->
| Dimension | GPUI / Kit / NSH | Dioxus / Blitz / AnyRender | egui 0.37 / craft-libs | Iced / libcosmic | Unification |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Reactivity & State** | Entity<T> slotmap + Context<T> (18 features); NSH Solid-style signals (10 features in nsh-engine) | GenerationalBox Signal<T> / RSX VDOM (crates.io reference) | Immediate-mode per-frame closure (15 egui features) + CraftStudioSession FigCraft history (11 features) | Elm Message/Task batch queue (3 features in iced_gpui) + OsConfigStore | Host long-lived state in GPUI Entity<T>; compose NSH TSX, egui inspectors, and Elm commands inside one window. |
| **Layout & Text Shaping** | Patched vendor/taffy 0.15.1 + gpui_wgpu CosmicTextSystem (vendor/cosmic-text 0.19) | Servo Stylo + stylo_taffy + Parley (de-vendored reference) | egui Grid/Strip layout + epaint harfrust 0.15 / skrifa 0.49; craft-fig/usd geometry decoders | Shares GPUI Taffy + M3/COSMIC breakpoints (aphrody_ui) + cosmic-text 0.19 | Single vendor/taffy across GPUI/NSH; converge cosmic-text up to harfrust 0.15 / skrifa 0.49 to eliminate epaint split. |
| **GPU & AccessKit Wiring** | Metal/D3D11 + gpui_wgpu (wgpu 30.0.0) + AccessKit 0.25.2 | AnyRender + Vello 0.11/0.12 (WgpuContext::shared_handles target) | egui-wgpu 0.37 patched onto vendor/wgpu 30.0.0 + vendor/accesskit 0.25.2 + Bevy Naga WGSL | iced_gpui runs on GPUI+wgpu 30 (avoids upstream iced wgpu 24–28 & sctk conflicts) | Zero duplicate WGPU/AccessKit crates anywhere in Cargo.lock. |
| **Design Tokens & Contrast** | Compile-time brand::design from google_design_tokens.toml across aphrody_ui, gpui-component, nsh-base | CSS tokens / M3 stylesheet (design-docs-pack) | egui_gpui::theme_bridge maps brand::design directly to egui::Visuals (WCAG AA verified) | cosmic_ron_to_os_theme + OsThemeCatalog (M3 + COSMIC RON palettes) | Zero duplicate hex tables; single compile-time source of truth in crates/brand. |
| **Measured Microbenchmarks** | GPUI tree+TSX build: 609 us; Lighthouse 100/100 | Hot-reload via subsecond =0.7.10 jump-table (debug devserver) | egui tessellate: 16473 us; craft_sniff: 2971 us / 100x; Bevy meshlets: 2565 us | Elm dispatch: 326 us / 1000x; libcosmic RON parse: 317 us / 100x | Measured live by `studio_parity_report` + `bun tooling/bun/gpui-vs-dioxus-audit.ts bench`. |
| **Dependency Conflict Status** | Lockfile: 2419 pkgs, 224 multi-version (35 patches) | subsecond =0.7.10 pin avoids dioxus 0.8 alpha anyrender/vello split | craft-libs: 17 patches (0 git dups); egui split: harfrust 0.15/skrifa 0.49 | Zero wgpu/winit/sctk conflict (upstream iced/libcosmic replaced by iced_gpui & aphrody_ui) | Next wins: align ropey 2.0 in nsh-base, bump cosmic-text -> harfrust 0.15, converge windows -> 0.62. |
<!-- END GPUI_DIOXUS_ECOSYSTEM_MATRIX -->

---

## 8. Host Tooling & Resolution Plan

<!-- BEGIN GPUI_DIOXUS_UNIFICATION -->
1. **Shared GPU, Layout, Text & A11y Foundations**: `vendor/wgpu` (`30.0.0`), `vendor/taffy` (`0.15.1`), `vendor/accesskit` (`0.25.2`), and `vendor/cosmic-text` (`0.19.0`) compile as single patched checkouts shared across GPUI, NSH, `egui 0.37`, `iced_gpui`, and `bevy_gpui`.
2. **Craft-Libs Zero-Duplicate Patching**: All `17` crates referenced upstream via `https://github.com/storytold/craft-libs` resolve through `[patch."https://github.com/storytold/craft-libs"]` to `vendor/craft-libs/crates/*`, leaving `0` duplicate git checkouts in [`Cargo.lock`](../../../Cargo.lock).
3. **Real Lockfile Splits to Target Next**: Align `nsh-base` onto `ropey 2.0` (when released), bump `vendor/cosmic-text` from `harfrust 0.12 / skrifa 0.44` to match `epaint 0.37` (`0.15 / 0.49`), and track Zed upgrades off `lsp-types 0.95.1`, `notify 8.2`, `dirs 6.0`, and legacy `windows 0.56..0.61`.
<!-- END GPUI_DIOXUS_UNIFICATION -->
