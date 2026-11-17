// Hermetic tests: a local Bun.serve origin replays recorded fixtures; nothing here touches the network.
// Run: bun test tooling/kernel-watch
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseFeed } from "./adapters.ts";
import { backoff, poll, pollSource } from "./poll.ts";
import { loadManifest, parseDuration, parseManifest, type Manifest } from "./manifest.ts";
import { decideCommit } from "./publish.ts";
import { Store, toFtsQuery } from "./store.ts";
import { buildCycles, closenessKey, compareVersions, distance, formatVersion, parseVersion } from "./version.ts";
import { computeDigests, computeVersions, generate } from "./views.ts";

const HERE = import.meta.dir;
const fixture = (name: string): string => readFileSync(join(HERE, "fixtures", name), "utf8");
const scratch = mkdtempSync(join(tmpdir(), "kernel-watch-"));
afterAll(() => rmSync(scratch, { recursive: true, force: true }));

const TAGS = ["v6.12", "v6.13", "v6.14", "v6.15", "v6.16", "v6.17", "v6.18", "v6.19", "v7.0", "v7.1", "v7.2", "v7.3", "v7.4-rc1", "v7.4-rc2", "v7.4-rc3"];
const tagsHtml = (): string =>
  `<table>${TAGS.map((t, i) => `<tr><td><a href='/pub/scm/linux.git/tag/?h=${t}'>${t}</a></td><td><a href='/snap'>x.tar.gz</a>&nbsp;&nbsp;</td><td>Linus Torvalds</td><td colspan='2'><span class='age-days' data-ut='${1700000000 + i * 86400}' title='t'>1 day</span></td></tr>`).join("\n")}</table>`;

interface Route {
  body: string;
  type: string;
}
let origin: Bun.Server<undefined>;
let base = "";
const routes = new Map<string, Route>();
const hits = new Map<string, number>();
const failures = new Map<string, { status: number; retryAfter?: number }>();
const seenAuth = new Map<string, string | null>();

const route = (path: string, body: string, type = "application/json"): void => void routes.set(path, { body, type });
function resetRoutes(): void {
  routes.clear();
  hits.clear();
  failures.clear();
  seenAuth.clear();
  route("/kernel-releases.json", fixture("kernel-releases.json"));
  route("/cgit-atom.xml", fixture("cgit-atom.xml"), "text/xml");
  route("/tags", tagsHtml(), "text/html");
  route("/gh/releases", fixture("github-releases.json"));
  route("/gh/commits", fixture("github-commits.json"));
  route("/gh/issues", fixture("github-issues.json"));
  route("/gh/pulls", fixture("github-pulls.json"));
  route("/gh/search-pr", fixture("github-search.json"));
  route("/gh/search-issue", fixture("github-search-issues.json"));
  route("/gl/mrs", fixture("gitlab-merge-requests.json"));
  route("/gl/tags", fixture("gitlab-tags.json"));
  route("/gl/commits", fixture("gitlab-commits.json"));
  route("/bodhi", fixture("bodhi.json"));
  route("/arch", fixture("arch-linux.json"));
  route("/android-makefile", fixture("makefile.b64"), "text/plain");
  route("/apkbuild", fixture("apkbuild.txt"), "text/plain");
  route("/ubuntu-mainline", fixture("ubuntu-mainline.html"), "text/html");
  route("/launchpad", fixture("launchpad.json"));
  route("/nix", fixture("nix-kernels.json"), "text/plain");
  route("/sitemap.xml", fixture("redox-sitemap.xml"), "application/xml");
  route("/auth/a", fixture("arch-linux.json"));
  route("/auth/b", fixture("arch-linux.json"));
  route("/flaky", "{}");
  route("/down", "{}");
}

