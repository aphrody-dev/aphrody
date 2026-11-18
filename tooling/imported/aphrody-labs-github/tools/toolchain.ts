#!/usr/bin/env bun
// One toolchain for every repository (templates/toolchain.toml): the Rust channel, the cargo tools, the Bun install policy and the
// Cargo profiles, in the shape of Zed's configuration.
//   bun tools/toolchain.ts update            move the Rust channel and the cargo tools to their latest stable release (live sources)
//   bun tools/toolchain.ts check [repo ...]  report every checkout that drifted from the policy (exit 1 when there is drift)
//   bun tools/toolchain.ts apply [repo ...]  write rust-toolchain.toml, the dev profiles of a Cargo workspace and the [install] section of bunfig.toml
//                                            of clean checkouts (release profiles stay a per-repository choice: check reports them)
//   bun tools/toolchain.ts tools             print the install command of the cargo tools for a build host or CI
//   bun tools/toolchain.ts host <ssh-host> install      install the toolchains the checkouts pin (with llvm-tools) and the cargo tools on a build host
//   bun tools/toolchain.ts host <ssh-host> unused <repo> find unused dependencies of a checkout (cargo-shear) on a build host
// Checkouts are the sibling directories of this repository (or --root <dir>); a checkout with uncommitted changes in a file that
// apply would write is skipped, so work in progress of another session is never overwritten.
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const HERE = resolve(import.meta.dir, "..");
const POLICY = join(HERE, "templates", "toolchain.toml");

export type Install = Record<string, string | number | boolean>;
export type Policy = {
  channel: string;
  profile: string;
  components: string[];
  targets: string[];
  tools: Record<string, string>;
  install: Install;
};
type Raw = {
  sccache?: { size: string; basedirs: string[] };
  rust: { channel: string; profile: string; components: string[]; targets: string[]; nightly?: { pinned: string } };
  tools: Record<string, string>;
  bun: Install;
  repo?: Record<string, { channel?: string; targets?: string[]; components?: string[]; bun?: Install }>;
};

export const parsePolicy = (text: string): Raw => Bun.TOML.parse(text) as unknown as Raw;

/** The policy of one repository: the defaults with its [repo.<name>] exceptions on top. */
export function policyOf(raw: Raw, repo: string): Policy {
  const own = raw.repo?.[repo] ?? {};
  return {
    channel: own.channel ?? raw.rust.channel,
    profile: raw.rust.profile,
    components: own.components ?? raw.rust.components,
    targets: own.targets ?? raw.rust.targets,
    tools: raw.tools,
    install: { ...raw.bun, ...(own.bun ?? {}) },
  };
}

const literal = (value: string | number | boolean) => (typeof value === "string" ? JSON.stringify(value) : String(value));

