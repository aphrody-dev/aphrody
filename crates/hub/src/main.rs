//! Hub service: read path, mirror mode, write path (create, preupload, NDJSON commit, LFS basic transfer),
//! listing and search, `whoami-v2` with static tokens, private repositories.
//!
//! Layout under `HUB_DATA`: `repos/{kind}/{ns}/{repo}/{rev}/{path}` (working tree of a revision),
//! `{rev}.sha` (head commit), `{rev}.lfs.json` (path -> {oid,size}), `.private` marker; `lfs/{oid}` content
//! addressed objects (sha256); `cache/{key}` and `{key}.meta` mirror answers.
//! Environment: `HUB_ADDR`, `HUB_DATA`, `HUB_UPSTREAM`, `HUB_UPSTREAM_TOKEN`, `HUB_TOKENS` (`token=user;...`,
//! empty = open mode), `HUB_PUBLIC_URL`, `HUB_LFS_THRESHOLD`, `HUB_MAX_BODY` (buffered bodies), `HUB_MAX_OBJECT` (LFS objects),
//! `HUB_MAX_CONNECTIONS`, `HUB_SECRET` (signed upload links).
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tiny_http::{Header, Method, Response, Server, StatusCode};

mod ui;

struct Cfg {
    data: PathBuf,
    upstream: Option<String>,
    upstream_token: Option<String>,
    tokens: Vec<(String, String)>,
    public: Option<String>,
    lfs_threshold: u64,
    max_body: u64,
    /// largest LFS object accepted (streamed to disk)
    max_object: u64,
    /// key of the signed LFS upload links (HUB_SECRET, else random per process)
    secret: Vec<u8>,
}

struct R {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    len: Option<usize>,
    /// streamed instead of `body`: (file, offset, length)
    file: Option<(PathBuf, u64, u64)>,
}

fn js(status: u16, v: Value) -> R {
    R { status, headers: vec![("Content-Type".into(), "application/json".into())], body: v.to_string().into_bytes(), len: None, file: None }
}

fn err(status: u16, msg: &str) -> R {
    js(status, json!({ "error": msg }))
}

fn main() {
    let envv = |k: &str| env::var(k).ok().filter(|v| !v.is_empty());
    let cfg = Cfg {
        data: PathBuf::from(envv("HUB_DATA").unwrap_or_else(|| "hub-data".into())),
        upstream: envv("HUB_UPSTREAM").map(|u| u.trim_end_matches('/').to_string()),
        upstream_token: envv("HUB_UPSTREAM_TOKEN"),
        tokens: envv("HUB_TOKENS")
            .map(|t| t.split(';').filter_map(|p| p.split_once('=')).map(|(a, b)| (a.trim().into(), b.trim().into())).collect())
            .unwrap_or_default(),
        public: envv("HUB_PUBLIC_URL").map(|u| u.trim_end_matches('/').to_string()),
        lfs_threshold: envv("HUB_LFS_THRESHOLD").and_then(|v| v.parse().ok()).unwrap_or(10 * 1024 * 1024),
        max_body: envv("HUB_MAX_BODY").and_then(|v| v.parse().ok()).unwrap_or(64 << 20),
        max_object: envv("HUB_MAX_OBJECT").and_then(|v| v.parse().ok()).unwrap_or(100 << 30),
        secret: envv("HUB_SECRET").map(String::into_bytes).unwrap_or_else(random_secret),
    };
    let addr = envv("HUB_ADDR").unwrap_or_else(|| "127.0.0.1:8090".into());
    // beyond loopback the hub must not be open to writes, and links must not follow the client's Host header
    let loopback = addr.starts_with("127.") || addr.starts_with("[::1]") || addr.starts_with("localhost:");
    if !loopback && cfg.tokens.is_empty() && envv("HUB_OPEN").as_deref() != Some("1") {
        eprintln!("{addr}: set HUB_TOKENS before serving beyond loopback (or HUB_OPEN=1 for an open hub)");
        std::process::exit(2);
    }
    if !loopback && cfg.public.is_none() {
        eprintln!("{addr}: set HUB_PUBLIC_URL (the https address clients use) before serving beyond loopback");
        std::process::exit(2);
    }
    let server = Server::http(&addr).expect("bind");
    let cfg = std::sync::Arc::new(cfg);
    let limit: usize = envv("HUB_MAX_CONNECTIONS").and_then(|v| v.parse().ok()).unwrap_or(256);
    let active = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for req in server.incoming_requests() {
        use std::sync::atomic::Ordering;
        // one thread per request, up to HUB_MAX_CONNECTIONS at once: a long download never holds the others, and a
        // flood of requests is answered 503 instead of exhausting memory
        if active.fetch_add(1, Ordering::SeqCst) >= limit {
            active.fetch_sub(1, Ordering::SeqCst);
            let _ = req.respond(Response::from_string("{\"error\":\"busy\"}").with_status_code(503));
            continue;
        }
        let (cfg, active) = (cfg.clone(), active.clone());
        std::thread::spawn(move || {
            handle(&cfg, req);
            active.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

/// Request bodies of JSON and NDJSON calls are read whole up to `max_body`; LFS uploads stream to disk.
fn handle(cfg: &Cfg, mut req: tiny_http::Request) {
    let method = req.method().clone();
    let url = req.url().to_string();
    let hdrs: Vec<(String, String)> = req.headers().iter().map(|h| (h.field.as_str().as_str().to_ascii_lowercase(), h.value.as_str().to_string())).collect();
    let r = if method == Method::Put && url.starts_with("/lfs-upload/") {
        lfs_put_stream(cfg, &url, &hdrs, req.as_reader())
    } else {
        let mut body = Vec::new();
        if matches!(method, Method::Post | Method::Put) {
            let _ = req.as_reader().take(cfg.max_body + 1).read_to_end(&mut body);
            if body.len() as u64 > cfg.max_body {
                let _ = req.respond(Response::from_string("{\"error\":\"request body too large (HUB_MAX_BODY)\"}").with_status_code(413));
                return;
            }
        }
        route(cfg, &method, &url, &hdrs, body)
    };
    let headers: Vec<Header> = r.headers.iter().filter_map(|(k, v)| Header::from_bytes(k.as_bytes(), v.as_bytes()).ok()).collect();
    if let Some((p, off, n)) = &r.file {
        if let Ok(mut fh) = fs::File::open(p) {
            use std::io::Seek;
            if fh.seek(std::io::SeekFrom::Start(*off)).is_ok() {
                let _ = req.respond(Response::new(StatusCode(r.status), headers, fh.take(*n), Some(*n as usize), None));
                return;
            }
        }
        let _ = req.respond(Response::from_string("{\"error\":\"storage\"}").with_status_code(500));
        return;
    }
    let len = r.len.unwrap_or(r.body.len());
    let _ = req.respond(Response::new(StatusCode(r.status), headers, Cursor::new(r.body), Some(len), None));
}

/// HMAC-SHA256 (RFC 2104), hex.
fn hmac(key: &[u8], msg: &[u8]) -> String {
    let mut k = if key.len() > 64 { Sha256::digest(key).to_vec() } else { key.to_vec() };
    k.resize(64, 0);
    let inner = Sha256::new().chain_update(k.iter().map(|b| b ^ 0x36).collect::<Vec<u8>>()).chain_update(msg).finalize();
    hex(&Sha256::new().chain_update(k.iter().map(|b| b ^ 0x5c).collect::<Vec<u8>>()).chain_update(inner).finalize())
}

/// Per-process secret from the standard library's OS-seeded hasher keys, when HUB_SECRET is not set.
fn random_secret() -> Vec<u8> {
    use std::hash::{BuildHasher, Hasher};
    (0..4u64).flat_map(|i| {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i);
        h.write_u128(now());
        h.finish().to_le_bytes()
    }).collect()
}

fn signature(cfg: &Cfg, oid: &str, exp: u64, user: &str) -> String {
    hmac(&cfg.secret, format!("{oid}\n{exp}\n{user}").as_bytes())
}

/// The uploader of an LFS object: the bearer token's user, else a valid, unexpired signed link.
fn lfs_uploader(cfg: &Cfg, oid: &str, q: &[(String, String)], h: &[(String, String)]) -> Option<String> {
    if let Some(u) = user_of(cfg, h) {
        return Some(u);
    }
    let (u, exp, sig) = (qget(q, "u")?, qget(q, "exp")?.parse::<u64>().ok()?, qget(q, "sig")?);
    let expect = signature(cfg, oid, exp, u);
    let same = expect.len() == sig.len() && expect.bytes().zip(sig.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0;
    (same && exp >= (now() / 1_000_000_000) as u64).then(|| u.to_string())
}

/// PUT /lfs-upload/{oid}: the body goes to a temporary file while it is hashed, then is kept under its oid.
fn lfs_put_stream(cfg: &Cfg, url: &str, h: &[(String, String)], body: &mut dyn Read) -> R {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let q = parse_q(query);
    let oid = path.trim_start_matches("/lfs-upload/").to_ascii_lowercase();
    if oid.len() != 64 || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
        return err(400, "invalid oid");
    }
    if lfs_uploader(cfg, &oid, &q, h).is_none() {
        return err(401, "Unauthorized");
    }
    let dir = cfg.data.join("lfs");
    if fs::create_dir_all(&dir).is_err() {
        return err(500, "storage");
    }
    let tmp = dir.join(format!("tmp-{oid}-{}", now()));
    let Ok(mut out) = fs::File::create(&tmp) else { return err(500, "storage") };
    let mut hasher = Sha256::new();
    let (mut buf, mut total) = (vec![0u8; 1 << 20], 0u64);
    loop {
        let n = match body.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => {
                let _ = fs::remove_file(&tmp);
                return err(400, "body");
            }
        };
        total += n as u64;
        if total > cfg.max_object || std::io::Write::write_all(&mut out, &buf[..n]).is_err() {
            let _ = fs::remove_file(&tmp);
            return err(413, "object too large (HUB_MAX_OBJECT)");
        }
        hasher.update(&buf[..n]);
    }
    drop(out);
    if hex(&hasher.finalize()) != oid {
        let _ = fs::remove_file(&tmp);
        return err(400, "oid does not match the content");
    }
    if fs::rename(&tmp, dir.join(&oid)).is_err() {
        let _ = fs::remove_file(&tmp);
        return err(500, "storage");
    }
    R { status: 200, headers: vec![], body: vec![], len: None, file: None }
}

// ---------- helpers ----------

fn pdecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && s.is_char_boundary(i + 1) && s.is_char_boundary(i + 3) {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                o.push(v);
                i += 3;
                continue;
            }
        }
        o.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

fn pencode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn parse_q(q: &str) -> Vec<(String, String)> {
    q.split('&').filter(|p| !p.is_empty()).map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (pdecode(k), pdecode(v))
    }).collect()
}

