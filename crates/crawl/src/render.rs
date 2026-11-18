//! Rendering through a CDP browser: one shared headless browser, one tab per page. Every request and redirect hop of
//! the page goes through the SSRF guard and the shields (Fetch domain interception).

use std::sync::Arc;
use std::time::Duration;

use ay_open_browser_engines::cdp::Client;
use ay_open_browser_engines::launch::{LaunchOptions, Launched, launch};
use ay_open_browser_engines::SsrfGuard;
use serde_json::json;
use url::Url;

use crate::shields::Shields;
use crate::{Error, Result};

pub struct BrowserPool {
    client: Client,
    guard: Arc<SsrfGuard>,
    shields: Shields,
    // Dropping the handle kills the browser.
    _launched: Launched,
}

impl BrowserPool {
    pub async fn start(guard: Arc<SsrfGuard>, shields: Shields) -> Result<Self> {
        let launched = launch(LaunchOptions::default()).await.map_err(|e| Error::Browser(e.to_string()))?;
        let client = Client::connect(&launched.ws_endpoint).await?;
        Ok(Self { client, guard, shields, _launched: launched })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// The DOM (`outerHTML`) of the page after its scripts ran.
    pub async fn render(&self, url: &Url, timeout: Duration) -> Result<String> {
        let target = self.client.create_target("about:blank").await?;
        let session = self.client.attach(&target).await?;
        let outcome = self.render_in(&session, url, timeout).await;
        session.close().await;
        outcome
    }

    async fn render_in(&self, session: &ay_open_browser_engines::Session, url: &Url, timeout: Duration) -> Result<String> {
        session.send("Fetch.enable", json!({ "patterns": [{ "urlPattern": "*" }] })).await?;
        let blocked: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
        let interceptor = {
            let (session, guard, shields, blocked) = (session.clone(), self.guard.clone(), self.shields.clone(), blocked.clone());
            let mut events = session.events();
            tokio::spawn(async move {
                while let Some(event) = events.next().await {
                    if event.method != "Fetch.requestPaused" {
                        continue;
                    }
                    let (Some(id), Some(raw)) = (event.params["requestId"].as_str().map(str::to_string), event.params["request"]["url"].as_str().map(str::to_string)) else { continue };
                    let verdict = verdict(&guard, &shields, &raw).await;
                    let (session, blocked) = (session.clone(), blocked.clone());
                    tokio::spawn(async move {
                        let _ = match verdict {
                            None => session.send("Fetch.continueRequest", json!({ "requestId": id })).await,
                            Some(reason) => {
                                blocked.lock().unwrap().push(reason);
                                session.send("Fetch.failRequest", json!({ "requestId": id, "errorReason": "BlockedByClient" })).await
                            }
                        };
                    });
                }
            })
        };
        let navigated = session.navigate(url.as_str(), timeout).await;
        interceptor.abort();
        let first_block = blocked.lock().unwrap().first().cloned();
        if let Err(error) = navigated {
            return Err(first_block.map_or_else(|| Error::Cdp(error), Error::Browser));
        }
        if let Some(reason) = blocked.lock().unwrap().iter().find(|r| r.starts_with("SSRF guard")) {
            return Err(Error::Browser(reason.clone()));
        }
        let html = session.evaluate("document.documentElement.outerHTML").await?;
        Ok(html.as_str().unwrap_or_default().to_string())
    }
}

/// None to let the request through, otherwise the reason it is refused.
async fn verdict(guard: &SsrfGuard, shields: &Shields, raw: &str) -> Option<String> {
    if ["data:", "blob:", "about:"].iter().any(|p| raw.starts_with(p)) {
        return None;
    }
    let url = match Url::parse(raw) {
        Ok(url) => url,
        Err(error) => return Some(format!("bad URL {raw}: {error}")),
    };
    if let Err(error) = guard.validate(&url).await {
        return Some(error.to_string());
    }
    shields.should_block(&url).then(|| format!("content shields blocked tracker URL: {raw}"))
}
