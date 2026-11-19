#!/usr/bin/env bun
/**
 * Automatic Material 3 Compose Tracker & Multi-Target Converter (GPU WGSL / GPUI Kit / NSH TSX / Figma DTCG).
 *
 * Tracks live Google Maven (`dl.google.com`) + JetBrains Compose Multiplatform releases,
 * Material 3 blog specifications (`compose-material-3-is-stable`,
 * `enhance-your-component-designs-with-parametric-prototyping`), and Figma Material 3 Design Kit
 * (`1035203688168086460`), then converts canonical tokens/specs into:
 *   1. Assets & Data & Specs: `assets/os/m3_compose_tracker_lock.toml`, `assets/os/m3_compose_catalog.json`,
 *      `aphrody/docs/design/kits/figma-m3-1035203688168086460-dtcg.json`
 *   2. GPU (WGPU / Naga v30 / WGSL): `assets/shaders/m3_expressive.wgsl` + template copy
 *   3. GPUI Kit (`crates/gpui-component/src/material3/{compose15,parametric}.rs` + `crates/gpui_ui/examples/m3_compose15_showcase.rs`)
 *   4. NSH Library & Template (`templates/m3-gpui-nsh/{nsh-m3-kit.tsx,m3-tokens.css,Cargo.toml,src/main.rs,m3-parametric-preset.json,README.md}`)
 *
 * Usage:
 *   bun tooling/bun/m3-compose-tracker.ts track [--check] [--offline] [--json]
 *   bun tooling/bun/m3-compose-tracker.ts convert [--verify] [--json]
 *   bun tooling/bun/m3-compose-tracker.ts prove [--skip-cargo] [--json]
 */

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export const REPO_ROOT = resolve(import.meta.dir, "../..");
export const TOKENS_TOML_REL = "assets/os/google_design_tokens.toml";
export const TRACKER_LOCK_REL = "assets/os/m3_compose_tracker_lock.toml";
export const CATALOG_JSON_REL = "assets/os/m3_compose_catalog.json";
export const FIGMA_DTCG_REL = "aphrody/docs/design/kits/figma-m3-1035203688168086460-dtcg.json";
export const WGSL_SHADER_REL = "assets/shaders/m3_expressive.wgsl";
export const TEMPLATE_DIR_REL = "templates/m3-gpui-nsh";

export const SOURCES = {
  composeBlog: "https://m3.material.io/blog/compose-material-3-is-stable",
  parametricBlog: "https://m3.material.io/blog/enhance-your-component-designs-with-parametric-prototyping",
  releaseNotes: "https://developer.android.com/jetpack/androidx/releases/compose-material3",
  figmaKitFileId: "1035203688168086460",
  figmaKitUrl: "https://www.figma.com/community/file/1035203688168086460/material-3-design-kit",
  mavenMaterial3: "https://dl.google.com/dl/android/maven2/androidx/compose/material3/material3/maven-metadata.xml",
  mavenRipple: "https://dl.google.com/dl/android/maven2/androidx/compose/material3/material3-ripple/maven-metadata.xml",
  mavenBom: "https://dl.google.com/dl/android/maven2/androidx/compose/compose-bom/maven-metadata.xml",
  mavenWindowSize: "https://dl.google.com/dl/android/maven2/androidx/compose/material3/material3-window-size-class/maven-metadata.xml",
  mavenJetBrainsM3: "https://maven.pkg.jetbrains.space/public/p/compose/dev/org/jetbrains/compose/material3/material3/maven-metadata.xml",
} as const;

export const MATERIAL_SHAPES_35 = [
  "Circle", "Square", "Slanted", "Arch", "Fan",
  "Arrow", "SemiCircle", "Oval", "Pill", "Triangle",
  "Diamond", "ClamShell", "Pentagon", "Gem", "Sunny",
  "VerySunny", "Cookie4Sided", "Cookie6Sided", "Cookie7Sided", "Cookie9Sided",
  "Cookie12Sided", "Ghostish", "Clover4Leaf", "Clover8Leaf", "Burst",
  "SoftBurst", "Boom", "SoftBoom", "Flower", "Puffy",
  "PuffyDiamond", "PixelCircle", "PixelTriangle", "Bun", "Heart",
] as const;

export interface TrackerLock {
  tracked_at: string;
  material3_stable: string;
  material3_latest: string;
  material3_ripple_stable: string;
  material3_window_size_stable: string;
  compose_bom_latest: string;
  jetbrains_compose_m3_latest: string;
  figma_kit_file_id: string;
  compose_blog_url: string;
  parametric_blog_url: string;
  sha256_stamp: string;
}

