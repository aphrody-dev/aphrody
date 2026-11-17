use std::fmt;

use serde::{Deserialize, Serialize};

/// W3C String-Meta text base direction (`"ltr"`, `"rtl"`, `"auto"`).
///
/// Matches <https://www.w3.org/TR/string-meta/> and HTML `dir` attribute semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDir {
    Ltr,
    Rtl,
    #[default]
    Auto,
}

impl TextDir {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
            Self::Auto => "auto",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ltr" => Some(Self::Ltr),
            "rtl" => Some(Self::Rtl),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }

    /// Resolves `Auto` using W3C First-Strong heuristics over `text`, falling back to `Ltr`.
    #[must_use]
    pub fn resolve_with_text(self, text: &str) -> Self {
        match self {
            Self::Ltr | Self::Rtl => self,
            Self::Auto => match strong_direction(text) {
                Some(dir) => dir,
                None => Self::Ltr,
            },
        }
    }
}

impl fmt::Display for TextDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonicalized BCP 47 language tag per RFC 5646 / W3C LTLI (<https://www.w3.org/TR/ltli/>).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bcp47Tag(String);

impl Bcp47Tag {
    /// Parses POSIX/platform or web locale forms (`fr_FR.UTF-8`, `zh-hans-cn`, `ar_EG@numbers=arab`)
    /// into canonical BCP 47 casing (`fr-FR`, `zh-Hans-CN`, `ar-EG`).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let stripped = raw.trim().split(['.', '@']).next().unwrap_or_default();
        if stripped.is_empty()
            || stripped.eq_ignore_ascii_case("c")
            || stripped.eq_ignore_ascii_case("posix")
        {
            return None;
        }
        let mut parts = stripped.split(['-', '_']).filter(|p| !p.is_empty());
        let lang_raw = parts.next()?;
        if !((2..=3).contains(&lang_raw.len()) || (5..=8).contains(&lang_raw.len()))
            || !lang_raw.bytes().all(|b| b.is_ascii_alphabetic())
        {
            return None;
        }

        let mut canonical = Vec::new();
        canonical.push(lang_raw.to_ascii_lowercase());

        let mut seen_script = false;
        let mut seen_region = false;
        let mut in_extension = false;

        for subtag in parts {
            if subtag.len() > 8 || !subtag.bytes().all(|b| b.is_ascii_alphanumeric()) {
                return None;
            }
            if subtag.len() == 1 {
                in_extension = true;
                canonical.push(subtag.to_ascii_lowercase());
                continue;
            }
            if in_extension {
                canonical.push(subtag.to_ascii_lowercase());
                continue;
            }
            if !seen_script
                && !seen_region
                && subtag.len() == 4
                && subtag.bytes().all(|b| b.is_ascii_alphabetic())
            {
                seen_script = true;
                canonical.push(title_case_subtag(subtag));
                continue;
            }
            if !seen_region
                && ((subtag.len() == 2 && subtag.bytes().all(|b| b.is_ascii_alphabetic()))
                    || (subtag.len() == 3 && subtag.bytes().all(|b| b.is_ascii_digit())))
            {
                seen_region = true;
                canonical.push(subtag.to_ascii_uppercase());
                continue;
            }
            if (5..=8).contains(&subtag.len())
                || (subtag.len() == 4 && subtag.as_bytes()[0].is_ascii_digit())
            {
                canonical.push(subtag.to_ascii_lowercase());
                continue;
            }
            return None;
        }

        Some(Self(canonical.join("-")))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn language(&self) -> &str {
        self.0.split('-').next().unwrap_or(&self.0)
    }

    #[must_use]
    pub fn script(&self) -> Option<&str> {
        self.0.split('-').skip(1).find(|s| {
            s.len() == 4
                && s.as_bytes()[0].is_ascii_uppercase()
                && s.bytes().all(|b| b.is_ascii_alphabetic())
        })
    }

    #[must_use]
    pub fn region(&self) -> Option<&str> {
        self.0.split('-').skip(1).find(|s| {
            (s.len() == 2 && s.bytes().all(|b| b.is_ascii_uppercase()))
                || (s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
        })
    }

    /// Returns the RFC 4647 lookup fallback chain ending in `"en"` if absent.
    #[must_use]
    pub fn fallback_chain(&self) -> Vec<String> {
        let mut chain = Vec::new();
        let mut cur: &str = &self.0;
        loop {
            chain.push(cur.to_owned());
            match cur.rfind('-') {
                Some(idx) => cur = &cur[..idx],
                None => break,
            }
        }
        if !chain.iter().any(|t| t == "en") {
            chain.push("en".to_owned());
        }
        chain
    }

    /// Infers the default writing-system base direction from script or language subtags.
    #[must_use]
    pub fn default_direction(&self) -> TextDir {
        if let Some(script) = self.script() {
            if matches!(
                script,
                "Arab"
                    | "Hebr"
                    | "Syrc"
                    | "Thaa"
                    | "Nkoo"
                    | "Adlm"
                    | "Mand"
                    | "Samr"
                    | "Yezi"
                    | "Rohg"
            ) {
                return TextDir::Rtl;
            }
            return TextDir::Ltr;
        }
        if is_rtl_language(self.language()) {
            TextDir::Rtl
        } else {
            TextDir::Ltr
        }
    }
}

impl fmt::Display for Bcp47Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn title_case_subtag(subtag: &str) -> String {
    let mut out = String::with_capacity(subtag.len());
    for (idx, b) in subtag.bytes().enumerate() {
        let ch = b as char;
        out.push(if idx == 0 {
            ch.to_ascii_uppercase()
        } else {
            ch.to_ascii_lowercase()
        });
    }
    out
}

/// Whether the primary ISO 639 language code defaults to right-to-left script.
#[must_use]
pub fn is_rtl_language(lang: &str) -> bool {
    matches!(
        lang.to_ascii_lowercase().as_str(),
        "ar" | "arc"
            | "azb"
            | "ckb"
            | "dv"
            | "fa"
            | "he"
            | "iw"
            | "ji"
            | "khw"
            | "ks"
            | "ku"
            | "mzn"
            | "nqo"
            | "pnb"
            | "ps"
            | "sd"
            | "syr"
            | "ug"
            | "ur"
            | "yi"
    )
}

/// Infers W3C First-Strong direction (`Some(Ltr)` or `Some(Rtl)`) from strong directional characters.
#[must_use]
pub fn strong_direction(text: &str) -> Option<TextDir> {
    for ch in text.chars() {
        match ch {
            // Explicit Unicode directional marks
            '\u{200E}' => return Some(TextDir::Ltr),
            '\u{200F}' | '\u{061C}' => return Some(TextDir::Rtl),
            // Hebrew, Arabic, Syriac, Thaana, N'Ko, Samaritan, Mandaic
            '\u{0590}'..='\u{08FF}'
            // Hebrew / Arabic Presentation Forms A & B
            | '\u{FB1D}'..='\u{FDFF}'
            | '\u{FE70}'..='\u{FEFF}'
            // Adlam, Hanifi Rohingya, Yezidi
            | '\u{1E900}'..='\u{1E95F}'
            | '\u{10D00}'..='\u{10D3F}'
            | '\u{10E80}'..='\u{10EBF}' => {
                // Skip U+FEFF BOM if used at start as non-directional byte order mark
                if ch != '\u{FEFF}' {
                    return Some(TextDir::Rtl);
                }
            }
            _ if ch.is_alphabetic() => return Some(TextDir::Ltr),
            _ => {}
        }
    }
    None
}

/// Wraps an interpolated text fragment with W3C-recommended Bidi Isolate controls
/// (`LRI`/`RLI`/`FSI` ... `PDI`), never legacy `LRE`/`RLE`/`LRO`/`RLO` embeddings.
#[must_use]
pub fn wrap_bidi_isolate(text: &str, dir: TextDir) -> String {
    let open = match dir {
        TextDir::Ltr => '\u{2066}',  // LRI
        TextDir::Rtl => '\u{2067}',  // RLI
        TextDir::Auto => '\u{2068}', // FSI
    };
    format!("{open}{text}\u{2069}") // PDI
}

/// W3C String-Meta representation (`value`, `lang`, `dir`) for APIs, events, and manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalizedString {
    pub value: String,
    pub lang: String,
    pub dir: TextDir,
}

impl LocalizedString {
    #[must_use]
    pub fn new(value: impl Into<String>, lang: &Bcp47Tag, dir: Option<TextDir>) -> Self {
        let value = value.into();
        let resolved_dir = dir.unwrap_or_else(|| {
            strong_direction(&value).unwrap_or_else(|| lang.default_direction())
        });
        Self {
            value,
            lang: lang.as_str().to_owned(),
            dir: resolved_dir,
        }
    }

    /// Serializes as W3C String-Meta JSON-LD object (`@value`, `@language`, `@direction`).
    #[must_use]
    pub fn to_json_ld(&self) -> serde_json::Value {
        serde_json::json!({
            "@value": self.value,
            "@language": self.lang,
            "@direction": self.dir.as_str(),
        })
    }
}

/// Negotiated result for HTTP `Accept-Language` -> `Content-Language` + `Vary: Accept-Language`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegotiatedLocale {
    pub locale: String,
    pub dir: TextDir,
    pub content_language: String,
    pub vary: &'static str,
}

