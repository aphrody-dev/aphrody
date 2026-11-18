// Private terms of the machine running this, read at run time and never written down (see private.ts): the
// matchers the gate uses, and redact() for generated files that quote other repositories' text.
import { existsSync, readFileSync } from "node:fs";
import { homedir, hostname, userInfo } from "node:os";
import { dirname, join } from "node:path";

export function privateMatchers(root: string, fleetFile?: string) {
  const a = { fleet: fleetFile };
  const sh = (cmd: string[]) => Bun.spawnSync(cmd, { cwd: root, stdout: "pipe", stderr: "ignore" }).stdout.toString().trim();
  const terms = new Map<string, { kind: string; weak: boolean }>();
  function add(value: string | undefined, kind: string, weakIfBare = false) {
    const v = value?.trim();
    if (!v || v.length < 3 || /^(localhost|root|user|admin|ubuntu|github\.com)$/i.test(v)) return;
    terms.set(v, { kind, weak: weakIfBare && /^[\w-]+$/.test(v) });
  }
  add(userInfo().username, "user name", true);
  add(homedir(), "home directory");
  add(homedir().replaceAll("\\", "/"), "home directory");
  for (let d = root; dirname(d) !== d; d = dirname(d)) {
    add(d, "local folder");
    add(d.replaceAll("\\", "/"), "local folder");
  }
  add(hostname(), "host name", true);
  add(sh(["git", "config", "user.email"]), "git e-mail");
  add(sh(["git", "config", "--global", "user.email"]), "git e-mail");
  const sshConfig = join(homedir(), ".ssh", "config");
  if (existsSync(sshConfig))
    for (const m of readFileSync(sshConfig, "utf8").matchAll(/^\s*(Host|HostName)\s+(.+)$/gim))
      for (const h of m[2]!.split(/\s+/)) if (!h.includes("*")) add(h, m[1] === "Host" ? "ssh alias" : "ssh host", true);
  if (a.fleet) {
    const fleet = Bun.TOML.parse(readFileSync(a.fleet, "utf8")) as { nodes?: Record<string, Record<string, unknown>> };
    for (const [name, n] of Object.entries(fleet.nodes ?? {})) {
      add(name, "fleet node", true);
      for (const k of ["public_ip", "mesh_ip", "ssh_alias", "host", "hostname"]) if (typeof n[k] === "string") add(n[k] as string, `fleet ${k}`, true);
    }
  }
  const networks = new Set([...terms].filter(([k, t]) => /^fleet .*ip$/.test(t.kind) && /^\d+\.\d+\.\d+\.\d+$/.test(k)).map(([k]) => `${k.split(".").slice(0, 3).join(".")}.`));
  
  const origin = sh(["git", "remote", "get-url", "origin"]).toLowerCase();
  const publicNames = new Set(origin.replace(/\.git$/, "").split(/[/:]/).slice(-2).flatMap((p) => [p, ...p.split(/[-_.]/)]).filter(Boolean));
  for (const [k, t] of terms) if (t.weak && (Bun.which(k) || publicNames.has(k.toLowerCase()) || k !== k.toLowerCase())) terms.delete(k);
  
  
  const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const alt = (xs: string[]) => (xs.length ? [...xs].sort((x, y) => y.length - x.length).map(esc).join("|") : "(?!)");
  const strong = alt([...terms].filter(([, t]) => !t.weak).map(([k]) => k));
  const weak = alt([...terms].filter(([, t]) => t.weak).map(([k]) => k));
  const end = String.raw`(?![\w-])`;
  const strongRe = new RegExp(String.raw`(?<![\w.-])(` + strong + ")" + end, "gi");
  const weakRe = new RegExp(
    [
      String.raw`\b(?:ssh|scp|rsync|sftp|mosh)\s+(?:-\S+\s+)*(?:[\w.-]+@)?(` + weak + ")" + end,
      String.raw`(?<![\w.-])@(` + weak + ")" + end,
      String.raw`\b(?:host|hosts|hostname|node|server|ssh_alias|alias)\w*["']?\s*[:=]\s*\[?\s*["']?(` + weak + ")" + end,
      String.raw`--(?:host|node|server)[= ](` + weak + ")" + end,
      "[\"'`](" + weak + ")[\"'`]",
      String.raw`\b(?:on|to|from|via|at)\s+(` + weak + ")" + end,
      String.raw`(?<=\s)\((` + weak + String.raw`)(?=[)\s;,])`,
      String.raw`(?<![\w.])(` + alt([...networks]) + String.raw`)(?=\d)`,
    ].join("|"),
    "g",
  );
  const kindOf = (v: string) => terms.get(v)?.kind ?? [...terms].find(([k]) => k.toLowerCase() === v.toLowerCase())?.[1].kind ?? "fleet network";
  
  
  // replace each private value (only the captured value, not its context) with a neutral placeholder
  const redact = (text: string) =>
    text
      .replace(strongRe, () => "<private>")
      .replace(weakRe, (m: string, ...groups: unknown[]) => {
        const v = groups.find((g) => typeof g === "string" && g) as string | undefined;
        return v ? m.replace(v, "<host>") : m;
      });
  return { terms, strongRe, weakRe, kindOf, redact };
}
