#!/usr/bin/env bun
// Build-host side of the rc lane: build or fetch each kernel, boot it in QEMU/KVM against an Arch root filesystem, run the smoke suite
// and write one record per kernel. Run on the build host only (docker, qemu and /dev/kvm are needed); the PC never builds.
//   bun lane-host.ts prepare       --root DIR   pull the image, build the Arch base root filesystem, build the kselftest subset
//   bun lane-host.ts run           --root DIR [--ids a,b] [--if-new]   build or fetch, assemble, boot and record every lane kernel
//   bun lane-host.ts boot <id>     --root DIR   assemble and boot the kernel already built or fetched
//   bun lane-host.ts status        --root DIR   latest record per kernel
//   bun lane-host.ts install-timer --root DIR   user systemd timer running "run --if-new" on the lanes.toml schedule
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { dmesgFrom, finalizeChecks, judgeKselftest, laneSummary, overall, parseSerial, type LaneRecord } from "./lane.ts";
import { loadLanes, type LaneConfig, type LaneKernel, type Lanes } from "./lanes.ts";
import { parseArchApi, parseCachyosIndex } from "./probe.ts";
import { compareVersions, parseVersion } from "./version.ts";

const HERE = import.meta.dir;
const SCRIPTS = join(HERE, "lane");
const now = (): number => Math.floor(Date.now() / 1000);

class Lane {
  readonly cache: string;
  readonly out: string;
  readonly work: string;
  readonly logs: string;
  readonly records: string;
  readonly rootfs: string;
  readonly state: string;
  constructor(readonly root: string, readonly lanes: Lanes) {
    this.cache = join(root, "cache");
    this.out = join(root, "out");
    this.work = join(root, "work");
    this.logs = join(root, "logs");
    this.records = join(root, "records");
    this.rootfs = join(root, "rootfs");
    this.state = join(root, "state");
    for (const dir of [this.cache, join(this.cache, "pacman"), this.out, this.work, this.logs, this.records, this.rootfs, this.state]) mkdirSync(dir, { recursive: true });
  }
  get config(): LaneConfig {
    return this.lanes.lane;
  }
  log(...parts: unknown[]): void {
    console.log(`[${new Date().toISOString().slice(11, 19)}]`, ...parts);
  }

  /** Runs a script of lane/ in a container of the configured image; the root is /lane, the scripts are /scripts, the pacman cache is shared. */
  async docker(name: string, script: string, env: Record<string, string>, opts: { privileged?: boolean; timeout: number }): Promise<void> {
    const envArgs = Object.entries(env).flatMap(([k, v]) => ["-e", `${k}=${v}`]);
    const args = [
      "run", "--rm", "--name", `lane-${name}`, ...(opts.privileged ? ["--privileged"] : []),
      "-v", `${this.root}:/lane`, "-v", `${SCRIPTS}:/scripts:ro`, "-v", `${join(this.cache, "pacman")}:/var/cache/pacman/pkg`,
      "-e", `TESTING_REPOS=${this.config.testingRepos.join(" ")}`, ...envArgs, this.config.image, "bash", `/scripts/${script}`,
    ];
    const logFile = join(this.logs, `${name}.log`);
    const proc = Bun.spawn(["docker", ...args], { stdout: Bun.file(logFile), stderr: Bun.file(logFile), timeout: opts.timeout * 1000 });
    const code = await proc.exited;
    if (code !== 0) throw new Error(`container ${name} exited ${code}; see ${logFile}\n${readFileSync(logFile, "utf8").split("\n").slice(-12).join("\n")}`);
  }

  async fetchText(url: string): Promise<string> {
    const res = await fetch(url, { headers: { "user-agent": "kernel-watch-lane" }, signal: AbortSignal.timeout(60_000) });
    if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
    return res.text();
  }

