// SPDX-License-Identifier: Apache-2.0
/**
 * Real OOXML .docx PKZIP Archive Builder, Zero-Eval Recursive-Descent Formula Engine,
 * RFC 5545 iCalendar (.ics), RFC 6350 vCard (.vcf), and Deterministic Bilingual Technical Translator.
 * 100% Pure Bun v1.4.4 — Zero External Dependencies, Zero eval().
 */

const CRC32_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[i] = c >>> 0;
  }
  return table;
})();

function computeCrc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (let i = 0; i < bytes.length; i++) {
    crc = CRC32_TABLE[(crc ^ bytes[i]!) & 0xff]! ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function escapeXml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

interface ZipEntrySpec {
  name: string;
  data: Uint8Array;
}

/**
 * Builds a standards-compliant PKZIP archive using Bun.deflateSync raw DEFLATE.
 */
export function buildPkZip(entries: ZipEntrySpec[]): Uint8Array {
  const encoder = new TextEncoder();
  const localChunks: Uint8Array[] = [];
  const centralChunks: Uint8Array[] = [];
  let offset = 0;

  for (const entry of entries) {
    const nameBytes = encoder.encode(entry.name);
    const crc = computeCrc32(entry.data);
    const compressed = new Uint8Array(Bun.deflateSync(entry.data));
    const useDeflate = compressed.byteLength < entry.data.byteLength;
    const payload = useDeflate ? compressed : entry.data;
    const method = useDeflate ? 8 : 0;

    // Local File Header (30 bytes + filename)
    const localHeader = new Uint8Array(30 + nameBytes.byteLength);
    const lv = new DataView(localHeader.buffer);
    lv.setUint32(0, 0x04034b50, true);
    lv.setUint16(4, 20, true); // version needed 2.0
    lv.setUint16(6, 0x0800, true); // UTF-8 flag
    lv.setUint16(8, method, true);
    lv.setUint16(10, 0, true);
    lv.setUint16(12, 0x5421, true);
    lv.setUint32(14, crc, true);
    lv.setUint32(18, payload.byteLength, true);
    lv.setUint32(22, entry.data.byteLength, true);
    lv.setUint16(26, nameBytes.byteLength, true);
    lv.setUint16(28, 0, true);
    localHeader.set(nameBytes, 30);

    localChunks.push(localHeader, payload);

    // Central Directory Header (46 bytes + filename)
    const centralHeader = new Uint8Array(46 + nameBytes.byteLength);
    const cv = new DataView(centralHeader.buffer);
    cv.setUint32(0, 0x02014b50, true);
    cv.setUint16(4, 20, true);
    cv.setUint16(6, 20, true);
    cv.setUint16(8, 0x0800, true);
    cv.setUint16(10, method, true);
    cv.setUint16(12, 0, true);
    cv.setUint16(14, 0x5421, true);
    cv.setUint32(16, crc, true);
    cv.setUint32(20, payload.byteLength, true);
    cv.setUint32(24, entry.data.byteLength, true);
    cv.setUint16(28, nameBytes.byteLength, true);
    cv.setUint16(30, 0, true);
    cv.setUint16(32, 0, true);
    cv.setUint16(34, 0, true);
    cv.setUint16(36, 0, true);
    cv.setUint32(38, 0, true);
    cv.setUint32(42, offset, true);
    centralHeader.set(nameBytes, 46);

    centralChunks.push(centralHeader);
    offset += localHeader.byteLength + payload.byteLength;
  }

  let centralSize = 0;
  for (const c of centralChunks) centralSize += c.byteLength;

  const eocd = new Uint8Array(22);
  const ev = new DataView(eocd.buffer);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(4, 0, true);
  ev.setUint16(6, 0, true);
  ev.setUint16(8, entries.length, true);
  ev.setUint16(10, entries.length, true);
  ev.setUint32(12, centralSize, true);
  ev.setUint32(16, offset, true);
  ev.setUint16(20, 0, true);

  const totalLen = offset + centralSize + eocd.byteLength;
  const out = new Uint8Array(totalLen);
  let pos = 0;
  for (const ch of [...localChunks, ...centralChunks, eocd]) {
    out.set(ch, pos);
    pos += ch.byteLength;
  }
  return out;
}

/**
 * Generates a genuine Microsoft Word / LibreOffice OOXML .docx binary archive.
 */
export function buildDocxArchive(title: string, bodyText: string): Uint8Array {
  const enc = new TextEncoder();
  const paragraphs = bodyText
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0);

  const contentTypesXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>`;

  const relsXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>`;

  const titlePara = `<w:p><w:r><w:rPr><w:b/><w:sz w:val="36"/></w:rPr><w:t>${escapeXml(title)}</w:t></w:r></w:p>`;
  const bodyParas = paragraphs
    .map(
      (p) =>
        `<w:p><w:r><w:rPr><w:sz w:val="22"/></w:rPr><w:t xml:space="preserve">${escapeXml(p)}</w:t></w:r></w:p>`,
    )
    .join("");

  const documentXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>${titlePara}${bodyParas}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body>
</w:document>`;

  return buildPkZip([
    { name: "[Content_Types].xml", data: enc.encode(contentTypesXml) },
    { name: "_rels/.rels", data: enc.encode(relsXml) },
    { name: "word/document.xml", data: enc.encode(documentXml) },
  ]);
}

// ============================================================================
// GridCraft Zero-Eval Recursive-Descent Formula Engine
// ============================================================================

export function expandCellRange(startRef: string, endRef: string): string[] {
  const m1 = /^([A-Z]+)(\d+)$/i.exec(startRef.trim());
  const m2 = /^([A-Z]+)(\d+)$/i.exec(endRef.trim());
  if (!m1 || !m2) return [];

  const c1 = m1[1]!.toUpperCase().charCodeAt(0);
  const r1 = Number.parseInt(m1[2]!, 10);
  const c2 = m2[1]!.toUpperCase().charCodeAt(0);
  const r2 = Number.parseInt(m2[2]!, 10);

  const out: string[] = [];
  for (let c = Math.min(c1, c2); c <= Math.max(c1, c2); c++) {
    for (let r = Math.min(r1, r2); r <= Math.max(r1, r2); r++) {
      out.push(`${String.fromCharCode(c)}${r}`);
    }
  }
  return out;
}

class FormulaParser {
  private pos = 0;

  constructor(
    private readonly expr: string,
    private readonly resolveCell: (ref: string) => number,
  ) {}

  parse(): number {
    const val = this.parseComparison();
    this.skipWs();
    return Number.isFinite(val) ? Math.round(val * 1_000_000) / 1_000_000 : 0;
  }

  private skipWs(): void {
    while (this.pos < this.expr.length && /\s/.test(this.expr[this.pos]!)) {
      this.pos++;
    }
  }

  private parseComparison(): number {
    let left = this.parseAddSub();
    this.skipWs();
    for (const op of [">=", "<=", "<>", "=", ">", "<"]) {
      if (this.expr.startsWith(op, this.pos)) {
        this.pos += op.length;
        const right = this.parseAddSub();
        if (op === ">=") left = left >= right ? 1 : 0;
        else if (op === "<=") left = left <= right ? 1 : 0;
        else if (op === "<>") left = left !== right ? 1 : 0;
        else if (op === "=") left = left === right ? 1 : 0;
        else if (op === ">") left = left > right ? 1 : 0;
        else if (op === "<") left = left < right ? 1 : 0;
        break;
      }
    }
    return left;
  }

  private parseAddSub(): number {
    let val = this.parseMulDiv();
    while (true) {
      this.skipWs();
      const ch = this.expr[this.pos];
      if (ch === "+" || ch === "-") {
        this.pos++;
        const rhs = this.parseMulDiv();
        val = ch === "+" ? val + rhs : val - rhs;
      } else {
        break;
      }
    }
    return val;
  }

  private parseMulDiv(): number {
    let val = this.parsePower();
    while (true) {
      this.skipWs();
      const ch = this.expr[this.pos];
      if (ch === "*" || ch === "/") {
        this.pos++;
        const rhs = this.parsePower();
        val = ch === "*" ? val * rhs : rhs === 0 ? 0 : val / rhs;
      } else {
        break;
      }
    }
    return val;
  }

  private parsePower(): number {
    const base = this.parseUnary();
    this.skipWs();
    if (this.expr[this.pos] === "^") {
      this.pos++;
      const exp = this.parsePower();
      return Math.pow(base, exp);
    }
    return base;
  }

  private parseUnary(): number {
    this.skipWs();
    if (this.expr[this.pos] === "-") {
      this.pos++;
      return -this.parseUnary();
    }
    if (this.expr[this.pos] === "+") {
      this.pos++;
      return this.parseUnary();
    }
    return this.parsePrimary();
  }

  private parsePrimary(): number {
    this.skipWs();
    if (this.expr[this.pos] === "(") {
      this.pos++;
      const v = this.parseComparison();
      this.skipWs();
      if (this.expr[this.pos] === ")") this.pos++;
      return v;
    }

    const numMatch = /^\d+(\.\d+)?/.exec(this.expr.slice(this.pos));
    if (numMatch) {
      this.pos += numMatch[0].length;
      return Number.parseFloat(numMatch[0]);
    }

    const identMatch = /^[A-Za-z_][A-Za-z0-9_]*/.exec(
      this.expr.slice(this.pos),
    );
    if (identMatch) {
      const ident = identMatch[0].toUpperCase();
      this.pos += ident.length;
      this.skipWs();

      if (this.expr[this.pos] === "(") {
        this.pos++; // consume '('
        const args = this.parseArgList();
        return this.applyFunction(ident, args);
      }

      return this.resolveCell(ident);
    }

    return 0;
  }

  private parseArgList(): number[] {
    const values: number[] = [];
    this.skipWs();
    if (this.expr[this.pos] === ")") {
      this.pos++;
      return values;
    }

    while (this.pos < this.expr.length) {
      this.skipWs();
      const rangeMatch = /^([A-Za-z]+\d+)\s*:\s*([A-Za-z]+\d+)/.exec(
        this.expr.slice(this.pos),
      );
      if (rangeMatch) {
        this.pos += rangeMatch[0].length;
        for (const ref of expandCellRange(rangeMatch[1]!, rangeMatch[2]!)) {
          values.push(this.resolveCell(ref));
        }
      } else {
        values.push(this.parseComparison());
      }

      this.skipWs();
      const sep = this.expr[this.pos];
      if (sep === "," || sep === ";") {
        this.pos++;
      } else if (sep === ")") {
        this.pos++;
        break;
      } else {
        break;
      }
    }
    return values;
  }

  private applyFunction(fn: string, args: number[]): number {
    switch (fn) {
      case "SUM":
        return args.reduce((a, b) => a + b, 0);
      case "AVG":
      case "AVERAGE":
        return args.length === 0
          ? 0
          : args.reduce((a, b) => a + b, 0) / args.length;
      case "MIN":
        return args.length === 0 ? 0 : Math.min(...args);
      case "MAX":
        return args.length === 0 ? 0 : Math.max(...args);
      case "ABS":
        return Math.abs(args[0] ?? 0);
      case "ROUND": {
        const factor = Math.pow(10, args[1] ?? 0);
        return Math.round((args[0] ?? 0) * factor) / factor;
      }
      case "IF":
        return (args[0] ?? 0) !== 0 ? (args[1] ?? 0) : (args[2] ?? 0);
      case "NPV": {
        const rate = args[0] ?? 0;
        let npv = 0;
        for (let i = 1; i < args.length; i++) {
          npv += args[i]! / Math.pow(1 + rate, i);
        }
        return npv;
      }
      default:
        return 0;
    }
  }
}

export function evaluateSpreadsheet(
  rawCells: Record<string, string>,
): Record<string, string | number> {
  const normalized: Record<string, string> = {};
  for (const [k, v] of Object.entries(rawCells)) {
    normalized[k.trim().toUpperCase()] = String(v ?? "").trim();
  }

  const memo = new Map<string, number>();
  const visiting = new Set<string>();

  const evalNumeric = (key: string): number => {
    const ref = key.toUpperCase();
    if (memo.has(ref)) return memo.get(ref)!;
    if (visiting.has(ref)) return 0; // break cycle deterministically

    const raw = normalized[ref] ?? "";
    if (!raw) return 0;
    if (!raw.startsWith("=")) {
      const n = Number.parseFloat(raw);
      const v = Number.isNaN(n) ? 0 : n;
      memo.set(ref, v);
      return v;
    }

    visiting.add(ref);
    const parser = new FormulaParser(raw.slice(1), evalNumeric);
    const computed = parser.parse();
    visiting.delete(ref);
    memo.set(ref, computed);
    return computed;
  };

  const out: Record<string, string | number> = {};
  for (const [k, raw] of Object.entries(normalized)) {
    if (raw.startsWith("=")) {
      out[k] = evalNumeric(k);
    } else {
      const asNum = Number(raw);
      out[k] = raw !== "" && !Number.isNaN(asNum) ? asNum : raw;
    }
  }
  return out;
}

// ============================================================================
// Calendar (.ics), Contacts (.vcf) & Bilingual Technical Translation
// ============================================================================

export interface CalendarEventSpec {
  id: string;
  title: string;
  startsAtIso: string;
  endsAtIso: string;
  location: string;
  description: string;
}

export function buildIcsCalendar(events: CalendarEventSpec[]): string {
  const fmtDate = (iso: string) =>
    iso.replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
  const lines = [
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    "PRODID:-//Aphrody Sovereign Workspace//M3 Calendar//EN",
    "CALSCALE:GREGORIAN",
  ];
  for (const ev of events) {
    lines.push(
      "BEGIN:VEVENT",
      `UID:${ev.id}@aphrody.com`,
      `DTSTAMP:${fmtDate(new Date().toISOString())}`,
      `DTSTART:${fmtDate(ev.startsAtIso)}`,
      `DTEND:${fmtDate(ev.endsAtIso)}`,
      `SUMMARY:${ev.title}`,
      `LOCATION:${ev.location}`,
      `DESCRIPTION:${ev.description.replace(/\r?\n/g, "\\n")}`,
      "END:VEVENT",
    );
  }
  lines.push("END:VCALENDAR");
  return `${lines.join("\r\n")}\r\n`;
}

export interface ContactCardSpec {
  id: string;
  fullName: string;
  email: string;
  organization: string;
  role: string;
  phone?: string;
}

export function buildVcfContacts(contacts: ContactCardSpec[]): string {
  return contacts
    .map((c) =>
      [
        "BEGIN:VCARD",
        "VERSION:3.0",
        `FN:${c.fullName}`,
        `ORG:${c.organization}`,
        `TITLE:${c.role}`,
        `EMAIL;TYPE=INTERNET:${c.email}`,
        ...(c.phone ? [`TEL;TYPE=CELL:${c.phone}`] : []),
        "END:VCARD",
      ].join("\r\n"),
    )
    .join("\r\n")
    .concat("\r\n");
}

const FR_EN_GLOSSARY: ReadonlyArray<[string, string]> = [
  ["moteur de recherche souverain", "sovereign search engine"],
  ["système de fichiers distribué", "distributed file system"],
  ["zéro dépendance externe", "zero external dependencies"],
  ["temps de réponse", "response latency"],
  ["base de données", "database"],
  ["feuille de calcul", "spreadsheet"],
  ["traitement de texte", "word processor"],
  ["intelligence locale", "local inference"],
  ["architecture", "architecture"],
  ["recherche", "search"],
  ["paquets", "packages"],
  ["fichiers", "files"],
  ["courrier", "mail"],
  ["agenda", "calendar"],
  ["bonjour", "hello"],
  ["bienvenue", "welcome"],
];

export function translateSovereignText(
  text: string,
  sourceLang: "fr" | "en",
  targetLang: "fr" | "en",
): { translated: string; matchedTerms: number } {
  if (sourceLang === targetLang || !text.trim()) {
    return { translated: text, matchedTerms: 0 };
  }

  let output = text;
  let matchedTerms = 0;

  for (const [fr, en] of FR_EN_GLOSSARY) {
    const from = sourceLang === "fr" ? fr : en;
    const to = sourceLang === "fr" ? en : fr;
    const re = new RegExp(from.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "gi");
    if (re.test(output)) {
      matchedTerms++;
      output = output.replace(re, to);
    }
  }

  return { translated: output, matchedTerms };
}
