//! Pure-Rust full-stack Google Design (`https://design.google/`), `DESIGN.md`,
//! Chrome `modern-web-guidance`, and Google Stitch (`https://stitch.withgoogle.com/`) engine.
//!
//! Combines `ureq` + `roxmltree` + `html5ever` + `serde_yaml_ng` + `image` into one
//! cross-platform zero-unwrap crate without subprocess `curl`/`wget` dependencies.

pub mod design_md;
pub mod http_mirror;
pub mod modern_web_guidance;
pub mod screen_capture;
pub mod stitch_client;
pub mod web_standards;

/// Live-synced `assets/os/design_google_sitemap_snapshot.toml` fallback snapshot.
pub const BUNDLED_DESIGN_SITEMAP_TOML: &str =
    include_str!("../../../assets/os/design_google_sitemap_snapshot.toml");

/// Authoritative root `DESIGN.md` bundled at compile time.
pub const BUNDLED_ROOT_DESIGN_MD: &str = include_str!("../../../DESIGN.md");

/// Authoritative 3-tier Material 3 Expressive + Google Sans Flex token TOML.
pub const BUNDLED_GOOGLE_DESIGN_TOKENS_TOML: &str =
    include_str!("../../../assets/os/google_design_tokens.toml");
