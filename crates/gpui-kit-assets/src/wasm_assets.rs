use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::{JsFuture, spawn_local};

async fn fetch_asset_bytes(url: &str) -> std::result::Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or_else(|| "browser window unavailable".to_owned())?;
    let resp_value = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|err| format!("{err:?}"))?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|err| format!("invalid fetch Response: {err:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let buf_promise = resp
        .array_buffer()
        .map_err(|err| format!("array_buffer failed: {err:?}"))?;
    let buf_value = JsFuture::from(buf_promise)
        .await
        .map_err(|err| format!("array_buffer await failed: {err:?}"))?;
    Ok(js_sys::Uint8Array::new(&buf_value).to_vec())
}

/// WASM implementation - download assets on-demand
pub struct Assets {
    endpoint: SharedString,
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pending: Arc<RwLock<HashMap<String, bool>>>,
}

impl Assets {
    pub fn new(endpoint: impl Into<SharedString>) -> Self {
        Self {
            endpoint: endpoint.into(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new("")
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        if path.starts_with("icons/") && path.ends_with(".svg") {
            // Check if already cached
            if let Ok(cache) = self.cache.read() {
                if let Some(data) = cache.get(path) {
                    return Ok(Some(Cow::Owned(data.clone())));
                }
            }

            // Check if download is already pending
            let is_pending = self
                .pending
                .read()
                .map(|p| p.contains_key(path))
                .unwrap_or(false);

            if !is_pending {
                // Mark as pending and start download
                if let Ok(mut pending) = self.pending.write() {
                    pending.insert(path.to_string(), true);
                }

                let url = format!("{}/assets/{}", self.endpoint, path);
                let path_clone = path.to_string();
                let cache = self.cache.clone();
                let pending = self.pending.clone();

                spawn_local(async move {
                    match fetch_asset_bytes(&url).await {
                        Ok(bytes) => {
                            if let Ok(mut cache) = cache.write() {
                                cache.insert(path_clone.clone(), bytes);
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to fetch icon {}: {}", path_clone, e);
                        }
                    }

                    // Remove from pending
                    if let Ok(mut pending) = pending.write() {
                        pending.remove(&path_clone);
                    }
                });
            }

            // Return error so GPUI will retry (but only log once per icon)
            Err(anyhow!("Wasm assets loading, will be available soon..."))
        } else {
            Ok(None)
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let _ = path;
        Ok(Vec::new())
    }
}
