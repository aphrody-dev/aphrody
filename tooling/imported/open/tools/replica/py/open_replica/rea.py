"""Binary analysis for a replica, for binaries the user may analyse (their own builds, open-source releases, files
they hold the rights to study for interoperability): the facts an interoperable rebuild needs (format,
architecture, linked libraries, imports and exports, strings that name protocols, endpoints and file formats),
written as Markdown so the retrieval index and the agent stages can use them. No code is copied from a binary.

LIEF reads the format; pyghidra (Ghidra headless, GHIDRA_INSTALL_DIR or the last Ghidra pyghidra used) adds the
analysed functions when available. Writes rea/<name>.md in the replica directory."""

from __future__ import annotations

import hashlib
import re
import tempfile
from pathlib import Path

INTERESTING = re.compile(rb"(https?://[\x21-\x7e]{6,200}|/api/[\x21-\x7e]{2,120}|[A-Za-z0-9_.-]+\.(?:json|proto|yaml|toml|sqlite|db)\b|application/[a-z0-9.+-]{3,60})")


def lief_facts(path: Path) -> list[str]:
    import lief

    b = lief.parse(str(path))
    if b is None:
        return ["LIEF: not an executable format it reads."]
    lines = [f"- format: {b.format.name}", f"- entrypoint: {hex(b.entrypoint)}"]
    lib = getattr(b, "libraries", None)
    if lib:
        lines.append(f"- libraries: {', '.join(str(x) for x in list(lib)[:60])}")
    imports = [f.name for f in getattr(b, "imported_functions", [])][:200]
    exports = [f.name for f in getattr(b, "exported_functions", [])][:200]
    if imports:
        lines.append(f"- imported functions ({len(imports)} shown): {', '.join(imports)}")
    if exports:
        lines.append(f"- exported functions ({len(exports)} shown): {', '.join(exports)}")
    return lines


def ghidra_facts(path: Path) -> list[str]:
    import pyghidra

    if not pyghidra.started():
        # the JVM starts once per process; later binaries reuse it
        pyghidra.start()
    lines: list[str] = []
    with tempfile.TemporaryDirectory() as tmp, pyghidra.open_project(tmp, "replica", create=True) as project:
        loader = pyghidra.program_loader().project(project).source(str(path)).name(path.name)
        with loader.load() as results:
            results.save(pyghidra.task_monitor())
        with pyghidra.program_context(project, "/" + path.name) as program:
            pyghidra.analyze(program, pyghidra.task_monitor(300))
            fm = program.getFunctionManager()
            names = [f.getName() for f in fm.getFunctions(True)]
            named = [n for n in names if not n.startswith(("FUN_", "thunk_FUN_"))]
            lines.append(f"- Ghidra: {program.getLanguageID()} {len(names)} functions, {len(named)} with names")
            lines.append(f"- named functions (first 300): {', '.join(named[:300])}")
    return lines


def report(path: Path, out: Path, ghidra: bool = True) -> Path:
    data = path.read_bytes()
    lines = [f"# {path.name}", "", f"{len(data)} bytes.", "", "## Format", ""]
    try:
        lines += lief_facts(path)
    except Exception as e:
        lines.append(f"- LIEF failed: {e}")
    if ghidra:
        lines += ["", "## Ghidra", ""]
        try:
            lines += ghidra_facts(path)
        except Exception as e:
            lines.append(f"- Ghidra analysis not run: {str(e)[:300]}")
    found = sorted({m.decode("ascii", "replace") for m in INTERESTING.findall(data)})[:400]
    lines += ["", "## Strings naming endpoints, formats and media types", ""] + [f"- `{s}`" for s in found] + [""]
    d = out / "rea"
    d.mkdir(parents=True, exist_ok=True)
    # a safe file name, unique per source path (two binaries may share a name)
    tag = hashlib.sha256(str(path.resolve()).encode()).hexdigest()[:8]
    f = d / f"{re.sub(r'[^A-Za-z0-9._-]', '_', path.name)[:80]}-{tag}.md"
    f.write_text("\n".join(lines), encoding="utf-8")
    return f
