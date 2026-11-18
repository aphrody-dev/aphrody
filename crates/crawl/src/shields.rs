//! Content shields: ad networks, tracking pixels and analytics beacons are never fetched, rendered or followed.

use url::Url;

pub const BLOCKED_TRACKER_DOMAINS: &[&str] = &[
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "google-analytics.com",
    "googletagmanager.com",
    "connect.facebook.net",
    "analytics.tiktok.com",
    "bat.bing.com",
    "hotjar.com",
    "criteo.com",
    "outbrain.com",
    "taboola.com",
    "scorecardresearch.com",
    "quantserve.com",
    "adnxs.com",
    "rubiconproject.com",
    "pubmatic.com",
    "openx.net",
];

#[derive(Debug, Clone, Default)]
pub struct Shields {
    extra: Vec<String>,
}

impl Shields {
    pub fn new(extra_blocked_domains: impl IntoIterator<Item = String>) -> Self {
        Self { extra: extra_blocked_domains.into_iter().map(|d| d.to_ascii_lowercase()).collect() }
    }

    pub fn should_block(&self, url: &Url) -> bool {
        let Some(host) = url.host_str() else { return false };
        let host = host.to_ascii_lowercase();
        let hit = |d: &str| host == d || host.strip_suffix(d).is_some_and(|rest| rest.ends_with('.'));
        if BLOCKED_TRACKER_DOMAINS.iter().copied().any(hit) || self.extra.iter().any(|d| hit(d)) {
            return true;
        }
        let path = url.path().to_ascii_lowercase();
        let query = url.query().unwrap_or_default();
        path.contains("/pagead/")
            || path.contains("/ads/")
            || path.ends_with("/pixel")
            || path.ends_with("/pixel.gif")
            || path.ends_with("/pixel.png")
            || path.ends_with("/beacon")
            || path.ends_with("/beacon.js")
            || (path.ends_with("/collect") && query.starts_with("v="))
    }

    pub fn should_block_str(&self, raw: &str) -> bool {
        Url::parse(raw).map(|u| self.should_block(&u)).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_trackers_and_subdomains() {
        let s = Shields::default();
        for blocked in ["https://www.google-analytics.com/collect?v=1", "https://x.doubleclick.net/a", "https://e.test/pagead/x", "https://e.test/a/pixel.gif"] {
            assert!(s.should_block_str(blocked), "{blocked}");
        }
        for ok in ["https://example.com/", "https://notdoubleclick.net/"] {
            assert!(!s.should_block_str(ok), "{ok}");
        }
        assert!(Shields::new(["ads.example".into()]).should_block_str("https://a.ads.example/x"));
    }
}