  async prepare(force = false): Promise<void> {
    const marker = join(this.state, "rootfs.json");
    const age = existsSync(marker) ? now() - JSON.parse(readFileSync(marker, "utf8")).at : Infinity;
    if (!force && existsSync(join(this.rootfs, "base")) && age < this.config.refreshRootfsDays * 86400) return;
    this.log("pulling", this.config.image);
    const pull = Bun.spawnSync(["docker", "pull", "-q", this.config.image]);
    if (pull.exitCode !== 0) throw new Error(`docker pull: ${pull.stderr.toString()}`);
    this.log("building the base root filesystem");
    await this.docker("rootfs", "rootfs.sh", {}, { privileged: true, timeout: 1800 });
    writeFileSync(marker, JSON.stringify({ at: now() }));
  }

  /** Kernel source mirror: a bare shallow repository holding exactly the tag being built. */
  mirrorTag(tag: string): void {
    const bare = join(this.cache, "linux.git");
    if (!existsSync(bare)) Bun.spawnSync(["git", "init", "-q", "--bare", bare]);
    const have = Bun.spawnSync(["git", "-C", bare, "rev-parse", "-q", "--verify", `refs/tags/${tag}`]);
    if (have.exitCode === 0) return;
    const fetch = Bun.spawnSync(["git", "-C", bare, "fetch", "-q", "--depth", "1", this.config.kernelGit, "tag", tag]);
    if (fetch.exitCode !== 0) throw new Error(`git fetch ${tag}: ${fetch.stderr.toString()}`);
  }

  async ensureKselftest(tag: string): Promise<void> {
    const marker = join(this.state, "kselftest.json");
    const targets = this.config.kselftestTargets.join(" ");
    if (existsSync(marker)) {
      const held = JSON.parse(readFileSync(marker, "utf8"));
      if (held.tag === tag && held.targets === targets && existsSync(join(this.work, "kselftest", "run_kselftest.sh"))) return;
    }
    this.mirrorTag(tag);
    this.log("building the kselftest subset from", tag);
    await this.docker("kselftest", "kselftest.sh", { KERNEL_MIRROR: "/lane/cache/linux.git", KERNEL_TAG: tag, TARGETS: this.config.kselftestTargets.join(" ") }, { timeout: 1800 });
    writeFileSync(marker, JSON.stringify({ tag, targets }));
  }