beforeAll(() => {
  origin = Bun.serve({
    port: 0,
    fetch(req) {
      const { pathname } = new URL(req.url);
      hits.set(pathname, (hits.get(pathname) ?? 0) + 1);
      if (pathname.startsWith("/auth/")) seenAuth.set(`${new URL(req.url).hostname}${pathname}`, req.headers.get("authorization"));
      const failure = failures.get(pathname);
      if (failure) return new Response("fail", { status: failure.status, headers: failure.retryAfter ? { "retry-after": String(failure.retryAfter) } : {} });
      const found = routes.get(pathname);
      if (!found) return new Response("not found", { status: 404 });
      const etag = `"${Bun.hash(found.body).toString(16)}"`;
      if (req.headers.get("if-none-match") === etag) return new Response(null, { status: 304, headers: { etag } });
      return new Response(found.body, { headers: { "content-type": found.type, etag } });
    },
  });
  base = `http://127.0.0.1:${origin.port}`;
  resetRoutes();
});

function testManifest(extra = ""): Manifest {
  const source = (id: string, kernel: string, adapter: string, kind: string, path: string, yields: string, more = ""): string =>
    `[[source]]\nid = "${id}"\nkernel = "${kernel}"\nadapter = "${adapter}"\nkind = "${kind}"\nurl = "${base}${path}"\nyields = ["${yields}"]\nmin_interval = "10m"\n${more}\n`;
  const kernel = (id: string, role: string, lineage: string): string => `[[kernel]]\nid = "${id}"\nname = "${id}"\nrole = "${role}"\nlineage = "${lineage}"\ndescription = "test"\n`;
  return parseManifest(`
[watch]
user_agent = "kernel-watch-test"
max_pages = 2
[watch.auth."127.0.0.1:${origin.port}"]
scheme = "Bearer"
env = ["TEST_TOKEN"]
${["linux:upstream:linux", "asterinas:research:own", "redox:research:own", "fedora:downstream:linux", "arch:downstream:linux", "android:downstream:linux", "alpine:downstream:linux", "rhel:downstream:linux", "nixos:downstream:linux", "ubuntu:downstream:linux"].map((k) => kernel(...(k.split(":") as [string, string, string]))).join("\n")}
${source("linux-releases", "linux", "kernel-org-releases", "releases-json", "/kernel-releases.json", "release")}
${source("linux-mainline-commits", "linux", "cgit-atom", "cgit-atom", "/cgit-atom.xml", "commit")}
${source("linux-tags", "linux", "cgit-tags", "cgit-atom", "/tags", "tag")}
${source("asterinas-releases", "asterinas", "github-releases", "forge-api", "/gh/releases", "release")}
${source("asterinas-commits", "asterinas", "github-commits", "forge-api", "/gh/commits", "commit")}
${source("asterinas-issues", "asterinas", "github-issues", "forge-api", "/gh/issues", "issue")}
${source("asterinas-pulls", "asterinas", "github-pulls", "forge-api", "/gh/pulls", "pr")}
${source("asterinas-counts", "asterinas", "github-open-counts", "forge-api", "/gh/search-pr", "metric", `issues_url = "${base}/gh/search-issue"`)}
${source("redox-merge-requests", "redox", "gitlab-merge-requests", "forge-api", "/gl/mrs", "pr")}
${source("redox-sitemap", "redox", "sitemap", "sitemap", "/sitemap.xml", "page")}
${source("fedora-ark-tags", "fedora", "gitlab-tags", "forge-api", "/gl/tags", "tag", `web = "https://example.invalid/tags"\nversion_from = '^kernel-(\\d.*)$'\nchannel = "kernel-ark"`)}
${source("fedora-bodhi", "fedora", "bodhi-updates", "package-index", "/bodhi", "package", `exclude_releases = ["ELN"]`)}
${source("rhel-c10s-commits", "rhel", "gitlab-commits", "forge-api", "/gl/commits", "commit", `version_from = '^\\[redhat\\] kernel-(\\d+\\.\\d+\\.\\d+)-\\d+'\nchannel = "centos-stream-10"`)}
${source("arch-linux", "arch", "json-version", "package-index", "/arch", "version", `path = "pkgver"\ntime_path = "last_update"\nchannel = "core/linux"`)}
${source("android-mainline-version", "android", "text-version", "gitiles", "/android-makefile", "version", `decode = "base64"\npatterns = ['^VERSION[ \\t]*=[ \\t]*(\\d+)', '^PATCHLEVEL[ \\t]*=[ \\t]*(\\d+)', '^SUBLEVEL[ \\t]*=[ \\t]*(\\d+)', '^EXTRAVERSION[ \\t]*=[ \\t]*(\\S*)']\ntemplate = "{1}.{2}.{3}{4}"\nchannel = "android-mainline"`)}
${source("alpine-lts-version", "alpine", "text-version", "package-index", "/apkbuild", "version", `patterns = ['^pkgver=(\\S+)']\nchannel = "edge/linux-lts"`)}
${source("ubuntu-launchpad", "ubuntu", "launchpad-sources", "package-index", "/launchpad", "package")}
${source("ubuntu-mainline-builds", "ubuntu", "html-index-versions", "html-index", "/ubuntu-mainline", "build", `channel = "mainline-ppa"`)}
${source("nixos-kernels", "nixos", "nix-kernels", "package-index", "/nix", "version")}
${extra}`);
}

