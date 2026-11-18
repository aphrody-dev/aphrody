// agent-browser driven from Bun: one isolated session per run, every command with --json, the answer's data returned.
// The binary is `agent-browser` on PATH, or REPLICA_BROWSER.

type Answer = { success?: boolean; data?: Record<string, unknown>; error?: unknown };

export class Browser {
  constructor(
    readonly session: string,
    readonly bin = process.env.REPLICA_BROWSER || "agent-browser",
  ) {}

  async run(...args: string[]): Promise<Record<string, unknown>> {
    const p = Bun.spawn([this.bin, "--session", this.session, "--json", ...args], { stdin: "ignore", stdout: "pipe", stderr: "pipe" });
    const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
    await p.exited;
    const line = out.trim().split("\n").filter(Boolean).at(-1) ?? "";
    let doc: Answer;
    try {
      doc = JSON.parse(line) as Answer;
    } catch {
      throw new Error(`${this.bin} ${args[0]}: ${(err || out).trim().slice(0, 300)}`);
    }
    if (!doc.success) throw new Error(`${this.bin} ${args[0]}: ${JSON.stringify(doc.error).slice(0, 300)}`);
    return doc.data ?? {};
  }

  open = (url: string) => this.run("open", url);
  wait = (ms: number) => this.run("wait", String(ms));
  viewport = (w: number, h: number) => this.run("set", "viewport", String(w), String(h));
  screenshot = (path: string) => this.run("screenshot", path);
  url = async () => String((await this.run("get", "url")).url ?? "");
  /** Evaluates an expression in the page; the result comes back as the string the page produced. */
  eval = async (js: string) => String((await this.run("eval", js)).result ?? "");
  close = () => this.run("close").catch(() => ({}));
}
