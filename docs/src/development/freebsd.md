---
title: Building Aphrody for FreeBSD
description: "Guide to building Aphrody for FreeBSD from source."
---

# Building Aphrody for FreeBSD

FreeBSD is not currently a supported platform, so this guide is experimental.

## Repository

Clone the [Aphrody repository](https://github.com/aphrody-dev/aphrody) with submodules (or sync via `bun run submodules:sync`).

## Dependencies

- Install the necessary system packages and rustup:

  ```sh
  script/freebsd
  ```

  If preferred, you can inspect [`script/freebsd`](https://github.com/aphrody-dev/aphrody/blob/main/script/freebsd) and perform the steps manually.

## Building from source

Once the dependencies are installed, you can build Aphrody using [Cargo](https://doc.rust-lang.org/cargo/).

For a debug build of the editor (or `--no-default-features` to omit WebRTC/collab):

```sh
cargo run -p zed --no-default-features
```

And to run the tests:

```sh
cargo test --workspace
```

In release mode, the primary CLI interface is the `cli` crate. You can run it in development with:

```sh
cargo run -p cli
```

### WebRTC Notice

Building `webrtc-sys` on FreeBSD currently fails due to missing upstream support and unavailable prebuilt binaries. Collaboration features that depend on WebRTC (audio calls and screen sharing) are disabled unless building without the `collab` feature (`cargo run -p zed --no-default-features`).

## Troubleshooting

### Cargo errors claiming that a dependency is using unstable features

Try `cargo clean` and `cargo build`.
