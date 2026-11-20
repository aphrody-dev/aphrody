//! Product identity constants and compatibility helpers generated from root `product.toml`.
//!
//! Ensures all user-facing names, paths, app identifiers, URL schemes, and environment
//! variables reflect the active product (`Aphrody`) while maintaining 100% backward
//! compatibility with the Zed ecosystem (`.zed/` project configs, `zed://` links & schemas,
//! `ZED_*` environment/task variables, `.ZedSans`/`.ZedMono` fonts, and `zed:extension` Wasm ABI).

use std::env;
use std::ffi::OsString;

pub use fw_m3 as design;
pub mod suite;

include!(concat!(env!("OUT_DIR"), "/product_constants.rs"));

/// Format `{ENV_PREFIX}_{suffix}` for diagnostics and user-facing override hints.
pub fn env_key(suffix: &str) -> String {
    format!("{ENV_PREFIX}_{suffix}")
}

/// Join `DOCS_URL` and a relative documentation slug (`"multibuffers"`, `"linux#..."`).
pub fn docs_page_url(slug: &str) -> String {
    let base = DOCS_URL.trim_end_matches('/');
    let trimmed = slug.trim_start_matches('/');
    if trimmed.is_empty() {
        base.to_owned()
    } else {
        format!("{base}/{trimmed}")
    }
}

/// Read `{ENV_PREFIX}_{suffix}` first (e.g. `APHRODY_LOG`), falling back to
/// `{LEGACY_ENV_PREFIX}_{suffix}` (e.g. `ZED_LOG`) for seamless Zed compatibility.
pub fn env_var(suffix: &str) -> Result<String, env::VarError> {
    let primary_key = env_key(suffix);
    match env::var(&primary_key) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) if ENV_PREFIX != LEGACY_ENV_PREFIX => {
            let legacy_key = format!("{LEGACY_ENV_PREFIX}_{suffix}");
            env::var(&legacy_key)
        }
        Err(err) => Err(err),
    }
}

/// Read `{ENV_PREFIX}_{suffix}` as an [`OsString`], falling back to `{LEGACY_ENV_PREFIX}_{suffix}`.
pub fn env_var_os(suffix: &str) -> Option<OsString> {
    let primary_key = format!("{ENV_PREFIX}_{suffix}");
    if let Some(value) = env::var_os(&primary_key) {
        return Some(value);
    }
    if ENV_PREFIX != LEGACY_ENV_PREFIX {
        let legacy_key = format!("{LEGACY_ENV_PREFIX}_{suffix}");
        return env::var_os(&legacy_key);
    }
    None
}

/// Check whether a boolean environment variable (`1` or `true`, case-insensitive) is set
/// on either `{ENV_PREFIX}_{suffix}` or `{LEGACY_ENV_PREFIX}_{suffix}`.
pub fn bool_env(suffix: &str) -> bool {
    env_var(suffix).is_ok_and(|val| val == "1" || val.eq_ignore_ascii_case("true"))
}

/// Strip either the primary URL scheme (`aphrody://...`) or any legacy compatible scheme
/// (`zed://...`), returning the remainder after `<scheme>://`.
pub fn strip_app_url_scheme(url: &str) -> Option<&str> {
    if let Some(rest) = url.strip_prefix(URL_SCHEME_PREFIX) {
        return Some(rest);
    }
    for legacy in LEGACY_URL_SCHEMES {
        if let Some(after_scheme) = url.strip_prefix(legacy)
            && let Some(rest) = after_scheme.strip_prefix("://")
        {
            return Some(rest);
        }
    }
    None
}

/// Strip either `aphrody-cli://` or `zed-cli://`.
pub fn strip_cli_url_scheme(url: &str) -> Option<&str> {
    if let Some(rest) = url.strip_prefix(CLI_URL_SCHEME_PREFIX) {
        return Some(rest);
    }
    for legacy in LEGACY_CLI_URL_SCHEMES {
        if let Some(after_scheme) = url.strip_prefix(legacy)
            && let Some(rest) = after_scheme.strip_prefix("://")
        {
            return Some(rest);
        }
    }
    None
}

