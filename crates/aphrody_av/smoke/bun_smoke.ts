// wasm-bindgen smoke test: AV_WASM=<dir from wasm-bindgen --target nodejs> bun smoke/bun_smoke.ts <clip.mp4>
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const av = require(`${process.env.AV_WASM}/aphrody_av.js`);
const data = new Uint8Array(readFileSync(process.argv[2]));

function check(condition: unknown, message: string): asserts condition {
  if (!condition) {
    console.error(`FAIL: ${message}`);
    process.exit(1);
  }
}

console.log("aphrody-av wasm", av.version());

const info = JSON.parse(av.probe(data, "clip.mp4"));
check(info.format === "mp4", `format ${info.format}`);
check(info.streams[0].codec === "h264" && info.streams[0].width === 64 && info.streams[0].height === 48 && info.streams[0].frames === 12, "video stream");
check(info.streams[1].codec === "aac" && info.streams[1].sample_rate === 48000 && info.streams[1].channels === 2, "audio stream");
console.log("probe:", info.format, info.streams.map((s: any) => `${s.kind}/${s.codec}`).join(","));

const decoder = new av.FrameDecoder(data, "clip.mp4");
let frames = 0;
let lastPts = -1;
let brightest = 0;
for (let frame = decoder.next_frame(); frame !== undefined; frame = decoder.next_frame()) {
  check(frame.width === 64 && frame.height === 48, "frame size");
  const rgba: Uint8Array = frame.rgba;
  check(rgba.length === 64 * 48 * 4, `rgba length ${rgba.length}`);
  check(frame.pts_seconds > lastPts, "pts increases");
  lastPts = frame.pts_seconds;
  brightest = Math.max(brightest, ...rgba.slice(0, 256));
  frames++;
  frame.free();
}
check(frames === 12, `decoded ${frames} frames`);
check(brightest > 100, "decoded pictures carry content");
console.log("decode:", frames, "frames, last pts", lastPts.toFixed(3));

const audio = av.decode_audio(data, "clip.mp4");
check(audio.sample_rate === 48000 && audio.channels === 2 && audio.samples.length > 30000, `audio ${audio.sample_rate} ${audio.channels} ${audio.samples.length}`);
console.log("audio:", audio.sample_rate, "Hz,", audio.channels, "ch,", audio.samples.length / audio.channels, "frames");

const converted = av.convert_video(data, "clip.mp4", JSON.stringify({ formats: ["svg", "md"], fps: 5, width: 32 }));
const paths: string[] = [];
for (let i = 0; i < converted.file_count; i++) paths.push(converted.file_path(i));
check(paths.includes("animated.svg") && paths.includes("video.md"), `paths ${paths}`);
const svg = new TextDecoder().decode(converted.file_bytes(paths.indexOf("animated.svg")));
check(svg.startsWith("<svg"), "animated svg");
console.log("convert:", converted.frames, "frames,", paths.length, "files");

let rejected = false;
try {
  av.probe(new Uint8Array(200).fill(0x5a), "x.bin");
} catch (error) {
  rejected = true;
  console.log("garbage rejected:", String(error).slice(0, 80));
}
check(rejected, "garbage input must throw");
check(JSON.parse(av.capabilities()).codecs.length > 20, "capabilities");
console.log("bun smoke: ok");
