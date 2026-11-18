//! robots.txt (RFC 9309): groups by user agent, the longest matching rule wins and Allow beats Disallow on a tie,
//! `*` matches any run of characters and a trailing `$` anchors the end. Crawl-delay and Sitemap lines are kept.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RobotsRules {
    pub allow: Vec<String>,
    pub disallow: Vec<String>,
    pub crawl_delay_secs: Option<f64>,
    pub sitemaps: Vec<String>,
}

impl RobotsRules {
    fn is_empty(&self) -> bool {
        self.allow.is_empty() && self.disallow.is_empty() && self.crawl_delay_secs.is_none()
    }

    /// Whether `path_and_query` (for example `/a/b?c=d`) may be fetched.
    pub fn allows(&self, path_and_query: &str) -> bool {
        let longest = |patterns: &[String]| patterns.iter().filter(|p| pattern_matches(p, path_and_query)).map(String::len).max();
        match (longest(&self.disallow), longest(&self.allow)) {
            (None, _) => true,
            (Some(d), Some(a)) => a >= d,
            (Some(_), None) => false,
        }
    }
}

/// Parses `text` for the product token `user_agent` (compared in lower case, by containment as RFC 9309 asks).
pub fn parse(text: &str, user_agent: &str) -> RobotsRules {
    let target = user_agent.to_ascii_lowercase();
    let (mut specific, mut wildcard) = (RobotsRules::default(), RobotsRules::default());
    let mut sitemaps = Vec::new();
    let mut agents: Vec<String> = Vec::new();
    let mut saw_rule = false;
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            agents.clear();
            saw_rule = false;
            continue;
        }
        let Some((field, value)) = line.split_once(':') else { continue };
        let (field, value) = (field.trim().to_ascii_lowercase(), value.trim());
        if field == "user-agent" {
            if saw_rule {
                agents.clear();
                saw_rule = false;
            }
            agents.push(value.to_ascii_lowercase());
            continue;
        }
        if field == "sitemap" {
            sitemaps.push(value.to_string());
            continue;
        }
        saw_rule = true;
        let for_specific = agents.iter().any(|a| a != "*" && !a.is_empty() && target.contains(a.as_str()));
        let for_wildcard = agents.iter().any(|a| a == "*");
        let push = |bucket: &mut RobotsRules| match field.as_str() {
            "allow" if !value.is_empty() => bucket.allow.push(value.to_string()),
            "disallow" if !value.is_empty() => bucket.disallow.push(value.to_string()),
            "crawl-delay" => {
                if let Ok(delay) = value.parse::<f64>() {
                    if delay >= 0.0 {
                        bucket.crawl_delay_secs = Some(delay);
                    }
                }
            }
            _ => {}
        };
        if for_specific {
            push(&mut specific);
        }
        if for_wildcard {
            push(&mut wildcard);
        }
    }
    let mut chosen = if specific.is_empty() { wildcard } else { specific };
    chosen.sitemaps = sitemaps;
    chosen
}

/// A robots.txt pattern against a path: prefix match with `*` wildcards, `$` at the end anchors.
pub fn pattern_matches(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    let (body, anchored) = pattern.strip_suffix('$').map_or((pattern, false), |p| (p, true));
    let parts: Vec<&str> = body.split('*').collect();
    if !text.starts_with(parts[0]) {
        return false;
    }
    if parts.len() == 1 {
        return !anchored || text.len() == parts[0].len();
    }
    let mut pos = parts[0].len();
    let last = parts.len() - 1;
    for (i, part) in parts.iter().enumerate().skip(1) {
        if i == last && anchored {
            return text.len() >= pos + part.len() && text.ends_with(part);
        }
        match text[pos..].find(part) {
            Some(found) => pos += found + part.len(),
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "User-agent: *\nDisallow: /private\nAllow: /private/open\nCrawl-delay: 2\nSitemap: https://e.test/sitemap.xml\n\nUser-agent: openbot\nDisallow: /x$\n";

    #[test]
    fn wildcard_group() {
        let rules = parse(SAMPLE, "other/1.0");
        assert!(!rules.allows("/private/a"));
        assert!(rules.allows("/private/open/a"), "longer Allow wins");
        assert!(rules.allows("/public"));
        assert_eq!(rules.crawl_delay_secs, Some(2.0));
        assert_eq!(rules.sitemaps, vec!["https://e.test/sitemap.xml"]);
    }

    #[test]
    fn specific_group_wins() {
        let rules = parse(SAMPLE, "OpenBot/2.0 (+https://e.test)");
        assert!(!rules.allows("/x"));
        assert!(rules.allows("/xy"), "$ anchors the end");
        assert!(rules.allows("/private"), "the * group does not apply to a named agent");
    }

    #[test]
    fn globs() {
        assert!(pattern_matches("/a/*/c", "/a/b/c/d"));
        assert!(pattern_matches("/*.pdf$", "/x/y.pdf"));
        assert!(!pattern_matches("/*.pdf$", "/x/y.pdf?z"));
        assert!(!pattern_matches("", "/x"));
    }
}