/// Strip either `aphrody-dock-action://` or `zed-dock-action://`.
pub fn strip_dock_action_scheme(url: &str) -> Option<&str> {
    if let Some(rest) = url.strip_prefix(DOCK_ACTION_SCHEME_PREFIX) {
        return Some(rest);
    }
    for legacy in LEGACY_DOCK_ACTION_SCHEMES {
        if let Some(after_scheme) = url.strip_prefix(legacy)
            && let Some(rest) = after_scheme.strip_prefix("://")
        {
            return Some(rest);
        }
    }
    None
}

/// Returns true if `url` starts with the primary or any legacy app URL scheme (`aphrody://` or `zed://`).
pub fn has_app_url_scheme(url: &str) -> bool {
    strip_app_url_scheme(url).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_constants_and_zed_compatibility() {
        assert_eq!(APP_NAME, "Aphrody");
        assert_eq!(APP_SLUG, "aphrody");
        assert_eq!(LOCAL_SETTINGS_FOLDER, ".aphrody");
        assert_eq!(LEGACY_LOCAL_SETTINGS_FOLDER, ".zed");
        assert_eq!(EXTENSION_REGISTRY_URL, "https://api.zed.dev");

        assert_eq!(
            strip_app_url_scheme("aphrody://file/path/to/main.rs"),
            Some("file/path/to/main.rs")
        );
        assert_eq!(
            strip_app_url_scheme("zed://file/path/to/main.rs"),
            Some("file/path/to/main.rs")
        );
        assert_eq!(
            strip_cli_url_scheme("aphrody-cli://pipe-123"),
            Some("pipe-123")
        );
        assert_eq!(strip_cli_url_scheme("zed-cli://pipe-123"), Some("pipe-123"));
        assert_eq!(strip_app_url_scheme("https://example.com"), None);
        assert_eq!(
            env_key("ALLOW_EMULATED_GPU"),
            format!("{ENV_PREFIX}_ALLOW_EMULATED_GPU")
        );
        assert_eq!(
            docs_page_url("/multibuffers"),
            format!("{}/multibuffers", DOCS_URL.trim_end_matches('/'))
        );
        assert!(!PLUGIN_NAME.is_empty());
        assert!(!PLUGIN_SCOPE.is_empty());
        assert!(!GIT_DEFAULT_BRANCH.is_empty());
        assert!(!UPSTREAM_ZED_BRANCH.is_empty());
        assert!(!UPSTREAM_AWESOME_GPUI_REPO.is_empty());
        assert!(!UPSTREAM_EXTENSIONS_REPO.is_empty());
        assert!(!UPSTREAM_WINGET_PKGS_REPO.is_empty());
        assert!(!UPSTREAM_MODERN_WEB_GUIDANCE_REPO.is_empty());
        assert!(!UPSTREAM_DESIGN_MD_REPO.is_empty());
        assert!(!UPSTREAM_STITCH_SDK_REPO.is_empty());
        assert!(!UPSTREAM_STITCH_SKILLS_REPO.is_empty());
        assert_eq!(DESIGN_SOURCE_OF_TRUTH, "https://design.google/");
        assert_eq!(DESIGN_STITCH_URL, "https://stitch.withgoogle.com/");
        assert_eq!(DESIGN_SANS_FLEX_FAMILY, "Google Sans Flex");
        assert_eq!(DESIGN_MD_PATH, "DESIGN.md");
        assert!(!DESIGN_SYSTEM.is_empty());
        assert!(!DESIGN_TOKENS_MANIFEST.is_empty());
        assert!(!DESIGN_THEMES_MANIFEST.is_empty());
        assert!(!DESIGN_SEED_HEX.is_empty());
        assert!(!DESIGN_VARIANT.is_empty());
    }
}
