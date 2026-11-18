import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { checkRepo, directionOf, fixEditorconfig, fixGitattributes, fixHtml, fixRepo, loadSpec, main, safeWrite, wellFormedLang } from "./check";

let dir: string;
beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "i18n-test-"));
});
afterEach(() => rmSync(dir, { recursive: true, force: true }));

const put = (rel: string, content: string | Uint8Array) => {
  mkdirSync(dirname(join(dir, rel)), { recursive: true });
  writeFileSync(join(dir, rel), content);
};
const clean = () => {
  put(".editorconfig", "root = true\n\n[*]\ncharset = utf-8\nend_of_line = lf\n");
  put(".gitattributes", "* text=auto eol=lf\n");
};
const ids = async () => Object.keys((await checkRepo(dir)).counts).sort();

describe("encoding", () => {
  test("clean repo has no finding", async () => {
    clean();
    put("a.ts", "export const x = 1;\n");
    expect((await checkRepo(dir)).findings).toEqual([]);
  });
  test("BOM, UTF-16, invalid UTF-8", async () => {
    clean();
    put("bom.json", Uint8Array.from([0xef, 0xbb, 0xbf, 0x7b, 0x7d]));
    put("u16.txt", Uint8Array.from([0xff, 0xfe, 0x61, 0x00]));
    put("latin.txt", Uint8Array.from([0x63, 0x61, 0x66, 0xe9, 0x0a]));
    expect(await ids()).toEqual(["bom", "utf16", "utf8-invalid"]);
  });
  test("BOM allowed on PowerShell, CRLF allowed on batch", async () => {
    clean();
    put("a.ps1", Uint8Array.from([0xef, 0xbb, 0xbf, 0x61, 0x0a]));
    put("a.bat", "echo hi\r\n");
    expect(await ids()).toEqual([]);
  });
  test("CRLF flagged with its line", async () => {
    clean();
    put("a.md", "one\ntwo\r\nthree\r\n");
    const r = await checkRepo(dir);
    expect(r.findings).toMatchObject([{ rule: "crlf", file: "a.md", line: 2 }]);
  });
  test("binary and excluded paths are skipped", async () => {
    clean();
    put("img.png", Uint8Array.from([0x89, 0x50, 0xe9, 0x00, 0xff]));
    put("node_modules/x/a.txt", Uint8Array.from([0xe9]));
    put("blob.dat2", Uint8Array.from([0x00, 0xe9, 0xff]));
    expect(await ids()).toEqual([]);
  });
});

describe("repo hygiene", () => {
  test("missing .editorconfig and .gitattributes", async () => {
    expect(await ids()).toEqual(["editorconfig-charset", "gitattributes-eol"]);
  });
  test("editorconfig without charset or with another charset", async () => {
    put(".gitattributes", "* text=auto eol=lf\n");
    put(".editorconfig", "[*]\nindent_size = 2\n");
    expect(await ids()).toEqual(["editorconfig-charset", "editorconfig-eol"]);
    put(".editorconfig", "[*]\ncharset = utf-8\nend_of_line = lf\n[*.txt]\ncharset = latin1\n");
    expect(await ids()).toEqual(["editorconfig-charset"]);
  });
  test("gitattributes needs eol=lf", async () => {
    put(".editorconfig", "[*]\ncharset = utf-8\nend_of_line = lf\n");
    put(".gitattributes", "* text=auto\n");
    expect(await ids()).toEqual(["gitattributes-eol"]);
  });
});

describe("html", () => {
  const html = (attrs: string, head = '<meta charset="utf-8">') => `<!doctype html>\n<html${attrs}>\n<head>${head}</head><body></body></html>\n`;
  test("missing lang, dir, charset", async () => {
    clean();
    put("a.html", html("", ""));
    expect(await ids()).toEqual(["html-charset-missing", "html-dir", "html-lang"]);
  });
  test("complete document is clean", async () => {
    clean();
    put("a.html", html(' lang="ar" dir="rtl"'));
    put("Layout.tsx", 'export const L = () => <html lang={locale} dir={dir}><body /></html>;\n');
    expect(await ids()).toEqual([]);
  });
  test("malformed lang and legacy charset", async () => {
    clean();
    put("a.html", html(' lang="en_US" dir="ltr"', '<meta charset="iso-8859-1">'));
    expect(await ids()).toEqual(["html-charset", "html-lang-malformed"]);
  });
  test("meta content-language and layout tsx", async () => {
    clean();
    put("a.html", html(' lang="en" dir="ltr"', '<meta charset="utf-8"><meta http-equiv="content-language" content="en">'));
    put("layout.tsx", "export default () => <html><body /></html>;\n");
    expect(await ids()).toEqual(["html-dir", "html-lang", "html-meta-content-language"]);
  });
  test("lang and direction helpers", () => {
    expect(wellFormedLang("zh-Hans")).toBe(true);
    expect(wellFormedLang("en_US")).toBe(false);
    expect(wellFormedLang("")).toBe(true);
    expect(directionOf("ar")).toBe("rtl");
    expect(directionOf("he-IL")).toBe("rtl");
    expect(directionOf("fr")).toBe("ltr");
    expect(directionOf("not a tag")).toBe("auto");
  });
});

