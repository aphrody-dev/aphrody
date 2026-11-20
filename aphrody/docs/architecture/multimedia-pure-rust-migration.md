# Pure-Rust Desktop OS, Multimedia Studio, Google Workspace, Cloud AI & YouTube Architecture

Canonical architecture and migration ledger for unifying the pure-Rust multimedia engine, dual YouTube/YouTube Music clients (headless agents + GPUI humans), complete Google Workspace & Cloud AI stack via `gcloud`/OAuth 2.0, Material 3 Expressive (Android 17/18) desktop/mobile design system, and auto-merged `aphrody-labs/*` reference forks.

## 1. Core Architectural Contract

- **Single Pure-Rust Desktop & Web Codebase**: runs natively on **Wayland**, **Windows**, **macOS**, **WebAssembly (`wasm32-unknown-unknown` / `wasm32-wasip2`)**, and **Chromium WebGPU** via `gpui`, `gpui_wgpu` (`wgpu 30` + `naga 30`), `aphrody_browser`, and `aphrody_desktop`.
- **Material 3 Expressive (Android 17/18 + Compose M3 1.5.0)**: unified 3-tier token graph in [`assets/os/google_design_tokens.toml`](../../../assets/os/google_design_tokens.toml), runtime tokens in [`crates/aphrody_ui/src/google_design.rs`](../../../crates/aphrody_ui/src/google_design.rs), live HCT dynamic color (`material-colors 0.5`), vector icons (`md-icons 0.4` + `lucide-static-svg 1.76`), Liquid Glass (`backdrop-blur-core 0.2` + `window-vibrancy 0.8`), and 35 `MaterialShapes` WGSL shaders ([`assets/shaders/m3_expressive.wgsl`](../../../assets/shaders/m3_expressive.wgsl)).
- **Zero Blocking Submodules**: heavy upstreams (`remotion`, `three.js`, `hyperframes`, `mcp-toolbox`, `google-cloud-rust`) remain GitHub-side auto-merged forks tracked under `referenceForks` in [`tooling/bun/forks-manifest.json`](../../../tooling/bun/forks-manifest.json) and synchronized automatically by [`tooling/bun/multimedia-forks-audit.ts`](../../../tooling/bun/multimedia-forks-audit.ts) and [`.github/workflows/aphrody-upstream-sync.yml`](../../../.github/workflows/aphrody-upstream-sync.yml).
- **Zero FFmpeg / Zero Headless Chromium / Zero Python Core**: all containers, codecs, GPU YUV->RGB shaders, audio playback, programmatic video rendering, video indexing, Google Cloud/Workspace/YouTube APIs, and 3D WebGPU pipelines execute natively in Rust.

## 2. Pure-Rust Equivalence Matrix (Google Rust / Go / TS / Python / Kotlin + Multimedia)

