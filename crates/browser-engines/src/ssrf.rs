//! SSRF policy for a crawler and for the pages of a headless browser: loopback, RFC 1918, link-local, cloud
//! metadata, CGNAT, benchmarking, documentation, multicast and every IPv4-embedding IPv6 form are refused, and a host
//! that does not resolve is refused too (fail closed).

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use url::{Host, Url};

#[derive(Debug, thiserror::Error)]
pub enum SsrfError {
    #[error("SSRF guard: unsupported protocol \"{0}\"")]
    Protocol(String),
    #[error("SSRF guard: host \"{0}\" is explicitly blocked")]
    Blocked(String),
    #[error("SSRF guard: host \"{0}\" is on an internal network")]
    Internal(String),
    #[error("SSRF guard: \"{host}\" is or resolves to a private or forbidden address {addr}")]
    Forbidden { host: String, addr: IpAddr },
    #[error("SSRF guard: host \"{0}\" did not resolve (fail closed)")]
    Unresolved(String),
    #[error("SSRF guard: URL has no host")]
    NoHost,
}

#[derive(Debug, Clone, Default)]
pub struct SsrfOptions {
    pub allow_private_network: bool,
    pub allowed_hosts: Vec<String>,
    pub blocked_hosts: Vec<String>,
}

pub fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, d] = ip.octets();
    a == 0
        || a == 127
        || a == 10
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 169 && b == 254)
        || (a == 100 && (64..=127).contains(&b))
        || (a == 198 && (b == 18 || b == 19))
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 0 && c == 0 && d != 9 && d != 10)
        || a >= 224
}

fn v4_of(hi: u16, lo: u16) -> Ipv4Addr {
    Ipv4Addr::new((hi >> 8) as u8, (hi & 255) as u8, (lo >> 8) as u8, (lo & 255) as u8)
}

/// Special-purpose IPv6 (RFC 6890) is forbidden; every IPv4-embedding form (compatible, mapped, NAT64, 6to4) is
/// judged on its embedded IPv4; Teredo and documentation ranges are forbidden.
pub fn is_forbidden_ipv6(ip: Ipv6Addr) -> bool {
    let [a, b, c, d, e, f, g, i] = ip.segments();
    if a == 0 && b == 0 && c == 0 && d == 0 && e == 0 {
        if f == 0 || f == 0xffff {
            return (g == 0 && i <= 1) || is_forbidden_ipv4(v4_of(g, i));
        }
        return true;
    }
    if a == 0x64 && b == 0xff9b {
        return !(c == 0 && d == 0 && e == 0 && f == 0) || is_forbidden_ipv4(v4_of(g, i));
    }
    if a == 0x2002 {
        return is_forbidden_ipv4(v4_of(b, c));
    }
    (a == 0x2001 && b == 0)
        || (a == 0x2001 && b == 0x0db8)
        || (a == 0x2001 && b < 0x200)
        || (a == 0x0100 && b == 0 && c == 0 && d == 0)
        || (a & 0xfe00) == 0xfc00
        || (a & 0xffc0) == 0xfe80
        || (a & 0xffc0) == 0xfec0
        || (a & 0xff00) == 0xff00
}

pub fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_ipv4(v4),
        IpAddr::V6(v6) => is_forbidden_ipv6(v6),
    }
}

#[derive(Debug, Clone, Default)]
pub struct SsrfGuard {
    allow_private_network: bool,
    allowed: HashSet<String>,
    blocked: HashSet<String>,
}

impl SsrfGuard {
    pub fn new(options: SsrfOptions) -> Self {
        let lower = |hosts: Vec<String>| hosts.into_iter().map(|h| h.to_ascii_lowercase()).collect();
        Self { allow_private_network: options.allow_private_network, allowed: lower(options.allowed_hosts), blocked: lower(options.blocked_hosts) }
    }

    pub fn allows_private_network(&self) -> bool {
        self.allow_private_network
    }

