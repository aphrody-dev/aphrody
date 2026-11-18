//! Tunnels for open, on the `opentunnel` client (crates.io): TLS terminates in this process, the relay never sees
//! plaintext or the private key. A profile owns one tunnel; each route name maps to a local address.

pub use opentunnel::{Client, ClientOptions, Error, Event, Identity, ProxyProtocol, Result, Route, Routes, State, Status, Tunnel};

use url::Url;

/// A tunnel that is up: its identity (host name) and the running connection.
pub struct Up {
    pub identity: Identity,
    pub tunnel: Tunnel,
}

impl Up {
    /// The public URL of a route: `https://<route>.<hostname>`.
    pub fn url(&self, route: &str) -> String {
        format!("https://{route}.{}", self.identity.hostname)
    }
}

/// `name=host:port` pairs to routes.
pub fn parse_routes<S: AsRef<str>>(specs: &[S]) -> std::result::Result<Routes, String> {
    let mut routes = Routes::new();
    for spec in specs {
        let (name, target) = spec.as_ref().split_once('=').ok_or_else(|| format!("route {:?} must be name=host:port", spec.as_ref()))?;
        let route = Route::new(target);
        route.validate(name)?;
        routes.insert(name.to_string(), route);
    }
    Ok(routes)
}

/// Creates the profile's tunnel when it has none (or refreshes its certificate) and connects `routes`.
pub async fn up(profile: &str, routes: Routes, api: Option<Url>) -> Result<Up> {
    // The client's TLS uses ring; the provider is installed once per process.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = Client::new(ClientOptions { api, ..Default::default() });
    let identity = client.ensure(profile).await?;
    let tunnel = client.connect(profile, routes)?;
    Ok(Up { identity, tunnel })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes() {
        let routes = parse_routes(&["web=127.0.0.1:3000", "api=127.0.0.1:4000"]).unwrap();
        assert_eq!(routes.len(), 2);
        assert!(parse_routes(&["nope"]).is_err());
    }
}
