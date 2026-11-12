// SPDX-License-Identifier: Apache-2.0
/**
 * Lazy-injected Sovereign Suite stylesheet.
 * Keeps initial critical-path style.css inside the < 14 KB TCP IW10 LCP budget.
 */
const SUITE_CSS = `
.sovereign-suite-shell{display:flex;flex-direction:column;height:100%;color:var(--text-color)}
.suite-toolbar{display:flex;flex-wrap:wrap;gap:8px;padding:12px 16px;border-bottom:1px solid var(--m3-outlineVariant);background:var(--m3-surfaceContainer)}
.suite-tab-btn,.suite-btn{min-height:40px;padding:8px 16px;border-radius:20px;border:1px solid var(--m3-outlineVariant);background:transparent;color:var(--text-color);font-size:13px;font-weight:500;cursor:pointer;text-decoration:none;display:inline-flex;align-items:center}
.suite-tab-btn.active,.suite-btn.primary{background:var(--m3-primary);color:var(--m3-onPrimary);border-color:var(--m3-primary)}
.suite-stage{flex:1;overflow:auto;padding:16px}
.suite-split{display:grid;grid-template-columns:1.2fr 1fr;gap:16px}
.suite-panel{display:flex;flex-direction:column;gap:12px}
.suite-panel.center{align-items:center;justify-content:center}
.suite-actions{display:flex;flex-wrap:wrap;align-items:center;gap:8px}
.suite-input,.suite-textarea,.suite-cell-input{padding:8px 12px;border-radius:8px;border:1px solid var(--m3-outlineVariant);background:transparent;color:var(--text-color);font-family:inherit;font-size:13px}
.suite-codeblock{margin:0;padding:12px;border-radius:8px;background:var(--m3-surfaceContainer);font-family:ui-monospace,Consolas,monospace;font-size:12px;overflow-x:auto;white-space:pre-wrap}
.suite-list{display:flex;flex-direction:column;gap:8px}
.suite-card-row{padding:12px;border-radius:8px;border:1px solid var(--m3-outlineVariant)}
.suite-badge{display:inline-block;padding:4px 8px;border-radius:12px;background:var(--m3-secondaryContainer);color:var(--m3-onSecondaryContainer);font-size:11px;font-weight:600}
.suite-muted,.suite-status{font-size:13px;color:var(--text-secondary);margin:0}
.suite-grid-table{width:100%;border-collapse:collapse}
.suite-grid-table th,.suite-grid-table td{border:1px solid var(--m3-outlineVariant);padding:4px}
`;

export function ensureSuiteStyles(): void {
  if (typeof document === "undefined") return;
  if (document.getElementById("aphrody-suite-styles")) return;
  const style = document.createElement("style");
  style.id = "aphrody-suite-styles";
  style.textContent = SUITE_CSS;
  document.head.appendChild(style);
}