describe("manifest", () => {
  test("the shipped sources.toml validates and covers the requested kernels", async () => {
    const manifest = await loadManifest(join(HERE, "sources.toml"));
    const ids = manifest.kernels.map((k) => k.id);
    for (const id of ["linux", "asterinas", "redox", "libernt", "android", "cachyos", "chromiumos", "alpine", "nixos", "fedora", "ubuntu", "arch", "rhel"]) expect(ids).toContain(id);
    for (const kernel of manifest.kernels) expect(manifest.sources.some((s) => s.kernel === kernel.id)).toBe(true);
    expect(manifest.sources.every((s) => s.url.startsWith("https://"))).toBe(true);
    const sourceIds = new Set(manifest.sources.map((s) => s.id));
    for (const org of manifest.kernels.flatMap((k) => k.orgs)) if (org.source) expect(sourceIds.has(org.source)).toBe(true);
  });

  test("names no absolute host path or account", () => {
    const text = readFileSync(join(HERE, "sources.toml"), "utf8");
    expect(text).not.toMatch(/\b[A-Za-z]:\\|\b[A-Za-z]:\/(?!\/)|\/home\/|\/Users\//);
  });

  test("every enabled source was verified live with status 200 and a recorded shape", async () => {
    const manifest = await loadManifest(join(HERE, "sources.toml"));
    const verified = JSON.parse(readFileSync(join(HERE, "fixtures", "verified.json"), "utf8")) as { sources: { id: string; status: number; shape: string }[]; sites: { status: number }[] };
    for (const source of manifest.sources.filter((s) => s.enabled)) {
      const found = verified.sources.find((v) => v.id === source.id);
      expect(found?.status, source.id).toBe(200);
      expect(found?.shape.length, source.id).toBeGreaterThan(0);
    }
    expect(verified.sites.length).toBeGreaterThan(20);
  });

  test("rejects bad entries and reports all problems at once", () => {
    const bad = `
[watch]
user_agent = "x"
[[kernel]]
id = "a"
name = "A"
role = "upstream"
lineage = "linux"
[[source]]
id = "s1"
kernel = "missing"
adapter = "nope"
kind = "forge-api"
url = "http://example.com/x?token=abc"
yields = []
min_interval = "10s"
[[source]]
id = "s1"
kernel = "a"
adapter = "atom"
kind = "feed"
url = "https://example.com/feed"
yields = ["x"]
`;
    expect(() => parseManifest(bad)).toThrow(/unknown kernel "missing"[\s\S]*unknown adapter "nope"[\s\S]*must be https[\s\S]*secret[\s\S]*yields[\s\S]*min_interval[\s\S]*duplicate source id "s1"/);
    expect(() => parseDuration("soon")).toThrow(/invalid duration/);
    expect(parseDuration("6h")).toBe(21600);
  });
});

describe("kernel versions", () => {
  test("parses distro version strings", () => {
    const cases: [string, string, number | null][] = [
      ["7.4-rc3", "7.4-rc3", 3], ["v7.4-rc3", "7.4-rc3", 3], ["7.4rc3-1", "7.4-rc3", 3], ["7.4.rc3", "7.4-rc3", 3],
      ["7.4.0-0.rc3.26.fc46", "7.4-rc3", 3], ["7.3.4.arch1", "7.3.4", null], ["6.12.0-284.el10", "6.12", null], ["6.8.0-151.151", "6.8", null], ["7.3.0", "7.3", null],
    ];
    for (const [raw, shown, rc] of cases) {
      const v = parseVersion(raw)!;
      expect(formatVersion(v), raw).toBe(shown);
      expect(v.rc, raw).toBe(rc);
    }
    expect(parseVersion("not-a-version")).toBeNull();
  });

  test("orders rc before final before stable updates, numerically", () => {
    const order = ["6.12.117", "7.3", "7.3.4", "7.4-rc2", "7.4-rc3", "7.4-rc10", "7.4", "7.4.1"].map((v) => parseVersion(v)!);
    const sorted = [...order].sort(() => Math.random() - 0.5).sort(compareVersions);
    expect(sorted.map(formatVersion)).toEqual(["6.12.117", "7.3", "7.3.4", "7.4-rc2", "7.4-rc3", "7.4-rc10", "7.4", "7.4.1"]);
    expect(compareVersions(parseVersion("7.4-rc3")!, parseVersion("7.3.4")!)).toBe(1);
  });

  test("counts mainline cycles from the tag list and sorts by closeness", () => {
    const cycles = buildCycles(TAGS);
    expect(cycles.slice(0, 3)).toEqual(["6.12", "6.13", "6.14"]);
    expect(cycles.at(-1)).toBe("7.4");
    const rc = parseVersion("7.4-rc3")!;
    const stable = parseVersion("7.3.4")!;
    const at = (v: string) => distance(parseVersion(v)!, rc, stable, cycles);
    expect(at("7.4-rc3")).toMatchObject({ cyclesBehindRc: 0, rcSteps: 0, relation: "mainline-rc" });
    expect(at("7.4-rc1")).toMatchObject({ cyclesBehindRc: 0, rcSteps: 2 });
    expect(at("7.3.2")).toMatchObject({ cyclesBehindRc: 1, patchesBehindStable: 2, relation: "latest-stable-cycle" });
    expect(at("6.12.117")).toMatchObject({ cyclesBehindRc: 12, cyclesBehindStable: 11, relation: "older" });
    expect(closenessKey(at("7.4-rc1"))).toEqual([0, 2, 0]);
  });
});

describe("poller", () => {
  test("ingests every adapter shape from the fixtures", async () => {
    const manifest = testManifest();
    const store = new Store(":memory:");
    const summary = await poll(store, manifest, { now: () => 1_000_000, env: {} });
    expect(summary.results.filter((r) => r.outcome === "failed")).toEqual([]);
    expect(summary.polled).toBe(manifest.sources.length);
    const kinds = (kernel: string, kind: string): number => store.items({ kernel, kind }).length;
    expect(kinds("linux", "commit")).toBe(3);
    expect(kinds("linux", "release")).toBeGreaterThan(3);
    expect(kinds("linux", "tag")).toBe(TAGS.length);
    expect(kinds("asterinas", "release")).toBe(3);
    expect(kinds("asterinas", "issue")).toBeGreaterThanOrEqual(3);
    expect(store.items({ kernel: "asterinas", kind: "issue" }).every((i) => i.meta?.state)).toBe(true);
    expect(kinds("redox", "pr")).toBe(3);
    expect(kinds("redox", "page")).toBe(3);
    expect(kinds("ubuntu", "package")).toBe(3);
    const commit = store.items({ kernel: "linux", kind: "commit" })[0]!;
    expect(commit.url).toMatch(/^https?:\/\/git\.kernel\.org\//);
    expect(parseFeed(fixture("cgit-atom.xml"), "commit", "git.kernel.org", 0)[0]!.url).toMatch(/^https:\/\/git\.kernel\.org\//);
    expect(commit.id).toMatch(/^[0-9a-f]{40}$/);
    expect(commit.author.length).toBeGreaterThan(0);
    const metrics = store.metricNames().filter((m) => m.source === "asterinas-counts").map((m) => m.name);
    expect(metrics).toEqual(["open_issues", "open_prs"]);
    store.close();
  });

  test("is idempotent: unchanged sources answer 304 and add nothing", async () => {
    const manifest = testManifest();
    const store = new Store(":memory:");
    let now = 2_000_000;
    await poll(store, manifest, { now: () => now, env: {} });
    const before = store.counts();
    expect((await poll(store, manifest, { now: () => now + 1, env: {} })).polled).toBe(0);
    now += 700;
    const again = await poll(store, manifest, { now: () => now, env: {} });
    expect(again.polled).toBe(manifest.sources.length);
    expect(again.results.every((r) => r.outcome === "not-modified" || r.outcome === "updated")).toBe(true);
    expect(again.results.filter((r) => r.outcome === "updated").every((r) => r.inserted === 0 && r.changed === 0)).toBe(true);
    expect(store.counts().items).toBe(before.items);
    store.close();
  });

  test("backs off on 429 with Retry-After and on 5xx exponentially, then recovers", async () => {
    resetRoutes();
    const manifest = testManifest(`[[source]]\nid = "flaky"\nkernel = "asterinas"\nadapter = "github-releases"\nkind = "forge-api"\nurl = "${base}/flaky"\nyields = ["release"]\nmin_interval = "1m"\n[[source]]\nid = "down"\nkernel = "asterinas"\nadapter = "github-releases"\nkind = "forge-api"\nurl = "${base}/down"\nyields = ["release"]\nmin_interval = "1m"\n`);
    const store = new Store(":memory:");
    failures.set("/flaky", { status: 429, retryAfter: 120 });
    failures.set("/down", { status: 503 });
    const flaky = manifest.sources.find((s) => s.id === "flaky")!;
    const down = manifest.sources.find((s) => s.id === "down")!;
    await pollSource(store, manifest, flaky, { now: () => 1000, env: {} });
    await pollSource(store, manifest, down, { now: () => 1000, env: {} });
    expect(store.state("flaky")).toMatchObject({ fails: 1, lastStatus: 429, nextAllowed: 1120 });
    expect(store.state("down")).toMatchObject({ fails: 1, lastStatus: 503, nextAllowed: 1060 });
    await pollSource(store, manifest, down, { now: () => 1060, env: {} });
    expect(store.state("down")).toMatchObject({ fails: 2, nextAllowed: 1060 + 120 });
    const hitsBefore = hits.get("/flaky") ?? 0;
    await poll(store, manifest, { only: ["flaky"], now: () => 1100, env: {} });
    expect(hits.get("/flaky")).toBe(hitsBefore);
    failures.clear();
    route("/flaky", fixture("github-releases.json"));
    const recovered = await poll(store, manifest, { only: ["flaky"], now: () => 1200, env: {} });
    expect(recovered.results[0]).toMatchObject({ outcome: "updated", inserted: 3 });
    expect(store.state("flaky").fails).toBe(0);
    expect(backoff({ status: 403, headers: new Headers({ "x-ratelimit-remaining": "0", "x-ratelimit-reset": "5000" }) }, 0, 60, 21600, 1000)).toBe(4001);
    expect(backoff(null, 20, 60, 600, 0)).toBe(600);
    resetRoutes();
    store.close();
  });

  test("is bounded and resumable", async () => {
    const manifest = testManifest();
    const store = new Store(":memory:");
    const first = await poll(store, manifest, { maxSources: 4, now: () => 5000, env: {} });
    expect(first.polled).toBe(4);
    expect(first.deferred).toBe(manifest.sources.length - 4);
    const second = await poll(store, manifest, { now: () => 5001, env: {} });
    expect(second.polled).toBe(manifest.sources.length - 4);
    const skipped = await poll(store, manifest, { force: true, budget: 0, now: () => 5002, env: {} });
    expect(skipped.results.every((r) => r.outcome === "skipped")).toBe(true);
    store.close();
  });

  test("sends the token only to the host the manifest names", async () => {
    resetRoutes();
    const manifest = testManifest(`[[source]]\nid = "auth-a"\nkernel = "arch"\nadapter = "json-version"\nkind = "package-index"\nurl = "http://127.0.0.1:${origin.port}/auth/a"\npath = "pkgver"\nchannel = "a"\nyields = ["version"]\n[[source]]\nid = "auth-b"\nkernel = "arch"\nadapter = "json-version"\nkind = "package-index"\nurl = "http://localhost:${origin.port}/auth/b"\npath = "pkgver"\nchannel = "b"\nyields = ["version"]\n`);
    const store = new Store(":memory:");
    await poll(store, manifest, { only: ["auth-a", "auth-b"], now: () => 1, env: { TEST_TOKEN: "s3cret" } });
    expect(seenAuth.get("127.0.0.1/auth/a")).toBe("Bearer s3cret");
    expect(seenAuth.get("localhost/auth/b")).toBeNull();
    seenAuth.clear();
    await poll(store, manifest, { only: ["auth-a"], force: true, now: () => 2, env: {} });
    expect(seenAuth.get("127.0.0.1/auth/a")).toBeNull();
    store.close();
  });

  test("full-text search ranks items and filters by kernel and kind", async () => {
    const store = new Store(":memory:");
    await poll(store, testManifest(), { now: () => 1, env: {} });
    const commit = store.items({ kernel: "asterinas", kind: "commit" })[0]!;
    const word = commit.title.split(/\s+/).sort((a, b) => b.length - a.length)[0]!;
    const hits = store.search(word, { kernel: "asterinas" });
    expect(hits.some((h) => h.id === commit.id)).toBe(true);
    expect(store.search(word, { kernel: "redox" }).every((h) => h.kernel === "redox")).toBe(true);
    expect(store.search("linux", { kind: "release" }).every((h) => h.kind === "release")).toBe(true);
    expect(() => toFtsQuery("  ")).toThrow(/empty/);
    expect(toFtsQuery('a "b" c-d')).toBe('"a" "b" "c-d"*');
    store.close();
  });
});

describe("views", () => {
  async function filled(manifest: Manifest): Promise<Store> {
    const store = new Store(":memory:");
    await poll(store, manifest, { now: () => 1_000_000, env: {} });
    return store;
  }
  const NOW = 1_800_000_000;

  test("derives the versions table from the store, closest to the rc first", async () => {
    const manifest = testManifest();
    const store = await filled(manifest);
    const view = computeVersions(store, manifest, NOW);
    expect(view.mainline).toBe("7.4-rc3");
    expect(view.stable).toBe("7.3.4");
    const byKernel = Object.fromEntries(view.distros.map((d) => [d.kernel, d]));
    expect(byKernel.nixos).toMatchObject({ channel: "testing", cyclesBehindRc: 0, rcSteps: 0 });
    expect(byKernel.ubuntu).toMatchObject({ channel: "mainline-ppa", normalized: "7.4-rc3", cyclesBehindRc: 0 });
    expect(byKernel.fedora).toMatchObject({ cyclesBehindRc: 0 });
    expect(byKernel.arch).toMatchObject({ normalized: "7.3.4", cyclesBehindRc: 1, patchesBehindStable: 0 });
    expect(byKernel.android).toMatchObject({ normalized: "7.3", cyclesBehindRc: 1 });
    expect(byKernel.alpine).toMatchObject({ normalized: "7.3.4", cyclesBehindRc: 1 });
    expect(byKernel.rhel).toMatchObject({ normalized: "6.12", cyclesBehindRc: 12 });
    const keys = view.distros.map((d) => closenessKey(d));
    expect(keys).toEqual([...keys].sort((a, b) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2]));
    expect(view.distros.at(-1)!.kernel).toBe("rhel");
    store.close();
  });

  test("digests count windows, merged PRs, metrics and version changes", async () => {
    resetRoutes();
    const manifest = testManifest();
    const store = new Store(":memory:");
    await poll(store, manifest, { now: () => 1_000_000, env: {} });
    const commits = JSON.parse(fixture("github-commits.json")) as { commit: { committer: { date: string } } }[];
    const newest = Math.max(...commits.map((c) => Date.parse(c.commit.committer.date) / 1000));
    const digest = computeDigests(store, manifest, Math.ceil(newest / 3600) * 3600).find((d) => d.kernel === "asterinas")!;
    const week = digest.windows[1]!;
    expect(week.counts.commit).toBe(commits.filter((c) => Date.parse(c.commit.committer.date) / 1000 >= week.since).length);
    expect(week.metrics.open_prs!.current).toBe(JSON.parse(fixture("github-search.json")).total_count);
    expect(week.commitsLowerBound).toBe(true);

    const nix = JSON.parse(fixture("nix-kernels.json"));
    nix.testing.version = "7.4-rc4";
    route("/nix", JSON.stringify(nix), "text/plain");
    await poll(store, manifest, { only: ["nixos-kernels"], force: true, now: () => 1_000_600, env: {} });
    const changes = computeDigests(store, manifest, 1_003_600).find((d) => d.kernel === "nixos")!.windows[0]!.versionChanges;
    expect(changes).toEqual([{ channel: "testing", version: "7.4-rc4", at: 1_000_600 }]);
    expect(store.versionRows().find((r) => r.channel === "testing")).toMatchObject({ version: "7.4-rc4", since: 1_000_600 });
    resetRoutes();
    store.close();
  });

  test("same store gives byte-identical files whatever the poll order", async () => {
    const manifest = testManifest();
    const reversed: Manifest = { ...manifest, sources: [...manifest.sources].reverse() };
    const a = await filled(manifest);
    const b = await filled(reversed);
    const one = await generate(a, manifest, { outDir: join(scratch, "a"), now: NOW });
    const two = await generate(b, reversed, { outDir: join(scratch, "b"), now: NOW });
    const again = await generate(a, manifest, { outDir: join(scratch, "a"), now: NOW });
    expect([...one.contents.keys()]).toEqual([...two.contents.keys()]);
    for (const [path, text] of one.contents) {
      expect(two.contents.get(path), path).toBe(text);
      expect(again.contents.get(path), path).toBe(text);
      expect(readFileSync(join(scratch, "a", path), "utf8"), path).toBe(text);
    }
    for (const path of ["versions.md", "state-of-kernels.md", "README.md", "sources.md", "kernels/linux.md", "data/versions.json", "data/digests.json", "data/kernels.json", "data/sources.json"]) expect(one.contents.has(path), path).toBe(true);
    JSON.parse(one.contents.get("data/versions.json")!);
    expect(one.contents.get("versions.md")).toContain("| 1 |");
    expect(one.contents.get("state-of-kernels.md")).toContain("Mainline: 7.4-rc3");
    a.close();
    b.close();
  });
});

describe("commit gate", () => {
  const base = { versionsChanged: false, anyChanged: true, lastCommit: 1000, now: 1000 + 600, slowSeconds: 21600 };
  test("version changes commit at once, activity waits, nothing changed skips", () => {
    expect(decideCommit({ ...base, versionsChanged: true })).toBe("commit");
    expect(decideCommit(base)).toBe("skip");
    expect(decideCommit({ ...base, now: 1000 + 21600 })).toBe("commit");
    expect(decideCommit({ ...base, anyChanged: false, versionsChanged: true })).toBe("skip");
    expect(decideCommit({ ...base, lastCommit: null })).toBe("commit");
  });
});
