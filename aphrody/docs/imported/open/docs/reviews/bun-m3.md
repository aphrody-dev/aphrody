# Review: apps/bun-m3

## Gemini Carbon, on d476776ff and f98b08dab

Score 9.5/20, "needs major work". Each point checked against the code; fixed in bdd531b08 unless stated.

| Point | Checked | Outcome |
|---|---|---|
| The "sandbox" is a lexical filter plus a temporary HOME: full file system, network and process access | true | Linux runs go through bubblewrap (read-only root, private `/tmp` and home, own PID, IPC and UTS namespaces, dies with the server). Windows runs are in a Job Object; reads of the user's files and network access stay possible there |
| `TEMP`/`TMP` inherited from the host; `APPDATA`/`LOCALAPPDATA` missing | true | all of them, and the XDG directories, point inside the working directory |
| `taskkill /T` rebuilds the tree from parent PIDs after the fact (PID reuse, orphans) | true | one Job Object per run with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, terminated on timeout and closed after every run |
| Coverage over-attribution: reads, inspection and `then` lookups count as use | true | calls are recorded apart from reads; a function counts only when called, a class when constructed or a static member is called, a value when read; protocol lookups are ignored |
| Proxies reach native code as arguments | true | receivers and arguments are unwrapped before every call and construction |
| Concurrent writes of the coverage JSON by child processes | true | append-only NDJSON |
| `bun pm trust` and `--trust` not skipped | true | skipped |
| Named files collide when a page shows two versions of one file | true | before each example, every named file is written in the version the page has at that point |
| Startup refuses when a `.git` exists above the temp directory | true | the check covers what Bun's resolution reads: `package.json`, `bunfig.toml`, `node_modules` |
| Servers that never stop count as timeouts | true | a run that started `Bun.serve` or `Bun.listen` and reaches the timeout without error is `serving` |
| `linkSync` fails across drives | handled before | falls back to `copyFileSync` |
| Angular in JIT ships the compiler and needs `unsafe-eval` under a CSP | true, open | Angular AOT needs the TypeScript 6 API (angular/angular#69704); no TypeScript 6 is shipped |
| The M3 count only proves host elements | partly | overlays (dialog, snackbar, bottom sheet, menu, date and time pickers) are counted only after they were opened by a user action in the session |
| 1,422 skills crowd an agent's skill routing | design | one skill per page is the requirement; routing by section is a client concern |
| `api.json` depends on the OS that generated it | true, open | Windows and Linux both list 407 entries on 1.4.5; the file records the Bun revision |
