# Compiling TypeScript to native code: scriptc and Bun AOT

## vercel-labs/scriptc

- "TypeScript-to-Native Compiler", Apache-2.0, experimental.
- Compiles TypeScript and JavaScript to native executables and WebAssembly using the types checked by the TypeScript compiler; no JavaScript engine embedded, Node.js not needed at run time.
- Covers a subset of JavaScript, TypeScript and the Node.js APIs; `scriptc coverage` reports what compiles; diagnostics and rewrite hints for the rest; values of unknown type (`unknown`, `any`, `JSON.parse`) are stored with their type and checked on conversion.
- npm dependencies compiled into the executable (partial support).
- Native integration: `--emit=obj` writes a relocatable object to link from Xcode, CMake or a linker; `--lib --profile` builds a library a host application calls; FFI to C declared in TypeScript and described by a JSON manifest.
- Installed with npm (Node.js 24+) or standalone archives.

## Bun `--compile` and AOT

- Produces a standalone executable embedding the Bun runtime (JavaScriptCore) and the program: full JavaScript semantics, every Bun and Node.js API Bun supports.
- `bytecode`: precompiled bytecode (faster start); `bytecodeOrder`: bytecode layout driven by a run profile.
- `aot: true`: compiles the program to machine code ahead of time; `aotExplain` reports the guards it set; `aotHarden` adds run-time checks; `obfuscate` strips names and locations; `allowCodeFromDisk` is off by default.
- `trim` removes the Bun and Node.js APIs never called.
- Cross-compilation to 9 targets (macOS, Linux glibc and musl, Windows, x64 and arm64).

## Comparison

| | scriptc | Bun AOT |
|---|---|---|
| Run-time engine | none (native code, minimal runtime) | JavaScriptCore embedded, program in machine code |
| Language coverage | subset, coverage report | complete |
| APIs | subset of Node.js | Bun + Node.js |
| Outputs | executable, WebAssembly, native object, library | executable |
| C interop | FFI by manifest, linkable object and library | `bun:ffi` |
| Maturity | experimental | part of Bun |

Reading: scriptc targets engine-less binaries and a linkable ABI (useful for the shared C ABI); Bun AOT keeps every semantic and API. Both get measured on the benchmark (size, start-up, throughput, memory, coverage); no figure is kept without a measurement.

## Cross TypeScript toolchain

Tools: oxc (parse, lint, format), `bun check`, TypeScript 7 (native compiler), scriptc, Bun AOT. References: the ECMAScript specification (ECMA-262, TC39) and Microsoft's TypeScript documentation. Goal: a system that compares engines, runtimes, linters and compilers on the same programs, with benchmarks.

Observed: `typescript@latest` is 7.0.2; its JavaScript API changed (`typescript/unstable/*`); `ts.createProgram` is no longer on the default export.
