//! Async Chrome DevTools Protocol client over one browser WebSocket, with flat sessions (`Target.attachToTarget`
//! with `flatten: true`): commands carry a `sessionId`, events fan out on a broadcast channel.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, thiserror::Error)]
pub enum CdpError {
    #[error("{0}")]
    Protocol(String),
    #[error("the browser connection is closed")]
    Closed,
    #[error("timed out after {0} ms")]
    Timeout(u64),
    #[error("transport: {0}")]
    Transport(String),
}

#[derive(Debug, Clone)]
pub struct Event {
    pub method: String,
    pub params: Value,
    pub session_id: Option<String>,
}

type Pending = Arc<std::sync::Mutex<HashMap<u64, oneshot::Sender<Result<Value, CdpError>>>>>;

struct Inner {
    out: mpsc::UnboundedSender<String>,
    next: AtomicU64,
    pending: Pending,
    events: broadcast::Sender<Event>,
}

/// One browser connection. Cheap to clone.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl Client {
    /// Connects to `ws://host:port/devtools/browser/<id>` (or a page endpoint).
    pub async fn connect(endpoint: &str) -> Result<Self, CdpError> {
        let (socket, _) = tokio_tungstenite::connect_async(endpoint).await.map_err(|e| CdpError::Transport(e.to_string()))?;
        let (mut sink, mut stream) = socket.split();
        let (out, mut rx) = mpsc::unbounded_channel::<String>();
        let pending: Pending = Arc::default();
        let (events, _) = broadcast::channel(4096);

        tokio::spawn(async move {
            while let Some(text) = rx.recv().await {
                if sink.send(Message::text(text)).await.is_err() {
                    break;
                }
            }
            let _ = sink.close().await;
        });

        let reader_pending = pending.clone();
        let reader_events = events.clone();
        tokio::spawn(async move {
            while let Some(Ok(message)) = stream.next().await {
                let Message::Text(text) = message else { continue };
                let Ok(value) = serde_json::from_str::<Value>(text.as_str()) else { continue };
                if let Some(id) = value.get("id").and_then(Value::as_u64) {
                    let reply = match value.get("error") {
                        Some(error) => Err(CdpError::Protocol(error.get("message").and_then(Value::as_str).unwrap_or("protocol error").to_string())),
                        None => Ok(value.get("result").cloned().unwrap_or(Value::Null)),
                    };
                    if let Some(tx) = reader_pending.lock().unwrap().remove(&id) {
                        let _ = tx.send(reply);
                    }
                } else if let Some(method) = value.get("method").and_then(Value::as_str) {
                    let _ = reader_events.send(Event {
                        method: method.to_string(),
                        params: value.get("params").cloned().unwrap_or(Value::Null),
                        session_id: value.get("sessionId").and_then(Value::as_str).map(str::to_string),
                    });
                }
            }
            for (_, tx) in reader_pending.lock().unwrap().drain() {
                let _ = tx.send(Err(CdpError::Closed));
            }
        });

        Ok(Self { inner: Arc::new(Inner { out, next: AtomicU64::new(1), pending, events }) })
    }

    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    pub async fn send(&self, method: &str, params: Value, session: Option<&str>) -> Result<Value, CdpError> {
        self.send_with_timeout(method, params, session, COMMAND_TIMEOUT).await
    }

    pub async fn send_with_timeout(&self, method: &str, params: Value, session: Option<&str>, limit: Duration) -> Result<Value, CdpError> {
        let id = self.inner.next.fetch_add(1, Ordering::Relaxed);
        let mut message = json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            message["sessionId"] = Value::String(session.to_string());
        }
        let (tx, rx) = oneshot::channel();
        self.inner.pending.lock().unwrap().insert(id, tx);
        if self.inner.out.send(message.to_string()).is_err() {
            self.inner.pending.lock().unwrap().remove(&id);
            return Err(CdpError::Closed);
        }
        match tokio::time::timeout(limit, rx).await {
            Ok(Ok(reply)) => reply,
            Ok(Err(_)) => Err(CdpError::Closed),
            Err(_) => {
                self.inner.pending.lock().unwrap().remove(&id);
                Err(CdpError::Timeout(limit.as_millis() as u64))
            }
        }
    }

    /// Opens a target at `url` and returns its target id.
    pub async fn create_target(&self, url: &str) -> Result<String, CdpError> {
        let reply = self.send("Target.createTarget", json!({ "url": url }), None).await?;
        reply["targetId"].as_str().map(str::to_string).ok_or_else(|| CdpError::Protocol("Target.createTarget returned no targetId".into()))
    }

    /// Attaches to a target in flat mode and returns a session bound to it.
    pub async fn attach(&self, target_id: &str) -> Result<Session, CdpError> {
        let reply = self.send("Target.attachToTarget", json!({ "targetId": target_id, "flatten": true }), None).await?;
        let id = reply["sessionId"].as_str().ok_or_else(|| CdpError::Protocol("Target.attachToTarget returned no sessionId".into()))?;
        Ok(Session { client: self.clone(), id: id.to_string(), target_id: target_id.to_string() })
    }

    pub async fn close_target(&self, target_id: &str) -> Result<(), CdpError> {
        self.send("Target.closeTarget", json!({ "targetId": target_id }), None).await.map(|_| ())
    }

    pub async fn version(&self) -> Result<Value, CdpError> {
        self.send("Browser.getVersion", json!({}), None).await
    }

    pub async fn close(&self) {
        let _ = self.send_with_timeout("Browser.close", json!({}), None, Duration::from_secs(2)).await;
    }
}