  /** The AUR PKGBUILD's tag and version, read from a throwaway clone on the host. */
  aurPlan(k: LaneKernel): { tag: string; version: string; commit: string; file: string } {
    const dir = join(this.cache, `aur-${k.package}`);
    rmSync(dir, { recursive: true, force: true });
    const clone = Bun.spawnSync(["git", "clone", "-q", "--depth", "1", this.config.aurGit, dir]);
    if (clone.exitCode !== 0) throw new Error(`git clone ${this.config.aurGit}: ${clone.stderr.toString()}`);
    const pkgbuild = readFileSync(join(dir, "PKGBUILD"), "utf8");
    const field = (name: string): string => new RegExp(`^${name}=(\\S+)`, "m").exec(pkgbuild)?.[1] ?? "";
    const tag = field("_tag").replace(/^["']|["']$/g, "");
    const pkgver = field("pkgver");
    const pkgrel = field("pkgrel");
    if (!tag || !pkgver || !pkgrel) throw new Error("could not read _tag, pkgver and pkgrel from the PKGBUILD");
    const commit = Bun.spawnSync(["git", "-C", dir, "rev-parse", "HEAD"]).stdout.toString().trim();
    rmSync(dir, { recursive: true, force: true });
    return { tag, version: `${pkgver}-${pkgrel}`, commit, file: `${k.package}-${pkgver}-${pkgrel}-x86_64.pkg.tar.zst` };
  }

  cpuLevel(): number {
    const flags = new Set((/^flags\s*:\s*(.*)$/m.exec(readFileSync("/proc/cpuinfo", "utf8"))?.[1] ?? "").split(" "));
    if (["avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl"].every((f) => flags.has(f))) return 4;
    if (["avx2", "bmi1", "bmi2", "fma", "movbe", "f16c"].every((f) => flags.has(f))) return 3;
    return 2;
  }

  /** The newest package of k among the CachyOS repositories this CPU can run. */
  async cachyosPlan(k: LaneKernel): Promise<{ repo: string; file: string; url: string; version: string }> {
    const level = this.cpuLevel();
    let pick: { repo: string; file: string; url: string; version: string } | null = null;
    for (const id of k.repos) {
      const need = /znver4|-v4$/.test(id) ? 4 : /-v3$/.test(id) ? 3 : 2;
      if (need > level) continue;
      const source = this.lanes.probe.cachyos.find((c) => c.id === id);
      if (!source) throw new Error(`lane kernel ${k.id} names unknown CachyOS repository ${id}`);
      for (const c of parseCachyosIndex(await this.fetchText(source.url), id, [k.package], source.url)) {
        const parsed = parseVersion(c.normalized)!;
        const held = pick ? parseVersion(pick.version)! : null;
        if (!pick || compareVersions(parsed, held!) > 0 || (compareVersions(parsed, held!) === 0 && Number(c.version.split("-").pop()) > Number(pick.version.split("-").pop()))) {
          pick = { repo: id, file: c.url!.split("/").pop()!, url: c.url!, version: c.version };
        }
      }
    }
    if (!pick) throw new Error(`no ${k.package} package in ${k.repos.join(", ")} for a CPU of x86-64-v${level}`);
    return pick;
  }

  async archRepoVersion(k: LaneKernel): Promise<string> {
    const rows = parseArchApi(await this.fetchText(`${this.lanes.probe.archApi}?name=${encodeURIComponent(k.package)}`)).filter((c) => c.origin === "repo" && c.name === k.package);
    if (!rows[0]) throw new Error(`${k.package} not found in the Arch repositories`);
    return rows[0].version;
  }

  /** Resolves where the kernel comes from, fetching or building the package; returns what the assemble step needs. */
  async acquire(k: LaneKernel): Promise<{ source: "repo" | "file" | "tree"; version: string; origin: string; buildSeconds: number | null }> {
    if (k.source === "repo") return { source: "repo", version: await this.archRepoVersion(k), origin: `Arch repository (pacman -S ${k.package} in the guest root filesystem)`, buildSeconds: null };
    if (k.source === "tree") {
      // The staged kernel (kconfig/stage.sh) lives in out/<id>; this lane only boots it.
      const staged = join(this.out, k.id);
      if (!existsSync(join(staged, "bzImage"))) throw new Error(`no staged kernel in ${staged}; build the tree and run kconfig/stage.sh first`);
      const stage = join(this.work, k.id, "pkgs");
      rmSync(stage, { recursive: true, force: true });
      mkdirSync(dirname(stage), { recursive: true });
      Bun.spawnSync(["cp", "-a", "--reflink=auto", staged, stage]);
      const release = readFileSync(join(staged, "release"), "utf8").trim();
      const source = readFileSync(join(staged, "source"), "utf8").trim();
      return { source: "tree", version: release, origin: `tree ${source} built with LLVM=1 and the config fragments of kconfig/fragments`, buildSeconds: null };
    }
    const pkgDir = join(this.out, k.package);
    mkdirSync(pkgDir, { recursive: true });
    const stage = join(this.work, k.id, "pkgs");
    const finish = (file: string): void => {
      this.stage(stage, join(pkgDir, file));
    };
    if (k.source === "cachyos") {
      const plan = await this.cachyosPlan(k);
      const target = join(pkgDir, plan.file);
      if (!existsSync(target)) {
        this.log("downloading", plan.url);
        const res = await fetch(plan.url, { headers: { "user-agent": "kernel-watch-lane" } });
        if (!res.ok) throw new Error(`${plan.url}: HTTP ${res.status}`);
        await Bun.write(target, res);
      }
      finish(plan.file);
      return { source: "file", version: plan.version, origin: `CachyOS repository ${plan.repo}: ${plan.file}`, buildSeconds: null };
    }
    const plan = this.aurPlan(k);
    const target = join(pkgDir, plan.file);
    const buildState = join(this.state, `${k.id}-${plan.version}.json`);
    if (!existsSync(target)) {
      this.mirrorTag(plan.tag);
      this.log("building", plan.file, "from tag", plan.tag);
      const started = now();
      await this.docker(`build-${k.id}`, "build-arch.sh", {
        AUR_PACKAGE: k.package, AUR_GIT: this.config.aurGit, KERNEL_MIRROR: "/lane/cache/linux.git", BUILD_UID: String(process.getuid?.() ?? 1000),
        JOBS: String(Math.max(1, Math.min(Number(Bun.env.LANE_JOBS ?? 10), navigator.hardwareConcurrency))),
      }, { timeout: this.config.buildTimeout });
      writeFileSync(buildState, JSON.stringify({ seconds: now() - started, commit: plan.commit, tag: plan.tag }));
    }
    finish(plan.file);
    const built = existsSync(buildState) ? JSON.parse(readFileSync(buildState, "utf8")) : null;
    return { source: "file", version: plan.version, origin: `AUR ${k.package} commit ${plan.commit.slice(0, 12)} built from tag ${plan.tag} in ${this.config.image} with ${this.config.testingRepos.join(" and ")} enabled`, buildSeconds: built?.seconds ?? null };
  }

  /** Copies the package into the per-kernel staging directory the assemble container reads (replacing any earlier one). */
  stage(dir: string, file: string): void {
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    Bun.spawnSync(["cp", file, dir]);
  }

  async assemble(k: LaneKernel, source: "repo" | "file" | "tree"): Promise<void> {
    this.log("assembling", k.id);
    await this.docker(`assemble-${k.id}`, "assemble.sh", { ID: k.id, SOURCE: source, PACKAGE: k.package, DISK_GB: String(this.config.diskGb), HOST_UID: String(process.getuid?.() ?? 1000) }, { privileged: true, timeout: 1800 });
  }

  async boot(k: LaneKernel): Promise<{ serial: string; seconds: number; timedOut: boolean }> {
    const dir = join(this.work, k.id);
    const serialPath = join(dir, "serial.log");
    const lanePath = join(dir, "lane.log");
    rmSync(serialPath, { force: true });
    rmSync(lanePath, { force: true });
    const c = this.config;
    const args = [
      "qemu-system-x86_64", "-enable-kvm", "-cpu", "host", "-smp", String(c.vmCpus), "-m", String(c.vmMemoryMb), "-display", "none", "-monitor", "none", "-no-reboot",
      "-kernel", join(dir, "vmlinuz"), "-initrd", join(dir, "initramfs.img"), "-append", "root=/dev/vda rw console=ttyS0,115200 loglevel=6",
      "-drive", `file=${join(dir, "root.img")},if=virtio,format=raw`, "-netdev", "user,id=n0", "-device", "virtio-net-pci,netdev=n0",
      "-object", "rng-random,id=rng0,filename=/dev/urandom", "-device", "virtio-rng-pci,rng=rng0", "-serial", `file:${serialPath}`,
      "-chardev", `file,id=lane,path=${lanePath}`, "-device", "isa-serial,chardev=lane",
    ];
    this.log("booting", k.id);
    const started = Date.now();
    const proc = Bun.spawn(args, { stdout: "ignore", stderr: "pipe", timeout: c.bootTimeout * 1000 });
    const code = await proc.exited;
    const seconds = Math.round((Date.now() - started) / 1000);
    const consoleLog = existsSync(serialPath) ? readFileSync(serialPath, "utf8") : "";
    const laneLog = existsSync(lanePath) ? readFileSync(lanePath, "utf8") : "";
    // The console repeats the guest's own lines; only its other lines (kernel banner, login prompt) are added to the clean log.
    const serial = `${laneLog}\n${consoleLog.split("\n").filter((l) => !l.includes("LANE-")).join("\n")}`;
    const timedOut = seconds >= c.bootTimeout - 1 && !laneLog.includes("LANE-END");
    if (code !== 0 && !consoleLog) throw new Error(`qemu exited ${code}: ${await new Response(proc.stderr).text()}`);
    return { serial, seconds, timedOut };
  }

  latest(id: string): LaneRecord | null {
    const file = join(this.records, id, "latest.json");
    return existsSync(file) ? (JSON.parse(readFileSync(file, "utf8")) as LaneRecord) : null;
  }

  async runKernel(k: LaneKernel, ifNew: boolean): Promise<LaneRecord | null> {
    const dir = join(this.work, k.id);
    mkdirSync(dir, { recursive: true });
    const acquired = await this.acquire(k);
    const previous = this.latest(k.id);
    if (ifNew && previous && previous.version === acquired.version && previous.status === "pass") {
      this.log(k.id, "already tested at", acquired.version);
      return null;
    }
    await this.assemble(k, acquired.source);
    const release = readFileSync(join(dir, "kernel-release"), "utf8").trim();
    const [pkgName, pkgVersion] = readFileSync(join(dir, "kernel-package"), "utf8").trim().split(" ");
    const { serial, seconds, timedOut } = await this.boot(k);
    writeFileSync(join(this.logs, `serial-${k.id}.log`), serial);
    const parsed = parseSerial(serial);
    const dmesg = dmesgFrom(parsed.metrics);
    const baseline = k.id === this.baselineId() ? null : this.latest(this.baselineId());
    const reference = previous?.metrics?.["kselftest-notok"] !== undefined ? { against: `previous run of ${k.id}`, notOk: previous.metrics["kselftest-notok"]! } : baseline?.metrics?.["kselftest-notok"] !== undefined ? { against: `${baseline.name} ${baseline.version}`, notOk: baseline.metrics["kselftest-notok"]! } : null;
    const checks = judgeKselftest(finalizeChecks(parsed, timedOut), parsed.metrics, parsed.kselftestFailures, reference);
    const against = previous?.dmesg ? { against: `previous run of ${k.id} (${previous.version})`, counts: previous.dmesg } : baseline?.dmesg ? { against: `${baseline.name} ${baseline.version}`, counts: baseline.dmesg } : null;
    const record: LaneRecord = {
      id: k.id, name: k.name, kernel: k.kernel, channel: k.channel, source: k.source, package: pkgName ?? k.package, version: pkgVersion ?? acquired.version,
      release: parsed.release ?? release, at: now(), buildSeconds: acquired.buildSeconds, bootSeconds: seconds, origin: acquired.origin,
      testing: parsed.testing, checks, metrics: parsed.metrics, kselftestFailures: parsed.kselftestFailures, dmesg, dmesgPrevious: against, status: overall(checks), serial: parsed.excerpt,
    };
    const outDir = join(this.records, k.id);
    mkdirSync(outDir, { recursive: true });
    writeFileSync(join(outDir, `${record.at}.json`), `${JSON.stringify(record, null, 2)}\n`);
    writeFileSync(join(outDir, "latest.json"), JSON.stringify(record));
    this.log(k.id, laneSummary(record), record.release);
    return record;
  }

  baselineId(): string {
    return this.config.kernels.find((x) => x.source === "repo")?.id ?? "";
  }
}

function parseArgs(argv: string[]): { command: string; rest: string[]; flags: Map<string, string> } {
  const flags = new Map<string, string>();
  const positional: string[] = [];
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]!;
    if (!arg.startsWith("--")) positional.push(arg);
    else if (argv[i + 1] === undefined || argv[i + 1]!.startsWith("--")) flags.set(arg.slice(2), "true");
    else (flags.set(arg.slice(2), argv[i + 1]!), i++);
  }
  return { command: positional[0] ?? "", rest: positional.slice(1), flags };
}

