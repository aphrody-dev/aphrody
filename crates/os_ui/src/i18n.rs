use std::collections::BTreeMap;

use os_session::{Bcp47Tag, LocalizedString, TextDir};
use serde::{Deserialize, Serialize};

/// Embedded W3C Internationalization & multi-script UI catalog read from `assets/os/locales.toml`.
pub const BUNDLED_OS_LOCALES_TOML: &str = include_str!("../../../assets/os/locales.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocaleEntry {
    pub name: String,
    pub native_name: String,
    pub dir: TextDir,
    #[serde(default = "default_decimal")]
    pub decimal: String,
    #[serde(default = "default_group")]
    pub group: String,
    #[serde(default)]
    pub indian_grouping: bool,
    #[serde(default)]
    pub messages: BTreeMap<String, String>,
}

fn default_decimal() -> String {
    ".".to_owned()
}

fn default_group() -> String {
    ",".to_owned()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsI18nCatalog {
    pub default_locale: String,
    pub locales: BTreeMap<String, LocaleEntry>,
}

impl OsI18nCatalog {
    pub fn bundled() -> anyhow::Result<Self> {
        let mut catalog: Self = toml::from_str(BUNDLED_OS_LOCALES_TOML)?;
        if catalog.default_locale.is_empty() {
            catalog.default_locale = "en".to_owned();
        }
        Ok(catalog)
    }

    pub fn load_with_user_override() -> anyhow::Result<Self> {
        let mut catalog = Self::bundled()?;
        if let Some(cfg) = dirs::config_dir() {
            let user_path = cfg.join(brand::APP_SLUG).join("locales.toml");
            if let Ok(raw) = std::fs::read_to_string(&user_path)
                && let Ok(extra) = toml::from_str::<Self>(&raw)
            {
                if !extra.default_locale.is_empty() {
                    catalog.default_locale = extra.default_locale;
                }
                for (tag, entry) in extra.locales {
                    catalog
                        .locales
                        .entry(tag)
                        .and_modify(|existing| {
                            existing.name = entry.name.clone();
                            existing.native_name = entry.native_name.clone();
                            existing.dir = entry.dir;
                            existing.messages.extend(entry.messages.clone());
                        })
                        .or_insert(entry);
                }
            }
        }
        Ok(catalog)
    }

    #[must_use]
    pub fn available_locales(&self) -> Vec<String> {
        self.locales.keys().cloned().collect()
    }

    fn candidate_keys(&self, requested: &str) -> Vec<String> {
        let mut matches = Vec::new();
        if let Some(tag) = Bcp47Tag::parse(requested) {
            let mut candidates = tag.fallback_chain();
            // W3C LTLI / CLDR likely-subtag bridge for Chinese script tags (`zh-CN` -> `zh-Hans`, `zh-TW` -> `zh-Hant`).
            if tag.language() == "zh" && tag.script().is_none() {
                let script_tag = match tag.region() {
                    Some("TW" | "HK" | "MO") => "zh-Hant",
                    _ => "zh-Hans",
                };
                if !candidates
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(script_tag))
                {
                    candidates.insert(1.min(candidates.len()), script_tag.to_owned());
                }
            }
            for candidate in candidates {
                if let Some((key, _)) = self
                    .locales
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(&candidate))
                    && !matches.contains(key)
                {
                    matches.push(key.clone());
                }
            }
            // Match any entry sharing primary language subtag if still missing before English fallback.
            for key in self.locales.keys() {
                if let Some(entry_tag) = Bcp47Tag::parse(key)
                    && entry_tag.language() == tag.language()
                    && !matches.contains(key)
                {
                    matches.push(key.clone());
                }
            }
        }
        matches
    }

    #[must_use]
    pub fn resolve_tag(&self, requested: &str) -> String {
        if let Some(first) = self.candidate_keys(requested).into_iter().next() {
            return first;
        }
        if self.locales.contains_key(&self.default_locale) {
            self.default_locale.clone()
        } else {
            "en".to_owned()
        }
    }

    #[must_use]
    pub fn entry(&self, requested: &str) -> Option<&LocaleEntry> {
        let resolved = self.resolve_tag(requested);
        self.locales.get(&resolved)
    }

    #[must_use]
    pub fn direction(&self, requested: &str) -> TextDir {
        self.entry(requested).map_or_else(
            || Bcp47Tag::parse(requested).map_or(TextDir::Ltr, |t| t.default_direction()),
            |e| e.dir,
        )
    }

    #[must_use]
    pub fn translate(&self, requested: &str, key: &str) -> String {
        for tag_key in self.candidate_keys(requested) {
            if let Some(val) = self.locales.get(&tag_key).and_then(|e| e.messages.get(key)) {
                return val.clone();
            }
        }
        self.locales
            .get(&self.default_locale)
            .and_then(|e| e.messages.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    #[must_use]
    pub fn translate_localized(&self, requested: &str, key: &str) -> LocalizedString {
        let tag_str = self.resolve_tag(requested);
        let tag = Bcp47Tag::parse(&tag_str).unwrap_or_default();
        let text = self.translate(&tag_str, key);
        let dir = self.direction(&tag_str);
        LocalizedString::new(text, &tag, Some(dir))
    }

    /// Formats an integer with CLDR-style grouping (including Indian 2,2,3 grouping for `hi`/`en-IN`).
    #[must_use]
    pub fn format_integer(&self, requested: &str, value: i64) -> String {
        let entry = self.entry(requested);
        let sep = entry.map_or(",", |e| e.group.as_str());
        let indian = entry.is_some_and(|e| e.indian_grouping);
        let negative = value < 0;
        let digits = value.unsigned_abs().to_string();
        if digits.len() <= 3 || sep.is_empty() {
            return if negative {
                format!("-{digits}")
            } else {
                digits
            };
        }
        let bytes = digits.as_bytes();
        let mut rev = String::with_capacity(digits.len() + 4);
        for (idx, &b) in bytes.iter().rev().enumerate() {
            if idx > 0 {
                let split = if indian {
                    idx == 3 || (idx > 3 && (idx - 3).is_multiple_of(2))
                } else {
                    idx.is_multiple_of(3)
                };
                if split {
                    for s in sep.chars().rev() {
                        rev.push(s);
                    }
                }
            }
            rev.push(b as char);
        }
        let formatted: String = rev.chars().rev().collect();
        if negative {
            format!("-{formatted}")
        } else {
            formatted
        }
    }
}

/// CLDR cardinal plural category for integer counts (`n >= 0`).
#[must_use]
pub fn plural_category(locale: &str, n: u64) -> PluralCategory {
    let lang = Bcp47Tag::parse(locale).map_or_else(|| "en".to_owned(), |t| t.language().to_owned());
    match lang.as_str() {
        "ar" => {
            let mod100 = n % 100;
            if n == 0 {
                PluralCategory::Zero
            } else if n == 1 {
                PluralCategory::One
            } else if n == 2 {
                PluralCategory::Two
            } else if (3..=10).contains(&mod100) {
                PluralCategory::Few
            } else if (11..=99).contains(&mod100) {
                PluralCategory::Many
            } else {
                PluralCategory::Other
            }
        }
        "fr" => {
            if n <= 1 {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        }
        "ru" | "uk" => {
            let mod10 = n % 10;
            let mod100 = n % 100;
            if mod10 == 1 && mod100 != 11 {
                PluralCategory::One
            } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
                PluralCategory::Few
            } else {
                PluralCategory::Many
            }
        }
        "ja" | "zh" | "ko" | "th" | "vi" | "id" => PluralCategory::Other,
        _ => {
            if n == 1 {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        }
    }
}

/// Formats UTC epoch seconds into canonical W3C/ISO 8601 exchange timestamp (`YYYY-MM-DDTHH:MM:SSZ`).
#[must_use]
pub fn format_iso_utc(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let secs_of_day = epoch_secs.rem_euclid(86_400);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

// Howard Hinnant civil-from-days algorithm (proleptic Gregorian calendar).
fn civil_from_days(days_since_unix: i64) -> (i64, u32, u32) {
    let z = days_since_unix + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + i64::from(m <= 2);
    (year, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_covers_all_locales_and_directions() {
        let catalog = OsI18nCatalog::bundled().expect("bundled locales.toml parses");
        let en = catalog.locales.get("en").expect("en base present");
        for (tag, entry) in &catalog.locales {
            for key in en.messages.keys() {
                assert!(
                    entry.messages.contains_key(key),
                    "locale '{tag}' missing message key '{key}'"
                );
            }
        }
        assert_eq!(catalog.direction("ar"), TextDir::Rtl);
        assert_eq!(catalog.direction("he"), TextDir::Rtl);
        assert_eq!(catalog.direction("fr-FR"), TextDir::Ltr);
        assert_eq!(catalog.resolve_tag("zh_CN"), "zh-Hans");
    }

    #[test]
    fn cldr_plurals_number_grouping_and_iso8601_match_specs() {
        let catalog = OsI18nCatalog::bundled().unwrap();
        assert_eq!(catalog.format_integer("en", 1_234_567), "1,234,567");
        assert_eq!(catalog.format_integer("hi", 1_234_567), "12,34,567");
        assert_eq!(catalog.format_integer("ar", 12_345), "12٬345");

        assert_eq!(plural_category("ar", 0), PluralCategory::Zero);
        assert_eq!(plural_category("ar", 2), PluralCategory::Two);
        assert_eq!(plural_category("ar", 7), PluralCategory::Few);
        assert_eq!(plural_category("fr", 0), PluralCategory::One);
        assert_eq!(plural_category("ru", 23), PluralCategory::Few);

        assert_eq!(format_iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_iso_utc(1_700_000_000), "2023-11-14T22:13:20Z");
    }
}
