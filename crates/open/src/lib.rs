//! open: one framework. Each surface is a crate of this workspace (or an upstream crate pinned in the workspace
//! manifest) and is re-exported here under a stable name, behind a feature.
//!
//! | Module | Crate | What it is |
//! | --- | --- | --- |
//! | `engines` | browser-engines | engine registry, headless launcher, CDP client, SSRF guard |
//! | `crawl` | crawl | robots-aware crawler, FTS5 shards, browser mirror, guarded fetch |
//! | `browse` | browse | agent browser: direct CDP and the relay of the Chrome extension |
//! | `tunnel` | tunnel | blind TLS tunnels |
//! | `router` | ai-router | multi-provider AI router |
//! | `tui` | tui | terminal UI on OpenTUI |
//! | `ui` | rune | GPU UI (Rune) |
//! | `abi` | c-abi | common C ABI primitives |

#[cfg(feature = "router")]
pub use ai_router as router;
#[cfg(feature = "browse")]
pub use ay_open_browse as browse;
#[cfg(feature = "engines")]
pub use ay_open_browser_engines as engines;
#[cfg(feature = "c-abi")]
pub use ay_open_c_abi as abi;
#[cfg(feature = "crawl")]
pub use ay_open_crawl as crawl;
#[cfg(feature = "tui")]
pub use ay_open_tui as tui;
#[cfg(feature = "tunnel")]
pub use ay_open_tunnel as tunnel;

/// Rune and its component kit and motion crates.
#[cfg(feature = "ui")]
pub mod ui {
    pub use rune::*;
    pub use rune_kit as kit;
    pub use rune_motion as motion;
}

/// The features this build carries, for `open --version` and diagnostics.
pub const FEATURES: &[&str] = &[
    #[cfg(feature = "engines")]
    "engines",
    #[cfg(feature = "crawl")]
    "crawl",
    #[cfg(feature = "browse")]
    "browse",
    #[cfg(feature = "tunnel")]
    "tunnel",
    #[cfg(feature = "router")]
    "router",
    #[cfg(feature = "tui")]
    "tui",
    #[cfg(feature = "ui")]
    "ui",
    #[cfg(feature = "c-abi")]
    "c-abi",
];
