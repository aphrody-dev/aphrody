//! Starts an ephemeral headless Chromium-family browser with a private profile and returns its DevTools endpoint.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::registry;

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("no Chromium or Chrome binary found: set CHROME_PATH or install one (see `open engines`)")]
    NoBinary,
    #[error("could not start {0}: {1}")]
    Spawn(PathBuf, std::io::Error),
    #[error("timed out waiting for the DevTools endpoint ({0} ms)")]
    Timeout(u64),
    #[error("the browser exited before printing its DevTools endpoint: {0}")]
    Exited(String),
}

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub binary: Option<PathBuf>,
    pub headless: bool,
    pub extra_args: Vec<String>,
    pub timeout: Duration,
    /// An unpacked extension to load (`--load-extension`); needs Chrome for Testing or Chromium.
    pub extension: Option<PathBuf>,
    /// Reuse this profile instead of a temporary one (kept on close).
    pub user_data_dir: Option<PathBuf>,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self { binary: None, headless: true, extra_args: Vec::new(), timeout: Duration::from_secs(20), extension: None, user_data_dir: None }
    }
}

pub struct Launched {
    pub binary: PathBuf,
    pub ws_endpoint: String,
    pub user_data_dir: PathBuf,
    temporary_profile: bool,
    child: Child,
}

impl Launched {
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Stops the browser and removes its temporary profile.
    pub async fn close(mut self) {
        let _ = self.child.kill().await;
        if self.temporary_profile {
            let _ = std::fs::remove_dir_all(&self.user_data_dir);
        }
    }
}

fn is_root() -> bool {
    #[cfg(unix)]
    {
        // SAFETY: geteuid has no preconditions and cannot fail.
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub async fn launch(options: LaunchOptions) -> Result<Launched, LaunchError> {
    let binary = options.binary.clone().or_else(registry::resolve_chromium).ok_or(LaunchError::NoBinary)?;
    let (profile, temporary_profile) = match &options.user_data_dir {
        Some(dir) => (dir.clone(), false),
        None => (temp_profile(), true),
    };
    let mut command = Command::new(&binary);
    if options.headless {
        command.arg("--headless=new");
    }
    command.args([
        "--remote-debugging-port=0".to_string(),
        "--remote-allow-origins=*".into(),
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--disable-background-networking".into(),
        "--disable-sync".into(),
        "--window-size=1440,900".into(),
    ]);
    match &options.extension {
        Some(dir) => {
            command.arg(format!("--disable-extensions-except={}", dir.display()));
            command.arg(format!("--load-extension={}", dir.display()));
        }
        None => {
            command.arg("--disable-extensions");
        }
    }
    if is_root() || std::env::var_os("OPEN_BROWSER_ENGINES_NO_SANDBOX").or_else(|| std::env::var_os("BROWSER_ENGINES_NO_SANDBOX")).is_some() {
        command.arg("--no-sandbox");
    }
    command.args(&options.extra_args).arg("about:blank");
    command.stdout(Stdio::null()).stderr(Stdio::piped()).kill_on_drop(true);
    let mut child = command.spawn().map_err(|e| LaunchError::Spawn(binary.clone(), e))?;
    let stderr = child.stderr.take().expect("stderr is piped");

    let wait = async {
        let mut lines = BufReader::new(stderr).lines();
        let mut seen = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(rest) = line.split("DevTools listening on ").nth(1) {
                let endpoint = rest.trim().to_string();
                // Keep draining so the browser never blocks on a full stderr pipe.
                tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
                return Ok(endpoint);
            }
            if seen.len() < 400 {
                seen.push_str(&line);
                seen.push('\n');
            }
        }
        Err(LaunchError::Exited(seen))
    };
    let ws_endpoint = match tokio::time::timeout(options.timeout, wait).await {
        Ok(result) => result?,
        Err(_) => {
            let _ = child.kill().await;
            return Err(LaunchError::Timeout(options.timeout.as_millis() as u64));
        }
    };
    Ok(Launched { binary, ws_endpoint, user_data_dir: profile, temporary_profile, child })
}

fn temp_profile() -> PathBuf {
    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
    let dir = std::env::temp_dir().join(format!("browser-engines-{}-{nonce}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Reads the `DevToolsActivePort` file of a running browser's profile: `ws://127.0.0.1:<port><path>`.
pub fn endpoint_from_profile(profile: &Path) -> Option<String> {
    let text = std::fs::read_to_string(profile.join("DevToolsActivePort")).ok()?;
    let mut lines = text.lines();
    let port = lines.next()?.trim();
    let path = lines.next()?.trim();
    Some(format!("ws://127.0.0.1:{port}{path}"))
}
