---
title: Building Aphrody for Linux
description: "Guide to building Aphrody for Linux from source."
---

# Building Aphrody for Linux

## Repository

Clone the [Aphrody repository](https://github.com/aphrody-dev/aphrody) with submodules (or sync submodules with `bun run submodules:sync`).

## Dependencies

- Install [rustup](https://www.rust-lang.org/tools/install)

- Install the necessary system libraries:

  ```sh
  script/linux
  ```

  If you prefer to install the system libraries manually, you can find the list of required packages in the `script/linux` file.

## Building from source

Once the dependencies are installed, you can build Aphrody using [Cargo](https://doc.rust-lang.org/cargo/).

For a debug build of Aphrody Studio:

```sh
cargo run -p zed
```

Or for the local-first editor core (`--no-default-features`) or Aphrody GPUI OS shell (`aphrody_desktop`):

```sh
cargo run -p zed --no-default-features
cargo run -p aphrody_desktop --bin aphrody-os
```

And to run the workspace tests:

```sh
cargo test --workspace
```

In release mode, the primary CLI interface is the `cli` crate. You can run it in development with:

```sh
cargo run -p cli
```

## Installing a development build

You can install a local build on your machine with:

```sh
./script/install-linux
```

This builds `zed` and the `cli` in release mode, installs the binary under `~/.local/bin`, and installs `.desktop` files to `~/.local/share`.

## Wayland & X11

Aphrody supports both X11 and Wayland. By default, we pick whichever we can find at runtime. If you're on Wayland and want to run in X11 mode, use the environment variable `WAYLAND_DISPLAY=''`.

## Notes for packaging Aphrody {#notes-for-packaging-zed}

This section is for distribution maintainers packaging Aphrody.

### Technical requirements

Aphrody has two main studio binaries (plus optional `aphrody-os` and `gpui-wgpu`):

- You will need to build `crates/cli` and make its binary available in `$PATH` (defaulting to `aphrody`, alongside optional compatibility aliases).
- You will need to build `crates/zed` (`--bin aphrody`) and place the studio binary alongside the launcher layout (for example `$PREFIX/libexec/aphrody-editor` or `$PREFIX/lib/aphrody/aphrody-editor`).
- If you are going to provide a `.desktop` file you can find a template in `crates/zed/resources/zed.desktop.in` (synchronized from `product.toml` via `cargo xtask brand apply`), and use `envsubst` to populate it with the values required. Rename it to `$APP_ID.desktop` so it follows FreeDesktop standards and make it executable (`chmod 755`).
- You will need to ensure that the necessary shared libraries are installed (inspect the built binary with `ldd` or check `script/bundle-linux`).
- For an example of a complete Linux bundle script, see [`script/bundle-linux`](https://github.com/aphrody-dev/aphrody/blob/main/script/bundle-linux).
- You can disable auto updates and provide custom update instructions by building or running with `APHRODY_UPDATE_EXPLANATION` (or `ZED_UPDATE_EXPLANATION`), for example: `APHRODY_UPDATE_EXPLANATION="Please use your system package manager to update Aphrody."`.
- Set `crates/zed/RELEASE_CHANNEL` to `stable`, `preview`, `canary`, `nightly`, `lts`, or `dev` (with no trailing newline) when packaging a non-dev build so credential storage uses the system keychain.

### Other things to note

- **Local-first defaults**: Aphrody ships with telemetry (`telemetry.diagnostics = false`, `telemetry.metrics = false`), automatic updates (`auto_update = false`), and sign-in/user-menu bars disabled in [`assets/settings/default.json`](https://github.com/aphrody-dev/aphrody/blob/main/assets/settings/default.json).
- **Toolchains & Extensions**: Aphrody maintains `zed:extension` WIT compatibility and can install user-requested language servers or extensions locally.

## Flatpak

> Current Flatpak integration exits the sandbox on startup when accessing host toolchains. Workflows that rely on strict Flatpak sandboxing may not work as expected.

To build & install the Flatpak package locally follow the steps below:

1. Install Flatpak for your distribution as outlined [here](https://flathub.org/setup).
2. Run the `script/flatpak/deps` script to install the required dependencies.
3. Run `script/flatpak/bundle-flatpak`.
4. Now the package has been installed and has a bundle available at `target/release/{app-id}.flatpak`.

## Memory profiling

[`heaptrack`](https://github.com/KDE/heaptrack) is quite useful for diagnosing memory leaks. To install it:

```sh
$ sudo apt install heaptrack heaptrack-gui
$ cargo install cargo-heaptrack
```

Then, to build and run Zed with the profiler attached:

```sh
$ cargo heaptrack -b zed
```

When this zed instance is exited, terminal output will include a command to run `heaptrack_interpret` to convert the `*.raw.zst` profile to a `*.zst` file which can be passed to `heaptrack_gui` for viewing.

## Perf recording

How to get a flamegraph with resolved symbols from a running Zed instance.
Use this when Zed is using a lot of CPU. It is not useful for hangs.

### During the incident

- Find the PID (process ID) using:
  `ps -eo size,pid,comm | grep zed | sort | head -n 1 | cut -d ' ' -f 2`
  Or find the PID of `zed-editor` with the highest RAM usage in something
  like htop/btop/top.

- Install perf:
  On Ubuntu (derivatives) run `sudo apt install linux-tools`.

- Perf record:
  Run `sudo perf record -g --call-graph dwarf -p <pid you just found>`, wait a few seconds to gather data, then press Ctrl+C. You should now have a `perf.data` file.
  The `--call-graph dwarf` option records callers of every sampled function and unwinds through the `.eh_frame` data kept in stripped release binaries; plain `-g` does not work without frame pointers.

- Make the output file user owned:
  run `sudo chown $USER:$USER perf.data`

- Get build info:
  Run zed again and type {#action zed::About} in the command pallet to get the exact commit.

The `perf.data` file can be sent to Zed together with the exact commit.

### Later

This can be done by Zed staff.

Released Linux binaries are stripped, but the unstripped binary for every release is archived in Sentry (uploaded by `script/bundle-linux`).
Download it from the Sentry project's Debug Files page (search by the release version or the binary's build id), then continue with the `perf buildid-cache` step below.
Stripping preserves the build id, so the downloaded binary matches the user's `perf.data`.

Alternatively, rebuild the binary with symbols:

- Check out the commit found previously and modify `Cargo.toml`.
  Apply the following diff, then make a release build.

```diff
[profile.release]
-debug = "limited"
+debug = "full"
```

- Add the symbols to the perf database:
  `perf buildid-cache -v -a <path to release zed binary>`

- Resolve the symbols from the db:
  `perf inject -i perf.data -o perf_with_symbols.data`

- Install flamegraph:
  `cargo install cargo-flamegraph`

- Render the flamegraph:
  `flamegraph --perfdata perf_with_symbols.data`

## Troubleshooting

### Cargo errors claiming that a dependency is using unstable features

Try `cargo clean` and `cargo build`.