/// A flat session bound to one target.
#[derive(Clone)]
pub struct Session {
    pub client: Client,
    pub id: String,
    pub target_id: String,
}

impl Session {
    pub async fn send(&self, method: &str, params: Value) -> Result<Value, CdpError> {
        self.client.send(method, params, Some(&self.id)).await
    }

    /// Events of this session only.
    pub fn events(&self) -> SessionEvents {
        SessionEvents { rx: self.client.events(), id: self.id.clone() }
    }

    /// Runs `expression` in the page and returns its JSON value.
    pub async fn evaluate(&self, expression: &str) -> Result<Value, CdpError> {
        let reply = self.send("Runtime.evaluate", json!({ "expression": expression, "returnByValue": true, "awaitPromise": true })).await?;
        if let Some(exception) = reply.get("exceptionDetails") {
            let text = exception.pointer("/exception/description").and_then(Value::as_str).or_else(|| exception["text"].as_str()).unwrap_or("evaluation failed");
            return Err(CdpError::Protocol(text.to_string()));
        }
        Ok(reply.pointer("/result/value").cloned().unwrap_or(Value::Null))
    }

    /// Navigates and waits for `Page.loadEventFired` (or `limit`).
    pub async fn navigate(&self, url: &str, limit: Duration) -> Result<(), CdpError> {
        self.send("Page.enable", json!({})).await?;
        let mut events = self.client.events();
        let reply = self.send("Page.navigate", json!({ "url": url })).await?;
        if let Some(error) = reply.get("errorText").and_then(Value::as_str) {
            return Err(CdpError::Protocol(format!("navigation failed: {error}")));
        }
        let wait = async {
            loop {
                match events.recv().await {
                    Ok(event) if event.session_id.as_deref() == Some(&self.id) && event.method == "Page.loadEventFired" => return Ok(()),
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return Err(CdpError::Closed),
                }
            }
        };
        tokio::time::timeout(limit, wait).await.map_err(|_| CdpError::Timeout(limit.as_millis() as u64))?
    }

    /// PNG bytes of the page; `full_page` captures beyond the viewport.
    pub async fn screenshot_png(&self, full_page: bool) -> Result<Vec<u8>, CdpError> {
        let reply = self.send("Page.captureScreenshot", json!({ "format": "png", "captureBeyondViewport": full_page })).await?;
        let data = reply["data"].as_str().ok_or_else(|| CdpError::Protocol("no screenshot data".into()))?;
        base64::engine::general_purpose::STANDARD.decode(data).map_err(|_| CdpError::Protocol("invalid screenshot data".into()))
    }

    pub async fn close(&self) {
        let _ = self.client.close_target(&self.target_id).await;
    }
}

/// A receiver filtered to one session.
pub struct SessionEvents {
    rx: broadcast::Receiver<Event>,
    id: String,
}

impl SessionEvents {
    /// The next event of the session; None once the connection is closed.
    pub async fn next(&mut self) -> Option<Event> {
        loop {
            match self.rx.recv().await {
                Ok(event) if event.session_id.as_deref() == Some(self.id.as_str()) => return Some(event),
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }
}
