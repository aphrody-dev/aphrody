import { describe, expect, test } from "bun:test";
import { auditProjectsObject, auditSettingsObject, hasCaseDuplicates } from "./agy-config-audit.ts";

describe("agy-config-audit", () => {
  test("detects case-insensitive duplicate workspace paths cleanly", () => {
    expect(hasCaseDuplicates(["C:/src", "c:\\src\\"])).toEqual(["C:/src vs c:\\src\\"]);
    expect(hasCaseDuplicates(["C:/src", "C:/src/aphrody"])).toEqual([]);
  });

  test("enforces schema, zero-deny, statusLine, title, and valid projects", () => {
    const settingsChecks = auditSettingsObject({
      $schema: "https://antigravity.google/schemas/v1/settings.json",
      trustedWorkspaces: [process.cwd()],
      statusLine: { type: "command", command: "bun status.ts" },
      title: { type: "command", command: "bun title.ts" },
    });
    expect(settingsChecks.every((c) => c.ok)).toBe(true);

    const denyCheck = auditSettingsObject({
      $schema: "https://antigravity.google/schemas/v1/settings.json",
      permissions: { deny: ["command(rm)"] },
      trustedWorkspaces: [process.cwd()],
      statusLine: { type: "command", command: "bun status.ts" },
      title: { type: "command", command: "bun title.ts" },
    }).find((c) => c.name === "settings:zero-deny");
    expect(denyCheck?.ok).toBe(false);

    const projChecks = auditProjectsObject(
      { projects: { [process.cwd().replaceAll("\\", "/")]: "aphrody" } },
      process.cwd(),
    );
    expect(projChecks[0]?.ok).toBe(true);
  });
});
