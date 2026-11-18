// SPDX-License-Identifier: Apache-2.0
//! Network and cosmetic filtering by Brave's `adblock` crate (upstream, MPL-2.0); glue only.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdblockCheckResult {
    pub matched: bool,
    pub blocked: bool,
    pub important: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception: Option<String>,
}

/// ABP/uBO network and cosmetic rules plus hosts-format lines, all evaluated by `adblock::Engine`
/// (generic and per-site cosmetic rules, `#@#` exceptions, `$generichide` / `$elemhide`, procedural
/// and scriptlet filters stay inside the engine; nothing is parsed here).
pub struct AdblockEngine {
    engine: adblock::Engine,
}

/// Cosmetic filtering of one page: CSS selectors to hide (`display: none !important`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticResult {
    pub hide_selectors: Vec<String>,
    /// `$generichide` / `$elemhide` matched the page: generic `##` rules are not applied.
    pub generichide: bool,
}

/// A hosts-file line (`0.0.0.0`, `127.0.0.1`, `::`, `::1`, then spaces or tabs, then hostnames).
fn is_hosts_line(l: &str) -> bool {
    let mut it = l.split_whitespace();
    matches!(it.next(), Some("0.0.0.0" | "127.0.0.1" | "::" | "::1")) && it.next().is_some()
}

impl AdblockEngine {
    #[must_use]
    pub fn from_rules(rules_text: &str) -> Self {
        use adblock::lists::{FilterFormat, FilterSet, ParseOptions};
        // debug = true keeps the raw filter text, reported in `filter` / `exception`.
        let mut set = FilterSet::new(true);
        let (hosts, standard): (Vec<&str>, Vec<&str>) = rules_text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .partition(|l| is_hosts_line(l));
        set.add_filter_list(standard.join("\n"), ParseOptions::default());
        set.add_filter_list(
            hosts.join("\n"),
            ParseOptions {
                format: FilterFormat::Hosts,
                ..ParseOptions::default()
            },
        );
        Self {
            engine: adblock::Engine::new_with_filter_set(set),
        }
    }

    /// Cosmetic filtering for `page_url` with the classes and ids present in its DOM: per-site
    /// selectors plus the generic rules those classes/ids trigger, minus exceptions, unless
    /// `$generichide` applies (adblock's documented two-step pipeline).
    #[must_use]
    pub fn cosmetic(&self, page_url: &str, classes: &[&str], ids: &[&str]) -> CosmeticResult {
        let res = self.engine.url_cosmetic_resources(page_url);
        let mut hide: Vec<String> = res.hide_selectors.into_iter().collect();
        if !res.generichide {
            hide.extend(
                self.engine
                    .hidden_class_id_selectors(classes, ids, &res.exceptions),
            );
        }
        hide.sort();
        hide.dedup();
        CosmeticResult {
            hide_selectors: hide,
            generichide: res.generichide,
        }
    }

    /// Check a request: `source_url` is the page that issued it (empty = unknown, treated as
    /// third-party by adblock), `request_type` an adblock/webRequest type (`script`, `image`,
    /// `xmlhttprequest`, `sub_frame`, `document`, `other`, ...).
    #[must_use]
    pub fn check_request(
        &self,
        url: &str,
        source_url: &str,
        request_type: &str,
    ) -> AdblockCheckResult {
        let Ok(request) = adblock::request::Request::new(url, source_url, request_type, "GET")
        else {
            return AdblockCheckResult {
                matched: false,
                blocked: false,
                important: false,
                filter: None,
                exception: None,
            };
        };
        let r = self.engine.check_network_request(&request);
        AdblockCheckResult {
            matched: r.filter.is_some() || r.exception.is_some(),
            blocked: r.should_block(),
            important: r.important,
            filter: r.filter.and_then(|f| f.raw_line),
            exception: r.exception.and_then(|f| f.raw_line),
        }
    }