/** Sets the given keys in a section of a TOML file, keeping every other line, section and comment; the section is added when missing. */
export function setSection(text: string, section: string, keys: Install): string {
  const newline = text.includes("\r\n") ? "\r\n" : "\n";
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const header = lines.findIndex((line) => line.replace(/\s*(#.*)?$/, "") === `[${section}]`);
  const wanted = new Map(Object.entries(keys));
  if (header === -1) {
    const body = [...wanted].map(([key, value]) => `${key} = ${literal(value)}`);
    const trimmed = lines.join("\n").replace(/\n*$/, "");
    return `${trimmed ? `${trimmed}\n\n` : ""}[${section}]\n${body.join("\n")}\n`.replace(/\n/g, newline);
  }
  let end = header + 1;
  while (end < lines.length && !/^\[/.test(lines[end]!)) end++;
  for (let index = header + 1; index < end; index++) {
    const key = lines[index]!.match(/^([A-Za-z][\w-]*)\s*=/)?.[1];
    if (key && wanted.has(key)) {
      lines[index] = `${key} = ${literal(wanted.get(key)!)}${lines[index]!.match(/\s+#.*$/)?.[0] ?? ""}`;
      wanted.delete(key);
    }
  }
  lines.splice(header + 1, 0, ...[...wanted].map(([key, value]) => `${key} = ${literal(value)}`));
  return lines.join("\n").replace(/\n/g, newline);
}

export const setInstall = (text: string, keys: Install): string => setSection(text, "install", keys);

/** The [install] values of a bunfig.toml that differ from the wanted ones. */
export function installDrift(text: string, keys: Install): string[] {
  const have = ((Bun.TOML.parse(text) as { install?: Install }).install ?? {}) as Install;
  return Object.entries(keys).filter(([key, value]) => have[key] !== value).map(([key, value]) => `bunfig [install] ${key} is ${have[key] === undefined ? "unset" : JSON.stringify(have[key])}, the policy says ${JSON.stringify(value)}`);
}

/** The dev-speed profile tables of the template (dev and its build override), as section name to keys. */
export function devProfiles(template: string): Record<string, Install> {
  const profile = (Bun.TOML.parse(template) as { profile: Record<string, Record<string, unknown>> }).profile.dev as Record<string, unknown>;
  const { "build-override": override, ...dev } = profile;
  return { "profile.dev": dev as Install, "profile.dev.build-override": override as Install };
}

export function renderToolchain(policy: Policy): string {
  const list = (items: string[]) => (items.length ? `[${items.map((item) => JSON.stringify(item)).join(", ")}]` : "[]");
  return [`[toolchain]`, `channel = ${JSON.stringify(policy.channel)}`, `profile = ${JSON.stringify(policy.profile)}`, `components = ${list(policy.components)}`, ...(policy.targets.length ? [`targets = ${list(policy.targets)}`] : []), ""].join("\n");
}

/** Replaces the channel of an existing rust-toolchain.toml and leaves the rest (comments, components, targets) alone. */
export function setChannel(text: string, channel: string): string {
  return text.replace(/^(\s*channel\s*=\s*)"[^"]*"/m, `$1${JSON.stringify(channel)}`);
}

export const channelOf = (text: string): string | undefined => (Bun.TOML.parse(text) as { toolchain?: { channel?: string } }).toolchain?.channel;

const PROFILE_KEYS = ["split-debuginfo", "incremental", "codegen-units", "debug", "lto"] as const;

/** The profile keys of a workspace manifest that differ from the template (only keys the manifest sets are compared). */
export function profileDrift(manifest: string, template: string): string[] {
  const have = (Bun.TOML.parse(manifest) as { profile?: Record<string, Record<string, unknown>> }).profile ?? {};
  const want = (Bun.TOML.parse(template) as { profile: Record<string, Record<string, unknown>> }).profile;
  const out: string[] = [];
  for (const name of ["dev", "release"]) {
    for (const key of PROFILE_KEYS) {
      const expected = want[name]?.[key];
      if (expected === undefined) continue;
      const actual = have[name]?.[key];
      if (actual !== expected) out.push(`[profile.${name}] ${key} is ${actual === undefined ? "unset" : JSON.stringify(actual)}, Zed's is ${JSON.stringify(expected)}`);
    }
  }
  if (!have.dev?.["build-override"]) out.push("[profile.dev.build-override] is unset: build scripts and proc macros compile a second time for the build platform");
  return out;
}

type Repo = { name: string; dir: string };
const repos = (root: string): Repo[] =>
  readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && existsSync(join(root, entry.name, ".git")))
    .map((entry) => ({ name: entry.name, dir: join(root, entry.name) }));

const read = (path: string) => (existsSync(path) ? readFileSync(path, "utf8") : undefined);
const git = (dir: string, ...args: string[]) => Bun.spawnSync(["git", "-C", dir, ...args], { stdout: "pipe", stderr: "pipe" }).stdout.toString();
const dirty = (dir: string, file: string) => git(dir, "status", "--porcelain", "--", file).trim() !== "";
const hasRust = (dir: string) => existsSync(join(dir, "Cargo.toml"));
const hasBun = (dir: string) => existsSync(join(dir, "package.json")) && /"workspaces"|"dependencies"|"devDependencies"/.test(read(join(dir, "package.json")) ?? "");

export function problems(repo: Repo, raw: Raw, template: string): string[] {
  const policy = policyOf(raw, repo.name);
  const out: string[] = [];
  if (hasRust(repo.dir)) {
    const text = read(join(repo.dir, "rust-toolchain.toml"));
    const channel = text ? channelOf(text) : undefined;
    if (channel === undefined) out.push("rust-toolchain.toml is missing");
    else if (channel !== policy.channel) out.push(`rust-toolchain.toml channel is ${channel}, the policy says ${policy.channel}`);
    const manifest = read(join(repo.dir, "Cargo.toml")) ?? "";
    if (/^\[workspace\]/m.test(manifest)) out.push(...profileDrift(manifest, template));
  }
  if (hasBun(repo.dir)) out.push(...installDrift(read(join(repo.dir, "bunfig.toml")) ?? "", policy.install));
  return out;
}

export function apply(repo: Repo, raw: Raw, template: string): string[] {
  const policy = policyOf(raw, repo.name);
  const done: string[] = [];
  if (hasRust(repo.dir)) {
    const file = join(repo.dir, "rust-toolchain.toml");
    if (!dirty(repo.dir, "rust-toolchain.toml")) {
      const text = read(file);
      const next = text === undefined ? renderToolchain(policy) : setChannel(text, policy.channel);
      if (next !== text) {
        writeFileSync(file, next);
        done.push("rust-toolchain.toml");
      }
    }
  }
  if (hasRust(repo.dir) && /^\[workspace\]/m.test(read(join(repo.dir, "Cargo.toml")) ?? "") && !dirty(repo.dir, "Cargo.toml")) {
    const file = join(repo.dir, "Cargo.toml");
    const text = read(file) ?? "";
    let next = text;
    for (const [section, keys] of Object.entries(devProfiles(template))) next = setSection(next, section, keys);
    if (next !== text) {
      writeFileSync(file, next);
      done.push("Cargo.toml");
    }
  }
  if (hasBun(repo.dir)) {
    const file = join(repo.dir, "bunfig.toml");
    if (!dirty(repo.dir, "bunfig.toml")) {
      const text = read(file) ?? "";
      const next = setInstall(text, policy.install);
      if (next !== text) {
        writeFileSync(file, next);
        done.push("bunfig.toml");
      }
    }
  }
  return done;
}

/** The bash script that brings a build host to the policy: every pinned toolchain with its components, the cargo tools, the sccache settings. */
export function hostScript(raw: Raw, channels: string[]): string {
  const components = [...raw.rust.components, "llvm-tools"].map((component) => `-c ${component}`).join(" ");
  const tools = Object.entries(raw.tools).map(([name, version]) => `${name}@${version}`).join(" ");
  const sccache = raw.sccache;
  return [
    "set -u",
    'export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"',
    ...channels.map((channel) => `rustup toolchain install ${channel} --profile minimal ${components} < /dev/null`),
    `cargo binstall --no-confirm --locked ${tools} < /dev/null`,
    ...(sccache
      ? [
          'config="$HOME/.config/sccache/config"',
          `if [ ! -f "$config" ]; then mkdir -p "$(dirname "$config")"; printf '[cache.disk]\ndir = "%s/.cache/sccache"\nsize = %s\n' "$HOME" "$(numfmt --from=iec ${sccache.size.replace(/G$/, "G")})" > "$config"; fi`,
          `echo "sccache: $(sccache --version), config $config"`,
          `echo "RUSTC_WRAPPER=\${RUSTC_WRAPPER:-unset} (set it, with CARGO_INCREMENTAL=0, in the host environment; basedirs ${sccache.basedirs.join(":")})"`,
        ]
      : []),
    "",
  ].join("\n");
}

async function ssh(host: string, script: string): Promise<number> {
  const name = `/tmp/toolchain-${Date.now()}.sh`;
  const write = Bun.spawn(["ssh", "-o", "BatchMode=yes", host, `cat > ${name}`], { stdin: new TextEncoder().encode(script), stdout: "inherit", stderr: "inherit" });
  await write.exited;
  const run = Bun.spawn(["ssh", "-o", "BatchMode=yes", host, `bash ${name} < /dev/null; code=$?; rm -f ${name}; exit $code`], { stdout: "inherit", stderr: "inherit" });
  return run.exited;
}

async function stageAndShear(host: string, repo: Repo): Promise<number> {
  const files = git(repo.dir, "ls-files", "-co", "--exclude-standard", "-z").split("\0").filter((file) => file && !/^(node_modules|target|third_party|vendor)\//.test(file));
  const tar = Bun.spawn(["tar", "-C", repo.dir, "-cf", "-", "--null", "-T", "-"], { stdin: new TextEncoder().encode(files.join("\0")), stdout: "pipe" });
  const stage = `/tmp/toolchain-unused-${repo.name}`;
  const extract = Bun.spawn(["ssh", "-o", "BatchMode=yes", host, `rm -rf ${stage} && mkdir -p ${stage} && tar xf - -C ${stage}`], { stdin: tar.stdout, stdout: "inherit", stderr: "inherit" });
  await extract.exited;
  return ssh(host, `export PATH="$HOME/.cargo/bin:$PATH"; cd ${stage} && cargo shear; code=$?; cd /; rm -rf ${stage}; exit $code`);
}

async function latestStable(): Promise<string> {
  const text = await (await fetch("https://static.rust-lang.org/dist/channel-rust-stable.toml")).text();
  const block = text.split("[pkg.rust]")[1] ?? "";
  const version = block.match(/^version = "(\d+\.\d+\.\d+)/m)?.[1];
  if (!version) throw new Error("cannot read the stable version");
  return version;
}

async function latestCrate(name: string): Promise<string> {
  const response = await fetch(`https://crates.io/api/v1/crates/${name}`, { headers: { "user-agent": "aphrody-labs toolchain (https://github.com/aphrody-labs/.github)" } });
  const version = ((await response.json()) as { crate?: { max_stable_version?: string } }).crate?.max_stable_version;
  if (!version) throw new Error(`cannot read the latest version of ${name}`);
  return version;
}

async function update(): Promise<void> {
  let text = readFileSync(POLICY, "utf8");
  const raw = parsePolicy(text);
  const stable = await latestStable();
  text = text.replace(/^(channel = )"[\d.]+"/m, `$1"${stable}"`);
  console.log(`rust ${raw.rust.channel} -> ${stable}`);
  for (const [name, current] of Object.entries(raw.tools)) {
    const latest = await latestCrate(name);
    if (latest !== current) console.log(`${name} ${current} -> ${latest}`);
    text = text.replace(new RegExp(`^(${name} = )"[^"]*"`, "m"), `$1"${latest}"`);
  }
  writeFileSync(POLICY, text);
}

if (import.meta.main) {
  const argv = Bun.argv.slice(2);
  const [command = "check"] = argv;
  const rootFlag = argv.indexOf("--root");
  const root = resolve(rootFlag >= 0 ? argv[rootFlag + 1]! : join(HERE, ".."));
  const names = command === "host" ? [] : argv.slice(1).filter((arg, index) => !arg.startsWith("--") && argv[index] !== "--root");
  const raw = parsePolicy(readFileSync(POLICY, "utf8"));
  const template = readFileSync(join(HERE, "templates", "cargo-profiles.toml"), "utf8");
  const selected = repos(root).filter((repo) => names.length === 0 || names.includes(repo.name));
  if (command === "update") await update();
  else if (command === "host") {
    const [, host, action, ...rest] = argv.filter((arg, index) => !arg.startsWith("--") && argv[index - 1] !== "--root");
    if (!host || (action !== "install" && action !== "unused")) {
      console.error("usage: toolchain.ts host <ssh-host> install | unused <repo>");
      process.exit(2);
    }
    if (action === "install") {
      const channels = [...new Set(repos(root).map((repo) => read(join(repo.dir, "rust-toolchain.toml"))).filter((text): text is string => !!text).map((text) => channelOf(text)).filter((channel): channel is string => !!channel))].sort();
      process.exit(await ssh(host, hostScript(raw, channels)));
    }
    const repo = repos(root).find((candidate) => candidate.name === rest[0]);
    if (!repo) throw new Error(`no checkout named ${rest[0]}`);
    process.exit(await stageAndShear(host, repo));
  } else if (command === "tools") console.log(`cargo binstall --locked --no-confirm ${Object.entries(raw.tools).map(([name, version]) => `${name}@${version}`).join(" ")}`);
  else if (command === "check") {
    let drift = 0;
    for (const repo of selected) {
      const list = problems(repo, raw, template);
      drift += list.length;
      console.log(`${list.length ? "DRIFT" : "ok   "} ${repo.name}${list.map((line) => `\n       ${line}`).join("")}`);
    }
    process.exit(drift ? 1 : 0);
  } else if (command === "apply") {
    for (const repo of selected) {
      const done = apply(repo, raw, template);
      console.log(`${repo.name}: ${done.length ? done.join(", ") : "nothing to write"}`);
    }
  } else {
    console.error("usage: toolchain.ts update | check [repo ...] | apply [repo ...] | tools [--root dir]");
    process.exit(2);
  }
}
