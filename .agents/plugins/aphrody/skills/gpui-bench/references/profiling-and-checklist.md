# GPUI Benchmark Profiling, Responsiveness & Review Checklist

Detailed reference guide extracted from the main `gpui-bench` skill so the skill stays concise within Antigravity's 24 KB limit.

## Headless rendering and frame data

`#[gpui::bench]` constructs its platform through `gpui::bench_platform` and requests `gpui_platform::current_headless_renderer`.

On macOS, the headless renderer uses Metal without showing a window; on Linux/Windows headless CI (`aphrody_desktop` / `gpui_wgpu`), verify real GPU submission explicitly or document headless no-op limits.

`BenchReport` metrics (per benchmark and per input):

- secondary measurements per iteration (median, min, max): instructions, foreground instructions, cycles, IPC, branch and cache misses, foreground context switches, page faults;
- dirty-to-draw duration, draw and present intervals, invalidations per frame;
- foreground task, action, and input-dispatch durations;
- p50, p90, p95, p99, and maximum durations;
- total and maximum frame-budget overruns (synthetic missed-deadline proxy, not literal display vsync drops).

## Responsiveness benchmarks

Measure both **Responsiveness** (longest uninterrupted foreground poll, p95/p99/max foreground duration, frame-budget overruns, dirty-to-draw delay, frame cadence) and **Completion** (total completion time + throughput) while keeping an independent foreground UI signal alive (spinner, cursor blink, scrolling, progress counter).

## Reproducing hangs safely

- Drive more work than queue capacity and let a concurrent producer refill while consumer drains.
- Use the production blocking, wake, gather, synchronization, and rendering path with realistic expensive inputs.
- Assert final state, ordering, and completion; keep deterministic regression tests alongside Criterion benches; bound every bench/profile invocation to <= 5 minutes (`timeout_ms <= 300000`).

## Before-and-after method & optimization order

1. Profile before optimizing: reduce total work/algorithmic complexity -> bound to viewport/change -> eliminate allocations/cloning -> improve data locality -> SIMD.
2. Run the exact same benchmark code on baseline and candidate with identical build profiles (`--profile release-fast`), features, and lockfile, starting with smoke mode and alternating runs if noisy.
3. Report raw percentiles, ranges, and confidence intervals without hiding tradeoffs.

## Profile with Instruments, xctrace, and perf

For symbol-attributed macOS builds:

```sh
RUSTFLAGS="-C link-arg=-Wl,-no_deduplicate -C codegen-units=1" \
  cargo bench -p <package> --bench <target> \
  --features bench-support --profile release-fast --no-run
dsymutil target/release-fast/deps/<target>-<hash>
xctrace record \
  --template "zed-bench" \
  --output /tmp/zed-profiles/<name>.trace \
  --launch -- target/release-fast/deps/<target>-<hash> \
  --bench <workload-filter> --profile-time 10
```

Inspect focused exports (never load raw full traces into context). On Linux (`fr1` / Ubuntu 26.10), pair `perf` (`kernel.perf_event_paranoid = 1`) / `flamegraph` with `gpui::BenchMeasurement` hardware counters.

## Using agents well

Delegate independent read-only analysis (production-path auditor, feature-isolation auditor, workload designer, profile analyst) while keeping a single owner for benchmark implementation and measured runs so CPU/GPU timings never contend concurrently.

## Review checklist

- [ ] User-visible performance hypothesis stated.
- [ ] Zero `test-support` occurrences in the benchmark feature graph (`cargo tree -e normal,build,dev,features`).
- [ ] Any `bench-support` seam is narrow and production-faithful.
- [ ] Production storage, executors, and render paths exercised; setup excluded from timed intervals.
- [ ] Deterministic, realistic, severe inputs with controlled cache state.
- [ ] Competing UI responsiveness signal present where foreground starvation matters.
- [ ] Frame metrics, completion throughput, instructions/IPC, and context switches reported together.
- [ ] Final correctness, ordering, and work counts asserted under bounded (<= 5 min) runs.
- [ ] Identical benchmark code and raw percentiles/regressions reported across baseline and candidate.

## Reporting results

Lead with whether the user-visible responsiveness problem was reproduced and resolved, then report:
1. Commits, command, profile, platform, FPS, inputs, sample config.
2. Foreground/frame metrics plus total completion before and after.
3. Correctness and work-count evidence.
4. Feature-isolation evidence (`cargo tree` check).
5. Headless-renderer limitations and noise sources.
6. Next hotspot to profile.

## References

- [Criterion measurement analysis](https://bheisler.github.io/criterion.rs/book/analysis.html)
- [Criterion advanced configuration](https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html)
- [Criterion external profiling](https://bheisler.github.io/criterion.rs/book/user_guide/profiling.html)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/general-tips.html)
