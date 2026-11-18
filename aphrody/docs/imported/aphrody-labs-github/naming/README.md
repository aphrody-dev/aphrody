# naming

Naming scheme and independence rules of the Aphrody organisations (aphrody-dev, aphrody-labs, rose-griffon).

- [NAMING.md](NAMING.md): the rules, per org and per repository kind, per registry; independence rules.
- [naming.toml](naming.toml): the same, machine-readable, plus the registry of known repositories and the legacy-name map.
- [check.ts](check.ts) with [check.test.ts](check.test.ts): checks one repository (`bun naming/check.ts <dir>`), never blocking unless `--strict`.
- [repos.ts](repos.ts): audits GitHub repository names live (`bun naming/repos.ts`).

Note for STANDARD.md and `tools/apply.ts`: link to `naming/NAMING.md` for repository, package and environment names, and to the independence rules for CI. Nothing in `standard.toml` needs to copy a name from here; a repository's own `naming.toml` is the single source of its names.
