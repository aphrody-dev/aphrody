import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ARCH_DOC_REL,
  buildDirectoryTree,
  buildEcosystemDocContent,
  buildLlmsTxtContent,
  buildRouteTree,
  buildSkillMdContent,
  buildXhArgs,
  escapeXml,
  extractHtmlPageInfo,
  findBrokenRelativeLinks,
  formatGitIngestDigest,
  LLMS_TXT_REL,
  normalizeCrawlUrl,
  type OrgIngestManifest,
  parsePaginatedJson,
  parseSphinxSearchIndex,
  parseXhResponseStatus,
  probeEmcc,
  REF_BEST_PRACTICES_REL,
  REF_REPOS_REL,
  REF_SITEMAP_REL,
  REF_WEBGPU_INTEROP_REL,
  REPO_ROOT,
  serializeSitemapXml,
  type SitemapCrawlReport,
  SKILL_MD_REL,
  validateLlmsTxt,
  validateSkillFrontmatter,
} from "./emscripten-crawl.ts";

describe("emscripten-crawl pure parsers, serializers & validators", () => {
  test("parseSphinxSearchIndex extracts docnames, titles, filenames, and routes", () => {
    const sampleJs = `Search.setIndex({"docnames":["index","docs/tools_reference/emcc","docs/porting/asyncify"],"titles":["Emscripten &amp; Docs","emcc CLI","Asynchronous <code>Code</code>"],"filenames":["index.rst","docs/tools_reference/emcc.rst","docs/porting/asyncify.rst"]})`;
    const parsed = parseSphinxSearchIndex(sampleJs, "https://emscripten.org");
    expect(parsed.docnames).toEqual(["index", "docs/tools_reference/emcc", "docs/porting/asyncify"]);
    expect(parsed.titles).toEqual(["Emscripten & Docs", "emcc CLI", "Asynchronous Code"]);
    expect(parsed.routes.map((r) => r.url)).toEqual([
      "https://emscripten.org/index.html",
      "https://emscripten.org/docs/tools_reference/emcc.html",
      "https://emscripten.org/docs/porting/asyncify.html",
    ]);
  });

  test("normalizeCrawlUrl and extractHtmlPageInfo separate internal HTML, assets, fragments, and external links", () => {
    const html = `<!doctype html>
<html>
<head>
  <title>Best Practices — Emscripten 7.0.1-git (dev) documentation</title>
  <link rel="stylesheet" href="../../_static/shibuya.css" />
  <link rel="canonical" href="https://emscripten.org/docs/compiling/best_practices.html" />
</head>
<body>
  <h1>Best Practices<a class="headerlink" href="#best-practices">¶</a></h1>
  <a href="Modularized-Output.html#using-smodularize">Modularized</a>
  <a href="../porting/pthreads.html">Pthreads</a>
  <a href="../../_static/logo.svg">Logo</a>
  <a href="mailto:team@example.org">Mail</a>
  <a href="https://github.com/emscripten-core/emscripten">GitHub</a>
</body>
</html>`;
    const pageUrl = "https://emscripten.org/docs/compiling/best_practices.html";
    const info = extractHtmlPageInfo(html, pageUrl);

    expect(info.title).toBe("Best Practices — Emscripten 7.0.1-git (dev) documentation");
    expect(info.h1).toBe("Best Practices");
    expect(info.docsVersion).toBe("7.0.1-git");
    expect(info.canonicalUrl).toBe("https://emscripten.org/docs/compiling/best_practices.html");
    expect(info.internalHtmlLinks).toEqual([
      "https://emscripten.org/docs/compiling/Modularized-Output.html",
      "https://emscripten.org/docs/porting/pthreads.html",
    ]);
    expect(info.internalAssetLinks).toEqual(["https://emscripten.org/_static/logo.svg"]);
    expect(info.externalLinks).toEqual(["https://github.com/emscripten-core/emscripten"]);
    expect(info.fragmentCount).toBe(1);

    expect(normalizeCrawlUrl("#only-anchor", pageUrl)).toBeNull();
    expect(normalizeCrawlUrl("javascript:void(0)", pageUrl)).toBeNull();
    expect(normalizeCrawlUrl("/", pageUrl)?.normalizedUrl).toBe("https://emscripten.org/index.html");
  });

  test("buildRouteTree groups routes hierarchically and serializeSitemapXml escapes XML cleanly", () => {
    const routes = [
      {
        url: "https://emscripten.org/index.html",
        path: "/index.html",
        title: "Home",
        h1: "Home",
        depth: 0,
        discoveredVia: ["seed" as const],
        crawled: true,
        httpStatus: 200,
        contentType: "text/html",
        outLinksCount: 2,
        externalLinksCount: 1,
        sourceFilename: "index.rst",
      },
      {
        url: "https://emscripten.org/docs/compiling/best_practices.html?a=1&b=2",
        path: "/docs/compiling/best_practices.html",
        title: "Best Practices",
        h1: "Best Practices",
        depth: 1,
        discoveredVia: ["bfs" as const],
        crawled: true,
        httpStatus: 200,
        contentType: "text/html",
        outLinksCount: 3,
        externalLinksCount: 0,
        sourceFilename: "docs/compiling/best_practices.rst",
      },
      // Duplicate URL should be deduplicated in XML
      {
        url: "https://emscripten.org/index.html",
        path: "/index.html",
        title: "Home",
        h1: "Home",
        depth: 0,
        discoveredVia: ["searchindex" as const],
        crawled: true,
        httpStatus: 200,
        contentType: "text/html",
        outLinksCount: 2,
        externalLinksCount: 1,
        sourceFilename: "index.rst",
      },
    ];

    const tree = buildRouteTree(routes);
    expect(tree.routeCount).toBe(3);
    expect(tree.children[0]?.segment).toBe("docs");
    expect(tree.children[0]?.children[0]?.segment).toBe("compiling");

    const xml = serializeSitemapXml(routes, "2026-11-16");
    expect(xml).toContain('<?xml version="1.0" encoding="UTF-8"?>');
    expect(xml).toContain("<loc>https://emscripten.org/docs/compiling/best_practices.html?a=1&amp;b=2</loc>");
    expect(xml.match(/<url>/g)?.length).toBe(2);
    expect(escapeXml(`a&b<c>"d"'e'`)).toBe("a&amp;b&lt;c&gt;&quot;d&quot;&apos;e&apos;");
  });

  test("buildXhArgs and parseXhResponseStatus format flags and parse HTTP status headers", () => {
    const args = buildXhArgs("https://emscripten.org/docs/tools_reference/emcc.html", {
      checkStatus: true,
      follow: true,
      timeoutSec: 15,
    });
    expect(args).toEqual([
      "--ignore-stdin",
      "--check-status",
      "--follow",
      "--timeout=15",
      "--print=hb",
      "GET",
      "https://emscripten.org/docs/tools_reference/emcc.html",
    ]);

    const rawXh = [
      "HTTP/2.0 301 Moved Permanently",
      "location: https://emscripten.org/docs/index.html",
      "",
      "HTTP/2.0 200 OK",
      "content-type: text/html; charset=utf-8",
      "",
      "<html></html>",
    ].join("\r\n");
    const parsed = parseXhResponseStatus(rawXh);
    expect(parsed.httpStatus).toBe(200);
    expect(parsed.statusLine).toBe("HTTP/2.0 200 OK");
    expect(parsed.contentType).toBe("text/html; charset=utf-8");
  });

  test("parsePaginatedJson, buildDirectoryTree, and formatGitIngestDigest emit canonical GitIngest output", () => {
    const paginated = parsePaginatedJson<{ name: string }>('[{"name":"emscripten"}][{"name":"emsdk"}]');
    expect(paginated.map((x) => x.name)).toEqual(["emscripten", "emsdk"]);

    const tree = buildDirectoryTree(["README.md", "src/settings.js", "tools/ports/emdawnwebgpu.py"], "emscripten-core-emscripten");
    expect(tree).toContain("└── emscripten-core-emscripten/");
    expect(tree).toContain("├── src/");
    expect(tree).toContain("└── README.md");

    const digest = formatGitIngestDigest({
      fullName: "emscripten-core/emscripten-ci",
      htmlUrl: "https://github.com/emscripten-core/emscripten-ci",
      defaultBranch: "main",
      commitSha: "02b09b1ef4b36a7030fc0c3fee05c830fc7234ef",
      reason: "unit-test",
      archived: false,
      isFork: false,
      language: "Dockerfile",
      stars: 9,
      treePaths: ["Dockerfile", "README.md"],
      treeTruncated: false,
      files: [{ path: "Dockerfile", content: "FROM ubuntu:24.04\n" }],
    });
    expect(digest).toContain("Repository: emscripten-core/emscripten-ci");
    expect(digest).toContain("Files analyzed: 1");
    expect(digest).toContain("FILE: Dockerfile");
    expect(digest).toContain("FROM ubuntu:24.04");
  });

  test("validateSkillFrontmatter and validateLlmsTxt enforce agentskills.io and llmstxt.org contracts", () => {
    const probe = probeEmcc();
    expect(typeof probe.emcc.present).toBe("boolean");
    expect(probe.platform).toBe(process.platform);

    const mockCrawl: SitemapCrawlReport = {
      crawledAt: "2026-11-16T22:00:00.000Z",
      baseUrl: "https://emscripten.org",
      siteTitle: "Emscripten 7.0.1-git (dev) documentation",
      emscriptenDocsVersion: "7.0.1-git",
      maxPagesCap: 80,
      concurrency: 6,
      sitemapXmlProbe: {
        url: "https://emscripten.org/sitemap.xml",
        args: [],
        exitCode: 0,
        ok: false,
        httpStatus: 503,
        statusLine: "HTTP/2.0 503 Service Unavailable",
        contentType: "text/html",
        bytes: 1024,
        elapsedMs: 120,
      },
      searchIndexProbe: {
        url: "https://emscripten.org/searchindex.js",
        httpStatus: 200,
        docCount: 90,
        titleCount: 90,
      },
      xhProbes: [],
      stats: {
        totalDiscoveredRoutes: 93,
        crawledHtmlPages: 80,
        okPages: 80,
        failedPages: 0,
        uncrawledQueuedRoutes: 13,
        totalStaticAssetsDiscovered: 10,
        uniqueExternalDomains: 5,
        totalExternalLinks: 20,
      },
      externalDomains: [],
      routeTree: { segment: "/", fullPath: "/", routeCount: 0, crawledCount: 0, routes: [], children: [] },
      routes: [],
    };
    const mockManifest: OrgIngestManifest = {
      ingestedAt: "2026-11-16T22:00:00.000Z",
      org: "emscripten-core",
      repoCount: 12,
      activeCount: 9,
      archivedCount: 3,
      forkCount: 3,
      gitingestApiProbe: {
        url: "https://gitingest.com/health",
        args: [],
        exitCode: 0,
        ok: true,
        httpStatus: 200,
        statusLine: "HTTP/2.0 200 OK",
        contentType: "application/json",
        bytes: 15,
        elapsedMs: 80,
      },
      repos: [
        {
          name: "emscripten",
          fullName: "emscripten-core/emscripten",
          htmlUrl: "https://github.com/emscripten-core/emscripten",
          description: "Emscripten: An LLVM-to-WebAssembly Compiler",
          homepage: "https://emscripten.org",
          language: "C++",
          stars: 27780,
          forks: 3593,
          openIssues: 1200,
          archived: false,
          isFork: false,
          defaultBranch: "main",
          pushedAt: "2026-11-16T22:00:00Z",
          sizeKb: 276522,
          licenseSpdx: "MIT",
          topics: [],
          ingestMethod: "gitingest-api",
          gitingestHttpStatus: 200,
          digestUrl: null,
          commitSha: "6102fc295451",
          filesAnalyzed: 10000,
          treeEntryCount: 10000,
          treeTruncated: false,
          estimatedTokens: "25.7M",
          digestPath: "target/gitingest/emscripten-core/emscripten.txt",
          digestBytes: 1024,
          roleSummary: "Core compiler",
        },
      ],
    };

    const llms = buildLlmsTxtContent(mockCrawl, mockManifest);
    const llmsValid = validateLlmsTxt(llms);
    expect(llmsValid.ok).toBe(true);
    expect(llmsValid.h1Count).toBe(1);
    expect(llmsValid.hasBlockquote).toBe(true);

    const skill = buildSkillMdContent(mockCrawl, mockManifest, probe);
    const skillValid = validateSkillFrontmatter(skill);
    expect(skillValid.ok).toBe(true);
    expect(skillValid.name).toBe("emscripten");
    expect(skillValid.descriptionLength).toBeLessThanOrEqual(1024);

    const eco = buildEcosystemDocContent(mockCrawl, mockManifest, probe);
    expect(eco).toContain("--use-port=emdawnwebgpu");
    expect(eco).toContain("wasm32-unknown-unknown");
    expect(eco).toContain("wasm32-wasip2");

    const badSkill = "---\nname: Bad_Name\ndescription: oops\n---\n";
    expect(validateSkillFrontmatter(badSkill).ok).toBe(false);
  });

  test("findBrokenRelativeLinks isolates checks cleanly in os.tmpdir() and verifies emitted repo docs", () => {
    const scratch = mkdtempSync(join(tmpdir(), "aphrody-emscripten-links-"));
    try {
      mkdirSync(join(scratch, "docs/references"), { recursive: true });
      writeFileSync(join(scratch, "docs/references/ok.md"), "# OK\n", "utf8");
      writeFileSync(join(scratch, "docs/SKILL.md"), "[ok](references/ok.md)\n[bad](references/missing.md)\n", "utf8");
      const broken = findBrokenRelativeLinks(["docs/SKILL.md"], scratch);
      expect(broken).toEqual(["docs/SKILL.md -> references/missing.md"]);
    } finally {
      rmSync(scratch, { recursive: true, force: true });
    }

    const generatedDocs = [
      ARCH_DOC_REL,
      LLMS_TXT_REL,
      SKILL_MD_REL,
      REF_REPOS_REL,
      REF_SITEMAP_REL,
      REF_BEST_PRACTICES_REL,
      REF_WEBGPU_INTEROP_REL,
    ];
    if (generatedDocs.every((rel) => existsSync(join(REPO_ROOT, rel)))) {
      expect(findBrokenRelativeLinks(generatedDocs, REPO_ROOT)).toEqual([]);
      const skillContent = readFileSync(join(REPO_ROOT, SKILL_MD_REL), "utf8");
      expect(validateSkillFrontmatter(skillContent).ok).toBe(true);
      const llmsContent = readFileSync(join(REPO_ROOT, LLMS_TXT_REL), "utf8");
      expect(validateLlmsTxt(llmsContent).ok).toBe(true);
    }
  });
});
