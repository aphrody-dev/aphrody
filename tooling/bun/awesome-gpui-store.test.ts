import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ALL_TARGETS,
  buildCatalogEntries,
  DEFAULT_CATALOG_PATH,
  loadStoreCatalog,
  packageEntry,
  parseAwesomeGpuiMarkdown,
  validateXmlWellFormed,
  verifyPackagedDir,
} from "./awesome-gpui-store.ts";

describe("awesome-gpui-store catalog, parser, and 22-target packager", () => {
  test("live synced awesome_gpui_store.toml parses cleanly and contains real upstream entries", () => {
    const catalog = loadStoreCatalog(DEFAULT_CATALOG_PATH);
    expect(catalog.schema_version).toBe(1);
    expect(catalog.source_repo).toBe("zed-industries/awesome-gpui");
    expect(catalog.entries.length).toBeGreaterThanOrEqual(180);

    const byId = new Map(catalog.entries.map((e) => [e.id, e]));
    const aphrody = byId.get("aphrody");
    expect(aphrody).toBeTruthy();
    expect(aphrody?.first_party).toBe(true);
    expect(aphrody?.targets.length).toBe(ALL_TARGETS.length);

    expect(byId.get("zed")?.category).toBe("app");
    expect(byId.get("gpui-component")?.category).toBe("library");
    expect(byId.get("gpui-mobile")?.category).toBe("library");
    expect(byId.get("create-gpui-app")?.category).toBe("tool");
    expect(byId.get("gpui-book")?.category).toBe("resource");
    expect(byId.get("gpui-calculator")?.category).toBe("showcase");

    const cats = new Set(catalog.entries.map((e) => e.category));
    expect(cats.has("app")).toBe(true);
    expect(cats.has("library")).toBe(true);
    expect(cats.has("showcase")).toBe(true);
    expect(cats.has("tool")).toBe(true);
    expect(cats.has("resource")).toBe(true);
  });

  test("parseAwesomeGpuiMarkdown handles rows with and without icons across all sections", () => {
    const sample = `
## Apps

### Developer Tools

| Image | Project | Description | Stars | Status |
| --- | --- | --- | ---: | :---: |
| <img src="https://example.com/icon.png" alt="" height="64"> | [Zed](https://github.com/zed-industries/zed) | High-performance editor for macOS, Windows, and Linux. | 95.1k 🔥 | 🟢 |
|  | [zlyph](https://github.com/douglance/zlyph) | Minimal GPUI editor. | 15 | ⚪ |

### Demos & Examples

| Image | Project | Description | Stars | Status |
| --- | --- | --- | ---: | :---: |
|  | [gpui-base64](https://github.com/badgooooor/gpui-base64) | Base64 encoder example. | 1 | 🟡 |

## Libraries

| Image | Project | Description | Stars | Status |
| --- | --- | --- | ---: | :---: |
|  | [Base GPUI](https://github.com/LukeTandjung/base-gpui) | UI component library with themes and WebAssembly support. | 93 | 📦 |

## Tooling

| Image | Project | Description | Stars | Status |
| --- | --- | --- | ---: | :---: |
|  | [create-gpui-app](https://github.com/zed-industries/create-gpui-app) | Scaffold a GPUI app. | 397 | ❓ |

## Resources

| Image | Project | Description | Stars | Status |
| --- | --- | --- | ---: | :---: |
|  | [gpui-book](https://github.com/MatinAniss/gpui-book) | Book teaching GPUI. | 288 | ⚪ |
`;
    const rows = parseAwesomeGpuiMarkdown(sample);
    expect(rows.length).toBe(6);
    expect(rows[0].stars).toBe(95100);
    expect(rows[0].status).toBe("active");
    expect(rows[1].iconUrl).toBe("");
    expect(rows[1].status).toBe("dormant");
    expect(rows[2].status).toBe("quiet");
    expect(rows[3].status).toBe("archived");
    expect(rows[4].status).toBe("unavailable");

    const built = buildCatalogEntries(rows, new Map(), false);
    expect(built[0].category).toBe("app");
    expect(built[2].category).toBe("showcase");
    expect(built[3].category).toBe("library");
    expect(built[3].subcategory).toBe("UI Kits");
    expect(built[3].platforms).toContain("web");
    expect(built[4].category).toBe("tool");
    expect(built[5].category).toBe("resource");
  });

  test("packages and structurally validates all 22 store & distro targets in temp dir", () => {
    const catalog = loadStoreCatalog(DEFAULT_CATALOG_PATH);
    const aphrody = catalog.entries.find((e) => e.id === "aphrody");
    const zedis = catalog.entries.find((e) => e.id === "zedis");
    expect(aphrody).toBeTruthy();
    expect(zedis).toBeTruthy();

    const dir = mkdtempSync(join(tmpdir(), "gpui-store-test-"));
    try {
      const resA = packageEntry(aphrody!, ALL_TARGETS, dir);
      const resB = packageEntry(zedis!, ["winget", "rpm", "deb", "brew", "oci"], dir);
      expect(resA.writtenFiles.length).toBeGreaterThanOrEqual(28);
      expect(resB.writtenFiles.length).toBeGreaterThanOrEqual(8);

      const report = verifyPackagedDir(dir);
      expect(report.errors).toEqual([]);
      expect(report.ok).toBe(true);
      expect(report.targetsChecked.length).toBe(ALL_TARGETS.length);

      // Parse specific artifacts across original 10 + 11 new distro/manager targets
      const wingetVer = Bun.YAML.parse(
        readFileSync(join(dir, "aphrody/winget/AphrodyLabs.Aphrody.yaml"), "utf8"),
      ) as Record<string, unknown>;
      expect(wingetVer.PackageIdentifier).toBe("AphrodyLabs.Aphrody");

      const appx = readFileSync(join(dir, "aphrody/msstore/AppxManifest.xml"), "utf8");
      expect(validateXmlWellFormed(appx, "AppxManifest.xml")).toEqual([]);
      expect(appx).toContain("1.30.0.0");

      const flatpak = Bun.YAML.parse(
        readFileSync(join(dir, "aphrody/flatpak/com.aphrody.Aphrody.yml"), "utf8"),
      ) as Record<string, unknown>;
      expect(flatpak.id).toBe("com.aphrody.Aphrody");

      const snap = Bun.YAML.parse(
        readFileSync(join(dir, "aphrody/snap/snap/snapcraft.yaml"), "utf8"),
      ) as Record<string, unknown>;
      expect(snap.name).toBe("aphrody");

      const cargo = Bun.TOML.parse(
        readFileSync(join(dir, "aphrody/cargo/Cargo.toml"), "utf8"),
      ) as { package: { name: string } };
      expect(cargo.package.name).toBe("zed");

      const npmPkg = JSON.parse(
        readFileSync(join(dir, "aphrody/npm/package.json"), "utf8"),
      ) as { nsh: { appId: string } };
      expect(npmPkg.nsh.appId).toBe("com.aphrody.Aphrody");

      const chromeMv3 = JSON.parse(
        readFileSync(join(dir, "aphrody/chrome/manifest.json"), "utf8"),
      ) as { manifest_version: number; content_security_policy: { extension_pages: string } };
      expect(chromeMv3.manifest_version).toBe(3);
      expect(chromeMv3.content_security_policy.extension_pages).toContain("'wasm-unsafe-eval'");

      // Newly expanded distro & package-manager targets
      expect(readFileSync(join(dir, "aphrody/apk/APKBUILD"), "utf8")).toContain("pkgname=aphrody");
      expect(readFileSync(join(dir, "aphrody/rpm/aphrody.spec"), "utf8")).toContain("Name:           aphrody");
      expect(readFileSync(join(dir, "aphrody/deb/DEBIAN/control"), "utf8")).toContain("Package: aphrody");
      expect(readFileSync(join(dir, "aphrody/xbps/srcpkgs/aphrody/template"), "utf8")).toContain(
        "build_style=cargo",
      );
      expect(readFileSync(join(dir, "aphrody/gentoo/aphrody-1.30.0.ebuild"), "utf8")).toContain("EAPI=8");
      expect(readFileSync(join(dir, "aphrody/freebsd/Makefile"), "utf8")).toContain("PORTNAME=\taphrody");
      expect(readFileSync(join(dir, "aphrody/brew/Formula/aphrody.rb"), "utf8")).toContain(
        "class Aphrody < Formula",
      );
      const scoop = JSON.parse(
        readFileSync(join(dir, "aphrody/scoop/bucket/aphrody.json"), "utf8"),
      ) as { bin: string; version: string };
      expect(scoop.bin).toBe("aphrody.exe");
      expect(scoop.version).toBe("1.30.0");

      const nuspec = readFileSync(join(dir, "aphrody/choco/aphrody.nuspec"), "utf8");
      expect(validateXmlWellFormed(nuspec, "aphrody.nuspec")).toEqual([]);

      const appImage = Bun.YAML.parse(
        readFileSync(join(dir, "aphrody/appimage/AppImageBuilder.yml"), "utf8"),
      ) as { version: number };
      expect(appImage.version).toBe(1);

      expect(readFileSync(join(dir, "aphrody/oci/Dockerfile"), "utf8")).toContain(
        "LABEL org.opencontainers.image.title=",
      );

      // Negative verification: ensure corrupt XML fails verifyPackagedDir
      writeFileSync(join(dir, "aphrody/msstore/AppxManifest.xml"), "<Package><Broken>", "utf8");
      const badReport = verifyPackagedDir(dir);
      expect(badReport.ok).toBe(false);
      expect(badReport.errors.length).toBeGreaterThan(0);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
