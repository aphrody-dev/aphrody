// Records trimmed copies of live responses as test fixtures: `bun fixtures/record.ts`.
// Lists keep three entries; personal fields (emails, avatars, node ids) and bulky nested objects are dropped.
import { join } from "node:path";

const DROP = new Set(["email", "avatar_url", "gravatar_id", "node_id", "head", "base", "_links", "labels", "assignees", "assignee", "requested_reviewers", "requested_teams", "milestone", "reactions", "repo", "auto_merge", "active_lock_reason", "closed_by", "performed_via_github_app", "sub_issues_summary", "issue_dependencies_summary", "author_association", "parents", "tree", "verification", "committer_email", "author_email", "pgp", "diffview", "patch"]);
const prune = (value: unknown): unknown => {
  if (Array.isArray(value)) return value.map(prune);
  if (value && typeof value === "object") return Object.fromEntries(Object.entries(value).filter(([key]) => !DROP.has(key) && !/_url$/.test(key) || key === "html_url" || key === "web_url").map(([k, v]) => [k, prune(v)]));
  return value;
};
const head = <T>(list: T[]): T[] => list.slice(0, 3);
const makefileHead = (b64: string): string => Buffer.from(Buffer.from(b64, "base64").toString("utf8").split("\n").slice(0, 8).join("\n") + "\n").toString("base64");
const atomHead = (xml: string): string => {
  const entries = [...xml.matchAll(/<entry[\s\S]*?<\/entry>/g)].slice(0, 3).map((m) => m[0]!.replace(/<email>[^<]*<\/email>/g, ""));
  return `${xml.slice(0, xml.indexOf("<entry"))}${entries.join("\n")}\n</feed>\n`;
};

const GH = "https://api.github.com/repos/asterinas/asterinas";
const jobs: [string, string, (text: string) => string][] = [
  ["github-releases.json", `${GH}/releases?per_page=3`, (t) => JSON.stringify(prune(head(JSON.parse(t))), null, 1)],
  ["github-commits.json", `${GH}/commits?per_page=3`, (t) => JSON.stringify(prune(head(JSON.parse(t))), null, 1)],
  ["github-issues.json", `${GH}/issues?state=all&sort=updated&per_page=30`, (t) => JSON.stringify(prune(head(JSON.parse(t).filter((i: any) => !i.pull_request)).concat(JSON.parse(t).filter((i: any) => i.pull_request).slice(0, 1))), null, 1)],
  ["github-pulls.json", `${GH}/pulls?state=all&sort=updated&per_page=3`, (t) => JSON.stringify(prune(head(JSON.parse(t))), null, 1)],
  ["github-search.json", "https://api.github.com/search/issues?q=repo:asterinas/asterinas+is:open+is:pr&per_page=1", (t) => JSON.stringify({ total_count: JSON.parse(t).total_count, items: [] })],
  ["github-search-issues.json", "https://api.github.com/search/issues?q=repo:asterinas/asterinas+is:open+is:issue&per_page=1", (t) => JSON.stringify({ total_count: JSON.parse(t).total_count, items: [] })],
  ["gitlab-commits.json", "https://gitlab.com/api/v4/projects/redhat%2Fcentos-stream%2Fsrc%2Fkernel%2Fcentos-stream-10/repository/commits?per_page=3", (t) => JSON.stringify(prune(head(JSON.parse(t))), null, 1)],
  ["gitlab-merge-requests.json", "https://gitlab.redox-os.org/api/v4/projects/redox-os%2Fkernel/merge_requests?state=all&order_by=updated_at&per_page=3", (t) => JSON.stringify(prune(head(JSON.parse(t))), null, 1)],
  ["gitlab-tags.json", "https://gitlab.com/api/v4/projects/cki-project%2Fkernel-ark/repository/tags?per_page=3&order_by=updated", (t) => JSON.stringify(prune(head(JSON.parse(t))), null, 1)],
  ["cgit-atom.xml", "https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/atom/?h=master", atomHead],
  ["kernel-releases.json", "https://www.kernel.org/releases.json", (t) => JSON.stringify(JSON.parse(t), null, 1)],
  ["arch-linux.json", "https://archlinux.org/packages/core/x86_64/linux/json/", (t) => JSON.stringify({ pkgname: JSON.parse(t).pkgname, pkgver: JSON.parse(t).pkgver, last_update: JSON.parse(t).last_update })],
  ["bodhi.json", "https://bodhi.fedoraproject.org/updates/?packages=kernel&status=stable&rows_per_page=3", (t) => JSON.stringify({ updates: JSON.parse(t).updates.map((u: any) => ({ alias: u.alias, title: u.title, status: u.status, url: u.url, date_stable: u.date_stable, release: { name: u.release.name }, builds: u.builds.slice(0, 1).map((b: any) => ({ nvr: b.nvr })), notes: "", user: { name: u.user?.name } })) }, null, 1)],
  ["nix-kernels.json", "https://raw.githubusercontent.com/NixOS/nixpkgs/master/pkgs/os-specific/linux/kernel/kernels-org.json", (t) => t],
  ["makefile.b64", "https://android.googlesource.com/kernel/common/+/refs/heads/android-mainline/Makefile?format=TEXT", makefileHead],
  ["apkbuild.txt", "https://gitlab.alpinelinux.org/api/v4/projects/alpine%2Faports/repository/files/main%2Flinux-lts%2FAPKBUILD/raw?ref=master", (t) => t.split("\n").slice(0, 6).join("\n") + "\n"],
  ["ubuntu-mainline.html", "https://kernel.ubuntu.com/mainline/", (t) => [...t.matchAll(/<tr><td valign="top"><img[^\n]*?href="v[^\n]*\n?/g)].slice(-4).map((m) => m[0]).join("")],
  ["launchpad.json", "https://api.launchpad.net/devel/ubuntu/+archive/primary?ws.op=getPublishedSources&source_name=linux&exact_match=true&order_by_date=true&ws.size=3", (t) => JSON.stringify({ entries: JSON.parse(t).entries.map((e: any) => ({ source_package_name: e.source_package_name, source_package_version: e.source_package_version, distro_series_link: e.distro_series_link, pocket: e.pocket, status: e.status, date_published: e.date_published, self_link: e.self_link })) }, null, 1)],
  ["redox-sitemap.xml", "https://www.redox-os.org/sitemap.xml", (t) => `<urlset>${[...t.matchAll(/<url>[\s\S]*?<\/url>/g)].slice(0, 3).map((m) => m[0]).join("")}</urlset>`],
];
for (const [name, url, transform] of jobs) {
  const res = await fetch(url, { headers: { "user-agent": "kernel-watch" } });
  if (!res.ok) throw new Error(`${name}: ${url} answered ${res.status}`);
  await Bun.write(join(import.meta.dir, name), `${transform(await res.text()).replace(/[\w.+-]+@[\w-]+(?:\.[\w-]+)+/g, "redacted@example.invalid").trimEnd()}\n`);
  console.log(`${res.status} ${name}`);
}
