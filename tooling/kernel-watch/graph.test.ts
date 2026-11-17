// Structure-report analyzers on a small synthetic tree; no clones, no cargo.
import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { cmakeReport, kconfigGraph, parseMaintainers, scanTree, styleOf, syscallCoverage } from "./graph.ts";

const root = mkdtempSync(join(tmpdir(), "kernel-watch-graph-"));
afterAll(() => rmSync(root, { recursive: true, force: true }));
const files: Record<string, string> = {
  "drivers/Kconfig": "source \"drivers/net/Kconfig\"\n",
  "drivers/net/Kconfig": "config NET_DRV\n\ttristate \"net\"\n\tdepends on PCI && BLOCK_CORE\n\tselect CRYPTO_AES\n",
  "drivers/net/Makefile": "obj-$(CONFIG_NET_DRV) += wifi/ nic.o\n",
  "drivers/pci/Kconfig": "config PCI\n\tbool \"PCI\"\n",
  "crypto/Kconfig": "config CRYPTO_AES\n\ttristate \"AES\"\n",
  "block/Kconfig": "config BLOCK_CORE\n\tbool \"block\"\n",
  "drivers/net/wifi/Makefile": "obj-y += a.o\n",
  "drivers/net/nic.c": "int main(void)\n{\n\treturn 0;\n}\n",
  "kernel/sched.rs": "fn main() {}\n",
  "base/CMakeLists.txt": "add_executable(app a.c)\nset_module_type(app win32cui)\nadd_importlibs(app kernel32 ntdll)\n",
  "dll/CMakeLists.txt": "add_library(kernel32 SHARED k.c)\nset_module_type(kernel32 win32dll)\nadd_importlibs(kernel32 ntdll)\n",
  "ntos/CMakeLists.txt": "add_library(ntdll SHARED n.c)\nset_module_type(ntdll nativedll)\n",
  "book/src/kernel/linux-compatibility/syscall-flag-coverage/files/fully_covered.scml": "close(fd);\nread(fd, buf, count);\n",
  "book/src/kernel/linux-compatibility/syscall-flag-coverage/files/lseek.scml": "// seek\nlseek(\n    fd, offset,\n    whence = SEEK_SET\n);\nflags = A | B;\n",
  ".clang-format": "BasedOnStyle: LLVM\nColumnLimit: 80\nUseTab: Always\n",
  "rustfmt.toml": "imports_granularity = \"Crate\"\n",
  ".editorconfig": "root = true\n[*.rs]\nindent_size = 4\n",
  "scripts/checkpatch.pl": "my $max_line_length = 100;\nmy $strict = 0; # --strict\n",
};
for (const [path, text] of Object.entries(files)) {
  mkdirSync(join(root, dirname(path)), { recursive: true });
  writeFileSync(join(root, path), text);
}
const list = Object.keys(files);

describe("structure analyzers", () => {
  test("scanTree sizes the top level and counts code lines by language", async () => {
    const tree = await scanTree(root, list);
    expect(tree.files).toBe(list.length);
    const drivers = tree.top.find((t) => t.name === "drivers/")!;
    expect(drivers.files).toBe(6);
    expect(tree.languages.find((l) => l.language === "C")).toMatchObject({ files: 1, lines: 4 });
    expect(tree.languages.find((l) => l.language === "Rust")).toMatchObject({ files: 1, lines: 1 });
  });

  test("kconfigGraph links groups through select, depends and Makefile descents", async () => {
    const graph = await kconfigGraph(root, list);
    expect(graph.symbols).toBe(4);
    const edge = (from: string, to: string, kind: string) => graph.edges.find((e) => e.from === from && e.to === to && e.kind === kind)?.weight;
    expect(edge("drivers/net", "crypto", "select")).toBe(1);
    expect(edge("drivers/net", "drivers/pci", "depends")).toBe(1);
    expect(edge("drivers/net", "block", "depends")).toBe(1);
    expect(graph.makeEdges.some((e) => e.from === "drivers/net" && e.to === "drivers/net")).toBe(false);
  });

  test("parseMaintainers reads subsystems without copying people", () => {
    const text = "Maintainers List\n-----------------------------------\n\nNETWORKING [GENERAL]\nM:\tSomebody <somebody@example.invalid>\nL:\tnetdev@vger.kernel.org\nS:\tMaintained\nT:\tgit git://example.invalid/net.git\nF:\tnet/\nF:\tinclude/net/\n\nTHE REST\nM:\tLinus\nS:\tBuried alive in reporters\nF:\t*\n";
    const subsystems = parseMaintainers(text);
    expect(subsystems.map((s) => s.name)).toEqual(["NETWORKING [GENERAL]", "THE REST"]);
    expect(subsystems[0]).toEqual({ name: "NETWORKING [GENERAL]", status: "Maintained", lists: 1, maintainers: 1, patterns: 2, trees: ["git git://example.invalid/net.git"] });
    expect(JSON.stringify(subsystems)).not.toContain("somebody");
  });

  test("cmakeReport inventories NT module types and the most linked modules", async () => {
    const report = await cmakeReport(root, list);
    expect(report.modules).toBe(3);
    expect(report.moduleTypes.map(([k]) => k).sort()).toEqual(["nativedll", "win32cui", "win32dll"]);
    expect(report.mostLinked[0]).toEqual({ module: "ntdll", linkedBy: 2 });
  });

  test("syscallCoverage counts SCML syscalls per category", async () => {
    const coverage = await syscallCoverage(root, list);
    expect(coverage.categories).toEqual([{ category: "files", scmlFiles: 2, fullyCovered: 2, flagLevel: 1 }]);
    expect(coverage.syscalls).toBe(3);
  });

  test("styleOf reports the style files a repository enforces", async () => {
    const style = await styleOf(root, new Set(list));
    expect(style.clangFormat).toEqual({ BasedOnStyle: "LLVM", ColumnLimit: "80", UseTab: "Always" });
    expect(style["rustfmt.toml"]).toEqual({ imports_granularity: "Crate" });
    expect(style.checkpatch).toEqual({ maxLineLength: "100", strictOption: true });
    expect((style.editorconfig as any)["*.rs"].indent_size).toBe("4");
    expect(style.present).toEqual(expect.arrayContaining([".clang-format", "rustfmt.toml", "scripts/checkpatch.pl"]));
  });
});
