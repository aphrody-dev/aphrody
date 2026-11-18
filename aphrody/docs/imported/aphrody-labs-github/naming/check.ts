#!/usr/bin/env bun
// naming-ignore-file
// Checks one repository against the Aphrody naming scheme (naming.toml, NAMING.md): names per registry, and independence from
// every other repository of the family. Reads only the checked repository's own files; never a sibling checkout, never the network.
// Usage: bun naming/check.ts [repoDir] [--scheme naming.toml] [--json] [--strict] [--inventory]
//   default: report only, exit 0 (a naming check never blocks a build). --strict: exit 1 when an error is found.
import { join, resolve, relative, dirname, posix, isAbsolute } from "node:path";

export type Severity = "error" | "warn" | "info";
export interface Finding {
  rule: string;
  severity: Severity;
  file: string;
  line?: number;
  message: string;
}
export interface Decl {
  id: string;
  kind: string;
  org: string;
  repo?: string; // GitHub repository name when it differs from id
  family?: string;
  names?: Record<string, string>; // overrides: npm_scope, crate_prefix, env_prefix, ghcr
  legacy?: { crates?: string[]; npm?: string[]; env?: string[]; repos?: string[] };
  exceptions?: Record<string, string>; // rule id -> reason, the finding is demoted to info
}
export interface Scheme {
  meta: { version: string; orgs: string[] };
  orgs: Record<string, { npm_scope: string; ghcr: string }>;
  kinds: Record<string, { orgs: string[]; pattern: string; prefix: string; publishes: string[] }>;
  env: { neutral: string[]; generic?: string[] };
  stages: { names: string[] };
  repos: Record<string, string>;
  families?: Record<string, { root: string; members: string[] }>;
  forks?: { aliases?: Record<string, string> };
}
export type Files = Map<string, string>;

const SHA = /^[0-9a-f]{40}$/i;
const BRANCHY = /^(main|master|HEAD|trunk|develop|dev|latest|v?\d+(\.\d+)*)$/i;
const SKIP_DIR = /(^|\/)(node_modules|target|third_party|vendor|upstream|dist|\.git|graphify-out|\.next|\.turbo|coverage|var|fixtures?|testdata|__fixtures__|__snapshots__)\//;
const SKIP_FILE = /(\.lock|\.lockb|\.min\.[a-z]+|\.map|\.snap|-lock\.json|\.svg|\.png|\.jpe?g|\.gif|\.ico|\.woff2?|\.ttf|\.wasm|\.zip|\.gz|\.pdf|\.mp4|\.node|\.dll|\.exe|\.so|\.a|\.rlib)$/i;
const CODE = /\.(ts|tsx|js|mjs|cjs|jsx|py|rs|go|sh|bash|ps1|cmd|bat|c|cc|cpp|h|hpp|cs|rb|lua|zig|just|mk|nix)$|(^|\/)(Dockerfile[^/]*|justfile|Makefile)$/;
const CONFIG = /\.(toml|json|jsonc|yml|yaml|csproj|fsproj|props|targets|hcl|ini|cfg)$/;
const DOC = /\.(md|mdx|txt|rst)$/i;

export const loadScheme = async (path = join(import.meta.dir, "naming.toml")): Promise<Scheme> => Bun.TOML.parse(await Bun.file(path).text()) as unknown as Scheme;

export const pascal = (s: string) => s.split(/[-_]/).filter(Boolean).map(w => w[0].toUpperCase() + w.slice(1)).join("");
export const shortOf = (id: string) => id.replace(/^aphrody-/, "").replace(/^aphrody$/, "");
export const envPrefixOf = (id: string) => id.toUpperCase().replace(/-/g, "_");
const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** The prefixes a repository may use, per registry, derived from its own declaration only. */
export function expected(decl: Decl, scheme: Scheme) {
  const org = scheme.orgs[decl.org] ?? scheme.orgs["aphrody-labs"];
  const repo = decl.repo ?? decl.id;
  const own = [decl.id, ...(decl.family ? [decl.family] : [])];
  const short = shortOf(decl.id);
  const kind = scheme.kinds[decl.kind];
  const free = !kind || kind.prefix === "upstream" || kind.prefix === "none" || String(decl.names?.upstream) === "true";
  return {
    free,
    repo,
    scope: decl.names?.npm_scope ?? org.npm_scope,
    npmStems: [...new Set([short, ...(decl.family ? [shortOf(decl.family)] : [])].filter(Boolean))],
    crateStems: decl.names?.crate_prefix ? [decl.names.crate_prefix.replace(/-$/, "")] : own,
    envPrefixes: decl.names?.env_prefix ? [decl.names.env_prefix] : own.map(i => envPrefixOf(i) + "_"),
    ghcr: decl.names?.ghcr ?? org.ghcr,
    pypiStems: own,
    nugetStem: "Aphrody." + (short ? pascal(short) : ""),
    cfStems: own,
  };
}

