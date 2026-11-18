//! The HTTP client of every fetch in this crate: addresses are checked at connect time by the SSRF guard, so DNS
//! rebinding between a check and a connection cannot reach a private address, and every redirect hop is checked too.

use std::error::Error as StdError;
use std::sync::Arc;
use std::time::Duration;

use ay_open_browser_engines::SsrfGuard;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::redirect::Policy;

struct GuardedResolver(Arc<SsrfGuard>);

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let guard = self.0.clone();
        Box::pin(async move {
            let host = name.as_str().to_string();
            let trusted = guard.trusts(&host);
            let addrs = guard.resolve_name(&host, 0, trusted).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)?;
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// Installs the ring crypto provider once; every TLS client of the workspace shares it.
pub fn init_tls() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub fn client(guard: Arc<SsrfGuard>, user_agent: &str, timeout: Duration) -> Result<reqwest::Client, reqwest::Error> {
    client_builder(guard, user_agent, timeout).build()
}

pub fn client_builder(guard: Arc<SsrfGuard>, user_agent: &str, timeout: Duration) -> reqwest::ClientBuilder {
    init_tls();
    let hop_guard = guard.clone();
    reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(10))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .dns_resolver(Arc::new(GuardedResolver(guard)))
        .redirect(Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("more than 5 redirects");
            }
            match hop_guard.check_url(attempt.url()) {
                Ok(()) => attempt.follow(),
                Err(error) => attempt.error(error),
            }
        }))
}
