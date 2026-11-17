import { describe, expect, test } from "bun:test";
import { parseWslSections, renderWslPage, wslVersionRows } from "./wsl.ts";

const sections = [
  "@@build", JSON.stringify({ ref: "linux-msft-wsl-6.18.1.1", commit: "a".repeat(40), release: "6.18.1.1-microsoft-standard-WSL2-example+", march: "example", kcflags: "-march=example", compiler: "gcc 15", buildSeconds: 600, bzImageBytes: 1048576, modulesVhdxBytes: 2097152, bzImageSha256: "b".repeat(64), modulesVhdxSha256: "c".repeat(64) }),
  "@@built", "1790000000",
  "@@boot-kvm", "[    0.000000] Linux version 6.18.1.1-microsoft-standard-WSL2-example+ (gcc 15) #1", "WSLTEST uname 6.18.1.1-microsoft-standard-WSL2-example+", "WSLTEST bugs 0", "WSLTEST end",
  "@@meta-kvm", "accel kvm seconds 3",
  "@@merge", JSON.stringify({ msftTag: "linux-msft-wsl-6.18.1.1", stableTag: "v6.18.1", upstreamTag: "v7.4-rc1", mergeBase: "d".repeat(40), commitsOnMsftSinceBase: 10, commitsOnUpstreamSinceBase: 20, delta: { filesChangedByMsft: 5, filesChangedByUpstreamSinceStable: 50, filesChangedByBoth: 2, mergeExitStatus: 1, conflictedFiles: 1, dxgkrnlFilesInMsftTree: 15, dxgkrnlFilesInUpstream: 0, dxgkrnlConflicts: 0 } }),
  "@@conflicts", "drivers/hv/Kconfig",
  "@@merged", "1790001000", "@@end", "",
].join("\n");

describe("wsl records", () => {
  test("parses build, boot and merge sections", () => {
    const records = parseWslSections(sections);
    expect(records.build).toMatchObject({ release: "6.18.1.1-microsoft-standard-WSL2-example+", builtAt: 1790000000 });
    expect(records.build!.boot).toMatchObject({ accel: "kvm", panic: false, seconds: 3, release: "6.18.1.1-microsoft-standard-WSL2-example+" });
    expect(records.merge).toMatchObject({ conflictedFiles: 1, conflicts: ["drivers/hv/Kconfig"], dxgkrnlConflicts: 0 });
  });

  test("a panic in the boot log is reported", () => {
    const records = parseWslSections(sections.replace("WSLTEST bugs 0", "WSLTEST bugs 2"));
    expect(records.build!.boot!.panic).toBe(true);
  });

  test("the page names what the boot test cannot show and the version row ranks like a polled one", () => {
    const records = parseWslSections(sections);
    const page = renderWslPage(records, "6.18.58.1", "7.4-rc3");
    expect(page).toContain("Not exercised by this test");
    expect(page).toContain("dxgkrnl");
    expect(renderWslPage(records, null, null)).toBe(renderWslPage(records, null, null));
    expect(wslVersionRows(records)).toEqual([expect.objectContaining({ kernel: "wsl", source: "wsl-build", channel: "aphrody build (example)" })]);
    expect(wslVersionRows({ build: null, merge: null })).toEqual([]);
  });
});
