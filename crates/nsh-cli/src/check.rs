//! `nsh check`, `nsh fmt`, and `nsh lint`: powered directly by Bun's native
//! multi-core TypeScript checker (`bun check`), formatter (`bun fmt`, including
//! Tailwind CSS class sorting), and linter (`bun lint`), with zero npm tarball
//! downloads and zero external Node/TypeScript dependencies.
//!
//! An app with Rust crates of its own (`nsh.json` `"rust"`) has its crates
//! checked first, each with `cargo check --all-targets` in its directory
//! ([`cargo_check`]): a Rust app (`"root"`) runs only that unless it has a
//! `tsconfig.json` (for its TypeScript tests); a mixed app runs both.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context as _, bail};

use crate::rust_app::RustApp;

/// Resolves the `bun` executable (`explicit`, else `NSH_BUN`, else `bun` on `PATH`).
pub(crate) fn bun_executable(explicit: Option<&Path>) -> OsString {
    if let Some(path) = explicit {
        return path.as_os_str().to_owned();
    }
    if let Some(env_bun) = env::var_os("NSH_BUN")
        && !env_bun.is_empty()
    {
        return env_bun;
    }
    OsString::from("bun")
}

/// Type checks the app of `entry` using `bun check --no-script -p <dir>`
/// (with `.nsh/types` and `.nsh/tsconfig.json` written beforehand) and fails
/// on type errors.
pub(crate) fn check(entry: &Path, bun: Option<&Path>, args: &[String]) -> anyhow::Result<()> {
    let dir = dunce::canonicalize(entry)?
        .parent()
        .context("entry has no directory")?
        .to_path_buf();
    if !dir.join("tsconfig.json").exists() {
        bail!(
            "{} has no tsconfig.json: `nsh init {}` adds one",
            dir.display(),
            dir.display()
        );
    }
    if !crate::init::tsconfig_extends_nsh(&dir) {
        crate::ui::warn(format!(
            "{} does not extend ./.nsh/tsconfig.json, where nsh keeps its TypeScript settings \
             (`lib`, the paths of `nsh` and the app's crates, the globals) current: add \
             `\"extends\": \"./.nsh/tsconfig.json\"` and drop the options it sets, as the \
             tsconfig.json of `nsh init` does",
            crate::ui::path(&dir.join("tsconfig.json"))
        ));
    }
    let exe = bun_executable(bun);
    let status = Command::new(&exe)
        .arg("check")
        .arg("--no-script")
        .arg("-p")
        .arg(&dir)
        .args(args)
        .status()
        .with_context(|| format!("run {} check (is bun installed on PATH?)", exe.to_string_lossy()))?;
    if !status.success() {
        bail!("type check failed ({status})");
    }
    log::info!("no type errors in {}", dir.display());
    Ok(())
}

/// Formats an app directory or specific files with `bun fmt` (formatting TS, TSX,
/// JSON, Markdown, and sorting imports and Tailwind CSS classes).
pub(crate) fn fmt(
    target_path: &Path,
    check_only: bool,
    bun: Option<&Path>,
    args: &[String],
) -> anyhow::Result<()> {
    let resolved = dunce::canonicalize(target_path)
        .with_context(|| format!("resolve {}", target_path.display()))?;
    let cwd = if resolved.is_dir() {
        resolved.clone()
    } else {
        resolved
            .parent()
            .context("target has no parent directory")?
            .to_path_buf()
    };
    let exe = bun_executable(bun);
    let mut cmd = Command::new(&exe);
    cmd.arg("fmt").current_dir(&cwd);
    if check_only {
        cmd.arg("--check");
    }
    if resolved.is_file() {
        cmd.arg(&resolved);
    }
    cmd.args(args);
    let status = cmd
        .status()
        .with_context(|| format!("run {} fmt", exe.to_string_lossy()))?;
    if !status.success() {
        bail!("bun fmt failed ({status})");
    }
    Ok(())
}

/// Lints an app directory or specific files with `bun lint` (1,000+ built-in
/// TypeScript/JSX rules with zero npm dependencies).
pub(crate) fn lint(
    target_path: &Path,
    fix: bool,
    bun: Option<&Path>,
    args: &[String],
) -> anyhow::Result<()> {
    let resolved = dunce::canonicalize(target_path)
        .with_context(|| format!("resolve {}", target_path.display()))?;
    let cwd = if resolved.is_dir() {
        resolved.clone()
    } else {
        resolved
            .parent()
            .context("target has no parent directory")?
            .to_path_buf()
    };
    let exe = bun_executable(bun);
    let mut cmd = Command::new(&exe);
    cmd.arg("lint").current_dir(&cwd);
    if fix {
        cmd.arg("--fix");
    }
    if resolved.is_file() {
        cmd.arg(&resolved);
    }
    cmd.args(args);
    let status = cmd
        .status()
        .with_context(|| format!("run {} lint", exe.to_string_lossy()))?;
    if !status.success() {
        bail!("bun lint failed ({status})");
    }
    Ok(())
}

/// Checks the crates of `app` with `cargo check --all-targets` (their
/// tests and examples too), each in its own directory, so it builds in the
/// workspace and target directory `cargo` there uses; fails on the first
/// with errors, showing cargo's output.
pub(crate) fn cargo_check(app: &RustApp) -> anyhow::Result<()> {
    for krate in app.crates() {
        let mut cmd = krate.cargo("check");
        cmd.arg("--all-targets");
        crate::ui::cargo_as(&mut cmd, ["checking", "checked"], krate.package())?;
    }
    Ok(())
}

/// Whether the app of `entry` has TypeScript to check: a TSX entry's app,
/// or a Rust app (`nsh.json` `"root"`) with a `tsconfig.json` for its test
/// files.
pub(crate) fn has_typescript(entry: &Path) -> bool {
    !crate::rust_app::is_root(entry)
        || entry
            .parent()
            .is_some_and(|dir| dir.join("tsconfig.json").is_file())
}

/// nsh's cache: `NSH_CACHE`, else the platform's cache directory.
pub(crate) fn cache() -> anyhow::Result<PathBuf> {
    if let Some(dir) = env::var_os("NSH_CACHE") {
        return Ok(PathBuf::from(dir));
    }
    let home = env::var_os("HOME").map(PathBuf::from);
    let base = if cfg!(windows) {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library/Caches"))
    } else {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".cache")))
    };
    Ok(base
        .context("no cache directory: set NSH_CACHE")?
        .join("nsh"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn explicit_bun_path_wins_over_default() {
        let custom = Path::new("/opt/bun/bin/bun");
        assert_eq!(super::bun_executable(Some(custom)), custom.as_os_str());
    }
}