/// Negotiates the best supported locale from an HTTP `Accept-Language` header using `q` weights
/// and RFC 4647 subtag prefix fallback.
#[must_use]
pub fn negotiate_locale(
    accept_language: Option<&str>,
    supported: &[&str],
    fallback: &str,
) -> NegotiatedLocale {
    let mut ranges: Vec<(f32, usize, Bcp47Tag)> = Vec::new();
    if let Some(header) = accept_language {
        for (idx, item) in header.split(',').enumerate() {
            let mut segments = item.split(';');
            let tag_raw = segments.next().unwrap_or_default().trim();
            if tag_raw.is_empty() || tag_raw == "*" {
                continue;
            }
            let Some(tag) = Bcp47Tag::parse(tag_raw) else {
                continue;
            };
            let mut q = 1.0_f32;
            for param in segments {
                let param = param.trim();
                if let Some(val) = param
                    .strip_prefix("q=")
                    .or_else(|| param.strip_prefix("Q="))
                    && let Ok(parsed) = val.trim().parse::<f32>()
                {
                    q = parsed.clamp(0.0, 1.0);
                }
            }
            if q > 0.0 {
                ranges.push((q, idx, tag));
            }
        }
    }

    ranges.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    for (_, _, req_tag) in &ranges {
        for candidate in req_tag.fallback_chain() {
            if let Some(matched) = supported
                .iter()
                .find(|s| s.eq_ignore_ascii_case(&candidate))
            {
                let parsed = Bcp47Tag::parse(matched);
                let dir = parsed
                    .as_ref()
                    .map_or(TextDir::Ltr, Bcp47Tag::default_direction);
                let canonical =
                    parsed.map_or_else(|| (*matched).to_owned(), |t| t.as_str().to_owned());
                return NegotiatedLocale {
                    dir,
                    content_language: canonical.clone(),
                    locale: canonical,
                    vary: "Accept-Language",
                };
            }
        }
    }

    let fb_tag = Bcp47Tag::parse(fallback);
    let fb_canonical = fb_tag
        .as_ref()
        .map_or_else(|| fallback.to_owned(), |t| t.as_str().to_owned());
    let dir = fb_tag
        .as_ref()
        .map_or(TextDir::Ltr, Bcp47Tag::default_direction);
    NegotiatedLocale {
        locale: fb_canonical.clone(),
        dir,
        content_language: fb_canonical,
        vary: "Accept-Language",
    }
}

