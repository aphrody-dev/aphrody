//! End to end: a 2 s TOML composition (shapes, text, spring, transition, captions) rendered
//! frame by frame must yield exactly 60 frames that change over time and repeat byte for byte.

use std::collections::BTreeMap;

use aphrody_media::{Composition, NoAssets, Renderer, Timeline};

const COMPOSITION: &str = r##"
id = "intro"
width = 160
height = 90
fps = 30
background = "#0b1020"

[variables]
title = "Aphrody"

[[tracks]]
id = "background"

[[tracks.clips]]
id = "panel"
start = 0
duration = 2
layer = { type = "shape", shape = "rect", radius = 12, fill = { from = "#1d4ed8", to = "#9333ea", angle = 45 } }
width = 120
height = 60
animate = [
  { property = "scale", keyframes = [
    { at = 0, value = 0.2, ease = { spring = { stiffness = 300.0, damping_ratio = 0.7 } } },
    { at = 1, value = 1.0 } ] },
]

[[tracks]]
id = "foreground"

[[tracks.clips]]
id = "title"
start = "10f"
end = 2
layer = { type = "text", text = "{{title}}", size = 20, weight = "bold" }
transition_in = { kind = "dissolve", duration = 0.5 }
animate = [
  { property = "y", keyframes = [ { at = 0, value = 70, ease = "ease_out" }, { at = 0.5, value = 45 } ] },
]

[[tracks.clips]]
id = "caption"
start = 1
duration = 1
layer = { type = "captions", size = 9, highlight = "#facc15", cues = [
  { start = 0, end = 1, words = [
    { start = 0, end = 0.5, text = "frame" }, { start = 0.5, end = 1, text = "exact" } ] },
] }
"##;

fn intro_timeline() -> Timeline {
    let composition = Composition::from_toml(COMPOSITION).expect("composition parses");
    Timeline::new(&composition, &BTreeMap::new()).expect("timeline resolves")
}

fn differing_pixels(a: &[u8], b: &[u8]) -> usize {
    a.chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(left, right)| left != right)
        .count()
}

#[test]
fn two_second_composition_renders_sixty_changing_deterministic_frames() {
    let timeline = intro_timeline();
    assert_eq!(timeline.duration_frames(), 60);
    let mut renderer = Renderer::new(timeline, NoAssets);

    let mut frames = Vec::new();
    for frame in 0..renderer.timeline().duration_frames() {
        frames.push(renderer.render_frame(frame).expect("frame renders"));
    }
    assert_eq!(frames.len(), 60);
    assert!(frames.iter().all(|frame| frame.width() == 160 && frame.height() == 90));

    let background = frames[0].data()[..4].to_vec();
    let total = 160 * 90;
    let panel_early = differing_pixels(frames[0].data(), frames[15].data());
    let panel_late = differing_pixels(frames[15].data(), frames[59].data());
    assert!(panel_early > total / 20, "spring growth changed {panel_early} px");
    assert!(panel_late > total / 100, "title, caption and settle changed {panel_late} px");
    assert!(
        frames[0].data().chunks_exact(4).all(|pixel| pixel[..3] == background[..3] || pixel[3] == 255),
        "frame 0 only holds opaque pixels"
    );

    let title_end = differing_pixels(frames[9].data(), frames[10].data());
    let dissolve_mid = differing_pixels(frames[10].data(), frames[18].data());
    assert!(dissolve_mid > title_end, "the dissolve reveals the title progressively");

    let caption_first = differing_pixels(frames[44].data(), frames[46].data());
    let caption_second = differing_pixels(frames[44].data(), frames[52].data());
    assert!(caption_first > 0 && caption_second > 0, "caption highlight moves with the words");

    let mut again = Renderer::new(intro_timeline(), NoAssets);
    for frame in [0_u64, 17, 33, 59] {
        let repeat = again.render_frame(frame).expect("repeat render");
        assert_eq!(repeat.data(), frames[frame as usize].data(), "frame {frame} is deterministic");
    }
}

#[test]
fn variable_override_changes_only_the_text_pixels() {
    let composition = Composition::from_toml(COMPOSITION).expect("parse");
    let overrides = BTreeMap::from([("title".to_string(), "Nie".to_string())]);
    let mut default_renderer = Renderer::new(
        Timeline::new(&composition, &BTreeMap::new()).expect("default"),
        NoAssets,
    );
    let mut overridden = Renderer::new(
        Timeline::new(&composition, &overrides).expect("override"),
        NoAssets,
    );
    let first = default_renderer.render_frame(40).expect("default frame");
    let second = overridden.render_frame(40).expect("override frame");
    let changed = differing_pixels(first.data(), second.data());
    assert!(changed > 20 && changed < 160 * 90 / 4, "only the title differs, got {changed} px");
}

#[cfg(feature = "video-encode")]
#[test]
fn mp4_export_holds_one_sample_per_frame() {
    use aphrody_media::Mp4Encoder;
    use craft_media::isobmff::{ReadSeekSource, open};

    let timeline = intro_timeline();
    let frames = timeline.duration_frames();
    let (fps_num, fps_den) = timeline.fps();
    let mut renderer = Renderer::new(timeline, NoAssets);
    let mut encoder = Mp4Encoder::new(160, 90, fps_num, fps_den, 20.0).expect("encoder");
    for frame in 0..frames {
        encoder
            .push_frame(&renderer.render_frame(frame).expect("frame"))
            .expect("push");
    }
    assert_eq!(encoder.frames(), 60);
    let bytes = encoder.finish().expect("finish");
    assert_eq!(&bytes[4..8], b"ftyp");

    let file = open(ReadSeekSource::new(std::io::Cursor::new(bytes)).expect("source")).expect("demux");
    let track = file.tracks.first().expect("video track");
    assert_eq!(track.samples.len(), 60);
    assert_eq!((track.width, track.height), (160, 90));
    assert_eq!(track.timescale, 30);
}
