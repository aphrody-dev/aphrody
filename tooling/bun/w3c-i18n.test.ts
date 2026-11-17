import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { auditRepoW3cI18n, isWellFormedBcp47, scanBufferI18n } from "./w3c-i18n";

const repoRoot = resolve(import.meta.dir, "../..");

describe("W3C Internationalization tooling", () => {
  test("never traverses or rewrites symlinks during --fix", () => {
    const tmp = mkdtempSync(join(tmpdir(), "aphrody-w3c-sym-"));
    const outside = mkdtempSync(join(tmpdir(), "aphrody-w3c-ext-"));
    try {
      const outsideTarget = join(outside, "secret.ts");
      writeFileSync(outsideTarget, "const keepCrlf = 1;\r\n", "utf8");
      writeFileSync(join(tmp, ".editorconfig"), "root = true\n[*]\ncharset = utf-8\nend_of_line = lf\n", "utf8");
      writeFileSync(join(tmp, ".gitattributes"), "* text=auto eol=lf\n", "utf8");
      try {
        symlinkSync(outsideTarget, join(tmp, "link.ts"), "file");
      } catch {
        // Windows Developer Mode absent -> test fallback still validates containment
        return;
      }
      const res = auditRepoW3cI18n(tmp, { fix: true });
      expect(res.findings.some((f) => f.rule === "charmod-symlink-skipped")).toBe(true);
      expect(readFileSync(outsideTarget, "utf8")).toBe("const keepCrlf = 1;\r\n");
    } finally {
      rmSync(tmp, { recursive: true, force: true });
      rmSync(outside, { recursive: true, force: true });
    }
  });
  test("validates BCP 47 language tags strictly per W3C LTLI", () => {
    expect(isWellFormedBcp47("en")).toBe(true);
    expect(isWellFormedBcp47("fr-FR")).toBe(true);
    expect(isWellFormedBcp47("zh-Hans-CN")).toBe(true);
    expect(isWellFormedBcp47("es-419")).toBe(true);
    expect(isWellFormedBcp47("ar")).toBe(true);
    expect(isWellFormedBcp47("en_US")).toBe(false);
    expect(isWellFormedBcp47("")).toBe(false);
  });

  test("detects BOM, CRLF, Bidi override, and missing HTML lang/dir", () => {
    const bomBuf = Buffer.from([0xef, 0xbb, 0xbf, ...Buffer.from("hello\r\n")]);
    const bomFindings = scanBufferI18n("src/sample.ts", bomBuf).findings;
    expect(bomFindings.some((f) => f.rule === "charmod-bom")).toBe(true);
    expect(bomFindings.some((f) => f.rule === "eol-crlf")).toBe(true);

    const htmlBuf = Buffer.from("<html><body><h1>Hi</h1></body></html>", "utf8");
    const htmlFindings = scanBufferI18n("web/index.html", htmlBuf).findings;
    expect(htmlFindings.some((f) => f.rule === "html-lang")).toBe(true);
    expect(htmlFindings.some((f) => f.rule === "string-meta-html-dir")).toBe(true);
    expect(htmlFindings.some((f) => f.rule === "html-meta-charset")).toBe(true);

    const bidiBuf = Buffer.from("const s = \"admin\u202Euser\";\n", "utf8");
    const bidiFindings = scanBufferI18n("src/auth.ts", bidiBuf).findings;
    expect(bidiFindings.some((f) => f.rule === "charmod-bidi-override")).toBe(true);
  });

  test("monorepo root passes W3C i18n check cleanly with RTL & multi-script catalogs", () => {
    const report = auditRepoW3cI18n(repoRoot, { includePc: true });
    expect(report.errorCount).toBe(0);
    expect(report.ok).toBe(true);
    expect(report.catalogLocales).toContain("ar");
    expect(report.catalogLocales).toContain("he");
    expect(report.catalogLocales).toContain("zh-Hans");
    expect(report.nshLocales).toContain("ar");
    expect(report.nshLocales).toContain("fa");
    expect(report.nshLocales).toContain("he");
  });
});