/// Detects the current process locale from standard environment variables (`LC_ALL`, `LC_MESSAGES`, `LANG`),
/// falling back cleanly to `"en"` without platform crashes on Wasm.
#[must_use]
pub fn detect_process_locale() -> Bcp47Tag {
    #[cfg(not(target_family = "wasm"))]
    {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(raw) = std::env::var(key)
                && let Some(tag) = Bcp47Tag::parse(&raw)
            {
                return tag;
            }
        }
    }
    Bcp47Tag("en".to_owned())
}

/// Trojan Source / Bidi safety hazard detected per W3C CharMod-Norm & Unicode UAX #9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BidiHazardKind {
    UnsafeOverrideOrEmbedding,
    UnclosedIsolate,
    UnexpectedPdiWithoutOpen,
    ReplacementCharacter,
    LeadingBom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidiHazard {
    pub kind: BidiHazardKind,
    pub byte_offset: usize,
    pub codepoint: char,
}

/// Scans text for Trojan Source Bidi controls (`LRE/RLE/LRO/RLO/PDF`), unclosed Bidi isolates,
/// unexpected `U+FFFD`, or leading `U+FEFF` BOM.
#[must_use]
pub fn scan_bidi_hazards(text: &str) -> Vec<BidiHazard> {
    let mut hazards = Vec::new();
    let mut open_isolates: usize = 0;

    for (idx, ch) in text.char_indices() {
        match ch {
            '\u{FEFF}' if idx == 0 => hazards.push(BidiHazard {
                kind: BidiHazardKind::LeadingBom,
                byte_offset: idx,
                codepoint: ch,
            }),
            '\u{FFFD}' => hazards.push(BidiHazard {
                kind: BidiHazardKind::ReplacementCharacter,
                byte_offset: idx,
                codepoint: ch,
            }),
            '\u{202A}'..='\u{202E}' => hazards.push(BidiHazard {
                kind: BidiHazardKind::UnsafeOverrideOrEmbedding,
                byte_offset: idx,
                codepoint: ch,
            }),
            '\u{2066}'..='\u{2068}' => open_isolates += 1,
            '\u{2069}' => {
                if open_isolates == 0 {
                    hazards.push(BidiHazard {
                        kind: BidiHazardKind::UnexpectedPdiWithoutOpen,
                        byte_offset: idx,
                        codepoint: ch,
                    });
                } else {
                    open_isolates -= 1;
                }
            }
            _ => {}
        }
    }

    if open_isolates > 0 {
        hazards.push(BidiHazard {
            kind: BidiHazardKind::UnclosedIsolate,
            byte_offset: text.len(),
            codepoint: '\u{2069}',
        });
    }

    hazards
}

