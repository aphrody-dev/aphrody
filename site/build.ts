// Generates site/public/*.html from the workspace itself: every value on the page is read from a
// file, a manifest or git at build time. Each page is self-contained code with no runtime requests:
// a WebGPU WGSL wallpaper, Material Symbols inlined as SVG paths (names checked against Google's
// codepoint list), and an inline spring integrator driven by the Material 3 springs in fw-m3 tokens.
// Run from the aphrody repo root: bun site/build.ts
import { $ } from "bun";
import { readdirSync, statSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const fw = resolve(root, "../fw");
const ay = resolve(root, "../ay");
const tokensPath = join(fw, "crates/fw-m3/tokens/google_design_tokens.toml");

const dirs = (path: string) => (existsSync(path) ? readdirSync(path).filter((name) => statSync(join(path, name)).isDirectory()) : []);
const bytes = (path: string) => statSync(join(root, path)).size;
const git = async (repo: string, ...args: string[]) => (await $`git -C ${repo} ${args}`.quiet().text()).trim();

// Official Material Symbols names (google/material-design-icons, Rounded variable font).
const codepointsUrl =
  "https://raw.githubusercontent.com/google/material-design-icons/master/variablefont/MaterialSymbolsRounded%5BFILL,GRAD,opsz,wght%5D.codepoints";
const symbolNames = new Set(
  (await (await fetch(codepointsUrl)).text()).split("\n").map((line) => line.split(" ")[0]).filter(Boolean),
);
// Every icon used on any page is inlined once per page as SVG symbols (outline + filled).
const usedIcons = new Set<string>();
const icon = (name: string) => {
  if (!symbolNames.has(name)) throw new Error(`not a Material Symbol: ${name}`);
  usedIcons.add(name);
  return name;
};

const tokens = Bun.TOML.parse(await Bun.file(tokensPath).text()) as any;
const dark = tokens.sys.color.dark;
const springs = tokens.sys.motion.springs;
const shape = tokens.sys.shape;
const typescale = tokens.sys.typescale;
const zedManifest = await Bun.file(join(root, "crates/zed/Cargo.toml")).text();
const version = zedManifest.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? "unknown";
const wrangler = Bun.TOML.parse(await Bun.file(join(root, ".cloudflare/aphrody-worker/wrangler.toml")).text()) as any;

// Motion's spring takes absolute damping; M3 tokens give a damping ratio (mass 1).
const spring = (key: string) => {
  const { stiffness, damping_ratio } = springs[key];
  return { type: "spring", stiffness, damping: Math.round(damping_ratio * 2 * Math.sqrt(stiffness) * 10) / 10 };
};

const platformIcons: Record<string, string> = {
  windows: "desktop_windows", macos: "laptop_mac", linux: "computer", ios: "phone_iphone",
  android: "android", web: "language", tui: "terminal", apple: "devices", gpui: "deployed_code",
};
const platforms = dirs(join(root, "crates"))
  .filter((name) => name.startsWith("nsh-platform-"))
  .map((name) => name.replace("nsh-platform-", ""))
  .filter((name) => platformIcons[name]);

type Row = { icon: string; name: string; value: string; swatch?: string };
type Section = { icon: string; path: string; rows: Row[]; video?: string };

const indexSections: Section[] = [
  {
    icon: icon("deployed_code"),
    path: "crates/zed",
    rows: [
      { icon: icon("terminal"), name: "bin", value: "aphrody" },
      { icon: icon("tag"), name: "version", value: version },
      { icon: icon("commit"), name: "aphrody@HEAD", value: await git(root, "rev-parse", "--short", "HEAD") },
      { icon: icon("history"), name: "commits", value: await git(root, "rev-list", "--count", "HEAD") },
    ],
  },
  {
    icon: icon("devices"),
    path: "crates/nsh-platform-*",
    rows: platforms.map((name) => ({ icon: icon(platformIcons[name]), name, value: `nsh-platform-${name}` })),
  },
  {
    icon: icon("folder"),
    path: "crates/",
    rows: [
      { icon: icon("folder_open"), name: "aphrody/crates", value: String(dirs(join(root, "crates")).length) },
      { icon: icon("folder_open"), name: "fw/crates", value: String(dirs(join(fw, "crates")).length) },
      { icon: icon("folder_open"), name: "ay/crates", value: String(dirs(join(ay, "crates")).length) },
      { icon: icon("interests"), name: "assets/icons/*.svg", value: String(readdirSync(join(root, "assets/icons")).filter((f) => f.endsWith(".svg")).length) },
    ],
  },
  {
    icon: icon("palette"),
    path: "fw/crates/fw-m3/tokens/google_design_tokens.toml",
    rows: [
      ...["background", "surface", "surface_container", "on_surface", "primary", "outline_variant"].map((key) => ({
        icon: icon("format_color_fill"), name: `sys.color.dark.${key}`, value: dark[key], swatch: dark[key],
      })),
      ...["extra_small_px", "medium_px", "extra_large_px", "full_px"].map((key) => ({
        icon: icon("rounded_corner"), name: `sys.shape.${key}`, value: String(shape[key]),
      })),
      ...["display_large", "headline_large", "body_large"].map((key) => ({
        icon: icon("text_fields"), name: `sys.typescale.${key}`, value: `${typescale[key].size_sp}/${typescale[key].line_height_sp} w${typescale[key].weight}`,
      })),
      ...["spatial_default", "effects_default"].map((key) => ({
        icon: icon("animation"), name: `sys.motion.springs.${key}`, value: `k${springs[key].stiffness} ζ${springs[key].damping_ratio}`,
      })),
      { icon: icon("touch_app"), name: "sys.density.touch_target_min_px", value: String(tokens.sys.density.touch_target_min_px) },
      { icon: icon("mouse"), name: "sys.density.pointer_target_min_px", value: String(tokens.sys.density.pointer_target_min_px) },
    ],
  },
  {
    icon: icon("description"),
    path: "./",
    rows: ["product.toml", "DESIGN.md", "Cargo.toml", "assets/icons/aphrody_character.svg"].map((file) => ({
      icon: icon("draft"), name: file, value: `${bytes(file)} B`,
    })),
  },
  {
    icon: icon("cloud_upload"),
    path: ".cloudflare/aphrody-worker/wrangler.toml",
    rows: [
      ...wrangler.env.web.routes.map((route: { pattern: string }) => ({ icon: icon("public"), name: "env.web.routes", value: route.pattern })),
      { icon: icon("timer"), name: "env.web.limits.cpu_ms", value: String(wrangler.env.web.limits.cpu_ms) },
      { icon: icon("calendar_month"), name: "compatibility_date", value: wrangler.env.web.compatibility_date },
      { icon: icon("rocket_launch"), name: "package.json scripts.cf:deploy", value: "bun run cf:deploy" },
    ],
  },
];

const product = Bun.TOML.parse(await Bun.file(join(root, "product.toml")).text()) as any;
const bins = [...zedManifest.matchAll(/\[\[bin\]\]\s*\nname\s*=\s*"([^"]+)"/g)].map((match) => match[1]);
const themeFile = "assets/themes/aphrody/aphrody.json";
const themes = (JSON.parse(await Bun.file(join(root, themeFile)).text()).themes as any[]);
const hex = (value: string) => value.slice(0, 7).toUpperCase();
const list = (value: string | string[]) => (Array.isArray(value) ? value.join(" ") : value);

const studioSections: Section[] = [
  {
    icon: icon("deployed_code"),
    path: "crates/zed/Cargo.toml",
    rows: [
      ...bins.map((bin) => ({ icon: icon("terminal"), name: "[[bin]] name", value: bin })),
      { icon: icon("tag"), name: "version", value: version },
      { icon: icon("draft"), name: "crates/zed/Cargo.toml", value: `${bytes("crates/zed/Cargo.toml")} B` },
    ],
  },
  {
    icon: icon("folder_managed"),
    path: "product.toml [paths]",
    rows: Object.entries(product.paths).map(([key, value]) => ({
      icon: icon(key.startsWith("legacy") ? "history" : "folder"), name: key, value: String(value),
    })),
  },
  {
    icon: icon("link"),
    path: "product.toml [product]",
    rows: ["url_scheme", "cli_url_scheme", "dock_action_scheme", "legacy_url_schemes", "legacy_cli_url_schemes", "legacy_dock_action_schemes"].map((key) => ({
      icon: icon(key.startsWith("legacy") ? "history" : "link"), name: key, value: `${list(product.product[key])}://`,
    })),
  },
  {
    icon: icon("public"),
    path: "product.toml [urls]",
    rows: ["server", "extension_registry", "docs", "releases"].map((key) => ({
      icon: icon(key === "extension_registry" ? "extension" : "public"), name: key, value: String(product.urls[key]).replace(/^https:\/\//, ""),
    })),
  },
  {
    icon: icon("palette"),
    path: themeFile,
    rows: themes.flatMap((theme) => [
      { icon: icon(theme.appearance === "light" ? "light_mode" : "dark_mode"), name: theme.name, value: hex(theme.style.background), swatch: hex(theme.style.background) },
      { icon: icon("format_color_fill"), name: `${theme.name} text.accent`, value: hex(theme.style["text.accent"]), swatch: hex(theme.style["text.accent"]) },
    ]),
  },
];

// Desktop entry shipped with the binary (freedesktop Desktop Entry Specification).
const desktopEntryPath = "crates/zed/resources/zed.desktop.in";
const desktopEntry = Object.fromEntries(
  (await Bun.file(join(root, desktopEntryPath)).text())
    .split("\n[Desktop Action")[0]
    .split("\n")
    .filter((line) => /^[A-Za-z]+=/.test(line))
    .map((line) => [line.slice(0, line.indexOf("=")), line.slice(line.indexOf("=") + 1)]),
);
studioSections.push({
  icon: icon("apps"),
  path: desktopEntryPath,
  rows: ["Type", "Exec", "Categories", "MimeType", "Actions", "Keywords"].map((key) => ({
    icon: icon(key === "MimeType" ? "description" : key === "Categories" ? "category" : "settings_applications"),
    name: key,
    value: String(desktopEntry[key] ?? "").replace(/;$/, "").replaceAll(";", " "),
  })),
});

// Media is data too: site/media.json lists clips; each is probed with ffprobe for its real metadata.
// Pages carry the probe values only; no clip is embedded or fetched by the page.
type Media = { url: string; world?: string };
const media = JSON.parse(await Bun.file(join(root, "site/media.json")).text()) as Media[];
const probed = await Promise.all(
  media.map(async (item) => {
    const probe = JSON.parse(await $`ffprobe -v error -print_format json -show_streams -show_format ${item.url}`.quiet().text());
    const video = probe.streams.find((stream: any) => stream.codec_type === "video");
    const [num, den] = String(video.avg_frame_rate).split("/").map(Number);
    return {
      ...item,
      path: new URL(item.url).pathname.split("/").slice(-2).join("/"),
      codec: `${video.codec_name} ${video.profile}`,
      size: `${video.width}x${video.height}`,
      fps: (num / den).toFixed(3),
      duration: `${Number(probe.format.duration).toFixed(1)} s`,
      bytes: `${probe.format.size} B`,
      pix: video.pix_fmt,
      audio: probe.streams.some((stream: any) => stream.codec_type === "audio") ? "yes" : "none",
    };
  }),
);
indexSections.unshift(
  ...probed.map((clip) => ({
    icon: icon("movie"),
    path: clip.path,
    rows: [
      { icon: icon("videocam"), name: "codec", value: clip.codec },
      { icon: icon("aspect_ratio"), name: "size", value: clip.size },
      { icon: icon("speed"), name: "fps", value: clip.fps },
      { icon: icon("timer"), name: "duration", value: clip.duration },
      { icon: icon("database"), name: "bytes", value: clip.bytes },
      { icon: icon("palette"), name: "pix_fmt", value: clip.pix },
      { icon: icon("volume_off"), name: "audio", value: clip.audio },
    ],
  })),
);

// Framework workspace: GPUI, GPUI Kit and NSH as they sit in the file tree.
const workspaceManifest = await Bun.file(join(root, "Cargo.toml")).text();
const workspaceVersion = workspaceManifest.match(/\[workspace\.package\][^[]*?^version\s*=\s*"([^"]+)"/ms)?.[1];
const crateVersion = async (crate: string) => {
  const manifest = await Bun.file(join(root, "crates", crate, "Cargo.toml")).text();
  const own = manifest.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  return own ?? (/^version\.workspace\s*=\s*true/m.test(manifest) ? (workspaceVersion ?? "workspace") : "unversioned");
};
const countFiles = (path: string): number =>
  existsSync(path)
    ? readdirSync(path, { withFileTypes: true }).reduce((n, entry) => n + (entry.isDirectory() ? countFiles(join(path, entry.name)) : 1), 0)
    : 0;
const gpuiCrates = dirs(join(root, "crates")).filter((name) => /^gpui([_-]|$)/.test(name)).sort();
const nshCrates = dirs(join(root, "crates")).filter((name) => name.startsWith("nsh-")).sort();
const kitManifest = Bun.TOML.parse(await Bun.file(join(root, "crates/gpui-kit/Cargo.toml")).text()) as any;
const templateDirs = dirs(join(root, "templates")).sort();
const frameworkSections: Section[] = [
  {
    icon: icon("deployed_code"),
    path: "crates/gpui*",
    rows: await Promise.all(gpuiCrates.map(async (name) => ({ icon: icon("package_2"), name, value: await crateVersion(name) }))),
  },
  {
    icon: icon("terminal"),
    path: `crates/nsh-* (${nshCrates.length})`,
    rows: nshCrates.map((name) => ({ icon: icon("description"), name, value: `${countFiles(join(root, "crates", name, "src"))} files` })),
  },
  {
    icon: icon("inventory_2"),
    path: "crates/gpui-kit/Cargo.toml",
    rows: [
      { icon: icon("tag"), name: "version", value: kitManifest.package.version },
      { icon: icon("toggle_on"), name: "features.default", value: list(kitManifest.features.default) },
      ...Object.keys(kitManifest.dependencies ?? {}).map((name) => ({ icon: icon("link"), name: `dependencies.${name}`, value: "workspace" })),
    ],
  },
  {
    icon: icon("folder_copy"),
    path: "templates/",
    rows: templateDirs.map((name) => ({ icon: icon("folder"), name, value: `${countFiles(join(root, "templates", name))} files` })),
  },
];

// Each page is a workspace; each section is a toplevel window inside it.
const pages = [
  { slug: "index", file: "index.html", icon: icon("folder_open"), label: "aphrody", sections: indexSections, thesis: true },
  { slug: "studio", file: "studio.html", icon: icon("code_blocks"), label: "studio", sections: studioSections, thesis: false },
  { slug: "framework", file: "framework.html", icon: icon("deployed_code"), label: "framework", sections: frameworkSections, thesis: false },
];

const status = [
  { icon: icon("download"), label: "Download", href: "https://aphrody.com/download" },
  { icon: icon("menu_book"), label: "Docs", href: "https://docs.aphrody.com" },
  { icon: icon("code"), label: "GitHub", href: "https://github.com/aphrody-labs/aphrody" },
  { icon: icon("monitor_heart"), label: "Status", href: "https://status.aphrody.com" },
];
for (const name of ["remove", "add", "open_in_full", "close_fullscreen"]) icon(name);
const dockIcon = icon("terminal");

// Material Symbols Rounded outline and filled paths (viewBox 0 -960 960 960), fetched once at build.
const symbolSvg = async (name: string, fill: boolean) => {
  const url = `https://raw.githubusercontent.com/google/material-design-icons/master/symbols/web/${name}/materialsymbolsrounded/${name}${fill ? "_fill1" : ""}_24px.svg`;
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  const paths = [...(await response.text()).matchAll(/<path d="([^"]+)"/g)].map((match) => match[1]);
  if (paths.length === 0) throw new Error(`${url}: no path`);
  return paths.map((d) => `<path d="${d}"/>`).join("");
};
const sprite = (
  await Promise.all(
    [...usedIcons].sort().map(async (name) =>
      `<symbol id="o-${name}" viewBox="0 -960 960 960">${await symbolSvg(name, false)}</symbol><symbol id="f-${name}" viewBox="0 -960 960 960">${await symbolSvg(name, true)}</symbol>`),
  )
).join("");

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const sym = (name: string) => `<svg class="sym" aria-hidden="true" data-icon="${name}"><use href="#o-${name}"/><use class="f" href="#f-${name}"/></svg>`;
const head = await git(root, "rev-parse", "--short", "HEAD");
const sizes = tokens.sys.window_size_classes;
const springTable = Object.fromEntries(
  ["spatial_default", "spatial_fast", "spatial_slow", "effects_default"].map((key) => [key, { k: springs[key].stiffness, z: springs[key].damping_ratio }]),
);
const href = (page: (typeof pages)[number]) => `/${page.slug === "index" ? "" : page.file}`;
// Workspace spheres: colour pair, diameter (CSS px), depth blur (CSS px), position (% of viewport).
const worlds = pages.map((page, index) => ({
  page,
  c1: [dark.primary, "#7FCFFF", "#C4C7C5"][index % 3],
  c2: ["#5559CF", "#131722", "#282A2C"][index % 3],
  size: [260, 180, 120][index % 3],
  blur: [0, 6, 12][index % 3],
  left: ["62%", "8%", "78%"][index % 3],
  top: ["14%", "58%", "62%"][index % 3],
}));

const renderWindow = (section: Section, index: number) => `
      <article class="window" data-window="${index}" aria-label="${esc(section.path)}">
        <header class="titlebar">
          ${sym(section.icon)}<h2 class="title">${esc(section.path)}</h2>
          <button class="ctl" data-action="minimize" aria-label="Minimize" aria-expanded="true">${sym("remove")}</button>
          <button class="ctl" data-action="maximize" aria-label="Maximize" aria-pressed="false">${sym("open_in_full")}</button>
        </header>
        <ul class="rows">${section.rows
          .map((row) => `
          <li class="row" tabindex="0">${sym(row.icon)}<span class="name">${esc(row.name)}</span>${row.swatch ? `<span class="swatch" style="background:${row.swatch}"></span>` : ""}<span class="value">${esc(row.value)}</span></li>`)
          .join("")}
        </ul>
      </article>`;

// WGSL: full-screen triangle; Genie gradient (290deg #131722 6% -> black 70%) plus a #5559CF glow,
// then lit spheres with a depth-blur edge. Colours are written as sRGB into a non-sRGB swapchain,
// which matches the CSS fallback exactly.
const wgsl = /* wgsl */ `
struct World { center: vec2f, radius: f32, blur: f32, c1: vec4f, c2: vec4f };
struct Scene { size: vec2f, count: f32, pad: f32, worlds: array<World, 4> };
@group(0) @binding(0) var<uniform> scene: Scene;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) frag: vec4f) -> @location(0) vec4f {
  let p = frag.xy;
  let a = radians(290.0);
  let dir = vec2f(sin(a), -cos(a));
  let len = abs(scene.size.x * sin(a)) + abs(scene.size.y * cos(a));
  let t = dot(p - scene.size * 0.5, dir) / len + 0.5;
  var col = mix(vec3f(0.0745, 0.0902, 0.1333), vec3f(0.0), smoothstep(0.06, 0.70, t));
  let glow = length((p - scene.size * vec2f(0.85, 1.10)) / (scene.size * vec2f(1.2, 0.8)));
  col = mix(col, vec3f(0.3333, 0.3490, 0.8118), 0.38 * (1.0 - smoothstep(0.0, 0.6, glow)));
  for (var i = 0u; i < u32(scene.count); i++) {
    let w = scene.worlds[i];
    let q = (p - w.center) / w.radius;
    let d = length(q);
    let edge = (w.blur + 1.0) / w.radius;
    let cover = 1.0 - smoothstep(1.0 - edge, 1.0 + edge, d);
    if (cover <= 0.0) { continue; }
    var s = mix(w.c1.rgb, w.c2.rgb, smoothstep(0.0, 0.7, length(q - vec2f(0.0, 0.2))));
    s += vec3f(0.55) * (1.0 - smoothstep(0.0, 0.44, length(q - vec2f(-0.36, -0.44))));
    s *= 1.0 - 0.45 * smoothstep(0.2, 1.0, dot(q, vec2f(0.55, 0.83)));
    col = mix(col, s, cover);
  }
  return vec4f(col, 1.0);
}`;

const renderPage = (current: (typeof pages)[number]) => `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="icon" href="data:,">
<title>${current.slug === "index" ? "Aphrody" : `Aphrody ${current.label}`}</title>
<style>
/* Tokens: fw-m3 (surfaces, accent, shape, breakpoints); wallpaper: labs.google/projectgenie (black page, 290deg #131722 -> #5559CF). */
:root{--black:#000;--genie-a:#131722;--genie-b:#5559CF;--surface:${dark.surface};--container:${dark.surface_container};--high:${dark.surface_container_high};
--text:${dark.on_surface};--muted:${dark.on_surface_variant};--line:${dark.outline_variant};--accent:${dark.primary};
--r-xl:${shape.extra_large_px}px;--r-l:${shape.large_px}px;--r-m:${shape.medium_px}px;--margin:16px;--gap:16px;--panel:44px;
--sans:"Google Sans Flex","Google Sans",system-ui,sans-serif;--mono:"Google Sans Code",ui-monospace,"Cascadia Mono",monospace}
@media (min-width:${sizes.medium_min_px}px){:root{--margin:24px;--gap:24px}}
*{box-sizing:border-box}
html,body{margin:0;min-height:100%;background:var(--black);color:var(--text)}
body{font:400 15px/22px var(--sans);overflow-x:hidden}
button{font:inherit;color:inherit;background:none;border:0;padding:0;cursor:pointer}
a{color:inherit;text-decoration:none}
.sym{width:20px;height:20px;flex:none;fill:var(--muted)}
.sym .f{opacity:0}
:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
/* wl_output background surface: WGSL on a canvas; CSS draws the same scene when WebGPU is missing */
.wallpaper{position:fixed;inset:0;z-index:0;overflow:hidden;
 background:radial-gradient(120% 80% at 85% 110%,color-mix(in srgb,var(--genie-b) 38%,transparent),transparent 60%),linear-gradient(290deg,var(--genie-a) 6%,var(--black) 70%)}
.wallpaper canvas{position:absolute;inset:0;width:100%;height:100%;display:none}
.wallpaper.gpu canvas{display:block}
.world{position:absolute;border-radius:50%;cursor:pointer;will-change:transform;
 background:radial-gradient(circle at 32% 28%,rgba(255,255,255,.55),transparent 22%),radial-gradient(circle at 50% 60%,var(--c1),var(--c2) 70%);
 box-shadow:inset 0 0 40px rgba(255,255,255,.18),inset -20px -30px 60px rgba(0,0,0,.45)}
.wallpaper.gpu .world{background:none;box-shadow:none;filter:none!important}
.world .sym{position:absolute;inset:0;margin:auto;width:40px;height:40px;fill:rgba(255,255,255,.85)}
/* zwlr_layer_shell_v1 top panel */
.panel{position:sticky;top:0;z-index:20;height:var(--panel);display:flex;align-items:center;gap:4px;padding:0 var(--margin);
 background:color-mix(in srgb,var(--surface) 72%,transparent);backdrop-filter:blur(20px);border-bottom:1px solid color-mix(in srgb,var(--line) 60%,transparent)}
.ws{display:inline-flex;align-items:center;gap:6px;height:32px;padding:0 12px;border-radius:999px;color:var(--muted)}
.ws .n{font:500 12px/1 var(--mono)}
.ws[aria-current]{background:var(--high);color:var(--text)}
.ws[aria-current] .sym{fill:var(--accent)}
.clock{margin:0 auto;font-variant-numeric:tabular-nums;font-weight:500}
.tray{display:flex;gap:2px}
.tray a{display:grid;place-items:center;width:48px;height:40px;border-radius:999px}
@media (max-width:${sizes.compact_max_px}px){.ws .label{display:none}.ws{padding:0 10px}.tray a{width:40px}}
/* workspace and xdg_toplevel windows, tiled by Material 3 breakpoints */
.workspace{position:relative;z-index:1;padding:var(--margin) var(--margin) calc(var(--margin) + 88px);pointer-events:none}
.workspace>*{pointer-events:auto}
.thesis{margin:48px 0 40px;font-size:clamp(40px,7vw,${typescale.display_large.size_sp * 1.6}px);line-height:1.04;letter-spacing:-.02em;font-weight:300;will-change:transform}
.thesis span{display:block}
.tiles{display:grid;grid-template-columns:1fr;gap:var(--gap);align-items:start}
@media (min-width:${sizes.expanded_min_px}px){.tiles{grid-template-columns:repeat(2,minmax(0,1fr))}}
@media (min-width:${sizes.extra_large_min_px}px){.tiles{grid-template-columns:repeat(3,minmax(0,1fr))}}
.window{background:color-mix(in srgb,var(--surface) 92%,transparent);border:1px solid color-mix(in srgb,var(--line) 70%,transparent);
 border-radius:var(--r-xl);overflow:hidden;transform-origin:50% 0;will-change:transform}
.window.max{grid-column:1/-1}
.titlebar{display:flex;align-items:center;gap:10px;height:48px;padding:0 6px 0 18px;background:color-mix(in srgb,var(--container) 80%,transparent)}
.titlebar>.sym{fill:var(--accent)}
.title{flex:1;min-width:0;margin:0;font:500 13px/20px var(--mono);color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.ctl{display:grid;place-items:center;width:48px;height:48px;border-radius:999px}
.rows{list-style:none;margin:0;padding:4px 18px 10px}
.rows[hidden]{display:none}
.row{display:flex;align-items:center;gap:12px;min-height:44px;border-top:1px solid color-mix(in srgb,var(--line) 55%,transparent)}
.row:first-child{border-top:0}
.row .name{flex:1;min-width:0;font:400 12.5px/18px var(--mono);color:var(--muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.row .value{font-size:17px;font-variant-numeric:tabular-nums;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:60%}
.swatch{width:16px;height:16px;border-radius:5px;border:1px solid var(--line)}
/* bottom layer-shell dock: one launcher per desktop entry */
.dock{position:fixed;left:50%;bottom:var(--margin);z-index:20;display:flex;gap:6px;padding:8px;transform:translateX(-50%);
 border-radius:999px;background:color-mix(in srgb,var(--surface) 72%,transparent);backdrop-filter:blur(20px);border:1px solid color-mix(in srgb,var(--line) 60%,transparent)}
.launcher{display:grid;place-items:center;width:52px;height:52px;border-radius:999px;background:var(--container);will-change:transform}
.launcher[aria-current]{background:var(--accent)}
.launcher[aria-current] .sym{fill:var(--black)}
.launcher .sym{width:24px;height:24px}
/* org.freedesktop.Notifications bubble */
.notification{position:fixed;top:calc(var(--panel) + 12px);right:var(--margin);z-index:30;display:flex;align-items:center;gap:12px;padding:12px 16px 12px 14px;
 border-radius:var(--r-l);background:color-mix(in srgb,var(--high) 96%,transparent);border:1px solid var(--line);opacity:0}
.notification .sym{fill:var(--accent);width:24px;height:24px}
.notification b{display:block;font-weight:500}
.notification code{font:400 12.5px/18px var(--mono);color:var(--muted)}
</style>
</head>
<body>
<svg width="0" height="0" style="position:absolute" aria-hidden="true">${sprite}</svg>
<div class="wallpaper" aria-hidden="true"><canvas></canvas>${worlds
  .map((world, index) => `<a class="world" href="${href(world.page)}" tabindex="-1" data-depth="${index + 1}" data-blur="${world.blur}" data-c1="${world.c1}" data-c2="${world.c2}" aria-label="${world.page.label}" style="--c1:${world.c1};--c2:${world.c2};width:${world.size}px;height:${world.size}px;left:${world.left};top:${world.top};filter:blur(${world.blur}px)">${sym(world.page.icon)}</a>`)
  .join("")}</div>
<nav class="panel" aria-label="Workspaces">
  ${pages.map((page, index) => `<a class="ws" href="${href(page)}"${page.slug === current.slug ? ' aria-current="page"' : ""}>${sym(page.icon)}<span class="n">${index + 1}</span><span class="label">${page.label}</span></a>`).join("")}
  <time class="clock" data-clock></time>
  <div class="tray">${status.map((item) => `<a href="${item.href}" aria-label="${item.label}">${sym(item.icon)}</a>`).join("")}</div>
</nav>
<main class="workspace" data-workspace="${current.slug}">
${current.thesis ? `  <h1 class="thesis"><span>Everything is a file.</span><span>Code is the design material.</span></h1>` : ""}
  <div class="tiles">${current.sections.map(renderWindow).join("")}
  </div>
</main>
<nav class="dock" aria-label="Applications">
  <a class="launcher" href="https://aphrody.com/download" aria-label="${esc(desktopEntry.Exec?.split(" ")[0] ?? "aphrody")}">${sym(dockIcon)}</a>
  ${pages.map((page) => `<a class="launcher" href="${href(page)}" aria-label="${page.label}"${page.slug === current.slug ? ' aria-current="page"' : ""}>${sym(page.icon)}</a>`).join("")}
</nav>
<aside class="notification" role="status" data-expire="5000">${sym(icon("deployed_code"))}<div><b>aphrody ${esc(version)}</b><code>${esc(head)}</code></div></aside>
<script type="module">
const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
const SPRINGS = ${JSON.stringify(springTable)};

// Spring integrator: one requestAnimationFrame loop, time-based (same motion at 60/120/144 Hz),
// semi-implicit Euler in <=4 ms substeps, stops when every spring has settled.
const live = new Set();
let frame = 0, last = 0;
const loop = (now) => {
  const dt = Math.min((now - (last || now)) / 1000, 0.05);
  last = now;
  for (const motion of live) {
    let settled = true;
    for (const key in motion.target) {
      const spring = SPRINGS[motion.spring[key] ?? motion.spring.all];
      const omega = Math.sqrt(spring.k), damping = 2 * spring.z * omega;
      let x = motion.value[key], v = motion.velocity[key] ?? 0;
      for (let left = dt; left > 0; left -= 0.004) {
        const h = Math.min(left, 0.004);
        v += (-spring.k * (x - motion.target[key]) - damping * v) * h;
        x += v * h;
      }
      if (Math.abs(x - motion.target[key]) < 1e-3 && Math.abs(v) < 1e-2) { x = motion.target[key]; v = 0; } else settled = false;
      motion.value[key] = x; motion.velocity[key] = v;
    }
    motion.apply(motion.value);
    if (settled) { live.delete(motion); motion.done?.(); }
  }
  frame = live.size ? requestAnimationFrame(loop) : (last = 0);
};
const state = new WeakMap();
const animate = (motion, to, spring = "spatial_default", done) => {
  Object.assign(motion.target, to);
  motion.spring = typeof spring === "string" ? { all: spring } : spring;
  motion.done = done;
  if (reduce) { Object.assign(motion.value, motion.target); motion.apply(motion.value); done?.(); return; }
  live.add(motion);
  if (!frame) frame = requestAnimationFrame(loop);
};
// Element motion: transform and opacity only.
const el = (node) => {
  let motion = state.get(node);
  if (!motion) {
    const value = { x: 0, y: 0, s: 1, sx: 1, o: Number(getComputedStyle(node).opacity) };
    motion = { value, velocity: {}, target: { ...value }, spring: { all: "spatial_default" },
      apply: (v) => { node.style.transform = \`translate(\${v.x}px,\${v.y}px) scale(\${v.s * v.sx},\${v.s})\`; node.style.opacity = v.o; } };
    state.set(node, motion);
  }
  return motion;
};
const move = (node, to, spring, done) => animate(el(node), to, spring, done);
const jump = (node, from) => { const m = el(node); Object.assign(m.value, from); m.apply(m.value); };

// Icons: outline at rest, filled while hovered or active (opacity of the filled <use>).
const fill = (node, on) => node.querySelectorAll(".sym .f").forEach((use) => move(use, { o: on ? 1 : 0 }, "effects_default"));
const setIcon = (button, name) => button.querySelectorAll("use").forEach((use) => use.setAttribute("href", (use.classList.contains("f") ? "#f-" : "#o-") + name));

const clock = document.querySelector("[data-clock]");
const tick = () => (clock.textContent = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }));
tick(); setInterval(tick, 15000);

// FLIP: record rects, change layout, then spring every window from its old rect.
const windows = [...document.querySelectorAll(".window")];
const flip = (change) => {
  const before = windows.map((w) => w.getBoundingClientRect());
  change();
  windows.forEach((w, i) => {
    const after = w.getBoundingClientRect();
    if (!after.width) return;
    jump(w, { x: before[i].left - after.left, y: before[i].top - after.top, sx: before[i].width / after.width });
    move(w, { x: 0, y: 0, sx: 1 });
  });
};
windows.forEach((win) => {
  const rows = win.querySelector(".rows");
  win.querySelector('[data-action="minimize"]').addEventListener("click", (event) => {
    const button = event.currentTarget;
    flip(() => { rows.hidden = !rows.hidden; });
    button.setAttribute("aria-expanded", String(!rows.hidden));
    setIcon(button, rows.hidden ? "add" : "remove");
  });
  win.querySelector('[data-action="maximize"]').addEventListener("click", (event) => {
    const button = event.currentTarget;
    flip(() => win.classList.toggle("max"));
    const max = win.classList.contains("max");
    button.setAttribute("aria-pressed", String(max));
    setIcon(button, max ? "close_fullscreen" : "open_in_full");
  });
});

// Map windows and the thesis onto the workspace like a compositor opening toplevels.
document.querySelectorAll(".thesis span").forEach((line, i) => { jump(line, { o: 0, y: 16 }); setTimeout(() => move(line, { o: 1, y: 0 }), i * 120); });
windows.forEach((w, i) => { jump(w, { o: 0, s: 0.94, y: 24 }); setTimeout(() => move(w, { o: 1, s: 1, y: 0 }), i * 50); });

const interactive = ".row, .ws, .tray a, .launcher, .ctl, .world";
document.addEventListener("pointerover", (event) => {
  const target = event.target.closest(interactive);
  if (!target || target.contains(event.relatedTarget)) return;
  fill(target, true);
  if (target.classList.contains("row")) move(target, { x: 4 }, "spatial_fast");
  if (target.classList.contains("launcher")) move(target, { s: 1.08 }, "spatial_fast");
  if (target.classList.contains("world")) worldScale(target, 1.08);
});
document.addEventListener("pointerout", (event) => {
  const target = event.target.closest(interactive);
  if (!target || target.contains(event.relatedTarget)) return;
  fill(target, false);
  if (target.classList.contains("world")) worldScale(target, 1);
  else move(target, { x: 0, s: 1 }, "spatial_fast");
});
document.addEventListener("pointerdown", (event) => { const t = event.target.closest(interactive); if (t && !t.classList.contains("world")) move(t, { s: 0.94 }, "spatial_fast"); });
document.addEventListener("pointerup", (event) => { const t = event.target.closest(interactive); if (t && !t.classList.contains("world")) move(t, { s: 1 }, "spatial_fast"); });
document.addEventListener("focusin", (event) => { const t = event.target.closest(interactive); if (t) fill(t, true); });
document.addEventListener("focusout", (event) => { const t = event.target.closest(interactive); if (t) fill(t, false); });

// Wallpaper: worlds are DOM anchors for hit-testing; WGSL draws them. Parallax on the slow spring.
const wall = document.querySelector(".wallpaper");
const canvas = wall.querySelector("canvas");
const worldNodes = [...wall.querySelectorAll(".world")];
const worldMotion = worldNodes.map((node) => {
  const motion = { value: { x: 0, y: 0, s: 1 }, velocity: {}, target: { x: 0, y: 0, s: 1 }, spring: { all: "spatial_slow" },
    apply: (v) => { node.style.transform = \`translate(\${v.x}px,\${v.y}px) scale(\${v.s})\`; draw(); } };
  return motion;
});
const worldScale = (node, s) => animate(worldMotion[worldNodes.indexOf(node)], { s }, "spatial_fast");
let draw = () => {};
if (!reduce) addEventListener("pointermove", (event) => {
  const dx = event.clientX / innerWidth - 0.5, dy = event.clientY / innerHeight - 0.5;
  worldMotion.forEach((motion, i) => animate(motion, { x: (dx * 40) / (i + 1), y: (dy * 30) / (i + 1) }, { x: "spatial_slow", y: "spatial_slow", s: "spatial_fast" }));
}, { passive: true });

const hex = (value) => [1, 3, 5].map((i) => parseInt(value.slice(i, i + 2), 16) / 255);
(async () => {
  const adapter = await navigator.gpu?.requestAdapter({ powerPreference: "high-performance" });
  const device = await adapter?.requestDevice();
  if (!device) return;
  const context = canvas.getContext("webgpu");
  const format = navigator.gpu.getPreferredCanvasFormat();
  const module = device.createShaderModule({ code: ${JSON.stringify(wgsl)} });
  const pipeline = device.createRenderPipeline({ layout: "auto", vertex: { module, entryPoint: "vs" }, fragment: { module, entryPoint: "fs", targets: [{ format }] }, primitive: { topology: "triangle-list" } });
  const uniform = device.createBuffer({ size: 16 + 4 * 48, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
  const bind = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: [{ binding: 0, resource: { buffer: uniform } }] });
  const data = new Float32Array(4 + 4 * 12);
  let rects = [];
  let pending = false;
  // Native resolution, always: backing store = CSS size x devicePixelRatio, no dynamic scaling.
  context.configure({ device, format, alphaMode: "opaque" });
  const max = device.limits.maxTextureDimension2D;
  const resize = ([entry]) => {
    // device-pixel-content-box is the exact device-pixel size (zoom and monitor changes included);
    // contentBox x devicePixelRatio is the fallback where it is not reported.
    const width = entry.devicePixelContentBoxSize?.[0].inlineSize ?? Math.round(entry.contentBoxSize[0].inlineSize * devicePixelRatio);
    const height = entry.devicePixelContentBoxSize?.[0].blockSize ?? Math.round(entry.contentBoxSize[0].blockSize * devicePixelRatio);
    canvas.width = Math.max(1, Math.min(width, max));
    canvas.height = Math.max(1, Math.min(height, max));
    worldNodes.forEach((node) => (node.style.transform = ""));
    rects = worldNodes.map((node) => node.getBoundingClientRect());
    worldMotion.forEach((motion) => motion.apply(motion.value));
    render();
  };
  const render = () => {
    pending = false;
    const dpr = canvas.width / canvas.clientWidth;
    data.set([canvas.width, canvas.height, worldNodes.length, 0]);
    worldNodes.forEach((node, i) => {
      const r = rects[i], v = worldMotion[i].value, o = 4 + i * 12;
      data.set([(r.left + r.width / 2 + v.x) * dpr, (r.top + r.height / 2 + v.y) * dpr, (r.width / 2) * v.s * dpr, Number(node.dataset.blur) * dpr, ...hex(node.dataset.c1), 1, ...hex(node.dataset.c2), 1], o);
    });
    device.queue.writeBuffer(uniform, 0, data);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({ colorAttachments: [{ view: context.getCurrentTexture().createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 1 } }] });
    pass.setPipeline(pipeline); pass.setBindGroup(0, bind); pass.draw(3); pass.end();
    device.queue.submit([encoder.finish()]);
  };
  // Draws are coalesced into the spring loop's frame; idle means no GPU work at all.
  draw = () => { if (!pending) { pending = true; queueMicrotask(render); } };
  wall.classList.add("gpu");
  const observer = new ResizeObserver(resize);
  try { observer.observe(canvas, { box: "device-pixel-content-box" }); } catch { observer.observe(canvas); }
})();

// Notification: shown once per session, expires after the server-style timeout.
const note = document.querySelector(".notification");
if (!sessionStorage.getItem("aphrody-notified")) {
  sessionStorage.setItem("aphrody-notified", "1");
  jump(note, { o: 0, x: 40 });
  move(note, { o: 1, x: 0 });
  setTimeout(() => move(note, { o: 0, x: 40 }, "spatial_default", () => note.remove()), Number(note.dataset.expire));
} else note.remove();
</script>
</body>
</html>
`;

for (const page of pages) await Bun.write(join(root, "site/public", page.file), renderPage(page));
console.log(JSON.stringify({ pages: pages.map((page) => ({ file: page.file, windows: page.sections.length, rows: page.sections.reduce((n, s) => n + s.rows.length, 0), bytes: Buffer.byteLength(renderPage(page)) })), icons: usedIcons.size, symbols: symbolNames.size }));
