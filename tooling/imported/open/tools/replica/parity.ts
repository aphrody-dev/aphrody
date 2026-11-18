// Feature parity of a replica: weights must 3, should 2, could 1; clone yes 1, partial 0.5, no 0; `skip` rows and
// rows the original does not have are left out. With layout scores (0-100 per screen) the overall score is 80 %
// features and 20 % layout.
import type { Row } from "./recon";

const WEIGHT: Record<string, number> = { must: 3, should: 2, could: 1, p0: 3, p1: 2, p2: 1 };
const CREDIT: Record<string, number> = { yes: 1, done: 1, partial: 0.5, no: 0, "": 0, todo: 0 };

export type Parity = {
  feature: number;
  layout: number | null;
  overall: number;
  counted: number;
  mustDone: number;
  mustTotal: number;
  missing: Row[];
  byArea: { area: string; score: number; features: number }[];
};

export function parity(rows: Row[], layout: number[] = []): Parity {
  const counted = rows
    .filter((r) => !["no", "n", "false"].includes((r.original || "yes").toLowerCase()) && !r.clone.toLowerCase().startsWith("skip"))
    .map((r) => ({ r, w: WEIGHT[r.priority.toLowerCase()] ?? 1, c: CREDIT[r.clone.toLowerCase()] ?? 0 }));
  const total = counted.reduce((n, x) => n + x.w, 0);
  const got = counted.reduce((n, x) => n + x.w * x.c, 0);
  const feature = total ? (100 * got) / total : 0;
  const areas = new Map<string, { g: number; t: number; n: number }>();
  for (const x of counted) {
    const a = areas.get(x.r.area || "(no area)") ?? { g: 0, t: 0, n: 0 };
    a.g += x.w * x.c;
    a.t += x.w;
    a.n++;
    areas.set(x.r.area || "(no area)", a);
  }
  const lay = layout.length ? layout.reduce((a, b) => a + b, 0) / layout.length : null;
  const round = (v: number) => Math.round(v * 10) / 10;
  return {
    feature: round(feature),
    layout: lay === null ? null : round(lay),
    overall: round(lay === null ? feature : 0.8 * feature + 0.2 * lay),
    counted: counted.length,
    mustDone: counted.filter((x) => x.w === 3 && x.c === 1).length,
    mustTotal: counted.filter((x) => x.w === 3).length,
    missing: counted
      .filter((x) => x.c < 1)
      .sort((a, b) => b.w - a.w || a.c - b.c || a.r.area.localeCompare(b.r.area))
      .map((x) => x.r),
    byArea: [...areas].map(([area, a]) => ({ area, score: a.t ? round((100 * a.g) / a.t) : 0, features: a.n })).sort((a, b) => a.score - b.score),
  };
}

export function parityMarkdown(p: Parity): string {
  const L = [`# Parity`, "", `Overall ${p.overall} (features ${p.feature}${p.layout === null ? "" : `, layout ${p.layout}`}); must-haves done ${p.mustDone}/${p.mustTotal}.`, "", "| area | score | features |", "|---|---|---|"];
  for (const a of p.byArea) L.push(`| ${a.area} | ${a.score} | ${a.features} |`);
  L.push("", "## Build next", "");
  for (const r of p.missing.slice(0, 5)) L.push(`- [${r.priority}] ${r.feature} (${r.area})${r.clone === "partial" ? ", partial" : ""}`);
  return L.join("\n") + "\n";
}