/// Validates raw UTF-8 bytes and reports whether a leading UTF-8 BOM (`0xEF 0xBB 0xBF`) was present.
pub fn validate_utf8_no_bom(bytes: &[u8]) -> Result<(&str, bool), std::str::Utf8Error> {
    let (body, had_bom) = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (&bytes[3..], true)
    } else {
        (bytes, false)
    };
    let text = std::str::from_utf8(body)?;
    Ok((text, had_bom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcp47_parses_canonical_and_posix_locales() {
        let fr = Bcp47Tag::parse("fr_fr.UTF-8").expect("fr_FR valid");
        assert_eq!(fr.as_str(), "fr-FR");
        assert_eq!(fr.language(), "fr");
        assert_eq!(fr.region(), Some("FR"));
        assert_eq!(fr.default_direction(), TextDir::Ltr);

        let zh = Bcp47Tag::parse("zh_hans_cn@collation=pinyin").expect("zh-Hans-CN valid");
        assert_eq!(zh.as_str(), "zh-Hans-CN");
        assert_eq!(zh.script(), Some("Hans"));
        assert_eq!(zh.region(), Some("CN"));
        assert_eq!(
            zh.fallback_chain(),
            vec!["zh-Hans-CN", "zh-Hans", "zh", "en"]
        );

        let ar = Bcp47Tag::parse("ar-EG").expect("ar-EG valid");
        assert_eq!(ar.default_direction(), TextDir::Rtl);
        assert!(Bcp47Tag::parse("C.UTF-8").is_none());
    }

    #[test]
    fn negotiate_accept_language_respects_q_weights_and_rtl() {
        let supported = ["en", "fr", "ar", "he", "ja", "zh-Hans"];
        let res = negotiate_locale(
            Some("en-US;q=0.7, ar-EG;q=0.95, fr;q=0.8"),
            &supported,
            "en",
        );
        assert_eq!(res.locale, "ar");
        assert_eq!(res.dir, TextDir::Rtl);
        assert_eq!(res.content_language, "ar");
        assert_eq!(res.vary, "Accept-Language");

        let fallback = negotiate_locale(Some("xx-YY;q=1.0"), &supported, "en");
        assert_eq!(fallback.locale, "en");
        assert_eq!(fallback.dir, TextDir::Ltr);
    }

    #[test]
    fn string_meta_and_bidi_isolate_follow_w3c_spec() {
        let ar = Bcp47Tag::parse("ar").unwrap();
        let meta = LocalizedString::new("أفرودي", &ar, None);
        assert_eq!(meta.dir, TextDir::Rtl);
        let json_ld = meta.to_json_ld();
        assert_eq!(json_ld["@language"], "ar");
        assert_eq!(json_ld["@direction"], "rtl");

        let isolated = wrap_bidi_isolate("Hello", TextDir::Ltr);
        assert!(isolated.starts_with('\u{2066}'));
        assert!(isolated.ends_with('\u{2069}'));
        assert!(scan_bidi_hazards(&isolated).is_empty());

        let trojan = "user\u{202E}admin";
        let hazards = scan_bidi_hazards(trojan);
        assert_eq!(hazards.len(), 1);
        assert_eq!(hazards[0].kind, BidiHazardKind::UnsafeOverrideOrEmbedding);
    }
}