describe("content rules", () => {
  test("hardcoded locale and date formats", async () => {
    clean();
    put("src/a.ts", 'const a = d.toLocaleDateString("en-US");\nconst f = fmt("MM/DD/YYYY");\nconst ok = d.toLocaleDateString(undefined); // fine\nconst s = "x".toLocaleString("en-US"); // i18n-ignore\n');
    const r = await checkRepo(dir);
    expect(r.findings.map((x) => [x.rule, x.line])).toEqual([["locale-hardcode", 1], ["locale-hardcode", 2]]);
  });
  test("tests and docs are exempt from content rules", async () => {
    clean();
    put("test/a.ts", 'd.toLocaleDateString("en-US");\n');
    expect(await ids()).toEqual([]);
  });
  test("bidi control in source", async () => {
    clean();
    put("src/a.ts", `const s = '\u202e';\n`);
    expect(await ids()).toEqual(["bidi-control"]);
  });
  test("UI literals without catalog, then with i18n library", async () => {
    clean();
    put("src/App.tsx", 'export const A = () => <button title="Save the file">Save changes now</button>;\n');
    expect(await ids()).toEqual(["no-i18n-catalog", "ui-string-literal"]);
    put("package.json", '{"dependencies":{"i18next":"1"}}\n');
    expect(await ids()).toEqual(["ui-string-literal"]);
  });
  test("catalog files are not UI violations", async () => {
    clean();
    put("src/locales/en/App.tsx", "export const A = () => <b>Save changes now</b>;\n");
    expect(await ids()).toEqual([]);
  });
});

describe("openapi", () => {
  const spec = (extra: string) => `openapi: 3.1.0\ninfo: {title: T, version: "1"}\npaths: {}\n${extra}`;
  test("no negotiation and unlabelled natural-language fields", async () => {
    clean();
    put("api/openapi.yaml", spec("components:\n  schemas:\n    Item:\n      type: object\n      properties:\n        title: {type: string}\n"));
    expect(await ids()).toEqual(["openapi-negotiation", "openapi-string-meta"]);
  });
  test("negotiation headers and Localizable schema pass", async () => {
    clean();
    put(
      "api/openapi.json",
      JSON.stringify({
        openapi: "3.1.0",
        info: { title: "T", version: "1" },
        paths: { "/x": { get: { parameters: [{ name: "Accept-Language", in: "header" }], responses: { 200: { description: "ok", headers: { "Content-Language": { schema: { type: "string" } } } } } } } },
        components: { schemas: { Localizable: { type: "object", properties: { value: { type: "string" }, lang: { type: "string" }, dir: { type: "string" } } }, Item: { type: "object", properties: { title: { $ref: "#/components/schemas/Localizable" } } } } },
      }),
    );
    expect(await ids()).toEqual([]);
  });
  test("non-OpenAPI yaml is ignored", async () => {
    clean();
    put("config.yaml", "title: hello\n");
    expect(await ids()).toEqual([]);
  });
});