async function installTimer(lane: Lane): Promise<void> {
  const dir = join(Bun.env.HOME ?? "", ".config", "systemd", "user");
  mkdirSync(dir, { recursive: true });
  const bun = process.execPath;
  const repo = resolve(HERE, "..", "..");
  writeFileSync(join(dir, "kernel-lane.service"), [
    "[Unit]", "Description=rc kernel lane: build or fetch, boot in QEMU and smoke-test the rc kernels", "",
    "[Service]", "Type=oneshot", `ExecStartPre=-/usr/bin/git -C ${repo} pull -q --ff-only`, `ExecStart=${bun} ${join(HERE, "lane-host.ts")} run --root ${lane.root} --if-new`, "TimeoutStartSec=8h", "",
  ].join("\n"));
  writeFileSync(join(dir, "kernel-lane.timer"), ["[Unit]", "Description=rc kernel lane schedule", "", "[Timer]", `OnCalendar=${lane.config.timer}`, "Persistent=true", "RandomizedDelaySec=300", "", "[Install]", "WantedBy=timers.target", ""].join("\n"));
  for (const args of [["daemon-reload"], ["enable", "--now", "kernel-lane.timer"]]) {
    const r = Bun.spawnSync(["systemctl", "--user", ...args]);
    if (r.exitCode !== 0) throw new Error(`systemctl --user ${args.join(" ")}: ${r.stderr.toString()}`);
  }
  console.log(Bun.spawnSync(["systemctl", "--user", "list-timers", "kernel-lane.timer", "--no-pager"]).stdout.toString());
}

