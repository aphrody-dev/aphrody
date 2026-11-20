//! Pure-Rust Google Workspace Discovery Service client covering Drive, Gmail, Calendar,
//! Docs, Sheets, Slides, Tasks, and Chat (modeled on official Rust `googleworkspace/cli`).

use anyhow::{Context as _, Result};
#[cfg(all(not(target_family = "wasm"), feature = "native"))]
use anyhow::bail;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::auth::GoogleAuthManager;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceServiceSpec {
    pub id: &'static str,
    pub base_url: &'static str,
    pub discovery_version: &'static str,
    pub default_scope: &'static str,
}

pub const WORKSPACE_SERVICES: &[WorkspaceServiceSpec] = &[
    WorkspaceServiceSpec {
        id: "drive",
        base_url: "https://www.googleapis.com/drive/v3",
        discovery_version: "v3",
        default_scope: "https://www.googleapis.com/auth/drive",
    },
    WorkspaceServiceSpec {
        id: "gmail",
        base_url: "https://gmail.googleapis.com/gmail/v1",
        discovery_version: "v1",
        default_scope: "https://www.googleapis.com/auth/gmail.modify",
    },
    WorkspaceServiceSpec {
        id: "calendar",
        base_url: "https://www.googleapis.com/calendar/v3",
        discovery_version: "v3",
        default_scope: "https://www.googleapis.com/auth/calendar",
    },
    WorkspaceServiceSpec {
        id: "docs",
        base_url: "https://docs.googleapis.com/v1",
        discovery_version: "v1",
        default_scope: "https://www.googleapis.com/auth/documents",
    },
    WorkspaceServiceSpec {
        id: "sheets",
        base_url: "https://sheets.googleapis.com/v4",
        discovery_version: "v4",
        default_scope: "https://www.googleapis.com/auth/spreadsheets",
    },
    WorkspaceServiceSpec {
        id: "slides",
        base_url: "https://slides.googleapis.com/v1",
        discovery_version: "v1",
        default_scope: "https://www.googleapis.com/auth/presentations",
    },
    WorkspaceServiceSpec {
        id: "tasks",
        base_url: "https://tasks.googleapis.com/tasks/v1",
        discovery_version: "v1",
        default_scope: "https://www.googleapis.com/auth/tasks",
    },
    WorkspaceServiceSpec {
        id: "chat",
        base_url: "https://chat.googleapis.com/v1",
        discovery_version: "v1",
        default_scope: "https://www.googleapis.com/auth/chat.spaces.readonly",
    },
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceDiscoveryRequest {
    pub service: String,
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub body: Option<Value>,
}

/// Resolves a workspace service specification by short identifier (`drive`, `gmail`, `calendar`, etc.).
#[must_use]
pub fn find_workspace_service(service_id: &str) -> Option<&'static WorkspaceServiceSpec> {
    WORKSPACE_SERVICES
        .iter()
        .find(|spec| spec.id.eq_ignore_ascii_case(service_id))
}

/// Pure-Rust Google Workspace client backed by [`GoogleAuthManager`].
#[derive(Debug, Clone)]
pub struct GoogleWorkspaceClient {
    #[cfg_attr(any(target_family = "wasm", not(feature = "native")), allow(dead_code))]
    auth: GoogleAuthManager,
}

impl GoogleWorkspaceClient {
    #[must_use]
    pub fn new(auth: GoogleAuthManager) -> Self {
        Self { auth }
    }

