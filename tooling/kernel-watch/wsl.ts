// Records of the WSL2 kernel build and of the merge experiment, the generated docs/aphrody/watch/wsl.md page, and the rows they add to the versions table.
export interface WslBuildRecord {
  /** Record id, also the data/wsl file stem. */
  id: string;
  /** Tag of the Microsoft tree the build started from. */
  ref: string;
  commit: string;
  /** uname -r of the built kernel. */
  release: string;
  march: string;
  kcflags: string;
  compiler: string;
  buildSeconds: number;
  bzImageBytes: number;
  modulesVhdxBytes: number;
  bzImageSha256: string;
  modulesVhdxSha256: string;
  /** Epoch seconds when the artifacts were produced. */
  builtAt: number;
  /** Direct kernel boot in QEMU on the build host; null when not run. */
  boot: { accel: string; cpu: string; release: string; panic: boolean; seconds: number; excerpt: string[]; notExercised: string[] } | null;
  /** Where the artifacts were published, null when they only exist on the build host. */
  distribution: { bucket: string; prefix: string; files: string[]; publicUrl: string | null } | null;
}
export interface WslMergeRecord {
  msftTag: string;
  upstreamTag: string;
  mergeBase: string;
  mergeBaseDescribe: string;
  commitsOnMsftSinceBase: number;
  commitsOnUpstreamSinceBase: number;
  filesChangedByMsft: number;
  filesChangedByUpstream: number;
  filesChangedByBoth: number;
  mergeExitStatus: number;
  conflictedFiles: number;
  dxgkrnlFilesInMsftTree: number;
  dxgkrnlFilesInUpstream: number;
  dxgkrnlConflicts: number;
  dxgkrnlFilesTouchedByMsftSinceBase: number;
  conflicts: string[];
  /** Epoch seconds of the experiment. */
  at: number;
}

export interface WslRecords {
  build: WslBuildRecord | null;
  merge: WslMergeRecord | null;
}

export async function readWslRecords(watchDir: string): Promise<WslRecords> {
  const read = async <T>(name: string): Promise<T | null> => {
    const file = Bun.file(`${watchDir}/data/wsl/${name}`);
    return (await file.exists()) ? (JSON.parse(await file.text()) as T) : null;
  };
  return { build: await read<WslBuildRecord>("build.json"), merge: await read<WslMergeRecord>("merge.json") };
}

/** Versions-table row of our build, fed to computeVersions like a polled version. */
export const wslVersionRows = (r: WslRecords): { kernel: string; source: string; channel: string; version: string; since: number; checked: number }[] =>
  r.build ? [{ kernel: "wsl", source: "wsl-build", channel: `aphrody build (${r.build.march})`, version: r.build.release, since: r.build.builtAt, checked: r.build.builtAt }] : [];

const iso = (epoch: number): string => new Date(epoch * 1000).toISOString().replace(/:\d\d\.\d+Z$/, "Z");
const mib = (bytes: number): string => `${(bytes / 1048576).toFixed(1)} MiB`;
const minutes = (seconds: number): string => `${Math.round(seconds / 60)} min`;