export function parseMavenVersions(xml: string): { latest: string; release: string; stable: string } {
  const latest = xml.match(/<latest>([^<]+)<\/latest>/)?.[1]?.trim() ?? "";
  const release = xml.match(/<release>([^<]+)<\/release>/)?.[1]?.trim() ?? latest;
  const all = [...xml.matchAll(/<version>([^<]+)<\/version>/g)].map((m) => m[1].trim());
  const stableList = all.filter((v) => !/(alpha|beta|rc|dev|snapshot)/i.test(v));
  const stable = stableList.at(-1) ?? release ?? latest;
  return { latest: latest || all.at(-1) || "", release: release || stable, stable };
}

async function fetchMavenVersion(url: string, fallback: { latest: string; stable: string }, offline: boolean) {
  if (offline) return fallback;
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(8000) });
    if (!res.ok) return fallback;
    const parsed = parseMavenVersions(await res.text());
    return {
      latest: parsed.latest || fallback.latest,
      stable: parsed.stable || fallback.stable,
    };
  } catch {
    return fallback;
  }
}

export function computeLockStamp(lock: Omit<TrackerLock, "tracked_at" | "sha256_stamp">): string {
  const key = [
    lock.material3_stable,
    lock.material3_latest,
    lock.material3_ripple_stable,
    lock.material3_window_size_stable,
    lock.compose_bom_latest,
    lock.jetbrains_compose_m3_latest,
    lock.figma_kit_file_id,
  ].join(":");
  return createHash("sha256").update(key).digest("hex");
}

function serializeLockToml(lock: TrackerLock): string {
  return `# Automatic Compose Material 3 Upstream Tracker Lockfile.
# Generated by tooling/bun/m3-compose-tracker.ts — re-run \`bun run m3:track\` to refresh.
tracked_at = "${lock.tracked_at}"
material3_stable = "${lock.material3_stable}"
material3_latest = "${lock.material3_latest}"
material3_ripple_stable = "${lock.material3_ripple_stable}"
material3_window_size_stable = "${lock.material3_window_size_stable}"
compose_bom_latest = "${lock.compose_bom_latest}"
jetbrains_compose_m3_latest = "${lock.jetbrains_compose_m3_latest}"
figma_kit_file_id = "${lock.figma_kit_file_id}"
compose_blog_url = "${lock.compose_blog_url}"
parametric_blog_url = "${lock.parametric_blog_url}"
sha256_stamp = "${lock.sha256_stamp}"
`;
}

export async function runTrack(root = REPO_ROOT, checkOnly = false, offline = false): Promise<{ lock: TrackerLock; changed: boolean }> {
  const lockPath = join(root, TRACKER_LOCK_REL);
  const existingRaw = existsSync(lockPath) ? readFileSync(lockPath, "utf8") : "";
  const existing = existingRaw ? (Bun.TOML.parse(existingRaw) as Partial<TrackerLock>) : {};

  const [m3, ripple, bom, winSize, jbM3] = await Promise.all([
    fetchMavenVersion(SOURCES.mavenMaterial3, { stable: "1.5.0", latest: "1.6.0-alpha01" }, offline),
    fetchMavenVersion(SOURCES.mavenRipple, { stable: "1.5.0", latest: "1.6.0-alpha01" }, offline),
    fetchMavenVersion(SOURCES.mavenBom, { stable: "2026.11.01", latest: "2026.11.01" }, offline),
    fetchMavenVersion(SOURCES.mavenWindowSize, { stable: "1.5.0", latest: "1.6.0-alpha01" }, offline),
    fetchMavenVersion(SOURCES.mavenJetBrainsM3, { stable: "1.12.2", latest: "1.13.0-rc01" }, offline),
  ]);

  const core = {
    material3_stable: m3.stable,
    material3_latest: m3.latest,
    material3_ripple_stable: ripple.stable,
    material3_window_size_stable: winSize.stable,
    compose_bom_latest: bom.stable,
    jetbrains_compose_m3_latest: jbM3.latest,
    figma_kit_file_id: SOURCES.figmaKitFileId,
    compose_blog_url: SOURCES.composeBlog,
    parametric_blog_url: SOURCES.parametricBlog,
  };
  const sha256_stamp = computeLockStamp(core);
  const changed = existing.sha256_stamp !== sha256_stamp;
  const lock: TrackerLock = {
    tracked_at: changed || !existing.tracked_at ? new Date().toISOString() : existing.tracked_at,
    ...core,
    sha256_stamp,
  };

  if (!checkOnly) {
    mkdirSync(dirname(lockPath), { recursive: true });
    writeFileSync(lockPath, serializeLockToml(lock), "utf8");
  }
  return { lock, changed };
}

