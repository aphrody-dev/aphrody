---
name: gpui-bench
description: >-
  Design, write, review, run, and interpret production-shaped GPUI Criterion
  benchmarks, including gpui::bench, BenchAppContext, renderer and task
  benchmarks, headless Metal frame data, responsiveness and hang regressions,
  feature isolation from test-support, and before/after performance evidence.
---

# GPUI Benchmarks

Use this skill when a user asks to benchmark GPUI code, reproduce a UI hang or frame drop, evaluate a performance fix, use `#[gpui::bench]`, interpret `BenchReport`, or review whether a GPUI benchmark represents production.

The primary goal is UI responsiveness. Throughput matters, but a UI that finishes work quickly while blocking input and frames is still regressed.

## Start with the performance question

Before editing, establish or derive:

1. What user-visible problem is being reproduced: slow computation, long foreground poll, delayed input, frame drops, scrolling hitch, GPU draw cost, or a true hang?
2. What production event starts the work, and what path performs it?
3. What competing UI work must remain responsive? Prefer a rendered progress indicator, cursor, spinner, or scrolling frame when practical.
4. What target frame rate applies? GPUI defaults to 120 FPS, an 8.33 ms frame budget. At 60 FPS the budget is 16.67 ms.
5. What fixed workload sizes expose a progression from normal to degraded to severe?
6. What state proves the workload completed without dropping, duplicating, or reordering work?
7. Which commits are the baseline and candidate, and can the exact same benchmark code run on both?

Ask only for inputs that cannot be derived from the repository, issue, trace, or existing benchmark.

## Non-negotiable rules

- A benchmark must not enable any crate's `test-support` feature, directly or transitively.
- Use production constructors, storage, executors, rendering, synchronization, and data sizes whenever practical.
- Do not use `TestAppContext`, deterministic test executors, reduced test-only CRDT settings, fake clocks, fake filesystems, or fake services merely because setup is easier.
- A narrow benchmark seam may simulate an external boundary such as a PTY, server, or filesystem event, but everything after that boundary should follow the production path.
- A performance fix should normally include or extend a benchmark that reproduces its problem. If it does not, explain why before implementing the fix.
- Every benchmark or profile invocation run by an agent must have a hard timeout of at most five minutes. Use `timeout_ms <= 300000` with the terminal tool.
- Run smoke and quick modes before measured runs. Never start an unbounded profile or Criterion run.
- Do not run measured benchmarks concurrently. Parallel benchmark processes contaminate each other's results.
- Preserve correctness assertions in benchmark fixtures. Faster output that loses work is not an improvement.
- Never invent timing, frame, percentile, or throughput results.

## Verify feature isolation first

Use GPUI's canonical `bench-support` feature. The published `bench` feature is a temporary compatibility alias; new code should not use it. Neither feature may enable `test-support`.

Inspect the complete feature graph for the actual benchmark package:

```sh
feature_tree="$(cargo tree -p <benchmark-package> -e normal,build,dev,features)"
if grep -F 'feature "test-support"' <<<"${feature_tree}"; then
  echo 'benchmark graph contains test-support' >&2
  exit 1
fi
```

Also use inverse feature trees when diagnosing a leak:

```sh
cargo tree -p <benchmark-package> -e features -i gpui
cargo tree -p <benchmark-package> -e features -i <crate-under-benchmark>
```

Cargo unifies features across every target in one package. If an unrelated benchmark needs `test-support`, move the production benchmark to a separate package when necessary; selecting one `--bench` target does not undo package-wide feature unification.

Add a repository script or CI check for important isolation rules. Do not rely only on a one-time manual `cargo tree` inspection.

## Benchmark-only APIs

Use these feature names:

- `bench-support`: benchmark infrastructure and narrow, production-faithful constructors or external-boundary seams.
- `test-support`: test doubles, mutation hooks, failure injection, and test harnesses. It is forbidden in benchmark graphs.

The `#[gpui::bench]` attribute keeps its existing name. The Cargo feature describes support consumed by benchmark targets, not the benchmark declaration itself.