fn qget<'a>(q: &'a [(String, String)], k: &str) -> Option<&'a str> {
    q.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
}

fn hget<'a>(h: &'a [(String, String)], k: &str) -> Option<&'a str> {
    h.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
}

/// One path segment that stays inside its directory on every platform: no separators, no drive prefix (`C:`),
/// no Windows device name (`con`, `nul`, `com1`...), no dot-only name.
fn safe_seg(s: &str) -> bool {
    const DEVICES: [&str; 22] = ["con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9"];
    let stem = s.split('.').next().unwrap_or("").to_ascii_lowercase();
    !s.is_empty() && !s.chars().all(|c| c == '.') && !s.contains(['/', '\\', '\0', ':']) && !DEVICES.contains(&stem.as_str())
}

/// A revision name a writer may create: letters, digits, `.`, `_`, `-` (`/` kept for branch names, stored as
/// %2F), no leading dot, and never the name of a revision's sidecar files (`.sha`, `.log`, `.lfs.json`).
fn valid_rev(rev: &str) -> bool {
    !rev.is_empty()
        && rev.len() <= 100
        && !rev.starts_with('.')
        && rev.split('/').all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)))
        && ![".sha", ".log", ".lfs.json"].iter().any(|x| rev.ends_with(x))
        && safe_seg(&rev_name(rev))
}