    /// Checks scheme, host name and IP literals. Names that need DNS are checked by [`SsrfGuard::resolve`].
    pub fn check_url(&self, url: &Url) -> Result<(), SsrfError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(SsrfError::Protocol(url.scheme().to_string()));
        }
        let host = url.host().ok_or(SsrfError::NoHost)?;
        let name = url.host_str().unwrap_or_default().trim_matches(['[', ']']).to_ascii_lowercase();
        if self.blocked.contains(&name) {
            return Err(SsrfError::Blocked(name));
        }
        if self.allow_private_network || self.allowed.contains(&name) {
            return Ok(());
        }
        match host {
            Host::Ipv4(ip) if is_forbidden_ipv4(ip) => Err(SsrfError::Forbidden { host: name, addr: ip.into() }),
            Host::Ipv6(ip) if is_forbidden_ipv6(ip) => Err(SsrfError::Forbidden { host: name, addr: ip.into() }),
            Host::Domain(_) if is_internal_name(&name) => Err(SsrfError::Internal(name)),
            _ => Ok(()),
        }
    }

    /// Validates the URL and resolves its host; every address must be allowed, the result is what a connector pins.
    pub async fn resolve(&self, url: &Url) -> Result<Vec<SocketAddr>, SsrfError> {
        self.check_url(url)?;
        let port = url.port_or_known_default().unwrap_or(80);
        let name = url.host_str().unwrap_or_default().trim_matches(['[', ']']).to_ascii_lowercase();
        if let Ok(ip) = name.parse::<IpAddr>() {
            return Ok(vec![SocketAddr::new(ip, port)]);
        }
        let trusted = self.allow_private_network || self.allowed.contains(&name);
        self.resolve_name(&name, port, trusted).await
    }

    /// Resolves a bare host name for a connector that has only the name and port.
    pub async fn resolve_name(&self, name: &str, port: u16, trusted: bool) -> Result<Vec<SocketAddr>, SsrfError> {
        let addrs: Vec<SocketAddr> = tokio::net::lookup_host((name, port)).await.map_err(|_| SsrfError::Unresolved(name.to_string()))?.collect();
        if addrs.is_empty() {
            return Err(SsrfError::Unresolved(name.to_string()));
        }
        if !trusted {
            if let Some(bad) = addrs.iter().find(|a| is_forbidden_ip(a.ip())) {
                return Err(SsrfError::Forbidden { host: name.to_string(), addr: bad.ip() });
            }
        }
        Ok(addrs)
    }

    /// Full validation of one URL: literals, names, then DNS.
    pub async fn validate(&self, url: &Url) -> Result<(), SsrfError> {
        self.resolve(url).await.map(|_| ())
    }

    pub fn trusts(&self, name: &str) -> bool {
        self.allow_private_network || self.allowed.contains(&name.to_ascii_lowercase())
    }
}

fn is_internal_name(name: &str) -> bool {
    name == "localhost"
        || name.ends_with(".localhost")
        || name.ends_with(".local")
        || name.ends_with(".internal")
        || name == "instance-data"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard() -> SsrfGuard {
        SsrfGuard::default()
    }

    #[test]
    fn ipv4_ranges() {
        for ip in ["127.0.0.1", "10.1.2.3", "172.16.0.1", "192.168.1.1", "169.254.169.254", "100.64.0.1", "0.0.0.0", "224.0.0.1"] {
            assert!(is_forbidden_ipv4(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "93.184.216.34", "172.32.0.1"] {
            assert!(!is_forbidden_ipv4(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn ipv6_embedding_forms() {
        for ip in ["::1", "::", "fe80::1", "fc00::1", "::ffff:127.0.0.1", "64:ff9b::7f00:1", "2002:7f00:1::", "2001::1", "ff02::1"] {
            assert!(is_forbidden_ipv6(ip.parse().unwrap()), "{ip}");
        }
        assert!(!is_forbidden_ipv6("2606:4700:4700::1111".parse().unwrap()));
        assert!(!is_forbidden_ipv6("::ffff:8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn urls() {
        let g = guard();
        for bad in ["ftp://example.com/", "http://localhost/", "http://a.internal/", "http://127.0.0.1:8080/", "http://[::1]/", "http://metadata.internal/"] {
            assert!(g.check_url(&Url::parse(bad).unwrap()).is_err(), "{bad}");
        }
        assert!(g.check_url(&Url::parse("https://example.com/").unwrap()).is_ok());
        let open = SsrfGuard::new(SsrfOptions { allow_private_network: true, ..Default::default() });
        assert!(open.check_url(&Url::parse("http://127.0.0.1/").unwrap()).is_ok());
        let blocked = SsrfGuard::new(SsrfOptions { blocked_hosts: vec!["example.com".into()], ..Default::default() });
        assert!(blocked.check_url(&Url::parse("https://example.com/").unwrap()).is_err());
    }
}
