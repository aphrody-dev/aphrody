# Upstream sync

## What runs

`bun tools/sync.ts` (dry run) and `bun tools/sync.ts --apply [--commit]`. For every fork in `forks.toml`:

1. **Observe.** `GET repos/<org>/<fork>` (is a fork, parent equals `upstream`, default branch equals the manifest), `GET .../branches/<branch>` (head), `GET .../compare/<upstream-owner>:<branch>...<org>:<branch>` (ahead and behind). Any disagreement is status `error` and the fork is left alone.
2. **Merge.** If the fork is behind: `POST repos/<org>/<fork>/merge-upstream {"branch": "<branch>"}`.
   - fork carries no commits: GitHub fast-forwards (`merge_type: fast-forward`), status `fast-forward`;
   - fork carries commits and the merge is clean: GitHub makes a merge commit (`merge_type: merge`), status `merged`;
   - fork carries commits and upstream conflicts (HTTP 409): nothing is written, status `conflict`;
   - fork carries commits and `policy = "ff-only"` in `forks.toml`: no call is made, status `diverged`.
3. **Pin.** The gitlink of `<group>/<name>` moves to the fork branch's head, straight in the index (`git update-index --add --cacheinfo 160000,<sha>,<path>`): no submodule is cloned. If the fork moved without a merge (new commits pushed to it), the status is `pin`.
4. **Record.** One JSON line per non-trivial fork is appended to `ledger/<YYYY-MM>.jsonl`:

   ```json
   {"ts":"2026-11-17T12:00:00.000Z","fork":"devtools/cargo","repo":"aphrody-labs/cargo","upstream":"rust-lang/cargo","branch":"master","old":"<sha>","new":"<sha>","merged":86,"ahead":0,"behind":86,"status":"fast-forward","message":"..."}
   ```

   `old` and `new` are the gitlink before and after, `merged` the upstream commits merged by this run, `ahead` and `behind` the divergence measured before the merge. `current` (nothing to do) is never written, and an entry identical to the fork's newest one (same status, pins, ahead, behind) is skipped: a conflict that persists is recorded once, so a second run with nothing new writes nothing.
5. **Commit** (`--commit`): one commit, `chore(sync): move N fork pins to upstream`, listing every fork. The tool refuses to start when the index already has staged paths, so the commit holds only gitlinks and the ledger.

Exit status is 1 only when a fork is in `error`; conflicts and diverged forks are recorded and exit 0.

## Policy per fork

`merge` (default) is the behaviour above. `ff-only` never merges into a fork that carries commits; it exists for forks that are worked on directly. `consumers/bun` is `ff-only`: the owner edits it daily (thousands of commits ahead) and an unattended merge commit on its main could land code nobody built. A diverged fork is reported, not forced.

## Why it runs here and not in the forks

The organization standard disables Actions on forks. The sync therefore runs from this repository: [.github/workflows/sync.yml](../.github/workflows/sync.yml), every six hours (`23 */6 * * *`) and on `workflow_dispatch`. It tests the tools (`bun test tools`), runs `tools/sync.ts --apply --commit`, then pushes to `main` (fetch, rebase, push; the Push step runs even when the sync step reports an error on one fork, so the other forks' bumps are not lost).

## Which token

The organization already provides what a cross-repository write needs, read with `gh api` on 2026-11-17:

| Name | Kind | Visibility |
| :-- | :-- | :-- |
| `APHRODY_APP_CLIENT_ID` | org variable | all |
| `APHRODY_APP_KEY` | org secret (private key of the org's own GitHub App `aphrody-labs`) | selected repositories |

The app is installed on all repositories of the organization with `contents: write` and `workflows: write`, which is what `merge-upstream` needs on a fork and what a push to this repository's `main` needs. The workflow mints an installation token with `actions/create-github-app-token` (pinned to a SHA), uses it for `GH_TOKEN` and for the push, and never writes a secret to disk or to the log. `APHRODY_APP_KEY` is visible to selected repositories only, so this repository had to be added to its list (`PUT /orgs/<org>/actions/secrets/APHRODY_APP_KEY/repositories/<id>`).

### Current state of the token (2026-11-17)

The first run of the workflow (run 44019759711, `workflow_dispatch`) failed in the step "Token for the forks and this repository" with `Invalid keyData` / `Failed to read private key`: the value stored in the org secret `APHRODY_APP_KEY` is not a PEM private key that `actions/create-github-app-token` can import. Nothing after that step ran on the runner, so the Sync and Push steps have not been exercised there. The variable `APHRODY_APP_CLIENT_ID` and the app installation are fine (the step reaches the key import). The key is not in the vault of the workstation either (`secretHas("APHRODY_APP_KEY")` is false), so it cannot be re-published from there. The same value explains the "not a usable private key" warning in `nie`'s vendor-sync workflow.

What is missing, exactly: a valid PEM private key of the GitHub App `aphrody-labs` stored as the org secret `APHRODY_APP_KEY`, visible to `legacy`, `nie` and `rust-mono`. Generating one is an owner action on the app's settings page (Generate a private key), then `gh secret set APHRODY_APP_KEY --org aphrody-labs --visibility selected --repos legacy,nie,rust-mono < key.pem`. No workflow change is needed after that: re-run `gh workflow run sync.yml -f apply=true`.

Until then the sync runs from a workstation with `bun tools/sync.ts --apply --commit` and a push, which is how the first real run was done (ledger entries of 2026-11-17T04:47Z).

## Failure modes

| Symptom | Cause | What to do |
| :-- | :-- | :-- |
| status `conflict` in the ledger | the fork carries commits that conflict with upstream | resolve in the fork on a machine that can build, push, the next run records the pin |
| status `error`, "parent is X" | the upstream was transferred or the fork re-created | fix `upstream` in `forks.toml` |
| status `error`, "branch ... does not exist" | upstream renamed its branch (rust-lang/llvm-project moves to a new `rustc/*` branch) | change `branch` in `forks.toml` |
| `merge-upstream HTTP 403` | the app lost access to the fork | reinstall the app on the repository |
| step "Token for the forks" fails | `APHRODY_APP_KEY` not shared with this repository, or the key is invalid | `gh secret` / org settings; the workflow cannot invent a credential |

## Tests

`bun test tools` replays recorded sessions (`tools/fixtures/`) and runs the sync against a stateful fake of the API in a temporary git repository: fast-forward, merge, conflict, diverged, pin-only, current, error, idempotence and the commit. No network, no build.
