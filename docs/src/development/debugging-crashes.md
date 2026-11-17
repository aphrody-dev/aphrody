---
title: Debugging Crashes
description: "Guide to debugging crashes for Aphrody development."
---

# Debugging Crashes

When Aphrody panics or crashes with crash reporting enabled, it notifies a sidecar process that inspects editor memory and writes a [minidump](https://chromium.googlesource.com/breakpad/breakpad/+/master/docs/getting_started_with_breakpad.md#the-minidump-file-format) to `~/Library/Logs/Aphrody`, `$XDG_DATA_HOME/aphrody/logs`, or `%LOCALAPPDATA%\Aphrody\logs`.

Aphrody ships with telemetry and automatic crash uploads disabled by default (`telemetry.diagnostics = false`, `telemetry.metrics = false`). You can analyze local minidumps directly by building or fetching an unstripped binary (or symbols) for your revision and running:

```sh
zstd -d ~/.local/share/aphrody/<uuid>.dmp -o minidump.dmp
minidump-stackwalk minidump.dmp
```

Alongside the minidump in your logs directory, you should also see a `<uuid>.json` file with metadata such as the panic message, span, and system specs.

## Using a Debugger

If you can reproduce the crash consistently, use a debugger to inspect program state at the crash point.

For setup details, see [Using a debugger](./debuggers.md#debugging-panics-and-crashes).
