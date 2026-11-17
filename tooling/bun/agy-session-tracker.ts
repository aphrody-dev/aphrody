#!/usr/bin/env bun
import { watch, existsSync, readdirSync, statSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

const BRAIN_DIR = join(homedir(), ".gemini", "antigravity-cli", "brain");
const PORT = Number(process.env.AGY_TRACKER_PORT || 4789);
const DEFAULT_SESSION = process.argv[2] || "aace0256-452b-4cd1-83a0-64b5bbce9f27";

interface ToolCall {
  name: string;
  args?: Record<string, unknown>;
}

interface RawStep {
  step_index: number;
  source: string;
  type: string;
  status?: string;
  created_at?: string;
  content?: string;
  thinking?: string;
  tool_calls?: ToolCall[];
}

interface SessionSummary {
  id: string;
  lastModified: string;
  stepCount: number;
  firstPrompt: string;
  isActive: boolean;
}

interface ParsedSession {
  id: string;
  lastModified: string;
  stepCount: number;
  startedAt: string;
  updatedAt: string;
  userPrompts: { step: number; time: string; text: string }[];
  toolCounts: Record<string, number>;
  filesTouched: { path: string; op: string; count: number }[];
  commands: { step: number; time: string; cmd: string; summary?: string }[];
  timeline: {
    step: number;
    type: string;
    source: string;
    status: string;
    time: string;
    summary: string;
    content: string;
    tools: { name: string; argsPreview: string }[];
  }[];
}

function listSessions(): SessionSummary[] {
  if (!existsSync(BRAIN_DIR)) return [];
  return readdirSync(BRAIN_DIR, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .map((d) => {
      const logPath = join(BRAIN_DIR, d.name, ".system_generated", "logs", "transcript.jsonl");
      if (!existsSync(logPath)) return null;
      const st = statSync(logPath);
      const raw = readFileSync(logPath, "utf8");
      const lines = raw.trim().split("\n").filter(Boolean);
      let firstPrompt = "";
      for (const line of lines.slice(0, 12)) {
        try {
          const parsed = JSON.parse(line) as RawStep;
          if (parsed.type === "USER_INPUT" && parsed.content) {
            const m = parsed.content.match(/<USER_REQUEST>\s*([\s\S]*?)\s*<\/USER_REQUEST>/);
            const txt = (m ? m[1] : parsed.content).trim();
            if (txt && txt !== "hey") {
              firstPrompt = txt.slice(0, 120);
              break;
            }
            if (!firstPrompt) firstPrompt = txt.slice(0, 120);
          }
        } catch {}
      }
      return {
        id: d.name,
        lastModified: st.mtime.toISOString(),
        stepCount: lines.length,
        firstPrompt: firstPrompt || "(no prompt)",
        isActive: Date.now() - st.mtimeMs < 15_000,
      };
    })
    .filter((x): x is SessionSummary => x !== null)
    .sort((a, b) => b.lastModified.localeCompare(a.lastModified));
}

function parseSession(sessionId: string): ParsedSession | null {
  const logPath = join(BRAIN_DIR, sessionId, ".system_generated", "logs", "transcript.jsonl");
  if (!existsSync(logPath)) return null;
  const st = statSync(logPath);
  const raw = readFileSync(logPath, "utf8");
  const lines = raw.trim().split("\n").filter(Boolean);

  const userPrompts: ParsedSession["userPrompts"] = [];
  const toolCounts: Record<string, number> = {};
  const fileMap = new Map<string, { path: string; op: string; count: number }>();
  const commands: ParsedSession["commands"] = [];
  const timeline: ParsedSession["timeline"] = [];

  let startedAt = "";
  let updatedAt = "";

  for (const line of lines) {
    let step: RawStep;
    try {
      step = JSON.parse(line) as RawStep;
    } catch {
      continue;
    }
    if (!startedAt && step.created_at) startedAt = step.created_at;
    if (step.created_at) updatedAt = step.created_at;

    if (step.type === "USER_INPUT" && step.content) {
      const m = step.content.match(/<USER_REQUEST>\s*([\s\S]*?)\s*<\/USER_REQUEST>/);
      userPrompts.push({
        step: step.step_index,
        time: step.created_at || "",
        text: (m ? m[1] : step.content).trim(),
      });
    }

    const tools: { name: string; argsPreview: string }[] = [];
    if (Array.isArray(step.tool_calls)) {
      for (const tc of step.tool_calls) {
        toolCounts[tc.name] = (toolCounts[tc.name] || 0) + 1;
        const args = tc.args || {};
        if ((tc.name === "read_file" || tc.name === "edit_file" || tc.name === "write_file") && typeof args.Path === "string") {
          const key = `${tc.name}:${args.Path}`;
          const prev = fileMap.get(key);
          if (prev) prev.count++;
          else fileMap.set(key, { path: args.Path, op: tc.name.replace("_file", ""), count: 1 });
        }
        if (tc.name === "run_command" && typeof args.CommandLine === "string") {
          commands.push({
            step: step.step_index,
            time: step.created_at || "",
            cmd: args.CommandLine,
            summary: typeof args.toolSummary === "string" ? args.toolSummary : undefined,
          });
        }
        tools.push({
          name: tc.name,
          argsPreview: JSON.stringify(args, null, 2).slice(0, 600),
        });
      }
    }

    const content = (step.content || "").trim();
    const summary =
      step.type === "USER_INPUT"
        ? (userPrompts[userPrompts.length - 1]?.text || "").slice(0, 140)
        : tools.length > 0
          ? tools.map((t) => t.name).join(", ")
          : content.slice(0, 140).replace(/\s+/g, " ");

    timeline.push({
      step: step.step_index,
      type: step.type,
      source: step.source,
      status: step.status || "DONE",
      time: step.created_at || "",
      summary,
      content: content.slice(0, 6000),
      tools,
    });
  }

  return {
    id: sessionId,
    lastModified: st.mtime.toISOString(),
    stepCount: lines.length,
    startedAt,
    updatedAt,
    userPrompts,
    toolCounts,
    filesTouched: Array.from(fileMap.values()).sort((a, b) => b.count - a.count),
    commands,
    timeline,
  };
}

function renderHtml(initialData: ParsedSession | null, sessions: SessionSummary[], liveMode: boolean): string {
  const payload = JSON.stringify({ initialData, sessions, liveMode }).replace(/</g, "\\u003c");
  return `<!doctype html>
<html lang="fr" data-theme="dark">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width,initial-scale=1" />
<title>Aphrody · AGY &amp; Gemini Session Studio</title>
<style>
  :root {
    --r-sm: 6px; --r-md: 10px; --r-lg: 14px;
    --ease-out: cubic-bezier(0.22, 1, 0.36, 1);
    --dur-fast: 140ms; --dur-med: 220ms;
    --font-sans: "Inter", ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
    --font-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }
  [data-theme="dark"] {
    --surface-canvas: #0a0d12;
    --surface-elevated: #111620;
    --surface-sunken: #0d1119;
    --surface-interactive: #171e2c;
    --surface-hover: #1e2738;
    --border-subtle: #1f293b;
    --border-strong: #2e3c54;
    --ink-primary: #edf2f7;
    --ink-secondary: #9fb0c7;
    --ink-muted: #64748b;
    --accent-primary: #38bdf8;
    --accent-soft: rgba(56, 189, 248, 0.14);
    --status-live: #34d399;
    --status-live-soft: rgba(52, 211, 153, 0.14);
    --status-warn: #fbbf24;
    --status-purple: #c084fc;
    --shadow-sm: 0 1px 2px rgba(0,0,0,0.4);
    --shadow-lg: 0 12px 32px rgba(0,0,0,0.55);
  }
  [data-theme="light"] {
    --surface-canvas: #f5f7fa;
    --surface-elevated: #ffffff;
    --surface-sunken: #eef2f6;
    --surface-interactive: #f8fafc;
    --surface-hover: #e2e8f0;
    --border-subtle: #e2e8f0;
    --border-strong: #cbd5e1;
    --ink-primary: #0f172a;
    --ink-secondary: #475569;
    --ink-muted: #94a3b8;
    --accent-primary: #0284c7;
    --accent-soft: rgba(2, 132, 199, 0.10);
    --status-live: #059669;
    --status-live-soft: rgba(5, 150, 105, 0.12);
    --status-warn: #d97706;
    --status-purple: #7c3aed;
    --shadow-sm: 0 1px 2px rgba(15,23,42,0.06);
    --shadow-lg: 0 12px 32px rgba(15,23,42,0.12);
  }
  @media (prefers-reduced-motion: reduce) {
    * { transition: none !important; animation: none !important; }
  }
  * { box-sizing: border-box; }
  body {
    margin: 0; font-family: var(--font-sans); background: var(--surface-canvas);
    color: var(--ink-primary); font-size: 13px; line-height: 1.5;
    display: grid; grid-template-rows: 52px 1fr; height: 100vh; overflow: hidden;
  }
  header {
    background: var(--surface-elevated); border-bottom: 1px solid var(--border-subtle);
    padding: 0 16px; display: flex; align-items: center; justify-content: space-between; gap: 12px;
  }
  .brand { display: flex; align-items: center; gap: 10px; font-weight: 700; letter-spacing: -0.01em; }
  .pulse { width: 8px; height: 8px; border-radius: 50%; background: var(--status-live); box-shadow: 0 0 0 0 var(--status-live); animation: pulse 2s infinite; }
  .pulse.idle { animation: none; background: var(--ink-muted); }
  @keyframes pulse { 0% { box-shadow: 0 0 0 0 rgba(52,211,153,0.6); } 70% { box-shadow: 0 0 0 6px rgba(52,211,153,0); } 100% { box-shadow: 0 0 0 0 rgba(52,211,153,0); } }
  .toolbar { display: flex; align-items: center; gap: 8px; }
  .btn, select, input {
    background: var(--surface-interactive); color: var(--ink-primary);
    border: 1px solid var(--border-subtle); border-radius: var(--r-sm);
    padding: 6px 10px; font-size: 12px; font-family: inherit;
    transition: border-color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out), transform var(--dur-fast) var(--ease-out);
  }
  .btn { cursor: pointer; font-weight: 600; display: inline-flex; align-items: center; gap: 6px; }
  .btn:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .btn:active { transform: scale(0.98); }
  .btn-primary { background: var(--accent-soft); color: var(--accent-primary); border-color: transparent; }
  .btn-primary:hover { border-color: var(--accent-primary); }

  .shell { display: grid; grid-template-columns: 260px 1fr 380px; height: 100%; overflow: hidden; }
  @media (max-width: 1100px) { .shell { grid-template-columns: 220px 1fr; } .inspector { display: none; } .inspector.open { display: flex; position: fixed; right: 0; top: 52px; bottom: 0; width: 380px; z-index: 20; box-shadow: var(--shadow-lg); } }
  .sidebar { background: var(--surface-sunken); border-right: 1px solid var(--border-subtle); display: flex; flex-direction: column; overflow: hidden; }
  .sidebar-head { padding: 12px; border-bottom: 1px solid var(--border-subtle); font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--ink-muted); }
  .session-list { flex: 1; overflow-y: auto; padding: 8px; display: grid; gap: 4px; align-content: start; }
  .session-item {
    padding: 8px 10px; border-radius: var(--r-sm); cursor: pointer; border: 1px solid transparent;
    transition: background var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out);
  }
  .session-item:hover { background: var(--surface-hover); }
  .session-item.active { background: var(--accent-soft); border-color: var(--accent-primary); }
  .session-title { font-weight: 600; font-size: 12px; display: flex; justify-content: space-between; align-items: center; }
  .session-sub { font-size: 11px; color: var(--ink-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px; }

  .workspace { overflow-y: auto; padding: 16px 20px; display: flex; flex-direction: column; gap: 16px; }
  .kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(145px, 1fr)); gap: 10px; }
  .kpi { background: var(--surface-elevated); border: 1px solid var(--border-subtle); border-radius: var(--r-md); padding: 12px 14px; box-shadow: var(--shadow-sm); }
  .kpi-label { font-size: 11px; color: var(--ink-muted); text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600; }
  .kpi-val { font-size: 20px; font-weight: 700; margin-top: 4px; font-variant-numeric: tabular-nums; color: var(--ink-primary); }

  .tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border-subtle); padding-bottom: 8px; }
  .tab { padding: 6px 12px; border-radius: var(--r-sm); font-size: 12px; font-weight: 600; color: var(--ink-secondary); cursor: pointer; border: 1px solid transparent; background: transparent; }
  .tab:hover { color: var(--ink-primary); background: var(--surface-hover); }
  .tab.active { color: var(--accent-primary); background: var(--accent-soft); }

  .card { background: var(--surface-elevated); border: 1px solid var(--border-subtle); border-radius: var(--r-md); padding: 14px; box-shadow: var(--shadow-sm); }
  .card h3 { margin: 0 0 10px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--ink-muted); }
  .pills { display: flex; flex-wrap: wrap; gap: 6px; }
  .pill { background: var(--surface-sunken); border: 1px solid var(--border-subtle); border-radius: 999px; padding: 3px 10px; font-size: 11px; font-variant-numeric: tabular-nums; }
  .pill b { color: var(--accent-primary); }

  .tl-list { display: grid; gap: 6px; }
  .tl-row {
    background: var(--surface-elevated); border: 1px solid var(--border-subtle); border-radius: var(--r-sm);
    padding: 8px 12px; display: flex; align-items: center; gap: 10px; cursor: pointer;
    transition: border-color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out);
  }
  .tl-row:hover { border-color: var(--border-strong); background: var(--surface-hover); }
  .tl-row.selected { border-color: var(--accent-primary); background: var(--accent-soft); }
  .badge { font-size: 10px; font-weight: 700; padding: 2px 6px; border-radius: 4px; text-transform: uppercase; font-family: var(--font-mono); }
  .USER_INPUT { background: var(--accent-soft); color: var(--accent-primary); }
  .PLANNER_RESPONSE { background: rgba(192,132,252,0.15); color: var(--status-purple); }
  .GENERIC { background: var(--status-live-soft); color: var(--status-live); }
  .SYSTEM_MESSAGE { background: rgba(251,191,36,0.15); color: var(--status-warn); }
  .mono { font-family: var(--font-mono); font-size: 11.5px; }

  .inspector { background: var(--surface-elevated); border-left: 1px solid var(--border-subtle); display: flex; flex-direction: column; overflow: hidden; }
  .inspector-head { padding: 12px 14px; border-bottom: 1px solid var(--border-subtle); display: flex; justify-content: space-between; align-items: center; font-weight: 700; font-size: 12px; }
  .inspector-body { flex: 1; overflow-y: auto; padding: 14px; font-family: var(--font-mono); font-size: 11.5px; white-space: pre-wrap; word-break: break-word; color: var(--ink-secondary); }
</style>
</head>
<body>
<header>
  <div class="brand">
    <span id="livePulse" class="pulse"></span>
    <span>Aphrody · AGY Session Studio</span>
    <span id="modePill" class="pill">LIVE</span>
  </div>
  <div class="toolbar">
    <input id="searchInput" type="search" placeholder="Filtrer (outil, fichier, texte)..." aria-label="Filtrer" />
    <button id="themeBtn" class="btn" title="Basculer thème clair/sombre">Thème</button>
    <button id="exportBtn" class="btn btn-primary">Exporter HTML Portable</button>
  </div>
</header>

<div class="shell">
  <aside class="sidebar">
    <div class="sidebar-head">Sessions AGY / Gemini</div>
    <div class="session-list" id="sessionList"></div>
  </aside>

  <main class="workspace">
    <div class="kpis" id="kpis"></div>
    <div class="tabs" role="tablist">
      <button class="tab active" data-tab="timeline">Timeline</button>
      <button class="tab" data-tab="overview">Vue d'ensemble &amp; Outils</button>
      <button class="tab" data-tab="files">Fichiers touchés</button>
      <button class="tab" data-tab="commands">Commandes</button>
    </div>
    <div id="tabContent"></div>
  </main>

  <aside class="inspector" id="inspector">
    <div class="inspector-head">
      <span id="inspTitle">Inspecteur d'étape</span>
      <button id="copyBtn" class="btn" style="padding:3px 8px;font-size:11px;">Copier</button>
    </div>
    <div class="inspector-body" id="inspBody">Sélectionnez une étape dans la timeline pour inspecter ses détails, arguments d'outils et sorties.</div>
  </aside>
</div>

<script>
const BOOT = ${payload};
let currentData = BOOT.initialData;
let sessions = BOOT.sessions || [];
const liveMode = BOOT.liveMode;
let activeTab = "timeline";
let selectedStep = null;

const el = (id) => document.getElementById(id);
const esc = (s) => String(s || "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

function applyTheme(t) {
  document.documentElement.setAttribute("data-theme", t);
  try { localStorage.setItem("agy-tracker-theme", t); } catch {}
}
applyTheme((() => { try { return localStorage.getItem("agy-tracker-theme"); } catch { return null; } })() || "dark");
el("themeBtn").onclick = () => applyTheme(document.documentElement.getAttribute("data-theme") === "dark" ? "light" : "dark");

function renderSidebar() {
  el("sessionList").innerHTML = sessions.map(s => \`
    <div class="session-item \${currentData && s.id === currentData.id ? "active" : ""}" data-id="\${esc(s.id)}">
      <div class="session-title">
        <span class="mono">\${esc(s.id.slice(0,8))}</span>
        <span class="pill">\${s.stepCount} \${s.isActive ? "●" : ""}</span>
      </div>
      <div class="session-sub">\${esc(s.firstPrompt)}</div>
    </div>
  \`).join("");
  el("sessionList").querySelectorAll(".session-item").forEach(node => {
    node.onclick = async () => {
      if (!liveMode) return;
      const res = await fetch("/api/session?id=" + encodeURIComponent(node.dataset.id));
      currentData = await res.json();
      selectedStep = null;
      renderAll();
    };
  });
}

function renderKpis() {
  if (!currentData) return;
  const d = currentData;
  const totalTools = Object.values(d.toolCounts).reduce((a, b) => a + b, 0);
  el("kpis").innerHTML = [
    ["Étapes", d.stepCount],
    ["Appels d'outils", totalTools],
    ["Fichiers", d.filesTouched.length],
    ["Commandes", d.commands.length],
    ["Dernière MAJ", d.updatedAt ? new Date(d.updatedAt).toLocaleTimeString() : "-"]
  ].map(([k, v]) => \`<div class="kpi"><div class="kpi-label">\${esc(k)}</div><div class="kpi-val">\${esc(v)}</div></div>\`).join("");
}

function renderTabContent() {
  if (!currentData) return;
  const d = currentData;
  const q = el("searchInput").value.toLowerCase();
  const root = el("tabContent");

  if (activeTab === "overview") {
    root.innerHTML = \`
      <div class="card">
        <h3>Requêtes Utilisateur</h3>
        <div class="tl-list">\${d.userPrompts.map(p => \`<div class="tl-row"><b>#\${p.step}</b><span>\${esc(p.text)}</span></div>\`).join("") || "Aucun prompt"}</div>
      </div>
      <div class="card" style="margin-top:12px;">
        <h3>Répartition des outils</h3>
        <div class="pills">\${Object.entries(d.toolCounts).sort((a,b)=>b[1]-a[1]).map(([k,v])=>\`<span class="pill">\${esc(k)}: <b>\${v}</b></span>\`).join("")}</div>
      </div>
    \`;
    return;
  }

  if (activeTab === "files") {
    const files = d.filesTouched.filter(f => !q || f.path.toLowerCase().includes(q)).slice(0, 80);
    root.innerHTML = \`<div class="card"><h3>Fichiers touchés (\${files.length})</h3><div class="tl-list">\${files.map(f => \`<div class="tl-row mono"><span class="badge GENERIC">\${esc(f.op)} ×\${f.count}</span><span>\${esc(f.path)}</span></div>\`).join("")}</div></div>\`;
    return;
  }

  if (activeTab === "commands") {
    const cmds = d.commands.filter(c => !q || c.cmd.toLowerCase().includes(q)).slice(-60).reverse();
    root.innerHTML = \`<div class="card"><h3>Commandes exécutées (\${cmds.length})</h3><div class="tl-list">\${cmds.map(c => \`<div class="tl-row mono"><b>#\${c.step}</b><span>\${c.summary ? esc(c.summary) + " — " : ""}\${esc(c.cmd)}</span></div>\`).join("")}</div></div>\`;
    return;
  }

  const items = d.timeline.filter(t => !q || t.summary.toLowerCase().includes(q) || t.content.toLowerCase().includes(q)).slice(-150).reverse();
  root.innerHTML = \`<div class="tl-list">\${items.map(t => \`
    <div class="tl-row \${selectedStep === t.step ? "selected" : ""}" data-step="\${t.step}">
      <span class="mono" style="color:var(--ink-muted)">#\${t.step}</span>
      <span class="badge \${esc(t.type)}">\${esc(t.type)}</span>
      <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">\${esc(t.summary)}</span>
      <span class="mono" style="color:var(--ink-muted)">\${t.time ? new Date(t.time).toLocaleTimeString() : ""}</span>
    </div>
  \`).join("")}</div>\`;

  root.querySelectorAll(".tl-row[data-step]").forEach(row => {
    row.onclick = () => {
      const s = Number(row.dataset.step);
      selectedStep = s;
      const item = d.timeline.find(x => x.step === s);
      if (item) {
        el("inspTitle").textContent = \`Étape #\${item.step} · \${item.type}\`;
        el("inspBody").textContent = (item.tools.map(x => "TOOL " + x.name + "\\n" + x.argsPreview).join("\\n\\n") + (item.tools.length ? "\\n\\n---\\n\\n" : "") + item.content) || "(vide)";
      }
      renderTabContent();
    };
  });
}

function renderAll() {
  el("modePill").textContent = liveMode ? "LIVE AUTO-RELOAD" : "PORTABLE SNAPSHOT";
  el("livePulse").className = "pulse" + (liveMode ? "" : " idle");
  renderSidebar();
  renderKpis();
  renderTabContent();
}

document.querySelectorAll(".tab").forEach(btn => {
  btn.onclick = () => {
    document.querySelectorAll(".tab").forEach(b => b.classList.remove("active"));
    btn.classList.add("active");
    activeTab = btn.dataset.tab;
    renderTabContent();
  };
});
el("searchInput").addEventListener("input", renderTabContent);
el("copyBtn").onclick = () => navigator.clipboard?.writeText(el("inspBody").textContent || "");
el("exportBtn").onclick = () => { if (liveMode && currentData) window.location.href = "/export?id=" + encodeURIComponent(currentData.id); };

if (liveMode) {
  const es = new EventSource("/events");
  es.onmessage = async (ev) => {
    if (ev.data === "reload" && currentData) {
      const [sRes, dRes] = await Promise.all([
        fetch("/api/sessions"),
        fetch("/api/session?id=" + encodeURIComponent(currentData.id))
      ]);
      sessions = await sRes.json();
      currentData = await dRes.json();
      renderAll();
    }
  };
}

renderAll();
</script>
</body>
</html>`;
}

const clients = new Set<ReadableStreamDefaultController<Uint8Array>>();
const encoder = new TextEncoder();

function broadcastReload() {
  for (const ctrl of clients) {
    try {
      ctrl.enqueue(encoder.encode("data: reload\n\n"));
    } catch {
      clients.delete(ctrl);
    }
  }
}

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
if (existsSync(BRAIN_DIR)) {
  watch(BRAIN_DIR, { recursive: true }, (_event, filename) => {
    if (filename && filename.endsWith("transcript.jsonl")) {
      if (debounceTimer) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(broadcastReload, 400);
    }
  });
}

Bun.serve({
  port: PORT,
  fetch(req) {
    const url = new URL(req.url);
    if (url.pathname === "/events") {
      const stream = new ReadableStream<Uint8Array>({
        start(ctrl) {
          clients.add(ctrl);
          ctrl.enqueue(encoder.encode(": connected\n\n"));
        },
        cancel(ctrl) {
          clients.delete(ctrl);
        },
      });
      return new Response(stream, {
        headers: { "Content-Type": "text/event-stream", "Cache-Control": "no-cache", Connection: "keep-alive" },
      });
    }
    if (url.pathname === "/api/sessions") return Response.json(listSessions());
    if (url.pathname === "/api/session") {
      const id = url.searchParams.get("id") || DEFAULT_SESSION;
      const data = parseSession(id);
      return data ? Response.json(data) : new Response("Not found", { status: 404 });
    }
    if (url.pathname === "/export") {
      const id = url.searchParams.get("id") || DEFAULT_SESSION;
      const data = parseSession(id);
      if (!data) return new Response("Not found", { status: 404 });
      const html = renderHtml(data, [{ id: data.id, lastModified: data.lastModified, stepCount: data.stepCount, firstPrompt: data.userPrompts[0]?.text || "", isActive: false }], false);
      return new Response(html, {
        headers: {
          "Content-Type": "text/html; charset=utf-8",
          "Content-Disposition": `attachment; filename="agy-session-${id.slice(0, 8)}.html"`,
        },
      });
    }
    const sessions = listSessions();
    const targetId = sessions.some((s) => s.id === DEFAULT_SESSION) ? DEFAULT_SESSION : sessions[0]?.id || DEFAULT_SESSION;
    const data = parseSession(targetId);
    return new Response(renderHtml(data, sessions, true), {
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  },
});

console.log(`AGY Session Studio running at http://localhost:${PORT}`);