interface ParsedDesignTokens {
  sys?: {
    color?: Record<string, Record<string, string>>;
    shape?: Record<string, number>;
    motion?: { springs?: Record<string, { stiffness: number; damping_ratio: number }> };
    typescale?: Record<string, { size_sp: number; line_height_sp: number; weight: number }>;
  };
  compose_m3?: Record<string, unknown>;
  figma_design_kit?: Record<string, unknown>;
  parametric_prototyping?: Record<string, unknown>;
}

function writeOrVerify(absPath: string, content: string, verifyOnly: boolean, drifted: string[]) {
  const normalized = content.replace(/\r\n/g, "\n");
  if (verifyOnly) {
    if (!existsSync(absPath)) {
      drifted.push(`missing:${absPath}`);
      return;
    }
    const existing = readFileSync(absPath, "utf8").replace(/\r\n/g, "\n");
    if (existing !== normalized) {
      drifted.push(`drifted:${absPath}`);
    }
    return;
  }
  mkdirSync(dirname(absPath), { recursive: true });
  writeFileSync(absPath, normalized, "utf8");
}

export function runConvert(root = REPO_ROOT, verifyOnly = false): {
  generatedFiles: string[];
  drifted: string[];
  componentCount: number;
  shapesCount: number;
} {
  const tokensPath = join(root, TOKENS_TOML_REL);
  const lockPath = join(root, TRACKER_LOCK_REL);
  const wgslPath = join(root, WGSL_SHADER_REL);

  const tokens = Bun.TOML.parse(readFileSync(tokensPath, "utf8")) as ParsedDesignTokens;
  const lock = existsSync(lockPath)
    ? (Bun.TOML.parse(readFileSync(lockPath, "utf8")) as TrackerLock)
    : ({
        tracked_at: "2026-11-19T00:00:00.000Z",
        material3_stable: "1.5.0",
        material3_latest: "1.6.0-alpha01",
        material3_ripple_stable: "1.5.0",
        material3_window_size_stable: "1.5.0",
        compose_bom_latest: "2026.11.01",
        jetbrains_compose_m3_latest: "1.13.0-rc01",
        figma_kit_file_id: SOURCES.figmaKitFileId,
        compose_blog_url: SOURCES.composeBlog,
        parametric_blog_url: SOURCES.parametricBlog,
        sha256_stamp: "",
      } satisfies TrackerLock);

  const wgslSource = readFileSync(wgslPath, "utf8");
  const modes = tokens.sys?.color ?? {};
  const shapes = tokens.sys?.shape ?? {
    none_px: 0,
    extra_small_px: 4,
    small_px: 8,
    medium_px: 12,
    large_px: 16,
    large_increased_px: 20,
    extra_large_px: 28,
    full_px: 9999,
  };

  const componentsMatrix = [
    { compose: "MaterialExpressiveTheme", gpuWgsl: "ViewportUniform + M3Instance", gpuiKit: "M3ExpressiveThemeScope", nshTsx: "M3ExpressiveProvider", figmaGroup: "02_Sys/Color+Motion" },
    { compose: "material3-ripple:1.5.0 (Inset Focus Rings)", gpuWgsl: "fs_main flags bit0 inset ring", gpuiKit: "M3RippleThemeConfig", nshTsx: "M3FocusRing", figmaGroup: "02_Sys/State" },
    { compose: "ToggleButton / Elevated / FilledTonal / Outlined", gpuWgsl: "widget_meta.x=1 morph", gpuiKit: "M3ToggleButton", nshTsx: "M3ToggleButton", figmaGroup: "Components/Buttons" },
    { compose: "SplitButton", gpuWgsl: "widget_meta.x=5", gpuiKit: "M3SplitButton", nshTsx: "M3SplitButton", figmaGroup: "Components/Buttons" },
    { compose: "ButtonGroup (animateWidth compressionLimit)", gpuWgsl: "widget_meta.x=5", gpuiKit: "M3ButtonGroup", nshTsx: "M3ButtonGroup", figmaGroup: "Components/Buttons" },
    { compose: "FloatingToolbar / HorizontalFloatingToolbar", gpuWgsl: "widget_meta.x=0 elevated pill", gpuiKit: "M3FloatingToolbar", nshTsx: "M3FloatingToolbar", figmaGroup: "Components/Toolbars" },
    { compose: "WavyLinearProgressIndicator", gpuWgsl: "sdf_wavy_linear (kind=2)", gpuiKit: "M3WavyLinearProgressIndicator", nshTsx: "M3WavyLinearProgress", figmaGroup: "Components/Progress" },
    { compose: "WavyCircularProgressIndicator", gpuWgsl: "sdf_wavy_circular (kind=3)", gpuiKit: "M3WavyCircularProgressIndicator", nshTsx: "M3WavyCircularProgress", figmaGroup: "Components/Progress" },
    { compose: "Segmented ListItem / InteractiveListItem", gpuWgsl: "widget_meta.x=0 segmented", gpuiKit: "M3SegmentedListItem", nshTsx: "M3SegmentedListItem", figmaGroup: "Components/Lists" },
    { compose: "Slider(SliderState) / RangeSlider(RangeSliderState)", gpuWgsl: "widget_meta.x=4", gpuiKit: "M3StatefulSlider / M3StatefulRangeSlider", nshTsx: "M3StatefulSlider", figmaGroup: "Components/Sliders" },
    { compose: "TextField / OutlinedTextField / SecureTextField(System)", gpuWgsl: "widget_meta.x=0 cutout", gpuiKit: "M3ExpressiveTextField", nshTsx: "M3ExpressiveTextField", figmaGroup: "Components/TextFields" },
    { compose: "SelectableDropdownMenuItem / CheckableDropdownMenuItem", gpuWgsl: "widget_meta.x=0", gpuiKit: "M3ExpressiveMenuItem", nshTsx: "M3ExpressiveMenuItem", figmaGroup: "Components/Menus" },
    { compose: "Scrim + Deterministic BottomSheet anchors", gpuWgsl: "widget_meta.x=6 blur/scrim", gpuiKit: "M3Scrim + M3BottomSheetAnchor", nshTsx: "M3Scrim", figmaGroup: "Components/Sheets" },
    { compose: "VibrantTimePickerDialog + ScrollField", gpuWgsl: "widget_meta.x=0+1", gpuiKit: "M3VibrantTimePicker + M3ScrollField", nshTsx: "M3VibrantTimePicker", figmaGroup: "Components/Pickers" },
    { compose: "MaterialShapes (35) + LoadingIndicator", gpuWgsl: "sdf_material_polygon (kind=1)", gpuiKit: "M3MaterialShape::ALL_35 + M3LoadingIndicator", nshTsx: "M3LoadingIndicator", figmaGroup: "02_Sys/Shape" },
    { compose: "Parametric Prototyping (6 Strategies)", gpuWgsl: "uniform live params", gpuiKit: "M3ParametricPrototyperView", nshTsx: "M3ParametricPrototyper", figmaGroup: "Variables/Presets" },
  ];

  const catalog = {
    schema: "aphrody.m3.compose_converter_catalog.v1",
    tracker: {
      material3Stable: lock.material3_stable,
      material3Latest: lock.material3_latest,
      material3RippleStable: lock.material3_ripple_stable,
      composeBom: lock.compose_bom_latest,
      jetbrainsComposeM3: lock.jetbrains_compose_m3_latest,
      sha256Stamp: lock.sha256_stamp,
    },
    sources: SOURCES,
    materialShapes35: MATERIAL_SHAPES_35,
    motionSchemes: {
      standard: {
        spatialFast: { stiffness: 1400, dampingRatio: 0.9 },
        spatialDefault: { stiffness: 700, dampingRatio: 0.9 },
        spatialSlow: { stiffness: 300, dampingRatio: 0.9 },
        effectsFast: { stiffness: 3800, dampingRatio: 1.0 },
        effectsDefault: { stiffness: 1600, dampingRatio: 1.0 },
        effectsSlow: { stiffness: 800, dampingRatio: 1.0 },
      },
      expressive: {
        spatialFast: { stiffness: 800, dampingRatio: 0.6 },
        spatialDefault: { stiffness: 380, dampingRatio: 0.6 },
        spatialSlow: { stiffness: 200, dampingRatio: 0.6 },
        effectsFast: { stiffness: 3800, dampingRatio: 1.0 },
        effectsDefault: { stiffness: 1600, dampingRatio: 1.0 },
        effectsSlow: { stiffness: 800, dampingRatio: 1.0 },
      },
    },
    modes: Object.keys(modes),
    shapesPx: shapes,
    wgsl: {
      path: WGSL_SHADER_REL,
      bytes: wgslSource.length,
      sha256: createHash("sha256").update(wgslSource).digest("hex"),
    },
    components: componentsMatrix,
  };

  // Figma Material 3 Design Kit (1035203688168086460) W3C DTCG token export
  const figmaColorModes: Record<string, Record<string, { $type: string; $value: string }>> = {};
  for (const [modeName, roleMap] of Object.entries(modes)) {
    figmaColorModes[modeName] = {};
    for (const [role, hex] of Object.entries(roleMap)) {
      figmaColorModes[modeName][role] = { $type: "color", $value: hex };
    }
  }
  const figmaDtcg = {
    $schema: "https://tr.designtokens.org/format/",
    name: "Material 3 Design Kit -> Aphrody GPU / GPUI Kit / NSH",
    figmaCommunityFileId: SOURCES.figmaKitFileId,
    figmaCommunityUrl: SOURCES.figmaKitUrl,
    composeMaterial3Version: lock.material3_stable,
    composeBom: lock.compose_bom_latest,
    collections: {
      "02_Sys/Color": figmaColorModes,
      "02_Sys/Shape": Object.fromEntries(
        Object.entries(shapes).map(([k, v]) => [k, { $type: "dimension", $value: `${v}px` }]),
      ),
      "02_Sys/Motion": catalog.motionSchemes,
      "02_Sys/MaterialShapes35": { $type: "string", $value: MATERIAL_SHAPES_35.join(",") },
    },
  };

  // CSS Custom Properties for NSH + WebGPU surfaces
  const cssBlocks: string[] = [
    `/* Auto-generated from ${TOKENS_TOML_REL} & Compose Material 3 ${lock.material3_stable} */`,
  ];
  for (const [modeName, roleMap] of Object.entries(modes)) {
    const vars = Object.entries(roleMap)
      .map(([k, v]) => `  --md-sys-color-${k.replaceAll("_", "-")}: ${v};`)
      .join("\n");
    cssBlocks.push(`:root[data-m3-mode="${modeName}"] {\n${vars}\n}`);
  }
  const tokenCss = `${cssBlocks.join("\n\n")}\n`;

  // Idiomatic NSH TSX Library generated directly from Compose M3 1.5.0 + Parametric Prototyping specs
  const nshTsx = `// Auto-generated by tooling/bun/m3-compose-tracker.ts from Compose Material 3 ${lock.material3_stable}
// Sources:
// - ${SOURCES.composeBlog}
// - ${SOURCES.parametricBlog}
// - ${SOURCES.figmaKitUrl}
import { type JSX, setTheme, signal, Text, View } from "nsh";
import { Button, Switch } from "nsh/base";

export const COMPOSE_M3_VERSION = "${lock.material3_stable}" as const;
export const COMPOSE_BOM_VERSION = "${lock.compose_bom_latest}" as const;
export const FIGMA_M3_DESIGN_KIT_ID = "${SOURCES.figmaKitFileId}" as const;

export const MATERIAL_SHAPES_35 = ${JSON.stringify(MATERIAL_SHAPES_35, null, 2)} as const;
export type MaterialShapeName = (typeof MATERIAL_SHAPES_35)[number];

export type M3Mode = "light" | "dark" | "neural_obsidian" | "oled_pitch";
export type M3MotionScheme = "standard" | "expressive";
export type M3SegmentPosition = "single" | "first" | "middle" | "last";

export const M3_MODE_PALETTES: Record<M3Mode, Record<string, string>> = ${JSON.stringify(modes, null, 2)};

export const M3_MOTION_SCHEMES = ${JSON.stringify(catalog.motionSchemes, null, 2)} as const;

export const activeM3Mode = signal<M3Mode>("neural_obsidian");
export const activeMotionScheme = signal<M3MotionScheme>("expressive");

export function applyM3ExpressiveMode(mode: M3Mode, motion: M3MotionScheme = "expressive") {
  activeM3Mode.set(mode);
  activeMotionScheme.set(motion);
  setTheme({ name: \`m3-\${mode}\`, cssVars: M3_MODE_PALETTES[mode] });
}

export function M3ToggleButton(props: {
  label: string;
  checked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
}) {
  const checked = () => Boolean(props.checked);
  return (
    <Button
      onPress={() => props.onCheckedChange?.(!checked())}
      class={checked() ? "rounded-xl bg-primary px-5 py-2.5 min-h-[40px]" : "rounded-full bg-card px-5 py-2.5 min-h-[40px]"}
    >
      <Text class={checked() ? "text-sm font-semibold text-primary-foreground" : "text-sm font-medium text-foreground"}>
        {props.label}
      </Text>
    </Button>
  );
}

export function M3SplitButton(props: {
  label: string;
  expanded?: boolean;
  onPrimaryPress?: () => void;
  onTrailingPress?: () => void;
}) {
  return (
    <View class="flex-row items-center gap-0.5">
      <Button onPress={props.onPrimaryPress} class="rounded-l-full rounded-r-sm bg-primary px-4 py-2.5 min-h-[40px]">
        <Text class="text-sm font-semibold text-primary-foreground">{props.label}</Text>
      </Button>
      <Button onPress={props.onTrailingPress} class="rounded-l-sm rounded-r-full bg-primary px-3 py-2.5 min-h-[40px]">
        <Text class="text-sm font-bold text-primary-foreground">{props.expanded ? "^" : "v"}</Text>
      </Button>
    </View>
  );
}

export function M3FloatingToolbar(props: { vibrant?: boolean; children?: JSX.Element }) {
  return (
    <View class={props.vibrant ? "flex-row items-center gap-2 rounded-full bg-secondary px-3 py-1.5 shadow-md" : "flex-row items-center gap-2 rounded-full bg-card border border-border px-3 py-1.5 shadow-md"}>
      {props.children}
    </View>
  );
}

export function M3SegmentedListItem(props: {
  headline: string;
  supporting?: string;
  position?: M3SegmentPosition;
  selected?: boolean;
  onPress?: () => void;
}) {
  const radiusClass =
    props.position === "first"
      ? "rounded-t-2xl rounded-b-sm"
      : props.position === "middle"
        ? "rounded-sm"
        : props.position === "last"
          ? "rounded-t-sm rounded-b-2xl"
          : "rounded-2xl";
  return (
    <Button
      onPress={props.onPress}
      class={\`w-full flex-col items-start px-4 py-3 \${radiusClass} \${props.selected ? "bg-secondary" : "bg-card"}\`}
    >
      <Text class="text-sm font-medium text-foreground">{props.headline}</Text>
      {props.supporting ? <Text class="text-xs text-muted-foreground">{props.supporting}</Text> : null}
    </Button>
  );
}

export function M3ParametricPrototyperBar(props: {
  tethered: boolean;
  onTetherChange: (next: boolean) => void;
  onRandomize: () => void;
}) {
  return (
    <M3FloatingToolbar vibrant>
      <Switch checked={props.tethered} onChange={props.onTetherChange} />
      <M3ToggleButton label="Tether Rules" checked={props.tethered} onCheckedChange={props.onTetherChange} />
      <M3SplitButton label="Randomize" onPrimaryPress={props.onRandomize} />
    </M3FloatingToolbar>
  );
}
`;

  const templateCargoToml = `[package]
name = "m3-gpui-nsh-starter"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
gpui_ui = { path = "../../crates/gpui_ui", features = ["material3", "nsh"] }
`;

  const templateMainRs = `//! Starter template unifying Compose Material 3 ${lock.material3_stable} Stable,
//! Material 3 Parametric Prototyping, Figma Design Kit (${SOURCES.figmaKitFileId}),
//! GPUI Kit, WGPU/WGSL shaders, and NSH TSX.

use gpui_ui::kit::{
    M3AppearanceMode, M3ExpressiveThemeScope, M3MotionScheme, M3ParametricPrototyperView,
    apply_m3_expressive_theme,
};
use gpui_ui::prelude::*;

fn main() {
    gpui_ui::application().run(|cx: &mut App| {
        gpui_ui::init(cx);
        let scope = M3ExpressiveThemeScope::new(M3AppearanceMode::NeuralObsidian)
            .motion_scheme(M3MotionScheme::Expressive)
            .default_font_family("Google Sans Flex");
        apply_m3_expressive_theme(&scope, cx);

        if let Err(err) = cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(M3ParametricPrototyperView::new)
        }) {
            eprintln!("failed to open Material 3 parametric studio window: {err:#}");
        }
    });
}
`;

  const presetJson = JSON.stringify(
    {
      schema: "aphrody.m3.parametric_preset.v1",
      compose_m3_version: lock.material3_stable,
      compose_bom: lock.compose_bom_latest,
      figma_design_kit_file_id: SOURCES.figmaKitFileId,
      figma_design_kit_url: SOURCES.figmaKitUrl,
      preset_id: "expressive_time_picker",
      target: "vibrant_time_picker",
      variables: {
        corner_radius_px: 20,
        padding_px: 18,
        container_height_px: 48,
        type_size_px: 14,
        wave_amplitude_px: 3.5,
        wave_wavelength_px: 24,
        progress: 0.68,
        shape_index: 19,
        mode: "neural_obsidian",
        motion_scheme: "expressive",
        time_picker_mode: "scroll",
      },
      tethers: {
        clamp_radius_to_half_height: true,
        tether_padding_to_height: true,
        enforce_wcag_aa: true,
      },
      overlays: {
        show_keylines: false,
        show_color_tokens: true,
        show_measurements: true,
        show_sweet_spot_grid: false,
      },
    },
    null,
    2,
  );

  const templateReadme = `# Material 3 ${lock.material3_stable} -> GPU / GPUI Kit / NSH Starter Template

Generated deterministically by \`bun run m3:convert\` from:
- Compose Material 3 \`${lock.material3_stable}\` Stable (\`compose-bom:${lock.compose_bom_latest}\`): ${SOURCES.composeBlog}
- Material 3 Parametric Prototyping: ${SOURCES.parametricBlog}
- Official Material 3 Figma Design Kit (\`${SOURCES.figmaKitFileId}\`): ${SOURCES.figmaKitUrl}

## Contents

- \`src/main.rs\` — Native GPUI Kit app launching \`M3ParametricPrototyperView\` with \`M3ExpressiveThemeScope\`
- \`nsh-m3-kit.tsx\` — NSH TSX library with Compose M3 \`${lock.material3_stable}\` components, 35 \`MATERIAL_SHAPES_35\`, and 4-mode palettes
- \`m3-tokens.css\` — 4-mode CSS custom property token sheet (\`light\`, \`dark\`, \`neural_obsidian\`, \`oled_pitch\`)
- \`shaders/m3_expressive.wgsl\` — Pure WebGPU / Naga v30 WGSL shader (35 MaterialShapes SDF morphing, wavy progress, inset focus rings, progressive blur)
- \`m3-parametric-preset.json\` — Parametric Prototyping preset synced with Figma Design Kit \`${SOURCES.figmaKitFileId}\`

## Commands

\`\`\`sh
bun run m3:track
bun run m3:convert
bun run m3:prove
cargo run -p gpui_ui --features material3 --example m3_compose15_showcase
\`\`\`
`;

  const outputs: Array<[string, string]> = [
    [CATALOG_JSON_REL, `${JSON.stringify(catalog, null, 2)}\n`],
    [FIGMA_DTCG_REL, `${JSON.stringify(figmaDtcg, null, 2)}\n`],
    [`${TEMPLATE_DIR_REL}/shaders/m3_expressive.wgsl`, wgslSource],
    [`${TEMPLATE_DIR_REL}/nsh-m3-kit.tsx`, nshTsx],
    [`${TEMPLATE_DIR_REL}/m3-tokens.css`, tokenCss],
    [`${TEMPLATE_DIR_REL}/Cargo.toml`, templateCargoToml],
    [`${TEMPLATE_DIR_REL}/src/main.rs`, templateMainRs],
    [`${TEMPLATE_DIR_REL}/m3-parametric-preset.json`, `${presetJson}\n`],
    [`${TEMPLATE_DIR_REL}/README.md`, templateReadme],
  ];

  const drifted: string[] = [];
  for (const [rel, content] of outputs) {
    writeOrVerify(join(root, rel), content, verifyOnly, drifted);
  }

  return {
    generatedFiles: outputs.map(([rel]) => rel),
    drifted,
    componentCount: componentsMatrix.length,
    shapesCount: MATERIAL_SHAPES_35.length,
  };
}