export function renderWslPage(records: WslRecords, microsoftRelease: string | null, mainline: string | null): string {
  const lines = [
    "# WSL2 kernel", "",
    "Generated from `data/wsl/` and the watch store; do not edit. The build, the boot test and the merge experiment run on the build host (see [PLAN.md](PLAN.md)); the install script is `tooling/kernel-watch/wsl/install-wsl-kernel.ts`. Both kernels appear in [versions.md](versions.md).", "",
    "## Releases", "",
    `- Microsoft latest release: ${microsoftRelease ?? "not observed yet"}`,
    `- Mainline reference: ${mainline ?? "not observed yet"}`,
    "- Fork: aphrody-labs/WSL2-Linux-Kernel, default branch tracking the newest linux-msft-wsl-*.y branch of the Microsoft tree.", "",
  ];
  const { build, merge } = records;
  if (!build) lines.push("## Build", "", "No build has been recorded yet.", "");
  else {
    lines.push(
      "## Build", "",
      "| | |", "| :-- | :-- |",
      `| Release (uname -r) | ${build.release} |`,
      `| Source | ${build.ref}, commit ${build.commit.slice(0, 12)} |`,
      `| Config | Microsoft/config-wsl, LOCALVERSION suffix for the target CPU |`,
      `| Target CPU | ${build.march}${build.kcflags ? ` (KCFLAGS \`${build.kcflags}\`)` : ""} |`,
      `| Compiler | ${build.compiler} |`,
      `| Build time | ${minutes(build.buildSeconds)} |`,
      `| bzImage | ${mib(build.bzImageBytes)}, sha256 \`${build.bzImageSha256}\` |`,
      `| modules.vhdx | ${mib(build.modulesVhdxBytes)}, sha256 \`${build.modulesVhdxSha256}\` |`,
      `| Built | ${iso(build.builtAt)} |`, "",
    );
    if (build.boot) {
      lines.push(
        "## Boot test", "",
        `Direct kernel boot with a tiny initramfs in QEMU (${build.boot.accel}, cpu ${build.boot.cpu}) on the build host: guest \`uname -r\` ${build.boot.release}, ${build.boot.panic ? "a panic or oops was seen" : "no panic or oops in the log"}, ${build.boot.seconds}s.`, "",
        "```", ...build.boot.excerpt, "```", "",
        `Not exercised by this test (they need a real WSL host): ${build.boot.notExercised.join("; ")}.`, "",
      );
    }
    if (build.distribution) {
      const url = build.distribution.publicUrl;
      lines.push("## Distribution", "", `Stored in the R2 bucket ${build.distribution.bucket} under \`${build.distribution.prefix}\`: ${build.distribution.files.join(", ")}.${url ? ` Public base URL: ${url}` : " The bucket has no public URL configured for this prefix; fetch with the cf CLI or wrangler."}`, "");
    }
    lines.push(
      "## Install on Windows", "",
      "Nothing here changes a machine by itself. On the Windows side, with Bun:", "",
      "```",
      "bun install-wsl-kernel.ts --from <directory or base URL holding manifest.json>          # dry run: prints the .wslconfig it would write",
      "bun install-wsl-kernel.ts --from <directory or base URL holding manifest.json> --apply  # verifies sha256, backs up and edits .wslconfig",
      "wsl --shutdown",
      "```", "",
      "The line it sets, under `[wsl2]` in `%UserProfile%\\.wslconfig`: `kernel=<path>\\\\bzImage`, plus `kernelModules=<path>\\\\modules.vhdx` when the modules image is installed.", "",
    );
  }
  if (merge) {
    lines.push(
      "## Following mainline with Microsoft's patches", "",
      `Experiment of ${iso(merge.at)}: \`git merge-tree\` of ${merge.upstreamTag} into ${merge.msftTag} in a throwaway blob-less repository (no tree was changed).`, "",
      "| | |", "| :-- | :-- |",
      `| Merge base | ${merge.mergeBaseDescribe} |`,
      `| Commits on the Microsoft side since the base | ${merge.commitsOnMsftSinceBase} |`,
      `| Commits on the upstream side since the base | ${merge.commitsOnUpstreamSinceBase} |`,
      `| Files changed by Microsoft / by upstream / by both | ${merge.filesChangedByMsft} / ${merge.filesChangedByUpstream} / ${merge.filesChangedByBoth} |`,
      `| Conflicted files | ${merge.conflictedFiles} |`,
      `| dxgkrnl (drivers/hv/dxgkrnl) files in the Microsoft tree / upstream | ${merge.dxgkrnlFilesInMsftTree} / ${merge.dxgkrnlFilesInUpstream} |`,
      `| dxgkrnl files conflicting | ${merge.dxgkrnlConflicts} |`, "",
    );
    if (merge.conflicts.length) lines.push("Conflicted files:", "", ...merge.conflicts.map((f) => `- ${f}`), "");
    else lines.push("The merge applied without textual conflicts. That says nothing about whether the result builds or boots.", "");
  }
  return `${lines.join("\n")}\n`;
}

const NOT_EXERCISED = ["hv_sock and the Hyper-V enlightenments (no Hyper-V host)", "dxgkrnl and /dev/dxg (WSLg GPU paravirtualisation needs a Windows host)", "WSLg, virtiofs shares and the WSL init"];

/**
 * Reads the newest build, its boot test and the merge experiment from the build host's WSL root (read only, over ssh) and returns the records.
 * The host side layout is written by wsl/build.sh, wsl/boot-test.sh and wsl/merge-test.sh.
 */