    /// A request whose page is unknown: adblock treats it as third-party, type `other`.
    /// Callers that know the page and the resource type use [`Self::check_request`].
    #[must_use]
    pub fn check_network_url(&self, url: &str) -> AdblockCheckResult {
        self.check_request(url, "", "other")
    }

    /// Per-site hide selectors only (no generic rules), sorted.
    #[must_use]
    pub fn hide_selectors_for(&self, page_url: &str) -> Vec<String> {
        self.cosmetic(page_url, &[], &[]).hide_selectors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adblock_engine_rules() {
        let rules = "\
            ! Test filter list\n\
            ||doubleclick.net^\n\
            ||ads.google.com^$important\n\
            @@||partner.doubleclick.net^\n\
            0.0.0.0 telemetry.tracker.io\n\
            /pagead/js/\n\
            ||cdn.tracker.io^$third-party\n\
            ##.ConsentBanner\n\
            google.com##.sponsored-slot\n\
        ";
        let engine = AdblockEngine::from_rules(rules);

        let r1 = engine.check_request(
            "https://ad.doubleclick.net/pixel.gif",
            "https://news.site.org/",
            "image",
        );
        assert!(r1.blocked);
        assert_eq!(r1.filter.as_deref(), Some("||doubleclick.net^"));

        let r2 = engine.check_request(
            "https://partner.doubleclick.net/sdk.js",
            "https://news.site.org/",
            "script",
        );
        assert!(!r2.blocked);
        assert_eq!(
            r2.exception.as_deref(),
            Some("@@||partner.doubleclick.net^")
        );

        let r3 = engine.check_request(
            "https://ads.google.com/banner.js",
            "https://news.site.org/",
            "script",
        );
        assert!(r3.blocked && r3.important);

        assert!(
            engine
                .check_request(
                    "https://telemetry.tracker.io/collect",
                    "https://news.site.org/",
                    "xmlhttprequest"
                )
                .blocked
        );

        // $third-party: blocked from another site, allowed first-party.
        assert!(
            engine
                .check_request(
                    "https://cdn.tracker.io/a.js",
                    "https://news.site.org/",
                    "script"
                )
                .blocked
        );
        assert!(
            !engine
                .check_request(
                    "https://cdn.tracker.io/a.js",
                    "https://cdn.tracker.io/",
                    "script"
                )
                .blocked
        );

        assert!(
            !engine
                .check_network_url("https://example.com/search?q=bun")
                .blocked
        );

        // Hosts lines with tabs and IPv6 addresses.
        let hosts = AdblockEngine::from_rules("0.0.0.0\tads.tab.io\n::1  ads.v6.io\n");
        assert!(hosts.check_network_url("https://ads.tab.io/x").blocked);
        assert!(hosts.check_network_url("https://ads.v6.io/x").blocked);

        // Cosmetic: per-site rule scoped to google.com, generic rule only when its class is on the page.
        let g = engine.cosmetic("https://www.google.com/", &["ConsentBanner"], &[]);
        assert!(g.hide_selectors.contains(&".sponsored-slot".to_string()));
        assert!(g.hide_selectors.contains(&".ConsentBanner".to_string()));
        let other = engine.cosmetic("https://news.site.org/", &["unrelated"], &[]);
        assert!(other.hide_selectors.is_empty());

        // Exceptions: `#@#` per site and `$generichide`.
        let ex = AdblockEngine::from_rules(
            "##.ad\nsite.org#@#.ad\n@@||quiet.org^$generichide\n",
        );
        assert!(
            ex.cosmetic("https://site.org/", &["ad"], &[])
                .hide_selectors
                .is_empty()
        );
        assert!(ex.cosmetic("https://quiet.org/", &["ad"], &[]).generichide);
        assert_eq!(
            ex.cosmetic("https://else.org/", &["ad"], &[])
                .hide_selectors,
            vec![".ad".to_string()]
        );
    }
}