async function main(): Promise<number> {
  const { command, rest, flags } = parseArgs(Bun.argv.slice(2));
  const root = flags.get("root");
  if (!command || !root) {
    console.error("usage: bun lane-host.ts <prepare|run|boot <id>|status|install-timer> --root DIR [--ids a,b] [--if-new] [--refresh] [--lanes PATH]");
    return 2;
  }
  const lanes = await loadLanes(resolve(flags.get("lanes") ?? join(HERE, "lanes.toml")));
  const lane = new Lane(resolve(root), lanes);
  const wanted = flags.get("ids")?.split(",");
  const kernels = lanes.lane.kernels.filter((k) => !wanted || wanted.includes(k.id));
  switch (command) {
    case "prepare":
      await lane.prepare(flags.has("refresh"));
      return 0;
    case "status":
      for (const k of lanes.lane.kernels) {
        const r = lane.latest(k.id);
        console.log(`${k.id.padEnd(16)} ${r ? `${laneSummary(r)} ${r.package} ${r.version} ${r.release}` : "no record"}`);
      }
      return 0;
    case "install-timer":
      await installTimer(lane);
      return 0;
    case "boot":
    case "run": {
      const lock = join(lane.root, "lane.lock");
      try {
        mkdirSync(lock);
      } catch {
        throw new Error(`another lane run holds ${lock}; remove it if no run is active`);
      }
      try {
        await lane.prepare(flags.has("refresh"));
        const list = command === "boot" ? kernels.filter((k) => k.id === rest[0]) : kernels;
        if (list.length === 0) throw new Error("no lane kernel selected");
        const tag = (await Promise.all(lanes.lane.kernels.filter((k) => k.source === "aur").map((k) => lane.aurPlan(k).tag)))[0];
        if (tag) await lane.ensureKselftest(tag);
        let failed = 0;
        for (const k of list) {
          try {
            const r = await lane.runKernel(k, flags.has("if-new"));
            if (r?.status === "fail") failed++;
          } catch (err) {
            failed++;
            console.error(`${k.id}: ${err instanceof Error ? err.message : String(err)}`);
          }
        }
        return failed ? 1 : 0;
      } finally {
        rmSync(lock, { recursive: true, force: true });
      }
    }
    default:
      console.error(`unknown command ${command}`);
      return 2;
  }
}

if (import.meta.main) process.exit(await main().catch((err) => (console.error(err instanceof Error ? err.message : err), 1)));
