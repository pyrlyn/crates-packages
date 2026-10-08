# crates — completed tasks

### T20.1. wasm-plugin-host: engine and worker

First slice of the shared WASM plugin host that cox and scull both build on (creator, 2026-10-07: extract the shared crate, `MIT OR Apache-2.0`). New crate `wasm-plugin-host`: `PluginHost` loads a module with extism (no WASI, no cache, a memory cap and a per-call deadline clamped to host maxima), runs it on a worker thread of its own with a control lane served before an event lane, cancels a call at its deadline through a watchdog, and maps extism failures to a typed `PluginError`. Host functions come from a `HostEnv` trait the application implements; the required export, thread names and limit defaults are options, so nothing names cox. Ported from cox `crates/cox-plugin/src/host.rs` and `error.rs` with no behaviour change. Done when the crate is in the workspace with the ported tests, registered for CI dry-run publish, bump, README and sonar.

Merged in pyrlyn/crates-packages#33 (`77ba2b1`): 10 tests, clippy, fmt, publish dry-run and `cargo +1.98 check` green. `rust.md` lists the crate (listepo/workspace#5, `0af0a5c`).

### T11. Add the required project files

The project was missing `AGENTS.md`, `done.md`, `roadmap.md`, `ideas.md`, and `toolchain.md`. Done means: all files exist, and `toolchain.md` lists the workspace crates per the rulebook (also update `rust.md` then).

Those files are in the workspace root. `toolchain.md` lists `cargo-changed-packages`, `file-backup`, `git-changed-paths`, `path-gates`, and `scoped-check`. `rust.md` names the same five under shared crates. `file-backup` tests already live in `src/lib.rs`; the other four crates already have `tests/`.