export async function fetchWslRecords(ssh: string, root: string): Promise<WslRecords> {
  const script = `
cd "$1" || exit 1
b=$(ls -td build/*/ 2>/dev/null | head -1); b=\${b%/}
[ -n "$b" ] && [ -f "$b/build.json" ] && { echo "@@build"; cat "$b/build.json"; echo "@@built"; stat -c %Y "$b/build.json"; }
for a in kvm tcg; do [ -f "$b/boot-$a.log" ] && { echo "@@boot-$a"; grep -aE "Linux version|WSLTEST|Kernel panic|Oops" "$b/boot-$a.log" | head -12; echo "@@meta-$a"; cat "$b/boot-$a.meta"; }; done
m=$(ls -td merge/*/ 2>/dev/null | head -1); m=\${m%/}
[ -n "$m" ] && [ -f "$m/report.json" ] && { echo "@@merge"; cat "$m/report.json"; echo "@@conflicts"; cat "$m/delta-conflicts.txt"; echo "@@merged"; stat -c %Y "$m/report.json"; }
echo "@@end"
`;
  const proc = Bun.spawn(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=20", ssh, "sh", "-s", "--", root], { stdin: new TextEncoder().encode(script), stdout: "pipe", stderr: "pipe" });
  const [out, err, code] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
  if (code !== 0) throw new Error(`ssh ${ssh}: ${err.trim().slice(0, 300)}`);
  return parseWslSections(out);
}

export function parseWslSections(out: string): WslRecords {
  const sections = new Map<string, string>();
  let current = "";
  for (const line of out.replace(/\r/g, "").split("\n")) {
    const mark = /^@@([\w-]+)$/.exec(line);
    if (mark) (current = mark[1]!, sections.set(current, ""));
    else if (current) sections.set(current, `${sections.get(current)}${line}\n`);
  }
  const json = <T>(name: string): T | null => (sections.has(name) ? (JSON.parse(sections.get(name)!) as T) : null);
  const num = (name: string): number => Number(sections.get(name)?.trim() ?? 0);

  let build: WslBuildRecord | null = null;
  const b = json<Record<string, string | number>>("build");
  if (b) {
    const accel = sections.has("boot-kvm") ? "kvm" : sections.has("boot-tcg") ? "tcg" : null;
    let boot: WslBuildRecord["boot"] = null;
    if (accel) {
      const log = sections.get(`boot-${accel}`)!.split("\n").filter(Boolean);
      const field = (key: string): string => log.find((l) => l.startsWith(`WSLTEST ${key} `))?.slice(`WSLTEST ${key} `.length) ?? "";
      const meta = /seconds (\d+)/.exec(sections.get(`meta-${accel}`) ?? "");
      boot = {
        accel, cpu: accel === "kvm" ? "host" : "max", release: field("uname"), panic: log.some((l) => /Kernel panic|Oops/.test(l)) || Number(field("bugs")) > 0,
        seconds: meta ? Number(meta[1]) : 0, excerpt: log.map((l) => l.replace(/\(\S+@\S+\) /, "")), notExercised: NOT_EXERCISED,
      };
    }
    build = {
      id: "aphrody-wsl", ref: String(b.ref), commit: String(b.commit), release: String(b.release), march: String(b.march), kcflags: String(b.kcflags), compiler: String(b.compiler),
      buildSeconds: Number(b.buildSeconds), bzImageBytes: Number(b.bzImageBytes), modulesVhdxBytes: Number(b.modulesVhdxBytes), bzImageSha256: String(b.bzImageSha256),
      modulesVhdxSha256: String(b.modulesVhdxSha256), builtAt: num("built"), boot, distribution: null,
    };
  }
  let merge: WslMergeRecord | null = null;
  const m = json<any>("merge");
  if (m) {
    merge = {
      msftTag: m.msftTag, upstreamTag: m.upstreamTag, mergeBase: m.mergeBase, mergeBaseDescribe: `${m.stableTag} lineage, merge base ${String(m.mergeBase).slice(0, 12)}`,
      commitsOnMsftSinceBase: m.commitsOnMsftSinceBase, commitsOnUpstreamSinceBase: m.commitsOnUpstreamSinceBase, filesChangedByMsft: m.delta.filesChangedByMsft,
      filesChangedByUpstream: m.delta.filesChangedByUpstreamSinceStable, filesChangedByBoth: m.delta.filesChangedByBoth, mergeExitStatus: m.delta.mergeExitStatus,
      conflictedFiles: m.delta.conflictedFiles, dxgkrnlFilesInMsftTree: m.delta.dxgkrnlFilesInMsftTree, dxgkrnlFilesInUpstream: m.delta.dxgkrnlFilesInUpstream,
      dxgkrnlConflicts: m.delta.dxgkrnlConflicts, dxgkrnlFilesTouchedByMsftSinceBase: m.delta.dxgkrnlFilesInMsftTree,
      conflicts: (sections.get("conflicts") ?? "").split("\n").filter(Boolean), at: num("merged"),
    };
  }
  return { build, merge };
}