const matchesStem = (name: string, stems: string[]) => stems.some(s => name === s || name.startsWith(s + "-"));
const startsAny = (name: string, prefixes: string[] = []) => prefixes.some(p => name.startsWith(p));

/** Known other repositories of the family, by bare name, from the registry plus aliases; the checked repository is excluded. */
export function siblingNames(decl: Decl, scheme: Scheme): Set<string> {
  const names = new Set<string>();
  const self = new Set([decl.id, decl.repo ?? decl.id, ...(decl.legacy?.repos ?? [])].map(s => s.toLowerCase()));
  for (const slug of Object.keys(scheme.repos)) {
    const name = slug.split("/")[1].toLowerCase();
    if (!self.has(name)) names.add(name);
  }
  return names;
}

const isSelfSlug = (org: string, repo: string, decl: Decl) => {
  const r = repo.replace(/\.git$/, "").toLowerCase();
  return [decl.id, decl.repo ?? decl.id, ...(decl.legacy?.repos ?? [])].some(n => n.toLowerCase() === r);
};

const OWN_SLUG = (orgs: string[]) => new RegExp(`(?:github\\.com[/:]|raw\\.githubusercontent\\.com/|api\\.github\\.com/repos/|(?<![\\w./-]))(${orgs.map(esc).join("|")})/([A-Za-z0-9._-]+)`, "g");

function jsonc(text: string): any {
  const out = text.replace(/("(?:[^"\\]|\\.)*")|\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, (m, s) => s ?? "").replace(/,(\s*[}\]])/g, "$1");
  return JSON.parse(out);
}
const lineOf = (text: string, index: number) => text.slice(0, index).split("\n").length;

