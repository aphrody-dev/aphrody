//! A browse `Browser` over a plain CDP connection: tabs are targets of one browser WebSocket, each tab's `Cdp` is a flat
//! session of it, with out-of-process iframes delivered as child sessions.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, anyhow, bail};
use async_trait::async_trait;
use browse_core::{Browser, Cdp, CdpError, CdpEvent, HandoffResult, TabId, TabInfo, TabOptions, TabState, Tool, ToolContext, ToolOutput, ToolSpec};
use ay_open_browser_engines::cdp::{Client, Event, Session};
use ay_open_browser_engines::launch::{LaunchOptions, Launched, launch};
use serde_json::{Value, json};
use tokio::sync::broadcast;

/// One tab's connection as browse sees it.
struct TabCdp {
    client: Client,
    session: String,
    events: Mutex<Option<broadcast::Sender<CdpEvent>>>,
}

impl TabCdp {
    fn new(client: Client, session: String) -> Arc<Self> {
        let (tx, _) = broadcast::channel(2048);
        let this = Arc::new(Self { client: client.clone(), session: session.clone(), events: Mutex::new(Some(tx)) });
        let weak = Arc::downgrade(&this);
        let mut source = client.events();
        tokio::spawn(async move {
            let mut children: HashSet<String> = HashSet::new();
            loop {
                let event: Event = match source.recv().await {
                    Ok(event) => event,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                };
                let Some(this) = weak.upgrade() else { break };
                let Some(from) = event.session_id.clone() else { continue };
                let mapped = if from == this.session {
                    None
                } else if children.contains(&from) {
                    Some(from.clone())
                } else {
                    continue;
                };
                if event.method == "Target.attachedToTarget" {
                    if let Some(child) = event.params["sessionId"].as_str() {
                        children.insert(child.to_string());
                    }
                }
                let detached = (event.method == "Target.detachedFromTarget" && event.params["sessionId"].as_str() == Some(this.session.as_str())) || (event.method == "Inspector.detached" && from == this.session);
                if let Some(tx) = this.events.lock().unwrap().as_ref() {
                    let _ = tx.send(CdpEvent { method: event.method, params: event.params, session_id: mapped });
                }
                if detached {
                    this.events.lock().unwrap().take();
                    break;
                }
            }
            if let Some(this) = weak.upgrade() {
                this.events.lock().unwrap().take();
            }
        });
        this
    }
}

#[async_trait]
impl Cdp for TabCdp {
    async fn send(&self, method: &str, params: Value, session_id: Option<&str>) -> Result<Value, CdpError> {
        self.client.send(method, params, Some(session_id.unwrap_or(&self.session))).await.map_err(|error| match error {
            ay_open_browser_engines::CdpError::Closed => CdpError::Disconnected,
            ay_open_browser_engines::CdpError::Timeout(ms) => CdpError::Timeout(ms),
            ay_open_browser_engines::CdpError::Protocol(message) if message.contains("No target") || message.contains("closed") => CdpError::TargetClosed,
            other => CdpError::Protocol(other.to_string()),
        })
    }

    fn events(&self) -> broadcast::Receiver<CdpEvent> {
        match self.events.lock().unwrap().as_ref() {
            Some(tx) => tx.subscribe(),
            // The attachment ended: hand back a stream that is already closed.
            None => broadcast::channel(1).1,
        }
    }
}

struct Tab {
    info: TabInfo,
    session: Session,
    cdp: Arc<TabCdp>,
    state: Arc<TabState>,
}

pub struct DirectBrowser {
    client: Client,
    tabs: tokio::sync::Mutex<HashMap<TabId, Tab>>,
    next: AtomicI64,
    // Keeps a browser this value launched alive; dropping it kills the process.
    _launched: Option<Launched>,
}

impl DirectBrowser {
    /// Connects to a running browser's DevTools WebSocket (`ws://127.0.0.1:<port>/devtools/browser/<id>`).
    pub async fn connect(endpoint: &str) -> anyhow::Result<Arc<Self>> {
        let client = Client::connect(endpoint).await.with_context(|| format!("connecting to {endpoint}"))?;
        Ok(Arc::new(Self { client, tabs: Default::default(), next: AtomicI64::new(1), _launched: None }))
    }