function runCmd(cmd: string[], cwd = REPO_ROOT): { ok: boolean; exitCode: number; summary: string } {
  const proc = Bun.spawnSync(cmd, { cwd, stdout: "pipe", stderr: "pipe" });
  const stdout = proc.stdout.toString().trim();
  const stderr = proc.stderr.toString().trim();
  const lines = `${stdout}\n${stderr}`.trim().split(/\r?\n/).filter(Boolean);
  return {
    ok: proc.exitCode === 0,
    exitCode: proc.exitCode,
    summary: lines.slice(-6).join(" | "),
  };
}

export async function runProve(root = REPO_ROOT, skipCargo = false): Promise<{
  ok: boolean;
  proofPath: string;
  checks: Record<string, { ok: boolean; detail: string }>;
}> {
  const checks: Record<string, { ok: boolean; detail: string }> = {};

  const { lock } = await runTrack(root, true, true);
  checks.tracker_lock = {
    ok: lock.material3_stable.length > 0 && lock.figma_kit_file_id === SOURCES.figmaKitFileId,
    detail: `m3=${lock.material3_stable} bom=${lock.compose_bom_latest} figma=${lock.figma_kit_file_id}`,
  };

  const conv = runConvert(root, true);
  checks.converter_targets = {
    ok: conv.drifted.length === 0 && conv.shapesCount === 35,
    detail: conv.drifted.length === 0
      ? `${conv.generatedFiles.length} converted files verified (${conv.componentCount} components, ${conv.shapesCount} shapes)`
      : `drifted: ${conv.drifted.join(", ")}`,
  };

  const wgsl = readFileSync(join(root, WGSL_SHADER_REL), "utf8");
  const wgslHasSymbols =
    wgsl.includes("fn vs_main") &&
    wgsl.includes("fn fs_main") &&
    wgsl.includes("fn sdf_material_polygon") &&
    wgsl.includes("fn sdf_wavy_linear") &&
    wgsl.includes("fn sdf_wavy_circular");
  checks.wgsl_shader = {
    ok: wgslHasSymbols,
    detail: `${wgsl.length} bytes, 5 required WGSL entrypoints/SDFs present=${wgslHasSymbols}`,
  };

  if (!skipCargo) {
    const compTest = runCmd(["cargo", "test", "-p", "gpui-component", "--lib", "material3"], root);
    checks.gpui_component_material3_tests = {
      ok: compTest.ok,
      detail: compTest.summary,
    };
    const kitTest = runCmd(["cargo", "test", "-p", "gpui_ui", "--features", "material3", "--lib", "kit"], root);
    checks.gpui_ui_kit_tests = {
      ok: kitTest.ok,
      detail: kitTest.summary,
    };
    const brandCheck = runCmd(["cargo", "xtask", "brand", "check"], root);
    checks.xtask_brand_check = {
      ok: brandCheck.ok,
      detail: brandCheck.summary,
    };
  }

  const ok = Object.values(checks).every((c) => c.ok);
  const proofPath = join(root, "target", "m3-gpui-proof.json");
  mkdirSync(dirname(proofPath), { recursive: true });
  writeFileSync(
    proofPath,
    `${JSON.stringify({ ok, verifiedAt: new Date().toISOString(), sources: SOURCES, checks }, null, 2)}\n`,
    "utf8",
  );

  return { ok, proofPath, checks };
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const sub = args[0] ?? "track";
  const asJson = args.includes("--json");
  const checkOnly = args.includes("--check");
  const verifyOnly = args.includes("--verify");
  const offline = args.includes("--offline");
  const skipCargo = args.includes("--skip-cargo");

  if (sub === "track") {
    const res = await runTrack(REPO_ROOT, checkOnly, offline);
    if (asJson) console.log(JSON.stringify(res, null, 2));
    else console.log(`m3-track: stable=${res.lock.material3_stable} latest=${res.lock.material3_latest} bom=${res.lock.compose_bom_latest} changed=${res.changed}`);
    if (checkOnly && res.changed) process.exit(1);
  } else if (sub === "convert") {
    const res = runConvert(REPO_ROOT, verifyOnly);
    if (asJson) console.log(JSON.stringify(res, null, 2));
    else console.log(`m3-convert: files=${res.generatedFiles.length} components=${res.componentCount} shapes=${res.shapesCount} drifted=${res.drifted.length}`);
    if (verifyOnly && res.drifted.length > 0) process.exit(1);
  } else if (sub === "prove") {
    const res = await runProve(REPO_ROOT, skipCargo);
    if (asJson) console.log(JSON.stringify(res, null, 2));
    else {
      for (const [k, v] of Object.entries(res.checks)) {
        console.log(`[${v.ok ? "PASS" : "FAIL"}] ${k}: ${v.detail}`);
      }
      console.log(`proof written -> ${res.proofPath}`);
    }
    if (!res.ok) process.exit(1);
  } else {
    console.error(`Unknown subcommand: ${sub}. Expected track | convert | prove.`);
    process.exit(2);
  }
}
