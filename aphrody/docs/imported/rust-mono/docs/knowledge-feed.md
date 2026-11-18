# Knowledge feed (phase 4 design, not implemented)

Goal: every agent can answer "what is happening in Rust" (issues, pull requests, RFCs, releases, blog posts, newsletters, chat) from one local store, minutes after it happens, without each agent crawling the web.

Nothing below is built. **Verified** means observed on 2026-11-17 with a real request; **assumption** means not tested.

## Sources

| Source | Access | Status |
| :-- | :-- | :-- |
| Issues and pull requests of every fork's upstream | `GET /repos/{o}/{r}/issues?state=all&since=<ts>&sort=updated` (issues and PRs together), conditional requests with `If-None-Match` (a 304 is free against the rate limit) | verified: 200 with an `ETag`, 5000 requests/hour authenticated |
| Comments and review comments | `GET /repos/{o}/{r}/issues/comments?since=`, `/pulls/comments?since=` | assumption (same family as above) |
| Everything in the org in one poll | `GET /orgs/rust-lang/events` (public events), `X-Poll-Interval: 60`, `ETag` | verified: 200, 60 s interval, ETag; the events API keeps only recent events (assumption: 300 events or 90 days per GitHub docs), so it is the low-latency tripwire and `since=` polling is the complete record |
| Releases | `GET /repos/rust-lang/rust/releases`; `RELEASES.md` in rust-lang/rust (944 KB, verified); `static.rust-lang.org/dist/channel-rust-{stable,beta,nightly}.toml` | stable channel file verified (200); latest stable 1.100.0 verified |
| RFCs | the `rfcs` fork's `text/` directory (648 entries, verified) plus PR threads of rust-lang/rfcs; the submodule pin already carries new RFC text after each sync | verified |
| Blog, Inside Rust, This Week in Rust | RSS/Atom: `blog.rust-lang.org/feed.xml`, `blog.rust-lang.org/inside-rust/feed.xml`, `this-week-in-rust.org/rss.xml` | all three verified: 200 `application/xml` |
| Governance | the `team`, `goals`, `compiler-team` forks (files under the pins); project-goal tracking issues through the issues API | verified as repositories |
| Zulip (rust-lang.zulipchat.com) | Zulip REST API, `GET /api/v1/messages` with a narrow per channel | server reachable and `realm_web_public_access_enabled` is true (verified), but an anonymous read of the `web-public` narrow returned `UNAUTHORIZED` (verified). Needs an API key of a bot or user on that realm: an owner step, not created here. Fallback: none that respects the service. |
| Upstream commits | already covered by the sync: `compare` per fork gives the commit list, `merge-upstream` merges them | verified in phase 3 |
| Crates.io | the sparse index `index.crates.io` (verified: answers) and the registry API; only for dependency questions | assumption on the API shape |

**Webhooks are not available**: they need admin on the upstream repository (rust-lang, bytecodealliance, llvm). Polling is the only option; webhooks would only report events of our own forks, which carry no discussion.

## Store

SQLite with FTS5 first, one file per source family under a data directory that is not in this repository (the repository stays code and manifests):

- `items(source, id, kind, repo, number, title, body, author, state, labels, created, updated, url, etag)`; primary key `(source, id)`; upsert on `updated`.
- `items_fts` (FTS5, `porter unicode61`) over title, body and comments, with `bm25` ranking and a `repo` / `kind` / date filter.
- `cursors(source, key, since, etag, polled)`: one row per feed, written after the batch commits, so a crash replays instead of skipping.

Optional tiers, only when measurements ask for them: Postgres (`pg_trgm` plus `tsvector`) when several writers or hosts need the same store; Redis for the "latest N events" stream agents tail; S3-compatible storage (R2) for raw JSON snapshots and large attachments (diffs, RFC revisions). None is needed to start: the whole Rust org history of issues and PRs is millions of small rows, well inside one SQLite file (assumption: size estimate; measure on the first full backfill).

## Cadence

- Org events: every 60 s (the server's `X-Poll-Interval`), always with `If-None-Match`.
- `issues?since=` per repository: every 5 minutes for the 20 busiest repositories (rust, cargo, rust-analyzer, rust-clippy, rfcs, rustc-perf, crater, bors, triagebot, ...), every 30 minutes for the rest. Budget: 51 repositories x 12 polls/hour is well under 5000/hour, and most answers are 304.
- Feeds (blog, Inside Rust, TWiR): every 30 minutes with `ETag` / `If-Modified-Since`.
- Zulip: long-poll event queue (`/api/v1/events`) when a key exists; otherwise off.
- Backfill: once, per repository, paged `issues?state=all` (assumption: hours at 5000 requests/hour for rust-lang/rust, which has hundreds of thousands of items; run it on the fr2 research host, not on a workstation).

Runs as a scheduled job on fr2 (research) writing to its own store, not as a GitHub Actions workflow: the poll interval of the events API is shorter than Actions' five-minute cron minimum, and the data is not repository state.

## How an agent queries it

A small read-only CLI and the same functions over a local HTTP endpoint, both Bun:

```sh
rust-feed search "async drop" --repo rust-lang/rust --kind pr --since 30d
rust-feed show rust-lang/rust#12345
rust-feed latest --repo rust-lang/rfcs --limit 20
rust-feed releases
```

Answers carry the canonical URL and the pinned commit of the fork (from `forks.toml` and the gitlink) so an agent can read the code the discussion is about. Agents never write to upstream through it: the store is read-only by construction, which matches the rule of no pull request, issue or comment on an upstream.

## Open points

1. A Zulip API key for the rust-lang realm (owner action).
2. Whether to also index the text of the books and RFC repositories in the same FTS store: likely yes, cheap, driven by the pins.
3. Retention and the exact size of the backfill: measure first.