export function checkFiles(files: Files, decl: Decl, scheme: Scheme): Finding[] {
  const out: Finding[] = [];
  const exp = expected(decl, scheme);
  const orgs = scheme.meta.orgs;
  const ownSlug = OWN_SLUG(orgs);
  const siblings = siblingNames(decl, scheme);
  const add = (rule: string, severity: Severity, file: string, message: string, line?: number) => {
    const reason = decl.exceptions?.[rule];
    out.push(reason ? { rule, severity: "info", file, line, message: `${message} (declared exception: ${reason})` } : { rule, severity, file, line, message });
  };

  // N001: the declaration itself.
  const kind = scheme.kinds[decl.kind];
  if (!kind) add("N001", "error", "naming.toml", `unknown kind "${decl.kind}"; known: ${Object.keys(scheme.kinds).join(", ")}`);
  else {
    if (!kind.orgs.includes(decl.org)) add("N001", "warn", "naming.toml", `kind ${decl.kind} lives in ${kind.orgs.join(" or ")}, not ${decl.org}`);
    if (kind.pattern !== "upstream" && kind.pattern !== "any" && !new RegExp(kind.pattern).test(exp.repo))
      add("N002", "error", "naming.toml", `repository name "${exp.repo}" does not match the ${decl.kind} rule ${kind.pattern}`);
  }
  if (!/^[a-z0-9.-]+$/.test(exp.repo) && kind?.pattern !== "upstream") add("N003", "error", "naming.toml", `repository name "${exp.repo}" must be lowercase kebab-case`);

  const inside = (from: string, target: string) => {
    if (isAbsolute(target) || /^[A-Za-z]:[\\/]/.test(target)) return false;
    return !posix.normalize(posix.join(dirname(from), target.replace(/\\/g, "/"))).startsWith("..");
  };
  const stemsCheck = (rule: string, file: string, name: string, stems: string[], what: string) => {
    if (exp.free) return;
    if (matchesStem(name, stems)) return;
    const legacy = startsAny(name, what === "crate" ? decl.legacy?.crates : what === "npm" ? decl.legacy?.npm : undefined);
    add(rule, legacy ? "info" : "warn", file, `${what} name "${name}" must start with ${stems.map(s => `"${s}"`).join(" or ")}${legacy ? " (declared legacy name)" : ""}`);
  };

  for (const [file, text] of files) {
    if (SKIP_DIR.test(file)) continue;
    const base = file.split("/").pop()!;
    const isDoc = DOC.test(file);
    const isWorkflow = /(^|\/)\.github\/workflows\/[^/]+\.ya?ml$/.test(file) || /(^|\/)(workflow-templates|\.gitea\/workflows)\/[^/]+\.ya?ml$/.test(file);

    // ---- .gitmodules ----------------------------------------------------------------------------------------
    if (base === ".gitmodules") {
      const re = /\[submodule "([^"]+)"\]([\s\S]*?)(?=\n\[submodule|$)/g;
      for (const m of text.matchAll(re)) {
        const url = /url\s*=\s*(\S+)/.exec(m[2])?.[1] ?? "";
        const branch = /branch\s*=\s*(\S+)/.exec(m[2])?.[1];
        const line = lineOf(text, m.index!);
        const relative_ = url.startsWith("../") || url.startsWith("./");
        const slug = new RegExp(`(${orgs.map(esc).join("|")})/([A-Za-z0-9._-]+?)(?:\\.git)?/?$`).exec(url);
        const target = relative_ ? url.replace(/^(\.\.?\/)+/, "").replace(/\.git$/, "").split("/").pop()! : slug?.[2];
        const known = target ? scheme.repos[`${slug?.[1] ?? decl.org}/${target}`] ?? Object.entries(scheme.repos).find(([k]) => k.split("/")[1].toLowerCase() === target.toLowerCase())?.[1] : undefined;
        if (target && (relative_ || slug) && !isSelfSlug(slug?.[1] ?? decl.org, target, decl)) {
          if (known && !["fork", "mirror", "runtime"].includes(known)) add("I001", "error", file, `submodule "${m[1]}" embeds ${target}, an original repository of the family: use its published package or a pinned optional download`, line);
          else add("I002", "warn", file, `submodule "${m[1]}" points into the family (${url}): must be a pinned fork or mirror and optional for the build`, line);
          if (relative_) add("I003", "info", file, `submodule "${m[1]}" uses a relative URL ${url}: it breaks when the repository is mirrored or renamed`, line);
        }
        if (branch && (relative_ || slug)) add("I004", known && !["fork", "mirror", "runtime"].includes(known) ? "error" : "warn", file, `submodule "${m[1]}" tracks branch ${branch}: pin a commit, a clean clone must give the same tree`, line);
      }
      continue;
    }

    // ---- Cargo ----------------------------------------------------------------------------------------------
    if (base === "Cargo.toml") {
      let t: any;
      try {
        t = Bun.TOML.parse(text);
      } catch {
        continue;
      }
      const name = t.package?.name;
      if (typeof name === "string" && t.package?.publish !== false) {
        if (name.includes("_") && !exp.free) add("N013", "warn", file, `crate "${name}" uses underscores: crates.io names are hyphenated`);
        stemsCheck("N011", file, name.replace(/_/g, "-"), exp.crateStems, "crate");
      }
      const walk = (tables: any, where: string) => {
        for (const [dep, spec] of Object.entries<any>(tables ?? {})) {
          if (typeof spec !== "object" || spec === null) continue;
          if (typeof spec.path === "string" && !inside(file, spec.path)) add("I010", "error", file, `${where} "${dep}" path ${spec.path} leaves the repository: use a published version, optional`);
          if (typeof spec.git === "string") {
            const sm = new RegExp(`(${orgs.map(esc).join("|")})/([A-Za-z0-9._-]+)`).exec(spec.git);
            if (sm && !isSelfSlug(sm[1], sm[2], decl)) {
              if (!(spec.rev || spec.tag) || (spec.branch && !spec.rev)) add("I011", "error", file, `${where} "${dep}" git dependency on ${sm[1]}/${sm[2]} without a pinned rev`);
              else if (!spec.optional) add("I012", "warn", file, `${where} "${dep}" git dependency on ${sm[1]}/${sm[2]} is required: make it optional (feature) or use the registry`);
            }
          }
        }
      };
      const sections = (root: any, label: string) => {
        for (const k of ["dependencies", "dev-dependencies", "build-dependencies"]) walk(root?.[k], `${label}${k}`);
        for (const [tn, tv] of Object.entries<any>(root?.target ?? {})) for (const k of ["dependencies", "dev-dependencies", "build-dependencies"]) walk(tv?.[k], `target.${tn}.${k}`);
      };
      sections(t, "");
      walk(t.workspace?.dependencies, "workspace.dependencies");
      for (const [reg, tbl] of Object.entries<any>(t.patch ?? {})) walk(tbl, `patch.${reg}`);
      walk(t.replace, "replace");
      for (const m of (t.workspace?.members ?? []).concat(t.workspace?.exclude ?? [])) if (typeof m === "string" && !inside(file, m)) add("I013", "error", file, `workspace member ${m} leaves the repository`);
      continue;
    }

    // ---- package.json ---------------------------------------------------------------------------------------
    if (base === "package.json") {
      let p: any;
      try {
        p = JSON.parse(text);
      } catch {
        continue;
      }
      const name: string | undefined = p.name;
      if (typeof name === "string" && !exp.free && p.private !== true) {
        const m = /^(@[^/]+)\/(.+)$/.exec(name);
        const unit = m ? m[2] : name;
        if (!m && p.private !== true) add("N010", "warn", file, `published package "${name}" must be scoped: ${exp.scope}/<${exp.npmStems[0] ?? "unit"}>-<unit>`);
        else if (m && m[1] !== exp.scope && !startsAny(name, decl.legacy?.npm)) add("N010", "warn", file, `package scope ${m[1]} must be ${exp.scope}`);
        else if (m && exp.npmStems.length && !matchesStem(unit.replace(/-monorepo$/, ""), exp.npmStems) && !startsAny(name, decl.legacy?.npm)) add("N012", "warn", file, `package "${name}" must start with ${exp.scope}/${exp.npmStems.join(" or ")}`);
      }
      for (const k of ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"]) {
        for (const [dep, spec] of Object.entries<any>(p[k] ?? {})) {
          if (typeof spec !== "string") continue;
          const loc = /^(?:file|link|portal):(.+)$/.exec(spec);
          if (loc && !inside(file, loc[1])) add("I020", "error", file, `${k}.${dep} = ${spec} leaves the repository`);
          const gh = /^(?:github:|git\+https?:\/\/github\.com\/|https?:\/\/github\.com\/)?([\w.-]+)\/([\w.-]+?)(?:\.git)?(?:#(.+))?$/.exec(spec);
          if (gh && orgs.includes(gh[1]) && !isSelfSlug(gh[1], gh[2], decl)) add("I021", SHA.test(gh[3] ?? "") ? "warn" : "error", file, `${k}.${dep} installs ${gh[1]}/${gh[2]}${gh[3] ? "#" + gh[3] : ""} from GitHub: use the registry version${SHA.test(gh[3] ?? "") ? " (pinned, keep optional)" : ""}`);
        }
      }
      if (typeof p.bin === "object" && p.bin) for (const b of Object.keys(p.bin)) if (!exp.free && !matchesStem(b, exp.cfStems)) add("N016", "info", file, `binary "${b}" should be ${decl.id} or ${decl.id}-<unit>`);
      for (const ws of Array.isArray(p.workspaces) ? p.workspaces : p.workspaces?.packages ?? []) if (typeof ws === "string" && !inside(file, ws)) add("I022", "error", file, `workspace ${ws} leaves the repository`);
      continue;
    }

    // ---- go.mod ---------------------------------------------------------------------------------------------
    if (base === "go.mod") {
      const mod = /^module\s+(\S+)/m.exec(text)?.[1];
      if (mod && !exp.free) {
        const m = /^github\.com\/([^/]+)\/([^/]+)/.exec(mod);
        if (!m || m[1].toLowerCase() !== decl.org || m[2].toLowerCase() !== exp.repo.toLowerCase()) add("N020", "warn", file, `Go module ${mod} must be github.com/${decl.org}/${exp.repo}[/unit]`);
      }
      for (const m of text.matchAll(/=>\s*(\S+)/g)) if (m[1].startsWith(".") && !inside(file, m[1])) add("I030", "error", file, `replace => ${m[1]} leaves the repository`, lineOf(text, m.index!));
      continue;
    }

    // ---- pyproject ------------------------------------------------------------------------------------------
    if (base === "pyproject.toml") {
      let t: any;
      try {
        t = Bun.TOML.parse(text);
      } catch {
        continue;
      }
      const name = t.project?.name;
      if (typeof name === "string") stemsCheck("N030", file, name.toLowerCase().replace(/_/g, "-"), exp.pypiStems, "PyPI");
      for (const [dep, src] of Object.entries<any>(t.tool?.uv?.sources ?? {})) {
        if (typeof src.path === "string" && !inside(file, src.path)) add("I040", "error", file, `uv source ${dep} path ${src.path} leaves the repository`);
        if (typeof src.git === "string" && ownSlug.test(src.git) && !SHA.test(src.rev ?? "")) add("I041", "error", file, `uv source ${dep} installs ${src.git} without a pinned rev`);
        ownSlug.lastIndex = 0;
      }
      continue;
    }

    // ---- .NET -----------------------------------------------------------------------------------------------
    if (/\.(cs|fs|vb)proj$/.test(base)) {
      const id = /<PackageId>([^<]+)<\/PackageId>/.exec(text)?.[1] ?? /<AssemblyName>([^<]+)<\/AssemblyName>/.exec(text)?.[1];
      if (id && !exp.free && !(id === exp.nugetStem.replace(/\.$/, "") || id.startsWith(exp.nugetStem))) add("N040", "warn", file, `.NET package/assembly "${id}" must start with ${exp.nugetStem}`);
      for (const m of text.matchAll(/<ProjectReference\s+Include="([^"]+)"/g)) if (!inside(file, m[1])) add("I050", "error", file, `ProjectReference ${m[1]} leaves the repository`, lineOf(text, m.index!));
      continue;
    }

    // ---- Cloudflare -----------------------------------------------------------------------------------------
    if (/^wrangler\.(toml|jsonc?)$/.test(base)) {
      let w: any;
      try {
        w = base.endsWith(".toml") ? Bun.TOML.parse(text) : jsonc(text);
      } catch {
        continue;
      }
      const names: [string, any][] = [["name", w.name]];
      for (const k of ["d1_databases", "r2_buckets", "kv_namespaces", "queues", "services"]) {
        const v = k === "queues" ? [...(w.queues?.producers ?? []), ...(w.queues?.consumers ?? [])] : w[k] ?? [];
        for (const b of v) names.push([k, b.database_name ?? b.bucket_name ?? b.queue ?? b.service ?? b.title ?? b.binding]);
      }
      for (const [k, n] of names) if (typeof n === "string" && !exp.free && !/^[A-Z_0-9]+$/.test(n)) {
        if (!matchesStem(n, exp.cfStems)) add("N060", "warn", file, `Cloudflare ${k} "${n}" must be ${decl.id}-<unit>[-${scheme.stages.names.join("|")}]`);
        else if (k === "services" && !isSelfSlug("", n, decl) && siblings.has(n.split("-")[0])) add("I061", "error", file, `service binding ${n} targets another repository's Worker`);
      }
      continue;
    }

    // ---- Workflows ------------------------------------------------------------------------------------------
    if (isWorkflow) {
      const lines = text.split("\n");
      lines.forEach((ln, i) => {
        const L = i + 1;
        const u = /^\s*-?\s*uses:\s*['"]?([^'"\s#]+)/.exec(ln);
        if (u) {
          const m = /^([\w.-]+)\/([\w.-]+)(\/[^@]*)?@(.+)$/.exec(u[1]);
          if (m && orgs.includes(m[1]) && !isSelfSlug(m[1], m[2], decl)) {
            if (SHA.test(m[4])) add("I070", "info", file, `uses ${m[1]}/${m[2]} pinned to a SHA: allowed, keep the job optional (continue-on-error) so it never blocks`, L);
            else if (m[2] === ".github") add("I071", "warn", file, `uses the org reusable workflow ${m[1]}/${m[2]}${m[3] ?? ""}@${m[4]}: a moving ref couples every repository's CI to one commit; pin a SHA (Dependabot keeps it current)`, L);
            else add("I071", "error", file, `uses ${m[1]}/${m[2]}${m[3] ?? ""}@${m[4]}: a branch or tag moves under you; pin a SHA`, L);
          }
        }
        const r = /^\s*(?:-\s*)?repository:\s*['"]?([\w.-]+)\/([\w.-]+)/.exec(ln);
        if (r && orgs.includes(r[1]) && !isSelfSlug(r[1], r[2], decl)) add("I072", "error", file, `checks out ${r[1]}/${r[2]} in CI: a clean clone of this repository must be enough`, L);
        if (/repos\/[\w.-]+\/[\w.-]+\/(dispatches|actions\/workflows\/[^/]+\/dispatches)/.test(ln)) {
          const d = /repos\/([\w.-]+)\/([\w.-]+)\//.exec(ln);
          if (d && orgs.includes(d[1]) && !isSelfSlug(d[1], d[2], decl)) add("I073", "error", file, `triggers a workflow of ${d[1]}/${d[2]}: no repository starts another's CI`, L);
        }
      });
    }

    // ---- Dockerfiles, compose, bake, workflows: image names ------------------------------------------------
    if (/(^|\/)(Dockerfile[^/]*|compose[^/]*\.ya?ml|docker-compose[^/]*\.ya?ml|docker-bake\.hcl)$/.test(file) || isWorkflow) {
      for (const m of text.matchAll(/ghcr\.io\/([\w.-]+)\/([\w.${}-]+)/g)) {
        const [, org, rest] = m;
        if (!orgs.includes(org.toLowerCase())) continue;
        const repo = rest.split(/[:@]/)[0];
        const L = lineOf(text, m.index!);
        if (repo.includes("$")) continue;
        if (isSelfSlug(org, repo.split("-")[0], decl) || isSelfSlug(org, repo, decl) || matchesStem(repo, [exp.repo, decl.id])) continue;
        const fromLine = text.split("\n")[L - 1];
        add("I060", /@sha256:/.test(fromLine) ? "info" : "warn", file, `image ghcr.io/${org}/${repo} belongs to another repository: pin a digest and keep a fallback`, L);
      }
    }

    if (SKIP_DIR.test(file) || SKIP_FILE.test(file)) continue;
    if (text.length > 600_000) continue;

    // ---- Generic text: sibling paths, branch-pinned fetches, hardcoded slugs, env vars ---------------------------
    const code = CODE.test(file) || isWorkflow;
    const cfg = CONFIG.test(file);
    if (!code && !cfg && !isDoc) continue;
    const sev = (strong: Severity): Severity => (isDoc ? "info" : strong);

    if (siblings.size) {
      const names = [...siblings].sort((a, b) => b.length - a.length).map(esc).join("|");
      const sib = new RegExp(`((?:\\.\\.[\\\\/]+)+|\\$WORKSPACE_ROOT[\\\\/]+|%WORKSPACE_ROOT%[\\\\/]+|[A-Za-z]:[\\\\/]src[\\\\/]+|/c/src/+|~/src/+|\\$HOME/src/+)(${names})(?![\\w-]|\\.\\w)`, "g");
      const depth = dirname(file) === "." ? 0 : dirname(file).split("/").length;
      for (const m of text.matchAll(sib)) {
        if (m[1].startsWith("..") && m[1].split(/[\\/]/).filter(Boolean).length <= depth) continue; // stays inside the repository
        const L = lineOf(text, m.index!);
        const lineText = text.split("\n")[L - 1];
        const hint = cfg && /checkout\s*=/.test(lineText);
        add("I090", hint ? "info" : sev(code ? "error" : "warn"), file, `reads the sibling checkout of ${m[2]} (${m[0]}): resolve it from env or config, default to the registry or skip`, L);
      }
      if (code || cfg) {
        for (const m of text.matchAll(/(?:\$WORKSPACE_ROOT|%WORKSPACE_ROOT%|process\.env\.WORKSPACE_ROOT|env::var\("WORKSPACE_ROOT"\))/g))
          add("I091", "info", file, "depends on a shared WORKSPACE_ROOT layout: accept it only as an optional override", lineOf(text, m.index!));
      }
    }

    for (const m of text.matchAll(/raw\.githubusercontent\.com\/([\w.-]+)\/([\w.-]+)\/(?:refs\/heads\/)?([\w.\/-]+?)\//g)) {
      if (!orgs.includes(m[1]) || isSelfSlug(m[1], m[2], decl)) continue;
      if (!SHA.test(m[3])) add("I080", sev(BRANCHY.test(m[3]) || !SHA.test(m[3]) ? "error" : "warn"), file, `fetches a file of ${m[1]}/${m[2]} at ${m[3]}: pin a SHA or vendor a checked copy`, lineOf(text, m.index!));
    }
    if (!isDoc) {
      for (const m of text.matchAll(/git\s+clone\b[^\n]*?github\.com[/:]([\w.-]+)\/([\w.-]+?)(?:\.git)?(?:\s|["'`]|$)/g)) {
        if (orgs.includes(m[1]) && !isSelfSlug(m[1], m[2], decl)) add("I081", "warn", file, `clones ${m[1]}/${m[2]} at run time: only a pinned SHA, only optional`, lineOf(text, m.index!));
      }
      for (const m of text.matchAll(/gh\s+(?:repo\s+clone|release\s+download|run\s+download)\s+([\w.-]+)\/([\w.-]+)/g)) {
        if (orgs.includes(m[1]) && !isSelfSlug(m[1], m[2], decl)) add("I082", "warn", file, `fetches from ${m[1]}/${m[2]} at run time: pin a release tag or SHA, keep optional`, lineOf(text, m.index!));
      }
    }
    if (!isDoc && !isWorkflow) {
      const seen = new Set<string>();
      for (const m of text.matchAll(ownSlug)) {
        const [, org, repo0] = m;
        const repo = repo0.replace(/\.git$/, "").replace(/[.]+$/, "");
        if (isSelfSlug(org, repo, decl) || repo === ".github" || !repo || repo.startsWith(decl.id + "-")) continue;
        const key = `${org}/${repo}`;
        if (seen.has(key)) continue;
        seen.add(key);
        add("I100", "info", file, `names another repository (${key}) in ${cfg ? "configuration" : "code"}: read it from this repository's identity file`, lineOf(text, m.index!));
      }
      const gen = /[Gg]enerated (?:by|from)[^\n]*?((?:aphrody-dev|aphrody-labs|rose-griffon)\/[\w.-]+)@?([0-9a-f]{7,40})?/.exec(text.slice(0, 600));
      if (gen && !isSelfSlug("", gen[1].split("/")[1], decl)) add("I110", "warn", file, `generated from ${gen[1]}${gen[2] ? "@" + gen[2] : ""}: a copy that must be refreshed from another repository; make it a published artifact or own the source`, 1);
    }

    // env vars owned by another repository of the family
    if (code) {
      const owner = new Map<string, string>();
      for (const slug of Object.keys(scheme.repos)) {
        const id = slug.split("/")[1];
        if (id.startsWith(".") || id === decl.id || id === (decl.repo ?? decl.id) || (scheme.env.generic ?? []).includes(id)) continue;
        owner.set(envPrefixOf(id) + "_", id);
      }
      for (const m of text.matchAll(/(?:process\.env\.|Bun\.env\.|process\.env\[["']|env::var(?:_os)?\(\s*["']|os\.environ(?:\.get)?[\[(]\s*["']|os\.getenv\(\s*["']|getenv\(\s*["']|GetEnvironmentVariable\(\s*["']|os\.Getenv\(\s*["'])([A-Z][A-Z0-9_]+)/g)) {
        const v = m[1];
        if (exp.envPrefixes.some(p => v.startsWith(p))) continue;
        const aliased = (decl.legacy?.env ?? []).some(p => v.startsWith(p));
        const pre = v.split("_")[0];
        if (scheme.env.neutral.includes(pre)) continue;
        for (const [p, id] of owner) if (v.startsWith(p)) { add("I120", aliased ? "info" : "warn", file, `reads ${v}, a variable owned by ${id}: declare your own ${exp.envPrefixes[0]}* name and accept the other only as an alias`, lineOf(text, m.index!)); break; }
      }
    }
  }
  return out;
}

// ---- Loading a repository --------------------------------------------------------------------------------------

export async function gitFiles(dir: string): Promise<string[]> {
  const p = Bun.spawn(["git", "-C", dir, "ls-files", "-z", "--cached", "--others", "--exclude-standard"], { stdout: "pipe", stderr: "ignore" });
  const raw = await new Response(p.stdout).text();
  if (await p.exited) {
    const { Glob } = await import("bun");
    const found: string[] = [];
    for await (const f of new Glob("**/*").scan({ cwd: dir, onlyFiles: true, dot: true })) found.push(f.replace(/\\/g, "/"));
    return found;
  }
  return raw.split("\0").filter(Boolean);
}

export async function loadFiles(dir: string): Promise<Files> {
  const files: Files = new Map();
  await Promise.all(
    (await gitFiles(dir)).map(async f => {
      if (SKIP_FILE.test(f) && !/\.lock$/.test(f)) return;
      if (!(CODE.test(f) || CONFIG.test(f) || DOC.test(f) || /(^|\/)\.gitmodules$/.test(f) || /Dockerfile/.test(f))) return;
      const file = Bun.file(join(dir, f));
      try {
        if (file.size > 600_000) return;
        const text = await file.text();
        if (/^\s*(?:\/\/|#|<!--)\s*naming-ignore-file/m.test(text.slice(0, 300))) return;
        files.set(f, text);
      } catch {}
    }),
  );
  return files;
}

export async function loadDecl(dir: string, files: Files): Promise<Decl | undefined> {
  const text = files.get("naming.toml") ?? (await Bun.file(join(dir, "naming.toml")).text().catch(() => undefined));
  if (!text) return undefined;
  const t = Bun.TOML.parse(text) as any;
  const n = t.naming ?? t;
  const id = n.id ?? dir.split(/[\\/]/).filter(Boolean).pop()!;
  return { id, kind: n.kind, org: n.org, repo: n.repo, family: n.family, names: n.names, legacy: n.legacy, exceptions: n.exceptions };
}

/** Names found in a repository, for the inventory: packages, crates, binaries, modules, images, env prefixes. */
export function inventory(files: Files): Record<string, string[]> {
  const inv: Record<string, Set<string>> = { crates: new Set(), npm: new Set(), pypi: new Set(), go: new Set(), nuget: new Set(), docker: new Set(), cloudflare: new Set(), envPrefixes: new Set(), bins: new Set() };
  for (const [f, text] of files) {
    const base = f.split("/").pop()!;
    try {
      if (base === "Cargo.toml") {
        const t = Bun.TOML.parse(text) as any;
        if (t.package?.name) inv.crates.add(t.package.name);
        for (const b of t.bin ?? []) if (b.name) inv.bins.add(b.name);
      } else if (base === "package.json") {
        const p = JSON.parse(text);
        if (p.name) inv.npm.add(p.name);
        if (typeof p.bin === "object" && p.bin) Object.keys(p.bin).forEach(b => inv.bins.add(b));
        else if (typeof p.bin === "string" && p.name) inv.bins.add(p.name.replace(/^@[^/]+\//, ""));
      } else if (base === "pyproject.toml") {
        const n = (Bun.TOML.parse(text) as any).project?.name;
        if (n) inv.pypi.add(n);
      } else if (base === "go.mod") {
        const m = /^module\s+(\S+)/m.exec(text)?.[1];
        if (m) inv.go.add(m);
      } else if (/\.(cs|fs)proj$/.test(base)) {
        const m = /<PackageId>([^<]+)</.exec(text)?.[1] ?? /<AssemblyName>([^<]+)</.exec(text)?.[1];
        if (m) inv.nuget.add(m);
      } else if (/^wrangler\.(toml|jsonc?)$/.test(base)) {
        const w: any = base.endsWith(".toml") ? Bun.TOML.parse(text) : jsonc(text);
        if (w.name) inv.cloudflare.add(w.name);
      }
    } catch {}
    for (const m of text.matchAll(/ghcr\.io\/[\w.-]+\/[\w.-]+/g)) inv.docker.add(m[0]);
    if (CODE.test(f)) for (const m of text.matchAll(/(?:process\.env\.|Bun\.env\.|env::var\(\s*"|getenv\(\s*"|os\.environ\[")([A-Z][A-Z0-9]*)_[A-Z0-9_]+/g)) inv.envPrefixes.add(m[1]);
  }
  return Object.fromEntries(Object.entries(inv).map(([k, v]) => [k, [...v].sort()]));
}

export function summarize(findings: Finding[]) {
  const by: Record<string, number> = {};
  for (const f of findings) by[`${f.severity}:${f.rule}`] = (by[`${f.severity}:${f.rule}`] ?? 0) + 1;
  return by;
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const flag = (n: string) => args.includes(n);
  const val = (n: string) => (args.includes(n) ? args[args.indexOf(n) + 1] : undefined);
  const dir = resolve(args.find((a, i) => !a.startsWith("--") && args[i - 1] !== "--scheme" && args[i - 1] !== "--kind" && args[i - 1] !== "--org") ?? ".");
  const scheme = await loadScheme(val("--scheme"));
  const files = await loadFiles(dir);
  let decl = await loadDecl(dir, files);
  if (!decl) {
    const repo = dir.split(/[\\/]/).filter(Boolean).pop()!;
    const known = Object.entries(scheme.repos).find(([k]) => k.split("/")[1].toLowerCase() === repo.toLowerCase());
    decl = { id: repo, kind: val("--kind") ?? known?.[1] ?? "app", org: val("--org") ?? known?.[0].split("/")[0] ?? "aphrody-labs" };
  }
  if (flag("--inventory")) {
    console.log(JSON.stringify({ decl, ...inventory(files) }, null, 2));
    process.exit(0);
  }
  const findings = checkFiles(files, decl, scheme);
  if (flag("--json")) console.log(JSON.stringify({ decl, findings }, null, 2));
  else {
    const order = { error: 0, warn: 1, info: 2 };
    for (const f of findings.filter(f => flag("--all") || f.severity !== "info").sort((a, b) => order[a.severity] - order[b.severity] || a.file.localeCompare(b.file)))
      console.log(`${f.severity.padEnd(5)} ${f.rule} ${f.file}${f.line ? ":" + f.line : ""} ${f.message}`);
    const s = summarize(findings);
    console.log(`\n${decl.org}/${decl.repo ?? decl.id} (${decl.kind}): ${findings.filter(f => f.severity === "error").length} errors, ${findings.filter(f => f.severity === "warn").length} warnings, ${findings.filter(f => f.severity === "info").length} notes`);
    console.log(Object.entries(s).map(([k, v]) => `${k}=${v}`).join(" "));
  }
  process.exit(flag("--strict") && findings.some(f => f.severity === "error") ? 1 : 0);
}
