//! Unified pure-Rust Google OAuth 2.0 & `gcloud` authentication layer:
//! 1. OAuth 2.0 PKCE S256 loopback authorization flow (reusing `oauth_callback_server`)
//! 2. OAuth 2.0 Device Authorization flow (headless agents & TV clients)
//! 3. Application Default Credentials (`authorized_user` JSON refresh without external CLI)
//! 4. Graceful `gcloud auth print-access-token` + `gcloud config get-value project` fallback
//!    when `GOOGLE_APPLICATION_CREDENTIALS` is unset or points to a missing file.

use anyhow::{Context as _, Result, bail};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use url::Url;

pub const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_DEVICE_CODE_ENDPOINT: &str = "https://oauth2.googleapis.com/device/code";

/// Standard OAuth 2.0 scopes spanning Google Cloud, Gemini/Vertex AI, YouTube, and Workspace.
pub const DEFAULT_GOOGLE_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/generative-language.retriever",
    "https://www.googleapis.com/auth/youtube",
    "https://www.googleapis.com/auth/youtube.force-ssl",
    "https://www.googleapis.com/auth/yt-analytics.readonly",
    "https://www.googleapis.com/auth/drive",
    "https://www.googleapis.com/auth/gmail.modify",
    "https://www.googleapis.com/auth/calendar",
    "https://www.googleapis.com/auth/documents",
    "https://www.googleapis.com/auth/spreadsheets",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialSource {
    ExplicitBearer,
    OAuthPkce,
    OAuthDeviceFlow,
    ApplicationDefaultUser,
    GcloudCliUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleTokenBundle {
    pub access_token: String,
    pub token_type: String,
    pub expires_in_secs: Option<u64>,
    pub refresh_token: Option<String>,
    pub quota_project: Option<String>,
    pub account: Option<String>,
    pub source: CredentialSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleAuthConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub project_id: Option<String>,
    pub region: String,
    pub scopes: Vec<String>,
}

impl Default for GoogleAuthConfig {
    fn default() -> Self {
        Self {
            client_id: std::env::var("APHRODY_GOOGLE_CLIENT_ID")
                .ok()
                .or_else(|| std::env::var("GOOGLE_OAUTH_CLIENT_ID").ok()),
            client_secret: std::env::var("APHRODY_GOOGLE_CLIENT_SECRET")
                .ok()
                .or_else(|| std::env::var("GOOGLE_OAUTH_CLIENT_SECRET").ok()),
            project_id: std::env::var("GOOGLE_CLOUD_PROJECT")
                .ok()
                .or_else(|| std::env::var("GCLOUD_PROJECT").ok()),
            region: std::env::var("GOOGLE_CLOUD_REGION")
                .unwrap_or_else(|_| "us-central1".to_string()),
            scopes: DEFAULT_GOOGLE_SCOPES
                .iter()
                .map(|scope| (*scope).to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PkceAuthRequest {
    pub authorization_url: String,
    pub code_verifier: String,
    pub code_challenge: String,
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAuthorizationStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleAuthStatus {
    pub authenticated: bool,
    pub source: Option<CredentialSource>,
    pub project_id: Option<String>,
    pub region: String,
    pub account: Option<String>,
    pub has_adc_file: bool,
    pub scopes_count: usize,
    pub diagnostic: Option<String>,
}

/// Computes an RFC 7636 PKCE `S256` code challenge from a verifier string.
#[must_use]
pub fn pkce_s256_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// Generates a deterministic-length URL-safe random token using system time + process entropy.
#[must_use]
pub fn generate_url_safe_entropy(seed_label: &str) -> String {
    let now_nanos = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(seed_label.as_bytes());
    hasher.update(now_nanos.to_le_bytes());
    #[cfg(not(target_family = "wasm"))]
    hasher.update(std::process::id().to_le_bytes());
    let first = hasher.finalize();

    let mut second_hasher = Sha256::new();
    second_hasher.update(first);
    second_hasher.update(b"aphrody-pkce-v1");
    let second = second_hasher.finalize();

    let mut combined = Vec::with_capacity(48);
    combined.extend_from_slice(&first);
    combined.extend_from_slice(&second[..16]);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(combined)
}

/// Builds a Google OAuth 2.0 Authorization Code + PKCE S256 request URL.
pub fn build_pkce_auth_request(
    client_id: &str,
    redirect_uri: &str,
    scopes: &[String],
) -> Result<PkceAuthRequest> {
    if client_id.trim().is_empty() {
        bail!("OAuth client_id cannot be empty");
    }
    let code_verifier = generate_url_safe_entropy("verifier");
    let code_challenge = pkce_s256_challenge(&code_verifier);
    let state = generate_url_safe_entropy("state");

    let mut url = Url::parse(GOOGLE_AUTH_ENDPOINT).context("invalid Google OAuth auth URL")?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &scopes.join(" "))
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");

    Ok(PkceAuthRequest {
        authorization_url: url.to_string(),
        code_verifier,
        code_challenge,
        state,
        redirect_uri: redirect_uri.to_string(),
    })
}

/// Resolves the standard `gcloud` Application Default Credentials JSON file path on native OSes.
#[cfg(not(target_family = "wasm"))]
#[must_use]
pub fn default_adc_path() -> Option<std::path::PathBuf> {
    if let Ok(explicit) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        let candidate = std::path::PathBuf::from(explicit);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        let candidate = std::path::PathBuf::from(appdata)
            .join("gcloud")
            .join("application_default_credentials.json");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let candidate = std::path::PathBuf::from(home)
            .join(".config")
            .join("gcloud")
            .join("application_default_credentials.json");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(target_family = "wasm")]
#[must_use]
pub fn default_adc_path() -> Option<std::path::PathBuf> {
    None
}

#[cfg(all(not(target_family = "wasm"), feature = "native"))]
#[derive(Debug, Deserialize)]
struct AuthorizedUserAdc {
    #[serde(rename = "type")]
    cred_type: String,
    client_id: Option<String>,
    client_secret: Option<String>,
    refresh_token: Option<String>,
    quota_project_id: Option<String>,
}

#[cfg(all(not(target_family = "wasm"), feature = "native"))]
#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    token_type: Option<String>,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
}

/// Manages Google OAuth 2.0 & `gcloud` authentication across native and headless environments.
#[derive(Debug, Clone)]
pub struct GoogleAuthManager {
    config: GoogleAuthConfig,
    explicit_bearer: Option<String>,
}

impl Default for GoogleAuthManager {
    fn default() -> Self {
        Self::new(GoogleAuthConfig::default())
    }
}

impl GoogleAuthManager {
    #[must_use]
    pub fn new(config: GoogleAuthConfig) -> Self {
        let explicit_bearer = std::env::var("APHRODY_GOOGLE_ACCESS_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self {
            config,
            explicit_bearer,
        }
    }

    #[must_use]
    pub fn with_bearer(mut self, bearer: impl Into<String>) -> Self {
        self.explicit_bearer = Some(bearer.into());
        self
    }

    #[must_use]
    pub fn config(&self) -> &GoogleAuthConfig {
        &self.config
    }

    /// Generates an OAuth 2.0 PKCE browser authorization request.
    pub fn prepare_pkce_login(&self, redirect_uri: &str) -> Result<PkceAuthRequest> {
        let client_id = self
            .config
            .client_id
            .as_deref()
            .context("set APHRODY_GOOGLE_CLIENT_ID or GOOGLE_OAUTH_CLIENT_ID for PKCE login")?;
        build_pkce_auth_request(client_id, redirect_uri, &self.config.scopes)
    }

    /// Starts an OAuth 2.0 Device Flow authorization session for headless agents.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn start_device_flow(&self) -> Result<DeviceAuthorizationStart> {
        let client_id = self
            .config
            .client_id
            .as_deref()
            .context("set APHRODY_GOOGLE_CLIENT_ID or GOOGLE_OAUTH_CLIENT_ID for device flow")?;
        let scope = self.config.scopes.join(" ");
        let mut response = ureq::post(GOOGLE_DEVICE_CODE_ENDPOINT)
            .send_form([("client_id", client_id), ("scope", scope.as_str())])
            .context("failed to initiate Google OAuth device flow")?;
        let text = response
            .body_mut()
            .read_to_string()
            .context("failed to read Google OAuth device response")?;
        let parsed: DeviceAuthorizationStart =
            serde_json::from_str(&text).context("invalid JSON from Google OAuth device endpoint")?;
        Ok(parsed)
    }

    /// Exchanges an authorization code + PKCE verifier for an OAuth 2.0 access/refresh token pair.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn exchange_pkce_code(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<GoogleTokenBundle> {
        let client_id = self
            .config
            .client_id
            .as_deref()
            .context("missing OAuth client_id for token exchange")?;
        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ];
        if let Some(secret) = self.config.client_secret.as_deref() {
            params.push(("client_secret", secret));
        }
        let mut response = ureq::post(GOOGLE_TOKEN_ENDPOINT)
            .send_form(params)
            .context("failed to exchange Google OAuth authorization code")?;
        let text = response
            .body_mut()
            .read_to_string()
            .context("failed to read OAuth token response")?;
        let parsed: OAuthTokenResponse =
            serde_json::from_str(&text).context("invalid OAuth token response JSON")?;
        Ok(GoogleTokenBundle {
            access_token: parsed.access_token,
            token_type: parsed.token_type.unwrap_or_else(|| "Bearer".to_string()),
            expires_in_secs: parsed.expires_in,
            refresh_token: parsed.refresh_token,
            quota_project: self.config.project_id.clone(),
            account: None,
            source: CredentialSource::OAuthPkce,
        })
    }

    /// Renders a styled callback page reusing the workspace's native `oauth_callback_server`.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    #[must_use]
    pub fn render_callback_html(title: &str, message: &str, is_error: bool) -> String {
        oauth_callback_server::oauth_callback_page(title, message, is_error)
    }

    /// Resolves a usable Google OAuth Bearer token across explicit token -> ADC file -> `gcloud` CLI.
    pub fn resolve_token(&self) -> Result<GoogleTokenBundle> {
        if let Some(token) = &self.explicit_bearer {
            return Ok(GoogleTokenBundle {
                access_token: token.clone(),
                token_type: "Bearer".to_string(),
                expires_in_secs: None,
                refresh_token: None,
                quota_project: self.config.project_id.clone(),
                account: None,
                source: CredentialSource::ExplicitBearer,
            });
        }

        #[cfg(all(not(target_family = "wasm"), feature = "native"))]
        {
            if let Ok(bundle) = self.resolve_from_adc_file() {
                return Ok(bundle);
            }
            if let Ok(bundle) = self.resolve_from_gcloud_cli() {
                return Ok(bundle);
            }
        }

        bail!(
            "no Google OAuth token available: configure APHRODY_GOOGLE_ACCESS_TOKEN, valid ADC, or sign in via `gcloud auth login`"
        )
    }

    /// Probes authentication readiness without leaking raw bearer tokens.
    #[must_use]
    pub fn status(&self) -> GoogleAuthStatus {
        let has_adc_file = default_adc_path().is_some();
        match self.resolve_token() {
            Ok(bundle) => GoogleAuthStatus {
                authenticated: true,
                source: Some(bundle.source),
                project_id: bundle.quota_project.or_else(|| self.config.project_id.clone()),
                region: self.config.region.clone(),
                account: bundle.account,
                has_adc_file,
                scopes_count: self.config.scopes.len(),
                diagnostic: None,
            },
            Err(err) => GoogleAuthStatus {
                authenticated: false,
                source: None,
                project_id: self.config.project_id.clone(),
                region: self.config.region.clone(),
                account: None,
                has_adc_file,
                scopes_count: self.config.scopes.len(),
                diagnostic: Some(err.to_string()),
            },
        }
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    fn resolve_from_adc_file(&self) -> Result<GoogleTokenBundle> {
        let path = default_adc_path().context("no valid ADC file found")?;
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read ADC file {}", path.display()))?;
        let adc: AuthorizedUserAdc =
            serde_json::from_str(&raw).context("failed to parse ADC JSON")?;
        if adc.cred_type != "authorized_user" {
            bail!("unsupported ADC credential type: {}", adc.cred_type);
        }
        let client_id = adc.client_id.context("ADC missing client_id")?;
        let client_secret = adc.client_secret.context("ADC missing client_secret")?;
        let refresh_token = adc.refresh_token.context("ADC missing refresh_token")?;

        let mut response = ureq::post(GOOGLE_TOKEN_ENDPOINT)
            .send_form([
                ("grant_type", "refresh_token"),
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token.as_str()),
            ])
            .context("failed to refresh Google ADC authorized_user token")?;
        let text = response
            .body_mut()
            .read_to_string()
            .context("failed to read ADC token refresh response")?;
        let parsed: OAuthTokenResponse =
            serde_json::from_str(&text).context("invalid JSON refreshing ADC token")?;

        Ok(GoogleTokenBundle {
            access_token: parsed.access_token,
            token_type: parsed.token_type.unwrap_or_else(|| "Bearer".to_string()),
            expires_in_secs: parsed.expires_in,
            refresh_token: Some(refresh_token),
            quota_project: adc.quota_project_id.or_else(|| self.config.project_id.clone()),
            account: None,
            source: CredentialSource::ApplicationDefaultUser,
        })
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    fn resolve_from_gcloud_cli(&self) -> Result<GoogleTokenBundle> {
        let token = run_gcloud(&["auth", "print-access-token"])
            .context("gcloud auth print-access-token failed")?;
        if token.is_empty() {
            bail!("gcloud returned an empty access token");
        }
        let project = self
            .config
            .project_id
            .clone()
            .or_else(|| run_gcloud(&["config", "get-value", "project"]).ok())
            .filter(|value| !value.is_empty() && value != "(unset)");
        let account = run_gcloud(&["config", "get-value", "account"])
            .ok()
            .filter(|value| !value.is_empty() && value != "(unset)");

        Ok(GoogleTokenBundle {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in_secs: Some(3600),
            refresh_token: None,
            quota_project: project,
            account,
            source: CredentialSource::GcloudCliUser,
        })
    }
}

#[cfg(all(not(target_family = "wasm"), feature = "native"))]
fn run_gcloud(args: &[&str]) -> Result<String> {
    #[cfg(target_os = "windows")]
    let candidates = ["gcloud.cmd", "gcloud"];
    #[cfg(not(target_os = "windows"))]
    let candidates = ["gcloud"];

    let mut last_error: Option<anyhow::Error> = None;
    for binary in candidates {
        match std::process::Command::new(binary)
            .args(args)
            .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
            .output()
        {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(text);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                last_error = Some(anyhow::anyhow!("{binary} {:?} failed: {stderr}", args));
            }
            Err(err) => {
                last_error = Some(err.into());
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("gcloud executable not found")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_s256_matches_rfc7636_test_vector() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(pkce_s256_challenge(verifier), expected);
    }

    #[test]
    fn build_pkce_auth_request_contains_required_params() {
        let scopes = vec![
            "https://www.googleapis.com/auth/youtube".to_string(),
            "https://www.googleapis.com/auth/cloud-platform".to_string(),
        ];
        let request = build_pkce_auth_request("test-client.apps.googleusercontent.com", "http://127.0.0.1:49152/callback", &scopes)
            .expect("pkce request");
        assert!(request.authorization_url.starts_with(GOOGLE_AUTH_ENDPOINT));
        assert!(request.authorization_url.contains("code_challenge_method=S256"));
        assert!(request.authorization_url.contains("access_type=offline"));
        assert_eq!(pkce_s256_challenge(&request.code_verifier), request.code_challenge);
    }

    #[test]
    fn explicit_bearer_resolves_without_external_network() {
        let manager = GoogleAuthManager::default().with_bearer("ya29.test-token");
        let bundle = manager.resolve_token().expect("explicit bearer");
        assert_eq!(bundle.access_token, "ya29.test-token");
        assert_eq!(bundle.source, CredentialSource::ExplicitBearer);
        let status = manager.status();
        assert!(status.authenticated);
        assert_eq!(status.source, Some(CredentialSource::ExplicitBearer));
    }
}