    /// Launches a headless Chromium-family browser (see `ay_open_browser_engines::registry`) and connects to it.
    pub async fn launch(options: LaunchOptions) -> anyhow::Result<Arc<Self>> {
        let launched = launch(options).await?;
        let client = Client::connect(&launched.ws_endpoint).await?;
        Ok(Arc::new(Self { client, tabs: Default::default(), next: AtomicI64::new(1), _launched: Some(launched) }))
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[async_trait]
impl Browser for DirectBrowser {
    async fn open_tab(&self, url: Option<&str>, options: &TabOptions) -> anyhow::Result<TabInfo> {
        let target = self.client.create_target("about:blank").await?;
        let session = self.client.attach(&target).await?;
        let cdp = TabCdp::new(self.client.clone(), session.id.clone());
        // Children are attached by the page tools through Target.setAutoAttach; nothing more to arrange here.
        if let Some(url) = url.filter(|u| *u != "about:blank") {
            session.navigate(url, Duration::from_secs(60)).await.map_err(|e| anyhow!("navigation to {url} failed: {e}"))?;
        }
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let info = TabInfo { tab: id, url: url.unwrap_or("about:blank").to_string(), title: String::new(), lent: false, opener: None, options: options.clone() };
        self.tabs.lock().await.insert(id, Tab { info: info.clone(), session, cdp, state: Arc::default() });
        Ok(info)
    }

    async fn tabs(&self) -> anyhow::Result<Vec<TabInfo>> {
        let mut tabs = self.tabs.lock().await;
        let mut list = Vec::new();
        for tab in tabs.values_mut() {
            if let Ok(reply) = self.client.send("Target.getTargetInfo", json!({ "targetId": tab.session.target_id }), None).await {
                if let Some(url) = reply["targetInfo"]["url"].as_str() {
                    tab.info.url = url.to_string();
                }
                if let Some(title) = reply["targetInfo"]["title"].as_str() {
                    tab.info.title = title.to_string();
                }
            }
            list.push(tab.info.clone());
        }
        list.sort_by_key(|t| t.tab);
        Ok(list)
    }

    async fn close_tab(&self, tab: TabId) -> anyhow::Result<()> {
        let removed = self.tabs.lock().await.remove(&tab).ok_or_else(|| anyhow!("no tab {tab}"))?;
        removed.session.close().await;
        Ok(())
    }

    async fn cdp(&self, tab: TabId) -> anyhow::Result<Arc<dyn Cdp>> {
        let tabs = self.tabs.lock().await;
        let tab = tabs.get(&tab).ok_or_else(|| anyhow!("no tab {tab}"))?;
        Ok(tab.cdp.clone())
    }

    async fn state(&self, tab: TabId) -> anyhow::Result<Arc<TabState>> {
        let tabs = self.tabs.lock().await;
        Ok(tabs.get(&tab).ok_or_else(|| anyhow!("no tab {tab}"))?.state.clone())
    }

    async fn handoff(&self, _tab: TabId, message: &str) -> anyhow::Result<HandoffResult> {
        Ok(HandoffResult { outcome: "cancelled".into(), note: Some(format!("This browser is driven directly over CDP and has no user toolbar to continue from; the request was: {message}")) })
    }

    async fn human_input(&self, _tab: TabId) -> bool {
        false
    }
}

/// `tabs` for a direct browser: list, new, close.
pub struct TabsTool;

#[async_trait]
impl Tool for TabsTool {
    fn spec(&self) -> ToolSpec {
        let mut properties = TabOptions::schema();
        properties.insert("action".into(), json!({ "type": "string", "enum": ["list", "new", "close"], "description": "What to do. Defaults to list." }));
        properties.insert("url".into(), json!({ "type": "string", "description": "new: the URL to open (else about:blank)." }));
        properties.insert("tab".into(), json!({ "type": "integer", "description": "close: the tab to close." }));
        ToolSpec {
            name: "tabs".into(),
            description: "List, open or close tabs. Tabs are the handle every other tool takes as `tab`. `new` opens a tab (at `url`, else about:blank), waits for it to load and returns its `tab` id; `navigate` without `tab` does the same.".into(),
            input_schema: json!({ "type": "object", "properties": properties, "additionalProperties": false }),
        }
    }

    async fn call(&self, ctx: ToolContext, args: Value) -> anyhow::Result<ToolOutput> {
        let action = args.get("action").and_then(Value::as_str).unwrap_or("list");
        match action {
            "list" => Ok(ToolOutput::text(json!({ "tabs": ctx.browser.tabs().await? }).to_string())),
            "new" => {
                let info = ctx.browser.open_tab(args.get("url").and_then(Value::as_str), &TabOptions::from_args(&args)?).await?;
                Ok(ToolOutput::text(serde_json::to_string(&info)?))
            }
            "close" => {
                let tab = browse_core::require_tab(&args)?;
                ctx.browser.close_tab(tab).await?;
                Ok(ToolOutput::text(json!({ "closed": [tab] }).to_string()))
            }
            other => bail!("Unknown action {other:?}: use list, new or close"),
        }
    }
}
