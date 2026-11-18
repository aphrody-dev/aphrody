//! Browse for open.
//!
//! The page tools (snapshot refs, actionability, human-like input, dialogs, uploads, emulation), captures
//! (screenshots, network, recordings) and the script runner are the `browse-*` crates of anomalyco/browse, pinned in
//! the workspace. Two ways to give them a browser:
//!
//! - [`DirectBrowser`]: tabs of any Chromium-family browser over its DevTools WebSocket (Chrome for Testing, the
//!   Aphrody browser, a headless shell). No extension, no relay; the engine stays outside this workspace.
//! - the relay ([`relay_tools`], [`serve_relay`]): the user's own browser through the MV3 extension of `extension/`.

mod direct;
mod mcp;

use std::sync::Arc;

pub use browse_capture as capture;
pub use browse_core as core;
pub use browse_page as page;
pub use browse_relay as relay;
pub use browse_script as script;
pub use direct::DirectBrowser;
pub use mcp::serve_stdio;

use browse_core::Tool;

/// Every tool of the relay build: the relay's own (tabs, handoff, status), page, capture and script tools.
pub fn relay_tools() -> Vec<Arc<dyn Tool>> {
    let mut tools = browse_relay::tools();
    tools.extend(browse_page::tools());
    tools.extend(browse_capture::tools());
    let script = browse_script::tools(&tools);
    tools.extend(script);
    tools
}

/// The tools of a [`DirectBrowser`] session: its own `tabs` tool plus page, capture and script tools.
pub fn direct_tools() -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = vec![Arc::new(direct::TabsTool)];
    tools.extend(browse_page::tools());
    tools.extend(browse_capture::tools());
    let script = browse_script::tools(&tools);
    tools.extend(script);
    tools
}

/// Pairs an unpacked extension (its `dist` directory) with the relay of this user: mints a pairing code and writes the
/// pairing file the extension reads when it starts. Returns the code in the form the extension's Options page takes.
pub fn pair_unpacked(extension_dir: &std::path::Path, port: Option<u16>) -> std::io::Result<String> {
    let paths = browse_relay::paths::Paths::from_env();
    let code = browse_relay::pairing::mint_pairing_code(&paths, 10 * 60 * 1000, None)?;
    browse_relay::pairing::write_extension_pairing(extension_dir, &code, port)?;
    Ok(browse_relay::pairing::format_pairing_code(&code))
}

/// Runs the relay the Chrome extension pairs with, until interrupted.
pub async fn serve_relay(options: browse_relay::ServeOptions) -> anyhow::Result<()> {
    browse_relay::serve(options, relay_tools()).await
}
