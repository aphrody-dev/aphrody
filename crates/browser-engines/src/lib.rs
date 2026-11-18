//! Browser engines for open.
//!
//! - [`registry`]: every browsing, automation and crawl engine with its upstream, how it is installed and which
//!   protocol it speaks; the JS and Python crawl libraries carry the Rust crate that replaces them.
//! - [`launch`]: starts a headless Chromium-family browser and returns its DevTools WebSocket.
//! - [`cdp`]: an async Chrome DevTools Protocol client (flat sessions, events, request interception).
//! - [`ssrf`]: the address policy every outgoing fetch of a crawl or a browser page goes through.
//!
//! The engine itself is never reimplemented here: a browser (Chrome for Testing, the Aphrody browser, Obscura,
//! Lightpanda, ...) is found or launched and driven over CDP.

pub mod cdp;
pub mod launch;
pub mod registry;
pub mod ssrf;

pub use cdp::{CdpError, Client, Event, Session};
pub use launch::{LaunchOptions, Launched};
pub use registry::{Engine, Install, Protocol};
pub use ssrf::{SsrfError, SsrfGuard};
