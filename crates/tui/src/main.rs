//! open-tui: full-screen terminal chat over the router. Configuration is the router's (AI_ROUTER_CATALOG,
//! AI_ROUTER_LOCAL, provider credentials in the environment).

use std::sync::Arc;

use ai_router::{Config, Router};

fn main() {
    let config = Config {
        local: std::env::var("AI_ROUTER_LOCAL").ok().map(|v| v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()),
        ..Config::default()
    };
    let router = match Router::open(&config) {
        Ok(router) => Arc::new(router),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = ay_open_tui::chat::run(router) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