fn safe_path(p: &str) -> bool {
    !p.is_empty() && !p.starts_with('/') && p.split('/').all(safe_seg)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn sha256(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

fn now() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
}

fn rfc3339(secs: u64) -> String {
    // civil-from-days (Howard Hinnant)
    let (d, rem) = ((secs / 86400) as i64, secs % 86400);
    let z = d + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{day:02}T{:02}:{:02}:{:02}.000Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn mtime(p: &Path) -> String {
    let s = fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    rfc3339(s)
}

fn repo_dir(cfg: &Cfg, kind: &str, ns: &str, repo: &str) -> Option<PathBuf> {
    (safe_seg(ns) && safe_seg(repo)).then(|| cfg.data.join("repos").join(kind).join(ns).join(repo))
}

fn rev_name(rev: &str) -> String {
    rev.replace('/', "%2F")
}

fn head_sha(dir: &Path, rev: &str) -> String {
    fs::read_to_string(dir.join(format!("{}.sha", rev_name(rev)))).map(|s| s.trim().to_string()).unwrap_or_else(|_| "0".repeat(40))
}

/// Maps a branch name or a commit id to the revision directory name.
fn resolve_rev(dir: &Path, rev: &str) -> Option<String> {
    if safe_seg(&rev_name(rev)) && dir.join(rev_name(rev)).is_dir() {
        return Some(rev.to_string());
    }
    (rev == head_sha(dir, "main") && dir.join("main").is_dir()).then(|| "main".to_string())
}

fn lfs_map(dir: &Path, rev: &str) -> Map<String, Value> {
    fs::read_to_string(dir.join(format!("{}.lfs.json", rev_name(rev)))).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_lfs_map(dir: &Path, rev: &str, m: &Map<String, Value>) {
    let _ = fs::write(dir.join(format!("{}.lfs.json", rev_name(rev))), Value::Object(m.clone()).to_string());
}

fn walk(root: &Path, sub: &str, recursive: bool, out: &mut Vec<(String, bool, u64)>) {
    let Ok(rd) = fs::read_dir(root.join(sub)) else { return };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = if sub.is_empty() { name } else { format!("{sub}/{name}") };
        let md = e.metadata().ok();
        let is_dir = md.as_ref().is_some_and(|m| m.is_dir());
        out.push((path.clone(), is_dir, md.map(|m| m.len()).unwrap_or(0)));
        if is_dir && recursive {
            walk(root, &path, true, out);
        }
    }
}

fn copy_dir(a: &Path, b: &Path) -> std::io::Result<()> {
    fs::create_dir_all(b)?;
    for e in fs::read_dir(a)?.flatten() {
        let t = b.join(e.file_name());
        if e.metadata()?.is_dir() { copy_dir(&e.path(), &t)? } else { fs::copy(e.path(), t).map(|_| ())? }
    }
    Ok(())
}

fn user_of(cfg: &Cfg, h: &[(String, String)]) -> Option<String> {
    if cfg.tokens.is_empty() {
        return Some("local".into());
    }
    let t = hget(h, "authorization")?.strip_prefix("Bearer ")?.trim().as_bytes();
    // constant-time comparison: response timing tells nothing about a token's bytes
    let eq = |a: &[u8], b: &[u8]| a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0;
    cfg.tokens.iter().find(|(k, _)| eq(k.as_bytes(), t)).map(|(_, u)| u.clone())
}

fn can_write(cfg: &Cfg, ns: &str, user: &Option<String>) -> bool {
    cfg.tokens.is_empty() || user.as_deref() == Some(ns)
}

fn can_read(cfg: &Cfg, dir: &Path, ns: &str, user: &Option<String>) -> bool {
    !dir.join(".private").exists() || can_write(cfg, ns, user)
}

fn plural(t: &str) -> &'static str {
    match t {
        "dataset" | "datasets" => "datasets",
        "space" | "spaces" => "spaces",
        _ => "models",
    }
}

fn base_url(cfg: &Cfg, h: &[(String, String)]) -> String {
    cfg.public.clone().unwrap_or_else(|| format!("http://{}", hget(h, "host").unwrap_or("localhost")))
}

fn new_sha(prev: &str, msg: &str) -> String {
    sha256(format!("{prev}\n{msg}\n{}", now()).as_bytes())[..40].to_string()
}

fn repo_json(cfg: &Cfg, kind: &str, ns: &str, repo: &str, dir: &Path, rev: &str, with_files: bool) -> Value {
    let id = format!("{ns}/{repo}");
    let mut v = json!({
        "_id": sha256(id.as_bytes())[..24], "id": id, "author": ns, "sha": head_sha(dir, rev),
        "private": dir.join(".private").exists(), "disabled": false, "gated": false, "tags": [],
        "downloads": 0, "likes": 0, "createdAt": mtime(dir), "lastModified": mtime(&dir.join(rev_name(rev))),
    });
    if kind == "models" {
        v["modelId"] = json!(format!("{ns}/{repo}"));
    }
    if with_files {
        let mut ents = vec![];
        walk(&dir.join(rev_name(rev)), "", true, &mut ents);
        v["siblings"] = ents.into_iter().filter(|e| !e.1).map(|e| json!({ "rfilename": e.0 })).collect::<Vec<_>>().into();
    }
    let _ = cfg;
    v
}

// ---------- routing ----------

fn route(cfg: &Cfg, m: &Method, url: &str, h: &[(String, String)], body: Vec<u8>) -> R {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let q = parse_q(query);
    let segs: Vec<String> = path.split('/').filter(|s| !s.is_empty()).map(pdecode).collect();
    let s: Vec<&str> = segs.iter().map(String::as_str).collect();
    let user = user_of(cfg, h);
    let head = *m == Method::Head;
    let get = *m == Method::Get || head;

    // LFS batch and object transfer
    if *m == Method::Post {
        if let Some(i) = s.iter().position(|x| x.ends_with(".git")) {
            if s[i + 1..] == ["info", "lfs", "objects", "batch"] {
                return lfs_batch(cfg, h, &s[..i], &body, &user);
            }
        }
    }
    match (m, s.as_slice()) {
        (Method::Put, ["lfs-upload", _]) => return lfs_put_stream(cfg, url, h, &mut Cursor::new(body)),
        (Method::Post, ["lfs-verify"]) => return lfs_verify(cfg, &body),
        (Method::Get | Method::Head, ["lfs-download", oid]) => {
            let p = cfg.data.join("lfs").join(oid);
            return match fs::metadata(&p).ok().filter(|_| oid.len() == 64 && oid.bytes().all(|b| b.is_ascii_hexdigit())) {
                Some(m) => ranged_file(h, p, m.len(), &format!("\"{oid}\""), vec![]),
                None => err(404, "Entry not found"),
            };
        }
        (Method::Get, ["api", "whoami-v2"]) => {
            return match user {
                Some(u) => js(200, json!({ "type": "user", "id": sha256(u.as_bytes())[..24], "name": u, "fullname": u, "email": format!("{u}@localhost"),
                    "emailVerified": true, "canPay": false, "isPro": false, "orgs": [],
                    "auth": { "type": "access_token", "accessToken": { "displayName": "token", "role": "write" } } })),
                None => err(401, "Invalid credentials in Authorization header"),
            };
        }
        (Method::Post, ["api", "repos", "create"]) => return create_repo(cfg, &body, &user),
        (Method::Get, ["api", k @ ("models" | "datasets" | "spaces")]) => return search(cfg, k, &q, &user),
        _ => {}
    }
    if s.first() == Some(&"api") && s.len() >= 4 {
        let kind = s[1];
        if !matches!(kind, "models" | "datasets" | "spaces") {
            // other Hub APIs (resolve-cache redirects of the upstream, among others) go to the mirror when there is one
            return if get { mirror(cfg, url, head, h) } else { err(404, "Not found") };
        }
        // names may contain a slash-less legacy form (`gpt2`): treat as `{repo}` without namespace is unsupported
        let (ns, repo) = (s[2], s[3]);
        let Some(dir) = repo_dir(cfg, kind, ns, repo) else { return err(400, "Invalid repository id") };
        let rest = &s[4..];
        if dir.is_dir() {
            if !can_read(cfg, &dir, ns, &user) {
                return err(404, "Repository not found");
            }
            match (m, rest) {
                (Method::Get | Method::Head, []) => return js(200, repo_json(cfg, kind, ns, repo, &dir, "main", true)),
                (Method::Get | Method::Head, ["revision", rev @ ..]) => {
                    let rev = rev.join("/");
                    return match resolve_rev(&dir, &rev) {
                        Some(r) => js(200, repo_json(cfg, kind, ns, repo, &dir, &r, true)),
                        None => err(404, "Revision not found"),
                    };
                }
                (Method::Get, ["commits", rev]) => {
                    let Some(r) = resolve_rev(&dir, rev) else { return err(404, "Revision not found") };
                    let mut v: Vec<Value> = fs::read_to_string(dir.join(format!("{}.log", rev_name(&r)))).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
                    v.reverse();
                    return js(200, Value::Array(v));
                }
                (Method::Get | Method::Head, ["tree", rev, sub @ ..]) => return tree(&dir, rev, &sub.join("/"), qget(&q, "recursive").is_some_and(|v| v != "false" && v != "0")),
                (Method::Post, ["paths-info", rev]) => return paths_info(&dir, rev, &body),
                (Method::Get, ["refs"]) => {
                    let mut br = vec![];
                    if let Ok(rd) = fs::read_dir(&dir) {
                        for e in rd.flatten().filter(|e| e.path().is_dir()) {
                            let n = e.file_name().to_string_lossy().replace("%2F", "/");
                            br.push(json!({ "name": n, "ref": format!("refs/heads/{n}"), "targetCommit": head_sha(&dir, &n) }));
                        }
                    }
                    return js(200, json!({ "branches": br, "converts": [], "tags": [], "pullRequests": [] }));
                }
                (Method::Post, ["preupload", rev]) => return if can_write(cfg, ns, &user) { preupload(cfg, &dir, rev, &body) } else { err(403, "Forbidden") },
                (Method::Post, ["commit", rev]) => return if can_write(cfg, ns, &user) { commit(cfg, h, &dir, rev, &body) } else { err(403, "Forbidden") },
                (Method::Delete, []) => return err(405, "Use DELETE /api/repos/delete"),
                _ => {}
            }
        } else if get {
            return mirror(cfg, url, head, h);
        }
        return err(404, "Repository not found");
    }
    if *m == Method::Delete && s == ["api", "repos", "delete"] {
        return delete_repo(cfg, &body, &user);
    }
    if s.first() == Some(&"api") {
        return if get { mirror(cfg, url, head, h) } else { err(404, "Not found") };
    }

    // resolve: /{ns}/{repo}/resolve/{rev}/{path}, datasets/ and spaces/ prefixed
    let (kind, rest) = match s.first() {
        Some(&"datasets") => ("datasets", &s[1..]),
        Some(&"spaces") => ("spaces", &s[1..]),
        _ => ("models", &s[..]),
    };
    if get && rest.len() >= 4 && rest[2] == "resolve" {
        let (ns, repo, rev, file) = (rest[0], rest[1], rest[3], rest[4..].join("/"));
        let Some(dir) = repo_dir(cfg, kind, ns, repo) else { return err(400, "Invalid repository id") };
        if dir.is_dir() {
            if !can_read(cfg, &dir, ns, &user) || !safe_path(&file) {
                return err(404, "Repository not found");
            }
            let Some(rv) = resolve_rev(&dir, rev) else { return err(404, "Revision not found") };
            let p = dir.join(rev_name(&rv)).join(&file);
            let Ok(meta) = fs::metadata(&p) else { return err(404, "Entry not found") };
            let mut extra = vec![("X-Repo-Commit".to_string(), head_sha(&dir, &rv))];
            // large (LFS) files stream from disk; regular files are under the LFS threshold and hashed whole
            if let Some(l) = lfs_map(&dir, &rv).get(&file) {
                let oid = l["oid"].as_str().unwrap_or("").to_string();
                extra.push(("X-Linked-Etag".into(), format!("\"{oid}\"")));
                extra.push(("X-Linked-Size".into(), meta.len().to_string()));
                return ranged_file(h, p, meta.len(), &format!("\"{oid}\""), extra);
            }
            let Ok(bytes) = fs::read(&p) else { return err(404, "Entry not found") };
            let etag = format!("\"{}\"", &sha256(&bytes)[..40]);
            return ranged(h, bytes, &etag, extra);
        }
        return if get { mirror(cfg, url, head, h) } else { err(404, "Not found") };
    }
    if get && hget(h, "accept").is_some_and(|a| a.contains("text/html")) {
        if let Some(r) = ui::page(cfg, &s, &q, &user) {
            return r;
        }
    }
    if get && s.is_empty() {
        return js(200, json!({ "service": "hub", "mode": if cfg.upstream.is_some() { "mirror+hosted" } else { "hosted" } }));
    }
    err(404, "Not found")
}

/// `ranged` for a file on disk: the answer streams the selected bytes instead of holding them.
fn ranged_file(h: &[(String, String)], p: PathBuf, total: u64, etag: &str, mut extra: Vec<(String, String)>) -> R {
    extra.push(("ETag".into(), etag.into()));
    extra.push(("Accept-Ranges".into(), "bytes".into()));
    extra.push(("Content-Type".into(), "application/octet-stream".into()));
    extra.push(("Cache-Control".into(), "public, max-age=0".into()));
    if let Some(r) = hget(h, "range").and_then(|r| parse_range(r, total as usize)) {
        extra.push(("Content-Range".into(), format!("bytes {}-{}/{}", r.0, r.1, total)));
        return R { status: 206, headers: extra, body: vec![], len: None, file: Some((p, r.0 as u64, (r.1 - r.0 + 1) as u64)) };
    }
    if hget(h, "range").is_some() && total > 0 {
        extra.push(("Content-Range".into(), format!("bytes */{total}")));
        return R { status: 416, headers: extra, body: vec![], len: None, file: None };
    }
    R { status: 200, headers: extra, body: vec![], len: None, file: Some((p, 0, total)) }
}

/// 200 or 206 answer with the client-facing headers; HEAD reuses it (the server drops the body).
fn ranged(h: &[(String, String)], bytes: Vec<u8>, etag: &str, mut extra: Vec<(String, String)>) -> R {
    let total = bytes.len();
    extra.push(("ETag".into(), etag.into()));
    extra.push(("Accept-Ranges".into(), "bytes".into()));
    extra.push(("Content-Type".into(), "application/octet-stream".into()));
    extra.push(("Cache-Control".into(), "public, max-age=0".into()));
    if let Some(r) = hget(h, "range").and_then(|r| parse_range(r, total)) {
        extra.push(("Content-Range".into(), format!("bytes {}-{}/{}", r.0, r.1, total)));
        return R { status: 206, headers: extra, body: bytes[r.0..=r.1].to_vec(), len: None, file: None };
    }
    if hget(h, "range").is_some() && total > 0 {
        extra.push(("Content-Range".into(), format!("bytes */{total}")));
        return R { status: 416, headers: extra, body: vec![], len: None, file: None };
    }
    R { status: 200, headers: extra, body: bytes, len: None, file: None }
}

fn parse_range(r: &str, total: usize) -> Option<(usize, usize)> {
    let (a, b) = r.strip_prefix("bytes=")?.split(',').next()?.split_once('-')?;
    if total == 0 {
        return None;
    }
    let (s, e) = match (a.trim(), b.trim()) {
        ("", n) => (total.saturating_sub(n.parse().ok()?), total - 1),
        (a, "") => (a.parse().ok()?, total - 1),
        (a, b) => (a.parse().ok()?, b.parse::<usize>().ok()?.min(total - 1)),
    };
    (s <= e && s < total).then_some((s, e))
}

fn tree(dir: &Path, rev: &str, sub: &str, recursive: bool) -> R {
    let Some(rv) = resolve_rev(dir, rev) else { return err(404, "Revision not found") };
    if !sub.is_empty() && !safe_path(sub) {
        return err(404, "Not found");
    }
    let lfs = lfs_map(dir, &rv);
    let mut e = vec![];
    walk(&dir.join(rev_name(&rv)), sub, recursive, &mut e);
    e.sort();
    let out: Vec<Value> = e.into_iter().map(|(p, d, size)| {
        let oid = sha256(p.as_bytes())[..40].to_string();
        if d {
            json!({ "type": "directory", "oid": oid, "path": p })
        } else if let Some(l) = lfs.get(&p) {
            json!({ "type": "file", "oid": oid, "size": size, "path": p, "lfs": { "oid": l["oid"], "size": size, "pointerSize": 134 } })
        } else {
            json!({ "type": "file", "oid": oid, "size": size, "path": p })
        }
    }).collect();
    js(200, Value::Array(out))
}

fn paths_info(dir: &Path, rev: &str, body: &[u8]) -> R {
    let Some(rv) = resolve_rev(dir, rev) else { return err(404, "Revision not found") };
    let text = String::from_utf8_lossy(body);
    let paths: Vec<String> = match serde_json::from_str::<Value>(&text) {
        Ok(v) => v["paths"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
        Err(_) => parse_q(&text).into_iter().filter(|(k, _)| k == "paths").map(|(_, v)| v).collect(),
    };
    let lfs = lfs_map(dir, &rv);
    let out: Vec<Value> = paths.into_iter().filter(|p| safe_path(p)).filter_map(|p| {
        let md = fs::metadata(dir.join(rev_name(&rv)).join(&p)).ok()?;
        let oid = sha256(p.as_bytes())[..40].to_string();
        Some(if md.is_dir() { json!({ "type": "directory", "oid": oid, "path": p }) }
            else if let Some(l) = lfs.get(&p) { json!({ "type": "file", "oid": oid, "size": md.len(), "path": p, "lfs": { "oid": l["oid"], "size": md.len(), "pointerSize": 134 } }) }
            else { json!({ "type": "file", "oid": oid, "size": md.len(), "path": p }) })
    }).collect();
    js(200, Value::Array(out))
}

fn search(cfg: &Cfg, kind: &str, q: &[(String, String)], user: &Option<String>) -> R {
    let (needle, author) = (qget(q, "search").map(str::to_lowercase), qget(q, "author"));
    let limit: usize = qget(q, "limit").and_then(|v| v.parse().ok()).unwrap_or(50).min(1000);
    let sort = qget(q, "sort").unwrap_or("lastModified");
    let mut rows: Vec<(String, Value)> = vec![];
    let root = cfg.data.join("repos").join(kind);
    for ns in fs::read_dir(&root).into_iter().flatten().flatten() {
        let nsn = ns.file_name().to_string_lossy().into_owned();
        if author.is_some_and(|a| a != nsn) {
            continue;
        }
        for r in fs::read_dir(ns.path()).into_iter().flatten().flatten() {
            let rn = r.file_name().to_string_lossy().into_owned();
            let id = format!("{nsn}/{rn}");
            if needle.as_ref().is_some_and(|n| !id.to_lowercase().contains(n)) || !can_read(cfg, &r.path(), &nsn, user) {
                continue;
            }
            let v = repo_json(cfg, kind, &nsn, &rn, &r.path(), "main", false);
            let key = match sort {
                "createdAt" | "lastModified" => v[sort].as_str().unwrap_or("").to_string(),
                "id" => id.clone(),
                _ => id.clone(),
            };
            rows.push((key, v));
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    if matches!(sort, "createdAt" | "lastModified") || qget(q, "direction") == Some("-1") {
        rows.reverse();
    }
    let list: Vec<Value> = rows.into_iter().take(limit).map(|r| r.1).collect();
    if list.is_empty() && cfg.upstream.is_some() {
        return mirror(cfg, &format!("/api/{kind}?{}", q.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")), false, &[]);
    }
    js(200, Value::Array(list))
}

fn create_repo(cfg: &Cfg, body: &[u8], user: &Option<String>) -> R {
    let Ok(v) = serde_json::from_slice::<Value>(body) else { return err(400, "Invalid JSON") };
    let Some(name) = v["name"].as_str() else { return err(400, "name is required") };
    let ns = v["organization"].as_str().map(String::from).or_else(|| user.clone());
    let Some(ns) = ns else { return err(401, "Invalid credentials in Authorization header") };
    if !can_write(cfg, &ns, user) {
        return err(403, "Forbidden");
    }
    let kind = plural(v["type"].as_str().unwrap_or("model"));
    let Some(dir) = repo_dir(cfg, kind, &ns, name) else { return err(400, "Invalid repository id") };
    if dir.exists() {
        return js(409, json!({ "error": "You already created this repo", "url": format!("/{}{ns}/{name}", prefix(kind)) }));
    }
    if fs::create_dir_all(dir.join("main")).is_err() || fs::write(dir.join("main.sha"), new_sha("", &format!("{ns}/{name}"))).is_err() {
        return err(500, "storage");
    }
    if v["private"].as_bool() == Some(true) {
        let _ = fs::write(dir.join(".private"), "");
    }
    let base = cfg.public.clone().unwrap_or_default();
    js(200, json!({ "url": format!("{base}/{}{ns}/{name}", prefix(kind)), "name": format!("{ns}/{name}"), "id": format!("{ns}/{name}") }))
}

fn prefix(kind: &str) -> &'static str {
    match kind {
        "datasets" => "datasets/",
        "spaces" => "spaces/",
        _ => "",
    }
}

fn delete_repo(cfg: &Cfg, body: &[u8], user: &Option<String>) -> R {
    let Ok(v) = serde_json::from_slice::<Value>(body) else { return err(400, "Invalid JSON") };
    let Some(name) = v["name"].as_str() else { return err(400, "name is required") };
    let Some(ns) = v["organization"].as_str().map(String::from).or_else(|| user.clone()) else { return err(401, "Unauthorized") };
    let Some(dir) = repo_dir(cfg, plural(v["type"].as_str().unwrap_or("model")), &ns, name) else { return err(400, "Invalid repository id") };
    if !can_write(cfg, &ns, user) {
        return err(403, "Forbidden");
    }
    if !dir.is_dir() {
        return err(404, "Repository not found");
    }
    let _ = fs::remove_dir_all(dir);
    js(200, json!({}))
}

fn preupload(cfg: &Cfg, dir: &Path, rev: &str, body: &[u8]) -> R {
    let Ok(v) = serde_json::from_slice::<Value>(body) else { return err(400, "Invalid JSON") };
    let files: Vec<Value> = v["files"].as_array().cloned().unwrap_or_default().iter().map(|f| {
        let size = f["size"].as_u64().unwrap_or(0);
        json!({ "path": f["path"], "uploadMode": if size > cfg.lfs_threshold { "lfs" } else { "regular" }, "shouldIgnore": false })
    }).collect();
    let sha = resolve_rev(dir, rev).map(|r| head_sha(dir, &r)).unwrap_or_else(|| head_sha(dir, "main"));
    js(200, json!({ "files": files, "commitOid": sha }))
}

fn commit(cfg: &Cfg, h: &[(String, String)], dir: &Path, rev: &str, body: &[u8]) -> R {
    let base = base_url(cfg, h);
    if !valid_rev(rev) {
        return err(400, "Invalid revision");
    }
    let rvdir = dir.join(rev_name(rev));
    if !rvdir.is_dir() && copy_dir(&dir.join("main"), &rvdir).is_err() {
        return err(500, "storage");
    }
    let mut map = lfs_map(dir, rev);
    let mut summary = String::new();
    for line in String::from_utf8_lossy(body).lines().filter(|l| !l.trim().is_empty()) {
        let Ok(op) = serde_json::from_str::<Value>(line) else { return err(400, "Invalid NDJSON line") };
        let val = &op["value"];
        let path = val["path"].as_str().unwrap_or("");
        match op["key"].as_str().unwrap_or("") {
            "header" => summary = val["summary"].as_str().unwrap_or("").to_string(),
            "file" => {
                if !safe_path(path) {
                    return err(400, "Invalid path");
                }
                let raw = match val["encoding"].as_str() {
                    Some("base64") => STANDARD.decode(val["content"].as_str().unwrap_or("")).ok(),
                    _ => Some(val["content"].as_str().unwrap_or("").as_bytes().to_vec()),
                };
                let Some(raw) = raw else { return err(400, "Invalid base64 content") };
                let t = rvdir.join(path);
                if fs::create_dir_all(t.parent().unwrap_or(&rvdir)).and_then(|_| fs::write(&t, raw)).is_err() {
                    return err(500, "storage");
                }
                map.remove(path);
            }
            "lfsFile" => {
                let oid = val["oid"].as_str().unwrap_or("");
                if !safe_path(path) || oid.len() != 64 || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return err(400, "Invalid lfsFile");
                }
                let obj = cfg.data.join("lfs").join(oid);
                if !obj.is_file() {
                    return err(400, &format!("LFS object {oid} was not uploaded"));
                }
                let t = rvdir.join(path);
                if fs::create_dir_all(t.parent().unwrap_or(&rvdir)).and_then(|_| fs::copy(&obj, &t).map(|_| ())).is_err() {
                    return err(500, "storage");
                }
                map.insert(path.into(), json!({ "oid": oid, "size": val["size"] }));
            }
            "deletedFile" => {
                if !safe_path(path) {
                    return err(400, "Invalid path");
                }
                let _ = fs::remove_file(rvdir.join(path));
                map.remove(path);
            }
            "deletedFolder" => {
                if !safe_path(path) {
                    return err(400, "Invalid path");
                }
                let _ = fs::remove_dir_all(rvdir.join(path));
                let pre = format!("{path}/");
                map.retain(|k, _| !k.starts_with(&pre));
            }
            _ => {}
        }
    }
    save_lfs_map(dir, rev, &map);
    let sha = new_sha(&head_sha(dir, rev), &summary);
    if fs::write(dir.join(format!("{}.sha", rev_name(rev))), &sha).is_err() {
        return err(500, "storage");
    }
    let title = summary.lines().next().unwrap_or("").to_string();
    let entry = json!({ "id": sha, "title": title, "message": "", "authors": [], "date": mtime(dir) });
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(dir.join(format!("{}.log", rev_name(rev)))) {
        use std::io::Write;
        let _ = writeln!(f, "{entry}");
    }
    let id = dir.strip_prefix(&cfg.data).ok().map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
    let (kind, rest) = id.strip_prefix("repos/").and_then(|r| r.split_once('/')).unwrap_or(("models", ""));
    let url = format!("{}/{}{rest}/commit/{sha}", base, if kind == "models" { String::new() } else { format!("{kind}/") });
    js(200, json!({ "success": true, "commitOid": sha, "commitUrl": url, "hookOutput": "" }))
}

fn lfs_batch(cfg: &Cfg, h: &[(String, String)], _repo: &[&str], body: &[u8], user: &Option<String>) -> R {
    let Ok(v) = serde_json::from_slice::<Value>(body) else { return err(400, "Invalid JSON") };
    let up = v["operation"].as_str() == Some("upload");
    if up && user.is_none() {
        return err(401, "Unauthorized");
    }
    // a client may PUT without its token: the link is signed for this object and user, valid for an hour
    let exp = (now() / 1_000_000_000) as u64 + 3600;
    let who = user.clone().unwrap_or_default();
    let base = base_url(cfg, h);
    let objects: Vec<Value> = v["objects"].as_array().cloned().unwrap_or_default().iter().map(|o| {
        let oid = o["oid"].as_str().unwrap_or("");
        let valid = oid.len() == 64 && oid.bytes().all(|b| b.is_ascii_hexdigit());
        let have = valid && cfg.data.join("lfs").join(oid).is_file();
        if !valid {
            json!({ "oid": oid, "size": o["size"], "error": { "code": 422, "message": "invalid oid" } })
        } else if up && have {
            json!({ "oid": oid, "size": o["size"] })
        } else if up {
            json!({ "oid": oid, "size": o["size"], "authenticated": true, "actions": {
                "upload": { "href": format!("{base}/lfs-upload/{oid}?u={}&exp={exp}&sig={}", pencode(&who), signature(cfg, &oid.to_ascii_lowercase(), exp, &who)), "header": {} },
                "verify": { "href": format!("{base}/lfs-verify"), "header": {} } } })
        } else if have {
            json!({ "oid": oid, "size": o["size"], "authenticated": true, "actions": { "download": { "href": format!("{base}/lfs-download/{oid}"), "header": {} } } })
        } else {
            json!({ "oid": oid, "size": o["size"], "error": { "code": 404, "message": "Object does not exist" } })
        }
    }).collect();
    let mut r = js(200, json!({ "transfer": "basic", "objects": objects, "hash_algo": "sha256" }));
    r.headers = vec![("Content-Type".into(), "application/vnd.git-lfs+json".into())];
    r
}


fn lfs_verify(cfg: &Cfg, body: &[u8]) -> R {
    let v: Value = serde_json::from_slice(body).unwrap_or_default();
    let oid = v["oid"].as_str().unwrap_or("");
    let ok = oid.len() == 64 && oid.bytes().all(|b| b.is_ascii_hexdigit()) && fs::metadata(cfg.data.join("lfs").join(oid)).is_ok_and(|m| v["size"].as_u64().is_none_or(|s| s == m.len()));
    if ok { js(200, json!({})) } else { err(404, "Object does not exist") }
}

/// Mirror mode: answers come from `HUB_UPSTREAM`, kept in `cache/{key}` (+ `.meta`), served from it afterwards.
fn mirror(cfg: &Cfg, url: &str, head: bool, h: &[(String, String)]) -> R {
    let Some(up) = cfg.upstream.as_deref() else { return err(404, "Repository not found") };
    let key = format!("{:.16}", sha256(url.as_bytes()));
    let (body_p, meta_p) = (cfg.data.join("cache").join(&key), cfg.data.join("cache").join(format!("{key}.meta")));
    const KEEP: [&str; 7] = ["content-type", "etag", "x-repo-commit", "x-linked-etag", "x-linked-size", "content-length", "location"];
    if let Ok(meta) = fs::read_to_string(&meta_p).and_then(|m| serde_json::from_str::<Value>(&m).map_err(std::io::Error::other)) {
        let headers: Vec<(String, String)> = meta["headers"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").into())).collect()).unwrap_or_default();
        let status = meta["status"].as_u64().unwrap_or(200) as u16;
        // a cached body answers GET and HEAD alike (the server sends no body to HEAD, the length stays exact)
        if let Ok(m) = fs::metadata(&body_p) {
            return ranged_file(h, body_p, m.len(), meta["etag"].as_str().unwrap_or("\"mirror\""), headers.into_iter().filter(|(k, _)| !matches!(k.as_str(), "content-length" | "etag" | "content-type")).collect());
        }
        if head && status != 200 {
            let len = meta["len"].as_u64().map(|l| l as usize);
            return R { status, headers: headers.into_iter().filter(|(k, _)| k != "content-length").collect(), body: vec![], len, file: None };
        }
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).max_redirects(if head { 0 } else { 5 }).build().into();
    let target = format!("{up}{url}");
    let auth = cfg.upstream_token.as_ref().map(|t| format!("Bearer {t}"));
    let resp = if head {
        let mut r = agent.head(&target);
        if let Some(a) = &auth { r = r.header("Authorization", a); }
        r.call()
    } else {
        let mut r = agent.get(&target);
        if let Some(a) = &auth { r = r.header("Authorization", a); }
        r.call()
    };
    let Ok(mut resp) = resp else { return err(502, "upstream unreachable") };
    let status = resp.status().as_u16();
    if head && status == 200 {
        // the upstream's HEAD may omit the length: the body is fetched and cached, and its exact length answers
        return mirror(cfg, url, false, h);
    }
    let headers: Vec<(String, String)> = KEEP.iter().filter_map(|k| resp.headers().get(*k).and_then(|v| v.to_str().ok()).map(|v| (k.to_string(), v.to_string()))).collect();
    let etag = headers.iter().find(|(k, _)| k == "etag").map(|(_, v)| v.clone()).unwrap_or_else(|| "\"mirror\"".into());
    let len = headers.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse::<usize>().ok());
    if head {
        if status < 400 {
            let _ = fs::create_dir_all(cfg.data.join("cache")).and_then(|_| fs::write(&meta_p, json!({ "status": status, "headers": hmap(&headers), "len": len, "etag": etag }).to_string()));
        }
        return R { status, headers: headers.into_iter().filter(|(k, _)| k != "content-length").collect(), body: vec![], len, file: None };
    }
    let passthrough: Vec<(String, String)> = headers.iter().filter(|(k, _)| k != "content-length").cloned().collect();
    if status != 200 {
        let Ok(body) = resp.body_mut().with_config().limit(1 << 20).read_to_vec() else { return err(502, "upstream body") };
        return R { status, headers: passthrough, body, len: None, file: None };
    }
    let tmp = cfg.data.join("cache").join(format!("{key}.part-{}", now()));
    let stored = fs::create_dir_all(cfg.data.join("cache"))
        .and_then(|_| fs::File::create(&tmp))
        .and_then(|mut out| std::io::copy(&mut resp.body_mut().with_config().limit(cfg.max_object).reader(), &mut out))
        .and_then(|n| fs::rename(&tmp, &body_p).map(|_| n))
        .and_then(|n| fs::write(&meta_p, json!({ "status": 200, "headers": hmap(&headers), "len": n, "etag": etag }).to_string()).map(|_| n));
    match stored {
        Ok(n) => ranged_file(h, body_p, n, &etag, passthrough.into_iter().filter(|(k, _)| !matches!(k.as_str(), "etag" | "content-type")).collect()),
        Err(_) => {
            let _ = fs::remove_file(&tmp);
            err(502, "upstream body")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(tokens: &str) -> Cfg {
        let d = env::temp_dir().join(format!("hub-test-{}", now()));
        Cfg {
            data: d,
            upstream: None,
            upstream_token: None,
            tokens: tokens.split(';').filter_map(|p| p.split_once('=')).map(|(a, b)| (a.into(), b.into())).collect(),
            public: Some("http://hub".into()),
            lfs_threshold: 8,
            max_body: 1 << 20,
            max_object: 1 << 20,
            secret: b"test".to_vec(),
        }
    }

    fn call(c: &Cfg, m: Method, url: &str, hd: &[(&str, &str)], body: &str) -> (u16, Value, R) {
        let h: Vec<(String, String)> = hd.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let mut r = route(c, &m, url, &h, body.as_bytes().to_vec());
        if let Some((p, off, n)) = r.file.take() {
            r.body = fs::read(p).unwrap()[off as usize..(off + n) as usize].to_vec();
        }
        (r.status, serde_json::from_slice(&r.body).unwrap_or(Value::Null), r)
    }

    fn hv<'a>(r: &'a R, k: &str) -> Option<&'a str> {
        r.headers.iter().find(|(a, _)| a.eq_ignore_ascii_case(k)).map(|(_, v)| v.as_str())
    }

    #[test]
    fn range_forms() {
        assert_eq!(parse_range("bytes=0-3", 10), Some((0, 3)));
        assert_eq!(parse_range("bytes=4-", 10), Some((4, 9)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=20-30", 10), None);
    }

    #[test]
    fn dates_and_paths() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00.000Z");
        assert!(safe_path("a/b.txt") && !safe_path("../x") && !safe_path("/x") && !safe_path("a//b"));
    }

    #[test]
    fn write_then_read_regular_and_lfs() {
        let c = cfg("");
        let (s, v, _) = call(&c, Method::Post, "/api/repos/create", &[], r#"{"name":"m","organization":"acme","type":"model"}"#);
        assert_eq!(s, 200, "{v}");
        assert_eq!(call(&c, Method::Post, "/api/repos/create", &[], r#"{"name":"m","organization":"acme"}"#).0, 409);
        let (_, pre, _) = call(&c, Method::Post, "/api/models/acme/m/preupload/main", &[], r#"{"files":[{"path":"a.txt","size":3,"sample":""},{"path":"big.bin","size":100,"sample":""}]}"#);
        assert_eq!(pre["files"][0]["uploadMode"], "regular");
        assert_eq!(pre["files"][1]["uploadMode"], "lfs");
        let big = "x".repeat(100);
        let oid = sha256(big.as_bytes());
        let (_, b, _) = call(&c, Method::Post, "/acme/m.git/info/lfs/objects/batch", &[("host", "hub")], &format!(r#"{{"operation":"upload","objects":[{{"oid":"{oid}","size":100}}]}}"#));
        assert!(b["objects"][0]["actions"]["upload"]["href"].as_str().unwrap().starts_with(&format!("http://hub/lfs-upload/{oid}?u=")));
        assert_eq!(call(&c, Method::Put, &format!("/lfs-upload/{oid}"), &[], &big).0, 200);
        assert_eq!(call(&c, Method::Put, &format!("/lfs-upload/{}", sha256(b"other")), &[], &big).0, 400);
        assert_eq!(call(&c, Method::Post, "/lfs-verify", &[], &format!(r#"{{"oid":"{oid}","size":100}}"#)).0, 200);
        let nd = format!(
            "{}\n{}\n{}\n",
            r#"{"key":"header","value":{"summary":"add files"}}"#,
            r#"{"key":"file","value":{"path":"dir/a.txt","encoding":"base64","content":"YWJj"}}"#,
            format!(r#"{{"key":"lfsFile","value":{{"path":"big.bin","algo":"sha256","oid":"{oid}","size":100}}}}"#)
        );
        let (s, cm, _) = call(&c, Method::Post, "/api/models/acme/m/commit/main", &[], &nd);
        assert_eq!(s, 200, "{cm}");
        let sha = cm["commitOid"].as_str().unwrap().to_string();
        let (_, _, r) = call(&c, Method::Get, "/acme/m/resolve/main/dir/a.txt", &[], "");
        assert_eq!((r.status, r.body.as_slice()), (200, &b"abc"[..]));
        assert_eq!(hv(&r, "x-repo-commit"), Some(sha.as_str()));
        assert!(hv(&r, "etag").is_some() && hv(&r, "x-linked-etag").is_none());
        let (_, _, r) = call(&c, Method::Get, "/acme/m/resolve/main/big.bin", &[("range", "bytes=0-9")], "");
        assert_eq!((r.status, r.body.len()), (206, 10));
        assert_eq!(hv(&r, "content-range"), Some("bytes 0-9/100"));
        let (_, _, r) = call(&c, Method::Head, &format!("/acme/m/resolve/{sha}/big.bin"), &[], "");
        assert_eq!(hv(&r, "x-linked-etag"), Some(format!("\"{oid}\"").as_str()));
        assert_eq!(hv(&r, "x-linked-size"), Some("100"));
        let (_, t, _) = call(&c, Method::Get, "/api/models/acme/m/tree/main?recursive=true", &[], "");
        assert_eq!(t.as_array().unwrap().len(), 3);
        assert!(t.as_array().unwrap().iter().any(|e| e["path"] == "big.bin" && e["lfs"]["oid"] == oid));
        let (_, info, _) = call(&c, Method::Get, "/api/models/acme/m", &[], "");
        assert_eq!(info["sha"], sha);
        assert_eq!(call(&c, Method::Post, "/api/models/acme/m/commit/main", &[], r#"{"key":"lfsFile","value":{"path":"z","oid":"00","size":1}}"#).0, 400);
        let (_, _, _) = call(&c, Method::Post, "/api/models/acme/m/commit/main", &[], r#"{"key":"deletedFolder","value":{"path":"dir"}}"#);
        assert_eq!(call(&c, Method::Get, "/acme/m/resolve/main/dir/a.txt", &[], "").0, 404);
        let _ = fs::remove_dir_all(&c.data);
    }

    #[test]
    fn html_pages_for_browsers_json_for_clients() {
        let c = cfg("");
        let ac = [("accept", "text/html")];
        call(&c, Method::Post, "/api/repos/create", &[], r#"{"name":"<b>x","organization":"acme"}"#);
        call(&c, Method::Post, "/api/repos/create", &[], r#"{"name":"m","organization":"acme"}"#);
        let (s, _, r) = call(&c, Method::Get, "/models?search=m", &ac, "");
        assert_eq!(s, 200);
        let t = String::from_utf8_lossy(&r.body).into_owned();
        assert!(t.contains("href=\"/acme/m\"") && !t.contains("<b>x"));
        let (s, _, r) = call(&c, Method::Get, "/acme/m", &ac, "");
        assert!(s == 200 && String::from_utf8_lossy(&r.body).contains("no files yet"));
        assert_eq!(call(&c, Method::Get, "/acme/none", &ac, "").0, 404);
        assert_eq!(call(&c, Method::Get, "/", &[], "").1["service"], "hub");
        let _ = fs::remove_dir_all(&c.data);
    }

    #[test]
    fn search_filters_and_private_access() {
        let c = cfg("t1=alice;t2=bob");
        let a = [("authorization", "Bearer t1")];
        let b = [("authorization", "Bearer t2")];
        assert_eq!(call(&c, Method::Get, "/api/whoami-v2", &[], "").0, 401);
        assert_eq!(call(&c, Method::Get, "/api/whoami-v2", &a, "").1["name"], "alice");
        assert_eq!(call(&c, Method::Post, "/api/repos/create", &a, r#"{"name":"secret","private":true}"#).0, 200);
        assert_eq!(call(&c, Method::Post, "/api/repos/create", &a, r#"{"name":"bert-base","type":"dataset"}"#).0, 200);
        assert_eq!(call(&c, Method::Post, "/api/repos/create", &b, r#"{"name":"x","organization":"alice"}"#).0, 403);
        assert_eq!(call(&c, Method::Get, "/api/models/alice/secret", &b, "").0, 404);
        assert_eq!(call(&c, Method::Get, "/api/models/alice/secret", &a, "").0, 200);
        assert_eq!(call(&c, Method::Post, "/api/models/alice/secret/commit/main", &b, "").0, 404);
        let (_, l, _) = call(&c, Method::Get, "/api/models?search=SEC", &b, "");
        assert_eq!(l.as_array().unwrap().len(), 0);
        let (_, l, _) = call(&c, Method::Get, "/api/models?search=sec&author=alice&limit=1", &a, "");
        assert_eq!(l[0]["id"], "alice/secret");
        let (_, l, _) = call(&c, Method::Get, "/api/datasets?search=bert", &b, "");
        assert_eq!(l[0]["id"], "alice/bert-base");
        let _ = fs::remove_dir_all(&c.data);
    }

    #[test]
    fn signed_upload_links_replace_tokens_in_urls() {
        let c = cfg("tok=alice");
        let big = "x".repeat(32);
        let oid = sha256(big.as_bytes());
        let exp = (now() / 1_000_000_000) as u64 + 60;
        let sig = signature(&c, &oid, exp, "alice");
        let ok = format!("/lfs-upload/{oid}?u=alice&exp={exp}&sig={sig}");
        assert_eq!(call(&c, Method::Put, &ok, &[], &big).0, 200);
        assert_eq!(call(&c, Method::Put, &format!("/lfs-upload/{oid}?u=mallory&exp={exp}&sig={sig}"), &[], &big).0, 401);
        let old = exp - 120;
        assert_eq!(call(&c, Method::Put, &format!("/lfs-upload/{oid}?u=alice&exp={old}&sig={}", signature(&c, &oid, old, "alice")), &[], &big).0, 401);
        assert_eq!(call(&c, Method::Put, &format!("/lfs-upload/{oid}"), &[], &big).0, 401);
        // the stored object streams back with ranges
        let (st, _, r) = call(&c, Method::Get, &format!("/lfs-download/{oid}"), &[("range", "bytes=2-5")], "");
        assert_eq!((st, r.body.len()), (206, 4));
    }

    #[test]
    fn segments_and_revisions_stay_inside_the_repository() {
        for bad in ["C:", "C:evil", "con", "NUL.txt", "com1.json", "..", "...", "a/b", "a\\b"] {
            assert!(!safe_seg(bad), "{bad}");
        }
        for good in ["config.json", "model-00001.safetensors", ".gitattributes"] {
            assert!(safe_seg(good), "{good}");
        }
        for bad in ["main.sha", "x.lfs.json", "a.log", ".hidden", "", "a b", "a//b"] {
            assert!(!valid_rev(bad), "{bad}");
        }
        for good in ["main", "v1.2.0", "refs-pr-1", "feature/x"] {
            assert!(valid_rev(good), "{good}");
        }
    }

    #[test]
    fn hmac_matches_rfc_4231_case_2() {
        assert_eq!(hmac(b"Jefe", b"what do ya want for nothing?"), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    }
}

fn hmap(h: &[(String, String)]) -> Value {
    Value::Object(h.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect())
}
