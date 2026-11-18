# kernel-watch

Follows Linux, Asterinas, Redox, LiberNT and the distribution kernels closest to the mainline rc. Bun built-ins only (fetch, bun:sqlite with FTS5, Bun.TOML, Bun.semver, Bun.serve in tests); no npm dependencies.

```sh
cd tooling/kernel-watch
bun cli.ts validate            # check sources.toml
bun cli.ts poll                # conditional, bounded, resumable; GITHUB_TOKEN or GH_TOKEN is used when set
bun cli.ts versions            # kernel versions table, closest to mainline first
bun cli.ts search "io_uring" --kernel linux
bun cli.ts status              # per-source poll state, failures, next allowed poll
bun cli.ts build               # regenerate ../../aphrody/docs/watch from the store
bun cli.ts verify --write      # request every URL in the manifest live and record status and shape
bun graph.ts clone|report      # structure reports; heavy, build host only (see below)
bun test                       # hermetic: local Bun.serve replays fixtures/
```

- `sources.toml` is the only place that names a host, URL or organisation.
- The store is `.store/watch.sqlite` (git-ignored), or `$WATCH_STORE`. In CI it lives in the Actions cache.
- `fixtures/record.ts` re-records the trimmed test fixtures from live endpoints; `fixtures/verified.json` is written by `verify --write`.
- Generated output (`aphrody/docs/watch`) is byte-identical for the same store and `--now`.
