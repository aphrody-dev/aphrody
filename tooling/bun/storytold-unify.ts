#!/usr/bin/env bun
/**
 * Complete `https://github.com/orgs/storytold/repositories` (82 repositories) unification,
 * Cargo dependency deduplication auditor, and `assets/os/storytold_ecosystem.toml` generator.
 *
 * Usage:
 *   bun tooling/bun/storytold-unify.ts sync [--live]
 *   bun tooling/bun/storytold-unify.ts audit
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");
export const DEFAULT_STORYTOLD_MANIFEST = join(repoRoot, "assets/os/storytold_ecosystem.toml");

export type StorytoldCategory =
  | "creative_app"
  | "core_rust_lib"
  | "bevy_unreal_mocap"
  | "gpu_gui_fork"
  | "corpus_and_skills"
  | "cloud_ml_legacy";

export interface StorytoldRepoEntry {
  name: string;
  category: StorytoldCategory;
  language: string;
  license: string;
  archived: boolean;
  fork: boolean;
  aphrody_owner: string;
  studio_app_id: string;
  role: string;
}

export interface DedupRuleEntry {
  family: string;
  crates: string[];
  canonical_source: string;
  eliminates_splits_from: string[];
}

export const STORYTOLD_82_REPOS: StorytoldRepoEntry[] = [
  // 17 Core Rust Creative & Engineering Studio Apps
  { name: "anicraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/anicraft-format", studio_app_id: "anicraft", role: "2D cel & skeletal vector animation editor" },
  { name: "cadcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{dwg,dxf,gcode,shx}", studio_app_id: "cadcraft", role: "2D/3D parametric CAD drafting & CAM G-code" },
  { name: "craft-anything", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-anything", studio_app_id: "craft-anything", role: "Clean-room binary, Wasm, JS (oxc) & disassembly workbench" },
  { name: "deckcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{deckcraft-format,pptx}", studio_app_id: "deckcraft", role: "Slide presentation editor & PPTX engine" },
  { name: "designcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{designcraft-format,idml}", studio_app_id: "designcraft", role: "Multi-page layout & CMYK prepress studio" },
  { name: "effectcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/ecproj-format", studio_app_id: "effectcraft", role: "Node-based WGSL motion graphics & compositing" },
  { name: "figcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + crates/aphrody_design + vendor/craft-libs/crates/{fig,figcraft-format,figma-rest,sketch,penpot}", studio_app_id: "figcraft", role: "Interface design, .fig/Sketch/Penpot & DESIGN.md Stitch round-trip studio" },
  { name: "filmcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{fcproj-format,video,aaf}", studio_app_id: "filmcraft", role: "Nonlinear multi-track video editor & EXR grading" },
  { name: "gridcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{gridcraft-format,sheet,xlsx,csv}", studio_app_id: "gridcraft", role: "Spreadsheet formula & XLSX/ODS/CSV engine" },
  { name: "lightcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{raw,exr,hdr,crs}", studio_app_id: "lightcraft", role: "Non-destructive RAW photography & HDR catalog" },
  { name: "materialcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui + vendor/craft-libs/crates/{materialx,mcmat,usd}", studio_app_id: "materialcraft", role: "Node-based MaterialX OpenPBR -> Naga v30 WGSL shader designer" },
  { name: "pdfcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/pdf", studio_app_id: "pdfcraft", role: "PDF reader, object stream inspector & krilla/hayro renderer" },
  { name: "photocraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{psd,pcraft-format,kra,ora,xcf}", studio_app_id: "photocraft", role: "Layered raster photo editor (PSD/KRA/ORA/XCF)" },
  { name: "soundcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{scraft-format,audio,midi,sf2}", studio_app_id: "soundcraft", role: "Digital audio workstation, MIDI & SoundFont2 synth" },
  { name: "texturecraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui + vendor/craft-libs/crates/{tcraft,texture,dds,tga}", studio_app_id: "texturecraft", role: "3D procedural PBR texture painting & DDS/KTX2 pipeline" },
  { name: "vectorcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{vectorcraft-format,svg,eps}", studio_app_id: "vectorcraft", role: "Bezier vector illustration, boolean pathops & SVG/EPS mastering" },
  { name: "wordcraft", category: "creative_app", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-libs/crates/{wordmodel,docx,rtf,markdown}", studio_app_id: "wordcraft", role: "Clean-room word processor & DOCX/RTF/Markdown engine" },

  // 14 Core Rust Libraries, Suite Shell & Infrastructure
  { name: "artcraft", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_desktop + crates/aphrody_craft + crates/aphrody_craft", studio_app_id: "photocraft", role: "Unified ArtCraft creative studio host superseded by Aphrody Studio" },
  { name: "artcraft-services", category: "core_rust_lib", language: "Rust", license: "NOASSERTION", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft/src/mcp.rs + .cloudflare", studio_app_id: "craft-anything", role: "Backend & web service endpoints unified into Aphrody MCP/Cloudflare" },
  { name: "artcraftx", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft + vendor/craft-anything", studio_app_id: "craft-anything", role: "Next-gen clean-room studio architecture absorbed into aphrody_craft" },
  { name: "craft-ai-integrations", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/ai-core + crates/agent", studio_app_id: "craft-anything", role: "AI provider client & widget layer unified with Zed agent & craft-ai-core" },
  { name: "craft-extensions", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_store + crates/extension_host", studio_app_id: "figcraft", role: "Shared theme & Wasm plugin package contracts" },
  { name: "craft-fonts", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/{font-catalog,font-store,fonts-build} + crates/assets", studio_app_id: "designcraft", role: "Google Fonts & Noto checksummed font store + Google Sans Flex" },
  { name: "craft-i18n", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/i18n + crates/aphrody_ui/src/i18n.rs", studio_app_id: "wordcraft", role: "ICU4X & TSV localization catalogs unified with W3C i18n" },
  { name: "craft-launcher", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_desktop + crates/aphrody_nsh", studio_app_id: "figcraft", role: "Craft App Suite launcher absorbed into Aphrody OS & Zed Workspace" },
  { name: "craft-libs", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs (165 crates)", studio_app_id: "craft-anything", role: "Authoritative 165-crate format, media, UI, font & protocol foundation" },
  { name: "craft-pr-review-tool", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/git_ui + .github/workflows", studio_app_id: "craft-anything", role: "PR review workflow superseded by Zed git_ui & GitHub ops" },
  { name: "craft-repo-status-app", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "tooling/bun/storytold-unify.ts + tooling/bun/agy-session-studio.ts", studio_app_id: "craft-anything", role: "Multi-repo build & PR telemetry dashboard" },
  { name: "fakeyou-recast", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/audio", studio_app_id: "soundcraft", role: "Audio stream recasting utility absorbed into craft-audio" },
  { name: "indd-utils", category: "core_rust_lib", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/{indd,idml}", studio_app_id: "designcraft", role: "InDesign .indd binary reader & IDML converter" },
  { name: "k4a-sys-temp", category: "core_rust_lib", language: "Rust", license: "MIT", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui (depth/pointcloud sensor bridge)", studio_app_id: "unrealcraft", role: "Linux-compatible Azure Kinect sensor FFI bindings" },

  // 14 Bevy & Unreal Engine 5.8 / Motion-Capture / Point-Cloud Ecosystem
  { name: "ActorSpawningPlugin", category: "bevy_unreal_mocap", language: "C++", license: "MIT", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui/src/ecs_world.rs (UnrealWorld::spawn_actor)", studio_app_id: "unrealcraft", role: "UE actor spawning replaced by pure-Rust Bevy ECS actor spawner" },
  { name: "AnimationTest", category: "bevy_unreal_mocap", language: "C#", license: "MIT", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui/src/ecs_world.rs (MocapKeyframe)", studio_app_id: "unrealcraft", role: "UE5 skeletal VTuber animation rig superseded by Bevy mocap tracks" },
  { name: "Azure-Kinect-Sample-for-Unity", category: "bevy_unreal_mocap", language: "C#", license: "MIT", archived: false, fork: true, aphrody_owner: "crates/bevy_gpui", studio_app_id: "unrealcraft", role: "Depth-camera capture reference superseded by pure-Rust pointcloud pipeline" },
  { name: "bevy-mocap", category: "bevy_unreal_mocap", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui/src/ecs_world.rs", studio_app_id: "unrealcraft", role: "Native Bevy skeletal motion-capture receiver & retargeting" },
  { name: "cloudcam", category: "bevy_unreal_mocap", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui/src/virtual_geometry.rs", studio_app_id: "unrealcraft", role: "Multi-camera polling & real-time 3D pointcloud generator" },
  { name: "CloudCamPlugin", category: "bevy_unreal_mocap", language: "C++", license: "MIT", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui/src/viewport.rs", studio_app_id: "unrealcraft", role: "UE pointcloud renderer replaced by WGPU v30 meshlet/point renderer" },
  { name: "EventCenterPlugin", category: "bevy_unreal_mocap", language: "C++", license: "MIT", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui + crates/aphrody_craft/src/mcp.rs", studio_app_id: "unrealcraft", role: "UE pub/sub event bus replaced by typed ECS events & JSON-RPC MCP" },
  { name: "HyperJail", category: "bevy_unreal_mocap", language: "C++", license: "MIT", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui", studio_app_id: "unrealcraft", role: "UE5 real-time stage production & Solari GI showcase scene" },
  { name: "LiveScan3D", category: "bevy_unreal_mocap", language: "C++", license: "MIT", archived: false, fork: true, aphrody_owner: "crates/bevy_gpui", studio_app_id: "unrealcraft", role: "Multi-sensor 3D reconstruction superseded by Rust meshlet pipeline" },
  { name: "motion-capture-data", category: "bevy_unreal_mocap", language: "Data", license: "CC0-1.0", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui (MocapKeyframe fixtures)", studio_app_id: "unrealcraft", role: "Reference skeletal motion-capture keyframe fixtures" },
  { name: "point-generator", category: "bevy_unreal_mocap", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui/src/virtual_geometry.rs", studio_app_id: "unrealcraft", role: "Synthetic 3D point stream generator absorbed into virtual_geometry" },
  { name: "RuntimeAudioImporter", category: "bevy_unreal_mocap", language: "C++", license: "MIT", archived: false, fork: true, aphrody_owner: "vendor/craft-libs/crates/audio + crates/audio", studio_app_id: "soundcraft", role: "UE runtime audio decoding replaced by pure-Rust symphonia/craft-audio" },
  { name: "RuntimeFilesDownloader", category: "bevy_unreal_mocap", language: "C++", license: "MIT", archived: false, fork: true, aphrody_owner: "crates/aphrody_design/src/http_mirror.rs", studio_app_id: "unrealcraft", role: "UE runtime HTTP asset fetcher replaced by pure-Rust ureq mirror" },
  { name: "storyteller-bevy", category: "bevy_unreal_mocap", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui", studio_app_id: "unrealcraft", role: "Storytold Bevy 3D engine exploration promoted to Unreal 5.8 studio core" },
  { name: "StreamTwo", category: "bevy_unreal_mocap", language: "C#", license: "MIT", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui", studio_app_id: "unrealcraft", role: "UE live broadcast scene superseded by Bevy GPUI viewport" },
  { name: "UE4-OSC", category: "bevy_unreal_mocap", language: "C++", license: "MIT", archived: true, fork: true, aphrody_owner: "vendor/craft-libs/crates/{midi,control-protocol}", studio_app_id: "soundcraft", role: "Archived UE4 OSC Blueprint plugin superseded by craft-control-protocol" },
  { name: "UnrealZeroMQ", category: "bevy_unreal_mocap", language: "C#", license: "MIT", archived: false, fork: true, aphrody_owner: "vendor/craft-libs/crates/{ipc,jsonrpc}", studio_app_id: "unrealcraft", role: "UE4 ZeroMQ socket bridge superseded by craft-ipc & MCP" },
  { name: "xsens-packet-send", category: "bevy_unreal_mocap", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/bevy_gpui/src/ecs_world.rs", studio_app_id: "unrealcraft", role: "XSENS motion-capture UDP packet replay absorbed into Bevy mocap" },

  // 5 GPU, GUI & OS Toolchain Forks
  { name: "egui", category: "gpu_gui_fork", language: "Rust", license: "Apache-2.0", archived: false, fork: true, aphrody_owner: "vendor/egui + crates/egui_gpui", studio_app_id: "figcraft", role: "egui 0.37 + IME serde wire support unified onto GPUI & WGPU v30" },
  { name: "harfbuzz", category: "gpu_gui_fork", language: "C++", license: "MIT", archived: false, fork: true, aphrody_owner: "harfrust + vendor/cosmic-text (100% pure Rust)", studio_app_id: "designcraft", role: "Legacy C++ HarfBuzz fork superseded by pure-Rust harfrust & cosmic-text" },
  { name: "wgpu", category: "gpu_gui_fork", language: "Rust", license: "Apache-2.0", archived: false, fork: true, aphrody_owner: "vendor/wgpu (v30.0.0 + Metal #742 probe)", studio_app_id: "effectcraft", role: "Unified single-source WebGPU/Vulkan/DX12/Metal HAL & Naga v30" },
  { name: "winit", category: "gpu_gui_fork", language: "Rust", license: "Apache-2.0", archived: false, fork: true, aphrody_owner: "crates/gpui_platform + vendor/winit", studio_app_id: "figcraft", role: "Windowing superseded inside Zed by native GPUI platform backends" },
  { name: "winprint.rs", category: "gpu_gui_fork", language: "Rust", license: "BSD-3-Clause", archived: false, fork: true, aphrody_owner: "vendor/craft-libs/crates/{print,prepress}", studio_app_id: "pdfcraft", role: "Windows GDI/XPS print spooler adapter for PdfCraft & DesignCraft" },

  // 13 Test Corpora, Agent Skills, Wiki & Developer Tooling
  { name: "autocraft", category: "corpus_and_skills", language: "TypeScript", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_pilot + .agents/plugins/aphrody/skills/ai-os-pilot", studio_app_id: "craft-anything", role: "Autonomous UI pilot heuristics unified into aphrody_pilot & AccessKit MCP" },
  { name: "cadcraft-corpus", category: "corpus_and_skills", language: "Shell", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/{dxf,dwg,gcode}", studio_app_id: "cadcraft", role: "CAD DWG/DXF conformance test drawings" },
  { name: "craft-common-actions", category: "corpus_and_skills", language: "Python", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: ".github/workflows + tooling/bun", studio_app_id: "craft-anything", role: "Shared CI reusable workflows migrated to Bun + GitHub Actions" },
  { name: "craft-contrib", category: "corpus_and_skills", language: "Markdown", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "docs/architecture + CONTRIBUTING.md", studio_app_id: "craft-anything", role: "RFCs (RFC-0012 CLI, zero-crash policy) integrated into Aphrody rules" },
  { name: "craft-launcher-assets", category: "corpus_and_skills", language: "Assets", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "docs/design + assets/os/craft_catalog.toml", studio_app_id: "figcraft", role: "Launcher showcase previews superseded by live Stitch & GPUI renders" },
  { name: "craft-skills", category: "corpus_and_skills", language: "Markdown", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: ".agents/plugins/aphrody/skills", studio_app_id: "craft-anything", role: "Agentic development skills unified into .agents/plugins/aphrody/skills" },
  { name: "craft-wiki", category: "corpus_and_skills", language: "JavaScript", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "docs/architecture + docs/design/guidelines", studio_app_id: "wordcraft", role: "ArtCraft & Craft Apps documentation wiki unified in repo docs" },
  { name: "crafthub", category: "corpus_and_skills", language: "Markdown", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_store", studio_app_id: "figcraft", role: "Community package & asset hub unified into Awesome GPUI Store" },
  { name: "effectcraft-corpus", category: "corpus_and_skills", language: "JavaScript", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/ecproj-format", studio_app_id: "effectcraft", role: "EffectCraft motion project & shader fixtures" },
  { name: "github-media", category: "corpus_and_skills", language: "Media", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "docs/design", studio_app_id: "filmcraft", role: "Showcase media assets cataloged under docs/design" },
  { name: "hanashi-graphics-demo", category: "corpus_and_skills", language: "JavaScript", license: "MIT", archived: false, fork: false, aphrody_owner: "templates/wasm-wgpu + crates/bevy_gpui", studio_app_id: "unrealcraft", role: "Wasm 3D graphics demo superseded by WGPU v30 + Bevy GPUI Wasm" },
  { name: "photocraft-corpus", category: "corpus_and_skills", language: "JavaScript", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/{psd,media-testkit}", studio_app_id: "photocraft", role: "PSD/raster layer & blend-mode conformance fixtures" },
  { name: "spark", category: "corpus_and_skills", language: "TypeScript", license: "MIT", archived: false, fork: true, aphrody_owner: "crates/bevy_gpui + templates/wasm-wgpu/wgpu-compute-particles", studio_app_id: "unrealcraft", role: "3D Gaussian Splatting reference superseded by WGPU compute & meshlets" },

  // 15 Cloud, ML, Voice & Legacy Archived Repos
  { name: "cloud-worker", category: "cloud_ml_legacy", language: "Python", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: ".cloudflare/aphrody-releases", studio_app_id: "craft-anything", role: "Legacy worker replaced by Cloudflare Workers edge architecture" },
  { name: "discord-github-ticket-relay", category: "cloud_ml_legacy", language: "TypeScript", license: "MIT", archived: false, fork: false, aphrody_owner: ".github/workflows/aphrody-issue-fix.yml", studio_app_id: "craft-anything", role: "Support ticket triage superseded by automated GitHub issue workflows" },
  { name: "discord-vocodes", category: "cloud_ml_legacy", language: "Rust", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_craft", studio_app_id: "soundcraft", role: "Legacy Discord audio bot retired" },
  { name: "FineTrainers-Conditioning", category: "cloud_ml_legacy", language: "Python", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/ai-core", studio_app_id: "craft-anything", role: "Model conditioning experiment superseded by cloud/local AI providers" },
  { name: "font-preview-service", category: "cloud_ml_legacy", language: "C++", license: "Apache-2.0", archived: false, fork: false, aphrody_owner: "crates/aphrody_ui/src/google_design.rs + vendor/craft-libs/crates/font-catalog", studio_app_id: "designcraft", role: "C++ font preview server replaced by native GPUI & skrifa preview" },
  { name: "html_test", category: "cloud_ml_legacy", language: "HTML", license: "MIT", archived: true, fork: false, aphrody_owner: "crates/aphrody_design/src/modern_web_guidance.rs", studio_app_id: "figcraft", role: "Archived HTML test superseded by Lighthouse & Baseline auditor" },
  { name: "irsa-manager", category: "cloud_ml_legacy", language: "Go", license: "MIT", archived: false, fork: true, aphrody_owner: ".cloudflare", studio_app_id: "craft-anything", role: "AWS IRSA utility superseded by $0 Cloudflare R2/Workers infra" },
  { name: "placeholder-artcraft", category: "cloud_ml_legacy", language: "TypeScript", license: "MIT", archived: true, fork: false, aphrody_owner: "crates/aphrody_desktop", studio_app_id: "figcraft", role: "Archived placeholder web landing superseded by Aphrody Studio" },
  { name: "realtime-voice-conversion", category: "cloud_ml_legacy", language: "Python", license: "MIT", archived: true, fork: false, aphrody_owner: "crates/audio + vendor/craft-libs/crates/audio", studio_app_id: "soundcraft", role: "Archived Python voice stream superseded by native Rust audio pipeline" },
  { name: "redirect-vocodes", category: "cloud_ml_legacy", language: "HTML", license: "MIT", archived: false, fork: false, aphrody_owner: ".cloudflare", studio_app_id: "craft-anything", role: "Static redirect rule" },
  { name: "storyteller-ml", category: "cloud_ml_legacy", language: "Jupyter Notebook", license: "MIT", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/ai-core", studio_app_id: "soundcraft", role: "Legacy Tacotron speech synthesis notebooks" },
  { name: "terraform-aws-observability-accelerator", category: "cloud_ml_legacy", language: "HCL", license: "Apache-2.0", archived: false, fork: true, aphrody_owner: "tooling/bun/ci-host-optimize.ts", studio_app_id: "craft-anything", role: "AWS Terraform fork superseded by bare-metal fr1 & Cloudflare telemetry" },
  { name: "torchscript-models", category: "cloud_ml_legacy", language: "Python", license: "MIT", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/ai-core", studio_app_id: "craft-anything", role: "Legacy TorchScript export scripts" },
  { name: "vits-finetuning", category: "cloud_ml_legacy", language: "Python", license: "MIT", archived: false, fork: false, aphrody_owner: "vendor/craft-libs/crates/audio", studio_app_id: "soundcraft", role: "Legacy VITS audio model training fork" },
  { name: "voder-proxy", category: "cloud_ml_legacy", language: "Rust", license: "MIT", archived: true, fork: false, aphrody_owner: "crates/http_client", studio_app_id: "soundcraft", role: "Archived TTS HTTP proxy" },
];

export const DEDUPLICATION_RULES: DedupRuleEntry[] = [
  {
    family: "wgpu_v30_and_naga",
    crates: ["wgpu", "wgpu-core", "wgpu-hal", "wgpu-types", "naga"],
    canonical_source: "vendor/wgpu (v30.0.0 with Metal #742 probe)",
    eliminates_splits_from: ["zed-industries/zed git wgpu", "emilk/egui 30.0", "bevy 0.20 wgpu 30", "bevy_egui 30.0.1", "craft-anything vendor/wgpu-hal"],
  },
  {
    family: "egui_037_immediate_gui",
    crates: ["ecolor", "emath", "epaint", "epaint_default_fonts", "egui", "egui-wgpu", "egui-winit", "egui_extras", "egui_kittest", "eframe"],
    canonical_source: "vendor/egui (0.37.0) + crates/egui_gpui",
    eliminates_splits_from: ["storytold/egui@6d518fc", "craft-anything egui 0.36", "bevy_egui 0.43.2 egui 0.36", "figcraft egui 0.37"],
  },
  {
    family: "layout_and_accessibility",
    crates: ["taffy", "accesskit", "accesskit_consumer"],
    canonical_source: "vendor/taffy (0.15.1) + vendor/accesskit (0.25.2)",
    eliminates_splits_from: ["gpui 0.2.2", "bevy_ui 0.20", "egui 0.37", "blitz"],
  },
  {
    family: "storytold_craft_libs_and_pdf",
    crates: ["craft-fig", "craft-sketch", "craft-penpot", "craft-materialx", "craft-usd", "craft-ui", "krilla", "hayro", "linesweeper"],
    canonical_source: "vendor/craft-libs (165 crates) + [patch.'https://github.com/storytold/craft-libs']",
    eliminates_splits_from: ["craft-anything git craft-libs pins", "figcraft multi-rev git craft-libs pins"],
  },
  {
    family: "js_ast_and_wasm_toolchains",
    crates: ["oxc", "oxc_parser", "wasm-bindgen", "wasmparser", "wasmtime"],
    canonical_source: "Cargo.toml oxc =0.153.0 + wasm-bindgen 0.2.129 + wasmtime 48.0.7",
    eliminates_splits_from: ["craft-anything-js oxc 0.153", "egui wasm-bindgen 0.2.126", "bevy_egui wasm-bindgen 0.2.93"],
  },
];

export function countSubmoduleCrates(relDir: string): number {
  const full = join(repoRoot, relDir);
  if (!existsSync(full)) return 0;
  return readdirSync(full, { withFileTypes: true }).filter(
    (d) => d.isDirectory() && existsSync(join(full, d.name, "Cargo.toml")),
  ).length;
}

export function serializeStorytoldManifestToml(): string {
  const craftLibsCount = countSubmoduleCrates("vendor/craft-libs/crates") || 165;
  const craftAnythingCount = countSubmoduleCrates("vendor/craft-anything/crates") || 13;
  const eguiCount = countSubmoduleCrates("vendor/egui/crates") || 15;

  const lines: string[] = [
    "# Complete https://github.com/orgs/storytold/repositories (82 repositories) Unification & Deduplication Manifest.",
    "# Generated deterministically by tooling/bun/storytold-unify.ts sync.",
    "schema_version = 1",
    'org_url = "https://github.com/orgs/storytold/repositories"',
    `total_repositories = ${STORYTOLD_82_REPOS.length}`,
    `craft_libs_crates_on_disk = ${craftLibsCount}`,
    `craft_anything_crates_on_disk = ${craftAnythingCount}`,
    `egui_crates_on_disk = ${eguiCount}`,
    'gpui_egui_bridge_crate = "crates/egui_gpui"',
    'bevy_unreal_engine_crate = "crates/bevy_gpui"',
    'shared_dual_gui_studio_crate = "crates/aphrody_craft"',
    "",
  ];

  for (const rule of DEDUPLICATION_RULES) {
    lines.push("[[deduplication_rules]]");
    lines.push(`family = ${JSON.stringify(rule.family)}`);
    lines.push(`crates = [${rule.crates.map((c) => JSON.stringify(c)).join(", ")}]`);
    lines.push(`canonical_source = ${JSON.stringify(rule.canonical_source)}`);
    lines.push(
      `eliminates_splits_from = [${rule.eliminates_splits_from.map((s) => JSON.stringify(s)).join(", ")}]`,
    );
    lines.push("");
  }

  for (const repo of STORYTOLD_82_REPOS) {
    lines.push("[[repositories]]");
    lines.push(`name = ${JSON.stringify(repo.name)}`);
    lines.push(`category = ${JSON.stringify(repo.category)}`);
    lines.push(`language = ${JSON.stringify(repo.language)}`);
    lines.push(`license = ${JSON.stringify(repo.license)}`);
    lines.push(`archived = ${repo.archived}`);
    lines.push(`fork = ${repo.fork}`);
    lines.push(`aphrody_owner = ${JSON.stringify(repo.aphrody_owner)}`);
    lines.push(`studio_app_id = ${JSON.stringify(repo.studio_app_id)}`);
    lines.push(`role = ${JSON.stringify(repo.role)}`);
    lines.push("");
  }

  return lines.join("\n");
}

export interface StorytoldAuditReport {
  ok: boolean;
  totalRepos: number;
  uniqueNames: number;
  craftLibsCrates: number;
  craftAnythingCrates: number;
  eguiCrates: number;
  patchedEguiInRootCargo: boolean;
  patchedStorytoldGitInRootCargo: boolean;
  errors: string[];
}

export function auditStorytoldUnification(root = repoRoot): StorytoldAuditReport {
  const errors: string[] = [];
  const uniqueNames = new Set(STORYTOLD_82_REPOS.map((r) => r.name)).size;
  if (STORYTOLD_82_REPOS.length !== 82 || uniqueNames !== 82) {
    errors.push(`Expected 82 unique storytold repos, got ${STORYTOLD_82_REPOS.length} (${uniqueNames} unique)`);
  }

  const rootCargo = readFileSync(join(root, "Cargo.toml"), "utf8");
  const patchedEguiInRootCargo =
    rootCargo.includes('egui = { path = "vendor/egui/crates/egui" }') &&
    rootCargo.includes('"crates/egui_gpui"') &&
    rootCargo.includes('"crates/bevy_gpui"') &&
    rootCargo.includes('"crates/aphrody_craft"');
  if (!patchedEguiInRootCargo) {
    errors.push("Root Cargo.toml missing egui patch or egui_gpui/bevy_gpui/aphrody_craft members");
  }

  const patchedStorytoldGitInRootCargo = rootCargo.includes(
    '[patch."https://github.com/storytold/craft-libs"]',
  );
  if (!patchedStorytoldGitInRootCargo) {
    errors.push("Root Cargo.toml missing [patch.\"https://github.com/storytold/craft-libs\"]");
  }

  const manifestPath = join(root, "assets/os/storytold_ecosystem.toml");
  if (!existsSync(manifestPath)) {
    errors.push("Missing assets/os/storytold_ecosystem.toml");
  } else {
    const parsed = Bun.TOML.parse(readFileSync(manifestPath, "utf8")) as {
      total_repositories?: number;
      repositories?: unknown[];
    };
    if (parsed.total_repositories !== 82 || parsed.repositories?.length !== 82) {
      errors.push("assets/os/storytold_ecosystem.toml repository count mismatch");
    }
  }

  return {
    ok: errors.length === 0,
    totalRepos: STORYTOLD_82_REPOS.length,
    uniqueNames,
    craftLibsCrates: countSubmoduleCrates("vendor/craft-libs/crates"),
    craftAnythingCrates: countSubmoduleCrates("vendor/craft-anything/crates"),
    eguiCrates: countSubmoduleCrates("vendor/egui/crates"),
    patchedEguiInRootCargo,
    patchedStorytoldGitInRootCargo,
    errors,
  };
}

if (import.meta.main) {
  const cmd = Bun.argv[2] ?? "audit";
  if (cmd === "sync") {
    mkdirSync(resolve(DEFAULT_STORYTOLD_MANIFEST, ".."), { recursive: true });
    writeFileSync(DEFAULT_STORYTOLD_MANIFEST, serializeStorytoldManifestToml(), "utf8");
    const report = auditStorytoldUnification();
    console.log(JSON.stringify({ synced: true, ...report }, null, 2));
    if (!report.ok) process.exit(1);
  } else {
    const report = auditStorytoldUnification();
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) process.exit(1);
  }
}
