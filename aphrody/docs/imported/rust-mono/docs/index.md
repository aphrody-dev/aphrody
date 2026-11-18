# rust-mono documentation

One superproject, 51 pinned forks, one sync loop. Read in this order.

| Question | Page |
| :-- | :-- |
| What is in the tree and why is it laid out this way? | [architecture.md](architecture.md) |
| How do forks stay current with upstream? What is the ledger? Which token runs it? | [sync.md](sync.md) |
| I am an agent: where is the code, the issue, the RFC, the commit? | [agents.md](agents.md) |
| What was forked, what was not, and which names changed? | [inventory.md](inventory.md) |
| How will issues, PRs, blog posts and chat reach the agents? (phase 4 design) | [knowledge-feed.md](knowledge-feed.md) |
| How will a Rust toolchain for the owner's PC be built and installed? (phase 5 design) | [toolchain-build.md](toolchain-build.md) |

Machine-readable sources: [forks.toml](../forks.toml) (what), [ledger/](../ledger) (what changed when), [.gitmodules](../.gitmodules) (generated).

Every statement of fact in these pages was read from the live GitHub API or from a repository on 2026-11-17. Anything marked **assumption** was not verified and says what would verify it.