describe("fix", () => {
  test("fixers are idempotent and clear the findings", async () => {
    put("bom.json", Uint8Array.from([0xef, 0xbb, 0xbf, ...new TextEncoder().encode('{"a":1}\r\n')]));
    put("latin.txt", Uint8Array.from([0x63, 0x61, 0x66, 0xe9, 0x0d, 0x0a]));
    put("u16.txt", Uint8Array.from([0xff, 0xfe, 0x61, 0x00, 0x0a, 0x00]));
    put("run.bat", "echo\r\n");
    put("page.html", "<!doctype html>\n<html>\n<head></head><body></body></html>\n");
    put("rtl.html", '<!doctype html>\n<html lang="he">\n<head><meta charset="utf-8"></head><body></body></html>\n');
    const changed = await fixRepo(dir);
    expect(changed).toEqual([".editorconfig", ".gitattributes", "bom.json", "latin.txt", "page.html", "rtl.html", "u16.txt"]);
    expect(readFileSync(join(dir, "latin.txt"), "utf8")).toBe("café\n");
    expect(readFileSync(join(dir, "u16.txt"), "utf8")).toBe("a\n");
    expect(readFileSync(join(dir, "bom.json"), "utf8")).toBe('{"a":1}\n');
    expect(readFileSync(join(dir, "run.bat"), "utf8")).toBe("echo\r\n");
    expect(readFileSync(join(dir, "page.html"), "utf8")).toContain('<html lang="en" dir="ltr">');
    expect(readFileSync(join(dir, "rtl.html"), "utf8")).toContain('<html lang="he" dir="rtl">');
    expect(await ids()).toEqual([]);
    expect(await fixRepo(dir)).toEqual([]);
  });
  test("legacy multi-byte text is not converted blindly", async () => {
    put("sjis.txt", Uint8Array.from([0x82, 0xa0, 0x82, 0xa2, 0x0a])); // Shift-JIS hiragana
    const before = readFileSync(join(dir, "sjis.txt"));
    await fixRepo(dir);
    expect(readFileSync(join(dir, "sjis.txt"))).toEqual(before);
    expect(await ids()).toContain("utf8-invalid");
  });
  test("editorconfig and gitattributes completion", () => {
    expect(fixEditorconfig("[*]\nindent_size = 2\n")).toContain("charset = utf-8");
    expect(fixEditorconfig("[*]\ncharset = latin1\n")).toContain("charset = utf-8");
    expect(fixEditorconfig("[*.md]\nx = y\n")).toContain("[*]\ncharset = utf-8");
    expect(fixGitattributes("* text=auto\n*.png binary\n")).toBe("* text=auto eol=lf\n*.bat text eol=crlf\n*.cmd text eol=crlf\n*.png binary\n");
    expect(fixHtml("<html><head></head></html>", "html", { default_lang: "fr" } as never)).toContain('<html lang="fr" dir="ltr">');
  });
});

describe("safe write", () => {
  const link = (target: string, path: string, type: "file" | "dir") => {
    try {
      symlinkSync(target, path, type === "dir" && process.platform === "win32" ? "junction" : type);
      return true;
    } catch {
      return false; // creating links needs a privilege on Windows
    }
  };
  test("writes in place, keeps the mode, leaves no temp file", () => {
    put("a.txt", "old");
    expect(safeWrite(dir, "a.txt", "new")).toBe(true);
    expect(readFileSync(join(dir, "a.txt"), "utf8")).toBe("new");
    expect(safeWrite(dir, "sub/b.txt", "x")).toBe(false); // missing directory
    expect(existsSync(join(dir, ".a.txt.i18n-" + process.pid + ".tmp"))).toBe(false);
  });
  test("a symlinked file is not written through", async () => {
    const outside = mkdtempSync(join(tmpdir(), "i18n-outside-"));
    try {
      writeFileSync(join(outside, "target.txt"), "keep");
      if (!link(join(outside, "target.txt"), join(dir, ".editorconfig"), "file")) return;
      expect(safeWrite(dir, ".editorconfig", "x")).toBe(false);
      await fixRepo(dir);
      expect(readFileSync(join(outside, "target.txt"), "utf8")).toBe("keep");
    } finally {
      rmSync(outside, { recursive: true, force: true });
    }
  });
  test("a symlinked directory cannot redirect a write outside the root", () => {
    const outside = mkdtempSync(join(tmpdir(), "i18n-outside-"));
    try {
      if (!link(outside, join(dir, "linked"), "dir")) return;
      expect(safeWrite(dir, "linked/x.txt", "x")).toBe(false);
      expect(existsSync(join(outside, "x.txt"))).toBe(false);
    } finally {
      rmSync(outside, { recursive: true, force: true });
    }
  });
});

describe("cli", () => {
  test("exit code follows fail-on and json report is written", async () => {
    put("a.txt", "x\r\n");
    const out = join(dir, "r.json");
    expect(await main([dir, "--quiet", "--json", out])).toBe(1);
    expect(JSON.parse(readFileSync(out, "utf8"))[0].counts.crlf).toBe(1);
    expect(await main([dir, "--quiet", "--fail-on", "none"])).toBe(0);
  });
  test("spec loads and every rule has a severity", async () => {
    const spec = await loadSpec();
    for (const r of Object.values(spec.rules)) expect(["error", "warn"]).toContain(r.severity);
  });
});
