//! The curl of this workspace: one guarded request, with the response head and body.

use std::sync::Arc;
use std::time::Duration;

use ay_open_browser_engines::SsrfGuard;
use ay_open_browser_engines::ssrf::SsrfOptions;
use reqwest::Method;
use url::Url;

use crate::{Result, net};

#[derive(Debug, Clone)]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub user_agent: String,
    pub timeout: Duration,
    pub follow_redirects: bool,
    pub ssrf: SsrfOptions,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: "GET".into(),
            headers: Vec::new(),
            body: None,
            user_agent: format!("open-fetch/{}", env!("CARGO_PKG_VERSION")),
            timeout: Duration::from_secs(30),
            follow_redirects: true,
            ssrf: SsrfOptions::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Response {
    pub url: String,
    pub status: u16,
    pub version: String,
    pub headers: Vec<(String, String)>,
    #[serde(skip)]
    pub body: Vec<u8>,
    pub elapsed_ms: f64,
}

pub async fn send(request: Request) -> Result<Response> {
    let started = std::time::Instant::now();
    let url = Url::parse(&request.url)?;
    let guard = Arc::new(SsrfGuard::new(request.ssrf.clone()));
    guard.check_url(&url)?;
    let mut builder = net::client_builder(guard, &request.user_agent, request.timeout);
    if !request.follow_redirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    let client = builder.build()?;
    let method = Method::from_bytes(request.method.to_ascii_uppercase().as_bytes()).unwrap_or(Method::GET);
    let mut call = client.request(method, url);
    for (name, value) in &request.headers {
        call = call.header(name, value);
    }
    if let Some(body) = request.body {
        call = call.body(body);
    }
    let response = call.send().await?;
    let (status, version, final_url) = (response.status().as_u16(), format!("{:?}", response.version()), response.url().to_string());
    let headers = response.headers().iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string())).collect();
    let body = response.bytes().await?.to_vec();
    Ok(Response { url: final_url, status, version, headers, body, elapsed_ms: started.elapsed().as_secs_f64() * 1000.0 })
}
