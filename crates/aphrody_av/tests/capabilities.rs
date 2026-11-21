use aphrody_av::caps::{codec_caps, format_caps, markdown};
use aphrody_av::{Codec, Format};

const DOC: &str = "docs/capabilities.md";

/// `docs/capabilities.md` is generated: run with `UPDATE_CAPABILITIES=1` to rewrite it.
#[test]
fn capability_doc_matches_the_code() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DOC);
    let generated = markdown();
    if std::env::var_os("UPDATE_CAPABILITIES").is_some() {
        std::fs::write(&path, &generated).expect("write doc");
    }
    let committed = std::fs::read_to_string(&path).expect("docs/capabilities.md");
    assert_eq!(
        committed, generated,
        "docs/capabilities.md is stale; regenerate with UPDATE_CAPABILITIES=1 cargo test -p aphrody_av --test capabilities"
    );
}

#[test]
fn matrix_is_consistent() {
    let codecs = codec_caps();
    assert_eq!(codecs.len(), Codec::ALL.len());
    for caps in &codecs {
        assert!(
            caps.decode != "no" || caps.encode,
            "{} can do nothing",
            caps.name
        );
        if !caps.encode {
            assert!(
                caps.containers.is_empty(),
                "{} claims mux without an encoder",
                caps.name
            );
        }
    }
    let formats = format_caps();
    assert_eq!(formats.len(), Format::ALL.len());
    assert!(
        codecs
            .iter()
            .find(|c| c.name == "h264")
            .is_some_and(|c| c.decode == "yes" && c.encode && c.containers.contains(&"mp4"))
    );
    assert!(
        codecs
            .iter()
            .find(|c| c.name == "mpeg4")
            .is_some_and(|c| c.decode == "no" && c.encode)
    );
    assert_eq!(Format::from_name("MKV"), Some(Format::Matroska));
    assert_eq!(Format::from_path("a/b/c.webm"), Some(Format::WebM));
    assert_eq!(Codec::from_name("libx264"), Some(Codec::H264));
}