When a useful function is currently test-gated:

1. Determine whether its behavior is production-safe or whether it is a fake.
2. Extract shared production logic into a private primitive when possible.
3. Expose the smallest wrapper under `#[cfg(any(test, feature = "bench-support"))]`.
4. Keep mutations, failure injection, and fake global state test-only unless the benchmark specifically models that production boundary.
5. Add a parity test when the benchmark wrapper must match an existing test helper.
6. Verify that enabling `bench-support` alone brings in no `test-support` feature.

Prefer a seam that supplies bytes or events to a production pipeline over widening an entire fake test harness.

## Model the production path

Map the workload before constructing the fixture:

```text
production trigger
    -> queue or event boundary
    -> background preparation
    -> foreground application
    -> invalidation
    -> layout
    -> prepaint
    -> paint
    -> present
    -> observable completion
```

Keep each relevant boundary in the benchmark. For a queue or stream, drive enough work to exceed capacity and let a concurrent producer refill it while the consumer drains. A burst that fits entirely in the queue cannot reproduce sustained backpressure.

Use realistic expensive shapes, not only easy append-only inputs. For rendering, include representative text lengths, styles, images, tools, comments, viewport sizes, and invalidation patterns. Change one workload dimension at a time so scaling remains interpretable.

Separate each iteration into:

1. **Prepare:** construct or reset state outside timing.
2. **Measure:** perform the production operation.
3. **Validate:** assert output, work counts, ordering, and completion outside timing when possible.

State whether caches, glyph atlases, storage, and other fixtures are cold or warm. Do not accidentally compare a cold baseline with a warm candidate.

## Choose the right GPUI measurement API

`#[gpui::bench]` creates a Criterion benchmark using `BenchAppContext` and a production-style threaded dispatcher. The benchmark function itself is synchronous; use the context's task APIs for asynchronous work.

The attribute supports options such as:

```rust
#[gpui::bench(
    fps = 120,
    inputs = workload_sizes(),
    input_name = "items",
    group = "Streaming update",
    sample_size = 10
)]
```

Inspect the current macro before depending on an option because the API is evolving.

For randomized workloads, take a `StdRng` parameter (by value) and draw the workload from it instead of hand-rolling seeds. The macro then runs one Criterion benchmark per seed, named `<input>/seed-<n>`, with `seed = N`, `seeds(...)`, `iterations = N`, `SEED`, and `ITERATIONS` meaning exactly what they do for `#[gpui::test]`, so `SEED=3` reproduces the same workload in a test and a benchmark. The RNG is rebuilt for every Criterion routine call, so warm-up and every sample see the same workload.

```rust
#[gpui::bench(inputs = tree_families(), input_name = "tree", iterations = 6)]
fn full_refresh(family: &TreeFamily, mut rng: StdRng, cx: &mut gpui::BenchAppContext) {
    let tree = family.sample(&mut rng);
    // ...
}
```

