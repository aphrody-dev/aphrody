//! Clean document extraction: title, description, headings, visible text and outgoing links of an HTML page.
//! script, style, noscript, iframe and svg subtrees never reach the text; links to shielded hosts are dropped.

use std::collections::HashSet;

use scraper::{ElementRef, Html, Node, Selector};
use url::Url;

use crate::shields::Shields;

const MAX_TEXT: usize = 32_768;
const SKIPPED: &[&str] = &["script", "style", "noscript", "iframe", "svg", "template"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CleanDocument {
    pub title: String,
    pub description: String,
    pub headings: Vec<String>,
    pub text: String,
    pub links: Vec<String>,
    pub trackers_blocked: usize,
}

fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("static selector")
}

fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_text(element: ElementRef<'_>, out: &mut Vec<String>) {
    for child in element.children() {
        match child.value() {
            Node::Text(text) => {
                let clean = squash(text);
                if !clean.is_empty() {
                    out.push(clean);
                }
            }
            Node::Element(el) if !SKIPPED.contains(&el.name()) => {
                if let Some(inner) = ElementRef::wrap(child) {
                    collect_text(inner, out);
                }
            }
            _ => {}
        }
    }
}

pub fn extract(html: &str, base: &Url, shields: &Shields) -> CleanDocument {
    let doc = Html::parse_document(html);
    let title = doc.select(&selector("title")).next().map(|t| squash(&t.text().collect::<String>())).unwrap_or_default();
    let description = doc
        .select(&selector(r#"meta[name="description"], meta[property="og:description"]"#))
        .filter_map(|m| m.value().attr("content"))
        .map(str::trim)
        .find(|c| !c.is_empty())
        .unwrap_or_default()
        .to_string();
    let headings = doc.select(&selector("h1, h2, h3")).map(|h| squash(&h.text().collect::<String>())).filter(|h| !h.is_empty()).collect();

    let mut seen = HashSet::new();
    let (mut links, mut trackers_blocked) = (Vec::new(), 0);
    for anchor in doc.select(&selector("a[href]")) {
        let href = anchor.value().attr("href").unwrap_or_default().trim();
        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") || href.starts_with("mailto:") {
            continue;
        }
        let Ok(mut url) = base.join(href) else { continue };
        url.set_fragment(None);
        if !seen.insert(url.to_string()) {
            continue;
        }
        if shields.should_block(&url) {
            trackers_blocked += 1;
        } else {
            links.push(url.to_string());
        }
    }

    let mut parts = Vec::new();
    if let Some(body) = doc.select(&selector("body")).next() {
        collect_text(body, &mut parts);
    }
    let mut text = parts.join(" ");
    if text.len() > MAX_TEXT {
        let mut cut = MAX_TEXT;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
    }
    CleanDocument { title, description, headings, text, links, trackers_blocked }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_clean_content() {
        let html = r##"<html><head><title> Hello  World </title><meta name="description" content="About it"><style>p{}</style></head>
        <body><h1>Top</h1><script>var x=1</script><p>Some <b>bold</b> text</p><a href="/a#f">A</a><a href="/a">dup</a>
        <a href="https://www.google-analytics.com/collect?v=1">t</a><a href="#top">skip</a><a href="mailto:x@y">m</a></body></html>"##;
        let doc = extract(html, &Url::parse("https://e.test/dir/").unwrap(), &Shields::default());
        assert_eq!(doc.title, "Hello World");
        assert_eq!(doc.description, "About it");
        assert_eq!(doc.headings, vec!["Top"]);
        assert_eq!(doc.text, "Top Some bold text A dup t skip m");
        assert_eq!(doc.links, vec!["https://e.test/a"]);
        assert_eq!(doc.trackers_blocked, 1);
    }
}