    pub fn build_url(&self, request: &WorkspaceDiscoveryRequest) -> Result<String> {
        let spec = find_workspace_service(&request.service)
            .with_context(|| format!("unknown Google Workspace service: {}", request.service))?;
        let trimmed_path = request.path.trim_start_matches('/');
        let base = format!("{}/{trimmed_path}", spec.base_url);
        let mut url = Url::parse(&base).context("invalid Google Workspace endpoint URL")?;
        if !request.query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in &request.query {
                pairs.append_pair(key, value);
            }
        }
        Ok(url.to_string())
    }

    /// Executes a dynamic Google Workspace REST call using OAuth 2.0 / `gcloud` credentials.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn call(&self, request: &WorkspaceDiscoveryRequest) -> Result<Value> {
        let url = self.build_url(request)?;
        let token = self.auth.resolve_token()?;
        let method = request.method.to_ascii_uppercase();

        let mut response = match method.as_str() {
            "GET" => {
                let mut req = ureq::get(&url)
                    .header("Authorization", &format!("Bearer {}", token.access_token));
                if let Some(project) = token.quota_project.as_deref() {
                    req = req.header("x-goog-user-project", project);
                }
                req.call()
            }
            "POST" => {
                let mut req = ureq::post(&url)
                    .header("Authorization", &format!("Bearer {}", token.access_token))
                    .header("Content-Type", "application/json");
                if let Some(project) = token.quota_project.as_deref() {
                    req = req.header("x-goog-user-project", project);
                }
                let encoded = serde_json::to_string(
                    request.body.as_ref().unwrap_or(&Value::Object(Default::default())),
                )
                .context("failed to encode Google Workspace POST JSON")?;
                req.send(encoded.as_str())
            }
            "PATCH" => {
                let mut req = ureq::patch(&url)
                    .header("Authorization", &format!("Bearer {}", token.access_token))
                    .header("Content-Type", "application/json");
                if let Some(project) = token.quota_project.as_deref() {
                    req = req.header("x-goog-user-project", project);
                }
                let encoded = serde_json::to_string(
                    request.body.as_ref().unwrap_or(&Value::Object(Default::default())),
                )
                .context("failed to encode Google Workspace PATCH JSON")?;
                req.send(encoded.as_str())
            }
            "DELETE" => {
                let mut req = ureq::delete(&url)
                    .header("Authorization", &format!("Bearer {}", token.access_token));
                if let Some(project) = token.quota_project.as_deref() {
                    req = req.header("x-goog-user-project", project);
                }
                req.call()
            }
            other => bail!("unsupported HTTP method for Google Workspace call: {other}"),
        }
        .with_context(|| format!("Google Workspace request failed: {method} {url}"))?;

        let body_text = response
            .body_mut()
            .read_to_string()
            .context("failed to read Google Workspace response body")?;
        if body_text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&body_text).context("invalid JSON from Google Workspace API")
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn list_drive_files(&self, page_size: u32, query_filter: Option<&str>) -> Result<Value> {
        let mut query = vec![
            ("pageSize".to_string(), page_size.to_string()),
            ("fields".to_string(), "files(id,name,mimeType,modifiedTime,size)".to_string()),
        ];
        if let Some(filter) = query_filter {
            query.push(("q".to_string(), filter.to_string()));
        }
        self.call(&WorkspaceDiscoveryRequest {
            service: "drive".to_string(),
            method: "GET".to_string(),
            path: "files".to_string(),
            query,
            body: None,
        })
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn list_gmail_messages(&self, max_results: u32, query_filter: Option<&str>) -> Result<Value> {
        let mut query = vec![("maxResults".to_string(), max_results.to_string())];
        if let Some(filter) = query_filter {
            query.push(("q".to_string(), filter.to_string()));
        }
        self.call(&WorkspaceDiscoveryRequest {
            service: "gmail".to_string(),
            method: "GET".to_string(),
            path: "users/me/messages".to_string(),
            query,
            body: None,
        })
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn list_calendar_events(&self, max_results: u32) -> Result<Value> {
        self.call(&WorkspaceDiscoveryRequest {
            service: "calendar".to_string(),
            method: "GET".to_string(),
            path: "calendars/primary/events".to_string(),
            query: vec![
                ("maxResults".to_string(), max_results.to_string()),
                ("singleEvents".to_string(), "true".to_string()),
                ("orderBy".to_string(), "startTime".to_string()),
            ],
            body: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_discovery_urls_for_all_workspace_services() {
        let client = GoogleWorkspaceClient::new(GoogleAuthManager::default());
        assert_eq!(WORKSPACE_SERVICES.len(), 8);
        let url = client
            .build_url(&WorkspaceDiscoveryRequest {
                service: "drive".to_string(),
                method: "GET".to_string(),
                path: "/files".to_string(),
                query: vec![("pageSize".to_string(), "10".to_string())],
                body: None,
            })
            .expect("drive url");
        assert_eq!(url, "https://www.googleapis.com/drive/v3/files?pageSize=10");

        let gmail_url = client
            .build_url(&WorkspaceDiscoveryRequest {
                service: "gmail".to_string(),
                method: "GET".to_string(),
                path: "users/me/messages".to_string(),
                query: Vec::new(),
                body: None,
            })
            .expect("gmail url");
        assert_eq!(gmail_url, "https://gmail.googleapis.com/gmail/v1/users/me/messages");
    }
}
