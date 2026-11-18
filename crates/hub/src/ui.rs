//! Server-rendered web UI: home, model/dataset/space lists with search, repository page with file list.
//! Served only to clients that accept `text/html`; API clients keep getting JSON. Colours come from CSS custom
//! properties (light and dark), no raw values in rules.
use super::*;

const CSS: &str = ":root{--bg:#fbfbfd;--fg:#1b1c20;--mut:#5d6068;--card:#fff;--line:#dcdde3;--acc:#5b4ee0;--on:#fff;--r:12px;--s:16px}\
@media(prefers-color-scheme:dark){:root{--bg:#121316;--fg:#ececf0;--mut:#9fa2ab;--card:#1b1c21;--line:#2d2f36;--acc:#9d93ff;--on:#15131f}}\
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:16px/1.5 system-ui,sans-serif}\
a{color:var(--acc)}a:focus-visible,input:focus-visible,button:focus-visible{outline:3px solid var(--acc);outline-offset:2px}\
header{display:flex;flex-wrap:wrap;gap:var(--s);align-items:center;padding:var(--s) calc(var(--s)*2);border-bottom:1px solid var(--line);background:var(--card)}\
header nav{display:flex;gap:var(--s);flex:1}header a{text-decoration:none;color:var(--fg);font-weight:600}\
main{max-width:960px;margin:0 auto;padding:calc(var(--s)*2)}h1{overflow-wrap:anywhere}\
form.s{display:flex;gap:8px}input[type=search]{flex:1;padding:10px 14px;border:1px solid var(--line);border-radius:var(--r);background:var(--card);color:var(--fg);font:inherit}\
button{padding:10px 18px;border:0;border-radius:var(--r);background:var(--acc);color:var(--on);font:inherit;font-weight:600;cursor:pointer}\
ul.l{list-style:none;margin:var(--s) 0;padding:0;display:grid;gap:8px}ul.l li{background:var(--card);border:1px solid var(--line);border-radius:var(--r);padding:12px var(--s);display:flex;justify-content:space-between;gap:var(--s);flex-wrap:wrap}\
ul.l a{overflow-wrap:anywhere;font-weight:600}.m{color:var(--mut);font-size:.9em}.e{padding:calc(var(--s)*2);text-align:center;color:var(--mut);border:1px dashed var(--line);border-radius:var(--r)}\
.b{display:inline-block;padding:2px 10px;border:1px solid var(--line);border-radius:99px;font-size:.8em;color:var(--mut)}\
@media(max-width:480px){main{padding:var(--s)}header{padding:var(--s)}}";

pub fn esc(s: &str) -> String {
    s.chars().fold(String::new(), |mut o, c| {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
        o
    })
}

fn prefix_of(kind: &str) -> &'static str {
    match kind {
        "datasets" => "/datasets",
        "spaces" => "/spaces",
        _ => "",
    }
}

fn html(status: u16, title: &str, body: &str) -> R {
    let page = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>{t}</title><style>{CSS}</style></head><body><header><a href=\"/\">Hub</a><nav aria-label=\"Main\">\
<a href=\"/models\">Models</a><a href=\"/datasets\">Datasets</a><a href=\"/spaces\">Spaces</a></nav></header><main>{body}</main></body></html>",
        t = esc(title)
    );
    R { status, headers: vec![("Content-Type".into(), "text/html; charset=utf-8".into())], body: page.into_bytes(), len: None, file: None }
}

fn not_found() -> R {
    html(404, "Not found", "<h1>Not found</h1><p class=\"e\">This page or repository does not exist on this hub.</p>")
}

fn list(cfg: &Cfg, kind: &str, q: &[(String, String)], user: &Option<String>) -> R {
    let label = match kind {
        "datasets" => "Datasets",
        "spaces" => "Spaces",
        _ => "Models",
    };
    let r = search(cfg, kind, q, user);
    let rows: Vec<Value> = serde_json::from_slice::<Value>(&r.body).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let needle = qget(q, "search").unwrap_or("");
    let mut b = format!(
        "<h1>{label}</h1><form class=\"s\" role=\"search\" action=\"/{kind}\" method=\"get\"><input type=\"search\" name=\"search\" value=\"{}\" aria-label=\"Search {label}\" placeholder=\"Search {label}\"><button type=\"submit\">Search</button></form>",
        esc(needle)
    );
    if rows.is_empty() {
        b.push_str("<p class=\"e\">Nothing here yet. Create a repository through the API or mirror one from an upstream hub.</p>");
    } else {
        b.push_str("<ul class=\"l\">");
        for v in &rows {
            let id = v["id"].as_str().unwrap_or("");
            b.push_str(&format!(
                "<li><a href=\"{}/{}\">{}</a><span class=\"m\">{}{}</span></li>",
                prefix_of(kind),
                esc(id),
                esc(id),
                esc(v["lastModified"].as_str().unwrap_or("")),
                if v["private"].as_bool() == Some(true) { " <span class=\"b\">private</span>" } else { "" }
            ));
        }
        b.push_str("</ul>");
    }
    html(200, label, &b)
}

fn repo(cfg: &Cfg, kind: &str, ns: &str, name: &str, user: &Option<String>) -> R {
    let Some(dir) = repo_dir(cfg, kind, ns, name) else { return not_found() };
    if !dir.is_dir() || !can_read(cfg, &dir, ns, user) {
        return not_found();
    }
    let v = repo_json(cfg, kind, ns, name, &dir, "main", true);
    let id = format!("{ns}/{name}");
    let mut b = format!(
        "<h1>{}</h1><p class=\"m\">{} <span class=\"b\">{}</span> commit <code>{}</code></p><h2>Files</h2>",
        esc(&id),
        esc(kind.trim_end_matches('s')),
        if v["private"].as_bool() == Some(true) { "private" } else { "public" },
        esc(&v["sha"].as_str().unwrap_or("").chars().take(8).collect::<String>())
    );
    let files = v["siblings"].as_array().cloned().unwrap_or_default();
    if files.is_empty() {
        b.push_str("<p class=\"e\">This repository has no files yet.</p>");
    } else {
        b.push_str("<ul class=\"l\">");
        for f in files {
            let p = f["rfilename"].as_str().unwrap_or("");
            b.push_str(&format!("<li><a href=\"{}/{}/resolve/main/{}\">{}</a></li>", prefix_of(kind), esc(&id), esc(p), esc(p)));
        }
        b.push_str("</ul>");
    }
    html(200, &id, &b)
}

/// HTML route; `None` when the path is not a page.
pub fn page(cfg: &Cfg, s: &[&str], q: &[(String, String)], user: &Option<String>) -> Option<R> {
    Some(match s {
        [] => {
            let b = "<h1>Hub</h1><p>Models, datasets and spaces hosted on this server, or mirrored from an upstream hub.</p>\
<form class=\"s\" role=\"search\" action=\"/models\" method=\"get\"><input type=\"search\" name=\"search\" aria-label=\"Search models\" placeholder=\"Search models\"><button type=\"submit\">Search</button></form>";
            html(200, "Hub", b)
        }
        ["models"] => list(cfg, "models", q, user),
        ["datasets"] => list(cfg, "datasets", q, user),
        ["spaces"] => list(cfg, "spaces", q, user),
        [k @ ("datasets" | "spaces"), ns, name] => repo(cfg, k, ns, name, user),
        [ns, name] if !matches!(*ns, "api" | "lfs-upload" | "lfs-download" | "lfs-verify") => repo(cfg, "models", ns, name, user),
        _ => return None,
    })
}