| Domain | Upstream Reference Forks (`aphrody-labs/*`) | Pure-Rust Aphrody Target |
| :--- | :--- | :--- |
| **Containers, Codecs, RAW, HDR, Print** | `craft-libs` (pinned in `Cargo.toml`), `filmcraft`, `photocraft`, `lightcraft`, `effectcraft`, `soundcraft` | [`crates/aphrody_media`](../../../crates/aphrody_media/Cargo.toml) (`player`, `hw_decode`, `gpu_yuv`, `audio_out`, `subtitles`, `timeline`, `photo_dev`, `export`) |
| **Network Ingest & Streaming** | `yt-dlp`, `omniget`, `libreget`, `kkdai/youtube` | `crates/aphrody_media::ingest` (HTTP range, HLS `.m3u8` + AES-128, DASH `.mpd`, SponsorBlock, workspace JS solver, opt-in extractor ABI) |
| **Programmatic Video & Shaders** | `hyperframes`, `remotion` | `crates/aphrody_media::motion` + [`crates/aphrody_nsh`](../../../crates/aphrody_nsh/Cargo.toml) TSX offscreen `gpui_wgpu` renderer -> `craft-video` |
| **Video Intelligence & Verification** | `watch-skill`, `filmcraft` (`candle` Whisper/Kokoro) | `crates/aphrody_media::watch` (luminance scene delta + `blake3` keyframes + SQLite FTS5 + Gemini multimodal) + `aphrody_pilot` MCP |
| **3D WebGPU, Gaussian Splat & TSL** | `three.js` (`WebGPU`, `TSL`, 57 loaders) | [`crates/bevy_gpui`](../../../crates/bevy_gpui/Cargo.toml) (glTF 2.0, 3D Gaussian Splat radix WGSL, PMREM IBL, 3D LUT `.cube`, Rust shader-node graph -> `naga 30` IR) |
| **Google OAuth 2.0 & `gcloud` Auth** | `google-cloud-rust`, `google-workspace-cli` (`googleworkspace/cli`), `gemini-cli`, `yup-oauth2`, `gcp_auth` | [`crates/aphrody_google`](../../../crates/aphrody_google/Cargo.toml)`::auth` + [`crates/oauth_callback_server`](../../../crates/oauth_callback_server/src/oauth_callback_server.rs) (PKCE loopback, Device Flow, ADC, `gcloud auth print-access-token` + `x-goog-user-project` fallback) |
| **Google Cloud AI (Kotlin/Go/Python/TS parity)** | `go-genai`, `js-genai`, `python-genai`, `java-genai`, `google-cloud-rust` (`aiplatform-v1`, `speech-v2`, `texttospeech-v1`, `vision-v1`), `gemini-rust`, `live-api-web-console`, `computer-use-preview` | `crates/aphrody_google::ai` + [`crates/google_ai`](../../../crates/google_ai/src/google_ai.rs) + [`crates/language_models`](../../../crates/language_models/src/provider/google.rs) (Gemini API + Vertex AI + Imagen/Veo/Lyria/Live/STT/TTS/Vision/Video Intelligence) |
| **Google Workspace (Drive, Gmail, Calendar, Sheets, Docs, Chat)** | `google-workspace-cli` (`googleworkspace/cli`), `gogcli`, `google_workspace_mcp`, `rclone`, `google-apis-rs` | `crates/aphrody_google::workspace` (Discovery Service dynamic client + chunked Drive sync + `ws_*` MCP & CLI commands) |
| **YouTube & YouTube Music (Headless + GPUI)** | `youtui` (`ytmapi-rs`), `rustypipe`, `YouTube.js`, `ytmusicapi`, `pear-desktop`, `sonora`, `google-apis-rs` (`google-youtube3`) | `crates/aphrody_google::{youtube_data, innertube, ytmusic}` + `aphrody_cli` (`yt`/`ytm` JSON/NDJSON) + `aphrody_pilot` MCP + `aphrody_desktop` (`YouTube`, `YouTubeMusic`) |
| **Agent Protocols (ADK, A2A 1.0, MCP Toolbox, A2UI)** | `adk-rust`, `adk-go`, `adk-js`, `adk-python`, `adk-java`, `A2A`, `a2a-rs`, `mcp-toolbox`, `a2ui`, `genkit`, `gemini-skills` | [`crates/aphrody_pilot`](../../../crates/aphrody_pilot/Cargo.toml) (MCP + A2A 1.0 + Computer Use AccessKit) + `aphrody_nsh` (A2UI streaming renderer) |
| **Material 3 Expressive Android 17/18** | `material-components-android`, Compose M3 1.5.0, `design.google` | [`assets/os/google_design_tokens.toml`](../../../assets/os/google_design_tokens.toml) + [`crates/aphrody_ui`](../../../crates/aphrody_ui/src/google_design.rs) + [`tooling/bun/design-maquettes.ts`](../../../tooling/bun/design-maquettes.ts) |

## 3. Desktop OS & Headless Agent Surfaces

- **Human GPUI Desktop (`crates/aphrody_desktop`)**:
  - `WindowKind::GoogleWorkspace` — Unified Drive, Gmail, Calendar, Docs, Sheets, and `gcloud` / OAuth 2.0 account control center.
  - `WindowKind::YouTube` — GPU video player (`gpui_wgpu`), chapters, subtitles, SponsorBlock, quality selector, offline download, and inline Gemini summary.
  - `WindowKind::YouTubeMusic` — Gapless Opus/AAC playback (`cpal`), synced lyrics, dynamic artwork HCT palette (`material-colors 0.5`), queue, radio/automix, and Dock mini-player.
  - `WindowKind::MediaPlayer` & `WindowKind::MediaStudio` — Universal player + NLE timeline, WGSL scopes, RAW photo develop, programmatic TSX motion, 3D `bevy_gpui` viewport, and AI generation (Imagen/Veo/Lyria/TTS).
- **Headless Agent Control Plane (`crates/aphrody_cli` + `crates/aphrody_pilot`)**:
  - `aphrody google`, `aphrody ws`, `aphrody yt`, `aphrody ytm`, `aphrody media` with deterministic `--json` / `--ndjson` output.
  - MCP tools: `google_auth_status`, `google_ai_generate`, `google_ai_analyze_media`, `ws_discovery_call`, `yt_search`, `yt_video_context`, `yt_transcript`, `ytm_search`, `ytm_lyrics`, `ytm_play_control`, `media_watch_index`, `media_watch_query`, `media_watch_verify`, `media_ingest_probe`, `media_render_composition`.

## 4. Verification Contract

- Fork sync audit: `bun tooling/bun/multimedia-forks-audit.ts --check`
- Tooling & hygiene: `bun run test:tooling`, `bun run hygiene`, `bun run design:audit`
- Native Rust checks: `cargo check -p aphrody_google -p aphrody_media -p bevy_gpui -p aphrody_pilot -p aphrody_desktop` and unit tests (`cargo test`)
- Wasm + WebGPU targets: `bun tooling/bun/web-build.ts check` (`wasm32-unknown-unknown` + `wasm32-wasip2`)
- Visual maquettes: `bun tooling/bun/design-maquettes.ts build` + headless Chromium CDP screenshot verification
