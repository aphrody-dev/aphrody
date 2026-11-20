import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import {
  checkVpnAllowLan,
  inspectDenyPolicy,
  isWireguardOrPrivateLanIp,
  parsePatchesSection,
  parseSshGOutput,
  repoRoot,
  runAudit,
} from "./vendor-lan-doctor.ts";

describe("vendor-lan-doctor", () => {
  test("parses [patch.crates-io] path and git entries", () => {
    const sample = `
[dependencies]
anyhow = "1"

[patch.crates-io]
taffy = { path = "local/taffy" }
wgpu = { git = "https://github.com/aphrody-labs/wgpu.git", tag = "v30.0.0-aphrody.20261120" }
cpal = { git = "https://github.com/RustAudio/cpal", rev = "e1612d5d" }

[profile.dev]
incremental = true
`;
    const parsed = parsePatchesSection(sample);
    expect(parsed).toEqual([
      { crateName: "taffy", kind: "path", target: "local/taffy" },
      { crateName: "wgpu", kind: "git", target: "https://github.com/aphrody-labs/wgpu.git" },
      { crateName: "cpal", kind: "git", target: "https://github.com/RustAudio/cpal" },
    ]);
  });

  test("classifies WireGuard LAN and parses ssh -G output", () => {
    expect(isWireguardOrPrivateLanIp("10.200.77.1")).toBe(true);
    expect(isWireguardOrPrivateLanIp("192.168.1.10")).toBe(true);
    expect(isWireguardOrPrivateLanIp("8.8.8.8")).toBe(false);

    const parsed = parseSshGOutput(
      "fr1",
      "user builder\nhostname 10.200.77.1\nport 22\n",
    );
    expect(parsed.configured).toBe(true);
    expect(parsed.hostname).toBe("10.200.77.1");
    expect(parsed.user).toBe("builder");
    expect(parsed.inWireguardLan).toBe(true);
  });

  test("offline workspace audit verifies deny.toml, pinned git patches, and AllowLan", () => {
    const deny = inspectDenyPolicy(join(repoRoot, "deny.toml"));
    expect(deny.denyTomlPresent).toBe(true);
    expect(deny.hasLicensesSection).toBe(true);
    expect(deny.hasSourcesSection).toBe(true);

    expect(checkVpnAllowLan(repoRoot)).toBe(true);

    const report = runAudit(repoRoot, true);
    expect(report.ok).toBe(true);
    expect(report.cargoPatches.missingPatchPaths).toEqual([]);
    expect(report.cargoPatches.patches.some((p) => p.kind === "git" && p.target.includes("aphrody-labs/wgpu"))).toBe(true);
    expect(report.cargoPatches.patches.some((p) => p.kind === "git" && p.target.includes("aphrody-labs/taffy"))).toBe(true);
  });
});
