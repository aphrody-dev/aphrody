// Run: bun add -d paper paperjs-offset @resvg/resvg-js && bun tooling/bun/glyphs/aphrody-character.ts assets/icons
// Builds an Aphrody character glyph on the Material Symbols grid (viewBox 0 -960 960 960,
// 80-unit strokes = weight 400 at 24px, 40-unit gaps, rounded joins like the Rounded set).
import paper from "paper";
import { PaperOffset } from "paperjs-offset";
import { Resvg } from "@resvg/resvg-js";

paper.setup(new paper.Size(960, 960));
const P = (d: string) => new paper.CompoundPath(d);
const circle = (x: number, y: number, r: number) => new paper.Path.Circle(new paper.Point(x, y), r);
const offset = (item: paper.PathItem, d: number) =>
  PaperOffset.offset(item as paper.Path, d, { join: "round", insert: false }) as paper.PathItem;
const round = (item: paper.PathItem, r: number) => offset(offset(item, -r), r);

// Head: hair mass with the face as negative space under a sweeping fringe.
const head = circle(480, -650, 170);
const face = P("M366,-640 A122 122 0 1 0 598,-690 Q540,-660 470,-655 Q410,-650 366,-640 Z");

// Long side ponytail over the viewer's left shoulder, joined to the hair mass.
const ponytail = P("M330,-700 C268,-600 262,-440 332,-300 C356,-276 386,-292 380,-322 C344,-420 346,-560 392,-650 Z");

// Jacket shoulders, same proportions as the official `person` bust.
const body = round(P("M160,-160 V-272 Q160,-306 177.5,-334.5 T224,-378 Q286,-409 350,-424.5 T480,-440 Q546,-440 610,-424.5 T736,-378 Q765,-363 782.5,-334.5 T800,-272 V-160 Z"), 30);

// Open collar: two shirt triangles either side of the tie, which runs into the jacket at the V point.
const collar = P("M350,-438 H610 L480,-220 Z");
const tie = P("M446,-438 H514 L500,-400 L530,-290 L480,-220 L430,-290 L460,-400 Z");

let silhouette: paper.PathItem = head.unite(ponytail);
const bodyCut = (body as paper.PathItem).subtract(offset(ponytail, 40));
silhouette = silhouette.subtract(face).unite(bodyCut);
const shirt = collar.subtract(tie);
const filled = silhouette.subtract(shirt);

// Outlined variant: hair stays a solid mass (as in face_4); the jacket becomes an 80-unit ring with lapel bands and a solid tie.
const ring = (shape: paper.PathItem) => shape.subtract(offset(shape, -80));
const headPart = head.unite(ponytail).subtract(face);
const lapels = offset(collar, 80).intersect(bodyCut).subtract(collar);
const outlined = headPart.unite(bodyCut.subtract(offset(body, -80).subtract(offset(ponytail, 120)).subtract(offset(collar.unite(tie), 80)))).unite(tie.intersect(bodyCut)).subtract(shirt);

const toSvg = (item: paper.PathItem) =>
  `<svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 -960 960 960" width="24"><path d="${(item as paper.PathItem).getPathData(undefined as never, 1)}"/></svg>`;
const out = process.argv[2] ?? ".";
await Bun.write(`${out}/aphrody_character_fill.svg`, toSvg(filled));
await Bun.write(`${out}/aphrody_character.svg`, toSvg(outlined));

// Preview sheet: both variants at 24px, 48px and large, next to the official `person`.
const official = (await (await fetch("https://fonts.gstatic.com/s/i/short-term/release/materialsymbolsrounded/person/fill1/24px.svg")).text()).match(/d="([^"]+)"/)![1];
const cell = (d: string, x: number, y: number, s: number) =>
  `<svg x="${x}" y="${y}" width="${s}" height="${s}" viewBox="0 -960 960 960"><path d="${d}"/></svg>`;
const ds = [filled.pathData, outlined.pathData, official];
let body2 = "";
ds.forEach((d, i) => {
  body2 += cell(d, 20 + i * 330, 20, 300);
  body2 += cell(d, 20 + i * 330, 340, 48);
  body2 += cell(d, 90 + i * 330, 352, 24);
});
const sheet = `<svg xmlns="http://www.w3.org/2000/svg" width="1010" height="420"><rect width="100%" height="100%" fill="#fff"/>${body2}</svg>`;
await Bun.write(`${out}/aphrody_character_preview.png`, new Resvg(sheet).render().asPng());
console.log(filled.pathData.length, outlined.pathData.length);