Register benchmark functions with `gpui::bench_group!` and `gpui::bench_main!` (drop-in for Criterion's `criterion_group!`/`criterion_main!`, including the `name = ...; config = ...; targets = ...` form). The generated functions take `&mut criterion::Criterion<gpui::BenchMeasurement>`; `bench_group!` supplies that measurement from the environment. `bench_main!` also installs `gpui::CountingAllocator` as the binary's global allocator, so a GPUI bench file must not declare its own `#[global_allocator]`; to measure with another allocator, write `gpui::bench_main!(allocator = mimalloc::MiMalloc; benches)`, which wraps it. `allocator = none;` installs no allocator and drops the allocation metrics.

### Measurements: wall time, hardware counters, rusage, and allocations

`gpui::BenchMeasurement` (from the gpui-independent `bench_metrics` crate) records several metrics per Criterion sample. Criterion analyzes one of them (the *primary*, shown on its `time:` line with confidence intervals and baseline comparison); the rest are *secondaries*, printed per iteration in the GPUI bench report as median/min/max. `BENCH_MEASUREMENT` selects the arrangement:

| `BENCH_MEASUREMENT` | Criterion analyzes | Reported as secondaries |
|---|---|---|
| unset (default) | wall time | every counter the machine supports (below) |
| `instructions` | process-wide retired instructions; fails fast if counters are unavailable | the rest, plus wall time |
| `foreground-instructions` | the benchmark thread's retired instructions; fails fast if unavailable | the rest, plus wall time |
| `allocations` | process-wide heap allocations; fails fast without `CountingAllocator` | the rest, plus wall time |
| `foreground-allocations` | the benchmark thread's heap allocations, without background workers' timing noise; fails fast without `CountingAllocator` | the rest, plus wall time |
| `wall-time` | wall time | nothing; no counters are opened |

For a CI gate on frame cost prefer `foreground-instructions`: it excludes background parsing and GPU-driver submission threads, whose work varies run to run, and on Apple Silicon the process-wide count includes driver jitter that widens the interval (about ±0.6% versus ±0.01% for the foreground thread).

Default secondaries, each skipped with a one-time note when unavailable:

- **instructions** (process-wide) and **foreground instructions** (the benchmark thread only, where GPUI polls tasks, lays out, paints, and submits). Near-deterministic, so a ~0.1% spread is typical and small regressions are detectable. Blind to waits, memory stalls, and GPU shader work. Their ratio, **foreground share of instructions**, shows work moving between the foreground and background threads even when the total is flat; with a headless renderer it also exposes driver-thread submission work (about 7% of a small Metal frame).
- **cycles** and **IPC** (`instructions / cycles`). Falling IPC with flat instructions means worse cache or branch behavior, not more work.
- **branch misses** and **cache misses** (last-level), for diagnosing an IPC drop. Linux only.
- **foreground context switches**: voluntary switches on the benchmark thread per iteration, i.e. how many times a frame blocked on a lock, channel, or fence. This is the metric that catches the waits instructions cannot see. Linux only.
- **page faults**: minor faults per iteration, a privilege-free proxy for fresh memory touched.
- **allocations**, **foreground allocations**, and **allocated bytes**: heap allocations (reallocations count as one each) across every thread and on the benchmark thread alone, and the bytes requested across every thread, per iteration. Background workers allocate on their own schedule, so the foreground count excludes that noise. It can still vary between samples when the workload does, e.g. frames coalescing or caches growing in early samples, so a CI gate needs a small tolerance. Arena placements (GPUI's element arena) are not heap allocations and are not counted once the arena is warm. Needs `bench_metrics::CountingAllocator` as the global allocator, which `gpui::bench_main!` installs; it wraps any allocator (`CountingAllocator<MiMalloc>`) and costs a few relaxed atomics per allocation. For heap held between iterations, read `bench_metrics::allocation_stats()` before and after setup; `live_bytes()` is allocated minus freed.

Run one benchmark process per machine at a time. Hardware counters are per task, so instruction counts stay correct next to a concurrent `cargo bench`, but wall time, IPC, and cache misses share the hardware and degrade, and two processes requesting counters can exceed a core's counter slots, at which point the kernel multiplexes and the counts become scaled estimates (a warning is printed once). Within one process, benchmarks already run sequentially; process-wide counters can still attribute a previous benchmark's trailing background work to the next benchmark's first sample, which the foreground-thread metrics are immune to.

Platform support: on Linux, hardware counters come from `perf_event_open` and need `CAP_PERFMON` or `/proc/sys/kernel/perf_event_paranoid` at 2 or below; Docker's default seccomp profile and many cloud VMs block it, and the default mode then reports wall time plus the `getrusage` metrics. On Apple Silicon, instructions and cycles (both scopes) come from `proc_pid_rusage` and `thread_selfcounts` with no privileges; Intel Macs have no source. Criterion baselines are keyed by benchmark id, not measurement, so use distinct `--save-baseline` names when switching the primary on one machine.

Never present instructions alone for a latency claim; pair them with wall time and the frame metrics from the same run. Other Criterion measurements plug in with `criterion_group!` and `config = Criterion::default().with_measurement(gpui::BenchMeasurement::new(primary).with_secondary("name", other))`.

#### Benchmarks outside GPUI

`bench_metrics` has no GPUI dependency, so any Criterion benchmark in the workspace (`rope`, `sum_tree`, `text`, ...) gets the same metrics and the same `BENCH_MEASUREMENT` switch. Add `bench_metrics.workspace = true` to `[dev-dependencies]`, take `&mut Criterion<bench_metrics::BenchMeasurement>` in the benchmark functions, and register them with:

```rust
criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_measurement(bench_metrics::BenchMeasurement::from_env_or_exit());
    targets = rope_benchmarks
}
```

That alone switches Criterion's analyzed metric with `BENCH_MEASUREMENT=instructions`. To also print the per-iteration secondaries, run each routine through a `MetricReport`, which counts iterations for you:

```rust
let report = bench_metrics::MetricReport::new();
group.bench_function("append", |b| report.iter(b, || rope.append(small.clone())));
report.print("  ");
```

Criterion's `Throughput` lines keep working alongside. Page faults per iteration are a useful allocation-pressure signal even without a counting allocator; for exact counts, install `bench_metrics::CountingAllocator` as the binary's `#[global_allocator]`.

### `bench_iter`: synchronous functions and compute work

Use `bench_iter` for synchronous application or compute code:

```rust
#[gpui::bench(inputs = sizes(), input_name = "bytes", group = "Parse")]
fn parse_document(byte_count: &usize, cx: &mut gpui::BenchAppContext) {
    let input = build_input(*byte_count);
    cx.bench_iter(|_| {
        std::hint::black_box(parse(std::hint::black_box(&input)));
    });
}
```

Build reusable fixtures outside the measured closure. Use `black_box` for pure computation whose result could otherwise be optimized away. If the code does not interact with GPUI, ordinary Criterion may be simpler.

### `bench_task`: asynchronous work to completion

Use `bench_task` when the measured operation returns a GPUI `Task` and setup can be reused:

```rust
cx.bench_task(|cx| fixture.run_one_workload(cx));
```

This captures foreground task polls, action handlers, input dispatches, and draws produced while the task runs. It can surface a long foreground poll even when no window manages to draw during the stall.

### `bench_batched_task`: per-iteration setup outside timing

Use `bench_batched_task` when each iteration needs fresh input or state:

```rust
cx.bench_batched_task(
    |cx| fixture.prepare_iteration(cx),
    |iteration, cx| iteration.run(cx),
);
```

The setup result and task output are dropped outside the timed operation. This is the preferred shape for queues, streams, synchronization, and workloads whose setup must not contaminate measurement.

### `bench_renderer`: frame production

Use `bench_renderer` when each iteration updates an entity in a window's render tree and should draw and present a frame:

```rust
let mut window = cx.add_empty_window();
let view = window.update(|window, cx| {
    window.replace_root(cx, |_window, cx| cx.new(|_| BenchmarkView::new()))
});

cx.bench_renderer(view, |view, _window, cx| {
    view.advance_one_step();
    cx.notify();
});
```

Do not benchmark a detached entity and call the result rendering performance. Do not call `run_until_idle` inside a measured loop merely to make the fixture pass: production does not drain all asynchronous work before every frame, and doing so erases the scheduling behavior being measured.

## Profiling, Responsiveness & Review Checklist

Detailed guidance for headless rendering metrics (`BenchReport`), responsiveness vs throughput benchmarks, hang reproduction, before-and-after methodology, Instruments / `xctrace` / Linux `perf` profiling, multi-agent benchmark coordination, and the accept checklist lives in [references/profiling-and-checklist.md](references/profiling-and-checklist.md).

Always verify feature isolation (`cargo tree -p <pkg> -e normal,build,dev,features` has zero `test-support`), bound every benchmark or profile invocation to `<= 300000` ms, never run measured benchmarks concurrently, and report raw before/after percentiles honestly.
